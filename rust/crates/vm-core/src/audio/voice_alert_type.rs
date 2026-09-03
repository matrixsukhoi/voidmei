//! VoiceAlertType 的 Rust 移植 (src/prog/audio/VoiceAlertType.java)
//!
//! 语音告警类型枚举
//! 集中定义所有告警的 key 和默认冷却时间
//!
//! 告警分类：
//! - 攻角类 (2): aoaCrit, aoaHigh
//! - 速度类 (3): warn_ias, warn_mach, warn_stall
//! - 结构类 (4): warn_gear, warn_flap, warn_loadfactor, warn_brake
//! - 引擎类 (6): warn_engineoverheat, fail_engine, warn_lowrpm, warn_highrpm, warn_lowpressure, warn_compressor
//! - 燃油类 (2): warn_lowfuel, fail_nofuel
//! - 高度类 (3): warn_altitude, warn_terrain, warn_highvario
//! - 舵效类 (3): rudderEff, elevatorEff, aileronEff
//! - 启动音效 (1): start1
//!
//! PORT: Java 枚举常量名 SCREAMING_SNAKE (AOA_CRIT) → Rust PascalCase (AoaCrit),
//! 下划线分词处按原分段还原 (FMStatus::NotAircraft 先例)。
//! PORT: Java 枚举常量携带构造参数 (private final 字段); Rust 枚举带字段的
//! 字段无法在 match 外直接读取, 故数据集中在 parts() 的 match 表 (§1 枚举带字段),
//! 与 Java 常量声明逐一对应、顺序即声明序。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceAlertType {
    // 攻角类
    AoaCrit,
    AoaHigh,

    // 速度类
    WarnIas,
    WarnMach,
    WarnStall,

    // 结构类
    WarnGear,
    WarnFlap,
    WarnLoadfactor,
    WarnBrake,

    // 引擎类
    WarnEngineoverheat,
    FailEngine,
    WarnLowrpm,
    WarnHighrpm,
    WarnLowpressure,
    WarnCompressor,

    // 燃油类
    WarnLowfuel,
    FailNofuel,

    // 高度类
    WarnAltitude,
    WarnTerrain,
    WarnHighvario,

    // 舵效类
    RudderEff,
    ElevatorEff,
    AileronEff,

    // 启动音效
    Start1,
}

/// 对应 Java 隐式 `values()`: 声明序全集。
/// Java 每次调用返回新数组; Rust 共享 const 数组 (枚举只读, 语义一致)。
pub const ALL: [VoiceAlertType; 24] = [
    VoiceAlertType::AoaCrit,
    VoiceAlertType::AoaHigh,
    VoiceAlertType::WarnIas,
    VoiceAlertType::WarnMach,
    VoiceAlertType::WarnStall,
    VoiceAlertType::WarnGear,
    VoiceAlertType::WarnFlap,
    VoiceAlertType::WarnLoadfactor,
    VoiceAlertType::WarnBrake,
    VoiceAlertType::WarnEngineoverheat,
    VoiceAlertType::FailEngine,
    VoiceAlertType::WarnLowrpm,
    VoiceAlertType::WarnHighrpm,
    VoiceAlertType::WarnLowpressure,
    VoiceAlertType::WarnCompressor,
    VoiceAlertType::WarnLowfuel,
    VoiceAlertType::FailNofuel,
    VoiceAlertType::WarnAltitude,
    VoiceAlertType::WarnTerrain,
    VoiceAlertType::WarnHighvario,
    VoiceAlertType::RudderEff,
    VoiceAlertType::ElevatorEff,
    VoiceAlertType::AileronEff,
    VoiceAlertType::Start1,
];

// PORT: Java values() 由编译器从枚举声明生成, 变体与数据在声明点绑定不可能漂移;
// Rust 拆成 enum + ALL + parts() 三处维护, 此编译期断言绑定 ALL 与变体集:
// 新增变体时 match 穷尽性强制加 arm, 计数与 ALL.len() 不符即编译失败,
// 防止 ALL 静默漏项 (from_key/get_configurable_keys 将找不到新告警)。
const fn variant_count() -> usize {
    match VoiceAlertType::Start1 {
        VoiceAlertType::AoaCrit => 1,
        VoiceAlertType::AoaHigh => 2,
        VoiceAlertType::WarnIas => 3,
        VoiceAlertType::WarnMach => 4,
        VoiceAlertType::WarnStall => 5,
        VoiceAlertType::WarnGear => 6,
        VoiceAlertType::WarnFlap => 7,
        VoiceAlertType::WarnLoadfactor => 8,
        VoiceAlertType::WarnBrake => 9,
        VoiceAlertType::WarnEngineoverheat => 10,
        VoiceAlertType::FailEngine => 11,
        VoiceAlertType::WarnLowrpm => 12,
        VoiceAlertType::WarnHighrpm => 13,
        VoiceAlertType::WarnLowpressure => 14,
        VoiceAlertType::WarnCompressor => 15,
        VoiceAlertType::WarnLowfuel => 16,
        VoiceAlertType::FailNofuel => 17,
        VoiceAlertType::WarnAltitude => 18,
        VoiceAlertType::WarnTerrain => 19,
        VoiceAlertType::WarnHighvario => 20,
        VoiceAlertType::RudderEff => 21,
        VoiceAlertType::ElevatorEff => 22,
        VoiceAlertType::AileronEff => 23,
        VoiceAlertType::Start1 => 24,
    }
}
const _: () = assert!(
    variant_count() == ALL.len(),
    "VoiceAlertType::ALL 与枚举变体集数量不一致: 新增变体需同步 ALL 数组"
);

impl VoiceAlertType {
    /// Java private final 字段 (key, cooldownSeconds) 的唯一读取点;
    /// 各行即 Java 常量声明的构造参数。
    fn parts(self) -> (&'static str, i32) {
        match self {
            Self::AoaCrit => ("aoaCrit", 1),
            Self::AoaHigh => ("aoaHigh", 8),
            Self::WarnIas => ("warn_ias", 10),
            Self::WarnMach => ("warn_mach", 10),
            Self::WarnStall => ("warn_stall", 2),
            Self::WarnGear => ("warn_gear", 7),
            Self::WarnFlap => ("warn_flap", 1),
            Self::WarnLoadfactor => ("warn_loadfactor", 2),
            Self::WarnBrake => ("warn_brake", 8),
            Self::WarnEngineoverheat => ("warn_engineoverheat", 60),
            Self::FailEngine => ("fail_engine", 60),
            Self::WarnLowrpm => ("warn_lowrpm", 10),
            Self::WarnHighrpm => ("warn_highrpm", 10),
            Self::WarnLowpressure => ("warn_lowpressure", 30),
            Self::WarnCompressor => ("warn_compressor", 0), // 状态驱动，无冷却
            Self::WarnLowfuel => ("warn_lowfuel", 60),
            Self::FailNofuel => ("fail_nofuel", 60),
            Self::WarnAltitude => ("warn_altitude", 5),
            Self::WarnTerrain => ("warn_terrain", 5),
            Self::WarnHighvario => ("warn_highvario", 5),
            Self::RudderEff => ("rudderEff", 10),
            Self::ElevatorEff => ("elevatorEff", 10),
            Self::AileronEff => ("aileronEff", 10),
            Self::Start1 => ("start1", 1),
        }
    }

    /// 获取告警键名
    /// @return 告警键名，如 "aoaCrit"
    pub fn get_key(&self) -> &'static str {
        self.parts().0
    }

    /// 获取冷却时间（秒）
    /// @return 冷却时间秒数
    pub fn get_cooldown_seconds(&self) -> i32 {
        self.parts().1
    }

    /// 根据 key 查找告警类型
    /// @param key 告警键名
    /// @return 对应的枚举值，找不到返回 null
    // PORT: Java null 返回值/入参 → Option<VoiceAlertType> / Option<&str>。
    pub fn from_key(key: Option<&str>) -> Option<VoiceAlertType> {
        let key = key?;
        ALL.iter().copied().find(|t| t.parts().0 == key)
    }
}

// =====================================================================
// Tests — 移植自 test/TestVoicePackConfig.java 的 VoiceAlertType 部分
// =====================================================================
#[cfg(test)]
mod tests;
