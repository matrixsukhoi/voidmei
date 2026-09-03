//! ComboRowRenderer 的写回链语义复刻 (src/ui/layout/renderer/ComboRowRenderer.java)。
//!
//! **D9 变更**: 渲染与读链已删 (归 vm-webui web 壳, 回显走整树 DTO),
//! 本模块仅存选项解析 (resolve_options) + 写链 (apply)。
//!
//! 语义保真:
//! - 选项解析 (Java getComboOptions): ":source" 存于 row.format (loader 覆写);
//!   "_FONTS_" / "_CROSSHAIRS_" 特例源, 其余按逗号字面量拆分。
//! - 写: row.value 存新串 → writeString (组字段 fontName + 服务同步) → onSave。
//! - INPUT/TEXT 文本行同走本写链 (经 Message::Combo 路由, 等价备案见 tests)。
//!
//! "_FONTS_" 的 AWT 系统字体族枚举无 Rust 对应物 (以当前值单选占位,
//! 显示不回退); Java 下拉弹出互斥逻辑 (registerComboBox/dismissActivePopups) 属
//! 窗口管理层, 不迁移。

use crate::render_context::RenderContext;
use crate::renderer_config_helper;
use vm_core::config::config_loader::{ConfigValue, GroupConfig};

use super::{find_row_path, row_by_path, row_by_path_mut};

/// 准星选项头部项 (Java: combined[0] = "软件渲染准星")
const SOFTWARE_CROSSHAIR: &str = "软件渲染准星";
/// Java: `File("image/gunsight")` — 相对 CWD
const CROSSHAIR_DIR: &str = "image/gunsight";

/// 解析下拉选项 (Java getComboOptions)。current 仅为 _FONTS_ 占位所需。
pub fn resolve_options(source: &str, current: &str) -> Vec<String> {
    match source {
        "_FONTS_" => vec![current.to_string()],
        "_CROSSHAIRS_" => crosshair_options(CROSSHAIR_DIR),
        // optionSource.split(",") — 空串 → [""] (与 Java split 逐位一致)
        _ => source.split(',').map(str::to_string).collect(),
    }
}

/// 目录条目名去扩展名 + 头部"软件渲染准星"; 目录缺失 → 仅头部
/// (Java dir.list() == null → files = new String[0])。dir 参数仅为测试可注入, 生产恒
/// [`CROSSHAIR_DIR`]。
pub(crate) fn crosshair_options(dir: &str) -> Vec<String> {
    let mut opts = vec![SOFTWARE_CROSSHAIR.to_string()];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(stripped) = vm_core::base::file_utils::get_file_name_no_ex(Some(&name)) {
                opts.push(stripped.to_string());
            }
        }
    }
    opts
}

/// 选中写回 (Java combo.addActionListener 闭包体)。
pub fn apply(panel: &mut GroupConfig, key: &str, value: &str, ctx: &dyn RenderContext) {
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
        .value = Some(ConfigValue::Str(value.to_string()));
    // writeString (PropertyBinder 组字段 fontName + 服务同步)
    renderer_config_helper::write_string(ctx, panel, prop.as_deref(), value);
    ctx.on_save();
}

#[cfg(test)]
mod tests;
