//! SwitchRowRenderer / SwitchInvRowRenderer 的 iced 语义复刻
//! (src/ui/layout/renderer/SwitchRowRenderer.java + SwitchInvRowRenderer.java)。
//!
//! 交互语义保真:
//! - SWITCH: 显示值 = readBool(PropertyBinder 组字段 → ConfigurationService → 默认);
//!   写回 = writeBool (组字段 + 服务同步), 绑定失败回落 row.value (Java L64-66)。
//! - SWITCH_INV: 显示值 = !服务值 (配置 "disableX=true" → 开关 OFF);
//!   写回 = 显示值取反落库 + row.value 存显示值 (Java L33-38)。
//!
//! PORT: Java 的 gpuCompatibilityMode 特例 (SwitchRowRenderer.java:19,28-59, 经
//! GPUCompatibilityHelper 存独立文件 + 重启对话框) 不迁移 — 该类在 D7 弃译清单
//! (iced 软件渲染本身即 GPU 兼容路线, 无 AWT 预启动约束)。

use iced::widget::{checkbox, Row};
use iced::Element;
use vm_core::config_loader::{ConfigValue, GroupConfig, RowConfig};
use vm_core::renderer_config_helper;
use vm_core::row_renderer_registry::RenderContext;

use super::{find_row_path, row_by_path, row_by_path_mut};
use crate::main_form::Message;

/// 显示值 (对位 Java render 期的 currentVal):
/// - SWITCH: RendererConfigHelper.readBool 优先级 PropertyBinder → 服务 → row.getBool()
///   (SwitchRowRenderer.java:23-33)
/// - SWITCH_INV: !getFromConfigService(prop, !row.getBool()) (SwitchInvRowRenderer.java:24)
pub fn read_display(row: &RowConfig, panel: &GroupConfig, ctx: &dyn RenderContext) -> bool {
    if row.r#type == "SWITCH_INV" {
        // prop 缺失域内不可达 (Java 直接 NPE), 折叠为 row.value 的显示值
        match row.property.as_deref() {
            Some(p) => !ctx.get_from_config_service(p, !row.get_bool()),
            None => row.get_bool(),
        }
    } else {
        // row.get_bool() 对 null value 抛 NPE 无 catch (Java 同, loader 保证 SWITCH 恒有值)
        renderer_config_helper::read_bool(ctx, panel, row, row.get_bool())
    }
}

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
        // Java SwitchInvRowRenderer.java:33-38: 显示 ON → 存 false (取反转储)
        if let Some(p) = prop.as_deref() {
            ctx.sync_to_config_service(p, !display_val);
        }
        row_by_path_mut(&mut panel.rows, &path)
            .expect("find_row_path 已定位")
            .value = Some(ConfigValue::Bool(display_val)); // row.value 存显示值
    } else {
        // Java SwitchRowRenderer.java:64-66: writeBool (PropertyBinder + 服务同步);
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

/// 复选框行视图。消息键: :target 优先; 无 :target 以 label 为键 (Java 闭包
/// prop=null 仍写 row.value + onSave — [`apply`] 的 write_bool(None) 回落分支承接;
/// property/label 双空残端 → 纯展示)。
pub fn view_row<'a>(
    row: &'a RowConfig,
    panel: &'a GroupConfig,
    ctx: &dyn RenderContext,
    panel_title: &'a str,
) -> Element<'a, Message> {
    let current = read_display(row, panel, ctx);
    let key = row
        .property
        .clone()
        .or_else(|| (!row.label.is_empty()).then(|| row.label.clone()));
    let cb = match key {
        Some(key) => {
            let p = panel_title.to_string();
            checkbox(row.label.clone(), current)
                .on_toggle(move |v| Message::Toggle { panel: p.clone(), key: key.clone(), value: v })
        }
        None => checkbox(row.label.clone(), current),
    };
    // Java createSwitchItem: [label ... switch]; iced checkbox 自带标签, 包 Row 留位对齐
    Row::with_children(vec![cb.into()]).spacing(8).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderers::test_util::MapCtx;
    use vm_core::config_loader::ConfigValue;

    fn switch_row(prop: Option<&str>, value: Option<bool>) -> RowConfig {
        let mut r = RowConfig::new("开关".into(), None, "%s".into());
        r.r#type = "SWITCH".into();
        r.property = prop.map(str::to_string);
        r.value = value.map(ConfigValue::Bool);
        r
    }

    fn inv_row(prop: &str, value: bool) -> RowConfig {
        let mut r = switch_row(Some(prop), Some(value));
        r.r#type = "SWITCH_INV".into();
        r
    }

    // SWITCH 读路径: 服务值压制 row 默认 (SwitchRowRenderer.java:30-33)
    #[test]
    fn read_display_switch_prefers_service() {
        let row = switch_row(Some("showSpeedBar"), Some(false)); // 默认 false
        let mut ctx = MapCtx::default();
        ctx.set("showSpeedBar", "true");
        let panel = GroupConfig::new("p".into());
        assert!(read_display(&row, &panel, &ctx));
        // 服务空值 → 回落 row.getBool() 默认
        let ctx2 = MapCtx::default();
        assert!(!read_display(&row, &panel, &ctx2));
    }

    // SWITCH 读路径: PropertyBinder 组字段 (visible) 压制服务同键 (read_bool 优先级 1)
    #[test]
    fn read_display_switch_group_field_wins() {
        let mut row = switch_row(Some("visible"), Some(false));
        row.value = Some(ConfigValue::Bool(false));
        let mut panel = GroupConfig::new("p".into());
        panel.visible = true; // 组字段
        let mut ctx = MapCtx::default();
        ctx.set("visible", "false"); // 服务侧 false 应被组字段压制
        assert!(read_display(&row, &panel, &ctx));
    }

    // SWITCH_INV 读路径: 双重取反 (配置 true=禁用 → 显示 OFF)
    #[test]
    fn read_display_switch_inv_double_inversion() {
        let row = inv_row("disableX", true); // row.value=true (显示值语义)
        let mut ctx = MapCtx::default();
        ctx.set("disableX", "true"); // 服务: 禁用
        let panel = GroupConfig::new("p".into());
        assert!(!read_display(&row, &panel, &ctx), "disableX=true → 显示 OFF");
        ctx.set("disableX", "false");
        assert!(read_display(&row, &panel, &ctx), "disableX=false → 显示 ON");
        // 服务空 → !(!row.getBool()) = row.getBool()
        let ctx2 = MapCtx::default();
        assert!(read_display(&row, &panel, &ctx2));
    }

    // SWITCH 写回: 非组字段 → write_bool 返回 false → row.value 回落 (Java L64-66)
    #[test]
    fn apply_switch_falls_back_to_row_value() {
        let mut panel = GroupConfig::new("p".into());
        let mut r = switch_row(Some("crosshairSwitch"), Some(true));
        r.value = Some(ConfigValue::Bool(true));
        panel.rows.push(r);
        let ctx = MapCtx::default();

        apply(&mut panel, "crosshairSwitch", false, &ctx);
        let row = row_by_path(&panel.rows, &[0]).unwrap();
        assert_eq!(row.value, Some(ConfigValue::Bool(false)));
        // write_bool 总同步服务 (RendererConfigHelper.java:127-134)
        assert_eq!(*ctx.calls.borrow(), vec!["sync:crosshairSwitch=false".to_string(), "on_save".to_string()]);
    }

    // SWITCH_INV 写回: 显示 true → 存 false; row.value=显示值 (Java L33-38)
    #[test]
    fn apply_switch_inv_stores_inverted() {
        let mut panel = GroupConfig::new("p".into());
        panel.rows.push(inv_row("disableLabel", false));
        let ctx = MapCtx::default();

        apply(&mut panel, "disableLabel", true, &ctx);
        let row = row_by_path(&panel.rows, &[0]).unwrap();
        assert_eq!(row.value, Some(ConfigValue::Bool(true))); // row.value=显示值
        assert_eq!(
            *ctx.calls.borrow(),
            vec!["sync:disableLabel=false".to_string(), "on_save".to_string()]
        );
    }

    // 组字段绑定开关: write_bool 命中 PropertyBinder, row.value 不回落
    #[test]
    fn apply_switch_binds_group_field() {
        let mut panel = GroupConfig::new("p".into());
        panel.rows.push(switch_row(Some("visible"), Some(false)));
        let ctx = MapCtx::default();

        apply(&mut panel, "visible", true, &ctx);
        assert!(panel.visible, "PropertyBinder 写组字段 visible");
        assert_eq!(panel.rows[0].value, Some(ConfigValue::Bool(false)), "绑定成功不回落 row.value");
    }

    // 未命中 key: 无副作用无 panic (消息域外防护, Java 闭包捕获无此面)
    #[test]
    fn apply_unknown_key_is_noop() {
        let mut panel = GroupConfig::new("p".into());
        panel.rows.push(switch_row(Some("k"), Some(true)));
        let ctx = MapCtx::default();
        apply(&mut panel, "absent", false, &ctx);
        assert_eq!(panel.rows[0].value, Some(ConfigValue::Bool(true)));
        assert!(ctx.calls.borrow().is_empty());
    }
}
