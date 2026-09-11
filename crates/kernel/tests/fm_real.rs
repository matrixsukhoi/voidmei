//! 真机 FM 集成测试 (黑盒迁移自 src/fm/data/realtests.rs, 母本 = Java test/ 三套:
//! TestSpitfireF24Power / TestTempestMk5Power / FMParserFuzzer)。
//! 每个真机场景首行探测仓库根 data/aces/gamedata/flightmodels 存在性,
//! 缺失即 return 跳过 (CI 干净 clone 自动跳; 对齐 build.py run_fm_test 语义)。
//! 纯合成场景 (JavaRandom/mutate 对拍, 扫描器边界) 不依赖 data, 恒执行。
//! WTAPC 参考值与容差断言原样保留 — 改阈值须先 grep blkx 原始值区分数据变更与回归。

#![allow(non_snake_case)] // 测试名 = 模块前缀英文 + 中文场景 (风格约定, 豁免蛇形)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use kernel::fm::data::json::extract_fuel_modifications_json;
use kernel::fm::data::{FuelModification, FuelType};
use kernel::fm::piston_model::optimal_power_advanced;
use kernel::fm::power_extractor::{extract_stages, extract_stages_with_fuel};

/// 项目内真机 FM 数据根 (cargo 测试 cwd 无关; 路径相对仓库根)
fn fm_root() -> String {
    format!(
        "{}/../../data/aces/gamedata/flightmodels",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// data/ 在场探测 (场景首行的跳过闸门)
fn have_data(central: &str, fm: &str) -> bool {
    Path::new(central).is_file() && Path::new(fm).is_file()
}

/// 真机 JSON 全量解析 (name 取文件名分量, 对齐旧 parse 的 display 约定)
fn parse_real(path: &str) -> Result<kernel::fm::data::FmData, String> {
    let name = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    kernel::fm::data::FmData::parse_named_json(path, &name)
}

/// 中央文件 JSON → 燃油修正 (读失败/serde 失败 → 默认无修正)
fn fuel_mod_from_json(path: &str) -> FuelModification {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .map(|root| extract_fuel_modifications_json(&root))
        .unwrap_or_default()
}

/// Java static passed/failed 计数器 (软断言, 全部执行完才判失败 — Java main
/// 尾部 failed>0 才 exit(1) 的语义)
struct Tally {
    passed: usize,
    failed: usize,
}

impl Tally {
    fn new() -> Self {
        Tally { passed: 0, failed: 0 }
    }

    fn finish(&self, suite: &str) {
        println!("\n=== Results === {suite}: passed {}, failed {}", self.passed, self.failed);
        assert_eq!(self.failed, 0, "{suite} 存在失败断言");
    }

    fn assert_close(&mut self, name: &str, actual: f64, expected: f64, tolerance: f64) {
        if (actual - expected).abs() <= tolerance {
            println!("  PASS: {name} = {actual:.2} (expected {expected:.2})");
            self.passed += 1;
        } else {
            println!("  FAIL: {name} = {actual:.2} (expected {expected:.2}, tol {tolerance:.2})");
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
}

// ==================== spitfire ← TestSpitfireF24Power.java ====================

/// wtapc 参考值 (300 km/h IAS, 15°C; Command: python wtapc.py --fm ... --ias 300)
const SPIT_WTAPC_MIL: [[f64; 2]; 13] = [
    [0.0, 1347.4],
    [1000.0, 1389.7],
    [2000.0, 1428.2],
    [3000.0, 1462.9],
    [4000.0, 1494.3],
    [4100.0, 1510.0], // Peak at critical alt
    [5000.0, 1419.6],
    [6000.0, 1281.1],
    [7000.0, 1304.3],
    [8000.0, 1325.0],
    [8100.0, 1340.0], // Stage 2 peak
    [9000.0, 1309.1],
    [10000.0, 1170.6],
];

const SPIT_WTAPC_WEP: [[f64; 2]; 12] = [
    [0.0, 2172.0],
    [1000.0, 2240.3],
    [1830.0, 2292.5], // WEP peak
    [2000.0, 2252.5],
    [3000.0, 2021.5],
    [4000.0, 1917.4],
    [5000.0, 1963.1],
    [6000.0, 2003.8],
    [7000.0, 1884.3],
    [8000.0, 1719.0],
    [9000.0, 1564.8],
    [10000.0, 1408.8],
];

fn spitfire_paths() -> (String, String) {
    let central = format!("{}/spitfire_f24.json", fm_root());
    let fm = format!("{}/fm/spitfire_f24.json", fm_root());
    (central, fm)
}

/// 燃油修正解析: 英式 150 辛烷值检测 + invertEnableLogic=false + 乘数
#[test]
fn spitfire_燃油修正解析() {
    let (central, fm) = spitfire_paths();
    if !have_data(&central, &fm) {
        return; // data/ 未解包, 对齐 build.py 跳过语义
    }
    let mut t = Tally::new();
    let fuel_mod = fuel_mod_from_json(&central);

    t.assert_true(
        "detected British 150 octane fuel",
        fuel_mod.r#type == FuelType::British150Octane,
    );
    // Spitfire F24 的 datamine 中 invertEnableLogic:false (150 辛烷是升级项非默认)
    t.assert_true(
        "invertEnableLogic is false (150 octane is upgrade, not default)",
        !fuel_mod.british_invert_logic,
    );
    t.assert_close("afterburnerMult", fuel_mod.british_afterburner_mult, 1.42, 0.01);
    t.assert_close(
        "afterburnerCompressorMult",
        fuel_mod.british_afterburner_compressor_mult,
        1.33,
        0.01,
    );
    t.finish("spitfire_燃油修正解析");
}

/// 参数提取 + 燃油修正行为: 增压器两级表 / WEP 参数 / 无燃油 vs 有燃油提取
#[test]
fn spitfire_参数提取与燃油行为() {
    let (central, fm) = spitfire_paths();
    if !have_data(&central, &fm) {
        return; // data/ 未解包 (build.py 跳过语义)
    }
    let mut t = Tally::new();
    let fmdata = match parse_real(&fm) {
        Ok(b) => b,
        Err(_) => {
            println!("  SKIP: Cannot parse FM file");
            return;
        }
    };

    // 无燃油修正的提取: 两级增压器
    let stages_no_fuel = extract_stages(Some(&fmdata));
    t.assert_true("extracted stages without fuel", stages_no_fuel.is_some());
    t.assert_true(
        "has 2 stages",
        stages_no_fuel.as_ref().is_some_and(|s| s.len() == 2),
    );

    // 有燃油修正的提取
    let fuel_mod = fuel_mod_from_json(&central);
    let stages_with_fuel = extract_stages_with_fuel(Some(&fmdata), Some(&fuel_mod));
    t.assert_true("extracted stages with fuel", stages_with_fuel.is_some());

    // invertEnableLogic=false → 燃油修正应改变 WEP 参数 (加 150 辛烷 = 增益)
    if let (Some(no_fuel), Some(with_fuel)) = (stages_no_fuel.as_ref(), stages_with_fuel.as_ref()) {
        let mut wep_changed = false;
        for i in 0..no_fuel.len() {
            if (no_fuel[i].wep_power_mult - with_fuel[i].wep_power_mult).abs() > 0.001
                || (no_fuel[i].wep_crit_alt - with_fuel[i].wep_crit_alt).abs() > 1.0
            {
                wep_changed = true;
            }
        }
        t.assert_true(
            "WEP parameters change with fuel (invertEnableLogic=false)",
            wep_changed,
        );
    }

    // FM 关键参数 (wtapc 同源)
    let comp = fmdata.compressor.as_ref().unwrap();
    t.assert_close("compressor NumSteps", fmdata.comp_num_steps as f64, 2.0, 0.0);
    t.assert_close("Stage 0 altitude", comp.alt[0], 4100.0, 0.0);
    t.assert_close("Stage 1 altitude", comp.alt[1], 8100.0, 0.0);
    t.assert_close("Stage 0 power", comp.power[0], 1510.0, 0.0);
    t.assert_close("Stage 1 power", comp.power[1], 1340.0, 0.0);
    t.assert_close("AfterburnerBoost", fmdata.aftb_coff, 1.41, 0.01);
    t.assert_close("AfterburnerManifoldPressure", fmdata.wep_manifold_pressure, 2.22, 0.01);
    t.assert_close("SpeedManifoldMultiplier", fmdata.speed_to_manifold_multiplier, 0.8, 0.01);
    t.finish("spitfire_参数提取与燃油行为");
}

/// 功率曲线对拍 wtapc: 军用/WEP 全高度表 + 峰值 (容差原样保留)
#[test]
fn spitfire_功率曲线对拍wtapc() {
    let (central, fm) = spitfire_paths();
    if !have_data(&central, &fm) {
        return; // data/ 未解包 (build.py 跳过语义)
    }
    let mut t = Tally::new();
    let fmdata = match parse_real(&fm) {
        Ok(b) => b,
        Err(_) => {
            println!("  SKIP: Cannot parse FM file");
            return;
        }
    };
    let fuel_mod = fuel_mod_from_json(&central);

    // wtapc 用满升级 → 带燃油修正的提取
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

    println!("\n  === Military Power Curve (300 km/h IAS) ===");
    let mut max_mil_error = 0.0f64;
    for r#ref in &SPIT_WTAPC_MIL {
        let actual =
            optimal_power_advanced(&stages, r#ref[0], false, speed_kmh, is_ias, sea_level_temp);
        let abs_diff = (actual - r#ref[1]).abs();
        println!("  {:5.0}    {actual:7.1}    {:5.1}", r#ref[0], r#ref[1]);
        max_mil_error = max_mil_error.max(abs_diff);
    }

    println!("\n  === WEP Power Curve (300 km/h IAS) ===");
    let mut max_wep_error = 0.0f64;
    for r#ref in &SPIT_WTAPC_WEP {
        let actual =
            optimal_power_advanced(&stages, r#ref[0], true, speed_kmh, is_ias, sea_level_temp);
        let abs_diff = (actual - r#ref[1]).abs();
        println!("  {:5.0}    {actual:7.1}    {:5.1}", r#ref[0], r#ref[1]);
        max_wep_error = max_wep_error.max(abs_diff);
    }

    println!("\n  max error: mil {max_mil_error:.1} hp, wep {max_wep_error:.1} hp");

    // 峰值扫描 (50m 步进)
    let (mut mil_pp, mut mil_pa) = (0.0f64, 0.0f64);
    let (mut wep_pp, mut wep_pa) = (0.0f64, 0.0f64);
    for alt in (0..=10000i32).step_by(50) {
        let mil = optimal_power_advanced(&stages, alt as f64, false, speed_kmh, is_ias, sea_level_temp);
        let wep = optimal_power_advanced(&stages, alt as f64, true, speed_kmh, is_ias, sea_level_temp);
        if mil > mil_pp {
            (mil_pp, mil_pa) = (mil, alt as f64);
        }
        if wep > wep_pp {
            (wep_pp, wep_pa) = (wep, alt as f64);
        }
    }
    println!("  peaks: mil {mil_pp:.1}@{mil_pa:.0}m, wep {wep_pp:.1}@{wep_pa:.0}m");

    // 容差断言 (母本原样: 调试期放宽值)
    t.assert_close("Military peak power", mil_pp, 1510.0, 50.0);
    t.assert_close("WEP peak power", wep_pp, 2292.5, 100.0);
    t.finish("spitfire_功率曲线对拍wtapc");
}

// ==================== tempest ← TestTempestMk5Power.java ====================

const TEMP_WTAPC_MIL: [[f64; 2]; 12] = [
    [0.0, 1982.4],
    [1000.0, 2031.5],
    [1730.0, 2064.7], // Peak at ~1730m
    [2000.0, 2001.8],
    [3000.0, 1773.7],
    [4000.0, 1704.3],
    [5000.0, 1726.7],
    [6000.0, 1615.6],
    [7000.0, 1432.2],
    [8000.0, 1269.0],
    [9000.0, 1124.1],
    [10000.0, 994.2],
];

const TEMP_WTAPC_WEP: [[f64; 2]; 11] = [
    [0.0, 2439.9], // Peak at sea level
    [1000.0, 2223.0],
    [2000.0, 2041.6],
    [3000.0, 2075.9],
    [4000.0, 2045.9],
    [5000.0, 1844.6],
    [6000.0, 1650.3],
    [7000.0, 1466.0],
    [8000.0, 1302.0],
    [9000.0, 1156.3],
    [10000.0, 1025.8],
];

fn tempest_paths() -> (String, String) {
    let central = format!("{}/tempest_mkv.json", fm_root());
    let fm = format!("{}/fm/tempest_mkv.json", fm_root());
    (central, fm)
}

/// 反转逻辑解析 (invertEnableLogic=true: 150 辛烷是默认态) + 参数提取
#[test]
fn tempest_反转逻辑与参数提取() {
    let (central, fm) = tempest_paths();
    if !have_data(&central, &fm) {
        return; // data/ 未解包 (build.py 跳过语义)
    }
    let mut t = Tally::new();

    let fuel_mod = fuel_mod_from_json(&central);
    t.assert_true(
        "detected British 150 octane fuel",
        fuel_mod.r#type == FuelType::British150Octane,
    );
    // Tempest Mk V: invertEnableLogic:b = true — 150 辛烷即默认燃油
    t.assert_true(
        "invertEnableLogic is true (150 octane is default)",
        fuel_mod.british_invert_logic,
    );

    let fmdata = match parse_real(&fm) {
        Ok(b) => b,
        Err(_) => {
            println!("  SKIP: Cannot parse FM file");
            return;
        }
    };

    // invertEnableLogic=true → 燃油修正不应改变 WEP 参数 (默认已是 150 辛烷)
    let stages_no_fuel = extract_stages(Some(&fmdata));
    let stages_with_fuel = extract_stages_with_fuel(Some(&fmdata), Some(&fuel_mod));
    if let (Some(no_fuel), Some(with_fuel)) = (stages_no_fuel.as_ref(), stages_with_fuel.as_ref()) {
        let mut wep_unchanged = true;
        for i in 0..no_fuel.len() {
            if (no_fuel[i].wep_power_mult - with_fuel[i].wep_power_mult).abs() > 0.001
                || (no_fuel[i].wep_crit_alt - with_fuel[i].wep_crit_alt).abs() > 1.0
            {
                wep_unchanged = false;
            }
        }
        t.assert_true(
            "WEP parameters unchanged (invertEnableLogic=true means no bonus)",
            wep_unchanged,
        );
    }

    // 两级增压器表 (期望值跟随游戏 FM 数据版本, 见母本注释)
    t.assert_true("extracted stages", stages_no_fuel.is_some());
    t.assert_true(
        "has 2 stages",
        stages_no_fuel.as_ref().is_some_and(|s| s.len() == 2),
    );
    let comp = fmdata.compressor.as_ref().unwrap();
    t.assert_close("compressor NumSteps", fmdata.comp_num_steps as f64, 2.0, 0.0);
    // Stage 0 临界高度: 期望值须跟随游戏 FM 数据版本更新
    // (WT 2.57.1.103 中 tempest_mkv 的 Altitude0 已从 1730 调整为 1447;
    //  fmdata 更新后若此处 FAIL, 先 grep blkx 原始值区分数据变更与程序回归)
    t.assert_close("Stage 0 altitude", comp.alt[0], 1447.0, 50.0);
    t.assert_close("Stage 1 altitude", comp.alt[1], 5000.0, 200.0);
    t.finish("tempest_反转逻辑与参数提取");
}

/// 军用与 WEP 功率曲线对拍 wtapc (300 km/h IAS, 15°C)
#[test]
fn tempest_军用与WEP功率曲线() {
    let (central, fm) = tempest_paths();
    if !have_data(&central, &fm) {
        return; // data/ 未解包 (build.py 跳过语义)
    }
    let mut t = Tally::new();
    let _ = central; // invertEnableLogic=true → 带不带燃油修正结果一致 (母本同)
    let fmdata = match parse_real(&fm) {
        Ok(b) => b,
        Err(_) => {
            println!("  SKIP: Cannot parse FM file");
            return;
        }
    };
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

    println!("\n  === Military Power Curve ===");
    let mut max_mil_error = 0.0f64;
    for r#ref in &TEMP_WTAPC_MIL {
        let actual =
            optimal_power_advanced(&stages, r#ref[0], false, speed_kmh, is_ias, sea_level_temp);
        println!("  {:5.0}    {actual:7.1}    {:5.1}", r#ref[0], r#ref[1]);
        max_mil_error = max_mil_error.max((actual - r#ref[1]).abs());
    }
    t.assert_true("Military max error < 5 hp", max_mil_error < 5.0);

    println!("\n  === WEP Power Curve ===");
    let mut max_wep_error = 0.0f64;
    for r#ref in &TEMP_WTAPC_WEP {
        let actual =
            optimal_power_advanced(&stages, r#ref[0], true, speed_kmh, is_ias, sea_level_temp);
        println!("  {:5.0}    {actual:7.1}    {:5.1}", r#ref[0], r#ref[1]);
        max_wep_error = max_wep_error.max((actual - r#ref[1]).abs());
    }
    t.assert_true("WEP max error < 10 hp", max_wep_error < 10.0);

    // WEP 峰值 (海平面)
    let (mut pp, mut pa) = (0.0f64, 0.0f64);
    for alt in (0..=10000i32).step_by(50) {
        let wep = optimal_power_advanced(&stages, alt as f64, true, speed_kmh, is_ias, sea_level_temp);
        if wep > pp {
            (pp, pa) = (wep, alt as f64);
        }
    }
    t.assert_close("WEP peak power", pp, 2439.9, 10.0);
    t.assert_close("WEP peak altitude", pa, 0.0, 100.0);
    t.finish("tempest_军用与WEP功率曲线");
}

// ==================== fuzzer ← FMParserFuzzer.java ====================
//
// 种子 = 真机 bf-109e-4 JSON (中等体积, 覆盖数值/引号/花括号特征);
// 变异 = 字节级/行级/结构级/语义级四类 13 策略, 每个变异体走完整解析管线。
// JavaRandom/mutate 与 Java 端逐位一致 (基线: build/基线/rand/RandOracle.java
// 在 OpenJDK 1.8.0_342 实测 dump)。

const FUZZ_ITERATIONS: usize = 200; // Java 原值, 无降档
const FUZZ_SEED: u64 = 20260825; // 固定种子保证可复现
const PER_CASE_LIMIT_MS: u128 = 5000; // 单变异体耗时上限 (疑似死循环判失败)

const STRATEGY_NAMES: [&str; 13] = [
    "truncate",
    "charReplace",
    "chunkPaste",
    "deleteLine",
    "shuffleLines",
    "commentLine",
    "stripIndent",
    "dropBrace",
    "killEquals",
    "injectNest",
    "numberMutate",
    "unquote",
    "jsonInject",
];

/// java.util.Random (OpenJDK 8) 的逐位移植 — 48-bit LCG
struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    fn new(seed: u64) -> Self {
        JavaRandom {
            seed: (seed ^ 0x5DEECE66D) & ((1 << 48) - 1),
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(0x5DEECE66D).wrapping_add(0xB) & ((1 << 48) - 1);
        (self.seed >> (48 - bits)) as i32
    }

    fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// n>0: 2 的幂走移位快路径, 否则模拒采样 (Java nextInt(int) 语义)
    fn next_int_bound(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        let n = n as i32;
        if (n & -n) == n {
            return ((n as i64 * (self.next(31) as i64)) >> 31) as usize;
        }
        loop {
            let bits = self.next(31);
            let val = bits % n;
            if !(bits.wrapping_sub(val).wrapping_add(n - 1) < 0) {
                return val as usize;
            }
        }
    }

    fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    fn next_double(&mut self) -> f64 {
        let hi = self.next(26) as u64;
        let lo = self.next(27) as u64;
        (((hi << 27) + lo) as f64) / ((1u64 << 53) as f64)
    }
}

// ---- 变异原语 (四类 13 种; 母本一比一) ----

fn mutate(s: &str, kind: i32, rnd: &mut JavaRandom) -> String {
    match kind {
        0 => truncate(s, rnd),
        1 => char_replace(s, rnd),
        2 => chunk_paste(s, rnd),
        3 => delete_lines(s, rnd),
        4 => shuffle_lines(s, rnd),
        5 => comment_lines(s, rnd),
        6 => strip_indent(s, rnd),
        7 => drop_brace(s, rnd),
        8 => kill_equals(s, rnd),
        9 => inject_nest(s, rnd),
        10 => number_mutate(s, rnd),
        11 => unquote(s, rnd),
        _ => json_inject(s, rnd),
    }
}

fn floor_char_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_char_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

fn truncate(s: &str, rnd: &mut JavaRandom) -> String {
    let len = s.len();
    if len < 32 {
        return s.to_string();
    }
    let cut = 1.max((len as f64 * (0.02 + rnd.next_double() * 0.53)) as i32) as usize;
    match rnd.next_int_bound(3) {
        0 => s[ceil_char_boundary(s, cut)..].to_string(),
        1 => s[..floor_char_boundary(s, len - cut)].to_string(),
        _ => {
            let at = floor_char_boundary(s, rnd.next_int_bound(len - cut));
            let end = ceil_char_boundary(s, at + cut);
            format!("{}{}", &s[..at], &s[end..])
        }
    }
}

fn char_replace(s: &str, rnd: &mut JavaRandom) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    if len < 16 {
        return s.to_string();
    }
    let pool: Vec<char> = "abcXYZ019 \t\n\r\"'{}[]<>=:,;.+-*/\\#$%&()!?|~^`_".chars().collect();
    let n = 1 + rnd.next_int_bound(8);
    for _ in 0..n {
        let at = rnd.next_int_bound(len);
        chars[at] = pool[rnd.next_int_bound(pool.len())];
    }
    chars.into_iter().collect()
}

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

fn delete_lines(s: &str, rnd: &mut JavaRandom) -> String {
    let lines: Vec<&str> = s.split('\n').collect();
    if lines.len() < 4 {
        return s.to_string();
    }
    let n = 1 + rnd.next_int_bound(3);
    let first = rnd.next_int_bound(lines.len());
    let mut sb = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i >= first && i < first + n {
            continue;
        }
        sb.push_str(line);
        sb.push('\n');
    }
    sb
}

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

fn comment_lines(s: &str, rnd: &mut JavaRandom) -> String {
    let mut lines: Vec<String> = s.split('\n').map(|l| l.to_string()).collect();
    if lines.len() < 4 {
        return s.to_string();
    }
    let n = 1 + rnd.next_int_bound(3);
    for _ in 0..n {
        let i = rnd.next_int_bound(lines.len());
        if !lines[i].trim().is_empty() {
            lines[i] = format!("//{}", lines[i]);
        }
    }
    join(&lines)
}

fn strip_indent(s: &str, rnd: &mut JavaRandom) -> String {
    let mut lines: Vec<String> = s.split('\n').map(|l| l.to_string()).collect();
    if lines.len() < 4 {
        return s.to_string();
    }
    let w = rnd.next_int_bound(lines.len());
    let end = lines.len().min(w + 30);
    for line in lines.iter_mut().take(end).skip(w) {
        let n = line
            .as_bytes()
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        *line = line[n..].to_string();
    }
    join(&lines)
}

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
    format!("{}{}", &s[..at], &s[at + 1..])
}

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

fn inject_nest(s: &str, rnd: &mut JavaRandom) -> String {
    let mut sb = String::from(s);
    let n = 1 + rnd.next_int_bound(3);
    for _ in 0..n {
        let at = ceil_char_boundary(&sb, rnd.next_int_bound(sb.len() + 1));
        sb.insert_str(at, "{\n");
    }
    sb
}

fn num_replacements() -> [String; 5] {
    [
        "NaN".to_string(),
        "1e999".to_string(),
        "-1e999".to_string(),
        "9".repeat(500),
        "-0".to_string(),
    ]
}

fn number_mutate(s: &str, rnd: &mut JavaRandom) -> String {
    let matches = find_num_matches(s);
    if matches.is_empty() {
        return s.to_string();
    }
    let pick = matches[rnd.next_int_bound(matches.len())];
    let repl = &num_replacements()[rnd.next_int_bound(5)];
    format!("{}{}{}", &s[..pick.0], repl, &s[pick.1..])
}

fn unquote(s: &str, rnd: &mut JavaRandom) -> String {
    let matches = find_quoted_matches(s);
    if matches.is_empty() {
        return s.to_string();
    }
    let pick = matches[rnd.next_int_bound(matches.len())];
    format!("{}{}{}", &s[..pick.0], &s[pick.0 + 1..pick.1 - 1], &s[pick.1..])
}

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

fn join<S: AsRef<str>>(lines: &[S]) -> String {
    let mut sb = String::new();
    for l in lines {
        sb.push_str(l.as_ref());
        sb.push('\n');
    }
    sb
}

/// Java `RE_NUM = \d+\.?\d*(?:[eE][-+]?\d+)?` 的 find() 全序列 (上限 5000)
fn find_num_matches(s: &str) -> Vec<(usize, usize)> {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"[0-9]+\.?[0-9]*(?:[eE][-+]?[0-9]+)?").unwrap()
    });
    RE.find_iter(s).map(|m| (m.start(), m.end())).take(5000).collect()
}

/// Java `RE_QUOTED = "([^"\n\r]{1,60})"` 的 find() 全序列 (上限 5000)
fn find_quoted_matches(s: &str) -> Vec<(usize, usize)> {
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r#""([^"\n\r]{1,60})""#).unwrap());
    RE.find_iter(s).map(|m| (m.start(), m.end())).take(5000).collect()
}

/// FNV-1a 64 位摘要 (双语言各 10 行即可逐字节对拍; 基线 RandOracle.java 同款)
fn fnv1a64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// 唯一临时文件路径 (Java Files.createTempFile 等价)
fn temp_file(prefix: &str, suffix: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("{prefix}{}_{}{suffix}", std::process::id(), n))
}

/// 文本变异 Fuzz (真机种子): 每个变异体走 FmData 全解析管线 —
/// ① 逃逸 panic 即失败; ② 单体 5s 限时; ③ 解析失败收敛 Err (不触解析字段)。
/// (母本腿2 FMLoader 抽样仍挂起 — TODO 备案, 禁为此放宽断言)
#[test]
fn fuzzer_文本变异鲁棒性() {
    let fm_path = format!("{}/fm/bf-109e-4.json", fm_root());
    if !Path::new(&fm_path).is_file() {
        return; // data/ 未解包, 对齐 build.py run_fm_test 跳过语义
    }
    let seed_text = String::from_utf8(std::fs::read(&fm_path).expect("种子文件读取"))
        .unwrap_or_else(|e| panic!("种子文件非 UTF-8, 纯 ASCII 域假设被打破: {e}"));
    println!("种子: {fm_path} ({} chars) | 迭代 {FUZZ_ITERATIONS} | 种子值 {FUZZ_SEED}", seed_text.len());

    let start = Instant::now();
    let (mut passed, mut failed) = (0usize, 0usize);
    let (mut valid_true, mut valid_false) = (0usize, 0usize);

    // 基线自检: 原始种子必须能正常解析 (否则数据/环境有误)
    let base = std::panic::catch_unwind(|| parse_real(&fm_path));
    match base {
        Err(_) => panic!("基线: 原始种子解析逃逸 panic"),
        Ok(Err(_)) => panic!("基线: 原始种子解析失败 (不应发生)"),
        Ok(Ok(b)) => {
            assert!(b.valid, "基线 parse 返回 Ok 但 valid=false (违反 json.rs 契约)");
            passed += 1;
        }
    }

    // 生成全部变异体 (单个 Random 顺序驱动, 固定种子下完全可复现)
    let mut rnd = JavaRandom::new(FUZZ_SEED);
    let mut strategy_count = [0usize; STRATEGY_NAMES.len()];
    let mut mutants: Vec<String> = Vec::new();
    let mut kinds: Vec<i32> = Vec::new();
    for _ in 0..FUZZ_ITERATIONS {
        let kind = rnd.next_int_bound(STRATEGY_NAMES.len()) as i32;
        strategy_count[kind as usize] += 1;
        mutants.push(mutate(&seed_text, kind, &mut rnd));
        kinds.push(kind);
    }

    // 腿1: 每个变异体直接走 FmData 全管线 (blkx→json 迁移: 内容注入入口已随
    // parse_str_json 退役, 变异体落临时文件后走 parse_named_json — 等价路径)
    let tmp = temp_file("voidmei_fuzz_", ".json");
    for i in 0..mutants.len() {
        let t0 = Instant::now();
        std::fs::write(&tmp, &mutants[i]).expect("变异体落盘");
        let parsed = std::panic::catch_unwind(|| parse_real(tmp.to_str().unwrap()));
        match parsed {
            Err(_) => {
                println!("  [失败] #{i} ({}) 逃逸异常[构造器]: panic", STRATEGY_NAMES[kinds[i] as usize]);
                failed += 1;
            }
            Ok(Err(_)) => valid_false += 1, // 解析失败收敛 Err = 文本版 valid=false
            Ok(Ok(b)) => {
                assert!(b.valid, "#{i} parse 返回 Ok 但 valid=false (违反 json.rs 契约)");
                valid_true += 1;
            }
        }
        let ms = t0.elapsed().as_millis();
        if ms > PER_CASE_LIMIT_MS {
            println!("  [失败] #{i} ({}) 单文件耗时 {ms} ms 超上限", STRATEGY_NAMES[kinds[i] as usize]);
            failed += 1;
        }
    }
    let _ = std::fs::remove_file(&tmp);

    println!("valid=true {valid_true}, valid=false {valid_false}, 耗时 {} ms", start.elapsed().as_millis());
    for (k, name) in STRATEGY_NAMES.iter().enumerate() {
        println!("  {name:<13} {}", strategy_count[k]);
    }
    assert_eq!(failed, 0, "FMParserFuzzer 存在失败项");
    assert!(passed >= 1);
}

/// JavaRandom/mutate 逐位对拍 (纯合成种子, 无 data 依赖):
/// OpenJDK 1.8.0_342 实测 dump (build/基线/rand/dump.txt)
#[test]
fn fuzzer_JavaRandom与mutate对拍() {
    // RI13 (seed 20260825, bound 13)
    let mut r = JavaRandom::new(20260825);
    let ri13: Vec<usize> = (0..20).map(|_| r.next_int_bound(13)).collect();
    assert_eq!(
        ri13,
        vec![4, 11, 0, 6, 9, 10, 0, 12, 12, 1, 5, 6, 1, 6, 8, 10, 12, 9, 3, 7],
        "RI13"
    );

    // RD (seed 20260825) — Java Double.toString 最短往返表示, 逐位一致
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
        "RD"
    );

    // RB (seed 20260825)
    let mut r3 = JavaRandom::new(20260825);
    let rb: Vec<bool> = (0..10).map(|_| r3.next_boolean()).collect();
    assert_eq!(rb, vec![false, true, false, false, false, true, false, false, true, false], "RB");

    // RIB 各 bound 域 (seed 42, 12 抽样): 2 的幂走移位快路径, 非幂走模拒采样
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
        (
            2000,
            &[1130, 763, 1248, 884, 1970, 1525, 1505, 918, 1519, 93, 1182, 1502],
        ),
    ];
    for &(bound, expected) in rib {
        let mut rb = JavaRandom::new(42);
        let vs: Vec<usize> = (0..12).map(|_| rb.next_int_bound(bound)).collect();
        assert_eq!(vs, expected.to_vec(), "RIB bound={bound}");
    }

    // Random(0) 裸 nextInt — LCG 经典已知值首项 -1155484576
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

    // mutate 逐策略对拍: 合成种子 (覆盖引号/数值/花括号/等号/缩进/多行特征)
    let seed = "unit {\n\tCompressor {\n\t\tNumSteps:i = 2\n\t\tAltitude0:r = 4100.5\n\t\t\"quoted str\" = 1.2e3 x\n\t}\n\tpower:r = -0.75\n\ttab\tand space line\n}\n";
    assert_eq!(seed.len(), 128, "SEEDLEN");
    assert_eq!(fnv1a64(seed), 2180320431869783377, "SEED 摘要");

    // (kind, Random 种子值, 变异体 len, FNV-1a) — Java FMParserFuzzer.mutate 反射实测 dump
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

/// 扫描器边界 + 非 ASCII 病态输入的变异原语稳健性 (纯合成, 无 data 依赖)
#[test]
fn fuzzer_扫描器边界与非ASCII稳健() {
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
    assert_eq!(find_quoted_matches(&inner61), &[], "61 字符超上限");
    let inner60 = format!("\"{}\"", "y".repeat(60));
    assert_eq!(find_quoted_matches(&inner60), &[(0, 62)], "恰 60 字符命中");

    // 收集上限 5000
    let many = "1 ".repeat(5001);
    assert_eq!(find_num_matches(&many).len(), 5000, "数值匹配收集上限");
    let manyq = "\"q\" ".repeat(5001);
    assert_eq!(find_quoted_matches(&manyq).len(), 5000, "引号匹配收集上限");

    // ---- 边界吸附 + 非 ASCII 病态输入下变异原语不 panic ----
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
            assert!(std::str::from_utf8(m.as_bytes()).is_ok(), "变异结果须为合法 UTF-8");
        }
    }
}
