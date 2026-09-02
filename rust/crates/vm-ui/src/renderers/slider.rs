//! SliderRowRenderer 的写回链语义复刻 (src/ui/layout/renderer/SliderRowRenderer.java)。
//!
//! **D9 变更**: 原 iced view_row 已删 (渲染归 vm-webui web 壳), 本模块仅存
//! 读链 (read_current) + 写链 (apply) + 区间守卫 (effective_range)。
//!
//! 交互语义保真:
//! - 读: 默认值 = row.getInt() (getInt 内部吞异常 → 0); readInt 优先级
//!   PropertyBinder 组字段 → ConfigurationService → 默认; 值钳入 [min,max] (Java L18-43)。
//! - 写 (apply): row.value → writeInt (组字段 + 服务同步) → panelColumns 特例 onRebuild
//!   (Java persistValue L53-66)。
//! - 拖拽持久化时机: Java 仅在 valueIsAdjusting==false (拖拽结束/spinner 变更) 落盘
//!   (L70-74); D1 期 iced 对位 = 拖拽期只走 apply 内存链, 释放 → Message::Save 落盘
//!   (D9 后 web 壳沿用同一消息约定)。
//! - min >= max 防崩溃守卫: max = min + 100 (Java L33-37)。

use vm_core::config::config_loader::{ConfigValue, GroupConfig, RowConfig};
use crate::renderer_config_helper;
use crate::row_renderer_registry::RenderContext;

use super::{find_row_path, row_by_path, row_by_path_mut};

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
    // 注意: 不调 ctx.on_save() — 落盘由拖拽释放 → Message::Save 承担 (拖拽时机保真)
}

#[cfg(test)]
mod tests;
