//! SliderRowRenderer 的 iced 语义复刻 (src/ui/layout/renderer/SliderRowRenderer.java)。
//!
//! 交互语义保真:
//! - 读: 默认值 = row.getInt() (getInt 内部吞异常 → 0); readInt 优先级
//!   PropertyBinder 组字段 → ConfigurationService → 默认; 值钳入 [min,max] (Java L18-43)。
//! - 写 (apply): row.value → writeInt (组字段 + 服务同步) → panelColumns 特例 onRebuild
//!   (Java persistValue L53-66)。
//! - 拖拽持久化时机: Java 仅在 valueIsAdjusting==false (拖拽结束/spinner 变更) 落盘
//!   (L70-74); iced 对位 = on_change 只走 apply 内存链, on_release → Message::Save 落盘。
//!   PORT(分歧备案): 键盘方向键/滚轮只产生 on_change (无 on_release), 其持久化顺延到
//!   Save 按钮/下次拖拽释放 — Java 对键盘/spinner 是即时落盘 (L92-95)。
//! - min >= max 防崩溃守卫: max = min + 100 (Java L33-37)。

use iced::widget::{column, row, slider, text};
use iced::Element;
use vm_core::config_loader::{ConfigValue, GroupConfig, RowConfig};
use vm_core::renderer_config_helper;
use vm_core::row_renderer_registry::RenderContext;

use super::{find_row_path, row_by_path, row_by_path_mut};
use crate::main_form::Message;

/// Java L33-37: Ensure min < max to avoid crash
pub fn effective_range(min: i32, max: i32) -> (i32, i32) {
    if min >= max {
        (min, min + 100)
    } else {
        (min, max)
    }
}

/// 显示值 (Java L18-43): readInt 优先级链 + 钳位。
pub fn read_current(row: &RowConfig, panel: &GroupConfig, ctx: &dyn RenderContext) -> i32 {
    let (min, max) = effective_range(row.min_val, row.max_val);
    // Java: defaultVal = 0; if (row.value != null) try { row.getInt() } catch {}
    // — getInt 自吞解析异常 → 0, 与 row.get_int() 的 None→0/Str→0 逐位一致
    let default_val = row.get_int();
    let current = renderer_config_helper::read_int(ctx, panel, row, default_val);
    current.clamp(min, max)
}

/// 值变更写回 (Java persistValue): 内存链, 不含落盘 (见模块文档拖拽时机)。
pub fn apply(panel: &mut GroupConfig, key: &str, value: i32, ctx: &dyn RenderContext) {
    let Some(path) = find_row_path(&panel.rows, key) else {
        return;
    };
    let prop = row_by_path(&panel.rows, &path).expect("find_row_path 已定位").property.clone();

    // Java L57-58: Update memory model so it saves to ui_layout.cfg
    row_by_path_mut(&mut panel.rows, &path)
        .expect("find_row_path 已定位")
        .value = Some(ConfigValue::Int(value));
    // Java L60: writeInt (PropertyBinder 组字段 + 服务同步)
    renderer_config_helper::write_int(ctx, panel, prop.as_deref(), value);
    // Java L62-64: panelColumns 特例触发整页重建
    if prop.as_deref() == Some("panelColumns") {
        ctx.on_rebuild();
    }
    // 注意: 不调 ctx.on_save() — 落盘由 on_release → Message::Save 承担 (拖拽时机保真)
}

/// 滑条行视图: [Label 当前值+单位] / [Slider] (Java createSliderItem 布局:
/// [Label] [Slider] [Spinner] [Unit]; iced 无 spinner, 值+单位并入标签行)。
pub fn view_row<'a>(
    row: &'a RowConfig,
    panel: &'a GroupConfig,
    ctx: &dyn RenderContext,
    panel_title: &'a str,
) -> Element<'a, Message> {
    let (min, max) = effective_range(row.min_val, row.max_val);
    let current = read_current(row, panel, ctx);
    let value_text = format!("{current}{}", row.unit); // :unit 后缀 (Java Spinner 旁单位)

    // 消息键: :target 优先; 无 :target 以 label 为键 (Java persistValue 对 prop=null
    // 仍写 row.value, writeInt(null) 不落服务); property/label 双空残端 → Ignore
    let key = row
        .property
        .clone()
        .or_else(|| (!row.label.is_empty()).then(|| row.label.clone()));
    let slider_el: Element<'a, Message> = match key {
        Some(key) => {
            let p = panel_title.to_string();
            // on_change: 拖拽实时更新 (滑块跟手, 内存链); on_release: 拖拽结束落盘
            // (对位 Java valueIsAdjusting==false 守卫)
            slider(min..=max, current, move |v| Message::Slider {
                panel: p.clone(),
                key: key.clone(),
                value: v,
            })
            .on_release(Message::Save)
            .into()
        }
        None => slider(min..=max, current, |_| Message::Ignore).into(),
    };

    column![
        row![text(row.label.clone()), text(value_text)].spacing(8),
        slider_el,
    ]
    .spacing(4)
    .into()
}

#[cfg(test)]
mod tests;
