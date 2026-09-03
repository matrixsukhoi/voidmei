//! ConfigurationService 的 Rust 移植 (src/prog/config/ConfigurationService.java) — 一比一翻译。
//!
//! Main configuration Service implementing ConfigProvider.
//! Handles loading, saving, and accessing application configuration.
//!
//! Java 内部类 GenericOverlaySettingsImpl / HUDSettingsImpl → 独立 struct
//! (任务裁决; 波11 三分至 overlay_settings.rs / hud_settings.rs 子文件);
//! 子类对父类的 extends → 组合 + 委托。
//! Application 静态字段 (loadAppCheck 写入面 / OverlaySettings 读取面) 与
//! Controller 轮询间隔字段均属未翻译类 — 以 config/app_state.rs 的**消费面依赖桩**
//! 顶住 (config_manager.rs 的 UIStateStorage 桩先例), 注入式持有, 不造全局。
//! PORT(重构波1): UIStateBus 路由总线经构造器注入 `Option<Arc<UIStateBus>>`
//! (Java `UIStateBus.getInstance()` 全局单例读法的依赖注入式收敛); 本服务
//! 原"未路由裸 EventBus + 桩 UiStateEvent"形态已退役, 发布统一走
//! ui_state_bus.rs 的路由总线与三参 publish。RESET_REQUEST 事件链由上层
//! 直接调用 reset_all_layout_defaults() 顶替 (见 init_config 注释)。

mod hud_settings;
mod overlay_settings;

pub use hud_settings::HUDSettingsImpl;
pub use overlay_settings::GenericOverlaySettingsImpl;

use std::sync::{Arc, RwLock};

use crate::base::bus::ui_state_bus::UIStateBus;
use crate::base::event::ui_state_events;
use crate::base::java_compat::{java_parse_boolean, java_trim};
use crate::base::logger;
use crate::base::ports::bkp_port;
use crate::config::config_api::{ConfigProvider, HUDSettings, OverlaySettings};
use crate::config::config_loader::{
    self, config_value_to_string, ConfigValue, GroupConfig, RowConfig,
};
use crate::config::config_manager;
use crate::lang::Lang;
use crate::ui_support::color::parse_color;

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
/// `private List<GroupConfig> layoutConfigs` (可 null) →
/// `RwLock<Option<Vec<GroupConfig>>>`; 字段共享给 OverlaySettings 视图
/// (Java 内部类隐式外部引用) → `Arc<ServiceInner>`。
/// setConfig/saveWindowPosition 等写方法在 Java 以非同步实例方法触达
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
    /// 参数为 UIStateBus 注入 (Java 读全局单例; §2.9 禁再造全局)。
    // Java 保真 — Arc<ServiceInner> 复刻 Java this 引用的跨方法共享,
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
    /// Rust 侧配置 !Send 不能进渲染线程, 组装层启动/WYSIWYG 色变时取快照
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
    /// 取 gc.x/y (ConfigurationService 读的同一组字段)。Rust host 在
    /// 渲染线程不碰 !Send 配置树 — 组装层启动时经此取快照 (vm-app
    /// ChannelPositionStore); 返回归一化 (0..1) 坐标, 无同题分组 = None
    /// (host 居中兜底, 对齐 Java gc=null 的 center 分支)。
    pub fn group_position(&self, section: &str) -> Option<(f64, f64)> {
        let configs = self.inner.layout_configs.read().expect(LC_LOCK_MSG);
        let list = configs.as_ref()?;
        group_index_by_title(list, section, true).map(|i| (list[i].x, list[i].y))
    }

    /// 组装层位置桥 (归一化直写): 与视图 save_window_position 同源
    /// (ConfigurationService — 命中首个同题分组写回 + saveLayoutConfig,
    /// 未命中只 warn), 差异仅在参数已是归一化坐标 (host 拖拽存档即归一化,
    /// 免像素/比例往返); 视图版带屏幕尺寸守卫, 本版无该守卫 (归一化无涉屏幕)。
    pub fn save_group_position(&self, section: &str, nx: f64, ny: f64) -> bool {
        let mut hit = false;
        {
            let mut configs = self.inner.layout_configs.write().expect(LC_LOCK_MSG);
            if let Some(list) = configs.as_mut() {
                // 命中首个同题分组写回 (Java 原状)
                if let Some(i) = group_index_by_title(list, section, true) {
                    list[i].x = nx;
                    list[i].y = ny;
                    hit = true;
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
    /// - `sourcePath`: Path to the config file to import
    /// 返回: true if import was successful
    ///
    /// Java: `public boolean importConfig(String sourcePath)`
    pub fn import_config(&self, source_path: &str) -> bool {
        let success = config_manager::import_config(source_path);
        if success {
            // Reload configuration
            let reloaded = config_loader::load_config(config_manager::get_user_config_path());
            *self.inner.layout_configs.write().expect(LC_LOCK_MSG) = Some(reloaded);
            // Notify all subscribers about the change
            self.inner
                .publish_config_changed(ui_state_events::ACTION_RESET_COMPLETED);
        }
        success
    }

    /// Resets configuration to factory defaults.
    ///
    /// 返回: true if reset was successful
    ///
    /// Java: `public boolean resetToFactory()`
    pub fn reset_to_factory(&self) -> bool {
        let success = config_manager::reset_to_factory();
        if success {
            // Reload configuration
            let reloaded = config_loader::load_config(config_manager::get_user_config_path());
            *self.inner.layout_configs.write().expect(LC_LOCK_MSG) = Some(reloaded);
            // Notify all subscribers about the change
            self.inner
                .publish_config_changed(ui_state_events::ACTION_RESET_COMPLETED);
        }
        success
    }

    /// Java: `public List<GroupConfig> getLayoutConfigs()`
    /// Java 返回活 List 引用 (可 null) — RwLock 内存储无法经 &self 长期
    /// 借出, 返回快照副本 (Option 对应 null 形态); 当前无跨期持有消费方。
    pub fn get_layout_configs(&self) -> Option<Vec<GroupConfig>> {
        self.inner.layout_configs.read().expect(LC_LOCK_MSG).clone()
    }

    /// 根据 groupTitle 查找最新的 GroupConfig
    /// 用于 UI 组件在 rebuild 时获取最新配置，解决导入配置后引用陈旧的问题
    ///
    /// - `groupTitle`: 要查找的 GroupConfig 的标题
    /// 返回: 找到的 GroupConfig，如果未找到则返回 null
    ///
    /// Java: `public GroupConfig findGroupByTitle(String groupTitle)`
    /// Java groupTitle 可 null (折叠为不可达, &str 无 null); 大小写敏感
    /// equals 与下方视图的 equalsIgnoreCase 是两条不同查找 (Java 原状)。
    pub fn find_group_by_title(&self, group_title: &str) -> Option<GroupConfig> {
        let configs = self.inner.layout_configs.read().expect(LC_LOCK_MSG);
        configs.as_ref().and_then(|list| {
            group_index_by_title(list, group_title, false).map(|i| list[i].clone())
        })
    }

    /// Java: `public void loadAppCheck(Controller c)` — 解析并应用配置到应用与
    /// Controller 状态 (替代 Controller.loadFromConfig())。
    /// Controller 未翻译 — 6 个轮询间隔字段以消费面桩 `ControllerIntervals`
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
        // 波21: Java long×float 提升窄化的仪式性 f32 退役 (值域 <2^24 精确, 行为不变)
        c.engine_info_interval_ms = (service_loop_interval_ms as f64 * 2.0) as i64;
        c.flight_info_interval_ms = (service_loop_interval_ms as f64 * 1.5) as i64;
        c.altitude_interval_ms = (service_loop_interval_ms as f64 * 1.5) as i64;
        c.gear_flaps_interval_ms = (service_loop_interval_ms as f64 * 2.0) as i64;
        c.control_input_interval_ms = service_loop_interval_ms;
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
                    // 备份端口统一走 bkp_port 饱和策略 (A3 收敛); 越域主端口在
                    // 下方 InetSocketAddress::new 的域检查先行 panic, as 收窄无新差异
                    app.app_port_bkp = i32::from(bkp_port(port as u16));
                    // Assuming httpIp is still from Lang or static 127.0.0.1
                    let mut ip = "127.0.0.1".to_string();
                    // Rust 无全局 Lang 状态 — init_lang() 静态表快照现取 (blkx 先例)
                    let lang_ip = Lang::init_lang().http_ip;
                    if !lang_ip.is_empty() {
                        ip = lang_ip.to_string();
                    }
                    app.request_dest = Some(InetSocketAddress::new(&ip, app.app_port));
                    app.request_dest_bkp = Some(InetSocketAddress::new(&ip, app.app_port_bkp));
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

        // C 类接线 (AWT GraphicsEnvironment 注册 fonts/ 目录字体并解析
        // defaultFont / WebLaF 全局字体注入), vm-app/vm-ui 波次处理;
        // 落地前 ApplicationState.default_font 维持 None。
        // TODO(port): init_font + update_weblaf_fonts 接线 (get_font_name 的
        // defaultFont 回退分支届时生效)。
    }

    // --- Helpers ---

    /// Java: `public void saveConfig()` — 已空实现 (不再使用 config.properties)
    pub fn save_config(&self) {
        // No longer using config.properties, all settings in ui_layout.cfg
    }

    /// Java: `public Color getColorConfig(String key)`
    /// java.awt.Color → [u8;4] RGBA (POC 先例; #RRGGBBAA 字节序经
    /// ColorHelper.parseColor 保真, 见 ui_support::color)
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
        // RwLock 下引用不可跨锁存活, 以 (组下标, 行路径) 定位 (单线程内等价)
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
                                config_value_to_string(
                                    row.default_value
                                        .as_ref()
                                        .expect("Phase 1 已保证 defaultValue 非 null")
                                )
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
        // 读锁内做 IO — **不变量**: config_loader::save_config 不得发事件
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
    /// 总线注入式 — 未注入时为无操作 (Java 全局单例恒存在)
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
    /// Java `sectionName.equalsIgnoreCase(gc.title)` (走 [`group_index_by_title`])。
    /// Java 逐字符简单大小写折叠 — 标题域 ASCII/CJK 下
    /// eq_ignore_ascii_case 等价 (CJK 无大小写)
    fn find_group_ignore_case(&self, section_name: &str) -> Option<GroupConfig> {
        let configs = self.layout_configs.read().expect(LC_LOCK_MSG);
        configs.as_ref().and_then(|list| {
            group_index_by_title(list, section_name, true).map(|i| list[i].clone())
        })
    }

    /// 就地写回分组 x/y (saveWindowPosition 的写面), 返回新值供日志
    fn set_group_position_ignore_case(
        &self,
        section_name: &str,
        x: f64,
        y: f64,
    ) -> Option<(f64, f64)> {
        let mut configs = self.layout_configs.write().expect(LC_LOCK_MSG);
        let list = configs.as_mut()?;
        let i = group_index_by_title(list, section_name, true)?;
        list[i].x = x;
        list[i].y = y;
        Some((list[i].x, list[i].y))
    }

    fn screen_size(&self) -> (i32, i32) {
        let app = self.app.read().expect(APP_LOCK_MSG);
        (app.screen_width, app.screen_height)
    }

    fn app_default_numfont_name(&self) -> String {
        self.app
            .read()
            .expect(APP_LOCK_MSG)
            .default_numfont_name
            .clone()
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
/// 借用版自由函数 — 输入切片生命周期透传 (调用方持快照或锁守卫)
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
/// 事件 publish 由调用方放锁后补发 (events 收集, §2.8)
fn update_rows_recursive(rows: &mut [RowConfig], key: &str, value: &str, events: &mut Vec<String>) {
    for row in rows.iter_mut() {
        if row.property.as_deref() == Some(key) || (row.property.is_none() && key == row.label) {
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
/// Java 收集对象引用; Rust 收集 (组下标, 行路径) — 单线程内两者等价
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
            let equal = r
                .value
                .as_ref()
                .is_some_and(|v| config_value_java_equals(dv, v));
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

/// "按标题查组"唯一底层: 返回 list 中首个命中分组的索引。
/// `ignore_case` 对应 Java 两条查找原状 (equals vs equalsIgnoreCase):
/// group_position / save_group_position / find_group_ignore_case /
/// set_group_position_ignore_case 走忽略大小写, find_group_by_title 走精确等值。
fn group_index_by_title(list: &[GroupConfig], title: &str, ignore_case: bool) -> Option<usize> {
    list.iter().position(|gc| {
        if ignore_case {
            title.eq_ignore_ascii_case(&gc.title)
        } else {
            title == gc.title
        }
    })
}

// =====================================================================
// Java 语义本地辅助 (私有) — 通用件已收敛到 base::java_compat
// =====================================================================

/// Java `Double.parseDouble(String)` 复刻 (消费域收敛版):
/// - JLS/FloatingDecimal: 先 `String.trim()` (<= U+0020), 允许尾缀 d/D/f/F;
/// - "NaN"/"Infinity" (±) 与 Rust from_str 的 "nan"/"inf"(任意大小写) 集合有差
///   — cfg 值域不可达;
/// 数值解析 (波22: 域收窄版复刻退役, std parse; cfg 值域普通十进制)。
fn parse_double(s: &str) -> Result<f64, ()> {
    java_trim(s).parse::<f64>().map_err(|_| ())
}

// =====================================================================
// prog.util.ColorHelper 的消费面 (解析本体已收敛到 ui_support::color)
// =====================================================================

/// java.awt.Color.WHITE — ColorHelper.parseColor 的默认色 (loadAppCheck 调用点)
const COLOR_WHITE: [u8; 4] = [255, 255, 255, 255];

// =====================================================================
// Tests — Java 侧无对应单测 (ConfigurationService 为手动验证), 本组为移植
// 边界钉子 + 真实 ui_layout.cfg 实例断言。
// =====================================================================
#[cfg(test)]
mod tests;
