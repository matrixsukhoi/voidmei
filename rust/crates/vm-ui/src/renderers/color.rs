//! ColorRowRenderer 的写回链语义复刻 (src/ui/layout/renderer/ColorRowRenderer.java)
//! + ColorHelper 的格式化面 (src/prog/util/ColorHelper.java)。
//!
//! **D9 变更**: 原 iced view 层、读链 (read_current) 及取色器 HSB 数学
//! 已删 — 渲染与取色归 vm-webui web 壳 (JS 侧
//! parseColorValue/rgbaToHex 与 vm-core 解析语义对齐), 本模块仅存 ColorHelper
//! 格式化 (to_decimal_string) + 写链 (apply)。
//!
//! **波13 变更**: parse_color/try_parse_color 解析本体收敛到
//! `vm_core::ui_support::color` (唯一真相, 与 configuration_service 共用);
//! 边缘语义 (Java 8 oracle 对拍, 2026-08-26) 的测试锚点仍在下方 tests。
//!
//! ColorHelper 语义:
//! - parse_color: hex (#RRGGBB / #RRGGBBAA) 与十进制 ("R, G, B[, A]") 双格式,
//!   失败回落默认色 (cfg :value 为 hex, 用户编辑后存十进制 — 双格式互通的原因)。
//! - to_decimal_string: 配置存储格式 (向后兼容)。
//!
//! 写回语义保真 (Java apply):
//! 主键存十进制 + legacy 分键 keyR/G/B/A (全库无读取方, 保真写入)
//! + row.value=十进制串 + onSave。
//!
//! PORT(提交时机备案): Java hex 输入 Enter/失焦提交; web 壳 JS 输入框
//! 同语义 (合法完整色串才提交), Message::ColorPicked 消息形状不变。

use crate::render_context::RenderContext;
use vm_core::config::config_loader::{ConfigValue, GroupConfig};

use super::{find_row_path, row_by_path, row_by_path_mut};

/// Java Color.WHITE (ColorRowRenderer 解析回落的默认白)
pub const WHITE: [u8; 4] = [255, 255, 255, 255];

/// Java toDecimalString: 存储格式 "R, G, B, A"。
/// PORT: Java null 分支 ("255, 255, 255, 255") 在 Rust 类型下不可达。
pub fn to_decimal_string(c: &[u8; 4]) -> String {
    format!("{}, {}, {}, {}", c[0], c[1], c[2], c[3])
}

/// 颜色变更写回 (Java applyColorChange)。经 main_form::update 的
/// ColorPicked 臂接线 (with_panel 模式, 与 switch/slider/combo 同构)。
pub fn apply(panel: &mut GroupConfig, key: &str, rgba: [u8; 4], ctx: &dyn RenderContext) {
    let Some(path) = find_row_path(&panel.rows, key) else {
        return;
    };
    let prop = row_by_path(&panel.rows, &path)
        .expect("find_row_path 已定位")
        .property
        .clone();
    let unified = to_decimal_string(&rgba);
    if let Some(p) = prop.as_deref() {
        // 主键十进制存储 (向后兼容, 对位 Java apply)
        ctx.sync_string_to_config_service(p, &unified);
        // legacy 分键 (拆通道写; 全库无读取方, 纯兼容面, 保真写入)
        ctx.sync_string_to_config_service(&format!("{p}R"), &rgba[0].to_string());
        ctx.sync_string_to_config_service(&format!("{p}G"), &rgba[1].to_string());
        ctx.sync_string_to_config_service(&format!("{p}B"), &rgba[2].to_string());
        ctx.sync_string_to_config_service(&format!("{p}A"), &rgba[3].to_string());
    }
    // row.value = unified (内存模型)
    row_by_path_mut(&mut panel.rows, &path)
        .expect("find_row_path 已定位")
        .value = Some(ConfigValue::Str(unified));
    // onSave
    ctx.on_save();
}

// =====================================================================
// Tests — ColorHelper 边缘语义全部取自 Java 8 oracle 对拍 (默认色 1,2,3,4)
// =====================================================================
#[cfg(test)]
mod tests;
