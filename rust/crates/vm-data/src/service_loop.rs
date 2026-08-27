//! 对应 Java: `src/prog/Service.java` 的**方法区**——run() 轮询循环 (L1799-1862) /
//! processPollingCycle (L1705-1796) / publishFlightDataEvent (L440-482) /
//! calculate 链接线 (L1115-1178) / resetvaria·clearvaria·resetEngLoad
//! (L1510-1666) / 构造器 (L1678-1699)。
//! (实例字段区 + TelemetrySource getter 见 service_fields.rs, D6 两 item 划分。)
//!
//! ## 结构裁决 (LIFETIMES / D6 / PORTING §2.8)
//!
//! - Java `Service` 的 public 字段被 EDT 混读无锁 → [`ServiceData`] 收进
//!   `RwLock` (service_fields.rs 模块头裁决), 本模块**任何锁的临界区内不调
//!   回调/不做 IO**: 方法开头 lock 取数据副本→释放→计算→短锁写回
//!   (PORTING §2.8 "Service 类多 synchronized 互相调用" 的锁粒度指示)。
//! - Java `Controller c` 反向引用 (环 1) 不迁移: 配置读经 [`ServiceConfig`]
//!   构造注入, c.initStatusBar/changeS2/changeS3/S4toS1/onAircraftChanged/
//!   c.Log.logTick 等 Controller 协作点逐处 `// TODO(port)` 标注 (Controller 波次)。
//! - Java `FlightDataBus.getInstance()`/`FMManager.getInstance()` 单例解散 →
//!   构造注入 `Arc` (LIFETIMES §1.1 "实例归调用方持有")。
//! - **顶层 catch_unwind** (PORTING §6 契约): 对齐 Java run() L1850 顶层
//!   `catch (Exception)` 丢一轮继续的语义——单条畸形遥测 (解析 panic 点, 如
//!   Boolean 拆箱 NPE 的复刻) 不允许杀死遥测线程。
//! - `Thread.interrupt()` 退出 → `Arc<AtomicBool>` 停机标志轮询 (§2.13);
//!   `Thread.sleep` → `exception_helper::sleep_quietly` (可中断睡眠)。
//! - run() 在独立线程由调用方经 [`start`] spawn, [`ServiceHandle`] 提供
//!   stop 生命周期 (Java Controller.start:634 `S1 = new Thread(Service);
//!   S1.setPriority(MAX_PRIORITY); S1.start()`)。
//! - HTTP 选型: 用 **vm-core `HttpHelper`** (HttpHelper.java 一比一翻译, 含
//!   buf 复用/CompletableFuture 等待/byte-perfect 读头语义), 不用本 crate
//!   `data::http` (POC 存量, 行为等价但非保真翻译)——保真度裁决, 见
//!   parser/mod.rs 并存说明。
//! - parser 选型: 用 **vm-core `parser::{State, Indicators}`** (保真版,
//!   任务指定); 已译 [`Deriver`] (data/derive.rs) 接口收 POC 版
//!   `StateRaw/IndicatorsRaw`, 本文件以 [`to_state_raw`]/[`to_indicators_raw`]
//!   适配 (字段级映射, 哨兵 -65535 同值穿透)。

use std::net::SocketAddr;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use vm_core::calc_helper::SimpleMovingAverage;
use vm_core::event::event_payload::EventPayload;
use vm_core::event::flight_data_event::{FlightDataEvent, OpaqueObject};
use vm_core::flight_analyzer::AnalyzerService;
use vm_core::flight_data_bus::FlightDataBus;
use vm_core::flight_log::{FlightLogSlot, FlightLogSnapshot};
use vm_core::fm::{FMHandle, FMManager};
use vm_core::http_helper::HttpHelper;
use vm_core::parser::state::MAX_ENG_NUM;
use vm_core::parser::{Indicators, MapInfo, MapObj, State};
use vm_core::ui_model::TelemetrySource as _;
use vm_core::{exception_helper, format, logger, G};

use crate::data::derive::Deriver;
use crate::data::json::{F_INVALID, IndicatorsRaw, StateRaw};
use crate::service_fields::{
    ServiceData, ENGINE_TYPE_JET, ENGINE_TYPE_PROP, ENGINE_TYPE_TURBOPROP, ENGINE_TYPE_UNKNOWN,
    NASTRING,
};

/// 读锁获取: **中毒穿透** (`into_inner`)——对齐 Java "异常后对象处于不一致状态
/// 继续用" 的宽松语义 (§6 契约: panic 被顶层 catch_unwind 吞掉后线程继续轮询,
/// 锁不得永久失效; std 的 poisoning 是 Rust 防御默认, 此处显式解除,
/// flight_data_bus.rs 的 AssertUnwindSafe 同一宽松契约)。
/// 临界区内确实只做赋值/字段拷贝, 中毒源只可能是跨锁 panic (如 publish 拆箱)。
fn read_data(data: &RwLock<ServiceData>) -> std::sync::RwLockReadGuard<'_, ServiceData> {
    data.read().unwrap_or_else(|e| e.into_inner())
}

/// 写锁获取: 中毒穿透 (同 [`read_data`])。
fn write_data(data: &RwLock<ServiceData>) -> std::sync::RwLockWriteGuard<'_, ServiceData> {
    data.write().unwrap_or_else(|e| e.into_inner())
}

/// Java `System.currentTimeMillis()` 的 crate 先例形态
/// (fm_manager.rs / flight_data_event.rs 同款): SystemTime → as_millis u128 →
/// as i64 截断; 时钟早于 epoch 时 Java 可得负值而 duration_since 报错 → 取 0。
fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// panic 载荷 → 文本 (fm_manager.rs 同款私有助手, 不越文件共用)。
/// 对齐 Java `"Service error: " + e.getClass().getSimpleName() + " at " + ...`
/// 的 "类型名" 槽位: Rust panic 无类型名, 以消息文本顶位 (见 run 的 PORT 注)。
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else {
        "null".to_string()
    }
}

/// Service 的构造参数集 —— 取代 Java 构造器从 `Controller xc` / `Application`
/// 静态字段读取的全部输入 (环 1 断裂 + §2.9 全局态解散):
/// - `service_loop_interval_ms` ← `xc.serviceLoopIntervalMs`
///   (ConfigurationService 读 ui_layout.cfg, 缺省 50)
/// - `app_port` ← `Application.appPort` (`Lang.httpPort` 缺省 8111;
///   `appPortBkp = appPort + 1111`, 备端口 9222)
/// - `http_header` ← `Application.httpHeader` (`Lang.httpHeader` 缺省 "\n")
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub service_loop_interval_ms: i64,
    pub app_port: u16,
    pub http_header: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        ServiceConfig {
            service_loop_interval_ms: 50,
            app_port: 8111,
            http_header: "\n".to_string(),
        }
    }
}

/// 核心轮询线程的组合体 (Java `class Service implements Runnable` 的方法面;
/// 数据面见 [`ServiceData`])。
///
/// 所有权: 整个 `Service` 被 move 进轮询线程 (run 的 `self`), 与 Java
/// "Service 对象由轮询线程独占写" 一致; `data`/`bus`/`fm_manager`/`stop`
/// 以 `Arc` 与调用方共享 (对应 Java public 字段被 EDT 混读 + 单例总线)。
/// `deriver`/`http_client`/`focus_monitor` 线程独占 (无锁, 对应 Java 字段
/// 仅轮询线程触碰)。
pub struct Service {
    /// 字段快照 (service_fields.rs; Java public 字段的 RwLock 形态)
    pub data: Arc<RwLock<ServiceData>>,
    /// 派生量状态机 (updateSpeed/updateTurn/updateSEP 的 SMA 真人,
    /// service_fields.rs "状态双主边界" 裁决的唯一主人)
    deriver: Deriver,
    /// Java `FMManager.getInstance()` 单例 → 构造注入
    fm_manager: Arc<FMManager>,
    /// Java `FlightDataBus.getInstance()` 单例 → 构造注入
    bus: Arc<FlightDataBus>,
    /// Java `public HttpHelper httpClient` (L1691 构造) —— 轮询线程独占
    http_client: HttpHelper,
    /// Java `private final FocusMonitor focusMonitor = new FocusMonitor()` (L117)。
    /// PORT: vm-core FocusMonitor 构造需注入 detector/coordinator 两依赖
    /// (Java 无参 new 的对应物缺位), 由调用方按需注入; None 时 tick 短路
    /// (Java 默认 enabled=false 时 tick 本就空转, 行为等价)。
    focus_monitor: Option<vm_core::focus_monitor::FocusMonitor>,
    /// FlightLog 共享槽 (Java Controller.logon+Log 二位一体的收敛形态):
    /// Controller 侧 (vm-app) openpad/closepad/换机换入换出, 本线程每轮
    /// logTick (Service.java:1824-1828)。None = 未开记录 (Java Log==null/logon=false)。
    flight_log: FlightLogSlot,
    /// 构造参数 (见 [`ServiceConfig`])
    pub config: ServiceConfig,
    /// §2.13 停机标志 (Java interrupt 的电平形态)
    pub stop: Arc<AtomicBool>,
}

/// Java `Application.requestDest = new InetSocketAddress(Lang.httpIp, appPort)`
/// (Lang.httpIp 缺省 "127.0.0.1"; ip 不再参数化, 域内恒本地回环)。
fn request_dest(config: &ServiceConfig) -> SocketAddr {
    format!("127.0.0.1:{}", config.app_port)
        .parse()
        .expect("requestDest 解析失败")
}

/// Java `Application.requestDestBkp` (`appPortBkp = appPort + 1111`)。
fn request_dest_bkp(config: &ServiceConfig) -> SocketAddr {
    // PORT: Java int 加法静默回绕; u16 域内 app_port>64424 溢出不可达 (缺省 8111),
    // saturating 根除 debug panic 回绕形态 (审查备案)
    format!("127.0.0.1:{}", config.app_port.saturating_add(1111))
        .parse()
        .expect("requestDestBkp 解析失败")
}

impl Service {
    /// 对应 Java 构造器 `public Service(Controller xc)` (L1678-1699)。
    /// PORT: `Controller xc` 参数解散为 (config, fm_manager, bus) 三注入
    /// (环 1 断裂; `freq = xc.serviceLoopIntervalMs` ← config 同名字段)。
    /// 语句顺序逐行保持——clearvaria() 在 mapinfo/sState 构造**之前**执行,
    /// 其尾部的 publishFlightDataEvent 因此读到 mapinfo=null/sState=null
    /// (事件载荷 state=None/mapGrid="--"), 与 Java 同一窗口。
    pub fn new(config: ServiceConfig, fm_manager: Arc<FMManager>, bus: Arc<FlightDataBus>) -> Self {
        let data = Arc::new(RwLock::new(ServiceData::default()));
        let mut svc = Service {
            data: Arc::clone(&data),
            // PORT: SMA 族构造提前到 struct 字面量 (Java 在 resetvaria L1587-1593
            // 构造, 窗口同 1000/freq; 加油重置路径见 reset_varia 的重建)
            deriver: Deriver::new(config.service_loop_interval_ms.max(1) as u64),
            fm_manager,
            bus,
            http_client: HttpHelper::new(&config.http_header),
            focus_monitor: None,
            flight_log: Arc::new(std::sync::Mutex::new(None)),
            stop: Arc::new(AtomicBool::new(false)),
            config,
        };
        {
            let mut d = write_data(&svc.data);
            // Java: freq = xc.serviceLoopIntervalMs;
            d.freq = svc.config.service_loop_interval_ms;
        }
        // Java: clearvaria();
        svc.clear_varia();
        {
            let mut d = write_data(&svc.data);
            // Java: mapinfo = new MapInfo();
            d.mapinfo = Some(MapInfo::new());
            // Java: ratio = freq / 1000.0f; —— long/float 提升为 float 除法,
            // 结果 float 拓宽存入 double 字段 (§2.12 浮点字面量保持)
            let ratio = (d.freq as f32 / 1000.0f32) as f64;
            d.ratio = ratio;
            // Java: ratio_1 = 1.0f - ratio; —— 1.0f 提升为 double 后的 double 减法
            d.ratio_1 = 1.0 - ratio;
            // Java: sState = new State(); sState.init();
            d.s_state = Some(State::new());
            d.s_state.as_mut().unwrap().init();
            // Java: sIndic = new Indicators(); sIndic.init();
            d.s_indic = Some(Indicators::new());
            d.s_indic.as_mut().unwrap().init();
            // Java: power/pitch/thrust/efficiency = new String[State.maxEngNum];
            // (null 填充; 覆盖 resetvaria 落下的 4 长度 nastring 数组, 保真次序)
            d.power = Some(vec![None; MAX_ENG_NUM]);
            d.pitch = Some(vec![None; MAX_ENG_NUM]);
            d.thrust = Some(vec![None; MAX_ENG_NUM]);
            d.efficiency = Some(vec![None; MAX_ENG_NUM]);
            // Java: FuelCheckMili = System.currentTimeMillis();
            d.fuel_check_mili = current_time_millis();
            // isFuelpressure = false;
        }
        svc
    }

    /// 注入焦点监控器 (Java 字段初始化器 `new FocusMonitor()` 的对位物;
    /// 见 struct 字段注)。
    pub fn set_focus_monitor(&mut self, fm: vm_core::focus_monitor::FocusMonitor) {
        self.focus_monitor = Some(fm);
    }

    /// 注入 FlightLog 共享槽 (Controller.start:633 建 Service 前, 对位 Java
    /// Service 轮询读 `c.Log` 的共享字段; 见 struct 字段注)。
    pub fn set_flight_log(&mut self, slot: FlightLogSlot) {
        self.flight_log = slot;
    }

    /// Service.java:1824-1828 的 logTick 调用面:
    /// `if (c.logon) { FlightLog tempLog = c.Log; if (tempLog != null) tempLog.logTick(); }`
    /// PORT: Controller.logon 与 c.Log 的二段判定收敛为槽 Some/None (Java 的
    /// logon 自 openpad 置 true 后无清零写点, 与 Log 非 null 同生同灭)。
    /// 锁序: 槽锁 (clone 后即释) → data 读锁 (快照后即释) → log 锁, 三段不嵌套。
    pub fn flight_log_tick(&self) {
        let temp_log = self
            .flight_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let Some(temp_log) = temp_log else { return };
        let snap = flight_log_snapshot(&read_data(&self.data));
        temp_log.lock().unwrap_or_else(|e| e.into_inner()).log_tick(&snap);
    }

    // ------------------------------------------------------------------
    // resetvaria / clearvaria / resetEngLoad (Java L1510-1666)
    // ------------------------------------------------------------------

    /// 对应 Java `public void clearvaria()` (L1662-1666)。
    fn clear_varia(&mut self) {
        // sState = null;
        // iIndic = null;
        self.reset_varia();
    }

    /// 对应 Java `public void resetvaria()` (L1528-1660)。
    /// PORT(锁序 §2.8): Java 方法体直线赋值无锁; Rust 把字段写入收进一个
    /// write 临界区 (锁内无回调无 IO), `resetEngLoad`/SMA 重建/尾部 publish
    /// 均在锁外——publish 需要读锁, 与未释放的写锁同线程重入即死锁。
    fn reset_varia(&mut self) {
        // R1 周期快照: 本方法（及下传的 resetEngLoad）全程使用这一次取到的句柄,
        // 可能从 Service 轮询线程或构造器调用, current() 均为纯 volatile 读
        let fm = self.fm_manager.current();
        {
            let mut d = write_data(&self.data);
            // PORT(快照字段): Java getter 现读单例 → Rust 读 d.fm 周期快照
            // (service_fields.rs struct 级裁决), 此处随 reset 同步
            d.fm = Arc::clone(&fm);
            // Java: loc = new double[2]; dir = new double[2];
            d.loc = Some([0.0; 2]);
            d.dir = Some([0.0; 2]);
            d.radio_alt_valid = Some(false);
            d.player_live = false;
            d.i_eng_type = ENGINE_TYPE_UNKNOWN;
            d.check_maxium_rpm = 0;
            d.compass_delta = 0.0;
            d.flap_check = 0;
            d.is_downing_flap = false;
            d.get_maximum_rpm = false;
            d.d_radio_alt = 0.0;
            d.cur_load = 0;
            d.wep_time = 0;
            d.energy_j_kg = 0.0;
            d.prev_energy_j_kg = 0.0;
            d.elapsed_time = 0;
            d.altper_circle = 0.0;
            d.check_alt = 0;
            d.altreg = 0.0;
            d.altp = 0.0;
            d.alt = 0.0;
            d.calc_period = 0;
            d.maximum_thr_rpm = 1.0;
            d.max_total_thr = 0;
            d.iastotascoff = 1.0;
            d.thurst_percent = 0.0;
            d.check_engine_flag = false;
            d.check_engine_type = 0;
            // Java: fueltime = Long.MAX_VALUE;
            d.fueltime = i64::MAX;
            d.check_pitch = 0;
            d.fuel_percent = 0;
            d.max_total_hp = 0;
            // Java L1564 对 maxTotalThr 的第二次赋值 (L1555 已赋 0), 保真保留
            d.max_total_thr = 0;
            d.diffspeed = 0.0;
            // Java: curLoadMinWorkTime = 99999 * 1000; —— int 乘法 99999000 拓宽 double
            d.cur_load_min_work_time = (99999 * 1000) as f64;
            /* 刷新引擎工作时间 */
            // (锁外调用, 见下)
            // if(c.getBlkx() != null && c.getBlkx().maxEngLoad !=
            // 0)c.getBlkx().resetEngineLoad();
            let now = current_time_millis();
            // Java: FuelCheckMili = System.currentTimeMillis();
            d.fuel_check_mili = now;
            // Java: lastMapPollTimeMs = FuelCheckMili; lastMainLoopTimeMs = FuelCheckMili;
            d.last_map_poll_time_ms = now;
            d.last_main_loop_time_ms = now;
            d.not_check_inch = false;
            d.altper_circlflag = false;
            // isFuelpressure = false;
            // Java L1577 对 notCheckInch 的第二次赋值, 保真保留
            d.not_check_inch = false;
            d.has_wing_sweep_vario = false;
            // Java: flapAllowSpeed/Angle = Float.MAX_VALUE —— float 拓宽 double (§2.12)
            d.flap_allow_speed = f32::MAX as f64;
            d.flap_allow_angle = f32::MAX as f64;
            d.total_fuel_prev = 0.0;
            d.is_state_jet = false;
            d.nitrokg = 0.0;
            d.nitro_consump = 0.0;
            d.nitro_eng_nr = 0;

            // Java L1587-1593: 7 个 SMA 构造, 窗口 (int)(1000/freq), fuelTimeSMA=4。
            // PORT(状态双主裁决, service_fields.rs 字段区 PORT 注): calc/diff/sep/
            // turnrds 四个 SMA 的真人在 Deriver (其 new/step 已按同窗口与同公式
            // 移植), ServiceData 侧对应槽位**保持 None**——防双胞胎真互相漂移;
            // sum/energyDiff/fuelTime 三个 Java 侧 addNewData 调用已被注释 (仅构造),
            // 按 service_fields 裁决由本波次直接构造:
            let freq = d.freq;
            // Java: (int)(1000/freq) —— long 整除后截断; freq<=0 时 Java 构造器
            // ArithmeticException, Rust 除零 panic 同构崩在构造期 (保真, 不防御)
            let n = (1000 / freq) as usize;
            d.sum_speed_sma = Some(SimpleMovingAverage::new(n));
            d.energy_diff_sma = Some(SimpleMovingAverage::new(n));
            d.fuel_time_sma = Some(SimpleMovingAverage::new(4));

            // R2 守卫: 无 FM 时保持 nitrokg/nitroConsump 归零值（与 updateWepTime 的守卫配套）
            if let Some(blkx) = &fm.blkx {
                // Java: nitrokg = fm.blkx.nitro; nitroConsump = fm.blkx.nitroDecr;
                d.nitrokg = blkx.nitro;
                d.nitro_consump = blkx.nitro_decr;
                // Java: engineLoad[] pL = fm.blx.engLoad; 循环体
                // pL[i].curWaterWorkTimeMili = pL[i].curWaterWorkTimeMili; —— 自赋值
                // 无操作 (保真保留为注释; 真正的会话态改写在 reset_eng_load)
            }

            // Initialize Strings to Defaults
            let na = || Some(NASTRING.to_string());
            d.total_hp_str = na();
            d.total_thrust_str = na();
            d.rpm = na();
            d.total_hp_eff_str = na();
            d.pressure_inch_hg = na();
            d.manifoldpressure = na();
            d.watertemp = na();
            d.oiltemp = na();
            d.total_fuel_str = na();
            d.fueltime_str = na();
            d.s_nitro = na();
            d.s_wep_time = na();
            d.s_eng_work_time = na();
            d.sd_thrust_percent = na();
            d.s_thurst_percent = na();
            d.s_avg_eff = na();
            d.tas = na();
            d.ias = na();
            d.m = na();
            d.aoa = na();
            d.aos = na();
            d.ny = na();
            d.s_n = na();
            d.wx = na();
            d.salt = na();
            d.s_radio_alt = na();
            d.vy = na();
            d.compass = na();
            d.throttle = na();
            d.s_sep = na();
            d.s_sep_abs = na();
            d.s_acc = na();
            d.s_turn_rate = na();
            d.s_turn_rds = na();
            d.s_wing_sweep = na();
            d.flaps = na();
            d.gear = na();
            d.aileron = na();
            d.elevator = na();
            d.rudder = na();
            // Java: svalid = "false";
            d.svalid = Some("false".to_string());

            // Java: efficiency/pitch = new String[4] (nastring 填充)
            // (构造器随后覆盖为 16 长度 null 数组; 加油重置路径则停留在此形状)
            d.efficiency = Some(vec![na(); 4]);
            d.pitch = Some(vec![na(); 4]);
        } // —— write 临界区结束 (publish 前必须释放, §2.8)

        // Java: resetEngLoad(fm); (L1568, 字段赋值序列中间——锁外执行)
        Self::reset_eng_load(&fm);
        // PORT(SMA 重建): Java L1587-1590 的 calc/diff/sep/turnrds 四 SMA 在本
        // 调用点重建 = Deriver 整体重建 (真人在彼, 见上)
        let freq = self.config.service_loop_interval_ms;
        self.deriver = Deriver::new(freq.max(1) as u64);

        // Java: publishFlightDataEvent(); (L1659)
        // Publish initial state immediately
        self.publish_flight_data_event();
    }

    /// 重置引擎耐久计时（engLoad 为共享会话状态, 就地改写语义见 FMHandle javadoc 声明,
    /// "换机 = 新 Blkx 实例" 天然保证会话状态不串机, 此处保持就地改写不变）。
    ///
    /// @param fm 本周期 FM 句柄快照（R1 下传）
    //  (以上 javadoc 逐字保留, Java L1510-1515)
    fn reset_eng_load(fm: &FMHandle) {
        // R2 hasFM 守卫: blkx 非 null 即 READY, 无 FM 时无耐久数据可重置
        // PORT(不可表达, §6 上报不越文件修): 会话态改写
        // `engLoad[idx].curWater/OilWorkTimeMili = WorkTime * 1000` 依赖
        // handle.rs 头注承诺的 "engLoad 就地改写以内部可变性承接" (reader 波次);
        // 现形状 blkx 经 Arc<FMHandle> 共享仅只读, 本方法暂无法落写。
        // TODO(port): engLoad 会话态改写 (blkx reader / 计算方法区波次)
        if let Some(_blkx) = &fm.blkx {
            // Java: for (idx in 0..blkx.maxEngLoad) {
            //   blkx.engLoad[idx].curWaterWorkTimeMili = blkx.engLoad[idx].WorkTime * 1000;
            //   blkx.engLoad[idx].curOilWorkTimeMili   = blkx.engLoad[idx].WorkTime * 1000; }
        }
    }

    // ------------------------------------------------------------------
    // run() 主循环 (Java L1798-1862)
    // ------------------------------------------------------------------

    /// 对应 Java `public void run()` (L1799-1861)。消费 `self` (move 进线程)。
    ///
    /// PORT(§2.13): Java `while(true)` 唯一出口是 `Thread.sleep` 抛
    /// InterruptedException → break; Rust 以 stop 电平标志轮询复刻
    /// (sleep_quietly 提前返回 + 循环内检查)。Java 恢复期 sleep 吞中断的
    /// 失效窗口 (L1857, LIFETIMES 审查修正 7) 在电平标志下天然消失。
    pub fn run(mut self) {
        // Main polling loop with exception recovery
        loop {
            // PORT(§6 契约): Java 顶层 `catch (Exception e)` (L1850) → catch_unwind。
            // AssertUnwindSafe: 与 Java "异常后对象处于不一致状态继续用" 同一
            // 宽松契约 (flight_data_bus.rs 同款论证)。
            match catch_unwind(AssertUnwindSafe(|| self.poll_once())) {
                Ok(Flow::Continue) => {}
                Ok(Flow::Interrupted) => {
                    // Thread was interrupted - exit the loop gracefully
                    logger::info("Service", "Service thread interrupted, exiting...");
                    break; // Exit the while(true) loop
                }
                Err(payload) => {
                    // Unexpected error - log and recover after short delay
                    // Java: "Service error: " + e.getClass().getSimpleName() + " at " +
                    // (e.getStackTrace().length > 0 ? e.getStackTrace()[0] : "unknown")
                    // PORT: Rust panic 无类型名/栈帧槽位, 以消息文本顶位
                    logger::error(
                        "Service",
                        &format!("Service error: {} at unknown", panic_message(payload)),
                    );
                    // Java: e.printStackTrace(); —— 默认 panic hook 已在展开前打印
                    // Java: Thread.sleep(1000) + catch (InterruptedException ignored)
                    // PORT: 可中断恢复睡眠 (§2.13); 置位即提前醒并在此退出
                    // (Java 吞中断后丢失退出信号属已知 bug, 电平标志根治)
                    exception_helper::sleep_quietly(&self.stop, 1000);
                    if self.stop.load(Ordering::SeqCst) {
                        break;
                    }
                }
            }
        }
    }

    /// run() 的 try 块体 (Java L1803-1844), 一轮轮询。
    /// 返回值: Java 的 InterruptedException 出口 → [`Flow::Interrupted`]。
    fn poll_once(&mut self) -> Flow {
        // Java: currentTimeMs = System.currentTimeMillis();
        let now = current_time_millis();
        let (freq, port_ocupied, last_main_loop_time_ms) = {
            let mut d = write_data(&self.data);
            d.current_time_ms = now;
            (d.freq, d.port_ocupied, d.last_main_loop_time_ms)
        };
        // long diffTime = currentTimeMs - lastMainLoopTimeMs;
        let diff_time = now - last_main_loop_time_ms;
        if diff_time >= freq {
            // 尝试GET数据
            // (HTTP IO 在锁外, §2.8; portOcupied 拆箱 None → panic 复刻 Java NPE,
            //  由 run 的 catch_unwind 兜住——字段初始化器 false 使 None 不可达)
            if port_ocupied != Some(true) {
                self.http_client.get_req_result(request_dest(&self.config), &self.stop);
            } else {
                self.http_client.get_req_result(request_dest_bkp(&self.config), &self.stop);
            }
            // Java: actualIntervalMs = (diffTime / freq) * freq;
            let actual_interval_ms = (diff_time / freq) * freq;
            {
                let mut d = write_data(&self.data);
                d.actual_interval_ms = actual_interval_ms;
                // Java: pollCycleDurationMs = actualIntervalMs;
                d.poll_cycle_duration_ms = actual_interval_ms;
                // Java: lastMainLoopTimeMs += actualIntervalMs;
                d.last_main_loop_time_ms += actual_interval_ms;
            }

            // 检查是否需要改变状态
            self.process_polling_cycle();

            // 焦点监控（内部有200ms节流）
            if let Some(fm) = self.focus_monitor.as_mut() {
                fm.tick();
            }

            // 记录
            // Java: if (c.logon) { FlightLog tempLog = c.Log; if (tempLog != null) tempLog.logTick(); }
            // (Service.java:1824-1828, 每轮一次 — 1024 行 flush 节奏在 FlightLog 内)
            self.flight_log_tick();
        }
        let (freq, port_ocupied, last_map_poll_time_ms) = {
            let d = read_data(&self.data);
            (d.freq, d.port_ocupied, d.last_map_poll_time_ms)
        };
        // long diffTime1 = currentTimeMs - lastMapPollTimeMs;
        let diff_time1 = now - last_map_poll_time_ms;
        if diff_time1 >= 10 * freq {
            {
                let mut d = write_data(&self.data);
                // Java: lastMapPollTimeMs = currentTimeMs;
                d.last_map_poll_time_ms = now;
            }
            if port_ocupied != Some(true) {
                self.http_client.get_req_map_obj_result(request_dest(&self.config));
            } else {
                self.http_client.get_req_map_obj_result(request_dest_bkp(&self.config));
            }
            // Java: MapObj.getPlayerLoc(httpClient.strMapObj, loc); getPlayerDir(..., dir);
            let str_map_obj = self.http_client.str_map_obj.clone();
            let mut d = write_data(&self.data);
            if let Some(loc) = d.loc.as_mut() {
                MapObj::get_player_loc(&str_map_obj, loc);
            }
            if let Some(dir) = d.dir.as_mut() {
                MapObj::get_player_dir(&str_map_obj, dir);
            }
        }

        // long sleeptime = currentTimeMs + freq - System.currentTimeMillis();
        let sleeptime = now + freq - current_time_millis();
        if sleeptime > 0 {
            // Java: Thread.sleep(sleeptime); —— InterruptedException → stop 轮询 (§2.13)
            exception_helper::sleep_quietly(&self.stop, sleeptime as u64);
            if self.stop.load(Ordering::SeqCst) {
                return Flow::Interrupted;
            }
        }
        Flow::Continue
    }

    // ------------------------------------------------------------------
    // processPollingCycle (Java L1701-1796)
    // ------------------------------------------------------------------

    /// Processes one polling cycle: updates state, calculates data, and publishes events.
    /// Previously named checkState() - renamed for clarity.
    /// (以上 javadoc 逐字保留, Java L1701-1704)
    fn process_polling_cycle(&mut self) {
        // int conState;
        let con_state: i32;
        {
            let mut d = write_data(&self.data);
            // 更新时间戳
            d.time_stamp = d.current_time_ms;
        }
        // Application.debugPrint("s:"+httpClient.strState+"s1:"+httpClient.strIndic);
        // 更新state

        // Java: c.initStatusBar();
        // TODO(port): Controller 状态条初始化 (Controller 波次)

        // Java: if (httpClient.strState.length() > 0 && httpClient.strIndic.length() > 0)
        // (strState 为跨线程 Arc<Mutex>——getReqResult 的子线程写, 读侧短锁克隆;
        //  锁中毒穿透与 read_data/write_data 同一 §6 策略——该 Mutex 与子线程共享,
        //  持锁 panic 不得令 Service 线程陷 "每轮 panic-恢复" 的活而死循环)
        let str_state = self
            .http_client
            .str_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let str_indic = self.http_client.str_indic.clone();
        if !str_state.is_empty() && !str_indic.is_empty() {
            // 改变状态为连接成功
            // Application.debugPrint(sState);
            let (s_flag, i_flag);
            {
                let mut d = write_data(&self.data);
                // Java: conState = sState.update(httpClient.strState);
                // (sState 构造器恒建, unwrap 复刻 Java 的 null 不可达域)
                con_state = d.s_state.as_mut().unwrap().update(&str_state);
                // Java: sIndic.update(httpClient.strIndic);
                d.s_indic.as_mut().unwrap().update(&str_indic);
                s_flag = d.s_state.as_ref().unwrap().flag;
                i_flag = d.s_indic.as_ref().unwrap().flag;
            }
            // Java: c.changeS2();
            // TODO(port): Controller 连接成功状态切换 (Controller 波次)
            if s_flag && i_flag {
                // 读取本轮判定所需快照 (锁外判, §2.8)
                let (i_type, total_thr, s_rpm, player_live, port_ocupied) = {
                    let d = read_data(&self.data);
                    (
                        d.s_indic.as_ref().unwrap().r#type.clone(),
                        d.s_state.as_ref().unwrap().total_thr,
                        d.s_state.as_ref().unwrap().rpm,
                        d.player_live,
                        d.port_ocupied,
                    )
                };
                /* 修复录像中没法使用的问题 */
                // Java: (!sIndic.type.equals("DUMMY_PLANE")) —— type 经 update 已
                // toUpperCase; Rust 侧 Indicators::update 恒产 Some (缺失→""),
                // None 不可达, Option 域内比较 (vm-core indicators.rs 既有防御)
                if i_type.as_deref() != Some("DUMMY_PLANE")
                    && ((total_thr != 0.0) || (s_rpm != 0))
                {
                    if !player_live {
                        // Java: if (!portOcupied) getReqMapInfoResult(requestDest);
                        // else getReqMapInfoResult(requestDestBkp);
                        let dest = if port_ocupied != Some(true) {
                            request_dest(&self.config)
                        } else {
                            request_dest_bkp(&self.config)
                        };
                        // (HTTP IO 在锁外, §2.8)
                        self.http_client.get_req_map_info_result(dest);
                        let mut d = write_data(&self.data);
                        // Java: mapinfo.update(httpClient.strMapInfo);
                        let str_map_info = self.http_client.str_map_info.clone();
                        d.mapinfo.as_mut().unwrap().update(&str_map_info);
                        // Application.debugPrint("grid_zero: " + mapinfo.grid_zeroX + ", " +
                        // mapinfo.grid_zeroY);
                    }
                    let mut d = write_data(&self.data);
                    // Java: playerLive = true;
                    d.player_live = true;
                }

                let player_live = {
                    let d = read_data(&self.data);
                    d.is_player_live()
                };
                if player_live {
                    // 读取map info

                    // Java: c.changeS3();// 打开面板
                    // TODO(port): Controller 打开面板 (Controller 波次)
                    // P4 换机轻量 swap: 只换 FM 句柄（FMManager 负责去重/负缓存/异步加载），
                    // 不再重启 Controller——旧版 S4toS1 重启销毁全部 overlay 致 HUD 闪断，且与
                    // 旧 FM 回退逻辑叠加曾构成 issue #55 换机死循环（P2 已断根，P4 删重启路径）。
                    // identify/onAircraftChanged 同目标零成本，10Hz 轮询安全。
                    {
                        let d = read_data(&self.data);
                        // Java: FMManager.getInstance().identify(sIndic.type);
                        let plane_type = d.s_indic.as_ref().unwrap().r#type.clone();
                        drop(d);
                        self.fm_manager.identify(plane_type.as_deref());
                        // R1: ServiceData.fm 周期句柄快照 (get_total_weight/has_wep
                        // 读它, service_fields.rs 裁决) —— current() 取在 data 写锁外,
                        // 锁序恒 data→fm 单向 (与 calculate()/reset_varia() 同款,
                        // 杜绝 fm 侧未来持锁发布时的 ABBA 形态)
                        let fm_cur = self.fm_manager.current();
                        let mut d = write_data(&self.data);
                        d.fm = fm_cur;
                    }
                    // Java: c.onAircraftChanged(sIndic.type);
                    // TODO(port): Controller 换机回调 (Controller 波次)
                    // speedvp = sState.IAS;
                    // 开始计算数据
                    self.calculate();

                    // 检测到加油，重置数据
                    {
                        let d = read_data(&self.data);
                        // Java: Math.abs(speedv) < 10 — speedv 状态主在 Deriver
                        // (updateEngineState 同源的 speedv() 外泄面; ServiceData
                        // 的 speedv 字段保持死存储, 不构成第二真相)
                        let speedv = self.deriver.speedv();
                        let total_fuel = d.total_fuel;
                        let total_fuel_prev = d.total_fuel_prev;
                        if (speedv.abs() < 10.0) && (total_fuel - total_fuel_prev > 1.0) {
                            // (临界区内不做 IO——先释放读锁再打日志, 头部 §2.8 自律)
                            drop(d);
                            // Java: String.format("Refueling detected (Fuel: %.1f -> %.1f).
                            // Resetting simulation variables.", ...)
                            // —— Formatter %.1f HALF_UP → format::format (§2.3)
                            logger::info(
                                "Service",
                                &format!(
                                    "Refueling detected (Fuel: {} -> {}). Resetting simulation variables.",
                                    format::format(total_fuel_prev, 1),
                                    format::format(total_fuel, 1)
                                ),
                            );
                            self.reset_varia();
                        }
                    }

                    // 0.5秒一次慢计算
                    {
                        let d = read_data(&self.data);
                        let freq = d.freq;
                        let calc_period = d.calc_period;
                        drop(d);
                        let mut d = write_data(&self.data);
                        // Java: ((calcPeriod++) % (500 / freq)) == 0 —— 后缀自增
                        d.calc_period += 1;
                        if calc_period % (500 / freq) == 0 {
                            // Java: slowcalculate((500 / freq) * freq) — 油耗慢计算
                            // + totalFuelPrev 追赶 (加油检测的 prev 写点)
                            let dtime = (500 / freq) * freq;
                            drop(d);
                            self.slow_calculate(dtime);
                        }
                    }

                    // 将数据转换格式
                    // TODO(port): formatDataAsStrings() —— 全量显示字符串格式化
                    // (计算方法区波次; 其尾部对 publishFlightDataEvent 的调用
                    // 由下方直接调用顶位, 发布时序不变——Java L431)
                    self.publish_flight_data_event();

                    // 写入文档
                    // c.writeDown();

                    // 检查死亡
                    {
                        let d = read_data(&self.data);
                        let total_thr = d.s_state.as_ref().unwrap().total_thr;
                        let rpm = d.s_state.as_ref().unwrap().rpm;
                        let ias = d.s_state.as_ref().unwrap().ias;
                        // Java: sState.totalThr == 0 && sState.RPM <= 0 && sState.IAS < 10
                        if total_thr == 0.0 && rpm <= 0 && ias < 10 {
                            // (临界区内不做 IO——先释放读锁再打日志, 头部 §2.8 自律)
                            drop(d);
                            logger::warn(
                                "Service",
                                "Player crash/stop detected. Simulation state invalidated.",
                            );
                            let mut d = write_data(&self.data);
                            d.player_live = false;
                        }
                    }
                }
            } else {
                // 状态置为等待游戏开始（状态1）
                // c.changeS2();//连接成功等待游戏开始

                // Java: c.S4toS1();
                // TODO(port): Controller 等待游戏开始状态 (Controller 波次)
                // 等待游戏开始
                exception_helper::sleep_quietly(&self.stop, 500);
            }
        } else {
            // 状态置为等待连接中
            con_state = -1;
            // Java: c.S4toS1();
            // TODO(port): Controller 等待连接状态 (Controller 波次)
            logger::debug("Service", "Waiting for game connection (8111/9222)...");
        }
        if con_state == -1 {
            // 端口连接可能有问题，切换端口
            // Application.debugPrint("切换端口\n");
            let mut d = write_data(&self.data);
            // Java: portOcupied = !portOcupied; (Boolean 拆箱→取反→装箱)
            d.port_ocupied = Some(d.port_ocupied != Some(true));
        }
    }

    // ------------------------------------------------------------------
    // calculate (Java L1115-1178) —— 本波次 Deriver 接线形态
    // ------------------------------------------------------------------

    /// 对应 Java `public void calculate()` (L1115-1178)。
    ///
    /// Java 链 17 个子方法中, 已译 [`Deriver`] 覆盖: updateClimbRate (L777) /
    /// updateSpeed (L840) / updateTurn (L788) / updateSEP (L986) 四公式族 +
    /// mach (updateSpeedRatio L1213-1215 的手动大气模型, R2 hasFM 守卫在写回段);
    /// updateCompass (L1101, 含 compass==-65535 的地图方向回退) / updateAlt
    /// (L739, 英制检测状态机 + 无线电高度有效性/英尺转米 + dRadioAlt 差分)
    /// 在写回段逐行落地。其余子方法属计算方法区后续波次:
    /// TODO(port): updateWepTime/updateTemp/checkOverheat/updateEngineState/
    /// updateFuel/checkWing/checkFlap/getMaximumRPM/updateSpeedRatio(比值段)/
    /// updateStallSpeed/updateOptimalCompressorStage
    /// TODO(port): Deriver 内 speedv/speedvp 未外泄的三处活代码——updateSpeed 尾部
    /// IASv/IASvp/TASv 写回、updateTurn 的 horizontalLoad (L821-826)、updateSEP 尾部
    /// energyJKg/energyM (L1024-1025); 消费方 formatDataAsStrings/HUDCalculator 同在
    /// 计算方法区波次, 届时一并接线
    fn calculate(&mut self) {
        // R1 周期快照（P3 迁移核心规则）: 整个 calculate 链路共用开头取到的一次 FM 句柄,
        // 并以参数下传给所有依赖 FM 的子方法 —— 保证单周期内全部 FM 派生量来自同一
        // Blkx 实例。FMManager.current() 是纯 volatile 读（无锁无 IO）; 换机时句柄由
        // loader 线程原子替换, 本周期内可能取到旧句柄（平滑过渡, 下一周期自然切换）
        let fm = self.fm_manager.current();

        // 获得开始时间
        // Java: elapsedTime = currentTimeMs - startTime;
        let actual_interval_ms;
        {
            let mut d = write_data(&self.data);
            d.fm = Arc::clone(&fm);
            d.elapsed_time = d.current_time_ms - d.start_time;
            // Java updateSEP/updateAlt 的分母是 actualIntervalMs (run() L1804 的区间
            // 量化值, HTTP 慢于一个周期时 = 2*freq 及以上)——传 freq 会令卡顿轮
            // 加速度/SEP 成倍失真
            actual_interval_ms = d.actual_interval_ms;
        }

        // Java calculate 链头两步 (L1125-1129): 增加 WEP 时间 / 更新温度
        // (顺序在 updateCompass 之前 — Rust 侧 Deriver step 之前同位)
        self.update_wep_time(&fm);
        self.update_temp();

        // 增加wep时间 / 更新温度，优先使用更精确的 / 检查是否过热… (TODO 列表见 doc)
        // 更新方向 / 更新爬升率 / 获得准确高度 / 更新速度 / 更新转弯半径 —— Deriver::step
        // (updateCompass/updateAlt 的非公式部分在下方写回段逐行落地)
        let (values, vy, radio_alt_raw, alt10k, dir) = {
            let d = read_data(&self.data);
            let s = to_state_raw(d.s_state.as_ref().unwrap());
            let i = to_indicators_raw(d.s_indic.as_ref().unwrap());
            // 写回段状态机输入: altitude_10k (IndicatorsRaw 无此槽, 取自保真版) /
            // dir (run() 的 getPlayerDir 产物) / 原始 radio_altitude (哨兵判定,
            // FlightValues.radio_altitude 已是回退后的值)
            let alt10k = d.s_indic.as_ref().unwrap().altitude_10k;
            let dir = d.dir;
            let vy = s.vy;
            let radio_alt_raw = i.radio_altitude;
            // (锁内只做字段拷贝, step 计算在锁外——§2.8 锁粒度)
            drop(d);
            let values = self.deriver.step(&s, &i, actual_interval_ms as f64);
            (values, vy, radio_alt_raw, alt10k, dir)
        };

        // 写回派生量 (FlightValues → ServiceData 字段, 来源映射见各字段)
        {
            let mut d = write_data(&self.data);
            // 整包快照 (FlightInfo overlay 数据源; 与下方散字段同源同值)
            d.flight_values = values;
            // R2 hasFM 守卫 (Java updateSpeedRatio L1191-1199): 无 FM 时整方法早退,
            // mach 保持上轮值 (初始 0)——否则无 FM 机型 mach 非 0, 破坏
            // hide-when-zero 显示行为
            if fm.blkx.is_some() {
                d.mach = values.mach;
            }
            // nVy ← vario (updateClimbRate)
            d.n_vy = values.vario;
            // An ← ny*G (updateTurn; FlightValues.ny = An/G, 往返还原)
            d.an = values.ny * G;
            d.sep = values.sep;
            d.acceleration = values.acceleration;
            d.turn_rate = values.turn_rate;
            // PORT: FlightValues.turn_radius = |turnRds| (已取绝对值);
            // get_turn_radius() 再 abs 无差 (abs 幂等), 带符号值丢失不改变任何
            // 现有读者行为 (全库读点均经 abs 或与 9999 比较)
            d.turn_rds = values.turn_radius;

            // Java: updateCompass (L1101-1113)
            // 如果有仪表罗盘，读取仪表罗表盘数据
            if values.compass != F_INVALID {
                d.compass_delta = values.compass;
            } else {
                // 否则读取地图中的方向数据 (dir 由 run() 的 getPlayerDir 持续更新;
                // resetvaria 恒建数组 → unwrap 复刻 Java 的 null 不可达域)
                let dir = dir.unwrap();
                if dir[1] < 0.0 {
                    d.compass_delta = (360.0 - (dir[0] / dir[1]).atan().to_degrees()) % 360.0;
                } else {
                    d.compass_delta = 180.0 - (dir[0] / dir[1]).atan().to_degrees();
                }
            }

            // Java: updateAlt (L739-775) —— 获得准确高度, 需依赖 Vy 因此位于爬升率后
            // altp = alt; alt = sState.heightm;
            d.altp = d.alt;
            d.alt = values.altitude;
            // altmeterp = altmeter; altmeter = sIndic.altitude_10k;
            d.altmeterp = d.altmeter;
            d.altmeter = alt10k;

            // 人类毒瘤英制飞机
            if !d.not_check_inch && vy.abs() > 0.0 {
                if (d.altmeter - d.altmeterp).abs() * 1000.0
                    > (2.0 * vy * actual_interval_ms as f64).abs()
                {
                    // checkAlt += actualIntervalMs —— int += long 复合赋值隐式窄化
                    // 为 (int)(long 和) 低 32 位截断 (§2.2 双转)
                    d.check_alt = (((d.check_alt as i64).wrapping_add(actual_interval_ms))
                        as u64 as u32) as i32;
                } else {
                    d.check_alt = (((d.check_alt as i64).wrapping_sub(actual_interval_ms))
                        as u64 as u32) as i32;
                }
                // Java Math.abs(Integer.MIN_VALUE)=MIN_VALUE 溢出语义 → wrapping_abs
                if d.check_alt.wrapping_abs() > 10000 {
                    d.not_check_inch = true;
                }
            }

            // 无线电高度
            d.p_radio_alt = d.radio_alt;
            // radioAlt = iIndic.radio_altitude;
            if radio_alt_raw == F_INVALID {
                d.radio_alt = d.alt;
                d.radio_alt_valid = Some(false);
            } else {
                d.radio_alt_valid = Some(true);
                if d.check_alt > 0 {
                    // radioAlt = sIndic.radio_altitude * 0.3048f —— float 字面量
                    // 先扩为 double 再乘 (§2.12: 0.3048f32 as f64 ≠ 0.3048)
                    d.radio_alt = radio_alt_raw * (0.3048f32 as f64);
                } else {
                    d.radio_alt = radio_alt_raw;
                }
            }
            // dRadioAlt = (ratio_1 * dRadioAlt) + ratio * 1000.0f * (radioAlt - pRadioAlt) / actualIntervalMs;
            d.d_radio_alt = (d.ratio_1 * d.d_radio_alt)
                + d.ratio * 1000.0 * (d.radio_alt - d.p_radio_alt) / actual_interval_ms as f64;

            // PORT(speedv/speedvp 不写回): 状态主在 Deriver 内部且 FlightValues
            // 未外泄——加油检测分支本波次恒死 (见 process_polling_cycle 注),
            // TODO(port): 计算方法区波次裁决外泄或迁移
        }

        // Java calculate 链 L1134-1136: updateEngineState (总功率/推力/百分比)
        // + updateFuel (总油量) — EngineInfo/EngineControl 面板数据源。
        // PORT(顺序备案): Java 在 updateTurn 与 updateSEP 之间调用, Rust 的
        // Deriver::step 将四公式族并成一步, 无法插中间 — 两方法不读 SEP 族字段,
        // 置于 step 写回后行为等价; speedv 取本轮 Deriver 值 (Java 同轮字段读)
        self.update_engine_state(&fm);
        self.update_fuel();

        // Java calculate 尾部两比值方法 (L1177-1178): 速度/马赫临界比值 + 失速速度
        // — MiniHUD 速度比值 bar 的数据源 (speed_limit_ratio 等 5 字段)
        self.update_speed_ratio(&fm);
        self.update_stall_speed(&fm);
    }

    /// 对应 Java `public void slowcalculate(long dtime)` (L517-560) — 0.5 秒一次
    /// 慢计算: 油量变化率/剩余油量时间 + **totalFuelPrev 追赶** (加油检测分支的
    /// prev 写点 — 未移植时 prev 恒 0, totalFuel 非零即每轮误判"加油"触发
    /// resetvaria, player_live 永假)。
    /// @param dtime (500/freq)*freq (Java 调用点 L1747)
    fn slow_calculate(&mut self, dtime: i64) {
        // 单一写锁临界区: fuelTimeSMA 的 addNewData 需 &mut (状态量更新), 临界区内
        // 仅纯内存计算无 IO/回调 (Java 本方法无锁直写, §2.8 锁粒度等价收紧)
        let mut d = write_data(&self.data);
        // Java: fuelDelta = (totalFuelPrev - totalFuel) / dtime — double/int
        let fuel_delta = (d.total_fuel_prev - d.total_fuel) / dtime as f64;
        if fuel_delta > 0.0 {
            d.fuelchange_time = d.last_main_loop_time_ms - d.fuel_lastchange_mili;
            d.fuel_lastchange_mili = d.last_main_loop_time_ms;
            d.fuel_change = d.total_fuel_prev - d.total_fuel; // 改变1公斤花了多长时间 (Java 注释原文)

            // fuelTimeSMA 的真人在 ServiceData 侧 (状态双主边界: 仅构造的
            // 三个 SMA 归 ServiceData, resetvaria 恒建 Some)
            let mut sma = d.fuel_time_sma.take().unwrap();
            if !d.low_acc_fuel {
                // 改用滑动平均 (Java 注释原文)
                d.fueltime = sma.add_new_data(d.total_fuel / fuel_delta) as i64;
            } else {
                // /* 已知油量不可能递增，考虑计算精度问题导致油量增多，因此取两者间最小值 */ (Java 注释原文)
                let tmpft =
                    sma.add_new_data(d.total_fuel * d.fuelchange_time as f64 / d.fuel_change) as i64;
                if d.fueltime > 0 {
                    // Java: fueltime < tmpft ? fueltime : tmpft
                    d.fueltime = if d.fueltime < tmpft { d.fueltime } else { tmpft };
                } else {
                    d.fueltime = tmpft;
                }
            }
            d.fuel_time_sma = Some(sma);
        } else {
            // 没有变化，使用上次 (Java 注释原文)
            if d.fuel_change == 0.0 {
                d.fueltime = 0;
            } else {
                let mut sma = d.fuel_time_sma.take().unwrap();
                let tmpft =
                    sma.add_new_data(d.total_fuel * d.fuelchange_time as f64 / d.fuel_change) as i64;
                d.fuel_time_sma = Some(sma);
                d.fueltime = tmpft;
            }
        }
        d.fuel_delta = fuel_delta;
        if d.fueltime < 0 {
            d.fueltime = i64::MAX;
        }
        // Java: FuelCheckMili = lastMainLoopTimeMs; totalFuelPrev = totalFuel;
        d.fuel_check_mili = d.last_main_loop_time_ms;
        d.total_fuel_prev = d.total_fuel;
    }

    // ------------------------------------------------------------------
    // updateWepTime / updateTemp / checkEngineJet / updateEngineState /
    // updateFuel (Java L707-723 / L726-737 / L484-514 / L883-962 / L964-984)
    // — EngineInfo/EngineControl 面板的功率/动力量/油量/温度数据源
    // ------------------------------------------------------------------

    /// 对应 Java `public void updateWepTime(FMHandle fm)` (L707-723)。
    /// @param fm 本周期 FM 句柄快照（R1 下传, Java javadoc 原文）
    fn update_wep_time(&mut self, fm: &FMHandle) {
        // 输入快照 (锁外判, §2.8): engineNum/throttles 读一轮
        let engine_num = {
            let d = read_data(&self.data);
            let s = d.s_state.as_ref().unwrap();
            let mut nitro_eng_nr = 0i32;
            let mut wep_time = d.wep_time;
            let n = s.engine_num;
            for i in 0..n as usize {
                // Java: sState.throttles[i] 越界 (engineNum > throttles 长度) 抛
                // AIOOBE → run 顶层 catch; 索引 panic 同收敛 (保真)
                if s.throttles[i] > 100 {
                    // 进入Wep状态 (Java 注释原文)
                    wep_time += d.poll_cycle_duration_ms;
                    nitro_eng_nr += 1;
                }
            }
            (n, nitro_eng_nr, wep_time)
        };
        let (n, nitro_eng_nr, wep_time) = engine_num;
        let mut d = write_data(&self.data);
        d.engine_num = n;
        d.nitro_eng_nr = nitro_eng_nr;
        d.wep_time = wep_time;
        // R2 守卫: 无 FM 时 blkx=null, nitrokg 归 0（显示 "-"）(Java 注释原文)
        d.nitrokg = if let Some(blkx) = fm.blkx.as_ref() {
            let v = blkx.nitro - (d.wep_time as f64 * d.nitro_consump) / 1000.0;
            if v < 0.0 {
                0.0
            } else {
                v
            }
        } else {
            0.0
        };
    }

    /// 对应 Java `public void updateTemp()` (L726-737) — 更新温度，优先使用更精确的。
    fn update_temp(&mut self) {
        let (noil, nwater) = {
            let d = read_data(&self.data);
            let i = d.s_indic.as_ref().unwrap();
            let s = d.s_state.as_ref().unwrap();
            let mut noil_temp = i.oil_temp;
            let mut nwater_temp = i.water_temp;
            if noil_temp <= -65534.0 {
                noil_temp = s.oiltemp;
            }
            if nwater_temp <= -65534.0 {
                nwater_temp = i.engine_temperature;
                if nwater_temp <= -65534.0 {
                    nwater_temp = s.watertemp;
                }
            }
            (noil_temp, nwater_temp)
        };
        let mut d = write_data(&self.data);
        d.noil_temp = noil;
        d.nwater_temp = nwater;
    }

    /// 对应 Java `public void checkEngineJet()` (L484-514) — 磁电机/桨距投票
    /// 状态机 (~5 秒收敛), 置 iEngType + checkEngineFlag。
    fn check_engine_jet(&mut self) {
        // TODO:自适应方式获得,由磁电机判断. 只有活塞才有磁电机 (Java 注释原文)
        let mut d = write_data(&self.data);
        if !d.check_engine_flag {
            // 输入快照先行 (s 的不可变借用与 d 的字段写互斥, 拆两段)
            let (magenato, pitch0) = {
                let s = d.s_state.as_ref().unwrap();
                (s.magenato, s.pitch[0])
            };
            if magenato < 0 {
                d.check_engine_type -= 1;
            } else {
                d.check_engine_type += 1;
            }
            // Java: sState.pitch[0] — 空 Vec 索引 AIOOBE → run 顶层 catch (保真)
            if pitch0 != -65535.0 {
                d.check_pitch += 1;
            } else {
                d.check_pitch -= 1;
            }

            if d.check_engine_type.wrapping_abs() >= 100 {
                d.check_engine_flag = true;
                if d.check_engine_type >= 0 {
                    d.i_eng_type = ENGINE_TYPE_PROP;
                } else {
                    // 涡桨 (Java 注释)
                    if d.check_pitch > 0 {
                        d.i_eng_type = ENGINE_TYPE_TURBOPROP;
                    } else {
                        d.i_eng_type = ENGINE_TYPE_JET;
                    }
                }
            }
        }
    }

    /// 对应 Java `public void updateEngineState(FMHandle fm)` (L883-962) —
    /// 计算总功率/推力及推力百分比。EngineInfo/EngineControl 的核心数据源。
    ///
    /// @param fm 本周期 FM 句柄快照（R1 下传, Java javadoc 原文）
    fn update_engine_state(&mut self, fm: &FMHandle) {
        self.check_engine_jet();
        // speedv (校正 TAS m/s) — Deriver 本轮值 (Java 字段直读的对应物)
        let speedv = self.deriver.speedv();

        // 输入快照 + 引擎循环 (锁外算, §2.8)
        let (is_jet, total_hp, total_hp_eff, total_thrust, avgeff) = {
            let d = read_data(&self.data);
            let is_jet = d.i_eng_type == ENGINE_TYPE_JET;
            let s = d.s_state.as_ref().unwrap();
            let engine_num = d.engine_num as usize;
            if !is_jet {
                // 活塞机或者涡浆机 (Java 注释原文)
                let mut ttotalhp = 0.0f64;
                let mut ttotalhpeff = 0.0f64;
                let mut ttotalthr = 0.0f64;
                for i in 0..engine_num {
                    ttotalthr += s.thrust[i] as f64;
                    ttotalhp += s.power[i];
                    ttotalhpeff += s.thrust[i] as f64 * G * speedv / 735.0;
                }
                // Java: (int) 截断向零 ↔ as i32 (§2.2)
                let total_hp = ttotalhp as i32;
                let total_hp_eff = ttotalhpeff as i32;
                let total_thrust = ttotalthr as i32;
                // Java: (double) 100 * ... int 提升回 double
                let avgeff = if total_hp != 0 {
                    100.0 * total_hp_eff as f64 / total_hp as f64
                } else {
                    0.0
                };
                (is_jet, total_hp, total_hp_eff, total_thrust, avgeff)
            } else {
                // 喷气机 (Java 注释原文)
                let mut ttotalthr = 0.0f64;
                for i in 0..engine_num {
                    ttotalthr += s.thrust[i] as f64;
                }
                let ttotalhpeff = (ttotalthr * G * speedv) / 735.0;
                // 元组槽位: (is_jet, total_hp, total_hp_eff, total_thrust, avgeff)
                (is_jet, 0, ttotalhpeff as i32, ttotalthr as i32, 0.0)
            }
        };

        {
            let mut d = write_data(&self.data);
            d.total_hp = total_hp;
            d.total_hp_eff = total_hp_eff;
            d.total_thrust = total_thrust;
            d.avgeff = avgeff;

            let throttle = d.s_state.as_ref().unwrap().throttle;
            // Java: maxTotalThr = (int)(ratio_1*maxTotalThr + ratio*totalThrust) —
            // int 提升 double 运算后 (int) 截断
            if d.max_total_thr < total_thrust && throttle >= 100 {
                d.max_total_thr =
                    (d.ratio_1 * d.max_total_thr as f64 + d.ratio * total_thrust as f64) as i32;
            }
            if d.max_total_hp < total_hp_eff && throttle >= 100 {
                d.max_total_hp =
                    (d.ratio_1 * d.max_total_hp as f64 + d.ratio * total_hp_eff as f64) as i32;
            }

            d.p_thurst_percent = d.thurst_percent;

            // R1: 峰值缓存直取句柄 (非 READY 两者 0 → 走 maxTotal 回退)
            let peak_power = if fm.blkx.is_some() { fm.peak_wep_power } else { 0.0 };
            let peak = if fm.blkx.is_some() { fm.peak_thrust } else { 0.0 };

            if is_jet {
                // Jet: current thrust / peak afterburner thrust (Java 注释原文)
                if peak > 0.0 {
                    d.thurst_percent = 100.0 * total_thrust as f64 / peak;
                } else if d.max_total_thr != 0 {
                    // Fallback to old algorithm (Java 注释原文)
                    d.thurst_percent = 100.0 * total_thrust as f64 / d.max_total_thr as f64;
                }
            } else {
                // Piston: current power / peak WEP power (Java 注释原文)
                if peak_power > 0.0 {
                    d.thurst_percent = 100.0 * total_hp as f64 / peak_power;
                } else if d.max_total_hp != 0 {
                    d.thurst_percent = 100.0 * total_hp as f64 / d.max_total_hp as f64;
                }
            }

            // Java: ... * 1000.0f / actualIntervalMs — 1000.0f float 字面量提升 (§2.12)
            let interval = d.actual_interval_ms;
            d.t_eng_response = (d.ratio_1 * d.t_eng_response)
                + d.ratio * (d.thurst_percent - d.p_thurst_percent) * (1000.0f32 as f64)
                / interval as f64;
        }
    }

    /// 对应 Java `public void updateFuel()` (L964-984) — 计算总油量。
    fn update_fuel(&mut self) {
        let (total_fuel, low_acc_fuel, fuel_percent) = {
            let d = read_data(&self.data);
            let i = d.s_indic.as_ref().unwrap();
            let s = d.s_state.as_ref().unwrap();
            let mut total_fuel = 0.0f64;
            let mut low_acc_fuel = false;
            if i.fuelnum != 0 {
                /* 修复su-27油箱显示不正确的问题 (Java 注释原文) */
                // Java: for (i = 0; i < 1; i++) — 循环上界写死 1 (只累加 fuel[0],
                // 注释掉的 fuelnum 上限是旧逻辑), 保真直译
                for k in 0..1 {
                    total_fuel += i.fuel[k];
                }
            }
            if total_fuel == 0.0 {
                low_acc_fuel = true;
                total_fuel = s.mfuel;
            }
            // Java: (int) (100 * totalFuel / sState.mfuel0) — double 除法截断
            let fuel_percent = (100.0 * total_fuel / s.mfuel0) as i32;
            (total_fuel, low_acc_fuel, fuel_percent)
        };
        let mut d = write_data(&self.data);
        d.total_fuel = total_fuel;
        d.low_acc_fuel = low_acc_fuel;
        d.fuel_percent = fuel_percent;
        // Java updateFuel 尾部未回写 totalFuelPrev (slowcalculate 的差分输入),
        // totalFuelPrev 写点在 slowcalculate (TODO(port) 慢计算波次)
    }

    // ------------------------------------------------------------------
    // updateSpeedRatio / updateStallSpeed (Java L1185-1231 / L1236-1266)
    // ------------------------------------------------------------------

    /// 对应 Java `public void updateSpeedRatio(FMHandle fm)` (L1185-1231) —
    /// 计算速度/马赫与临界值比值及舵面锁定比值。
    ///
    /// PORT(mach 单写者): Java L1213-1215 的手动大气模型 mach 写字段 — Rust 侧
    /// 同公式已由 Deriver 承接且写回段带 R2 hasFM 守卫 (本方法早退域与 Deriver
    /// 写回守卫同域, 值恒一致), 此处 mach 为局部量不再写字段 (防双写者漂移)。
    /// @param fm 本周期 FM 句柄快照（R1 下传, Java javadoc 原文）
    fn update_speed_ratio(&mut self, fm: &FMHandle) {
        // R2 hasFM 守卫 (Java L1194-1198): 无 FM 时比值归零（UI 端 hide-when-zero 隐藏）
        let Some(blkx) = fm.blkx.as_ref() else {
            let mut d = write_data(&self.data);
            d.speed_limit_ratio = 0.0;
            d.aileron_lock_ratio = 0.0;
            d.rudder_lock_ratio = 0.0;
            return;
        };

        let mut wing_sweep = 0.0f64;
        // 锁外快照输入 (§2.8): wsweep/ias/heightm 三读一写锁内取, 计算锁外
        let (ias, height_m) = {
            let d = read_data(&self.data);
            if d.is_wing_sweep_valid() {
                wing_sweep = d.s_indic.as_ref().unwrap().wsweep_indicator;
            }
            (d.get_ias(), d.s_state.as_ref().unwrap().heightm)
        };

        let ias_limit = blkx.get_vne_v_wing(wing_sweep);
        let mach_limit = blkx.get_mne_v_wing(wing_sweep);
        let aileron_lock_speed = blkx.aileron_eff;
        let rudder_lock_speed = blkx.rudder_eff;

        // 1. 根据地球大气模型计算mach (Java 注释原文)
        let ias_per_mach = 3.6 * (1.4 / 1.225 * 101325.0
            * (1.0 - 0.0000225577 * height_m).powf(5.25588))
            .sqrt();
        let mach = ias / ias_per_mach;

        // 2. 计算速度比值 (Java 注释原文)
        let ias_ratio = ias / ias_limit;
        let mach_ratio = mach / mach_limit;
        // 3. 计算更大的速度 (Java 注释原文)
        let mut d = write_data(&self.data);
        if ias_per_mach == 0.0 || ias_ratio >= mach_ratio {
            d.speed_limit_ratio = ias_ratio;
            d.aileron_lock_ratio = aileron_lock_speed / ias_limit;
            d.rudder_lock_ratio = rudder_lock_speed / ias_limit;
            d.unit_mach_limit_ratio = ias_per_mach / ias_limit;
        } else {
            d.speed_limit_ratio = mach_ratio;
            d.aileron_lock_ratio = aileron_lock_speed / (mach_limit * ias_per_mach);
            d.rudder_lock_ratio = rudder_lock_speed / (mach_limit * ias_per_mach);
            d.unit_mach_limit_ratio = 1.0 / mach_limit;
        }
    }

    /// 对应 Java `public void updateStallSpeed(FMHandle fm)` (L1236-1266) —
    /// 计算失速速度。
    ///
    /// PORT(flap 来源): Java `flap` 字段由 checkFlap L1045 `flap = sState.flaps`
    /// 赋值 (唯一写点, 恒等) — checkFlap 未移植, 此处直读 s_state.flaps 同值。
    /// @param fm 本周期 FM 句柄快照（R1 下传, Java javadoc 原文）
    fn update_stall_speed(&mut self, fm: &FMHandle) {
        // R2 hasFM 守卫 (Java L1243-1245): 无 FM 时保持上次值/初始值 0（UI 端按无效值隐藏）
        let Some(blkx) = fm.blkx.as_ref() else {
            return;
        };
        let Some(nf) = blkx.no_flaps_wing.as_ref() else {
            return; // doLoad=false 形态的占位 blkx (翼数据未装载)
        };
        let Some(ff) = blkx.full_flaps_wing.as_ref() else {
            return;
        };
        let Some(fu) = blkx.fuselage.as_ref() else {
            return;
        };

        let (flap, mfuel) = {
            let d = read_data(&self.data);
            (
                d.s_state.as_ref().unwrap().flaps as f64,
                d.s_state.as_ref().unwrap().mfuel,
            )
        };

        // 主升力面积因数载荷 (Java 注释原文)
        let wing_body_lift_area_load_no_flap = blkx.a_wing * nf.cl_crit_high
            + blkx.a_fuselage
                * blkx.fuse_cl_high
                * (nf.aoa_crit_high / fu.aoa_crit_high);
        let wing_body_lift_area_load_full_flap = blkx.a_wing * ff.cl_crit_high
            + blkx.a_fuselage
                * blkx.fuse_cl_high
                * (ff.aoa_crit_high / fu.aoa_crit_high);
        let current_weight = blkx.nofuelweight + mfuel;

        // 假设战雷的襟翼是线性的 (Java 注释原文)
        // 单位换算: 3.6 / 单位制混用: 1 / 1.225 (Java 注释原文)
        let flap_factor = flap / 100.0;
        let total_lift_area = (1.0 - flap_factor) * wing_body_lift_area_load_no_flap
            + flap_factor * wing_body_lift_area_load_full_flap;
        let mut d = write_data(&self.data);
        d.stall_speed = 3.6 * ((2.0 * current_weight * G) / (1.225 * total_lift_area)).sqrt();
    }

    // ------------------------------------------------------------------
    // publishFlightDataEvent (Java L434-482)
    // ------------------------------------------------------------------

    /// Publishes flight data to FlightDataBus.
    /// Pre-computes HUDData on Service thread to offload work from EDT.
    ///
    /// @deprecated Method name is legacy - renamed to publishFlightDataEvent() for clarity.
    /// (以上 javadoc 逐字保留, Java L434-438)
    fn publish_flight_data_event(&mut self) {
        // 载荷三件套在锁内取齐后**先释放读锁再 publish**——订阅方回调若再取
        // data 锁, 同线程 read→write 重入即死锁 (§2.8; Java 无此形态因其无锁)
        let (payload, state_box, indic_box) = {
            let d = read_data(&self.data);
            // Build type-safe payload (replaces legacy Map<String, String>)
            // Java: if (loc != null && mapinfo != null) { … } else mapGrid = "--";
            let map_grid = match (&d.loc, &d.mapinfo) {
                (Some(loc), Some(mi)) => {
                    // Java: char map_x = (char) ('A' + (loc[1] * mapinfo.mapStage) + mapinfo.inGameOffset);
                    // PORT: (char) 强转 = double→int→低 16 位; as i32 饱和与 Java
                    // 截断仅在极端值域分叉 (§2.2), 地图坐标域远离, as 链等价
                    let xf = ('A' as u32) as f64 + (loc[1] * mi.map_stage) + mi.in_game_offset;
                    // PORT: 未配对代理区 (surrogate) 在 Java char 合法而 Rust char
                    // 非法, 坍缩为 U+FFFD (域内 A-Z 不可达)
                    let map_x = char::from_u32(xf as i32 as u16 as u32)
                        .unwrap_or('\u{FFFD}');
                    // Java: int map_y = (int) (loc[0] * mapinfo.mapStage + mapinfo.inGameOffset + 1);
                    let map_y = (loc[0] * mi.map_stage + mi.in_game_offset + 1.0) as i32;
                    // Java: String.format("%c%d", map_x, map_y)
                    format!("{}{}", map_x, map_y)
                }
                _ => "--".to_string(),
            };

            let payload = EventPayload::builder()
                .map_grid(map_grid)
                // Java 自动拆箱 Boolean——null 时 NPE 由 run() 顶层 catch 兜住;
                // unwrap 的 panic 同构 (构造链恒置 Some, None 不可达)
                .fatal_warn(d.fatal_warn.unwrap())
                .radio_alt_valid(d.radio_alt_valid.unwrap())
                .is_downing_flap(d.is_downing_flap)
                // Java: timeStr(fueltimeStr) —— null 病态分支在 Rust String 下坍缩
                // 为 Builder 缺省 "--:--" (map_to_payload 先例; resetvaria 恒置
                // nastring 使 None 不可达)
                .time_str(
                    d.fueltime_str
                        .clone()
                        .unwrap_or_else(|| "--:--".to_string()),
                )
                // Java: isJet(iEngType == ENGINE_TYPE_JET)
                .is_jet(d.i_eng_type == ENGINE_TYPE_JET)
                .engine_check_done(d.check_engine_flag)
                .optimal_compressor_stage(d.optimal_compressor_stage)
                .compressor_stage_mismatch(d.compressor_stage_mismatch)
                .build();

            // Java: new FlightDataEvent(payload, sState, sIndic) —— 传引用 (共享可变)。
            // PORT: LIFETIMES §2.3 裁决 "事件对象改为每帧不可变快照"——逐字段
            // 手工快照 (State/Indicators 未 derive Clone, 不越文件改 §6)。
            // 消费方经 event.get_state() downcast 到 vm_core::parser::State。
            let state_box: Option<OpaqueObject> =
                d.s_state.as_ref().map(|s| Box::new(snapshot_state(s)) as OpaqueObject);
            let indic_box: Option<OpaqueObject> = d
                .s_indic
                .as_ref()
                .map(|i| Box::new(snapshot_indicators(i)) as OpaqueObject);
            (payload, state_box, indic_box)
        };

        let event = FlightDataEvent::new(payload, state_box, indic_box);

        // Pre-compute HUDData on Service thread (reduces EDT latency by ~40-60ms)
        // Java: if (c != null && c.configService != null) { … HUDCalculator.calculate … }
        // 已收口 (形态选择备案): HUDCalculator 批六已完整落地, 但预计算改由
        // win32 线程锁内执行 (审查 B-W2 — Service 线程算 + 跨线程传 HUDData 的
        // Rust 形态不如共享句柄直喂), 事件 hud_data 恒 None, 消费方走句柄

        // Java: FlightDataBus.getInstance().publish(event); → 构造注入实例
        // (回调线程 = 本 Service 线程, 对齐 Java 同步逐个调用)
        self.bus.publish(&event);
    }
}

/// run() 循环的控制流出口 (Java InterruptedException → break 的对应物)。
enum Flow {
    Continue,
    Interrupted,
}

// ------------------------------------------------------------------
// 快照/适配 helpers (无 Java 对应——服务于 §2.3 不可变快照与 Deriver 接口)
// ------------------------------------------------------------------

/// 保真版 [`State`] → POC 版 `StateRaw` (Deriver 接口适配)。
/// 哨兵 -65535 (I_INVALID/F_INVALID 同值) 原样穿透, 判定语义不变。
fn to_state_raw(s: &State) -> StateRaw {
    StateRaw {
        // int 拓宽 f64 (Java State int 字段的 double 消费点)
        ias: s.ias as f64,
        tas: s.tas as f64,
        height_m: s.heightm,
        vy: s.vy,
        wx: s.wx,
        aoa: s.aoa,
        aos: s.aos,
        ny: s.ny,
    }
}

/// 保真版 [`Indicators`] → POC 版 `IndicatorsRaw` (Deriver 接口适配)。
fn to_indicators_raw(i: &Indicators) -> IndicatorsRaw {
    IndicatorsRaw {
        // 保真版 valid 为字符串 "true"/"false" (getString 语义)
        valid: i.valid.as_deref() == Some("true"),
        speed: i.speed,
        vario: i.vario,
        aviahorizon_roll: i.aviahorizon_roll,
        aviahorizon_pitch: i.aviahorizon_pitch,
        compass: i.compass,
        radio_altitude: i.radio_altitude,
        wsweep: i.wsweep_indicator,
    }
}

/// [`State`] 逐字段快照 (§2.3 事件不可变快照; State 未 derive Clone)。
/// pub: vm-app 喂数侧重建 FlightDataEvent 时复用 (app_shell feed_overlays_live —
/// 通道边界丢 OpaqueObject 后按 live guard 现值重打快照, 见彼处 PORT 注)。
pub fn snapshot_state(s: &State) -> State {
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

/// [`Indicators`] 逐字段快照 (§2.3)。
/// PORT(army): 私有字段 `army` (vm-core 模块私有) 跨 crate 不可读写, 快照
/// 经 `Indicators::new()` 落默认值——全库无 army 读者 (仅 update 内部 tank
/// 过滤使用), 无行为差异; 故用 new()+逐字段赋值而非 struct 字面量。
/// pub: 同 [`snapshot_state`] (vm-app 喂数侧重建事件复用)。
pub fn snapshot_indicators(i: &Indicators) -> Indicators {
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

// ------------------------------------------------------------------
// FlightLog 接线面 (D6 边界的 vm-data 侧落地, 见 flight_log.rs 模块头 PORT)
// ------------------------------------------------------------------

/// Java `null` 引用在字符串拼接里的字面量 (Java `bw.write(xs.IAS + ",")` 的
/// IAS==null 写出 "null,")。Rust 字段是 Option<String>, None → "null" 保真;
/// NASTRING ("-", resetvaria 初值) 原样透传 (与 Java formatDataAsStrings 未跑时的值一致)。
fn jstr(v: &Option<String>) -> String {
    v.clone().unwrap_or_else(|| "null".to_string())
}

/// logTick 时刻的 ServiceData → FlightLogSnapshot 构造面 (flight_log.rs 模块头
/// PORT 注的 "vm-data 侧快照构造面")。字段逐一对应 ServiceData/State 字段
/// (Service.java 的 xs 公有字段直读, 语义 = 读锁内一次成组快照)。
pub fn flight_log_snapshot(d: &ServiceData) -> FlightLogSnapshot {
    FlightLogSnapshot {
        elapsed_time: d.elapsed_time,
        throttle: jstr(&d.throttle),
        ias: jstr(&d.ias),
        tas: jstr(&d.tas),
        mach: jstr(&d.m),
        salt: jstr(&d.salt),
        watertemp: jstr(&d.watertemp),
        oiltemp: jstr(&d.oiltemp),
        vy: jstr(&d.vy),
        s_sep: jstr(&d.s_sep),
        // Java: xs.sState.Ny (State.java:27)
        ny: d.s_state.as_ref().map(|s| s.ny).unwrap_or(0.0),
        wx: jstr(&d.wx),
        total_hp_str: jstr(&d.total_hp_str),
        // Java: xs.efficiency[0] — 数组/元素 null 时拼接产出 "null"
        efficiency_0: d
            .efficiency
            .as_ref()
            .and_then(|v| v.first())
            .cloned()
            .flatten()
            .unwrap_or_else(|| "null".to_string()),
        total_hp_eff_str: jstr(&d.total_hp_eff_str),
        rpm: jstr(&d.rpm),
        total_thrust: d.total_thrust,
        acceleration: d.acceleration,
        rpm_throttle: jstr(&d.rpm_throttle),
        pitch_0: d
            .pitch
            .as_ref()
            .and_then(|v| v.first())
            .cloned()
            .flatten()
            .unwrap_or_else(|| "null".to_string()),
        radiator: jstr(&d.radiator),
        mixture: jstr(&d.mixture),
        compressorstage: d.s_state.as_ref().map(|s| s.compressorstage).unwrap_or(0),
        magenato: d.s_state.as_ref().map(|s| s.magenato).unwrap_or(0),
        manifoldpressure: jstr(&d.manifoldpressure),
        flaps: jstr(&d.flaps),
        elevator: d.s_state.as_ref().map(|s| s.elevator).unwrap_or(0),
        aileron: d.s_state.as_ref().map(|s| s.aileron).unwrap_or(0),
        rudder: d.s_state.as_ref().map(|s| s.rudder).unwrap_or(0),
        aoa: jstr(&d.aoa),
        aos: jstr(&d.aos),
        alt: d.alt,
        check_alt: d.check_alt,
        ias_v: d.ias_v,
        sep: d.sep,
        state_wx: d.s_state.as_ref().map(|s| s.wx).unwrap_or(0.0),
        // Java: init 读 `s.sIndic.type`; sIndic 存在而 type 键缺失 → null →
        // toUpperCase() NPE (Java 崩 openpad 线程), Rust 以空串降级 (PORT 备案:
        // 常态不可达 — 换机/开局时 type 必在; 崩线程无观察面)
        indic_type: d
            .s_indic
            .as_ref()
            .and_then(|i| i.r#type.clone())
            .unwrap_or_default(),
    }
}

/// [`AnalyzerService`] 的 ServiceData 适配器 (flight_analyzer.rs 接线合同的
/// vm-data 侧 impl)。getter 逐字段读锁 (合同义务 2: 逐字段调用对应 Java 逐字段
/// 读取时刻); sIndic 缺失 panic 对齐 Java NPE (合同义务 1)。
pub struct ServiceAnalyzerSource {
    data: Arc<RwLock<ServiceData>>,
}

impl ServiceAnalyzerSource {
    pub fn new(data: Arc<RwLock<ServiceData>>) -> Self {
        ServiceAnalyzerSource { data }
    }
}

impl AnalyzerService for ServiceAnalyzerSource {
    fn s_indic_type(&self) -> Option<String> {
        let d = read_data(&self.data);
        let s = d
            .s_indic
            .as_ref()
            .expect("PORT: Java NPE — xs.sIndic 为 null 时 FlightAnalyzer.init 首行 NPE");
        s.r#type.clone()
    }
    fn i_eng_type(&self) -> i32 {
        read_data(&self.data).i_eng_type
    }
    fn elapsed_time(&self) -> i64 {
        read_data(&self.data).elapsed_time
    }
    fn total_hp(&self) -> i32 {
        read_data(&self.data).total_hp
    }
    fn total_thrust(&self) -> i32 {
        read_data(&self.data).total_thrust
    }
    fn total_hp_eff(&self) -> i32 {
        read_data(&self.data).total_hp_eff
    }
    fn sep(&self) -> f64 {
        read_data(&self.data).sep
    }
}

// ------------------------------------------------------------------
// start/stop 生命周期 (Java Controller.start:634 / stop:807 的 Service 侧)
// ------------------------------------------------------------------

/// Service 线程句柄: stop 置位 + join (对应 Java `S1.interrupt()` 后线程退出)。
pub struct ServiceHandle {
    /// 停机标志 (调用方可预置/轮询)
    pub stop: Arc<AtomicBool>,
    /// 数据快照共享句柄 (测试/调用方读 ServiceData)
    pub data: Arc<RwLock<ServiceData>>,
    join: Option<JoinHandle<()>>,
}

impl ServiceHandle {
    /// 停止轮询线程并等待退出 (Java: `S1.interrupt()`, run 的 sleep 抛
    /// InterruptedException → break)。幂等。
    /// 返回线程是否正常退出 (false = panic 逃逸出 run, §6 契约破坏的观测点)。
    pub fn stop(&mut self) -> bool {
        self.stop.store(true, Ordering::SeqCst);
        match self.join.take() {
            Some(j) => j.join().is_ok(),
            None => true,
        }
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        // 兜底: 忘记显式 stop 也不泄漏线程 (§2.13 电平标志的优势形态)
        self.stop();
    }
}

/// 在独立线程启动 Service 轮询 (调用方持 [`ServiceHandle`] 管理生命周期)。
/// PORT: Java `S1.setPriority(Thread.MAX_PRIORITY)` —— Rust std 线程无优先级
/// 概念, 不复刻 (Windows 下可后续经 SetThreadPriority 补, C 类窗口波次裁决)。
pub fn start(service: Service) -> ServiceHandle {
    let stop = Arc::clone(&service.stop);
    let data = Arc::clone(&service.data);
    let join = std::thread::Builder::new()
        .name("Service".to_string())
        .spawn(move || service.run())
        .expect("Service 线程创建失败");
    ServiceHandle {
        stop,
        data,
        join: Some(join),
    }
}

// =====================================================================
// Tests
// =====================================================================
#[cfg(test)]
mod tests;
