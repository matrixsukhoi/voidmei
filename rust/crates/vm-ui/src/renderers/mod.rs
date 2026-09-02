//! RowRenderer 族的**写回链 + 纯数据层** (对应 src/ui/layout/renderer/ + RowRendererRegistry.java)。
//!
//! **D9 变更**: 原 iced view 分发层 (view_row/fallback_row 及各渲染器 view_row) 已删
//! — 表单渲染归 vm-webui web 壳。本层保留:
//! - 各类型 apply 写回链 (`switch::apply`/`slider::apply`/`combo::apply`/`color::apply`,
//!   对位 Java renderer 闭包体的配置写路径);
//! - 纯数据函数 (`combo::resolve_options`/`slider::effective_range`/`color` 解析与
//!   格式化/`color_picker` HSB↔RGB 数学);
//! - 行定位助手 (find_row_path/row_by_path, main_form 消息定位在用)。
//!
//! Java 各渲染器的读链 (read_display/read_current) 一并保留 — web 壳回显当前值
//! 需要与 Java 同优先级的取值链 (PropertyBinder → 服务 → 行值)。
//! PORT: 已注册未落地的渲染器 (HOTKEY/VOICE/FILELIST/FMLIST/VOICE_GLOBAL/INFO) 的
//! 专属交互依赖系统钩子与音频子系统; 未知类型对位 Java defaultRenderer=
//! DataRowRenderer (点击写配置, 走 `data::read_display` 同键规则)。

pub mod color;
pub mod color_picker;
pub mod combo;
pub mod data;
pub mod slider;
pub mod switch;
pub mod text;

use vm_core::config::config_loader::RowConfig;

// =====================================================================
// 行定位助手 (消息 key → 行路径)
// =====================================================================

/// 在行树内按 :target (property) DFS 定位行, 返回索引路径; 无 property 的行以
/// label 匹配 (与服务侧 update_rows_recursive 同一命中谓词 — 无 :target 控件以
/// label 为消息键)。消息 key 来自行自身 (视图闭包捕获), 恒可命中。
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

    use vm_core::config::configuration_service::ConfigurationService;
    use crate::row_renderer_registry::RenderContext;

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
        // 掺 PID: 防两个测试进程并发跑时同名临时文件 truncate/read 竞争 (config_loader 实测同款踩坑)
        let p = std::env::temp_dir().join(format!("vm_ui_renderers_{}_{name}.cfg", std::process::id()));
        std::fs::write(&p, cfg).unwrap();
        let bus = Arc::new(vm_core::base::bus::ui_state_bus::UIStateBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&bus)));
        config.load_layout(p.to_str().unwrap());
        crate::main_form::MainFormState::new(config, bus, persist)
    }
}

#[cfg(test)]
mod tests;
