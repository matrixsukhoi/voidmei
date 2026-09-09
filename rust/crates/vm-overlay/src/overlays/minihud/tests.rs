use super::*;
use vm_core::base::event::event_payload::EventPayload;
use vm_core::base::format::fmt_f;
use vm_core::config::config_api::OverlaySettings;
use vm_core::derived::hud_data::{Builder, HUDData};
use vm_core::game_api::parser::State;

use crate::overlays::bars::{FlapAngleBar, LinearGauge};
use crate::overlays::rows::{
    AltitudeReadout, AoaGauge, EnergyReadout, GLoadReadout, ManeuverBar, MechPart, SepReadout,
    SpeedReadout,
};
use crate::render::canvas::PixCanvas;

const FONTS: &str = "../../../fonts";

fn font_path() -> PathBuf {
    Path::new(FONTS).join("sarasa-mono-sc-bold.ttf")
}

/// 出厂页 (factory_default.json 的 minihud-default; init/reinit 的 doc 实参)
fn doc() -> vm_core::config::json_model::PageDoc {
    vm_core::config::json_store::factory()
        .pages
        .iter()
        .find(|d| d.id == "minihud-default")
        .cloned()
        .expect("出厂页 minihud-default 应存在")
}

// ===== 测试设置 (cfg :default 快照, FakeHud 同款形态) =====

struct GroupStub;

/// 可变测试设置: 与 ui_layout.cfg 的 MiniHUD panel :default 同值
#[derive(Clone)]
struct TestSettings {
    crosshair_scale: i32,
    font_size_add: i32,
    display_crosshair: bool,
    draw_hud_text: bool,
    show_attitude: bool,
    enable_flap_bar: bool,
    show_speed_bar: bool,
    draw_hud_mach: bool,
    speed_label_disabled: bool,
    altitude_label_disabled: bool,
    sep_label_disabled: bool,
    radar_alt: bool,
    show_speed: bool,
    show_aoa: bool,
    show_alt: bool,
    show_energy: bool,
    show_flaps: bool,
    show_brk: bool,
    show_gear: bool,
    show_sep: bool,
    show_g_load: bool,
    show_maneuver: bool,
    layout_debug: bool,
}

impl Default for TestSettings {
    fn default() -> Self {
        TestSettings {
            crosshair_scale: 113, // cfg :value (crosshairScale)
            font_size_add: 0,
            display_crosshair: true,
            draw_hud_text: true,
            show_attitude: true,
            enable_flap_bar: true,
            show_speed_bar: true,
            draw_hud_mach: true,
            speed_label_disabled: false,
            altitude_label_disabled: false,
            sep_label_disabled: false,
            radar_alt: false,
            show_speed: true,
            show_aoa: true,
            show_alt: true,
            show_energy: true,
            show_flaps: true,
            show_brk: true,
            show_gear: true,
            show_sep: true,
            show_g_load: true,
            show_maneuver: true,
            layout_debug: false,
        }
    }
}

impl OverlaySettings for TestSettings {
    type GroupConfig = GroupStub;
    fn get_font_name(&self) -> String {
        "text".into()
    }
    fn get_num_font_name(&self) -> String {
        "num".into()
    }
    fn get_font_size_add(&self) -> i32 {
        self.font_size_add
    }
    fn get_bool(&self, key: &str, def: bool) -> bool {
        if key == "enableLayoutDebug" {
            return self.layout_debug;
        }
        def
    }
    fn get_int(&self, _k: &str, def: i32) -> i32 {
        def
    }
    fn get_string(&self, _k: &str, def: &str) -> String {
        def.to_string()
    }
    fn get_group_config(&self) -> Option<&GroupStub> {
        None
    }
    fn auto_hide_on_focus_loss(&self) -> bool {
        false
    }
}

impl HUDSettings for TestSettings {
    fn get_num_font(&self) -> String {
        "Sarasa Mono SC".into()
    }
    fn get_crosshair_scale(&self) -> i32 {
        self.crosshair_scale
    }
    fn get_crosshair_name(&self) -> String {
        "软件渲染准星".into()
    }
    fn is_display_crosshair(&self) -> bool {
        self.display_crosshair
    }
    fn use_texture_crosshair(&self) -> bool {
        false
    }
    fn draw_hud_text(&self) -> bool {
        self.draw_hud_text
    }
    fn show_attitude_gauge(&self) -> bool {
        self.show_attitude
    }
    fn get_aoa_warning_ratio(&self) -> f64 {
        0.2
    }
    fn get_aoa_bar_warning_ratio(&self) -> f64 {
        0.25
    }
    fn enable_flap_angle_bar(&self) -> bool {
        self.enable_flap_bar
    }
    fn show_speed_bar(&self) -> bool {
        self.show_speed_bar
    }
    fn draw_hud_mach(&self) -> bool {
        self.draw_hud_mach
    }
    fn is_speed_label_disabled(&self) -> bool {
        self.speed_label_disabled
    }
    fn is_altitude_label_disabled(&self) -> bool {
        self.altitude_label_disabled
    }
    fn is_sep_label_disabled(&self) -> bool {
        self.sep_label_disabled
    }
    fn show_hud_speed(&self) -> bool {
        self.show_speed
    }
    fn show_hud_aoa(&self) -> bool {
        self.show_aoa
    }
    fn show_hud_altitude(&self) -> bool {
        self.show_alt
    }
    fn show_hud_energy(&self) -> bool {
        self.show_energy
    }
    fn show_hud_mechanization(&self) -> bool {
        false
    }
    fn show_hud_flaps(&self) -> bool {
        self.show_flaps
    }
    fn show_hud_airbrake(&self) -> bool {
        self.show_brk
    }
    fn show_hud_gear(&self) -> bool {
        self.show_gear
    }
    fn show_hud_sep(&self) -> bool {
        self.show_sep
    }
    fn show_hud_g_load(&self) -> bool {
        self.show_g_load
    }
    fn show_hud_maneuver_bar(&self) -> bool {
        self.show_maneuver
    }
    fn is_attitude_indicator_inertial_mode(&self) -> bool {
        false
    }
    fn is_gpu_compatibility_mode(&self) -> bool {
        false
    }
    fn always_show_radar_altitude(&self) -> bool {
        self.radar_alt
    }
}

fn overlay() -> MiniHudOverlay {
    MiniHudOverlay::init(
        false,
        100,
        &TestSettings::default(),
        1.0,
        &font_path(),
        &doc(),
    )
    .unwrap()
}

// ===== java_f / pad_width 基线 =====

/// 历史基线: String.format 的 %f HALF_UP 与宽度填充
#[test]
fn java_f_oracle() {
    assert_eq!(fmt_f(0.85, 2), "0.85");
    assert_eq!(fmt_f(20.0, 0), "20");
    assert_eq!(
        fmt_f(2.675, 2),
        "2.67",
        "波21: 精确二进制值 nearest-even (实值 2.67499...)"
    );
    assert_eq!(fmt_f(-0.04, 1), "-0.0", "舍到零的负数保负号");
    assert_eq!(pad_width("0.85".into(), 5, false), " 0.85");
    assert_eq!(pad_width("360".into(), 5, false), "  360");
    assert_eq!(pad_width("30".into(), 4, true), "30  ");
    assert_eq!(fmt_d(7, 3), "  7");
    assert_eq!(fmt_d(110, 3), "110");
}

// ===== MinimalHUDContext 基线 =====

/// crossScale=113, dpi=1.0 全链手算 (MinimalHUDContext.java:96-153 逐行)。
#[test]
fn ctx_metrics_match_java_math() {
    let s = TestSettings::default();
    let ctx = MinimalHudContext::create(&s, 1.0, &font_path()).unwrap();
    assert_eq!(ctx.cross_scale, 113); // round(113*1.0)
    assert_eq!(ctx.hud_font_size, 113 / 4); // 28
    assert_eq!(ctx.bar_width, 28 / 4); // 7
    assert_eq!(ctx.line_width, 28 / 10); // 2 (非零分支)
    assert_eq!(ctx.width, (113.0 * 2.25) as i32); // 254 (crosshair on)
    assert_eq!(ctx.height, (113.0 * 1.5) as i32 + (28.0 * 3.5) as i32); // 169+98=267
    assert_eq!(ctx.cross_x, 127); // 254/2
    assert_eq!(ctx.cross_y, 133); // 267/2 (int 除截断)
    assert_eq!(ctx.round_compass, 22); // Math.round(28*0.8f)=round(22.4)
                                       // 标签全开 → 5.5f: (int)(28*5.5f)=154
    assert_eq!(ctx.right_draw, 154);
    assert_eq!(ctx.compass_diameter, 35); // round(2*28*0.618)=round(34.608)
    assert_eq!(ctx.compass_radius, 18); // round(35/2.0)=round(17.5)=18 (§2.3)
    assert_eq!(ctx.compass_inner_mark_radius, 22); // round(0.618*35)
    assert!((ctx.aoa_length - (154.0 - 28.0 / 1.5)).abs() < 1e-9); // 135.33..
    assert_eq!(ctx.half_line, 1); // round(2/2.0f)=1
    assert_eq!(ctx.stroke_thick_w, 3.0); // halfLine+2
    assert_eq!(ctx.stroke_thin_w, 1.0);
    assert_eq!(ctx.hud_font_size_small, (28.0f32 * 0.75) as i32); // 21
    assert_eq!(ctx.fonts.draw.size, 28);
    assert_eq!(ctx.fonts.small.size, 21);
    assert_eq!(ctx.fonts.s_small.size, 14); // 28/2 int 除
}

/// dpi=2.0 与标签全关 (multiplier 3.5f) / 无准星分支
#[test]
fn ctx_metrics_dpi_and_branches() {
    let s = TestSettings::default();
    let ctx = MinimalHudContext::create(&s, 2.0, &font_path()).unwrap();
    assert_eq!(ctx.cross_scale, 226); // round(113*2.0)
    assert_eq!(ctx.hud_font_size, 226 / 4); // 56
    assert_eq!(ctx.line_width, 5); // 56/10
    assert_eq!(ctx.half_line, 3); // round(5/2.0f)=round(2.5)=3 (§2.3)
    assert_eq!(ctx.width, (226.0 * 2.25) as i32); // 508
    assert_eq!(ctx.height, (226.0 * 1.5) as i32 + (56.0 * 3.5) as i32); // 339+196=535
    assert_eq!(ctx.round_compass, 45); // round(56*0.8f)=round(44.8)
    assert_eq!(ctx.right_draw, (56.0f32 * 5.5) as i32); // 308

    // 标签全关 → 3.5f
    let mut s2 = TestSettings::default();
    s2.speed_label_disabled = true;
    s2.altitude_label_disabled = true;
    s2.sep_label_disabled = true;
    let ctx2 = MinimalHudContext::create(&s2, 1.0, &font_path()).unwrap();
    assert_eq!(ctx2.right_draw, (28.0f32 * 3.5) as i32); // 98

    // 无准星: width = (int)(113*2.25) - 28 = 226
    let mut s3 = TestSettings::default();
    s3.display_crosshair = false;
    let ctx3 = MinimalHudContext::create(&s3, 1.0, &font_path()).unwrap();
    assert_eq!(ctx3.width, 226);
}

/// hudFontSize 下限钳 8 (crossScale 极小; MinimalHUDContext.java:104-105)
#[test]
fn ctx_min_font_size_clamp() {
    let mut s = TestSettings::default();
    s.crosshair_scale = 4;
    let ctx = MinimalHudContext::create(&s, 1.0, &font_path()).unwrap();
    assert_eq!(ctx.hud_font_size, 8, "4/4+0=1 → 钳 8");
    assert_eq!(ctx.line_width, 1, "8/10=0 → 1 分支");
    assert_eq!(ctx.bar_width, 2);
}

// ===== refreshTemplates 预览串 基线 =====

#[test]
fn refresh_templates_preview_strings() {
    let o = overlay();
    assert_eq!(o.lines[0], "M 0.85", "drawHudMach: M + %5.2f(0.85)");
    assert_eq!(o.lines[1], "ALT  1024", "标签开: ALT + %6s(1024)");
    assert_eq!(
        o.lines[2], "    BRKGEAR",
        "襟翼条启用 → 4 空格 + BRK + GEAR"
    );
    // ↑ 符号行: SEP 标签开 + "↑%-4s"("30") — ↑ 为格式串字面量前缀
    assert!(o.lines[3].starts_with("SEP↑30"), "lines[3]={}", o.lines[3]);
    assert_eq!(o.lines[3], "SEP↑30  ");
    assert_eq!(o.lines[4], "G  2.0");
    assert_eq!(o.line_aoa, "α 20");
    assert_eq!(o.rel_energy, "E114514");
    assert_eq!(o.throttley, 100);
    assert_eq!(o.aoa_y, 10);
    assert_eq!(o.throttle_color, colors().shade_shape);
    assert_eq!(o.aoa_color, colors().num);
    assert_eq!(o.aoa_bar_color, colors().num);

    // 变体: mach 关 + 标签关 + 雷达开 + 襟翼条关
    let mut s = TestSettings::default();
    s.draw_hud_mach = false;
    s.speed_label_disabled = true;
    s.altitude_label_disabled = true;
    s.sep_label_disabled = true;
    s.radar_alt = true;
    s.enable_flap_bar = false;
    let o2 = MiniHudOverlay::init(false, 100, &s, 1.0, &font_path(), &doc()).unwrap();
    assert_eq!(o2.lines[0], "  360", "%5s 无前缀");
    assert_eq!(o2.lines[1], "R 1024", "雷达: R + %5s");
    assert_eq!(o2.lines[2], "F100BRKGEAR");
    assert_eq!(o2.lines[3], "↑30  ", "SEP 标签关 (↑ 仍为格式串字面量前缀)");
}

// ===== 组件清单与布局 =====

/// 出厂页建树: 16 组件 cell 全在 (displayCrosshair=true) + 引擎节点集
/// (原 initComponentsLayout 分发序断言 — W2 后分发面 = cells 表;
/// 行族原子化后 5 复合行 → 10 原子件)
#[test]
fn components_order_and_nodes() {
    let o = overlay();
    let ids = [
        "aoa", "speed", "energy", "altitude", "flaps", "airbrake", "gear", "sep", "gload",
        "maneuverbar", "flap", "attitude", "compass", "speedBar", "throttle", "crosshair",
    ];
    assert_eq!(o.cells.len(), 16);
    for id in ids {
        assert!(o.cells.contains_key(id), "cell {id} 应存在");
        assert!(o.layout.engine.get_node(id).is_some(), "节点 {id} 应存在");
    }
    // displayCrosshair=false: visibleWhen 不满足 → crosshair 节点和 cell 都不建
    let mut s = TestSettings::default();
    s.display_crosshair = false;
    let o2 = MiniHudOverlay::init(false, 100, &s, 1.0, &font_path(), &doc()).unwrap();
    assert_eq!(o2.cells.len(), 15, "W2: 建树门控 → crosshair cell 不建");
    assert!(!o2.cells.contains_key("crosshair"), "crosshair cell 不建");
    assert!(o2.layout.engine.get_node("crosshair").is_none());
}

/// updateComponents 可见性开关族 (Java L309-373)
#[test]
fn visibility_switches_from_settings() {
    let mut s = TestSettings::default();
    s.show_speed_bar = false; // 油门条/速度条互斥 (手册 §9.1)
    let o = MiniHudOverlay::init(false, 100, &s, 1.0, &font_path(), &doc()).unwrap();
    assert!(o.cells.get("throttle").unwrap().is_visible());
    assert!(!o.cells.get("speedBar").unwrap().is_visible());

    let mut s2 = TestSettings::default();
    s2.show_attitude = false; // 罗盘/姿态互斥 (Java L316-322)
    let o2 = MiniHudOverlay::init(false, 100, &s2, 1.0, &font_path(), &doc()).unwrap();
    assert!(o2.cells.get("compass").unwrap().is_visible());
    assert!(!o2.cells.get("attitude").unwrap().is_visible());

    let mut s3 = TestSettings::default();
    s3.draw_hud_text = false; // master 总闸
    let o3 = MiniHudOverlay::init(false, 100, &s3, 1.0, &font_path(), &doc()).unwrap();
    assert!(!o3.cells.get("flap").unwrap().is_visible());
    assert!(!o3.cells.get("speed").unwrap().is_visible());
    assert!(!o3.cells.get("gload").unwrap().is_visible());
    assert!(
        o3.cells.get("crosshair").unwrap().is_visible(),
        "准星不受 drawHUDtext 管 (L323-324)"
    );

    // 行级独立开关 (原子化后 show* 键直控各原子件外壳): 只关速度 (L342-346)
    let mut s4 = TestSettings::default();
    s4.show_speed = false;
    let o4 = MiniHudOverlay::init(false, 100, &s4, 1.0, &font_path(), &doc()).unwrap();
    assert!(!o4.cells.get("speed").unwrap().is_visible(), "showHUDSpeed=false");
    assert!(o4.cells.get("aoa").unwrap().is_visible(), "AoA 独立可见");

    // 三段全关 → 三原子件全隐; 单开襟翼 → 仅襟翼可见 (Java L360-362)
    let mut s5 = TestSettings::default();
    s5.show_flaps = false;
    s5.show_brk = false;
    s5.show_gear = false;
    let o5 = MiniHudOverlay::init(false, 100, &s5, 1.0, &font_path(), &doc()).unwrap();
    for id in ["flaps", "airbrake", "gear"] {
        assert!(!o5.cells.get(id).unwrap().is_visible(), "{id} 应隐藏");
    }

    let mut s6 = TestSettings::default();
    s6.show_brk = false;
    s6.show_gear = false;
    let o6 = MiniHudOverlay::init(false, 100, &s6, 1.0, &font_path(), &doc()).unwrap();
    let vis6 = |o: &MiniHudOverlay, id: &str| o.cells.get(id).unwrap().is_visible();
    assert!((vis6(&o6, "flaps"), vis6(&o6, "airbrake"), vis6(&o6, "gear")) == (true, false, false));
}

/// 预览模式 (init service_present=false) 行族吃 lines 预览串; 油门条 0
#[test]
fn preview_rows_fed_from_lines() {
    let o = overlay();
    let r0 = o
        .cells
        .get("speed")
        .unwrap()
        .downcast_ref::<SpeedReadout>()
        .unwrap();
    assert_eq!(r0.0.text, "M 0.85");
    let aoa = o
        .cells
        .get("aoa")
        .unwrap()
        .downcast_ref::<AoaGauge>()
        .unwrap();
    assert_eq!(aoa.aoa_text, "α 20");
    let r1 = o
        .cells
        .get("altitude")
        .unwrap()
        .downcast_ref::<AltitudeReadout>()
        .unwrap();
    assert_eq!(r1.0.text, "ALT  1024");
    let en = o
        .cells
        .get("energy")
        .unwrap()
        .downcast_ref::<EnergyReadout>()
        .unwrap();
    assert_eq!(en.energy_text, "E114514");
    // 行2 预览: 各段独立解析 lines[2] ("    BRKGEAR"; enableFlapAngleBar=true
    // → 襟翼段 4 空格 → 空数据 + F100 模板回退)
    let seg = |id: &str| o.cells.get(id).unwrap().downcast_ref::<MechPart>().unwrap();
    let flaps = seg("flaps");
    assert_eq!((flaps.text.as_str(), flaps.template.as_str()), ("", "F100"));
    let brk = seg("airbrake");
    assert_eq!((brk.text.as_str(), brk.template.as_str()), ("BRK", "BRK"));
    let gear = seg("gear");
    assert_eq!((gear.text.as_str(), gear.template.as_str()), ("GEA", "GEA"));
    let thr = o
        .cells
        .get("throttle")
        .unwrap()
        .downcast_ref::<LinearGauge>()
        .unwrap();
    assert_eq!(thr.display_value, "  0", "预览无 service → throttleValue=0, %3d");
}

// ===== 事件驱动更新 =====

fn sample_data() -> HUDData {
    let mut b = Builder::default();
    b.speed_str = "M0.72".into();
    b.warn_vne = true;
    b.aoa_str = "14".into();
    b.aoa_ratio = 0.55;
    b.aoa_color = [255, 0, 0, 255];
    b.aoa_bar_color = [255, 0, 0, 255];
    b.alt_str = "R 245".into();
    b.warn_altitude = true;
    b.energy_str = "E3200".into();
    b.mechanization_str = "F100BRKGEA".into();
    b.flaps_wing_str = "F100".into();
    b.airbrake_str = "BRK".into();
    b.gear_str = "GEA".into();
    b.warn_configuration = true;
    b.sep_str = " 12".into();
    b.maneuver_state_str = "G2.1".into();
    b.maneuver_index = 0.37;
    b.throttle = 87;
    b.throttle_color = [0, 255, 0, 255];
    b.map_grid = "C4".into();
    b.heading = 271.5;
    b.build()
}

/// W-B 事件瘦身后事件不再携带 HUDData (update_from_event 恒现场 calculate),
/// 手造 HUDData 的分发链以直调分发段 (组件 on_data_update + 全局状态 +
/// maneuver 会话量, 同 update_from_event 尾段) 覆盖, 断言原样保留
fn dispatch_data(o: &mut MiniHudOverlay, data: &HUDData, fatal: bool) {
    let env = UpdateEnv {
        data,
        frame: None,
        fmdata: None,
        payload: None,
        compressor_stages: None,
        now_ms: 0,
        maneuver_len: o.maneuver_index_len,
        maneuver_ticks: o.tick_scale,
        lang: None,
    };
    for cell in o.cells.values() {
        cell.on_data_update(&env);
    }
    o.warn_vne = data.warn_vne;
    o.warn_rh = data.warn_altitude;
    o.warning.set_blink_x(fatal);
    // maneuver 会话量 (update_from_event 尾段 — 更新在分发后, Row4 用上一帧 len)
    o.maneuver_index = data.maneuver_index;
    let right_draw = o.ctx.right_draw;
    o.maneuver_index_len = java_round_long_narrowed(
        data.maneuver_index / MANEUVER_FULL_SCALE * right_draw as f64,
    );
    o.tick_scale = TickScale {
        ticks: MANEUVER_TICK_STEPS
            .map(|step| java_round_long_narrowed(step / MANEUVER_FULL_SCALE * right_draw as f64)),
    };
}

/// updateFromEvent: HUDData 消费 + len 族 + 布尔状态 + blink (L433-468)
#[test]
fn update_from_event_dispatches() {
    let mut o = overlay();
    let data = sample_data();
    dispatch_data(&mut o, &data, true);

    let r0 = o
        .cells
        .get("speed")
        .unwrap()
        .downcast_ref::<SpeedReadout>()
        .unwrap();
    assert_eq!(r0.0.text, "M0.72");
    assert!(r0.0.is_warning, "warnVne → 主文字警告态");
    let aoa = o
        .cells
        .get("aoa")
        .unwrap()
        .downcast_ref::<AoaGauge>()
        .unwrap();
    assert_eq!(aoa.aoa_text, "14");
    // aoaY = (int)(0.55 × (int)aoaLength=135) = 74, 未达 rightDraw=154 钳制线
    assert_eq!(aoa.aoa_y, 74);

    let r1 = o
        .cells
        .get("altitude")
        .unwrap()
        .downcast_ref::<AltitudeReadout>()
        .unwrap();
    assert_eq!(r1.0.text, "R 245");
    assert!(o.warn_rh, "warnAltitude → warnRH");
    let en = o
        .cells
        .get("energy")
        .unwrap()
        .downcast_ref::<EnergyReadout>()
        .unwrap();
    assert_eq!(en.energy_text, "E3200");

    // 行2 三段直取 (原 HUDMechanizationRow.onDataUpdate Java:66-68)
    let mech = |id: &str| o.cells.get(id).unwrap().downcast_ref::<MechPart>().unwrap();
    assert_eq!(
        (
            mech("flaps").text.as_str(),
            mech("airbrake").text.as_str(),
            mech("gear").text.as_str()
        ),
        ("F100", "BRK", "GEA")
    );
    assert!(mech("gear").is_warning, "warnConfiguration");

    let sep = o
        .cells
        .get("sep")
        .unwrap()
        .downcast_ref::<SepReadout>()
        .unwrap();
    assert_eq!(sep.0.text, " 12");

    let g = o
        .cells
        .get("gload")
        .unwrap()
        .downcast_ref::<GLoadReadout>()
        .unwrap();
    assert_eq!(g.0.text, "G2.1");
    // 机动条: index 现帧, len 用上一帧会话量 (分发后才更新, 原保真语义)
    let mbar = o
        .cells
        .get("maneuverbar")
        .unwrap()
        .downcast_ref::<ManeuverBar>()
        .unwrap();
    assert_eq!(mbar.maneuver_index, 0.37);
    assert_eq!(mbar.maneuver_index_len, 0, "首帧 len = 上一帧会话量 (0)");

    // len 族: rightDraw=154 (updateLegacyComponents L487-495 手算)
    assert_eq!(o.maneuver_index_len, 114); // round(0.37/0.5*154)=round(113.96)
    assert_eq!(o.tick_scale.ticks, [31, 62, 92, 123, 154]); // 各档 round(档位/0.5*154)

    let thr = o
        .cells
        .get("throttle")
        .unwrap()
        .downcast_ref::<LinearGauge>()
        .unwrap();
    assert_eq!((thr.cur_value, thr.display_value.as_str()), (87, " 87"), "%3d(87)");
    assert_eq!(
        thr.value_color,
        Some([0, 255, 0, 255]),
        "onDataUpdate 注入 throttleColor"
    );

    assert!(o.warn_vne);
    // blink: 致命警告已置位 → drawBlinkX 有输出 (帧序归 WarningBlinkHost 单测)
    let mut cv = PixCanvas::new(o.ctx.width, 40).unwrap();
    o.warning.draw_blink_x(&mut cv, o.ctx.width, 40, false);
    assert!(
        cv.pixmap().data().iter().any(|&b| b != 0),
        "fatalWarn → X 可见"
    );
}

/// onFlightData 节流 (refreshInterval=100ms): 窗口内跳过 (Java L418-431)
#[test]
fn on_flight_data_throttle_gate() {
    let mut o = overlay();
    let s = TestSettings::default();
    let payload = EventPayload::builder().build();
    let src = MockSrc {
        alt: 5300.0,
        sep: -13.2,
    };
    let st = State::new();
    let colors = HudColors::application_defaults();
    assert!(
        o.on_flight_data(1000, Some(&st), None, &payload, Some(&src), None, &s, &colors),
        "首帧 (0→1000)"
    );
    assert!(
        !o.on_flight_data(
            1050,
            Some(&st),
            None,
            &payload,
            Some(&src),
            None,
            &s,
            &colors
        ),
        "+50ms 跳过"
    );
    assert!(
        !o.on_flight_data(
            1099,
            Some(&st),
            None,
            &payload,
            Some(&src),
            None,
            &s,
            &colors
        ),
        "+99ms 跳过"
    );
    assert!(
        o.on_flight_data(
            1100,
            Some(&st),
            None,
            &payload,
            Some(&src),
            None,
            &s,
            &colors
        ),
        "+100ms 放行"
    );
    let sep = o
        .cells
        .get("sep")
        .unwrap()
        .downcast_ref::<SepReadout>()
        .unwrap();
    assert_eq!(sep.0.text, "SEP↓-13 ", "放行帧已更新 (现场 calculate 的 sep_str)");
}

// ===== 现场计算: service 喂入 → calculate 现算 =====

/// MockSrc: TelemetrySource 全量最小实现 (签名漂移即编译失败, 同
/// 既有 parser 测试 mock 形态)
struct MockSrc {
    alt: f64,
    sep: f64,
}

impl FormulaView for MockSrc {
    // W7: var_value 桩 (alt/sep 字段驱动)
    fn var_value(&self, name: &str) -> Option<f64> {
        match name {
            "altitude" => Some(self.alt),
            "sep" => Some(self.sep),
            "throttle" => Some(64.0),
            // W-E 后 HUD 只走公式槽 — 桩按场景直供 (无 FM 缺省 125)
            "flap_allow_angle" => Some(125.0),
            _ => vm_core::formula::registry::registry()
                .lookup(name)
                .map(|_| 0.0),
        }
    }

    // 公式槽桩: warn_vne 直供 (airbrake==100 场景对位公式判定)
    fn get_formula_value(&self, name: &str) -> Option<f64> {
        match name {
            "warn_vne" => Some(1.0),
            _ => None,
        }
    }
}

/// Java L442-447 同窗口: HUDCalculator.calculate 现算 (service 喂入;
/// W-B 早退守卫要求 state 在场, 传中性 State; throttle 现经 sState.throttle
/// 进 HUDData → LinearGauge.on_data_update)
#[test]
fn update_from_event_calculates_from_service() {
    let mut o = overlay();
    let s = TestSettings::default();
    let src = MockSrc {
        alt: 5300.0,
        sep: 0.0,
    };
    let mut st = State::new();
    st.throttle = 64;
    let payload = EventPayload::builder().build();
    o.update_from_event(
        Some(&st),
        None,
        &payload,
        Some(&src),
        None,
        &s,
        &HudColors::application_defaults(),
    );
    // altStr = "ALT" + %6.0f(5300) (HUDCalculator 的标签前缀语义 — 标签开时
    // refreshTemplates 的 lines[1] 同格式, Java L177 注释 "Format must match
    // HUDCalculator" 即此对齐契约)
    let r1 = o
        .cells
        .get("altitude")
        .unwrap()
        .downcast_ref::<AltitudeReadout>()
        .unwrap();
    assert_eq!(r1.0.text, "ALT  5300");
    // throttle = 64 → " 64" (sState.throttle → data.throttle → 组件)
    let thr = o
        .cells
        .get("throttle")
        .unwrap()
        .downcast_ref::<LinearGauge>()
        .unwrap();
    assert_eq!(thr.display_value, " 64");
}

/// state 快照直传 → hud_calculator 的 sState 整块生效 (flaps/airbrake 到
/// 组件)。喂数链 (vm-app feed_overlays_live) 曾以 None/None 重构事件, 该块整体
/// 跳过致襟翼条等恒 0 — 本测试钉住 "喂数必须带 state" 的消费侧契约。
#[test]
fn update_from_event_consumes_state_snapshot() {
    let mut o = overlay();
    let s = TestSettings::default();
    let src = MockSrc {
        alt: 5300.0,
        sep: 0.0,
    };
    let mut st = vm_core::game_api::parser::State::new();
    st.flaps = 50;
    st.airbrake = 100;
    let payload = EventPayload::builder().build();
    o.update_from_event(
        Some(&st),
        None,
        &payload,
        Some(&src),
        None,
        &s,
        &HudColors::application_defaults(),
    );
    // FlapAngleBar: flaps=50 / allowAngle=125 (无 FM 缺省) → " 50/125"
    let flap = o
        .cells
        .get("flap")
        .unwrap()
        .downcast_ref::<FlapAngleBar>()
        .unwrap();
    assert_eq!(flap.display_text(), " 50/125", "襟翼条应吃到 sState.flaps");
    // airbrake=100 → warnVne (sState 块生效的旁证)
    assert!(o.warn_vne, "减速板 100% → warnVne");
}

// ===== 渲染循环 =====

/// draw: DAG 布局驱动 + 组件有输出 (预览数据已注入; paintComponent L250-255)
#[test]
fn draw_renders_content() {
    let mut o = overlay();
    let plan = o.sizing().unwrap();
    assert!(plan.new_width > 0 && plan.new_height > 0);
    let mut cv = PixCanvas::new(plan.new_width, plan.new_height).unwrap();
    o.draw(&mut cv, false);
    assert!(
        cv.pixmap().data().iter().any(|&b| b != 0),
        "HUD 帧有内容 (行文字/罗盘/准星)"
    );

    // debug 开启 (enableLayoutDebug): 调试框路径不 panic 且仍渲染
    let mut s = TestSettings::default();
    s.layout_debug = true;
    let mut o2 =
        MiniHudOverlay::init(false, 100, &s, 1.0, &font_path(), &doc()).unwrap();
    let plan2 = o2.sizing().unwrap();
    let mut cv2 = PixCanvas::new(plan2.new_width, plan2.new_height).unwrap();
    o2.draw(&mut cv2, false);
    assert!(cv2.pixmap().data().iter().any(|&b| b != 0));
}

/// reinitConfig: 配置翻转后引擎重建 + 可见性翻转 + 字体档换新 (WYSIWYG 链)
#[test]
fn reinit_config_rebuilds() {
    let mut o = overlay();
    assert!(o.cells.get("speedBar").unwrap().is_visible());
    let mut s = TestSettings::default();
    s.show_speed_bar = false;
    s.draw_hud_mach = false;
    s.font_size_add = 4; // hudFontSize 28 → 32
    o.reinit_config(&s, &doc()).unwrap();
    assert!(!o.cells.get("speedBar").unwrap().is_visible());
    assert!(o.cells.get("throttle").unwrap().is_visible());
    assert_eq!(o.fonts.draw.size, 32, "ctx 重建 → 字体档换新");
    // 模板已刷新 (mach 关 → SPD 前缀)
    let tpl = {
        let r0 = o
            .cells
            .get("speed")
            .unwrap()
            .downcast_ref::<SpeedReadout>()
            .unwrap();
        r0.0.template.clone()
    };
    assert_eq!(tpl.as_deref(), Some("SPD  360"));
    // 重建后渲染仍工作
    let plan = o.sizing().unwrap();
    let mut cv = PixCanvas::new(plan.new_width, plan.new_height).unwrap();
    o.draw(&mut cv, false);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0));
}

/// draw_rect_1px = Graphics.drawRect(x,y,w,h) 1px 环 (drawDebug)
#[test]
fn debug_frame_ring_geometry() {
    let mut cv = PixCanvas::new(20, 20).unwrap();
    crate::render::primitives::ring1px(&mut cv, 5, 5, 10, 6, [255, 255, 255, 255]);
    let a = |x: i32, y: i32| cv.pixmap().data()[((y * cv.width() + x) * 4) as usize + 3];
    assert_eq!(a(5, 5), 255, "左上角");
    assert_eq!(a(15, 5), 255, "右上角 (x+w 含端点)");
    assert_eq!(a(15, 11), 255, "右下角 (y+h 含端点)");
    assert_eq!(a(10, 8), 0, "内部空");
    assert_eq!(a(4, 5), 0, "左侧外");
    assert_eq!(a(16, 5), 0, "右侧外");
}

// ===== host 挂载 =====

/// spec: 尺寸取自动尺寸计划; render 闭包可画
#[test]
fn overlay_spec_sizes_and_renders() {
    let s = TestSettings::default();
    // 参数仓 hud 快照与工厂 settings 同源 (生产面: inputs.hud == params.hud);
    // pages = 出厂页 (spec 工厂按 id 取 minihud-default)
    let cell = Rc::new(RefCell::new(ReinitParams {
        hud: HudSettingsSnapshot::build(&s),
        pages: vm_core::config::json_store::factory_pages_arc(),
        ..Default::default()
    }));
    let (handle, mut spec) =
        minihud_overlay_spec(false, 100, &s, 1.0, &font_path(), &cell).unwrap();
    assert_eq!(spec.id, "minihud-default"); // R3: 条目键 = 页 id
    assert_eq!(spec.config_key, "crosshairSwitch");
    let plan = handle.borrow().sizing().unwrap();
    assert_eq!((spec.width, spec.height), (plan.new_width, plan.new_height));
    assert!(spec.width > 0 && spec.height > 0);
    // render 闭包可执行 (host render_tick 的等价调用)
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0));

    // WYSIWYG reinit: hud 快照换字号/开关 → reinit_config 重建 + 新尺寸
    // (TestSettings 全显行集; fontadd 0→6 行高变大 → 高度必增)
    let (_w0, h0) = (spec.width, spec.height);
    cell.borrow_mut().hud.font_size_add = 6;
    let (w1, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h1 > h0, "字号增量后高度应变大 ({} → {})", h0, h1);
    assert!(w1 > 0);
    // reinit 后 render 闭包可画 (新布局/字体, 不 panic)
    let mut cv2 = PixCanvas::new(w1, h1).unwrap();
    (spec.render)(&mut cv2);
    assert!(cv2.pixmap().data().iter().any(|&b| b != 0));
}
