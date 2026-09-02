//! GearFlapsOverlay (ui/overlay/GearFlapsOverlay.java) — 起落架/襟翼状态条。
//! 重构波2 自 overlays_field1.rs 拆出。
//!
//! 襟翼竖条 (UIBaseElements.drawVBarTextNum) + 起落架/减速板状态告警文本;
//! onFlightData 100ms 节流。公共节流常量 FIELD_OVERLAY_REFRESH_INTERVAL_MS
//! 随本文件 (PowerInfo 同源消费)。

use std::cell::RefCell;
use std::rc::Rc;

use crate::render::palette::{aa, colors};
use crate::render::font::LoadedFont;
use crate::platform::host::{OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::render::canvas::PixCanvas;
use crate::render::primitives::{draw_h_rect, ring1px, text_shaded_auto};
use vm_core::base::format::java_round_f64;
use vm_core::base::format::java_round_f32;
use vm_core::formula::registry::FormulaView;
use vm_core::lang::Lang;

/// Throttling to prevent EDT task accumulation (FieldOverlay.java:37-38
/// REFRESH_INTERVAL_MS) — 公共节流常量 (PowerInfo 等 FieldOverlay 族同源)
pub const FIELD_OVERLAY_REFRESH_INTERVAL_MS: i64 = 50;

/// UIBaseElements.drawVBar (UIBaseElements.java:112-130): 竖条 (底对齐, shade 环 +
/// c 内芯); val_height<0 分支为条自 y 向下生长 (GearFlaps 值域 0..100 不可达, 保真保留)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawVBar(g2d,x,y,width,height,val_height,borderwidth,c)
fn draw_v_bar(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    val_h: i32,
    bw: i32,
    c: [u8; 4],
) {
    if val_h >= 0 {
        ring1px(cv, x, y - h, w - 1, h - 1, colors().shade_shape);
        cv.fill_rect(x + bw, y + bw - val_h, w - 2 * bw, val_h - 2 * bw, c);
    } else {
        ring1px(cv, x, y, w - 1, -h - 1, colors().shade_shape); // 负高 → 不绘制
        cv.fill_rect(x + bw, y + bw, w - 2 * bw, -val_h - 2 * bw, c);
    }
}

/// UIBaseElements.drawVBarTextNum (UIBaseElements.java:144-154): 竖条 + 随值指针横线 +
/// 数值文本。lbl 形参在 Java 中传入后未绘制 (drawVBarText 的标签绘制已注释), 保真保留
#[allow(clippy::too_many_arguments)] // 对齐 Java drawVBarTextNum(g2d,x,y,width,height,val_height,borderwidth,c,lbl,num,lblFont,numFont)
fn draw_v_bar_text_num(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    val_h: i32,
    bw: i32,
    c: [u8; 4],
    _lbl: &str,
    num: &str,
    _lbl_font: &LoadedFont,
    num_font: &LoadedFont,
    aa: bool,
) {
    let val_h = if val_h > h { h } else { val_h };
    draw_v_bar(cv, x, y, w, h, val_h, bw, c);
    // 指针横线 (drawHRect): colorLabel, 总宽 = width + 3*numFontSize
    draw_h_rect(cv, x, y - val_h - 1, w + 3 * num_font.size, 3, 1, colors().label);
    // 数值文本: shade (+1,+1) + 本色 colorLabel (基线 y-val_height-2)
    text_shaded_auto(cv, num_font, x + w, y - val_h - 2, num, colors().label, aa);
}

/// Throttling to prevent EDT task accumulation (gear/flaps are low-frequency data)
/// (GearFlapsOverlay.java:28-29 REFRESH_INTERVAL_MS)
pub const GEAR_FLAPS_REFRESH_INTERVAL_MS: i64 = 100;

/// 起落襟翼面板状态: 几何 (reinitConfig) + 动态数据 (drawTick) + 绘制 (paintComponent)
pub struct GearFlapsState {
    /// 节流基准 (GearFlapsOverlay.java:30 lastRefreshTime, System.currentTimeMillis 毫秒)
    pub last_refresh_time: i64,
    pub font_size: i32,
    pub bar_width: i32,
    pub bar_height: i32,
    /// 内容区宽 (2*fontSize)
    pub width: i32,
    /// 内容区高 (5*fontSize)
    pub height: i32,
    /// 窗口总宽 (width + 4*fontSize + sw*2)
    pub total_width: i32,
    /// 窗口总高 (height + sw*2)
    pub total_height: i32,
    /// 襟翼填充像素高
    pub flap_pix: i32,
    /// 襟翼百分比文本 (Java "%3d")
    pub flap_text: String,
    /// 状态告警文本 (起落架/减速板)
    pub warn_text: String,
    pub warn_color: [u8; 4],
}

impl GearFlapsState {
    /// reinitConfig 几何段 (GearFlapsOverlay.java:95-142)。
    /// show_edge = enablegearAndFlapsEdge 开关 (sw=10)
    pub fn new(font_add: i32, dpi_scale: f64, show_edge: bool) -> Self {
        // fontSize = round((24 + fontadd) * dpiScale)
        let font_size = java_round_f64((24.0 + font_add as f64) * dpi_scale);
        let bar_width = font_size >> 1;
        let bar_height = 4 * font_size;
        let width = 2 * font_size;
        let height = 5 * font_size;
        // 初始 (预览) 襟翼 50%
        let flap_pix = bar_height * 50 / 100;
        let flap_text = format!("{:>3}", 50);
        let sw = if show_edge { 10 } else { 0 };
        GearFlapsState {
            last_refresh_time: 0,
            font_size,
            bar_width,
            bar_height,
            width,
            height,
            total_width: width + 4 * font_size + sw * 2,
            total_height: height + sw * 2,
            flap_pix,
            flap_text,
            warn_text: String::new(),
            warn_color: colors().num,
        }
    }

    /// onFlightData → drawTick (GearFlapsOverlay.java:199-256) 的单事件语义:
    /// 100ms 节流闩 → (invokeLater lambda 内) drawTick (:220-256): 起落架/减速板
    /// 状态文本 + 襟翼像素/文本。
    /// PORT: System.currentTimeMillis 由调用方注入 now_ms (field2 先例); 返回
    /// false = 节流跳过 (Java 原方法 void, 宿主可据此省重绘)
    pub fn update_tick(&mut self, now_ms: i64, lang: &Lang, s: &dyn FormulaView) -> bool {
        // Throttling prevents EDT task accumulation
        if now_ms - self.last_refresh_time < GEAR_FLAPS_REFRESH_INTERVAL_MS {
            return false; // Skip this update, too soon
        }
        self.last_refresh_time = now_ms;
        // Java (int) 强转截断; 值域 0..100
        let gear = s.var_value("gear").unwrap_or(0.0) as i32;
        let mut flaps = s.var_value("flaps").unwrap_or(0.0) as i32;
        let airbrake = s.var_value("airbrake").unwrap_or(0.0) as i32;

        if gear >= 0 {
            if gear == 0 {
                self.warn_text.clear();
                self.warn_color = colors().num;
            } else if gear == 100 {
                self.warn_text = lang.g_gear.to_string();
                self.warn_color = colors().num;
            } else {
                self.warn_text = lang.g_gear_down.to_string();
                self.warn_color = colors().warning;
            }
            if airbrake > 0 {
                self.warn_text.push(' ');
                self.warn_text.push_str(lang.g_brake);
                self.warn_color = colors().warning;
            }
        }
        // gear < 0 (无数据): 保留上次告警状态 (Java 同)

        if flaps >= 0 {
            self.flap_pix = flaps * self.bar_height / 100;
        } else {
            self.flap_pix = 0;
            flaps = 0;
        }
        self.flap_text = format!("{:>3}", flaps);
        true
    }

    /// paintComponent (GearFlapsOverlay.java:158-187)
    pub fn draw(
        &self,
        cv: &mut PixCanvas,
        font_num: &LoadedFont,
        font_label: &LoadedFont,
        aa: bool,
    ) {
        let fs = self.font_size;
        let mut dy = fs >> 1;
        // 已经有指示条, 不需要文字了. 暂时注释掉, 不删除.
        // (Java 注释掉的 drawLabelBOSType 调用原位保留于此)
        dy += self.bar_height;
        // 条画在 (0, dy), 数值 "F"+flapText
        let num = format!("F{}", self.flap_text);
        draw_v_bar_text_num(
            cv, 0, dy, self.bar_width, self.bar_height, self.flap_pix, 1, colors().num,
            "", &num, font_num, font_label, aa,
        );
        // 告警文本: (width, baseline=fontSize), fontLabel, 无阴影
        // PORT: Java 判 warnText != null (恒真, 空串绘制无输出), 空串等价无绘制
        cv.draw_text(font_label, self.width, fs, &self.warn_text, self.warn_color, aa);
    }
}

// ---------------------------------------------------------------------------
// live 喂数形态工厂 (minihud_overlay_spec 先例: render 闭包与喂入方共享句柄)
// ---------------------------------------------------------------------------

/// 起落襟翼共享句柄
pub type GearFlapsHandle = Rc<RefCell<GearFlapsState>>;

/// 起落襟翼 OverlaySpec + live 句柄 (Java Controller.java:709 注册键 enablegearAndFlaps)。
/// 初始态 = 襟翼 50% 无告警 (new 的预览初值), 游戏模式由喂入方 update_tick 推进。
/// PORT(WYSIWYG): 字号/边缘开关随 [`ReinitParams`] 仓 — reinit 闭包重建几何 +
/// 双字体 (Java reinitConfig :95-142), 返回新 (total_width, total_height)
pub fn gear_flaps_overlay_spec(
    fonts_dir: &std::path::Path,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(GearFlapsHandle, OverlaySpec), String> {
    let (font_add, dpi_scale, show_edge) = {
        let p = params.borrow();
        (p.font_add_gear, p.dpi_scale, p.gear_show_edge)
    };
    let state = GearFlapsState::new(font_add, dpi_scale, show_edge);
    let bold = fonts_dir.join("sarasa-mono-sc-bold.ttf");
    // fontNum = BOLD(fontSize); fontLabel = BOLD(round(fontSize/2.0f)) (reinitConfig)
    let font_num = Rc::new(RefCell::new(Rc::new(LoadedFont::new(
        &bold,
        state.font_size,
    )?)));
    let font_label = Rc::new(RefCell::new(Rc::new(LoadedFont::new(
        &bold,
        java_round_f32(state.font_size as f32 / 2.0),
    )?)));
    let (w, h) = (state.total_width, state.total_height);
    let handle: GearFlapsHandle = Rc::new(RefCell::new(state));
    let render_handle = Rc::clone(&handle);
    // reinit 闭包: 几何 + 双字体重建 (Java reinitConfig 同段; flap 50%/warn 清空
    // 的预览复位语义原样保留)
    let reinit_handle = Rc::clone(&handle);
    let (reinit_num, reinit_label) = (Rc::clone(&font_num), Rc::clone(&font_label));
    let reinit_params = Rc::clone(params);
    let reinit_bold = bold;
    let reinit: ReinitFn = Box::new(move || {
        let (fa, dpi, edge) = {
            let p = reinit_params.borrow();
            (p.font_add_gear, p.dpi_scale, p.gear_show_edge)
        };
        let new_state = GearFlapsState::new(fa, dpi, edge);
        let (num, label) = match (
            LoadedFont::new(&reinit_bold, new_state.font_size),
            LoadedFont::new(&reinit_bold, java_round_f32(new_state.font_size as f32 / 2.0)),
        ) {
            (Ok(n), Ok(l)) => (Rc::new(n), Rc::new(l)),
            (r, _) => {
                if let Err(e) = r {
                    vm_core::base::logger::error("GearFlaps", &format!("reinit 字体重载失败: {}", e));
                }
                return None;
            }
        };
        let (w, h) = (new_state.total_width, new_state.total_height);
        *reinit_handle.borrow_mut() = new_state;
        *reinit_num.borrow_mut() = num;
        *reinit_label.borrow_mut() = label;
        Some((w, h))
    });
    Ok((
        handle,
        OverlaySpec {
            id: "enablegearAndFlaps".to_string(),
            config_key: "enablegearAndFlaps".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                // aa = 运行时仓 (cfg AAEnable 可关 — 同 engine_control 先例)
                let (num, label) = (font_num.borrow(), font_label.borrow());
                render_handle.borrow().draw(cv, &num, &label, aa());
            }),
            reinit: Some(reinit),
        },
    ))
}
