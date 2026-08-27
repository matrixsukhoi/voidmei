//! ButtonRowRenderer 的 iced 语义复刻 (src/ui/layout/renderer/ButtonRowRenderer.java)。
//!
//! Java 的五个动作按钮 (按 :target 分派, ui_layout.cfg 实况):
//!
//! | :target        | Java 行为 (出处)                                        | 依赖 (批十三)        |
//! |----------------|---------------------------------------------------------|----------------------|
//! | resetConfig    | 确认对话框 → publish(ACTION_RESET_REQUEST) (L38-62)     | DialogService + Controller |
//! | openComparison | 读 selectedFM0/1 → CompactComparisonWindow (L64-77)     | 对比窗口 (未译)      |
//! | openPowerCurve | 读 FM 配置 → PowerCurveWindow (L79-105)                 | 功率曲线窗口 (未译)  |
//! | importConfig   | 拖放导入对话框 → importConfig 热重载 (L109-119,166-186)  | 文件对话框           |
//! | factoryReset   | 确认对话框 → resetToFactory (L121-147)                  | DialogService        |
//!
//! 动作本体全部依赖对话框/窗口/Controller (批十三接线), 且消息枚举本批冻结 (规格
//! 无 ButtonAction 变体) — 消息转发 = on_press(Message::Ignore) 占位保按压链路,
//! 未知 :target 按钮不加 on_press (Java 五个 if 全不中 → 无监听器的纯按钮, L25-147)。
//! PORT: WebPopOver 气泡关闭 (disposeAllPopovers) 由窗口管理层承担, 不迁移。
//!
//! :fgcolor 前景色 (L149-154 + parseColor L188-202): 本地十进制解析 "R, G, B"
//! (≥3 段, 逐段 Java String.trim (码点 ≤0x20, 共用 color::java_trim), 越界/非法 →
//! 不着色) — 注意与 ColorHelper 语义不同: 无 hex、无 alpha (Color(r,g,b) 恒 255)、
//! 无钳位 (越界抛异常 → null)。

use iced::widget::{button, text, Button};
use iced::{Color, Element};
use vm_core::config_loader::RowConfig;

use crate::main_form::Message;

/// Java parseColor (L188-202): 十进制 "R, G, B" (≥3 段) → 不透明前景色;
/// 非法/越界/不足 3 段 → None (不着色)。
fn parse_fg_color(s: &str) -> Option<[u8; 4]> {
    // Java split(","): 尾部空串丢弃 (与 color.rs 的对拍口径一致)
    let mut parts: Vec<&str> = s.split(',').collect();
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    if parts.len() < 3 {
        return None;
    }
    // Java p[i].trim() = String.trim (码点 <=0x20, 非 Unicode 空白集) — 与
    // color.rs 共用 java_trim 对齐口径 (nbsp 不删 → parseInt 失败)
    let r = super::color::java_trim(parts[0]).parse::<i32>().ok()?;
    let g = super::color::java_trim(parts[1]).parse::<i32>().ok()?;
    let b = super::color::java_trim(parts[2]).parse::<i32>().ok()?;
    // Java new Color(r,g,b): 任一越界抛 IllegalArgumentException → catch → null
    // (无钳位 — 与 ColorHelper.parseColor 的钳位语义不同)
    let in_range = |v: i32| (0..=255).contains(&v);
    if !in_range(r) || !in_range(g) || !in_range(b) {
        return None;
    }
    Some([r as u8, g as u8, b as u8, 255]) // Color(r,g,b) alpha 恒 255, 第 4 段忽略
}

/// 已知动作 :target 集合 (Java 五个 if 的分派键)
const KNOWN_ACTIONS: [&str; 5] = [
    "resetConfig",
    "openComparison",
    "openPowerCurve",
    "importConfig",
    "factoryReset",
];

/// 按钮行视图: 动作按钮 (文本 = row.label, Java L25)。
pub fn view_row(row: &RowConfig) -> Element<'_, Message> {
    let mut btn = Button::new(text(row.label.clone()));
    if let Some(fg) = row.fg_color.as_deref().and_then(parse_fg_color) {
        // Java L149-153: btn.setForeground(color)
        btn = btn.style(move |_, _| button::Style {
            text_color: Color::from_rgba8(fg[0], fg[1], fg[2], 1.0),
            ..Default::default()
        });
    }
    match row.property.as_deref() {
        // 已接动作 (审查轮 2-D): 确认模态 → 直调 (main_form update 执行链)
        Some(p @ ("resetConfig" | "factoryReset")) => {
            btn.on_press(Message::ButtonAction { action: p.to_string() }).into()
        }
        // 未迁移动作 (openComparison/openPowerCurve/importConfig — 窗口/文件
        // 对话框未译, 模块文档表备案): 按压链路保通但不执行
        Some(p) if KNOWN_ACTIONS.contains(&p) => btn.on_press(Message::Ignore).into(),
        // 未知 :target / 无 :target: 无监听器纯按钮 (Java 无 if 命中)
        _ => btn.into(),
    }
}

// =====================================================================
// Tests
// =====================================================================
#[cfg(test)]
mod tests;
