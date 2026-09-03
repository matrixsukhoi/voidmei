//! Field 系 overlay 组件的域级集成测试 (波10 合并: 原 overlays_field1/field2
//! 壳下 tests.rs 迁此 — 共享 bold/px/字体助手与跨组件用例, 先例 fm::store_tests)。
//! 取数面 = 下方显式 use (波16 裁撤 mod.rs 转发面, 单一真相路径)。

use super::control_surfaces::{control_surfaces_overlay_spec, ControlSurfacesOverlay, CsFonts};
use super::engine_control::{engine_control_overlay_spec, EngineControlState};
use super::fm_unpacked::{
    add_lines, fm_unpacked_data_overlay_spec, generate_lines, FmUnpackedDataOverlay, FmUnpackedFeed,
};
use super::gauges::{GaugeBarStyle, GaugeMarker, MarkedGauge, MarkerType};
use super::gear_flaps::{gear_flaps_overlay_spec, GearFlapsState};
use super::power_info::{power_info_overlay_spec, PowerInfoState};
use crate::layout::ui_constants::ENGINE_DEFAULT_REFRESH_MS;
use crate::platform::host::OverlayHost;
use crate::platform::reinit::ReinitParams;
use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;
use crate::render::palette::{aa, colors};
#[cfg(test)]
use crate::render::primitives::butt_line;
use crate::render::renderers::{BosStyleRenderer, RenderContext};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use vm_core::base::event::EventPayload;
use vm_core::base::format::{fmt_f, java_string_format, FmtArg};
use vm_core::config::config_api::ConfigProvider;
use vm_core::fm::data::{FmData, FmParts};
use vm_core::fm::FMManager;
use vm_core::formula::registry::FormulaView;
use vm_core::lang::Lang;

// ==== 原 overlays_field1/tests.rs (engine_control/gauges/gear_flaps/power_info) ====

const FONTS: &str = "../../../fonts";

fn bold(size: i32) -> LoadedFont {
    LoadedFont::new(
        std::path::Path::new(FONTS)
            .join("sarasa-mono-sc-bold.ttf")
            .as_path(),
        size,
    )
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
/// (全签名锁定模式 — 对齐既有 parser 测试 mock 家族)
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
impl FormulaView for MockTele {
    // W7: var_value 桩 (字段直映); 名字先经 canonical 可达性检查 — 消费方以
    // 短名发问 (PowerSource::getter / VisExpr / 仪表, W10 单名制),
    // 桩只答可达名, 未注册名 None (对位生产行为, 不做 _ => 0.0 兜底)
    fn var_value(&self, name: &str) -> Option<f64> {
        let name = crate::overlays::flight_info::canonical_var_name(name)?;
        Some(match name.as_str() {
            "throttle" => self.throttle,
            "rpm_throttle" => self.rpm_throttle,
            "power_percent" => self.power_percent,
            "mixture_state" => self.mixture,
            "radiator" => self.radiator,
            "compressor_stage" => self.compressor_stage,
            "fuel_percent" => self.fuel_percent,
            "gear" => self.gear,
            "flaps" => self.flaps,
            "airbrake" => self.airbrake,
            "horse_power" => self.horse_power,
            "thrust" => self.thrust,
            "rpm" => self.rpm,
            "prop_pitch" => self.pitch,
            "prop_efficiency" => self.prop_eff,
            "eff_hp" => self.eff_hp,
            "manifold_pressure_display" => self.manifold,
            "manifold_pressure" => self.manifold,
            "mass_fuel" => self.mass_fuel,
            "total_weight" => self.total_weight,
            "fuel_time_mili" => self.fuel_time_mili as f64,
            "wep_kg" => self.wep_kg,
            "wep_time" => self.wep_time,
            "booster_fuel_kg" => self.booster_kg,
            "booster_fuel_percent" => self.booster_pct,
            "has_booster" => self.booster as u8 as f64,
            "water_temp" => self.water_temp,
            "oil_temp" => self.oil_temp,
            "heat_tolerance" => self.heat_tol,
            "engine_response" => self.engine_resp,
            "is_jet_engine" => self.jet as u8 as f64,
            "is_piston_engine" => self.piston as u8 as f64,
            "has_wep" => self.wep as u8 as f64,
            "is_imperial" => (self.manifold_prec == 1) as u8 as f64,
            _ => 0.0,
        })
    }
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
/// w=2 竖线镜像 = 列 x-1..x × 行 ya..yb-1 (与 primitives::butt_line 文档互证)
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
    assert!(!vm_core::ui_support::row_def::Cond::IsJetEngine.eval(&t, 0.0));
    assert!(vm_core::ui_support::row_def::Cond::IsPistonEngine.eval(&t, 0.0));
    assert!(!vm_core::ui_support::row_def::Cond::HasWep.eval(&t, 0.0));
    assert!(vm_core::ui_support::row_def::Cond::Gt(0.0).eval(&t, 0.1));
    assert!(!vm_core::ui_support::row_def::Cond::Gt(0.0).eval(&t, 0.0));
    assert!(vm_core::ui_support::row_def::Cond::Lte(0.0).eval(&t, 0.0));
    assert!(vm_core::ui_support::row_def::Cond::Eq(1.0).eval(&t, 1.00001));
    assert!(!vm_core::ui_support::row_def::Cond::Eq(1.0).eval(&t, 1.0002));
    // f64 边界: 字面量 1.0001-1.0 实际差 ≈ 9.9999e-5 < 0.0001 → 视为相等
    // (vm-core 求值器测试同款 基线: "(= value 1)" 对 1.0001 为 true)
    assert!(vm_core::ui_support::row_def::Cond::Eq(1.0).eval(&t, 1.0001));
    assert!(!vm_core::ui_support::row_def::Cond::NotEq(1.0).eval(&t, 1.0001));
    assert!(!vm_core::ui_support::row_def::Cond::NotEq(1.0).eval(&t, 1.0));
    let not_jet = vm_core::ui_support::row_def::Cond::Not(Box::new(
        vm_core::ui_support::row_def::Cond::IsJetEngine,
    ));
    assert!(not_jet.eval(&t, 0.0));
    let and = vm_core::ui_support::row_def::Cond::And(
        Box::new(vm_core::ui_support::row_def::Cond::IsPistonEngine),
        Box::new(vm_core::ui_support::row_def::Cond::NotEq(1.0)),
    );
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
    assert_eq!(
        alpha(&cv, 57, 30),
        colors().num[3],
        "填充右下内 (row31 与环底行重叠)"
    );
    // (58,25) 为分隔线主线列 (x+pixVal); 其右 2px 才是填充外
    assert_eq!(alpha(&cv, 60, 25), 0, "填充/分隔线右外");
    // 边框环 (drawRect(10,20,95,11)): 右边框列 105 纯 shade
    assert_eq!(alpha(&cv, 105, 25), colors().shade_shape[3]);
    assert_eq!(
        alpha(&cv, 10, 20),
        242,
        "左上角 = shade over fill (SrcOver)"
    );
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
    assert!(
        alpha(&cv, 33, 25) > colors().num[3],
        "标记左列 = warning over fill"
    );
    assert!(alpha(&cv, 34, 25) > colors().num[3], "标记右列");
    assert_eq!(alpha(&cv, 32, 25), colors().num[3], "标记左外 = 纯填充");
    assert_eq!(alpha(&cv, 35, 25), colors().num[3], "标记右外 = 纯填充");
    // 标记端行: Java 线终点 y=barY+thickness(=32) → 中心规则行 20..31 亮;
    // 行 31 = warning over fill 再叠边框环底 ≈247, 行 32 (条外) 无标记
    assert!(alpha(&cv, 34, 31) > colors().num[3], "标记贯穿到条底行");
    assert_eq!(alpha(&cv, 34, 32), 0, "条外行无标记");
    // 分隔线承袭 tickStroke (2px): 主线列 57..58, 影线列 58..59 (先主线后影线,
    // col58 = 影线 over 主线 ≈243); 延伸行到 46
    assert_eq!(
        alpha(&cv, 57, 45),
        colors().num[3],
        "主线 (2px) 左列 (纯主线)"
    );
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
    assert_eq!(
        alpha(&cv, bar_x + 3, 10),
        colors().shade_shape[3],
        "背景条顶端"
    );
    // 填充自底向上: pixVal = 20 → rows 30..49; 背景在 rows 10..29
    assert_eq!(
        alpha(&cv, bar_x + 3, 15),
        colors().shade_shape[3],
        "上半背景"
    );
    // 下半填充 = colorNum SrcOver colorShadeShape 背景 → alpha ≈ 243 (双层)
    assert!(
        (242..=244).contains(&alpha(&cv, bar_x + 3, 35)),
        "下半填充 (叠背景)"
    );
    // 分隔线: sepY = 10+40-1-20 = 29, 环 29..31 + fill 内芯行 30
    assert_eq!(alpha(&cv, 5, 29), colors().shade_shape[3], "分隔环上边");
    // 内芯行 30: 取文本与条之间的列 (bar_x-1), 避开文本 descender
    assert_eq!(alpha(&cv, bar_x - 1, 30), colors().num[3], "分隔内芯");
}

// ---- PowerInfo ----

/// 测试 defs: 从仓库 ui_layout.cfg 编译 "动力信息" 19 行 (与生产同源)
fn defs19() -> std::sync::Arc<Vec<vm_core::ui_support::row_def::RowDef>> {
    std::sync::Arc::new(crate::overlays::flight_info::cfg_rows("动力信息"))
}

/// cfg 驱动快照 (W-D): "动力信息" 组 19 行, 关键行 (进气压动态通道 / 燃油时
/// TIME_MM_SS) 与原静态表逐值一致 — cfg 是行定义唯一来源的守卫锚
#[test]
fn power_field_defs_snapshot() {
    use vm_core::ui_support::row_def::{Cond, DisplayMode, FormatKind};
    let defs = crate::overlays::flight_info::cfg_rows("动力信息");
    assert_eq!(defs.len(), 19);
    assert_eq!(defs[0].label, "功  率");
    assert_eq!(defs[0].na_when, Some(Cond::Lte(0.0)));
    let manifold = &defs[6];
    assert_eq!(manifold.label, "进气压");
    assert_eq!(
        manifold.display,
        DisplayMode::ImperialManifold,
        "进气压动态单位/精度通道"
    );
    assert_eq!(
        manifold.visible_when,
        Some(Cond::And(
            Box::new(Cond::IsPistonEngine),
            Box::new(Cond::NotEq(1.0))
        ))
    );
    let fuel_time = &defs[10];
    assert_eq!(fuel_time.source, "fuel_time_mili * 0.001");
    assert_eq!(fuel_time.format, FormatKind::TimeMmSs);
    assert_eq!(defs[17].na_when, Some(Cond::Gt(90000.0)));
}

/// 更新路径 (FieldOverlay.onFlightData 零 GC): visible-when / na-when "-" /
/// TIME_MM_SS / 动态单位精度 / 预览不受影响
#[test]
fn power_info_update_paths() {
    let mut st = PowerInfoState::new(defs19());
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
    assert_eq!(
        (find("功  率").buffer.as_str(), find("功  率").length),
        ("1200", 4)
    );
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
    assert_eq!(
        st.fields()
            .iter()
            .find(|x| x.label == "转  速")
            .unwrap()
            .buffer,
        "2400"
    );
    t.rpm = 2400.0;
    // na-when: 功率 0 → "-"
    t.horse_power = 0.0;
    assert!(st.update(200, &t));
    assert_eq!(st.fields()[0].buffer, "-");
    assert_eq!(st.fields()[0].length, 1);
    // 动态单位/精度: 英制 (is_imperial) → Java "P/x.x''" + 1 位 (生产: unit/prec
    // 同源 is_imperial, 不可独立分叉; 旧桩的独立 unit/prec 字段属想象行为已删)
    t.manifold_prec = 1;
    t.manifold = 44.6;
    assert!(st.update(300, &t));
    let m = st.fields().iter().find(|x| x.label == "进气压").unwrap();
    let inhg = vm_core::base::format::format(44.6 * 760.0 / 25.4, 1);
    assert_eq!(m.unit, format!("P/{inhg}''"));
    assert_eq!(m.buffer, "44.6");
    assert_eq!(m.precision, 1);
    // 喷气机: 功率/桨距角/桨效率/实功率 隐藏, 推力仍在
    t.jet = true;
    t.piston = false;
    assert!(st.update(400, &t));
    assert!(!st.fields()[0].visible, "喷气机隐藏功率");
    assert!(
        st.fields()
            .iter()
            .find(|x| x.label == "推  力")
            .unwrap()
            .visible
    );
    // 进气压 value=1 (容差内) → 活塞机也隐藏
    let mut st2 = PowerInfoState::new(defs19());
    let mut t2 = MockTele {
        manifold: 1.0,
        ..MockTele::default()
    };
    t2.piston = true;
    assert!(st2.update(100, &t2));
    assert!(
        !st2.fields()
            .iter()
            .find(|x| x.label == "进气压")
            .unwrap()
            .visible
    );
}

/// 预览 = 构造后不 update: previewValue 原样落 currentValue, 全部可见
#[test]
fn power_info_preview_state() {
    let st = PowerInfoState::new(defs19());
    assert!(st.fields().iter().all(|f| f.visible));
    assert_eq!(st.fields()[0].current_value, "1200");
    assert_eq!(
        st.fields()
            .iter()
            .find(|x| x.label == "进气压")
            .unwrap()
            .current_value,
        "1.2"
    );
    assert_eq!(st.fields().iter().filter(|f| f.visible).count(), 19);
}

/// CloseAllOverlays 数据面重置 (app_shell reset_handles_preview_values 调用面):
/// live 残留 (buffer/可见性/节流基准) → reset_preview → 构造态 previewValue 静态。
/// 场景: 托盘 live→preview 后重开的预览窗不得显示上次 live 数值
#[test]
fn power_info_reset_preview_restores_statics() {
    let mut st = PowerInfoState::new(defs19());
    // 活塞机形态 (buffer 写入面) + 助推标志 false (live 驱动的可见性残留面)
    let t = MockTele {
        horse_power: 1200.0,
        manifold: 0.98,
        piston: true,
        ..MockTele::default()
    };
    assert!(st.update(100, &t));
    assert_eq!(st.fields()[0].buffer, "1200", "live 已写 buffer");
    assert!(
        !st.fields()
            .iter()
            .find(|x| x.label == "助推燃料")
            .unwrap()
            .visible,
        "助推 false → live 隐藏 (preview 构造态为全可见)"
    );
    // 重置 → 构造态: buffer 清空 / 全可见 / currentValue 回 previewValue
    st.reset_preview();
    assert!(st
        .fields()
        .iter()
        .all(|f| f.length == 0 && f.buffer.is_empty()));
    assert!(
        st.fields().iter().all(|f| f.visible),
        "可见性回构造态 (live 残留清除)"
    );
    assert_eq!(st.fields()[0].current_value, "1200");
    assert_eq!(
        st.fields()
            .iter()
            .find(|x| x.label == "进气压")
            .unwrap()
            .current_value,
        "1.2"
    );
    assert_eq!(st.last_refresh_time, 0, "节流基准复位 (重进游戏首帧不误吞)");
}

/// BOS 网格绘制: 出像素 (工厂闭包形态由 live 工厂测试覆盖)
#[test]
fn power_info_draw_renders() {
    let ctx = RenderContext::load(std::path::Path::new(FONTS), 0, 2).unwrap();
    let st = PowerInfoState::new(defs19());
    let (w, h) = st.preferred_size(&ctx);
    let mut cv = PixCanvas::new(w, h).unwrap();
    let mut renderer = BosStyleRenderer::default();
    st.draw(&mut cv, &ctx, &mut renderer);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0), "预览网格非空");
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
    assert_eq!(
        st.gauges()
            .iter()
            .filter(|g| g.marked_gauge.is_some())
            .count(),
        1
    );
    // 禁用探测: disableEngineInfoThrottle=true → 6 仪表, 高度重算 (row_num 仍 4)
    let st2 = EngineControlState::new(&l, 0, 1.0, &|k| k == "disableEngineInfoThrottle", &|_| {
        String::new()
    });
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
    assert_eq!(
        (
            thr.gauge.gauge.cur_value,
            thr.gauge.gauge.display_value.as_str()
        ),
        (55, "55")
    );
    let comp = st.gauge_by_key("compressor").unwrap();
    let mg = comp.marked_gauge.as_ref().unwrap();
    // max=1 → val=0 → 显示 1 基档号 "1" (display 通道)
    assert_eq!(
        (mg.current_value, mg.value_len, mg.display_value.as_str()),
        (0.0, 0, "1")
    );
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
    let st = EngineControlState::new(
        &l,
        0,
        1.0,
        &|_| false,
        &cfg_of(&[("dataPollIntervalMs", "50")]),
    );
    assert_eq!(st.refresh_interval, 100);
    // legacy "Interval" 回退: 33 → 66
    let st2 = EngineControlState::new(&l, 0, 1.0, &|_| false, &cfg_of(&[("Interval", "33")]));
    assert_eq!(st2.refresh_interval, 66);
    // dataPollIntervalMs 优先于 legacy
    let st3 = EngineControlState::new(
        &l,
        0,
        1.0,
        &|_| false,
        &cfg_of(&[("dataPollIntervalMs", "20"), ("Interval", "999")]),
    );
    assert_eq!(st3.refresh_interval, 40);
    // 解析失败 → parseLongSafe 默认 100 → ×2 = 200
    let st4 = EngineControlState::new(
        &l,
        0,
        1.0,
        &|_| false,
        &cfg_of(&[("dataPollIntervalMs", "abc")]),
    );
    assert_eq!(st4.refresh_interval, 200);
    // 双键空 → 保持字段初始默认 100 (POC 空配置读取器同此)
    let st5 = EngineControlState::new(&l, 0, 1.0, &|_| false, &|_| String::new());
    assert_eq!(st5.refresh_interval, ENGINE_DEFAULT_REFRESH_MS);

    // 节流随间隔生效: interval=100 (dataPollIntervalMs=50) — 首事件 0 跳过,
    // +50 拒绝, +100 放行
    let mut st6 = EngineControlState::new(
        &l,
        0,
        1.0,
        &|_| false,
        &cfg_of(&[("dataPollIntervalMs", "50")]),
    );
    let t = MockTele::default();
    assert!(
        !st6.update(0, &t, &payload(false, false, -1), None),
        "0-0 < 100 跳过"
    );
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
    let mut t2 = MockTele {
        rpm_throttle: -1.0,
        ..t
    };
    t2.throttle = 60.0;
    assert!(st.update(400, &t2, &payload(false, false, -1), None));
    let pitch = st.gauge_by_key("pitch").unwrap();
    assert!(!pitch.visible);
    assert_eq!(pitch.gauge.gauge.cur_value, 60, "隐藏时不更新");
    assert_eq!(g("throttle", &st).0, 60);

    // MIXTURE -1 → 隐藏; COMPRESSOR stage 0 → 隐藏
    let t3 = MockTele {
        mixture: -1.0,
        compressor_stage: 0.0,
        ..t2
    };
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
    assert_eq!(
        comp.marked_gauge.as_ref().unwrap().markers[0].ratio,
        0.5,
        "optimal 1/2"
    );
    // 闩锁: 后续事件 is_jet=false 不翻转; 量程一次性 (stages 变 5 不改)
    assert!(st.update(400, &t, &payload(false, true, -1), Some(5)));
    assert!(st.is_jet(), "jetLabelUpdated 闩锁");
    assert_eq!(
        st.gauge_by_key("compressor").unwrap().gauge.gauge.max_value,
        2
    );
    assert_eq!(
        st.gauge_by_key("compressor")
            .unwrap()
            .marked_gauge
            .as_ref()
            .unwrap()
            .markers[0]
            .ratio,
        -1.0,
        "optimal 无效 → 隐藏"
    );
    // 喷气机隐藏仪表: 更新跳过 (值保持)
    let mixture_before = st.gauge_by_key("mixture").unwrap().gauge.gauge.cur_value;
    let t2 = MockTele { mixture: 50.0, ..t };
    assert!(st.update(600, &t2, &payload(false, true, -1), None));
    assert_eq!(
        st.gauge_by_key("mixture").unwrap().gauge.gauge.cur_value,
        mixture_before
    );
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
    assert_eq!(
        alpha(&cv, bar1_x, 72),
        colors().shade_shape[3],
        "竖条1 环左上"
    );
    assert_eq!(
        alpha(&cv, bar1_x + 11, 100),
        colors().shade_shape[3],
        "竖条1 环右边"
    );
    assert_eq!(
        alpha(&cv, bar1_x + 3, 110),
        0,
        "填充上方 (val=48 → rows 119+)"
    );
    assert_eq!(alpha(&cv, bar1_x + 3, 130), colors().num[3], "填充段内");
    // 竖条2 (pitch): dx = (5*24)>>1 = 60
    let tw2 = font_label.measure("桨") + font_label.measure("60");
    let bar2_x = 12 + 60 + tw2 + 2;
    assert_eq!(
        alpha(&cv, bar2_x, 72),
        colors().shade_shape[3],
        "竖条2 环左上 (dx 推进)"
    );
    // 横条 (mixture, 第一个横向): (12, 168+12=180)
    assert_eq!(alpha(&cv, 12, 180), colors().shade_shape[3], "横条环左上");
    assert_eq!(alpha(&cv, 14, 182), colors().num[3], "横条填充内");
    assert_eq!(alpha(&cv, 70, 182), 0, "横条填充外 (val=48)");
    // 横条第 2 行 (radiator, y=210) 在非 jet 下存在
    assert_eq!(
        alpha(&cv, 12, 210),
        colors().shade_shape[3],
        "radiator 第二横行"
    );
    // 喷气机: 隐藏 mixture/radiator/compressor (FUEL 不在 isJetHiddenGauge 列表,
    // 仍画在第一横行 y=180); 第二横行无输出
    let mut st_jet = EngineControlState::new(&l, 0, 1.0, &|_| false, &|_| String::new());
    let t_jet = MockTele { mixture: 60.0, ..t };
    assert!(st_jet.update(200, &t_jet, &payload(true, true, -1), None));
    let mut cv2 = PixCanvas::new(st_jet.width, st_jet.height).unwrap();
    st_jet.draw(&mut cv2, &font_label, false);
    assert_eq!(
        alpha(&cv2, 12, 180),
        colors().shade_shape[3],
        "jet 下 fuel 横条仍在 (第一横行)"
    );
    assert_eq!(alpha(&cv2, 12, 210), 0, "jet 隐藏 radiator → 第二横行空");
    assert_eq!(
        alpha(&cv2, bar1_x, 72),
        colors().shade_shape[3],
        "jet 保留竖条"
    );
}

// ---- GearFlaps ----

/// reinitConfig 几何 + drawTick 状态机 (含 gear<0 保留旧告警)
#[test]
fn gear_flaps_geometry_and_tick() {
    let l = lang();
    let mut st = GearFlapsState::new(0, 1.0, false);
    assert_eq!(
        (
            st.font_size,
            st.bar_width,
            st.bar_height,
            st.width,
            st.height
        ),
        (24, 12, 96, 48, 120)
    );
    assert_eq!((st.total_width, st.total_height), (144, 120));
    assert_eq!((st.flap_pix, st.flap_text.as_str()), (48, " 50"));
    // 边框: sw=10
    let st_e = GearFlapsState::new(0, 1.0, true);
    assert_eq!((st_e.total_width, st_e.total_height), (164, 140));

    // gear=100: 起落架已放 (colorNum); flaps=25; 首事件 now=0: 0-0 < 100 → 跳过
    let mut t = MockTele {
        gear: 100.0,
        flaps: 25.0,
        airbrake: 0.0,
        ..MockTele::default()
    };
    assert!(!st.update_tick(0, &l, &t), "首事件 now=0 被节流 (Java 同)");
    assert_eq!(
        (st.flap_pix, st.flap_text.as_str()),
        (48, " 50"),
        "跳过时保持预览初值"
    );
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
    st.update_tick(
        100,
        &l,
        &MockTele {
            gear: 100.0,
            flaps: 50.0,
            airbrake: 0.0,
            ..MockTele::default()
        },
    );
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
    assert_eq!(
        alpha(&cv, 20, 59),
        colors().shade_shape[3],
        "指针环上边 (条外段)"
    );
    assert_eq!(alpha(&cv, 47, 60), colors().shade_shape[3], "指针环右边列");
    // "F 50" 文本: 基线 (12, 108-48-2=58), fontLabel
    let text_zone = (12..48).any(|x| (44..58).any(|y| alpha(&cv, x, y) > 0));
    assert!(text_zone, "襟翼数值文本存在");
    // 告警 "起落架": 基线 (width=48, fontSize=24), fontLabel
    let warn_zone = (48..100).any(|x| (10..25).any(|y| alpha(&cv, x, y) > 0));
    assert!(warn_zone, "起落架告警文本存在");
}

// ---- live 喂数形态工厂 (句柄共享: render 闭包与喂入方同一 state) ----

/// 测试参数仓 (缺省值 + 覆写便捷)
fn params_cell(mutate: impl FnOnce(&mut ReinitParams)) -> Rc<RefCell<ReinitParams>> {
    let mut p = ReinitParams::default();
    // W-D: 行定义走 cfg (与生产同源)
    p.flight.rows = std::sync::Arc::new(crate::overlays::flight_info::cfg_rows("飞行信息"));
    p.power.rows = std::sync::Arc::new(crate::overlays::flight_info::cfg_rows("动力信息"));
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
        power_info_overlay_spec(fonts, &params_cell(|p| p.power.columns = 2)).unwrap();
    let t = MockTele {
        horse_power: 1200.0,
        ..MockTele::default()
    };
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
    assert_eq!(
        (spec2.width, spec2.height),
        (192, 306),
        "尺寸与 preview 工厂一致"
    );

    // disable 键实效 (审查轮 1-B): 7 仪表全关 → 布局窗口显著变矮
    // (EngineControlState::new 的 calculateLayout 按存活仪表数算高)
    let (_h, spec_off) = engine_control_overlay_spec(
        fonts,
        Rc::clone(&lang_rc),
        &params_cell(|p| {
            p.service_loop_interval_ms = 50;
            p.engine.disables = [true; 7];
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
    let t2 = MockTele {
        throttle: 80.0,
        ..MockTele::default()
    };
    assert!(h_engine
        .borrow_mut()
        .update(200, &t2, &payload(false, false, -1), None));
    assert_eq!(
        h_engine
            .borrow()
            .gauge_by_key("throttle")
            .unwrap()
            .gauge
            .gauge
            .cur_value,
        80
    );
    let mut cv2 = PixCanvas::new(spec2.width, spec2.height).unwrap();
    (spec2.render)(&mut cv2);
    assert!(cv2.pixmap().data().iter().any(|&b| b != 0));

    // GearFlaps: gear=100/flaps=25 → 告警文本 + flap_pix
    let (h_gear, mut spec3) = gear_flaps_overlay_spec(fonts, &params_cell(|_| {})).unwrap();
    let t3 = MockTele {
        gear: 100.0,
        flaps: 25.0,
        ..MockTele::default()
    };
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
    cell.borrow_mut().power.font_add = 6;
    let (w1, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h1 > h0, "字号增量 0→6 后高度应变大 ({} → {})", h0, h1);
    assert!(w1 > 0);
}

/// EngineControl: fontadd 0→6 → 高度变大; 7 仪表全关 → 显著变矮 (disable 生效)
#[test]
fn engine_control_reinit_resizes_for_font_and_disables() {
    let fonts = std::path::Path::new(FONTS);
    let cell = params_cell(|_| {});
    let (h, mut spec) = engine_control_overlay_spec(fonts, Rc::new(lang()), &cell).unwrap();
    let h0 = spec.height;
    cell.borrow_mut().engine.font_add = 6;
    let (_, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h1 > h0, "字号增量后高度应变大 ({} → {})", h0, h1);
    // 全关: 存活仪表 0 → 布局显著变矮 (state 已重建, live 值复位为预览半量程)
    cell.borrow_mut().engine.disables = [true; 7];
    let (_, h2) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h2 < h1, "全关仪表后应显著变矮 ({} → {})", h1, h2);
    assert!(
        h.borrow().gauge_by_key("throttle").is_none(),
        "全关后 throttle 仪表移除"
    );
}

/// GearFlaps: fontadd 0→6 → 总尺寸变大; 边缘开关 → sw=10 外扩 (Java sw·2)
#[test]
fn gear_flaps_reinit_grows_with_font_and_edge() {
    let fonts = std::path::Path::new(FONTS);
    let cell = params_cell(|_| {});
    let (h, mut spec) = gear_flaps_overlay_spec(fonts, &cell).unwrap();
    let (w0, h0) = (spec.width, spec.height);
    cell.borrow_mut().gear.show_edge = true;
    let (we, _) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert_eq!(we - w0, 20, "enablegearAndFlapsEdge → sw=10 双侧外扩");
    // 字号 0→6: 更高 (state 重建, 预览复位: flap 50%)
    cell.borrow_mut().gear.font_add = 6;
    cell.borrow_mut().gear.show_edge = false;
    let (_, h2) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h2 > h0, "字号增量后高度应变大 ({} → {})", h0, h2);
    assert_eq!(
        h.borrow().flap_pix,
        h.borrow().bar_height * 50 / 100,
        "reinit 复位预览 50%"
    );
}

/// 守卫: overlay 层全部 var_value 消费名经生产双通道 (公式槽 getter 别名 +
/// registry getter 索引) 可达 — 断链即面板行消失/恒 0/仪表恒零
/// (live 显示回归的回归锚, 名单 = 逐调用点 grep 收录)
#[test]
fn all_overlay_var_consumers_reachable() {
    let canon = crate::overlays::flight_info::canonical_var_name;
    // 1. 动力信息 19 行 :target 短名 (PowerSource::getter, W10 单名制)
    let power = [
        "horse_power",
        "thrust",
        "rpm",
        "prop_pitch",
        "prop_efficiency",
        "eff_hp",
        "manifold_pressure_display",
        "power_percent",
        "mass_fuel",
        "total_weight",
        "fuel_time_mili",
        "wep_kg",
        "wep_time",
        "booster_fuel_kg",
        "booster_fuel_percent",
        "water_temp",
        "oil_temp",
        "heat_tolerance",
        "engine_response",
    ];
    for g in power {
        assert!(canon(g).is_some(), "动力信息 target {g} 解析断链");
    }
    // 2. VisExpr 判定名 (overlays_field1 eval 消费)
    for n in [
        "is_jet_engine",
        "is_piston_engine",
        "has_wep",
        "has_booster",
    ] {
        assert!(canon(n).is_some(), "VisExpr 名 {n} 未注册 (判定恒假)");
    }
    // 3. 仪表/操纵面/地平仪短名面 (app_shell feed_overlays_live + gauges)
    for n in [
        "throttle",
        "rpm_throttle",
        "radiator",
        "power_percent",
        "mixture_state",
        "fuel_percent",
        "gear",
        "flaps",
        "airbrake",
        "compressor_stage",
        "aileron",
        "elevator",
        "rudder",
        "wing_sweep",
        "wing_sweep_valid",
        "aoa",
        "aos",
        "aviahorizon_pitch",
        "aviahorizon_roll",
        "compass",
    ] {
        assert!(canon(n).is_some(), "短名 {n} 未注册");
    }
    // 4. hud_calculator v() 取值名 (公式名/短名)
    for n in [
        "aileron_lock_ratio",
        "altitude",
        "compass",
        "energy_jkg",
        "ias",
        "mach",
        "radio_altitude",
        "radio_altitude_valid",
        "rudder_lock_ratio",
        "sep",
        "speed_limit_ratio",
        "stall_speed",
        "unit_mach_limit_ratio",
        "wing_sweep",
        "wing_sweep_valid",
    ] {
        assert!(canon(n).is_some(), "hud_calculator 名 {n} 未注册");
    }
}

// ==== 原 overlays_field2/tests.rs (control_surfaces/fm_unpacked) ====

const BOLD: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";
const REGULAR: &str = "../../../fonts/sarasa-mono-sc-regular.ttf";

fn font(path: &str, size: i32) -> LoadedFont {
    LoadedFont::new(std::path::Path::new(path), size).unwrap()
}

/// 直通色 → tiny-skia 预乘取整 ((c*a+127)/255), 断言基准用
fn premul(c: [u8; 4]) -> [u8; 4] {
    [
        ((c[0] as u32 * c[3] as u32 + 127) / 255) as u8,
        ((c[1] as u32 * c[3] as u32 + 127) / 255) as u8,
        ((c[2] as u32 * c[3] as u32 + 127) / 255) as u8,
        c[3],
    ]
}

// ---- java_format_f / java_string_format: 历史基线 对拍 ----

/// 精确二进制值 nearest-even (波21: Rust {:.N} 语义, HALF_UP 复刻退役)
#[test]
fn fmt_f_rounding_oracle() {
    assert_eq!(fmt_f(5.25, 1), "5.2"); // 精确半点取偶
    assert_eq!(fmt_f(2.675, 2), "2.67"); // 实值 2.67499... 舍下
    assert_eq!(fmt_f(0.5, 0), "0");
    assert_eq!(fmt_f(2.5, 0), "2");
    assert_eq!(fmt_f(2.675, 1), "2.7"); // 实值 .6749 一位小数仍进位
}

/// 常规/负数/补零/NaN/-0.0/整域
#[test]
fn java_format_f_domains() {
    assert_eq!(fmt_f(3050.0, 1), "3050.0");
    assert_eq!(fmt_f(-8.4, 1), "-8.4");
    assert_eq!(fmt_f(9.0, 2), "9.00");
    assert_eq!(fmt_f(0.105, 3), "0.105");
    assert_eq!(fmt_f(-0.04, 1), "-0.0", "负号保留 (Java Formatter)");
    assert_eq!(fmt_f(f64::NAN, 1), "NaN");
    assert_eq!(fmt_f(f64::INFINITY, 0), "inf"); // 波21: Rust 原生
                                                // 巨整数域: Rust {:.1} 按精确二进制值展开 (1e26 的 IEEE 实值)
    assert_eq!(fmt_f(1e26, 1), "100000000000000004764729344.0");
    // 小数 |x|<1 的 prec=0
    assert_eq!(fmt_f(0.49999999999999994, 0), "0");
}

/// java_string_format: %s/%d/%.Nf 顺序展开 + %% 字面 (bFlapRestrict 模板)
#[test]
fn java_string_format_engine() {
    let t = "襟翼限速(km/h)%d: %.0f%% / %.0f\n";
    assert_eq!(
        java_string_format(t, &[FmtArg::D(1), FmtArg::F(95.0), FmtArg::F(640.0)]),
        "襟翼限速(km/h)1: 95% / 640\n"
    );
    assert_eq!(
        java_string_format("FM文件: %s - %s", &[FmtArg::S("a"), FmtArg::S("b")]),
        "FM文件: a - b"
    );
    // %s 收 null 字段 → "null"
    assert_eq!(java_string_format("V: %s", &[FmtArg::S("null")]), "V: null");
}

/// 模板/实参错配 → panic (Java UnknownFormatConversionException /
/// MissingFormatArgumentException 的崩溃语义)
#[test]
#[should_panic]
fn java_string_format_missing_arg_panics() {
    let _ = java_string_format("%s %s", &[FmtArg::S("a")]);
}

/// %d 位点收浮点实参 → panic (Java IllegalFormatConversionException;
/// 曾静默 `v as i64` 输出与 doc "两语言同为崩溃语义" 矛盾, 已对齐)
#[test]
#[should_panic(expected = "IllegalFormatConversionException")]
fn java_string_format_f_at_d_panics() {
    let _ = java_string_format("%d", &[FmtArg::F(1.5)]);
}

/// %d 位点收字符串实参 → panic (Java 同抛 IllegalFormatConversionException)
#[test]
#[should_panic(expected = "IllegalFormatConversionException")]
fn java_string_format_s_at_d_panics() {
    let _ = java_string_format("%d", &[FmtArg::S("x")]);
}

/// addLines 的 Java trim 语义: 只剥 ≤ U+0020, 全角空格 U+3000 保留
/// (Rust `str::trim` 会多剥一层 — 域内不可达, 本测试锁定复刻边界)
#[test]
fn add_lines_java_trim_semantics() {
    let mut lines = Vec::new();
    add_lines(&mut lines, "a\u{3000}  \nb\u{3000}\n  \t\n");
    assert_eq!(
        lines,
        vec!["a\u{3000}".to_string(), "b\u{3000}".to_string()]
    );
}

// ---- ControlSurfacesOverlay ----

/// init/reinitConfig 几何公式 (Java :225-271, :107-111):
/// fontSize=24 → width=144, rudderValPix=108, twidth=240, theight=180,
/// locate=4, stroke=2; enableAxisEdge 加 sw=10
#[test]
fn control_surfaces_geometry() {
    let mut ov = ControlSurfacesOverlay::new();
    ov.init(0, 1.0, false, 30, 40, true);
    assert_eq!(ov.font_size, 24);
    assert_eq!(ov.label_font_size, 12, "Math.round(24/2.0f)");
    assert_eq!((ov.width, ov.height), (144, 144));
    assert_eq!(ov.rudder_val_pix, 108, "(50+100)*144/200 初值");
    assert_eq!(
        (ov.content_width, ov.content_height),
        (240, 180),
        "(int)(144+96)/(int)(144+36)"
    );
    assert_eq!(ov.shade_width, 0);
    assert_eq!((ov.total_width, ov.total_height), (240, 180));
    assert_eq!((ov.px, ov.py), (72, 72));
    assert_eq!((ov.locate_size, ov.stroke_size), (4, 2));
    assert_eq!((ov.lx, ov.ly), (30, 40), "OverlaySettings 坐标透传");
    assert!(ov.has_service, "游戏模式");
    // 初值 50 (Java :91-94)
    assert_eq!(ov.elevator_num, "50");
    assert_eq!(ov.wing_sweep_num, "50");

    // enableAxisEdge: sw=10 外扩 (Java :250-256)
    let mut ov2 = ControlSurfacesOverlay::new();
    ov2.init(0, 1.0, true, 0, 0, false);
    assert_eq!(ov2.shade_width, 10);
    assert_eq!((ov2.total_width, ov2.total_height), (260, 200));
    assert!(!ov2.has_service, "preview: s == null");

    // fontadd=-6 → fontSize 18, width 108, twidth (int)(108+72)=180
    let mut ov3 = ControlSurfacesOverlay::new();
    ov3.init(-6, 1.0, false, 0, 0, false);
    assert_eq!(ov3.font_size, 18);
    assert_eq!(ov3.width, 108);
    assert_eq!(ov3.content_width, 180);
    assert_eq!(ov3.content_height, (108.0 + 27.0) as i32, "135");

    // 奇数字号: fontSize 25 (dpi 校准) → label = Math.round(12.5f) = 13
    let mut ov4 = ControlSurfacesOverlay::new();
    ov4.reinit_config(1, 1.0, false, 0, 0);
    assert_eq!(ov4.font_size, 25);
    assert_eq!(ov4.label_font_size, 13);
}

/// onFlightData: 50ms 节流 + preview 不更新数据 + 游标/条位换算 (Java :280-312)
#[test]
fn control_surfaces_throttle_and_mapping() {
    let mut ov = ControlSurfacesOverlay::new();
    ov.init(0, 1.0, false, 0, 0, true);

    // 首事件: lastRefreshTime=0 → 0-0 < 50 恒真 → 被跳过? Java 同:
    // 初值 0, now=0 时 0-0=0 < 50 → skip。用 now=100 起测
    assert!(
        !ov.on_flight_data(0, 0.0, 0.0, 0.0, 0.0, false),
        "0-0 < 50 跳过 (Java 同)"
    );
    assert!(ov.on_flight_data(100, -100.0, 100.0, 0.0, 0.85, true));
    assert_eq!(
        (ov.px, ov.py),
        (0, 144),
        "副翼 -100 → 左缘; 升降舵 100 → 底缘"
    );
    assert_eq!(ov.rudder_val_pix, 72, "方向舵 0 → 中位");
    assert_eq!(
        ov.wing_sweep_num, "85",
        "可变翼 0.85 → 85 (isWingSweepValid)"
    );
    assert_eq!(ov.elevator_num, "100");
    assert_eq!(ov.aileron_num, "-100");

    // 节流: +30ms 跳过, +50ms 放行
    assert!(!ov.on_flight_data(130, 0.0, 0.0, 0.0, 0.0, false));
    assert!(ov.on_flight_data(150, 50.7, -25.9, 100.0, -65535.0, false));
    // (int) 截断向零: 50.7→50, -25.9→-25
    assert_eq!(
        (ov.px, ov.py),
        ((100 + 50) * 144 / 200, (100 - 25) * 144 / 200)
    );
    assert_eq!(ov.rudder_val_pix, 144, "满舵 → 全宽");
    assert_eq!(ov.aileron_num, "50");
    assert_eq!(ov.elevator_num, "-25");
    assert_eq!(ov.wing_sweep_num, "0", "wsweep -65535 无效标记 → 0");

    // preview (s == null): 返回 true (repaint 恒调度) 但数据保持
    let mut pv = ControlSurfacesOverlay::new();
    pv.init_preview(0, 1.0, false, 0, 0);
    assert!(pv.on_flight_data(100, -100.0, -100.0, -100.0, 0.5, true));
    assert_eq!((pv.px, pv.py), (72, 72), "初值 50 → 中心");
    assert_eq!(pv.rudder_val_pix, 108, "初值条位");
    assert_eq!(pv.rudder_num, "50");
}

/// draw 像素: 边框/十字影子+主十字/横条+游标 (alpha=255 语义区, 预乘=直通)
#[test]
fn control_surfaces_draw_pixels() {
    let mut ov = ControlSurfacesOverlay::new();
    ov.init(0, 1.0, false, 0, 0, true);
    let f_num = font(BOLD, 24);
    let f_label = font(BOLD, 12);
    let f_unit = font(REGULAR, 12);
    let fonts = CsFonts {
        num: &f_num,
        label: &f_label,
        unit: &f_unit,
    };
    let mut cv = PixCanvas::new(240, 180).unwrap();
    ov.draw(&mut cv, &fonts, false);

    // 边框 (BasicStroke(1), colorShadeShape): 四角 — Java 4 条 drawLine 各自独立
    // 描边, 角点被两条线 SrcOver 叠两次: 42 + 213·42/255 ≈ 77 (Java 同值)
    let corner_blend = [0u8, 0, 0, 77];
    for (x, y) in [(0, 0), (143, 0), (0, 143), (143, 143)] {
        assert_eq!(px(&cv, x, y), corner_blend, "边框角双叠 ({x},{y})");
    }
    assert_eq!(px(&cv, 0, 72), colors().shade_shape, "左边框中点");
    // 边框外无字
    assert_eq!(px(&cv, 60, 60), [0, 0, 0, 0], "十字区中心空");

    // 主十字 (colorNum, 中心 (72,72) 偏移 -1, 线宽 2): 六条独立 drawLine 的
    // 描边互相交叠 — 断言取**单笔画覆盖**点 (Java 同样叠出混合 alpha):
    // 主横线 y=71 (行 70/71 实心, 臂 x∈[68,73]); 主竖线 x=71 (列 70/71 实心)
    assert_eq!(
        px(&cv, 69, 70),
        premul(colors().num),
        "主横线单覆盖点 (69,70)"
    );
    assert_eq!(
        px(&cv, 70, 69),
        premul(colors().num),
        "主竖线单覆盖点 (70,69)"
    );
    // 主线交叠中心 (行70/71 × 列70/71): 240+240·15/255 → 饱和 255
    assert_eq!(px(&cv, 70, 70)[3], 255, "主十字中心核心双叠饱和");
    // 影子十字 (colorShadeShape, 轴 y=72/x=72, 偏移 +1): 在主线臂端外侧露出 —
    // 影横臂延至 x=74 (主横臂 x≤73), 影竖臂延至 y=74 (主竖臂 y≤73) → 单覆盖点
    assert_eq!(
        px(&cv, 74, 71),
        colors().shade_shape,
        "影横臂右尖端 (74,71)"
    );
    assert_eq!(
        px(&cv, 71, 74),
        colors().shade_shape,
        "影竖臂下尖端 (71,74)"
    );
    // 影子自身交点 (72,72) 双叠: 42+213·42/255 ≈ 77 (Java 同)
    assert_eq!(px(&cv, 72, 72), [0, 0, 0, 77], "影子交点双叠");

    // 底部方向舵横条 (y=height=144 起, 高 12): 外框阴影 + 内填 colorNum。
    // 条顶左角 (0,144) 与 locater 左边框线端点 (drawLine(0,0,0,r), r=144
    // 含端点) 重叠 → SrcOver 双叠 77 (Java 同序同叠); 条底右角单覆盖
    assert_eq!(
        px(&cv, 0, 144),
        [0, 0, 0, 77],
        "条顶左角 (与边框线端点双叠)"
    );
    assert_eq!(
        px(&cv, 143, 155),
        colors().shade_shape,
        "条底边框右角 (144+12-1)"
    );
    assert_eq!(
        px(&cv, 2, 150),
        premul(colors().num),
        "条内填充 (初值 108 宽)"
    );
    assert_eq!(
        px(&cv, 105, 150),
        premul(colors().num),
        "条内填充右段 (x ≤ 106)"
    );
    assert_eq!(px(&cv, 109, 150), [0, 0, 0, 0], "游标右缘外空 (x=109)");

    // 游标竖线 (x=106..108, y=144..167): 阴影框 + colorLabel 中心 1px。
    // 顶行与条顶边框重叠 → 双叠 77; 中心列 (x=107) 从 y=145 起, 底段无条遮挡
    assert_eq!(
        px(&cv, 106, 144),
        [0, 0, 0, 77],
        "游标左上角 (与条顶边框双叠)"
    );
    assert_eq!(
        px(&cv, 106, 160),
        colors().shade_shape,
        "游标左框单覆盖 (条外段)"
    );
    assert_eq!(
        px(&cv, 107, 160),
        premul(colors().label),
        "游标中心 colorLabel (条外段)"
    );
    assert_eq!(
        px(&cv, 107, 166),
        premul(colors().label),
        "游标下端 (144+24-2)"
    );
}

/// draw 文本带: 4 行 BOS 标签 (数字 x=width 基线 24; 标签/单位 x=width+54)
/// 与方向舵数字 (x=rudderValPix, 基线 168) 有字形像素落点
#[test]
fn control_surfaces_draw_text_zones() {
    let mut ov = ControlSurfacesOverlay::new();
    ov.init(0, 1.0, false, 0, 0, true);
    let f_num = font(BOLD, 24);
    let f_label = font(BOLD, 12);
    let f_unit = font(REGULAR, 12);
    let fonts = CsFonts {
        num: &f_num,
        label: &f_label,
        unit: &f_unit,
    };
    let mut cv = PixCanvas::new(240, 180).unwrap();
    ov.draw(&mut cv, &fonts, false);

    let has_ink = |x0: i32, x1: i32, y0: i32, y1: i32| -> bool {
        (x0..x1).any(|x| (y0..y1).any(|y| px(&cv, x, y)[3] > 0))
    };
    // 数字 "50" @ (144, 24 基线), fontNum 24 — lwidth=(9*24)>>2=54
    assert!(has_ink(144, 180, 4, 26), "首行数字带 (升降舵 50)");
    // 标签名 "升降舵" @ (198, 12 基线) + 单位 "%" @ (198, 24 基线)
    assert!(has_ink(198, 240, 2, 14), "首行标签名带");
    assert!(has_ink(198, 216, 14, 26), "首行单位带");
    // 第四行 (可变翼) dy = 12 + 3*36 = 120 基线
    assert!(has_ink(198, 240, 110, 132), "第四行标签带 (dy=120)");
    // 方向舵数字 "50" @ (108, 168 基线) fontLabel 12
    assert!(has_ink(108, 132, 156, 170), "条值数字带");
}

// ---- FmUnpackedDataOverlay ----

/// 测试用 ConfigProvider stub (HashMap + RefCell, 与 vm-core config_provider 测试同式)
struct MapConfig {
    values: RefCell<HashMap<String, String>>,
}

impl MapConfig {
    fn new() -> Self {
        MapConfig {
            values: RefCell::new(HashMap::new()),
        }
    }
    fn set(&self, k: &str, v: &str) {
        self.values
            .borrow_mut()
            .insert(k.to_string(), v.to_string());
    }
}

impl ConfigProvider for MapConfig {
    fn get_config(&self, key: &str) -> Option<String> {
        self.values.borrow().get(key).cloned()
    }
    fn set_config(&self, key: &str, value: &str) {
        self.values
            .borrow_mut()
            .insert(key.to_string(), value.to_string());
    }
    fn is_field_disabled(&self, _key: &str) -> bool {
        false
    }
}

/// 全字段齐备的测试 blkx (期望值 = 历史基线 手算, HALF_UP 判别值混入)
fn full_fmdata() -> FmData {
    let mut b = FmData::default();
    b.read_file_name = Some("spitfire_mk24".to_string());
    b.version = Some("2.35.0.9".to_string());
    b.emptyweight = 3050.0;
    b.maxfuelweight = 780.45; // %.1f HALF_UP → "780.5"
    b.critical_speed = 230.0; // ×3.6 = 828.000...01 → "828"
    b.vne = 1050.0;
    b.raw_wing_crit_overload = Some([-196000.0, 441000.0]);
    b.grossweight = 5000.0; // full: 1.2·(2·raw/(g·w)∓1) → (-8.4, 20.4)
    b.halfweight = 4000.0; // half → (-10.8, 25.8)
    b.flaps_destruction_num = 2;
    let mut flaps = [[0.0; 2]; 6];
    flaps[0] = [0.0, 640.0];
    flaps[1] = [0.95, 520.0]; // ×100 = 94.99... → %.0f → "95"
    b.flaps_destruction_ind_speed = Some(flaps);
    b.elav_eff = 580.0;
    b.aileron_eff = 640.0;
    b.rudder_eff = 700.0;
    b.elav_power_loss = 0.25; // %.1f HALF_UP → "0.3"
    b.aileron_power_loss = 0.35; // → "0.4"
    b.rudder_power_loss = 0.45; // → "0.5"
    b.nitro = 120.0;
    b.nitro_decr = 2.0; // 120/(2·60) = 1.0
    b.avg_eng_recovery_rate = 3.25; // %.1f HALF_UP → "3.3"
    b.no_flap_wll = 9.0; // (9+1)/2 = 5.0
    b.full_flap_wll = 13.0; // 7.0
    b.moment_of_inertia = Some([12000.0, 25000.0, 8000.0]); // [P:m[2], R:m[0], Y:m[1]]
    b.a_wing = 25.8;
    b.a_fuselage = 5.4;
    b.oswalds_efficiency_number = 0.75;
    b.aspect_ratio = 6.0;
    b.swept_wing_angle = 0.0;
    b.cd_s = 0.42;
    b.ind_cd_f = 0.003; // 4000·0.003 ≈ 12.000...002 → "12"
    b.radiator_cd = 0.021;
    b.oil_radiator_cd = 0.017;
    let mut wing = FmParts::default();
    wing.name = Some("机翼 无襟翼".to_string());
    wing.cd_min = 0.0285; // %.3f HALF_UP → "0.029"
    wing.cl0 = 0.05;
    wing.aoa_crit_low = -14.4;
    wing.aoa_crit_high = 18.6;
    wing.cl_crit_low = -1.15;
    wing.cl_crit_high = 1.55;
    b.no_flaps_wing = Some(wing.clone());
    let mut ff = FmParts::default();
    ff.name = Some("机翼 全襟翼".to_string());
    ff.cd_min = 0.0331;
    ff.cl0 = 0.12;
    ff.aoa_crit_low = -13.1;
    ff.aoa_crit_high = 20.2;
    ff.cl_crit_low = -1.35;
    ff.cl_crit_high = 1.85;
    b.full_flaps_wing = Some(ff);
    let mut fuse = FmParts::default();
    fuse.name = Some("机身".to_string());
    fuse.cd_min = 0.0151;
    fuse.cl0 = 0.02;
    fuse.aoa_crit_low = -27.9;
    fuse.aoa_crit_high = 27.9;
    fuse.cl_crit_low = -0.41;
    fuse.cl_crit_high = 0.49;
    b.fuselage = Some(fuse);
    let mut fin = FmParts::default();
    fin.name = Some("垂尾".to_string());
    fin.cd_min = 0.0081;
    fin.cl0 = 0.0;
    fin.aoa_crit_low = -16.2;
    fin.aoa_crit_high = 16.2;
    fin.cl_crit_low = -0.62;
    fin.cl_crit_high = 0.62;
    b.fin = Some(fin);
    let mut stab = FmParts::default();
    stab.name = Some("平尾".to_string());
    stab.cd_min = 0.0062;
    stab.cl0 = -0.06;
    stab.aoa_crit_low = -15.5;
    stab.aoa_crit_high = 15.5;
    stab.cl_crit_low = -0.55;
    stab.cl_crit_high = 0.55;
    b.stab = Some(stab);
    b
}

/// generateLines 全量 (config None → 全启用) 的逐行 基线
#[test]
fn generate_lines_full_field_list() {
    let lines = generate_lines(Some(&full_fmdata()), None);
    let expect_prefix = [
        "FM文件: spitfire_mk24 - 2.35.0.9",
        "空重(kg): 3050.0",
        "最大燃油重量(kg): 780.5", // %.1f HALF_UP 判别
        "临界速度(km/h): [828, 1050]",
        "允许过载(满/半油): [-8.4, 20.4], [-10.8, 25.8]",
        "襟翼限速(km/h)0: 0% / 640",
        "襟翼限速(km/h)1: 95% / 520",
        "三舵有效速度(km/h): [ 升降580, 副翼640, 方向700 ]",
        "三舵锁舵因数: [ 升降0.2, 副翼0.3, 方向0.5 ]", // %.1f HALF_UP ×3
        "加力(kg)/时限(分钟): 120.0 / 1.0",
        "平均耐热条恢复速率: 3.2", // %.1f HALF_UP 判别
        "千米最大升力过载: 5.0 / 7.0(襟) @ 350IAS",
        "三轴转动惯量: [ P: 8000, R: 12000, Y: 25000 ]",
        "主升力面积: 25.8机翼, 5.4机身",
        "主升力面积因数载荷: 9.00 / 13.00(襟)",
        "翼展效率: 0.75 展弦比: 6.0 后掠角: 0.0",
        "主阻力面积因数及加速度系数: 0.42 / 0.105",
        "诱导阻力因数及加速度系数: 0.003 / 12",
        "散热/油冷器阻力系数: 0.021 / 0.017",
    ];
    assert!(
        lines.len() >= expect_prefix.len() + 25,
        "全字段行数 ≥ 44, 实 {}",
        lines.len()
    );
    for (i, want) in expect_prefix.iter().enumerate() {
        assert_eq!(&lines[i], want, "第 {i} 行");
    }
    // FM 器件段 (addFmParts ×5 段, 每段表头+4 行)
    assert_eq!(lines[19], "------fm器件 机翼 无襟翼------");
    assert_eq!(lines[20], "零升阻力系数: 0.029", "%.3f HALF_UP 判别");
    assert_eq!(lines[21], "零攻角升力: 0.050");
    assert_eq!(lines[22], "临界攻角: [-14.4, 18.6]");
    assert_eq!(lines[23], "临界攻角升力系数: [-1.15, 1.55]");
    let idx = lines
        .iter()
        .position(|l| l == "------fm器件 平尾------")
        .expect("第五段 (Stab)");
    assert_eq!(
        &lines[idx + 1..idx + 5],
        [
            "零升阻力系数: 0.006",
            "零攻角升力: -0.060",
            "临界攻角: [-15.5, 15.5]",
            "临界攻角升力系数: [-0.55, 0.55]",
        ]
    );
}

/// 无数据 / null 字段 ("null" 文本) / 空白模板行裁剪
#[test]
fn generate_lines_no_data_and_null_fields() {
    assert_eq!(
        generate_lines(None, None),
        vec![
            "FM Data Preview".to_string(),
            "[No Data Loaded]".to_string()
        ]
    );
    // readFileName/version 为 null → %s 打 "null" (Java Formatter 行为)
    let mut b = FmData::default();
    b.emptyweight = 1.0;
    let lines = generate_lines(Some(&b), None);
    assert_eq!(lines[0], "FM文件: null - null");
}

/// 字段开关: false 关 / 空串与缺失默认开 / parseBoolean 仅 "true" (忽略大小写)
#[test]
fn generate_lines_field_switches() {
    let cfg = MapConfig::new();
    cfg.set("showWeight", "false");
    cfg.set("showCritSpeed", "FALSE"); // parseBoolean 忽略大小写 → false
    cfg.set("showLift", ""); // 空串 → 默认启用
    cfg.set("showDrag", "yes"); // 非 "true" → false
    let lines = generate_lines(Some(&full_fmdata()), Some(&cfg));
    assert!(
        !lines.iter().any(|l| l.starts_with("空重")),
        "showWeight=false 关"
    );
    assert!(
        !lines.iter().any(|l| l.starts_with("临界速度")),
        "FALSE (忽略大小写) 关"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("主升力面积")),
        "空串默认开"
    );
    assert!(
        !lines.iter().any(|l| l.starts_with("主阻力面积")),
        "yes → false"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("加力")),
        "其余段不受影响"
    );
    // fmVersion 恒显 → "[No Fields Enabled]" 占位不可达 (Java 同)
    assert!(lines.iter().any(|l| l.starts_with("FM文件")));
}

/// nitro ≤ 0 段隐藏 (Java :212 blkx.nitro > 0 门控)
#[test]
fn generate_lines_nitro_gate() {
    let mut b = full_fmdata();
    b.nitro = 0.0;
    let lines = generate_lines(Some(&b), None);
    assert!(!lines.iter().any(|l| l.contains("加力")));
    b.nitro = 60.0;
    b.nitro_decr = 1.0;
    let lines = generate_lines(Some(&b), None);
    assert!(lines.iter().any(|l| l == "加力(kg)/时限(分钟): 60.0 / 1.0"));
}

/// 表头谓词 (Java :87/:118 startsWith 覆盖默认 contains) + 斑马交互
#[test]
fn fm_overlay_header_matcher() {
    let f = font(REGULAR, 14);
    let mut ov = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    ov.init(None, &f);
    assert!(ov.base.zebra.is_header("FM文件: x"));
    assert!(ov.base.zebra.is_header("------fm器件: 机翼"));
    assert!(
        !ov.base.zebra.is_header("prefix FM文件"),
        "startsWith 不含中缀"
    );
    assert!(
        !ov.base.zebra.is_header("含 fm器件 中缀的行"),
        "默认 contains 已被覆盖"
    );
}

/// 游戏模式门控: 初始隐藏不取数; toggle 后取数并脏; 同数据不脏 (Java :67/:318)
#[test]
fn fm_overlay_toggle_visibility_gating() {
    let f = font(REGULAR, 14);
    let mut ov = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    ov.init(None, &f);
    assert!(!ov.is_visible_now(), "游戏模式初始隐藏");
    assert!(!ov.tick(), "隐藏分支不取数不显示");
    assert!(!ov.base.window_visible);

    ov.toggle();
    assert!(ov.is_visible_now());
    ov.reload_fm_data(Some(Arc::new(full_fmdata())));
    assert!(ov.tick(), "首帧脏 (lastData=null → 行清单入基座)");
    assert!(ov.base.window_visible);
    assert!(!ov.tick(), "同数据 equals → 不脏");

    ov.toggle();
    assert!(!ov.tick(), "再隐藏 → 不取数");
    assert!(!ov.base.window_visible);
}

/// reload/reinit 换 blkx → 行清单随脏检查刷新; None → 占位 (Java :130-151)
#[test]
fn fm_overlay_reload_and_reinit() {
    let f = font(REGULAR, 14);
    let mut ov = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    ov.init(None, &f);
    ov.toggle(); // 可见化以走取数分支

    // last_data 为基座私有字段, 内容经 generate_lines() 断言、刷新经脏标志断言
    ov.reload_fm_data(Some(Arc::new(full_fmdata())));
    assert!(ov.tick());
    assert!(ov.generate_lines()[0].starts_with("FM文件: spitfire"));

    ov.reload_fm_data(None);
    assert!(ov.tick(), "清单变化 ([No Data Loaded]) → 脏");
    assert_eq!(
        ov.generate_lines(),
        vec![
            "FM Data Preview".to_string(),
            "[No Data Loaded]".to_string()
        ]
    );
    assert!(!ov.tick(), "同清单 → 不脏");

    // reinit_config: FMManager.current() 快照注入 (Java :146-147)
    let mut b = FmData::default();
    b.read_file_name = Some("tempest_mk5".to_string());
    ov.reinit_config(Some(Arc::new(b)), &f);
    assert!(ov.tick(), "reinit 换机 → 清单变化 → 脏");
    assert!(ov.generate_lines()[0].starts_with("FM文件: tempest_mk5"));
    // 预览模式绕过可见门控 (BaseOverlay.run:235 isPreview ||)
    let mut pv = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    pv.init_preview(None, &f);
    assert!(pv.is_visible_now());
    assert!(pv.base.is_preview);
    assert!(pv.tick(), "preview 隐藏语义下仍取数");
}

/// QA 批十终检: 五个 overlay (field1 三件 + 本文件两件) 的内容渲染函数经
/// OverlaySpec 装入 OverlayHost 走全链 (register → open_all → render_tick →
/// present → close_all)。field2 两组件的完整组装 (动态窗口高/逐条目可见性/
/// 预览闭包工厂) 按模块头 PORT 注留组装层, 此处只证 host 的 render 闭包通道
/// (RenderFn) 对二者同样可用 — Java 侧五件同经 OverlayManager 注册装载。
/// 窗口生命周期语义 (销毁序/分流/拖拽) 由 host.rs 自有测试覆盖, 此处 mock 只记
/// present 次数并断言缓冲尺寸。
#[test]
fn five_overlays_mount_into_overlay_host() {
    use crate::platform::host::{OverlayHost, OverlaySpec};
    use crate::platform::{OverlayEvent, OverlayWindow, WindowConfig};
    use std::cell::Cell;
    use std::rc::Rc;

    struct MiniWin {
        presents: Rc<Cell<u32>>,
        size: (i32, i32),
    }
    impl OverlayWindow for MiniWin {
        fn present(&mut self, buf: &[u8]) -> Result<(), String> {
            assert_eq!(buf.len(), (self.size.0 * self.size.1 * 4) as usize);
            self.presents.set(self.presents.get() + 1);
            Ok(())
        }
        fn set_position(&mut self, _x: i32, _y: i32) {}
        fn position(&self) -> (i32, i32) {
            (0, 0)
        }
        fn set_click_through(&mut self, _on: bool) {}
        fn poll_event(&mut self) -> Option<OverlayEvent> {
            None
        }
        fn screen_size(&self) -> (i32, i32) {
            (1920, 1080)
        }
    }

    let presents = Rc::new(Cell::new(0u32));
    let p_counter = Rc::clone(&presents);
    let mut host = OverlayHost::with_factory(Box::new(move |cfg: WindowConfig| {
        let size = (cfg.width, cfg.height);
        Ok(Box::new(MiniWin {
            presents: Rc::clone(&p_counter),
            size,
        }) as Box<dyn OverlayWindow>)
    }));

    // ①~③ field1 三键 (engineInfoSwitch/enableEngineControl/enablegearAndFlaps):
    // POC 预览工厂已随重构波2 退役, 此处以最小手工 spec 顶位 (host 通道语义
    // 与内容函数无关, 真实内容渲染由 ④⑤ + field1 自有测试覆盖)
    for key in [
        "engineInfoSwitch",
        "enableEngineControl",
        "enablegearAndFlaps",
    ] {
        host.register(OverlaySpec {
            id: key.into(),
            config_key: key.into(),
            width: 40,
            height: 12,
            render: Box::new(|_cv| {}),
            reinit: None,
        });
    }
    // ④ ControlSurfaces (Java 键 enableAxis): draw 内容函数手工包进 render 闭包
    //    (P5 组装契约 (c) 预览工厂留组装层, 此处同形态验证)
    let mut cs = ControlSurfacesOverlay::new();
    cs.init_preview(0, 1.0, false, 0, 0);
    let f_num = font(BOLD, cs.font_size);
    let f_label = font(BOLD, cs.label_font_size);
    let f_unit = font(REGULAR, cs.label_font_size);
    let (cw, ch) = (cs.total_width, cs.total_height);
    host.register(OverlaySpec {
        id: "enableAxis".into(),
        config_key: "enableAxis".into(),
        width: cw,
        height: ch,
        render: Box::new(move |cv| {
            let fonts = CsFonts {
                num: &f_num,
                label: &f_label,
                unit: &f_unit,
            };
            cs.draw(cv, &fonts, aa());
        }),
        reinit: None,
    });
    // ⑤ FMUnpackedData (Java 键 enableFMPrint): render(&mut) 同通道
    let f_list = font(REGULAR, 14);
    let mut fm = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    fm.init_preview(None, &f_list);
    assert!(fm.tick(), "preview 首帧取数 (占位两行清单)");
    let (fw, fh) = (fm.base.width, fm.base.height);
    assert!(fw > 0 && fh > 0);
    host.register(OverlaySpec {
        id: "enableFMPrint".into(),
        config_key: "enableFMPrint".into(),
        width: fw,
        height: fh,
        render: Box::new(move |cv| {
            fm.render(cv, &f_list, aa());
        }),
        reinit: None,
    });

    // 全链: 开 → 首帧五窗各 present 一次 (尺寸逐窗断言) → 静态内容脏检查抑制
    // → close_all 后槽位全空不再渲染
    host.open_all().unwrap();
    assert_eq!(host.active_ids().len(), 5, "五个 overlay 全部装载打开");
    host.render_tick().unwrap();
    assert_eq!(presents.get(), 5, "首帧五窗各一次 present");
    host.render_tick().unwrap();
    assert_eq!(presents.get(), 5, "静态预览内容: 脏检查抑制");
    host.close_all();
    host.render_tick().unwrap();
    assert_eq!(presents.get(), 5, "槽位全空: 不再 present");
    assert!(host.active_ids().is_empty());
}

/// live 工厂: 尺寸 = 内容区 (fontAdd 0/dpi 1 → fs=24, w=144, twidth=240,
/// theight=180), has_service 初值 false (init_preview), 喂入侧置 true 后
/// on_flight_data 才推数据; render 闭包共享句柄画到新值
#[test]
fn control_surfaces_overlay_spec_shared_state() {
    let fonts_dir = std::path::Path::new("../../../fonts");
    let cell = Rc::new(RefCell::new(ReinitParams::default()));
    let (h, mut spec) = control_surfaces_overlay_spec(fonts_dir, &cell).unwrap();
    assert_eq!(
        (spec.width, spec.height),
        (240, 180),
        "内容区尺寸 (无 sw 边框)"
    );
    assert_eq!(
        (spec.id.as_str(), spec.config_key.as_str()),
        ("enableAxis", "enableAxis")
    );
    // 初值 px = width/2 = 72 (游标居中, Java init :108)
    assert_eq!(h.borrow().px, 72);
    // has_service=false: 数据不更新 (preview 形态)
    assert!(h
        .borrow_mut()
        .on_flight_data(100, 100.0, 0.0, 0.0, 0.0, false));
    assert_eq!(h.borrow().px, 72, "preview 门控: 数据保持");
    // 游戏形态 (喂入方切换 has_service, app_shell 承载): aileron=100 → px=144
    h.borrow_mut().has_service = true;
    assert!(h
        .borrow_mut()
        .on_flight_data(200, 100.0, 0.0, 0.0, 0.0, false));
    assert_eq!(h.borrow().px, 144);
    assert_eq!(h.borrow().aileron_num, "100");
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0));

    // WYSIWYG reinit: fontAdd 0→6 → fs=30 → w=180, twidth=300, theight=225
    cell.borrow_mut().axis.font_add = 6;
    let (w1, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert_eq!((w1, h1), (300, 225), "字号 6 的内容区 (fs=30)");
    assert_eq!(h.borrow().font_size, 30, "state 已换新几何");
    // reinit 后 render 闭包可画 (共享字体单元已更新, 不 panic)
    let mut cv2 = PixCanvas::new(w1, h1).unwrap();
    (spec.render)(&mut cv2);
    assert!(cv2.pixmap().data().iter().any(|&b| b != 0));
}

/// CloseAllOverlays 数据面重置 (app_shell reset_handles_preview_values 调用面):
/// live 残留 (num 串 + 游标/舵条) → reset_preview → initPreview 的
/// "Initial Values (50)" + 游标居中。场景: 托盘 live→preview 后重开的
/// 预览窗不得显示上次 live 舵面值
#[test]
fn control_surfaces_reset_preview_restores_initial_values() {
    let fonts_dir = std::path::Path::new("../../../fonts");
    let cell = Rc::new(RefCell::new(ReinitParams::default()));
    let (h, _spec) = control_surfaces_overlay_spec(fonts_dir, &cell).unwrap();
    // live 残留: has_service=true 喂非 50 值 (副翼 100/升降 -80/舵 60/翼扫 40)
    h.borrow_mut().has_service = true;
    assert!(h
        .borrow_mut()
        .on_flight_data(200, 100.0, -80.0, 60.0, 40.0, true));
    assert_eq!(h.borrow().aileron_num, "100");
    // 重置 → 初值段: 四 num 串 "50" + 游标/舵条回几何中心 (init :91-94/:108-111)
    h.borrow_mut().reset_preview();
    let cs = h.borrow();
    assert_eq!(
        (
            cs.elevator_num.as_str(),
            cs.aileron_num.as_str(),
            cs.rudder_num.as_str(),
            cs.wing_sweep_num.as_str()
        ),
        ("50", "50", "50", "50")
    );
    assert_eq!(
        (cs.px, cs.py, cs.rudder_val_pix),
        (cs.width / 2, cs.height / 2, (50 + 100) * cs.width / 200),
        "游标居中 + 舵条半量程"
    );
}

// ---- FmUnpackedData spec 工厂 + FmUnpackedFeed (P5 组装契约 (a)(b)(c) 销号面) ----

/// 最小 mock 窗口: 只记 set_visible/set_size 调用序 (host/tests.rs MockWindow 同款形态)
struct FeedMockWin {
    log: Rc<RefCell<Vec<String>>>,
}

impl crate::platform::OverlayWindow for FeedMockWin {
    fn present(&mut self, _buf: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn set_position(&mut self, _x: i32, _y: i32) {}
    fn position(&self) -> (i32, i32) {
        (60, 100)
    }
    fn set_click_through(&mut self, _on: bool) {}
    fn set_topmost(&mut self, _on: bool) {}
    fn set_visible(&mut self, visible: bool) {
        self.log.borrow_mut().push(format!("set_visible:{visible}"));
    }
    fn set_size(&mut self, w: i32, h: i32) {
        self.log.borrow_mut().push(format!("set_size:{w},{h}"));
    }
    fn poll_event(&mut self) -> Option<crate::platform::OverlayEvent> {
        None
    }
    fn screen_size(&self) -> (i32, i32) {
        (1920, 1080)
    }
}

fn feed_host(log: &Rc<RefCell<Vec<String>>>) -> OverlayHost {
    let log = Rc::clone(log);
    OverlayHost::with_factory(Box::new(move |_cfg| {
        Ok(Box::new(FeedMockWin {
            log: Rc::clone(&log),
        }) as Box<dyn crate::platform::OverlayWindow>)
    }))
}

fn feed_fm() -> Arc<FMManager> {
    Arc::new(FMManager::new(
        Arc::new(vm_core::base::bus::EventBus::new()),
    ))
}

/// 工厂初态 = initPreview 形态 (恒可见 + 空数据 — 注册期 = Java 无实例形态;
/// 数据装载见 [`fm_unpacked_preview_session_pumps_data`] — Java 预览实例的
/// run 线程同样在跑, 审查 B2-2); spec 尺寸 = init 几何
/// (logicalHeight 1080/dpi 1 → scaleFactor 0.75 → 324×864, BaseOverlay.java:94-95)
#[test]
fn fm_unpacked_spec_preview_shape_and_render() {
    let (h, mut spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    assert_eq!(
        (spec.id.as_str(), spec.config_key.as_str()),
        ("enableFMPrint", "enableFMPrint")
    );
    assert_eq!(
        (spec.width, spec.height),
        (324, 864),
        "init 几何 (round(12·36·0.75) × 12·72)"
    );
    {
        let fm = h.borrow();
        assert!(fm.visible, "preview: always visible (:113)");
        assert!(fm.base.is_preview, "preview: isPreview=true (:110)");
        assert_eq!(fm.base.width, 324);
    }
    // 空数据渲染: dataPanel 底色铺满 (非零像素), 无文本行
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0), "panel 底色");
}

/// 预览会话数据装载 (审查 B2-2 回归锚): Java needsThread=true — 预览实例同样
/// 起 run() 线程 (OverlayManager.refreshPreview :326-331), isPreview 分支每
/// 200ms generateLines → 预览窗显示 FM 字段行 (非空面板)。Rust 对位 = 泵不做
/// 会话门控: preview 形态 tick 取数 → dirty → adjustPosition 高度自适应。
#[test]
fn fm_unpacked_preview_session_pumps_data() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = feed_host(&log);
    let (h, spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    host.register(spec);
    // 预览物化 (Java refreshPreview: 工厂 initPreview + 起线程)
    host.refresh_preview().unwrap();
    // 预览期的 FM 装载面 (Java previewInitializer 的 setBlkx(current) /
    // reinitConfig 直读 — 事件订阅仅游戏 init, reload 不走)
    h.borrow_mut()
        .reinit_config(Some(Arc::new(full_fmdata())), &font(REGULAR, 14));
    let mut feed = FmUnpackedFeed::new();
    log.borrow_mut().clear();
    // 泵 (无会话门控): preview 取数 → 高度自适应 resize + 拉起 (幂等可见)
    feed.pump(&mut host, "enableFMPrint", &h, 1_000);
    let row_h = crate::overlays::list::ZebraList::row_height(&font(REGULAR, 14));
    let lines = h.borrow().generate_lines().len() as i32;
    assert!(lines >= 44, "预览装载 FM 行清单 (实测 {lines})");
    assert_eq!(
        h.borrow().base.height,
        lines * row_h,
        "preview 首轮高度自适应 (非 864 初始空面板)"
    );
    assert!(
        h.borrow().base.window_visible,
        "preview isPreview 绕过可见门控"
    );
    // 数据稳定零冗余
    feed.pump(&mut host, "enableFMPrint", &h, 1_300);
    assert_eq!(log.borrow().len(), 1, "稳定期仅首帧 resize 一次");
}

/// 游戏会话全链 (Java run() 循环 + FM_OVERLAY_TOGGLE/FM_CHANGED 的组装面驱动):
/// 隐藏起步 → FM_CHANGED 重载 + 热键切换 → tick 取数 → 高度自适应落 resize +
/// 可见拉起 → 数据稳定零冗余调用 (脏检查/幂等守卫) → 再切换隐藏
#[test]
fn fm_unpacked_feed_game_flow() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = feed_host(&log);
    let (h, spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    host.register(spec);
    host.open_all().unwrap();
    // 游戏形态 (渲染线程 OpenAllOverlays 处理点同款): isPreview=false + 隐藏起步
    {
        let mut fm = h.borrow_mut();
        fm.base.is_preview = false;
        fm.visible = false;
    }
    host.set_entry_visible("enableFMPrint", false);
    let mut feed = FmUnpackedFeed::new();
    log.borrow_mut().clear();
    // ① 隐藏态 tick (else 分支): 不取数, 窗口保持隐藏, 高度不动
    feed.pump(&mut host, "enableFMPrint", &h, 1_000);
    assert_eq!(
        h.borrow().base.height,
        864,
        "隐藏分支不取数, 高度保持 init 值"
    );
    assert!(log.borrow().is_empty(), "无窗口动作 (幂等守卫)");
    // ② FM_CHANGED reload + 热键切换可见
    h.borrow_mut().reload_fm_data(Some(Arc::new(full_fmdata())));
    h.borrow_mut().toggle();
    // ③ 可见分支首 tick: 取数 → dirty → adjustPosition → resize + 拉起窗口
    feed.pump(&mut host, "enableFMPrint", &h, 1_300);
    let row_h = crate::overlays::list::ZebraList::row_height(&font(REGULAR, 14));
    let lines = h.borrow().generate_lines().len() as i32;
    assert!(lines >= 44, "全字段行数 (实测 {lines})");
    assert_eq!(
        h.borrow().base.height,
        lines * row_h,
        "高度 = 行数×行高 (adjustPosition, 未触 1040 钳制)"
    );
    assert_eq!(
        *log.borrow(),
        vec![
            "set_visible:true".to_string(),
            format!("set_size:324,{}", lines * row_h)
        ],
        "拉起 + resize 各恰一次"
    );
    // ④ 数据稳定: 脏检查 + 幂等 → 零窗口动作
    feed.pump(&mut host, "enableFMPrint", &h, 1_600);
    assert_eq!(log.borrow().len(), 2, "稳定期零冗余调用 (Issue #54 防抖)");
    // ⑤ 再切换: 隐藏 (else 分支 setVisible(false), 幂等记录拦重复)
    h.borrow_mut().toggle();
    feed.pump(&mut host, "enableFMPrint", &h, 1_900);
    assert_eq!(log.borrow().last().unwrap(), "set_visible:false");
}

/// show* 开关实效 (engine_disables 实效测试先例): config 全关 → 仅 FM 版本行
/// (最小面) vs 全开 (None = 默认启用) → 显著更高
// PORT(allow): MapConfig 含 RefCell (!Sync) — 工厂签名的 Arc<dyn ConfigProvider>
// 无 Send 约束 (Rc 句柄恒留本线程), 与 Java 引用共享同构
#[test]
#[allow(clippy::arc_with_non_send_sync)]
fn fm_unpacked_field_switches_change_height() {
    let fm = feed_fm();
    let row_h = crate::overlays::list::ZebraList::row_height(&font(REGULAR, 14));
    // 全关 (16 键 "false" → 仅 fmVersion 恒显行)
    let cfg_off = MapConfig::new();
    for key in [
        "showWeight",
        "showCritSpeed",
        "showGLoadLimits",
        "showFlapLimits",
        "showControlEffectiveness",
        "showNitro",
        "showHeatRecovery",
        "showMaxLiftLoad",
        "showInertia",
        "showLift",
        "showDrag",
        "showNoFlapsWing",
        "showFullFlapsWing",
        "showFuselage",
        "showFin",
        "showStab",
    ] {
        cfg_off.set(key, "false");
    }
    let (h_off, _) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        Some(Arc::new(cfg_off)),
        &fm,
    )
    .unwrap();
    h_off
        .borrow_mut()
        .reload_fm_data(Some(Arc::new(full_fmdata())));
    h_off.borrow_mut().tick();
    assert_eq!(
        h_off.borrow().base.height,
        row_h,
        "全关 = 仅 FM 版本一行的高度"
    );
    // 全开 (config None → isFieldEnabled 默认启用)
    let (h_on, _) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &fm,
    )
    .unwrap();
    h_on.borrow_mut()
        .reload_fm_data(Some(Arc::new(full_fmdata())));
    h_on.borrow_mut().tick();
    assert!(
        h_on.borrow().base.height > 20 * row_h,
        "全开显著更高 (实测 {} vs 最小 {})",
        h_on.borrow().base.height,
        row_h
    );
}

/// reset_preview (渲染线程 CloseAllOverlays → reset_handles_preview_values 调用面):
/// live 行残留 → 预览重开为空面板 (Java closeAll 销毁实例 + 预览工厂新建)
#[test]
fn fm_unpacked_reset_preview_clears_live_lines() {
    let (h, mut spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    // live 会话残留: 游戏形态 + FM 数据 + 可见
    {
        let mut fm = h.borrow_mut();
        fm.base.is_preview = false;
        fm.visible = true;
        fm.reload_fm_data(Some(Arc::new(full_fmdata())));
        assert!(fm.tick(), "数据到达 (dirty)");
    }
    // 行内容入画: 文本带存在白色墨迹 (斑马行白字)
    let (w0, h0) = (spec.width, spec.height.min(200));
    let has_ink = |c: &PixCanvas| {
        c.pixmap()
            .data()
            .chunks_exact(4)
            .any(|p| p[3] > 200 && p[0] > 200 && p[1] > 200 && p[2] > 200)
    };
    let mut cv = PixCanvas::new(w0, h0).unwrap();
    (spec.render)(&mut cv);
    assert!(has_ink(&cv), "live 行文本墨迹");
    // 重置: 可见/预览态/lastData 清空 → 空面板
    h.borrow_mut().reset_preview();
    {
        let fm = h.borrow();
        assert!(fm.visible && fm.base.is_preview, "preview 形态");
    }
    let mut cv2 = PixCanvas::new(w0, h0).unwrap();
    (spec.render)(&mut cv2);
    assert!(!has_ink(&cv2), "重置后无文本行 (Java 新实例空面板)");
}

/// reinit 闭包 (Java reinitConfig): setBlkx(FMManager.current().blkx) — 未就绪
/// 句柄 blkx=None → 清空 (占位容忍); 返回 None (无 setBounds, 高度待下次数据
/// 变更自纠); 清指纹后 render 通道可用
#[test]
fn fm_unpacked_reinit_clears_fmdata_and_keeps_render() {
    let (h, mut spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    h.borrow_mut().reload_fm_data(Some(Arc::new(full_fmdata())));
    assert!(h.borrow().generate_lines().len() >= 44, "重载后有数据");
    assert!(
        (spec.reinit.as_mut().unwrap())().is_none(),
        "reinitConfig 无 setBounds (Java 同 — 返回 None 仅清指纹)"
    );
    assert_eq!(
        h.borrow().generate_lines(),
        vec![
            "FM Data Preview".to_string(),
            "[No Data Loaded]".to_string()
        ],
        "setBlkx(current=None) 清空 → 占位清单"
    );
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0));
}
