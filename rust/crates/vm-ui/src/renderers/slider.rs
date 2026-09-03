//! SliderRowRenderer 的写回链语义复刻 (src/ui/layout/renderer/SliderRowRenderer.java)。
//!
//! **D9 变更**: 渲染与读链已删 (归 vm-webui web 壳, 回显走整树 DTO),
//! 本模块仅存写链 (apply) + 区间守卫 (effective_range)。
//!
//! 写回语义保真:
//! - 写 (apply): row.value → writeInt (组字段 + 服务同步)。
//! - 拖拽持久化时机: Java 仅在 valueIsAdjusting==false (拖拽结束/spinner 变更) 落盘;
//!   web 壳沿用同一消息约定 (拖拽期只走 apply 内存链, 释放 → Message::Save 落盘)。
//! - min >= max 防崩溃守卫: max = min + 100。
//!
//! PORT(panelColumns 特例退役): Java persistValue 对 panelColumns 触发 onRebuild
//! 重建整页; D9 后视图刷新归 web 壳 (writeInt 的服务 CONFIG_CHANGED 广播面),
//! Rust 侧 on_rebuild 无消费效果, 该分支不再保留。

use crate::render_context::RenderContext;
use crate::renderer_config_helper;
use vm_core::config::config_loader::{ConfigValue, GroupConfig};

use super::{find_row_path, row_by_path, row_by_path_mut};

/// Ensure min < max to avoid crash
pub fn effective_range(min: i32, max: i32) -> (i32, i32) {
    if min >= max {
        (min, min + 100)
    } else {
        (min, max)
    }
}

/// 值变更写回 (Java persistValue): 内存链, 不含落盘 (见模块文档拖拽时机)。
pub fn apply(panel: &mut GroupConfig, key: &str, value: i32, ctx: &dyn RenderContext) {
    let Some(path) = find_row_path(&panel.rows, key) else {
        return;
    };
    let prop = row_by_path(&panel.rows, &path)
        .expect("find_row_path 已定位")
        .property
        .clone();

    // Update memory model so it saves to ui_layout.cfg
    row_by_path_mut(&mut panel.rows, &path)
        .expect("find_row_path 已定位")
        .value = Some(ConfigValue::Int(value));
    // writeInt (PropertyBinder 组字段 + 服务同步)
    renderer_config_helper::write_int(ctx, panel, prop.as_deref(), value);
    // 注意: 不调 ctx.on_save() — 落盘由拖拽释放 → Message::Save 承担 (拖拽时机保真)
}

#[cfg(test)]
mod tests;
