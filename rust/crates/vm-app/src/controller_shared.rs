//! ControllerShared — 防过期世代号 + 跨线程可读状态快照。重构波2 自 app_shell.rs 拆出。

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::controller_state::ControllerState;
use vm_core::base::logger;
use vm_core::config::configuration_service::ControllerIntervals;

/// FlightDataBus 事件流静默判定阈值 (审查 B1 补偿, 见
/// [`ControllerShared::last_flight_event_ms`] 注): player_live 轮每 ~50ms 发布
/// 一帧, 2s = 40 轮静默 — 比 Java 的串空即时判定更宽容 (网络抖动/加载切换不误判)。
pub const FLIGHT_SILENT_EXIT_MS: i64 = 2000;

/// Controller 实例字段中需要跨线程读的部分。
/// PORT(世代号归属): Java previewGeneration 是 Controller 实例字段 (AtomicLong,
/// 每次托盘重建归零 — 旧核在途回调持旧世代号比对旧核, 靠 stop() 的 ++ 兜底);
/// Rust 收敛为 AppShell 级单调 (跨重建不重置), 防过期判定只会更严, 无假接受面。
pub struct ControllerShared {
    /// Java `AtomicLong previewGeneration` 实例字段
    pub preview_generation: AtomicU64,
    /// Java `public ControllerState State` — 主线程写, 渲染线程读 (stale 守卫)。
    /// Java 无锁靠 UI 线程单线程; Rust 以 RwLock 承载跨线程读
    pub state: RwLock<ControllerState>,
    /// Java loadAppCheck 写入的轮询间隔组 (ConfigurationService.load_app_check 目标)
    pub intervals: Mutex<ControllerIntervals>,
    /// host.overlays_hidden 的跨线程镜像 (Java AlwaysOnTopCoordinator.
    /// overlaysHidden volatile — FocusMonitor 经通道桥查询; 渲染线程处理
    /// Hide/Show 命令时与 host 同步置位)
    pub overlays_hidden: AtomicBool,
    /// 低频杂项标志 (showStatus/sessionAircraftType/currentFmHotkeyCode)
    pub flags: Mutex<ControllerFlags>,
    /// 游戏模式 Service 数据快照句柄 (start() 建 / stop() 清;
    /// 渲染线程 live 喂入 + 主线程 tick 驱动读)。
    /// 重构波4: 类型从 Arc<RwLock<ServiceData>> (共享锁读) 改为帧仓 —
    /// 读者零锁取不可变整帧, feed_overlays_live 持锁跨计算的 B-W2 备案消亡
    pub live: RwLock<Option<Arc<vm_data::frame::FrameStore>>>,
    /// OverlayContext.isPreviewMode 的跨线程替身 (Java: forPreviewMode/forGameMode
    /// 两种 ctx 构建)。语义 = **会话窗口形态** (审查 blocker 收口): openpad→false /
    /// CloseAll/重建核→true; RefreshPreviews 仅在激活探测期临时置 true (对位 Java
    /// refreshPreviews 传 forPreviewMode ctx, 见渲染线程命令处理点 PORT 注)。
    pub overlay_ctx_preview: AtomicBool,
    /// 最后一次 FlightDataEvent 到达时间 (ms epoch; 0 = 本核会话未见)。
    /// PORT(B1 补偿): vm-data 不外泄原始串 (http_client 轮询线程独占),
    /// 游戏退出 (HTTP 失败 → 串复位空串, http_helper NSTRING) 时 State/Indicators
    /// 的 update 不执行, flags 保留陈旧真值 — Java 的 "串空 → S4toS1" 路径
    /// 在 flags 判定下不可达。以 "事件流静默超时"
    /// 顶替: player_live 轮每 ~50ms 发布一帧, 游戏退出即停发; 静默超过
    /// [`FLIGHT_SILENT_EXIT_MS`] 且 flags/playerLive 陈旧真值 → 判定会话结束。
    /// vm-data 后续波次补 raw_strings_valid 外泄后回收本补偿。
    pub last_flight_event_ms: AtomicI64,
    /// overlay present 帧数 (渲染线程 50ms 渲染节拍, 活跃 overlay 存在时 +1;
    /// host 跨重建存活 → 跨核单调累积, 冒烟断言面)。host 无逐窗 present 计数
    /// 外泄 (render_tick Result 不分首帧/脏检查抑制), 以"活跃窗口在场的成功
    /// render_tick 次数"为 present 帧数的保守代理 (首帧必 present, 计数≥它)。
    pub render_frames: AtomicU64,
    /// 逐 overlay present 帧数 (注册面以 0 落键, 渲染节拍逐活跃窗口 +1;
    /// 从未激活/注册失败的项如实暴露 — 冒烟"全部注册 overlay present>0"判据)。
    /// 代理语义同 render_frames 注 (在场成功 render_tick ≥ 真实 present 数)。
    pub overlay_present: Mutex<BTreeMap<String, u64>>,
}

/// Controller 低频杂项字段 (Java Controller 实例字段的收敛)
#[derive(Debug, Clone, Default)]
pub struct ControllerFlags {
    /// `private boolean showStatus` (loadFromConfig 同步; StatusBar 未移植, 仅保位)
    pub show_status: bool,
    /// `private String sessionAircraftType` (onAircraftChanged 幂等去重)
    pub session_aircraft_type: Option<String>,
    /// `private int currentFmHotkeyCode` (热键重绑定跟踪)
    pub current_fm_hotkey_code: i32,
}

impl ControllerShared {
    pub fn new() -> Self {
        ControllerShared {
            preview_generation: AtomicU64::new(0),
            state: RwLock::new(ControllerState::Init),
            intervals: Mutex::new(ControllerIntervals::default()),
            overlays_hidden: AtomicBool::new(false),
            flags: Mutex::new(ControllerFlags::default()),
            live: RwLock::new(None),
            overlay_ctx_preview: AtomicBool::new(true),
            last_flight_event_ms: AtomicI64::new(0),
            render_frames: AtomicU64::new(0),
            overlay_present: Mutex::new(BTreeMap::new()),
        }
    }

    /// 托盘重建新核前复位 (Java 构造器显式赋值 `State = ControllerState.INIT`;
    /// 审查 A-W1: sessionAircraftType 是 Controller 实例字段, Java 每次托盘重建随新
    /// 实例归 null — Rust flags 跨核共享, 需显式复位, 否则
    /// 重建后首个不同机型被误判 is_switch。overlay_ctx_preview 同理回预览态初值,
    /// 防残留游戏模式值影响 INIT 期的激活探测)
    pub fn reset_for_rebuild(&self) {
        *self.state.write().expect("Controller 状态锁中毒") = ControllerState::Init;
        self.flags
            .lock()
            .expect("flags 锁中毒")
            .session_aircraft_type = None;
        self.overlay_ctx_preview.store(true, Ordering::SeqCst);
        self.last_flight_event_ms.store(0, Ordering::SeqCst);
    }

    /// State 快照读 (跨线程安全; 主线程写点: 各状态转移方法)
    pub fn state(&self) -> ControllerState {
        *self.state.read().expect("Controller 状态锁中毒")
    }

    /// 注册面落键: overlay id → 0 (逐窗 present 计数起点)。注册失败不落键 —
    /// 冒烟断言按 9 键全集判 (窗口条目), 缺键即注册失败如实暴露 (不假通过);
    /// thrustdFS 呈键但计数可为 0 (激活需喷气机 — 冒烟场景 p-51d 为螺旋桨)
    pub(crate) fn note_registered_overlay(&self, id: &str) {
        self.overlay_present
            .lock()
            .expect("overlay_present 锁中毒")
            .entry(id.to_string())
            .or_insert(0);
    }

    pub(crate) fn set_state(&self, s: ControllerState) {
        *self.state.write().expect("Controller 状态锁中毒") = s;
    }
}

impl Default for ControllerShared {
    fn default() -> Self {
        Self::new()
    }
}

/// 防过期守卫 (渲染线程消费 UiCommand::RefreshPreviews 时调用)。
/// Java refreshPreviews 的 UI 线程派发内守卫
/// (`State != PREVIEW || previewGeneration.get() != generation`)。
/// Java 防抖路径 (configChanged/fmChanged 任务体) 无此守卫 (★2 违规波及面),
/// Rust 统一经本守卫 — 世代号不匹配或已离开 PREVIEW 即丢弃。
pub fn is_stale_refresh(shared: &ControllerShared, generation: u64) -> bool {
    let state = shared.state();
    let current = shared.preview_generation.load(Ordering::SeqCst);
    if state != ControllerState::Preview || current != generation {
        logger::info(
            "Controller",
            &format!(
                "Skipping stale preview refresh (gen={}, current={}, state={})",
                generation, current, state
            ),
        );
        true
    } else {
        false
    }
}
