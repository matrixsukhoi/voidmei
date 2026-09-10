//! 键表集中 (重构波2 自 app_shell.rs 各段收敛): MiniHUD/FM拆包数据 interest 键、
//! FM show* 字段键、全局五色键、overlay 位置组映射。

// (R3: MINIHUD_INTEREST_KEYS 退役 — 兴趣键由页文档 interestKeys 声明)

// (R3: FM_UNPACKED_INTEREST_KEYS 退役 — 兴趣键由页文档 interestKeys 声明)

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
