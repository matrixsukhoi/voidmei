//! ConfigurationService 的 Rust 移植 (src/prog/config/ConfigurationService.java) — 一比一翻译。
//!
//! Main configuration Service implementing ConfigProvider.
//! Handles loading, saving, and accessing application configuration.
//!
//! PORT: Java 内部类 GenericOverlaySettingsImpl / HUDSettingsImpl → 独立 struct
//! (任务裁决); 子类对父类的 extends → 组合 + 委托 (§1 继承禁令)。
//! PORT: Application 静态字段 (loadAppCheck 写入面 / OverlaySettings 读取面) 与
//! Controller 轮询间隔字段均属未翻译类 — 以 config/app_state.rs 的**消费面依赖桩**
//! 顶住 (config_manager.rs 的 UIStateStorage 桩先例), 注入式持有, 不造全局 (§2.9)。
//! PORT(重构波1): UIStateBus 路由总线经构造器注入 `Option<Arc<UIStateBus>>`
//! (Java `UIStateBus.getInstance()` 全局单例读法的依赖注入式收敛); 本服务
//! 原"未路由裸 EventBus + 桩 UiStateEvent"形态已退役, 发布统一走
//! ui_state_bus.rs 的路由总线与三参 publish。RESET_REQUEST 事件链由上层
//! 直接调用 reset_all_layout_defaults() 顶替 (见 init_config 注释)。

use std::sync::{Arc, RwLock};

use crate::config::config_api::{ConfigProvider, HUDSettings, OverlaySettings};
use crate::config::config_loader::{self, ConfigValue, GroupConfig, RowConfig};
use crate::config::config_manager;
use crate::base::event::ui_state_events;
use crate::lang::Lang;
use crate::base::logger;
use crate::base::bus::ui_state_bus::UIStateBus;

/// RwLock 中毒消息 (Java 无锁; 对应持锁线程崩溃后的一致性未知面)
const LC_LOCK_MSG: &str = "layoutConfigs 锁中毒";
const APP_LOCK_MSG: &str = "Application 状态锁中毒";

/// 配置写值钩子类型 (见 ServiceInner.write_hook 文档)
pub type WriteHook = Box<dyn Fn(&str, &str) + Send + Sync>;

// 重构波2: Application/Controller 消费面桩 + 抗锯齿值域枚举迁至 config/app_state.rs;
// 旧路径 (crate::config::configuration_service::xxx) 经 re-export 保持有效
pub use crate::config::app_state::{
    AppFont, ApplicationState, ControllerIntervals, GlobalColors, GraphAaSetting,
    InetSocketAddress, TextAaSetting,
};

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
    /// UIStateBus 路由总线 (重构波1: 原桩裸 EventBus 退役)
    ui_state_bus: Option<Arc<UIStateBus>>,
    /// 配置写值钩子 (重构波1): set_config 每键写树后、CONFIG_CHANGED 广播前
    /// 同步调用 — App 层注册以直写跨线程快照 (voice_*/FM show* 键), 保证
    /// 快照新值先于订阅者 (VoiceWarning reload) 执行。闭包只捕获 Send 快照,
    /// 不触碰 !Send 配置树 (锁纪律同 §2.8: 钩子在放锁后调用)。
    write_hook: RwLock<Option<WriteHook>>,
}

impl ConfigurationService {
    /// Java: `public ConfigurationService()` — 构造器为空。
    /// PORT: 参数为 UIStateBus 注入 (Java 读全局单例; §2.9 禁再造全局)。
    // PORT: Java 保真 — Arc<ServiceInner> 复刻 Java this 引用的跨方法共享,
    // 内部 RwLock 字段按 Java 字段语义组织, 不为 Sync 约束改形状
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new(ui_state_bus: Option<Arc<UIStateBus>>) -> Self {
        // Initialize config class (property loader)
        // Note: Actual loading happens in initConfig()
        ConfigurationService {
            inner: Arc::new(ServiceInner {
                layout_configs: RwLock::new(None),
                app: RwLock::new(ApplicationState::new()),
                ui_state_bus,
                write_hook: RwLock::new(None),
            }),
        }
    }

    /// 注册配置写值钩子 (见 ServiceInner.write_hook 文档); 覆盖式单装。
    pub fn set_write_hook(&self, hook: WriteHook) {
        *self.inner.write_hook.write().expect(APP_LOCK_MSG) = Some(hook);
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
        //           if (ACTION_RESET_REQUEST.equals(key)) resetAllLayoutDefaults(); })
        // PORT(重构波1 裁决): Java 构造器内的该订阅在 Rust 不接线 — 配置树
        // !Send (Rc<SExp>) 无法被订阅闭包捕获, RESET_REQUEST 事件链由上层
        // 直接调用 reset_all_layout_defaults() 顶替 (vm-ui main_form 的
        // "resetConfig" 按钮, 生产无 ACTION_RESET_REQUEST 发布点);
        // reset 内部的 RESET_COMPLETED 广播照常走注入总线。总线嵌套 publish
        // 死锁已修 (ui_state_bus.rs), 该链路即便走总线也已安全。
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

    /// 五色运行时快照 (Java 组件直接读 Application.colorNum 等静态;
    /// Rust 侧配置 !Send 不能进 win32 线程, 组装层启动/WYSIWYG 色变时取快照
    /// 注入 vm-overlay 的 global_colors 仓)
    pub fn global_colors(&self) -> GlobalColors {
        let app = self.inner.app.read().expect("app 状态锁中毒");
        GlobalColors {
            num: app.color_num,
            label: app.color_label,
            unit: app.color_unit,
            warning: app.color_warning,
            shade_shape: app.color_shade_shape,
        }
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
            app.thread_sleep_time = service_loop_interval_ms / 3;

            app.color_num = color_num;
            app.color_label = color_label;
            app.color_unit = color_unit;
            app.color_warning = color_warning;
            app.color_shade_shape = color_shade_shape;

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
                if let Ok(k) = fm_key.parse::<i32>() {
                    app.display_fm_key = k;
                }
            }

            // --- HTTP Port Sync ---
            let port_str = self.inner.get_config_j("httpPort");
            if !port_str.is_empty() {
                if let Ok(port) = port_str.parse::<i32>() {
                    app.app_port = port;
                    // PORT §2.2: Java int + int 静默回绕 ↔ wrapping_add
                    app.app_port_bkp = port.wrapping_add(1111);
                    // Assuming httpIp is still from Lang or static 127.0.0.1
                    let mut ip = "127.0.0.1".to_string();
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

        // PORT: C 类接线 (AWT GraphicsEnvironment 注册 fonts/ 目录字体并解析
        // defaultFont / WebLaF 全局字体注入), vm-app/vm-ui 波次处理;
        // 落地前 ApplicationState.default_font 维持 None。
        // TODO(port): init_font + update_weblaf_fonts 接线 (get_font_name 的
        // defaultFont 回退分支届时生效)。
    }

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
                if let Some(sk) = &gc.switch_key {
                    if key == sk {
                        return gc.visible.to_string(); // String.valueOf(boolean)
                    }
                }
            }
        }
        String::new()
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
        // 产生顺序补发全部事件 (事件数量与顺序与 Java 一致)。
        // 重构波1: 广播前先过写值钩子 — 跨线程快照 (voice_*/FM show*) 先于
        // 订阅者拿到新值 (VoiceWarning reload 在 publish 栈内同步读快照)
        for k in events {
            if let Some(hook) = self.write_hook.read().expect(APP_LOCK_MSG).as_ref() {
                hook(&k, value);
            }
            self.publish_config_changed(&k);
        }
    }

    /// Java: `public boolean isFieldDisabled(String key)`
    fn is_field_disabled(&self, key: &str) -> bool {
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
                }
            }
        }
        false
    }

    /// Java: `public boolean resetAllLayoutDefaults()`
    fn reset_all_layout_defaults(&self) -> bool {
        let mut changed = false;
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
            bus.publish(
                ui_state_events::CONFIG_CHANGED,
                Some("ConfigurationService"),
                Some(data),
            );
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
        if row.property.as_deref() == Some(key) {
            return Some(row);
        }
        // Priority 2: Match label if property is missing
        if row.property.is_none() && key == row.label {
            return Some(row);
        }
        // Recurse
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

            row.value = match &row.value {
                Some(ConfigValue::Bool(_)) => {
                    Some(ConfigValue::Bool(java_parse_boolean(&val_to_store)))
                }
                Some(ConfigValue::Int(_)) => match val_to_store.parse::<i32>() {
                    Ok(i) => Some(ConfigValue::Int(i)),
                    Err(_) => Some(ConfigValue::Str(val_to_store)),
                },
                // Double / Str / null → String (Java else 分支)
                _ => Some(ConfigValue::Str(val_to_store)),
            };
            events.push(key.to_string());
        }
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
                        ConfigValue::Int(i) => return Some(f64::from(*i)),
                        ConfigValue::Double(d) => return Some(*d),
                        other => {
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
            // Rust f64 除零同义 — 与父类实现不同, 子类无 screen>0 守卫, 保真)
            self.base.service.set_group_position_ignore_case(
                &self.base.section_name,
                x / f64::from(screen_w),
                y / f64::from(screen_h),
            );
            self.base.service.save_layout_config();
        } else {
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
        java_trim(&self.base.service.get_config_j("crosshairName")).to_string()
    }

    /// Java: `@Override public boolean isDisplayCrosshair()`
    fn is_display_crosshair(&self) -> bool {
        self.base.get_bool("displayCrosshair", false)
    }

    /// Java: `@Override public boolean useTextureCrosshair()`
    fn use_texture_crosshair(&self) -> bool {
        let name = self.get_crosshair_name();
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
    // [ \t\n\x0B\f\r] (默认无 UNICODE_CHARACTER_CLASS)
    let cleaned: String = decimal
        .chars()
        .filter(|c| !matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r'))
        .collect();
    let mut parts: Vec<&str> = cleaned.split(',').collect();
    while parts.last() == Some(&"") {
        parts.pop();
    }

    if parts.len() >= 3 {
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
// Tests — Java 侧无对应单测 (ConfigurationService 为手动验证), 本组为移植
// 边界钉子 + 真实 ui_layout.cfg 实例断言。
// =====================================================================
#[cfg(test)]
mod tests;
