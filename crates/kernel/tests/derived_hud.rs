//! HUD 派生黑盒场景 (kernel::derived): HUDData/Builder 值对象 + GaugeMarker
//! 分档与 CoW + hud_calculator 典型帧求值 (手造 State/Indicators/FormulaView
//! 桩, 无 FM 降级路径) 与 I_INVALID 哨兵降级。
#![allow(non_snake_case)] // 测试名 = 模块前缀英文 + 中文场景 (风格约定, 豁免蛇形)

use std::borrow::Cow;
use std::collections::HashMap;

use expect_test::expect;
use kernel::base::event::event_payload::EventPayload;
use kernel::config::config_api::hud_settings::HUDSettings;
use kernel::config::config_api::overlay_settings::OverlaySettings;
use kernel::derived::hud_calculator::{calculate, get_string_width, HudColors};
use kernel::derived::hud_data::{Builder, GaugeMarker, HUDData, MarkerType};
use kernel::formula::registry::FormulaView;
use kernel::game_api::parser::{F_INVALID, Indicators, State};

// ---- 桩: FormulaView (变量/公式槽表) ----

struct View(HashMap<&'static str, f64>);

impl FormulaView for View {
    fn var_value(&self, name: &str) -> Option<f64> {
        self.0.get(name).copied()
    }
    fn get_formula_value(&self, name: &str) -> Option<f64> {
        match name {
            "energy_m" => Some(2444.0),
            "warn_vne" => Some(1.0),
            "warn_stall" => Some(1.0),
            "warn_altitude" => Some(0.0),
            "maneuver_index" => Some(0.37),
            _ => None,
        }
    }
}

fn typical_view() -> View {
    View(HashMap::from([
        ("ias", 474.0),
        ("mach", 0.62),
        ("altitude", 1200.4),
        ("radio_altitude", 0.0),
        ("sep", 15.3),
        ("compass", 271.9),
        ("flap_allow_angle", 60.0),
        ("radio_altitude_valid", 0.0),
        ("speed_limit_ratio", 0.4),
        ("aileron_lock_ratio", 0.1),
        ("rudder_lock_ratio", 0.2),
        ("unit_mach_limit_ratio", 0.3),
        ("stall_speed", 144.0),
        ("wing_sweep_valid", 0.0),
        ("wing_sweep", 0.0),
    ]))
}

// ---- 桩: HUDSettings (全开关走缺省关/关键项可控) ----

struct TestHud {
    mach_mode: bool,
    flap_bar: bool,
}

impl OverlaySettings for TestHud {
    type GroupConfig = ();
    fn get_font_name(&self) -> String {
        String::new()
    }
    fn get_num_font_name(&self) -> String {
        String::new()
    }
    fn get_font_size_add(&self) -> i32 {
        0
    }
    fn get_bool(&self, _k: &str, def: bool) -> bool {
        def
    }
    fn get_int(&self, _k: &str, def: i32) -> i32 {
        def
    }
    fn get_string(&self, _k: &str, def: &str) -> String {
        def.to_string()
    }
    fn get_group_config(&self) -> Option<&Self::GroupConfig> {
        None
    }
    fn auto_hide_on_focus_loss(&self) -> bool {
        false
    }
}

impl HUDSettings for TestHud {
    fn get_num_font(&self) -> String {
        String::new()
    }
    fn get_crosshair_scale(&self) -> i32 {
        100
    }
    fn get_crosshair_name(&self) -> String {
        String::new()
    }
    fn is_display_crosshair(&self) -> bool {
        true
    }
    fn use_texture_crosshair(&self) -> bool {
        false
    }
    fn draw_hud_text(&self) -> bool {
        true
    }
    fn show_attitude_gauge(&self) -> bool {
        true
    }
    fn get_aoa_warning_ratio(&self) -> f64 {
        0.85
    }
    fn get_aoa_bar_warning_ratio(&self) -> f64 {
        0.6
    }
    fn enable_flap_angle_bar(&self) -> bool {
        self.flap_bar
    }
    fn show_speed_bar(&self) -> bool {
        true
    }
    fn draw_hud_mach(&self) -> bool {
        self.mach_mode
    }
    fn is_speed_label_disabled(&self) -> bool {
        false
    }
    fn is_altitude_label_disabled(&self) -> bool {
        false
    }
    fn is_sep_label_disabled(&self) -> bool {
        false
    }
    fn show_hud_speed(&self) -> bool {
        true
    }
    fn show_hud_aoa(&self) -> bool {
        true
    }
    fn show_hud_altitude(&self) -> bool {
        true
    }
    fn show_hud_energy(&self) -> bool {
        true
    }
    fn show_hud_mechanization(&self) -> bool {
        true
    }
    fn show_hud_flaps(&self) -> bool {
        true
    }
    fn show_hud_airbrake(&self) -> bool {
        true
    }
    fn show_hud_gear(&self) -> bool {
        true
    }
    fn show_hud_sep(&self) -> bool {
        true
    }
    fn show_hud_g_load(&self) -> bool {
        true
    }
    fn show_hud_maneuver_bar(&self) -> bool {
        true
    }
    fn is_attitude_indicator_inertial_mode(&self) -> bool {
        false
    }
    fn is_gpu_compatibility_mode(&self) -> bool {
        false
    }
    fn always_show_radar_altitude(&self) -> bool {
        false
    }
}

// ---- HUDData 值对象 ----

/// Builder 灌典型值 → build 字段快照 (Java Builder 公有字段直赋语义)
#[test]
fn hud_data_Builder灌典型值快照() {
    let b = Builder {
        ias: 474.0,
        mach: 0.62,
        altitude: 1200.0,
        radio_altitude: 0.0,
        vertical_speed: -3.5,
        heading: 271.9,
        pitch: -5.0,
        roll: 10.0,
        slip: -1.5,
        aoa: 12.5,
        pitch_valid: true,
        throttle: 105,
        flaps: 50.0,
        gear: 100.0,
        airbrake: 30.0,
        flap_allow_angle: 60.0,
        energy_m: 2444.0,
        g_load: 2.0,
        turn_rate: 8.5,
        maneuver_index: 0.37,
        is_mach_mode: false,
        is_gear_down: true,
        is_flaps_down: true,
        is_airbrake_active: true,
        warn_vne: true,
        warn_stall: false,
        warn_gear: false,
        warn_altitude: false,
        aoa_color: [1, 2, 3, 4],
        aoa_bar_color: [5, 6, 7, 8],
        throttle_color: [255, 0, 0, 255],
        map_grid: "C4".into(),
        time_str: "12:34".into(),
        speed_str: "SPD   474".into(),
        alt_str: "ALT  1200".into(),
        aoa_str: "α 13".into(),
        energy_str: "E 2444".into(),
        mechanization_str: "F 50BRKGEA".into(),
        flaps_wing_str: "F 50".into(),
        airbrake_str: "BRK".into(),
        gear_str: "GEA".into(),
        sep_str: "SEP↓3.5".into(),
        maneuver_state_str: "G  2.0".into(),
        warn_configuration: true,
        aoa_ratio: 0.42,
        speed_bar_speed_ratio: 0.4,
        speed_bar_stall_ratio: 0.12,
        speed_bar_unit_mach_limit_ratio: 0.3,
        speed_bar_aileron_lock_ratio: 0.1,
        speed_bar_rudder_lock_ratio: 0.2,
    };
    let d = b.build();
    let actual = format!(
        "ias={:.1} mach={} alt={:.0} vs={:.1} hdg={:.1} pitch={:.1} roll={:.1} slip={:.1} aoa={:.1}\n\
         thr={} flaps={} gear={} brk={} e_m={:.0} g={} mi={}\n\
         flags={}{}{}{} warn={}{}{}{}\n\
         grid={} time={} spd={:?} alt_s={:?} aoa_s={:?} e_s={:?}\n\
         flap_w={:?} brk_s={:?} gear_s={:?} sep_s={:?} ms={:?}\n\
         aoa_ratio={} bar=({:.2}/{:.2}/{:.2}/{:.2}/{:.2})",
        d.ias, d.mach, d.altitude, d.vertical_speed, d.heading, d.pitch, d.roll, d.slip, d.aoa,
        d.throttle, d.flaps, d.gear, d.airbrake, d.energy_m, d.g_load, d.maneuver_index,
        d.is_mach_mode as u8, d.is_gear_down as u8, d.is_flaps_down as u8, d.is_airbrake_active as u8,
        d.warn_vne as u8, d.warn_stall as u8, d.warn_gear as u8, d.warn_altitude as u8,
        d.map_grid, d.time_str, d.speed_str, d.alt_str, d.aoa_str, d.energy_str,
        d.flaps_wing_str, d.airbrake_str, d.gear_str, d.sep_str, d.maneuver_state_str,
        d.aoa_ratio, d.speed_bar_speed_ratio, d.speed_bar_stall_ratio,
        d.speed_bar_unit_mach_limit_ratio, d.speed_bar_aileron_lock_ratio, d.speed_bar_rudder_lock_ratio,
    );
    expect![[r#"
        ias=474.0 mach=0.62 alt=1200 vs=-3.5 hdg=271.9 pitch=-5.0 roll=10.0 slip=-1.5 aoa=12.5
        thr=105 flaps=50 gear=100 brk=30 e_m=2444 g=2 mi=0.37
        flags=0111 warn=1000
        grid=C4 time=12:34 spd="SPD   474" alt_s="ALT  1200" aoa_s="α 13" e_s="E 2444"
        flap_w="F 50" brk_s="BRK" gear_s="GEA" sep_s="SEP↓3.5" ms="G  2.0"
        aoa_ratio=0.42 bar=(0.40/0.12/0.30/0.10/0.20)"#]]
    .assert_eq(&actual);

    // Builder 非消费式: 可重复 build
    assert_eq!(b.build(), d);
}

/// HUDData::empty = Builder 默认值 (数值 0 / 色 GREEN / 串空)
#[test]
fn hud_data_empty与默认色() {
    let e = HUDData::empty();
    assert_eq!(e, Builder::default().build());
    let actual = format!(
        "zeros: {}|{}|{}|{}\ncolors: {:?}|{:?}|{:?}\nstrs: {:?}|{:?}|{:?}",
        e.ias, e.throttle, e.aoa, e.pitch_valid,
        e.aoa_color, e.aoa_bar_color, e.throttle_color,
        e.speed_str, e.map_grid, e.mechanization_str,
    );
    expect![[r#"
        zeros: 0|0|0|false
        colors: [0, 255, 0, 255]|[0, 255, 0, 255]|[0, 255, 0, 255]
        strs: ""|""|"""#]]
    .assert_eq(&actual);
}

// ---- GaugeMarker ----

/// 仪表标记: 默认隐藏 (-1) / is_visible 分档 / with_ratio 的 CoW 语义
#[test]
fn gauge_marker_分档与Cow更新() {
    // Builder 默认: ratio=-1 (隐藏), RED, LINE_FULL
    let m0 = GaugeMarker::builder().id("vne".into()).build();
    assert!(!m0.is_visible(), "默认 ratio=-1 越界隐藏");
    assert_eq!(m0.ratio, -1.0);
    assert_eq!(m0.color, [255, 0, 0, 255], "Builder 默认 Color.RED");

    // 分档: [0,1] 可见, 越界隐藏
    let vis: Vec<bool> = [-0.001, 0.0, 0.5, 1.0, 1.001]
        .iter()
        .map(|&r| GaugeMarker::builder().ratio(r).build().is_visible())
        .collect();
    assert_eq!(vis, vec![false, true, true, true, false]);

    // with_ratio CoW: 同值 (容差 0.0001 内) 借用, 异值克隆
    let m = GaugeMarker::builder()
        .id("stall".into())
        .r#type(MarkerType::LinePartial)
        .ratio(0.25)
        .width_ratio(0.8)
        .side(-1)
        .label("ST".into())
        .build();
    let same = m.with_ratio(0.25005); // 容差内
    assert!(matches!(same, Cow::Borrowed(_)), "容差内返回自身 (零分配)");
    let moved = m.with_ratio(0.5);
    let Cow::Owned(m2) = moved else { panic!("异值应克隆") };
    assert_eq!(m2.ratio, 0.5);
    assert_eq!(m2.id, "stall");
    assert_eq!(m2.r#type, MarkerType::LinePartial);
    assert_eq!(m2.width_ratio, 0.8);
    assert_eq!(m2.side, -1);
    assert_eq!(m2.label, "ST");
    assert_eq!(m.ratio, 0.25, "原标记不受影响");
}

// ---- hud_calculator ----

/// 典型帧求值 (无 FM): 姿态/显示串族/速度比例条全链路 (expect 快照)
#[test]
fn hud_calculator_典型帧无FM求值() {
    let mut st = State::new();
    st.aos = -1.5;
    st.throttle = 105; // > 100 → throttle_color = RED
    st.flaps = 50;
    st.gear = 100;
    st.airbrake = 0;
    st.aoa = 12.5;
    st.ny = 2.0;

    let mut ind = Indicators::new();
    ind.aviahorizon_pitch = 5.0; // pitch = -5
    ind.aviahorizon_roll = -10.0; // roll = 10

    let view = typical_view();
    let settings = TestHud { mach_mode: false, flap_bar: false };
    let payload = EventPayload::builder()
        .map_grid("C4".into())
        .time_str("--:--".into())
        .build();

    let d = calculate(
        Some(&st),
        Some(&ind),
        &payload,
        Some(&view),
        None, // 无 FM: aoa 色走降级分支, mach 类告警线不接管
        &settings,
        &HudColors::application_defaults(),
    );

    let actual = format!(
        "ias={} mach={} alt={:.1} vs={:.1} hdg={:.1} pitch={:.1} valid={} roll={:.1} slip={:.1} aoa={:.1}\n\
         thr={} flaps={} gear={} brk={} flap_allow={}\n\
         e_m={:.0} g={} mi={} aoa_ratio={:.4}\n\
         flags: mach={} gear={} flaps={} brk={}\n\
         warn: vne={} stall={} gear={} alt={} cfg={}\n\
         colors: aoa={:?} bar={:?} thr={:?}\n\
         grid={} spd={:?} alt_s={:?} aoa_s={:?} e_s={:?}\n\
         sep_s={:?} ms={:?} flap_w={:?} brk_s={:?} gear_s={:?}\n\
         bar: speed={:.3} stall={:.4} unit_mach={} aileron={} rudder={}",
        d.ias, d.mach, d.altitude, d.vertical_speed, d.heading, d.pitch, d.pitch_valid, d.roll, d.slip, d.aoa,
        d.throttle, d.flaps, d.gear, d.airbrake, d.flap_allow_angle,
        d.energy_m, d.g_load, d.maneuver_index, d.aoa_ratio,
        d.is_mach_mode as u8, d.is_gear_down as u8, d.is_flaps_down as u8, d.is_airbrake_active as u8,
        d.warn_vne as u8, d.warn_stall as u8, d.warn_gear as u8, d.warn_altitude as u8, d.warn_configuration as u8,
        d.aoa_color, d.aoa_bar_color, d.throttle_color,
        d.map_grid, d.speed_str, d.alt_str, d.aoa_str, d.energy_str,
        d.sep_str, d.maneuver_state_str, d.flaps_wing_str, d.airbrake_str, d.gear_str,
        d.speed_bar_speed_ratio, d.speed_bar_stall_ratio, d.speed_bar_unit_mach_limit_ratio,
        d.speed_bar_aileron_lock_ratio, d.speed_bar_rudder_lock_ratio,
    );
    expect![[r#"
        ias=474 mach=0.62 alt=1200.4 vs=15.3 hdg=271.9 pitch=-5.0 valid=true roll=10.0 slip=-1.5 aoa=12.5
        thr=105 flaps=50 gear=100 brk=0 flap_allow=60
        e_m=2444 g=2 mi=0 aoa_ratio=0.4167
        flags: mach=0 gear=1 flaps=1 brk=0
        warn: vne=1 stall=0 gear=0 alt=0 cfg=0
        colors: aoa=[27, 255, 128, 240] bar=[27, 255, 128, 240] thr=[255, 0, 0, 255]
        grid=C4 spd="SPD   474" alt_s="ALT  1200" aoa_s="α 12" e_s="E 2444"
        sep_s="SEP↑15  " ms="G  2.0" flap_w="F 50" brk_s="" gear_s="GEA"
        bar: speed=0.400 stall=0.1215 unit_mach=0.3 aileron=0.1 rudder=0.2"#]]
    .assert_eq(&actual);
}

/// I_INVALID 哨兵降级: 姿态仪表缺席 → pitch_valid=false 归零;
/// flaps 正/负哨兵 → 0; aos 哨兵 → slip 保持 0 (缺失 ≠ 0 的显示面收敛)
#[test]
fn hud_calculator_哨兵值降级() {
    let mut st = State::new();
    st.aos = F_INVALID; // 侧滑缺失
    st.flaps = 65535; // 正哨兵 (Java AIOOBE 域产物)
    st.gear = 0;
    st.aoa = 0.0;
    st.throttle = 60;

    let mut ind = Indicators::new();
    ind.aviahorizon_pitch = F_INVALID; // 姿态缺席
    ind.aviahorizon_roll = F_INVALID;

    let view = typical_view();
    let settings = TestHud { mach_mode: false, flap_bar: false };
    let payload = EventPayload::builder().build();

    let d = calculate(
        Some(&st),
        Some(&ind),
        &payload,
        Some(&view),
        None,
        &settings,
        &HudColors::application_defaults(),
    );
    assert!(!d.pitch_valid, "aviahorizon_pitch = F_INVALID → pitch 无效");
    assert_eq!(d.pitch, 0.0);
    assert_eq!(d.roll, 0.0);
    assert_eq!(d.slip, 0.0, "aos 哨兵 → slip 不接管");
    assert_eq!(d.flaps, 0.0, "flaps 正哨兵 65535 → 0");
    assert!(!d.is_flaps_down);
    assert_eq!(d.throttle_color, [255, 255, 255, 255], "throttle ≤ 100 → WHITE");
    // State 缺席 → 早退空帧 (Java null 守卫)
    let empty = calculate(None, None, &payload, None, None, &settings, &HudColors::application_defaults());
    assert_eq!(empty, HUDData::empty(), "state/source 缺席 → Builder 默认空帧");
}

/// mach 显示模式 + get_string_width 三重守卫
#[test]
fn hud_calculator_mach模式与字符串宽度() {
    let st = State::new();
    let view = typical_view();
    let payload = EventPayload::builder().build();

    // mach 模式: speed_str 走 "M{mach}" 形态
    let mach_mode = TestHud { mach_mode: true, flap_bar: false };
    let d = calculate(
        Some(&st),
        None,
        &payload,
        Some(&view),
        None,
        &mach_mode,
        &HudColors::application_defaults(),
    );
    expect!["M 0.62"].assert_eq(&d.speed_str);
    assert!(d.is_mach_mode);

    // get_string_width: text None / 空 / font None 三重早退 → 0
    let font = 42;
    assert_eq!(get_string_width(None, Some(&font), |_, _| 99), 0);
    assert_eq!(get_string_width(Some(""), Some(&font), |_, _| 99), 0);
    assert_eq!(get_string_width::<i32>(Some("abc"), None, |_, _| 99), 0);
    // 度量闭包注入 (离屏 FontMetrics 的接替面)
    assert_eq!(get_string_width(Some("ab"), Some(&font), |_, s| s.len() as i32 * 10), 20);
}
