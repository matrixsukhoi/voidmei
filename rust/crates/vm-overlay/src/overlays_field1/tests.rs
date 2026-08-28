use super::*;
use vm_core::ui_model::TelemetrySource;

const FONTS: &str = "../../../fonts";

fn bold(size: i32) -> LoadedFont {
    LoadedFont::new(std::path::Path::new(FONTS).join("sarasa-mono-sc-bold.ttf").as_path(), size)
        .unwrap()
}

/// 读预乘 RGBA 像素
fn px(cv: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
    let i = ((y * cv.width() + x) * 4) as usize;
    let d = &cv.pixmap().data()[i..i + 4];
    [d[0], d[1], d[2], d[3]]
}

fn alpha(cv: &PixCanvas, x: i32, y: i32) -> u8 {
    px(cv, x, y)[3]
}

/// 测试用 TelemetrySource mock: 相关字段可配, 其余 0/false
/// (对齐 vm-core telemetry_source.rs 测试 mock 的全签名锁定模式)
struct MockTele {
    throttle: f64,
    rpm_throttle: f64,
    power_percent: f64,
    mixture: f64,
    radiator: f64,
    compressor_stage: f64,
    fuel_percent: f64,
    gear: f64,
    flaps: f64,
    airbrake: f64,
    horse_power: f64,
    thrust: f64,
    rpm: f64,
    pitch: f64,
    prop_eff: f64,
    eff_hp: f64,
    manifold: f64,
    manifold_unit: &'static str,
    manifold_prec: i32,
    mass_fuel: f64,
    total_weight: f64,
    fuel_time_mili: i64,
    wep_kg: f64,
    wep_time: f64,
    booster_kg: f64,
    booster_pct: f64,
    water_temp: f64,
    oil_temp: f64,
    heat_tol: f64,
    engine_resp: f64,
    jet: bool,
    piston: bool,
    wep: bool,
    booster: bool,
}

impl Default for MockTele {
    fn default() -> Self {
        MockTele {
            throttle: 0.0,
            rpm_throttle: 0.0,
            power_percent: 0.0,
            mixture: 0.0,
            radiator: 0.0,
            compressor_stage: 0.0,
            fuel_percent: 0.0,
            gear: 0.0,
            flaps: 0.0,
            airbrake: 0.0,
            horse_power: 0.0,
            thrust: 0.0,
            rpm: 0.0,
            pitch: 0.0,
            prop_eff: 0.0,
            eff_hp: 0.0,
            manifold: 0.0,
            manifold_unit: "Ata",
            manifold_prec: 2,
            mass_fuel: 0.0,
            total_weight: 0.0,
            fuel_time_mili: 0,
            wep_kg: 0.0,
            wep_time: 0.0,
            booster_kg: 0.0,
            booster_pct: 0.0,
            water_temp: 0.0,
            oil_temp: 0.0,
            heat_tol: 0.0,
            engine_resp: 0.0,
            jet: false,
            piston: true,
            wep: false,
            booster: false,
        }
    }
}

#[allow(clippy::too_many_lines)]
impl TelemetrySource for MockTele {
    fn get_ias(&self) -> f64 { 0.0 }
    fn get_tas(&self) -> f64 { 0.0 }
    fn get_mach(&self) -> f64 { 0.0 }
    fn get_aoa(&self) -> f64 { 0.0 }
    fn get_aos(&self) -> f64 { 0.0 }
    fn get_ny(&self) -> f64 { 0.0 }
    fn get_vario(&self) -> f64 { 0.0 }
    fn get_altitude(&self) -> f64 { 0.0 }
    fn get_radio_altitude(&self) -> f64 { 0.0 }
    fn is_radio_altitude_valid(&self) -> bool { false }
    fn get_compass(&self) -> f64 { 0.0 }
    fn get_sep(&self) -> f64 { 0.0 }
    fn get_acceleration(&self) -> f64 { 0.0 }
    fn get_turn_rate(&self) -> f64 { 0.0 }
    fn get_turn_radius(&self) -> f64 { 0.0 }
    fn is_turn_radius_valid(&self) -> bool { false }
    fn get_roll_rate(&self) -> f64 { 0.0 }
    fn get_energy_jkg(&self) -> f64 { 0.0 }
    fn get_mass_fuel(&self) -> f64 { self.mass_fuel }
    fn get_total_weight(&self) -> f64 { self.total_weight }
    fn get_fuel_time_mili(&self) -> i64 { self.fuel_time_mili }
    fn get_throttle(&self) -> f64 { self.throttle }
    fn get_rpm(&self) -> f64 { self.rpm }
    fn get_manifold_pressure(&self) -> f64 { self.manifold }
    fn get_water_temp(&self) -> f64 { self.water_temp }
    fn get_oil_temp(&self) -> f64 { self.oil_temp }
    fn get_pitch(&self) -> f64 { self.pitch }
    fn get_eff_hp(&self) -> f64 { self.eff_hp }
    fn get_thrust(&self) -> f64 { self.thrust }
    fn get_horse_power(&self) -> f64 { self.horse_power }
    fn get_engine_response(&self) -> f64 { self.engine_resp }
    fn get_prop_efficiency(&self) -> f64 { self.prop_eff }
    fn get_wep_kg(&self) -> f64 { self.wep_kg }
    fn get_wep_time(&self) -> f64 { self.wep_time }
    fn get_heat_tolerance(&self) -> f64 { self.heat_tol }
    fn get_power_percent(&self) -> f64 { self.power_percent }
    fn get_manifold_pressure_pounds(&self) -> f64 { 0.0 }
    fn get_manifold_pressure_inch_hg(&self) -> f64 { 0.0 }
    fn get_manifold_pressure_display(&self) -> f64 { self.manifold }
    fn get_manifold_pressure_display_unit(&self) -> String { self.manifold_unit.to_string() }
    fn get_manifold_pressure_display_precision(&self) -> i32 { self.manifold_prec }
    fn get_unknown_mixture(&self) -> f64 { self.mixture }
    fn get_radiator(&self) -> f64 { self.radiator }
    fn get_compressor_stage(&self) -> f64 { self.compressor_stage }
    fn get_fuel_percent(&self) -> f64 { self.fuel_percent }
    fn get_rpm_throttle(&self) -> f64 { self.rpm_throttle }
    fn get_gear(&self) -> f64 { self.gear }
    fn get_flaps(&self) -> f64 { self.flaps }
    fn get_airbrake(&self) -> f64 { self.airbrake }
    fn get_aileron(&self) -> f64 { 0.0 }
    fn get_elevator(&self) -> f64 { 0.0 }
    fn get_rudder(&self) -> f64 { 0.0 }
    fn get_wing_sweep(&self) -> f64 { 0.0 }
    fn is_wing_sweep_valid(&self) -> bool { false }
    fn get_speed_limit_ratio(&self) -> f64 { 0.0 }
    fn get_aileron_lock_ratio(&self) -> f64 { 0.0 }
    fn get_rudder_lock_ratio(&self) -> f64 { 0.0 }
    fn get_unit_mach_limit_ratio(&self) -> f64 { 0.0 }
    fn get_stall_speed(&self) -> f64 { 0.0 }
    fn is_imperial(&self) -> bool { false }
    fn get_aviahorizon_pitch(&self) -> f64 { 0.0 }
    fn get_aviahorizon_roll(&self) -> f64 { 0.0 }
    fn is_jet_engine(&self) -> bool { self.jet }
    fn is_prop_engine(&self) -> bool { !self.jet }
    fn is_piston_engine(&self) -> bool { self.piston }
    fn is_turboprop_engine(&self) -> bool { false }
    fn is_engine_check_done(&self) -> bool { true }
    fn has_wep(&self) -> bool { self.wep }
    fn get_booster_fuel_kg(&self) -> f64 { self.booster_kg }
    fn get_booster_fuel_percent(&self) -> f64 { self.booster_pct }
    fn has_booster(&self) -> bool { self.booster }
}

fn payload(is_jet: bool, engine_check_done: bool, optimal: i32) -> EventPayload {
    EventPayload::builder()
        .is_jet(is_jet)
        .engine_check_done(engine_check_done)
        .optimal_compressor_stage(optimal)
        .build()
}

// ---- butt_line (GraphicsUtil.createPreciseStroke 像素盒约定) ----

/// w=2 CAP_BUTT 约定钉死: aa=false = 行 y-1..y × 列 xa..xb-1 (右端列不点亮);
/// w=2 竖线镜像 = 列 x-1..x × 行 ya..yb-1 (与 gauges_bars::hline_butt2 文档互证)
#[test]
fn butt_line_width2_center_rule() {
    let white = [255u8, 255, 255, 255];
    let mut cv = PixCanvas::new(40, 40).unwrap();
    butt_line(&mut cv, 5, 10, 25, 10, 2, white, false);
    for x in [5, 15, 24] {
        assert_eq!(alpha(&cv, x, 9), 255, "上邻行 ({x},9)");
        assert_eq!(alpha(&cv, x, 10), 255, "中线 ({x},10)");
    }
    assert_eq!(alpha(&cv, 25, 10), 0, "右端列不点亮");
    assert_eq!(alpha(&cv, 4, 10), 0, "左端外");
    assert_eq!(alpha(&cv, 15, 8), 0, "线体上方 2px");
    assert_eq!(alpha(&cv, 15, 11), 0, "线体下方 1px");
    let mut cv2 = PixCanvas::new(40, 40).unwrap();
    butt_line(&mut cv2, 10, 5, 10, 25, 2, white, false);
    for y in [5, 15, 24] {
        assert_eq!(alpha(&cv2, 9, y), 255, "左邻列 (9,{y})");
        assert_eq!(alpha(&cv2, 10, y), 255, "中列 (10,{y})");
    }
    assert_eq!(alpha(&cv2, 10, 25), 0, "下端行不点亮 (镜像)");
    assert_eq!(alpha(&cv2, 11, 15), 0, "右侧外");
}

/// aa=true 分离覆盖模型: 行 y 全值 / y±1 半值 / 端点列半覆盖 / 四角 1/4
#[test]
fn butt_line_width2_aa_coverage() {
    let white = [255u8, 255, 255, 255];
    let mut cv = PixCanvas::new(40, 40).unwrap();
    butt_line(&mut cv, 10, 20, 30, 20, 2, white, true);
    assert_eq!(alpha(&cv, 20, 20), 255, "内部中线全值");
    assert_eq!(alpha(&cv, 20, 19), 128, "上柔边半值");
    assert_eq!(alpha(&cv, 20, 21), 128, "下柔边半值");
    assert_eq!(alpha(&cv, 10, 20), 128, "左端列半覆盖");
    assert_eq!(alpha(&cv, 10, 19), 64, "左上角 1/4");
    assert_eq!(alpha(&cv, 20, 22), 0, "柔边外");
    // 1px 线: 规整后覆盖盒边界为整数像素边界, 无柔边
    let mut cv2 = PixCanvas::new(40, 40).unwrap();
    butt_line(&mut cv2, 10, 20, 30, 20, 1, white, true);
    assert_eq!(alpha(&cv2, 20, 20), 255);
    assert_eq!(alpha(&cv2, 20, 19), 0);
    assert_eq!(alpha(&cv2, 20, 21), 0);
}

// ---- VisExpr ----

/// 求值语义: 谓词/比较/= 容差(0.0001 边界)/Not/And — 与 vm-core 求值器一致
#[test]
fn vis_expr_semantics() {
    let t = MockTele::default(); // piston=true, jet=false
    assert!(!VisExpr::IsJetEngine.eval(&t, 0.0));
    assert!(VisExpr::IsPistonEngine.eval(&t, 0.0));
    assert!(!VisExpr::HasWep.eval(&t, 0.0));
    assert!(VisExpr::Gt(0.0).eval(&t, 0.1));
    assert!(!VisExpr::Gt(0.0).eval(&t, 0.0));
    assert!(VisExpr::Lte(0.0).eval(&t, 0.0));
    assert!(VisExpr::Eq(1.0).eval(&t, 1.00001));
    assert!(!VisExpr::Eq(1.0).eval(&t, 1.0002));
    // f64 边界: 字面量 1.0001-1.0 实际差 ≈ 9.9999e-5 < 0.0001 → 视为相等
    // (vm-core 求值器测试同款 oracle: "(= value 1)" 对 1.0001 为 true)
    assert!(VisExpr::Eq(1.0).eval(&t, 1.0001));
    assert!(!VisExpr::NotEq(1.0).eval(&t, 1.0001));
    assert!(!VisExpr::NotEq(1.0).eval(&t, 1.0));
    let not_jet = VisExpr::Not(&VisExpr::IsJetEngine);
    assert!(not_jet.eval(&t, 0.0));
    let and = VisExpr::And(&VisExpr::IsPistonEngine, &VisExpr::NotEq(1.0));
    assert!(and.eval(&t, 0.98));
    assert!(!and.eval(&t, 1.0));
}

// ---- MarkedGauge 状态 ----

/// 标记通道: 默认隐藏 / updateMarkerRatio 容差内不写 / 未命中 id 无操作 /
/// update_buffer vs update_display 双通道
#[test]
fn marked_gauge_marker_and_value_channels() {
    let mut mg = MarkedGauge::new();
    mg.add_marker(GaugeMarker {
        id: "optimal".to_string(),
        ..GaugeMarker::default()
    });
    assert_eq!(mg.markers[0].ratio, -1.0);
    assert!(!mg.markers[0].is_visible(), "ratio<0 隐藏");
    mg.update_marker_ratio("optimal", 0.5);
    assert_eq!(mg.markers[0].ratio, 0.5);
    // 容差内 (|Δ|<0.0001) 不更新 (Java withRatio 返回自身)
    mg.update_marker_ratio("optimal", 0.50005);
    assert_eq!(mg.markers[0].ratio, 0.5);
    mg.update_marker_ratio("optimal", 1.5);
    assert_eq!(mg.markers[0].ratio, 1.5);
    assert!(!mg.markers[0].is_visible(), "ratio>1 隐藏");
    mg.update_marker_ratio("nope", 0.7); // 未命中 → 无操作
    assert_eq!(mg.markers.len(), 1);
    // 值通道
    mg.update_buffer(2, "3");
    assert_eq!((mg.current_value, mg.value_len), (2.0, 1));
    assert_eq!(mg.value_buffer, "3");
    mg.update_display(2, "3");
    assert_eq!(mg.value_len, 0, "字符串通道使 buffer 失效");
    assert_eq!(mg.display_value, "3");
    // setMaxValue + pix clamp
    mg.set_max_value(100.0);
    mg.current_value = 150.0;
    assert_eq!(mg.pix_value(96), 96, "clamp 到 length");
    mg.current_value = -3.0;
    assert_eq!(mg.pix_value(96), 0, "clamp 到 0");
}

// ---- MarkedGauge 横向像素 (EngineControl COMPRESSOR 形态) ----

/// 横向条几何: 背景透明不可见 / 填充 pixVal / 边框环 / 分隔线 (无标记=1px) /
/// 数值文本在条下方
#[test]
fn marked_gauge_horizontal_geometry() {
    let font = bold(12);
    let mut mg = MarkedGauge::new();
    mg.set_bar_style(GaugeBarStyle {
        fill_color: colors().num,
        background_color: [0, 0, 0, 0], // COMPRESSOR 透明背景
        border_color: colors().shade_shape,
        show_border: true,
        vertical: false,
        stroke_width: 2,
    });
    mg.set_max_value(100.0);
    mg.update_display(50, "50"); // pixVal = round(50*96/100) = 48
    let mut cv = PixCanvas::new(140, 60).unwrap();
    mg.draw(&mut cv, 10, 20, 96, 12, &font, false);
    // 背景 (透明) + 未填充段: 无输出 (fillRect alpha=0 无效)
    assert_eq!(alpha(&cv, 100, 25), 0, "填充外/边框内无背景");
    // 填充: (10,20,48,12) → 右缘列 57
    assert_eq!(alpha(&cv, 11, 21), colors().num[3], "填充左上内");
    assert_eq!(alpha(&cv, 57, 30), colors().num[3], "填充右下内 (row31 与环底行重叠)");
    // (58,25) 为分隔线主线列 (x+pixVal); 其右 2px 才是填充外
    assert_eq!(alpha(&cv, 60, 25), 0, "填充/分隔线右外");
    // 边框环 (drawRect(10,20,95,11)): 右边框列 105 纯 shade
    assert_eq!(alpha(&cv, 105, 25), colors().shade_shape[3]);
    assert_eq!(alpha(&cv, 10, 20), 242, "左上角 = shade over fill (SrcOver)");
    // 分隔线 (无标记 → 1px): 主线列 x+pixVal=58, 影线列 59,
    // 行 y..y+sepHeight = 20..20+(12+12+2)=46
    assert_eq!(alpha(&cv, 58, 45), colors().num[3], "主线延伸到条下方");
    assert_eq!(alpha(&cv, 59, 45), colors().shade_shape[3], "影线 (+1)");
    assert_eq!(alpha(&cv, 60, 45), 0, "线右邻空");
    assert_eq!(alpha(&cv, 58, 47), 0, "线末端外");
    // 数值文本: 基线 (x+pixVal, y+thickness+fontSize) = (58, 44), "50" 在其右
    let text_zone = (58..110).any(|x| (32..45).any(|y| alpha(&cv, x, y) > 0));
    assert!(text_zone, "条下方有数值文本");
}

/// 标记可见时: LINE_FULL 竖刻度 (2px butt, 列 markerX-1..markerX) +
/// 分隔线承袭 tickStroke 变 2px (列 pixVal-1..pixVal)
#[test]
fn marked_gauge_marker_line_and_wide_separator() {
    let font = bold(12);
    let mut mg = MarkedGauge::new();
    mg.set_bar_style(GaugeBarStyle {
        fill_color: colors().num,
        background_color: [0, 0, 0, 0],
        border_color: colors().shade_shape,
        show_border: true,
        vertical: false,
        stroke_width: 2,
    });
    mg.set_max_value(100.0);
    mg.update_display(50, "50"); // pixVal = 48 → markerX = 10 + (int)(96*0.5) = 58
    // ratio 0.25 → markerX = 10 + (int)(96*0.25) = 34 (避开分隔线列 57..59)
    mg.add_marker(GaugeMarker {
        id: "optimal".to_string(),
        marker_type: MarkerType::LineFull,
        ratio: 0.25,
        color: colors().warning,
        ..GaugeMarker::default()
    });
    let mut cv = PixCanvas::new(140, 60).unwrap();
    mg.draw(&mut cv, 10, 20, 96, 12, &font, false);
    // LINE_FULL 标记 (列 33..34, 行 20..30): warning(a=100) SrcOver 填充(a=240) → 加深
    assert!(alpha(&cv, 33, 25) > colors().num[3], "标记左列 = warning over fill");
    assert!(alpha(&cv, 34, 25) > colors().num[3], "标记右列");
    assert_eq!(alpha(&cv, 32, 25), colors().num[3], "标记左外 = 纯填充");
    assert_eq!(alpha(&cv, 35, 25), colors().num[3], "标记右外 = 纯填充");
    // 标记端行: Java 线终点 y=barY+thickness(=32) → 中心规则行 20..31 亮;
    // 行 31 = warning over fill 再叠边框环底 ≈247, 行 32 (条外) 无标记
    assert!(alpha(&cv, 34, 31) > colors().num[3], "标记贯穿到条底行");
    assert_eq!(alpha(&cv, 34, 32), 0, "条外行无标记");
    // 分隔线承袭 tickStroke (2px): 主线列 57..58, 影线列 58..59 (先主线后影线,
    // col58 = 影线 over 主线 ≈243); 延伸行到 46
    assert_eq!(alpha(&cv, 57, 45), colors().num[3], "主线 (2px) 左列 (纯主线)");
    assert_eq!(alpha(&cv, 58, 45), 243, "主线右列 = 主线+影线叠加");
    assert_eq!(alpha(&cv, 59, 45), colors().shade_shape[3], "影线纯段");
    assert_eq!(alpha(&cv, 60, 45), 0, "影线右外");
}

/// 竖向条几何 (MarkedGauge 默认 vertical): 文本左条右, 填充自底向上,
/// 分隔线 = drawSeparator (shade 环 + fill 1px 内芯)
#[test]
fn marked_gauge_vertical_geometry() {
    let font = bold(12);
    let mut mg = MarkedGauge::new(); // fill=colors().num, bg=colors().shade_shape, vertical
    mg.label = "节".to_string();
    mg.set_max_value(100.0);
    mg.update_display(50, "50");
    let mut cv = PixCanvas::new(120, 60).unwrap();
    mg.draw(&mut cv, 5, 10, 40, 12, &font, false);
    // barX = x + labelW + valueW + 2
    let bar_x = 5 + font.measure("节") + font.measure("50") + 2;
    // 背景条 (shade): 顶端行 10
    assert_eq!(alpha(&cv, bar_x + 3, 10), colors().shade_shape[3], "背景条顶端");
    // 填充自底向上: pixVal = 20 → rows 30..49; 背景在 rows 10..29
    assert_eq!(alpha(&cv, bar_x + 3, 15), colors().shade_shape[3], "上半背景");
    // 下半填充 = colorNum SrcOver colorShadeShape 背景 → alpha ≈ 243 (双层)
    assert!((242..=244).contains(&alpha(&cv, bar_x + 3, 35)), "下半填充 (叠背景)");
    // 分隔线: sepY = 10+40-1-20 = 29, 环 29..31 + fill 内芯行 30
    assert_eq!(alpha(&cv, 5, 29), colors().shade_shape[3], "分隔环上边");
    // 内芯行 30: 取文本与条之间的列 (bar_x-1), 避开文本 descender
    assert_eq!(alpha(&cv, bar_x - 1, 30), colors().num[3], "分隔内芯");
}

// ---- PowerInfo ----

/// 常量表快照: 19 项, 关键行 (进气压动态通道 / 燃油时 TIME_MM_SS) 与 cfg 一致
#[test]
fn power_field_defs_snapshot() {
    assert_eq!(POWER_FIELD_DEFS.len(), 19);
    assert_eq!(POWER_FIELD_DEFS[0].label, "功  率");
    assert_eq!(POWER_FIELD_DEFS[0].na_when, Some(VisExpr::Lte(0.0)));
    let manifold = &POWER_FIELD_DEFS[6];
    assert_eq!(manifold.label, "进气压");
    assert_eq!(manifold.unit_source, Some(DynSource::ManifoldDisplay));
    assert_eq!(manifold.precision_source, Some(DynSource::ManifoldDisplay));
    assert_eq!(manifold.visible_when, Some(VisExpr::And(&VisExpr::IsPistonEngine, &VisExpr::NotEq(1.0))));
    let fuel_time = &POWER_FIELD_DEFS[10];
    assert_eq!(fuel_time.source, PowerSource::FuelTimeMiliMul001);
    assert_eq!(fuel_time.format, PowerFormat::TimeMmSs);
    assert_eq!(POWER_FIELD_DEFS[17].na_when, Some(VisExpr::Gt(90000.0)));
}

/// 更新路径 (FieldOverlay.onFlightData 零 GC): visible-when / na-when "-" /
/// TIME_MM_SS / 动态单位精度 / 预览不受影响
#[test]
fn power_info_update_paths() {
    let mut st = PowerInfoState::new();
    let mut t = MockTele {
        horse_power: 1200.0,
        thrust: 1000.0,
        rpm: 2400.0,
        pitch: 55.0,
        prop_eff: 85.0,
        eff_hp: 1100.0,
        manifold: 0.98,
        power_percent: 95.0,
        mass_fuel: 500.0,
        total_weight: 3500.0,
        fuel_time_mili: 2750, // 2.75s → "00'02"
        water_temp: 90.0,
        oil_temp: 80.0,
        heat_tol: 60.0,
        engine_resp: 10.0,
        ..MockTele::default()
    };
    // 首事件 now=0: 0-0 < 50 → 节流跳过 (Java 同, lastRefreshTime 初值 0)
    assert!(!st.update(0, &t));
    assert!(st.update(100, &t));
    let f = st.fields();
    let find = |l: &str| f.iter().find(|x| x.label == l).unwrap();
    assert_eq!((find("功  率").buffer.as_str(), find("功  率").length), ("1200", 4));
    assert_eq!(find("桨距角").buffer, "55.0");
    assert_eq!(find("进气压").buffer, "0.98");
    assert_eq!(find("进气压").unit, "Ata");
    assert_eq!(find("燃油时").buffer, "00'02");
    assert_eq!(find("燃油量").buffer, "500");
    // 加力/助推字段: 特性标志 false → 隐藏
    assert!(!find("加力量").visible);
    assert!(!find("助推燃料").visible);
    // 节流: 间隔内 (+30ms < 50) 拒绝且数据不更新 (转速改 9999 不生效)
    t.rpm = 9999.0;
    assert!(!st.update(130, &t));
    assert_eq!(st.fields().iter().find(|x| x.label == "转  速").unwrap().buffer, "2400");
    t.rpm = 2400.0;
    // na-when: 功率 0 → "-"
    t.horse_power = 0.0;
    assert!(st.update(200, &t));
    assert_eq!(st.fields()[0].buffer, "-");
    assert_eq!(st.fields()[0].length, 1);
    // 动态单位/精度: 英制 Psi + 0 位
    t.manifold_unit = "Psi";
    t.manifold_prec = 0;
    t.manifold = 44.6;
    assert!(st.update(300, &t));
    let m = st.fields().iter().find(|x| x.label == "进气压").unwrap();
    assert_eq!((m.unit.as_str(), m.buffer.as_str(), m.precision), ("Psi", "45", 0));
    // 喷气机: 功率/桨距角/桨效率/实功率 隐藏, 推力仍在
    t.jet = true;
    t.piston = false;
    assert!(st.update(400, &t));
    assert!(!st.fields()[0].visible, "喷气机隐藏功率");
    assert!(st.fields().iter().find(|x| x.label == "推  力").unwrap().visible);
    // 进气压 value=1 (容差内) → 活塞机也隐藏
    let mut st2 = PowerInfoState::new();
    let mut t2 = MockTele { manifold: 1.0, ..MockTele::default() };
    t2.piston = true;
    assert!(st2.update(100, &t2));
    assert!(!st2.fields().iter().find(|x| x.label == "进气压").unwrap().visible);
}

/// 预览 = 构造后不 update: previewValue 原样落 currentValue, 全部可见
#[test]
fn power_info_preview_state() {
    let st = PowerInfoState::new();
    assert!(st.fields().iter().all(|f| f.visible));
    assert_eq!(st.fields()[0].current_value, "1200");
    assert_eq!(st.fields().iter().find(|x| x.label == "进气压").unwrap().current_value, "1.2");
    assert_eq!(st.fields().iter().filter(|f| f.visible).count(), 19);
}

/// CloseAllOverlays 数据面重置 (app_shell reset_handles_preview_values 调用面):
/// live 残留 (buffer/可见性/节流基准) → reset_preview → 构造态 previewValue 静态。
/// 场景: 托盘 live→preview 后重开的预览窗不得显示上次 live 数值
#[test]
fn power_info_reset_preview_restores_statics() {
    let mut st = PowerInfoState::new();
    // 活塞机形态 (buffer 写入面) + 助推标志 false (live 驱动的可见性残留面)
    let t = MockTele {
        horse_power: 1200.0,
        manifold: 0.98,
        piston: true,
        ..MockTele::default()
    };
    assert!(st.update(100, &t));
    assert_eq!(st.fields()[0].buffer, "1200", "live 已写 buffer");
    assert!(!st.fields().iter().find(|x| x.label == "助推燃料").unwrap().visible,
        "助推 false → live 隐藏 (preview 构造态为全可见)");
    // 重置 → 构造态: buffer 清空 / 全可见 / currentValue 回 previewValue
    st.reset_preview();
    assert!(st.fields().iter().all(|f| f.length == 0 && f.buffer.is_empty()));
    assert!(st.fields().iter().all(|f| f.visible), "可见性回构造态 (live 残留清除)");
    assert_eq!(st.fields()[0].current_value, "1200");
    assert_eq!(st.fields().iter().find(|x| x.label == "进气压").unwrap().current_value, "1.2");
    assert_eq!(st.last_refresh_time, 0, "节流基准复位 (重进游戏首帧不误吞)");
}

/// BOS 网格绘制 + 预览闭包工厂: 出像素且尺寸正确
#[test]
fn power_info_draw_and_preview_spec() {
    let ctx = RenderContext::load(std::path::Path::new(FONTS), 0, 2).unwrap();
    let st = PowerInfoState::new();
    let (w, h) = st.preferred_size(&ctx);
    let mut cv = PixCanvas::new(w, h).unwrap();
    let mut renderer = BosStyleRenderer::default();
    st.draw(&mut cv, &ctx, &mut renderer);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0), "预览网格非空");
    // 工厂闭包 (OverlayHost 渲染闭包形态)
    let mut spec = power_info_preview_spec(std::path::Path::new(FONTS), 0, 2).unwrap();
    assert_eq!((spec.width, spec.height), (w, h));
    let mut cv2 = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv2);
    assert_eq!(cv2.to_premul_bgra(), cv.to_premul_bgra(), "工厂闭包与直绘一致");
}

// ---- EngineControl ----

fn lang() -> Lang {
    Lang::init_lang()
}

/// initGaugeFields + calculateLayout: 7 仪表 (3 竖 4 横), 高度公式移位优先级保真;
/// 开关禁用过滤
#[test]
fn engine_control_defs_and_layout() {
    let l = lang();
    let st = EngineControlState::new(&l, 0, 1.0, &|_| false, &|_| String::new());
    assert_eq!(st.gauges.len(), 7);
    assert_eq!(st.font_size, 24);
    assert_eq!(st.width, 192); // 24*8
    // (24*4+24*9)>>1 + (4+1)*(24+6) = 156 + 150 = 306
    assert_eq!(st.height, 306);
    // Lang 标签接线 (Lang.eThrottle="节" 等)
    assert_eq!(st.gauges[0].gauge.gauge.label, "节");
    assert_eq!(st.gauges[5].gauge.gauge.label, "增");
    // COMPRESSOR 挂 MarkedGauge, 其余不挂
    assert!(st.gauges[5].marked_gauge.is_some());
    assert_eq!(st.gauges().iter().filter(|g| g.marked_gauge.is_some()).count(), 1);
    // 禁用探测: disableEngineInfoThrottle=true → 6 仪表, 高度重算 (row_num 仍 4)
    let st2 = EngineControlState::new(&l, 0, 1.0, &|k| k == "disableEngineInfoThrottle", &|_| String::new());
    assert_eq!(st2.gauges.len(), 6);
    assert!(st2.gauge_by_key("throttle").is_none());
    assert_eq!(st2.height, 306, "竖条不进行数公式");
    // DPI: 150% → fontsize=round(36)=36
    let st3 = EngineControlState::new(&l, 0, 1.5, &|_| false, &|_| String::new());
    assert_eq!(st3.font_size, 36);
}

/// updateGaugesPreview: val=maxValue/2, COMPRESSOR 显示 1 基档号, 标记 0.5;
/// new() (reinitConfig 链) 末尾即置此初值 — 游戏模式首个有效事件前显示半量程
#[test]
fn engine_control_preview_values() {
    let l = lang();
    let mut st = EngineControlState::new(&l, 0, 1.0, &|_| false, &|_| String::new());
    // reinitConfig:187 updateGaugesPreview (游戏模式初值 = 半量程可见)
    let thr0 = st.gauge_by_key("throttle").unwrap();
    assert_eq!(thr0.gauge.gauge.cur_value, 55);
    assert!(st.gauges().iter().all(|g| g.visible));
    // Java initPreview (:171-172) 对 init 链结果原样二次调用 (幂等)
    st.update_preview();
    let thr = st.gauge_by_key("throttle").unwrap();
    assert_eq!((thr.gauge.gauge.cur_value, thr.gauge.gauge.display_value.as_str()), (55, "55"));
    let comp = st.gauge_by_key("compressor").unwrap();
    let mg = comp.marked_gauge.as_ref().unwrap();
    // max=1 → val=0 → 显示 1 基档号 "1" (display 通道)
    assert_eq!((mg.current_value, mg.value_len, mg.display_value.as_str()), (0.0, 0, "1"));
    assert_eq!(mg.markers[0].ratio, 0.5);
    assert!(st.gauges().iter().all(|g| g.visible));
}

/// loadRefreshInterval (EngineControlOverlay.java:202-212): dataPollIntervalMs×2 /
/// legacy "Interval" 回退 / 解析失败 → parseLongSafe 默认 100×2 / 双键空保持默认
#[test]
fn engine_control_load_refresh_interval() {
    let l = lang();
    // getConfigSafe 模拟: 按键查表 (闭包参数引用需 HRTB, 用嵌套 fn 表达)
    fn cfg_of<'a>(pairs: &'a [(&'static str, &'static str)]) -> impl Fn(&str) -> String + 'a {
        move |k: &str| {
            pairs
                .iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.to_string())
                .unwrap_or_default()
        }
    }
    // dataPollIntervalMs=50 → 100
    let st = EngineControlState::new(&l, 0, 1.0, &|_| false,
        &cfg_of(&[("dataPollIntervalMs", "50")]));
    assert_eq!(st.refresh_interval, 100);
    // legacy "Interval" 回退: 33 → 66
    let st2 = EngineControlState::new(&l, 0, 1.0, &|_| false,
        &cfg_of(&[("Interval", "33")]));
    assert_eq!(st2.refresh_interval, 66);
    // dataPollIntervalMs 优先于 legacy
    let st3 = EngineControlState::new(&l, 0, 1.0, &|_| false,
        &cfg_of(&[("dataPollIntervalMs", "20"), ("Interval", "999")]));
    assert_eq!(st3.refresh_interval, 40);
    // 解析失败 → parseLongSafe 默认 100 → ×2 = 200
    let st4 = EngineControlState::new(&l, 0, 1.0, &|_| false,
        &cfg_of(&[("dataPollIntervalMs", "abc")]));
    assert_eq!(st4.refresh_interval, 200);
    // 双键空 → 保持字段初始默认 100 (POC 空配置读取器同此)
    let st5 = EngineControlState::new(&l, 0, 1.0, &|_| false, &|_| String::new());
    assert_eq!(st5.refresh_interval, ENGINE_DEFAULT_REFRESH_MS);

    // 节流随间隔生效: interval=100 (dataPollIntervalMs=50) — 首事件 0 跳过,
    // +50 拒绝, +100 放行
    let mut st6 = EngineControlState::new(&l, 0, 1.0, &|_| false,
        &cfg_of(&[("dataPollIntervalMs", "50")]));
    let t = MockTele::default();
    assert!(!st6.update(0, &t, &payload(false, false, -1), None), "0-0 < 100 跳过");
    assert!(st6.update(100, &t, &payload(false, false, -1), None));
    assert!(!st6.update(150, &t, &payload(false, false, -1), None));
    assert!(st6.update(200, &t, &payload(false, false, -1), None));
    // reinit 复用: 宿主可再调 load_refresh_interval 随配置更新间隔
    st6.load_refresh_interval(&cfg_of(&[("dataPollIntervalMs", "10")]));
    assert_eq!(st6.refresh_interval, 20);
}

/// updateGaugesZeroGC: 各类型取值/格式化, PITCH/MIXTURE -1 隐藏, COMPRESSOR 档位换算
#[test]
fn engine_control_update_zero_gc() {
    let l = lang();
    let mut st = EngineControlState::new(&l, 0, 1.0, &|_| false, &|_| String::new());
    let mut t = MockTele {
        throttle: 55.0,
        rpm_throttle: 60.0,
        power_percent: 80.0,
        mixture: 100.0,
        radiator: 30.0,
        compressor_stage: 2.0,
        fuel_percent: 64.0,
        ..MockTele::default()
    };
    assert!(st.update(200, &t, &payload(false, false, -1), None));
    let g = |k: &str, s: &EngineControlState| {
        let x = s.gauge_by_key(k).unwrap();
        (x.gauge.gauge.cur_value, x.gauge.gauge.display_value.clone())
    };
    assert_eq!(g("throttle", &st), (55, "55".to_string()));
    assert_eq!(g("pitch", &st), (60, "60".to_string()));
    assert_eq!(g("mixture", &st), (100, "100".to_string()));
    assert_eq!(g("fuel", &st), (64, "64".to_string()));
    // COMPRESSOR: stage=2 → 条值 1, 显示 "2"; MarkedGauge buffer 通道
    let comp = st.gauge_by_key("compressor").unwrap();
    assert_eq!(comp.gauge.gauge.cur_value, 1);
    assert_eq!(comp.gauge.gauge.display_value, "2");
    let mg = comp.marked_gauge.as_ref().unwrap();
    assert_eq!((mg.value_len, mg.value_buffer.as_str()), (1, "2"));
    // 无 optimal 数据 (payload -1 / stages None) → 标记隐藏
    assert_eq!(mg.markers[0].ratio, -1.0);

    // 节流: refreshInterval 默认 100 (配置未接), +50ms 拒绝且数据不更新
    t.throttle = 77.0;
    assert!(!st.update(250, &t, &payload(false, false, -1), None));
    assert_eq!(g("throttle", &st).0, 55, "节流内跳过, 值不更新");

    // PITCH -1 (自动桨): 整条隐藏且值不更新
    let mut t2 = MockTele { rpm_throttle: -1.0, ..t };
    t2.throttle = 60.0;
    assert!(st.update(400, &t2, &payload(false, false, -1), None));
    let pitch = st.gauge_by_key("pitch").unwrap();
    assert!(!pitch.visible);
    assert_eq!(pitch.gauge.gauge.cur_value, 60, "隐藏时不更新");
    assert_eq!(g("throttle", &st).0, 60);

    // MIXTURE -1 → 隐藏; COMPRESSOR stage 0 → 隐藏
    let t3 = MockTele { mixture: -1.0, compressor_stage: 0.0, ..t2 };
    assert!(st.update(600, &t3, &payload(false, false, -1), None));
    assert!(!st.gauge_by_key("mixture").unwrap().visible);
    assert!(!st.gauge_by_key("compressor").unwrap().visible);
}

/// 喷气机闩锁 + 增压器量程一次性写入 + optimal 标记比率
#[test]
fn engine_control_jet_latch_and_compressor_range() {
    let l = lang();
    let mut st = EngineControlState::new(&l, 0, 1.0, &|_| false, &|_| String::new());
    let t = MockTele {
        throttle: 100.0,
        compressor_stage: 1.0,
        ..MockTele::default()
    };
    // 引擎检测完成 + 喷气机 + 3 档增压器
    assert!(st.update(200, &t, &payload(true, true, 1), Some(3)));
    assert!(st.is_jet());
    let comp = st.gauge_by_key("compressor").unwrap();
    assert_eq!(comp.gauge.gauge.max_value, 2, "量程 = stages-1");
    assert_eq!(comp.marked_gauge.as_ref().unwrap().max_value, 2.0);
    assert_eq!(comp.marked_gauge.as_ref().unwrap().markers[0].ratio, 0.5, "optimal 1/2");
    // 闩锁: 后续事件 is_jet=false 不翻转; 量程一次性 (stages 变 5 不改)
    assert!(st.update(400, &t, &payload(false, true, -1), Some(5)));
    assert!(st.is_jet(), "jetLabelUpdated 闩锁");
    assert_eq!(st.gauge_by_key("compressor").unwrap().gauge.gauge.max_value, 2);
    assert_eq!(
        st.gauge_by_key("compressor").unwrap().marked_gauge.as_ref().unwrap().markers[0].ratio,
        -1.0,
        "optimal 无效 → 隐藏"
    );
    // 喷气机隐藏仪表: 更新跳过 (值保持)
    let mixture_before = st.gauge_by_key("mixture").unwrap().gauge.gauge.cur_value;
    let t2 = MockTele { mixture: 50.0, ..t };
    assert!(st.update(600, &t2, &payload(false, true, -1), None));
    assert_eq!(st.gauge_by_key("mixture").unwrap().gauge.gauge.cur_value, mixture_before);
}

/// drawGauges 布局: 竖条 (x+dx, y-4fs) dx += (5fs)>>1; 横条 (x, y+dy) dy += fs+fs>>2;
/// jet 隐藏仪表不画
#[test]
fn engine_control_draw_layout() {
    let l = lang();
    let font_label = bold(12); // round(24/2)
    let mut st = EngineControlState::new(&l, 0, 1.0, &|_| false, &|_| String::new());
    let t = MockTele {
        throttle: 55.0, // pix = round(55*96/110) = 48
        rpm_throttle: 60.0,
        power_percent: 80.0,
        mixture: 60.0, // pix = round(60*96/120) = 48
        radiator: 30.0,
        compressor_stage: 1.0,
        fuel_percent: 64.0,
        ..MockTele::default()
    };
    assert!(st.update(200, &t, &payload(false, false, -1), None));
    let mut cv = PixCanvas::new(st.width, st.height).unwrap();
    st.draw(&mut cv, &font_label, false);
    // x = 12, y = 168; 竖条 top = y-4fs = 72
    let tw1 = font_label.measure("节") + font_label.measure("55");
    let bar1_x = 12 + tw1 + 2;
    assert_eq!(alpha(&cv, bar1_x, 72), colors().shade_shape[3], "竖条1 环左上");
    assert_eq!(alpha(&cv, bar1_x + 11, 100), colors().shade_shape[3], "竖条1 环右边");
    assert_eq!(alpha(&cv, bar1_x + 3, 110), 0, "填充上方 (val=48 → rows 119+)");
    assert_eq!(alpha(&cv, bar1_x + 3, 130), colors().num[3], "填充段内");
    // 竖条2 (pitch): dx = (5*24)>>1 = 60
    let tw2 = font_label.measure("桨") + font_label.measure("60");
    let bar2_x = 12 + 60 + tw2 + 2;
    assert_eq!(alpha(&cv, bar2_x, 72), colors().shade_shape[3], "竖条2 环左上 (dx 推进)");
    // 横条 (mixture, 第一个横向): (12, 168+12=180)
    assert_eq!(alpha(&cv, 12, 180), colors().shade_shape[3], "横条环左上");
    assert_eq!(alpha(&cv, 14, 182), colors().num[3], "横条填充内");
    assert_eq!(alpha(&cv, 70, 182), 0, "横条填充外 (val=48)");
    // 横条第 2 行 (radiator, y=210) 在非 jet 下存在
    assert_eq!(alpha(&cv, 12, 210), colors().shade_shape[3], "radiator 第二横行");
    // 喷气机: 隐藏 mixture/radiator/compressor (FUEL 不在 isJetHiddenGauge 列表,
    // 仍画在第一横行 y=180); 第二横行无输出
    let mut st_jet = EngineControlState::new(&l, 0, 1.0, &|_| false, &|_| String::new());
    let t_jet = MockTele { mixture: 60.0, ..t };
    assert!(st_jet.update(200, &t_jet, &payload(true, true, -1), None));
    let mut cv2 = PixCanvas::new(st_jet.width, st_jet.height).unwrap();
    st_jet.draw(&mut cv2, &font_label, false);
    assert_eq!(alpha(&cv2, 12, 180), colors().shade_shape[3], "jet 下 fuel 横条仍在 (第一横行)");
    assert_eq!(alpha(&cv2, 12, 210), 0, "jet 隐藏 radiator → 第二横行空");
    assert_eq!(alpha(&cv2, bar1_x, 72), colors().shade_shape[3], "jet 保留竖条");
}

/// 预览闭包工厂: 尺寸/内容 (半量程 + optimal 示例标记)
#[test]
fn engine_control_preview_spec_renders() {
    let l = lang();
    let mut spec = engine_control_preview_spec(std::path::Path::new(FONTS), &l, 0, 1.0).unwrap();
    assert_eq!((spec.width, spec.height), (192, 306));
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0));
}

// ---- GearFlaps ----

/// reinitConfig 几何 + drawTick 状态机 (含 gear<0 保留旧告警)
#[test]
fn gear_flaps_geometry_and_tick() {
    let l = lang();
    let mut st = GearFlapsState::new(0, 1.0, false);
    assert_eq!(
        (st.font_size, st.bar_width, st.bar_height, st.width, st.height),
        (24, 12, 96, 48, 120)
    );
    assert_eq!((st.total_width, st.total_height), (144, 120));
    assert_eq!((st.flap_pix, st.flap_text.as_str()), (48, " 50"));
    // 边框: sw=10
    let st_e = GearFlapsState::new(0, 1.0, true);
    assert_eq!((st_e.total_width, st_e.total_height), (164, 140));

    // gear=100: 起落架已放 (colorNum); flaps=25; 首事件 now=0: 0-0 < 100 → 跳过
    let mut t = MockTele { gear: 100.0, flaps: 25.0, airbrake: 0.0, ..MockTele::default() };
    assert!(!st.update_tick(0, &l, &t), "首事件 now=0 被节流 (Java 同)");
    assert_eq!((st.flap_pix, st.flap_text.as_str()), (48, " 50"), "跳过时保持预览初值");
    assert!(st.update_tick(100, &l, &t));
    assert_eq!(st.warn_text, "起落架");
    assert_eq!(st.warn_color, colors().num);
    assert_eq!((st.flap_pix, st.flap_text.as_str()), (24, " 25"));
    // 节流: 100ms 间隔内 (+50ms) 拒绝且数据不更新
    t.flaps = 90.0;
    assert!(!st.update_tick(150, &l, &t));
    assert_eq!(st.flap_pix, 24, "节流内跳过, 数据保持");
    t.flaps = 25.0;
    // gear=50: 收起落告警 + 减速板 → 警告色, 文本追加
    t.gear = 50.0;
    t.airbrake = 60.0;
    assert!(st.update_tick(200, &l, &t));
    assert_eq!(st.warn_text, "收起落 减速板");
    assert_eq!(st.warn_color, colors().warning);
    // gear=0: 告警清空
    t.gear = 0.0;
    t.airbrake = 0.0;
    assert!(st.update_tick(300, &l, &t));
    assert_eq!(st.warn_text, "");
    assert_eq!(st.warn_color, colors().num);
    // flaps<0: 归零显示 "  0"
    t.flaps = -5.0;
    assert!(st.update_tick(400, &l, &t));
    assert_eq!((st.flap_pix, st.flap_text.as_str()), (0, "  0"));
    // gear<0: 保留上次告警
    t.gear = -1.0;
    t.flaps = 50.0;
    assert!(st.update_tick(500, &l, &t));
    assert_eq!(st.warn_color, colors().num, "上次为空告警 (gear=0) 保留");
    t.gear = -1.0;
    assert!(st.update_tick(600, &l, &t));
    assert_eq!(st.flap_pix, 48);
}

/// paintComponent 像素: 竖条环/填充 + 指针横线 + "F 50" 文本 + 告警文本
#[test]
fn gear_flaps_draw_pixels() {
    let l = lang();
    let mut st = GearFlapsState::new(0, 1.0, false);
    let font_num = bold(24);
    let font_label = bold(12);
    st.update_tick(100, &l, &MockTele {
        gear: 100.0, flaps: 50.0, airbrake: 0.0, ..MockTele::default()
    });
    let mut cv = PixCanvas::new(st.total_width, st.total_height).unwrap();
    st.draw(&mut cv, &font_num, &font_label, false);
    // dy = 12+96 = 108; 条盒 rows 12..107 × cols 0..11
    assert_eq!(alpha(&cv, 0, 12), colors().shade_shape[3], "条环左上");
    assert_eq!(alpha(&cv, 11, 107), colors().shade_shape[3], "条环右下");
    // 填充: (1, 108+1-48=61, 10, 46) rows 61..106; 指针线行 59..61 叠在 61 上
    assert_eq!(alpha(&cv, 5, 65), colors().num[3], "填充内");
    assert_eq!(alpha(&cv, 5, 55), 0, "填充上方");
    // 指针横线 drawHRect(0, 108-48-1=59, 12+36=48, 3): 内芯行 60 colorLabel
    assert_eq!(alpha(&cv, 5, 60), colors().label[3], "指针线内芯");
    // (0,59) = 指针环上边 over 条环左列 (42 over 42 → 77); 取条宽外的 (20,59)
    assert_eq!(alpha(&cv, 0, 59), 77, "指针环上边叠条环 (SrcOver)");
    assert_eq!(alpha(&cv, 20, 59), colors().shade_shape[3], "指针环上边 (条外段)");
    assert_eq!(alpha(&cv, 47, 60), colors().shade_shape[3], "指针环右边列");
    // "F 50" 文本: 基线 (12, 108-48-2=58), fontLabel
    let text_zone = (12..48).any(|x| (44..58).any(|y| alpha(&cv, x, y) > 0));
    assert!(text_zone, "襟翼数值文本存在");
    // 告警 "起落架": 基线 (width=48, fontSize=24), fontLabel
    let warn_zone = (48..100).any(|x| (10..25).any(|y| alpha(&cv, x, y) > 0));
    assert!(warn_zone, "起落架告警文本存在");
}

/// 预览闭包工厂: 尺寸 (含边框) + 内容非空
#[test]
fn gear_flaps_preview_spec_renders() {
    let mut spec = gear_flaps_preview_spec(std::path::Path::new(FONTS), 0, 1.0, true).unwrap();
    assert_eq!((spec.width, spec.height), (164, 140));
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0));
}

// ---- live 喂数形态工厂 (句柄共享: render 闭包与喂入方同一 state) ----

/// 测试参数仓 (缺省值 + 覆写便捷)
fn params_cell(mutate: impl FnOnce(&mut ReinitParams)) -> Rc<RefCell<ReinitParams>> {
    let mut p = ReinitParams::default();
    mutate(&mut p);
    Rc::new(RefCell::new(p))
}

/// 三工厂: 句柄喂入后 render 闭包画到新值 (共享 state 生效); 尺寸与 preview 工厂一致
#[test]
fn live_spec_handles_share_state_with_render() {
    let l = lang();
    let fonts = std::path::Path::new(FONTS);
    // PowerInfo: 功率 1200 → 首字段 buffer
    let (h_power, mut spec) =
        power_info_overlay_spec(fonts, &params_cell(|p| p.power_columns = 2)).unwrap();
    let t = MockTele { horse_power: 1200.0, ..MockTele::default() };
    assert!(h_power.borrow_mut().update(100, &t));
    assert_eq!(h_power.borrow().fields()[0].buffer, "1200");
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0));

    // EngineControl: throttle 80 → gauge 值; render 走 &mut 通道不 panic
    let lang_rc = Rc::new(lang());
    let (h_engine, mut spec2) = engine_control_overlay_spec(
        fonts,
        Rc::clone(&lang_rc),
        &params_cell(|p| p.service_loop_interval_ms = 50),
    )
    .unwrap();
    assert_eq!((spec2.width, spec2.height), (192, 306), "尺寸与 preview 工厂一致");

    // disable 键实效 (审查轮 1-B): 7 仪表全关 → 布局窗口显著变矮
    // (EngineControlState::new 的 calculateLayout 按存活仪表数算高)
    let (_h, spec_off) = engine_control_overlay_spec(
        fonts,
        Rc::clone(&lang_rc),
        &params_cell(|p| {
            p.service_loop_interval_ms = 50;
            p.engine_disables = [true; 7];
        }),
    )
    .unwrap();
    assert!(
        spec_off.height < 306,
        "全关 ({}) 应矮于全开 (306) — 曾 never-wired 恒显全部 7 条",
        spec_off.height
    );
    // dataPollIntervalMs=50 → refreshInterval=100 (loadRefreshInterval ×2)
    assert_eq!(h_engine.borrow().refresh_interval, 100);
    let t2 = MockTele { throttle: 80.0, ..MockTele::default() };
    assert!(h_engine.borrow_mut().update(200, &t2, &payload(false, false, -1), None));
    assert_eq!(h_engine.borrow().gauge_by_key("throttle").unwrap().gauge.gauge.cur_value, 80);
    let mut cv2 = PixCanvas::new(spec2.width, spec2.height).unwrap();
    (spec2.render)(&mut cv2);
    assert!(cv2.pixmap().data().iter().any(|&b| b != 0));

    // GearFlaps: gear=100/flaps=25 → 告警文本 + flap_pix
    let (h_gear, mut spec3) = gear_flaps_overlay_spec(fonts, &params_cell(|_| {})).unwrap();
    let t3 = MockTele { gear: 100.0, flaps: 25.0, ..MockTele::default() };
    assert!(h_gear.borrow_mut().update_tick(100, &l, &t3));
    assert_eq!(h_gear.borrow().flap_pix, 24);
    let mut cv3 = PixCanvas::new(spec3.width, spec3.height).unwrap();
    (spec3.render)(&mut cv3);
    assert!(cv3.pixmap().data().iter().any(|&b| b != 0));
}

// ---- WYSIWYG reinit (Java reinitConfig → 新 preferred_size/setBounds) ----

/// PowerInfo: fontadd 0→6 → reinit 闭包返回更大高度 (host 侧走 resize_entry)
#[test]
fn power_info_reinit_grows_with_font_add() {
    let fonts = std::path::Path::new(FONTS);
    let cell = params_cell(|_| {});
    let (_h, mut spec) = power_info_overlay_spec(fonts, &cell).unwrap();
    let h0 = spec.height;
    cell.borrow_mut().font_add_power = 6;
    let (w1, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(
        h1 > h0,
        "字号增量 0→6 后高度应变大 ({} → {})",
        h0,
        h1
    );
    assert!(w1 > 0);
}

/// EngineControl: fontadd 0→6 → 高度变大; 7 仪表全关 → 显著变矮 (disable 生效)
#[test]
fn engine_control_reinit_resizes_for_font_and_disables() {
    let fonts = std::path::Path::new(FONTS);
    let cell = params_cell(|_| {});
    let (h, mut spec) =
        engine_control_overlay_spec(fonts, Rc::new(lang()), &cell).unwrap();
    let h0 = spec.height;
    cell.borrow_mut().font_add_engine = 6;
    let (_, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h1 > h0, "字号增量后高度应变大 ({} → {})", h0, h1);
    // 全关: 存活仪表 0 → 布局显著变矮 (state 已重建, live 值复位为预览半量程)
    cell.borrow_mut().engine_disables = [true; 7];
    let (_, h2) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h2 < h1, "全关仪表后应显著变矮 ({} → {})", h1, h2);
    assert!(h.borrow().gauge_by_key("throttle").is_none(), "全关后 throttle 仪表移除");
}

/// GearFlaps: fontadd 0→6 → 总尺寸变大; 边缘开关 → sw=10 外扩 (Java sw·2)
#[test]
fn gear_flaps_reinit_grows_with_font_and_edge() {
    let fonts = std::path::Path::new(FONTS);
    let cell = params_cell(|_| {});
    let (h, mut spec) = gear_flaps_overlay_spec(fonts, &cell).unwrap();
    let (w0, h0) = (spec.width, spec.height);
    cell.borrow_mut().gear_show_edge = true;
    let (we, _) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert_eq!(we - w0, 20, "enablegearAndFlapsEdge → sw=10 双侧外扩");
    // 字号 0→6: 更高 (state 重建, 预览复位: flap 50%)
    cell.borrow_mut().font_add_gear = 6;
    cell.borrow_mut().gear_show_edge = false;
    let (_, h2) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h2 > h0, "字号增量后高度应变大 ({} → {})", h0, h2);
    assert_eq!(h.borrow().flap_pix, h.borrow().bar_height * 50 / 100, "reinit 复位预览 50%");
}
