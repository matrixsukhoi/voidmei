//! fm_sidecar — W3C FM 黑盒组件 (数据面走渲染线程节拍 tick, 非 FormulaView 喂数)。
//!
//! FM 拆包列表与推力曲线图的共同形态: 数据供给是 200ms/1000ms 自节流泵 +
//! FMManager 句柄直读 + 自管可见性/动态高度 (host 交互), 与通用短名喂数
//! (on_data_update 逐帧) 不同源 — 故经 [`WidgetSidecar`] 暴露第二数据面,
//! host 交互 (resize/visible/close) 以 [`SidecarAction`] 返回值承载, 渲染线程
//! 消费落 host (对位旧 FmUnpackedFeed/DrawFrameSimplFeed 泵的直接调用面)。
//!
//! 组件包现有生产 state (fm_unpacked.rs / draw_frame_simpl.rs 不动), 其
//! `on_data_update` 恒空 — 数据全走 sidecar tick。

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use vm_core::config::config_api::ConfigProvider;
use vm_core::fm::FMManager;
use vm_core::formula::registry::FormulaView;

use crate::layout::hud_layout_node::Dimension;
use crate::overlays::draw_frame_simpl::{DrawFrameSimpl, DfsFonts};
use crate::overlays::fm_unpacked::{generate_lines, FmUnpackedDataOverlay};
use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;

use super::env::{FactoryCtx, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::PageFonts;
use super::registry::{HudWidget, PropSchema, WidgetCategory, WidgetMeta};

// =====================================================================
// sidecar 契约
// =====================================================================

/// sidecar tick 的 host 交互动作 (渲染线程消费落 host:
/// `Resize` → resize_entry / `SetVisible` → set_entry_visible /
/// `SetVisibleResize` → 双落 / `Close` → close + set_entry_zombie)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidecarAction {
    /// 无窗口动作 (纯数据推进)
    None,
    /// 窗口尺寸变化 (FmUnpacked 高度自适应 — adjustPosition 的 setSize 副作用)
    Resize(i32, i32),
    /// 逐条目可见性 (run() 双分支的 setVisible 落地)
    SetVisible(bool),
    /// 同轮双动作 (可见门控 + 高度自适应并发)
    SetVisibleResize(bool, i32, i32),
    /// 条目销毁 (DrawFrameSimpl 10s 自动退场: host.close + zombie 双落)
    Close,
}

/// sidecar tick 的输入面 (渲染线程按节拍组装; 数据源 = FMManager 句柄 +
/// live 帧快照, 不经 FlightDataBus 事件)
pub struct SidecarCtx<'a> {
    pub now_ms: i64,
    /// 组件所在页面/条目 id (host 交互动作的消费键)
    pub page_id: &'a str,
    /// FM 句柄直读源 (current() 快照; 绝不触发加载)
    pub fm: &'a FMManager,
    /// FM show* 开关快照读 (generateLines 逐 tick 直读面)
    pub fm_field_config: &'a dyn Fn(&str) -> Option<String>,
    /// Application.displayFmKey 对位 (推力曲线 1000ms 节流/自动退场门控)
    pub display_fm_key: i32,
    /// live 帧快照 (None = 预览无 Service — 退场判定冻结保窗口)
    pub frame: Option<&'a dyn FormulaView>,
    /// jetOnly 激活策略的会话标志 (渲染线程注入)
    pub is_jet: bool,
    /// FM_OVERLAY_TOGGLE 事件脉冲 (本节拍内到达 = 翻转可见)
    pub toggle_pulse: bool,
    /// openpad 会话脉冲 (本节拍内游戏形态启动: is_preview=false + 隐藏起步
    /// + FM 缓存直读 — 原 on_open_all 的两 init 段)
    pub game_mode_pulse: bool,
    /// FM_CHANGED 最新载荷 (本节拍内; tick 消费 — FmList 的 reload 面)
    pub fm_changed: Option<Arc<vm_core::fm::data::FmData>>,
}

/// FM 黑盒组件的特殊数据面 (tick 由渲染线程节拍驱动; host 交互面见 [`SidecarAction`])
pub trait WidgetSidecar {
    fn tick(&mut self, ctx: &mut SidecarCtx) -> SidecarAction;
}

/// SidecarCtx.fm_field_config (show* 快照闭包) → ConfigProvider 适配
/// (generateLines 的实参面; 只读 — set/is_field_disabled 无消费方)
struct ConfigFn<'a>(&'a dyn Fn(&str) -> Option<String>);

impl ConfigProvider for ConfigFn<'_> {
    fn get_config(&self, key: &str) -> Option<String> {
        (self.0)(key)
    }
    fn set_config(&self, _key: &str, _value: &str) {}
    fn is_field_disabled(&self, _key: &str) -> bool {
        false
    }
}

// =====================================================================
// core.fm.list — FM 拆包数据列表
// =====================================================================

/// FM 拆包列表复合组件 (包 [`FmUnpackedDataOverlay`] + 200ms tick 泵逻辑)。
/// 构造参数口径 (旧 spec 工厂同源, W3 组件化继承):
/// 逻辑高/字号 12/字体 regular 14+add, 初始态 = initPreview 形态 (恒可见 + 空数据)。
pub struct FmListWidget {
    state: FmUnpackedDataOverlay,
    /// sarasa regular 14+fm_font_add (旧工厂 FontSlot 同口径)
    font: LoadedFont,
    canvas: PixCanvas,
    /// 200ms 节流基准 (泵节拍; 0 = 首轮放行)
    last_ms: i64,
    /// 上轮窗口尺寸 (resize 判定的 before 快照)
    last_size: (i32, i32),
}

fn f_fm_list(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let cfg = fctx.gauge_cfg.cloned().unwrap_or_default();
    let mut state =
        FmUnpackedDataOverlay::new(cfg.logical_height, cfg.dpi_scale, 12);
    let font = LoadedFont::new(
        &fctx
            .fonts_dir
            .as_deref()
            .ok_or("fm.list 需要 FactoryCtx.fonts_dir")?
            .join("sarasa-mono-sc-regular.ttf"),
        14 + cfg.fm_font_add,
    )?;
    // config = None: show* 开关面由 sidecar tick 经 ctx.fm_field_config 现取
    state.init_preview(None, &font);
    let canvas = PixCanvas::new(state.base.width, state.base.height)?;
    let last_size = (state.base.width, state.base.height);
    Ok(Box::new(FmListWidget {
        state,
        font,
        canvas,
        last_ms: 0,
        last_size,
    }))
}

impl FmListWidget {
    /// 内部 state 只读借出 (测试断言面: 会话形态 visible/is_preview; 生产勿用)
    pub fn state(&self) -> &FmUnpackedDataOverlay {
        &self.state
    }
}

impl HudWidget for FmListWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, _env: &UpdateEnv) {
        // 数据面全走 sidecar tick (200ms 自节流 + FM 直读)
    }

    fn reset_preview(&mut self) {
        // CloseAll 后回 preview 会话形态 (visible=true/is_preview=true/lastData=null)
        self.state.reset_preview();
    }

    fn sidecar(&mut self) -> Option<&mut (dyn WidgetSidecar + 'static)> {
        let s: &mut dyn WidgetSidecar = self;
        Some(s)
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        let (w, h) = (self.state.base.width, self.state.base.height);
        if !self.canvas.clear(w, h) {
            return;
        }
        let (state, canvas, font) = (&mut self.state, &mut self.canvas, &self.font);
        state.render(canvas, font, aa);
        let frame = canvas.straight_frame();
        if !cv.composite_straight_frame_at(x, y, frame, w, h, aa) {
            vm_core::base::logger::warn("fm.list", "伴画布尺寸与缓冲不符, 本帧丢弃");
        }
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        // 构造期快照 (动态高度由 sidecar Resize 落 host, 页面布局不跟随)
        Dimension::new(self.state.base.width, self.state.base.height)
    }
}

impl WidgetSidecar for FmListWidget {
    /// 泵的单轮搬运 (旧 FmUnpackedFeed 同语义): 200ms 节流 → tick (可见门控/取数/
    /// 脏检查/高度自适应; panic 吞后冻结) → 可见性 + 尺寸变化落动作返回。
    fn tick(&mut self, ctx: &mut SidecarCtx) -> SidecarAction {
        // 事件面 (渲染线程节拍收集的脉冲在此消费)
        if ctx.game_mode_pulse {
            // Java init :57-94 单实例对位: isPreview=false + initially hidden
            // + fmDataAdapter.setBlkx(current().blkx)
            self.state.base.is_preview = false;
            self.state.visible = false;
            self.state.reload_fm_data(ctx.fm.current().fmdata.clone().map(Arc::new));
        }
        if ctx.toggle_pulse {
            self.state.toggle();
        }
        if let Some(d) = ctx.fm_changed.take() {
            self.state.reload_fm_data(Some(d));
        }
        // getRefreshInterval() = 200ms (base 字段单一真相源)
        let interval_ms = self.state.base.refresh_interval_ms as i64;
        if ctx.now_ms.saturating_sub(self.last_ms) < interval_ms {
            return SidecarAction::None;
        }
        self.last_ms = ctx.now_ms;
        // 数据快照: FM 句柄直读 (P3 current() 语义) + show* 开关现取
        // (config 适配器 Rc — sidecar 单线程面, 闭包 move 所需的 owned 形态)
        let fmdata = ctx.fm.current().fmdata.clone().map(Arc::new);
        let config: Option<std::rc::Rc<dyn ConfigProvider>> =
            Some(std::rc::Rc::new(ConfigFn(ctx.fm_field_config)));
        // PORT(panic 边界): generateLines 的保真 panic 点
        // 吞掉后冻结 (doit=false = Java "杀 run 线程" 形态)
        let state = &mut self.state;
        let ticked = catch_unwind(AssertUnwindSafe(|| {
            // FmUnpackedDataOverlay::tick 同式 (visible 门控 + generate_lines 直供)
            state.base.visible_now = state.visible;
            state.base.tick(move || {
                Some(generate_lines(fmdata.as_deref(), config.as_deref()))
            })
        }));
        if ticked.is_err() {
            vm_core::base::logger::error(
                "fm.list",
                "run 轮 panic 已吞 (畸形 FM 字段, 对位 Java 杀 run 线程), 本组件冻结",
            );
            self.state.base.stop(); // doit=false: 后续 tick 短路
            return SidecarAction::None;
        }
        // run() 双分支的 setVisible 落地 + adjustPosition 的 setSize 副作用
        let (w, h) = (self.state.base.width, self.state.base.height);
        let vis = self.state.base.window_visible;
        if (w, h) != self.last_size {
            self.last_size = (w, h);
            SidecarAction::SetVisibleResize(vis, w, h)
        } else {
            SidecarAction::SetVisible(vis)
        }
    }
}

// =====================================================================
// core.fm.thrust_chart — 推力-真空速曲线
// =====================================================================

/// 推力曲线复合组件 (包 [`DrawFrameSimpl`] + 1000ms tick 泵逻辑)。
/// 构造参数口径 (旧 spec 工厂同源): 三档 regular 字体 12/16/18,
/// 恒 900×500, 初始态 = initPreview 形态 (恒可见 + 空 FM 缓存)。
pub struct ThrustChartWidget {
    state: DrawFrameSimpl,
    /// 三档字体 (num12/text12 同档; text16/text18)
    fonts: (LoadedFont, LoadedFont, LoadedFont),
    canvas: PixCanvas,
    /// 1000ms 节流基准 (displayFmKey != 0 路径)
    last_ms: i64,
    /// 10s 退场等待起点 (Some = 已命中退出条件, 泵沉睡中)
    exit_wait_start: Option<i64>,
    /// run 循环已终止 (Close 动作后; reset_preview 复位)
    exited: bool,
}

fn f_thrust_chart(
    _props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let regular = fctx
        .fonts_dir
        .as_deref()
        .ok_or("fm.thrust_chart 需要 FactoryCtx.fonts_dir")?
        .join("sarasa-mono-sc-regular.ttf");
    let fonts = (
        LoadedFont::new(&regular, 12)?,
        LoadedFont::new(&regular, 16)?,
        LoadedFont::new(&regular, 18)?,
    );
    let mut state = DrawFrameSimpl::new();
    // fm 缓存初值 None: sidecar tick 每轮直读 current() 刷新
    // (旧工厂 initPreview(current) 的周期化等价, 首轮 tick 即到位)
    state.init_preview(None);
    let canvas = PixCanvas::new(900, 500)?;
    Ok(Box::new(ThrustChartWidget {
        state,
        fonts,
        canvas,
        last_ms: 0,
        exit_wait_start: None,
        exited: false,
    }))
}

impl HudWidget for ThrustChartWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, _env: &UpdateEnv) {
        // 数据面全走 sidecar tick (1000ms 自节流 + FM 直读)
    }

    fn reset_preview(&mut self) {
        // 回 preview 会话形态 + run 循环重生 (旧 DrawFrameSimplFeed::reset 同式)
        self.state.reset_preview();
        self.last_ms = 0;
        self.exit_wait_start = None;
        self.exited = false;
    }

    fn sidecar(&mut self) -> Option<&mut (dyn WidgetSidecar + 'static)> {
        let s: &mut dyn WidgetSidecar = self;
        Some(s)
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        if !self.canvas.clear(900, 500) {
            return;
        }
        let fonts = DfsFonts {
            num12: &self.fonts.0,
            text16: &self.fonts.1,
            text18: &self.fonts.2,
            text12: &self.fonts.0,
        };
        // PORT(panic 边界, 旧 spec render 闭包同契约): 畸形 FM 推力表的索引
        // panic 吞掉留空画布, 不毒化组件
        let state = &self.state;
        let canvas = &mut self.canvas;
        let r = catch_unwind(AssertUnwindSafe(|| state.draw(canvas, &fonts, aa)));
        if r.is_err() {
            vm_core::base::logger::error(
                "fm.thrust_chart",
                "paint panic 已吞 (畸形 FM 推力表), 本帧空画布",
            );
        }
        let frame = canvas.straight_frame();
        if !cv.composite_straight_frame_at(x, y, frame, 900, 500, aa) {
            vm_core::base::logger::warn("fm.thrust_chart", "伴画布尺寸与缓冲不符, 本帧丢弃");
        }
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        // setBounds 字面量恒 900×500 (Java 无 reinit 面)
        Dimension::new(900, 500)
    }
}

impl WidgetSidecar for ThrustChartWidget {
    /// 泵的单轮搬运 (旧 DrawFrameSimplFeed 同语义): 退场等待 → 1000ms 节流 →
    /// FM 缓存刷新 (FM_CHANGED reload 的周期化等价) → 自管可见性 →
    /// displayFmKey==0 的收腿退场判定。
    fn tick(&mut self, ctx: &mut SidecarCtx) -> SidecarAction {
        if ctx.game_mode_pulse {
            // Java init :514-528 单实例对位: initFmHandleCache + isPreview=false
            self.state.init(ctx.fm.current().fmdata.clone().map(Arc::new));
        }
        if ctx.toggle_pulse {
            self.state.toggle();
        }
        if let Some(d) = ctx.fm_changed.take() {
            self.state.reload_fm(Some(d));
        }
        if self.exited {
            return SidecarAction::None; // run 已终止 (Java dispose 后僵在 entry 里)
        }
        if let Some(start) = self.exit_wait_start {
            // sleepQuietly(10000) 等待期: 到点 break → dispose (close + zombie)
            if ctx.now_ms.saturating_sub(start) >= 10_000 {
                vm_core::base::logger::info("fm.thrust_chart", "Exiting run loop, disposing");
                self.exited = true;
                return SidecarAction::Close;
            }
            return SidecarAction::None;
        }
        // 配置了热键: sleepQuietly(1000) 节流
        if ctx.display_fm_key != 0 && ctx.now_ms.saturating_sub(self.last_ms) < 1000 {
            return SidecarAction::None;
        }
        self.last_ms = ctx.now_ms;
        // fm 句柄缓存刷新 (P3: current() 快照, 绝不查加载)
        self.state
            .reload_fm(ctx.fm.current().fmdata.clone().map(Arc::new));
        let should_show = self.state.should_show();
        // displayFmKey == 0: 收起落架/滑跑油门则退场 (frame None = 预览冻结判定)
        if ctx.display_fm_key == 0 {
            if let Some(frame) = ctx.frame {
                let gear = frame.var_value("gear").unwrap_or(0.0);
                let speedv = frame.var_value("speedv").unwrap_or(0.0);
                let throttle = frame.var_value("throttle").unwrap_or(0.0);
                if gear != 100.0 || (speedv > 10.0 && throttle > 0.0) {
                    self.exit_wait_start = Some(ctx.now_ms);
                }
            }
        }
        SidecarAction::SetVisible(should_show)
    }
}

// =====================================================================
// 注册表
// =====================================================================

const EMPTY_PROPS: &[PropSchema] = &[];

/// 工厂速记 (composite 恒 true — 黑盒包整窗 state)
const fn meta(
    type_name: &'static str,
    display_zh: &'static str,
    category: WidgetCategory,
    config_keys: &'static [&'static str],
    factory: super::registry::WidgetFactory,
) -> WidgetMeta {
    WidgetMeta {
        type_name,
        display_zh,
        category,
        composite: true,
        props_schema: EMPTY_PROPS,
        config_keys,
        data_shorts: &[],
        factory,
    }
}

/// W3C 注册表 (FM 黑盒; 数据面 = sidecar tick)
pub(super) const REGISTRY_ENTRIES: &[WidgetMeta] = &[
    meta(
        "core.fm.list",
        "FM拆包数据列表",
        WidgetCategory::List,
        &[
            "enableFMPrint",
            "displayFmKey",
            "fontName",
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
        ],
        f_fm_list,
    ),
    meta(
        "core.fm.thrust_chart",
        "推力-真空速曲线",
        WidgetCategory::Chart,
        // Java 键集为空 (registerWithStrategy 无 with_interest, 同款)
        &[],
        f_thrust_chart,
    ),
];
