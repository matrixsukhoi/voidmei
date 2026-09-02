//! 真机 FM 集成测试 (D4 验收项) — Java `test/` 三套真机 FM 测试的一比一移植:
//! - [`spitfire`] ← `test/TestSpitfireF24Power.java`
//! - [`tempest`]  ← `test/TestTempestMk5Power.java`
//! - [`fuzzer`]   ← `test/FMParserFuzzer.java`
//!
//! PORT (data/ 缺失跳过): Java 端由 build.py 的 run_fm_test 机制探测 data/ 决定
//! 是否挂起整套 (FMParserFuzzer 类注); Rust 端对齐 D4 验收注语义 — 每个测试在
//! 数据文件缺失时 return early (reader.rs real 先例, "data/ 缺失自动跳过, 对齐
//! build.py 语义")。路径相对仓库根: cargo 测试经 `CARGO_MANIFEST_DIR` 上溯三级
//! (reader.rs / sexp_parser.rs 测试同款约定)。
//!
//! PORT (blkx→json 迁移终态): 功率断言走 parse_real (parse_named_json);
//! fuzzer 变异对象为 JSON 原文 (合成种子承担 JavaRandom/mutate 移植对拍,
//! 真机种子腿为变异鲁棒性烟雾)。仍挂起: 腿2 的 FMLoader.load 抽样腿
//! (见 fuzz 模块内 TODO(port)); 禁为此放宽阈值或删断言 (§6)。
//!
//! oracle: fuzzer 的 JavaRandom/mutate 移植值来自 OpenJDK 1.8.0_342 实测 dump
//! (build/oracle/rand/RandOracle.java, §5.1 双实现对拍方法论)。

/// PORT(reader 波次开关): 见模块头注。getload/getAllplotdata 批次已落地,
/// 本开关已置 true — 依赖 getload 字段的功率断言段与 fuzz 腿1 管线全量执行。
/// 后续波次若再挂断言 (如腿2 FMLoader), 翻回 false 时必须同步 grep 本文件内
/// 全部 TODO(port) 逐个销号, 否则覆盖永久半挂; [`getload_wired_follows_reader_todo`]
/// canary 钉住 reader.rs 标注与本开关的一致性 (标注移除而开关未翻即判失败)。
const GETLOAD_WIRED: bool = true;

/// 项目内真机 FM 数据根 (cargo 测试 cwd 无关; data/ 缺失由各测试自行 return early,
/// 对齐 build.py 跳过语义 — reader.rs real 先例)
fn fm_root() -> String {
    format!(
        "{}/../../../data/aces/gamedata/flightmodels",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// 真机 JSON 全量解析 (blkx→json 迁移: name 取文件名分量, 对齐旧 parse 的
/// display 约定; read_file_name 只进 fmdata 版本串)
fn parse_real(path: &str) -> Result<crate::fmdata::FmData, String> {
    let name = std::path::Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    crate::fmdata::FmData::parse_named_json(path, &name)
}

/// 中央文件 JSON → 燃油修正 (读失败/serde 失败 → 默认无修正)
fn fuel_mod_from_json(path: &str) -> crate::fmdata::types::FuelModification {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .map(|root| crate::fmdata::json::extract_fuel_modifications_json(&root))
        .unwrap_or_default()
}

/// GETLOAD_WIRED ↔ reader.rs getload TODO(port) 一致性 canary:
/// reader 波次把 getload 接入 parse 时, 按 PORTING.md §0.4 纪理会移除 reader.rs 内
/// 的 `TODO(port): getload` 标注 — 标注已消失而开关仍为 false 即"永久假通过"
/// (no-fake-test-pass), 此处直接判失败, 强制翻开关恢复全量断言。
/// 反向 (标注在而开关 true) 不判错: 翻开关先行、标注清理同波次补齐属正常顺序。
#[test]
fn getload_wired_follows_reader_todo() {
    let reader_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/fmdata/reader.rs"
    ))
    .expect("reader.rs 源码可读");
    // PORT: 常量哨兵断言是 no-fake-test-pass 机制本体, 保真不削
    #[allow(clippy::assertions_on_constants)]
    if !reader_src.contains("TODO(port): getload") {
        assert!(
            GETLOAD_WIRED,
            "reader.rs 的 getload TODO(port) 已移除 (波次已落地?) 但 GETLOAD_WIRED 仍为 false \
             — 挂起理由失效, 须翻开关并 grep 本文件全部 TODO(port) 逐个销号"
        );
    }
}

// ==================== spitfire ← test/TestSpitfireF24Power.java ====================

mod spitfire {
    //! Verifies power curve calculations for Spitfire F24 (150 octane fuel aircraft).
    //!
    //! This test validates:
    //! 1. invertEnableLogic detection for British 150 octane fuel
    //! 2. Correct WEP parameter extraction with/without fuel modification
    //! 3. Power curve accuracy vs wtapc reference values
    //!
    //! Run with: ./script/test.sh spitfire

    use super::GETLOAD_WIRED;
    use super::fm_root;
    use super::{fuel_mod_from_json, parse_real};
    use crate::fmdata::FuelType;
    use crate::fm_power_extractor::{extract_stages, extract_stages_with_fuel};
    use crate::piston_power_model::optimal_power_advanced;
    use std::path::Path;

    // wtapc reference values at 300 km/h IAS, 15°C
    // Command: python wtapc.py --fm ... --central ... --ias 300
    const WTAPC_MIL: [[f64; 2]; 13] = [
        [0.0, 1347.4], [1000.0, 1389.7], [2000.0, 1428.2], [3000.0, 1462.9],
        [4000.0, 1494.3], [4100.0, 1510.0],  // Peak at critical alt
        [5000.0, 1419.6], [6000.0, 1281.1], [7000.0, 1304.3],
        [8000.0, 1325.0], [8100.0, 1340.0],  // Stage 2 peak
        [9000.0, 1309.1], [10000.0, 1170.6],
    ];

    const WTAPC_WEP: [[f64; 2]; 12] = [
        [0.0, 2172.0], [1000.0, 2240.3], [1830.0, 2292.5],  // WEP peak
        [2000.0, 2252.5], [3000.0, 2021.5], [4000.0, 1917.4],
        [5000.0, 1963.1], [6000.0, 2003.8], [7000.0, 1884.3],
        [8000.0, 1719.0], [9000.0, 1564.8], [10000.0, 1408.8],
    ];

    /// Java static passed/failed 计数器 (软断言, 全部执行完才判失败 — Java main
    /// 尾部 failed>0 才 exit(1) 的语义, 每个测试方法独立持有)
    struct Tally {
        passed: usize,
        failed: usize,
    }

    impl Tally {
        fn new() -> Self {
            Tally { passed: 0, failed: 0 }
        }

        /// Java main 尾: Summary 打印 + `if (failed > 0) System.exit(1);`
        fn finish(&self) {
            println!("\n=== Results ===");
            println!("Passed: {}, Failed: {}", self.passed, self.failed);
            assert_eq!(self.failed, 0, "TestSpitfireF24Power 存在失败断言");
        }
    }

    fn spitfire_paths() -> (String, String) {
        // PORT: Rust 测试固定仓库相对路径 (fm_root 先例), data 缺失 return early
        let central = format!("{}/spitfire_f24.json", fm_root());
        let fm = format!("{}/fm/spitfire_f24.json", fm_root());
        (central, fm)
    }

    fn have_data(central: &str, fm: &str) -> bool {
        Path::new(central).is_file() && Path::new(fm).is_file()
    }

    // ==================== Test Cases ====================

    #[test]
    fn test_fuel_modification_parsing() {
        let (central_path, _fm_path) = spitfire_paths();
        if !have_data(&central_path, &_fm_path) {
            return; // data/ 未解包, 对齐 build.py 跳过语义 (模块头注)
        }
        let mut t = Tally::new();
        println!("Testing fuel modification parsing...");

        let fuel_mod = fuel_mod_from_json(&central_path);

        // Verify fuel type detected
        t.assert_true(
            "detected British 150 octane fuel",
            fuel_mod.r#type == FuelType::British150Octane,
        );

        // Verify invertEnableLogic parsing
        // Spitfire F24 has invertEnableLogic:false in the datamine
        t.assert_false(
            "invertEnableLogic is false (150 octane is upgrade, not default)",
            fuel_mod.british_invert_logic,
        );

        // Verify fuel modification multipliers
        t.assert_close("afterburnerMult", fuel_mod.british_afterburner_mult, 1.42, 0.01);
        t.assert_close(
            "afterburnerCompressorMult",
            fuel_mod.british_afterburner_compressor_mult,
            1.33,
            0.01,
        );

        println!("  Fuel type: {}", fuel_mod.r#type);
        println!("  invertEnableLogic: {}", fuel_mod.british_invert_logic);
        println!("  afterburnerMult: {:.3}", fuel_mod.british_afterburner_mult);
        println!("  afterburnerCompressorMult: {:.3}", fuel_mod.british_afterburner_compressor_mult);
        t.finish();
    }

    #[test]
    fn test_parameter_extraction() {
        let (central_path, fm_path) = spitfire_paths();
        if !have_data(&central_path, &fm_path) {
            return; // data/ 未解包 (build.py 跳过语义)
        }
        let mut t = Tally::new();
        println!("\nTesting parameter extraction...");

        // Parse FM file
        // PORT: Blkx::parse 当前等价 doLoad=false (getload 属 reader 波次, 模块头注);
        // Java 显式传的 name ("spitfire_f24") 由 parse 取文件名分量承接 — 该值只在
        // getload L1471 版本串使用, 未落地前无行为差异 (reader.rs L51-56 注)
        let fmdata = match parse_real(&fm_path) {
            Ok(b) => b,
            Err(_) => {
                println!("  SKIP: Cannot parse FM file");
                return;
            }
        };

        // PORT: 以下断言全部依赖 getload 填充字段或 extract_stages, getload 未译前
        // 挂起 (模块头注 GETLOAD_WIRED); Java 原流程此处直接执行
        if !GETLOAD_WIRED {
            eprintln!("  SKIP: getload 未译 (reader 波次 TODO(port)), 参数提取断言暂挂");
            return;
        }

        // Extract WITHOUT fuel modification
        let stages_no_fuel = extract_stages(Some(&fmdata));
        t.assert_not_null("extracted stages without fuel", &stages_no_fuel);
        t.assert_true(
            "has 2 stages",
            stages_no_fuel.is_some() && stages_no_fuel.as_ref().unwrap().len() == 2,
        );

        if let Some(stages) = stages_no_fuel.as_ref() {
            println!("\n  === Without Fuel Modification ===");
            for (i, s) in stages.iter().enumerate() {
                println!("  Stage {i}:");
                println!("    critAlt: {:.0}m, critPower: {:.1}hp", s.crit_alt, s.crit_power);
                println!("    deckPower: {:.1}hp", s.deck_power);
                println!("    wepCritAlt: {:.0}m, wepPowerMult: {:.4}", s.wep_crit_alt, s.wep_power_mult);
            }
        }

        // Extract WITH fuel modification
        let fuel_mod = fuel_mod_from_json(&central_path);
        let stages_with_fuel = extract_stages_with_fuel(Some(&fmdata), Some(&fuel_mod));

        if let Some(stages) = stages_with_fuel.as_ref() {
            println!("\n  === With Fuel Modification ===");
            for (i, s) in stages.iter().enumerate() {
                println!("  Stage {i}:");
                println!("    critAlt: {:.0}m, critPower: {:.1}hp", s.crit_alt, s.crit_power);
                println!("    deckPower: {:.1}hp", s.deck_power);
                println!("    wepCritAlt: {:.0}m, wepPowerMult: {:.4}", s.wep_crit_alt, s.wep_power_mult);
            }
        }

        // Verify expected FM parameters
        t.assert_close("compressor NumSteps", fmdata.comp_num_steps as f64, 2.0, 0.0);
        t.assert_close("Stage 0 altitude", fmdata.comp_alt.as_ref().unwrap()[0], 4100.0, 0.0);
        t.assert_close("Stage 1 altitude", fmdata.comp_alt.as_ref().unwrap()[1], 8100.0, 0.0);
        t.assert_close("Stage 0 power", fmdata.comp_power.as_ref().unwrap()[0], 1510.0, 0.0);
        t.assert_close("Stage 1 power", fmdata.comp_power.as_ref().unwrap()[1], 1340.0, 0.0);
        t.assert_close("AfterburnerBoost", fmdata.aftb_coff, 1.41, 0.01);
        t.assert_close("AfterburnerManifoldPressure", fmdata.wep_manifold_pressure, 2.22, 0.01);
        t.assert_close("SpeedManifoldMultiplier", fmdata.speed_to_manifold_multiplier, 0.8, 0.01);
        t.finish();
    }

    #[test]
    fn test_invert_enable_logic_behavior() {
        let (central_path, fm_path) = spitfire_paths();
        if !have_data(&central_path, &fm_path) {
            return; // data/ 未解包 (build.py 跳过语义)
        }
        let mut t = Tally::new();
        println!("\nTesting invertEnableLogic behavior...");

        let fmdata = match parse_real(&fm_path) {
            Ok(b) => b,
            Err(_) => {
                println!("  SKIP: Cannot load files");
                return;
            }
        };
        let fuel_mod = fuel_mod_from_json(&central_path);
        // extractFuelModifications 各分支皆返 new FuelModification() (Blkx.java
        // L63 起), 该 null 检查在 Java 本就是死代码; Rust 端同样无此冗余检查,
        // 行为一致

        // PORT: 以下依赖 extract_stages (getload 字段前置), getload 未译前挂起
        if !GETLOAD_WIRED {
            eprintln!("  SKIP: getload 未译 (reader 波次 TODO(port)), 断言暂挂");
            return;
        }

        // Since invertEnableLogic is FALSE for Spitfire F24:
        // - The modification represents ADDING 150 octane fuel
        // - WEP parameters SHOULD be boosted when fuel is applied

        let stages_no_fuel = extract_stages(Some(&fmdata));
        let stages_with_fuel = extract_stages_with_fuel(Some(&fmdata), Some(&fuel_mod));

        if let (Some(no_fuel), Some(with_fuel)) = (stages_no_fuel.as_ref(), stages_with_fuel.as_ref()) {
            // With invertEnableLogic=false, fuel mod SHOULD change WEP params
            let mut wep_changed = false;
            for i in 0..no_fuel.len() {
                let no_fuel_mult = no_fuel[i].wep_power_mult;
                let with_fuel_mult = with_fuel[i].wep_power_mult;
                let no_fuel_wep_alt = no_fuel[i].wep_crit_alt;
                let with_fuel_wep_alt = with_fuel[i].wep_crit_alt;

                println!(
                    "  Stage {i}: wepMult {:.4} → {:.4}, wepCritAlt {:.0} → {:.0}",
                    no_fuel_mult, with_fuel_mult, no_fuel_wep_alt, with_fuel_wep_alt
                );

                if (no_fuel_mult - with_fuel_mult).abs() > 0.001
                    || (no_fuel_wep_alt - with_fuel_wep_alt).abs() > 1.0
                {
                    wep_changed = true;
                }
            }

            t.assert_true(
                "WEP parameters change with fuel (invertEnableLogic=false)",
                wep_changed,
            );
        }
        t.finish();
    }

    #[test]
    fn test_power_curve_calculations() {
        let (central_path, fm_path) = spitfire_paths();
        if !have_data(&central_path, &fm_path) {
            return; // data/ 未解包 (build.py 跳过语义)
        }
        let mut t = Tally::new();
        println!("\nTesting power curve calculations vs wtapc...");

        let fmdata = match parse_real(&fm_path) {
            Ok(b) => b,
            Err(_) => {
                println!("  SKIP: Cannot parse FM file");
                return;
            }
        };
        let fuel_mod = fuel_mod_from_json(&central_path);

        // PORT: 功率曲线断言依赖 extract_stages (getload 字段前置), 未译前挂起
        if !GETLOAD_WIRED {
            eprintln!("  SKIP: getload 未译 (reader 波次 TODO(port)), 功率曲线断言暂挂");
            return;
        }

        // Use stages WITH fuel modification (since wtapc uses full upgrades)
        let stages = match extract_stages_with_fuel(Some(&fmdata), Some(&fuel_mod)) {
            Some(s) => s,
            None => {
                println!("  SKIP: Cannot extract stages");
                return;
            }
        };

        let speed_kmh = 300.0;
        let is_ias = true;
        let sea_level_temp = 15.0;

        // Test Military power curve
        println!("\n  === Military Power Curve (300 km/h IAS) ===");
        println!("  Alt(m)    VoidMei    wtapc    Diff");
        println!("  ------    -------    -----    ----");

        let mut mil_errors = 0;
        let mut max_mil_error = 0.0f64;
        for r#ref in &WTAPC_MIL {
            let alt = r#ref[0];
            let expected = r#ref[1];
            let actual = optimal_power_advanced(&stages, alt, false, speed_kmh, is_ias, sea_level_temp);
            let diff = actual - expected;
            let abs_diff = diff.abs();

            let status = if abs_diff < 5.0 { "✓" } else if abs_diff < 20.0 { "~" } else { "✗" };
            println!("  {alt:5.0}    {actual:7.1}    {expected:5.1}    {diff:+.1} {status}");

            if abs_diff > 1.0 {
                mil_errors += 1;
            }
            if abs_diff > max_mil_error {
                max_mil_error = abs_diff;
            }
        }

        // Test WEP power curve
        println!("\n  === WEP Power Curve (300 km/h IAS) ===");
        println!("  Alt(m)    VoidMei    wtapc    Diff");
        println!("  ------    -------    -----    ----");

        let mut wep_errors = 0;
        let mut max_wep_error = 0.0f64;
        for r#ref in &WTAPC_WEP {
            let alt = r#ref[0];
            let expected = r#ref[1];
            let actual = optimal_power_advanced(&stages, alt, true, speed_kmh, is_ias, sea_level_temp);
            let diff = actual - expected;
            let abs_diff = diff.abs();

            let status = if abs_diff < 5.0 { "✓" } else if abs_diff < 50.0 { "~" } else { "✗" };
            println!("  {alt:5.0}    {actual:7.1}    {expected:5.1}    {diff:+.1} {status}");

            if abs_diff > 1.0 {
                wep_errors += 1;
            }
            if abs_diff > max_wep_error {
                max_wep_error = abs_diff;
            }
        }

        // Summary
        println!("\n  === Accuracy Summary ===");
        println!(
            "  Military: max error {:.1} hp, {}/{} points within 1hp",
            max_mil_error,
            WTAPC_MIL.len() - mil_errors,
            WTAPC_MIL.len()
        );
        println!(
            "  WEP: max error {:.1} hp, {}/{} points within 1hp",
            max_wep_error,
            WTAPC_WEP.len() - wep_errors,
            WTAPC_WEP.len()
        );

        // Find peak values
        let mut mil_peak_power = 0.0f64;
        let mut mil_peak_alt = 0.0f64;
        let mut wep_peak_power = 0.0f64;
        let mut wep_peak_alt = 0.0f64;
        for alt in (0..=10000i32).step_by(50) {
            let mil_power = optimal_power_advanced(&stages, alt as f64, false, speed_kmh, is_ias, sea_level_temp);
            let wep_power = optimal_power_advanced(&stages, alt as f64, true, speed_kmh, is_ias, sea_level_temp);
            if mil_power > mil_peak_power {
                mil_peak_power = mil_power;
                mil_peak_alt = alt as f64;
            }
            if wep_power > wep_peak_power {
                wep_peak_power = wep_power;
                wep_peak_alt = alt as f64;
            }
        }

        println!("\n  === Peak Values ===");
        println!("  Military: {:.1} hp @ {:.0}m (wtapc: 1510.0 hp @ 4100m)", mil_peak_power, mil_peak_alt);
        println!("  WEP: {:.1} hp @ {:.0}m (wtapc: 2292.5 hp @ 1830m)", wep_peak_power, wep_peak_alt);

        // Acceptance criteria (relaxed for now - need debugging)
        t.assert_close("Military peak power", mil_peak_power, 1510.0, 50.0);
        t.assert_close("WEP peak power", wep_peak_power, 2292.5, 100.0);
        t.finish();
    }

    // ==================== Utility Methods ====================

    impl Tally {
        fn assert_close(&mut self, name: &str, actual: f64, expected: f64, tolerance: f64) {
            if (actual - expected).abs() <= tolerance {
                println!("  PASS: {name} = {actual:.2} (expected {expected:.2})");
                self.passed += 1;
            } else {
                println!(
                    "  FAIL: {name} = {actual:.2} (expected {expected:.2}, tolerance {tolerance:.2})"
                );
                self.failed += 1;
            }
        }

        fn assert_true(&mut self, name: &str, condition: bool) {
            if condition {
                println!("  PASS: {name}");
                self.passed += 1;
            } else {
                println!("  FAIL: {name}");
                self.failed += 1;
            }
        }

        fn assert_false(&mut self, name: &str, condition: bool) {
            self.assert_true(name, !condition);
        }

        fn assert_not_null<T>(&mut self, name: &str, obj: &Option<T>) {
            self.assert_true(&format!("{name} not null"), obj.is_some());
        }
    }

}

// ==================== tempest ← test/TestTempestMk5Power.java ====================

mod tempest {
    //! Verifies power curve calculations for Tempest Mk V (150 octane fuel aircraft).
    //!
    //! This test validates:
    //! 1. invertEnableLogic detection for British 150 octane fuel (inverted case)
    //! 2. Correct behavior when 150-octane is the DEFAULT fuel (no bonus applied)
    //! 3. Power curve accuracy vs wtapc reference values
    //!
    //! The Tempest Mk V FM file already contains 150-octane power values (invertEnableLogic=true),
    //! so VoidMei should NOT apply any fuel bonus.
    //!
    //! Run with: ./script/test.sh tempest

    use super::GETLOAD_WIRED;
    use super::fm_root;
    use super::{fuel_mod_from_json, parse_real};
    use crate::fmdata::FuelType;
    use crate::fm_power_extractor::extract_stages;
    use crate::piston_power_model::optimal_power_advanced;
    use std::path::Path;

    // wtapc reference values at 300 km/h IAS, 15C
    // Command: python wtapc.py --fm ... --central ... --ias 300
    const WTAPC_MIL: [[f64; 2]; 12] = [
        [0.0, 1982.4], [1000.0, 2031.5], [1730.0, 2064.7],  // Peak at ~1730m
        [2000.0, 2001.8], [3000.0, 1773.7], [4000.0, 1704.3],
        [5000.0, 1726.7], [6000.0, 1615.6], [7000.0, 1432.2],
        [8000.0, 1269.0], [9000.0, 1124.1], [10000.0, 994.2],
    ];

    const WTAPC_WEP: [[f64; 2]; 11] = [
        [0.0, 2439.9],  // Peak at sea level
        [1000.0, 2223.0], [2000.0, 2041.6], [3000.0, 2075.9],
        [4000.0, 2045.9], [5000.0, 1844.6], [6000.0, 1650.3],
        [7000.0, 1466.0], [8000.0, 1302.0], [9000.0, 1156.3],
        [10000.0, 1025.8],
    ];

    struct Tally {
        passed: usize,
        failed: usize,
    }

    impl Tally {
        fn new() -> Self {
            Tally { passed: 0, failed: 0 }
        }

        fn finish(&self) {
            println!("\n=== Results ===");
            println!("Passed: {}, Failed: {}", self.passed, self.failed);
            assert_eq!(self.failed, 0, "TestTempestMk5Power 存在失败断言");
        }
    }

    fn tempest_paths() -> (String, String) {
        let central = format!("{}/tempest_mkv.json", fm_root());
        let fm = format!("{}/fm/tempest_mkv.json", fm_root());
        (central, fm)
    }

    fn have_data(central: &str, fm: &str) -> bool {
        Path::new(central).is_file() && Path::new(fm).is_file()
    }

    // ==================== Test Cases ====================

    /// Tests that invertEnableLogic is correctly parsed as a boolean value,
    /// not just detected by keyword presence.
    #[test]
    fn test_invert_enable_logic_detection() {
        let (central_path, _fm_path) = tempest_paths();
        if !have_data(&central_path, &_fm_path) {
            return; // data/ 未解包, 对齐 build.py 跳过语义 (realtests 模块头注)
        }
        let mut t = Tally::new();
        println!("Testing invertEnableLogic detection...");

        let fuel_mod = fuel_mod_from_json(&central_path);

        // Verify fuel type detected
        t.assert_true(
            "detected British 150 octane fuel",
            fuel_mod.r#type == FuelType::British150Octane,
        );

        // Tempest Mk V has invertEnableLogic:b = true
        // This means 150-octane is the DEFAULT fuel state
        t.assert_true(
            "invertEnableLogic is true (150 octane is default)",
            fuel_mod.british_invert_logic,
        );

        println!("  Fuel type: {}", fuel_mod.r#type);
        println!("  invertEnableLogic: {}", fuel_mod.british_invert_logic);
        t.finish();
    }

    /// Tests that fuel modification is NOT applied when invertEnableLogic=true.
    /// Since 150-octane is default, the modification represents REMOVING it.
    #[test]
    fn test_fuel_modification_behavior() {
        let (central_path, fm_path) = tempest_paths();
        if !have_data(&central_path, &fm_path) {
            return; // data/ 未解包 (build.py 跳过语义)
        }
        let mut t = Tally::new();
        println!("\nTesting fuel modification behavior (invertEnableLogic=true)...");

        let fmdata = match parse_real(&fm_path) {
            Ok(b) => b,
            Err(_) => {
                println!("  SKIP: Cannot load files");
                return;
            }
        };
        let fuel_mod = fuel_mod_from_json(&central_path);

        // PORT: 以下依赖 extract_stages (getload 字段前置), getload 未译前挂起
        // (realtests 模块头注 GETLOAD_WIRED)
        if !GETLOAD_WIRED {
            eprintln!("  SKIP: getload 未译 (reader 波次 TODO(port)), 断言暂挂");
            return;
        }

        // Extract stages with and without fuel modification
        let stages_no_fuel = extract_stages(Some(&fmdata));
        let stages_with_fuel = crate::fm_power_extractor::extract_stages_with_fuel(
            Some(&fmdata),
            Some(&fuel_mod),
        );

        if let (Some(no_fuel), Some(with_fuel)) = (stages_no_fuel.as_ref(), stages_with_fuel.as_ref()) {
            // With invertEnableLogic=true, fuel mod should NOT change WEP params
            let mut wep_unchanged = true;
            for i in 0..no_fuel.len() {
                let no_fuel_mult = no_fuel[i].wep_power_mult;
                let with_fuel_mult = with_fuel[i].wep_power_mult;
                let no_fuel_wep_alt = no_fuel[i].wep_crit_alt;
                let with_fuel_wep_alt = with_fuel[i].wep_crit_alt;

                println!(
                    "  Stage {i}: wepMult {:.4} → {:.4}, wepCritAlt {:.0} → {:.0}",
                    no_fuel_mult, with_fuel_mult, no_fuel_wep_alt, with_fuel_wep_alt
                );

                if (no_fuel_mult - with_fuel_mult).abs() > 0.001
                    || (no_fuel_wep_alt - with_fuel_wep_alt).abs() > 1.0
                {
                    wep_unchanged = false;
                }
            }

            t.assert_true(
                "WEP parameters unchanged (invertEnableLogic=true means no bonus)",
                wep_unchanged,
            );
        }
        t.finish();
    }

    #[test]
    fn test_parameter_extraction() {
        let (central_path, fm_path) = tempest_paths();
        if !have_data(&central_path, &fm_path) {
            return; // data/ 未解包 (build.py 跳过语义)
        }
        let mut t = Tally::new();
        println!("\nTesting parameter extraction...");

        // (name 由文件名分量承接, 未落地前无行为差异, 见 spitfire 同款注)
        let fmdata = match parse_real(&fm_path) {
            Ok(b) => b,
            Err(_) => {
                println!("  SKIP: Cannot parse FM file");
                return;
            }
        };

        // PORT: 以下断言依赖 getload 填充字段 / extract_stages, 未译前挂起
        if !GETLOAD_WIRED {
            eprintln!("  SKIP: getload 未译 (reader 波次 TODO(port)), 参数提取断言暂挂");
            return;
        }

        // Since invertEnableLogic=true, we can extract with or without fuel mod - same result
        let stages = extract_stages(Some(&fmdata));
        t.assert_not_null("extracted stages", &stages);
        t.assert_true(
            "has 2 stages",
            stages.is_some() && stages.as_ref().unwrap().len() == 2,
        );

        if let Some(stages) = stages.as_ref() {
            println!("\n  === Extracted Stage Parameters ===");
            for (i, s) in stages.iter().enumerate() {
                println!("  Stage {i}:");
                println!("    critAlt: {:.0}m, critPower: {:.1}hp", s.crit_alt, s.crit_power);
                println!("    deckPower: {:.1}hp", s.deck_power);
                println!("    wepCritAlt: {:.0}m, wepPowerMult: {:.4}", s.wep_crit_alt, s.wep_power_mult);
            }
        }

        // Verify expected FM parameters (specific to Tempest Mk V)
        t.assert_close("compressor NumSteps", fmdata.comp_num_steps as f64, 2.0, 0.0);
        // Stage 0 critical altitude: 期望值须跟随游戏 FM 数据版本更新
        // (WT 2.57.1.103 中 tempest_mkv 的 Altitude0 已从 1730 调整为 1447;
        //  fmdata 更新后若此处 FAIL, 先 grep blkx 原始值区分数据变更与程序回归)
        t.assert_close("Stage 0 altitude", fmdata.comp_alt.as_ref().unwrap()[0], 1447.0, 50.0);
        // Stage 1 critical altitude should be around 5000m
        t.assert_close("Stage 1 altitude", fmdata.comp_alt.as_ref().unwrap()[1], 5000.0, 200.0);
        t.finish();
    }

    #[test]
    fn test_power_curve_military() {
        let (central_path, fm_path) = tempest_paths();
        if !have_data(&central_path, &fm_path) {
            return; // data/ 未解包 (build.py 跳过语义)
        }
        let mut t = Tally::new();
        println!("\nTesting military power curve vs wtapc...");

        let fmdata = match parse_real(&fm_path) {
            Ok(b) => b,
            Err(_) => {
                println!("  SKIP: Cannot parse FM file");
                return;
            }
        };

        // PORT: 功率曲线断言依赖 extract_stages (getload 字段前置), 未译前挂起
        if !GETLOAD_WIRED {
            eprintln!("  SKIP: getload 未译 (reader 波次 TODO(port)), 功率曲线断言暂挂");
            return;
        }

        // For Tempest Mk V, since invertEnableLogic=true, fuel mod doesn't change anything
        // Use stages directly without fuel modification (or with - same result)
        let stages = match extract_stages(Some(&fmdata)) {
            Some(s) => s,
            None => {
                println!("  SKIP: Cannot extract stages");
                return;
            }
        };

        let speed_kmh = 300.0;
        let is_ias = true;
        let sea_level_temp = 15.0;

        println!("\n  === Military Power Curve (300 km/h IAS) ===");
        println!("  Alt(m)    VoidMei    wtapc    Diff");
        println!("  ------    -------    -----    ----");

        let mut errors = 0;
        let mut max_error = 0.0f64;
        for r#ref in &WTAPC_MIL {
            let alt = r#ref[0];
            let expected = r#ref[1];
            let actual = optimal_power_advanced(&stages, alt, false, speed_kmh, is_ias, sea_level_temp);
            let diff = actual - expected;
            let abs_diff = diff.abs();

            let status = if abs_diff < 5.0 { "OK" } else if abs_diff < 20.0 { "~" } else { "X" };
            println!("  {alt:5.0}    {actual:7.1}    {expected:5.1}    {diff:+.1} {status}");

            if abs_diff > 5.0 {
                errors += 1;
            }
            if abs_diff > max_error {
                max_error = abs_diff;
            }
        }
        let _ = errors;

        println!("\n  Max error: {:.1} hp", max_error);
        t.assert_true("Military max error < 5 hp", max_error < 5.0);
        t.finish();
    }

    #[test]
    fn test_power_curve_wep() {
        let (central_path, fm_path) = tempest_paths();
        if !have_data(&central_path, &fm_path) {
            return; // data/ 未解包 (build.py 跳过语义)
        }
        let mut t = Tally::new();
        println!("\nTesting WEP power curve vs wtapc...");

        let fmdata = match parse_real(&fm_path) {
            Ok(b) => b,
            Err(_) => {
                println!("  SKIP: Cannot parse FM file");
                return;
            }
        };

        // PORT: 同上, getload 未译前挂起
        if !GETLOAD_WIRED {
            eprintln!("  SKIP: getload 未译 (reader 波次 TODO(port)), 功率曲线断言暂挂");
            return;
        }

        let stages = match extract_stages(Some(&fmdata)) {
            Some(s) => s,
            None => {
                println!("  SKIP: Cannot extract stages");
                return;
            }
        };

        let speed_kmh = 300.0;
        let is_ias = true;
        let sea_level_temp = 15.0;

        println!("\n  === WEP Power Curve (300 km/h IAS) ===");
        println!("  Alt(m)    VoidMei    wtapc    Diff");
        println!("  ------    -------    -----    ----");

        let mut errors = 0;
        let mut max_error = 0.0f64;
        for r#ref in &WTAPC_WEP {
            let alt = r#ref[0];
            let expected = r#ref[1];
            let actual = optimal_power_advanced(&stages, alt, true, speed_kmh, is_ias, sea_level_temp);
            let diff = actual - expected;
            let abs_diff = diff.abs();

            let status = if abs_diff < 10.0 { "OK" } else if abs_diff < 50.0 { "~" } else { "X" };
            println!("  {alt:5.0}    {actual:7.1}    {expected:5.1}    {diff:+.1} {status}");

            if abs_diff > 10.0 {
                errors += 1;
            }
            if abs_diff > max_error {
                max_error = abs_diff;
            }
        }
        let _ = errors;

        // Find peak values
        let mut wep_peak_power = 0.0f64;
        let mut wep_peak_alt = 0.0f64;
        for alt in (0..=10000i32).step_by(50) {
            let wep_power = optimal_power_advanced(&stages, alt as f64, true, speed_kmh, is_ias, sea_level_temp);
            if wep_power > wep_peak_power {
                wep_peak_power = wep_power;
                wep_peak_alt = alt as f64;
            }
        }

        println!("\n  Max error: {:.1} hp", max_error);
        println!("  VoidMei WEP peak: {:.1} hp @ {:.0}m (wtapc: 2439.9 hp @ 0m)", wep_peak_power, wep_peak_alt);

        // Acceptance criteria
        t.assert_true("WEP max error < 10 hp", max_error < 10.0);
        t.assert_close("WEP peak power", wep_peak_power, 2439.9, 10.0);
        t.assert_close("WEP peak altitude", wep_peak_alt, 0.0, 100.0);
        t.finish();
    }

    // ==================== Utility Methods ====================

    impl Tally {
        fn assert_close(&mut self, name: &str, actual: f64, expected: f64, tolerance: f64) {
            if (actual - expected).abs() <= tolerance {
                println!("  PASS: {name} = {actual:.2} (expected {expected:.2} +/- {tolerance:.2})");
                self.passed += 1;
            } else {
                println!(
                    "  FAIL: {name} = {actual:.2} (expected {expected:.2} +/- {tolerance:.2})"
                );
                self.failed += 1;
            }
        }

        fn assert_true(&mut self, name: &str, condition: bool) {
            if condition {
                println!("  PASS: {name}");
                self.passed += 1;
            } else {
                println!("  FAIL: {name}");
                self.failed += 1;
            }
        }

        fn assert_not_null<T>(&mut self, name: &str, obj: &Option<T>) {
            self.assert_true(&format!("{name} not null"), obj.is_some());
        }
    }

}

// ==================== fuzzer ← test/FMParserFuzzer.java ====================

mod fuzzer {
    //! blkx 文本变异 Fuzz 测试 (P6) —— FM 物理文件解析管线的防御性验收
    //!
    //! 种子取项目内 data/ 的真机 JSON (默认 fm/bf-109e-4.json —— 中等体积且带
    //! PASSPORT.ALT/IAS 曲线数组, 能覆盖 getAllplotdata 的 parseDouble/split 路径;
    //! 注意 spitfire_f24.blkx 无 PASSPORT 块, 用它当种子会让腿1的管线阶段空转),
    //! 对其施加字节级/行级/结构级/语义级四类变异, 每个变异体走完整生产管线:
    //!
    //!   腿1 (每个变异体): new Blkx(临时文件, name) —— 构造器含 getload() (P1 已加
    //!       固: 构造器不得抛异常, 失败置 valid=false) → valid 时接 getAllplotdata() +
    //!       finalizeLoading() (与 FMLoader.load 的 5/6 两步完全一致)
    //!
    //!   腿2 (抽样 30 个变异体): FMLoader.load(planeName) —— 写临时 data 目录结构
    //!       (FMDataPaths.setDataRoot 注入), 断言返回句柄契约:
    //!       status ∈ {READY, MISSING, CORRUPT} 且 READY ⇒ blkx != null、
    //!       isMissingLike ⇒ blkx == null (P2 回归的直接扩展)
    //!
    //! 验收标准 (每个变异体):
    //!   ① 任何 Throwable 逃逸即失败 (OutOfMemoryError 单独标记并提示)
    //!   ② 单变异体限时 5s (变异集合有限 + 固定种子顺序执行, 超时视为疑似死循环)
    //!   ③ valid 布尔与对象状态自洽 (valid=false 时不访问解析字段)
    //!
    //! 固定种子 (默认 20260825) 保证可复现; --seed/--iterations 可覆盖。
    //!
    //! 运行方式: python script/build.py test fuzz-blkx
    //!   (build.py 会传 --central <data/.../bf-109e-4.json> --fm <data/.../fm/bf-109e-4.json>;
    //!    data/ 缺失时 build.py 的 run_fm_test 机制自动跳过整套)
    //!
    //! PORT: 本移植固定 --central/--fm 为仓库相对路径 (realtests 模块头注);
    //! getload/getAllplotdata 批次均已落地 — 腿1 全管线 (构造器 + getAllplotdata
    //! + finalizeLoading) 恢复; 腿2 (FMLoader.load 抽样) 仍挂起: fm_loader.rs 与
    //! fm_data_paths (含 set_data_root) 虽已落地, 临时数据根注入的测试接线属
    //! 后续批次, 见腿2 处 TODO(port) 标注。
    //! JavaRandom/mutate 与 Java 端逐位一致 (oracle 对拍, 见下方测试)。

    use super::fm_root;
    use crate::fmdata::FmData;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    /// 默认迭代数 (变异体个数) — Java 原值即 200, 无降档
    const DEFAULT_ITERATIONS: usize = 200;
    /// 默认随机种子 —— 固定值保证变异序列可复现
    const DEFAULT_SEED: u64 = 20260825;
    /// 腿2 抽样走 FMLoader 的变异体个数
    #[allow(dead_code)] // 腿2 挂起 (TODO(port) 接线批次), 落地后消费
    const LOADER_SAMPLES: usize = 30;
    /// 单变异体耗时上限 (ms), 超过判失败 (疑似死循环)
    const PER_CASE_LIMIT_MS: u128 = 5000;

    /// 字节级字符替换池: ASCII 全谱, 含换行/引号/花括号/等号等结构性字符
    const ASCII_POOL: &str = "abcXYZ019 \t\n\r\"'{}[]<>=:,;.+-*/\\#$%&()!?|~^`_";

    /// 语义级: 数值字面量的替换池 (NaN / 上下溢 / 500 位长数字 / 负零)
    fn num_replacements() -> [String; 5] {
        [
            "NaN".to_string(),
            "1e999".to_string(),
            "-1e999".to_string(),
            "9".repeat(500),
            "-0".to_string(),
        ]
    }

    // 变异策略名 (下标即策略编号, 输出统计用)
    const STRATEGY_NAMES: [&str; 13] = [
        "truncate", "charReplace", "chunkPaste", "deleteLine", "shuffleLines",
        "commentLine", "stripIndent", "dropBrace", "killEquals", "injectNest",
        "numberMutate", "unquote", "jsonInject",
    ];

    /// 逃逸异常计数等 (Java static 计数器, 单测试线程内持有)
    #[derive(Default)]
    struct Counters {
        passed: usize,
        failed: usize,
        fuzz_cases: usize,
        /// 逃逸异常计数 (构造器阶段 / 管线阶段分开记, 便于定位)
        ctor_exceptions: usize,
        pipeline_exceptions: usize,
        valid_true: usize,
        valid_false: usize,
    }

    // ==================== java.util.Random 逐位移植 ====================

    /// java.util.Random (OpenJDK 8) 的逐位移植 — 48-bit LCG。
    /// FMParserFuzzer 的 "固定种子保证可复现" 依赖与 Java 完全一致的随机序列
    /// (oracle: build/oracle/rand/RandOracle.java 在 OpenJDK 1.8.0_342 实测
    /// dump, 下方 java8_oracle_java_random 对拍钉死)
    struct JavaRandom {
        /// 已 scramble 的 48-bit 内部状态 (Java private seed 字段)
        seed: u64,
    }

    impl JavaRandom {
        /// Java `new Random(long seed)` → setSeed:
        /// `seed = (s ^ 0x5DEECE66DL) & ((1L << 48) - 1);`
        fn new(seed: u64) -> Self {
            JavaRandom {
                seed: (seed ^ 0x5DEECE66D) & ((1 << 48) - 1),
            }
        }

        /// Java `protected int next(int bits)`:
        /// `seed = (seed * 0x5DEECE66DL + 0xBL) & ((1L << 48) - 1);`
        /// `return (int)(seed >>> (48 - bits));`
        fn next(&mut self, bits: u32) -> i32 {
            // PORT: Java long 乘加静默回绕 ↔ wrapping_mul/wrapping_add (§2.2)
            self.seed = self
                .seed
                .wrapping_mul(0x5DEECE66D)
                .wrapping_add(0xB)
                & ((1 << 48) - 1);
            (self.seed >> (48 - bits)) as i32
        }

        /// Java `public int nextInt()` = next(32)
        fn next_int(&mut self) -> i32 {
            self.next(32)
        }

        /// Java `public int nextInt(int n)` (n > 0):
        /// 2 的幂走 `(n * (long)next(31)) >> 31` 快路径, 否则模拒采样;
        /// 拒绝条件 `bits - val + (n-1) < 0` 是 Java int 溢出回绕判定 (§2.2)
        fn next_int_bound(&mut self, n: usize) -> usize {
            debug_assert!(n > 0);
            let n = n as i32;
            if (n & -n) == n {
                // i.e., n is a power of 2
                return ((n as i64 * (self.next(31) as i64)) >> 31) as usize;
            }
            loop {
                let bits = self.next(31);
                let val = bits % n;
                // PORT: 拒绝判定显式加括号 — Rust 一元 `!` 优先级高于 `<`, 无括号会
                // 解析为 `(~x) < 0`; 对 i32 恰与 `!(x < 0)` 补码等价 (~x<0 ⟺ x>=0),
                // RIB oracle 已钉死行为, 括号只为杜绝后续改类型/"修正"写法时引入真 bug
                if !(bits.wrapping_sub(val).wrapping_add(n - 1) < 0) {
                    return val as usize;
                }
            }
        }

        /// Java `public long nextLong()`: `((long)next(32) << 32) + next(32)`
        fn next_long(&mut self) -> i64 {
            // PORT: Java long 移位/加法静默回绕 ↔ wrapping (§2.2)
            ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
        }

        /// Java `public boolean nextBoolean()`: `next(1) != 0`
        fn next_boolean(&mut self) -> bool {
            self.next(1) != 0
        }

        /// Java `public double nextDouble()`:
        /// `(((long)next(26) << 27) + next(27)) * 0x1.0p-53`
        fn next_double(&mut self) -> f64 {
            let hi = self.next(26) as u64; // 0..2^26-1, 非负
            let lo = self.next(27) as u64; // 0..2^27-1, 非负
            // PORT: Rust 无十六进制浮点字面量, 2^-53 以除法表达 (精确幂次, 逐位一致)
            (((hi << 27) + lo) as f64) / ((1u64 << 53) as f64)
        }
    }

    // ==================== 变异原语 (四类 13 种) ====================

    fn mutate(s: &str, kind: i32, rnd: &mut JavaRandom) -> String {
        match kind {
            0 => truncate(s, rnd),        // 字节级: 头/中/尾截断
            1 => char_replace(s, rnd),    // 字节级: 随机字符替换 (ASCII 全谱)
            2 => chunk_paste(s, rnd),     // 字节级: 段落复制粘贴
            3 => delete_lines(s, rnd),    // 行级: 随机删行
            4 => shuffle_lines(s, rnd),   // 行级: 行乱序
            5 => comment_lines(s, rnd),   // 行级: 前插 // 注释化
            6 => strip_indent(s, rnd),    // 行级: 缩进清空
            7 => drop_brace(s, rnd),      // 结构级: 删一个 { 或 } (括号失配)
            8 => kill_equals(s, rnd),     // 结构级: 某个 = 换成空格
            9 => inject_nest(s, rnd),     // 结构级: 注入额外嵌套 "{\n" 块
            10 => number_mutate(s, rnd),  // 语义级: 数值字面量换 NaN/1e999/长数字等
            11 => unquote(s, rnd),        // 语义级: 去掉某个字符串的引号
            12 => json_inject(s, rnd),    // 语义级: 注入 JSON 片段替换随机区间
            _ => char_replace(s, rnd),
        }
    }

    /// PORT(§2.1): Java String 按 UTF-16 码元索引/length; 种子域纯 ASCII (od 实测)
    /// 下字节索引与码元索引一致。随机偏移统一吸附到 char 边界, 病态非 ASCII 输入
    /// 下防 UTF-8 切片 panic (输出与 Java UTF-16 语义允许微偏, 域内无差异)
    fn floor_char_boundary(s: &str, i: usize) -> usize {
        let mut i = i.min(s.len());
        while i > 0 && !s.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    /// 同上, 向上吸附
    fn ceil_char_boundary(s: &str, i: usize) -> usize {
        let mut i = i.min(s.len());
        while i < s.len() && !s.is_char_boundary(i) {
            i += 1;
        }
        i
    }

    /// 字节级-截断: 头部/中部/尾部随机去掉一段 (最多 55%)
    fn truncate(s: &str, rnd: &mut JavaRandom) -> String {
        let len = s.len();
        if len < 32 {
            return s.to_string();
        }
        let cut = 1.max((len as f64 * (0.02 + rnd.next_double() * 0.53)) as i32) as usize;
        let mode = rnd.next_int_bound(3);
        if mode == 0 {
            return s[ceil_char_boundary(s, cut)..].to_string(); // 头部截断
        }
        if mode == 1 {
            return s[..floor_char_boundary(s, len - cut)].to_string(); // 尾部截断
        }
        let at = rnd.next_int_bound(len - cut); // 中部截断
        let at = floor_char_boundary(s, at);
        let end = ceil_char_boundary(s, at + cut);
        format!("{}{}", &s[..at], &s[end..])
    }

    /// 字节级-字符替换: 1~8 个随机位置换成 ASCII 池字符 (含换行/引号/花括号/等号)
    fn char_replace(s: &str, rnd: &mut JavaRandom) -> String {
        // Java StringBuilder.setCharAt 按 UTF-16 码元替换 ↔ char 级替换
        // (BMP 域与码元一一对应; 池字符全 ASCII)
        let mut chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        if len < 16 {
            return s.to_string();
        }
        let pool: Vec<char> = ASCII_POOL.chars().collect();
        let n = 1 + rnd.next_int_bound(8);
        for _ in 0..n {
            let at = rnd.next_int_bound(len);
            chars[at] = pool[rnd.next_int_bound(pool.len())];
        }
        chars.into_iter().collect()
    }

    /// 字节级-段落复制粘贴: 取一段 (≤10%) 复制插入到随机位置
    fn chunk_paste(s: &str, rnd: &mut JavaRandom) -> String {
        let len = s.len();
        if len < 64 {
            return s.to_string();
        }
        let clen = 1 + rnd.next_int_bound(2000.min(2.max(len / 10)));
        let from = floor_char_boundary(s, rnd.next_int_bound(len - clen));
        let chunk = &s[from..ceil_char_boundary(s, from + clen)];
        let at = floor_char_boundary(s, rnd.next_int_bound(len));
        format!("{}{}{}", &s[..at], chunk, &s[at..])
    }

    /// 行级-删行: 随机删 1~3 行
    fn delete_lines(s: &str, rnd: &mut JavaRandom) -> String {
        let lines: Vec<&str> = s.split('\n').collect();
        if lines.len() < 4 {
            return s.to_string();
        }
        let n = 1 + rnd.next_int_bound(3);
        let mut sb = String::new();
        let first = rnd.next_int_bound(lines.len());
        for (i, line) in lines.iter().enumerate() {
            if i >= first && i < first + n {
                continue; // 跳过被删的连续行
            }
            sb.push_str(line);
            sb.push('\n');
        }
        sb
    }

    /// 行级-乱序: 随机交换 2~4 对行
    fn shuffle_lines(s: &str, rnd: &mut JavaRandom) -> String {
        let mut lines: Vec<&str> = s.split('\n').collect();
        if lines.len() < 4 {
            return s.to_string();
        }
        let n = 2 + rnd.next_int_bound(3);
        for _ in 0..n {
            let i = rnd.next_int_bound(lines.len());
            let j = rnd.next_int_bound(lines.len());
            lines.swap(i, j);
        }
        join(&lines)
    }

    /// 行级-注释化: 随机 1~3 个非空行前插 //
    fn comment_lines(s: &str, rnd: &mut JavaRandom) -> String {
        let mut lines: Vec<String> = s.split('\n').map(|l| l.to_string()).collect();
        if lines.len() < 4 {
            return s.to_string();
        }
        let n = 1 + rnd.next_int_bound(3);
        for _ in 0..n {
            let i = rnd.next_int_bound(lines.len());
            // 行内无 \n 且域内 ASCII, Rust trim() 同域 (§2.1)
            if !lines[i].trim().is_empty() {
                lines[i] = format!("//{}", lines[i]);
            }
        }
        join(&lines)
    }

    /// 行级-缩进清空: 随机窗口内 ≤30 行去掉行首空白 (破坏花括号缩进结构)
    fn strip_indent(s: &str, rnd: &mut JavaRandom) -> String {
        let mut lines: Vec<String> = s.split('\n').map(|l| l.to_string()).collect();
        if lines.len() < 4 {
            return s.to_string();
        }
        let w = rnd.next_int_bound(lines.len());
        let end = lines.len().min(w + 30);
        for line in lines.iter_mut().take(end).skip(w) {
            // PORT: crate 无 regex 依赖, 手写等价扫描 (空格/制表符是 ASCII 单字节,
            // 前缀必在 char 边界收尾)
            let n = line
                .as_bytes()
                .iter()
                .take_while(|&&b| b == b' ' || b == b'\t')
                .count();
            *line = line[n..].to_string();
        }
        join(&lines)
    }

    /// 结构级-括号失配: 随机删除一个 '{' 或 '}'
    fn drop_brace(s: &str, rnd: &mut JavaRandom) -> String {
        let brace_pos: Vec<usize> = s
            .as_bytes()
            .iter()
            .enumerate()
            .filter(|(_, &b)| b == b'{' || b == b'}')
            .map(|(i, _)| i)
            .collect();
        if brace_pos.is_empty() {
            return s.to_string();
        }
        let at = brace_pos[rnd.next_int_bound(brace_pos.len())];
        // '{'/'}' 是单字节 ASCII 字符, at+1 恒为合法 char 边界
        format!("{}{}", &s[..at], &s[at + 1..])
    }

    /// 结构级-赋值破坏: 随机一个 '=' 换成空格
    fn kill_equals(s: &str, rnd: &mut JavaRandom) -> String {
        let eq_pos: Vec<usize> = s
            .as_bytes()
            .iter()
            .enumerate()
            .filter(|(_, &b)| b == b'=')
            .map(|(i, _)| i)
            .collect();
        if eq_pos.is_empty() {
            return s.to_string();
        }
        let at = eq_pos[rnd.next_int_bound(eq_pos.len())];
        format!("{} {}", &s[..at], &s[at + 1..])
    }

    /// 结构级-嵌套注入: 在随机位置插入 1~3 个 "{\n" (刻意不配对, 制造额外嵌套)
    fn inject_nest(s: &str, rnd: &mut JavaRandom) -> String {
        let mut sb = String::from(s);
        let n = 1 + rnd.next_int_bound(3);
        for _ in 0..n {
            let at = ceil_char_boundary(&sb, rnd.next_int_bound(sb.len() + 1));
            sb.insert_str(at, "{\n");
        }
        sb
    }

    /// 语义级-数值变异: 随机一个数值字面量换成 NaN/1e999/-1e999/500 位长数字/负零
    fn number_mutate(s: &str, rnd: &mut JavaRandom) -> String {
        let matches = find_num_matches(s);
        if matches.is_empty() {
            return s.to_string();
        }
        let pick = matches[rnd.next_int_bound(matches.len())];
        let repl = &num_replacements()[rnd.next_int_bound(5)];
        format!("{}{}{}", &s[..pick.0], repl, &s[pick.1..])
    }

    /// 语义级-去引号: 随机一个带引号字符串去掉两侧引号
    fn unquote(s: &str, rnd: &mut JavaRandom) -> String {
        let matches = find_quoted_matches(s);
        if matches.is_empty() {
            return s.to_string();
        }
        let pick = matches[rnd.next_int_bound(matches.len())];
        format!("{}{}{}", &s[..pick.0], &s[pick.0 + 1..pick.1 - 1], &s[pick.1..])
    }

    /// 语义级-JSON 注入: 随机区间 (≤5%) 整段替换为 JSON 片段
    fn json_inject(s: &str, rnd: &mut JavaRandom) -> String {
        let len = s.len();
        if len < 40 {
            return s.to_string();
        }
        let span = 1 + rnd.next_int_bound(2.max(len / 20));
        let from = rnd.next_int_bound(len - span);
        let json = if rnd.next_boolean() {
            "{\"a\":1}"
        } else {
            "{\"x\":[1,2,3],\"y\":null}"
        };
        let from = floor_char_boundary(s, from);
        let to = ceil_char_boundary(s, from + span);
        format!("{}{}{}", &s[..from], json, &s[to..])
    }

    // ==================== 工具 ====================

    fn join<S: AsRef<str>>(lines: &[S]) -> String {
        // (每行都补 \n, 含末尾原空行 — 与原串相比可能多一个尾部换行, 保真)
        let mut sb = String::new();
        for l in lines {
            sb.push_str(l.as_ref());
            sb.push('\n');
        }
        sb
    }

    /// Java `RE_NUM = Pattern.compile("\\d+\\.?\\d*(?:[eE][-+]?\\d+)?")` 的
    /// find() 全序列手写移植 (vm-core 无 regex 依赖): 最左非重叠匹配, ASCII \d,
    /// 逐原子贪婪 + 可选指数组整体匹配/整体放弃 — 与 java.util.regex 回溯语义
    /// 一致 (见 num_and_quoted_scanner_boundaries 边界测试); 收集上限 5000
    fn find_num_matches(s: &str) -> Vec<(usize, usize)> {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i].is_ascii_digit() {
                let start = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1; // \d+
                }
                if i < b.len() && b[i] == b'.' {
                    i += 1; // \.? (贪婪)
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1; // \d*
                    }
                }
                // (?:[eE][-+]?\d+)? — 完整匹配才整体消费, 否则回退到 i (可选组匹配空)
                let save = i;
                if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                    let mut j = i + 1;
                    if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                        j += 1;
                    }
                    if j < b.len() && b[j].is_ascii_digit() {
                        while j < b.len() && b[j].is_ascii_digit() {
                            j += 1;
                        }
                        i = j;
                    } else {
                        i = save;
                    }
                }
                out.push((start, i));
                if out.len() >= 5000 {
                    break;
                }
            } else {
                i += 1;
            }
        }
        out
    }

    /// Java `RE_QUOTED = Pattern.compile("\"([^\"\\n\\r]{1,60})\"")` 的 find() 全序列
    /// 手写移植: 引号内 ≤60 个非引号/非换行字符; {1,60} 贪婪 — 取最长合法 run 后
    /// 必须紧跟闭合引号 (回溯缩短不可能命中, 见边界测试); 收集上限 5000
    fn find_quoted_matches(s: &str) -> Vec<(usize, usize)> {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'"' {
                let start = i;
                let mut j = i + 1;
                while j < b.len()
                    && (j - start - 1) < 60
                    && b[j] != b'"'
                    && b[j] != b'\n'
                    && b[j] != b'\r'
                {
                    j += 1;
                }
                if j < b.len() && b[j] == b'"' {
                    out.push((start, j + 1));
                    i = j + 1;
                    if out.len() >= 5000 {
                        break;
                    }
                    continue;
                }
            }
            i += 1;
        }
        out
    }

    /// FNV-1a 64 位摘要 — oracle 对拍用 (build/oracle/rand/RandOracle.java 同款
    /// 实现; vm-core 无 md5 依赖, 双语言各 10 行即可逐字节对拍)
    fn fnv1a64(s: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in s.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Java `Files.createTempFile(prefix, suffix)` 等价: 唯一临时文件路径
    fn temp_file(prefix: &str, suffix: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("{prefix}{}_{}{suffix}", std::process::id(), n))
    }

    // ==================== 基线 ====================

    /// 原始种子必须 valid (真机数据本应可解析; 失败说明种子选择或环境有误)
    /// (blkx→json 迁移: parse_str_json 内容注入, getload 已内含)
    fn baseline_check(seed_text: &str, _seed_name: &str, c: &mut Counters) -> bool {
        let ok = (|| {
            let b = match FmData::parse_str_json("fuzz_baseline.json", seed_text) {
                Ok(b) => b,
                Err(_) => {
                    println!("  [失败] 基线: 原始种子解析后 valid=false (不应发生)");
                    c.failed += 1;
                    return false;
                }
            };
            // 同 run_direct_pipeline: Ok ⇒ valid==true 契约钉死
            assert!(b.valid, "基线 parse 返回 Ok 但 valid=false (违反 json.rs 契约)");
            println!("  [通过] 基线: 原始种子全管线解析成功");
            c.passed += 1;
            true
        })();
        ok
    }

    // ==================== 腿1: Blkx 直连全管线 ====================

    /// 单变异体执行: parse_str_json 内容注入 (getload 内含且自带 catch_unwind
    /// 收敛 Err)。断言① 逃逸 panic 即失败; ② 单体 5s 限时; ③ valid=false
    /// (收敛 Err) 不触碰解析字段。
    fn run_direct_pipeline(
        mutant: &str,
        kind: i32,
        index: usize,
        _tmp_fmdata: &str,
        _seed_name: &str,
        c: &mut Counters,
    ) {
        c.fuzz_cases += 1;

        let t0 = Instant::now();
        // 外层 catch_unwind 承接断言① — 收敛机制之外仍逃逸的 panic 即失败
        // (原文本版两相位中的 plotdata/finalize_loading 已随曲线链/data 串退役)
        let parsed = std::panic::catch_unwind(|| FmData::parse_str_json("fuzz.json", mutant));
        match parsed {
            Err(_) => {
                c.ctor_exceptions += 1;
                println!(
                    "  [失败] #{index} ({}) 逃逸异常[构造器]: panic",
                    STRATEGY_NAMES[kind as usize]
                );
                dump_mutant(mutant, index);
                c.failed += 1;
                return;
            }
            Ok(Err(_)) => {
                // 断言③: 守卫/解析失败收敛 Err (= 文本版 valid=false) 时刻意
                // 不访问任何解析字段 (对象应安全废弃)
                c.valid_false += 1;
            }
            Ok(Ok(b)) => {
                // 契约钉死 (json.rs 不变式 "Ok 恒 valid==true")
                assert!(b.valid, "#{} parse 返回 Ok 但 valid=false (违反 json.rs 契约)", index);
                c.valid_true += 1;
            }
        }
        let ms = t0.elapsed().as_millis();
        // 断言②: 单体限时 (变异集合有限 + 顺序执行可复现, 超时即疑似死循环)
        if ms > PER_CASE_LIMIT_MS {
            println!(
                "  [失败] #{index} ({}) 单文件耗时 {ms} ms 超过 {PER_CASE_LIMIT_MS} ms 上限",
                STRATEGY_NAMES[kind as usize]
            );
            dump_mutant(mutant, index);
            c.failed += 1;
        }
    }

    /// 失败现场留存: 把出问题的变异体写到 build/ 下供人工复现 (build/ 已 gitignore)。
    /// PORT: Java `new File("build")` 同为 cwd 相对 — Java 经 build.py 跑时 cwd=仓库根
    /// (build/ 在场, 留存生效), cargo test 时 cwd=crate 根 (rust/crates/vm-core/build/
    /// 通常不存在 → 静默跳过); 仅影响失败现场留存位置, 不影响断言
    fn dump_mutant(mutant: &str, index: usize) {
        let build_dir = Path::new("build");
        if !build_dir.is_dir() {
            return; // 无 build 目录 (如 CI 精简环境) 时静默跳过, 不影响测试结果
        }
        let out = build_dir.join(format!("fuzz_fail_{index}.json"));
        let _ = std::fs::write(&out, mutant);
    }

    // ==================== main ====================

    #[test]
    fn fuzz_fmdata_text_mutations() {
        // Java main 由 build.py 传 --central/--fm; PORT: 固定仓库相对路径
        let central_path = format!("{}/bf-109e-4.json", fm_root());
        let fm_path = format!("{}/fm/bf-109e-4.json", fm_root());
        if !Path::new(&fm_path).is_file() {
            return; // data/ 未解包, 对齐 build.py run_fm_test 跳过语义 (模块头注)
        }
        let iterations = DEFAULT_ITERATIONS;
        let seed = DEFAULT_SEED;

        println!("=== fmdata 文本变异 Fuzz 测试 ===\n");

        // 静态表 (parse 内部自取), crate 暂无 Logger, 无需降噪

        // 读文件用平台默认字符集 —— 与 Blx 构造器内 FileReader 一致,
        // 保证"读出->变异->写回->Blkx 再读"对非 ASCII 字节往返一致
        // PORT: 平台字符集 (中文 Windows=GBK) ↔ UTF-8; 种子域纯 ASCII (od 实测,
        // reader.rs 先例), 等价。域假设被打破 (未来数据版本混入非 ASCII 字节) 时
        // 显式炸明原因 — Java 平台字符集解码不失败, 不以 io 错误面目误导排查
        let seed_text =
            String::from_utf8(std::fs::read(&fm_path).expect("种子文件读取")).unwrap_or_else(
                |e| panic!("种子文件非 UTF-8, §2.1 纯 ASCII 域假设被打破: {e}"),
            );
        let seed_name = {
            let n = Path::new(&fm_path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            n[..n.rfind('.').unwrap()].to_string()
        };
        println!(
            "种子: {} ({} chars) | 迭代 {iterations} | 种子值 {seed}\n",
            fm_path,
            seed_text.len()
        );

        let start = Instant::now();

        let mut c = Counters::default();

        // ---- 基线自检: 原始种子必须能正常解析 (否则数据有问题, 本套件直接判失败) ----
        if !baseline_check(&seed_text, &seed_name, &mut c) {
            panic!("失败: {} (基线未通过)", c.failed);
        }

        // ---- 生成全部变异体 (单个 Random 顺序驱动, 固定种子下完全可复现) ----
        let mut rnd = JavaRandom::new(seed);
        let mut mutants: Vec<String> = Vec::new();
        let mut kinds: Vec<i32> = Vec::new();
        let mut strategy_count = [0usize; STRATEGY_NAMES.len()];
        for _ in 0..iterations {
            let kind = rnd.next_int_bound(STRATEGY_NAMES.len()) as i32;
            strategy_count[kind as usize] += 1;
            mutants.push(mutate(&seed_text, kind, &mut rnd));
            kinds.push(kind);
        }

        // ---- 腿1: 每个变异体直接走 Blkx 全管线 ----
        println!(
            "-- 腿1: FmData 全管线 (构造器 + getAllplotdata + finalizeLoading) x{} --",
            mutants.len()
        );
        let tmp_fmdata = temp_file("voidmei_fuzz_", ".json");
        let tmp_str = tmp_fmdata.to_string_lossy().into_owned();
        for i in 0..mutants.len() {
            run_direct_pipeline(&mutants[i], kinds[i], i, &tmp_str, &seed_name, &mut c);
        }
        let _ = std::fs::remove_file(&tmp_fmdata);
        println!(
            "  完成: valid=true {} 个, valid=false {} 个, 逃逸异常 {} 个",
            c.valid_true,
            c.valid_false,
            c.ctor_exceptions + c.pipeline_exceptions
        );

        // ---- 腿2: 抽样变异体走 FMLoader.load (P2 句柄契约回归) ----
        if Path::new(&central_path).is_file() {
            // TODO(port): fm_loader.rs/fm_data_paths (含 set_data_root) 已落地,
            // 但临时数据根注入的测试接线未做 — 腿2 整段挂起, 不做无覆盖的死代码
            // 移植。接线批次按 Java runLoaderLeg 补: 临时 data 根注入
            // (fm_data_paths::set_data_root) + 中央文件真机原件拷入 + 物理文件名
            // 取中央文件 fmFile 字段 (extractFmFile, 回退 fm/<机型>.blkx 约定,
            // FMLoader 拼 fmfile+"x") + step = max(1, mutants/LOADER_SAMPLES)
            // 抽样 + fm_loader::load(plane), 断言句柄契约:
            // status ∈ {READY,MISSING,CORRUPT} ∧ READY⇔blkx!=null
            // ∧ isMissingLike⇒blkx==null; finally 还原数据根 "./data" + rmtree
            println!("\n-- 腿2 跳过: FMLoader 接线属后续批次 TODO(port) --");
        } else {
            println!("\n-- 腿2 跳过: 未提供有效的 --central (FMLoader 契约测试需要中央文件) --");
        }

        let elapsed = start.elapsed().as_millis();

        // ---- 汇总 ----
        println!("\n-- 变异策略分布 --");
        for (k, name) in STRATEGY_NAMES.iter().enumerate() {
            println!("  {name:<13} {}", strategy_count[k]);
        }
        println!("\n共 {} 个变异体, 总耗时 {elapsed} ms", c.fuzz_cases);
        println!("\n=== 测试结果 ===");
        println!("通过: {}", c.passed);
        println!("失败: {}", c.failed);

        assert_eq!(c.failed, 0, "FMParserFuzzer 存在失败项");
    }

    // =====================================================================
    // 边界/oracle 测试 — 期望值来自 OpenJDK 1.8.0_342 实测 dump
    // (build/oracle/rand/RandOracle.java: java.util.Random 原语流 + 经反射调用
    // bin/FMParserFuzzer 的私有 mutate; 摘要为 FNV-1a 64, 双语言同实现)
    // =====================================================================

    /// JavaRandom 逐位对拍: RI13/RD/RB (seed 20260825)、RIB (seed 42, 各 bound
    /// 域 — 2 的幂走移位快路径, 非幂走模拒采样)、RI (seed 0)、RL (seed -1)
    #[test]
    fn java8_oracle_java_random() {
        let mut r = JavaRandom::new(20260825);
        let ri13: Vec<usize> = (0..20).map(|_| r.next_int_bound(13)).collect();
        assert_eq!(
            ri13,
            vec![4, 11, 0, 6, 9, 10, 0, 12, 12, 1, 5, 6, 1, 6, 8, 10, 12, 9, 3, 7],
            "RI13"
        );

        let mut r2 = JavaRandom::new(20260825);
        let rd: Vec<f64> = (0..10).map(|_| r2.next_double()).collect();
        assert_eq!(
            rd,
            vec![
                0.26896081851807585,
                0.13900414146384943,
                0.2035343014870551,
                0.1737803292881298,
                0.7001690485633186,
                0.7783809576362863,
                0.8187746434214956,
                0.0688239131104933,
                0.5522897950671415,
                0.32519065658100865,
            ],
            "RD (Java Double.toString 最短往返表示, 逐位一致)"
        );

        let mut r3 = JavaRandom::new(20260825);
        let rb: Vec<bool> = (0..10).map(|_| r3.next_boolean()).collect();
        assert_eq!(rb, vec![false, true, false, false, false, true, false, false, true, false], "RB");

        // 各 bound 域 (seed 42, 12 抽样): (bound, 期望序列)
        let rib: &[(usize, &[usize])] = &[
            (1, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (2, &[1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0]),
            (3, &[2, 0, 0, 2, 0, 1, 2, 2, 1, 2, 2, 2]),
            (4, &[2, 0, 2, 0, 1, 3, 1, 2, 2, 0, 3, 1]),
            (8, &[5, 0, 5, 0, 2, 7, 2, 5, 5, 0, 7, 3]),
            (13, &[0, 7, 9, 12, 0, 4, 0, 3, 3, 2, 0, 0]),
            (30, &[20, 3, 18, 14, 0, 25, 5, 8, 19, 23, 2, 2]),
            (50, &[30, 13, 48, 34, 20, 25, 5, 18, 19, 43, 32, 2]),
            (64, &[46, 3, 43, 3, 19, 60, 17, 45, 42, 5, 57, 28]),
            (100, &[30, 63, 48, 84, 70, 25, 5, 18, 19, 93, 82, 2]),
            (500, &[130, 263, 248, 384, 470, 25, 5, 418, 19, 93, 182, 2]),
            (2000, &[1130, 763, 1248, 884, 1970, 1525, 1505, 918, 1519, 93, 1182, 1502]),
        ];
        for &(bound, expected) in rib {
            let mut rb = JavaRandom::new(42);
            let vs: Vec<usize> = (0..12).map(|_| rb.next_int_bound(bound)).collect();
            assert_eq!(vs, expected.to_vec(), "RIB bound={bound}");
        }

        // Random(0) 裸 nextInt() (32 位带符号) — LCG 经典已知值首项 -1155484576
        let mut r0 = JavaRandom::new(0);
        let ri: Vec<i32> = (0..5).map(|_| r0.next_int()).collect();
        assert_eq!(ri, vec![-1155484576, -723955400, 1033096058, -1690734402, -1557280266], "RI");

        // Random(-1) nextLong 序列 (48-bit mask + 符号扩展路径)
        let mut rm1 = JavaRandom::new(u64::MAX);
        let rl: Vec<i64> = (0..5).map(|_| rm1.next_long()).collect();
        assert_eq!(
            rl,
            vec![4961115982468162243, 226341162490527646, -6233441030884181172, 7681931065131779340, -3206673117535979274],
            "RL"
        );
    }

    /// mutate 逐策略对拍: 合成种子 (与 oracle 驱动同串, 覆盖引号/数值/花括号/
    /// 等号/缩进/多行特征), 每策略 3 个种子值 (s*1000003, s=1..3), 断言 (len, FNV)
    #[test]
    fn java8_oracle_mutate_synthetic_seed() {
        let seed = "unit {\n\tCompressor {\n\t\tNumSteps:i = 2\n\t\tAltitude0:r = 4100.5\n\t\t\"quoted str\" = 1.2e3 x\n\t}\n\tpower:r = -0.75\n\ttab\tand space line\n}\n";
        assert_eq!(seed.len(), 128, "SEEDLEN");
        assert_eq!(fnv1a64(seed), 2180320431869783377, "SEED 摘要");

        // (kind, Random 种子值, 变异体 len, FNV-1a) — Java FMParserFuzzer.mutate
        // 反射实测 dump (build/oracle/rand/dump.txt 的 MUT 行)
        let expected: &[(i32, u64, usize, u64)] = &[
            (0, 1000003, 101, 7942275050905475666),
            (0, 2000006, 62, 6008632289986650139),
            (0, 3000009, 75, 1900055459795406739),
            (1, 1000003, 128, 15745905061403712505),
            (1, 2000006, 128, 18419926186000616403),
            (1, 3000009, 128, 12977240643137439424),
            (2, 1000003, 136, 5609415801410960409),
            (2, 2000006, 132, 6313873520851764035),
            (2, 3000009, 132, 8662040781920836794),
            (3, 1000003, 109, 16240632171490147049),
            (3, 2000006, 127, 5165888100325241988),
            (3, 3000009, 126, 6453889614460086521),
            (4, 1000003, 129, 10854685119088187987),
            (4, 2000006, 129, 12139891116576668861),
            (4, 3000009, 129, 3930821916679596039),
            (5, 1000003, 133, 16781790601606669645),
            (5, 2000006, 131, 4444069168536690195),
            (5, 3000009, 131, 7156188852496035879),
            (6, 1000003, 128, 8919188402479937654),
            (6, 2000006, 129, 9956035144382960033),
            (6, 3000009, 119, 8196740564354154911),
            (7, 1000003, 127, 5630866010663283380),
            (7, 2000006, 127, 5165888100325241988),
            (7, 3000009, 127, 5165888100325241988),
            (8, 1000003, 128, 6885895681910970216),
            (8, 2000006, 128, 7724102417915451316),
            (8, 3000009, 128, 7724102417915451316),
            (9, 1000003, 132, 3586208781010445095),
            (9, 2000006, 130, 15901497229529787184),
            (9, 3000009, 130, 4926350075822839932),
            (10, 1000003, 125, 16022897660519819880),
            (10, 2000006, 624, 12239862842419567219),
            (10, 3000009, 130, 7620011606591156482),
            (11, 1000003, 126, 14478821148710163695),
            (11, 2000006, 126, 14478821148710163695),
            (11, 3000009, 126, 14478821148710163695),
            (12, 1000003, 133, 2785610248271761475),
            (12, 2000006, 131, 3434674900744992367),
            (12, 3000009, 146, 5421981286544649330),
        ];
        for &(kind, s, len, h) in expected {
            let mut mr = JavaRandom::new(s);
            let m = mutate(seed, kind, &mut mr);
            assert_eq!((m.len(), fnv1a64(&m)), (len, h), "kind {kind} seed {s}");
        }
    }

    /// 主循环消耗序烟雾: 真机 bf-109e-4 JSON 种子 + Random(20260825) 单序列驱动
    /// 13 轮变异, 断言种子指纹 + 每轮变异体可全管线解析 (无逃逸 panic)。
    /// (blkx→json 迁移: 种子载体由 BlkText 换为 JSON 后, Java dump 的逐轮
    ///  (kind, len, FNV) 期望表随之失效 — mutate/Random 的移植对拍职责由
    ///  合成种子的 java8_oracle_mutate 腿承担; 本腿保留真实数据变异鲁棒性)
    #[test]
    fn java8_oracle_mutate_real_seed_loop() {
        let fm_path = format!("{}/fm/bf-109e-4.json", fm_root());
        if !Path::new(&fm_path).is_file() {
            return; // data/ 未解包 (build.py 跳过语义)
        }
        let seed_text = std::fs::read_to_string(&fm_path).unwrap();
        // 种子身份指纹 (防 data 被静默更换; 换游戏版本重跑 fmdatajson 后需同步更新)
        assert_eq!((seed_text.len(), fnv1a64(&seed_text)), (26387, 15339736856552207565), "FMSEED");

        let mut rnd = JavaRandom::new(20260825);
        for i in 0..13 {
            let k = rnd.next_int_bound(STRATEGY_NAMES.len()) as i32;
            let m = mutate(&seed_text, k, &mut rnd);
            // 每轮变异体过全管线 (parse_str_json 内含 getload+plotdata, panic
            // 收敛 Err 合法; 断言① 只针对逃逸 panic — 直接调用即断言)
            let _ = FmData::parse_str_json("fuzz_seed_loop.json", &m);
            let _ = i;
        }
    }

    /// RE_NUM / RE_QUOTED 手写扫描器与 java.util.regex 语义边界
    #[test]
    fn num_and_quoted_scanner_boundaries() {
        // ---- RE_NUM: \d+\.?\d*(?:[eE][-+]?\d+)? ----
        assert_eq!(find_num_matches("a1.2e5b"), &[(1, 6)], "常规数值 (end 开区间)");
        assert_eq!(find_num_matches("1..2"), &[(0, 2), (3, 4)], "点后无数字仍消费 '.'");
        assert_eq!(find_num_matches("5e"), &[(0, 1)], "指数无数字不消费 e");
        assert_eq!(find_num_matches("5e+"), &[(0, 1)], "符号后无数字不消费");
        assert_eq!(find_num_matches("1.2e+3x"), &[(0, 6)], "带符号指数");
        assert_eq!(find_num_matches("1.e3"), &[(0, 4)], "点后直接指数");
        assert_eq!(find_num_matches("007 42"), &[(0, 3), (4, 6)], "多匹配");
        assert_eq!(find_num_matches("-0.75"), &[(1, 5)], "负号不属于数值字面量");
        assert_eq!(find_num_matches("no digits"), &[], "无匹配");
        assert_eq!(find_num_matches("12e34.5"), &[(0, 5), (6, 7)], "指数后不再吃 '.'");

        // ---- RE_QUOTED: "([^"\n\r]{1,60})" ----
        assert_eq!(find_quoted_matches("\"abc\""), &[(0, 5)]);
        assert_eq!(find_quoted_matches("\"unterminated"), &[], "无闭合引号");
        assert_eq!(find_quoted_matches("a\"b\"c\"d\""), &[(1, 4), (5, 8)], "多匹配");
        assert_eq!(find_quoted_matches("\"a\nb\""), &[], "换行中断不可回溯命中");
        // {1,60} 上界: 内容 61 字符 → 超界无匹配; 恰 60 字符 → 命中
        let inner61 = format!("\"{}\"", "x".repeat(61));
        assert_eq!(find_quoted_matches(&inner61), &[], "61 字符超 {{1,60}}");
        let inner60 = format!("\"{}\"", "y".repeat(60));
        assert_eq!(find_quoted_matches(&inner60), &[(0, 62)], "恰 60 字符命中");

        // 收集上限 5000 (Java: while (m.find() && matches.size() < 5000))
        let many = "1 ".repeat(5001);
        assert_eq!(find_num_matches(&many).len(), 5000, "数值匹配收集上限");
        let manyq = "\"q\" ".repeat(5001);
        assert_eq!(find_quoted_matches(&manyq).len(), 5000, "引号匹配收集上限");
    }

    /// 边界吸附助手 + 非 ASCII 病态输入下变异原语不 panic (§2.1 防御边界;
    /// 输出允许与 Java UTF-16 语义微偏, 见 floor/ceil_char_boundary 注)
    #[test]
    fn char_boundary_and_non_ascii_robustness() {
        assert_eq!(floor_char_boundary("aé中", 2), 1, "中点吸附到 'é' 起点");
        assert_eq!(floor_char_boundary("aé中", 3), 3);
        assert_eq!(ceil_char_boundary("aé", 2), 3, "中点向上吸附");
        assert_eq!(floor_char_boundary("abc", 99), 3, "越界收敛到 len");
        assert_eq!(ceil_char_boundary("abc", 0), 0);

        // 混合 CJK/引号/花括号/数值的长种子, 13 策略 × 4 种子全跑不 panic
        let seed = format!(
            "unit {{\n\t数值 = {}\n\t\"中文引号\" = {}\n\t{{\n\t}}\n{}\n",
            "1.5e3",
            "-0.75",
            "中".repeat(40)
        );
        for kind in 0..13i32 {
            for s in 1..5u64 {
                let mut r = JavaRandom::new(s * 7919);
                let m = mutate(&seed, kind, &mut r);
                // 变异结果必须是合法 UTF-8 (String 类型系统保证, 显式断言巩固)
                assert!(std::str::from_utf8(m.as_bytes()).is_ok());
            }
        }
    }
}

