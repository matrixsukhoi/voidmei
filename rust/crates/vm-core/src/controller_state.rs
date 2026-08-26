//! 对应 Java: `src/prog/ControllerState.java` (一比一翻译)
//!
//! Controller 生命周期状态枚举。Java 侧 `Controller.State` 字段的赋值与
//! `==` 比较 (`State == ControllerState.PREVIEW` 等) 由派生 PartialEq 的
//! 相同枚举比较承接; 生命周期流转 INIT→CONNECTED→IN_GAME→PREVIEW 见
//! LIFETIMES.md §"ControllerState"。

use std::fmt;

/// Represents the State of the Controller lifecycle.
// PORT: Java 枚举常量全大写 (INIT/CONNECTED/IN_GAME/PREVIEW) → Rust 驼峰
// (Init/Connected/InGame/Preview), 语义不变 (fm/status.rs 同款先例);
// Java 枚举默认 toString()=常量名 的字符串形态由 Display 保留 ——
// Controller.java:897 (`", state=" + State`) 与 OverlayManager.java:119
// (`"(state=" + tc.State + ")"`) 的日志拼接依赖该形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerState {
    /// Initial State - waiting for status bar initialization
    Init,
    /// Connected to game server - waiting to enter game
    Connected,
    /// In game - all overlays active
    InGame,
    /// Preview mode
    Preview,
}

/// Java `values()` 的声明序 (INIT→CONNECTED→IN_GAME→PREVIEW),
/// `fromLegacyValue` 按此序遍历取首个匹配。
const VALUES: [ControllerState; 4] = [
    ControllerState::Init,
    ControllerState::Connected,
    ControllerState::InGame,
    ControllerState::Preview,
];

impl ControllerState {
    /// Get the legacy integer value for backwards compatibility.
    // PORT: Java `private final int legacyValue` 字段 (构造器注入, 仅经本 getter
    // 暴露) → match 常量编码; 字段不可直达, 保持 Java 私有语义 (§0.7 免 getter
    // 规则只针对 public 字段)。
    pub fn get_legacy_value(&self) -> i32 {
        match *self {
            ControllerState::Init => 1,
            ControllerState::Connected => 2,
            ControllerState::InGame => 3,
            ControllerState::Preview => 4,
        }
    }

    /// Convert legacy integer flag to ControllerState.
    // Java enhanced-for over values(), 声明序首个 legacyValue 匹配返回, 兜底 INIT。
    pub fn from_legacy_value(value: i32) -> ControllerState {
        for state in VALUES.iter() {
            if state.get_legacy_value() == value {
                return *state;
            }
        }
        ControllerState::Init
    }
}

/// 对应 Java 枚举默认 `toString()` = `name()` = 声明常量名 (含下划线)。
// PORT: Java 8 oracle 实测 (/tmp 临时工程, 用完已删): 四态 toString/name
// 均为声明名 "INIT"/"CONNECTED"/"IN_GAME"/"PREVIEW"。
impl fmt::Display for ControllerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ControllerState::Init => "INIT",
            ControllerState::Connected => "CONNECTED",
            ControllerState::InGame => "IN_GAME",
            ControllerState::Preview => "PREVIEW",
        };
        f.write_str(s)
    }
}

// =====================================================================
// Tests — Java 侧无独立测试文件; 公共面 (get_legacy_value / from_legacy_value
// / Display=toString 形态) 按"每个公共函数写边界测试"规则补齐, 期望值取自
// Java 8 oracle dump。四态互异由 Rust enum 判别式唯一性 + 派生 PartialEq
// 编译期保证, 不写空转测试 (§5)。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// Java 8 oracle 对拍 (§5.1): 声明序 legacyValue = 1/2/3/4,
    /// toString 与 name() 同形 (日志拼接依赖)。
    #[test]
    fn test_display_and_legacy_values_match_java() {
        let expected = [
            ("INIT", 1),
            ("CONNECTED", 2),
            ("IN_GAME", 3),
            ("PREVIEW", 4),
        ];
        for (i, (name, legacy)) in expected.iter().enumerate() {
            let s = VALUES[i];
            assert_eq!(s.to_string(), *name);
            assert_eq!(s.get_legacy_value(), *legacy);
        }
    }

    /// Java 8 oracle 对拍: fromLegacyValue 命中 1~4 (声明序首个匹配)。
    #[test]
    fn test_from_legacy_value_round_trip() {
        assert_eq!(ControllerState::from_legacy_value(1), ControllerState::Init);
        assert_eq!(ControllerState::from_legacy_value(2), ControllerState::Connected);
        assert_eq!(ControllerState::from_legacy_value(3), ControllerState::InGame);
        assert_eq!(ControllerState::from_legacy_value(4), ControllerState::Preview);
    }

    /// Java 8 oracle 对拍: 未匹配值 (0/5/-1/INT_MIN/INT_MAX) 兜底 INIT。
    #[test]
    fn test_from_legacy_value_unknown_defaults_to_init() {
        for v in [0, 5, -1, i32::MIN, i32::MAX] {
            assert_eq!(ControllerState::from_legacy_value(v), ControllerState::Init);
        }
    }
}
