//! 键表集中 (重构波2 自 app_shell.rs 各段收敛): MiniHUD/FM拆包数据 interest 键、
//! FM show* 字段键、全局五色键、overlay 位置组映射。

/// MiniHUD withInterest 键 (与 Java 逐字对齐; 测试
/// minihud_interest_keys_hit_ui_layout_cfg 以此为源核对 cfg 键空间 — 审查 W1:
/// 曾笔误 "showAttitudeIndicator", 前缀匹配下不命中任何 cfg 键, 开关失效)
pub const MINIHUD_INTEREST_KEYS: [&str; 13] = [
    "displayCrosshair",
    "drawHUD",
    "disableHUD",
    "crosshair",
    "miniHUD",
    "enableLayoutDebug",
    "enableFlapAngleBar",
    "hudMach",
    "showSpeedBar",
    "showAttitudeGauge",
    "attitudeIndicatorInertialMode",
    "alwaysShowRadarAltitude",
    "showHUD",
];

/// FM拆包数据 withInterest 键 (与 Java 逐字对齐, 20 键)。
/// 注: fmInfoColumn 在 cfg 无 :target 项 (Java 同为死键, 原样搬移不裁 —
/// PowerInfo "S." 死前缀同款备案); selectedFM 前缀命中 cfg 的 selectedFM0/1;
/// fontName 同时命中全局前缀 "font" (is_global_config 全量刷新, Java 同)
pub const FM_UNPACKED_INTEREST_KEYS: [&str; 20] = [
    "displayFmKey",
    "selectedFM",
    "fmInfoColumn",
    "fontName",
    "showWeight",
    "showCritSpeed",
    "showGLoadLimits",
    "showFlapLimits",
    "showControlEffectiveness",
    "showNitro",
    "showHeatRecovery",
    "showMaxLiftLoad",
    "showInertia",
    "showLift",
    "showDrag",
    "showNoFlapsWing",
    "showFullFlapsWing",
    "showFuselage",
    "showFin",
    "showStab",
];

/// FM拆包数据 show* 开关键集 (Java isFieldEnabled 实参全集, 16 键;
/// interest 键 displayFmKey/selectedFM/fmInfoColumn/fontName 不入 —
/// 段开关不读它们)。消费面 = core.fm.field/meta 原子组件的 sidecar tick
/// 逐 tick 直读 (原 generateLines 的同一直读面), 快照链 =
/// ConfigSnapshots.fm_field)
pub const FM_FIELD_KEYS: [&str; 16] = [
    "showWeight",
    "showCritSpeed",
    "showGLoadLimits",
    "showFlapLimits",
    "showControlEffectiveness",
    "showNitro",
    "showHeatRecovery",
    "showMaxLiftLoad",
    "showInertia",
    "showLift",
    "showDrag",
    "showNoFlapsWing",
    "showFullFlapsWing",
    "showFuselage",
    "showFin",
    "showStab",
];

/// 全局五色 cfg 键 (Java loadFromConfig 读入 Application 静态)
pub const GLOBAL_COLOR_KEYS: [&str; 5] =
    ["fontNum", "fontLabel", "fontUnit", "fontWarn", "fontShade"];

// R2 位置链重构: OVERLAY_SECTIONS (id→panel 标题位置映射) 已删 —
// 窗口位置唯一真源 = PageDoc.pos, host 条目键由 PageDoc::host_key() 派生
