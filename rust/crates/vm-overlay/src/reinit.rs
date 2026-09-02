//! WYSIWYG reinit 参数包 — CONFIG_CHANGED 后各 overlay 重建 state/字体/几何的输入。
//!
//! PORT(WYSIWYG 断链修复): Java 链 CONFIG_CHANGED → 防抖 → refreshPreviews(key) →
//! 各 overlay `reinitConfig()` (重读配置 + 重建 Font + 重算布局 + setBounds 改窗口
//! 尺寸)。Rust 侧配置树 !Send 不能进 win32 线程, 原实现注册面是 spawn 时一次性
//! 快照 (`OverlayInputs`), CONFIG_CHANGED 后无刷新通道 — 快照冻结, reinit 无人调,
//! 窗口尺寸注册期定死三层断链。
//!
//! 修复形态 (五色直送/AA 开关同款 "配置 !Send, 值随命令进 win32 线程" 模式):
//! 主线程 CONFIG_CHANGED 时即时读配置重建本参数包 → `UiCommand::ReinitOverlays`
//! 送 win32 线程的线程局部仓 (`Rc<RefCell<ReinitParams>>`) → 各 spec 工厂的
//! reinit 闭包 (OverlaySpec.reinit) 读取最新值重建 state, 返回新 (w,h) 由 host
//! resize_entry 落窗口 — 对位 Java reinitConfig 的 setBounds 副作用。

use std::sync::Arc;

use vm_core::config::config_api::HudSettingsSnapshot;
use vm_core::ui_support::row_def::RowDef;

/// reinit 参数包 (纯值 Send; 各字段来源 = OverlayInputs 同源配置键)。
/// PORT(取舍备案): 不整包重送 `OverlayInputs` — 颜色/AA 有专命令
/// (SetGlobalColors/SetAa), 本包只收 reinit 实际消费面。
///
/// PORT(批 C 边框族备案, 不修项写明):
/// - `enableAxisEdge` / `enablegearAndFlapsEdge` / `enableAttitudeIndicatorEdge`:
///   Java setShadeWidth 是 WebLaF 窗口装饰边距层, host 无边框层 — 前两者经
///   reinit 链进 state 几何 (gear 的 sw·2 外扩计入 total 尺寸; axis 的 sw 仅
///   state 字段, spec 尺寸钉内容区, 见 overlays_field2.rs "PORT(边框不承载)"),
///   后者 (attitude) 同为装饰层不承载。
/// - `flightInfoEdge` (Java FieldOverlay.edgeKey): reinitConfig 里只进
///   setShadeWidth 装饰层, **不进** setBounds 宽高 (width = getTotalWidth()),
///   Rust 无装饰层 → 无对应行为, 备案不修。
/// - `attitudeIndicatorUseNumColor`: Java :253 读键写 transParentWhite — 但该
///   字段无读取者 (键被读、值写进死字段, 无可观测行为) — 不复刻
///   (gauge_attitude.rs "PORT(精确定性)" 注同源)。
#[derive(Debug, Clone, PartialEq)]
pub struct ReinitParams {
    pub dpi_scale: f64,
    /// 引擎控制: 字号增量 (getOverlaySettings("引擎控制").getFontSizeAdd)
    pub font_add_engine: i32,
    /// 引擎控制: 7 仪表 disable 开关 (ENGINE_DISABLE_KEYS 序)
    pub engine_disables: [bool; 7],
    /// 引擎控制/Service 轮询间隔 (loadRefreshInterval 读 dataPollIntervalMs)
    pub service_loop_interval_ms: i64,
    /// 动力信息: 字号增量 + 列数
    pub font_add_power: i32,
    pub power_columns: i32,
    /// 飞行信息: 字号增量 + 列数
    pub font_add_flight: i32,
    pub flight_columns: i32,
    /// 起落襟翼: 字号增量 + 边缘开关 (enablegearAndFlapsEdge)
    pub font_add_gear: i32,
    pub gear_show_edge: bool,
    /// 操纵面: 字号增量 + 边缘开关 (enableAxisEdge)
    pub font_add_axis: i32,
    pub axis_show_edge: bool,
    /// FM拆包数据: 字号增量 (getOverlaySettings("FM拆包数据").getFontSizeAdd;
    /// cfg 该组无字号滑条, 恒走 OverlaySettings 默认 0 — setupFont 的 14+add 面)
    pub font_add_fm: i32,
    /// 地平仪: 宽高 (attitudeIndicatorWidth/Height, 工厂内再 DPI 缩放) +
    /// 喂入节流 (attitudeIndicatorFreqMs) + 开关族
    pub attitude_width: i32,
    pub attitude_height: i32,
    pub attitude_freq_ms: i64,
    pub attitude_show_direction: bool,
    pub attitude_show_aoa_limits: bool,
    /// MiniHUD 全量设置快照 (reinit_config 的 S: HUDSettings 实参)
    pub hud: HudSettingsSnapshot,
    /// W-D cfg 驱动行定义 (主线程从 ui_layout.cfg 编译, 行开关过滤后随包进 win32)
    pub flight_rows: Arc<Vec<RowDef>>,
    pub power_rows: Arc<Vec<RowDef>>,
}

impl Default for ReinitParams {
    /// 缺省 = Java 各 reinitConfig 的无配置回退值 (地平仪 150×300/40ms/
    /// direction false / AoA 极限 true, AttitudeOverlay.java:232-248; 其余组
    /// fontadd=0/单列/边框关)
    fn default() -> Self {
        ReinitParams {
            dpi_scale: 1.0,
            font_add_engine: 0,
            engine_disables: [false; 7],
            service_loop_interval_ms: 50,
            font_add_power: 0,
            power_columns: 1,
            font_add_flight: 0,
            flight_columns: 1,
            font_add_gear: 0,
            gear_show_edge: false,
            font_add_axis: 0,
            axis_show_edge: false,
            font_add_fm: 0,
            attitude_width: 150,
            attitude_height: 300,
            attitude_freq_ms: 40,
            attitude_show_direction: false,
            attitude_show_aoa_limits: true,
            hud: HudSettingsSnapshot::default(),
            flight_rows: Arc::new(Vec::new()),
            power_rows: Arc::new(Vec::new()),
        }
    }
}
