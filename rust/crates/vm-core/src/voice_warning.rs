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
//!   (VoiceWarning.java:233/311, 全文件无读取点), init* 的 c 参数无消费 —
//!   不落字段/参数; configProvider 在 Java 经 `c.getConfigService()` 取得,
//!   Rust 由构造注入。
//!
//! PORT (Java 现存泄漏根治, LIFETIMES §2.1/§6.3.1 — 本文件主任务):
//! Java 版两个订阅在生产路径均泄漏: (1) dispose() (VoiceWarning.java:506) 本身
//! 只退订 FlightDataListener, 漏退订 UIStateBus configHandler (332 订阅无对应
//! unsubscribe); (2) 该 dispose() 根本无人调用 — VoiceWarning 非
//! java.awt.Window, OverlayEntry.close 的 `instance instanceof Window`
//! (OverlayManager.java:362) 为 false。每次进出游戏模式 new 一个 VoiceWarning,
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
//! (Service.java:1687-1689); Rust 快照架构 (vm-data ServiceData) 下由
//! [`VoiceWarningService::s_state`]/`s_indic`] 返回当前副本, run() 每轮循环
//! 开头刷新 — 与 Java "check 方法执行时读到最新遥测" 语义同类 (快照粒度 =
//! 一个 tick, Java 逐字段读的竞态窗口同样存在且无同步)。
//!
//! PORT (Java f 后缀字面量): `0.95f` 的 f32 值 ≠ f64 字面量 0.95, 必须写
//! `0.95f32 as f64` (init/updateDynamicParameters 的告警线乘 0.95f 处);
//! 其余 f 后缀字面量 (2.0f/8.0f/10.0f/0.75f/-8) 均为二进制精确值, f32→f64
//! 与同名 f64 字面量逐位相等, 直接写 f64 字面量。

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::audio::voice_pack_config::VOICE_PREFIX;
use crate::audio::{VoiceAlertType, VoicePackConfig};
use crate::blkx::Blkx;
use crate::bus::Subscription;
use crate::config_api::ConfigProvider;
use crate::event::flight_data_event::FlightDataEvent;
use crate::event::ui_state_events;
use crate::flight_data_bus::FlightDataBus;
use crate::fm::FMManager;
use crate::logger;
use crate::parser::{Indicators, State};
use crate::ui_state_bus::{UIStateBus, UiStateEvent};
use crate::voice_resource_manager::{SoundClip, SoundPlayer, VoiceResourceManager};

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
            // Java: this.coolDownMs = coolDownSeconds * 1000;
            cool_down_ms: cool_down_seconds * 1000,
            key: key.to_string(),
        }
    }

    /// 重新加载音频资源（线程安全）
    /// 必须同步以防止多个线程同时 reload
    ///
    /// PORT: `synchronized` 整方法 monitor → 持 clip 锁贯穿方法体 (临界区等价);
    /// 外层 configProvider/VoiceResourceManager 单例取用 → 参数注入。
    pub fn reload(&self, config_provider: &dyn ConfigProvider, resource_manager: &VoiceResourceManager) {
        let mut clip_slot = self.clip.lock().expect(CLIP_LOCK_MSG);

        // 先关闭旧资源
        if let Some(old_clip) = clip_slot.as_ref() {
            // Java: try { if (oldClip.isRunning()) oldClip.stop(); oldClip.close(); }
            //      catch (Exception e) { Logger.warn("VoiceAlert", "关闭旧 Clip 失败: " + key); }
            // PORT: SoundClip 面不可失败 (D7), catch 腿不可达
            if old_clip.is_running() {
                old_clip.stop();
            }
            old_clip.close();
        }

        // 解析配置
        // Java: String configKey = VoicePackConfig.withVoicePrefix(key); — key 非 null
        let config_key = VoicePackConfig::with_voice_prefix(Some(&self.key)).unwrap();
        let val = config_provider.get_config(&config_key);
        let config = VoicePackConfig::parse(val.as_deref());

        if !config.enabled {
            self.available.store(false, Ordering::SeqCst);
            *clip_slot = None;
            return;
        }

        // 加载新 Clip
        // Java: this.clip = VoiceResourceManager.getInstance().loadClip(key, config.packName);
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

        // Java: try { c.setFramePosition(0); c.start(); }
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
            // Java: try { ... } catch (Exception e) { // ignore }
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
    /// Java: `private boolean playCompleted` (仅 getClip 写入, 无读取点)
    #[allow(dead_code)]
    play_completed: bool,

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
    /// playWav/getClip 的 Java `AudioSystem` 直开面 (不经 VoiceResourceManager;
    /// 两方法全库无调用方, legacy 保真) — 与 resource_manager 注入同一实现即等价
    legacy_player: Arc<dyn SoundPlayer>,

    pub aoa_warning_line: f64,
    pub ias_warning_line: f64,
    pub mach_warning_line: f64,

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

    // 速度相关
    ias_warn: Option<Arc<VoiceAlert>>,
    mach_warn: Option<Arc<VoiceAlert>>,
    stall_warn: Option<Arc<VoiceAlert>>,

    // 起落架/襟翼/减速板
    gear_warning_line: f64,
    gear_warn: Option<Arc<VoiceAlert>>,
    flap_warn: Option<Arc<VoiceAlert>>,
    brake_warn: Option<Arc<VoiceAlert>>,
    is_gear_alive: bool,
    is_flap_alive: bool,
    gear_check: i64,
    flap_check: i64,

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
    blkx: Option<Blkx>,
    nofuelweight: f64,

    // 引擎相关
    eng_warn: Option<Arc<VoiceAlert>>,
    eng_fail: Option<Arc<VoiceAlert>>,
    eng_fail_invert: Option<Arc<VoiceAlert>>,
    rpm_low_warn: Option<Arc<VoiceAlert>>,
    rpm_high_warn: Option<Arc<VoiceAlert>>,
    pub eng_damage: bool,

    // 燃油相关
    lowfuel_warning_line: i32,
    fuel_warn: Option<Arc<VoiceAlert>>,
    fuel_prs_warn: Option<Arc<VoiceAlert>>,
    oof_warn: Option<Arc<VoiceAlert>>,
    fuel_check: i64,
    fuel_p_check: i32,
    oof_check: i32,

    // 高度相关
    height_warn: Option<Arc<VoiceAlert>>,
    terrain_warn: Option<Arc<VoiceAlert>>,
    vario_warn: Option<Arc<VoiceAlert>>,

    // 舵效相关
    rudder_eff_ias: i32,
    elevator_eff_ias: i32,
    aileron_eff_ias: i32,
    rudder_eff: Option<Arc<VoiceAlert>>,
    elevator_eff: Option<Arc<VoiceAlert>>,
    aileron_eff: Option<Arc<VoiceAlert>>,
    /// Java 声明带 `= false` 初始化器; 原版只写不读 (elevator 告警触发块在 Java
    /// 源码中即缺失, 保真保留写点) — §2.10 按有意处理
    #[allow(dead_code)]
    elevator_eff_check: bool,
    aileron_eff_check: bool,
    rudder_eff_check: bool,

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
            play_completed: false,
            config_provider,
            resource_manager,
            fm_manager,
            ui_state_bus,
            flight_data_bus,
            legacy_player,
            aoa_warning_line: 0.0,
            ias_warning_line: 0.0,
            mach_warning_line: 0.0,
            alerts: Arc::new(Mutex::new(HashMap::new())),
            config_subscription: None,
            aoa_crit: None,
            aoa_high: None,
            ias_warn: None,
            mach_warn: None,
            stall_warn: None,
            gear_warning_line: 0.0,
            gear_warn: None,
            flap_warn: None,
            brake_warn: None,
            is_gear_alive: false,
            is_flap_alive: false,
            gear_check: 0,
            flap_check: 0,
            ny_warning_line0: 0.0,
            ny_warning_line1: 0.0,
            ny_warn: None,
            blkx: None,
            nofuelweight: 0.0,
            eng_warn: None,
            eng_fail: None,
            eng_fail_invert: None,
            rpm_low_warn: None,
            rpm_high_warn: None,
            eng_damage: false,
            lowfuel_warning_line: 0,
            fuel_warn: None,
            fuel_prs_warn: None,
            oof_warn: None,
            fuel_check: 0,
            fuel_p_check: 0,
            oof_check: 0,
            height_warn: None,
            terrain_warn: None,
            vario_warn: None,
            rudder_eff_ias: 0,
            elevator_eff_ias: 0,
            aileron_eff_ias: 0,
            rudder_eff: None,
            elevator_eff: None,
            aileron_eff: None,
            elevator_eff_check: false,
            aileron_eff_check: false,
            rudder_eff_check: false,
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
        // Java: alerts.put(key, this) — 同 key 后写覆盖 (engFail/engFailInvert
        // 共用 "fail_engine", map 最终指向后注册的 invert 实例, 保真)
        self.alerts
            .lock()
            .expect(ALERTS_LOCK_MSG)
            .insert(key.to_string(), Arc::clone(&alert));
        // Java: 构造器尾部的 reload()
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
        // Java: this.configProvider = c.getConfigService(); — Rust 构造注入

        // PORT: Java 捕获活引用; Rust 取首帧快照 (run 每轮再刷新)
        self.st = s.s_state();
        self.indic = s.s_indic();

        // Config Listener - 使用 VoicePackConfig 工具方法
        if self.config_subscription.is_none() {
            // Java: configHandler = key -> { ... };
            //       UIStateBus.getInstance().subscribe(CONFIG_CHANGED, configHandler);
            let alerts = Arc::clone(&self.alerts);
            let config_provider = Arc::clone(&self.config_provider);
            let resource_manager = Arc::clone(&self.resource_manager);
            self.config_subscription = Some(self.ui_state_bus.subscribe(
                ui_state_events::CONFIG_CHANGED,
                move |msg: &UiStateEvent| {
                    // Java: if (key instanceof String) — CONFIG_CHANGED 载荷 = String 配置键
                    if let Some(str_key) = msg.data.as_deref() {
                        if str_key.starts_with(VOICE_PREFIX) {
                            // Java: String alertKey = VoicePackConfig.stripVoicePrefix(strKey);
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
        // Java: VoiceAlert startSound = new VoiceAlert("start1", 1);
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
        if let Some(b) = fm.blkx.as_ref() {
            // Java: b.NoFlapsWing.AoACritHigh — NoFlapsWing null 即 NPE ≡ unwrap panic
            self.aoa_warning_line = b.no_flaps_wing.as_ref().unwrap().aoa_crit_high;
        }
    }

    /// 初始化速度告警
    fn init_speed_warnings(&mut self) {
        self.ias_warn = Some(self.new_alert(
            "warn_ias",
            VoiceAlertType::WarnIas.get_cooldown_seconds() as i64,
        ));
        self.mach_warn = Some(self.new_alert(
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
        let b = fm.blkx.as_ref();
        self.ias_warning_line = 0.0;
        self.mach_warning_line = 0.0;
        if let Some(b) = b {
            // Java: b.vne * 0.95f — 0.95f 的 f32 值 ≠ f64 0.95, 显式转换 (§2.12)
            self.ias_warning_line = b.vne * 0.95f32 as f64;
            self.mach_warning_line = b.vne_mach * 0.95f32 as f64;
        }
        // Java: Float.MAX_VALUE 赋给 double 字段 (拓宽)
        if self.ias_warning_line == 0.0 {
            self.ias_warning_line = f32::MAX as f64;
        }
        if self.mach_warning_line == 0.0 {
            self.mach_warning_line = f32::MAX as f64;
        }
    }

    /// 初始化结构告警（起落架、襟翼、过载、减速板）
    fn init_structure_warnings(&mut self) {
        self.gear_warn = Some(self.new_alert(
            "warn_gear",
            VoiceAlertType::WarnGear.get_cooldown_seconds() as i64,
        ));
        self.flap_warn = Some(self.new_alert(
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
        let b = fm.blkx.clone();

        // 起落架速度限制
        self.gear_warning_line = 0.0;
        if let Some(bb) = b.as_ref() {
            self.gear_warning_line = bb.gear_destruction_ind_speed;
        }
        if self.gear_warning_line == 0.0 {
            self.gear_warning_line = 450.0;
        }

        // 过载限制
        self.blkx = b;
        self.nofuelweight = self.blkx.as_ref().map_or(0.0, |b| b.nofuelweight);
        self.ny_warning_line0 = 0.0;
        self.ny_warning_line1 = 0.0;
        if let Some(bb) = self.blkx.as_ref() {
            // Java: b.maxAllowGload[0] — 数组 null 即 NPE ≡ unwrap panic
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
        self.eng_fail = Some(self.new_alert(
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
        self.lowfuel_warning_line = 10;
        self.fuel_warn = Some(self.new_alert(
            "warn_lowfuel",
            VoiceAlertType::WarnLowfuel.get_cooldown_seconds() as i64,
        ));
        self.fuel_prs_warn = Some(self.new_alert(
            "warn_lowpressure",
            VoiceAlertType::WarnLowpressure.get_cooldown_seconds() as i64,
        ));
        self.oof_warn = Some(self.new_alert(
            "fail_nofuel",
            VoiceAlertType::FailNofuel.get_cooldown_seconds() as i64,
        ));
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
        self.rudder_eff = Some(self.new_alert(
            "rudderEff",
            VoiceAlertType::RudderEff.get_cooldown_seconds() as i64,
        ));
        self.elevator_eff = Some(self.new_alert(
            "elevatorEff",
            VoiceAlertType::ElevatorEff.get_cooldown_seconds() as i64,
        ));
        self.aileron_eff = Some(self.new_alert(
            "aileronEff",
            VoiceAlertType::AileronEff.get_cooldown_seconds() as i64,
        ));

        self.rudder_eff_ias = 65535;
        self.elevator_eff_ias = 65535;
        self.aileron_eff_ias = 65535;

        // R1 快照（P3 迁移）: 开头取一次 FM 句柄, blkx 非 null 即 READY;
        // 无 FM → 舵效告警线保持 65535（告警关闭）
        let fm = self.fm_manager.current();
        if let Some(b) = fm.blkx.as_ref() {
            // Java: (int) b.rudderEff — JLS 5.1.3 double→int (NaN→0/越界饱和)
            // 与 Rust `as i32` 逐位同语义, 无需双转 (§2.2)
            self.rudder_eff_ias = b.rudder_eff as i32;
            self.elevator_eff_ias = b.elav_eff as i32;
            self.aileron_eff_ias = b.aileron_eff as i32;
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
        // Java: flightDataListener = new FlightDataListener() {...};
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
            // Java: FlightDataBus.getInstance().unregister(flightDataListener);
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
        let b = fm.blkx.as_ref();
        if let Some(b) = b {
            // Java: if (b.isVWing) — Boolean 拆箱 null→NPE ≡ unwrap panic
            // (hud_calculator.rs 同款先例)
            if b.is_v_wing.unwrap() {
                vwing = self.indic.wsweep_indicator;
            }
            self.aoa_warning_line = b.get_aoa_high_v_wing(vwing, flaps);
            // Java: * 0.95f — f32 字面量值, 显式转换 (§2.12)
            self.ias_warning_line = b.get_vne_v_wing(vwing) * 0.95f32 as f64;
            self.mach_warning_line = b.get_mne_v_wing(vwing) * 0.95f32 as f64;
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

        if self.st.ias as f64 >= self.ias_warning_line {
            self.alert(&self.ias_warn, "iasWarn").play_once(t);
            fatal = true;
        }

        if self.st.m >= self.mach_warning_line {
            self.alert(&self.mach_warn, "machWarn").play_once(t);
            fatal = true;
        }

        fatal
    }

    /// 起落架告警检测
    /// 原位置：run() 第 484-487 行
    ///
    /// @return true 如果是致命告警
    fn check_gear_warning(&mut self, t: i64) -> bool {
        if self.is_gear_alive
            && (self.st.gear > 0)
            && self.st.ias as f64 >= self.gear_warning_line
        {
            self.alert(&self.gear_warn, "gearWarn").play_once(t);
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
            self.alert(&self.flap_warn, "flapWarn").play_once(t);
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
            // Java: if (oofCheck++ > 16) — 后自增: 先比旧值, 计数恒 +1
            let old = self.oof_check;
            self.oof_check += 1;
            if old > 16 {
                self.oof_check = 0;
                self.alert(&self.oof_warn, "oofWarn").play_once(t);
            }
        }

        // 低油量告警
        if xs.fuel_percent() <= self.lowfuel_warning_line {
            // 持续 16 tick 以上，解决退出游戏时的误告警
            let old = self.fuel_check;
            self.fuel_check += 1;
            if old > 16 {
                self.fuel_check = 0;
                self.alert(&self.fuel_warn, "fuelWarn").play_once(t);
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
    fn check_fuel_pressure_warning(&mut self, t: i64) {
        // 油压过低检测
        if (self.indic.fuel_pressure >= 0.0)
            && (self.st.throttle as f64 - self.indic.fuel_pressure * 10.0) > 2.0
        {
            // PORT: Java 复合赋值 `fuelPCheck += sleepTime` (int += long) 隐式窄化
            // 强转 — 值域 (0,2000] 内与直接加等价, 无回绕面 (§2.2)
            self.fuel_p_check += SLEEP_TIME as i32;
            if self.fuel_p_check >= 2000 {
                self.fuel_p_check = 0;
                self.alert(&self.fuel_prs_warn, "fuelPrsWarn").play_once(t);
                self.eng_damage = true;
            }
        } else {
            self.fuel_p_check = 0;
        }

        // 引擎损坏后油压为0的告警
        if self.eng_damage && self.indic.fuel_pressure == 0.0 {
            self.fuel_p_check += SLEEP_TIME as i32;
            if self.fuel_p_check >= 1000 {
                self.alert(&self.eng_fail, "engFail").play_once(t);
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
            // Java: st.RPM * 100.0f — int * float 为 float 算术链, 再除 double
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
            .blkx
            .as_ref()
            .is_some_and(|b| b.raw_wing_crit_overload.is_some())
            && self.nofuelweight > 0.0
        {
            let current_weight = self.nofuelweight + self.st.mfuel;
            // Java: blkx.getMaxAllowGloadForWeight(currentWeight) — 守卫只查
            // rawWingCritOverload != null 且 nofuelweight > 0, currentWeight =
            // nofuelweight + mfuel 仍可 <= 0 (mfuel 可为负值/哨兵), 此时走回退
            // 分支返回 maxAllowGload (可为 null) → Java 在 dynamicLimits[0] 处
            // NPE (Blkx.java:809-810); 此处 None ≡ 该 NPE, panic 对位
            let dynamic_limits = self
                .blkx
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
        // 副翼舵效
        if self.st.ias >= self.aileron_eff_ias {
            if !self.aileron_eff_check {
                self.alert(&self.aileron_eff, "aileronEff").play_once(t);
            }
            self.aileron_eff_check = true;
        } else {
            self.aileron_eff_check = false;
        }

        // 方向舵舵效
        if self.st.ias >= self.rudder_eff_ias {
            if !self.rudder_eff_check {
                self.alert(&self.rudder_eff, "rudderEff").play_once(t);
            }
            self.rudder_eff_check = true;
        } else {
            self.rudder_eff_check = false;
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
        if self.is_gear_alive && self.st.ias as f64 > self.gear_warning_line {
            // 超速持续 10 秒后标记为损坏
            self.gear_check += SLEEP_TIME;
            if self.gear_check >= GEAR_DAMAGE_THRESHOLD_MS {
                self.gear_check = 0;
                self.is_gear_alive = false;
            }
        } else {
            self.gear_check = 0;
        }

        // 襟翼完好性判断
        if self.is_flap_alive && self.st.ias as f64 > xs.flap_allow_speed() {
            // 超速持续 15 秒后标记为损坏
            self.flap_check += SLEEP_TIME;
            if self.flap_check >= FLAP_DAMAGE_THRESHOLD_MS {
                self.flap_check = 0;
                self.is_flap_alive = false;
            }
        } else {
            self.flap_check = 0;
        }

        // 收起后恢复
        if !self.is_gear_alive && self.st.gear == 0 {
            self.is_gear_alive = true;
        }
        if !self.is_flap_alive && self.st.flaps == 0 {
            self.is_flap_alive = true;
        }
    }

    // ==================== Legacy 直播面 (全库无调用方, 保真翻译) ====================

    /// Java: `public void playWav(String Path)` — Legacy tool for direct playWav
    pub fn play_wav(&self, path: &str) {
        // Java: AudioSystem.getAudioInputStream + getLine + open + start;
        //      catch (Exception e) { e.printStackTrace(); }
        // PORT: AudioSystem 直开面 → legacy_player 注入 (与 resource_manager 共用
        // 同一实现即等价); 异常腿收敛为 Err 分支, printStackTrace 收窄为 DEBUG
        // 闸门 stderr (voice_resource_manager.rs 同款先例)
        match self.legacy_player.open_clip(Path::new(path)) {
            Ok(audio_clip) => {
                // Java: audioClip.start();
                audio_clip.start();
            }
            Err(e) => {
                if logger::get_level().value() <= logger::Level::Debug.value() {
                    let _ = writeln!(std::io::stderr().lock(), "playWav: {e:?}");
                }
            }
        }
    }

    /// Java: `Clip getClip(String audioFilePath)` — Legacy method for backward
    /// compatibility (package-private, 全库无调用方 — 仅本文件测试消费)
    #[allow(dead_code)]
    fn get_clip(&mut self, audio_file_path: &str) -> Option<Box<dyn SoundClip>> {
        let audio_file = PathBuf::from(audio_file_path);
        self.play_completed = false;
        // Java: AudioSystem.getAudioInputStream/getLine/open + 三类异常各自
        // debugPrint 不同文案 (UnsupportedAudioFile/LineUnavailable/IOException);
        // PORT: SoundPlayer 注入面 (D7) 将三类异常收敛为单一 Err, 差异化文案
        // 不可复刻 — 取 IOException 腿文案 ("Error playing the audio file.")
        match self.legacy_player.open_clip(&audio_file) {
            Ok(audio_clip) => Some(audio_clip),
            Err(e) => {
                // Java: Application.debugPrint("Error playing the audio file.")
                //      = Logger.info("Legacy", t) (flight_analyzer.rs 先例)
                logger::info("Legacy", "Error playing the audio file.");
                if logger::get_level().value() <= logger::Level::Debug.value() {
                    let _ = writeln!(std::io::stderr().lock(), "{e:?}");
                }
                None // Java: audioClip 保持 null 返回
            }
        }
    }
}

/// §2.13 辅助: 睡眠至 deadline 或运行标志翻 false (Java `Thread.sleep` 被
/// interrupt 打断的对位 — 本类线程停机链: OverlayEntry.close 反射置 doit 因
/// 包私有字段 getField 失败从不生效, 唯一有效停机是 thread.interrupt() →
/// while 内 InterruptedException 腿置 doit=false, 故 "标志翻 false" ≡
/// "收到中断", 退出时效等价)。
/// 角落偏差: 进入时 doit 已 false 则立即返回; Java 的 sleepQuietly(1000) 是
/// 无条件睡眠 (init(None)/已停机角落会残留 1 秒线程) — 无 join 方, 外部行为
/// 无差异。
/// PORT: [`crate::exception_helper::sleep_quietly`] 的 stop 语义是 **true=停**,
/// 本类 doit 是 **true=运行**, 极性相反且 AtomicBool 无反视图可复用, 就地写
/// 翻转版 (§6 跨文件观察, 只上报不越文件修; 双审查复核属实):
/// other_service.rs:366 与 flight_log.rs:926 把 true=运行 的标志 (is_run/
/// logon) 直接传给了 stop 语义的 sleep_quietly (立即返回 → 运行期热自旋),
/// 请主 agent 裁决修那两处调用点 (或加 run 极性辅助函数)。
fn sleep_while_run(run: &AtomicBool, millis: u64) {
    let deadline = Instant::now() + Duration::from_millis(millis);
    while run.load(Ordering::SeqCst) {
        let now = Instant::now();
        if now >= deadline {
            return;
        }
        let chunk = std::cmp::min(deadline - now, Duration::from_millis(10));
        std::thread::sleep(chunk);
    }
}

// =====================================================================
// Tests — Java 侧无对应单测 (VoiceWarning 手动验证), 本组为 B 类行为钉子:
// mock SoundPlayer + 总线注入, 断言 订阅→检测→播放 触发链 (PORTING §5.3)。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::voice_alert_type;
    use crate::bus::EventBus;
    use crate::event::event_payload::EventPayload;
    use crate::voice_resource_manager::SoundError;
    use std::sync::atomic::AtomicUsize;

    static DIR_N: AtomicUsize = AtomicUsize::new(0);

    /// 每测试独立临时 voice 根目录 (voice_resource_manager 测试先例)
    fn tmp_dir() -> PathBuf {
        let n = DIR_N.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("vm_core_vw_{}_{n}", std::process::id()))
    }

    /// State 逐字段快照 (State 未 derive Clone; vm-data service_loop.rs 的
    /// snapshot_state 同款逐字段副本, 本文件内自备 mock 用)
    fn snapshot_state(s: &State) -> State {
        State {
            valid: s.valid.clone(),
            flag: s.flag,
            engine_num: s.engine_num,
            aileron: s.aileron,
            elevator: s.elevator,
            rudder: s.rudder,
            flaps: s.flaps,
            gear: s.gear,
            tas: s.tas,
            ias: s.ias,
            m: s.m,
            aoa: s.aoa,
            heightm: s.heightm,
            aos: s.aos,
            ny: s.ny,
            vy: s.vy,
            wx: s.wx,
            throttle: s.throttle,
            rpm_throttle: s.rpm_throttle,
            radiator: s.radiator,
            oilradiator: s.oilradiator,
            mixture: s.mixture,
            compressorstage: s.compressorstage,
            magenato: s.magenato,
            power: s.power.clone(),
            rpm: s.rpm,
            manifoldpressure: s.manifoldpressure,
            watertemp: s.watertemp,
            oiltemp: s.oiltemp,
            mfuel: s.mfuel,
            mfuel_1: s.mfuel_1,
            mfuel0: s.mfuel0,
            mfuel0_1: s.mfuel0_1,
            pitch: s.pitch.clone(),
            thrust: s.thrust.clone(),
            efficiency: s.efficiency.clone(),
            airbrake: s.airbrake,
            total_thr: s.total_thr,
            throttles: s.throttles.clone(),
        }
    }

    /// Indicators 逐字段快照 (army 为 parser 模块私有, 经 new() 落默认值 —
    /// VoiceWarning 无 army 读者; vm-data snapshot_indicators 同款形状)
    fn snapshot_indicators(i: &Indicators) -> Indicators {
        let mut s = Indicators::new();
        s.valid = i.valid.clone();
        s.r#type = i.r#type.clone();
        s.stype = i.stype.clone();
        s.flag = i.flag;
        s.speed = i.speed;
        s.pedals = i.pedals;
        s.stick_elevator = i.stick_elevator;
        s.stick_ailerons = i.stick_ailerons;
        s.altitude_hour = i.altitude_hour;
        s.altitude_min = i.altitude_min;
        s.altitude_10k = i.altitude_10k;
        s.bank = i.bank;
        s.turn = i.turn;
        s.compass = i.compass;
        s.clock_hour = i.clock_hour;
        s.clock_min = i.clock_min;
        s.clock_sec = i.clock_sec;
        s.manifold_pressure = i.manifold_pressure;
        s.rpm = i.rpm;
        s.oil_pressure = i.oil_pressure;
        s.water_temperature = i.water_temperature;
        s.engine_temperature = i.engine_temperature;
        s.mixture = i.mixture;
        s.fuel = i.fuel;
        s.fuel_pressure = i.fuel_pressure;
        s.oxygen = i.oxygen;
        s.gears_lamp = i.gears_lamp;
        s.flaps = i.flaps;
        s.trimmer = i.trimmer;
        s.throttle = i.throttle;
        s.weapon1 = i.weapon1;
        s.weapon2 = i.weapon2;
        s.weapon3 = i.weapon3;
        s.prop_pitch_hour = i.prop_pitch_hour;
        s.prop_pitch_min = i.prop_pitch_min;
        s.ammo_counter1 = i.ammo_counter1;
        s.ammo_counter2 = i.ammo_counter2;
        s.ammo_counter3 = i.ammo_counter3;
        s.oil_temp = i.oil_temp;
        s.water_temp = i.water_temp;
        s.fuelnum = i.fuelnum;
        s.vario = i.vario;
        s.aviahorizon_pitch = i.aviahorizon_pitch;
        s.aviahorizon_roll = i.aviahorizon_roll;
        s.wsweep_indicator = i.wsweep_indicator;
        s.radio_altitude = i.radio_altitude;
        s.mach = i.mach;
        s
    }

    /// 播放日志中指定 key 的 start 次数
    fn starts(log: &Mutex<Vec<String>>, key: &str) -> usize {
        let target = format!("{key}:start");
        log.lock().unwrap().iter().filter(|s| s.as_str() == target).count()
    }

    // ---- mock SoundClip/SoundPlayer (D7 trait 的测试替身) ----

    struct MockClip {
        key: String,
        log: Arc<Mutex<Vec<String>>>,
        running: Arc<AtomicBool>,
    }

    impl SoundClip for MockClip {
        fn start(&self) {
            self.log.lock().unwrap().push(format!("{}:start", self.key));
            self.running.store(true, Ordering::SeqCst);
        }
        fn stop(&self) {
            self.running.store(false, Ordering::SeqCst);
        }
        fn is_running(&self) -> bool {
            self.running.load(Ordering::SeqCst)
        }
        fn set_frame_position(&self, _frame: i32) {}
        fn close(&self) {
            self.running.store(false, Ordering::SeqCst);
        }
        fn master_gain_range(&self) -> Option<(f32, f32)> {
            None // 控件不支持 (不走音量路径)
        }
        fn set_master_gain(&self, _value: f32) {}
    }

    struct MockPlayer {
        /// 每次 open_clip 尝试的文件 stem 序列
        calls: Mutex<Vec<String>>,
        /// key → 最近一次创建 clip 的 running 句柄 (测试侧翻转模拟播放结束)
        running_handles: Mutex<HashMap<String, Arc<AtomicBool>>>,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl MockPlayer {
        fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
            let key = path.file_stem().unwrap().to_string_lossy().into_owned();
            self.calls.lock().unwrap().push(key.clone());
            if !path.exists() {
                return Err("mock: file missing".into());
            }
            let running = Arc::new(AtomicBool::new(false));
            self.running_handles
                .lock()
                .unwrap()
                .insert(key.clone(), Arc::clone(&running));
            Ok(Box::new(MockClip {
                key,
                log: Arc::clone(&self.log),
                running,
            }))
        }

        /// 模拟指定 key 的 clip 播放结束 (is_running → false)
        fn finish(&self, key: &str) {
            if let Some(r) = self.running_handles.lock().unwrap().get(key) {
                r.store(false, Ordering::SeqCst);
            }
        }
    }

    /// Box<dyn SoundPlayer> 的转发壳 (让测试保留 mock 句柄)
    struct PlayerForward(Arc<MockPlayer>);

    impl SoundPlayer for PlayerForward {
        fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
            self.0.open_clip(path)
        }
    }

    // ---- mock ConfigProvider ----

    struct MapConfig {
        values: Mutex<HashMap<String, String>>,
    }

    impl ConfigProvider for MapConfig {
        fn get_config(&self, key: &str) -> Option<String> {
            self.values.lock().unwrap().get(key).cloned()
        }
        fn set_config(&self, key: &str, value: &str) {
            self.values
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
        }
        fn is_field_disabled(&self, key: &str) -> bool {
            self.get_config(key).as_deref() == Some("true")
        }
    }

    // ---- mock VoiceWarningService ----

    struct MockSvcData {
        current_time_ms: i64,
        player_live: bool,
        is_downing_flap: bool,
        flap_allow_angle: f64,
        flap_allow_speed: f64,
        total_fuel: f64,
        fuel_percent: i32,
        radio_alt: f64,
        d_radio_alt: f64,
        cur_load_min_work_time: f64,
        maximum_thr_rpm: f64,
        get_maximum_rpm: bool,
        is_eng_jet: bool,
        stall_speed: f64,
        st: State,
        indic: Indicators,
        fatal_warn: Option<bool>,
    }

    impl MockSvcData {
        fn new() -> Self {
            let mut st = State::new();
            st.init(); // Java Service 构造即 sState.init() — 引擎数组 len 16
            MockSvcData {
                current_time_ms: 0,
                player_live: true,
                is_downing_flap: false,
                // Java Service 字段缺省 (service_fields.rs 文档): Float.MAX_VALUE 拓宽
                flap_allow_angle: f32::MAX as f64,
                flap_allow_speed: f32::MAX as f64,
                total_fuel: 100.0,
                fuel_percent: 100,
                radio_alt: 0.0,
                d_radio_alt: 0.0,
                // Java Service 缺省 99999*1000
                cur_load_min_work_time: 99999.0 * 1000.0,
                maximum_thr_rpm: 1.0, // Java Service 缺省 1
                get_maximum_rpm: false,
                is_eng_jet: false,
                stall_speed: 0.0,
                st,
                indic: Indicators::new(),
                fatal_warn: None,
            }
        }
    }

    struct MockSvc {
        data: Mutex<MockSvcData>,
    }

    impl VoiceWarningService for MockSvc {
        fn current_time_ms(&self) -> i64 {
            self.data.lock().unwrap().current_time_ms
        }
        fn player_live(&self) -> bool {
            self.data.lock().unwrap().player_live
        }
        fn set_fatal_warn(&self, v: bool) {
            self.data.lock().unwrap().fatal_warn = Some(v);
        }
        fn is_downing_flap(&self) -> bool {
            self.data.lock().unwrap().is_downing_flap
        }
        fn flap_allow_angle(&self) -> f64 {
            self.data.lock().unwrap().flap_allow_angle
        }
        fn flap_allow_speed(&self) -> f64 {
            self.data.lock().unwrap().flap_allow_speed
        }
        fn total_fuel(&self) -> f64 {
            self.data.lock().unwrap().total_fuel
        }
        fn fuel_percent(&self) -> i32 {
            self.data.lock().unwrap().fuel_percent
        }
        fn radio_alt(&self) -> f64 {
            self.data.lock().unwrap().radio_alt
        }
        fn d_radio_alt(&self) -> f64 {
            self.data.lock().unwrap().d_radio_alt
        }
        fn cur_load_min_work_time(&self) -> f64 {
            self.data.lock().unwrap().cur_load_min_work_time
        }
        fn maximum_thr_rpm(&self) -> f64 {
            self.data.lock().unwrap().maximum_thr_rpm
        }
        fn get_maximum_rpm(&self) -> bool {
            self.data.lock().unwrap().get_maximum_rpm
        }
        fn is_eng_jet(&self) -> bool {
            self.data.lock().unwrap().is_eng_jet
        }
        fn get_stall_speed(&self) -> f64 {
            self.data.lock().unwrap().stall_speed
        }
        fn s_state(&self) -> State {
            snapshot_state(&self.data.lock().unwrap().st)
        }
        fn s_indic(&self) -> Indicators {
            snapshot_indicators(&self.data.lock().unwrap().indic)
        }
    }

    impl MockSvc {
        fn edit(&self, f: impl FnOnce(&mut MockSvcData)) {
            f(&mut self.data.lock().unwrap());
        }
    }

    // ---- 测试环境 ----

    struct TestEnv {
        vw: VoiceWarning,
        player: Arc<MockPlayer>,
        svc: Arc<MockSvc>,
        ui_bus: Arc<UIStateBus>,
        fd_bus: Arc<FlightDataBus>,
        config: Arc<MapConfig>,
        log: Arc<Mutex<Vec<String>>>,
        _dir: PathBuf,
    }

    fn env() -> TestEnv {
        let dir = tmp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        // 为全部告警 key 造 wav, 避开 load_clip 缺失文件的错误日志噪音
        for ty in voice_alert_type::ALL {
            let _ = std::fs::write(dir.join(format!("{}.wav", ty.get_key())), b"wav");
        }
        let player = Arc::new(MockPlayer {
            calls: Mutex::new(Vec::new()),
            running_handles: Mutex::new(HashMap::new()),
            log: Arc::new(Mutex::new(Vec::new())),
        });
        let log = Arc::clone(&player.log);
        let mgr = Arc::new(VoiceResourceManager::new_with_voice_dir(
            Box::new(PlayerForward(Arc::clone(&player))),
            dir.to_str().unwrap().to_string(),
        ));
        let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
        let ui_bus = Arc::new(UIStateBus::new());
        let fd_bus = Arc::new(FlightDataBus::new());
        let config = Arc::new(MapConfig {
            values: Mutex::new(HashMap::new()),
        });
        let svc = Arc::new(MockSvc {
            data: Mutex::new(MockSvcData::new()),
        });
        let mut vw = VoiceWarning::new(
            Arc::clone(&config) as Arc<dyn ConfigProvider + Send + Sync>,
            mgr,
            fm,
            Arc::clone(&ui_bus),
            Arc::clone(&fd_bus),
            Arc::new(PlayerForward(Arc::clone(&player))) as Arc<dyn SoundPlayer>,
        );
        vw.init(Some(Arc::clone(&svc) as Arc<dyn VoiceWarningService>));
        TestEnv {
            vw,
            player,
            svc,
            ui_bus,
            fd_bus,
            config,
            log,
            _dir: dir,
        }
    }

    fn fd_event(compressor_stage_mismatch: bool) -> FlightDataEvent {
        FlightDataEvent::new(
            EventPayload::builder()
                .compressor_stage_mismatch(compressor_stage_mismatch)
                .build(),
            None,
            None,
        )
    }

    // ---- init / 注册表 ----

    // Java init(S=null): doit=false 短路
    #[test]
    fn init_none_service_disables() {
        let TestEnv { mut vw, .. } = env();
        vw.init(None);
        assert!(!vw.doit.load(Ordering::SeqCst));
    }

    // init 注册全部告警; "fail_engine" 同 key 后写覆盖 → map 指向 invert 实例
    // (Java ConcurrentHashMap.put 覆盖语义保真); start1 启动音效播放一次
    #[test]
    fn init_registers_alerts_and_fail_engine_overwrite() {
        let TestEnv { vw, log, .. } = env();
        let map = vw.alerts.lock().unwrap();
        // 25 个告警实例 - 1 个 fail_engine 重键 = 24 个去重注册项
        assert_eq!(map.len(), 24, "alerts map 应含 24 个去重 key");
        assert!(Arc::ptr_eq(
            map.get("fail_engine").unwrap(),
            vw.eng_fail_invert.as_ref().unwrap()
        ));
        assert!(!Arc::ptr_eq(
            map.get("fail_engine").unwrap(),
            vw.eng_fail.as_ref().unwrap()
        ));
        assert!(map.contains_key("start1"));
        drop(map);
        // 启动音效: init 尾部 playOnce(currentTimeMs=0)
        assert_eq!(starts(&log, "start1"), 1);
    }

    // init 后的默认告警线 (无 FM): aoa=15, ias/mach=Float.MAX_VALUE 拓宽,
    // gear=450, ny=-4/10, 舵效 65535, 低油量线 10
    #[test]
    fn init_default_lines_without_fm() {
        let TestEnv { vw, .. } = env();
        assert_eq!(vw.aoa_warning_line, 15.0);
        assert_eq!(vw.ias_warning_line, f32::MAX as f64);
        assert_eq!(vw.mach_warning_line, f32::MAX as f64);
        assert_eq!(vw.gear_warning_line, 450.0);
        assert_eq!(vw.ny_warning_line0, -4.0);
        assert_eq!(vw.ny_warning_line1, 10.0);
        assert_eq!(vw.rudder_eff_ias, 65535);
        assert_eq!(vw.elevator_eff_ias, 65535);
        assert_eq!(vw.aileron_eff_ias, 65535);
        assert_eq!(vw.lowfuel_warning_line, 10);
        assert!(vw.is_gear_alive && vw.is_flap_alive && !vw.eng_damage);
    }

    // ---- VoiceAlert 冷却/播放 ----

    // 冷却期内不重播; 冷却过后 clip 仍在播放则继续压制; 播放结束后可重播
    #[test]
    fn voice_alert_cooldown_and_running_gate() {
        let TestEnv { vw, player, log, .. } = env();
        let alert = vw.aoa_crit.as_ref().unwrap(); // 冷却 1s

        assert!(!alert.is_playing(0)); // 初始静默
        alert.play_once(0);
        assert_eq!(starts(&log, "aoaCrit"), 1);
        assert!(alert.is_act.load(Ordering::SeqCst));

        alert.play_once(500); // 冷却期内 (500-0 <= 1000)
        assert_eq!(starts(&log, "aoaCrit"), 1);

        player.finish("aoaCrit"); // 模拟播放结束
        alert.play_once(1001); // 冷却过 + 不再 running → 重播
        assert_eq!(starts(&log, "aoaCrit"), 2);

        alert.play_once(1002); // 刚播过, 冷却重新计时
        assert_eq!(starts(&log, "aoaCrit"), 2);
    }

    // 不可用 (资源缺失/配置禁用) 时 is_playing 恒 true — 防重试循环; play_once 无操作
    #[test]
    fn voice_alert_unavailable_pretends_playing() {
        let TestEnv { vw, log, .. } = env();
        // 无 wav 的 key → reload 后 available=false
        let a = VoiceAlert::new("no_such_key", 1);
        a.reload(&*vw.config_provider, &vw.resource_manager);
        assert!(a.is_playing(0), "不可用时假装在播放");
        a.play_once(0); // 无操作, 不 panic
        assert_eq!(starts(&log, "no_such_key"), 0);
    }

    // ---- configHandler 触发链 (UIStateBus 注入) ----

    // CONFIG_CHANGED(voice_<key>) → 命中 alerts 注册表 → reload; 非语音键/未知键忽略
    #[test]
    fn config_changed_reloads_registered_alert() {
        let TestEnv {
            vw,
            config,
            ui_bus,
            player,
            ..
        } = env();
        let opens = |key: &str| {
            player
                .calls
                .lock()
                .unwrap()
                .iter()
                .filter(|k| k.as_str() == key)
                .count()
        };
        let before = opens("aoaCrit");

        // 命中: voice_aoaCrit
        assert_eq!(
            ui_bus.publish(
                ui_state_events::CONFIG_CHANGED,
                Some("test"),
                Some("voice_aoaCrit")
            ),
            1
        );
        assert_eq!(opens("aoaCrit"), before + 1, "应触发一次 reload 重开");

        // 非语音键: handler 前缀过滤
        let delivered = ui_bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("test"),
            Some("showSpeedBar"),
        );
        assert_eq!(delivered, 1);
        assert_eq!(opens("aoaCrit"), before + 1);

        // 未知语音键: 注册表未命中, 无 reload
        ui_bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("test"),
            Some("voice_nope"),
        );
        assert_eq!(opens("nope"), 0);

        // 配置禁用: voice_aoaHigh = "default|false" → reload 后 available=false
        config.set_config("voice_aoaHigh", "default|false");
        ui_bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("test"),
            Some("voice_aoaHigh"),
        );
        assert!(vw.aoa_high.as_ref().unwrap().is_playing(0));
    }

    // ---- 订阅注销 (Java 泄漏根治 — LIFETIMES §2.1) ----

    // dispose 显式注销两条总线订阅
    #[test]
    fn dispose_unsubscribes_both_buses() {
        let TestEnv {
            mut vw,
            ui_bus,
            fd_bus,
            ..
        } = env();
        assert_eq!(
            ui_bus.publish(
                ui_state_events::CONFIG_CHANGED,
                Some("t"),
                Some("voice_aoaCrit")
            ),
            1
        );
        assert_eq!(fd_bus.subscriber_count(), 1);
        vw.dispose();
        assert_eq!(
            ui_bus.publish(
                ui_state_events::CONFIG_CHANGED,
                Some("t"),
                Some("voice_aoaCrit")
            ),
            0,
            "Java 版此处仍会送达 (configHandler 泄漏), Rust 版已根治"
        );
        assert_eq!(fd_bus.subscriber_count(), 0);
    }

    // 忘记 dispose 时 Drop 兜底注销 (泄漏在类型层面不可能)
    #[test]
    fn drop_unsubscribes_both_buses() {
        let TestEnv {
            vw,
            ui_bus,
            fd_bus,
            ..
        } = env();
        assert_eq!(fd_bus.subscriber_count(), 1);
        drop(vw); // 不调 dispose 直接 drop
        assert_eq!(fd_bus.subscriber_count(), 0);
        assert_eq!(
            ui_bus.publish(
                ui_state_events::CONFIG_CHANGED,
                Some("t"),
                Some("voice_aoaCrit")
            ),
            0
        );
    }

    // ---- check* 触发链 (mock Service 数据 + mock 播放日志) ----

    // 临界攻角: aoaCrit 播放 + aoaHigh 被标记已播放 (不重复触发) + fatal;
    // 高攻角带: 仅 aoaHigh 播放, 非 fatal
    #[test]
    fn check_aoa_warning_crit_marks_high() {
        let TestEnv {
            mut vw,
            log,
            ..
        } = env();
        vw.st.ias = 100;
        vw.st.aoa = 14.5; // > 15-1

        assert!(vw.check_aoa_warning(1000));
        assert_eq!(starts(&log, "aoaCrit"), 1);
        assert_eq!(starts(&log, "aoaHigh"), 0, "aoaHigh 不应播放");
        // 同时标记 aoaHigh 为已播放 (Java 包内直写字段)
        assert!(vw.aoa_high.as_ref().unwrap().is_act.load(Ordering::SeqCst));
        assert_eq!(
            vw.aoa_high
                .as_ref()
                .unwrap()
                .last_time_play
                .load(Ordering::SeqCst),
            1000
        );

        // 高攻角预警带: 15*0.75=11.25 < aoa <= 14。
        // t 须 > 1000+8000: crit 分支已把 aoaHigh 标记为已播放 (t=1000),
        // 8s 冷却期内 playOnce 被压制 (Java 防双告警语义)
        vw.st.aoa = 12.0;
        assert!(!vw.check_aoa_warning(10_000));
        assert_eq!(starts(&log, "aoaHigh"), 1);

        // IAS 门禁: <= 80 直接短路
        vw.st.ias = 80;
        vw.st.aoa = 30.0;
        assert!(!vw.check_aoa_warning(3000));
        assert_eq!(starts(&log, "aoaCrit"), 1, "IAS 不足不触发");
    }

    // IAS / Mach 双超限各自播放并置 fatal
    #[test]
    fn check_speed_warning_ias_mach_fatal() {
        let TestEnv {
            mut vw,
            log,
            ..
        } = env();
        vw.ias_warning_line = 500.0;
        vw.mach_warning_line = 0.8;
        vw.st.ias = 600;
        vw.st.m = 0.9;

        assert!(vw.check_speed_warning(0));
        assert_eq!(starts(&log, "warn_ias"), 1);
        assert_eq!(starts(&log, "warn_mach"), 1);

        // 均未超限
        let TestEnv {
            vw: mut vw2,
            log: log2,
            ..
        } = env();
        vw2.ias_warning_line = 500.0;
        vw2.mach_warning_line = 0.8;
        vw2.st.ias = 400;
        vw2.st.m = 0.5;
        assert!(!vw2.check_speed_warning(0));
        assert_eq!(starts(&log2, "warn_ias"), 0);
        assert_eq!(starts(&log2, "warn_mach"), 0);
    }

    // 起落架超速 fatal / 减速板组合 / 大下降率
    #[test]
    fn check_gear_brake_vario_warnings() {
        let TestEnv {
            mut vw,
            log,
            ..
        } = env();
        // gear: 放下 (gear>0) 且 IAS >= 450
        vw.st.ias = 500;
        vw.st.gear = 100;
        assert!(vw.check_gear_warning(0));
        assert_eq!(starts(&log, "warn_gear"), 1);

        // brake: 起落架未放下但减速板展开
        vw.st.gear = 0;
        vw.st.airbrake = 90;
        vw.check_brake_warning(0);
        assert_eq!(starts(&log, "warn_brake"), 1);

        // vario: 起落架放下且下降率过高
        vw.st.gear = 50;
        vw.st.vy = -9.0;
        vw.check_vario_warning(0);
        assert_eq!(starts(&log, "warn_highvario"), 1);

        // 起落架损坏后不再告警
        vw.is_gear_alive = false;
        vw.st.gear = 100;
        vw.st.vy = -20.0;
        assert!(!vw.check_gear_warning(1000));
        assert_eq!(starts(&log, "warn_gear"), 1);
    }

    // 襟翼告警两条路径 (静止超限 / 正在下襟翼的宽限)
    #[test]
    fn check_flap_warning_conditions() {
        let TestEnv {
            mut vw,
            svc,
            player,
            log,
            ..
        } = env();
        // cond1: 非下襟翼态, 允许角-当前角 < 2 且襟翼非 0
        svc.edit(|d| {
            d.is_downing_flap = false;
            d.flap_allow_angle = 10.0;
        });
        vw.st.flaps = 9;
        assert!(vw.check_flap_warning(0));
        assert_eq!(starts(&log, "warn_flap"), 1);

        // cond2: 下襟翼态, 允许角-当前角 < 8
        player.finish("warn_flap"); // 首Frame clip 已停 (否则 isRunning 压制重播)
        svc.edit(|d| {
            d.is_downing_flap = true;
            d.flap_allow_angle = 10.0;
        });
        vw.st.flaps = 5; // 10-5=5 < 8
        assert!(vw.check_flap_warning(1500)); // warn_flap 冷却 1s (须严格大于)
        assert_eq!(starts(&log, "warn_flap"), 2);

        // 均不满足: 差值足够大
        svc.edit(|d| {
            d.is_downing_flap = true;
            d.flap_allow_angle = 20.0;
        });
        vw.st.flaps = 5; // 20-5=15 >= 8
        assert!(!vw.check_flap_warning(2000));
        assert_eq!(starts(&log, "warn_flap"), 2);
    }

    // 无油/低油量 16 tick 门限 (后自增语义: 第 18 次调用才播放)
    #[test]
    fn check_fuel_warning_tick_gates() {
        let TestEnv {
            mut vw,
            svc,
            log,
            ..
        } = env();
        svc.edit(|d| {
            d.total_fuel = 0.0;
            d.fuel_percent = 5; // <= 10
        });
        for t in 0..17 {
            vw.check_fuel_warning(t * 100);
        }
        assert_eq!(starts(&log, "fail_nofuel"), 0, "17 次未过门限");
        assert_eq!(starts(&log, "warn_lowfuel"), 0);
        vw.check_fuel_warning(1700); // 第 18 次: old=17 > 16
        assert_eq!(starts(&log, "fail_nofuel"), 1);
        assert_eq!(starts(&log, "warn_lowfuel"), 1);
        // 计数已清零, 立即再调不重播
        vw.check_fuel_warning(1800);
        assert_eq!(starts(&log, "fail_nofuel"), 1);
        assert_eq!(starts(&log, "warn_lowfuel"), 1);
    }

    // 油压低 → 2s 后 warn_lowpressure + 引擎损坏标记; 损坏后油压 0 → 1s 后 fail_engine
    #[test]
    fn check_fuel_pressure_warning_chain() {
        let TestEnv {
            mut vw,
            log,
            ..
        } = env();
        vw.st.throttle = 100;
        vw.indic.fuel_pressure = 0.1; // 100 - 0.1*10 = 99 > 2

        for _ in 0..19 {
            vw.check_fuel_pressure_warning(0);
        }
        assert!(!vw.eng_damage, "1900ms 未到 2000 门限");
        vw.check_fuel_pressure_warning(0); // 第 20 次: 2000
        assert!(vw.eng_damage);
        assert_eq!(starts(&log, "warn_lowpressure"), 1);

        // 引擎损坏后油压归零 → fail_engine, 损坏标记清除。
        // 注意 Java 两块共用 fuelPCheck 计数器: fp=0 时低油压块条件仍真
        // (throttle-0 > 2), 每次调用两块各 +100 → 第 5 次调用即达 1000 门限
        vw.indic.fuel_pressure = 0.0;
        for _ in 0..4 {
            vw.check_fuel_pressure_warning(0);
        }
        assert_eq!(starts(&log, "fail_engine"), 0);
        vw.check_fuel_pressure_warning(0); // 第 5 次: 累计 1000
        assert!(!vw.eng_damage);
        assert_eq!(starts(&log, "fail_engine"), 1);

        // 条件消失 → 计数清零
        vw.indic.fuel_pressure = 10.0; // 100 - 100 = 0, 不 > 2
        vw.check_fuel_pressure_warning(0);
        assert_eq!(vw.fuel_p_check, 0);
    }

    // 倒飞断油 (fail_engine 冷却 5s 的 invert 实例) + 转速低/高
    #[test]
    fn check_inverted_flight_and_rpm_warnings() {
        let TestEnv {
            mut vw,
            svc,
            log,
            ..
        } = env();
        // 倒飞: Ny<0, 油门>50, 推力<50
        vw.st.ny = -1.0;
        vw.st.throttle = 60;
        vw.st.thrust[0] = 10;
        vw.check_inverted_flight_warning(0);
        assert_eq!(starts(&log, "fail_engine"), 1, "invert 实例共用 fail_engine 键");

        // 转速低: 非喷气 + 桨距有效, 油门-30 > RPM*100/maxRPM。
        // (须先解除倒飞态 — Ny<0+油门大+推力低 会短路转速检测)
        svc.edit(|d| {
            d.is_eng_jet = false;
            d.maximum_thr_rpm = 1000.0;
        });
        vw.st.ny = 1.0;
        vw.st.thrust[0] = 100;
        vw.st.throttle = 100;
        vw.st.rpm = 100; // 100*100/1000 = 10 < 70
        vw.check_rpm_warning(0);
        assert_eq!(starts(&log, "warn_lowrpm"), 1);

        // 转速高: 自适应已学习 + RPM*100/maxRPM >= 105
        svc.edit(|d| d.get_maximum_rpm = true);
        vw.st.rpm = 1100; // 110 >= 105; 70 > 110 为假 → 低转速不再触发
        vw.check_rpm_warning(10_000); // warn_lowrpm 冷却 10s
        assert_eq!(starts(&log, "warn_highrpm"), 1);
        assert_eq!(starts(&log, "warn_lowrpm"), 1);
    }

    // 失速: 存活 + 未放起落架 + 有下降率 + IAS <= 失速速度
    #[test]
    fn check_stall_warning() {
        let TestEnv {
            mut vw,
            svc,
            log,
            ..
        } = env();
        svc.edit(|d| d.stall_speed = 150.0);
        vw.st.gear = 0;
        vw.st.vy = -1.0;
        vw.st.ias = 140;
        vw.check_stall_warning(0);
        assert_eq!(starts(&log, "warn_stall"), 1);

        // 速度高于失速速度: 不触发
        vw.st.ias = 160;
        vw.check_stall_warning(2000); // 冷却 2s
        assert_eq!(starts(&log, "warn_stall"), 1);

        // 无下降率 (Vy==0): 不触发
        vw.st.ias = 140;
        vw.st.vy = 0.0;
        vw.check_stall_warning(4000);
        assert_eq!(starts(&log, "warn_stall"), 1);
    }

    // 高度告警 (下降率 > 高度/10) 优先; 其次地形告警 (无线电高度变化率)
    #[test]
    fn check_altitude_and_terrain_warning() {
        let TestEnv {
            mut vw,
            svc,
            log,
            ..
        } = env();
        vw.st.gear = 0; // 且 playerLive=true
        vw.st.heightm = 100.0;
        vw.st.vy = -20.0; // -20 < -100/10 = -10
        assert!(vw.check_altitude_warning(0));
        assert_eq!(starts(&log, "warn_altitude"), 1);

        // 地形: 下降率未触高度线, 但无线电高度骤降
        svc.edit(|d| {
            d.radio_alt = 100.0;
            d.d_radio_alt = -15.0; // < -100/10
        });
        vw.st.vy = -5.0;
        assert!(vw.check_altitude_warning(5000)); // 冷却 5s
        assert_eq!(starts(&log, "warn_terrain"), 1);

        // 起落架放下 → 短路
        vw.st.gear = 100;
        assert!(!vw.check_altitude_warning(10_000));
        assert_eq!(starts(&log, "warn_altitude"), 1);
    }

    // 过载动态阈值: rawWingCritOverload 存在时按当前重量重算上下限
    #[test]
    fn check_load_factor_dynamic_limits() {
        let TestEnv {
            mut vw,
            log,
            ..
        } = env();
        // 静态线上限拉高到 20 — 动态计算应压回 ~12, ny=15 触发
        vw.ny_warning_line1 = 20.0;
        vw.st.ny = 15.0;
        assert!(!vw.check_load_factor_warning(0), "静态线内不触发");
        assert_eq!(starts(&log, "warn_loadfactor"), 0);

        let mut b = Blkx::default();
        // 正 g 限 = 1.2*(2*raw1/(g*6000) - 1) = 12 → raw1 = 11*g*6000/2
        b.raw_wing_crit_overload = Some([0.0, 11.0 * crate::g * 6000.0 / 2.0]);
        vw.blkx = Some(b);
        vw.nofuelweight = 5000.0;
        vw.st.mfuel = 1000.0; // currentWeight = 6000
        assert!(vw.check_load_factor_warning(3000), "动态上限 ~12, ny=15 应触发");
        assert_eq!(starts(&log, "warn_loadfactor"), 1);
    }

    // 舵效告警边沿锁存: 越线首帧播放, 持续越线不重播, 回线复位
    #[test]
    fn control_effectiveness_edge_latch() {
        let TestEnv {
            mut vw,
            player,
            log,
            ..
        } = env();
        vw.aileron_eff_ias = 300;
        vw.rudder_eff_ias = 400;
        vw.st.ias = 350;

        vw.check_control_effectiveness_warning(0);
        assert_eq!(starts(&log, "aileronEff"), 1);
        assert_eq!(starts(&log, "rudderEff"), 0, "方向舵未越线");

        vw.check_control_effectiveness_warning(100);
        assert_eq!(starts(&log, "aileronEff"), 1, "锁存期内不重播");

        vw.st.ias = 200; // 回线复位
        vw.check_control_effectiveness_warning(200);
        player.finish("aileronEff"); // 首Frame clip 已停 (否则 isRunning 压制重播)
        vw.st.ias = 350; // 再次越线 → 重新播放 (冷却 10s 已过)
        vw.check_control_effectiveness_warning(30_000);
        assert_eq!(starts(&log, "aileronEff"), 2);
    }

    // 增压器档位不匹配: FlightDataBus 事件 → currentMismatch → 3s 延迟告警,
    // 每 3s 重复, 状态解除即取消
    #[test]
    fn compressor_mismatch_delayed_and_repeating() {
        let TestEnv {
            mut vw,
            fd_bus,
            player,
            log,
            ..
        } = env();
        // 总线注入断言: 发布事件 → 订阅闭包写 currentMismatch
        fd_bus.publish(&fd_event(true));
        assert!(vw.current_mismatch.load(Ordering::SeqCst));

        vw.check_compressor_warning(1000); // false→true: 定时器 = 1000+3000
        assert_eq!(vw.pending_compressor_warn_time, 4000);
        assert_eq!(starts(&log, "warn_compressor"), 0, "3s 延迟内不告警");
        vw.check_compressor_warning(3999);
        assert_eq!(starts(&log, "warn_compressor"), 0);

        vw.check_compressor_warning(4000); // 到点告警, 重排 7000
        assert_eq!(starts(&log, "warn_compressor"), 1);
        assert_eq!(vw.pending_compressor_warn_time, 7000);

        player.finish("warn_compressor"); // 冷却 0, 仅 running 压制需解除
        vw.check_compressor_warning(7000); // 每 3s 重复
        assert_eq!(starts(&log, "warn_compressor"), 2);

        fd_bus.publish(&fd_event(false)); // 一致了 → 取消定时器
        vw.check_compressor_warning(7200);
        assert_eq!(vw.pending_compressor_warn_time, 0);
        player.finish("warn_compressor");
        vw.check_compressor_warning(20_000);
        assert_eq!(starts(&log, "warn_compressor"), 2, "解除后不再重复");
    }

    // 起落架/襟翼超速计时损坏 (10s/15s), 收起后恢复。
    // gear 须为放下态 (>0): 同方法尾部的 "收起后恢复" 分支在 gear==0 时会把
    // 刚标记的损坏立即复原 (Java 同序行为)
    #[test]
    fn structure_health_damage_and_restore() {
        let TestEnv {
            mut vw,
            svc,
            log,
            ..
        } = env();
        vw.st.ias = 500; // > gear 线 450
        vw.st.gear = 100; // 放下态, 阻断同帧复原
        for _ in 0..99 {
            vw.update_structure_health(); // 9900ms
        }
        assert!(vw.is_gear_alive, "9900ms 未到 10s 门限");
        vw.update_structure_health(); // 第 100 次: 10000
        assert!(!vw.is_gear_alive, "超速 10s 标记损坏");

        // 收起后恢复
        vw.st.gear = 0;
        vw.update_structure_health();
        assert!(vw.is_gear_alive);

        // 襟翼: 允许速度 300, 超速 15s 损坏
        svc.edit(|d| d.flap_allow_speed = 300.0);
        vw.st.flaps = 50;
        for _ in 0..150 {
            vw.update_structure_health();
        }
        assert!(!vw.is_flap_alive, "超速 15s 标记损坏");
        vw.st.flaps = 0;
        vw.update_structure_health();
        assert!(vw.is_flap_alive);

        // 损坏期间的 gear 告警不再触发 (checkGearWarning 的 isGearAlive 门禁)
        assert_eq!(starts(&log, "warn_gear"), 0);
    }

    // 无 FM 时 updateDynamicParameters 保持既有告警线 (R1 快照降级路径)
    #[test]
    fn update_dynamic_parameters_keeps_lines_without_fm() {
        let TestEnv {
            mut vw,
            svc,
            ..
        } = env();
        svc.edit(|d| d.indic.wsweep_indicator = 0.5); // 有 sweep 指示也无 FM 可用
        vw.st.flaps = 30;
        let (aoa, ias, mach) = (
            vw.aoa_warning_line,
            vw.ias_warning_line,
            vw.mach_warning_line,
        );
        vw.update_dynamic_parameters();
        assert_eq!(vw.aoa_warning_line, aoa);
        assert_eq!(vw.ias_warning_line, ias);
        assert_eq!(vw.mach_warning_line, mach);
    }

    // ---- Legacy 直播面 ----

    // playWav/getClip: 文件缺失走异常腿不 panic; 存在文件返回可用句柄
    #[test]
    fn play_wav_and_get_clip_legacy_paths() {
        let TestEnv {
            mut vw,
            log,
            _dir,
            ..
        } = env();
        vw.play_wav("no_such_file.wav"); // Err 分支: 吞掉不 panic
        assert_eq!(starts(&log, "no_such_file"), 0);

        assert!(vw.get_clip("no_such_file.wav").is_none());

        let clip = vw.get_clip(_dir.join("aoaCrit.wav").to_str().unwrap());
        assert!(clip.is_some(), "存在的文件应打开成功");
        assert!(!clip.as_ref().unwrap().is_running());
    }

    // ---- run() 主循环 (§2.13 线程映射) ----

    // 启动延迟后正常打点 (写 fatalWarn), 停机标志翻转后循环退出
    #[test]
    fn run_loop_ticks_and_exits_on_stop() {
        let TestEnv {
            mut vw,
            svc,
            ..
        } = env();
        let doit = Arc::clone(&vw.doit);
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            vw.run();
            let _ = tx.send(());
        });

        // 启动延迟 1s + 至少一个 100ms tick
        std::thread::sleep(Duration::from_millis(1500));
        assert!(
            svc.data.lock().unwrap().fatal_warn.is_some(),
            "至少一轮 tick 应写 fatalWarn"
        );

        doit.store(false, Ordering::SeqCst); // ≈ Java OverlayEntry.close 的 interrupt
        let done = rx.recv_timeout(Duration::from_secs(2));
        assert!(done.is_ok(), "停机后 run() 应及时退出");
        let _ = handle.join();
    }
}
