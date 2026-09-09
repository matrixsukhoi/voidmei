//! GenericOverlaySettingsImpl — Java ConfigurationService 非静态内部类的独立 struct
//! (波11 自 configuration_service.rs 三分拆出; 经 `use super::*` 取父模块私有面,
//! 子模块对父模块项天然可见)。

use super::*;

// =====================================================================
// GenericOverlaySettingsImpl — Java 非静态内部类 → 独立 struct
// =====================================================================

/// Java: `private class GenericOverlaySettingsImpl implements OverlaySettings`
///
/// 内部类持外部类实例 (ConfigurationService.this) → 持共享内核 Arc。
pub struct GenericOverlaySettingsImpl {
    /// Java: `protected final String sectionName`
    pub(crate) section_name: String,
    pub(super) service: Arc<ServiceInner>,
    /// trait get_group_config 的借出载体: 视图构建时的分组快照。
    /// Java 每次调用 getGroupConfig() 重查并返回**活引用**; RwLock 内
    /// 存储无法经 &self 借出引用 — 本 trait 方法返回构建时快照 (config_api
    /// 注释认可的快照契约); 其余读取方法均逐调用重查保真 (见
    /// get_group_config_snapshot)。需最新快照的调用方重建视图 (Java 端
    /// ui.model 消费方即构建期 populate, 语义等价)。
    group_snapshot: Option<GroupConfig>,
}

impl GenericOverlaySettingsImpl {
    /// Java: `public GenericOverlaySettingsImpl(String sectionName)`
    /// (私有构造: 视图仅经 getOverlaySettings 工厂产出, 同模块内调用)
    pub(super) fn new(service: Arc<ServiceInner>, section_name: &str) -> Self {
        let group_snapshot = service.find_group_ignore_case(section_name);
        GenericOverlaySettingsImpl {
            section_name: section_name.to_string(),
            service,
            group_snapshot,
        }
    }

    /// Java getGroupConfig() 的重查体 (供本视图各读取方法逐调用取最新状态)
    pub(super) fn get_group_config_snapshot(&self) -> Option<GroupConfig> {
        self.service.find_group_ignore_case(&self.section_name)
    }
}

impl OverlaySettings for GenericOverlaySettingsImpl {
    type GroupConfig = GroupConfig;

    /// Java: `public GroupConfig getGroupConfig()` — 见 group_snapshot 字段注释
    fn get_group_config(&self) -> Option<&GroupConfig> {
        self.group_snapshot.as_ref()
    }

    // 位置面 (get_window_x/y, save_window_position) 已随 R2 位置链重构退役:
    // 窗口位置唯一真源 = PageDoc.pos, host 侧 PagePositionStore 存档

    /// Java: `public String getFontName()`
    fn get_font_name(&self) -> String {
        let gc = self.get_group_config_snapshot();
        if let Some(gc) = &gc {
            if let Some(fname) = &gc.font_name {
                if !fname.is_empty() {
                    return fname.clone();
                }
            }
        }
        let global_font = self.service.get_config_j("GlobalTextFont");
        if !global_font.is_empty() {
            return global_font;
        }
        self.service.app_default_font_name()
    }

    /// Java: `public String getNumFontName()`
    fn get_num_font_name(&self) -> String {
        let global_font = self.service.get_config_j("GlobalNumFont");
        if !global_font.is_empty() {
            return global_font;
        }
        self.service.app_default_numfont_name()
    }

    /// Java: `public int getFontSizeAdd()`
    fn get_font_size_add(&self) -> i32 {
        let gc = self.get_group_config_snapshot();
        match gc {
            Some(gc) => gc.font_size,
            None => 0,
        }
    }

    /// Java: `public boolean getBool(String key, boolean def)`
    fn get_bool(&self, key: &str, def: bool) -> bool {
        let gc = self.get_group_config_snapshot();
        if let Some(gc) = gc {
            if let Some(row) = find_row_recursive(&gc.rows, key) {
                // Handle inversion for SWITCH_INV
                if row.r#type == "SWITCH_INV" {
                    return !row.get_bool();
                }
                return row.get_bool();
            }
        }
        let val = self.service.get_config_j(key);
        if val.is_empty() {
            return def;
        }
        java_parse_boolean(&val)
    }

    /// Java: `public int getInt(String key, int def)`
    fn get_int(&self, key: &str, def: i32) -> i32 {
        let gc = self.get_group_config_snapshot();
        if let Some(gc) = gc {
            if let Some(row) = find_row_recursive(&gc.rows, key) {
                return row.get_int();
            }
        }
        let val = self.service.get_config_j(key);
        if val.is_empty() {
            return def;
        }
        val.parse::<i32>().unwrap_or(def)
    }

    /// Java: `public String getString(String key, String def)`
    fn get_string(&self, key: &str, def: &str) -> String {
        let gc = self.get_group_config_snapshot();
        if let Some(gc) = gc {
            if let Some(row) = find_row_recursive(&gc.rows, key) {
                return row.get_str();
            }
        }
        let val = self.service.get_config_j(key);
        if val.is_empty() {
            return def.to_string();
        }
        val
    }

    /// Java: `public boolean autoHideOnFocusLoss()`
    fn auto_hide_on_focus_loss(&self) -> bool {
        // 从全局设置读取配置，默认关闭
        self.get_bool("autoHideOnFocusLoss", false)
    }
}
