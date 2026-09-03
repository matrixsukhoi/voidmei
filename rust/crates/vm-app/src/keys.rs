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

/// FM拆包数据 generateLines 逐 tick 直读的开关键集 (Java isFieldEnabled
/// 实参全集, 16 键; interest 键 displayFmKey/
/// selectedFM/fmInfoColumn/fontName 不入 — generateLines 不读它们)
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
pub const GLOBAL_COLOR_KEYS: [&str; 5] = ["fontNum", "fontLabel", "fontUnit", "fontWarn", "fontShade"];

/// 窗口 overlay id → 配置组标题 (Java Controller 各 init 的 getOverlaySettings
/// 字面量; MiniHUD 经 getHUDSettings → sectionName "MiniHUD")。位置持久化按此映射读写
/// GroupConfig.x/y; 测试 overlay_sections_hit_ui_layout_cfg 以 cfg 为源核对。
/// **键列 = live 模式窗口条目单一来源**: main.rs 冒烟的逐窗断言集、render_thread.rs
/// 注册面备案均由此派生, 新增窗口条目只改本表一处。
/// (flightInfoSwitch 原走 POC window.rs 专径, 后收编为正式条目; enableVoiceWarn/
/// thrustdFS 非常规窗口条目不列 — 见 render_thread.rs register_live_overlays 备案)
pub const OVERLAY_SECTIONS: [(&str, &str); 8] = [
    ("enableEngineControl", "引擎控制"),
    ("engineInfoSwitch", "动力信息"),
    ("crosshairSwitch", "MiniHUD"),
    ("flightInfoSwitch", "飞行信息"),
    ("enableAxis", "舵面值"),
    ("enableAttitudeIndicator", "地平仪"),
    ("enablegearAndFlaps", "起落襟翼"),
    ("enableFMPrint", "FM拆包数据"),
];
