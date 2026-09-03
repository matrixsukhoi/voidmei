#![allow(clippy::borrow_interior_mutable_const)] // UNRESOLVED 含 Mutex (见 handle.rs 注)

use super::*;

/// Java check(boolean, String) 计数式断言 → assert! 宏 (失败即 panic), 描述逐字保留
fn check(cond: bool, desc: &str) {
    assert!(cond, "FAIL: {desc}");
}

fn test_unresolved_sentinel() {
    // -- UNRESOLVED 哨兵字段值测试 --
    let h = &FMHandle::UNRESOLVED;
    check(h.name.is_none(), "哨兵 name 应为 null");
    check(
        h.status == FMStatus::Unresolved,
        "哨兵 status 应为 UNRESOLVED",
    );
    check(h.fmdata.is_none(), "哨兵 fmdata 应为 null");
    check(h.peak_wep_power == 0.0, "哨兵 peakWepPower 应为 0");
    check(h.peak_thrust == 0.0, "哨兵 peakThrust 应为 0");
    check(
        h.compressor_stages.is_none(),
        "哨兵 compressorStages 应为 null",
    );
    check(!h.has_fm(), "哨兵 hasFM 应为 false");
    check(!h.is_missing_like(), "哨兵 isMissingLike 应为 false");
}

fn test_ready_handle() {
    // -- READY 句柄语义测试 --
    // dummy Blkx: 路径不存在的文件 → 对象非 null 即可 (hasFM 只看 status 与 blkx 非空)
    // Java `new parser.Blkx("__no_such_file__.blkx", "dummy")` 构造出的
    // valid=false 空壳对象 ↔ Blkx::default() (字段全默认值)
    let dummy = FmData::default();
    let stages = [CompressorStageParams::default()];
    let h = FMHandle::ready(
        Some("plane1".to_string()),
        Some(dummy),
        1850.5,
        0.0,
        Some(stages.to_vec()),
    );

    check(
        h.status == FMStatus::Ready,
        "ready() 工厂 status 应为 READY",
    );
    check(
        h.name.as_deref() == Some("plane1"),
        "name 应保留规范化机型名",
    );
    // Java `h.blkx == dummy` 引用同一性 → 所有权模型无引用同一性可判,
    // 退化为存在性检查 (值即传入值)
    check(h.fmdata.is_some(), "fmdata 应携带解析对象");
    check(h.peak_wep_power == 1850.5, "peakWepPower 应保留传入值");
    check(h.peak_thrust == 0.0, "活塞机 peakThrust 应为 0");
    // Java `h.compressorStages == stages` 引用同一性 → 内容相等
    // (CompressorStageParams 为 Copy 值, 内容比较等价)
    check(
        h.compressor_stages.as_deref() == Some(stages.as_slice()),
        "compressorStages 应保留传入引用",
    );
    check(h.has_fm(), "READY 且 fmdata 非空 → hasFM 为 true");
    check(!h.is_missing_like(), "READY 不是 missing-like");

    // 喷气机形态: stages=null, peakThrust>0
    let jet = FMHandle::ready(
        Some("me262".to_string()),
        Some(FmData::default()),
        0.0,
        1800.0,
        None,
    );
    check(
        jet.has_fm() && jet.compressor_stages.is_none() && jet.peak_thrust == 1800.0,
        "喷气机句柄: stages null / thrust 1800",
    );
}

fn test_missing_handle() {
    // -- MISSING 句柄语义测试 --
    let h = FMHandle::missing(Some("ghost".to_string()));
    check(
        h.status == FMStatus::Missing,
        "missing() 工厂 status 应为 MISSING",
    );
    check(h.name.as_deref() == Some("ghost"), "name 应为机型名");
    check(h.fmdata.is_none(), "MISSING 不携带 fmdata");
    check(
        h.peak_wep_power == 0.0 && h.peak_thrust == 0.0,
        "MISSING 功率/推力应为 0",
    );
    check(!h.has_fm(), "MISSING hasFM 应为 false");
    check(h.is_missing_like(), "MISSING isMissingLike 应为 true");
}

fn test_corrupt_handle() {
    // -- CORRUPT 句柄语义测试 --
    let h = FMHandle::corrupt(Some("badplane".to_string()));
    check(
        h.status == FMStatus::Corrupt,
        "corrupt() 工厂 status 应为 CORRUPT",
    );
    check(h.name.as_deref() == Some("badplane"), "name 应为机型名");
    check(h.fmdata.is_none(), "CORRUPT 不携带 fmdata");
    check(!h.has_fm(), "CORRUPT hasFM 应为 false");
    check(h.is_missing_like(), "CORRUPT isMissingLike 应为 true");
}

/// NOT_AIRCRAFT: 非飞机载具（坦克/军舰）——无 FM 但也不是数据缺失，不该弹缺失 toast
fn test_not_aircraft_handle() {
    // -- NOT_AIRCRAFT 句柄语义测试 (陆战坦克) --
    let h = FMHandle::not_aircraft(Some("tankmodels/us_n4a3e8_76_sherman".to_string()));
    check(
        h.status == FMStatus::NotAircraft,
        "notAircraft() 工厂 status 应为 NOT_AIRCRAFT",
    );
    check(
        h.name.as_deref() == Some("tankmodels/us_n4a3e8_76_sherman"),
        "name 应保留原始载具名",
    );
    check(h.fmdata.is_none(), "NOT_AIRCRAFT 不携带 fmdata");
    check(!h.has_fm(), "NOT_AIRCRAFT hasFM 应为 false (HUD 走降级)");
    check(
        !h.is_missing_like(),
        "NOT_AIRCRAFT 不是 missing-like (不进负缓存/不弹缺失 toast)",
    );
}

fn test_missing_like_semantics() {
    // -- isMissingLike 全枚举覆盖测试 --
    check(
        !FMHandle::UNRESOLVED.is_missing_like(),
        "UNRESOLVED 不是 missing-like",
    );
    check(
        FMHandle::missing(Some("x".into())).fmdata.is_none(),
        "MISSING 永不携带 fmdata",
    );
    check(
        FMHandle::corrupt(Some("x".into())).fmdata.is_none(),
        "CORRUPT 永不携带 fmdata",
    );
    check(
        FMHandle::corrupt(Some("x".into())).is_missing_like(),
        "CORRUPT 属于 missing-like",
    );
    check(
        FMHandle::missing(Some("x".into())).is_missing_like(),
        "MISSING 属于 missing-like",
    );
    check(
        !FMHandle::not_aircraft(Some("x/y".into())).is_missing_like(),
        "NOT_AIRCRAFT 不属于 missing-like",
    );
}

#[test]
fn unresolved_sentinel() {
    test_unresolved_sentinel();
}

#[test]
fn ready_handle() {
    test_ready_handle();
}

#[test]
fn missing_handle() {
    test_missing_handle();
}

#[test]
fn corrupt_handle() {
    test_corrupt_handle();
}

#[test]
fn not_aircraft_handle() {
    test_not_aircraft_handle();
}

#[test]
fn missing_like_semantics() {
    test_missing_like_semantics();
}

/// 边界补充 (Java 测试未覆盖): status=READY 但 blkx=null 的防御分支 ——
/// hasFM 必须为 false (javadoc 明示 "对调用方一律视为无 FM")
#[test]
fn ready_with_null_fmdata_has_no_fm() {
    let h = FMHandle::ready(Some("x".to_string()), None, 100.0, 0.0, None);
    check(!h.has_fm(), "READY 但 fmdata 为 null → hasFM 应为 false");
}

/// 历史基线 对拍: toString 五形态 + 六态枚举名, 期望值为
/// build/基线/FMHandleOracle.java 在 OpenJDK 1.8.0_342 的实测 dump
/// (临时文件, 用完已删除)。
#[test]
fn java8_oracle_tostring() {
    assert_eq!(
        FMHandle::UNRESOLVED.to_string(),
        "FMHandle[UNRESOLVED null]"
    );
    assert_eq!(
        FMHandle::ready(
            Some("plane1".to_string()),
            Some(FmData::default()),
            1850.5,
            0.0,
            Some(vec![CompressorStageParams::default()]),
        )
        .to_string(),
        "FMHandle[READY plane1]"
    );
    assert_eq!(
        FMHandle::missing(Some("ghost".to_string())).to_string(),
        "FMHandle[MISSING ghost]"
    );
    assert_eq!(
        FMHandle::not_aircraft(Some("tankmodels/us_n4a3e8_76_sherman".to_string())).to_string(),
        "FMHandle[NOT_AIRCRAFT tankmodels/us_n4a3e8_76_sherman]"
    );
    assert_eq!(
        FMHandle::corrupt(Some("badplane".to_string())).to_string(),
        "FMHandle[CORRUPT badplane]"
    );
}
