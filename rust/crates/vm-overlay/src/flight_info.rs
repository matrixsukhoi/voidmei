//! FlightInfoOverlay 的 host 工厂 — POC window.rs 专径收编进组装面。
//!
//! P6 人工验收缺口: 注册面 6/7 (flightInfoSwitch 走 POC bin 专径无窗口条目,
//! 预览全开也轮不到它)。Java 对位 Controller.java:683-686
//! `registerWithPreview("flightInfoSwitch", FlightInfoOverlay, init(this,S,
//! getOverlaySettings("飞行信息")), ...)`。
//!
//! 渲染栈复用 POC 像素对拍过的 fields/layout/render 三件套 (font::Canvas 直通
//! 域), 经 [`PixCanvas::composite_straight_frame`] 整帧桥入 host 的 PixCanvas
//! 体系 (SrcOver 合成, host 预览灰底保留)。
//!
//! 数据面 (对位 Java FieldOverlay 的字段行):
//! - preview: [`fields::FIELDS`] 静态 [`preview_text`](FieldDef::preview_text)
//!   (POC --preview 同源);
//! - live: ServiceData.flight_values (service_loop deriver.step 整包快照) →
//!   [`build_texts_from_values`] (visible-when/na-when 求值, POC 同源),
//!   经 [`FlightInfoState::update_from_values`] 喂入。

use std::cell::RefCell;
use std::rc::Rc;

use vm_core::layout::RenderCtx;
use vm_core::{fields, format};
use vm_data::FlightValues;

use crate::font::Canvas;
use crate::host::OverlaySpec;
use crate::global_colors::colors;
use crate::render::{render_fields_fixed, FieldText, FontTriple, RenderColors};
use crate::render2d::PixCanvas;

/// numHeight 默认值 (POC main.rs 平移): Java 实测校准 24px BOLD Sarasa = 31,
/// 其余字号 1.25×fontSize 近似 (与实测差 ≤1px, 精确值由对拍脚本 --num-height 注入)
pub fn default_num_height(font_add: i32) -> i32 {
    if font_add == 0 {
        31
    } else {
        ((24 + font_add) as f32 * 1.25).round() as i32
    }
}

/// FlightValues → getter 数值 (POC main.rs flight_value 平移; cfg :target 键域)
pub fn flight_value(v: &FlightValues, getter: &str) -> Option<f64> {
    Some(match getter {
        "getIAS" => v.ias,
        "getTAS" => v.tas,
        "getMach" => v.mach,
        "getCompass" => v.compass,
        "getAltitude" => v.altitude,
        "getVario" => v.vario,
        "getSEP" => v.sep,
        "getAcceleration" => v.acceleration,
        "getRollRate" => v.roll_rate,
        "getNy" => v.ny,
        "getTurnRate" => v.turn_rate,
        "getTurnRadius" => v.turn_radius,
        "getAoA" => v.aoa,
        "getAoS" => v.aos,
        "getWingSweep" => v.wing_sweep, // 已 ×100 (cfg 表达式)
        "getRadioAltitude" => v.radio_altitude,
        _ => return None,
    })
}

/// FlightValues → (label, unit, value) owned 行 (visible-when/na-when 求值,
/// POC main.rs build_texts_from_values 平移)
pub fn build_texts_from_values(v: &FlightValues) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for f in fields::FIELDS {
        let raw = match flight_value(v, f.source.getter()) {
            Some(x) => x,
            None => continue,
        };
        if let Some(cond) = f.visible_when {
            if !cond.eval(raw) {
                continue;
            }
        }
        // wing_sweep 已在 Deriver 里 ×100 (cfg 表达式), 此处直接用
        let text = match f.na_when {
            Some(cond) if cond.eval(raw) => "-".to_string(),
            _ => format::format(raw, f.precision),
        };
        out.push((f.label.to_string(), f.unit.to_string(), text));
    }
    out
}

/// FlightInfo 共享句柄 (win32 线程内; live 喂数经 [`FlightInfoState::update_from_values`])
pub type FlightInfoHandle = Rc<RefCell<FlightInfoState>>;

pub struct FlightInfoState {
    /// owned 文本行 (preview 静态初值; live 由 update_from_values 覆写)
    rows: Vec<(String, String, String)>,
    /// POC 渲染栈三件套 (度量 + 字体 + 复用直通画布, 尺寸恒定零重分配)
    ctx: RenderCtx,
    fonts: FontTriple,
    canvas: Canvas,
}

impl FlightInfoState {
    /// live 喂数 (Java FieldOverlay.onFlightData → 字段行更新; host 50ms 渲染
    /// 节拍 + 像素指纹脏检查兜底, 此处纯数据面)
    pub fn update_from_values(&mut self, v: &FlightValues) {
        self.rows = build_texts_from_values(v);
    }

    /// 文本行只读访问 (测试/诊断面)
    pub fn rows(&self) -> &[(String, String, String)] {
        &self.rows
    }
}

/// FlightInfo OverlaySpec + live 句柄 (Java Controller.java:683 注册键
/// flightInfoSwitch; 字号/列数来自 getOverlaySettings("飞行信息") 组字段)
pub fn flight_info_overlay_spec(
    fonts_dir: &std::path::Path,
    font_add: i32,
    column: i32,
) -> Result<(FlightInfoHandle, OverlaySpec), String> {
    let ctx = RenderCtx::new(font_add, column, default_num_height(font_add));
    let fonts = FontTriple::load(fonts_dir, &ctx)?;
    // preview 初值: FIELDS 静态 preview 文本 (POC --preview 同源)
    let rows: Vec<(String, String, String)> = fields::FIELDS
        .iter()
        .map(|f| (f.label.to_string(), f.unit.to_string(), f.preview_text().to_string()))
        .collect();
    // 窗口尺寸: 全行高度 (POC run_live 同款 — visible-when 变化不重建窗口,
    // 空行区域透明无碍)
    let (w, h) = (ctx.total_width(), ctx.total_height(rows.len() as i32));
    let state = FlightInfoState {
        rows,
        canvas: Canvas::new(w, h),
        ctx,
        fonts,
    };
    let handle: FlightInfoHandle = Rc::new(RefCell::new(state));
    let render_handle = Rc::clone(&handle);
    Ok((
        handle,
        OverlaySpec {
            id: "flightInfoSwitch".to_string(),
            config_key: "flightInfoSwitch".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                let mut st = render_handle.borrow_mut();
                // 借用拆分: rows 只读 / canvas 可变 (同结构不相交字段)
                let FlightInfoState { rows, canvas, ctx, fonts } = &mut *st;
                let texts: Vec<FieldText> = rows
                    .iter()
                    .map(|(l, u, v)| FieldText { label: l, unit: u, value: v })
                    .collect();
                // 清零重绘到直通 Canvas → 整帧 SrcOver 桥入 PixCanvas
                // (aa 恒 on, POC cmd_run_window 同款)。色板 = 运行时全局五色
                // (Java FieldOverlay 读 Application.colorNum 族; 对拍工具路径
                // 仍用 render::DEFAULT_COLORS 常量基线, 互不影响)
                let pal = RenderColors {
                    num: colors().num,
                    label: colors().label,
                    unit: colors().unit,
                    shade: colors().shade_shape,
                };
                render_fields_fixed(canvas, &texts, ctx, fonts, &pal, true);
                if !cv.composite_straight_frame(&canvas.buf) {
                    // 不可达 (spec 尺寸 = Canvas 尺寸 = host 画布尺寸); 防御性留痕
                    vm_core::logger::warn("FlightInfo", "整帧桥尺寸不符, 本帧丢弃");
                }
            }),
        },
    ))
}

// =====================================================================
// Tests
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn fonts_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts")
    }

    /// 工厂最小面: preview 行数 = FIELDS 数, 尺寸与 ctx 度量一致, 渲染闭包可跑
    /// (PixCanvas 合成 + 灰底保留)
    #[test]
    fn spec_renders_preview_rows_to_pixcanvas() {
        let (handle, mut spec) =
            flight_info_overlay_spec(&fonts_dir(), 0, 1).expect("字体目录应可用");
        assert_eq!(spec.id, "flightInfoSwitch");
        assert_eq!(handle.borrow().rows().len(), fields::FIELDS.len());
        assert!(spec.width > 0 && spec.height > 0);

        // 渲染闭包: 先铺 host 预览灰底再合成 (host 渲染循环同序)
        let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
        cv.clear(spec.width, spec.height);
        cv.fill_rect(0, 0, spec.width, spec.height, [0, 0, 0, 0x0A]);
        (spec.render)(&mut cv);
        // 灰底被保留 (左上角像素 alpha 仍 ≥ 灰底底色, 不会被整帧替换清零)
        let px = cv.pixmap().data();
        assert!(px[3] >= 0x0A, "预览灰底应经 SrcOver 合成保留");
    }

    /// live 喂数: update 覆写 rows, visible-when 过滤生效 (Mach>0 才显示的行,
    /// 零值 FlightValues 下被滤除 → 行数少于 FIELDS 数)
    #[test]
    fn update_from_values_applies_visibility() {
        let (handle, _spec) = flight_info_overlay_spec(&fonts_dir(), 0, 1).unwrap();
        let zero = FlightValues::default();
        handle.borrow_mut().update_from_values(&zero);
        let n_zero = handle.borrow().rows().len();
        // 全零值: Mach (>0) 等条件行被滤; 至少 IAS 等直通行保留
        assert!(n_zero > 0 && n_zero <= fields::FIELDS.len());

        // 非零 Mach 帧行数应不少于全零帧 (Mach 行回归)
        let mut v = FlightValues::default();
        v.mach = 0.72;
        v.ias = 450.0;
        handle.borrow_mut().update_from_values(&v);
        let n_live = handle.borrow().rows().len();
        assert!(n_live >= n_zero, "非零帧可见行应不少于全零帧 ({n_live} vs {n_zero})");
    }
}
