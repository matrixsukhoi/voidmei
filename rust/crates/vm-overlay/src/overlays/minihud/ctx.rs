//! MinimalHudContext 上下文 (src/ui/overlay/MinimalHUDContext.java 一比一):
//! 不可变配置快照 — 全部派生量 (字号/线宽/罗盘直径/rightDraw) 从
//! crossScale×dpiScale 级联; 字体 = 三份 BOLD 字号档 (零分配纪律: Rc 共享)。

use std::path::Path;
use std::rc::Rc;

use vm_core::base::format::java_round_f32;
use vm_core::config::config_api::HUDSettings;

use crate::render::font::LoadedFont;

use super::java_round_long_narrowed;

/// ctx 的三份字体 (Java drawFont/drawFontSmall/drawFontSSmall; BOLD 由字体文件承载)
#[derive(Clone)]
pub struct MiniHudFonts {
    /// drawFont: BOLD hudFontSize
    pub draw: Rc<LoadedFont>,
    /// drawFontSmall: BOLD hudFontSizeSmall
    pub small: Rc<LoadedFont>,
    /// drawFontSSmall: BOLD hudFontSize/2
    pub s_small: Rc<LoadedFont>,
}

/// Immutable context object holding pre-calculated metrics and resources for
/// MinimalHUD. Generated from HUDSettings. (Java javadoc 原文)
///
/// BasicStroke 字段按 rows.rs HUDManeuverRow 口径折为 f32 宽度 —
/// CAP_ROUND+JOIN_ROUND 由 PixCanvas::draw_line 线型族固定承载。
#[derive(Clone)]
pub struct MinimalHudContext {
    // --- Layout Dimensions ---
    pub width: i32,
    pub height: i32,
    pub hud_font_size: i32,
    pub hud_font_size_small: i32,
    pub window_x: i32,
    pub window_y: i32,

    // --- Component Metrics ---
    pub cross_scale: i32,
    pub cross_x: i32,
    pub cross_y: i32,
    pub round_compass: i32,
    pub right_draw: i32,
    pub compass_diameter: i32,
    pub compass_radius: i32,
    pub compass_inner_mark_radius: i32,
    pub aoa_length: f64,

    // --- Styling ---
    pub line_width: i32,
    pub bar_width: i32,
    pub half_line: i32,
    /// drawFont/drawFontSmall/drawFontSSmall (Java Font 字段; Rust 句柄共享)
    pub fonts: MiniHudFonts,
    /// strokeThick = BasicStroke(halfLine+2, CAP_ROUND, JOIN_ROUND) 的宽度
    pub stroke_thick_w: f32,
    /// strokeThin = BasicStroke(halfLine, ...) 的宽度
    pub stroke_thin_w: f32,
    // --- Resources ---
    // crosshairImageScaled (纹理准星双线性缩放缓存) 不迁移 —
    // gauge_crosshair.rs 头注裁决: 软件矢量路径为唯一视觉语义, 配置项裁剪。
}

impl MinimalHudContext {
    /// Factory method to create a context from settings.
    /// Contains all the detailed calculation logic previously in
    /// MinimalHUD.reinitConfig. (Java javadoc 原文)
    ///
    /// DPI Scaling: All pixel dimensions are multiplied by dpiScale
    /// (参数注入, LIFETIMES Env; Java 读 Application.dpiScale)。
    ///
    /// Java `new Font(...)` 恒成功 (家族缺失时 AWT 兜底字体); Rust
    /// LoadedFont::new 读文件可失败 → Result。font_path 由调用方从 settings 的
    /// 字体族名解析 (cfg 缺省 "Sarasa Mono SC" → 随包 sarasa-mono-sc-bold.ttf)。
    pub fn create<S: HUDSettings>(
        settings: &S,
        dpi_scale: f64,
        font_path: &Path,
    ) -> Result<Self, String> {
        // 1. Basic Metrics - apply DPI scaling
        // (int) Math.round(double) = round→long→int 窄化
        let base_cross_scale = settings.get_crosshair_scale();
        let cross_scale = java_round_long_narrowed(base_cross_scale as f64 * dpi_scale);

        let f_add = settings.get_font_size_add();
        // Font size is derived from crossScale (already scaled)
        let mut hud_font_size =
            cross_scale / 4 + java_round_long_narrowed(f_add as f64 * dpi_scale);
        // Ensure minimum size to prevent crash
        if hud_font_size < 8 {
            hud_font_size = 8;
        }

        let bar_width = hud_font_size / 4;
        let mut line_width = if hud_font_size / 10 == 0 {
            1
        } else {
            hud_font_size / 10
        };

        // 2. Window Dimensions - derived from scaled crossScale
        // (int)(double) 强转 = JLS 5.1.3 截断+饱和, 与 Rust as i32 一致
        let width = if !settings.is_display_crosshair() {
            (cross_scale as f64 * 2.25) as i32 - hud_font_size
        } else {
            (cross_scale as f64 * 2.25) as i32
        };
        // 两个独立 (int) 强转后再相加 (Java 原样, 不合并为一个表达式)
        let height = (cross_scale as f64 * 1.5) as i32 + (hud_font_size as f64 * 3.5) as i32;

        let window_x = settings.get_window_x(width);
        let window_y = settings.get_window_y(height);

        let cross_x = width / 2;
        let cross_y = height / 2;

        // 3. Component Details - derived from scaled hudFontSize
        if line_width == 0 {
            line_width = 1;
        }

        // Math.round(hudFontSize * 0.8f) — int*float 提升 float,
        // Math.round(float)→int (非 double 版, §2.3 双语义)
        let round_compass = java_round_f32(hud_font_size as f32 * 0.8f32);

        // Dynamic rightDraw calculation (WYSWYG Overlap Fix)
        // Standard value (~5 chars): 3.5f * fontSize
        // Labeled value (~9 chars): 5.5f * fontSize
        let multiplier = if !settings.is_speed_label_disabled()
            || !settings.is_altitude_label_disabled()
            || !settings.is_sep_label_disabled()
        {
            5.5f32
        } else {
            3.5f32
        };
        // (int)(int * float) — float 乘法后截断 (保 f32 链, §2.12)
        let right_draw = (hud_font_size as f32 * multiplier) as i32;

        let compass_diameter = java_round_long_narrowed(2.0 * hud_font_size as f64 * 0.618);
        let compass_radius = java_round_long_narrowed(compass_diameter as f64 / 2.0);
        let compass_inner_mark_radius = java_round_long_narrowed(0.618 * compass_diameter as f64);

        // Adjusted for dynamic rightDraw
        let aoa_length = right_draw as f64 - hud_font_size as f64 / 1.5;

        // 4. Strokes & Fonts - scaled stroke widths for crisp lines (Java 注释)
        let half_line = if line_width / 2 == 0 {
            1
        } else {
            java_round_f32(line_width as f32 / 2.0f32)
        };
        let stroke_thick_w = (half_line + 2) as f32;
        let stroke_thin_w = half_line as f32;

        let n_font = settings.get_num_font();
        let draw = Rc::new(LoadedFont::new(font_path, hud_font_size)?);
        // (int)(hudFontSize * 0.75f) — float 链截断
        let hud_font_size_small = (hud_font_size as f32 * 0.75f32) as i32;
        let small = Rc::new(LoadedFont::new(font_path, hud_font_size_small)?);
        // new Font(nFont, BOLD, hudFontSize / 2) — int 除法在造 Font 之前
        let s_small = Rc::new(LoadedFont::new(font_path, hud_font_size / 2)?);
        let _ = n_font; // 家族名已由 font_path 承载 (模块头映射裁决)

        // 5. Resource Loading (IO) — 纹理准星链不迁移 (模块头 PORT 注)

        vm_core::base::logger::info(
            "MinimalHUD",
            &format!(
                "MinimalHUD Config: Width={}, Height={}, CrossWidth={}",
                width, height, cross_scale
            ),
        );

        Ok(MinimalHudContext {
            width,
            height,
            hud_font_size,
            hud_font_size_small,
            window_x,
            window_y,
            cross_scale,
            cross_x,
            cross_y,
            round_compass,
            right_draw,
            compass_diameter,
            compass_radius,
            compass_inner_mark_radius,
            aoa_length,
            line_width,
            bar_width,
            half_line,
            fonts: MiniHudFonts {
                draw,
                small,
                s_small,
            },
            stroke_thick_w,
            stroke_thin_w,
        })
    }
}
