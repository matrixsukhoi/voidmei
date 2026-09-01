//! Service 轮询线程主体: run() 循环 → HTTP 轮询 → calculate 链 (状态机 + 公式步)
//! → publish。数据快照见 service_fields.rs。
//!
//! 锁纪律: 临界区内不调回调/不做 IO——先取副本→释放→计算→短锁写回。
//! 顶层 catch_unwind: 单条畸形遥测 panic 不杀线程, 丢一轮继续 (锁中毒穿透
//! 见 read_data/write_data)。
//! HTTP/parser 用 vm-core 保真版 (HttpHelper / parser::{State, Indicators})。

use std::net::SocketAddr;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use vm_core::calc_helper::SimpleMovingAverage;
use vm_core::event::event_payload::EventPayload;
use vm_core::event::flight_data_event::FlightDataEvent;
use vm_core::formula::registry::FormulaView as _; // var_value 取数唯一接口
use vm_core::flight_analyzer::AnalyzerService;
use vm_core::flight_data_bus::FlightDataBus;
use vm_core::flight_log::{FlightLogSlot, FlightLogSnapshot};
use vm_core::fm::{FMHandle, FMManager};
use vm_core::http_helper::HttpHelper;
use vm_core::parser::{Indicators, MapInfo, MapObj, State};
use vm_core::{exception_helper, format, logger, G};

use vm_core::string_helper::F_INVALID;
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

/// C 级会话量收集 (W6: registry Session 通道的供值面 — 聚合/状态机产物
/// 暂经 ServiceData 字段搬运, W8 公式化后逐项消亡)
pub(crate) fn session_inputs(d: &ServiceData) -> vm_core::formula::registry::SessionInputs {
    vm_core::formula::registry::SessionInputs {
        total_fuel: d.total_fuel,
        fuel_time_mili: d.fueltime as f64,
        total_hp: d.total_hp as f64,
        total_hp_eff: d.total_hp_eff as f64,
        total_thrust: d.total_thrust as f64,
        n_water_temp: d.nwater_temp,
        n_oil_temp: d.noil_temp,
        radio_alt: d.radio_alt,
        compass_delta: d.compass_delta,
        nitro_kg: d.nitrokg,
        wep_time: d.s_wep_time_val as f64,
        heat_tolerance: d.cur_load_min_work_time / 1000.0,
        thurst_percent: d.thurst_percent,
        t_eng_response: d.t_eng_response,
        avgeff: d.avgeff,
        // 原 getFuelPercent getter: i32 字段拓宽 (EngineControl 油量表数据源)
        fuel_percent: d.fuel_percent as f64,
        // Java getManifoldPressureDisplay: 英制 → (manifold-1)*14.696 psi,
        // 公制 → manifoldpressure 直读 (曾误走 trait default 恒 0, live 进气压行失真)
        manifold_display: {
            let s = d.s_state.as_ref();
            if d.check_alt > 0 {
                s.map_or(0.0, |s| (s.manifoldpressure - 1.0) * 14.696)
            } else {
                s.map_or(0.0, |s| s.manifoldpressure)
            }
        },
        is_imperial: d.check_alt > 0,
        is_jet: d.check_engine_flag && d.i_eng_type == ENGINE_TYPE_JET,
        // Java isPropEngine = PROP || TURBOPROP (isPistonEngine 才是 ==PROP);
        // 曾漏 TURBOPROP 致涡桨机 is_prop_engine 恒 false
        is_prop: d.check_engine_flag
            && (d.i_eng_type == ENGINE_TYPE_PROP || d.i_eng_type == ENGINE_TYPE_TURBOPROP),
        is_piston: d.check_engine_flag && d.i_eng_type == ENGINE_TYPE_PROP,
        is_turboprop: d.check_engine_flag && d.i_eng_type == ENGINE_TYPE_TURBOPROP,
        engine_check_done: d.check_engine_flag,
    }
}

/// fueltimeStr (Java formatDataAsStrings L265-278): 无效 → "-";
/// <100 分钟 → "%02d'%02d" (秒位向下取整到十位); 否则 "%.0f" 分钟
/// ((float)fueltime/60000 的 float 除法域再拓宽, §2.12)。
fn format_fueltime(fueltime: i64) -> String {
    if fueltime <= 0 || fueltime > 24 * 3600 * 1000 {
        NASTRING.to_string()
    } else if fueltime / 60000 < 100 {
        format!(
            "{}'{}",
            format::java_d0(fueltime / 60000, 2),
            format::java_d0((fueltime / 1000) % 60 / 10 * 10, 2)
        )
    } else {
        format::java_f((fueltime as f32 / 60000.0f32) as f64, 0)
    }
}

/// W-C 取数单通道后的唯一写回点: maximum_thr_rpm 是 get_maximum_rpm_learn
/// 状态机的存储 (C 级, 公式可覆写), 其余派生量一律走公式槽 (var_value),
/// 不再有 ServiceData 副本字段。NaN 不写 (公式 invalid → 状态机保持原值)。
fn write_back(d: &mut ServiceData, name: &str, v: f64) {
    if v.is_nan() {
        return;
    }
    if name == "maximum_thr_rpm" {
        d.maximum_thr_rpm = v;
    }
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
    /// 公式系统 (无 Java 对应, doc/formula_system_design.md §2 裁决 A1):
    /// 本线程每帧求值单点; Arc 共享给编辑器保存链跨线程 install (热更新)。
    pub formula: Arc<vm_core::formula::FormulaManager>,
    /// L2 规则引擎 (formula_step 尾部求值, 触发事件写 ServiceData.rule_triggers)
    rule_engine: vm_core::formula::rules::RuleEngine,
    /// 换机检测 (FM name 变化 → 公式状态原语全清 + adapter 重建, 设计 §3.5)
    last_fm_name: Option<String>,
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
        // 公式装载 (内置+用户文件, 逻辑收敛在 FormulaManager::load_from_files)
        let formula = Arc::new(vm_core::formula::FormulaManager::new());
        formula.load_from_files();
        let mut svc = Service {
            data: Arc::clone(&data),
            fm_manager,
            bus,
            http_client: HttpHelper::new(&config.http_header),
            focus_monitor: None,
            flight_log: Arc::new(std::sync::Mutex::new(None)),
            formula,
            rule_engine: vm_core::formula::rules::RuleEngine::new(),
            last_fm_name: None,
            stop: Arc::new(AtomicBool::new(false)),
            config,
        };
        // 规则装载 (formulas.cfg/user 的 (rule ...) 段)
        {
            let builtin = std::fs::read_to_string(vm_core::formula::persistence::BUILTIN_FORMULAS_PATH)
                .unwrap_or_default();
            let user = std::fs::read_to_string(vm_core::formula::persistence::USER_FORMULAS_PATH)
                .unwrap_or_default();
            let mut rules = vm_core::formula::persistence::parse_rules(&builtin);
            rules.extend(vm_core::formula::persistence::parse_rules(&user));
            svc.rule_engine.install(&rules, vm_core::formula::registry());
        }
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
            d.player_live = false;
            d.i_eng_type = ENGINE_TYPE_UNKNOWN;
            d.check_maxium_rpm = 0;
            d.compass_delta = 0.0;
            d.get_maximum_rpm = false;
            d.d_radio_alt = 0.0;
            d.wep_time = 0;
            d.elapsed_time = 0;
            d.check_alt = 0;
            d.altp = 0.0;
            d.alt = 0.0;
            d.calc_period = 0;
            d.maximum_thr_rpm = 1.0;
            d.max_total_thr = 0;
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
            // Java: curLoadMinWorkTime = 99999 * 1000; —— int 乘法 99999000 拓宽 double
            d.cur_load_min_work_time = (99999 * 1000) as f64;
            /* 刷新引擎工作时间 */
            // (锁外调用, 见下)
            // if(c.getBlkx() != null && c.getBlkx().maxEngLoad !=
            // 0)c.getBlkx().resetEngineLoad();
            let now = current_time_millis();
            // Java: lastMapPollTimeMs = FuelCheckMili; lastMainLoopTimeMs = FuelCheckMili;
            d.last_map_poll_time_ms = now;
            d.last_main_loop_time_ms = now;
            d.not_check_inch = false;
            // isFuelpressure = false;
            // Java L1577 对 notCheckInch 的第二次赋值, 保真保留
            d.not_check_inch = false;
            d.total_fuel_prev = 0.0;
            d.nitrokg = 0.0;
            d.nitro_consump = 0.0;
            d.nitro_eng_nr = 0;

            // Java L1587-1593: 7 个 SMA 构造, 窗口 (int)(1000/freq), fuelTimeSMA=4。
            // PORT(状态双主裁决, service_fields.rs 字段区 PORT 注): calc/diff/sep/
            // turnrds 四个 SMA 的真人在 Deriver (其 new/step 已按同窗口与同公式
            // 移植), ServiceData 侧对应槽位**保持 None**——防双胞胎真互相漂移;
            // sum/energyDiff/fuelTime 三个 Java 侧 addNewData 调用已被注释 (仅构造),
            // 按 service_fields 裁决由本波次直接构造:
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

        } // —— write 临界区结束 (publish 前必须释放, §2.8)

        // Java: resetEngLoad(fm); (L1568, 字段赋值序列中间——锁外执行)
        // (方法体已随 engLoad 会话态批次迁至 overheat.rs, 关联函数签名不变)
        Self::reset_eng_load(&fm);
        // PORT(SMA 重建): Java L1587-1590 的 calc/diff/sep/turnrds 四 SMA 在本
        // 调用点重建 = Deriver 整体重建 (真人在彼, 见上)
        let _freq = self.config.service_loop_interval_ms;
        // W2: Deriver 消解 — SMA 状态改由公式状态仓承载, 会话重置在此
        self.formula.reset_states();

        // Java: publishFlightDataEvent(); (L1659)
        // Publish initial state immediately
        self.publish_flight_data_event();
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
                        // Java: Math.abs(speedv) < 10 — W-C 起直读公式槽
                        let speedv = d.var_value("speedv").unwrap_or(0.0);
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

                    // 批2: formatDataAsStrings 镜像层已拆 — 显示文本由消费侧
                    // 就地格式化 (FlightLog CSV / EventPayload.time_str)
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


    // ------------------------------------------------------------------
    // publishFlightDataEvent (Java L434-482)
    // ------------------------------------------------------------------

    /// Publishes flight data to FlightDataBus.
    /// Pre-computes HUDData on Service thread to offload work from EDT.
    ///
    /// @deprecated Method name is legacy - renamed to publishFlightDataEvent() for clarity.
    /// (以上 javadoc 逐字保留, Java L434-438)
 
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

        // Java calculate 链 L1130-1131: 检查过热, 计算引擎健康度 — overheat.rs
        // (engLoad 会话态走 FMHandle.eng_load_state, D 批次)
        self.check_overheat(&fm);

        // 增加wep时间 / 更新温度，优先使用更精确的 / 检查是否过热… (TODO 列表见 doc)
        // 更新方向 / 更新爬升率 / 获得准确高度 / 更新速度 / 更新转弯半径 —— Deriver::step
        // (updateCompass/updateAlt 的非公式部分在下方写回段逐行落地)
        let (vy, radio_alt_raw, alt10k, dir, indic_compass, heightm, n_vy) = {
            let d = read_data(&self.data);
            let s = d.s_state.as_ref().unwrap();
            // 写回段状态机输入: altitude_10k / dir / 原始 radio_altitude (哨兵判定) /
            // vario (仪表罗盘优先) — W2: Deriver step 消解, 直通量就地内联,
            // 派生量由公式接管 (下方 formula_step), FlightValues 整包快照删除
            // (FlightInfo 改吃 TelemetrySource 散字段)
            let alt10k = d.s_indic.as_ref().unwrap().altitude_10k;
            let dir = d.dir;
            let vy = s.vy;
            let radio_alt_raw = d.s_indic.as_ref().unwrap().radio_altitude;
            let indic_compass = d.s_indic.as_ref().unwrap().compass;
            let heightm = s.heightm;
            let n_vy = if d.s_indic.as_ref().unwrap().vario != F_INVALID {
                d.s_indic.as_ref().unwrap().vario
            } else {
                vy
            };
            drop(d);
            (vy, radio_alt_raw, alt10k, dir, indic_compass, heightm, n_vy)
        };

        // 直通量写回 + 公式接管 (W2: 原 Deriver step 的直通部分; 公式含
        // an/sep/turn 族/speedv/mach 全链, 位级对拍见 tests w2_deriver_takeover)
        {
            let mut d = write_data(&self.data);
            // nVy ← vario (updateClimbRate 的 indic 优先回退)
            d.n_vy = n_vy;

            // Java: updateCompass (L1101-1113)
            // 如果有仪表罗盘，读取仪表罗表盘数据
            if indic_compass != F_INVALID {
                d.compass_delta = indic_compass;
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
            d.alt = heightm;
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
                } else {
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

        // 公式系统步 (W2: 提前至 updateEngineState 前 — speedv 等 HP 有效功率
        // 输入需本帧公式值, 尾部求值会引入一帧滞后; 快照输入 state/indicators/
        // alt/n_vy 直通均已就绪)
        self.formula_step(&fm);

        // Java calculate 链 L1134-1136: updateEngineState (总功率/推力/百分比)
        // + updateFuel (总油量) — EngineInfo/EngineControl 面板数据源。
        // PORT(W2): speedv 为公式接管值 (Deriver 消解), 读 ServiceData 散字段
        self.update_engine_state(&fm);
        self.update_fuel();

        // Java calculate 链 L1168-1170 (updateSEP 之后): 襟翼判断 / 最大转速 —
        // methods_engine.rs (可变翼判断已删: registry wing_sweep_valid 直通替代)
        // (check_flap 已 W8 公式化 — is_downing_flap/flap_allow_* 走公式写回)
        self.get_maximum_rpm_learn(&fm);

        // Java calculate 尾部两比值方法 (L1177-1178): 速度/马赫临界比值 + 失速速度
        // — MiniHUD 速度比值 bar 的数据源 (speed_limit_ratio 等 5 字段)

        // Java calculate 链尾 (L1173): 最佳增压器档位/失配提示 — methods_engine.rs
        // (公式步已提前至 updateEngineState 前 — speedv 本帧值依赖; 此处不再调)
        self.update_optimal_compressor_stage(&fm);
    }

    /// 公式一帧: 换机检测(adapter 重建+状态清零) → 组快照 → 求值 → 写回。
    /// fm.* 变量经 FMDataAdapter (Blkx→消费面快照, 换机时一次转换);
    /// 无 FM 时传 None → fm.* 全 NaN (设计 §3.6)。
    fn formula_step(&mut self, fm: &FMHandle) {
        // 换机 → adapter 重建 + 状态原语全清 (设计 §3.5)
        if self.last_fm_name.as_deref() != fm.name.as_deref() {
            self.formula.reset_states();
            self.last_fm_name = fm.name.clone();
            // W6: fm.* 直绑 blkx, adapter 三层已删 — 无重建动作
        }
        let (results, slots, snap, interval_ms) = {
            let d = read_data(&self.data);
            let meta = vm_core::formula::MetaInputs {
                interval_ms: d.actual_interval_ms.max(1) as f64,
                freq: d.freq as f64,
                fm_loaded: fm.blkx.is_some(),
                ..Default::default()
            };
            // W6 直通: 原始三元组 + C 级会话量 (FMDataSource/adapter 三层已删)
            let raw = vm_core::formula::registry::RawInputs {
                state: d.s_state.as_ref(),
                indic: d.s_indic.as_ref(),
                blkx: fm.blkx.as_ref(),
            };
            let session = session_inputs(&d);
            // 快照重建供规则求值 (formula.eval_frame 内部快照已 move 进缓存)
            let snap = vm_core::formula::registry::assemble_snapshot(&raw, &session, &meta);
            (
                self.formula.eval_frame(&raw, &session, &meta, current_time_millis() as u64),
                self.formula.current().slots_arc(),
                snap,
                meta.interval_ms,
            )
        };
        // L2 规则求值 (公式之后同快照; 触发事件写 ServiceData, 消费面 vm-app 接)
        let triggers =
            self.rule_engine.eval(&snap, &results, current_time_millis() as u64, interval_ms);
        let mut d = write_data(&self.data);
        d.formula_values = results;
        d.formula_slots = slots.clone();
        d.rule_triggers = triggers;
        // 接管型公式统一写回 (W1b 通用机制, 设计 §5 同名规则):
        // 公式名命中可写白名单 → 求值结果覆写 ServiceData 对应字段。
        // NaN 守卫: 公式 invalid/缺输入不覆写, 保持 Rust 路径值 (双保险)。
        let set = self.formula.current();
        for f in &set.formulas {
            if f.err.is_some() {
                continue;
            }
            let Some(&slot) = set.slots.get(&f.def.name) else { continue };
            let v = d.formula_values.get(slot);
            write_back(&mut d, &f.def.name, v);
        }
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
        if d.fueltime < 0 {
            d.fueltime = i64::MAX;
        }
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
            if v < 0.0 { 0.0 } else { v }
        } else {
            0.0
        };
        // twepTime (原 formatStrings 段): WEP 剩余秒数 — registry wep_time 变量
        // 数据源 (session_inputs)。Java 仅在 nitro!=0 且 nitroEngNr!=0 时写,
        // 其余分支保持上轮值 (保真)。
        // (int)(((blkx.nitro / blkx.nitroDecr - wepTime / 1000)) / nitroEngNr)
        // —— wepTime/1000 是 long 整除后才并入 double
        if let Some(blkx) = fm.blkx.as_ref() {
            if blkx.nitro != 0.0 && nitro_eng_nr != 0 {
                d.s_wep_time_val = ((blkx.nitro / blkx.nitro_decr
                    - (wep_time as i64 / 1000) as f64)
                    / nitro_eng_nr as f64) as i32 as i64;
            }
        }
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
        // speedv (校正 TAS m/s) — W-C: 直读公式槽 (formula_step 已先行)
        let speedv = read_data(&self.data).var_value("speedv").unwrap_or(0.0);

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

   fn publish_flight_data_event(&mut self) {
        // W-B 事件瘦身: 事件只承载标量 payload; State/Indicators/派生量由消费方
        // 持共享 ServiceData guard 现取, 不再装箱逐字段快照。
        let payload = {
            let d = read_data(&self.data);
            // Java: if (loc != null && mapinfo != null) { … } else mapGrid = "--";
            let map_grid = match (&d.loc, &d.mapinfo) {
                (Some(loc), Some(mi)) => {
                    // Java: char map_x = (char) ('A' + (loc[1] * mapinfo.mapStage) + mapinfo.inGameOffset);
                    let xf = ('A' as u32) as f64 + (loc[1] * mi.map_stage) + mi.in_game_offset;
                    let map_x = char::from_u32(xf as i32 as u16 as u32).unwrap_or('\u{FFFD}');
                    let map_y = (loc[0] * mi.map_stage + mi.in_game_offset + 1.0) as i32;
                    format!("{}{}", map_x, map_y)
                }
                _ => "--".to_string(),
            };

            EventPayload::builder()
                .map_grid(map_grid)
                .fatal_warn(d.fatal_warn.unwrap())
                // W-C: 直读公式槽 (is_downing_flap 公式)
                .is_downing_flap(d.var_value("is_downing_flap").unwrap_or(0.0) != 0.0)
                // 批2: fueltime 文本现算 (镜像字段已拆)
                .time_str(format_fueltime(d.fueltime))
                // Java: isJet(iEngType == ENGINE_TYPE_JET)
                .is_jet(d.i_eng_type == ENGINE_TYPE_JET)
                .engine_check_done(d.check_engine_flag)
                .optimal_compressor_stage(d.optimal_compressor_stage)
                .compressor_stage_mismatch(d.compressor_stage_mismatch)
                .build()
        };

        let event = FlightDataEvent::new(payload);
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



// ------------------------------------------------------------------
// FlightLog 接线面 (D6 边界的 vm-data 侧落地, 见 flight_log.rs 模块头 PORT)
// ------------------------------------------------------------------

/// Java `null` 引用在字符串拼接里的字面量 (Java `bw.write(xs.IAS + ",")` 的
/// IAS==null 写出 "null,")。Rust 字段是 Option<String>, None → "null" 保真;
/// NASTRING ("-", resetvaria 初值) 原样透传 (与 Java formatDataAsStrings 未跑时的值一致)。

/// logTick 时刻的 ServiceData → FlightLogSnapshot 构造面 (flight_log.rs 模块头
/// PORT 注的 "vm-data 侧快照构造面")。字段逐一对应 ServiceData/State 字段
/// (Service.java 的 xs 公有字段直读, 语义 = 读锁内一次成组快照)。
pub fn flight_log_snapshot(d: &ServiceData) -> FlightLogSnapshot {
    // 批2: String 镜像层已拆 — CSV 列就地格式化 (语义与 Java formatDataAsStrings
    // 逐行对齐, java_f 族 = vm_core::format)。State 缺失的病态帧列值 "null"
    // (对位原 jstr(None) 的 Java null 拼接)。
    let st = d.s_state.as_ref();
    let col = |f: &dyn Fn(&State) -> String| st.map_or_else(|| "null".to_string(), f);
    let na = NASTRING;
    // %d 列 (负值 → "-"): rpmThrottle/radiator/mixture/throttle
    let int_na = |v: i32| if v >= 0 { v.to_string() } else { na.to_string() };
    // SEP 取整 (可读性): (long)SEP/50*2.5, 0 → 1 (W-C: 直读公式槽, None→0)
    let sep = d.var_value("sep").unwrap_or(0.0);
    let mut sep_acc = ((sep as i64) / 50) as f64;
    sep_acc *= 2.5;
    if sep_acc == 0.0 { sep_acc = 1.0; }
    let sep_rounded = format::java_round(sep / sep_acc) as f64 * sep_acc;
    FlightLogSnapshot {
        elapsed_time: d.elapsed_time,
        throttle: col(&|s| int_na(s.throttle)),
        ias: col(&|s| s.ias.to_string()),
        tas: col(&|s| s.tas.to_string()),
        mach: col(&|s| format::java_f(s.m, 2)),
        salt: format::java_f(d.alt, 0),
        watertemp: if d.nwater_temp != -65535.0 {
            format::java_f(d.nwater_temp, 0)
        } else {
            na.to_string()
        },
        oiltemp: format::java_f(d.noil_temp, 0),
        vy: format::java_f(d.n_vy, 1),
        s_sep: format::java_f(sep_rounded, 0),
        // Java: xs.sState.Ny (State.java:27)
        ny: st.map(|s| s.ny).unwrap_or(0.0),
        wx: col(&|s| format::java_f(s.wx.abs(), 0)),
        total_hp_str: if d.total_hp == 0 { na.to_string() } else { d.total_hp.to_string() },
        // Java: xs.efficiency[0] — 数组/元素 null 时拼接产出 "null"
        efficiency_0: st.map_or_else(
            || "null".to_string(),
            |s| {
                let e0 = s.efficiency.first().copied().unwrap_or(0.0);
                if e0 == 0.0 { na.to_string() } else { format::java_f(e0, 0) }
            },
        ),
        total_hp_eff_str: if d.total_hp_eff >= 100000 {
            // %.2f of /1e6 — int/float float 除法域再拓宽 (§2.12)
            format::java_f((d.total_hp_eff as f32 / 1000000.0f32) as f64, 2)
        } else {
            d.total_hp_eff.to_string()
        },
        rpm: col(&|s| s.rpm.to_string()),
        total_thrust: d.total_thrust,
        acceleration: d.var_value("acceleration").unwrap_or(0.0),
        rpm_throttle: col(&|s| int_na(s.rpm_throttle)),
        pitch_0: st.map_or_else(
            || "null".to_string(),
            |s| {
                let p0 = s.pitch.first().copied().unwrap_or(-65535.0);
                if p0 != -65535.0 { format::java_f(p0, 1) } else { na.to_string() }
            },
        ),
        radiator: col(&|s| int_na(s.radiator)),
        mixture: col(&|s| int_na(s.mixture)),
        compressorstage: st.map(|s| s.compressorstage).unwrap_or(0),
        magenato: st.map(|s| s.magenato).unwrap_or(0),
        manifoldpressure: st.map_or_else(
            || "null".to_string(),
            |s| {
                if s.manifoldpressure != 1.0 {
                    if d.check_alt > 0 {
                        // 英制: 显示 Boost psi (%+.1f)
                        format::java_f_plus((s.manifoldpressure - 1.0) * 14.696, 1)
                    } else {
                        format::java_f(s.manifoldpressure, 2)
                    }
                } else {
                    na.to_string()
                }
            },
        ),
        flaps: col(&|s| s.flaps.to_string()),
        elevator: st.map(|s| s.elevator).unwrap_or(0),
        aileron: st.map(|s| s.aileron).unwrap_or(0),
        rudder: st.map(|s| s.rudder).unwrap_or(0),
        aoa: col(&|s| if s.aoa != -65535.0 { format::java_f(s.aoa, 1) } else { na.to_string() }),
        aos: col(&|s| if s.aos != -65535.0 { format::java_f(s.aos, 1) } else { na.to_string() }),
        alt: d.alt,
        check_alt: d.check_alt,
        // EM 图速度分档 (原 IASv 平滑值未移植, 用 State 直读 IAS, 显示 0-3 位不敏感)
        ias_v: st.map(|s| s.ias as f64).unwrap_or(0.0),
        sep,
        state_wx: st.map(|s| s.wx).unwrap_or(0.0),
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
        // W-C: 直读公式槽, None→0
        read_data(&self.data).var_value("sep").unwrap_or(0.0)
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
// calculate 链方法族的跨文件 impl 宿主 (接线调用统一在 calculate 内, 见各模块头注)
mod methods_engine;
mod overheat;

#[cfg(test)]
mod tests;
