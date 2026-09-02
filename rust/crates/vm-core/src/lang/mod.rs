//! lang 域 (波8 自根留升域; lang.rs 并入 mod.rs 消除 module_inception —
//! `lang::lang::Lang` 双层同名嵌套退役, `crate::lang::Lang` 为唯一真相路径)。
//! `prog.i18n.Lang` 的 Rust 移植 (src/prog/i18n/Lang.java)。
//!
//! Java 形态: ~300 个 `public static String` 字段 + `initLang()` 经
//! `prog.config.Config("./lang/cur.properties")` (java.util.Properties, UTF-8) 逐键填充,
//! `updateLanguage()` 对缺失/空值键统一回退为 `""`。
//!
//! PORT: 按迁移任务裁决, Properties 运行时加载固化为静态表 [`crate::lang::table`]
//! (键值 = Java 加载后的实际值, Java 8 oracle 实测生成); 字段类型为 `&'static str`。
//! PORT: Java 静态可变全局 → Rust struct 实例 (禁 `static mut`); 调用方持有 `Lang`,
//! 后续若需全局单例由 AppState 收口 (LIFETIMES 静态可变全局)。
//! PORT: 类依赖 `prog.Application` 仅存在于注释, 无实际耦合, 不引入。


pub mod table;


/// `prog.config.Config` 的最小只读替身 (仅 `getValue` 语义)。
/// PORT: Java 原类在构造时读文件; 本移植数据来自静态表, 待 `crate::config::Config`
/// 移植落地后可收敛复用。
#[derive(Default)]
#[derive(Clone)]
pub struct Config;

impl Config {
    /// 对应 Java: `new prog.config.Config(filePath)`
    /// PORT: 不再读文件, 参数仅保留调用点原貌
    pub fn new(_file_path: &str) -> Config {
        Config
    }

    /// 对应 Java: `public String getValue(String key)` — 键存在返回值 (可为空串), 缺失返回 `""`
    pub fn get_value(&self, key: &str) -> &'static str {
        table::config_get_value(key)
    }
}

// PORT: Java 静态字段隐式初始化为 null (PORTING §2.10); Rust 无 null 字符串,
// derive(Default) 以 ""/None 占位 — init_lang() 全量覆写后与 Java 终态一致。
// 消费方只应使用 init_lang() 的产物 (Java 端同样在 Application 启动即调 initLang)。
#[derive(Default)]
#[derive(Clone)]
pub struct Lang {
    pub app_name: &'static str,
    pub app_tooltips: &'static str,

    pub close: &'static str,
    pub about: &'static str,
    pub aboutcontent: &'static str,
    pub aboutcontentsub1: &'static str,
    pub aboutcontentsub2: &'static str,
    pub failaddto_tray: &'static str,
    pub http_header: &'static str,
    pub http_ip: &'static str,
    pub http_port: &'static str,
    pub systemerror: &'static str,
    // MainForm
    pub m_cancel: &'static str,
    pub m_start: &'static str,
    pub m_display_preview: &'static str,
    pub m_close_preview: &'static str,
    pub m_move_panel: &'static str,
    pub m_simple_mode: &'static str,
    pub m_detailed_mode: &'static str,
    pub m_basic_settings: &'static str,
    pub m_display_overlay: &'static str,
    pub m_hotkey_toggle: &'static str,
    pub m_wait_hotkey: &'static str,

    pub m_p1_num_color: &'static str,
    pub m_p1_num_color_blank: &'static str,
    pub m_p1_label_color: &'static str,
    pub m_p1_label_color_blank: &'static str,
    pub m_p1_unit_color: &'static str,
    pub m_p1_unit_color_blank: &'static str,
    pub m_p1_warn_color: &'static str,
    pub m_p1_warn_color_blank: &'static str,
    pub m_p1_shade_color: &'static str,
    pub m_p1_shade_color_blank: &'static str,

    pub m_p1_temp_notification: &'static str,
    pub m_p1_temp_notification_blank: &'static str,
    pub m_p1_voice_warning: &'static str,
    pub m_p1_voice_warning_blank: &'static str,
    pub m_p1draw_font_shape: &'static str,
    pub m_p1draw_font_shape_blank: &'static str,
    pub m_p1_aa_enable: &'static str,
    pub m_p1_aa_enable_blank: &'static str,
    pub m_p1_global_number_font: &'static str,
    pub m_p1_global_number_font_blank: &'static str,
    pub m_p1_interval: &'static str,

    // 新增音量
    pub m_p1voice_volume: &'static str,
    pub m_p1voice_volume_blank: &'static str,
    // 新增是否关闭状态条
    pub m_p1_status_bar: &'static str,
    pub m_p1_status_bar_blank: &'static str,

    pub m_p2_engine_panel: &'static str,
    pub m_p2_engine_panel_blank: &'static str,
    pub m_p2_engine_glass_edge: &'static str,
    pub m_p2_engine_glass_edge_blank: &'static str,
    pub m_p2_panel_font: &'static str,
    pub m_p2_font_adjust: &'static str,

    pub m_p2_engine_blank: &'static str,
    pub m_p2ei_horse_power: &'static str,
    pub m_p2ei_horse_power_blank: &'static str,
    pub m_p2ei_thrust: &'static str,
    pub m_p2ei_thrust_blank: &'static str,
    pub m_p2ei_rpm: &'static str,
    pub m_p2ei_rpm_blank: &'static str,
    pub m_p2ei_prop_pitch: &'static str,
    pub m_p2ei_prop_pitch_blank: &'static str,
    pub m_p2ei_eff_eta: &'static str,
    pub m_p2ei_eff_eta_blank: &'static str,
    pub m_p2ei_eff_hp: &'static str,
    pub m_p2ei_eff_hp_blank: &'static str,
    pub m_p2ei_pressure: &'static str,
    pub m_p2ei_pressure_blank: &'static str,
    pub m_p2ei_power_percent: &'static str,
    pub m_p2ei_power_percent_blank: &'static str,
    pub m_p2ei_fuel_kg: &'static str,
    pub m_p2ei_fuel_kg_blank: &'static str,
    pub m_p2ei_fuel_time: &'static str,
    pub m_p2ei_fuel_time_blank: &'static str,
    pub m_p2ei_wep_kg: &'static str,
    pub m_p2ei_wep_kg_blank: &'static str,
    pub m_p2ei_wep_time: &'static str,
    pub m_p2ei_wep_time_blank: &'static str,
    pub m_p2ei_temp: &'static str,
    pub m_p2ei_temp_blank: &'static str,
    pub m_p2ei_oil_temp: &'static str,
    pub m_p2ei_oil_temp_blank: &'static str,
    pub m_p2ei_heat_tolerance: &'static str,
    pub m_p2ei_heat_tolerance_blank: &'static str,
    pub m_p2ei_eng_response: &'static str,
    pub m_p2ei_eng_response_blank: &'static str,

    pub m_p3_crosshair: &'static str,
    pub m_p3_crosshair_blank: &'static str,
    pub m_p3_crosshair_display: &'static str,
    pub m_p3_crosshair_display_blank: &'static str,
    pub m_p3_text: &'static str,
    pub m_p3_text_blank: &'static str,
    pub m_p3_flap_angle_bar: &'static str,
    pub m_p3_flap_angle_bar_blank: &'static str,
    pub m_p3_crosshair_texture: &'static str,
    pub m_p3_crosshair_texture_blank: &'static str,
    pub m_p3_choose_texture: &'static str,
    pub m_p3_choose_texture_blank: &'static str,
    pub m_p3_crosshair_size: &'static str,
    pub m_p3_mono_font: &'static str,
    pub m_p3_mono_font_blank: &'static str,

    pub m_p4_flight_info_panel: &'static str,
    pub m_p4attitude_indicator_panel: &'static str,
    pub m_p4attitude_indicator_panel_blank: &'static str,
    pub m_p4_fm_panel: &'static str,
    pub m_p4_fm_panel_blank: &'static str,

    pub m_p4fi_ias: &'static str,
    pub m_p4fi_ias_blank: &'static str,
    pub m_p4fi_tas: &'static str,
    pub m_p4fi_tas_blank: &'static str,
    pub m_p4fi_mach: &'static str,
    pub m_p4fi_mach_blank: &'static str,
    pub m_p4fi_compass: &'static str,
    pub m_p4fi_compass_blank: &'static str,
    pub m_p4fi_height: &'static str,
    pub m_p4fi_height_blank: &'static str,
    pub m_p4fi_vario: &'static str,
    pub m_p4fi_vario_blank: &'static str,
    pub m_p4fi_sep: &'static str,
    pub m_p4fi_sep_blank: &'static str,
    pub m_p4fi_acc: &'static str,
    pub m_p4fi_acc_blank: &'static str,
    pub m_p4fi_wx: &'static str,
    pub m_p4fi_wx_blank: &'static str,
    pub m_p4fi_ny: &'static str,
    pub m_p4fi_ny_blank: &'static str,
    pub m_p4fi_turn: &'static str,
    pub m_p4fi_turn_blank: &'static str,
    pub m_p4fi_turn_radius: &'static str,
    pub m_p4fi_turn_radius_blank: &'static str,
    pub m_p4fi_ao_a: &'static str,
    pub m_p4fi_ao_a_blank: &'static str,
    pub m_p4fi_ao_s: &'static str,
    pub m_p4fi_ao_s_blank: &'static str,
    pub m_p4fi_wing_sweep: &'static str,
    pub m_p4fi_wing_sweep_blank: &'static str,
    pub m_p4fi_radio_alt: &'static str,
    pub m_p4fi_radio_alt_blank: &'static str,

    pub m_p4_flight_info_blank: &'static str,
    pub m_p4_flight_info_glass_edge: &'static str,
    pub m_p4_flight_info_glass_edge_blank: &'static str,
    pub m_p4_panel_font: &'static str,
    pub m_p4_font_adjust: &'static str,
    pub m_p4_column_adjust: &'static str,

    pub m_p5_logging_and_charting: &'static str,
    pub m_p5_logging_and_charting_blank: &'static str,
    pub m_p5_information: &'static str,
    pub m_p5_information_blank: &'static str,
    pub m_p5_fm_choose: &'static str,
    pub m_p5_fm_choose_blank: &'static str,
    pub m_p5_fm_display_key: &'static str,
    pub m_p5_fm_display_key_tip: &'static str,
    pub m_p5_fm_print_enable: &'static str,
    pub m_p5_fm_print_enable_blank: &'static str,

    pub m_p6_axis_panel: &'static str,
    pub m_p6_axis_panel_blank: &'static str,
    pub m_p6_axis_edge: &'static str,
    pub m_p6_axis_edge_blank: &'static str,
    pub m_p6_gear_and_flaps: &'static str,
    pub m_p6_gear_and_flaps_edge: &'static str,

    pub m_p6_gear_and_flaps_edge_blank: &'static str,
    pub m_p6engine_control: &'static str,
    pub m_p6engine_control_blank: &'static str,

    pub m_p6ec_throttle: &'static str,
    pub m_p6ec_throttle_blank: &'static str,
    pub m_p6ec_pitch: &'static str,
    pub m_p6ec_pitch_blank: &'static str,
    pub m_p6ec_mixture: &'static str,
    pub m_p6ec_mixture_blank: &'static str,
    pub m_p6ec_radiator: &'static str,
    pub m_p6ec_radiator_blank: &'static str,
    pub m_p6ec_compressor: &'static str,
    pub m_p6ec_compressor_blank: &'static str,
    pub m_p6ec_l_fuel: &'static str,
    pub m_p6ec_l_fuel_blank: &'static str,

    pub m_flight_info: &'static str,
    pub m_engine_info: &'static str,
    pub m_control_info: &'static str,
    pub m_logging_and_analysis: &'static str,
    pub m_crosshair: &'static str,
    pub m_advanced_option: &'static str,

    // OtherService
    pub o_skey_word1: &'static str,
    pub o_skey_word2: &'static str,

    // DrawFrame
    pub d_fprev: &'static str,
    pub d_fnext: &'static str,
    pub d_f_title1: &'static str,
    pub d_f_title1_x: &'static str,
    pub d_f_title1_y: &'static str,
    pub d_f_title2: &'static str,
    pub d_f_title2_x: &'static str,
    pub d_f_title2_y: &'static str,
    pub d_f_title3: &'static str,
    pub d_f_title3_x: &'static str,
    pub d_f_title3_y: &'static str,
    pub d_f_title4: &'static str,
    pub d_f_title4_x: &'static str,
    pub d_f_title4_y: &'static str,
    pub d_f_title5: &'static str,
    pub d_f_title5_x: &'static str,
    pub d_f_title5_y: &'static str,
    pub d_f_title_hz: &'static str,

    pub m_reset_confirm_title: &'static str,
    pub m_reset_confirm_content: &'static str,

    // Config Manager i18n
    pub m_config_error_title: &'static str,
    pub m_config_error_content: &'static str,
    pub m_config_merged_title: &'static str,
    pub m_merge_added_panels: &'static str,
    pub m_merge_added_items: &'static str,
    pub m_merge_updated_items: &'static str,
    pub m_import_config_title: &'static str,
    pub m_import_confirm_title: &'static str,
    pub m_import_confirm_content: &'static str,
    pub m_import_success_title: &'static str,
    pub m_import_success_content: &'static str,
    pub m_import_fail_title: &'static str,
    pub m_import_fail_content: &'static str,
    pub m_factory_reset_confirm_title: &'static str,
    pub m_factory_reset_confirm_content: &'static str,
    pub m_factory_reset_success_title: &'static str,
    pub m_factory_reset_success_content: &'static str,
    pub m_factory_reset_fail_title: &'static str,
    pub m_factory_reset_fail_content: &'static str,

    // Config Import Dialog - 拖放导入
    pub m_import_drop_zone_title: &'static str,
    pub m_import_drop_zone_subtitle: &'static str,
    pub m_import_drop_zone_format: &'static str,
    pub m_import_drop_zone_release: &'static str,
    pub m_import_drop_zone_invalid: &'static str,
    pub m_import_file_selected: &'static str,
    pub m_import_file_none: &'static str,
    pub m_import_button_import: &'static str,

    // Version Update Dialog
    pub m_update_available_title: &'static str,
    pub m_update_available_content: &'static str,
    pub m_update_available_link_text: &'static str,

    // EngineInfo

    pub e_throttle: &'static str,
    pub e_proppitch: &'static str,
    pub e_mixture: &'static str,
    pub e_compressor: &'static str,
    pub e_radiator: &'static str,
    pub e_thurst_p: &'static str,
    pub e_fuel_per: &'static str,
    pub e_pitch_deg: &'static str,
    pub e_magneto: &'static str,
    pub e_type: &'static str,
    pub e_power: &'static str,
    pub e_thurst: &'static str,
    pub e_eff_power: &'static str,
    pub e_power_percent: &'static str,
    pub e_fuel: &'static str,
    pub e_fuel_p: &'static str,
    pub e_fuel_prs: &'static str,
    pub e_rpm: &'static str,
    pub e_temp: &'static str,
    pub e_eff: &'static str,
    pub e_fueltime: &'static str,
    pub e_weptime: &'static str,
    pub e_wep: &'static str,
    pub e_atm: &'static str,
    pub e_oil: &'static str,
    pub e_overheat: &'static str,
    pub e_eng_res: &'static str,
    pub e_title: &'static str,

    // FlightInfo
    pub f_ias: &'static str,
    pub f_tas: &'static str,
    pub f_compass: &'static str,
    pub f_mach: &'static str,
    pub f_wx: &'static str,
    pub f_tr: &'static str,
    pub f_t_rr: &'static str,
    pub f_alt: &'static str,
    pub f_vario: &'static str,
    pub f_acc: &'static str,
    pub f_sep: &'static str,
    pub f_ao_a: &'static str,
    pub f_ao_s: &'static str,
    pub f_ws: &'static str,
    pub f_ra: &'static str,
    pub f_gl: &'static str,
    pub f_title: &'static str,

    // GearAndFlaps
    pub g_flaps: &'static str,
    pub g_title: &'static str,
    pub g_gear: &'static str,
    pub g_gear_down: &'static str,
    pub g_brake: &'static str,

    // StatusBar
    pub s_title: &'static str,
    pub s_wait: &'static str,
    pub s_enter: &'static str,
    pub s_check: &'static str,

    // StickValue
    pub v_aileron: &'static str,
    pub v_elevator: &'static str,
    pub v_rudder: &'static str,
    pub v_vario_w: &'static str,
    pub v_title: &'static str,

    // Controller
    pub c_startlog: &'static str,
    pub c_savelog: &'static str,
    pub c_plsopen: &'static str,
    pub c_openpad: &'static str,
    pub c_enginedmg: Option<&'static str>, // PORT: Java 中从未在 initLang() 赋值, 保持 null
    pub c_warn1min: Option<&'static str>, // PORT: Java 中从未在 initLang() 赋值, 保持 null
    pub c_eng_bomb: Option<&'static str>, // PORT: Java 中从未在 initLang() 赋值, 保持 null

    // flightlog
    pub l1: &'static str,
    pub l2: &'static str,
    pub l3: &'static str,
    pub l4: &'static str,
    pub l5: &'static str,
    pub l6: &'static str,
    pub l7: &'static str,
    pub l8: &'static str,
    pub l9: &'static str,
    pub l10: &'static str,
    pub l11: &'static str,
    pub l12: &'static str,
    pub l13: &'static str,
    pub l14: &'static str,
    pub l15: &'static str,
    pub l16: &'static str,
    pub l17: &'static str,
    pub l18: &'static str,
    pub l19: &'static str,
    pub l20: &'static str,
    pub l21: &'static str,
    pub l22: &'static str,
    pub l23: &'static str,
    pub l24: &'static str,
    pub l25: &'static str,
    pub l26: &'static str,
    pub l27: &'static str,
    pub l28: &'static str,
    pub l29: &'static str,
    pub l30: &'static str,
    pub l31: &'static str,
    pub lfail_create: &'static str,
    pub lfail_write: &'static str,

    // FlightAnalyzer
    pub f_a1: &'static str,
    pub f_a2: &'static str,
    pub f_a3: &'static str,
    pub f_a4: &'static str,

    pub f_a_roll1: &'static str,
    pub f_a_roll2: &'static str,
    pub f_a_roll3: &'static str,

    pub f_a_turn1: &'static str,
    pub f_a_turn2: &'static str,
    pub f_a_turn3: &'static str,
    pub f_a_turn4: &'static str,

    pub noblkx: &'static str,
    /** FM 缺失/损坏的右下角 toast 提示 (检视需求: 告知用户该飞机无 FM 数据) */
    pub fm_missing_toast: &'static str,
    pub fm_corrupt_toast: &'static str,
    pub b_fm_parts: &'static str,
    pub b_cd_min: &'static str,
    pub b_cl0: &'static str,
    pub b_ao_a_crit: &'static str,
    pub b_ao_a_crit_cl: &'static str,

    // 还未加入
    pub b_fm_version: &'static str,
    pub b_weight: &'static str,
    pub b_crit_speed: &'static str,
    pub b_allow_load_factor: &'static str,
    pub b_average_heat_recovery: &'static str,
    pub b_nitro: &'static str,
    pub b_flap_restrict: &'static str,
    pub b_eff_speed_and_power_loss: &'static str,
    pub b_inertia: &'static str,

    pub b_max_lift_load350: &'static str,

    pub b_lift: &'static str,
    pub b_drag: &'static str,

    pub lanuage_config: Config, // PORT: Java 类型 prog.config.Config, 拼写错误 lanuage 原样保留

}

impl Lang {
    /// 对应 Java: `public static String updateLanguage(String key, String dft)`
    /// 取语言配置值; 空值 (缺失键或空串值) 时返回 `""`。
    /// PORT: Java 在空值分支把 dft 覆写为 "" 再返回 —— 传入的默认值实际永不生效
    /// (原行为保真保留); 形参因永不读取改名 `_dft` 消未用告警。
    pub fn update_language(lanuage_config: &Config, key: &str, _dft: &str) -> &'static str {
        let v = lanuage_config.get_value(key);
        if !v.is_empty() {
            // Application.debugPrint(v);
            v
        } else {
            // Application.debugPrint(key);
            ""
        }
    }

    /// 对应 Java: `public static void initLang()` — 建表并挨个更新全部字段
    // PORT: Java 保真 — initLang 逐行 `lang.f = updateLanguage(cfg, "k", lang.f)`
    // 直译 (读旧值写新值交错, 无法也不应改 struct 字面量)
    #[allow(clippy::field_reassign_with_default)]
    pub fn init_lang() -> Lang {
        // 重构波6: 纯函数 (静态表数据源, 无运行时 cfg) — OnceLock 缓存首建,
        // 调用点 (全仓 26 处) 零改动; clone = 365 个 &str 位拷贝 (原每次重建
        // + 逐字段 update_language)。logger 的 OnceLock 不可变豁免同款形态
        static LANG: std::sync::OnceLock<Lang> = std::sync::OnceLock::new();
        LANG.get_or_init(Self::build_lang).clone()
    }

    fn build_lang() -> Lang {
        let mut lang = Lang::default();

        // PORT: Java 运行时读取该文件 (prog.config.Config + java.util.Properties UTF-8);
        // 本移植固化为静态表 (见 table.rs), Config 为 getValue 语义的只读替身
        lang.lanuage_config = Config::new("./lang/cur.properties");
        let cfg = &lang.lanuage_config;

        // 挨个更新
        lang.app_name = Lang::update_language(cfg, "appName", lang.app_name);
        lang.app_tooltips = Lang::update_language(cfg, "appTooltips", lang.app_tooltips);
        lang.close = Lang::update_language(cfg, "close", lang.close);
        lang.about = Lang::update_language(cfg, "about", lang.about);
        lang.aboutcontent = Lang::update_language(cfg, "aboutcontent", lang.aboutcontent);
        lang.aboutcontentsub1 = Lang::update_language(cfg, "aboutcontentsub1", lang.aboutcontentsub1);
        lang.aboutcontentsub2 = Lang::update_language(cfg, "aboutcontentsub2", lang.aboutcontentsub2);
        lang.failaddto_tray = Lang::update_language(cfg, "failaddtoTray", lang.failaddto_tray);
        lang.http_header = Lang::update_language(cfg, "httpHeader", lang.http_header);
        lang.http_ip = Lang::update_language(cfg, "httpIp", lang.http_ip);
        lang.http_port = Lang::update_language(cfg, "httpPort", lang.http_port);
        // Application.debugPrint(httpHeader);
        lang.systemerror = Lang::update_language(cfg, "Systemerror", lang.systemerror);
        lang.m_cancel = Lang::update_language(cfg, "mCancel", lang.m_cancel);
        lang.m_start = Lang::update_language(cfg, "mStart", lang.m_start);
        lang.m_display_preview = Lang::update_language(cfg, "mDisplayPreview", lang.m_display_preview);

        lang.m_reset_confirm_title = Lang::update_language(cfg, "mResetConfirmTitle", "确认重置");
        lang.m_reset_confirm_content = Lang::update_language(cfg, "mResetConfirmContent", "确定要重置所有配置项吗？\n此操作不可撤销。");

        // Config Manager i18n
        lang.m_config_error_title = Lang::update_language(cfg, "mConfigErrorTitle", "配置错误");
        lang.m_config_error_content = Lang::update_language(cfg, "mConfigErrorContent", "用户配置文件解析失败，将临时使用默认配置。\n请检查 ui_layout.user.cfg 文件是否损坏。");
        lang.m_config_merged_title = Lang::update_language(cfg, "mConfigMergedTitle", "配置已更新");
        lang.m_merge_added_panels = Lang::update_language(cfg, "mMergeAddedPanels", "新增面板:");
        lang.m_merge_added_items = Lang::update_language(cfg, "mMergeAddedItems", "新增配置项:");
        lang.m_merge_updated_items = Lang::update_language(cfg, "mMergeUpdatedItems", "更新配置项:");
        lang.m_import_config_title = Lang::update_language(cfg, "mImportConfigTitle", "选择配置文件");
        lang.m_import_confirm_title = Lang::update_language(cfg, "mImportConfirmTitle", "确认导入");
        lang.m_import_confirm_content = Lang::update_language(cfg, "mImportConfirmContent", "确定要导入此配置文件吗？\n当前配置将被备份到 ui_layout.user.cfg.bak");
        lang.m_import_success_title = Lang::update_language(cfg, "mImportSuccessTitle", "导入成功");
        lang.m_import_success_content = Lang::update_language(cfg, "mImportSuccessContent", "配置文件已成功导入，请重启程序以应用所有更改。");
        lang.m_import_fail_title = Lang::update_language(cfg, "mImportFailTitle", "导入失败");
        lang.m_import_fail_content = Lang::update_language(cfg, "mImportFailContent", "配置文件导入失败，请检查文件格式是否正确。");
        lang.m_factory_reset_confirm_title = Lang::update_language(cfg, "mFactoryResetConfirmTitle", "确认恢复出厂设置");
        lang.m_factory_reset_confirm_content = Lang::update_language(cfg, "mFactoryResetConfirmContent", "确定要恢复出厂设置吗？\n所有自定义配置将被清除，当前配置将被备份。");
        lang.m_factory_reset_success_title = Lang::update_language(cfg, "mFactoryResetSuccessTitle", "恢复成功");
        lang.m_factory_reset_success_content = Lang::update_language(cfg, "mFactoryResetSuccessContent", "配置已恢复为出厂设置，请重启程序以应用所有更改。");
        lang.m_factory_reset_fail_title = Lang::update_language(cfg, "mFactoryResetFailTitle", "恢复失败");
        lang.m_factory_reset_fail_content = Lang::update_language(cfg, "mFactoryResetFailContent", "恢复出厂设置失败，请检查模板文件是否存在。");

        // Config Import Dialog - 拖放导入
        lang.m_import_drop_zone_title = Lang::update_language(cfg, "mImportDropZoneTitle", "拖放配置文件到此处");
        lang.m_import_drop_zone_subtitle = Lang::update_language(cfg, "mImportDropZoneSubtitle", "或点击选择文件");
        lang.m_import_drop_zone_format = Lang::update_language(cfg, "mImportDropZoneFormat", "支持的格式: *.cfg, *.bak");
        lang.m_import_drop_zone_release = Lang::update_language(cfg, "mImportDropZoneRelease", "松开以导入");
        lang.m_import_drop_zone_invalid = Lang::update_language(cfg, "mImportDropZoneInvalid", "不支持的文件格式，请选择 .cfg 或 .bak 文件");
        lang.m_import_file_selected = Lang::update_language(cfg, "mImportFileSelected", "已选择: %s");
        lang.m_import_file_none = Lang::update_language(cfg, "mImportFileNone", "未选择文件");
        lang.m_import_button_import = Lang::update_language(cfg, "mImportButtonImport", "导入配置");

        lang.m_update_available_title = Lang::update_language(cfg, "mUpdateAvailableTitle", "发现新版本");
        lang.m_update_available_content = Lang::update_language(cfg, "mUpdateAvailableContent", "GitHub上已发布新版本: %s<br>当前版本: %s<br>请点击下方链接下载更新。");
        lang.m_update_available_link_text = Lang::update_language(cfg, "mUpdateAvailableLinkText", "前往下载页面");
        lang.m_close_preview = Lang::update_language(cfg, "mClosePreview", lang.m_close_preview);
        lang.m_move_panel = Lang::update_language(cfg, "mMovePanel", lang.m_move_panel);
        lang.m_simple_mode = Lang::update_language(cfg, "mSimpleMode", lang.m_simple_mode);
        lang.m_detailed_mode = Lang::update_language(cfg, "mDetailedMode", lang.m_detailed_mode);
        lang.m_basic_settings = Lang::update_language(cfg, "mBasicSettings", lang.m_basic_settings);
        lang.m_display_overlay = Lang::update_language(cfg, "mDisplayOverlay", lang.m_display_overlay);
        lang.m_hotkey_toggle = Lang::update_language(cfg, "mHotkeyToggle", lang.m_hotkey_toggle);
        lang.m_wait_hotkey = Lang::update_language(cfg, "mWaitHotkey", lang.m_wait_hotkey);

        lang.m_p1_temp_notification = Lang::update_language(cfg, "mP1TempNotification", lang.m_p1_temp_notification);
        lang.m_p1_temp_notification_blank = Lang::update_language(cfg, "mP1TempNotificationBlank", lang.m_p1_temp_notification_blank);
        lang.m_p1draw_font_shape = Lang::update_language(cfg, "mP1drawFontShape", lang.m_p1draw_font_shape);
        lang.m_p1draw_font_shape_blank = Lang::update_language(cfg, "mP1drawFontShapeBlank", lang.m_p1draw_font_shape_blank);
        lang.m_p1_aa_enable = Lang::update_language(cfg, "mP1AAEnable", lang.m_p1_aa_enable);
        lang.m_p1_aa_enable_blank = Lang::update_language(cfg, "mP1AAEnableBlank", lang.m_p1_aa_enable_blank);
        lang.m_p1_voice_warning = Lang::update_language(cfg, "mP1VoiceWarning", lang.m_p1_voice_warning);
        lang.m_p1_voice_warning_blank = Lang::update_language(cfg, "mP1VoiceWarningBlank", lang.m_p1_voice_warning_blank);
        lang.m_p1_global_number_font = Lang::update_language(cfg, "mP1GlobalNumberFont", lang.m_p1_global_number_font);
        lang.m_p1_global_number_font_blank = Lang::update_language(cfg, "mP1GlobalNumberFontBlank", lang.m_p1_global_number_font_blank);
        lang.m_p1_interval = Lang::update_language(cfg, "mP1Interval", lang.m_p1_interval);
        // 新增音量
        lang.m_p1voice_volume = Lang::update_language(cfg, "mP1voiceVolume", lang.m_p1voice_volume);
        lang.m_p1voice_volume_blank = Lang::update_language(cfg, "mP1voiceVolumeBlank", lang.m_p1voice_volume_blank);
        // 新增是否关闭状态条
        lang.m_p1_status_bar = Lang::update_language(cfg, "mP1StatusBar", lang.m_p1_status_bar);
        lang.m_p1_status_bar_blank = Lang::update_language(cfg, "mP1StatusBarBlank", lang.m_p1_status_bar_blank);

        lang.m_p1_num_color = Lang::update_language(cfg, "mP1NumColor", lang.m_p1_num_color);
        lang.m_p1_num_color_blank = Lang::update_language(cfg, "mP1NumColorBlank", lang.m_p1_num_color_blank);
        lang.m_p1_label_color = Lang::update_language(cfg, "mP1LabelColor", lang.m_p1_label_color);
        lang.m_p1_label_color_blank = Lang::update_language(cfg, "mP1LabelColorBlank", lang.m_p1_label_color_blank);
        lang.m_p1_unit_color = Lang::update_language(cfg, "mP1UnitColor", lang.m_p1_unit_color);
        lang.m_p1_unit_color_blank = Lang::update_language(cfg, "mP1UnitColorBlank", lang.m_p1_unit_color_blank);
        lang.m_p1_warn_color = Lang::update_language(cfg, "mP1WarnColor", lang.m_p1_warn_color);
        lang.m_p1_warn_color_blank = Lang::update_language(cfg, "mP1WarnColorBlank", lang.m_p1_warn_color_blank);
        lang.m_p1_shade_color = Lang::update_language(cfg, "mP1ShadeColor", lang.m_p1_shade_color);
        lang.m_p1_shade_color_blank = Lang::update_language(cfg, "mP1ShadeColorBlank", lang.m_p1_shade_color_blank);

        lang.m_p2_engine_panel = Lang::update_language(cfg, "mP2EnginePanel", lang.m_p2_engine_panel);
        lang.m_p2_engine_panel_blank = Lang::update_language(cfg, "mP2EnginePanelBlank", lang.m_p2_engine_panel_blank);
        lang.m_p2_engine_glass_edge = Lang::update_language(cfg, "mP2EngineGlassEdge", lang.m_p2_engine_glass_edge);
        lang.m_p2_engine_glass_edge_blank = Lang::update_language(cfg, "mP2EngineGlassEdgeBlank", lang.m_p2_engine_glass_edge_blank);
        lang.m_p2_panel_font = Lang::update_language(cfg, "mP2PanelFont", lang.m_p2_panel_font);
        lang.m_p2_font_adjust = Lang::update_language(cfg, "mP2FontAdjust", lang.m_p2_font_adjust);

        lang.m_p2_engine_blank = Lang::update_language(cfg, "mP2EngineBlank", lang.m_p2_engine_blank);
        lang.m_p2ei_horse_power = Lang::update_language(cfg, "mP2eiHorsePower", lang.m_p2ei_horse_power);
        lang.m_p2ei_horse_power_blank = Lang::update_language(cfg, "mP2eiHorsePowerBlank", lang.m_p2ei_horse_power_blank);
        lang.m_p2ei_thrust = Lang::update_language(cfg, "mP2eiThrust", lang.m_p2ei_thrust);
        lang.m_p2ei_thrust_blank = Lang::update_language(cfg, "mP2eiThrustBlank", lang.m_p2ei_thrust_blank);
        lang.m_p2ei_rpm = Lang::update_language(cfg, "mP2eiRPM", lang.m_p2ei_rpm);
        lang.m_p2ei_rpm_blank = Lang::update_language(cfg, "mP2eiRPMBlank", lang.m_p2ei_rpm_blank);
        lang.m_p2ei_prop_pitch = Lang::update_language(cfg, "mP2eiPropPitch", lang.m_p2ei_prop_pitch);
        lang.m_p2ei_prop_pitch_blank = Lang::update_language(cfg, "mP2eiPropPitchBlank", lang.m_p2ei_prop_pitch_blank);
        lang.m_p2ei_eff_eta = Lang::update_language(cfg, "mP2eiEffEta", lang.m_p2ei_eff_eta);
        lang.m_p2ei_eff_eta_blank = Lang::update_language(cfg, "mP2eiEffEtaBlank", lang.m_p2ei_eff_eta_blank);
        lang.m_p2ei_eff_hp = Lang::update_language(cfg, "mP2eiEffHp", lang.m_p2ei_eff_hp);
        lang.m_p2ei_eff_hp_blank = Lang::update_language(cfg, "mP2eiEffHpBlank", lang.m_p2ei_eff_hp_blank);
        lang.m_p2ei_pressure = Lang::update_language(cfg, "mP2eiPressure", lang.m_p2ei_pressure);
        lang.m_p2ei_pressure_blank = Lang::update_language(cfg, "mP2eiPressureBlank", lang.m_p2ei_pressure_blank);
        lang.m_p2ei_power_percent = Lang::update_language(cfg, "mP2eiPowerPercent", lang.m_p2ei_power_percent);
        lang.m_p2ei_power_percent_blank = Lang::update_language(cfg, "mP2eiPowerPercentBlank", lang.m_p2ei_power_percent_blank);
        lang.m_p2ei_fuel_kg = Lang::update_language(cfg, "mP2eiFuelKg", lang.m_p2ei_fuel_kg);
        lang.m_p2ei_fuel_kg_blank = Lang::update_language(cfg, "mP2eiFuelKgBlank", lang.m_p2ei_fuel_kg_blank);
        lang.m_p2ei_fuel_time = Lang::update_language(cfg, "mP2eiFuelTime", lang.m_p2ei_fuel_time);
        lang.m_p2ei_fuel_time_blank = Lang::update_language(cfg, "mP2eiFuelTimeBlank", lang.m_p2ei_fuel_time_blank);
        lang.m_p2ei_wep_kg = Lang::update_language(cfg, "mP2eiWepKg", lang.m_p2ei_wep_kg);
        lang.m_p2ei_wep_kg_blank = Lang::update_language(cfg, "mP2eiWepKgBlank", lang.m_p2ei_wep_kg_blank);
        lang.m_p2ei_wep_time = Lang::update_language(cfg, "mP2eiWepTime", lang.m_p2ei_wep_time);
        lang.m_p2ei_wep_time_blank = Lang::update_language(cfg, "mP2eiWepTimeBlank", lang.m_p2ei_wep_time_blank);
        lang.m_p2ei_temp = Lang::update_language(cfg, "mP2eiTemp", lang.m_p2ei_temp);
        lang.m_p2ei_temp_blank = Lang::update_language(cfg, "mP2eiTempBlank", lang.m_p2ei_temp_blank);
        lang.m_p2ei_oil_temp = Lang::update_language(cfg, "mP2eiOilTemp", lang.m_p2ei_oil_temp);
        lang.m_p2ei_oil_temp_blank = Lang::update_language(cfg, "mP2eiOilTempBlank", lang.m_p2ei_oil_temp_blank);
        lang.m_p2ei_heat_tolerance = Lang::update_language(cfg, "mP2eiHeatTolerance", lang.m_p2ei_heat_tolerance);
        lang.m_p2ei_heat_tolerance_blank = Lang::update_language(cfg, "mP2eiHeatToleranceBlank", lang.m_p2ei_heat_tolerance_blank);
        lang.m_p2ei_eng_response = Lang::update_language(cfg, "mP2eiEngResponse", lang.m_p2ei_eng_response);
        lang.m_p2ei_eng_response_blank = Lang::update_language(cfg, "mP2eiEngResponseBlank", lang.m_p2ei_eng_response_blank);

        lang.m_p3_crosshair = Lang::update_language(cfg, "mP3Crosshair", lang.m_p3_crosshair);
        lang.m_p3_crosshair_blank = Lang::update_language(cfg, "mP3CrosshairBlank", lang.m_p3_crosshair_blank);
        lang.m_p3_crosshair_display = Lang::update_language(cfg, "mP3CrosshairDisplay", lang.m_p3_crosshair_display);
        lang.m_p3_crosshair_display_blank = Lang::update_language(cfg, "mP3CrosshairDisplayBlank", lang.m_p3_crosshair_display_blank);
        lang.m_p3_text = Lang::update_language(cfg, "mP3Text", lang.m_p3_text);
        lang.m_p3_text_blank = Lang::update_language(cfg, "mP3TextBlank", lang.m_p3_text_blank);
        lang.m_p3_flap_angle_bar = Lang::update_language(cfg, "mP3FlapAngleBar", lang.m_p3_flap_angle_bar);
        lang.m_p3_flap_angle_bar_blank = Lang::update_language(cfg, "mP3FlapAngleBarBlank", lang.m_p3_flap_angle_bar_blank);
        lang.m_p3_crosshair_texture = Lang::update_language(cfg, "mP3CrosshairTexture", lang.m_p3_crosshair_texture);
        lang.m_p3_crosshair_texture_blank = Lang::update_language(cfg, "mP3CrosshairTextureBlank", lang.m_p3_crosshair_texture_blank);
        lang.m_p3_choose_texture = Lang::update_language(cfg, "mP3ChooseTexture", lang.m_p3_choose_texture);
        lang.m_p3_choose_texture_blank = Lang::update_language(cfg, "mP3ChooseTextureBlank", lang.m_p3_choose_texture_blank);
        lang.m_p3_crosshair_size = Lang::update_language(cfg, "mP3CrosshairSize", lang.m_p3_crosshair_size);
        lang.m_p3_mono_font = Lang::update_language(cfg, "mP3MonoFont", lang.m_p3_mono_font);
        lang.m_p3_mono_font_blank = Lang::update_language(cfg, "mP3MonoFontBlank", lang.m_p3_mono_font_blank);

        lang.m_p4_flight_info_panel = Lang::update_language(cfg, "mP4FlightInfoPanel", lang.m_p4_flight_info_panel);
        lang.m_p4attitude_indicator_panel = Lang::update_language(cfg, "mP4attitudeIndicatorPanel", lang.m_p4attitude_indicator_panel);
        lang.m_p4attitude_indicator_panel_blank = Lang::update_language(cfg, "mP4attitudeIndicatorPanelBlank", lang.m_p4attitude_indicator_panel_blank);
        lang.m_p4_fm_panel = Lang::update_language(cfg, "mP4FMPanel", lang.m_p4_fm_panel);
        lang.m_p4_fm_panel_blank = Lang::update_language(cfg, "mP4FMPanelBlank", lang.m_p4_fm_panel_blank);
        lang.m_p4fi_ias = Lang::update_language(cfg, "mP4fiIAS", lang.m_p4fi_ias);
        lang.m_p4fi_ias_blank = Lang::update_language(cfg, "mP4fiIASBlank", lang.m_p4fi_ias_blank);
        lang.m_p4fi_tas = Lang::update_language(cfg, "mP4fiTAS", lang.m_p4fi_tas);
        lang.m_p4fi_tas_blank = Lang::update_language(cfg, "mP4fiTASBlank", lang.m_p4fi_tas_blank);
        lang.m_p4fi_mach = Lang::update_language(cfg, "mP4fiMach", lang.m_p4fi_mach);
        lang.m_p4fi_mach_blank = Lang::update_language(cfg, "mP4fiMachBlank", lang.m_p4fi_mach_blank);
        lang.m_p4fi_compass = Lang::update_language(cfg, "mP4fiCompass", lang.m_p4fi_compass);
        lang.m_p4fi_compass_blank = Lang::update_language(cfg, "mP4fiCompassBlank", lang.m_p4fi_compass_blank);
        lang.m_p4fi_height = Lang::update_language(cfg, "mP4fiHeight", lang.m_p4fi_height);
        lang.m_p4fi_height_blank = Lang::update_language(cfg, "mP4fiHeightBlank", lang.m_p4fi_height_blank);
        lang.m_p4fi_vario = Lang::update_language(cfg, "mP4fiVario", lang.m_p4fi_vario);
        lang.m_p4fi_vario_blank = Lang::update_language(cfg, "mP4fiVarioBlank", lang.m_p4fi_vario_blank);
        lang.m_p4fi_sep = Lang::update_language(cfg, "mP4fiSEP", lang.m_p4fi_sep);
        lang.m_p4fi_sep_blank = Lang::update_language(cfg, "mP4fiSEPBlank", lang.m_p4fi_sep_blank);
        lang.m_p4fi_acc = Lang::update_language(cfg, "mP4fiAcc", lang.m_p4fi_acc);
        lang.m_p4fi_acc_blank = Lang::update_language(cfg, "mP4fiAccBlank", lang.m_p4fi_acc_blank);
        lang.m_p4fi_wx = Lang::update_language(cfg, "mP4fiWx", lang.m_p4fi_wx);
        lang.m_p4fi_wx_blank = Lang::update_language(cfg, "mP4fiWxBlank", lang.m_p4fi_wx_blank);
        lang.m_p4fi_ny = Lang::update_language(cfg, "mP4fiNy", lang.m_p4fi_ny);
        lang.m_p4fi_ny_blank = Lang::update_language(cfg, "mP4fiNyBlank", lang.m_p4fi_ny_blank);
        lang.m_p4fi_turn = Lang::update_language(cfg, "mP4fiTurn", lang.m_p4fi_turn);
        lang.m_p4fi_turn_blank = Lang::update_language(cfg, "mP4fiTurnBlank", lang.m_p4fi_turn_blank);
        lang.m_p4fi_turn_radius = Lang::update_language(cfg, "mP4fiTurnRadius", lang.m_p4fi_turn_radius);
        lang.m_p4fi_turn_radius_blank = Lang::update_language(cfg, "mP4fiTurnRadiusBlank", lang.m_p4fi_turn_radius_blank);
        lang.m_p4fi_ao_a = Lang::update_language(cfg, "mP4fiAoA", lang.m_p4fi_ao_a);
        lang.m_p4fi_ao_a_blank = Lang::update_language(cfg, "mP4fiAoABlank", lang.m_p4fi_ao_a_blank);
        lang.m_p4fi_ao_s = Lang::update_language(cfg, "mP4fiAoS", lang.m_p4fi_ao_s);
        lang.m_p4fi_ao_s_blank = Lang::update_language(cfg, "mP4fiAoSBlank", lang.m_p4fi_ao_s_blank);
        lang.m_p4fi_wing_sweep = Lang::update_language(cfg, "mP4fiWingSweep", lang.m_p4fi_wing_sweep);
        lang.m_p4fi_wing_sweep_blank = Lang::update_language(cfg, "mP4fiWingSweepBlank", lang.m_p4fi_wing_sweep_blank);
        lang.m_p4fi_radio_alt = Lang::update_language(cfg, "mP4fiRadioAlt", lang.m_p4fi_radio_alt);
        lang.m_p4fi_radio_alt_blank = Lang::update_language(cfg, "mP4fiRadioAltBlank", lang.m_p4fi_radio_alt_blank);
        lang.m_p4_flight_info_blank = Lang::update_language(cfg, "mP4FlightInfoBlank", lang.m_p4_flight_info_blank);
        lang.m_p4_flight_info_glass_edge = Lang::update_language(cfg, "mP4FlightInfoGlassEdge", lang.m_p4_flight_info_glass_edge);
        lang.m_p4_flight_info_glass_edge_blank = Lang::update_language(cfg, "mP4FlightInfoGlassEdgeBlank", lang.m_p4_flight_info_glass_edge_blank);
        lang.m_p4_panel_font = Lang::update_language(cfg, "mP4PanelFont", lang.m_p4_panel_font);
        lang.m_p4_font_adjust = Lang::update_language(cfg, "mP4FontAdjust", lang.m_p4_font_adjust);
        lang.m_p4_column_adjust = Lang::update_language(cfg, "mP4ColumnAdjust", lang.m_p4_column_adjust);
        lang.m_p5_logging_and_charting = Lang::update_language(cfg, "mP5LoggingAndCharting", lang.m_p5_logging_and_charting);
        lang.m_p5_logging_and_charting_blank = Lang::update_language(cfg, "mP5LoggingAndChartingBlank", lang.m_p5_logging_and_charting_blank);
        lang.m_p5_information = Lang::update_language(cfg, "mP5Information", lang.m_p5_information);
        lang.m_p5_information_blank = Lang::update_language(cfg, "mP5InformationBlank", lang.m_p5_information_blank);
        lang.m_p5_fm_choose = Lang::update_language(cfg, "mP5FMChoose", lang.m_p5_fm_choose);
        lang.m_p5_fm_choose_blank = Lang::update_language(cfg, "mP5FMChooseBlank", lang.m_p5_fm_choose_blank);
        lang.m_p5_fm_display_key = Lang::update_language(cfg, "mP5FMDisplayKey", lang.m_p5_fm_display_key);
        lang.m_p5_fm_display_key_tip = Lang::update_language(cfg, "mP5FMDisplayKeyTip", lang.m_p5_fm_display_key_tip);
        lang.m_p5_fm_print_enable = Lang::update_language(cfg, "mP5FMPrintEnable", lang.m_p5_fm_print_enable);
        lang.m_p5_fm_print_enable_blank = Lang::update_language(cfg, "mP5FMPrintEnableBlank", lang.m_p5_fm_print_enable_blank);

        lang.m_p6_axis_panel = Lang::update_language(cfg, "mP6AxisPanel", lang.m_p6_axis_panel);
        lang.m_p6_axis_panel_blank = Lang::update_language(cfg, "mP6AxisPanelBlank", lang.m_p6_axis_panel_blank);
        lang.m_p6_axis_edge = Lang::update_language(cfg, "mP6AxisEdge", lang.m_p6_axis_edge);
        lang.m_p6_axis_edge_blank = Lang::update_language(cfg, "mP6AxisEdgeBlank", lang.m_p6_axis_edge_blank);
        lang.m_p6_gear_and_flaps = Lang::update_language(cfg, "mP6GearAndFlaps", lang.m_p6_gear_and_flaps);
        lang.m_p6_gear_and_flaps_edge = Lang::update_language(cfg, "mP6GearAndFlapsEdge", lang.m_p6_gear_and_flaps_edge);
        lang.m_p6_gear_and_flaps_edge_blank = Lang::update_language(cfg, "mP6GearAndFlapsEdgeBlank", lang.m_p6_gear_and_flaps_edge_blank);
        lang.m_p6engine_control = Lang::update_language(cfg, "mP6engineControl", lang.m_p6engine_control);
        lang.m_p6engine_control_blank = Lang::update_language(cfg, "mP6engineControlBlank", lang.m_p6engine_control_blank);
        lang.m_p6ec_throttle = Lang::update_language(cfg, "mP6ecThrottle", lang.m_p6ec_throttle);
        lang.m_p6ec_throttle_blank = Lang::update_language(cfg, "mP6ecThrottleBlank", lang.m_p6ec_throttle_blank);
        lang.m_p6ec_pitch = Lang::update_language(cfg, "mP6ecPitch", lang.m_p6ec_pitch);
        lang.m_p6ec_pitch_blank = Lang::update_language(cfg, "mP6ecPitchBlank", lang.m_p6ec_pitch_blank);
        lang.m_p6ec_mixture = Lang::update_language(cfg, "mP6ecMixture", lang.m_p6ec_mixture);
        lang.m_p6ec_mixture_blank = Lang::update_language(cfg, "mP6ecMixtureBlank", lang.m_p6ec_mixture_blank);
        lang.m_p6ec_radiator = Lang::update_language(cfg, "mP6ecRadiator", lang.m_p6ec_radiator);
        lang.m_p6ec_radiator_blank = Lang::update_language(cfg, "mP6ecRadiatorBlank", lang.m_p6ec_radiator_blank);
        lang.m_p6ec_compressor = Lang::update_language(cfg, "mP6ecCompressor", lang.m_p6ec_compressor);
        lang.m_p6ec_compressor_blank = Lang::update_language(cfg, "mP6ecCompressorBlank", lang.m_p6ec_compressor_blank);
        lang.m_p6ec_l_fuel = Lang::update_language(cfg, "mP6ecLFuel", lang.m_p6ec_l_fuel);
        lang.m_p6ec_l_fuel_blank = Lang::update_language(cfg, "mP6ecLFuelBlank", lang.m_p6ec_l_fuel_blank);

        lang.m_flight_info = Lang::update_language(cfg, "mFlightInfo", lang.m_flight_info);
        lang.m_engine_info = Lang::update_language(cfg, "mEngineInfo", lang.m_engine_info);
        lang.m_control_info = Lang::update_language(cfg, "mControlInfo", lang.m_control_info);
        lang.m_logging_and_analysis = Lang::update_language(cfg, "mLoggingAndAnalysis", lang.m_logging_and_analysis);
        lang.m_crosshair = Lang::update_language(cfg, "mCrosshair", lang.m_crosshair);
        lang.m_advanced_option = Lang::update_language(cfg, "mAdvancedOption", lang.m_advanced_option);
        lang.o_skey_word1 = Lang::update_language(cfg, "oSkeyWord1", lang.o_skey_word1);
        lang.o_skey_word2 = Lang::update_language(cfg, "oSkeyWord2", lang.o_skey_word2);
        lang.d_fprev = Lang::update_language(cfg, "dFprev", lang.d_fprev);
        lang.d_fnext = Lang::update_language(cfg, "dFnext", lang.d_fnext);
        lang.d_f_title1 = Lang::update_language(cfg, "dFTitle1", lang.d_f_title1);
        lang.d_f_title1_x = Lang::update_language(cfg, "dFTitle1X", lang.d_f_title1_x);
        lang.d_f_title1_y = Lang::update_language(cfg, "dFTitle1Y", lang.d_f_title1_y);
        lang.d_f_title2 = Lang::update_language(cfg, "dFTitle2", lang.d_f_title2);
        lang.d_f_title2_x = Lang::update_language(cfg, "dFTitle2X", lang.d_f_title2_x);
        lang.d_f_title2_y = Lang::update_language(cfg, "dFTitle2Y", lang.d_f_title2_y);
        lang.d_f_title3 = Lang::update_language(cfg, "dFTitle3", lang.d_f_title3);
        lang.d_f_title3_x = Lang::update_language(cfg, "dFTitle3X", lang.d_f_title3_x);
        lang.d_f_title3_y = Lang::update_language(cfg, "dFTitle3Y", lang.d_f_title3_y);
        lang.d_f_title4 = Lang::update_language(cfg, "dFTitle4", lang.d_f_title4);
        lang.d_f_title4_x = Lang::update_language(cfg, "dFTitle4X", lang.d_f_title4_x);
        lang.d_f_title4_y = Lang::update_language(cfg, "dFTitle4Y", lang.d_f_title4_y);
        lang.d_f_title5 = Lang::update_language(cfg, "dFTitle5", lang.d_f_title5);
        lang.d_f_title5_x = Lang::update_language(cfg, "dFTitle5X", lang.d_f_title5_x);
        lang.d_f_title5_y = Lang::update_language(cfg, "dFTitle5Y", lang.d_f_title5_y);
        lang.d_f_title_hz = Lang::update_language(cfg, "dFTitleHZ", lang.d_f_title_hz);
        lang.e_throttle = Lang::update_language(cfg, "eThrottle", lang.e_throttle);
        lang.e_proppitch = Lang::update_language(cfg, "eProppitch", lang.e_proppitch);
        lang.e_mixture = Lang::update_language(cfg, "eMixture", lang.e_mixture);
        lang.e_compressor = Lang::update_language(cfg, "eCompressor", lang.e_compressor);
        lang.e_radiator = Lang::update_language(cfg, "eRadiator", lang.e_radiator);
        lang.e_thurst_p = Lang::update_language(cfg, "eThurstP", lang.e_thurst_p);
        lang.e_fuel_per = Lang::update_language(cfg, "eFuelPer", lang.e_fuel_per);
        lang.e_pitch_deg = Lang::update_language(cfg, "ePitchDeg", lang.e_pitch_deg);
        lang.e_magneto = Lang::update_language(cfg, "eMagneto", lang.e_magneto);
        lang.e_type = Lang::update_language(cfg, "eType", lang.e_type);
        lang.e_power = Lang::update_language(cfg, "ePower", lang.e_power);
        lang.e_thurst = Lang::update_language(cfg, "eThurst", lang.e_thurst);
        lang.e_eff_power = Lang::update_language(cfg, "eEffPower", lang.e_eff_power);
        lang.e_power_percent = Lang::update_language(cfg, "ePowerPercent", lang.e_power_percent);
        lang.e_fuel = Lang::update_language(cfg, "eFuel", lang.e_fuel);
        lang.e_fuel_p = Lang::update_language(cfg, "eFuelP", lang.e_fuel_p);
        lang.e_fuel_prs = Lang::update_language(cfg, "eFuelPrs", lang.e_fuel_prs);
        lang.e_rpm = Lang::update_language(cfg, "eRPM", lang.e_rpm);
        lang.e_temp = Lang::update_language(cfg, "eTemp", lang.e_temp);
        lang.e_eff = Lang::update_language(cfg, "eEff", lang.e_eff);
        lang.e_fueltime = Lang::update_language(cfg, "eFueltime", lang.e_fueltime);
        lang.e_weptime = Lang::update_language(cfg, "eWeptime", lang.e_weptime);
        lang.e_wep = Lang::update_language(cfg, "eWep", lang.e_wep);
        lang.e_atm = Lang::update_language(cfg, "eATM", lang.e_atm);
        lang.e_oil = Lang::update_language(cfg, "eOil", lang.e_oil);
        lang.e_overheat = Lang::update_language(cfg, "eOverheat", lang.e_overheat);
        lang.e_eng_res = Lang::update_language(cfg, "eEngRes", lang.e_eng_res);
        lang.e_title = Lang::update_language(cfg, "eTitle", lang.e_title);
        lang.f_ias = Lang::update_language(cfg, "fIAS", lang.f_ias);
        lang.f_tas = Lang::update_language(cfg, "fTAS", lang.f_tas);
        lang.f_compass = Lang::update_language(cfg, "fCompass", lang.f_compass);
        lang.f_mach = Lang::update_language(cfg, "fMach", lang.f_mach);
        lang.f_wx = Lang::update_language(cfg, "fWx", lang.f_wx);
        lang.f_tr = Lang::update_language(cfg, "fTR", lang.f_tr);
        lang.f_t_rr = Lang::update_language(cfg, "fTRr", lang.f_t_rr);
        lang.f_alt = Lang::update_language(cfg, "fAlt", lang.f_alt);
        lang.f_vario = Lang::update_language(cfg, "fVario", lang.f_vario);
        lang.f_acc = Lang::update_language(cfg, "fAcc", lang.f_acc);
        lang.f_sep = Lang::update_language(cfg, "fSEP", lang.f_sep);
        lang.f_ao_a = Lang::update_language(cfg, "fAoA", lang.f_ao_a);
        lang.f_ao_s = Lang::update_language(cfg, "fAoS", lang.f_ao_s);
        lang.f_ws = Lang::update_language(cfg, "fWs", lang.f_ws);
        lang.f_ra = Lang::update_language(cfg, "fRa", lang.f_ra);
        lang.f_gl = Lang::update_language(cfg, "fGL", lang.f_gl);
        lang.f_title = Lang::update_language(cfg, "fTitle", lang.f_title);
        lang.g_flaps = Lang::update_language(cfg, "gFlaps", lang.g_flaps);
        lang.g_title = Lang::update_language(cfg, "gTitle", lang.g_title);
        lang.g_gear = Lang::update_language(cfg, "gGear", lang.g_gear);
        lang.g_gear_down = Lang::update_language(cfg, "gGearDown", lang.g_gear_down);
        lang.g_brake = Lang::update_language(cfg, "gBrake", lang.g_brake);
        lang.s_title = Lang::update_language(cfg, "sTitle", lang.s_title);
        lang.s_wait = Lang::update_language(cfg, "sWait", lang.s_wait);
        lang.s_enter = Lang::update_language(cfg, "sEnter", lang.s_enter);
        lang.s_check = Lang::update_language(cfg, "sCheck", lang.s_check);
        lang.v_aileron = Lang::update_language(cfg, "vAileron", lang.v_aileron);
        lang.v_elevator = Lang::update_language(cfg, "vElevator", lang.v_elevator);
        lang.v_rudder = Lang::update_language(cfg, "vRudder", lang.v_rudder);
        lang.v_vario_w = Lang::update_language(cfg, "vVarioW", lang.v_vario_w);
        lang.v_title = Lang::update_language(cfg, "vTitle", lang.v_title);
        lang.c_startlog = Lang::update_language(cfg, "cStartlog", lang.c_startlog);
        lang.c_savelog = Lang::update_language(cfg, "cSavelog", lang.c_savelog);
        lang.c_plsopen = Lang::update_language(cfg, "cPlsopen", lang.c_plsopen);
        lang.c_openpad = Lang::update_language(cfg, "cOpenpad", lang.c_openpad);

        lang.l1 = Lang::update_language(cfg, "l1", lang.l1);
        lang.l2 = Lang::update_language(cfg, "l2", lang.l2);
        lang.l3 = Lang::update_language(cfg, "l3", lang.l3);
        lang.l4 = Lang::update_language(cfg, "l4", lang.l4);
        lang.l5 = Lang::update_language(cfg, "l5", lang.l5);
        lang.l6 = Lang::update_language(cfg, "l6", lang.l6);
        lang.l7 = Lang::update_language(cfg, "l7", lang.l7);
        lang.l8 = Lang::update_language(cfg, "l8", lang.l8);
        lang.l9 = Lang::update_language(cfg, "l9", lang.l9);
        lang.l10 = Lang::update_language(cfg, "l10", lang.l10);
        lang.l11 = Lang::update_language(cfg, "l11", lang.l11);
        lang.l12 = Lang::update_language(cfg, "l12", lang.l12);
        lang.l13 = Lang::update_language(cfg, "l13", lang.l13);
        lang.l14 = Lang::update_language(cfg, "l14", lang.l14);
        lang.l15 = Lang::update_language(cfg, "l15", lang.l15);
        lang.l16 = Lang::update_language(cfg, "l16", lang.l16);
        lang.l17 = Lang::update_language(cfg, "l17", lang.l17);
        lang.l18 = Lang::update_language(cfg, "l18", lang.l18);
        lang.l19 = Lang::update_language(cfg, "l19", lang.l19);
        lang.l20 = Lang::update_language(cfg, "l20", lang.l20);
        lang.l21 = Lang::update_language(cfg, "l21", lang.l21);
        lang.l22 = Lang::update_language(cfg, "l22", lang.l22);
        lang.l23 = Lang::update_language(cfg, "l23", lang.l23);
        lang.l24 = Lang::update_language(cfg, "l24", lang.l24);
        lang.l25 = Lang::update_language(cfg, "l25", lang.l25);
        lang.l26 = Lang::update_language(cfg, "l26", lang.l26);
        lang.l27 = Lang::update_language(cfg, "l27", lang.l27);
        lang.l28 = Lang::update_language(cfg, "l28", lang.l28);
        lang.l29 = Lang::update_language(cfg, "l29", lang.l29);
        lang.l30 = Lang::update_language(cfg, "l30", lang.l30);
        lang.l31 = Lang::update_language(cfg, "l31", lang.l31);
        lang.lfail_create = Lang::update_language(cfg, "lfailCreate", lang.lfail_create);
        lang.lfail_write = Lang::update_language(cfg, "lfailWrite", lang.lfail_write);
        lang.f_a1 = Lang::update_language(cfg, "fA1", lang.f_a1);
        lang.f_a2 = Lang::update_language(cfg, "fA2", lang.f_a2);
        lang.f_a3 = Lang::update_language(cfg, "fA3", lang.f_a3);
        lang.f_a4 = Lang::update_language(cfg, "fA4", lang.f_a4);
        lang.f_a_roll1 = Lang::update_language(cfg, "fA_roll1", lang.f_a_roll1);
        lang.f_a_roll2 = Lang::update_language(cfg, "fA_roll2", lang.f_a_roll2);
        lang.f_a_roll3 = Lang::update_language(cfg, "fA_roll3", lang.f_a_roll3);
        lang.f_a_turn1 = Lang::update_language(cfg, "fA_turn1", lang.f_a_turn1);
        lang.f_a_turn2 = Lang::update_language(cfg, "fA_turn2", lang.f_a_turn2);
        lang.f_a_turn3 = Lang::update_language(cfg, "fA_turn3", lang.f_a_turn3);
        lang.f_a_turn4 = Lang::update_language(cfg, "fA_turn4", lang.f_a_turn4);

        lang.noblkx = Lang::update_language(cfg, "noblkx", lang.noblkx);
        lang.fm_missing_toast = Lang::update_language(cfg, "fmMissingToast", lang.fm_missing_toast);
        lang.fm_corrupt_toast = Lang::update_language(cfg, "fmCorruptToast", lang.fm_corrupt_toast);
        lang.b_fm_parts = Lang::update_language(cfg, "bFmParts", lang.b_fm_parts);
        lang.b_cd_min = Lang::update_language(cfg, "bCdMin", lang.b_cd_min);
        lang.b_cl0 = Lang::update_language(cfg, "bCl0", lang.b_cl0);
        lang.b_ao_a_crit = Lang::update_language(cfg, "bAoACrit", lang.b_ao_a_crit);
        lang.b_ao_a_crit_cl = Lang::update_language(cfg, "bAoACritCl", lang.b_ao_a_crit_cl);

        lang.b_fm_version = Lang::update_language(cfg, "bFmVersion", lang.b_fm_version);
        lang.b_weight = Lang::update_language(cfg, "bWeight", lang.b_weight);
        lang.b_crit_speed = Lang::update_language(cfg, "bCritSpeed", lang.b_crit_speed);
        lang.b_allow_load_factor = Lang::update_language(cfg, "bAllowLoadFactor", lang.b_allow_load_factor);
        lang.b_average_heat_recovery = Lang::update_language(cfg, "bAverageHeatRecovery", lang.b_average_heat_recovery);
        lang.b_nitro = Lang::update_language(cfg, "bNitro", lang.b_nitro);
        lang.b_flap_restrict = Lang::update_language(cfg, "bFlapRestrict", lang.b_flap_restrict);
        lang.b_eff_speed_and_power_loss = Lang::update_language(cfg, "bEffSpeedAndPowerLoss", lang.b_eff_speed_and_power_loss);
        lang.b_inertia = Lang::update_language(cfg, "bInertia", lang.b_inertia);
        lang.b_max_lift_load350 = Lang::update_language(cfg, "bMaxLiftLoad350", lang.b_max_lift_load350);
        lang.b_lift = Lang::update_language(cfg, "bLift", lang.b_lift);
        lang.b_drag = Lang::update_language(cfg, "bDrag", lang.b_drag);

        lang
    }
}

// ---------- 边界测试 (期望值 = Java 8 oracle 实测, 生成方法见 table.rs 头部) ----------
#[cfg(test)]
mod tests;

