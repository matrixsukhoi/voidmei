//! PageOverlay — W3 页面的通用编排器 (一页一 OverlaySpec)。
//!
//! 与 minihud 编排器 (HUDData 页面派生) 并列: 本编排器服务通用短名面
//! (FormulaView) 的页面 — 组件 on_data_update 从 env.frame 拉值, 节流
//! 闩留在组件内 (现有实现形态)。preview = env.frame None, 组件走静态值。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kernel::base::logger;
use kernel::config::config_api::HudSettingsSnapshot;
use kernel::config::json_model::PageDoc;
use kernel::lang::Lang;

use crate::layout::hud_layout_node::HUDLayoutNodeExt;
use crate::layout::minihud_layout::AutoSizingPlan;
use crate::overlays::minihud::{MinimalHudContext, MiniHudFonts};
use crate::overlays::spec_common::keyed_spec_id;
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
    /// D2 缩略图组装面 (build 时从 fctx 拷; thumbnail_ctx 重组 FactoryCtx 用)
    fonts_dir: Option<std::path::PathBuf>,
    /// 本地化文案源 (engine.gauge 工厂硬依赖 lang, 缺席其小样即失败 → 存克隆)
    lang: Option<Lang>,
    gauge_cfg: Option<super::env::GaugeCfg>,
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
        // 生产: visibleWhen 的配置键求值 = settings.bools 全键面; 键不在快照
        // (数据条件/未知键) 宽容建成 — 数据条件由组件 props.visibleWhen
        // 运行时承担; 编辑器: 全显
        let visible: &dyn Fn(&str) -> Option<bool> = if edit_view {
            &|_: &str| Some(true)
        } else {
            &|k: &str| settings.bools.get(k).copied()
        };
        let visible_default = !edit_view;
        let (canvas_w, canvas_h, line_height) = page_canvas(doc, fctx);
        let inputs = PageBuildInputs {
            doc,
            fctx,
            visible_src: visible,
            visible_default,
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
            fonts_dir: fctx.fonts_dir.clone(),
            lang: fctx.lang.cloned(),
            gauge_cfg: fctx.gauge_cfg.cloned(),
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

    /// 运行时包围盒收敛 (sidecar 数据推进后): 重算布局 → 返回新窗口尺寸。
    /// fm-list 原子页的窗口高度跟随面 — 行归零/恢复 → 链式补位 → 包围盒
    /// 收缩/扩张; 替代旧 fm.list sidecar 的行数滞回 Resize (无滞回, 每 tick
    /// 收敛一步)。尺寸无变化时调用方按 entry 现值比较后免 resize (保持脏检查)。
    pub fn refresh_sizing(&mut self, padding: i32) -> Option<(i32, i32)> {
        let plan = self.layout.engine.apply_auto_sizing(padding);
        let (w, h) = (plan.new_width, plan.new_height);
        self.layout.sizing = Some(plan);
        Some((w, h))
    }

    // ---- R6 编辑面便捷访问 (真窗即画布: 命中/装饰/单组件重建) ----

    /// 换装单组件实例 (编辑面改 props 后单组件重建; node.component + cells 双替换)
    pub fn set_component_cell(&mut self, id: &str, cell: WidgetCell) -> bool {
        let Some(node) = self.layout.engine.get_node(id) else {
            return false;
        };
        node.borrow_mut().component = cell.clone();
        self.cells.insert(id.to_string(), cell);
        true
    }

    /// 行高 (坐标换算基: 画布 px ↔ pos 单位)
    pub fn line_height(&self) -> f64 {
        self.fonts.draw.size as f64
    }

    /// 页面主字体 (编辑会话右键菜单的文本渲染/度量共用)
    pub fn draw_font(&self) -> Rc<crate::render::font::LoadedFont> {
        Rc::clone(&self.fonts.draw)
    }

    /// D2 组件库缩略图的工厂环境 (build 同源参数的再借出 — 编辑器侧栏
    /// 不碰 fctx 细节, 一站取齐全部输入)
    pub fn thumbnail_ctx(&self) -> FactoryCtx<'_> {
        FactoryCtx {
            minihud_ctx: self.minihud_ctx.as_ref(),
            fonts: Rc::clone(&self.fonts),
            lang: self.lang.as_ref(),
            fonts_dir: self.fonts_dir.clone(),
            gauge_cfg: self.gauge_cfg.as_ref(),
        }
    }
}

// =====================================================================
// spec 工厂 (一页一 OverlaySpec)
// =====================================================================

/// 页面构建参数 (各页差异量的收敛包; 字号由调用方按 dpi 解析)
pub struct PageSpecParams {
    pub doc: PageDoc,
    pub font_path: std::path::PathBuf,
    /// 页面主字号 (px, dpi 后)
    pub font_size: i32,
    pub lang: Lang,
    pub settings: HudSettingsSnapshot,
    pub debug: bool,
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

    // R3 声明式: 实例键 = 页文档 id (位置存档/激活探测关联键统一);
    // 激活键 = activation.key (host 探测经 id 查策略表), 无 activation = 空
    // (恒显策略)。thrustdFS 别名/出厂键分叉等旧式全部退役
    let key = params
        .doc
        .activation
        .as_ref()
        .map(|a| a.key.clone())
        .unwrap_or_default();
    Ok((
        handle,
        keyed_spec_id(
            &params.doc.id,
            &key,
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
    let rc_font = LoadedFont::new_cached(&p.font_path, p.font_size)?;
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
        lang: Some(&p.lang),
        fonts_dir: Some(p.font_path.parent().map(|x| x.to_path_buf()).unwrap_or_default()),
        // 复合组件参数真值 (dpi/组字号/节流 — assemble 装配; 缺席仅测试/兜底)
        gauge_cfg: Some(&p.gauge_cfg),
    };
    let page = PageOverlay::build(&p.doc, &fctx, &p.settings, p.debug, false);
    let (w, h) = match page.sizing() {
        Some(s) => (s.new_width, s.new_height),
        None => (300, 200),
    };
    // (F 修复: fm-list 高度钳制特判退役 — 单列表组件自管高度,
    // refresh_sizing 包围盒收敛链承接)
    Ok((page, w, h))
}

/// 单档页面字号 (基准 + 增量, dpi 缩放; W3 各页共用口径)
pub fn page_font_size(base: i32, add: i32, dpi: f64) -> i32 {
    ((base as f64 + add as f64) * dpi).round() as i32
}

// (R8: solve 快照面退役 — 真窗即画布, 编辑装饰经 EditBridge.on_paint)

