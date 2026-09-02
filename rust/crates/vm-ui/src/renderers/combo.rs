//! ComboRowRenderer 的写回链语义复刻 (src/ui/layout/renderer/ComboRowRenderer.java)。
//!
//! **D9 变更**: 原 iced view_row 已删 (渲染归 vm-webui web 壳), 本模块仅存
//! 选项解析 (resolve_options) + 读链 (read_current) + 写链 (apply)。
//!
//! 交互语义保真:
//! - 选项解析 (Java getComboOptions L68-87): ":source" 存于 row.format (loader 覆写);
//!   "_FONTS_" / "_CROSSHAIRS_" 特例源, 其余按逗号字面量拆分。
//! - 读 (Java L30-33): readString 优先级 PropertyBinder → 服务 → row.getStr();
//!   空值不预选 (Java `currentVal != null && !currentVal.isEmpty()` 守卫)。
//! - 写 (Java L52-62): row.value 存新串 → writeString (组字段 fontName + 服务同步) → onSave。
//!
//! PORT: "_FONTS_" 的 AWT 系统字体族枚举无 Rust 对应物 (D1 期以当前值单选占位,
//! 显示不回退); Java 下拉弹出互斥逻辑 (registerComboBox/dismissActivePopups) 属
//! 窗口管理层, 不迁移。

use vm_core::config_loader::{ConfigValue, GroupConfig, RowConfig};
use crate::renderer_config_helper;
use crate::row_renderer_registry::RenderContext;

use super::{find_row_path, row_by_path, row_by_path_mut};

/// 准星选项头部项 (Java L81-83: combined[0] = "软件渲染准星")
const SOFTWARE_CROSSHAIR: &str = "软件渲染准星";
/// Java L75: `File dir = new File("image/gunsight")` — 相对 CWD
const CROSSHAIR_DIR: &str = "image/gunsight";

/// 解析下拉选项 (Java getComboOptions)。current 仅为 _FONTS_ 占位所需。
pub fn resolve_options(source: &str, current: &str) -> Vec<String> {
    match source {
        "_FONTS_" => vec![current.to_string()],
        "_CROSSHAIRS_" => crosshair_options(CROSSHAIR_DIR),
        // Java L86: optionSource.split(",") — 空串 → [""] (与 Java split 逐位一致)
        _ => source.split(',').map(str::to_string).collect(),
    }
}

/// Java L76-85: 目录条目名去扩展名 + 头部"软件渲染准星"; 目录缺失 → 仅头部
/// (dir.list() == null → files = new String[0])。dir 参数仅为测试可注入, 生产恒
/// [`CROSSHAIR_DIR`]。
pub(crate) fn crosshair_options(dir: &str) -> Vec<String> {
    let mut opts = vec![SOFTWARE_CROSSHAIR.to_string()];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(stripped) = vm_core::file_utils::get_file_name_no_ex(Some(&name)) {
                opts.push(stripped.to_string());
            }
        }
    }
    opts
}

/// 当前选中串 (Java L30: readString(ctx, groupConfig, row, row.getStr()))。
pub fn read_current(row: &RowConfig, panel: &GroupConfig, ctx: &dyn RenderContext) -> String {
    renderer_config_helper::read_string(ctx, panel, row, &row.get_str())
}

/// 选中写回 (Java combo.addActionListener 闭包体 L52-62)。
pub fn apply(panel: &mut GroupConfig, key: &str, value: &str, ctx: &dyn RenderContext) {
    let Some(path) = find_row_path(&panel.rows, key) else {
        return;
    };
    let prop = row_by_path(&panel.rows, &path).expect("find_row_path 已定位").property.clone();
    // Java L57-58: Update memory model so it saves to ui_layout.cfg
    row_by_path_mut(&mut panel.rows, &path)
        .expect("find_row_path 已定位")
        .value = Some(ConfigValue::Str(value.to_string()));
    // Java L60: writeString (PropertyBinder 组字段 fontName + 服务同步)
    renderer_config_helper::write_string(ctx, panel, prop.as_deref(), value);
    ctx.on_save();
}

#[cfg(test)]
mod tests;
