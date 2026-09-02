//! WYSIWYG reinit 参数包 — CONFIG_CHANGED 后各 overlay 重建 state/字体/几何的输入。
//!
//! PORT(WYSIWYG 断链修复): Java 链 CONFIG_CHANGED → 防抖 → refreshPreviews(key) →
//! 各 overlay `reinitConfig()` (重读配置 + 重建 Font + 重算布局 + setBounds 改窗口
//! 尺寸)。Rust 侧配置树 !Send 不能进渲染线程, 原实现注册面是 spawn 时一次性
//! 快照 (`OverlayInputs`), CONFIG_CHANGED 后无刷新通道 — 快照冻结, reinit 无人调,
//! 窗口尺寸注册期定死三层断链。
//!
//! 修复形态 (五色直送/AA 开关同款 "配置 !Send, 值随命令进渲染线程" 模式):
//! 主线程 CONFIG_CHANGED 时即时读配置重建本参数包 → `UiCommand::ReinitOverlays`
//! 送渲染线程的线程局部仓 (`Rc<RefCell<ReinitParams>>`) → 各 spec 工厂的
//! reinit 闭包 (OverlaySpec.reinit) 读取最新值重建 state, 返回新 (w,h) 由 host
//! resize_entry 落窗口 — 对位 Java reinitConfig 的 setBounds 副作用。
//!
//! F15 分组嵌套: 各 overlay 组配置收为子结构 (共用形态共享类型, 各组自带
//! Default 的 Java 回退值 — 加字段只改本组); 顶层只留跨组消费面 (DPI/轮询节流/
//! 地平仪喂入节流/MiniHUD 快照)。

use std::sync::Arc;

use vm_core::config::config_api::HudSettingsSnapshot;
use vm_core::ui_support::row_def::RowDef;

/// 引擎控制组 (getOverlaySettings("引擎控制"))。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EngineGroup {
    /// 字号增量 (getFontSizeAdd)
    pub font_add: i32,
    /// 7 仪表 disable 开关 (ENGINE_DISABLE_KEYS 序)
    pub disables: [bool; 7],
}

/// 列表型面板组 (动力信息/飞行信息共用形态): 字号增量 + 列数 + W-D 行定义。
#[derive(Debug, Clone, PartialEq)]
pub struct ListGroup {
    /// 字号增量 (getFontSizeAdd)
    pub font_add: i32,
    /// 列数 (动力 hudColumns / 飞行 flightInfoColumn)
    pub columns: i32,
    /// W-D cfg 驱动行定义 (主线程从 ui_layout.cfg 编译, 行开关过滤后随包进渲染线程)
    pub rows: Arc<Vec<RowDef>>,
}

impl Default for ListGroup {
    /// 缺省 = Java 无配置回退: fontadd=0 / 单列 / 空行表
    fn default() -> Self {
        ListGroup {
            font_add: 0,
            columns: 1,
            rows: Arc::new(Vec::new()),
        }
    }
}

/// 边框开关组 (起落襟翼/操纵面共用形态): 字号增量 + 边缘模式。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EdgeGroup {
    /// 字号增量 (getFontSizeAdd)
    pub font_add: i32,
    /// 边缘开关 (起落襟翼 enablegearAndFlapsEdge / 操纵面 enableAxisEdge, cfg 缺省 false)
    pub show_edge: bool,
}

/// FM拆包数据组: 字号增量 (getOverlaySettings("FM拆包数据").getFontSizeAdd;
/// cfg 该组无字号滑条, 恒走 OverlaySettings 默认 0 — setupFont 的 14+add 面)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FmGroup {
    pub font_add: i32,
}

/// 地平仪组 (getOverlaySettings("地平仪"))。喂入节流 freq_ms 在顶层 —
/// 宿主 attitude_feed 消费, 不属绘制面组。
#[derive(Debug, Clone, PartialEq)]
pub struct AttitudeGroup {
    /// 宽高 (attitudeIndicatorWidth/Height, 工厂内再 DPI 缩放)
    pub width: i32,
    pub height: i32,
    /// 开关族 (attitudeIndicatorDisplayDirection/DisplayAoALimits)
    pub show_direction: bool,
    pub show_aoa_limits: bool,
}

impl Default for AttitudeGroup {
    /// 缺省 = Java reinitConfig 回退值 (AttitudeOverlay.java:232-248)
    fn default() -> Self {
        AttitudeGroup {
            width: 150,
            height: 300,
            show_direction: false,
            show_aoa_limits: true,
        }
    }
}

/// reinit 参数包 (纯值 Send; 各组来源 = OverlayInputs 同源配置键)。
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
    /// Application.dpiScale (各组几何的 DPI 缩放共用)
    pub dpi_scale: f64,
    /// Service 轮询间隔 (loadRefreshInterval 读 dataPollIntervalMs;
    /// 引擎控制 refreshInterval 同源)
    pub service_loop_interval_ms: i64,
    /// 地平仪喂入节流 (attitudeIndicatorFreqMs; 宿主 attitude_feed 消费)
    pub attitude_freq_ms: i64,
    /// 引擎控制组
    pub engine: EngineGroup,
    /// 动力信息组
    pub power: ListGroup,
    /// 飞行信息组
    pub flight: ListGroup,
    /// 起落襟翼组
    pub gear: EdgeGroup,
    /// 操纵面组
    pub axis: EdgeGroup,
    /// FM拆包数据组
    pub fm: FmGroup,
    /// 地平仪组
    pub attitude: AttitudeGroup,
    /// MiniHUD 全量设置快照 (reinit_config 的 S: HUDSettings 实参)
    pub hud: HudSettingsSnapshot,
}

impl Default for ReinitParams {
    /// 缺省 = Java 各 reinitConfig 的无配置回退值 (组内值见各组 Default;
    /// 顶层 dpi=1.0 / 轮询 50ms / 地平仪节流 40ms)
    fn default() -> Self {
        ReinitParams {
            dpi_scale: 1.0,
            service_loop_interval_ms: 50,
            attitude_freq_ms: 40,
            engine: Default::default(),
            power: Default::default(),
            flight: Default::default(),
            gear: Default::default(),
            axis: Default::default(),
            fm: Default::default(),
            attitude: Default::default(),
            hud: HudSettingsSnapshot::default(),
        }
    }
}
