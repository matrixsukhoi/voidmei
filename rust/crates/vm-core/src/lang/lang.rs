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

use crate::lang::table;

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
    // public static String eThrottle;
    // public static String eProppitch;
    // public static String eMixture;
    // public static String eCompressor;
    // public static String eRadiator;
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
    // public static String eFueltimeP;
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

    // app
    // public static String appName = "VoidMei";
    // public static String appTooltips = "WT8111端口信息分析、显示、记录工具";
    //
    // public static String close="关闭";
    // public static String about="关于";
    // public static String
    // aboutcontent="1.本程序对游戏程序及进程无任何修改,所有信息通过HTTP/GET请求读取WT官方提供的8111端口或离线拆包数据获得.\n\r";
    // public static String
    // aboutcontentsub1="2.本程序只是兴趣使然的创作,禁止用于任何商业用途.程序代码完全开源,加入QQ群620027287可获得最新源码";
    // public static String aboutcontentsub2="3.本程序的设计目标是帮助WT玩家更好理解飞行与空战.";
    // public static String failaddtoTray="托盘加入失败";
    // public static String httpHeader="Mozilla/5.0 (Windows NT 10.0; Win64; x64)
    // AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.63 Safari/537.36
    // Edg/102.0.1245.33"
    // + "\r\n";
    // public static String Systemerror="该程序在VISTA/WIN
    // 7以下的操作系统上运行会造成游戏丢帧、卡顿现象，建议您更新系统";
    // //MainForm
    // public static String mCancel="取 消";
    // public static String mStart="开 始";
    // public static String mPlsclosePreview="请关闭预览窗口再继续";
    // public static String mDisplayPreview="显示预览";
    // public static String mSavePosition="保存位置";
    // public static String mMovePanel="请拖动面板进行位置调整，调整完保存位置";
    // public static String mPreviewWarning="预览窗口已经打开，请勿再次打开";
    // public static String mPositionSaved="窗口位置已保存";
    // public static String mPreviewNotOpen="请打开预览窗口移动位置再保存";
    //
    //
    // public static String mP1NumColor=" 数字色 ";
    // public static String mP1NumColorBlank=" ";
    // public static String mP1LabelColor=" 标签色 ";
    // public static String mP1LabelColorBlank=" ";
    // public static String mP1UnitColor=" 单位色 ";
    // public static String mP1UnitColorBlank=" ";
    // public static String mP1WarnColor=" 告警色 ";
    // public static String mP1WarnColorBlank=" ";
    // public static String mP1ShadeColor=" 描边色 ";
    // public static String mP1ShadeColorBlank=" ";
    //
    // public static String mP1TempNotification=" 温度通知 ";
    // public static String mP1TempNotificationBlank=" ";
    // public static String mP1VoiceWarning=" 语音告警 ";
    // public static String mP1VoiceWarningBlank=" ";
    // public static String mP1drawFontShape=" 简化字体绘制 ";
    // public static String mP1drawFontShapeBlank=" ";
    // public static String mP1GlobalNumberFont=" 全局数字字体 ";
    // public static String mP1GlobalNumberFontBlank=" ";
    // public static String mP1Interval="数据采集时间间隔";
    //
    // public static String mP2EnginePanel=" 发动机面板";
    // public static String mP2EnginePanelBlank=" ";
    // public static String mP2EngineGlassEdge="玻璃边框";
    // public static String mP2EngineGlassEdgeBlank=" ";
    // public static String mP2PanelFont="面板显示字体";
    // public static String mP2FontAdjust="字体大小调整 ";
    //
    //
    // public static String mP2EngineBlank=" ";
    // public static String mP2eiHorsePower="显示功 率";
    // public static String mP2eiHorsePowerBlank=" ";
    // public static String mP2eiThrust="显示推 力";
    // public static String mP2eiThrustBlank=" ";
    // public static String mP2eiRPM="显示转 速";
    // public static String mP2eiRPMBlank=" ";
    // public static String mP2eiPropPitch="显示桨距角";
    // public static String mP2eiPropPitchBlank=" ";
    // public static String mP2eiEffEta="显示桨效率";
    // public static String mP2eiEffEtaBlank=" ";
    // public static String mP2eiEffHp="显示实功率";
    // public static String mP2eiEffHpBlank=" ";
    // public static String mP2eiPressure="显示进气压";
    // public static String mP2eiPressureBlank=" ";
    // public static String mP2eiPowerPercent="显示动力量";
    // public static String mP2eiPowerPercentBlank=" ";
    // public static String mP2eiFuelKg="显示燃油量";
    // public static String mP2eiFuelKgBlank=" ";
    // public static String mP2eiFuelTime="显示燃油时";
    // public static String mP2eiFuelTimeBlank=" ";
    // public static String mP2eiWepKg="显示加力量";
    // public static String mP2eiWepKgBlank=" ";
    // public static String mP2eiWepTime="显示加力时";
    // public static String mP2eiWepTimeBlank=" ";
    // public static String mP2eiTemp="显示温 度";
    // public static String mP2eiTempBlank=" ";
    // public static String mP2eiOilTemp="显示油 温";
    // public static String mP2eiOilTempBlank=" ";
    // public static String mP2eiHeatTolerance="显示耐热时";
    // public static String mP2eiHeatToleranceBlank=" ";
    // public static String mP2eiEngResponse="显示响应速";
    // public static String mP2eiEngResponseBlank=" ";
    //
    //
    // public static String mP3Crosshair=" 自定义HUD ";
    // public static String mP3CrosshairBlank=" ";
    // public static String mP3CrosshairDisplay=" 显示准星 ";
    // public static String mP3CrosshairDisplayBlank=" ";
    // public static String mP3Text="最小HUD";
    // public static String mP3TextBlank=" ";
    // public static String mP3CrosshairTexture=" 准星贴图 ";
    // public static String mP3CrosshairTextureBlank=" ";
    // public static String mP3ChooseTexture="选择准星贴图 ";
    // public static String mP3ChooseTextureBlank=" ";
    // public static String mP3CrosshairSize=" 自定义HUD大小";
    //
    // public static String mP4FlightInfoPanel="飞行信息面板";
    // public static String mP4attitudeIndicatorPanel="地平仪面板 ";
    // public static String mP4attitudeIndicatorPanelBlank=" ";
    // public static String mP4FMPanel="FM拆包信息";
    // public static String mP4FMPanelBlank=" ";
    //
    //
    // public static String mP4fiIAS="显示示空速";
    // public static String mP4fiIASBlank=" ";
    // public static String mP4fiTAS="显示真空速";
    // public static String mP4fiTASBlank=" ";
    // public static String mP4fiMach="显示马赫数";
    // public static String mP4fiMachBlank=" ";
    // public static String mP4fiCompass="显示航 向";
    // public static String mP4fiCompassBlank=" ";
    // public static String mP4fiHeight="显示高 度";
    // public static String mP4fiHeightBlank=" ";
    // public static String mP4fiVario="显示爬升率";
    // public static String mP4fiVarioBlank=" ";
    // public static String mP4fiSEP="显示ＳＥＰ";
    // public static String mP4fiSEPBlank=" ";
    // public static String mP4fiAcc="显示加速度";
    // public static String mP4fiAccBlank=" ";
    // public static String mP4fiWx="显示滚转率";
    // public static String mP4fiWxBlank=" ";
    // public static String mP4fiNy="显示过 载";
    // public static String mP4fiNyBlank=" ";
    // public static String mP4fiTurn="显示转弯率";
    // public static String mP4fiTurnBlank=" ";
    // public static String mP4fiTurnRadius="显示转半径";
    // public static String mP4fiTurnRadiusBlank=" ";
    // public static String mP4fiAoA="显示攻 角";
    // public static String mP4fiAoABlank=" ";
    // public static String mP4fiAoS="显示侧滑角";
    // public static String mP4fiAoSBlank=" ";
    // public static String mP4fiWingSweep="显示可变翼";
    // public static String mP4fiWingSweepBlank=" ";
    // public static String mP4fiRadioAlt="显示雷达高";
    // public static String mP4fiRadioAltBlank=" ";
    //
    // public static String mP4FlightInfoBlank=" ";
    // public static String mP4FlightInfoGlassEdge="玻璃边框";
    // public static String mP4FlightInfoGlassEdgeBlank=" ";
    // public static String mP4PanelFont="面板显示字体 ";
    // public static String mP4FontAdjust="字体大小调整 ";
    // public static String mP4ColumnAdjust="面板每行个数 ";
    //
    // public static String mP5LoggingAndCharting="飞行记录和图表生成";
    // public static String mP5LoggingAndChartingBlank=" ";
    // public static String mP5Information="通知记录信息";
    // public static String mP5InformationBlank=" ";
    //
    // public static String mP6AxisPanel=" 舵面值面板";
    // public static String mP6AxisPanelBlank=" ";
    // public static String mP6AxisEdge=" 舵面值边框";
    // public static String mP6AxisEdgeBlank=" ";
    // public static String mP6GearAndFlaps="起落架与襟翼面板";
    // public static String mP6GearAndFlapsEdge="起落架与襟翼边框";
    //
    // public static String mP6GearAndFlapsEdgeBlank=" ";
    // public static String mP6engineControl=" 发动机控制面板";
    // public static String mP6engineControlBlank="";
    //
    // public static String mP6ecThrottle="显示节流阀";
    // public static String mP6ecThrottleBlank=" ";
    // public static String mP6ecPitch="显示桨 距";
    // public static String mP6ecPitchBlank=" ";
    // public static String mP6ecMixture="显示混合比";
    // public static String mP6ecMixtureBlank=" ";
    // public static String mP6ecRadiator="显示散热器";
    // public static String mP6ecRadiatorBlank=" ";
    // public static String mP6ecCompressor="显示增压器";
    // public static String mP6ecCompressorBlank=" ";
    // public static String mP6ecLFuel="显示燃油量";
    // public static String mP6ecLFuelBlank=" ";
    //
    //
    // public static String mFlightInfo="飞行信息";
    // public static String mEngineInfo="发动机信息";
    // public static String mControlInfo="飞行控制信息";
    // public static String mLoggingAndAnalysis="记录与分析";
    // public static String mCrosshair="自定义HUD";
    // public static String mAdvancedOption="高级设置";
    //
    // //OtherService
    // public static String oSkeyWord1="热";
    // public static String oSkeyWord2="温";
    //
    // //DrawFrame
    // public static String dFprev="上一个";
    // public static String dFnext="下一个";
    // public static String dFTitle1="时间-高度曲线";
    // public static String dFTitle1X="时间";
    // public static String dFTitle1Y="高度";
    // public static String dFTitle2="功率-高度包线";
    // public static String dFTitle2X="功率";
    // public static String dFTitle2Y="高度";
    // public static String dFTitle3="推力-高度包线";
    // public static String dFTitle3X="推力";
    // public static String dFTitle3Y="高度";
    // public static String dFTitle4="实功率-高度包线";
    // public static String dFTitle4X="实功率";
    // public static String dFTitle4Y="高度";
    // public static String dFTitle5="SEP-高度包线";
    // public static String dFTitle5X="SEP";
    // public static String dFTitle5Y="高度";
    // public static String dFTitleHZ="性能曲线生成";
    //
    // //EngineInfo
    // public static String eThrottle="节";
    // public static String eProppitch="桨";
    // public static String eMixture="混";
    // public static String eCompressor="增";
    // public static String eRadiator="散";
    // public static String eThurstP="推";
    // public static String eFuelPer="油";
    // public static String ePitchDeg="桨距角";
    //// public static String eThrottle="节流阀";
    //// public static String eProppitch="桨 距";
    //// public static String eMixture="混合比";
    //// public static String eCompressor="增压器";
    //// public static String eRadiator="散热器";
    // public static String eMagneto="";
    // public static String eType="机 型";
    // public static String ePower="功 率";
    // public static String eThurst="推 力";
    // public static String eEffPower="实功率";
    // public static String ePowerPercent="动力量";
    // public static String eFuel="燃油量";
    // public static String eFuelP="燃加力";
    // public static String eFuelPrs="油 压";
    // public static String eRPM="转 速";
    // public static String eTemp="温 度";
    // public static String eEff="桨效率";
    // public static String eFueltime="燃油时";
    // public static String eWeptime="加力时";
    // public static String eWep="加力量";
    //// public static String eFueltimeP="与";
    // public static String eATM="进气压";
    // public static String eOil="油 温";
    // public static String eOverheat="耐热时";
    // public static String eEngRes="响应速";
    // public static String eTitle="发动机面板";
    //
    // //FlightInfo
    // public static String fIAS="表 速";
    // public static String fTAS="真空速";
    // public static String fCompass="航 向";
    // public static String fMach="马赫数";
    // public static String fWx="滚转率";
    // public static String fTR="转半径";
    // public static String fTRr="转弯率";
    // public static String fAlt="高 度";
    // public static String fVario="爬升率";
    // public static String fAcc="加速度";
    // public static String fSEP="ＳＥＰ";
    // public static String fAoA="攻 角";
    // public static String fAoS="侧滑角";
    // public static String fWs="可变翼";
    // public static String fRa="雷达高";
    // public static String fGL="过 载";
    // public static String fTitle="飞行信息面板";
    //
    // //GearAndFlaps
    // public static String gFlaps="襟 翼";
    // public static String gTitle="飞行状态";
    // public static String gGear="起落架";
    // public static String gGearDown="收起落";
    // public static String gBrake="减速板";
    //
    // //StatusBar
    // public static String sTitle="状态条";
    // public static String sWait="等待建立连接";
    // public static String sEnter="等待进入游戏";
    // public static String sCheck="检测到游戏开始";
    //
    // //StickValue
    // public static String vAileron="副 翼";
    // public static String vElevator="升降舵";
    // public static String vRudder="方向舵";
    // public static String vVarioW="可变翼";
    // public static String vTitle="操纵面面板";
    //
    // //Controller
    // public static String cStartlog="开始记录端口信息";
    // public static String cSavelog="端口信息保存至";
    // public static String cPlsopen=",请用EXCEL打开";
    // public static String cOpenpad="'您已加入游戏，面板将在' s '秒内打开'";
    // public static String cEnginedmg="'发动机将在' s '秒内损坏，请及时散热'";
    // public static String cWarn1min="引擎还有一分钟损坏";
    // public static String cEngBomb="引擎爆炸了，请立即回航";
    //
    // //flightlog
    // public static String l1="时间/s,";
    // public static String l2="节流阀/%,";
    // public static String l3="表 速/kph,";
    // public static String l4="真空速/kph,";
    // public static String l5="马赫数/Ma,";
    // public static String l6="高 度/m,";
    // public static String l7="温 度/℃,";
    // public static String l8="油 温/℃,";
    // public static String l9="爬升率/m/s,";
    // public static String l10="ＳＥＰ*/m/s,";
    // public static String l11="过 载/G,";
    // public static String l12="滚转率/deg/s,";
    // public static String l13="功 率/bhp,";
    // public static String l14="桨效率/%,";
    // public static String l15="实功率*/bhp,";
    // public static String l16="转 速/rpm,";
    // public static String l17="推 力/kg,";
    // public static String l18="加速度*/m/s^2,";
    // public static String l19="桨 距/%,";
    // public static String l20="桨距角/deg,";
    // public static String l21="散热器/%,";
    // public static String l22="混合比/%,";
    // public static String l23="增压器/档,";
    // public static String l24="磁电机/档,";
    // public static String l25="进气压/ata,";
    // public static String l26="襟 翼/%,";
    // public static String l27="升降舵/%,";
    // public static String l28="滚转舵/%,";
    // public static String l29="方向舵/%,";
    // public static String l30="攻 角/deg,";
    // public static String l31="侧滑角/deg,";
    // public static String lfailCreate="记录文件创建失败";
    // public static String lfailWrite="记录文件写入失败";
    //
    // //FlightAnalyzer
    // public static String fA1="到达 ";
    // public static String fA2="米，用时 ";
    // public static String fA3="秒，平均爬升率 ";
    // public static String fA4="米/秒，记录完成";
    //
    // public static String fA_roll1="速度 ";
    // public static String fA_roll2="km/h下的最大滚转率: ";
    // public static String fA_roll3="度/秒,记录完成";
    //
    // public static String fA_turn1="速度 ";
    // public static String fA_turn2="km/h下的最大法向过载: ";
    // public static String fA_turn3="G, 此时SEP为: ";
    // public static String fA_turn4="m/s, 记录完成;";
    //
    //
    // public static String noblkx = "找不到blkx文件,请使用最新WT拆包aces.vromfs.bin";
    // public static String bFmParts = "------fm器件 %s------\n";
    // public static String bCdMin = "零升阻力系数: %.3f\n";
    // public static String bCl0 = "零攻角升力: %.3f\n";
    // public static String bAoACrit = "临界攻角: [%.1f, %.1f]\n";
    // public static String bAoACritCl = "临界攻角升力系数: [%.2f, %.2f]\n";
    //
    //
    // // 还未加入
    // public static String bFmVersion = "FM文件: %s - %s";
    // public static String bWeight = "空重(kg): %.1f\n最大燃油重量(kg): %.1f\n";
    // public static String bCritSpeed = "临界速度(km/h): [%.0f, %.0f]\n";
    // public static String bAllowLoadFactor = "允许过载(满/半油): [%.1f, %.1f], [%.1f,
    // %.1f]\n";
    // public static String bAverageHeatRecovery = "平均耐热条恢复速率: %.1f\n";
    // public static String bNitro = "加力(kg)/时限(分钟): %.1f / %.1f\n";
    // public static String bFlapRestrict = "襟翼限速(km/h)%d: %.0f%% / %.0f\n";
    // public static String bEffSpeedAndPowerLoss = "三舵有效速度(km/h): [ %.0f, %.0f,
    // %.0f ]\n三舵锁舵因数: [ %.1f, %.1f, %.1f ]\n";
    // public static String bInertia = "三轴转动惯量: [ %.0f, %.0f, %.0f ]\n";
    //
    // public static String bMaxLiftLoad350 = "千米最大升力过载: %.1f / %.1f(襟) @ 350IAS\n";
    //
    // public static String bLift = "主升力面积: %.1f机翼, %.1f机身\n主升力面积因数载荷: %.2f /
    // %.2f(襟)\n翼展效率: %.2f 展弦比: %.1f 后掠角: %.1f\n";
    // public static String bDrag = "主阻力面积因数与加速度系数: %.2f / %.2f\n诱导阻力因数及加速度系数: %.3f
    // / %.0f\n散热/油冷器阻力系数: %.3f / %.3f\n";
    //
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

        // public static String bFmVersion = "FM文件: %s - %s";
        // public static String bWeight = "空重(kg): %.1f\n最大燃油重量(kg): %.1f\n";
        // public static String bCritSpeed = "临界速度(km/h): [%.0f, %.0f]\n";
        // public static String bAllowLoadFactor = "允许过载(满/半油): [%.1f, %.1f], [%.1f,
        // %.1f]\n";
        // public static String bAverageHeatRecovery = "平均耐热条恢复速率: %.1f\n";
        // public static String bNitro = "加力(kg)/时限(分钟): %.1f / %.1f\n";
        // public static String bFlapRestrict = "襟翼限速(km/h)%d: %.0f%% / %.0f\n";
        // public static String bEffSpeedAndPowerLoss = "三舵有效速度(km/h): [ %.0f, %.0f,
        // %.0f ]\n三舵锁舵因数: [ %.1f, %.1f, %.1f ]\n";
        // public static String bInertia = "三轴转动惯量: [ %.0f, %.0f, %.0f ]\n";
        //
        // public static String bMaxLiftLoad350 = "千米最大升力过载: %.1f / %.1f(襟) @ 350IAS\n";
        //
        // public static String bLift = "主升力面积: %.1f机翼, %.1f机身\n主升力面积因数载荷: %.2f /
        // %.2f(襟)\n翼展效率: %.2f 展弦比: %.1f 后掠角: %.1f\n";
        // public static String bDrag = "主阻力面积因数与加速度系数: %.2f / %.2f\n诱导阻力因数及加速度系数: %.3f
        // / %.0f\n散热/油冷器阻力系数: %.3f / %.3f\n";
        //
        // Application.debugPrint("语言初始化完成\n");
        lang
    }
}

// ---------- 边界测试 (期望值 = Java 8 oracle 实测, 生成方法见 table.rs 头部) ----------
#[cfg(test)]
mod tests;

// ---------- 以下为 Java 源文件类尾的注释块 (注释版 initEng), 逐字保留 ----------
//
// public static void initEng(){
//// appName = "VoidMei";
//// appTooltips = "WT tool for displaying,logging and analyzing flightdata by
// NWPU-ACer";
////
//// close="Close";
//// about="About";
//// aboutcontent="All information is from locahost:8111. No modification to
// gamecontent";
//// aboutcontentsub1="";
//// aboutcontentsub2="";
//// failaddtoTray="Fail to add to tray";
//// httpHeader="User-Agent:AppleWebKit/537.88"+ "\r\n";
//// Systemerror="The program use the AERO glass features, so it will
// performance very badly at previous version of Windows older than Vista that
// don't support the AERO glass.";
//// //MainForm
//// mCancel="Cancel";
//// mStart="Confirm";
//// mPlsclosePreview="Please save position then continue";
//// mDisplayPreview="Preview Panel";
//// mSavePosition="Save Position";
//// mMovePanel="Please drag the panel to adjust window location then save
// position";
//// mPreviewWarning="Preview has already open";
//// mPositionSaved="Position Saved";
//// mPreviewNotOpen="Please open the Preview";
////
//// mP1TempNotification="Temperature notification";
//// mP1TempNotificationBlank="";
//// mP1GlobalNumberFont="Global Font for Numbers";
//// mP1GlobalNumberFontBlank=" ";
//// mP1Interval="Data collect interval";
////
//// mP2EnginePanel="Engine Panel";
//// mP2EnginePanelBlank="";
//// mP2EngineGlassEdge="Glass edge";
//// mP2EngineGlassEdgeBlank=" ";
//// mP2PanelFont="Font in the panel ";
//// mP2FontAdjust="Fontsize adjustment ";
////
//// mP3Crosshair="Customed crosshair";
//// mP3CrosshairBlank="";
//// mP3Text="HUD Text";
//// mP3TextBlank="";
//// mP3CrosshairTexture="Crosshair texture";
//// mP3CrosshairTextureBlank="";
//// mP3ChooseTexture="Choose the crosshair texture";
//// mP3ChooseTextureBlank="";
//// mP3CrosshairSize="Crosshair size";
////
//// mP4FlightInfoPanel="Flight status Panel";
//// mP4FlightInfoBlank="";
//// mP4FlightInfoGlassEdge="Glass edge";
//// mP4FlightInfoGlassEdgeBlank=" ";
//// mP4PanelFont="Font in the panel ";
//// mP4FontAdjust="Fontsize adjustment ";
////
//// mP5LoggingAndCharting="Logging and Charting";
//// mP5LoggingAndChartingBlank="";
//// mP5Information="Logdata notification";
//// mP5InformationBlank="";
////
//// mP6AxisPanel="Control surface panel";
//// mP6AxisPanelBlank="";
//// mP6AxisEdge="Control surface glass edge";
//// mP6AxisEdgeBlank=" ";
//// mP6GearAndFlaps="Gear and flaps panel ";
//// mP6GearAndFlapsEdge="Gear and flaps glass edge ";
////
//// mFlightInfo="FlightStatus";
//// mEngineInfo="EngineStatus";
//// mControlInfo="FlightControl";
//// mLoggingAndAnalysis="LogAndAnalysis";
//// mCrosshair="CustomedCrosshair";
//// mAdvancedOption="AdvancedOptions";
////
//// //OtherService
//// oSkeyWord1="heat";
//// oSkeyWord2="temp";
////
//// //DrawFrame
//// dFprev="Prev";
//// dFnext="Next";
//// dFTitle1="Time-Altitude Curve";
//// dFTitle1X="Time";
//// dFTitle1Y="Altitude";
//// dFTitle2="Power-Altitude Envelope";
//// dFTitle2X="Power";
//// dFTitle2Y="Altitude";
//// dFTitle3="Thrust-Altitude Envelope";
//// dFTitle3X="Thrust";
//// dFTitle3Y="Altitude";
//// dFTitle4="EffectivePower*-Altitude Envelope";
//// dFTitle4X="EffectivePower*";
//// dFTitle4Y="Altitude";
//// dFTitle5="SEP*-Altitude Envelope";
//// dFTitle5X="SEP*";
//// dFTitle5Y="Altitude";
//// dFTitleHZ=" Performance Curve Chart";
////
//// //EngineInfo
////
//// eThrottle="Throttle";
//// eProppitch="RPM Thr";
//// eMixture="Mixture";
//// eCompressor="Compressor";
//// eRadiator="Radiator";
//// eMagneto="Magneto";
//// eType="Type";
//// ePower="PWR";
//// eThurst="THR";
//// eEffPower="EffPwr";
//// eFuel="Fuel";
//// eRPM="RPM";
//// eTemp="Temp";
//// eEff="Eff";
//// eFueltime="Fuel";
//// eFuelPrs="FuelPrs";
//// eATM="MPrs";
//// eOil="Oil";
//// eOverheat="Heat";
//// eTitle="EnginePanel";
////
//// //FlightInfo
//// fIAS="IAS";
//// fTAS="TAS";
//// fCompass="HDG";
//// fMach="Mach";
//// fWx="Roll";
//// fAlt="ALT";
//// fVario="Vario";
//// fSEP="SEP*";
//// fGL="Load";
//// fTitle="FlightStatus";
////
//// //GearAndFlaps
//// gFlaps="Flaps";
//// gTitle="GearAndFlaps";
//// gGear="Gear";
//// gBrake="Brake";
////
//// //StatusBar
//// sTitle="StatusBar";
//// sWait="Waitting for connect";
//// sEnter="Watting for gamestart";
//// sCheck="Waitting for Start";
////
//// //StickValue
//// vAileron="Aileron";
//// vElevator="Elevator";
//// vRudder="Rudder";
//// vTitle="Control Surface Panel";
////
//// //Controller
//// cStartlog="Start logging flightdata";
//// cSavelog="Flightdata save to";
//// cPlsopen=", you can open it with EXCEL";
//// cOpenpad="'The panels will open in ' s ' seconds'";
//// cEnginedmg="'Engine will damage in ' s ' seconds'";
//// cWarn1min="Engine will die in 1 minute";
//// cEngBomb="Engine died, please try to go back to airport";
//// //flightlog
//// l1="Time/s,";
//// l2="Throttle/%,";
//// l3="IAS/kph,";
//// l4="TAS/kph,";
//// l5="Mach Number/Ma,";
//// l6="Altitude/m,";
//// l7="Temperature/℃,";
//// l8="Oil Temp/℃,";
//// l9="Vario/m/s,";
//// l10="SEP*/m/s,";
//// l11="Load/G,";
//// l12="Roll Rate/deg/s,";
//// l13="Power/bhp,";
//// l14="Efficiency/%,";
//// l15="EffectivePower*/bhp,";
//// l16="RPM/rpm,";
//// l17="Thrust/kg,";
//// l18="Acceleration*/m/s^2,";
//// l19="RPM throttle/%,";
//// l20="PropPitch/deg,";
//// l21="Radiator/%,";
//// l22="Mixture/%,";
//// l23="Compressor/stage,";
//// l24="Magneto/stage,";
//// l25="ManifoldPressure/ata,";
//// l26="Flaps/%,";
//// l27="Elevator/%,";
//// l28="Aileron/%,";
//// l29="Rudder/%,";
//// l30="AoA α/deg,";
//// l31="AoS β/deg,";
//// lfailCreate="Fail to create logging file";
//// lfailWrite="Fail to write logging file";
////
//// //FlightAnalyzer
//// fA1="Altitude ";
//// fA2="m, spends ";
//// fA3="seconds, average climb rate ";
//// fA4="m/s, analysis success";
// }
