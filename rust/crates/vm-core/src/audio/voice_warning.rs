//! VoiceWarning 的 Rust 移植 (src/prog/audio/VoiceWarning.java) — 一比一翻译。
//!
//! 语音告警系统
//!
//! 线程模型：
//! - Service 线程 → run() → VoiceAlert.playOnce()
//! - UI 线程 → configHandler → VoiceAlert.reload()
//! - FlightDataBus → flightDataListener → currentMismatch (volatile)
//!
//! 重构说明：
//! - 使用 ConcurrentHashMap 替代 HashMap 保证线程安全
//! - VoiceAlert 类使用 volatile 字段和 synchronized reload() 保证线程安全
//! - run() 方法拆分为独立的 check* 方法，每个方法负责一种告警
//!
//! PORT (依赖注入面, Java 进程级静态依赖的解散 — focus_monitor.rs 先例):
//! - `VoiceResourceManager.getInstance()` / `FMManager.getInstance()` /
//!   `UIStateBus.getInstance()` / `FlightDataBus.getInstance()` 四个全局单例
//!   → 构造参数注入 (§2.9 禁再造全局静态; 组装层持同一 Arc 表达单例语义)。
//! - `prog.Service` (vm-data, C 类波次; vm-core 禁依赖 vm-data — D3/D6 分层) →
//!   本文件定义消费面 trait [`VoiceWarningService`] (Java 对 xS 的全部字段/
//!   方法访问点逐项签名化)。
//! - `prog.Controller` (xc 字段 + init* 方法参数): xc 是 write-only 字段
//!   (全文件无读取点), init* 的 c 参数无消费 —
//!   不落字段/参数; configProvider 在 Java 经 `c.getConfigService()` 取得,
//!   Rust 由构造注入。
//!
//! PORT (Java 现存泄漏根治, LIFETIMES §2.1/§6.3.1 — 本文件主任务):
//! Java 版两个订阅在生产路径均泄漏: (1) dispose() 本身
//! 只退订 FlightDataListener, 漏退订 UIStateBus configHandler
//! (订阅无对应 unsubscribe); (2) 该 dispose() 根本无人调用 — VoiceWarning 非
//! java.awt.Window, OverlayEntry.close 的 `instance instanceof Window`
//! 判定为 false。每次进出游戏模式 new 一个 VoiceWarning,
//! 旧实例连同其全部 VoiceAlert/音频 clip 被两条总线永久持有 (含 Clip 原生句柄,
//! 随重建累积)。Rust 版两个订阅均以 RAII Subscription 字段由 VoiceWarning 持有:
//! dispose() 显式注销 + Drop 兜底, 泄漏在类型层面不可能发生。
//!
//! PORT (线程/同步映射, LIFETIMES §3.2):
//! - `volatile Boolean doit` → `Arc<AtomicBool>` (doit 标志族裁决; Java 缺省
//!   null — init 前调 run() 会 NPE, 折衷为 false 并以此注记, init 成功后不可达);
//! - `volatile Clip clip / boolean available / isAct / long lastTimePlay` →
//!   `Mutex<Option<Box<dyn SoundClip>>>` + 三个原子 (volatile 逐字段可见性);
//! - `synchronized reload()` → 以 clip Mutex 为 monitor (临界区 = 整个方法体,
//!   对齐 Java 整方法 monitor);
//!   语义收紧备案: Java 的 playOnce/isPlaying/close 均非 synchronized (仅
//!   volatile 读), 可与 reload 并发交错 — 撞上已 close 的旧 clip 时
//!   setFramePosition/start 抛异常进 debug 日志腿跳过; Rust 三方法同持 clip
//!   Mutex, 播放路径与 reload 串行化 (中途交错窗口消失, 确定性化)。SoundClip
//!   实现契约: is_running/set_frame_position/start/stop/close 在持锁下调用,
//!   须非长阻塞 — 否则 UI 线程 reload() 同锁等待 (无死锁面, 仅延迟耦合);
//! - `ConcurrentHashMap<String, VoiceAlert> alerts` → `Mutex<HashMap<..>>`
//!   (LIFETIMES §3.2 "访问频率不高" 裁决; 只做 get/insert, 无迭代序依赖 §2.5);
//! - `volatile boolean currentMismatch` → `Arc<AtomicBool>` (FlightDataBus
//!   回调写 / run 读)。
//!
//! PORT (st/indic 活引用 → 快照刷新): Java 的 `st = xS.sState; indic = xS.sIndic`
//! 捕获的是 Service 构造时创建、之后被 Service 线程**原地改写**的活对象引用
//! (Service 线程每轮轮询原地更新); Rust 快照架构 (vm-data ServiceData) 下由
//! [`VoiceWarningService::s_state`]/`s_indic`] 返回当前副本, run() 每轮循环
//! 开头刷新 — 与 Java "check 方法执行时读到最新遥测" 语义同类 (快照粒度 =
//! 一个 tick, Java 逐字段读的竞态窗口同样存在且无同步)。
//!
//! PORT (Java f 后缀字面量): `0.95f` 的 f32 值 ≠ f64 字面量 0.95, 必须写
//! `0.95f32 as f64` (init/updateDynamicParameters 的告警线乘 0.95f 处);
//! 其余 f 后缀字面量 (2.0f/8.0f/10.0f/0.75f/-8) 均为二进制精确值, f32→f64
//! 与同名 f64 字面量逐位相等, 直接写 f64 字面量。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::audio::voice_pack_config::VOICE_PREFIX;
use crate::audio::voice_resource_manager::{SoundClip, SoundPlayer, VoiceResourceManager};
use crate::audio::{VoiceAlertType, VoicePackConfig};
use crate::base::bus::flight_data_bus::FlightDataBus;
use crate::base::bus::ui_state_bus::{UIStateBus, UiStateEvent};
use crate::base::bus::Subscription;
use crate::base::event::flight_data_event::FlightDataEvent;
use crate::base::event::ui_state_events;
use crate::base::exception_helper::sleep_while_run;
use crate::base::logger;
use crate::config::config_api::ConfigProvider;
use crate::fm::data::FmData;
use crate::fm::FMManager;
use crate::game_api::parser::{Indicators, State};

/// clip 槽位锁中毒消息 (Java 无锁; 对应持锁线程崩溃后的一致性未知面)
const CLIP_LOCK_MSG: &str = "VoiceAlert clip 锁中毒";
/// alerts 注册表锁中毒消息
const ALERTS_LOCK_MSG: &str = "VoiceWarning alerts 注册表锁中毒";

/// Java: `public static final int maxEndVoiceNum = 4;`
pub const MAX_END_VOICE_NUM: i32 = 4;

/// Java: `public static final long sleepTime = 100;` (主循环节拍, ms)
pub const SLEEP_TIME: i64 = 100;

/// Java: `private static final long GEAR_DAMAGE_THRESHOLD_MS = 10000;`
/// 起落架超速 10 秒后标记为损坏
const GEAR_DAMAGE_THRESHOLD_MS: i64 = 10000;

/// Java: `private static final long FLAP_DAMAGE_THRESHOLD_MS = 15000;`
/// 襟翼超速 15 秒后标记为损坏
const FLAP_DAMAGE_THRESHOLD_MS: i64 = 15000;

/// Java: `private static final long COMPRESSOR_WARN_DELAY = 3000;`
/// 3-second delay before warning
const COMPRESSOR_WARN_DELAY: i64 = 3000;

/// VoiceWarning 对 `prog.Service` 的依赖面 (focus_monitor.rs 消费方 trait 先例)。
///
/// Java 直接读 `xS` 的 public 字段/方法 (Service 为 C 类波次, vm-core 不可依赖
/// vm-data — D3/D6 分层), 本 trait = 该访问面的逐项签名化; vm-data Service 波次
/// 落地时为其 impl 本 trait 接线。
///
/// st/indic 两项为快照读 (见模块头 PORT 注): Java 侧是 Service 内被原地改写的
/// 活对象引用, Rust 实现返回当前副本, run() 每轮刷新。
pub trait VoiceWarningService: Send + Sync {
    /// Java: `xS.currentTimeMs` (long)
    fn current_time_ms(&self) -> i64;
    /// Java: `xS.playerLive` (boolean)
    fn player_live(&self) -> bool;
    /// Java: `xS.fatalWarn = fatal` (Boolean 字段在 VoiceWarning 的唯一写点)
    fn set_fatal_warn(&self, v: bool);
    /// Java: `xS.isDowningFlap` (boolean)
    fn is_downing_flap(&self) -> bool;
    /// Java: `xS.flapAllowAngle` (double)
    fn flap_allow_angle(&self) -> f64;
    /// Java: `xS.flapAllowSpeed` (double)
    fn flap_allow_speed(&self) -> f64;
    /// Java: `xS.totalFuel` (double)
    fn total_fuel(&self) -> f64;
    /// Java: `xS.fuelPercent` (int)
    fn fuel_percent(&self) -> i32;
    /// Java: `xS.radioAlt` (double)
    fn radio_alt(&self) -> f64;
    /// Java: `xS.dRadioAlt` (double)
    fn d_radio_alt(&self) -> f64;
    /// Java: `xS.curLoadMinWorkTime` (double)
    fn cur_load_min_work_time(&self) -> f64;
    /// Java: `xS.maximumThrRPM` (double)
    fn maximum_thr_rpm(&self) -> f64;
    /// Java: `xS.getMaximumRPM` (boolean 字段, 与同名方法区分)
    fn get_maximum_rpm(&self) -> bool;
    /// Java: `xS.isEngJet()`
    fn is_eng_jet(&self) -> bool;
    /// Java: `xS.getStallSpeed()`
    fn get_stall_speed(&self) -> f64;
    /// Java: `st = xS.sState` (Service 构造即创建, 永非 null)
    fn s_state(&self) -> State;
    /// Java: `indic = xS.sIndic`
    fn s_indic(&self) -> Indicators;
}

// =====================================================================
// VoiceAlert (Java 非静态内部类 → 独立 struct, §1 内部类规则)
// =====================================================================

/// 线程安全的语音告警封装类
///
/// 线程安全保证：
/// - clip 字段使用 volatile
/// - reload() 方法使用 synchronized
/// - playOnce() 不需要同步（只读 clip 引用，播放操作由底层 Clip 保证）
///
/// PORT: Java 非静态内部类持外层 VoiceWarning.this (构造器副作用 alerts.put +
/// reload 依赖外层字段) → 独立 struct, 注册与首次 reload 移至父侧工厂
/// [`VoiceWarning::new_alert`], reload 的外层依赖 (configProvider/
/// VoiceResourceManager) 改为调用点参数传入。
/// PORT: Clip → `Box<dyn SoundClip>` (D7 注入面); Java `catch (Exception)` 的
/// 播放/关闭异常腿在 trait 面不可失败, 相应 catch 分支不可达 (各处 PORT 注)。
pub struct VoiceAlert {
    /// volatile 确保可见性
    /// PORT: Java `private volatile Clip clip` → Mutex 承载独占访问 + 原子换引用
    clip: Mutex<Option<Box<dyn SoundClip>>>,
    /// volatile 确保可见性
    available: AtomicBool,
    /// volatile 确保可见性, package-private for aoaHigh special case
    /// (pub: 对应 Java 同包直写 — VoiceWarning.checkAoAWarning 的特殊标记)
    pub is_act: AtomicBool,
    /// volatile 确保可见性, package-private for aoaHigh special case
    pub last_time_play: AtomicI64,
    /// Java: `private final long coolDownMs`
    cool_down_ms: i64,
    /// Java: `private final String key`
    key: String,
}

impl VoiceAlert {
    /// Java: `public VoiceAlert(String key, long coolDownSeconds)`
    ///
    /// Java 构造器体内 `this.isAct = false; alerts.put(key, this); reload();`
    /// 三步中后两步依赖外层 VoiceWarning (内部类), 移至父侧工厂
    /// [`VoiceWarning::new_alert`] 保序执行。
    pub fn new(key: &str, cool_down_seconds: i64) -> Self {
        VoiceAlert {
            clip: Mutex::new(None),
            available: AtomicBool::new(false),
            is_act: AtomicBool::new(false),
            last_time_play: AtomicI64::new(0),
            cool_down_ms: cool_down_seconds * 1000,
            key: key.to_string(),
        }
    }

    /// 重新加载音频资源（线程安全）
    /// 必须同步以防止多个线程同时 reload
    ///
    /// PORT: `synchronized` 整方法 monitor → 持 clip 锁贯穿方法体 (临界区等价);
    /// 外层 configProvider/VoiceResourceManager 单例取用 → 参数注入。
    pub fn reload(
        &self,
        config_provider: &dyn ConfigProvider,
        resource_manager: &VoiceResourceManager,
    ) {
        let mut clip_slot = self.clip.lock().expect(CLIP_LOCK_MSG);

        // 先关闭旧资源
        if let Some(old_clip) = clip_slot.as_ref() {
            //      catch (Exception e) { Logger.warn("VoiceAlert", "关闭旧 Clip 失败: " + key); }
            // PORT: SoundClip 面不可失败 (D7), catch 腿不可达
            if old_clip.is_running() {
                old_clip.stop();
            }
            old_clip.close();
        }

        // 解析配置
        let config_key = VoicePackConfig::with_voice_prefix(Some(&self.key)).unwrap();
        let val = config_provider.get_config(&config_key);
        let config = VoicePackConfig::parse(val.as_deref());

        if !config.enabled {
            self.available.store(false, Ordering::SeqCst);
            *clip_slot = None;
            return;
        }

        // 加载新 Clip
        *clip_slot = resource_manager.load_clip(&self.key, Some(&config.pack_name));
        self.available.store(clip_slot.is_some(), Ordering::SeqCst);
    }

    /// 播放一次（带冷却时间检查）
    /// 不需要同步：volatile 保证读取最新值，播放操作是原子的
    pub fn play_once(&self, time: i64) {
        if self.is_playing(time) {
            return;
        }

        self.is_act.store(true, Ordering::SeqCst);
        self.last_time_play.store(time, Ordering::SeqCst);

        // 本地引用，避免中途被 reload 改变 — Rust: 锁守卫即一致的本地引用
        let c = self.clip.lock().expect(CLIP_LOCK_MSG);
        if !self.available.load(Ordering::SeqCst) || c.is_none() {
            return;
        }

        //      catch (Exception e) { Logger.debug("VoiceAlert", "播放失败: " + key + " - " + e.getMessage()); }
        // PORT: SoundClip 面不可失败, catch 腿不可达
        let c = c.as_ref().unwrap();
        c.set_frame_position(0);
        c.start();
    }

    /// 检查是否正在播放（或在冷却期内）
    pub fn is_playing(&self, time: i64) -> bool {
        let c = self.clip.lock().expect(CLIP_LOCK_MSG);
        if !self.available.load(Ordering::SeqCst) || c.is_none() {
            return true; // 不可用时假装在播放，防止重试循环
        }

        if self.is_act.load(Ordering::SeqCst) {
            // PORT: Java long 时间差 (§2.2 时间差类运算) — wrapping 复刻静默回绕
            if time.wrapping_sub(self.last_time_play.load(Ordering::SeqCst)) <= self.cool_down_ms {
                return true; // 在冷却期内
            }
            let still = c.is_some() && c.as_ref().unwrap().is_running();
            self.is_act.store(still, Ordering::SeqCst);
            return still;
        }
        false
    }

    /// 关闭资源
    pub fn close(&self) {
        let mut clip_slot = self.clip.lock().expect(CLIP_LOCK_MSG);
        if let Some(c) = clip_slot.as_ref() {
            // PORT: SoundClip 面不可失败, 空 catch 腿不可达 (§2.7)
            if c.is_running() {
                c.stop();
            }
            c.close();
        }
        *clip_slot = None;
    }

    /// Java: `public String getKey()`
    pub fn get_key(&self) -> &str {
        &self.key
    }
}

// =====================================================================
// WarningSlot (F4: 告警三元组收编)
// =====================================================================

/// 告警槽位 (F4): `Option<Arc<VoiceAlert>>` + 告警线 + 计数 三元组收编。
/// 原平铺形态 (xxxWarn / xxxCheck / xxxLine 各自命名字段) 模式重复。
/// - `line`: 告警线/阈值; 无静态线的组不使用 (恒 0, 阈值在 check 内动态取/内联);
///   舵效组存 FM f64 值经 i32 截断后的速度线 (保 Java float→int 语义)。
/// - `check`: 去抖/持续计数 (ms); 舵效组复用为 0/1 边沿锁存标记; 无计数的组
///   不使用 (恒 0)。原 i32 计数 (fuelPCheck/oofCheck) 值域 (0,2000] 远离回绕,
///   收宽为 i64 无行为差异。
#[derive(Default)]
struct WarningSlot {
    alert: Option<Arc<VoiceAlert>>,
    line: f64,
    check: i64,
}

impl WarningSlot {
    /// 装配: 挂告警实例, line/check 归零 (init 腿随后按原逻辑覆写)
    fn armed(alert: Arc<VoiceAlert>) -> Self {
        WarningSlot {
            alert: Some(alert),
            line: 0.0,
            check: 0,
        }
    }
}

// =====================================================================
// VoiceWarning 本体
// =====================================================================

/// Java: `public class VoiceWarning implements Runnable`
///
/// PORT: Runnable → `run(&mut self)` 关联方法 (调用方 std::thread 持本结构)。
/// 组装契约 (vm-data Controller 波次注意): run 需把整个结构 **move 进语音线程**,
/// move 后 `dispose(&mut self)` 不可再达 — 停机路径 = move 前克隆 `pub doit`
/// 的 Arc、外部翻 false, 线程退出时 Drop 兜底注销双订阅 (测试
/// `run_loop_ticks_and_exits_on_stop` 演示该模式); dispose 仅在 move 前可用。
pub struct VoiceWarning {
    /// Java: `long GCCheckMili;` (全文件无读写点, 保真保留 §2.10)
    pub gcc_check_mili: i64,
    /// Java: `Service xS` (可 null → Option; init 前的 null 读面见 [`VoiceWarning::xs`])
    xs: Option<Arc<dyn VoiceWarningService>>,
    /// Java: `State st` — 活引用 → 每轮刷新的快照 (见模块头 PORT 注)
    st: State,
    /// Java: `Indicators indic` — 同上
    indic: Indicators,
    /// Java: `volatile Boolean doit` (volatile 保证可见性)
    pub doit: Arc<AtomicBool>,

    // ---- 注入面 (Java 全局单例/Controller 访问的解散, 见模块头 PORT 注) ----
    /// Java: `private ConfigProvider configProvider` (原经 `c.getConfigService()`)
    config_provider: Arc<dyn ConfigProvider + Send + Sync>,
    /// Java: `VoiceResourceManager.getInstance()`
    resource_manager: Arc<VoiceResourceManager>,
    /// Java: `FMManager.getInstance()`
    fm_manager: Arc<FMManager>,
    /// Java: `prog.event.UIStateBus.getInstance()`
    ui_state_bus: Arc<UIStateBus>,
    /// Java: `prog.event.FlightDataBus.getInstance()`
    flight_data_bus: Arc<FlightDataBus>,
    /// playWav/getClip 的 Java `AudioSystem` 直开面 — 两方法已随保真残留退役删除,
    /// 字段与 `new` 第 6 参数仅为 vm-app voice_setup 装配腿签名锁定而留
    // DEAD(kept): 装配签名由 vm-app voice_setup.rs 锁定, AppShell 收口波次连带退役
    #[allow(dead_code)]
    legacy_player: Arc<dyn SoundPlayer>,

    pub aoa_warning_line: f64,

    /// Java: `private ConcurrentHashMap<String, VoiceAlert> alerts`
    /// (UI 线程热重载，Service 线程播放) — 与 configHandler 闭包共享所有权
    alerts: Arc<Mutex<HashMap<String, Arc<VoiceAlert>>>>,
    /// Java: `private Consumer<Object> configHandler` + UIStateBus 订阅
    /// PORT: 闭包 + Subscription 二位一体; `configHandler == null` 判重 →
    /// `is_none()` (Java 版 handler 字段在 dispose 后不置 null, 但其泄漏的
    /// 旧 handler 引用的 alerts map 与新 init 重填的 map 是同一个 — Rust 侧
    /// map 为共享 Arc, 退订+重订后行为等价且不泄漏)
    config_subscription: Option<Subscription<UiStateEvent>>,

    // 攻角提示
    aoa_crit: Option<Arc<VoiceAlert>>,
    aoa_high: Option<Arc<VoiceAlert>>,
    // (Java `private Controller xc` — write-only 字段无读取点, 不落地, 见模块头)

    // 速度相关 (F4: ias/mach = alert+告警线; stall 无附属字段, 平铺保留)
    ias: WarningSlot,
    mach: WarningSlot,
    stall_warn: Option<Arc<VoiceAlert>>,

    // 起落架/襟翼/减速板 (F4: gear = alert+限速线+损坏计时; flap = alert+损坏
    // 计时, 无静态线 — flapAllowSpeed 每轮动态取; brake 无附属字段, 平铺保留)
    gear: WarningSlot,
    flap: WarningSlot,
    brake_warn: Option<Arc<VoiceAlert>>,
    is_gear_alive: bool,
    is_flap_alive: bool,

    // 过载
    pub ny_warning_line0: f64,
    pub ny_warning_line1: f64,
    ny_warn: Option<Arc<VoiceAlert>>,
    /// Java: `private parser.Blkx blkx` — init 时捕获的引用 → 按值快照
    /// (本类只读 rawWingCritOverload, 无会话态读写, 克隆无行为差异)。
    /// 代价备案: init_structure_warnings 的 `fm.blkx.clone()` 是全量深拷 vs
    /// Java O(1) 引用别名 — 仅 init 时一次 (每 tick 的
    /// update_dynamic_parameters 走 as_ref 无拷); fm/handle.rs 已预留
    /// Arc<FMHandle>/Arc<Blkx> 共享裁决, P4/P5 组装时回收
    fmdata: Option<FmData>,
    nofuelweight: f64,

    // 引擎相关 (F4/A6: engFail = alert + "引擎损坏后"腿的独立计数器)
    eng_warn: Option<Arc<VoiceAlert>>,
    eng_fail: WarningSlot,
    eng_fail_invert: Option<Arc<VoiceAlert>>,
    rpm_low_warn: Option<Arc<VoiceAlert>>,
    rpm_high_warn: Option<Arc<VoiceAlert>>,
    pub eng_damage: bool,

    // 燃油相关 (F4: fuel = alert+低油量线+去抖; fuel_prs/oof = alert+去抖,
    // 无静态线 — 油压门限在 check 内联)
    fuel: WarningSlot,
    fuel_prs: WarningSlot,
    oof: WarningSlot,

    // 高度相关
    height_warn: Option<Arc<VoiceAlert>>,
    terrain_warn: Option<Arc<VoiceAlert>>,
    vario_warn: Option<Arc<VoiceAlert>>,

    // 舵效相关 (F4: = alert + 速度线 + 0/1 边沿锁存; elevator 的锁存位原版只写
    // 不读 — 告警触发块在 Java 源码中即缺失, 保真保留写点, 见 check 方法)
    rudder: WarningSlot,
    elevator: WarningSlot,
    aileron: WarningSlot,

    // 增压器档位告警
    compressor_stage_warn: Option<Arc<VoiceAlert>>,
    /// Java: `private volatile boolean currentMismatch` (from FlightDataBus)
    /// — 与订阅闭包共享
    current_mismatch: Arc<AtomicBool>,
    /// For detecting state change (false→true, true→false)
    last_mismatch: bool,
    /// 0 = no pending warning, >0 = scheduled warning time
    pending_compressor_warn_time: i64,
    /// Java: `private FlightDataListener flightDataListener` + register/unregister
    /// PORT: RAII Subscription (Drop 即注销); init 二次调用时旧句柄被覆盖 drop
    /// = 自动退订 (Java 版此处会泄漏旧 listener — 顺手根治, 与 configHandler 同款)
    flight_data_subscription: Option<Subscription<FlightDataEvent>>,
}

impl VoiceWarning {
    /// Java 隐式默认构造器 (字段取缺省值) + PORT 注入面装配。
    pub fn new(
        config_provider: Arc<dyn ConfigProvider + Send + Sync>,
        resource_manager: Arc<VoiceResourceManager>,
        fm_manager: Arc<FMManager>,
        ui_state_bus: Arc<UIStateBus>,
        flight_data_bus: Arc<FlightDataBus>,
        // 见 legacy_player 字段注 (保留仅为 vm-app 装配腿签名兼容)
        legacy_player: Arc<dyn SoundPlayer>,
    ) -> Self {
        VoiceWarning {
            gcc_check_mili: 0,
            xs: None,
            // PORT: Java `State st` 缺省 null (init 前读会 NPE) — 快照架构需要
            // 占位值, 取零值 State; init 前 run()/check 不可达 (doit=false 门禁)
            st: State::new(),
            indic: Indicators::new(),
            doit: Arc::new(AtomicBool::new(false)),
            config_provider,
            resource_manager,
            fm_manager,
            ui_state_bus,
            flight_data_bus,
            legacy_player,
            aoa_warning_line: 0.0,
            alerts: Arc::new(Mutex::new(HashMap::new())),
            config_subscription: None,
            aoa_crit: None,
            aoa_high: None,
            ias: WarningSlot::default(),
            mach: WarningSlot::default(),
            stall_warn: None,
            gear: WarningSlot::default(),
            flap: WarningSlot::default(),
            brake_warn: None,
            is_gear_alive: false,
            is_flap_alive: false,
            ny_warning_line0: 0.0,
            ny_warning_line1: 0.0,
            ny_warn: None,
            fmdata: None,
            nofuelweight: 0.0,
            eng_warn: None,
            eng_fail: WarningSlot::default(),
            eng_fail_invert: None,
            rpm_low_warn: None,
            rpm_high_warn: None,
            eng_damage: false,
            fuel: WarningSlot::default(),
            fuel_prs: WarningSlot::default(),
            oof: WarningSlot::default(),
            height_warn: None,
            terrain_warn: None,
            vario_warn: None,
            rudder: WarningSlot::default(),
            elevator: WarningSlot::default(),
            aileron: WarningSlot::default(),
            compressor_stage_warn: None,
            current_mismatch: Arc::new(AtomicBool::new(false)),
            last_mismatch: false,
            pending_compressor_warn_time: 0,
            flight_data_subscription: None,
        }
    }

    /// Java `new VoiceAlert(key, coolDownSeconds)` 构造器副作用的复刻点:
    /// 注册到 alerts map + 初次 reload (内部类对外层字段的访问溶解为父侧工厂,
    /// §1 内部类→独立 struct 规则; 顺序 = Java 构造器语句序: put → reload)。
    fn new_alert(&self, key: &str, cool_down_seconds: i64) -> Arc<VoiceAlert> {
        let alert = Arc::new(VoiceAlert::new(key, cool_down_seconds));
        // 共用 "fail_engine", map 最终指向后注册的 invert 实例, 保真)
        self.alerts
            .lock()
            .expect(ALERTS_LOCK_MSG)
            .insert(key.to_string(), Arc::clone(&alert));
        alert.reload(&*self.config_provider, &self.resource_manager);
        alert
    }

    /// Java `xS` 字段读的 null 面对位: init 前为 null (Java NPE) ≡ panic;
    /// doit 门禁保证 run/check 路径在 init 成功后才可达。
    fn xs(&self) -> &Arc<dyn VoiceWarningService> {
        self.xs
            .as_ref()
            .expect("xS 未初始化 (Java: xS 为 null 的 NPE 对位; init(S) 成功后不可达)")
    }

    /// Java 告警字段的 null 解引用对位 (init 前 null ≡ NPE; init 后不可达)。
    fn alert<'a>(&self, slot: &'a Option<Arc<VoiceAlert>>, name: &str) -> &'a Arc<VoiceAlert> {
        slot.as_ref()
            .unwrap_or_else(|| panic!("{name} 未初始化 (Java: null 字段 NPE 对位; init 后不可达)"))
    }

    /// Java: `public void init(Controller c, Service S)`
    /// PORT: Controller 参数无消费面 (见模块头); `Service S` 可 null → Option。
    pub fn init(&mut self, s: Option<Arc<dyn VoiceWarningService>>) {
        let Some(s) = s else {
            self.doit.store(false, Ordering::SeqCst);
            return;
        };
        self.xs = Some(Arc::clone(&s));

        // PORT: Java 捕获活引用; Rust 取首帧快照 (run 每轮再刷新)
        self.st = s.s_state();
        self.indic = s.s_indic();

        // Config Listener - 使用 VoicePackConfig 工具方法
        if self.config_subscription.is_none() {
            //       UIStateBus.getInstance().subscribe(CONFIG_CHANGED, configHandler);
            let alerts = Arc::clone(&self.alerts);
            let config_provider = Arc::clone(&self.config_provider);
            let resource_manager = Arc::clone(&self.resource_manager);
            self.config_subscription = Some(self.ui_state_bus.subscribe(
                ui_state_events::CONFIG_CHANGED,
                move |msg: &UiStateEvent| {
                    if let Some(str_key) = msg.data.as_deref() {
                        if str_key.starts_with(VOICE_PREFIX) {
                            let alert_key =
                                VoicePackConfig::strip_voice_prefix(Some(str_key)).unwrap();
                            // ConcurrentHashMap 线程安全 ↔ Mutex 快照取值
                            let alert = alerts
                                .lock()
                                .expect(ALERTS_LOCK_MSG)
                                .get(&alert_key)
                                .cloned();
                            if let Some(alert) = alert {
                                alert.reload(&*config_provider, &resource_manager); // synchronized 方法
                                logger::info(
                                    "VoiceWarning",
                                    &format!("Reloaded voice clip: {alert_key}"),
                                );
                            }
                        }
                    }
                },
            ));
        }

        // 初始化告警 - 使用 VoiceAlertType 获取默认冷却时间
        self.init_aoa_warnings();
        self.init_speed_warnings();
        self.init_structure_warnings();
        self.init_engine_warnings();
        self.init_fuel_warnings();
        self.init_altitude_warnings();
        self.init_control_effectiveness_warnings();
        self.init_compressor_warning();

        // 播放启动音效
        let start_sound = self.new_alert("start1", 1);
        start_sound.play_once(s.current_time_ms());

        // 初始化状态
        self.eng_damage = false;
        self.is_gear_alive = true;
        self.is_flap_alive = true;
        self.last_mismatch = false;
        self.pending_compressor_warn_time = 0;
        self.doit.store(true, Ordering::SeqCst);
    }

    /// 初始化攻角告警
    fn init_aoa_warnings(&mut self) {
        self.aoa_crit = Some(self.new_alert(
            "aoaCrit",
            VoiceAlertType::AoaCrit.get_cooldown_seconds() as i64,
        ));
        self.aoa_high = Some(self.new_alert(
            "aoaHigh",
            VoiceAlertType::AoaHigh.get_cooldown_seconds() as i64,
        ));
        self.aoa_warning_line = 15.0;

        // R1 快照（P3 迁移）: 开头取一次 FM 句柄, blkx 非 null 即 READY;
        // 无 FM（未识别/加载中/MISSING）→ 保持默认告警线 15
        let fm = self.fm_manager.current();
        if let Some(b) = fm.fmdata.as_ref() {
            self.aoa_warning_line = b.no_flaps_wing.as_ref().unwrap().aoa_crit_high;
        }
    }

    /// 初始化速度告警
    fn init_speed_warnings(&mut self) {
        self.ias = WarningSlot::armed(self.new_alert(
            "warn_ias",
            VoiceAlertType::WarnIas.get_cooldown_seconds() as i64,
        ));
        self.mach = WarningSlot::armed(self.new_alert(
            "warn_mach",
            VoiceAlertType::WarnMach.get_cooldown_seconds() as i64,
        ));
        self.stall_warn = Some(self.new_alert(
            "warn_stall",
            VoiceAlertType::WarnStall.get_cooldown_seconds() as i64,
        ));

        // R1 快照（P3 迁移）: 开头取一次 FM 句柄, blkx 非 null 即 READY;
        // 无 FM → 告警线保持 MAX_VALUE（速度/马赫告警关闭）
        let fm = self.fm_manager.current();
        let b = fm.fmdata.as_ref();
        self.ias.line = 0.0;
        self.mach.line = 0.0;
        if let Some(b) = b {
            self.ias.line = b.vne * 0.95;
            self.mach.line = b.vne_mach * 0.95;
        }
        if self.ias.line == 0.0 {
            self.ias.line = f32::MAX as f64;
        }
        if self.mach.line == 0.0 {
            self.mach.line = f32::MAX as f64;
        }
    }

    /// 初始化结构告警（起落架、襟翼、过载、减速板）
    fn init_structure_warnings(&mut self) {
        self.gear = WarningSlot::armed(self.new_alert(
            "warn_gear",
            VoiceAlertType::WarnGear.get_cooldown_seconds() as i64,
        ));
        self.flap = WarningSlot::armed(self.new_alert(
            "warn_flap",
            VoiceAlertType::WarnFlap.get_cooldown_seconds() as i64,
        ));
        self.ny_warn = Some(self.new_alert(
            "warn_loadfactor",
            VoiceAlertType::WarnLoadfactor.get_cooldown_seconds() as i64,
        ));
        self.brake_warn = Some(self.new_alert(
            "warn_brake",
            VoiceAlertType::WarnBrake.get_cooldown_seconds() as i64,
        ));

        // R1 快照（P3 迁移）: 开头取一次 FM 句柄, blkx 非 null 即 READY;
        // 无 FM → 起落架限速/过载限制走默认值
        // PORT: Java 持 blkx 引用跨 tick 存活 → 按值克隆 (本类只读, 见字段注)
        let fm = self.fm_manager.current();
        let b = fm.fmdata.clone();

        // 起落架速度限制
        self.gear.line = 0.0;
        if let Some(bb) = b.as_ref() {
            self.gear.line = bb.gear_destruction_ind_speed;
        }
        if self.gear.line == 0.0 {
            self.gear.line = 450.0;
        }

        // 过载限制
        self.fmdata = b;
        self.nofuelweight = self.fmdata.as_ref().map_or(0.0, |b| b.nofuelweight);
        self.ny_warning_line0 = 0.0;
        self.ny_warning_line1 = 0.0;
        if let Some(bb) = self.fmdata.as_ref() {
            let g = bb.max_allow_gload.unwrap();
            self.ny_warning_line0 = g[0];
            self.ny_warning_line1 = g[1];
        }
        if self.ny_warning_line0 == 0.0 {
            self.ny_warning_line0 = -4.0;
        }
        if self.ny_warning_line1 == 0.0 {
            self.ny_warning_line1 = 10.0;
        }
    }

    /// 初始化引擎告警
    fn init_engine_warnings(&mut self) {
        self.eng_warn = Some(self.new_alert(
            "warn_engineoverheat",
            VoiceAlertType::WarnEngineoverheat.get_cooldown_seconds() as i64,
        ));
        self.eng_fail = WarningSlot::armed(self.new_alert(
            "fail_engine",
            VoiceAlertType::FailEngine.get_cooldown_seconds() as i64,
        ));
        // engFailInvert 使用不同的冷却时间（5秒），用于倒飞断油检测
        self.eng_fail_invert = Some(self.new_alert("fail_engine", 5));
        self.rpm_low_warn = Some(self.new_alert(
            "warn_lowrpm",
            VoiceAlertType::WarnLowrpm.get_cooldown_seconds() as i64,
        ));
        self.rpm_high_warn = Some(self.new_alert(
            "warn_highrpm",
            VoiceAlertType::WarnHighrpm.get_cooldown_seconds() as i64,
        ));
    }

    /// 初始化燃油告警
    fn init_fuel_warnings(&mut self) {
        self.fuel = WarningSlot::armed(self.new_alert(
            "warn_lowfuel",
            VoiceAlertType::WarnLowfuel.get_cooldown_seconds() as i64,
        ));
        self.fuel_prs = WarningSlot::armed(self.new_alert(
            "warn_lowpressure",
            VoiceAlertType::WarnLowpressure.get_cooldown_seconds() as i64,
        ));
        self.oof = WarningSlot::armed(self.new_alert(
            "fail_nofuel",
            VoiceAlertType::FailNofuel.get_cooldown_seconds() as i64,
        ));
        // 低油量线 (Java int 10 → f64 无损)
        self.fuel.line = 10.0;
    }

    /// 初始化高度告警
    fn init_altitude_warnings(&mut self) {
        self.height_warn = Some(self.new_alert(
            "warn_altitude",
            VoiceAlertType::WarnAltitude.get_cooldown_seconds() as i64,
        ));
        self.terrain_warn = Some(self.new_alert(
            "warn_terrain",
            VoiceAlertType::WarnTerrain.get_cooldown_seconds() as i64,
        ));
        self.vario_warn = Some(self.new_alert(
            "warn_highvario",
            VoiceAlertType::WarnHighvario.get_cooldown_seconds() as i64,
        ));
    }

    /// 初始化舵效告警
    fn init_control_effectiveness_warnings(&mut self) {
        self.rudder = WarningSlot::armed(self.new_alert(
            "rudderEff",
            VoiceAlertType::RudderEff.get_cooldown_seconds() as i64,
        ));
        self.elevator = WarningSlot::armed(self.new_alert(
            "elevatorEff",
            VoiceAlertType::ElevatorEff.get_cooldown_seconds() as i64,
        ));
        self.aileron = WarningSlot::armed(self.new_alert(
            "aileronEff",
            VoiceAlertType::AileronEff.get_cooldown_seconds() as i64,
        ));

        self.rudder.line = 65535.0;
        self.elevator.line = 65535.0;
        self.aileron.line = 65535.0;

        // R1 快照（P3 迁移）: 开头取一次 FM 句柄, blkx 非 null 即 READY;
        // 无 FM → 舵效告警线保持 65535（告警关闭）
        let fm = self.fm_manager.current();
        if let Some(b) = fm.fmdata.as_ref() {
            // Java float→int 截断语义: 先 `as i32` 再拓宽入 f64 槽位 (§2.2)
            self.rudder.line = b.rudder_eff as i32 as f64;
            self.elevator.line = b.elav_eff as i32 as f64;
            self.aileron.line = b.aileron_eff as i32 as f64;
        }
    }

    /// 初始化增压器档位告警
    fn init_compressor_warning(&mut self) {
        // 增压器档位告警由状态变化驱动，无冷却时间
        self.compressor_stage_warn = Some(self.new_alert(
            "warn_compressor",
            VoiceAlertType::WarnCompressor.get_cooldown_seconds() as i64,
        ));

        // 订阅 FlightDataBus 获取增压器档位不匹配事件
        //       FlightDataBus.getInstance().register(flightDataListener);
        // PORT: 匿名 listener → RAII Subscription (见字段注的泄漏根治说明)
        let current_mismatch = Arc::clone(&self.current_mismatch);
        let sub = self
            .flight_data_bus
            .register(move |event: &FlightDataEvent| {
                let payload = event.get_payload();
                current_mismatch.store(payload.compressor_stage_mismatch, Ordering::SeqCst);
            });
        self.flight_data_subscription = Some(sub);
    }

    /// Cleans up resources when VoiceWarning is disposed.
    ///
    /// PORT (Java bug 修复): Java 版只退订 FlightDataListener, 漏退订 UIStateBus
    /// 的 configHandler (LIFETIMES §2.1 泄漏, 见模块头) — Rust 版两个订阅都注销;
    /// 即使忘记调 dispose, Drop 也会兜底注销。
    /// 组装契约: 仅在结构被 move 进语音线程**前**可调 (见结构体注); move 后
    /// 停机走外部翻 doit + Drop 兜底, 双订阅同样被注销。
    pub fn dispose(&mut self) {
        self.doit.store(false, Ordering::SeqCst);
        if let Some(sub) = self.flight_data_subscription.take() {
            sub.unsubscribe();
        }
        // Java 版无此步 (泄漏点) — 根治补齐
        if let Some(sub) = self.config_subscription.take() {
            self.ui_state_bus
                .unsubscribe(ui_state_events::CONFIG_CHANGED, sub);
        }
    }

    // ==================== 主循环 ====================

    /// 对应 Java: `public void run()` (Runnable)。
    /// 组装契约: 调用方把 self move 进语音线程执行本方法; 停机 = 外部持 move 前
    /// 克隆的 doit Arc 翻 false (见结构体注)。
    pub fn run(&mut self) {
        // 启动延迟，使用统一异常处理
        // PORT §2.13: `ExceptionHelper.sleepQuietly(1000)` 吞中断 (提前返回、
        // 不置 doit=false、不清标志) → 睡至 1s 或停机请求到达, while 重查 doit。
        // Java 停机链事实: OverlayEntry.close 的反射 getField("doit") 因 doit 为
        // 包私有字段抛 NoSuchFieldException 被吞 (置 false 从不生效), dispose()
        // 也不被调 (非 Window) — 循环实际唯一出口是 while 内 InterruptedException
        // 腿置 doit=false 退出, 故 "收到中断 → doit 翻 false" ≡ Rust 的
        // "doit 翻 false 即中断", 退出时效等价
        sleep_while_run(&self.doit, 1000);

        while self.doit.load(Ordering::SeqCst) {
            // PORT §2.13: `Thread.sleep(sleepTime)` + InterruptedException →
            // `doit = false; break;` — 分片睡眠轮询运行标志, 提前返回即中断腿
            sleep_while_run(&self.doit, SLEEP_TIME as u64);
            if !self.doit.load(Ordering::SeqCst) {
                break; // 对齐 Java catch 块 (标志已被中断方置 false)
            }

            let mut fatal = false;
            let xs = Arc::clone(self.xs());
            let t = xs.current_time_ms();

            // 更新动态参数（可变翼等）
            // PORT: Java 的 st/indic 是 Service 内原地改写的活引用 (init 捕获一次),
            // 此处按快照架构每轮刷新 (读到的 = 本轮开头的一致视图, 见模块头)
            self.st = xs.s_state();
            self.indic = xs.s_indic();
            self.update_dynamic_parameters();

            // 执行所有告警检测，收集 fatal 状态
            // (Java `fatal |= check*()` — |= 不短路, 方法恒被调用, Rust 同)
            fatal |= self.check_aoa_warning(t);
            fatal |= self.check_speed_warning(t);
            fatal |= self.check_gear_warning(t);
            self.check_brake_warning(t);
            fatal |= self.check_flap_warning(t);
            self.check_vario_warning(t);
            self.check_engine_overheat_warning(t);
            self.check_fuel_warning(t);
            fatal |= self.check_altitude_warning(t);
            self.check_fuel_pressure_warning(t);
            self.check_inverted_flight_warning(t);
            self.check_rpm_warning(t);
            self.check_stall_warning(t);
            fatal |= self.check_load_factor_warning(t);
            self.check_control_effectiveness_warning(t);
            self.check_compressor_warning(t);

            // 更新起落架/襟翼完好性状态
            self.update_structure_health();

            xs.set_fatal_warn(fatal);
        }
    }

    // ==================== 动态参数更新 ====================

    /// 更新可变翼相关的动态参数
    /// 原位置：run() 第 439-455 行
    fn update_dynamic_parameters(&mut self) {
        let mut vwing: f64 = 0.0;
        let flaps: i32 = if self.st.flaps > 0 { self.st.flaps } else { 0 };
        // R1 快照（P3 迁移）: 每次调用开头取一次 FM 句柄（本方法在 VoiceWarning 自有
        // 线程的 run() 循环里周期执行, 纯 volatile 读）; blkx 非 null 即 READY,
        // 无 FM → 跳过动态告警线更新（沿用 init 时/上次的值）
        let fm = self.fm_manager.current();
        let b = fm.fmdata.as_ref();
        if let Some(b) = b {
            // (hud_calculator.rs 同款先例)
            if b.is_v_wing.unwrap() {
                vwing = self.indic.wsweep_indicator;
            }
            self.aoa_warning_line = b.get_aoa_high_v_wing(vwing, flaps);
            self.ias.line = b.get_vne_v_wing(vwing) * 0.95;
            self.mach.line = b.get_mne_v_wing(vwing) * 0.95;
        }
    }

    // ==================== 告警检测方法 ====================

    /// 攻角告警检测
    /// 原位置：run() 第 458-468 行
    ///
    /// @return true 如果是致命告警
    fn check_aoa_warning(&mut self, t: i64) -> bool {
        let xs = Arc::clone(self.xs());
        if !xs.player_live() || self.st.ias <= 80 {
            return false;
        }

        if self.st.aoa > self.aoa_warning_line - 1.0 {
            // 临界攻角告警
            self.alert(&self.aoa_crit, "aoaCrit").play_once(t);
            // 同时标记 aoaHigh 为已播放，防止同时触发两个告警
            // (Java 包内直写字段 → pub 原子字段的 store)
            self.alert(&self.aoa_high, "aoaHigh")
                .last_time_play
                .store(t, Ordering::SeqCst);
            self.alert(&self.aoa_high, "aoaHigh")
                .is_act
                .store(true, Ordering::SeqCst);
            return true;
        } else if self.st.aoa > self.aoa_warning_line * 0.75 {
            // 高攻角预警
            self.alert(&self.aoa_high, "aoaHigh").play_once(t);
        }
        false
    }

    /// 速度告警检测（IAS 和 Mach）
    /// 原位置：run() 第 472-480 行
    ///
    /// @return true 如果是致命告警
    fn check_speed_warning(&mut self, t: i64) -> bool {
        let mut fatal = false;

        if self.st.ias as f64 >= self.ias.line {
            self.alert(&self.ias.alert, "iasWarn").play_once(t);
            fatal = true;
        }

        if self.st.m >= self.mach.line {
            self.alert(&self.mach.alert, "machWarn").play_once(t);
            fatal = true;
        }

        fatal
    }

    /// 起落架告警检测
    /// 原位置：run() 第 484-487 行
    ///
    /// @return true 如果是致命告警
    fn check_gear_warning(&mut self, t: i64) -> bool {
        if self.is_gear_alive && (self.st.gear > 0) && self.st.ias as f64 >= self.gear.line {
            self.alert(&self.gear.alert, "gearWarn").play_once(t);
            return true;
        }
        false
    }

    /// 减速板告警检测
    /// 原位置：run() 第 490-492 行
    fn check_brake_warning(&mut self, t: i64) {
        // 起落架未放下但减速板已展开
        if self.st.gear < 100 && self.st.airbrake >= 90 {
            self.alert(&self.brake_warn, "brakeWarn").play_once(t);
        }
    }

    /// 襟翼告警检测
    /// 原位置：run() 第 495-505 行
    ///
    /// @return true 如果是致命告警
    fn check_flap_warning(&mut self, t: i64) -> bool {
        let xs = Arc::clone(self.xs());
        // 条件1: 不是正在下襟翼的状态
        let cond1 = self.is_flap_alive
            && !xs.is_downing_flap()
            && (xs.flap_allow_angle() - self.st.flaps as f64) < 2.0
            && (self.st.flaps != 0);
        // 条件2: 正在下襟翼的状态
        let cond2 = self.is_flap_alive
            && xs.is_downing_flap()
            && (xs.flap_allow_angle() - self.st.flaps as f64) < 8.0;

        if cond1 || cond2 {
            self.alert(&self.flap.alert, "flapWarn").play_once(t);
            return true;
        }
        false
    }

    /// 下降率告警检测
    /// 原位置：run() 第 508-512 行
    fn check_vario_warning(&mut self, t: i64) {
        // 起落架放下且下降率过高
        if self.is_gear_alive && self.st.gear >= 50 && self.st.vy <= -8.0 {
            self.alert(&self.vario_warn, "varioWarn").play_once(t);
        }
    }

    /// 引擎过热告警检测
    /// 原位置：run() 第 547-549 行
    fn check_engine_overheat_warning(&mut self, t: i64) {
        let xs = Arc::clone(self.xs());
        // curLoadMinWorkTime < 300 秒表示引擎即将过热
        if xs.cur_load_min_work_time() < (300 * 1000) as f64 {
            self.alert(&self.eng_warn, "engWarn").play_once(t);
        }
    }

    /// 燃油告警检测（低油量和无油）
    /// 原位置：run() 第 551-563 行
    fn check_fuel_warning(&mut self, t: i64) {
        let xs = Arc::clone(self.xs());
        // 无油告警
        if xs.total_fuel() == 0.0 {
            let old = self.oof.check;
            self.oof.check += 1;
            if old > 16 {
                self.oof.check = 0;
                self.alert(&self.oof.alert, "oofWarn").play_once(t);
            }
        }

        // 低油量告警 (line 为原 i32 阈值的 f64 无损拓宽, i32→f64 比较语义不变)
        if xs.fuel_percent() as f64 <= self.fuel.line {
            // 持续 16 tick 以上，解决退出游戏时的误告警
            let old = self.fuel.check;
            self.fuel.check += 1;
            if old > 16 {
                self.fuel.check = 0;
                self.alert(&self.fuel.alert, "fuelWarn").play_once(t);
            }
        }
    }

    /// 高度告警检测（含地形告警）
    /// 原位置：run() 第 568-581 行
    ///
    /// @return true 如果是致命告警
    fn check_altitude_warning(&mut self, t: i64) -> bool {
        let xs = Arc::clone(self.xs());
        // 起落架未放下且玩家存活
        if self.st.gear > 0 || !xs.player_live() {
            return false;
        }

        // 下降率等于高度的 10 分之一会触发警告
        // (Java `-st.heightm / 10.0f`: 一元负号先结合; 10.0f 为精确值)
        if self.st.vy < -self.st.heightm / 10.0 {
            self.alert(&self.height_warn, "heightWarn").play_once(t);
            return true;
        } else {
            // 触发高度警告优先，其次是触发地形警告
            if xs.radio_alt() > 0.0 && xs.d_radio_alt() < -xs.radio_alt() / 10.0 {
                self.alert(&self.terrain_warn, "terrainWarn").play_once(t);
                return true;
            }
        }
        false
    }

    /// 燃油压力告警检测
    /// 原位置：run() 第 583-600 行
    ///
    /// A6 (fuelPCheck 拆分): 原版"油压低"与"引擎损坏后"两腿共用一个计数器 —
    /// fp==0 且 throttle>2 时两腿同 tick 各 +100 (合计 +200), 油压低腿的 else
    /// 清零还会把损坏腿的累积一并清掉。拆分为 fuel_prs.check / eng_fail.check
    /// 两个独立计数器消除该隐形耦合; 损坏腿此后严格按自身 10 tick 计满 (原版
    /// 受油压低腿的加速/清零扰动)。损坏腿播放时清零, 避免下一轮损坏周期从
    /// 残留值起立即触发。
    fn check_fuel_pressure_warning(&mut self, t: i64) {
        // 油压过低检测
        if (self.indic.fuel_pressure >= 0.0)
            && (self.st.throttle as f64 - self.indic.fuel_pressure * 10.0) > 2.0
        {
            // 原版 Java 复合赋值 `fuelPCheck += sleepTime` (int += long) 隐式
            // 窄化 — 值域 (0,2000] 无回绕面; 收编为 i64 后窄化面消失 (§2.2)
            self.fuel_prs.check += SLEEP_TIME;
            if self.fuel_prs.check >= 2000 {
                self.fuel_prs.check = 0;
                self.alert(&self.fuel_prs.alert, "fuelPrsWarn").play_once(t);
                self.eng_damage = true;
            }
        } else {
            self.fuel_prs.check = 0;
        }

        // 引擎损坏后油压为0的告警 (独立计数, 不受油压低腿清零/加速影响)
        if self.eng_damage && self.indic.fuel_pressure == 0.0 {
            self.eng_fail.check += SLEEP_TIME;
            if self.eng_fail.check >= 1000 {
                self.eng_fail.check = 0;
                self.alert(&self.eng_fail.alert, "engFail").play_once(t);
                self.eng_damage = false;
            }
        }
    }

    /// 倒飞断油告警检测
    /// 原位置：run() 第 604-609 行
    fn check_inverted_flight_warning(&mut self, t: i64) {
        // 倒飞时油门大但推力低
        if self.st.ny < 0.0 && self.st.throttle > 50 {
            // st.thrust[0] — Java int[16] 越界 AIOOBE ≡ Rust 索引 panic (§1)
            if self.st.thrust[0] < 50 {
                self.alert(&self.eng_fail_invert, "engFailInvert")
                    .play_once(t);
            }
        }
    }

    /// 转速告警检测（低转速和高转速）
    /// 原位置：run() 第 612-628 行
    fn check_rpm_warning(&mut self, t: i64) {
        let xs = Arc::clone(self.xs());
        // 倒飞时不检测转速
        if self.st.ny < 0.0 && self.st.throttle > 50 && self.st.thrust[0] < 50 {
            return;
        }

        // 定距桨特殊处理：不是喷气但桨距无效
        if !(!xs.is_eng_jet() && self.st.rpm_throttle < 0) {
            // 时提升 (§2.12: f32 中间量显式保持)
            if (self.st.throttle - 30) as f64
                > (self.st.rpm as f32 * 100.0f32) as f64 / xs.maximum_thr_rpm()
            {
                // 转速低
                self.alert(&self.rpm_low_warn, "rpmLowWarn").play_once(t);
            }
        }

        // 高转速告警
        if xs.get_maximum_rpm()
            && xs.maximum_thr_rpm() > 0.0
            && (self.st.rpm as f32 * 100.0f32) as f64 / xs.maximum_thr_rpm() >= 105.0
        {
            self.alert(&self.rpm_high_warn, "rpmHighWarn").play_once(t);
        }
    }

    /// 失速告警检测
    /// 原位置：run() 第 632-634 行
    fn check_stall_warning(&mut self, t: i64) {
        let xs = Arc::clone(self.xs());
        // 没放下起落架、有下降率、速度低于失速速度
        if xs.player_live()
            && self.st.gear == 0
            && self.st.vy != 0.0
            && xs.get_stall_speed() != 0.0
            && self.st.ias as f64 <= xs.get_stall_speed()
        {
            self.alert(&self.stall_warn, "stallWarn").play_once(t);
        }
    }

    /// 过载告警检测
    /// 原位置：run() 第 637-651 行
    ///
    /// @return true 如果是致命告警
    fn check_load_factor_warning(&mut self, t: i64) -> bool {
        // 使用动态阈值
        let mut current_ny_min = self.ny_warning_line0;
        let mut current_ny_max = self.ny_warning_line1;

        if self
            .fmdata
            .as_ref()
            .is_some_and(|b| b.raw_wing_crit_overload.is_some())
            && self.nofuelweight > 0.0
        {
            let current_weight = self.nofuelweight + self.st.mfuel;
            // rawWingCritOverload != null 且 nofuelweight > 0, currentWeight =
            // nofuelweight + mfuel 仍可 <= 0 (mfuel 可为负值/哨兵), 此时走回退
            // 分支返回 maxAllowGload (可为 null) → Java 在 dynamicLimits[0] 处
            // NPE; 此处 None ≡ 该 NPE, panic 对位
            let dynamic_limits = self
                .fmdata
                .as_ref()
                .unwrap()
                .get_max_allow_gload_for_weight(current_weight)
                .expect("回退分支 maxAllowGload 可为 null — None panic ≡ Java dynamicLimits[0] 的 NPE (currentWeight<=0 时可达)");
            current_ny_min = dynamic_limits[0];
            current_ny_max = dynamic_limits[1];
        }

        let xs = Arc::clone(self.xs());
        if xs.player_live() && (self.st.ny > current_ny_max || self.st.ny < current_ny_min) {
            self.alert(&self.ny_warn, "nyWarn").play_once(t);
            return true;
        }
        false
    }

    /// 舵效告警检测（副翼和方向舵）
    /// 原位置：run() 第 653-669 行
    fn check_control_effectiveness_warning(&mut self, t: i64) {
        // 副翼舵效 (check 复用为 0/1 边沿锁存: 越线首帧播放, 持续越线不重播)
        if self.st.ias as f64 >= self.aileron.line {
            if self.aileron.check == 0 {
                self.alert(&self.aileron.alert, "aileronEff").play_once(t);
            }
            self.aileron.check = 1;
        } else {
            self.aileron.check = 0;
        }

        // 方向舵舵效
        if self.st.ias as f64 >= self.rudder.line {
            if self.rudder.check == 0 {
                self.alert(&self.rudder.alert, "rudderEff").play_once(t);
            }
            self.rudder.check = 1;
        } else {
            self.rudder.check = 0;
        }
    }

    /// 增压器档位不匹配告警检测
    /// 原位置：run() 第 671-690 行
    fn check_compressor_warning(&mut self, t: i64) {
        let is_mismatch = self.current_mismatch.load(Ordering::SeqCst); // Read volatile once

        // Detect state change (false→true or true→false)
        if is_mismatch != self.last_mismatch {
            if is_mismatch {
                // false → true: 不一致了，启动 3 秒定时器
                self.pending_compressor_warn_time = t + COMPRESSOR_WARN_DELAY;
            } else {
                // true → false: 一致了，取消定时器
                self.pending_compressor_warn_time = 0;
            }
            self.last_mismatch = is_mismatch;
        }

        // Check if it's time to play the warning
        if self.pending_compressor_warn_time > 0 && t >= self.pending_compressor_warn_time {
            self.alert(&self.compressor_stage_warn, "compressorStageWarn")
                .play_once(t);
            // 每 3 秒重复告警直到不匹配状态解除
            self.pending_compressor_warn_time = t + COMPRESSOR_WARN_DELAY;
        }
    }

    // ==================== 结构完好性更新 ====================

    /// 更新起落架和襟翼的完好性状态
    /// 原位置：run() 第 514-543 行
    fn update_structure_health(&mut self) {
        let xs = Arc::clone(self.xs());
        // 起落架完好性判断
        if self.is_gear_alive && self.st.ias as f64 > self.gear.line {
            // 超速持续 10 秒后标记为损坏
            self.gear.check += SLEEP_TIME;
            if self.gear.check >= GEAR_DAMAGE_THRESHOLD_MS {
                self.gear.check = 0;
                self.is_gear_alive = false;
            }
        } else {
            self.gear.check = 0;
        }

        // 襟翼完好性判断
        if self.is_flap_alive && self.st.ias as f64 > xs.flap_allow_speed() {
            // 超速持续 15 秒后标记为损坏
            self.flap.check += SLEEP_TIME;
            if self.flap.check >= FLAP_DAMAGE_THRESHOLD_MS {
                self.flap.check = 0;
                self.is_flap_alive = false;
            }
        } else {
            self.flap.check = 0;
        }

        // 收起后恢复
        if !self.is_gear_alive && self.st.gear == 0 {
            self.is_gear_alive = true;
        }
        if !self.is_flap_alive && self.st.flaps == 0 {
            self.is_flap_alive = true;
        }
    }
}

/// fire-and-forget 播放的 clip 保活持有 (审查 B-B1 修复)。
///
/// Java 形态 (试听 / playWav): clip 局部引用
/// start() 后出作用域, 原生 line 靠 GC finalizer **非确定性延迟释放**而自然
/// 播完。Rust 的确定性 Drop (SoundClip RAII 兜底契约: Drop 等价 close →
/// winmm waveOutReset+waveOutClose 立即停) 与 fire-and-forget 消费形态直接
/// 冲突 — 调用点提交即被掐断 (试听无声)。
///
/// 对位 GC 延迟语义: 起后台线程持 clip 轮询 `is_running()` 至播完再释放;
/// 60s 上限兜底 (语音包告警音最长数秒, 防异常滞留泄漏设备句柄)。
/// **不适用**告警路径 (VoiceAlert 持有 clip 至 reload/close, 生命周期由
/// VoiceWarning 管理, Drop 契约是防原生句柄泄漏的正向依赖)。
/// 返回 JoinHandle 供测试同步; 生产调用点忽略。
pub fn hold_clip_until_done(clip: Box<dyn SoundClip>) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("VoicePreview".to_string())
        .spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(60);
            while clip.is_running() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
            clip.close(); // 显式释放 (Drop 兜底幂等, Java close 状态机)
        })
        .expect("VoicePreview 保活线程创建失败")
}

// =====================================================================
// Tests — Java 侧无对应单测 (VoiceWarning 手动验证), 本组为 B 类行为钉子:
// mock SoundPlayer + 总线注入, 断言 订阅→检测→播放 触发链 (PORTING §5.3)。
// =====================================================================
#[cfg(test)]
mod tests;
