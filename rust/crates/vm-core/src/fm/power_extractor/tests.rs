use super::*;
use crate::fm::data::json::extract_fuel_modifications_json;
use crate::fm::piston_model::optimal_power_advanced;

/// Java 8 oracle 混合容差 (atmosphere_model.rs / piston_power_model.rs 同款):
/// 1e-12·max(|expected|,1); Math.pow 跨 libm 允许最后几位 ULP 差异
fn check(name: &str, actual: f64, expected: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= 1e-12 * expected.abs().max(1.0),
        "oracle mismatch {name}: rust={actual:?} java={expected:?}"
    );
}

/// 逐字段对拍一个 stage (18 个 f64 字段 + 2 个标量字段)
fn assert_stage(tag: &str, a: &CompressorStageParams, e: &CompressorStageParams) {
    check(&format!("{tag}.critAlt"), a.crit_alt, e.crit_alt);
    check(&format!("{tag}.critPower"), a.crit_power, e.crit_power);
    check(&format!("{tag}.deckPower"), a.deck_power, e.deck_power);
    check(&format!("{tag}.deckAlt"), a.deck_alt, e.deck_alt);
    check(&format!("{tag}.curvature"), a.curvature, e.curvature);
    check(&format!("{tag}.wepCritAlt"), a.wep_crit_alt, e.wep_crit_alt);
    check(
        &format!("{tag}.wepPowerMult"),
        a.wep_power_mult,
        e.wep_power_mult,
    );
    check(
        &format!("{tag}.speedManifoldMult"),
        a.speed_manifold_mult,
        e.speed_manifold_mult,
    );
    check(
        &format!("{tag}.constRpmAlt"),
        a.const_rpm_alt,
        e.const_rpm_alt,
    );
    check(
        &format!("{tag}.constRpmPower"),
        a.const_rpm_power,
        e.const_rpm_power,
    );
    check(&format!("{tag}.ceilingAlt"), a.ceiling_alt, e.ceiling_alt);
    check(
        &format!("{tag}.ceilingPower"),
        a.ceiling_power,
        e.ceiling_power,
    );
    check(
        &format!("{tag}.oldAltitude"),
        a.old_altitude,
        e.old_altitude,
    );
    check(&format!("{tag}.oldPower"), a.old_power, e.old_power);
    check(
        &format!("{tag}.oldPowerNewRpm"),
        a.old_power_new_rpm,
        e.old_power_new_rpm,
    );
    check(&format!("{tag}.wepDeckAlt"), a.wep_deck_alt, e.wep_deck_alt);
    check(
        &format!("{tag}.wepConstRpmAlt"),
        a.wep_const_rpm_alt,
        e.wep_const_rpm_alt,
    );
    check(
        &format!("{tag}.stage0DeckAlt"),
        a.stage0_deck_alt,
        e.stage0_deck_alt,
    );
    assert_eq!(a.stage_index, e.stage_index, "{tag}.stageIndex");
    assert_eq!(a.exact_altitudes, e.exact_altitudes, "{tag}.exactAltitudes");
}

/// 期望值构造助手 (Java oracle dump 的 stage 全字段)
// PORT: Java 保真 — 20 参逐字段对应 oracle dump, 不打包成结构体
#[allow(clippy::too_many_arguments)]
fn exp(
    crit_alt: f64,
    crit_power: f64,
    deck_power: f64,
    deck_alt: f64,
    curvature: f64,
    wep_crit_alt: f64,
    wep_power_mult: f64,
    speed_mm: f64,
    const_rpm_alt: f64,
    const_rpm_power: f64,
    ceiling_alt: f64,
    ceiling_power: f64,
    old_altitude: f64,
    old_power: f64,
    old_power_new_rpm: f64,
    wep_deck_alt: f64,
    wep_const_rpm_alt: f64,
    stage0_deck_alt: f64,
    stage_index: i32,
    exact_altitudes: bool,
) -> CompressorStageParams {
    CompressorStageParams {
        crit_alt,
        crit_power,
        deck_power,
        deck_alt,
        curvature,
        wep_crit_alt,
        wep_power_mult,
        speed_manifold_mult: speed_mm,
        const_rpm_alt,
        const_rpm_power,
        ceiling_alt,
        ceiling_power,
        old_altitude,
        old_power,
        old_power_new_rpm,
        wep_deck_alt,
        wep_const_rpm_alt,
        stage0_deck_alt,
        stage_index,
        exact_altitudes,
    }
}

// ---- 真机 fixture (FM 参数取自 data/aces/gamedata/flightmodels/, f32 拓宽见模块注) ----

/// spitfire_f24.blkx (fm) — TestSpitfireF24Power 的被测机
// PORT: Blkx 含 Java-private 字段 (blkx 模块树内可见, mod.rs 设计), 外部
/// struct 字面量 + ..default() 不可用 → default() 后逐 pub 字段赋值
fn spitfire_f24() -> FmData {
    let mut b = FmData::default();
    b.comp_num_steps = 2;
    b.is_jet = false;
    b.military_rpm = 2600.0;
    b.wep_rpm = 2750.0;
    b.shaft_rpm_max = 0.0;
    b.rpm_nom = 2600.0;
    b.governor_max_param = 2600.0;
    b.military_mp = 1.61f32 as f64;
    b.wep_manifold_pressure = 2.22f32 as f64;
    b.aftb_coff = 1.41f32 as f64;
    b.throttle_boost = 1.0001f32 as f64;
    b.octane_afterburner_mult = 1.0;
    b.speed_to_manifold_multiplier = 0.8f32 as f64;
    b.deck_power = 1360.0;
    b.comp_pressure_at_rpm0 = 0.3f32 as f64;
    b.comp_omega_factor_sq = 1.0;
    b.has_comp_omega_factor_sq = true;
    b.explicit_exact_altitudes = Some(true);
    b.compressor = Some(CompressorData {
        alt: vec![4100.0, 8100.0],
        power: vec![1510.0, 1340.0],
        ceil: vec![10000.0, 12000.0],
        ceil_pwr: vec![600.0, 830.0],
        rpm_ratio: vec![0.5, 0.5],
        boost: vec![1.01f32 as f64, 0.98f32 as f64],
        has_boost: Some(vec![true, true]),
        const_rpm_alt: Some(vec![18034.6f32 as f64, 18034.6f32 as f64]),
        const_rpm_power: Some(vec![200.0, 200.0]),
    });
    b.comp_afterburner_pressure_boost = Some(vec![0.0, 0.0]);
    b
}

/// yak-3.blkx (fm) — 苏联 B-100 油料 (spm=1.018) 路径
fn yak3() -> FmData {
    let mut b = FmData::default();
    b.comp_num_steps = 2;
    b.is_jet = false;
    b.military_rpm = 2700.0;
    b.wep_rpm = 2700.0;
    b.shaft_rpm_max = 2700.0;
    b.rpm_nom = 0.0;
    b.governor_max_param = 0.0;
    b.military_mp = 1.447f32 as f64;
    b.wep_manifold_pressure = 1.48f32 as f64;
    b.aftb_coff = 1.0;
    b.throttle_boost = 1.0;
    b.octane_afterburner_mult = 1.0;
    b.speed_to_manifold_multiplier = 1.0;
    b.deck_power = 1290.0;
    b.comp_pressure_at_rpm0 = 0.4f32 as f64;
    b.comp_omega_factor_sq = 0.0;
    b.has_comp_omega_factor_sq = true;
    b.explicit_exact_altitudes = Some(true);
    b.compressor = Some(CompressorData {
        alt: vec![300.0, 2600.0],
        power: vec![1310.0, 1240.0],
        ceil: vec![5000.0, 9000.0],
        ceil_pwr: vec![670.0, 510.0],
        rpm_ratio: vec![1.0, 1.0],
        boost: vec![1.0, 1.0],
        has_boost: Some(vec![true, true]),
        const_rpm_alt: Some(vec![18300.0, 18300.0]),
        const_rpm_power: Some(vec![1310.0, 1240.0]),
    });
    b.comp_afterburner_pressure_boost = Some(vec![0.0, 0.0]);
    b
}

/// spitfire_ix.blkx (fm) — Merlin 66: RPMNom=3000 > military=2850,
/// 触发 definition_alt_power_adjuster (含 deck/ceiling 调整与级联)
fn spitfire_ix() -> FmData {
    let mut b = FmData::default();
    b.comp_num_steps = 2;
    b.is_jet = false;
    b.military_rpm = 2850.0;
    b.wep_rpm = 3000.0;
    b.shaft_rpm_max = 0.0;
    b.rpm_nom = 3000.0;
    b.governor_max_param = 2999.0;
    b.military_mp = 1.81f32 as f64;
    b.wep_manifold_pressure = 2.22f32 as f64;
    b.aftb_coff = 1.28f32 as f64;
    b.throttle_boost = 1.0001f32 as f64;
    b.octane_afterburner_mult = 1.0;
    b.speed_to_manifold_multiplier = 0.65f32 as f64;
    b.deck_power = 1330.0;
    b.comp_pressure_at_rpm0 = 0.3f32 as f64;
    b.comp_omega_factor_sq = 1.0;
    b.has_comp_omega_factor_sq = true;
    b.explicit_exact_altitudes = Some(true);
    b.compressor = Some(CompressorData {
        alt: vec![3600.0, 6800.0],
        power: vec![1440.0, 1340.0],
        ceil: vec![10000.0, 9090.0],
        ceil_pwr: vec![500.0, 930.0],
        rpm_ratio: vec![0.5, 0.5],
        boost: vec![1.0f32 as f64, 0.97f32 as f64],
        has_boost: Some(vec![true, true]),
        const_rpm_alt: Some(vec![18034.6f32 as f64, -2000.0]),
        const_rpm_power: Some(vec![200.0, 950.0]),
    });
    b.comp_afterburner_pressure_boost = Some(vec![0.0, 0.0]);
    b
}

/// tempest_mkv.blkx (fm) — Sabre II: invertEnableLogic=true 机型 (150 辛烷为默认,
/// FM 本身已含 150 辛烷值), RPMMax=3701/RPMAfterburner=3701, GovernorMaxParam=3700
fn tempest_mkv() -> FmData {
    let mut b = FmData::default();
    b.comp_num_steps = 2;
    b.is_jet = false;
    // militaryRPM/wepRPM/governorMaxParam 在 Blkx.getload 中走 Double.parseDouble
    // (非 getdouble/Float.parseFloat), 整数值无拓宽差
    b.military_rpm = 3700.0;
    b.wep_rpm = 3701.0;
    b.shaft_rpm_max = 0.0;
    b.rpm_nom = 0.0;
    b.governor_max_param = 3700.0;
    b.military_mp = 1.477f32 as f64; // max(ATA0..2): 0.65/1.398/1.477
    b.wep_manifold_pressure = 1.817f32 as f64;
    b.aftb_coff = 1.235f32 as f64;
    b.throttle_boost = 1.001f32 as f64;
    b.octane_afterburner_mult = 1.0; // 文件缺失 → getdouble 0 → 回退 1.0
    b.speed_to_manifold_multiplier = 0.7f32 as f64;
    b.deck_power = 1995.0;
    b.comp_pressure_at_rpm0 = 0.3f32 as f64;
    b.comp_omega_factor_sq = 0.0; // 显式 0 (文件存在该键)
    b.has_comp_omega_factor_sq = true;
    b.explicit_exact_altitudes = Some(true);
    b.compressor = Some(CompressorData {
        alt: vec![1447.0, 4981.0],
        power: vec![2065.0, 1735.0],
        ceil: vec![1447.1f32 as f64, 9144.0],
        ceil_pwr: vec![2064.97f32 as f64, 1015.0],
        rpm_ratio: vec![0.5, 0.5],
        boost: vec![1.0, 1.0],
        has_boost: Some(vec![true, true]),
        const_rpm_alt: Some(vec![18093.2f32 as f64, 18093.2f32 as f64]),
        const_rpm_power: Some(vec![2001.08f32 as f64, 2001.08f32 as f64]),
    });
    b.comp_afterburner_pressure_boost = Some(vec![0.0, 0.0]); // 键缺失 → 0
    b
}

/// 中央文件文本 (types.rs 测试同款格式)
/// 内嵌中央 JSON → 燃油修正 (serde 解析; 常量合法 JSON, unwrap 恒成功)
fn fuel_mod_json(central: &str) -> crate::fm::data::FuelModification {
    let root: serde_json::Value = serde_json::from_str(central).unwrap();
    extract_fuel_modifications_json(&root)
}

const CENTRAL_SPITFIRE_F24: &str = "{\"modifications\": {\"150_octan_fuel\": {\"invertEnableLogic\": false, \"effects\": {\"afterburnerMult\": 1.42, \"afterburnerCompressorMult\": 1.33}}}}";
/// yak-3.json (flightmodels 根, 中央文件) — 苏联 B-100 油料
const CENTRAL_YAK3: &str =
    "{\"modifications\": {\"ussr_fuel_b-100\": {\"effects\": {\"addHorsePowers\": 50.0}}}}";
/// tempest_mkv.json (flightmodels 根, 中央文件) — invertEnableLogic=true
const CENTRAL_TEMPEST_MKV: &str = "{\"modifications\": {\"150_octan_fuel\": {\"invertEnableLogic\": true, \"effects\": {\"afterburnerMult\": 0.4167, \"afterburnerCompressorMult\": 0.411}}}}";

// ---- oracle: spitfire_f24 级参数 (无油料 + 150 辛烷) ----

#[test]
fn java8_oracle_spitfire_f24_stages() {
    let fmdata = spitfire_f24();
    let speed_mm = 0.8f32 as f64;

    // 无油料
    let stages = extract_stages(Some(&fmdata)).unwrap();
    assert_eq!(stages.len(), 2);
    assert_stage(
        "spit_nofuel[0]",
        &stages[0],
        &exp(
            4100.0,
            1510.0,
            1360.0,
            0.0,
            0.5,
            2204.0,
            1.4365986458951632,
            speed_mm,
            18034.599609375,
            200.0,
            10000.0,
            600.0,
            4100.0,
            1510.0,
            1510.0,
            -2090.0,
            0.0,
            0.0,
            0,
            true,
        ),
    );
    assert_stage(
        "spit_nofuel[1]",
        &stages[1],
        &exp(
            8100.0,
            1340.0,
            1088.0,
            0.0,
            0.5,
            6392.0,
            1.3939274392789431,
            speed_mm,
            18034.599609375,
            200.0,
            12000.0,
            830.0,
            8100.0,
            1340.0,
            1340.0,
            -2090.0,
            0.0,
            0.0,
            1,
            true,
        ),
    );

    // 150 辛烷 (invertEnableLogic=false → 应用加成, 仅 WEP 参数变化)
    let fuel = fuel_mod_json(CENTRAL_SPITFIRE_F24);
    assert_eq!(fuel.r#type, FuelType::British150Octane);
    let stages = extract_stages_with_fuel(Some(&fmdata), Some(&fuel)).unwrap();
    assert_stage(
        "spit_fuel[0]",
        &stages[0],
        &exp(
            4100.0,
            1510.0,
            1360.0,
            0.0,
            0.5,
            1502.0,
            1.6120470661360677,
            speed_mm,
            18034.599609375,
            200.0,
            10000.0,
            600.0,
            4100.0,
            1510.0,
            1510.0,
            -2090.0,
            0.0,
            0.0,
            0,
            true,
        ),
    );
    assert_stage(
        "spit_fuel[1]",
        &stages[1],
        &exp(
            8100.0,
            1340.0,
            1088.0,
            0.0,
            0.5,
            5760.0,
            1.5641645252254845,
            speed_mm,
            18034.599609375,
            200.0,
            12000.0,
            830.0,
            8100.0,
            1340.0,
            1340.0,
            -2090.0,
            0.0,
            0.0,
            1,
            true,
        ),
    );
}

// ---- oracle: spitfire_f24 功率曲线 (300 km/h IAS, 15C) + TestSpitfireF24Power 断言移植 ----

#[test]
fn java8_oracle_spitfire_f24_power_curve() {
    let fmdata = spitfire_f24();
    let fuel = fuel_mod_json(CENTRAL_SPITFIRE_F24);
    let stages = extract_stages_with_fuel(Some(&fmdata), Some(&fuel)).unwrap();

    // Java 实测值 (wtapc 参考表的相同高度点)
    let mil = [
        (0.0, 1_347.392_094_045_017),
        (1000.0, 1389.8180391263297),
        (1830.0, 1422.0049800128452),
        (2000.0, 1428.2754521790348),
        (3000.0, 1463.0547845674043),
        (4000.0, 1494.4314160977465),
        (4100.0, 1_497.392_094_045_017),
        (5000.0, 1419.5820703859804),
        (6000.0, 1281.0453572285796),
        (7000.0, 1304.3300257488868),
        (8000.0, 1325.1061795411342),
        (8100.0, 1327.0541081198485),
        (9000.0, 1309.5830015928336),
        (10000.0, 1170.6209315511492),
    ];
    let wep = [
        (0.0, 2172.0594721402026),
        (1000.0, 2240.4520924365825),
        (1830.0, 2292.3389560605847),
        (2000.0, 2252.2040537265793),
        (3000.0, 2020.2184732230976),
        (4000.0, 1917.7207364971378),
        (4100.0, 1922.4758689193295),
        (5000.0, 1963.0683450432875),
        (6000.0, 2003.7657029817524),
        (7000.0, 1884.3087499654412),
        (8000.0, 1718.4280598352361),
        (8100.0, 1702.8754344459285),
        (9000.0, 1565.2725233772742),
        (10000.0, 1408.8425620388034),
    ];
    for (alt, expected) in mil {
        check(
            &format!("spit mil@{alt:.0}"),
            optimal_power_advanced(&stages, alt, false, 300.0, true, 15.0),
            expected,
        );
    }
    for (alt, expected) in wep {
        check(
            &format!("spit wep@{alt:.0}"),
            optimal_power_advanced(&stages, alt, true, 300.0, true, 15.0),
            expected,
        );
    }

    // 峰值: Java oracle 精确对拍
    let mut mil_peak = 0.0f64;
    let mut wep_peak = 0.0f64;
    // PORT: Java `for (int alt = 0; alt <= 10000; alt += 50)` int 步进循环
    for alt in (0..=10000i32).step_by(50) {
        let alt_f = alt as f64;
        let m = optimal_power_advanced(&stages, alt_f, false, 300.0, true, 15.0);
        let w = optimal_power_advanced(&stages, alt_f, true, 300.0, true, 15.0);
        if m > mil_peak {
            mil_peak = m;
        }
        if w > wep_peak {
            wep_peak = w;
        }
    }
    check("spit milPeak", mil_peak, 1_508.925_763_857_947);
    check("spit wepPeak", wep_peak, 2290.5371881238357);

    // TestSpitfireF24Power.testPowerCurveCalculations 验收断言移植:
    // assertClose("Military peak power", milPeakPower, 1510.0, 50.0);
    // assertClose("WEP peak power", wepPeakPower, 2292.5, 100.0);
    assert!(
        (mil_peak - 1510.0).abs() <= 50.0,
        "Military peak power vs wtapc 1510"
    );
    assert!(
        (wep_peak - 2292.5).abs() <= 100.0,
        "WEP peak power vs wtapc 2292.5"
    );
}

// ---- oracle: yak-3 苏联 B-100 油料 (soviet_octane_adder, spm=1.018) ----

#[test]
fn java8_oracle_yak3_soviet_fuel() {
    let fmdata = yak3();
    let fuel = fuel_mod_json(CENTRAL_YAK3);
    assert_eq!(fuel.r#type, FuelType::SovietB100);
    assert_eq!(fuel.soviet_octane_hp_bonus, 50.0);

    // 无油料: 无 WEP 机型 (aftbCoff=1 → wepMult=1, WEP 曲线与军用一致)
    let stages = extract_stages(Some(&fmdata)).unwrap();
    assert_stage(
        "yak3_nofuel[0]",
        &stages[0],
        &exp(
            300.0, 1310.0, 1290.0, 0.0, 1.0, 300.0, 1.0, 1.0, 18300.0, 1310.0, 5000.0, 670.0,
            300.0, 1310.0, 1310.0, 0.0, 0.0, 0.0, 0, true,
        ),
    );
    assert_stage(
        "yak3_nofuel[1]",
        &stages[1],
        &exp(
            2600.0, 1240.0, 1032.0, 0.0, 1.0, 2600.0, 1.0, 1.0, 18300.0, 1240.0, 9000.0, 510.0,
            2600.0, 1240.0, 1240.0, 0.0, 0.0, 0.0, 1, true,
        ),
    );

    // B-100 (addHorsePowers=50): 全功率值 ×1.018
    let stages = extract_stages_with_fuel(Some(&fmdata), Some(&fuel)).unwrap();
    assert_stage(
        "yak3_fuel[0]",
        &stages[0],
        &exp(
            300.0,
            1333.58,
            1313.22,
            0.0,
            1.0,
            300.0,
            1.0,
            1.0,
            18300.0,
            1333.58,
            5000.0,
            682.060_000_000_000_1,
            300.0,
            1333.58,
            1333.58,
            0.0,
            0.0,
            0.0,
            0,
            true,
        ),
    );
    assert_stage(
        "yak3_fuel[1]",
        &stages[1],
        &exp(
            2600.0,
            1262.32,
            1050.576,
            0.0,
            1.0,
            2600.0,
            1.0,
            1.0,
            18300.0,
            1262.32,
            9000.0,
            519.180_000_000_000_1,
            2600.0,
            1262.32,
            1262.32,
            0.0,
            0.0,
            0.0,
            1,
            true,
        ),
    );
}

// ---- oracle: spitfire_ix RPM 调整 (definition_alt_power_adjuster) ----

#[test]
fn java8_oracle_spitfire_ix_rpm_adjuster() {
    let fmdata = spitfire_ix();
    let speed_mm = 0.65f32 as f64;
    let deck_alt_adj = -614.535_722_854_266_6;

    let stages = extract_stages(Some(&fmdata)).unwrap();
    assert_stage(
        "spix_nofuel[0]",
        &stages[0],
        &exp(
            3035.0,
            1414.9355361480882,
            1297.5481529514284,
            deck_alt_adj,
            0.5,
            1986.0,
            1.2894766986926651,
            speed_mm,
            18034.599609375,
            198.55,
            9524.0,
            500.0,
            3600.0,
            1440.0,
            1429.5600000000002,
            -1756.0,
            0.0,
            deck_alt_adj,
            0,
            true,
        ),
    );
    // stage1: constRpmAlt=-2000 (power ≠ oldPower → 走 /rpmBoost 分支), deckPower 级联 minDeck
    assert_stage(
        "spix_nofuel[1]",
        &stages[1],
        &exp(
            6280.0,
            1_317.959_589_650_867,
            1072.0,
            deck_alt_adj,
            0.5,
            5314.0,
            1.2507924346241095,
            speed_mm,
            -2000.0,
            943.112_500_000_000_1,
            8601.0,
            930.0,
            6800.0,
            1340.0,
            1_330.285,
            -1756.0,
            0.0,
            deck_alt_adj,
            1,
            true,
        ),
    );

    // 150 辛烷 (abm=1.75, abcm=2.14): 在已调整的 WEP 参数上后处理
    let fuel = FuelModification {
        british_afterburner_mult: 1.75,
        british_afterburner_compressor_mult: 2.14,
        british_invert_logic: false,
        r#type: FuelType::British150Octane,
        ..Default::default()
    };
    let stages = extract_stages_with_fuel(Some(&fmdata), Some(&fuel)).unwrap();
    assert_stage(
        "spix_fuel[0]",
        &stages[0],
        &exp(
            3035.0,
            1414.9355361480882,
            1297.5481529514284,
            deck_alt_adj,
            0.5,
            419.0,
            1.501_031_452_684_01,
            speed_mm,
            18034.599609375,
            198.55,
            9524.0,
            500.0,
            3600.0,
            1440.0,
            1429.5600000000002,
            -1756.0,
            0.0,
            deck_alt_adj,
            0,
            true,
        ),
    );
    assert_stage(
        "spix_fuel[1]",
        &stages[1],
        &exp(
            6280.0,
            1_317.959_589_650_867,
            1072.0,
            deck_alt_adj,
            0.5,
            3869.0,
            1.456_000_552_048_344,
            speed_mm,
            -2000.0,
            943.112_500_000_000_1,
            8601.0,
            930.0,
            6800.0,
            1340.0,
            1_330.285,
            -1756.0,
            0.0,
            deck_alt_adj,
            1,
            true,
        ),
    );
}

// ---- oracle: tempest_mkv (invertEnableLogic=true 机型, 无 RPM 调整路径) ----

#[test]
fn java8_oracle_tempest_mkv_stages() {
    let fmdata = tempest_mkv();
    let speed_mm = 0.7f32 as f64;

    let stages = extract_stages(Some(&fmdata)).unwrap();
    assert_eq!(stages.len(), 2);
    assert_stage(
        "temp_nofuel[0]",
        &stages[0],
        &exp(
            1447.0,
            2065.0,
            1995.0,
            0.0,
            0.5,
            -276.0,
            1.2362353427420838,
            speed_mm,
            18_093.199_218_75,
            2001.0799560546875,
            1447.0999755859375,
            2_064.969_970_703_125,
            1447.0,
            2065.0,
            2065.0,
            -1781.0,
            0.0,
            0.0,
            0,
            true,
        ),
    );
    assert_stage(
        "temp_nofuel[1]",
        &stages[1],
        &exp(
            4981.0,
            1735.0,
            1596.0,
            0.0,
            0.5,
            3400.0,
            1.2362353427420838,
            speed_mm,
            18_093.199_218_75,
            2001.0799560546875,
            9144.0,
            1015.0,
            4981.0,
            1735.0,
            1735.0,
            -1781.0,
            0.0,
            0.0,
            1,
            true,
        ),
    );

    // 150 辛烷 invertEnableLogic=true → 不加成 (与无油料完全一致)
    let fuel = fuel_mod_json(CENTRAL_TEMPEST_MKV);
    assert_eq!(fuel.r#type, FuelType::British150Octane);
    assert!(fuel.british_invert_logic);
    let stages = extract_stages_with_fuel(Some(&fmdata), Some(&fuel)).unwrap();
    assert_stage(
        "temp_fuel[0]",
        &stages[0],
        &exp(
            1447.0,
            2065.0,
            1995.0,
            0.0,
            0.5,
            -276.0,
            1.2362353427420838,
            speed_mm,
            18_093.199_218_75,
            2001.0799560546875,
            1447.0999755859375,
            2_064.969_970_703_125,
            1447.0,
            2065.0,
            2065.0,
            -1781.0,
            0.0,
            0.0,
            0,
            true,
        ),
    );
    assert_stage(
        "temp_fuel[1]",
        &stages[1],
        &exp(
            4981.0,
            1735.0,
            1596.0,
            0.0,
            0.5,
            3400.0,
            1.2362353427420838,
            speed_mm,
            18_093.199_218_75,
            2001.0799560546875,
            9144.0,
            1015.0,
            4981.0,
            1735.0,
            1735.0,
            -1781.0,
            0.0,
            0.0,
            1,
            true,
        ),
    );
}

// ---- oracle: tempest_mkv 功率曲线 (300 km/h IAS, 15C) + TestTempestMk5Power 断言移植 ----

#[test]
fn java8_oracle_tempest_mkv_power_curve() {
    let fmdata = tempest_mkv();
    let stages = extract_stages(Some(&fmdata)).unwrap();

    // Java 实测值 (invert=true 机型, 油料不改变结果, 与 Java 测试同用无油料级)
    let mil = [
        (0.0, 1982.1485424919429),
        (1000.0, 2_031.571_975_675_931),
        (1730.0, 2_064.712_288_887_363),
        (2000.0, 2001.0719318704785),
        (3000.0, 1773.3185985020484),
        (4000.0, 1704.1738213842752),
        (5000.0, 1726.6303467845094),
        (6000.0, 1615.3744254417463),
        (7000.0, 1432.2817673942739),
        (8000.0, 1_268.914_132_656_07),
        (9000.0, 1123.6030029761318),
        (10000.0, 994.780_295_170_574_6),
    ];
    let wep = [
        (0.0, 2441.076377387389),
        (1000.0, 2222.94701594032),
        (1730.0, 2076.682887909097),
        (2000.0, 2041.7125330963042),
        (3000.0, 2075.909061190071),
        (4000.0, 2046.7054138147478),
        (5000.0, 1845.3496781561112),
        (6000.0, 1650.074489978947),
        (7000.0, 1466.0583319832253),
        (8000.0, 1301.866688222294),
        (9000.0, 1155.8226246160557),
        (10000.0, 1026.3501487353587),
    ];
    for (alt, expected) in mil {
        check(
            &format!("temp mil@{alt:.0}"),
            optimal_power_advanced(&stages, alt, false, 300.0, true, 15.0),
            expected,
        );
    }
    for (alt, expected) in wep {
        check(
            &format!("temp wep@{alt:.0}"),
            optimal_power_advanced(&stages, alt, true, 300.0, true, 15.0),
            expected,
        );
    }

    // 50 m 步进峰值: Java oracle 精确对拍 (1730 非步进点, 峰在 1700)
    let mut mil_peak = 0.0f64;
    let mut wep_peak = 0.0f64;
    // PORT: Java `for (int alt = 0; alt <= 10000; alt += 50)` int 步进循环
    for alt in (0..=10000i32).step_by(50) {
        let alt_f = alt as f64;
        let m = optimal_power_advanced(&stages, alt_f, false, 300.0, true, 15.0);
        let w = optimal_power_advanced(&stages, alt_f, true, 300.0, true, 15.0);
        if m > mil_peak {
            mil_peak = m;
        }
        if w > wep_peak {
            wep_peak = w;
        }
    }
    check("temp milPeak", mil_peak, 2_063.397_170_779_843);
    check("temp wepPeak", wep_peak, 2_441.076_377_387_389);
}

// ---- oracle: synthetic 分支 (Java 直接设 public 字段构造, 双精度字面量, 无 f32 拓宽) ----

#[test]
fn java8_oracle_synthetic_branches() {
    // syn1: AfterburnerBoostMul1=0 显式禁 WEP + deckPower=0 走 0.8*compPower[0]
    let mut syn1 = FmData::default();
    syn1.comp_num_steps = 2;
    syn1.compressor = Some(CompressorData {
        alt: vec![4100.0, 8100.0],
        power: vec![1510.0, 1340.0],
        ceil: vec![10000.0, 12000.0],
        ceil_pwr: vec![600.0, 830.0],
        rpm_ratio: vec![0.5, 0.5],
        boost: vec![0.9, 0.0],
        has_boost: Some(vec![true, true]),
        ..Default::default()
    });
    syn1.military_rpm = 2600.0;
    syn1.wep_rpm = 2750.0;
    syn1.military_mp = 1.61;
    syn1.wep_manifold_pressure = 2.22;
    syn1.aftb_coff = 1.41;
    syn1.throttle_boost = 1.0;
    syn1.octane_afterburner_mult = 1.0;
    syn1.speed_to_manifold_multiplier = 0.8;
    syn1.deck_power = 0.0;
    syn1.comp_pressure_at_rpm0 = 0.3;
    syn1.comp_omega_factor_sq = 1.0;
    syn1.has_comp_omega_factor_sq = true;
    syn1.explicit_exact_altitudes = Some(true);
    let stages = extract_stages(Some(&syn1)).unwrap();
    assert_stage(
        "syn1[0]",
        &stages[0],
        &exp(
            4100.0,
            1510.0,
            1208.0,
            0.0,
            0.5,
            2204.0,
            1.2800094274420408,
            0.8,
            0.0,
            0.0,
            10000.0,
            600.0,
            4100.0,
            1510.0,
            1510.0,
            -2090.0,
            0.0,
            0.0,
            0,
            true,
        ),
    );
    // stage1: WEP 禁用 (wepMult=1, wepCritAlt=critAlt, wepDeckAlt=0)
    assert_stage(
        "syn1[1]",
        &stages[1],
        &exp(
            8100.0, 1340.0, 1072.0, 0.0, 0.5, 8100.0, 1.0, 0.8, 0.0, 0.0, 12000.0, 830.0, 8100.0,
            1340.0, 1340.0, 0.0, 0.0, 0.0, 1, true,
        ),
    );

    // syn2: 旧格式 (无 OmegaFactorSq) + ShaftRPMMax 优先 + ConstRPM 调整
    // (hasBoost 缺席 → None, 消费方 is_some_and=false)
    let mut syn2 = FmData::default();
    syn2.comp_num_steps = 1;
    syn2.compressor = Some(CompressorData {
        alt: vec![5000.0],
        power: vec![1500.0],
        ceil: vec![10000.0],
        ceil_pwr: vec![700.0],
        rpm_ratio: vec![1.2],
        boost: vec![1.05],
        const_rpm_alt: Some(vec![1000.0]),
        const_rpm_power: Some(vec![1450.0]),
        ..Default::default()
    });
    syn2.military_rpm = 2400.0;
    syn2.wep_rpm = 2700.0;
    syn2.shaft_rpm_max = 2695.0;
    syn2.rpm_nom = 2600.0;
    syn2.governor_max_param = 2500.0;
    syn2.military_mp = 1.42;
    syn2.wep_manifold_pressure = 1.65;
    syn2.aftb_coff = 1.15;
    syn2.throttle_boost = 1.0;
    syn2.octane_afterburner_mult = 1.0;
    syn2.speed_to_manifold_multiplier = 0.9;
    syn2.deck_power = 1400.0;
    syn2.comp_pressure_at_rpm0 = 0.2;
    syn2.comp_omega_factor_sq = 0.0;
    syn2.has_comp_omega_factor_sq = false;
    syn2.explicit_exact_altitudes = None; // → !hasCompOmegaFactorSq = true
    let stages = extract_stages(Some(&syn2)).unwrap();
    assert_stage(
        "syn2[0]",
        &stages[0],
        &exp(
            3571.0,
            1427.2405762633953,
            1_310.624_962_826_641,
            -1610.7846991254821,
            1.2,
            3884.0,
            1.250_379_971_590_909,
            0.9,
            1000.0,
            1401.6821765265818,
            8753.0,
            700.0,
            5000.0,
            1500.0,
            1450.0160446826708,
            -1258.0,
            0.0,
            -1610.7846991254821,
            0,
            true,
        ),
    );

    // syn3: militaryMP=0 → wepCritAlt 走 critAlt*0.9 前需先过 mult≈1 早退 (此处 mult=1)
    let mut syn3_base = FmData::default();
    syn3_base.comp_num_steps = 1;
    syn3_base.compressor = Some(CompressorData {
        alt: vec![3000.0],
        power: vec![1200.0],
        ceil: vec![8000.0],
        ceil_pwr: vec![500.0],
        rpm_ratio: vec![0.0],
        boost: vec![1.0],
        ..Default::default()
    });
    syn3_base.military_rpm = 2600.0;
    syn3_base.wep_rpm = 2600.0;
    syn3_base.military_mp = 0.0;
    syn3_base.wep_manifold_pressure = 0.0;
    syn3_base.aftb_coff = 1.0;
    syn3_base.throttle_boost = 1.0;
    syn3_base.octane_afterburner_mult = 1.0;
    syn3_base.speed_to_manifold_multiplier = 0.95;
    syn3_base.deck_power = 1180.0;
    syn3_base.comp_pressure_at_rpm0 = 0.25;
    syn3_base.comp_omega_factor_sq = 0.5;
    syn3_base.has_comp_omega_factor_sq = true;
    syn3_base.explicit_exact_altitudes = None;
    let syn3_exp = exp(
        3000.0, 1200.0, 1180.0, 0.0, 1.0, 3000.0, 1.0, 0.95, 0.0, 0.0, 8000.0, 500.0, 3000.0,
        1200.0, 1200.0, 0.0, 0.0, 0.0, 0, false,
    );
    let stages = extract_stages(Some(&syn3_base)).unwrap();
    assert_stage("syn3[0]", &stages[0], &syn3_exp);

    // syn3b: militaryMP>0 但 mult≈1 → wepCritAlt=critAlt / wepDeckAlt=deckAlt
    let mut syn3b = syn3_base.clone();
    syn3b.military_mp = 1.35;
    syn3b.wep_manifold_pressure = 1.5;
    let stages = extract_stages(Some(&syn3b)).unwrap();
    assert_stage("syn3b[0]", &stages[0], &syn3_exp);

    // syn4: 苏联油 — bonus≠50 → spm=1.0 (与 syn3b 同); bonus=50 → ×1.018
    let sov30 = FuelModification {
        soviet_octane_hp_bonus: 30.0,
        r#type: FuelType::SovietB95,
        ..Default::default()
    };
    let stages = extract_stages_with_fuel(Some(&syn3b), Some(&sov30)).unwrap();
    assert_stage("syn4_sov30[0]", &stages[0], &syn3_exp);
    let sov50 = FuelModification {
        soviet_octane_hp_bonus: 50.0,
        r#type: FuelType::SovietB100,
        ..Default::default()
    };
    let stages = extract_stages_with_fuel(Some(&syn3b), Some(&sov50)).unwrap();
    assert_stage(
        "syn4_sov50[0]",
        &stages[0],
        &exp(
            3000.0, 1_221.6, 1_201.24, 0.0, 1.0, 3000.0, 1.0, 0.95, 0.0, 0.0, 8000.0, 509.0,
            3000.0, 1_221.6, 1_221.6, 0.0, 0.0, 0.0, 0, false,
        ),
    );

    // syn5: 英国油 invertEnableLogic=true → 不加成 (与 syn3b 相同)
    let inv = FuelModification {
        british_afterburner_mult: 1.3,
        british_afterburner_compressor_mult: 1.2,
        british_invert_logic: true,
        r#type: FuelType::British100Spitfire,
        ..Default::default()
    };
    let stages = extract_stages_with_fuel(Some(&syn3b), Some(&inv)).unwrap();
    assert_stage("syn5_inv[0]", &stages[0], &syn3_exp);

    // syn6: 显式 ExactAltitudes=false + ConstRPM → wepConstRpmAlt 分支 +
    //       AfterburnerPressureBoost>0 + compConstRpm 数组短于级数 (i<len 守卫)
    let mut syn6 = FmData::default();
    syn6.comp_num_steps = 2;
    syn6.compressor = Some(CompressorData {
        alt: vec![5000.0, 8000.0],
        power: vec![1500.0, 1300.0],
        ceil: vec![10000.0, 11000.0],
        ceil_pwr: vec![700.0, 600.0],
        rpm_ratio: vec![1.0, 1.0],
        boost: vec![1.05, 1.02],
        has_boost: Some(vec![false, false]),
        // 数组短于级数 (1 < 2) — 验证 i<len 守卫
        const_rpm_alt: Some(vec![1200.0]),
        const_rpm_power: Some(vec![1450.0]),
    });
    syn6.military_rpm = 2400.0;
    syn6.wep_rpm = 2600.0;
    syn6.military_mp = 1.42;
    syn6.wep_manifold_pressure = 1.65;
    syn6.aftb_coff = 1.15;
    syn6.throttle_boost = 1.0;
    syn6.octane_afterburner_mult = 1.0;
    syn6.speed_to_manifold_multiplier = 0.9;
    syn6.deck_power = 1400.0;
    syn6.comp_pressure_at_rpm0 = 0.2;
    syn6.comp_omega_factor_sq = 0.1;
    syn6.has_comp_omega_factor_sq = true;
    syn6.explicit_exact_altitudes = Some(false);
    syn6.comp_afterburner_pressure_boost = Some(vec![1.08, 1.05]);
    let stages = extract_stages(Some(&syn6)).unwrap();
    assert_stage(
        "syn6[0]",
        &stages[0],
        &exp(
            5000.0,
            1500.0,
            1400.0,
            0.0,
            1.0,
            4984.0,
            1.2281840277777778,
            0.9,
            1200.0,
            1450.0,
            10000.0,
            700.0,
            5000.0,
            1500.0,
            1500.0,
            -18.0,
            1_182.229_918_848_382,
            0.0,
            0,
            false,
        ),
    );
    assert_stage(
        "syn6[1]",
        &stages[1],
        &exp(
            8000.0,
            1300.0,
            1120.0,
            0.0,
            1.0,
            7790.0,
            1.1930930555555554,
            0.9,
            0.0,
            0.0,
            11000.0,
            600.0,
            8000.0,
            1300.0,
            1300.0,
            -257.0,
            0.0,
            0.0,
            1,
            false,
        ),
    );
}

// ---- oracle: null / 守卫边界 ----

#[test]
fn java8_oracle_null_and_guard_boundaries() {
    assert!(!is_piston_engine(None));
    assert_eq!(get_wep_boost_factor(None), 1.0);
    assert_eq!(get_speed_manifold_multiplier(None), 1.0);
    assert!(extract_stages(None).is_none());
    assert!(extract_stages_with_fuel(None, None).is_none());

    // compNumSteps<=0 → null
    let zero_steps = FmData::default(); // comp_num_steps 默认 0
    assert!(!is_piston_engine(Some(&zero_steps)));
    assert!(extract_stages(Some(&zero_steps)).is_none());

    // isJet → false
    let mut jet = FmData::default();
    jet.comp_num_steps = 1;
    jet.is_jet = true;
    assert!(!is_piston_engine(Some(&jet)));

    // 有数据时工具函数直读 (Java spitfire 实测: wepBoost=1.41(f32) speedMM=0.8(f32))
    let fmdata = spitfire_f24();
    assert!(is_piston_engine(Some(&fmdata)));
    check(
        "spit wepBoost",
        get_wep_boost_factor(Some(&fmdata)),
        1.409_999_966_621_399,
    );
    check(
        "spit speedMM",
        get_speed_manifold_multiplier(Some(&fmdata)),
        0.800_000_011_920_929,
    );
}

// ---- TestSpitfireF24Power.testParameterExtraction 断言移植 (fixture 自检) ----

#[test]
fn java_test_port_parameter_extraction() {
    let fmdata = spitfire_f24();
    // assertClose(name, actual, expected, tolerance) — Java 断言逐条
    assert_eq!(fmdata.comp_num_steps as f64, 2.0, "compressor NumSteps");
    let comp = fmdata.compressor.as_ref().unwrap();
    assert!((comp.alt[0] - 4100.0).abs() <= 0.0, "Stage 0 altitude");
    assert!((comp.alt[1] - 8100.0).abs() <= 0.0, "Stage 1 altitude");
    assert!((comp.power[0] - 1510.0).abs() <= 0.0, "Stage 0 power");
    assert!((comp.power[1] - 1340.0).abs() <= 0.0, "Stage 1 power");
    assert!((fmdata.aftb_coff - 1.41).abs() <= 0.01, "AfterburnerBoost");
    assert!(
        (fmdata.wep_manifold_pressure - 2.22).abs() <= 0.01,
        "AfterburnerManifoldPressure"
    );
    assert!(
        (fmdata.speed_to_manifold_multiplier - 0.8).abs() <= 0.01,
        "SpeedManifoldMultiplier"
    );

    // assertNotNull + has 2 stages
    let stages = extract_stages(Some(&fmdata));
    assert!(stages.is_some(), "extracted stages without fuel");
    let stages = stages.unwrap();
    assert_eq!(stages.len(), 2, "has 2 stages");
}

// ---- TestSpitfireF24Power.testInvertEnableLogicBehavior 断言移植 ----

#[test]
fn java_test_port_invert_enable_logic_behavior() {
    let fmdata = spitfire_f24();
    let fuel = fuel_mod_json(CENTRAL_SPITFIRE_F24);
    // fuelMod != null 由类型系统保证; Java 的 `fuelMod == null` SKIP 分支不移植

    // Since invertEnableLogic is FALSE for Spitfire F24:
    // - The modification represents ADDING 150 octane fuel
    // - WEP parameters SHOULD be boosted when fuel is applied
    let stages_no_fuel = extract_stages(Some(&fmdata)).unwrap();
    let stages_with_fuel = extract_stages_with_fuel(Some(&fmdata), Some(&fuel)).unwrap();

    // With invertEnableLogic=false, fuel mod SHOULD change WEP params
    let mut wep_changed = false;
    for i in 0..stages_no_fuel.len() {
        let no_fuel_mult = stages_no_fuel[i].wep_power_mult;
        let with_fuel_mult = stages_with_fuel[i].wep_power_mult;
        let no_fuel_wep_alt = stages_no_fuel[i].wep_crit_alt;
        let with_fuel_wep_alt = stages_with_fuel[i].wep_crit_alt;

        if (no_fuel_mult - with_fuel_mult).abs() > 0.001
            || (no_fuel_wep_alt - with_fuel_wep_alt).abs() > 1.0
        {
            wep_changed = true;
        }
    }
    assert!(
        wep_changed,
        "WEP parameters change with fuel (invertEnableLogic=false)"
    );
}

// ---- TestTempestMk5Power 断言移植 (fixture 自检, wtapc 参考值) ----

/// TestTempestMk5Power.testInvertEnableLogicDetection + testFuelModificationBehavior +
/// testParameterExtraction 断言逐条移植
#[test]
fn java_test_port_tempest_invert_enable_logic() {
    let fmdata = tempest_mkv();
    let fuel = fuel_mod_json(CENTRAL_TEMPEST_MKV);

    // fuelMod != null 由类型系统保证; Java 的 SKIP 分支不移植

    // Tempest Mk V has invertEnableLogic:b = true (150 octane is default)
    assert_eq!(
        fuel.r#type,
        FuelType::British150Octane,
        "detected British 150 octane fuel"
    );
    assert!(
        fuel.british_invert_logic,
        "invertEnableLogic is true (150 octane is default)"
    );

    // With invertEnableLogic=true, fuel mod should NOT change WEP params
    let stages_no_fuel = extract_stages(Some(&fmdata)).unwrap();
    let stages_with_fuel = extract_stages_with_fuel(Some(&fmdata), Some(&fuel)).unwrap();
    let mut wep_unchanged = true;
    for i in 0..stages_no_fuel.len() {
        let no_fuel_mult = stages_no_fuel[i].wep_power_mult;
        let with_fuel_mult = stages_with_fuel[i].wep_power_mult;
        let no_fuel_wep_alt = stages_no_fuel[i].wep_crit_alt;
        let with_fuel_wep_alt = stages_with_fuel[i].wep_crit_alt;

        if (no_fuel_mult - with_fuel_mult).abs() > 0.001
            || (no_fuel_wep_alt - with_fuel_wep_alt).abs() > 1.0
        {
            wep_unchanged = false;
        }
    }
    assert!(
        wep_unchanged,
        "WEP parameters unchanged (invertEnableLogic=true means no bonus)"
    );

    // assertClose(name, actual, expected, tolerance) — Java 断言逐条
    assert_eq!(fmdata.comp_num_steps as f64, 2.0, "compressor NumSteps");
    let comp_alt = &fmdata.compressor.as_ref().unwrap().alt;
    // 期望值须跟随游戏 FM 数据版本更新 (WT 2.57.1.103 中 Altitude0 已从 1730 → 1447)
    assert!((comp_alt[0] - 1447.0).abs() <= 50.0, "Stage 0 altitude");
    assert!((comp_alt[1] - 5000.0).abs() <= 200.0, "Stage 1 altitude");
    assert_eq!(stages_no_fuel.len(), 2, "has 2 stages");
}

/// TestTempestMk5Power.testPowerCurveMilitary + testPowerCurveWEP 断言逐条移植
/// (wtapc 参考表, 300 km/h IAS, 15C)
#[test]
fn java_test_port_tempest_power_curve() {
    let fmdata = tempest_mkv();
    let stages = extract_stages(Some(&fmdata)).unwrap();

    let wtapc_mil = [
        (0.0, 1982.4),
        (1000.0, 2031.5),
        (1730.0, 2064.7),
        (2000.0, 2001.8),
        (3000.0, 1773.7),
        (4000.0, 1704.3),
        (5000.0, 1726.7),
        (6000.0, 1615.6),
        (7000.0, 1432.2),
        (8000.0, 1269.0),
        (9000.0, 1124.1),
        (10000.0, 994.2),
    ];
    let wtapc_wep = [
        (0.0, 2439.9),
        (1000.0, 2223.0),
        (2000.0, 2041.6),
        (3000.0, 2075.9),
        (4000.0, 2045.9),
        (5000.0, 1844.6),
        (6000.0, 1650.3),
        (7000.0, 1466.0),
        (8000.0, 1302.0),
        (9000.0, 1156.3),
        (10000.0, 1025.8),
    ];

    let mut mil_max_err = 0.0f64;
    for (alt, expected) in wtapc_mil {
        let actual = optimal_power_advanced(&stages, alt, false, 300.0, true, 15.0);
        mil_max_err = mil_max_err.max((actual - expected).abs());
    }
    assert!(
        mil_max_err < 5.0,
        "Military max error < 5 hp (was {mil_max_err})"
    );

    let mut wep_max_err = 0.0f64;
    for (alt, expected) in wtapc_wep {
        let actual = optimal_power_advanced(&stages, alt, true, 300.0, true, 15.0);
        wep_max_err = wep_max_err.max((actual - expected).abs());
    }
    assert!(
        wep_max_err < 10.0,
        "WEP max error < 10 hp (was {wep_max_err})"
    );

    // 峰值: Java `for (int alt = 0; alt <= 10000; alt += 50)` 同步进步进
    let mut wep_peak = 0.0f64;
    let mut wep_peak_alt = 0.0f64;
    for alt in (0..=10000i32).step_by(50) {
        let w = optimal_power_advanced(&stages, alt as f64, true, 300.0, true, 15.0);
        if w > wep_peak {
            wep_peak = w;
            wep_peak_alt = alt as f64;
        }
    }
    assert!(
        (wep_peak - 2439.9).abs() <= 10.0,
        "WEP peak power vs wtapc 2439.9"
    );
    assert!(
        (wep_peak_alt - 0.0).abs() <= 100.0,
        "WEP peak altitude vs wtapc 0"
    );
}
