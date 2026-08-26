//! RowRenderer 族的 iced 语义复刻分发层 (对应 src/ui/layout/renderer/ + RowRendererRegistry.java)。
//!
//! Java: DynamicDataPage.buildContainer (DynamicDataPage.java:184-243) 按 row.type 经
//! RowRendererRegistry.get 取策略渲染器; 本层按同键集合分发到各渲染器模块
//! (SWITCH/SWITCH_INV/SLIDER/COMBO/COLOR/BUTTON/DATA/TEXT/INPUT 九键 + COLOR 弹层
//! color_picker)。
//! PORT: 已注册未落地的渲染器 (HOTKEY/VOICE/FILELIST/FMLIST/VOICE_GLOBAL/INFO) 走
//! [`fallback_row`] 只读占位 (Java 各有专属渲染器, 语义面见其 <clinit> 注册表;
//! 热键/语音/文件列表依赖系统钩子与音频子系统, 批十三接线)。**未知类型**走
//! data::view_row 交互开关 — 对位 Java defaultRenderer=DataRowRenderer 的完整语义
//! (DataRowRenderer.java:25-31: WebSwitch 点击即 syncStringToConfigService + onSave,
//! 非只读)。

pub mod button;
pub mod color;
pub mod color_picker;
pub mod combo;
pub mod data;
pub mod slider;
pub mod switch;
pub mod text;

use iced::widget::{text as text_widget, tooltip, Row};
use iced::{Element, Length};
use vm_core::config_loader::{GroupConfig, RowConfig};
use vm_core::row_renderer_registry::RenderContext;

use crate::main_form::Message;

/// 渲染一行配置项为 iced 元素 (对位 Java renderer.render(row, groupConfig, ctx))。
///
/// @param row          行配置 (ui_layout.cfg 的 (item ...))
/// @param panel        行所属 panel 的 GroupConfig (PropertyBinder 字段绑定目标,
///                     Java buildContainer 传入的 groupConfig 即 panel 级)
/// @param ctx          读侧上下文 (view 纯函数, 只读)
/// @param panel_title  行所属 panel 标题 (消息定位, 对位 Java 闭包捕获)
/// @param options_of   下拉选项解析器 (source, current) → options (含磁盘缓存, main_form 注入)
pub fn view_row<'a>(
    row: &'a RowConfig,
    panel: &'a GroupConfig,
    ctx: &dyn RenderContext,
    panel_title: &'a str,
    options_of: &dyn Fn(&str, &str) -> Vec<String>,
) -> Element<'a, Message> {
    let el: Element<'a, Message> = match row.r#type.as_str() {
        // RowRendererRegistry.java <clinit> 的键集合 (TEXT/INPUT 为同渲染器别名, L25-26)
        "SWITCH" | "SWITCH_INV" => switch::view_row(row, panel, ctx, panel_title),
        "SLIDER" => slider::view_row(row, panel, ctx, panel_title),
        "COMBO" => combo::view_row(row, panel, ctx, panel_title, options_of),
        "COLOR" => color::view_row(row, ctx, panel_title),
        "TEXT" | "INPUT" => text::view_row(row, panel, ctx, panel_title),
        "DATA" => data::view_row(row, ctx, panel_title),
        "BUTTON" => button::view_row(row),
        // 已注册未落地的专属渲染器键 (Java 各有实现, 本批只读占位)
        "INFO" | "VOICE" | "VOICE_GLOBAL" | "HOTKEY" | "FMLIST" | "FILELIST" => {
            fallback_row(row, ctx)
        }
        // 未知类型 → Java getOrDefault 兜底 defaultRenderer=DataRowRenderer:
        // 可交互开关, 点击写配置 (非只读, DataRowRenderer.java:25-31)
        _ => data::view_row(row, ctx, panel_title),
    };
    match row.desc.as_deref() {
        // :desc tooltip — Java ReplicaBuilder.createXxxItem(..., desc, descImg) 内建气泡;
        // :desc-img 图片气泡不迁移 (无 AWT 图像加载, 需要时随图片资产批次补)
        Some(d) if !d.is_empty() => {
            tooltip(el, text_widget(d.to_string()), tooltip::Position::FollowCursor).into()
        }
        _ => el,
    }
}

/// 已注册未落地类型的只读占位行: label + 当前配置值/静态值。
/// PORT: 各类型的专属渲染器 (INFO 富文本超链接/热键捕获/语音选择…) 批十三;
/// 未知类型不再走此占位 (改走 data::view_row, 见 view_row 分发注释)。
fn fallback_row<'a>(row: &'a RowConfig, ctx: &dyn RenderContext) -> Element<'a, Message> {
    let value = match row.property.as_deref() {
        Some(p) => ctx.get_string_from_config_service(p, &row.get_str()),
        None => row.get_str(),
    };
    if row.label.is_empty() {
        text_widget(value).into()
    } else {
        Row::with_children(vec![
            text_widget(row.label.clone()).width(Length::Fill).into(),
            text_widget(value).into(),
        ])
        .spacing(8)
        .into()
    }
}

// =====================================================================
// 行定位助手 (消息 key → 行路径)
// =====================================================================

/// 在行树内按 :target (property) DFS 定位行, 返回索引路径; 无 property 的行以
/// label 匹配 (与服务侧 update_rows_recursive 同一命中谓词 — 无 :target 控件以
/// label 为消息键)。消息 key 来自行自身 (view 闭包捕获), 恒可命中。
pub(crate) fn find_row_path(rows: &[RowConfig], key: &str) -> Option<Vec<usize>> {
    for (i, r) in rows.iter().enumerate() {
        if r.property.as_deref() == Some(key) || (r.property.is_none() && key == r.label) {
            return Some(vec![i]);
        }
        if !r.children.is_empty() {
            if let Some(mut tail) = find_row_path(&r.children, key) {
                let mut path = vec![i];
                path.append(&mut tail);
                return Some(path);
            }
        }
    }
    None
}

pub(crate) fn row_by_path<'a>(rows: &'a [RowConfig], path: &[usize]) -> Option<&'a RowConfig> {
    let (&first, rest) = path.split_first()?;
    let row = rows.get(first)?;
    if rest.is_empty() {
        Some(row)
    } else {
        row_by_path(&row.children, rest)
    }
}

pub(crate) fn row_by_path_mut<'a>(
    rows: &'a mut [RowConfig],
    path: &[usize],
) -> Option<&'a mut RowConfig> {
    let (&first, rest) = path.split_first()?;
    let row = rows.get_mut(first)?;
    if rest.is_empty() {
        Some(row)
    } else {
        row_by_path_mut(&mut row.children, rest)
    }
}

// =====================================================================
// Tests
// =====================================================================
#[cfg(test)]
pub(crate) mod test_util {
    //! 渲染器单测的最小 RenderContext 替身 (键值表), 对位 Java RenderContext 契约;
    //! 写路径断言走 main_form 测试的真实 ConfigurationService 链, 不在此造假。

    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::Arc;

    use vm_core::bus::EventBus;
    use vm_core::configuration_service::ConfigurationService;
    use vm_core::row_renderer_registry::RenderContext;

    #[derive(Default)]
    pub(crate) struct MapCtx {
        pub values: HashMap<String, String>,
        pub calls: RefCell<Vec<String>>,
    }

    impl MapCtx {
        pub fn set(&mut self, k: &str, v: &str) {
            self.values.insert(k.to_string(), v.to_string());
        }
    }

    impl RenderContext for MapCtx {
        fn on_save(&self) {
            self.calls.borrow_mut().push("on_save".into());
        }
        fn on_rebuild(&self) {
            self.calls.borrow_mut().push("on_rebuild".into());
        }
        fn is_updating(&self) -> bool {
            false
        }
        fn sync_to_config_service(&self, key: &str, value: bool) {
            self.calls
                .borrow_mut()
                .push(format!("sync:{key}={value}"));
        }
        fn get_from_config_service(&self, key: &str, default_val: bool) -> bool {
            // DynamicDataPage.java:155-161 同语义
            match self.values.get(key) {
                Some(v) if !v.is_empty() => v.eq_ignore_ascii_case("true"),
                _ => default_val,
            }
        }
        fn sync_string_to_config_service(&self, key: &str, value: &str) {
            self.calls
                .borrow_mut()
                .push(format!("syncStr:{key}={value}"));
        }
        fn get_string_from_config_service(&self, key: &str, default_val: &str) -> String {
            // DynamicDataPage.java:169-174 同语义
            match self.values.get(key) {
                Some(v) if !v.is_empty() => v.clone(),
                _ => default_val.to_string(),
            }
        }
    }

    /// 真实链路状态工厂 (各渲染器测试共享): cfg 落 tmp → ConfigurationService 装载
    /// → MainFormState。不录总线事件 (需事件断言用例走 main_form::tests 的 mk_state)。
    pub(crate) fn state_from_cfg(
        name: &str,
        cfg: &str,
        persist: Option<String>,
    ) -> crate::main_form::MainFormState {
        let p = std::env::temp_dir().join(format!("vm_ui_renderers_{name}.cfg"));
        std::fs::write(&p, cfg).unwrap();
        let bus = Arc::new(EventBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&bus)));
        config.load_layout(p.to_str().unwrap());
        crate::main_form::MainFormState::new(config, bus, persist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vm_core::config_loader::ConfigValue;

    // find_row_path: 按层定位 + 首个命中 (DFS 前序, 对位消息 key 的唯一来源行)
    #[test]
    fn find_row_path_locates_nested_and_misses() {
        let mut g = GroupConfig::new("p".into());
        let mut header = RowConfig::new("组".into(), None, "%s".into());
        header.r#type = "HEADER".into();
        let mut inner = RowConfig::new("内".into(), None, "%s".into());
        inner.r#type = "SWITCH".into();
        inner.property = Some("k2".into());
        header.children.push(inner);
        let mut top = RowConfig::new("顶".into(), None, "%s".into());
        top.r#type = "SWITCH".into();
        top.property = Some("k1".into());
        g.rows.push(top);
        g.rows.push(header);

        assert_eq!(find_row_path(&g.rows, "k1"), Some(vec![0]));
        assert_eq!(find_row_path(&g.rows, "k2"), Some(vec![1, 0]));
        assert_eq!(find_row_path(&g.rows, "absent"), None);
        // row_by_path / row_by_path_mut 往返
        assert_eq!(row_by_path(&g.rows, &[1, 0]).unwrap().property.as_deref(), Some("k2"));
        row_by_path_mut(&mut g.rows, &[1, 0]).unwrap().label = "改名".into();
        assert_eq!(row_by_path(&g.rows, &[1, 0]).unwrap().label, "改名");
        // 空路径 / 越界
        assert!(row_by_path(&g.rows, &[]).is_none());
        assert!(row_by_path(&g.rows, &[9]).is_none());
    }

    // 分发冒烟: 真实 cfg 树 → view_row 对九键 + 已注册未落地键 + 未知键全部产出
    // 元素 (数据驱动, 对位 RowRendererRegistry.get 的恒有产出 + defaultRenderer 兜底)
    #[test]
    fn view_row_dispatches_all_registered_types() {
        let p = std::env::temp_dir().join("vm_ui_renderers_dispatch.cfg");
        std::fs::write(
            &p,
            r##"(panel "全类型"
  (item "开关" :type switch :target "k1" :value true)
  (item "反相" :type switch-inv :target "k2" :value false)
  (item "滑条" :type slider :target "k3" :min 0 :max 10 :value 5)
  (item "下拉" :type combo :target "k4" :source "A,B" :value "A")
  (item "颜色" :type color :target "fontWarn" :value "#FF2400FF")
  (item "文本" :type input :target "httpPort" :value 8111)
  (item "别名" :type text :value "t")
  (item "数据" :type data :target "getIAS" :value true)
  (item "按钮" :type button :target "factoryReset")
  (item "热键" :type hotkey :target "hudHotkey")
  (item "野类型" :type mystery :target "mkey")
)"##,
        )
        .unwrap();
        let config = vm_core::configuration_service::ConfigurationService::new(None);
        config.load_layout(p.to_str().unwrap());
        let groups = config.get_layout_configs().unwrap();
        let ctx = crate::main_form::ReadContext::new(&config);
        let panel = &groups[0];
        let no_opts = |_: &str, _: &str| Vec::<String>::new();
        for row in &panel.rows {
            // 每类型各产出元素 (panic/错型即败); hotkey 走 fallback_row 占位,
            // mystery 未知类型走 data::view_row (Java defaultRenderer 交互开关)
            let _el = view_row(row, panel, &ctx, &panel.title, &no_opts);
        }
        assert_eq!(panel.rows.len(), 11);
        assert_eq!(panel.rows[4].value, Some(ConfigValue::Str("#FF2400FF".into())));
    }
}
