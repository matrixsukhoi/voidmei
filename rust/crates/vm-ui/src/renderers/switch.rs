//! SwitchRowRenderer / SwitchInvRowRenderer 的写回链语义复刻
//! (src/ui/layout/renderer/SwitchRowRenderer.java + SwitchInvRowRenderer.java)。
//!
//! **D9 变更**: 渲染与读链已删 (归 vm-webui web 壳, 回显走整树 DTO),
//! 本模块仅存写链 (apply)。
//!
//! 写回语义保真:
//! - SWITCH: writeBool (组字段 + 服务同步), 绑定失败回落 row.value。
//! - SWITCH_INV: 显示值取反落库 + row.value 存显示值。
//! - DATA 行开关同走本写链 (经 Message::Toggle 路由, 等价备案见 tests)。
//!
//! Java 的 gpuCompatibilityMode 特例 (SwitchRowRenderer, 经
//! GPUCompatibilityHelper 存独立文件 + 重启对话框) 不迁移 — 该类在 D7 弃译清单。

use crate::render_context::RenderContext;
use crate::renderer_config_helper;
use vm_core::config::config_loader::{ConfigValue, GroupConfig};

use super::{find_row_path, row_by_path, row_by_path_mut};

/// 开关翻转写回 (对位 Java sw.addActionListener 闭包体, value 为**显示值**)。
pub fn apply(panel: &mut GroupConfig, key: &str, display_val: bool, ctx: &dyn RenderContext) {
    let Some(path) = find_row_path(&panel.rows, key) else {
        return;
    };
    let (rtype, prop) = {
        let r = row_by_path(&panel.rows, &path).expect("find_row_path 已定位");
        (r.r#type.clone(), r.property.clone())
    };

    if rtype == "SWITCH_INV" {
        // Java SwitchInvRowRenderer: 显示 ON → 存 false (取反转储)
        if let Some(p) = prop.as_deref() {
            ctx.sync_to_config_service(p, !display_val);
        }
        row_by_path_mut(&mut panel.rows, &path)
            .expect("find_row_path 已定位")
            .value = Some(ConfigValue::Bool(display_val)); // row.value 存显示值
    } else {
        // Java SwitchRowRenderer: writeBool (PropertyBinder + 服务同步);
        // 绑定失败 (非组字段属性) 时 row.value 回落存值
        let bound = renderer_config_helper::write_bool(ctx, panel, prop.as_deref(), display_val);
        if !bound {
            row_by_path_mut(&mut panel.rows, &path)
                .expect("find_row_path 已定位")
                .value = Some(ConfigValue::Bool(display_val));
        }
    }
    ctx.on_save();
}

#[cfg(test)]
mod tests;
