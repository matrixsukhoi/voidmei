//! 对应 Java: `src/prog/ControllerState.java` (一比一翻译)
//!
//! Controller 生命周期状态枚举。Java 侧 `Controller.State` 字段的赋值与
//! `==` 比较 (`State == ControllerState.PREVIEW` 等) 由派生 PartialEq 的
//! 相同枚举比较承接; 生命周期流转 INIT→CONNECTED→IN_GAME→PREVIEW 见
//! LIFETIMES.md §"ControllerState"。

use std::fmt;

/// Represents the State of the Controller lifecycle.
// Java 枚举常量全大写 (INIT/CONNECTED/IN_GAME/PREVIEW) → Rust 驼峰
// (Init/Connected/InGame/Preview), 语义不变 (fm/status.rs 同款先例);
// Java 枚举默认 toString()=常量名 的字符串形态由 Display 保留 ——
// 日志拼接 (`", state=" + State` / `"(state=" + tc.State + ")"`) 依赖该形态。
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

/// 对应 Java 枚举默认 `toString()` = `name()` = 声明常量名 (含下划线)。
// 历史基线 (/tmp 临时工程, 用完已删): 四态 toString/name
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
// Tests — Java 侧无独立测试文件; 公共面 (Display=toString 形态) 按"每个
// 公共函数写边界测试"规则补齐, 期望值取自 历史基线 dump。四态互异由
// Rust enum 判别式唯一性 + 派生 PartialEq 编译期保证, 不写空转测试。
// (B16: get/from_legacy_value 生产零调用已删, 其对拍测试一并移除)
// =====================================================================
#[cfg(test)]
mod tests;
