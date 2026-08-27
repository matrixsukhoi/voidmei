//! ComboRowRenderer 的 iced 语义复刻 (src/ui/layout/renderer/ComboRowRenderer.java)。
//!
//! 交互语义保真:
//! - 选项解析 (Java getComboOptions L68-87): ":source" 存于 row.format (loader 覆写);
//!   "_FONTS_" / "_CROSSHAIRS_" 特例源, 其余按逗号字面量拆分。
//! - 读 (Java L30-33): readString 优先级 PropertyBinder → 服务 → row.getStr();
//!   空值不预选 (Java `currentVal != null && !currentVal.isEmpty()` 守卫)。
//! - 写 (Java L52-62): row.value 存新串 → writeString (组字段 fontName + 服务同步) → onSave。
//!
//! PORT: "_FONTS_" 的 AWT 系统字体族枚举无 iced 对应物 (软件渲染无 GraphicsEnvironment,
//! 字体子系统另批) → 以当前值单选占位 (显示不回退); Java 下拉弹出互斥逻辑
//! (registerComboBox/dismissActivePopups) 由 iced 窗口系统天然承担, 不迁移。

use iced::widget::{pick_list, Row};
use iced::Element;
use vm_core::config_loader::{ConfigValue, GroupConfig, RowConfig};
use vm_core::renderer_config_helper;
use vm_core::row_renderer_registry::RenderContext;

use super::{find_row_path, row_by_path, row_by_path_mut};
use crate::main_form::Message;

/// 准星选项头部项 (Java L81-83: combined[0] = "软件渲染准星")
const SOFTWARE_CROSSHAIR: &str = "软件渲染准星";
/// Java L75: `File dir = new File("image/gunsight")` — 相对 CWD
const CROSSHAIR_DIR: &str = "image/gunsight";

/// 解析下拉选项 (Java getComboOptions)。current 仅为 _FONTS_ 占位所需。
pub fn resolve_options(source: &str, current: &str) -> Vec<String> {
    match source {
        // Java: optionSource == null → new String[0]; format 恒非 null, 空串走 split 域
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

/// 下拉行视图 (pick_list 对位 WebComboBox)。
pub fn view_row<'a>(
    row: &'a RowConfig,
    panel: &'a GroupConfig,
    ctx: &dyn RenderContext,
    panel_title: &'a str,
    options_of: &dyn Fn(&str, &str) -> Vec<String>,
) -> Element<'a, Message> {
    let current = read_current(row, panel, ctx);
    // Java L32-34: 空值不预选
    let selected = if current.is_empty() { None } else { Some(current) };
    let options = options_of(&row.format, selected.as_deref().unwrap_or_default());

    // 消息键: :target 优先; 无 :target 以 label 为键 (Java L52-61 对 prop=null 仍写
    // row.value + onSave, writeString(null) 不落服务); property/label 双空残端 → Ignore
    let key = row
        .property
        .clone()
        .or_else(|| (!row.label.is_empty()).then(|| row.label.clone()));
    let list_el: Element<'a, Message> = match key {
        Some(key) => {
            let p = panel_title.to_string();
            pick_list(options, selected, move |v| Message::Combo {
                panel: p.clone(),
                key: key.clone(),
                value: v,
            })
            .into()
        }
        None => pick_list(options, selected, |_| Message::Ignore).into(),
    };
    Row::with_children(vec![
        iced::widget::text(row.label.clone()).into(),
        list_el,
    ])
    .spacing(8)
    .into()
}

#[cfg(test)]
mod tests;
