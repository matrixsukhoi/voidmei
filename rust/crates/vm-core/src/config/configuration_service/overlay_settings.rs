//! GenericOverlaySettingsImpl — Java ConfigurationService 非静态内部类的独立 struct
//! (波11 自 configuration_service.rs 三分拆出; 经 `use super::*` 取父模块私有面,
//! 子模块对父模块项天然可见)。

use super::*;

// =====================================================================
// GenericOverlaySettingsImpl — Java 非静态内部类 → 独立 struct
// =====================================================================

/// Java: `private class GenericOverlaySettingsImpl implements OverlaySettings`
///
/// PORT: 内部类持外部类实例 (ConfigurationService.this) → 持共享内核 Arc。
pub struct GenericOverlaySettingsImpl {
    /// Java: `protected final String sectionName`
    pub(crate) section_name: String,
    pub(super) service: Arc<ServiceInner>,
    /// trait get_group_config 的借出载体: 视图构建时的分组快照。
    /// PORT: Java 每次调用 getGroupConfig() 重查并返回**活引用**; RwLock 内
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

    /// Java: `public int getWindowX(int width)`
    fn get_window_x(&self, width: i32) -> i32 {
        let gc = self.get_group_config_snapshot();
        let (screen_w, _) = self.service.screen_size();
        if let Some(gc) = gc {
            // PORT §2.3: Math.round(double)=floor(x+0.5); §2.2: (int) 窄化 =
            // 低 32 位 (双转复刻)
            let px = ((gc.x * f64::from(screen_w) + 0.5).floor() as i64) as u32 as i32;
            // PORT: Java String.format %.4f (HALF_UP) vs Rust {:.4} (半偶) —
            // 仅第 5 位小数恰为半点时日志文本有差异 (debug 级, 坐标域罕见)
            logger::debug(
                "OverlaySettings",
                &format!(
                    "[{}] getWindowX: gc.x={:.4}, screen={} => {}",
                    self.section_name, gc.x, screen_w, px
                ),
            );
            return px;
        }
        // Java int 除法 (向零截断)
        let cx = (screen_w - width) / 2;
        logger::debug(
            "OverlaySettings",
            &format!(
                "[{}] getWindowX: gc=null => center {}",
                self.section_name, cx
            ),
        );
        cx
    }

    /// Java: `public int getWindowY(int height)`
    fn get_window_y(&self, height: i32) -> i32 {
        let gc = self.get_group_config_snapshot();
        let (_, screen_h) = self.service.screen_size();
        if let Some(gc) = gc {
            // 同 getWindowX 的 round/窄化复刻
            let py = ((gc.y * f64::from(screen_h) + 0.5).floor() as i64) as u32 as i32;
            logger::debug(
                "OverlaySettings",
                &format!(
                    "[{}] getWindowY: gc.y={:.4}, screen={} => {}",
                    self.section_name, gc.y, screen_h, py
                ),
            );
            return py;
        }
        let cy = (screen_h - height) / 2;
        logger::debug(
            "OverlaySettings",
            &format!(
                "[{}] getWindowY: gc=null => center {}",
                self.section_name, cy
            ),
        );
        cy
    }

    /// Java: `public void saveWindowPosition(double x, double y)`
    fn save_window_position(&self, x: f64, y: f64) {
        let (screen_w, screen_h) = self.service.screen_size();
        // PORT: 查找+判定+写回收敛在同一写锁作用域 (Java 引用一经获取即稳定)
        let mut wrote: Option<(f64, f64)> = None;
        let mut gc_ok = false;
        {
            let mut configs = self.service.layout_configs.write().expect(LC_LOCK_MSG);
            if let Some(list) = configs.as_mut() {
                for gc in list.iter_mut() {
                    if self.section_name.eq_ignore_ascii_case(&gc.title) {
                        gc_ok = true;
                        if screen_w > 0 && screen_h > 0 {
                            gc.x = x / f64::from(screen_w);
                            gc.y = y / f64::from(screen_h);
                            wrote = Some((gc.x, gc.y));
                        }
                        break;
                    }
                }
            }
        }
        if let Some((rx, ry)) = wrote {
            logger::debug(
                "OverlaySettings",
                &format!(
                    "[{}] saveWindowPosition: {:.6},{:.6} => rel {:.4},{:.4}",
                    self.section_name, x, y, rx, ry
                ),
            );
            self.service.save_layout_config();
        } else {
            logger::warn(
                "OverlaySettings",
                &format!(
                    "[{}] CANNOT save position: gc={}, screen={}x{}",
                    self.section_name,
                    if gc_ok { "OK" } else { "null" },
                    screen_w,
                    screen_h
                ),
            );
        }
    }

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
