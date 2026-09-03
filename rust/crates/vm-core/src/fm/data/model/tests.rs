use super::*;
use crate::fm::data::types::{FmParts, SweepLevel};

/// 基线 场景: 3 档可变翼 (sweep 0/0.5/1) + 静态守卫字段 (与 BlkxModelOracle 一致)
fn oracle_sweep_fmdata() -> FmData {
    let mut b = FmData::default();
    let sweeps = [0.0, 0.5, 1.0];
    let vnes = [800.0, 700.0, 600.0];
    let machs = [0.85, 0.82, 0.80];
    let aoa_highs = [16.0, 18.0, 20.0];
    let aoa_lows = [-16.0, -18.0, -20.0];
    let mut levels = Vec::with_capacity(3);
    for i in 0..3 {
        let mut l = SweepLevel::default();
        l.sweep = sweeps[i];
        l.vne = vnes[i];
        l.vne_mach = machs[i];
        let mut nf = FmParts::default();
        nf.aoa_crit_high = aoa_highs[i];
        nf.aoa_crit_low = aoa_lows[i];
        l.no_flaps = Some(nf);
        levels.push(l);
    }
    b.sweep_levels = Some(levels);
    b.vne = 999.5;
    b.vne_mach = 1.75;
    let mut nf = FmParts::default();
    nf.aoa_crit_high = 16.5;
    nf.aoa_crit_low = -16.5;
    b.no_flaps_wing = Some(nf);
    let mut ff = FmParts::default();
    ff.aoa_crit_high = 22.5;
    b.full_flaps_wing = Some(ff);
    b
}

/// 基线: vne_025/000/050/120/nan — 区间插值/下界/节点/上界钳位/NaN 落末档
#[test]
fn java8_oracle_get_vne_v_wing() {
    let b = oracle_sweep_fmdata();
    assert_eq!(
        b.get_vne_v_wing(0.25).to_bits(),
        0x4087_7000_0000_0000,
        "vne_025=750"
    );
    assert_eq!(
        b.get_vne_v_wing(0.0).to_bits(),
        0x4089_0000_0000_0000,
        "vne_000=800"
    );
    assert_eq!(
        b.get_vne_v_wing(0.5).to_bits(),
        0x4085_E000_0000_0000,
        "vne_050=700"
    );
    assert_eq!(
        b.get_vne_v_wing(1.2).to_bits(),
        0x4082_C000_0000_0000,
        "vne_120=600"
    );
    assert_eq!(
        b.get_vne_v_wing(f64::NAN).to_bits(),
        0x4082_C000_0000_0000,
        "vne_nan → 末档 (NaN 比较恒 false)"
    );
}

/// 基线: mne_025/150 — Mach 限值插值与上界钳位
#[test]
fn java8_oracle_get_mne_v_wing() {
    let b = oracle_sweep_fmdata();
    assert_eq!(
        b.get_mne_v_wing(0.25).to_bits(),
        0x3FEA_B851_EB85_1EB8,
        "mne_025=0.835"
    );
    assert_eq!(
        b.get_mne_v_wing(1.5).to_bits(),
        0x3FE9_9999_9999_999A,
        "mne_150=0.8"
    );
}

/// 基线: aoah_v0_* / aoah_v025 / aoah_v075 — vwing==0 襟翼混合 + 档位插值
#[test]
fn java8_oracle_get_aoa_high_v_wing() {
    let b = oracle_sweep_fmdata();
    // vwing==0: 16.5 + (22.5-16.5)*flaps/100
    assert_eq!(
        b.get_aoa_high_v_wing(0.0, 50).to_bits(),
        0x4033_8000_0000_0000,
        "f50=19.5"
    );
    assert_eq!(
        b.get_aoa_high_v_wing(0.0, 0).to_bits(),
        0x4030_8000_0000_0000,
        "f0=16.5"
    );
    assert_eq!(
        b.get_aoa_high_v_wing(0.0, 100).to_bits(),
        0x4036_8000_0000_0000,
        "f100=22.5"
    );
    assert_eq!(
        b.get_aoa_high_v_wing(0.0, -25).to_bits(),
        0x402E_0000_0000_0000,
        "f-25=15.0"
    );
    // vwing!=0: noFlaps.AoACritHigh 档位插值 (flaps 参数不参与)
    assert_eq!(
        b.get_aoa_high_v_wing(0.25, 77).to_bits(),
        0x4031_0000_0000_0000,
        "v025=17.0"
    );
    assert_eq!(
        b.get_aoa_high_v_wing(0.75, 0).to_bits(),
        0x4033_0000_0000_0000,
        "v075=19.0"
    );
}

/// 基线: aoal_v000/v025/vneg — Low 版无 vwing==0 混合分支 (源码不对称保真)
#[test]
fn java8_oracle_get_aoa_low_v_wing() {
    let b = oracle_sweep_fmdata();
    assert_eq!(
        b.get_aoa_low_v_wing(0.0, 50).to_bits(),
        0xC030_0000_0000_0000,
        "v000=-16.0 首档"
    );
    assert_eq!(
        b.get_aoa_low_v_wing(0.25, 50).to_bits(),
        0xC031_0000_0000_0000,
        "v025=-17.0"
    );
    assert_eq!(
        b.get_aoa_low_v_wing(-0.3, 50).to_bits(),
        0xC030_0000_0000_0000,
        "vneg=-16.0 下界"
    );
}

/// 基线: guard_null_* / guard_one_* — sweepLevels null 或 ≤1 档回落静态字段
#[test]
fn java8_oracle_sweep_guards_null_and_single() {
    let mut b = oracle_sweep_fmdata();
    b.sweep_levels = None;
    assert_eq!(
        b.get_vne_v_wing(0.7).to_bits(),
        0x408F_3C00_0000_0000,
        "null→vne=999.5"
    );
    assert_eq!(
        b.get_mne_v_wing(0.7).to_bits(),
        0x3FFC_0000_0000_0000,
        "null→vneMach=1.75"
    );
    assert_eq!(
        b.get_aoa_high_v_wing(0.7, 50).to_bits(),
        0x4030_8000_0000_0000,
        "null→16.5"
    );
    assert_eq!(
        b.get_aoa_low_v_wing(0.7, 50).to_bits(),
        0xC030_8000_0000_0000,
        "null→-16.5"
    );
    // 空表 (Java size()==0 <= 1 同路径)
    b.sweep_levels = Some(Vec::new());
    assert_eq!(
        b.get_vne_v_wing(0.7).to_bits(),
        0x408F_3C00_0000_0000,
        "empty→vne"
    );

    // 单档: 守卫先于 interp_sweep_level 的 n==1 分支 (返回静态字段而非档位值)
    let mut l = SweepLevel::default();
    l.sweep = 0.3;
    l.vne = 700.0;
    l.vne_mach = 0.9;
    let mut nf = FmParts::default();
    nf.aoa_crit_high = 15.0;
    nf.aoa_crit_low = -15.0;
    l.no_flaps = Some(nf);
    b.sweep_levels = Some(vec![l]);
    assert_eq!(
        b.get_vne_v_wing(0.9).to_bits(),
        0x408F_3C00_0000_0000,
        "one→vne=999.5 非 700"
    );
    assert_eq!(
        b.get_mne_v_wing(0.9).to_bits(),
        0x3FFC_0000_0000_0000,
        "one→vneMach=1.75"
    );
    assert_eq!(
        b.get_aoa_high_v_wing(0.9, 50).to_bits(),
        0x4030_8000_0000_0000,
        "one→16.5 非 15.0"
    );
    assert_eq!(
        b.get_aoa_low_v_wing(0.9, 50).to_bits(),
        0xC030_8000_0000_0000,
        "one→-16.5"
    );
}

/// 基线: gload_fallback/w0/wneg/calc/calc2 — 动态 G 限与回退
#[test]
fn java8_oracle_get_max_allow_gload_for_weight() {
    let mut c = FmData::default();
    c.max_allow_gload = Some([-3.8, 8.5]);
    // raw 为 null → 回退静态值 (Java 返回字段引用)
    assert_eq!(
        b(&c.get_max_allow_gload_for_weight(5000.0)),
        [0xC00E_6666_6666_6666, 0x4021_0000_0000_0000],
        "fallback"
    );
    // 双 null (未加载对象) → Java 返回 null ↔ None
    assert!(
        FmData::default()
            .get_max_allow_gload_for_weight(5000.0)
            .is_none(),
        "全默认→None"
    );
    // weight<=0 → 回退
    c.raw_wing_crit_overload = Some([-196000.0, 441000.0]);
    assert_eq!(
        b(&c.get_max_allow_gload_for_weight(0.0)),
        [0xC00E_6666_6666_6666, 0x4021_0000_0000_0000],
        "w0"
    );
    assert_eq!(
        b(&c.get_max_allow_gload_for_weight(-1.0)),
        [0xC00E_6666_6666_6666, 0x4021_0000_0000_0000],
        "wneg"
    );
    // 常规: 1.2*(2*raw/(g*w)±1), g=9.80
    assert_eq!(
        b(&c.get_max_allow_gload_for_weight(5000.0)),
        [0xC020_CCCC_CCCC_CCCD, 0x4034_6666_6666_6666],
        "calc (-8.4, 20.4)"
    );
    assert_eq!(
        b(&c.get_max_allow_gload_for_weight(3825.75)),
        [0xC026_B170_3F1D_3D0D, 0x403B_079E_4700_E4AF],
        "calc2 非整权重"
    );
}

/// 基线: fmw_*/fmo_*/fm_max0 — 档位检索 (严格小于) 与越界返回 max_eng_load
#[test]
fn java8_oracle_findmax_water_and_oil_load() {
    let mut d = FmData::default();
    let wl = [85.0, 95.0, 105.0];
    let ol = [90.0, 100.0, 110.0];
    let mut loads = Vec::with_capacity(3);
    for i in 0..3 {
        let mut e = EngineLoad::default();
        e.water_limit = wl[i];
        e.oil_limit = ol[i];
        loads.push(e);
    }
    d.eng_load = Some(loads);
    d.max_eng_load = 3;
    assert_eq!(
        d.findmax_water_load(d.eng_load.as_deref().unwrap(), 80.0),
        0,
        "fmw_80"
    );
    assert_eq!(
        d.findmax_water_load(d.eng_load.as_deref().unwrap(), 85.0),
        1,
        "fmw_85 严格<: 等于界限进下一档"
    );
    assert_eq!(
        d.findmax_water_load(d.eng_load.as_deref().unwrap(), 90.0),
        1,
        "fmw_90"
    );
    assert_eq!(
        d.findmax_water_load(d.eng_load.as_deref().unwrap(), 104.0),
        2,
        "fmw_104"
    );
    assert_eq!(
        d.findmax_water_load(d.eng_load.as_deref().unwrap(), 110.0),
        3,
        "fmw_110 越界→maxEngLoad"
    );
    assert_eq!(
        d.findmax_oil_load(d.eng_load.as_deref().unwrap(), 89.0),
        0,
        "fmo_89"
    );
    assert_eq!(
        d.findmax_oil_load(d.eng_load.as_deref().unwrap(), 95.0),
        1,
        "fmo_95"
    );
    assert_eq!(
        d.findmax_oil_load(d.eng_load.as_deref().unwrap(), 110.0),
        3,
        "fmo_110"
    );
    assert_eq!(
        d.findmax_water_load(d.eng_load.as_deref().unwrap(), 0.0),
        0,
        "fm_max0"
    );
}

/// 基线: pt_aft — 峰值推力 (MIL 路径无生产消费方已删, 2026-09 收敛单路)
#[test]
fn java8_oracle_peak_thrust() {
    let mut e = FmData::default();
    e.peak_thr_aft = 1500.5;
    assert_eq!(e.peak_thrust().to_bits(), 0x4097_7200_0000_0000, "pt_aft");
}

/// calculatePeakThrust 边界: null 表/零维数 → 0; 网格寻峰; 全负表保持 0 (peak 初值)
#[test]
fn calculate_peak_thrust_boundaries() {
    let mut e = FmData::default();
    assert_eq!(e.calculate_peak_thrust(None), 0.0, "null 表");
    let row0 = vec![100.0, 900.0];
    let row1 = vec![1500.0, 300.0];
    let table = vec![row0, row1];
    assert_eq!(
        e.calculate_peak_thrust(Some(&table)),
        0.0,
        "alt/vel 维数为 0"
    );
    e.alt_thr_num = 2;
    assert_eq!(e.calculate_peak_thrust(Some(&table)), 0.0, "vel 维数为 0");
    e.vel_thr_num = 2;
    assert_eq!(
        e.calculate_peak_thrust(Some(&table)),
        1500.0,
        "网格最大值在 [1][0]"
    );
    // 全负: peak 从 0 起步, > 比较不命中 → 0 (Java 同)
    let neg = vec![vec![-5.0, -1.0], vec![-9.0, -3.0]];
    assert_eq!(e.calculate_peak_thrust(Some(&neg)), 0.0, "全负表→0");
    // 维数小于表: 只扫前缀 (Java 循环上界语义)
    e.alt_thr_num = 1;
    assert_eq!(
        e.calculate_peak_thrust(Some(&table)),
        900.0,
        "alt=1 只扫首行"
    );
}

/// calculatePeakThrust: 内层行短于 vel_thr_num → Java AIOOBE ↔ panic
#[test]
#[should_panic]
fn calculate_peak_thrust_short_row_panics() {
    let mut e = FmData::default();
    e.alt_thr_num = 1;
    e.vel_thr_num = 2;
    let short = vec![vec![1.0]];
    let _ = e.calculate_peak_thrust(Some(&short));
}

/// Java 隐式初始化: 数值 0 / bool false / 引用 null↔None 抽样
#[test]
fn default_matches_java_implicit_init() {
    let b = FmData::default();
    assert!(!b.valid);
    assert_eq!(b.vne, 0.0);
    assert_eq!(b.emptyweight, 0.0);
    assert_eq!(b.max_eng_load, 0);
    assert_eq!(b.engine_num, 0);
    assert!(!b.is_jet);
    assert!(!b.has_comp_omega_factor_sq);
    assert!(b.fmdata.is_none());
    assert!(b.sweep_levels.is_none());
    assert!(b.no_flaps_wing.is_none());
    assert!(b.is_v_wing.is_none(), "Boolean 装箱 null");
    assert!(
        b.explicit_exact_altitudes.is_none(),
        "ExactAltitudes 未定义"
    );
    assert!(b.max_allow_gload.is_none() && b.raw_wing_crit_overload.is_none());
    assert!(b.max_thr_aft.is_none() && b.altitude_thr.is_none());
    assert_eq!(b.military_rpm, 0.0);
    assert_eq!(b.throttle_boost, 0.0, "getload 才会补 1.0 默认");
}

/// 未加载对象解引用: Java NPE ↔ unwrap panic
#[test]
#[should_panic]
fn unloaded_aoa_blend_panics_like_npe() {
    let b = FmData::default();
    let _ = b.get_aoa_high_v_wing(0.0, 50);
}

/// getVersion: 文件不存在 → null↔None (cargo 测试 cwd 为 crate 根, 无 data/aces)
#[test]
fn get_version_missing_file_returns_none() {
    let b = FmData::default();
    let exists = std::path::Path::new("./data/aces/version").exists();
    assert_eq!(
        b.get_version().is_some(),
        exists,
        "与 Java 同路径存在性对齐"
    );
}

/// 基线 bits 数组解包小工具
fn b(a: &Option<[f64; 2]>) -> [u64; 2] {
    let a = a.as_ref().unwrap();
    [a[0].to_bits(), a[1].to_bits()]
}
