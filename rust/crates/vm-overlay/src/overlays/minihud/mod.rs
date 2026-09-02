//! minihud: MiniHUDOverlay 主体 C 类语义复刻 (src/ui/overlay/MiniHUDOverlay.java)
//! 波16 拆分为目录模块 (原单文件 ~1500 行四职责):
//! - ctx: MinimalHudContext + MiniHudFonts 上下文 (配置快照)
//! - comp: MiniHudComponent/CompCell/Inner 装配层 + 组件装配方法
//! - 本文件: MiniHudOverlay 编排器 + minihud_overlay_spec 工厂
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`MinimalHudContext`] | src/ui/overlay/MinimalHUDContext.java | 不可变配置快照: 全部派生量 (字号/线宽/罗盘直径/rightDraw) 从 crossScale×dpiScale 级联; 字体 = 三份 BOLD 字号档 |
//! | [`MiniHudComponent`]+[`CompCell`] | ui/component/HUDComponent.java + AbstractHUDComponent.java | 组件接口的组装层 seam: getPreferredSize/isVisible/setVisible/onDataUpdate; 异构组件装箱为枚举 |
//! | [`MiniHudOverlay`] | src/ui/overlay/MiniHUDOverlay.java | 编排器: 组件创建 → 风格/模板注入 → DAG 布局 (minihud_layout::build_mihud_layout) → 渲染循环 (doLayout+render+drawBlinkX) |
//! | [`minihud_overlay_spec`] | Controller.java:671 registerWithPreview("crosshairSwitch") | OverlayHost 挂载: render 闭包持共享句柄, 数据侧经 [`MiniHudHandle`] 外部喂入 |
//!
//! 渲染循环 (Java paintComponent L241-256):
//! `engine.do_layout()` (惰性拓扑 + 锚点求解) → `engine.render(cb)` (可见节点按拓扑序
//! 逐个 `component.draw(g,x,y)`, debug 开启时紧跟 1px 调试框) → `draw_blink_x`
//! (致命警告 X, 压在 HUD 内容之上)。
//!
//! 零分配纪律 (手册 §11.4): draw 路径不 new — 字体/颜色经 [`MiniHudFonts`] Rc 共享,
//! 组件句柄 [`CompCell`] 克隆仅是引用计数; Java 侧对应 "严禁在 draw() 循环中 new
//! Color/Font" (缓存复用)。
//!
//! 映射裁决:
//! - Java `List<HUDComponent> components` (initComponentsLayout 添加序) 与布局引擎
//!   节点图**共享同一批组件对象** → [`CompCell`](Rc<RefCell>) 双持: overlay 具名字段
//!   (风格/模板/可见性写入口) + engine 节点负载 (渲染读出口), Java 引用共享语义落地。
//! - `Math.round` 双语义 (§2.3): Math.round(float)→int 与 Math.round(double)→long→
//!   (int) 窄化 (§2.2 双转) 分别落 java_round_f32/[`java_round_long_narrowed`]。
//! - `String.format` 的 %N.Mf / %Ns / %Nd → vm_core::base::format 收敛点
//!   (`java_f`/`pad_width`, 重构波13 收割本地副本)。
//! - Application 静态色 (colorNum/colorShadeShape) → gauges_bars 常量 (同源)。
//! - Application.dpiScale (LIFETIMES §1.2 Env 只读) → 参数注入 (调用方持 Env)。
//! - Font(family, BOLD, size) 的家族名 → Rust 按字体文件路径加载 (font.rs 只吃
//!   文件); MonoNumFont 的 cfg 缺省 "Sarasa Mono SC" 映射到随包
//!   sarasa-mono-sc-bold.ttf, 由调用方解析路径。
//! - crosshairImageScaled 纹理链 (MinimalHUDContext.java:161-178) 不迁移 —
//!   gauge_crosshair.rs 头部裁决: 软件矢量路径是唯一视觉语义。
//! - Java 死字段 (hudCheckMili/realSpdPitch/firstDraw/throttley/throttleColor/
//!   inAction/disableAttitude) 保真保留 (§2.10 + hud_layout_node ignoreBounds
//!   先例: write-only 状态不删), 各带 PORT 注。

mod comp;
mod ctx;

pub use comp::{CompCell, MiniHudComponent, MiniHudComponentInner};
pub use ctx::{MiniHudFonts, MinimalHudContext};

use crate::render::primitives;
use vm_core::base::format::pad_width;
use crate::render::palette::{aa, colors};
use crate::overlays::rows::{MANEUVER_FULL_SCALE, MANEUVER_TICK_STEPS, TickScale};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use vm_core::fm::data::FmData;
use vm_core::config::config_api::HUDSettings;
use vm_core::base::event::event_payload::EventPayload;
use vm_core::derived::hud_calculator::{self, HudColors};
use vm_core::telemetry::parser::{Indicators, State};
use vm_core::derived::hud_data::HUDData;
use crate::layout::hud_layout_node::HUDLayoutNodeExt;
use vm_core::formula::registry::FormulaView;

use crate::platform::host::{OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::overlays::spec_common::keyed_spec;
use crate::layout::minihud_layout::{AutoSizingPlan, BuiltMiniHudLayout, ModernHUDLayoutEngine};
use crate::render::canvas::PixCanvas;
use crate::overlays::warning::WarningBlinkHost;

// ---------------------------------------------------------------------------
// Java Math / printf 复刻 (§2.3/§2.2; 取整族收敛 vm_core::base::format)
// ---------------------------------------------------------------------------

/// Java `(int) Math.round(double)`: round 返回 long, (int) 窄化取低 32 位
/// (§2.2 双转; 值域内与饱和无差, 防御性对齐 Java 溢出行为)
fn java_round_long_narrowed(x: f64) -> i32 {
    let l = (x + 0.5).floor() as i64;
    (l as u32) as i32
}

/// Java `String.format("%Nd", v)` = pad_width(十进制) 组合: 右对齐补空格
/// (v 为 i32, 无舍入; 算法本体在 vm_core::base::format::pad_width)
fn fmt_d(v: i32, width: usize) -> String {
    pad_width(v.to_string(), width, false)
}

// ---------------------------------------------------------------------------
// MiniHUDOverlay (src/ui/overlay/MiniHUDOverlay.java 主体)
// ---------------------------------------------------------------------------

/// overlay 的共享数据句柄 (host render 闭包与数据喂入方各持一份;
/// 单线程 RefCell — host 是主循环单线程独占, 上层 Controller 须同线程喂入)
pub type MiniHudHandle = Rc<RefCell<MiniHudOverlay>>;

/// MinimalHUD overlay for displaying compact flight information.
/// Being migrated to event-driven architecture. (Java 类 javadoc 原文)
pub struct MiniHudOverlay {
    ctx: MinimalHudContext,
    /// 字体快照 (与 ctx.fonts 同源; reinit_config 重建 ctx 时同步换)
    fonts: Rc<MiniHudFonts>,
    /// 字体文件路径 (reinit_config 重建字体用)
    font_path: PathBuf,
    /// Application.dpiScale 参数注入 (LIFETIMES Env 只读快照)
    dpi_scale: f64,
    /// Java service 字段的在场语义 (null = 预览模式; 遥测数据经参数喂入)
    service_present: bool,

    // Reactive Components List (initComponentsLayout 添加序 = onDataUpdate 分发序)
    components: Vec<CompCell>,

    // 具名组件句柄 (Java 字段; 与布局节点图共享同一对象)
    crosshair_gauge: CompCell,
    flap_angle_bar: CompCell,
    compass_gauge: CompCell,
    attitude_indicator_gauge: CompCell,
    speed_ratio_bar: CompCell,
    /// hudRows (5 行; 行链 spec 按 rows.len() 截断)
    hud_rows: Vec<CompCell>,
    throttle_bar: CompCell,

    // 0. Aux Overlays — warningOverlay 组合于 WarningBlinkHost (drawBlinkX 链)
    warning: WarningBlinkHost,

    // --- Modern Layout Engine Integration ---
    layout: BuiltMiniHudLayout<CompCell>,

    // Java 遗留/只写字段 (模块头映射裁决; §2.10 保真保留)
    /// refreshTemplates 的预览行串 (lines[5] 未用, Java 数组长 6 原样)
    lines: [String; 6],
    rel_energy: String,
    line_aoa: String,
    /// Java public int throttley (refreshTemplates 写 100; 无读者)
    throttley: i32,
    /// refreshTemplates 写 10 → init 钳 ctx.rightDraw; preview row0.update 入参
    aoa_y: i32,
    /// Java public Color throttleColor (写无读; Application.colorShadeShape)
    throttle_color: [u8; 4],
    aoa_color: [u8; 4],
    aoa_bar_color: [u8; 4],
    /// Java public boolean inAction (恒 false; row2 预览 update 入参)
    in_action: bool,
    /// Java private boolean disableAttitude (恒 false; 姿态仪可见性入参)
    disable_attitude: bool,
    /// Java private double realSpdPitch (死字段 — 全库无读写, 声明保真保留)
    #[allow(dead_code)] // PORT: Java MiniHUDOverlay.java:286 同名死字段
    real_spd_pitch: f64,
    /// Java private boolean firstDraw (reinitConfig 写 true; 无读者)
    first_draw: bool,
    /// Java public long hudCheckMili (死字段 — 全库无读写, 声明保真保留)
    #[allow(dead_code)] // PORT: Java MiniHUDOverlay.java:283 同名死字段
    hud_check_mili: i64,
    /// update_legacy_components 更新, update_components 预览路径读
    maneuver_index: f64,
    maneuver_index_len: i32,
    /// 各档刻度距离 (原 len10..len50 五连字段收敛, 档位表在 rows.rs)
    tick_scale: TickScale,

    // Java public boolean warnRH / warnVne (updateFromEvent 写; 外层消费)
    pub warn_rh: bool,
    pub warn_vne: bool,

    // Throttling for refresh rate (Java:412-415)
    refresh_interval: i64,
    last_refresh_time: i64,
}

impl MiniHudOverlay {
    /// Java init(Controller c, Service s, HUDSettings settings) (L217-281)。
    /// `service_loop_interval_ms` = controller.serviceLoopIntervalMs (blinkTicks/
    /// refreshInterval 同源); `service_present` = (s != null); Rust 侧 service /
    /// controller 不入结构 — 遥测经 [`on_flight_data`] 参数喂入 (单线程 host 模型,
    /// 模块头映射裁决)。
    pub fn init<S: HUDSettings>(
        service_present: bool,
        service_loop_interval_ms: i64,
        settings: &S,
        dpi_scale: f64,
        font_path: &Path,
    ) -> Result<Self, String> {
        vm_core::base::logger::info("MinimalHUD", "init called");
        let ctx = MinimalHudContext::create(settings, dpi_scale, font_path)?;
        let fonts = Rc::new(ctx.fonts.clone());
        // Java initComponentsLayout 之前各组件字段为 null → 首轮 reinitConfig 的
        // applyStyle/updateComponents 对组件全空转 (initModernLayout 空表早退)。
        // Rust 无 null: 占位组件即刻可查 (空引擎不渲染), initComponentsLayout
        // 建齐真身后整体替换 — 调用序列与 Java 逐行对应。
        let ng = Self::named_gauge_cells(&fonts, ctx.round_compass);
        let empty_engine = ModernHUDLayoutEngine::new(ctx.width, ctx.height);
        let mut overlay = MiniHudOverlay {
            crosshair_gauge: ng.crosshair_gauge,
            flap_angle_bar: ng.flap_angle_bar,
            compass_gauge: ng.compass_gauge,
            attitude_indicator_gauge: ng.attitude_indicator_gauge,
            speed_ratio_bar: ng.speed_ratio_bar,
            throttle_bar: ng.throttle_bar,
            fonts,
            font_path: font_path.to_path_buf(),
            dpi_scale,
            service_present,
            components: Vec::new(),
            hud_rows: Vec::new(),
            warning: WarningBlinkHost::new(service_loop_interval_ms),
            layout: BuiltMiniHudLayout { engine: empty_engine, sizing: None },
            lines: std::array::from_fn(|_| String::new()),
            rel_energy: String::new(),
            line_aoa: String::new(),
            throttley: 0,
            aoa_y: 0,
            throttle_color: colors().shade_shape,
            aoa_color: colors().num,
            aoa_bar_color: colors().num,
            in_action: false,
            disable_attitude: false,
            real_spd_pitch: 0.0,
            first_draw: true,
            hud_check_mili: 0,
            maneuver_index: 0.0,
            maneuver_index_len: 0,
            tick_scale: TickScale::default(),
            warn_rh: false,
            warn_vne: false,
            refresh_interval: service_loop_interval_ms,
            last_refresh_time: 0,
            ctx,
        };

        overlay.reinit_config(settings)?;

        if overlay.aoa_y > overlay.ctx.right_draw {
            overlay.aoa_y = overlay.ctx.right_draw;
        }
        overlay.aoa_color = colors().num;
        overlay.aoa_bar_color = colors().num;

        overlay.init_components_layout(settings);

        // PORT: Java 读 service 字段 — 游戏模式 S1.start() 先于 overlay 激活
        // (Controller.java:633-641), sState 可能已轮询出值, throttle 分支可吃到
        // 真值; 组装层此阶段无遥测口可传 → None, throttle 闪 0, 由下一放行的
        // on_flight_data (≤refreshInterval) 覆盖, 影响 ≤1 帧
        overlay.update_components(settings, None);

        Ok(overlay)
    }

    /// Java reinitConfig() (L127-159) — ctx 快照重建 + 模板 + 风格 + 布局引擎重建。
    /// PORT: setBounds (L143-146) 的窗口几何副作用归 OverlayHost (spec 尺寸取
    /// applyAutoSizing 计划); Java 先 setBounds 再被 applyAutoSizing 的
    /// window.setSize 覆盖, 净效果 = 内容包围盒 + 2×LAYOUT_PADDING。
    pub fn reinit_config<S: HUDSettings>(&mut self, settings: &S) -> Result<(), String> {
        vm_core::base::logger::info("MinimalHUD", "reinitConfig called");

        // Create Immutable Context
        self.ctx = MinimalHudContext::create(settings, self.dpi_scale, &self.font_path)?;
        self.fonts = Rc::new(self.ctx.fonts.clone());

        // 1. Refresh mock data and templates (WYSIWYG support)
        self.refresh_templates(settings);

        // Apply dimensions (Initial guess, will be refined by dynamic layout)
        // (Java 注释原文; setBounds → 宿主, 见方法头 PORT 注)

        // 2. Sync Component State (Style & Visibility) BEFORE Layout
        // This ensures getContentBounds() sees the correct visible components
        //
        self.apply_style_to_components(settings);
        // PORT: Java 此处 updateComponents() 读 service 字段 — 游戏模式 WYSIWYG
        // reinit 时 sState 可非 null (throttle 吃真值); Rust 恒传 None → 油门条
        // 闪 0, 下一放行 on_flight_data (≤refreshInterval) 修复, 影响 ≤1 帧
        self.update_components(settings, None);

        // 3. Setup Layout Engine & Dynamic Sizing
        self.init_modern_layout(settings);

        self.first_draw = true;
        // repaint() → 宿主 render_tick 标脏 (host 脏检查逐字节, 无需显式)
        Ok(())
    }

    /// Java updateComponents() (L309-402)。
    /// `service` = Java service 字段处的遥测读取口 (throttle 分支);
    /// 行 0/1 预览串分支按 Java 语义读 **service 字段在场性** (init 决定),
    /// 不随本参数摆动 (WYSIWYG 游戏内 reinit 亦不推预览串)。
    fn update_components<S: HUDSettings>(
        &mut self,
        settings: &S,
        service: Option<&dyn FormulaView>,
    ) {
        self.update_component_visibility(settings);

        if self.hud_rows.len() >= 5 {
            self.update_row_visibility(settings);
            self.update_row_values();
        }

        // PORT: Java `service != null && service.sState != null` — sState 空判
        // 折入 TelemetrySource 实现域 (Service 批次); getThrottle 返回 double
        // 而 Java 读 int 字段 sState.throttle → as i32 (JLS 5.1.3 同义)
        let mut throttle_value = 0;
        if let Some(s) = service {
            throttle_value = s.var_value("throttle").unwrap_or(0.0) as i32;
        }
        self.push_throttle(throttle_value);
    }

    /// updateComponents 仪表可见性段 (L311-323)
    fn update_component_visibility<S: HUDSettings>(&mut self, settings: &S) {
        let text_visible = settings.draw_hud_text();

        let enable_flap_bar = settings.enable_flap_angle_bar();
        self.flap_angle_bar.set_visible(text_visible && enable_flap_bar);
        let show_attitude = settings.show_attitude_gauge();
        self.compass_gauge.set_visible(text_visible && !show_attitude);
        self.attitude_indicator_gauge
            .set_visible(text_visible && show_attitude && !self.disable_attitude);
        // Dynamic position based on current Width/CrossX —
        // Position handled by ModernHUDLayoutEngine (Java 注释原文; ctx 空块不复刻)
        self.crosshair_gauge.set_visible(settings.is_display_crosshair());
        let show_speed = settings.show_speed_bar();
        self.throttle_bar.set_visible(text_visible && !show_speed);
        self.speed_ratio_bar.set_visible(text_visible && show_speed);
    }

    /// updateComponents 行可见性段 (L325-360; 调用方保证 hud_rows.len()>=5)
    fn update_row_visibility<S: HUDSettings>(&mut self, settings: &S) {
        // Java: master = drawHudText() 二次读取 (与 text_visible 同源, 保真保留)
        let master = settings.draw_hud_text();

        // 组件级独立可见性控制
        // Row 0: Speed + AoA — 两个独立组件
        let row0_speed = master && settings.show_hud_speed();
        let row0_aoa = master && settings.show_hud_aoa();
        self.hud_rows[0].set_visible(row0_speed || row0_aoa);
        self.hud_rows[0].map_inner(|inner| {
            if let MiniHudComponentInner::Row0(r) = inner {
                r.set_show_speed(row0_speed);
                r.set_show_aoa(row0_aoa);
            }
        });

        // Row 1: Altitude + Energy — 两个独立组件
        let row1_alt = master && settings.show_hud_altitude();
        let row1_energy = master && settings.show_hud_energy();
        self.hud_rows[1].set_visible(row1_alt || row1_energy);
        self.hud_rows[1].map_inner(|inner| {
            if let MiniHudComponentInner::Row1(r) = inner {
                r.set_show_altitude(row1_alt);
                r.set_show_energy(row1_energy);
            }
        });

        // Row 2: 襟翼/可变翼 + 减速板 + 起落架 — 三个独立组件
        let row2_flaps = master && settings.show_hud_flaps();
        let row2_brk = master && settings.show_hud_airbrake();
        let row2_gear = master && settings.show_hud_gear();
        self.hud_rows[2].set_visible(row2_flaps || row2_brk || row2_gear);
        self.hud_rows[2].map_inner(|inner| {
            if let MiniHudComponentInner::Row2(r) = inner {
                r.set_show_flaps(row2_flaps);
                r.set_show_airbrake(row2_brk);
                r.set_show_gear(row2_gear);
            }
        });

        // Row 3: 单组件（爬升率）
        self.hud_rows[3].set_visible(master && settings.show_hud_sep());

        // Row 4: G-force + ManeuverBar — 两个独立组件
        let row4_g_load = master && settings.show_hud_g_load();
        let row4_bar = master && settings.show_hud_maneuver_bar();
        self.hud_rows[4].set_visible(row4_g_load || row4_bar);
        self.hud_rows[4].map_inner(|inner| {
            if let MiniHudComponentInner::Row4(r) = inner {
                r.set_show_g_load(row4_g_load);
                r.set_show_maneuver_bar(row4_bar);
            }
        });
    }

    /// updateComponents 行值段 (L362-397; 调用方保证 hud_rows.len()>=5)。
    /// 行 0/1 预览串仅 service 缺席 (init 的 service_present) 时推 — 游戏模式由
    /// onDataUpdate 事件路径覆写
    fn update_row_values(&mut self) {
        // Row 0, 1: Only update in preview mode (service == null)
        // In game mode, they are updated via onDataUpdate() from FlightDataEvent
        // (Java 注释原文; service==null 即 init 的 service_present=false)
        if !self.service_present {
            let (l0, laoa, aoa_y, a_col, ab_col) = (
                self.lines[0].clone(),
                self.line_aoa.clone(),
                self.aoa_y,
                self.aoa_color,
                self.aoa_bar_color,
            );
            self.hud_rows[0].map_inner(|inner| {
                if let MiniHudComponentInner::Row0(r) = inner {
                    r.update(&l0, false, &laoa, aoa_y, a_col, ab_col);
                }
            });
            let (l1, lrel) = (self.lines[1].clone(), self.rel_energy.clone());
            self.hud_rows[1].map_inner(|inner| {
                if let MiniHudComponentInner::Row1(r) = inner {
                    // 能量颜色已统一使用 Application.colorNum，不再需要传入颜色参数
                    //
                    r.update(&l1, false, &lrel);
                }
            });
        }

        // Row 2: Standard (Flaps/Gear)
        let l2 = self.lines[2].clone();
        let in_action = self.in_action;
        self.hud_rows[2].map_inner(|inner| {
            if let MiniHudComponentInner::Row2(r) = inner {
                r.update(&l2, in_action);
            }
        });
        // Row 3: Standard (SEP)
        let l3 = self.lines[3].clone();
        self.hud_rows[3].map_inner(|inner| {
            if let MiniHudComponentInner::Row3(r) = inner {
                r.update(&l3, false);
            }
        });
        // Row 4: Maneuver (G)
        let l4 = self.lines[4].clone();
        let (mi, l, ticks) = (self.maneuver_index, self.maneuver_index_len, self.tick_scale);
        self.hud_rows[4].map_inner(|inner| {
            if let MiniHudComponentInner::Row4(r) = inner {
                r.update(&l4, false, mi, l, ticks);
            }
        });
    }

    /// throttleBar 推值 (updateComponents 的 service 口与 updateFromEvent 的
    /// HUDData 口双路径共用; C27 双份收敛)
    fn push_throttle(&mut self, v: i32) {
        self.throttle_bar.map_inner(|inner| {
            if let MiniHudComponentInner::ThrottleBar(t) = inner {
                t.update(v, &fmt_d(v, 3));
            }
        });
    }

    // --- Event-Driven Update ---

    /// Java onFlightData(FlightDataEvent) (L418-431)。
    /// 返回 false = 节流跳过 (Java return); true = 已进入 updateFromEvent。
    /// `now_ms` = System.currentTimeMillis (宿主时钟注入, 可测)。
    /// W-B 事件瘦身后直参: State/Indicators/payload 由调用方从共享 guard 借引用传入。
    #[allow(clippy::too_many_arguments)]
    pub fn on_flight_data<S: HUDSettings>(
        &mut self,
        now_ms: i64,
        state: Option<&State>,
        indic: Option<&Indicators>,
        payload: &EventPayload,
        service: Option<&dyn FormulaView>,
        fmdata: Option<&FmData>,
        settings: &S,
        colors: &HudColors,
    ) -> bool {
        // Throttling prevents EDT task accumulation when events arrive faster
        // than processing
        if now_ms - self.last_refresh_time < self.refresh_interval {
            return false; // Skip this update, too soon
        }
        self.last_refresh_time = now_ms;

        self.update_from_event(state, indic, payload, service, fmdata, settings, colors);
        // root.repaint() → 宿主 render_tick (脏检查逐字节, 无需显式标脏)
        true
    }

    /// Java updateFromEvent(FlightDataEvent) (L433-468)
    #[allow(clippy::too_many_arguments)]
    fn update_from_event<S: HUDSettings>(
        &mut self,
        state: Option<&State>,
        indic: Option<&Indicators>,
        payload: &EventPayload,
        service: Option<&dyn FormulaView>,
        fmdata: Option<&FmData>,
        settings: &S,
        colors: &HudColors,
    ) {
        // (Java 的 FMManager.current().blkx 快照语义由调用方以 blkx=None 表达 —
        // 非 READY 句柄降级)
        let data = hud_calculator::calculate(state, indic, payload, service, fmdata, settings, colors);

        // Dispatch to Reactive Components
        for comp in &self.components {
            comp.0.borrow_mut().on_data_update(&data);
        }

        // Update Legacy Components (Bridge) & Global State
        self.warn_vne = data.warn_vne;
        self.warn_rh = data.warn_altitude;
        // blinkX = event.getPayload().fatalWarn (Java:458)
        self.warning.set_blink_x(payload.fatal_warn);

        if self.hud_rows.len() >= 5 {
            // Let's call a legacy bridge method explicitly
            self.update_legacy_components(&data);
        }

        self.push_throttle(data.throttle);
    }

    /// Java updateLegacyComponents(HUDData) (L470-496)
    fn update_legacy_components(&mut self, data: &HUDData) {
        // Row 0, 1, 2 are refactored (Akb, Energy, Mechanization). They use
        // onDataUpdate.
        // Row 3: SEP
        let sep = data.sep_str.clone();
        self.hud_rows[3].map_inner(|inner| {
            if let MiniHudComponentInner::Row3(r) = inner {
                r.update(&sep, false);
            }
        });
        // Row 4: Maneuver
        // ManeuverRow update signature is complex.
        let (ms, mi) = (data.maneuver_state_str.clone(), data.maneuver_index);
        let (l, ticks) = (self.maneuver_index_len, self.tick_scale);
        self.hud_rows[4].map_inner(|inner| {
            if let MiniHudComponentInner::Row4(r) = inner {
                r.update(&ms, false, mi, l, ticks);
            }
        });
        // Note: maneuverIndexLen variables are member fields of MinimalHUD
        // calculated in legacy loop.
        let right_draw = self.ctx.right_draw;
        // PORT: (int) Math.round(double) — round→long→(int) 窄化 (§2.2 双转);
        // 求值序 (index / 0.5) * rightDraw 与 Java 左结合一致; 各档刻度走
        // 档位表 (N=0.5 档字面 0.5/0.5 由表驱动消解)
        self.maneuver_index_len =
            java_round_long_narrowed(data.maneuver_index / MANEUVER_FULL_SCALE * right_draw as f64);
        self.tick_scale = TickScale {
            ticks: MANEUVER_TICK_STEPS
                .map(|step| java_round_long_narrowed(step / MANEUVER_FULL_SCALE * right_draw as f64)),
        };
    }

    /// Java paintComponent 主体 (L241-256): doLayout + render + drawBlinkX。
    /// aa = graphAASetting (生产恒 ON; false 供对拍)。
    pub fn draw(&mut self, cv: &mut PixCanvas, aa: bool) {
        // (render2d 口径)
        {
            self.layout.engine.do_layout();
            let engine = &self.layout.engine;
            engine.render(|node, x, y, dbg| {
                // dbg=None: component.draw(g, x, y); Some(color): drawDebug 的
                // 1px 线框 (ModernHUDLayoutEngine.java:187-189 drawRect(x,y,w,h))
                match dbg {
                    None => {
                        let comp = node.borrow().component.0.clone();
                        comp.borrow_mut().draw(cv, x, y, aa);
                    }
                    Some(color) => {
                        let r = node.get_pixel_rect();
                        primitives::ring1px(cv, x, y, r.width, r.height, color);
                    }
                }
            });
        }
        // drawBlinkX(g2d) — X 只盖 ctx.width × ctx.height (crosshair 双宽窗口同,
        // warning_overlay.rs 头注保真)
        let (w, h) = (self.ctx.width, self.ctx.height);
        self.warning.draw_blink_x(cv, w, h, aa);
    }

    /// 自动尺寸计划 (initModernLayout 尾部 applyAutoSizing 的窗口尺寸来源;
    /// None = Java components 空裸 return 分支, 宿主保持初始尺寸)
    pub fn sizing(&self) -> Option<AutoSizingPlan> {
        self.layout.sizing
    }

    pub fn ctx(&self) -> &MinimalHudContext {
        &self.ctx
    }
}

// ---------------------------------------------------------------------------
// OverlayHost 挂载 (Controller.java:671 registerWithPreview("crosshairSwitch"))
// ---------------------------------------------------------------------------

/// MiniHUD 的 OverlayHost 注册件: 返回 (共享句柄, spec)。
/// render 闭包持句柄克隆画帧; 数据侧 (Controller/Service 批次) 持同一句柄调
/// [`MiniHudOverlay::on_flight_data`] — host 现仅 render 通道 (overlays_field1
/// 备案), 数据钩子以共享句柄承载, 不扩 host 接口。
///
/// spec 尺寸 = applyAutoSizing 计划 (Java: setBounds 初值被 applyAutoSizing 的
/// window.setSize 覆盖, 净效果 = 内容包围盒 + 2×LAYOUT_PADDING)。
/// PORT(WYSIWYG 收口, 原"创建时快照冻结"备案): reinit 闭包随 [`ReinitParams`] 仓
/// 走 reinit_config (ctx/模板/风格/布局引擎全量重建, Java L127-159), 新 sizing()
/// 计划经返回值交 host resize_entry — 对位 Java reinitConfig→applyAutoSizing 的
/// window.setSize 副作用, 窗口不再冻结在创建尺寸。
/// `service_loop_interval_ms` / `service_present` 语义见 [`MiniHudOverlay::init`]。
pub fn minihud_overlay_spec<S: HUDSettings>(
    service_present: bool,
    service_loop_interval_ms: i64,
    settings: &S,
    dpi_scale: f64,
    font_path: &Path,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(MiniHudHandle, OverlaySpec), String> {
    let overlay = MiniHudOverlay::init(
        service_present,
        service_loop_interval_ms,
        settings,
        dpi_scale,
        font_path,
    )?;
    let (w, h) = match overlay.sizing() {
        Some(p) => (p.new_width, p.new_height),
        // Java 空 components 裸 return: 窗口保持 setBounds 初值 (5 行恒在, 不可达)
        None => (overlay.ctx().width, overlay.ctx().height),
    };
    let handle: MiniHudHandle = Rc::new(RefCell::new(overlay));
    let render_handle = Rc::clone(&handle);
    // reinit 闭包: reinit_config(最新 hud 快照) → 新 sizing 计划 (setBounds 面);
    // 重建失败 (字体文件缺失等) 留痕并保持旧尺寸
    let reinit_handle = Rc::clone(&handle);
    let reinit_params = Rc::clone(params);
    let reinit: ReinitFn = Box::new(move || {
        let hud = reinit_params.borrow().hud.clone();
        let mut o = reinit_handle.borrow_mut();
        if let Err(e) = o.reinit_config(&hud) {
            vm_core::base::logger::error("MinimalHUD", &format!("reinit_config 失败: {}", e));
            return None;
        }
        let (w, h) = match o.sizing() {
            Some(p) => (p.new_width, p.new_height),
            None => (o.ctx().width, o.ctx().height),
        };
        Some((w, h))
    });
    Ok((
        handle,
        // Java LinkedHashMap 键 = configKey (Controller.java:671)
        keyed_spec(
            "crosshairSwitch",
            w,
            h,
            Box::new(move |cv: &mut PixCanvas| {
                // aa = 运行时仓 (cfg AAEnable 可关)
                render_handle.borrow_mut().draw(cv, aa());
            }),
            Some(reinit),
        ),
    ))
}

#[cfg(test)]
mod tests;
