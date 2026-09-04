//! PageOverlay — W3 页面的通用编排器 (一页一 OverlaySpec)。
//!
//! 与 minihud 编排器 (HUDData 页面派生) 并列: 本编排器服务通用短名面
//! (FormulaView) 的页面 — 组件 on_data_update 从 env.frame 拉值, 节流
//! 闩留在组件内 (现有实现形态)。preview = env.frame None, 组件走静态值。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use vm_core::base::logger;
use vm_core::config::config_api::HudSettingsSnapshot;
use vm_core::config::json_model::PageDoc;
use vm_core::lang::Lang;
use vm_core::ui_support::row_def::RowDef;

use crate::layout::hud_layout_node::HUDLayoutNodeExt;
use crate::layout::minihud_layout::AutoSizingPlan;
use crate::overlays::minihud::{MinimalHudContext, MiniHudFonts};
use crate::overlays::spec_common::keyed_spec;
use crate::platform::host::OverlaySpec;
use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;
use crate::render::palette::aa;
use crate::render::primitives;

use super::env::{FactoryCtx, StyleEnv, UpdateEnv};
use super::page_layout::{build_page_layout, BuiltPageLayout, PageBuildInputs};
use super::registry::WidgetCell;

// =====================================================================
// 页面编排器
// =====================================================================

/// 通用页面编排器 (W3 页; 数据面分发 + 渲染循环)
pub struct PageOverlay {
    pub cells: HashMap<String, WidgetCell>,
    pub layout: BuiltPageLayout,
    fonts: Rc<MiniHudFonts>,
    /// minihud 族组件的派生上下文 (build 时从 fctx 拷; 缺席 = 页无 minihud 组件)
    minihud_ctx: Option<MinimalHudContext>,
}

/// 共享句柄 (spec render 闭包与喂数方各持一份, 单线程 RefCell)
pub type PageHandle = Rc<RefCell<PageOverlay>>;

/// 页画布语义解析: minihud 页 = ctx 派生 (宽×2 容纳 crosshair 右半区锚定),
/// 其余 = 4096 自由画布。返回 (canvas_w, canvas_h, line_height)。
fn page_canvas(doc: &PageDoc, fctx: &FactoryCtx) -> (i32, i32, f64) {
    if doc.canvas.as_deref() == Some("minihud") {
        if let Some(ctx) = fctx.minihud_ctx {
            return (ctx.width * 2, ctx.height, ctx.hud_font_size as f64);
        }
    }
    (4096, 4096, fctx.fonts.draw.size as f64)
}

impl PageOverlay {
    /// 建树 + 风格注入 (reinit 同一入口整体重建)。
    /// `edit_view` = 编辑器快照: visibleWhen 恒满足 (布局编辑视图显示全部组件,
    /// 条件显隐是运行时行为)。
    pub fn build(
        doc: &PageDoc,
        fctx: &FactoryCtx,
        settings: &HudSettingsSnapshot,
        debug: bool,
        edit_view: bool,
    ) -> Self {
        let visible: &dyn Fn(&str) -> Option<bool> = if edit_view {
            &|_: &str| Some(true)
        } else {
            &|_: &str| None
        };
        let (canvas_w, canvas_h, line_height) = page_canvas(doc, fctx);
        let inputs = PageBuildInputs {
            doc,
            fctx,
            visible_src: visible, // 生产: W3 出厂页键控门控在编排器; 编辑器: 全显
            canvas_w,
            canvas_h,
            line_height,
            debug,
        };
        let layout = build_page_layout(&inputs);
        let cells = layout.cells.clone();
        let fonts = Rc::clone(&fctx.fonts);
        let page = PageOverlay {
            cells,
            layout,
            fonts,
            minihud_ctx: fctx.minihud_ctx.cloned(),
        };
        page.apply_styles(settings);
        page
    }

    fn apply_styles(&self, settings: &HudSettingsSnapshot) {
        let style = StyleEnv {
            fonts: Rc::clone(&self.fonts),
            settings,
            minihud_ctx: self.minihud_ctx.as_ref(),
        };
        for cell in self.cells.values() {
            cell.apply_style(&style);
        }
    }

    /// 数据分发 (节流在组件内; preview = frame None)
    pub fn feed(&mut self, env: &UpdateEnv) {
        for cell in self.cells.values() {
            cell.on_data_update(env);
        }
    }

    /// 渲染循环: doLayout + render (含 debug 线框)
    pub fn draw(&mut self, cv: &mut PixCanvas, aa_on: bool) {
        self.layout.engine.do_layout();
        let engine = &self.layout.engine;
        engine.render(|node, x, y, dbg| match dbg {
            None => {
                let comp = node.borrow().component.clone();
                comp.draw(cv, x, y, aa_on);
            }
            Some(color) => {
                let r = node.get_pixel_rect();
                primitives::ring1px(cv, x, y, r.width, r.height, color);
            }
        });
    }

    /// 自动尺寸计划 (None = 空, 宿主保持初始)
    pub fn sizing(&self) -> Option<AutoSizingPlan> {
        self.layout.sizing
    }
}

// =====================================================================
// spec 工厂 (一页一 OverlaySpec)
// =====================================================================

/// 页面构建参数 (各页差异量的收敛包; 字号由调用方按 dpi 解析)
pub struct PageSpecParams {
    pub doc: PageDoc,
    /// host 条目键覆盖 (默认 doc.switch_key; 推力曲线 "thrustdFS")
    pub entry_key: Option<String>,
    pub font_path: std::path::PathBuf,
    /// 页面主字号 (px, dpi 后)
    pub font_size: i32,
    /// fields.grid 行源
    pub rows: HashMap<String, Arc<Vec<RowDef>>>,
    /// 引擎控制 7 仪表 disable 集
    pub engine_disables: [bool; 7],
    pub lang: Lang,
    pub settings: HudSettingsSnapshot,
    pub debug: bool,
    /// fields.grid 页配置 (font_add, columns — 本页 ListGroup 快照)
    pub fields_cfg: Option<(i32, i32)>,
    /// 仪表组件配置 (dpi/节流/组开关 — ReinitParams 全量快照)
    pub gauge_cfg: super::env::GaugeCfg,
    /// reinit 时重取最新参数 (参数仓快照面; 捕获 Rc<RefCell<ReinitParams>> 族)
    pub refresh: Box<dyn Fn() -> PageSpecParams>,
}

/// 通用页面 spec: 出厂页 → (PageHandle, OverlaySpec)。
/// reinit 闭包经 [`PageSpecParams::refresh`] 重取最新参数整体重建编排器
/// (对位各旧 spec 工厂的 reinit 闭包族)。
pub fn page_overlay_spec(params: PageSpecParams) -> Result<(PageHandle, OverlaySpec), String> {
    let (page, w, h) = build_page(&params)?;
    let handle: PageHandle = Rc::new(RefCell::new(page));
    let render_handle = Rc::clone(&handle);

    let refresh = params.refresh;
    let reinit_handle = Rc::clone(&handle);
    let reinit: crate::platform::host::ReinitFn = Box::new(move || {
        let fresh = refresh();
        match build_page(&fresh) {
            Ok((new_page, nw, nh)) => {
                *reinit_handle.borrow_mut() = new_page;
                Some((nw, nh))
            }
            Err(e) => {
                logger::error("PageOverlay", &format!("reinit 失败: {e}"));
                None
            }
        }
    });

    let switch_key = params
        .entry_key
        .clone()
        .or_else(|| params.doc.switch_key.clone())
        .unwrap_or_else(|| params.doc.id.clone());
    Ok((
        handle,
        keyed_spec(
            &switch_key,
            w,
            h,
            Box::new(move |cv: &mut PixCanvas| {
                render_handle.borrow_mut().draw(cv, aa());
            }),
            Some(reinit),
        ),
    ))
}

fn build_page(p: &PageSpecParams) -> Result<(PageOverlay, i32, i32), String> {
    let font = LoadedFont::new(&p.font_path, p.font_size)?;
    let rc_font = Rc::new(font);
    let fonts = Rc::new(MiniHudFonts {
        draw: Rc::clone(&rc_font),
        small: Rc::clone(&rc_font),
        s_small: rc_font,
    });
    // minihud 族组件的派生 ctx (用户可把 minihud 组件拖进任意页 → 真窗同样可渲染;
    // 字体文件上面已验证可载, create 内部的三份加载必成功)
    let minihud_ctx =
        MinimalHudContext::create(&p.settings, p.gauge_cfg.dpi_scale, &p.font_path).ok();
    let fctx = FactoryCtx {
        minihud_ctx: minihud_ctx.as_ref(),
        fonts,
        rows: &p.rows,
        engine_disables: Some(p.engine_disables),
        lang: Some(&p.lang),
        fonts_dir: Some(p.font_path.parent().map(|x| x.to_path_buf()).unwrap_or_default()),
        fields_cfg: p.fields_cfg,
        // W3B/W3C 复合组件参数 (主线收口: 从 ReinitParams 各组随 refresh 闭包注入;
        // 当前 None → 组件工厂走 GaugeCfg::default 的 Java 回退缺省)
        gauge_cfg: None,
    };
    let page = PageOverlay::build(&p.doc, &fctx, &p.settings, p.debug, false);
    let (w, h) = match page.sizing() {
        Some(s) => (s.new_width, s.new_height),
        None => (300, 200),
    };
    Ok((page, w, h))
}

/// 单档页面字号 (基准 + 增量, dpi 缩放; W3 各页共用口径)
pub fn page_font_size(base: i32, add: i32, dpi: f64) -> i32 {
    ((base as f64 + add as f64) * dpi).round() as i32
}

// =====================================================================
// W4 编辑器快照面 (solve_page): 布局求解 + PNG 预览 — 与真窗同管线
// =====================================================================

/// solve 产物 (编辑器画布的数据面: 矩形集 + 像素快照)
pub struct SolveResult {
    pub line_height_px: i32,
    pub page_w: i32,
    pub page_h: i32,
    /// (id, x, y, w, h) — 锚点求解后的组件矩形 (画布系)
    pub items: Vec<(String, i32, i32, i32, i32)>,
    /// RGBA 直通 PNG 字节流 (与真 overlay 同 PixCanvas+swash 管线 = 像素一致)
    pub png: Vec<u8>,
}

/// 页面快照求解 (编辑器 100ms 防抖调用; 主线程 — Rc 边界内)。
/// 组件 preview 初值 (fields 行 preview 串/仪表半量程), 与真窗 preview 一致。
pub fn solve_page_snapshot(
    doc: &PageDoc,
    fctx: &FactoryCtx,
    settings: &HudSettingsSnapshot,
) -> Result<SolveResult, String> {
    let page = PageOverlay::build(doc, fctx, settings, false, true);
    let line_height_px = page_canvas(doc, fctx).2 as i32;
    // minihud 族 preview 模板 (行文本示例; 与真窗 preview 同源构造)
    let templates = crate::overlays::minihud::preview_templates(
        settings,
        (0.0, 0, crate::overlays::rows::TickScale::default()),
        false,
    );
    for cell in page.cells.values() {
        cell.push_templates(&templates);
    }
    let Some(sizing) = page.sizing() else {
        return Ok(SolveResult {
            line_height_px,
            page_w: 1,
            page_h: 1,
            items: Vec::new(),
            png: Vec::new(),
        });
    };
    // 布局求解后的组件矩形 (engine 的 pixel rect + 偏移)
    let mut items = Vec::new();
    for (id, cell) in &page.cells {
        let _ = cell;
        let node = page.layout.engine.get_node(id);
        if let Some(node) = node {
            let r = node.get_pixel_rect();
            let off = (sizing.offset_x, sizing.offset_y);
            items.push((id.clone(), r.x + off.0, r.y + off.1, r.width, r.height));
        }
    }
    // 渲染快照 (清零重画到独立画布)
    let mut cv = crate::render::canvas::PixCanvas::new(sizing.new_width, sizing.new_height)?;
    let mut page = page;
    page.draw(&mut cv, crate::render::palette::aa());
    let rgba = cv.straight_frame().to_vec();
    let mut png_out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png_out, cv.width() as u32, cv.height() as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(|e| e.to_string())?;
        writer
            .write_image_data(rgba.chunks(4).flat_map(|p| p.iter().copied()).collect::<Vec<_>>().as_slice())
            .map_err(|e| e.to_string())?;
    }
    Ok(SolveResult {
        line_height_px,
        page_w: sizing.new_width,
        page_h: sizing.new_height,
        items,
        png: png_out,
    })
}
