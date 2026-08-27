//! ConfigurationService 的 Rust 移植 (src/prog/config/ConfigurationService.java) — 一比一翻译。
//!
//! Main configuration Service implementing ConfigProvider.
//! Handles loading, saving, and accessing application configuration.
//!
//! PORT: Java 内部类 GenericOverlaySettingsImpl / HUDSettingsImpl → 独立 struct
//! (任务裁决); 子类对父类的 extends → 组合 + 委托 (§1 继承禁令)。
//! PORT: Application 静态字段 (loadAppCheck 写入面 / OverlaySettings 读取面) 与
//! Controller 轮询间隔字段均属未翻译类 — 以本文件尾部的**消费面依赖桩**顶住
//! (config_manager.rs 的 UIStateStorage 桩先例), 注入式持有, 不造全局 (§2.9)。
//! PORT: UIStateBus 未翻译 (B 类后续批次) — 总线经构造器注入
//! `Option<Arc<EventBus<UiStateEvent>>>`, Java `UIStateBus.getInstance()` 全局
//! 单例读法收敛为依赖注入, 避免状态分裂; 订阅接线被 Rc<SExp> 阻塞, 见
//! init_config 内 PORT 注释 (跨文件问题只标注不越文件修复, §6)。

use std::sync::{Arc, RwLock};

use crate::bus::EventBus;
use crate::config_api::{ConfigProvider, HUDSettings, OverlaySettings};
use crate::config_loader::{self, ConfigValue, GroupConfig, RowConfig};
use crate::config_manager;
use crate::event::ui_state_events;
use crate::lang::Lang;
use crate::logger;

/// RwLock 中毒消息 (Java 无锁; 对应持锁线程崩溃后的一致性未知面)
const LC_LOCK_MSG: &str = "layoutConfigs 锁中毒";
const APP_LOCK_MSG: &str = "Application 状态锁中毒";

// =====================================================================
// UIStateBus 消费面 (依赖桩, 非翻译)
// =====================================================================

/// UIStateBus 消息: Java `publish(eventType, source, data)` 弱类型三元组的
/// 强类型化。eventType = 总线路由键 (Java 端 subscribe 按它分发); data 对应
/// Java `Object` payload — 本文件全部发布点 (CONFIG_CHANGED 路由) 的 payload
/// 恒为 String (config key / ACTION_RESET_*), 故收敛为 String。
/// PORT: bus::EventBus 无 eventType 路由 (Java subscribe 按类型分发后才调
/// handler) — 本总线广播给**全部**订阅者, 订阅方必须自行按 `event_type` 过滤,
/// 否则跨类型串台。本类型是 UiStateEvent 的唯一定义点, 其他模块不得复制本地
/// 副本 (统一消息类型上提见下 TODO)。
/// TODO(port): UIStateBus 波次落地后切换到 crate 统一消息类型 (LIFETIMES:
/// `enum UiEvent` + App.ui_events 方案)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiStateEvent {
    pub event_type: String,
    pub source: String,
    pub data: String,
}

// =====================================================================
// java.awt.RenderingHints 抗锯齿常量 → 自定义枚举 (CLASSIFY 裁决)
// =====================================================================

/// 文本抗锯齿值域 (VALUE_TEXT_ANTIALIAS_ON / _OFF)。
/// Gasp = VALUE_TEXT_ANTIALIAS_GASP — Application 的声明默认值,
/// loadAppCheck 只写 On/Off, Gasp 仅为保真初始态 (§2.10)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAaSetting {
    On,
    Off,
    Gasp,
}

/// 图形抗锯齿值域 (VALUE_ANTIALIAS_ON / _OFF)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphAaSetting {
    On,
    Off,
}

// =====================================================================
// 主服务
// =====================================================================

/// Java: `public class ConfigurationService implements ConfigProvider`
///
/// PORT: `private List<GroupConfig> layoutConfigs` (可 null) →
/// `RwLock<Option<Vec<GroupConfig>>>`; 字段共享给 OverlaySettings 视图
/// (Java 内部类隐式外部引用) → `Arc<ServiceInner>`。
/// PORT: setConfig/saveWindowPosition 等写方法在 Java 以非同步实例方法触达
/// 共享字段, Rust trait 契约 (&self) 由内部可变性承担 (config_api 裁决);
/// 锁纪律 §2.8: 锁内只改值, 广播在放锁后 (Java publish 内联执行 handler,
/// handler 可重入读配置 — Mutex/RwLock 不可重入)。
#[derive(Clone)]
pub struct ConfigurationService {
    inner: Arc<ServiceInner>,
}

struct ServiceInner {
    layout_configs: RwLock<Option<Vec<GroupConfig>>>,
    /// Application 静态态消费面 (依赖桩, 见文件尾 ApplicationState 文档)
    app: RwLock<ApplicationState>,
    ui_state_bus: Option<Arc<EventBus<UiStateEvent>>>,
}

impl ConfigurationService {
    /// Java: `public ConfigurationService()` — 构造器为空。
    /// PORT: 参数为 UIStateBus 注入 (Java 读全局单例; §2.9 禁再造全局)。
    // PORT: Java 保真 — Arc<ServiceInner> 复刻 Java this 引用的跨方法共享,
    // 内部 RwLock 字段按 Java 字段语义组织, 不为 Sync 约束改形状
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new(ui_state_bus: Option<Arc<EventBus<UiStateEvent>>>) -> Self {
        // Initialize config class (property loader)
        // Note: Actual loading happens in initConfig()
        ConfigurationService {
            inner: Arc::new(ServiceInner {
                layout_configs: RwLock::new(None),
                app: RwLock::new(ApplicationState::new()),
                ui_state_bus,
            }),
        }
    }

    /// Java: `public void initConfig()`
    pub fn init_config(&self) {
        // Load layout config using ConfigManager (handles first-run, upgrade, errors)
        let configs = config_manager::initialize();
        logger::info(
            "ConfigurationService",
            &format!("Loaded layout config with {} groups.", configs.len()),
        );
        *self.inner.layout_configs.write().expect(LC_LOCK_MSG) = Some(configs);

        // Subscribe to global reset requests (EDA implementation)
        // Java: UIStateBus.getInstance().subscribe(CONFIG_CHANGED, key -> {
        //           if (ACTION_RESET_REQUEST.equals(key)) resetAllLayoutDefaults(); })
        // PORT(跨文件, §6 只标注): bus.rs 的 subscribe 要求回调 Send + 'static,
        // 而 config_loader::GroupConfig 含 Rc<SExp> (visible_when/na_when) →
        // 配置树 !Send, 本服务无法被订阅闭包捕获。接线待两选一裁决:
        // (a) config_loader 的 Rc→Arc 化; (b) UIStateBus/AppState 波次收口
        // (LIFETIMES §2.9)。接线前 RESET_REQUEST 事件链路由上层直接调用
        // reset_all_layout_defaults() 顶替 (publish 侧不受影响, 已完整落地)。
        // TODO(port): CONFIG_CHANGED 订阅接线 (见迁移报告)。
    }

    /// Java: `public void loadLayout(String path)`
    pub fn load_layout(&self, path: &str) {
        let configs = config_loader::load_config(path);
        logger::info(
            "ConfigurationService",
            &format!("Loaded layout config with {} groups.", configs.len()),
        );
        *self.inner.layout_configs.write().expect(LC_LOCK_MSG) = Some(configs);
    }

    /// Java: `public void saveLayoutConfig()` (实现体在 ServiceInner, 视图共享)
    pub fn save_layout_config(&self) {
        self.inner.save_layout_config();
    }

    /// 组装层位置桥 (归一化直读): Java overlay init 时经 OverlaySettings.loadPosition
    /// 取 gc.x/y (ConfigurationService.java:430-457 读的同一组字段)。Rust host 在
    /// win32 线程不碰 !Send 配置树 — 组装层启动时经此取快照 (vm-app
    /// ChannelPositionStore); 返回归一化 (0..1) 坐标, 无同题分组 = None
    /// (host 居中兜底, 对齐 Java gc=null 的 center 分支)。
    pub fn group_position(&self, section: &str) -> Option<(f64, f64)> {
        let configs = self.inner.layout_configs.read().expect(LC_LOCK_MSG);
        let list = configs.as_ref()?;
        list.iter()
            .find(|gc| section.eq_ignore_ascii_case(&gc.title))
            .map(|gc| (gc.x, gc.y))
    }

    /// 组装层位置桥 (归一化直写): 与视图 save_window_position 同源
    /// (ConfigurationService.java:460-472 — 命中首个同题分组写回 + saveLayoutConfig,
    /// 未命中只 warn), 差异仅在参数已是归一化坐标 (host 拖拽存档即归一化,
    /// 免像素/比例往返); 视图版带屏幕尺寸守卫, 本版无该守卫 (归一化无涉屏幕)。
    pub fn save_group_position(&self, section: &str, nx: f64, ny: f64) -> bool {
        let mut hit = false;
        {
            let mut configs = self.inner.layout_configs.write().expect(LC_LOCK_MSG);
            if let Some(list) = configs.as_mut() {
                // Java: 命中首个同题分组即用 (equalsIgnoreCase)
                for gc in list.iter_mut() {
                    if section.eq_ignore_ascii_case(&gc.title) {
                        gc.x = nx;
                        gc.y = ny;
                        hit = true;
                        break;
                    }
                }
            }
        }
        if hit {
            logger::debug(
                "OverlaySettings",
                &format!("[{section}] saveGroupPosition: rel {nx:.4},{ny:.4}"),
            );
            self.save_layout_config();
        } else {
            logger::warn(
                "OverlaySettings",
                &format!("[{section}] CANNOT save position: gc=null"),
            );
        }
        hit
    }

    /// Imports configuration from an external file.
    ///
    /// @param sourcePath Path to the config file to import
    /// @return true if import was successful
    ///
    /// Java: `public boolean importConfig(String sourcePath)`
    pub fn import_config(&self, source_path: &str) -> bool {
        let success = config_manager::import_config(source_path);
        if success {
            // Reload configuration
            let reloaded =
                config_loader::load_config(config_manager::get_user_config_path());
            *self.inner.layout_configs.write().expect(LC_LOCK_MSG) = Some(reloaded);
            // Notify all subscribers about the change
            self.inner
                .publish_config_changed(ui_state_events::ACTION_RESET_COMPLETED);
        }
        success
    }

    /// Resets configuration to factory defaults.
    ///
    /// @return true if reset was successful
    ///
    /// Java: `public boolean resetToFactory()`
    pub fn reset_to_factory(&self) -> bool {
        let success = config_manager::reset_to_factory();
        if success {
            // Reload configuration
            let reloaded =
                config_loader::load_config(config_manager::get_user_config_path());
            *self.inner.layout_configs.write().expect(LC_LOCK_MSG) = Some(reloaded);
            // Notify all subscribers about the change
            self.inner
                .publish_config_changed(ui_state_events::ACTION_RESET_COMPLETED);
        }
        success
    }

    /// Java: `public List<GroupConfig> getLayoutConfigs()`
    /// PORT: Java 返回活 List 引用 (可 null) — RwLock 内存储无法经 &self 长期
    /// 借出, 返回快照副本 (Option 对应 null 形态); 当前无跨期持有消费方。
    pub fn get_layout_configs(&self) -> Option<Vec<GroupConfig>> {
        self.inner
            .layout_configs
            .read()
            .expect(LC_LOCK_MSG)
            .clone()
    }

    /// 根据 groupTitle 查找最新的 GroupConfig
    /// 用于 UI 组件在 rebuild 时获取最新配置，解决导入配置后引用陈旧的问题
    ///
    /// @param groupTitle 要查找的 GroupConfig 的标题
    /// @return 找到的 GroupConfig，如果未找到则返回 null
    ///
    /// Java: `public GroupConfig findGroupByTitle(String groupTitle)`
    /// PORT: Java groupTitle 可 null (折叠为不可达, &str 无 null); 大小写敏感
    /// equals 与下方视图的 equalsIgnoreCase 是两条不同查找 (Java 原状)。
    pub fn find_group_by_title(&self, group_title: &str) -> Option<GroupConfig> {
        let configs = self.inner.layout_configs.read().expect(LC_LOCK_MSG);
        if let Some(list) = configs.as_ref() {
            for gc in list {
                if group_title == gc.title {
                    return Some(gc.clone());
                }
            }
        }
        None
    }

    /// Java: `public void loadAppCheck(Controller c)` — 解析并应用配置到应用与
    /// Controller 状态 (替代 Controller.loadFromConfig())。
    /// PORT: Controller 未翻译 — 6 个轮询间隔字段以消费面桩 `ControllerIntervals`
    /// 经 &mut 传入 (Java 直写 c 字段); Application 静态写入落在内部 ApplicationState。
    pub fn load_app_check(&self, c: &mut ControllerIntervals) {
        let mut service_loop_interval_ms: i64 = 50;
        {
            // Try new config key first, fallback to legacy key for backward compatibility
            let mut interval_str = self.inner.get_config_j("dataPollIntervalMs");
            if interval_str.is_empty() {
                interval_str = self.inner.get_config_j("Interval"); // Legacy key fallback
            }
            if !interval_str.is_empty() {
                // PORT §2.15: Long.parseLong 失败 → catch(NumberFormatException) → 50
                service_loop_interval_ms = interval_str.parse::<i64>().unwrap_or(50);
            }
        }
        c.service_loop_interval_ms = service_loop_interval_ms;
        // PORT §2.14: Java `(long)(serviceLoopIntervalMs * 2f)` — long×float 提升
        // float(f32) 后窄化 long (JLS 5.1.2 ↔ Rust as: NaN→0/饱和/向零, 同义)
        c.engine_info_interval_ms = (service_loop_interval_ms as f32 * 2f32) as i64;
        c.flight_info_interval_ms = (service_loop_interval_ms as f32 * 1.5f32) as i64;
        c.altitude_interval_ms = (service_loop_interval_ms as f32 * 1.5f32) as i64;
        c.gear_flaps_interval_ms = (service_loop_interval_ms as f32 * 2f32) as i64;
        c.control_input_interval_ms = (service_loop_interval_ms as f32 * 1f32) as i64;
        // 先取颜色 (短锁完成, 不与 Application 写锁嵌套 — 全库嵌套锁序单向 app→layout)
        let color_num = self.get_color_config("fontNum");
        let color_label = self.get_color_config("fontLabel");
        let color_unit = self.get_color_config("fontUnit");
        let color_warning = self.get_color_config("fontWarn");
        let color_shade_shape = self.get_color_config("fontShade");
        {
            let mut app = self.inner.app.write().expect(APP_LOCK_MSG);
            // Java: (long)(serviceLoopIntervalMs / 3) — long/long 整除 (无浮点)
            app.thread_sleep_time = service_loop_interval_ms / 3;

            app.color_num = color_num;
            app.color_label = color_label;
            app.color_unit = color_unit;
            app.color_warning = color_warning;
            app.color_shade_shape = color_shade_shape;

            // Java: try { vol...; if (非空) voiceVolumn = parseInt } catch { voiceVolumn = 0 }
            let vol = self.inner.get_config_j("voiceVolume");
            if !vol.is_empty() {
                // catch(NumberFormatException) → 0 // Default
                app.voice_volumn = vol.parse::<i32>().unwrap_or(0);
            }

            // Application.drawFontShape = !Boolean.parseBoolean(getConfig("simpleFont"));
            // (被注释的原代码, 原样保留)
            let aa = self.inner.get_config_j("AAEnable");
            app.aa_enable = if !aa.is_empty() {
                java_parse_boolean(&aa)
            } else {
                false // Default
            };

            if app.aa_enable {
                app.text_aa_setting = TextAaSetting::On; // VALUE_TEXT_ANTIALIAS_ON
                app.graph_aa_setting = GraphAaSetting::On; // VALUE_ANTIALIAS_ON
            } else {
                app.text_aa_setting = TextAaSetting::Off; // VALUE_TEXT_ANTIALIAS_OFF
                app.graph_aa_setting = GraphAaSetting::Off; // VALUE_ANTIALIAS_OFF
            }

            let fm_key = self.inner.get_config_j("displayFmKey");
            if !fm_key.is_empty() {
                // Java: try { displayFmKey = parseInt } catch (NumberFormatException e) { // Ignore }
                if let Ok(k) = fm_key.parse::<i32>() {
                    app.display_fm_key = k;
                }
            }

            // --- HTTP Port Sync ---
            let port_str = self.inner.get_config_j("httpPort");
            if !port_str.is_empty() {
                // Java: try { ... } catch (NumberFormatException e) { // Ignore }
                if let Ok(port) = port_str.parse::<i32>() {
                    app.app_port = port;
                    // PORT §2.2: Java int + int 静默回绕 ↔ wrapping_add
                    app.app_port_bkp = port.wrapping_add(1111);
                    // Assuming httpIp is still from Lang or static 127.0.0.1
                    let mut ip = "127.0.0.1".to_string();
                    // Java: Lang.httpIp 为启动期 initLang() 覆写的静态字段;
                    // Rust 无全局 Lang 状态 — init_lang() 静态表快照现取 (blkx 先例)
                    let lang_ip = Lang::init_lang().http_ip;
                    if !lang_ip.is_empty() {
                        ip = lang_ip.to_string();
                    }
                    app.request_dest = Some(InetSocketAddress::new(&ip, app.app_port));
                    app.request_dest_bkp =
                        Some(InetSocketAddress::new(&ip, app.app_port_bkp));
                    logger::info(
                        "ConfigurationService",
                        &format!("HTTP Port synchronized: {port}"),
                    );
                }
            }

            // --- Sync Global Fonts ---
            let global_num_font = self.inner.get_config_j("GlobalNumFont");
            if !global_num_font.is_empty() {
                app.default_numfont_name = global_num_font.clone();
                logger::info(
                    "ConfigurationService",
                    &format!("Global Num font synchronized: {global_num_font}"),
                );
            }

            let global_text_font = self.inner.get_config_j("GlobalTextFont");
            if !global_text_font.is_empty() {
                app.default_font_name = global_text_font.clone();
                logger::info(
                    "ConfigurationService",
                    &format!("Global Text font synchronized: {global_text_font}"),
                );
            }
        }

        // Java: Application.initFont(); Application.updateWebLafFonts();
        // PORT: C 类接线 (AWT GraphicsEnvironment 注册 fonts/ 目录字体并解析
        // defaultFont / WebLaF 全局字体注入), vm-app/vm-ui 波次处理;
        // 落地前 ApplicationState.default_font 维持 None。
        // TODO(port): init_font + update_weblaf_fonts 接线 (get_font_name 的
        // defaultFont 回退分支届时生效)。
    }

    // Java: `private synchronized void scheduleBackgroundSave() { ... }` (Removed)
    // (被移除的原代码注释, 原样保留)

    // --- Helpers ---

    /// Java: `public void saveConfig()` — 已空实现 (不再使用 config.properties)
    pub fn save_config(&self) {
        // No longer using config.properties, all settings in ui_layout.cfg
    }

    /// Java: `public Color getColorConfig(String key)`
    /// PORT: java.awt.Color → [u8;4] RGBA (POC 先例; #RRGGBBAA 字节序经
    /// ColorHelper.parseColor 保真, 见本地 parse_color)
    pub fn get_color_config(&self, key: &str) -> [u8; 4] {
        let val = self.inner.get_config_j(key);
        parse_color(&val, COLOR_WHITE)
    }

    /// Java: `public void setColorConfig(String key, Color c)`
    pub fn set_color_config(&self, key: &str, c: [u8; 4]) {
        let r = i32::from(c[0]);
        let g = i32::from(c[1]);
        let b = i32::from(c[2]);
        let a = i32::from(c[3]);
        // Java: R + ", " + G + ", " + B + ", " + A
        let unified = format!("{r}, {g}, {b}, {a}");

        self.set_config(key, &unified);
    }

    // --- OverlaySettings Implementation ---

    /// Java: `public OverlaySettings getOverlaySettings(String sectionName)`
    pub fn get_overlay_settings(&self, section_name: &str) -> GenericOverlaySettingsImpl {
        GenericOverlaySettingsImpl::new(Arc::clone(&self.inner), section_name)
    }

    // --- HUDSettings Implementation ---

    /// Java: `public HUDSettings getHUDSettings()`
    pub fn get_hud_settings(&self) -> HUDSettingsImpl {
        HUDSettingsImpl::new(Arc::clone(&self.inner))
    }

    // ---- ApplicationState 注入/快照 (Java 全局静态的替代面, 测试与 vm-app 用) ----

    /// Application.screenWidth/screenHeight 注入点
    /// (Java: Toolkit.getScreenSize() → DPIHelper 全局覆写)
    pub fn set_screen_size(&self, width: i32, height: i32) {
        let mut app = self.inner.app.write().expect(APP_LOCK_MSG);
        app.screen_width = width;
        app.screen_height = height;
    }

    /// Application 静态态快照 (断言/上层同步用)
    pub fn application_state(&self) -> ApplicationState {
        self.inner.app.read().expect(APP_LOCK_MSG).clone()
    }

    /// 重置全部布局默认 (供上层顶替未接线的 RESET_REQUEST 事件链, 见 init_config)
    pub fn reset_all_layout_defaults(&self) -> bool {
        self.inner.reset_all_layout_defaults()
    }
}

// --- ConfigProvider Implementation ---

impl ConfigProvider for ConfigurationService {
    /// Java: `public String getConfig(String key)` — 本实现未找到恒返回 ""
    /// (非 null), Option 契约的 None 形态不可达 (接口契约允许, 实现保真)。
    fn get_config(&self, key: &str) -> Option<String> {
        Some(self.inner.get_config_j(key))
    }

    fn set_config(&self, key: &str, value: &str) {
        self.inner.set_config(key, value);
    }

    fn is_field_disabled(&self, key: &str) -> bool {
        self.inner.is_field_disabled(key)
    }
}

// =====================================================================
// ServiceInner — 共享内核 (ConfigurationService 与 OverlaySettings 视图共用,
// 对应 Java 内部类持有的外部类实例)
// =====================================================================

impl ServiceInner {
    /// Java: `public String getConfig(String key)` 主体 (内部类经外部类调用)
    fn get_config_j(&self, key: &str) -> String {
        // Priority 1: Check in-memory LayoutConfigs (Source of Truth)
        let configs = self.layout_configs.read().expect(LC_LOCK_MSG);
        if let Some(list) = configs.as_ref() {
            for gc in list {
                if let Some(row) = find_row_recursive(&gc.rows, key) {
                    // Handle inversion for SWITCH_INV
                    if row.r#type == "SWITCH_INV" {
                        return (!row.get_bool()).to_string(); // String.valueOf(boolean)
                    }
                    return row.get_str();
                }

                // Check Group SwitchKey (e.g. flightInfoSwitch maps to Group visibility)
                // Java: key.equals(gc.switchKey) — switchKey 可 null (equals(null)=false)
                if let Some(sk) = &gc.switch_key {
                    if key == sk {
                        return gc.visible.to_string(); // String.valueOf(boolean)
                    }
                }
            }
        }
        String::new() // Java: return ""
    }

    /// Java: `public void setConfig(String key, String value)` 主体
    fn set_config(&self, key: &str, value: &str) {
        // 1. Update LayoutConfigs (if exists)
        let mut events: Vec<String> = Vec::new();
        {
            let mut configs = self.layout_configs.write().expect(LC_LOCK_MSG);
            if let Some(list) = configs.as_mut() {
                for gc in list.iter_mut() {
                    // Check Rows - recursive update ALL matching instances
                    update_rows_recursive(&mut gc.rows, key, value, &mut events);

                    // Check Group SwitchKey
                    if let Some(sk) = &gc.switch_key {
                        if key == sk {
                            gc.visible = java_parse_boolean(value);
                            events.push(key.to_string());
                        }
                    }
                }
            }
        }
        // PORT §2.8: Java 在改值点内联 publish (UIStateBus 同步执行 handler,
        // handler 可重入读配置); RwLock 不可重入 → 锁内只改值, 放锁后按原
        // 产生顺序补发全部事件 (事件数量与顺序与 Java 一致)
        for k in events {
            self.publish_config_changed(&k);
        }
    }

    /// Java: `public boolean isFieldDisabled(String key)`
    fn is_field_disabled(&self, key: &str) -> bool {
        // Java: key == null || key.isEmpty() — &str 无 null, 折叠空串判定
        if key.is_empty() {
            return false;
        }

        let configs = self.layout_configs.read().expect(LC_LOCK_MSG);
        if let Some(list) = configs.as_ref() {
            for gc in list {
                if let Some(row) = find_row_recursive(&gc.rows, key) {
                    if row.r#type == "SWITCH_INV" {
                        // For SWITCH_INV, the key is usually "disableXXX".
                        // Logic must match getConfig() which returns !row.getBool().
                        // If getConfig() returns "true", it is DISABLED.
                        // Thus isFieldDisabled should return !row.getBool().
                        return !row.get_bool();
                    }
                    if let Some(ConfigValue::Bool(b)) = &row.value {
                        // For 'data' or 'switch', if value is false, it means it is HIDDEN/DISABLED.
                        return !b;
                    }
                    // Java: 非 Boolean 值不返回, 继续扫描后续分组 (原控制流保真)
                }
            }
        }
        false
    }

    /// Java: `public boolean resetAllLayoutDefaults()`
    fn reset_all_layout_defaults(&self) -> bool {
        let mut changed = false;
        // Java: List<RowConfig> pendingChanges — 持引用两阶段;
        // PORT: RwLock 下引用不可跨锁存活, 以 (组下标, 行路径) 定位 (单线程内等价)
        let mut pending: Vec<(usize, Vec<usize>)> = Vec::new();

        // Phase 1: Collect changes (Prepare)
        {
            let configs = self.layout_configs.read().expect(LC_LOCK_MSG);
            if let Some(list) = configs.as_ref() {
                for (gi, g) in list.iter().enumerate() {
                    let mut path = Vec::new();
                    collect_reset_candidates_recursive(&g.rows, gi, &mut path, &mut pending);
                }
            }
        }

        // Phase 2: Apply changes (Commit)
        if !pending.is_empty() {
            let mut configs = self.layout_configs.write().expect(LC_LOCK_MSG);
            if let Some(list) = configs.as_mut() {
                for (gi, path) in &pending {
                    if let Some(row) = row_by_path(list, *gi, path) {
                        logger::info(
                            "ConfigReset",
                            &format!(
                                "Resetting {} ({}) to default: {}",
                                row.label,
                                row.property.as_deref().unwrap_or("no-key"),
                                config_value_to_java_string(row.default_value.as_ref().expect(
                                    "Phase 1 已保证 defaultValue 非 null"
                                ))
                            ),
                        );
                        row.value = row.default_value.clone();
                    }
                }
            }
            changed = true;
        }

        // Phase 3: Persist and Notify
        if changed {
            // Java: saveLayoutConfig() — 锁外在下方 read 完成后调用
            self.save_layout_config();
            // Broadcast global reset event so all components refresh
            self.publish_config_changed(ui_state_events::ACTION_RESET_COMPLETED);
        }
        changed
    }

    /// Java: `public void saveLayoutConfig()` (视图亦经外部类调用)
    fn save_layout_config(&self) {
        // PORT: 读锁内做 IO — **不变量**: config_loader::save_config 不得发事件
        // 或回读本服务配置, 否则读锁自死锁 (§2.8; Java 无锁, 此串行化是 Rust
        // 新增保守面); 当前实现满足 (无回调重入面)。
        let configs = self.layout_configs.read().expect(LC_LOCK_MSG);
        if let Some(list) = configs.as_ref() {
            logger::info(
                "ConfigurationService",
                &format!(
                    "ACTION: ConfigurationService: Saving to {}",
                    config_manager::get_user_config_path()
                ),
            );
            config_loader::save_config(config_manager::get_user_config_path(), list);
        }
    }

    /// Java: `UIStateBus.getInstance().publish(CONFIG_CHANGED, "ConfigurationService", data)`
    /// PORT: 总线注入式 — 未注入时为无操作 (Java 全局单例恒存在)
    fn publish_config_changed(&self, data: &str) {
        if let Some(bus) = &self.ui_state_bus {
            bus.publish(&UiStateEvent {
                event_type: ui_state_events::CONFIG_CHANGED.to_string(),
                source: "ConfigurationService".to_string(),
                data: data.to_string(),
            });
        }
    }

    /// GenericOverlaySettingsImpl.getGroupConfig 的查找体:
    /// Java `sectionName.equalsIgnoreCase(gc.title)`。
    /// PORT: Java 逐字符简单大小写折叠 — 标题域 ASCII/CJK 下
    /// eq_ignore_ascii_case 等价 (CJK 无大小写)
    fn find_group_ignore_case(&self, section_name: &str) -> Option<GroupConfig> {
        let configs = self.layout_configs.read().expect(LC_LOCK_MSG);
        if let Some(list) = configs.as_ref() {
            for gc in list {
                if section_name.eq_ignore_ascii_case(&gc.title) {
                    return Some(gc.clone());
                }
            }
        }
        None
    }

    /// 就地写回分组 x/y (saveWindowPosition 的写面), 返回新值供日志
    fn set_group_position_ignore_case(
        &self,
        section_name: &str,
        x: f64,
        y: f64,
    ) -> Option<(f64, f64)> {
        let mut configs = self.layout_configs.write().expect(LC_LOCK_MSG);
        if let Some(list) = configs.as_mut() {
            for gc in list.iter_mut() {
                if section_name.eq_ignore_ascii_case(&gc.title) {
                    gc.x = x;
                    gc.y = y;
                    return Some((gc.x, gc.y));
                }
            }
        }
        None
    }

    fn screen_size(&self) -> (i32, i32) {
        let app = self.app.read().expect(APP_LOCK_MSG);
        (app.screen_width, app.screen_height)
    }

    fn app_default_numfont_name(&self) -> String {
        self.app.read().expect(APP_LOCK_MSG).default_numfont_name.clone()
    }

    /// Java: `Application.defaultFont.getFontName()` — initFont() 前为 null → NPE
    fn app_default_font_name(&self) -> String {
        self.app
            .read()
            .expect(APP_LOCK_MSG)
            .default_font
            .as_ref()
            .expect("java.lang.NullPointerException: Application.defaultFont")
            .name
            .clone()
    }
}

/// Java: `private RowConfig findRowRecursive(List<RowConfig> rows, String key)`
/// PORT: 借用版自由函数 — 输入切片生命周期透传 (调用方持快照或锁守卫)
fn find_row_recursive<'a>(rows: &'a [RowConfig], key: &str) -> Option<&'a RowConfig> {
    for row in rows {
        // Priority 1: Match property target exactly
        // Java: key.equals(row.property) — property 可 null (equals(null)=false)
        if row.property.as_deref() == Some(key) {
            return Some(row);
        }
        // Priority 2: Match label if property is missing
        if row.property.is_none() && key == row.label {
            return Some(row);
        }
        // Recurse
        // Java: row.children != null && !row.children.isEmpty() — Vec 恒非 null
        if !row.children.is_empty() {
            if let Some(found) = find_row_recursive(&row.children, key) {
                return Some(found);
            }
        }
    }
    None
}

/// Java: `private void updateRowsRecursive(List<RowConfig> rows, String key, String value)`
/// PORT: 事件 publish 由调用方放锁后补发 (events 收集, §2.8)
fn update_rows_recursive(
    rows: &mut [RowConfig],
    key: &str,
    value: &str,
    events: &mut Vec<String>,
) {
    for row in rows.iter_mut() {
        // Java: key.equals(row.property) || (row.property == null && key.equals(row.label))
        if row.property.as_deref() == Some(key)
            || (row.property.is_none() && key == row.label)
        {
            // Update typed value based on existing type to maintain consistency
            // Handle inversion for SWITCH_INV
            let val_to_store = if row.r#type == "SWITCH_INV" {
                (!java_parse_boolean(value)).to_string() // String.valueOf(!parseBoolean)
            } else {
                value.to_string()
            };

            // Java: instanceof Boolean → Integer → 其他/默认 String
            row.value = match &row.value {
                Some(ConfigValue::Bool(_)) => {
                    Some(ConfigValue::Bool(java_parse_boolean(&val_to_store)))
                }
                Some(ConfigValue::Int(_)) => match val_to_store.parse::<i32>() {
                    Ok(i) => Some(ConfigValue::Int(i)),
                    // Java: catch (Exception e) { row.value = valToStore; } (存字符串)
                    Err(_) => Some(ConfigValue::Str(val_to_store)),
                },
                // Double / Str / null → String (Java else 分支)
                _ => Some(ConfigValue::Str(val_to_store)),
            };
            events.push(key.to_string());
        }
        // Java: row.children != null && !row.children.isEmpty()
        if !row.children.is_empty() {
            update_rows_recursive(&mut row.children, key, value, events);
        }
    }
}

/// Java `defaultValue.equals(value)` 的分派复刻: 按 defaultValue 运行时类型
/// 走对应 equals — **Double.equals 是 doubleToLongBits 位级比较** (NaN==NaN
/// 为 true、+0.0!=-0.0), 与 Rust 派生 PartialEq 的数值比较 (NaN!=NaN、
/// 0.0==-0.0) 在这两类位形上相反; 异型 (如 Double vs Int) instanceof 恒 false。
fn config_value_java_equals(default_v: &ConfigValue, value: &ConfigValue) -> bool {
    match (default_v, value) {
        (ConfigValue::Bool(a), ConfigValue::Bool(b)) => a == b,
        (ConfigValue::Int(a), ConfigValue::Int(b)) => a == b,
        (ConfigValue::Double(a), ConfigValue::Double(b)) => {
            // doubleToLongBits 将全部 NaN 折叠为规范位形 → NaN 互等
            (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
        }
        (ConfigValue::Str(a), ConfigValue::Str(b)) => a == b,
        _ => false,
    }
}

/// Java: `private void collectResetCandidatesRecursive(List<RowConfig> rows, List<RowConfig> pendingChanges)`
/// PORT: Java 收集对象引用; Rust 收集 (组下标, 行路径) — 单线程内两者等价
/// (引用在 Java 端也仅用于 Phase 2 的原位回写)
fn collect_reset_candidates_recursive(
    rows: &[RowConfig],
    group_index: usize,
    path: &mut Vec<usize>,
    pending: &mut Vec<(usize, Vec<usize>)>,
) {
    for (i, r) in rows.iter().enumerate() {
        // Check self
        // Java: r.defaultValue != null && !r.defaultValue.equals(r.value)
        // (value 为 null 时 equals 返回 false → 仍收集)
        if let Some(dv) = &r.default_value {
            let equal = r.value.as_ref().is_some_and(|v| config_value_java_equals(dv, v));
            if !equal {
                let mut p = path.clone();
                p.push(i);
                pending.push((group_index, p));
            }
        }
        // Recurse
        if !r.children.is_empty() {
            path.push(i);
            collect_reset_candidates_recursive(&r.children, group_index, path, pending);
            path.pop();
        }
    }
}

/// 按 (组下标, 行路径) 定位行 (Phase 2 写面) — path[0] 索引组内 rows, 其后逐层 children
fn row_by_path<'a>(
    list: &'a mut [GroupConfig],
    group_index: usize,
    path: &[usize],
) -> Option<&'a mut RowConfig> {
    let gc = list.get_mut(group_index)?;
    if path.is_empty() {
        return None;
    }
    let row = gc.rows.get_mut(path[0])?;
    row_by_path_children(row, &path[1..])
}

fn row_by_path_children<'a>(row: &'a mut RowConfig, path: &[usize]) -> Option<&'a mut RowConfig> {
    if path.is_empty() {
        return Some(row);
    }
    let &first = path.first()?;
    let child = row.children.get_mut(first)?;
    row_by_path_children(child, &path[1..])
}

// =====================================================================
// GenericOverlaySettingsImpl — Java 非静态内部类 → 独立 struct
// =====================================================================

/// Java: `private class GenericOverlaySettingsImpl implements OverlaySettings`
///
/// PORT: 内部类持外部类实例 (ConfigurationService.this) → 持共享内核 Arc。
pub struct GenericOverlaySettingsImpl {
    /// Java: `protected final String sectionName`
    pub(crate) section_name: String,
    service: Arc<ServiceInner>,
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
    fn new(service: Arc<ServiceInner>, section_name: &str) -> Self {
        let group_snapshot = service.find_group_ignore_case(section_name);
        GenericOverlaySettingsImpl {
            section_name: section_name.to_string(),
            service,
            group_snapshot,
        }
    }

    /// Java getGroupConfig() 的重查体 (供本视图各读取方法逐调用取最新状态)
    fn get_group_config_snapshot(&self) -> Option<GroupConfig> {
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
            // Java: (int) Math.round(gc.x * Application.screenWidth)
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
            &format!("[{}] getWindowX: gc=null => center {}", self.section_name, cx),
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
            &format!("[{}] getWindowY: gc=null => center {}", self.section_name, cy),
        );
        cy
    }

    /// Java: `public void saveWindowPosition(double x, double y)`
    fn save_window_position(&self, x: f64, y: f64) {
        let (screen_w, screen_h) = self.service.screen_size();
        // Java: gc = getGroupConfig(); if (gc != null && sw > 0 && sh > 0) {...} else {warn}
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
                            // Java: gc.x = x / screenWidth (double / int → double)
                            gc.x = x / f64::from(screen_w);
                            gc.y = y / f64::from(screen_h);
                            wrote = Some((gc.x, gc.y));
                        }
                        break; // Java: 命中首个同题分组即用
                    }
                }
            }
        }
        if let Some((rx, ry)) = wrote {
            // Java: %f 默认 6 位小数 (Rust {:.6} 同宽度; 舍入差异同上 {:.4} 注)
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
            // Java: gc.fontName != null && !gc.fontName.isEmpty()
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
        // Java: val == null || val.isEmpty() — 本实现 getConfig 恒非 null
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
        // Java: try { Integer.parseInt } catch (NumberFormatException) { return def } (§2.15)
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

// =====================================================================
// HUDSettingsImpl — Java `private class HUDSettingsImpl extends
// GenericOverlaySettingsImpl implements HUDSettings` → 组合 + 委托
// =====================================================================

pub struct HUDSettingsImpl {
    /// PORT: extends → 组合 (§1); 父类方法经 base 委托 (Java 单实现继承)
    base: GenericOverlaySettingsImpl,
}

impl HUDSettingsImpl {
    /// Java: `public HUDSettingsImpl() { super("MiniHUD"); }`
    /// (私有构造: 视图仅经 getHUDSettings 工厂产出, 同模块内调用)
    fn new(service: Arc<ServiceInner>) -> Self {
        HUDSettingsImpl {
            base: GenericOverlaySettingsImpl::new(service, "MiniHUD"),
        }
    }

    /// Java: `private double getDouble(String key, double def)`
    fn get_double(&self, key: &str, def: f64) -> f64 {
        let val = self.base.service.get_config_j(key);
        // Java: val.isEmpty() ? def : Double.parseDouble(val); catch → def
        if val.is_empty() {
            def
        } else {
            java_parse_double(&val).unwrap_or(def)
        }
    }

    /// Java: `public String getNumFont()`
    fn get_num_font(&self) -> String {
        let mut font = self.base.service.get_config_j("MonoNumFont");
        if font.is_empty() {
            font = self.base.service.get_config_j("GlobalNumFont");
        }
        if font.is_empty() {
            self.base.service.app_default_numfont_name()
        } else {
            font
        }
    }

    /// Java: `private double getDoubleFromLayoutFirst(String section, String property, double defaultVal)`
    fn get_double_from_layout_first(
        &self,
        section: &str,
        property: &str,
        default_val: f64,
    ) -> f64 {
        // Priority 1: Check in-memory LayoutConfigs
        // (锁作用域内完成查找, 放锁后再走 Priority 2 — 不嵌套)
        let found = {
            let configs = self.base.service.layout_configs.read().expect(LC_LOCK_MSG);
            configs
                .as_ref()
                .and_then(|list| layout_first_double(list, section, property))
        };
        // Priority 2: Check global config.properties
        found.unwrap_or_else(|| self.get_double(property, default_val))
    }
}

/// getDoubleFromLayoutFirst 的 Priority 1 查找体 (锁内执行)
fn layout_first_double(list: &[GroupConfig], section: &str, property: &str) -> Option<f64> {
    for gc in list {
        if section.eq_ignore_ascii_case(&gc.title) {
            if let Some(row) = find_row_recursive(&gc.rows, property) {
                if let Some(v) = &row.value {
                    match v {
                        // Java: row.value instanceof Number → doubleValue()
                        ConfigValue::Int(i) => return Some(f64::from(*i)),
                        ConfigValue::Double(d) => return Some(*d),
                        other => {
                            // Java: try { Double.parseDouble(row.value.toString()) }
                            //      catch (NumberFormatException e) { // ignore } → 继续循环
                            if let Ok(d) =
                                java_parse_double(&config_value_to_java_string(other))
                            {
                                return Some(d);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

impl OverlaySettings for HUDSettingsImpl {
    type GroupConfig = GroupConfig;

    /// Java: `@Override public String getNumFontName() { return getNumFont(); }`
    fn get_num_font_name(&self) -> String {
        self.get_num_font()
    }

    /// Java: `@Override public int getWindowX(int canvasWidth)`
    fn get_window_x(&self, canvas_width: i32) -> i32 {
        let gc = self.base.get_group_config_snapshot();
        let (screen_w, _) = self.base.service.screen_size();
        if let Some(gc) = gc {
            // (int) Math.round(gc.x * Application.screenWidth) — 双转窄化复刻
            return ((gc.x * f64::from(screen_w) + 0.5).floor() as i64) as u32 as i32;
        }
        self.base
            .get_int("crosshairX", (screen_w - canvas_width) / 2)
    }

    /// Java: `@Override public int getWindowY(int canvasHeight)`
    fn get_window_y(&self, canvas_height: i32) -> i32 {
        let gc = self.base.get_group_config_snapshot();
        let (_, screen_h) = self.base.service.screen_size();
        if let Some(gc) = gc {
            return ((gc.y * f64::from(screen_h) + 0.5).floor() as i64) as u32 as i32;
        }
        self.base
            .get_int("crosshairY", (screen_h - canvas_height) / 2)
    }

    /// Java: `@Override public void saveWindowPosition(double x, double y)`
    fn save_window_position(&self, x: f64, y: f64) {
        let gc = self.base.get_group_config_snapshot();
        let (screen_w, screen_h) = self.base.service.screen_size();
        if gc.is_some() {
            // Java: gc.x = x / screenWidth (double / int; sw=0 时 Java 得 ±Inf/NaN,
            // Rust f64 除零同义 — 与父类实现不同, 子类无 screen>0 守卫, 保真)
            self.base.service.set_group_position_ignore_case(
                &self.base.section_name,
                x / f64::from(screen_w),
                y / f64::from(screen_h),
            );
            self.base.service.save_layout_config();
        } else {
            // Java: setConfig("crosshairX", Integer.toString((int) x))
            // (int) double: JLS 5.1.3 (NaN→0/饱和/向零) ↔ Rust as i32 同义
            self.base
                .service
                .set_config("crosshairX", &(x as i32).to_string());
            self.base
                .service
                .set_config("crosshairY", &(y as i32).to_string());
        }
    }

    // ---- 以下为 GenericOverlaySettingsImpl 继承成员的委托 ----

    fn get_group_config(&self) -> Option<&GroupConfig> {
        self.base.get_group_config()
    }

    fn get_font_name(&self) -> String {
        self.base.get_font_name()
    }

    fn get_font_size_add(&self) -> i32 {
        self.base.get_font_size_add()
    }

    fn get_bool(&self, key: &str, def: bool) -> bool {
        self.base.get_bool(key, def)
    }

    fn get_int(&self, key: &str, def: i32) -> i32 {
        self.base.get_int(key, def)
    }

    fn get_string(&self, key: &str, def: &str) -> String {
        self.base.get_string(key, def)
    }

    fn auto_hide_on_focus_loss(&self) -> bool {
        self.base.auto_hide_on_focus_loss()
    }
}

impl HUDSettings for HUDSettingsImpl {
    /// Java: `@Override public String getNumFont()`
    fn get_num_font(&self) -> String {
        HUDSettingsImpl::get_num_font(self)
    }

    /// Java: `@Override public int getCrosshairScale()`
    fn get_crosshair_scale(&self) -> i32 {
        let scale = self.base.get_int("crosshairScale", 70);
        if scale == 0 {
            1
        } else {
            scale
        }
    }

    /// Java: `@Override public String getCrosshairName()`
    fn get_crosshair_name(&self) -> String {
        // Java: getConfig("crosshairName").trim() — java.lang.String.trim (<= U+0020)
        java_trim(&self.base.service.get_config_j("crosshairName")).to_string()
    }

    /// Java: `@Override public boolean isDisplayCrosshair()`
    fn is_display_crosshair(&self) -> bool {
        self.base.get_bool("displayCrosshair", false)
    }

    /// Java: `@Override public boolean useTextureCrosshair()`
    fn use_texture_crosshair(&self) -> bool {
        let name = self.get_crosshair_name();
        // Java: name != null && !name.isEmpty() && !"软件渲染准星".equalsIgnoreCase(name)
        // (getCrosshairName 恒非 null; CJK 无大小写折叠, equalsIgnoreCase ≡ equals)
        !name.is_empty() && name != "软件渲染准星"
    }

    /// Java: `@Override public boolean drawHUDText()`
    fn draw_hud_text(&self) -> bool {
        self.base.get_bool("drawHUDtext", true)
    }

    /// Java: `@Override public boolean showAttitudeGauge()`
    fn show_attitude_gauge(&self) -> bool {
        self.base.get_bool("showAttitudeGauge", true)
    }

    /// Java: `@Override public double getAoAWarningRatio()`
    fn get_aoa_warning_ratio(&self) -> f64 {
        let val = self.get_double_from_layout_first(
            &self.base.section_name.clone(),
            "miniHUDaoaWarningRatio",
            25.0,
        );
        if val > 1.0 {
            val / 100.0
        } else {
            val
        }
    }

    /// Java: `@Override public double getAoABarWarningRatio()`
    fn get_aoa_bar_warning_ratio(&self) -> f64 {
        let val = self.get_double_from_layout_first(
            &self.base.section_name.clone(),
            "miniHUDaoaBarWarningRatio",
            0.0,
        );
        if val > 1.0 {
            val / 100.0
        } else {
            val
        }
    }

    /// Java: `@Override public boolean enableFlapAngleBar()`
    fn enable_flap_angle_bar(&self) -> bool {
        self.base.get_bool("enableFlapAngleBar", true)
    }

    /// Java: `@Override public boolean showSpeedBar()`
    fn show_speed_bar(&self) -> bool {
        self.base.get_bool("showSpeedBar", true)
    }

    /// Java: `@Override public boolean drawHudMach()`
    fn draw_hud_mach(&self) -> bool {
        self.base.get_bool("hudMach", false)
    }

    /// Java: `@Override public boolean isSpeedLabelDisabled()`
    fn is_speed_label_disabled(&self) -> bool {
        self.base.get_bool("disableHUDSpeedLabel", false)
    }

    /// Java: `@Override public boolean isAltitudeLabelDisabled()`
    fn is_altitude_label_disabled(&self) -> bool {
        self.base.get_bool("disableHUDHeightLabel", false)
    }

    /// Java: `@Override public boolean isSEPLabelDisabled()`
    fn is_sep_label_disabled(&self) -> bool {
        self.base.get_bool("disableHUDSEPLabel", false)
    }

    /// Java: `@Override public boolean showHUDSpeed()`
    fn show_hud_speed(&self) -> bool {
        self.base.get_bool("showHUDSpeed", true)
    }

    /// Java: `@Override public boolean showHUDAoA()`
    fn show_hud_aoa(&self) -> bool {
        self.base.get_bool("showHUDAoA", true)
    }

    /// Java: `@Override public boolean showHUDAltitude()`
    fn show_hud_altitude(&self) -> bool {
        self.base.get_bool("showHUDAltitude", true)
    }

    /// Java: `@Override public boolean showHUDEnergy()`
    fn show_hud_energy(&self) -> bool {
        self.base.get_bool("showHUDEnergy", true)
    }

    /// Java: `@Override public boolean showHUDMechanization()`
    fn show_hud_mechanization(&self) -> bool {
        self.base.get_bool("showHUDMechanization", true)
    }

    /// Java: `@Override public boolean showHUDFlaps()`
    fn show_hud_flaps(&self) -> bool {
        self.base.get_bool("showHUDFlaps", true)
    }

    /// Java: `@Override public boolean showHUDAirbrake()`
    fn show_hud_airbrake(&self) -> bool {
        self.base.get_bool("showHUDAirbrake", true)
    }

    /// Java: `@Override public boolean showHUDGear()`
    fn show_hud_gear(&self) -> bool {
        self.base.get_bool("showHUDGear", true)
    }

    /// Java: `@Override public boolean showHUDSep()`
    fn show_hud_sep(&self) -> bool {
        self.base.get_bool("showHUDSep", true)
    }

    /// Java: `@Override public boolean showHUDGLoad()`
    fn show_hud_g_load(&self) -> bool {
        self.base.get_bool("showHUDGLoad", true)
    }

    /// Java: `@Override public boolean showHUDManeuverBar()`
    fn show_hud_maneuver_bar(&self) -> bool {
        self.base.get_bool("showHUDManeuverBar", true)
    }

    /// Java: `@Override public boolean isAttitudeIndicatorInertialMode()`
    fn is_attitude_indicator_inertial_mode(&self) -> bool {
        self.base.get_bool("attitudeIndicatorInertialMode", false)
    }

    /// Java: `@Override public boolean isGPUCompatibilityMode()`
    /// (接口注释: 底层 GPUCompatibilityHelper 经 CLASSIFY 裁决不迁移; 本实现
    /// 按源文件原样读配置存储值)
    fn is_gpu_compatibility_mode(&self) -> bool {
        self.base.get_bool("gpuCompatibilityMode", false)
    }

    /// Java: `@Override public boolean alwaysShowRadarAltitude()`
    fn always_show_radar_altitude(&self) -> bool {
        self.base.get_bool("alwaysShowRadarAltitude", false)
    }
}

// =====================================================================
// Java 语义本地辅助 (私有) — 与 config_loader/config_manager 同款复刻
// =====================================================================

/// Java `String.trim()`: 剥首尾所有 `<= U+0020` 的字符 — 与 Rust `str::trim`
/// (Unicode White_Space, 会剥 U+3000 等) 不同; config_loader/config_manager 同款。
fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c as u32) <= 0x20)
}

/// Java `Boolean.parseBoolean(String)` = equalsIgnoreCase("true") — 非 "true" 一律 false。
fn java_parse_boolean(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
}

/// Java `String.valueOf(Object)` 的 ConfigValue 域 (Boolean/Integer/Double/String
/// 各自 toString)。PORT: config_loader 的同款为私有未导出 — 本地同构副本,
/// 待其导出后收敛。
fn config_value_to_java_string(v: &ConfigValue) -> String {
    match v {
        ConfigValue::Bool(b) => b.to_string(),
        ConfigValue::Int(i) => i.to_string(),
        ConfigValue::Double(d) => java_double_to_string(*d),
        ConfigValue::Str(s) => s.clone(),
    }
}

/// Java `Double.toString(double)` 一比一复刻 — 与 config_loader 私有同名函数
/// 同实现 (其未导出, 本文件无法跨模块复用):
/// - 10^-3 ≤ |d| < 10^7 → 十进制平原式, 恒至少一位小数 ("1.0");
/// - 否则科学计数 "D.DDDE±x" ('E' 后仅负指数带 '-', 正指数无 '+');
/// - 最短可区分数字串; NaN/±0/±Inf 特判。
///
/// PORT: 数字串取 Rust `{:e}` 最短往返表示, 与 Java FloatingDecimal 在
/// JDK-4511638 域 (极罕见多位尾数) 外逐位一致 — cfg 值域 oracle 对拍无差异。
fn java_double_to_string(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d == 0.0 {
        return if d.is_sign_negative() { "-0.0".to_string() } else { "0.0".to_string() };
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let neg = d.is_sign_negative();
    let a = d.abs();
    // "{:e}" → "D.DDDe±n"; a > 0 有限, 恒此形态 (最短往返数字, 无尾随零)
    let sci = format!("{:e}", a);
    let epos = sci.find('e').unwrap();
    let mant = &sci[..epos];
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let mut s = String::new();
    if (-3..=6).contains(&exp10) {
        // 平原式
        if exp10 >= 0 {
            let ip = exp10 as usize + 1; // 整数部分位数
            if digits.len() > ip {
                s.push_str(&digits[..ip]);
                s.push('.');
                s.push_str(&digits[ip..]);
            } else {
                s.push_str(&digits);
                s.push_str(&"0".repeat(ip - digits.len()));
                s.push_str(".0"); // 恒至少一位小数
            }
        } else {
            s.push_str("0.");
            s.push_str(&"0".repeat((-exp10 - 1) as usize));
            s.push_str(&digits);
        }
    } else {
        // 科学计数
        s.push_str(&digits[..1]);
        s.push('.');
        if digits.len() > 1 {
            s.push_str(&digits[1..]);
        } else {
            s.push('0');
        }
        s.push('E');
        s.push_str(&exp10.to_string());
    }
    if neg {
        s.insert(0, '-');
    }
    s
}

/// Java `Double.parseDouble(String)` 复刻 (消费域收敛版):
/// - JLS/FloatingDecimal: 先 `String.trim()` (<= U+0020), 允许尾缀 d/D/f/F;
/// - "NaN"/"Infinity" (±) 与 Rust from_str 的 "nan"/"inf"(任意大小写) 集合有差
///   — cfg 值域不可达;
/// - 十六进制浮点 ("0x1.8p1") Java 可解析而 Rust 不可 → Err ≡ NumberFormatException
///   → 调用方回退默认 (cfg 域不可达, PORT §6 以 Java 8 oracle 为准的域内等价)。
fn java_parse_double(s: &str) -> Result<f64, ()> {
    let t = java_trim(s);
    let core = t.strip_suffix(|c: char| matches!(c, 'd' | 'D' | 'f' | 'F')).unwrap_or(t);
    core.parse::<f64>().map_err(|_| ())
}

// =====================================================================
// prog.util.ColorHelper 的消费面内联 (依赖桩, 非独立翻译)
// =====================================================================

/// java.awt.Color.WHITE — ColorHelper.parseColor 的默认色 (loadAppCheck 调用点)
const COLOR_WHITE: [u8; 4] = [255, 255, 255, 255];

/// Java: `ColorHelper.parseColor(String text, Color defaultColor)`
/// 支持 hex (#RRGGBB / #RRGGBBAA) 与十进制 (R, G, B / R, G, B, A),
/// 失败回默认色 (never throws)。
/// PORT: java.awt.Color → [u8;4] RGBA (POC 先例, §3 字节序: R,G,B,A 序);
/// ColorHelper 未翻译 — 一比一内联 (该文件波次落地后收敛)。
fn parse_color(text: &str, default_color: [u8; 4]) -> [u8; 4] {
    // Java: text == null || text.trim().isEmpty() — 调用域 text 恒非 null
    if java_trim(text).is_empty() {
        return default_color;
    }

    let trimmed = java_trim(text);

    // Try hex format first
    if trimmed.starts_with('#') {
        return parse_hex_color(trimmed, default_color);
    }

    // Try decimal format
    parse_decimal_color(trimmed, default_color)
}

/// Java: `ColorHelper.parseHexColor(String hex, Color defaultColor)`
fn parse_hex_color(hex: &str, default_color: [u8; 4]) -> [u8; 4] {
    // Java: try {...} catch (Exception e) { /* Fall through to default */ }
    // PORT §2.1: substring 按 UTF-16 码元、parseInt 遇非 ASCII 抛异常 → 默认;
    // Rust 字节切片遇多字节字符会 panic — 整段 ASCII 校验把该路径收敛到默认
    let h = &hex[1..]; // Remove #  ('#' 单字节, 切片安全)
    if !h.is_ascii() {
        return default_color;
    }
    let b = h.as_bytes();
    let two = |i: usize| u8::from_str_radix(&h[i..i + 2], 16);
    if b.len() == 6 {
        // #RRGGBB - no alpha (new Color(r,g,b) → alpha 255)
        match (two(0), two(2), two(4)) {
            (Ok(r), Ok(g), Ok(bl)) => [r, g, bl, 255],
            _ => default_color,
        }
    } else if b.len() == 8 {
        // #RRGGBBAA - with alpha
        match (two(0), two(2), two(4), two(6)) {
            (Ok(r), Ok(g), Ok(bl), Ok(a)) => [r, g, bl, a],
            _ => default_color,
        }
    } else {
        default_color
    }
}

/// Java: `ColorHelper.parseDecimalColor(String decimal, Color defaultColor)`
fn parse_decimal_color(decimal: &str, default_color: [u8; 4]) -> [u8; 4] {
    // Java: decimal.replaceAll("\\s+", "") — Java 正则 \s = ASCII 空白六类
    // [ \t\n\x0B\f\r] (默认无 UNICODE_CHARACTER_CLASS)
    let cleaned: String = decimal
        .chars()
        .filter(|c| !matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r'))
        .collect();
    // Java: cleaned.split(",") — limit 0 语义: 剔除全部尾部空串, 保留中间空串
    let mut parts: Vec<&str> = cleaned.split(',').collect();
    while parts.last() == Some(&"") {
        parts.pop();
    }

    if parts.len() >= 3 {
        // Java: parseInt 各段, 任一 NumberFormatException → catch → 默认
        let (r, g, bl) = match (parts[0].parse::<i32>(), parts[1].parse::<i32>(), parts[2].parse::<i32>()) {
            (Ok(r), Ok(g), Ok(b)) => (r, g, b),
            _ => return default_color,
        };
        let a = if parts.len() >= 4 {
            match parts[3].parse::<i32>() {
                Ok(v) => v,
                Err(_) => return default_color,
            }
        } else {
            255
        };

        // Clamp values to valid range
        return [
            clamp(r, 0, 255) as u8,
            clamp(g, 0, 255) as u8,
            clamp(bl, 0, 255) as u8,
            clamp(a, 0, 255) as u8,
        ];
    }
    default_color
}

/// Java: `ColorHelper.clamp(int value, int min, int max)` = Math.max(min, Math.min(max, value))
fn clamp(value: i32, min: i32, max: i32) -> i32 {
    std::cmp::max(min, std::cmp::min(max, value))
}

// =====================================================================
// 依赖桩 (非翻译) — Application / Controller / java.net.InetSocketAddress
// 的本文件消费面。TODO(port): 各自波次落地后切换到真实类型。
// =====================================================================

/// java.net.InetSocketAddress 消费面: (hostname, port) 二元组。
/// Java 允许未解析主机名 (构造器内捕获 UnknownHostException, 持 hostname
/// 不解析), 故不以 SocketAddr (需可解析 IP) 建模; 端口保持 int — JDK 构造器
/// 对 port ∉ [0,65535] 抛 IllegalArgumentException (见 new)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InetSocketAddress {
    pub host: String,
    pub port: i32,
}

impl InetSocketAddress {
    /// Java: `new InetSocketAddress(String hostname, int port)`
    /// PORT §1: JDK 构造器对 port < 0 或 > 65535 抛 IllegalArgumentException —
    /// **不是** NumberFormatException, 不被 loadAppCheck 的 catch 捕获, 直接
    /// 传播出方法 (Java 调用方面对崩溃); Rust 以 panic! 复刻该抛出面。
    /// (hostname null 检查同抛 IAE, 但 &str 域不可达。)
    pub fn new(host: &str, port: i32) -> Self {
        if !(0..=65535).contains(&port) {
            panic!("port out of range: {port} (java.lang.IllegalArgumentException)");
        }
        InetSocketAddress {
            host: host.to_string(),
            port,
        }
    }
}

/// Application.defaultFont 的最小替身 (java.awt.Font 属 C 类)。
/// initFont() 落地前为 None (Java: 静态字段 null → getFontName() NPE)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppFont {
    /// Java: `Font.getFontName()` 的返回域
    pub name: String,
}

/// Application 静态字段的消费面 (依赖桩, 非翻译):
/// 仅收录 ConfigurationService.java 读/写触达的成员; 声明默认值 =
/// Application.java 字段初始化值 (§2.10 按有意保真)。
/// PORT: Java 全局静态 → 服务持有的注入态 (§2.9 禁裸全局; vm-app 波次收口)。
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationState {
    /// `public static long threadSleepTime = 33`
    pub thread_sleep_time: i64,
    /// `public static Color colorNum = new Color(27, 255, 128, 240)`
    pub color_num: [u8; 4],
    /// `public static Color colorLabel = new Color(27, 255, 128, 166)`
    pub color_label: [u8; 4],
    /// `public static Color colorUnit = new Color(166, 166, 166, 220)`
    pub color_unit: [u8; 4],
    /// `public static Color colorWarning = new Color(216, 33, 13, 100)`
    pub color_warning: [u8; 4],
    /// `public static Color colorShadeShape = new Color(0, 0, 0, 42)`
    pub color_shade_shape: [u8; 4],
    /// `public static int voiceVolumn = 100`
    pub voice_volumn: i32,
    /// `public static Boolean aaEnable = true`
    pub aa_enable: bool,
    /// `public static Object textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_GASP`
    pub text_aa_setting: TextAaSetting,
    /// `public static Object graphAASetting = RenderingHints.VALUE_ANTIALIAS_ON`
    pub graph_aa_setting: GraphAaSetting,
    /// `public static int displayFmKey = NativeKeyEvent.VC_P` (VC_P = 25)
    pub display_fm_key: i32,
    /// `public static int appPort` (声明默认 0)
    pub app_port: i32,
    /// `public static int appPortBkp` (声明默认 0)
    pub app_port_bkp: i32,
    /// `public static SocketAddress requestDest` (声明默认 null)
    pub request_dest: Option<InetSocketAddress>,
    /// `public static SocketAddress requestDestBkp` (声明默认 null)
    pub request_dest_bkp: Option<InetSocketAddress>,
    /// `public static String defaultNumfontName = "Roboto"`
    pub default_numfont_name: String,
    /// `public static String defaultFontName = "Microsoft YaHei UI"`
    pub default_font_name: String,
    /// `public static Font defaultFont` (声明默认 null; initFont 赋值)
    pub default_font: Option<AppFont>,
    /// `public static int screenWidth` (声明默认 0; getScreenSize() 覆写)
    pub screen_width: i32,
    /// `public static int screenHeight` (声明默认 0)
    pub screen_height: i32,
}

impl ApplicationState {
    /// Java 字段声明初始化值的等价构造
    pub fn new() -> Self {
        ApplicationState {
            thread_sleep_time: 33,
            color_num: [27, 255, 128, 240],
            color_label: [27, 255, 128, 166],
            color_unit: [166, 166, 166, 220],
            color_warning: [216, 33, 13, 100],
            color_shade_shape: [0, 0, 0, 42],
            voice_volumn: 100,
            aa_enable: true,
            text_aa_setting: TextAaSetting::Gasp,
            graph_aa_setting: GraphAaSetting::On,
            display_fm_key: 25,
            app_port: 0,
            app_port_bkp: 0,
            request_dest: None,
            request_dest_bkp: None,
            default_numfont_name: "Roboto".to_string(),
            default_font_name: "Microsoft YaHei UI".to_string(),
            default_font: None,
            screen_width: 0,
            screen_height: 0,
        }
    }
}

impl Default for ApplicationState {
    fn default() -> Self {
        ApplicationState::new()
    }
}

/// Controller 轮询间隔字段的消费面 (依赖桩, 非翻译):
/// loadAppCheck 写入的 6 个 `public long` 字段 (Java 声明默认 0, §2.10)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControllerIntervals {
    pub service_loop_interval_ms: i64,
    pub engine_info_interval_ms: i64,
    pub flight_info_interval_ms: i64,
    pub altitude_interval_ms: i64,
    pub gear_flaps_interval_ms: i64,
    pub control_input_interval_ms: i64,
}

// =====================================================================
// Tests — Java 侧无对应单测 (ConfigurationService 为手动验证), 本组为移植
// 边界钉子 + 真实 ui_layout.cfg 实例断言。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_loader::ConfigValue;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static CFG_N: AtomicUsize = AtomicUsize::new(0);

    fn tmp_cfg(content: &str) -> String {
        let n = CFG_N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir()
            .join(format!("vm_core_cfgsvc_{}_{n}.cfg", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        fs::write(&p, content).unwrap();
        p
    }

    /// 无总线服务 + 从临时 cfg 装载
    fn svc(content: &str) -> ConfigurationService {
        let s = ConfigurationService::new(None);
        s.load_layout(&tmp_cfg(content));
        s
    }

    /// 带总线服务 + 事件记录器 (返回订阅句柄保活 — EventBus RAII, Drop 即退订)
    fn svc_bus(
        content: &str,
    ) -> (
        ConfigurationService,
        Arc<Mutex<Vec<UiStateEvent>>>,
        crate::bus::Subscription<UiStateEvent>,
    ) {
        let bus = Arc::new(EventBus::new());
        let log = Arc::new(Mutex::new(Vec::new()));
        let l2 = Arc::clone(&log);
        let sub = bus.subscribe(move |ev: &UiStateEvent| l2.lock().unwrap().push(ev.clone()));
        let s = ConfigurationService::new(Some(bus));
        s.load_layout(&tmp_cfg(content));
        (s, log, sub)
    }

    /// 仓库根真实 ui_layout.cfg 装载 (config_loader 同款路径)
    fn repo_cfg_path() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../ui_layout.cfg")
            .to_str()
            .unwrap()
            .to_string()
    }

    /// reset/save 类用例会向 CWD 写 ./ui_layout.user.cfg (ConfigManager 路径常量
    /// 相对固定) — Drop 守卫清理。注: 不 chdir 沙箱, 因跨模块 CWD 测试锁尚无
    /// 共享基建 (config_manager.rs 测试注释 / 审查 B4), CWD 变更会与既有用例互扰。
    struct UserCfgGuard;
    impl Drop for UserCfgGuard {
        fn drop(&mut self) {
            let _ = fs::remove_file("./ui_layout.user.cfg");
            let _ = fs::remove_file("./ui_layout.user.cfg.bak");
        }
    }

    // ---- getConfig 优先级 / findRowRecursive ----

    #[test]
    fn get_config_priority_row_groupkey_missing() {
        let s = svc(
            "(panel \"G1\" :switch-key \"g1Switch\" :visible true\n\
             \x20 (item \"s\" :type switch :target \"k1\" :value true)\n\
             \x20 (item \"d\" :type data :target \"getIAS\" :unit \"Km/h\")\n\
             \x20 (group \"Sub\" (item \"n\" :type switch :target \"k2\" :value false))\n\
             \x20 (item \"onlyLabel\" :type info :value 7)\n)\
            ",
        );
        // 命中行 target
        assert_eq!(s.get_config("k1"), Some("true".to_string()));
        // 嵌套 group 子行递归
        assert_eq!(s.get_config("k2"), Some("false".to_string()));
        // 分组 switchKey → visible
        assert_eq!(s.get_config("g1Switch"), Some("true".to_string()));
        // property 缺失时按 label 命中
        assert_eq!(s.get_config("onlyLabel"), Some("7".to_string()));
        // 未找到: Java 实现恒返回 "" (非 null)
        assert_eq!(s.get_config("missing"), Some(String::new()));
    }

    // ---- SWITCH_INV 反转 ----

    #[test]
    fn switch_inv_inversion_cycle() {
        let s = svc("(panel \"P\" (item \"i\" :type switch-inv :target \"disableX\" :value true))");
        // getConfig = String.valueOf(!getBool()): value true → "false"
        assert_eq!(s.get_config("disableX"), Some("false".to_string()));
        // isFieldDisabled = !getBool(): value true → 未禁用
        assert!(!s.is_field_disabled("disableX"));
        // 视图 getBool 同反转
        let v = s.get_overlay_settings("P");
        assert!(!v.get_bool("disableX", false));

        // setConfig("true") → 存储 !parseBoolean("true") = false
        s.set_config("disableX", "true");
        assert_eq!(s.get_config("disableX"), Some("true".to_string()));
        assert!(s.is_field_disabled("disableX"));
        assert!(v.get_bool("disableX", false));
    }

    // ---- setConfig 全实例更新 + 类型一致性 ----

    #[test]
    fn set_config_updates_all_instances_and_types() {
        let s = svc(
            "(panel \"A\" (item \"i\" :type switch :target \"k1\" :value false))\n\
             (panel \"B\" (item \"j\" :type switch :target \"k1\" :value false)\n\
             \x20 (item \"s\" :type slider :target \"k2\" :value 1))\
            ",
        );
        // 递归更新 ALL 匹配实例 (两个分组同名 key)
        s.set_config("k1", "true");
        let cfgs = s.get_layout_configs().unwrap();
        assert_eq!(cfgs[0].rows[0].value, Some(ConfigValue::Bool(true)));
        assert_eq!(cfgs[1].rows[0].value, Some(ConfigValue::Bool(true)));

        // Integer 行: 可解析 → Int; 不可解析 → 存 String (Java catch 分支)
        s.set_config("k2", "42");
        assert_eq!(s.get_config("k2"), Some("42".to_string()));
        s.set_config("k2", "abc");
        assert_eq!(s.get_config("k2"), Some("abc".to_string()));
    }

    #[test]
    fn set_config_group_switchkey_visible_and_event() {
        let (s, log, _sub) = svc_bus(
            "(panel \"P\" :switch-key \"pSwitch\" :visible false\n\
             \x20 (item \"x\" :type switch :target \"k\" :value true))\
            ",
        );
        s.set_config("pSwitch", "true");
        assert_eq!(s.get_config("pSwitch"), Some("true".to_string()));
        assert!(s.get_layout_configs().unwrap()[0].visible);
        // parseBoolean 非 "true" 一律 false
        s.set_config("pSwitch", "whatever");
        assert_eq!(s.get_config("pSwitch"), Some("false".to_string()));
        assert_eq!(log.lock().unwrap().len(), 2);
    }

    #[test]
    fn set_config_event_payloads_via_bus() {
        let (s, log, _sub) = svc_bus(
            "(panel \"P\" :switch-key \"pSwitch\"\n\
             \x20 (item \"x\" :type switch :target \"k\" :value true))\
            ",
        );
        s.set_config("k", "false");
        s.set_config("pSwitch", "true");
        let events = log.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            UiStateEvent {
                event_type: "configChanged".to_string(),
                source: "ConfigurationService".to_string(),
                data: "k".to_string(),
            }
        );
        assert_eq!(events[1].data, "pSwitch");
        assert_eq!(events[1].event_type, "configChanged");
    }

    /// initConfig/loadLayout 之前 (layoutConfigs == null): 全部读面为空、写面 no-op
    #[test]
    fn set_config_without_layout_noop() {
        // 单总线 + 记录器订阅保活 + 未装载布局的服务
        let bus = Arc::new(EventBus::new());
        let log = Arc::new(Mutex::new(Vec::new()));
        let l2 = Arc::clone(&log);
        let _sub = bus.subscribe(move |ev: &UiStateEvent| l2.lock().unwrap().push(ev.clone()));
        let s = ConfigurationService::new(Some(bus));
        assert_eq!(s.get_layout_configs(), None);
        assert_eq!(s.get_config("any"), Some(String::new()));
        s.set_config("any", "v"); // 无事件、无崩溃
        assert!(!s.is_field_disabled("any"));
        assert!(!s.reset_all_layout_defaults());
        assert!(log.lock().unwrap().is_empty());
    }

    // ---- isFieldDisabled ----

    #[test]
    fn is_field_disabled_matrix() {
        let s = svc(
            "(panel \"P\"\n\
             \x20 (item \"inv1\" :type switch-inv :target \"disableX\" :value true)\n\
             \x20 (item \"inv2\" :type switch-inv :target \"disableY\" :value false)\n\
             \x20 (item \"sw\" :type switch :target \"boolFalse\" :value false)\n\
             \x20 (item \"sw2\" :type switch :target \"boolTrue\" :value true)\n\
             \x20 (item \"sl\" :type slider :target \"intRow\" :value 5)\n)\
            ",
        );
        assert!(!s.is_field_disabled("")); // Java: null/empty → false
        // SWITCH_INV: !getBool
        assert!(!s.is_field_disabled("disableX")); // value true → !true
        assert!(s.is_field_disabled("disableY")); // value false → !false
        // Boolean 值: false → 禁用
        assert!(s.is_field_disabled("boolFalse"));
        assert!(!s.is_field_disabled("boolTrue"));
        // 非 Boolean 值: 不返回, 继续扫描 → false
        assert!(!s.is_field_disabled("intRow"));
        assert!(!s.is_field_disabled("never-set"));
    }

    // ---- resetAllLayoutDefaults 三阶段 ----

    #[test]
    fn reset_all_layout_defaults_phases_and_notify() {
        let _guard = UserCfgGuard;
        let (s, log, _sub) = svc_bus(
            "(panel \"P\" :switch-key \"pSwitch\" :visible false\n\
             \x20 (group \"G\"\n\
             \x20   (item \"a\" :type switch :target \"k1\" :value true :default false)\n\
             \x20   (item \"b\" :type slider :target \"k2\" :value 9 :default 7)\n\
             \x20   (item \"c\" :type switch :target \"k3\" :value true :default true)\n\
             \x20   (item \"d\" :type combo :target \"kc\" :default \"A\"))\n)\
            ",
        );
        // value null + default "A": Java defaultValue.equals(null)=false → 仍收集
        assert_eq!(s.get_config("kc"), Some("null".to_string())); // getStr 对 null → "null"

        s.set_config("k1", "true"); // 先产生一条普通变更事件
        assert!(s.reset_all_layout_defaults());

        // Phase 2: 偏离 default 的行回写 default; 相同的 k3 不动
        assert_eq!(s.get_config("k1"), Some("false".to_string()));
        assert_eq!(s.get_config("k2"), Some("7".to_string()));
        assert_eq!(s.get_config("k3"), Some("true".to_string()));
        assert_eq!(s.get_config("kc"), Some("A".to_string()));
        // 分组 visible 非 RowConfig, 不在重置范围
        assert_eq!(s.get_config("pSwitch"), Some("false".to_string()));
        // Phase 3: 恰一条 RESET_COMPLETED, 顺序在 set 事件之后
        let events = log.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "k1");
        assert_eq!(events[1].data, "RESET_COMPLETED");
        assert_eq!(events[1].event_type, "configChanged");
    }

    #[test]
    fn reset_all_layout_defaults_no_change() {
        let (s, log, _sub) = svc_bus(
            "(panel \"P\" (item \"a\" :type switch :target \"k1\" :value true :default true))",
        );
        assert!(!s.reset_all_layout_defaults());
        assert!(log.lock().unwrap().is_empty());
    }

    /// 失败路径: 源缺失 / 模板缺失 (crate CWD 无 ./ui_layout.cfg) → false 且不发事件
    #[test]
    fn import_reset_failure_paths() {
        let _guard = UserCfgGuard;
        let (s, log, _sub) = svc_bus("(panel \"P\")");
        assert!(!s.import_config("definitely_missing_zzz.cfg"));
        assert!(!s.reset_to_factory());
        assert!(log.lock().unwrap().is_empty());
        assert_eq!(s.get_config("k"), Some(String::new()));
    }

    // ---- 组装层位置桥 (group_position / save_group_position) ----

    #[test]
    fn group_position_read_write_roundtrip() {
        let _guard = UserCfgGuard; // save_group_position 落盘 → Drop 清理 ./ui_layout.user.cfg
        let s = svc("(panel \"飞行信息\" :x 0.0602 :y 0.1188)");
        // 读: 归一化原值 (忽略大小写, 对齐视图 getGroupConfig 语义)
        assert_eq!(s.group_position("飞行信息"), Some((0.0602, 0.1188)));
        assert_eq!(s.group_position("FLIGHT 信息"), None); // 未命中 → None (host 居中兜底)
        // 写: 归一化直写 + 回读一致 (落盘副作用同 save_window_position 测试先例)
        assert!(s.save_group_position("飞行信息", 0.25, 0.75));
        assert_eq!(s.group_position("飞行信息"), Some((0.25, 0.75)));
        // 未命中写: false 不 panic (Java warn 分支)
        assert!(!s.save_group_position("不存在", 0.1, 0.1));
        // 与视图像素面一致性: 写归一化后 get_window_x 跟随 (同源字段)
        s.set_screen_size(1920, 1080);
        assert_eq!(s.get_overlay_settings("飞行信息").get_window_x(0), 480); // round(0.25*1920)
    }

    // ---- findGroupByTitle (大小写敏感) vs 视图 getGroupConfig (忽略大小写) ----

    #[test]
    fn find_group_by_title_case_sensitive() {
        let s = svc("(panel \"Alpha\" :x 0.5 :y 0.25)\n(panel \"beta\")");
        assert!(s.find_group_by_title("Alpha").is_some());
        assert!(s.find_group_by_title("alpha").is_none()); // equals 大小写敏感
        assert!(s.find_group_by_title("").is_none());
        assert!((s.find_group_by_title("Alpha").unwrap().x - 0.5).abs() < 1e-12);

        // 视图查找忽略大小写
        s.set_screen_size(100, 100);
        let v = s.get_overlay_settings("ALPHA");
        assert_eq!(v.get_window_x(0), 50); // round(0.5*100)
        assert_eq!(v.get_window_y(0), 25);
    }

    // ---- 真实 ui_layout.cfg: 飞行信息视图 (任务必测项) ----

    #[test]
    fn overlay_settings_flight_info_repo() {
        let s = ConfigurationService::new(None);
        s.load_layout(&repo_cfg_path());
        s.set_screen_size(1920, 1080);
        let v = s.get_overlay_settings("飞行信息");

        // gc.x=0.0602 * 1920 = 115.584 → Math.round → 116
        assert_eq!(v.get_window_x(300), 116);
        // gc.y=0.1188 * 1080 = 128.304 → 128
        assert_eq!(v.get_window_y(300), 128);

        // :font "Sarasa Mono SC" (面板级字体)
        assert_eq!(v.get_font_name(), "Sarasa Mono SC");
        // 无 :font-size 面板属性 → 0 ("大小"滑条是行级 fontSize, 不影响 getFontSizeAdd)
        assert_eq!(v.get_font_size_add(), 0);

        // 组内行读取
        assert!(v.get_bool("flightInfoSwitch", false));
        assert!(!v.get_bool("flightInfoEdge", true));
        assert_eq!(v.get_int("flightInfoColumn", 0), 1);
        assert_eq!(v.get_int("fontSize", -99), 0);

        // trait 借出面
        assert_eq!(v.get_group_config().map(|g| g.title.as_str()), Some("飞行信息"));
    }

    // ---- 视图回退分支 ----

    #[test]
    fn overlay_center_fallback_and_guard_branches() {
        let s = svc("(panel \"A\" :x 0.9 :y 0.9)");
        s.set_screen_size(1920, 1080);
        // 分组不存在 → 居中回退 (int 除法)
        let v = s.get_overlay_settings("Zzz");
        assert_eq!(v.get_window_x(400), (1920 - 400) / 2);
        assert_eq!(v.get_window_y(300), (1080 - 300) / 2);
        // gc=null 分支 saveWindowPosition → warn (无保存、无崩溃)
        v.save_window_position(10.0, 10.0);

        // 分组存在但 screen<=0 → Java gc!=null && sw>0 && sh>0 为假 → warn
        let s2 = svc("(panel \"A\" :x 0.9 :y 0.9)");
        s2.set_screen_size(0, 0);
        s2.get_overlay_settings("A").save_window_position(10.0, 10.0);

        // screen=0 的读面: round(0.9*0)=0 (Java 同)
        assert_eq!(s2.get_overlay_settings("A").get_window_x(50), 0);
    }

    // ---- 真实 ui_layout.cfg: MiniHUD HUDSettings ----

    #[test]
    fn hud_settings_repo_minihud() {
        let s = ConfigurationService::new(None);
        s.load_layout(&repo_cfg_path());
        s.set_screen_size(1920, 1080);
        let h = s.get_hud_settings();

        // gc.x=0.3891*1920=747.072 → 747; gc.y=0.7042*1080=760.536 → 761
        assert_eq!(h.get_window_x(400), 747);
        assert_eq!(h.get_window_y(400), 761);

        assert_eq!(h.get_crosshair_scale(), 113); // :value 113
        assert_eq!(h.get_crosshair_name(), "软件渲染准星");
        assert!(h.is_display_crosshair()); // displayCrosshair=true
        assert!(!h.use_texture_crosshair()); // "软件渲染准星" → false
        assert!(h.draw_hud_text()); // drawHUDtext=true
        assert!(h.show_attitude_gauge()); // 默认 true (cfg 无该键)

        // AoA 比率: 布局优先 (Int 20 → 20>1 → 0.2; Int 25 → 0.25)
        assert!((h.get_aoa_warning_ratio() - 0.2).abs() < 1e-12);
        assert!((h.get_aoa_bar_warning_ratio() - 0.25).abs() < 1e-12);

        // 字体链: MonoNumFont = "Sarasa Mono SC"
        assert_eq!(h.get_num_font(), "Sarasa Mono SC");
        assert_eq!(h.get_num_font_name(), "Sarasa Mono SC");
        // getFontSizeAdd 走父类 (MiniHUD 无 :font-size → 0)
        assert_eq!(h.get_font_size_add(), 0);
    }

    // ---- HUDSettings 准星回退分支 ----

    #[test]
    fn hud_crosshair_fallback_writeback() {
        let s = svc(
            "(panel \"Other\"\n\
             \x20 (item \"cx\" :type slider :target \"crosshairX\" :value 500)\n\
             \x20 (item \"cy\" :type slider :target \"crosshairY\" :value 300))\
            ",
        );
        s.set_screen_size(1920, 1080);
        let h = s.get_hud_settings();
        // 无 "MiniHUD" 分组 → crosshairX/Y 行优先, 缺省才用居中默认
        assert_eq!(h.get_window_x(400), 500);
        assert_eq!(h.get_window_y(300), 300);

        // 位置写回走 setConfig((int)x)
        h.save_window_position(123.7, -45.9);
        assert_eq!(s.get_config("crosshairX"), Some("123".to_string()));
        assert_eq!(s.get_config("crosshairY"), Some("-45".to_string()));

        // 行也不存在 → 居中默认 (int 除法)
        let s2 = svc("(panel \"Other\")");
        s2.set_screen_size(1920, 1080);
        let h2 = s2.get_hud_settings();
        assert_eq!(h2.get_window_x(400), (1920 - 400) / 2);
        assert_eq!(h2.get_window_y(300), (1080 - 300) / 2);
    }

    // ---- AoA 比率归一 + getDoubleFromLayoutFirst ----

    #[test]
    fn hud_aoa_ratio_and_layout_first() {
        // 字符串值: parse 成功; Int 值: Number.doubleValue; >1 归一 /100
        let s = svc(
            "(panel \"MiniHUD\"\n\
             \x20 (group \"G\"\n\
             \x20   (item \"a\" :type info :target \"miniHUDaoaWarningRatio\" :value \"0.5\")\n\
             \x20   (item \"b\" :type info :target \"miniHUDaoaBarWarningRatio\" :value 100)\n\
             \x20   (item \"c\" :type info :target \"extraKey\" :value 2.5)\n\
             \x20   (item \"d\" :type info :target \"edgeKey\" :value 1)\n)\
            ",
        );
        let h = s.get_hud_settings();
        assert!((h.get_aoa_warning_ratio() - 0.5).abs() < 1e-12); // 0.5 ≤ 1 → 原值
        assert!((h.get_aoa_bar_warning_ratio() - 1.0).abs() < 1e-12); // 100 → 1.0
        assert!((h.get_double_from_layout_first("MiniHUD", "extraKey", 9.0) - 2.5).abs() < 1e-12); // Double 分支
        assert!((h.get_double_from_layout_first("MiniHUD", "edgeKey", 9.0) - 1.0).abs() < 1e-12); // Int 分支

        // 不可解析字符串 → catch ignore → Priority 2 全局 getDouble → 默认 25 → 0.25
        let s2 = svc(
            "(panel \"MiniHUD\" (group \"G\"\n\
             \x20 (item \"a\" :type info :target \"miniHUDaoaWarningRatio\" :value \"abc\")))\
            ",
        );
        assert!((s2.get_hud_settings().get_aoa_warning_ratio() - 0.25).abs() < 1e-12);

        // 段名不匹配 → Priority 1 落空; Priority 2 的 getDouble→getConfig 是
        // 全局行查找 (不限段), 命中 Other 面板的同 target 行 → 90 → 0.9 (Java 同流)
        let s3 = svc("(panel \"Other\" (item \"a\" :type info :target \"miniHUDaoaWarningRatio\" :value 90))");
        assert!((s3.get_hud_settings().get_aoa_warning_ratio() - 0.9).abs() < 1e-12);
    }

    // ---- loadAppCheck: 真实 ui_layout.cfg 全同步链 (任务必测项) ----

    #[test]
    fn load_app_check_repo_full_sync() {
        let s = ConfigurationService::new(None);
        s.load_layout(&repo_cfg_path());
        let mut c = ControllerIntervals::default();
        s.load_app_check(&mut c);
        let app = s.application_state();

        // dataPollIntervalMs=80: 2f/1.5f/1f 系数与 /3 整除
        assert_eq!(c.service_loop_interval_ms, 80);
        assert_eq!(c.engine_info_interval_ms, 160);
        assert_eq!(c.flight_info_interval_ms, 120);
        assert_eq!(c.altitude_interval_ms, 120);
        assert_eq!(c.gear_flaps_interval_ms, 160);
        assert_eq!(c.control_input_interval_ms, 80);
        assert_eq!(app.thread_sleep_time, 26); // 80/3 整除

        // 颜色 (#RRGGBBAA 语义: fontUnit #E89332FF → R=E8 G=93 B=32 A=FF)
        assert_eq!(app.color_num, [255, 255, 255, 255]);
        assert_eq!(app.color_label, [255, 255, 255, 255]);
        assert_eq!(app.color_unit, [232, 147, 50, 255]);
        assert_eq!(app.color_warning, [255, 36, 0, 255]);
        assert_eq!(app.color_shade_shape, [0, 0, 0, 255]);

        // voiceVolume=100 / AAEnable=true → On/On (默认链路: 值在 → parseBoolean)
        assert_eq!(app.voice_volumn, 100);
        assert!(app.aa_enable);
        assert_eq!(app.text_aa_setting, TextAaSetting::On);
        assert_eq!(app.graph_aa_setting, GraphAaSetting::On);

        // displayFmKey=25 (VC_P)
        assert_eq!(app.display_fm_key, 25);

        // HTTP 端口: 8111 / +1111; Lang.httpIp 默认 127.0.0.1
        assert_eq!(app.app_port, 8111);
        assert_eq!(app.app_port_bkp, 9222);
        assert_eq!(
            app.request_dest,
            Some(InetSocketAddress::new("127.0.0.1", 8111))
        );
        assert_eq!(
            app.request_dest_bkp,
            Some(InetSocketAddress::new("127.0.0.1", 9222))
        );

        // 全局字体同步
        assert_eq!(app.default_numfont_name, "Sarasa Mono SC");
        assert_eq!(app.default_font_name, "Sarasa Mono SC");
    }

    // ---- loadAppCheck: 键缺失时的默认链 ----

    #[test]
    fn load_app_check_missing_keys_defaults() {
        let s = svc("(panel \"P\" (item \"s\" :type switch :target \"someKey\" :value true))");
        let mut c = ControllerIntervals::default();
        s.load_app_check(&mut c);
        let app = s.application_state();

        // 间隔默认 50
        assert_eq!(c.service_loop_interval_ms, 50);
        assert_eq!(c.engine_info_interval_ms, 100);
        assert_eq!(c.flight_info_interval_ms, 75);
        assert_eq!(c.altitude_interval_ms, 75);
        assert_eq!(c.gear_flaps_interval_ms, 100);
        assert_eq!(c.control_input_interval_ms, 50);
        assert_eq!(app.thread_sleep_time, 16); // 50/3

        // 颜色缺失 → ColorHelper 默认 Color.WHITE
        assert_eq!(app.color_num, [255, 255, 255, 255]);
        assert_eq!(app.color_warning, [255, 255, 255, 255]);

        // voiceVolume 缺失 → 不触碰声明默认 100
        assert_eq!(app.voice_volumn, 100);

        // AAEnable 缺失 → false (else 分支) → Off
        assert!(!app.aa_enable);
        assert_eq!(app.text_aa_setting, TextAaSetting::Off);
        assert_eq!(app.graph_aa_setting, GraphAaSetting::Off);

        // 端口缺失 → 不同步
        assert_eq!(app.app_port, 0);
        assert_eq!(app.request_dest, None);

        // 字体缺失 → 声明默认
        assert_eq!(app.default_numfont_name, "Roboto");
        assert_eq!(app.default_font_name, "Microsoft YaHei UI");
    }

    // ---- loadAppCheck: 非法值恢复 ----

    #[test]
    fn load_app_check_bad_values_recovery() {
        let s = svc(
            "(panel \"P\"\n\
             \x20 (item \"i\" :type slider :target \"Interval\" :value 100)\n\
             \x20 (item \"v\" :type slider :target \"voiceVolume\" :value 0)\n\
             \x20 (item \"p\" :type data :target \"httpPort\" :value \"abc\")\n\
             \x20 (item \"a\" :type switch :target \"AAEnable\" :value false))\
            ",
        );
        let mut c = ControllerIntervals::default();
        s.load_app_check(&mut c);
        let app = s.application_state();

        // dataPollIntervalMs 缺失 → legacy "Interval"=100
        assert_eq!(c.service_loop_interval_ms, 100);
        assert_eq!(c.engine_info_interval_ms, 200);
        assert_eq!(c.flight_info_interval_ms, 150);
        assert_eq!(app.thread_sleep_time, 33);

        // voiceVolume="0" → 0 (合法解析)
        assert_eq!(app.voice_volumn, 0);
        // httpPort="abc" → NumberFormatException → Ignore (端口面不动)
        assert_eq!(app.app_port, 0);
        assert_eq!(app.request_dest, None);
        // AAEnable=false → Off
        assert!(!app.aa_enable);
        assert_eq!(app.text_aa_setting, TextAaSetting::Off);

        // 新键非数值 → NumberFormatException → 50 (legacy 键不再回看)
        let s2 = svc(
            "(panel \"P\"\n\
             \x20 (item \"n\" :type data :target \"dataPollIntervalMs\" :value \"xyz\")\n\
             \x20 (item \"i\" :type slider :target \"Interval\" :value 300))\
            ",
        );
        let mut c2 = ControllerIntervals::default();
        s2.load_app_check(&mut c2);
        assert_eq!(c2.service_loop_interval_ms, 50);
    }

    // ---- ColorHelper 消费面 ----

    #[test]
    fn color_parse_matrix() {
        let w = [255, 255, 255, 255];
        // hex
        assert_eq!(parse_color("#FF5500", w), [255, 85, 0, 255]); // RGB → alpha 255
        assert_eq!(parse_color("#FF5500AA", w), [255, 85, 0, 170]);
        assert_eq!(parse_color("#ff5500aa", w), [255, 85, 0, 170]); // 小写 hex
        // decimal
        assert_eq!(parse_color("255, 85, 0, 170", w), [255, 85, 0, 170]);
        assert_eq!(parse_color("255, 85, 0", w), [255, 85, 0, 255]);
        // clamp
        assert_eq!(parse_color(" 300 , -5 , 0 , 999 ", w), [255, 0, 0, 255]);
        // Java split 尾部空串剔除: "1,2,3," → 3 段
        assert_eq!(parse_color("1,2,3,", w), [1, 2, 3, 255]);
        // 失败 → 默认
        assert_eq!(parse_color("1,2", w), w);
        assert_eq!(parse_color("#GGGGGG", w), w);
        assert_eq!(parse_color("#12345", w), w); // 长度非 6/8
        assert_eq!(parse_color("1,2,c,4", w), w);
        assert_eq!(parse_color("", w), w);
        assert_eq!(parse_color("   ", w), w);
    }

    #[test]
    fn set_color_config_roundtrip() {
        let s = svc("(panel \"P\" (item \"c\" :type color :target \"fontNum\" :value \"#FFFFFFFF\"))");
        // 原值 hex 可读
        assert_eq!(s.get_color_config("fontNum"), [255, 255, 255, 255]);
        // 写回十进制 "R, G, B, A" 格式 (Java: R + ", " + G + ...)
        s.set_color_config("fontNum", [1, 2, 3, 4]);
        assert_eq!(s.get_config("fontNum"), Some("1, 2, 3, 4".to_string()));
        assert_eq!(s.get_color_config("fontNum"), [1, 2, 3, 4]);
    }

    // ---- trait 对象分发 ----

    #[test]
    fn dyn_trait_dispatch() {
        let s = svc(
            "(panel \"MiniHUD\" :x 0.5 :y 0.5\n\
             \x20 (item \"sw\" :type switch :target \"showSpeedBar\" :value false))\
            ",
        );
        s.set_screen_size(100, 100);

        // Box<dyn ConfigProvider> (面向接口编程)
        let p: Box<dyn ConfigProvider> = Box::new(s.clone());
        p.set_config("showSpeedBar", "true");
        assert_eq!(p.get_config("showSpeedBar"), Some("true".to_string()));
        assert!(!p.is_field_disabled("showSpeedBar"));

        // Box<dyn HUDSettings> + 上转 &dyn OverlaySettings (单 vtable 槽)
        let h: Box<dyn HUDSettings<GroupConfig = GroupConfig>> = Box::new(s.get_hud_settings());
        assert!(h.show_speed_bar(), "setConfig 后视图重查应见新值");
        assert_eq!(h.get_window_x(0), 50);
        let base: &dyn OverlaySettings<GroupConfig = GroupConfig> = &*h;
        assert_eq!(base.get_window_x(0), 50);
        assert_eq!(base.get_font_size_add(), 0);
    }

    // ---- saveConfig 空方法 / saveLayoutConfig 空配置守卫 ----

    #[test]
    fn save_config_noop_and_empty_layout_guard() {
        let s = ConfigurationService::new(None);
        s.save_config(); // Java: 空方法 (No longer using config.properties)
        s.save_layout_config(); // layoutConfigs == null → 无落盘/无日志副作用, 不崩溃
    }

    // ---- 修复波次: InetSocketAddress 桩抛出面 / Double.equals 位级 / toString ----

    #[test]
    fn inet_socket_address_port_bounds() {
        assert_eq!(InetSocketAddress::new("host", 0).port, 0);
        let b = InetSocketAddress::new("host", 65535);
        assert_eq!((b.host.as_str(), b.port), ("host", 65535));
    }

    /// JDK: port 越界 → IllegalArgumentException (非 NumberFormatException,
    /// 不被 loadAppCheck 的 catch 捕获) — Rust panic! 复刻
    #[test]
    #[should_panic(expected = "port out of range")]
    fn inet_socket_address_negative_port_panics() {
        let _ = InetSocketAddress::new("host", -1);
    }

    #[test]
    #[should_panic(expected = "port out of range")]
    fn inet_socket_address_overflow_port_panics() {
        let _ = InetSocketAddress::new("host", 65536);
    }

    /// Double.toString 本地副本 — 期望值来自 Java 8 oracle 逐字面量对拍
    /// (config_loader 同款 battery; 科学计数域 0.0001/1e7 为修复覆盖点)
    #[test]
    fn java_double_to_string_matches_java8_oracle() {
        let cases = [
            (0.03125, "0.03125"),
            (1.5, "1.5"),
            (1.0e7, "1.0E7"),
            (9999999.0, "9999999.0"),
            (0.001, "0.001"),
            (0.0001, "1.0E-4"),
            (1.0e-5, "1.0E-5"),
            (123456789012345.6, "1.234567890123456E14"),
            (0.0, "0.0"),
            (1.0000000000000002, "1.0000000000000002"),
            (0.002, "0.002"),
        ];
        for (d, want) in cases {
            assert_eq!(java_double_to_string(d), want, "{d} → 期望 {want}");
        }
        assert_eq!(java_double_to_string(-0.0), "-0.0");
        assert_eq!(java_double_to_string(f64::NAN), "NaN");
        assert_eq!(java_double_to_string(f64::INFINITY), "Infinity");
        assert_eq!(java_double_to_string(f64::NEG_INFINITY), "-Infinity");
        // ConfigValue 面 (reset 日志的 default 回显文本)
        assert_eq!(config_value_to_java_string(&ConfigValue::Double(1.0e7)), "1.0E7");
        assert_eq!(config_value_to_java_string(&ConfigValue::Double(20.0)), "20.0");
    }

    /// Double.equals = doubleToLongBits 位级: NaN==NaN true、+0.0!=-0.0;
    /// 异型 instanceof 恒 false — 均与派生 PartialEq 相反 (Java 语义钉子)
    #[test]
    fn config_value_java_equals_double_bits() {
        assert!(config_value_java_equals(
            &ConfigValue::Double(f64::NAN),
            &ConfigValue::Double(f64::NAN)
        ));
        assert!(!config_value_java_equals(&ConfigValue::Double(0.0), &ConfigValue::Double(-0.0)));
        assert!(!config_value_java_equals(&ConfigValue::Double(-0.0), &ConfigValue::Double(0.0)));
        assert!(config_value_java_equals(&ConfigValue::Double(2.5), &ConfigValue::Double(2.5)));
        assert!(!config_value_java_equals(&ConfigValue::Double(1.0), &ConfigValue::Int(1)));
        assert!(config_value_java_equals(&ConfigValue::Int(1), &ConfigValue::Int(1)));
        assert!(config_value_java_equals(&ConfigValue::Bool(true), &ConfigValue::Bool(true)));
        assert!(!config_value_java_equals(
            &ConfigValue::Str("a".to_string()),
            &ConfigValue::Str("b".to_string())
        ));
    }

    /// 收集判定端到端: value=-0.0 / default=+0.0 → Java equals 不等 → 收集
    /// (派生 PartialEq 会判等漏收)。cfg 文本无法到达该位形 (解析器 -0.0 折叠
    /// Int(0)), 故直接构造 RowConfig。
    #[test]
    fn reset_collect_negative_zero_double() {
        let mut r = RowConfig::new("z".to_string(), None, String::new());
        r.value = Some(ConfigValue::Double(-0.0));
        r.default_value = Some(ConfigValue::Double(0.0));
        let mut pending = Vec::new();
        let mut path = Vec::new();
        collect_reset_candidates_recursive(&[r], 0, &mut path, &mut pending);
        assert_eq!(pending.len(), 1); // Java: Double(0.0).equals(Double(-0.0))=false

        // NaN 值 == NaN 默认 → Java equals 相等 → 不收集
        let mut r2 = RowConfig::new("n".to_string(), None, String::new());
        r2.value = Some(ConfigValue::Double(f64::NAN));
        r2.default_value = Some(ConfigValue::Double(f64::NAN));
        let mut pending2 = Vec::new();
        let mut path2 = Vec::new();
        collect_reset_candidates_recursive(&[r2], 0, &mut path2, &mut pending2);
        assert!(pending2.is_empty());
    }
}
