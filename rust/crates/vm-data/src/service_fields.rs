//! 对应 Java: `src/prog/Service.java` 的**实例字段区 + TelemetrySource getter 区**
//! (一比一翻译, D6: Service 链落 vm-data; 计算方法区归 service_loop 波次)。
//!
//! ## 设计裁决 (LIFETIMES / D6)
//!
//! Java `Service` 的 public 字段被全库宽松并发读写 (EDT 的 HUDCalculator 回退路径 +
//! Service 轮询线程同帧混读, 无锁无 volatile) → Rust 把字段集中为本 [`ServiceData`],
//! 由 service_loop 持 `RwLock<ServiceData>`, getter 一律 `&self` 快照读
//! (对应 LIFETIMES §7 "TelemetrySnapshot + ArcSwap" 的 RwLock std 形态)。
//!
//! - **派生量字段不重复翻译计算**: `Deriver` 已按 Service 公式逐行移植
//!   (update_speed/update_climb_rate/update_turn/update_sep), service_loop 每周期把
//!   `FlightValues` 写回本 struct 的对应字段 (来源映射见各字段 `// PORT:` 注)。
//! - **纯读 getter 保持字段直读语义** (getNy=an/g 等, Java L1875-2283 逐行对应)。
//!
//! PORT(不迁移项): 计算方法区 (calculate/formatDataAsStrings/update*/check*/resetvaria
//! 等全部方法体)、`run()` 轮询循环、`publishFlightDataEvent` 均归 service_loop 波次
//! (D6 两 item 划分), 本文件只含字段 + getter。

use std::sync::Arc;

use vm_core::calc_helper::SimpleMovingAverage;
use vm_core::fm::FMHandle;
use vm_core::parser::{Indicators, MapInfo, State};
use vm_core::ui_model::TelemetrySource;

/// Service 的实例字段全集 (Java 字段区 L27-230 / L651-678 / L1180-1184 / L1234,
/// 声明顺序与 Java 一致, 注释逐字保留)。
///
/// PORT(可见性): Java public → pub; Java private / 包私有 → pub(crate) (Rust 裸私有
/// 是**模块**私有, 兄弟模块 service_loop (计算方法波次) 写入会 E0616 编译失败;
/// pub(crate) 才是 Java "类内/包内可见" 的 crate 级对应物)。Java 全库无读写的死字段
/// (gc_check_mili/slow_check_mili/interval_check_ms/test_check_mili) 保持模块私有。
/// PORT(快照字段 fm, 无 Java 对应字段): Java getter `getTotalWeight`/`hasWep` 现读
/// 全局单例 `FMManager.getInstance().current()`; Rust 单例已解散 (FMManager PORT 注),
/// 按 LIFETIMES §7 "读 FM → 只读快照取代反引用" 裁决, 由 service_loop 每周期把
/// `current()` 快照 (R1 周期句柄) 写入 `fm` 字段 —— getter 语义 = 读本周期句柄,
/// 与 Java calculate 链的 R1 快照纪律一致。
/// PORT(死字段豁免): 标 `#[allow(dead_code)]` 的字段写者在 service_loop 波次
/// (blkx/mod.rs 字段波次同款先例的规模化形态; pub 字段是库公共 API 不触发 lint,
/// 仅非 pub 字段 (私有 / pub(crate)) 需要)。
pub struct ServiceData {
    // ---- L26-34 (静态字段 cH/buf 不迁移: buf 全库无读写死代码, cH 无状态工具改自由
    // 函数; 裁决见 LIFETIMES §1.3) ----
    // PORT: Java `public CalcHelper.SimpleMovingAverage xxx` 无初始化器 → 字段默认
    // null, resetvaria() 才构造 (窗口 1000/freq, fuelTimeSMA 为 4) → Option
    // PORT(状态双主边界): Deriver (data/derive.rs) 自持 calc/turnrds/diff/sep 四个
    // SMA 及 an/turn_rds/speedv 族同源状态, 而 FlightValues 不携带 SMA 态 ——
    // service_loop 波次必须裁决唯一状态主人 (Java resetvaria L1591-1597 重建 SMA 的
    // 语义打在主人侧), 防 ServiceData 侧双胞胎永远 None / 两份真相互相漂移;
    // sum_speed_sma/energy_diff_sma/fuel_time_sma 无 Deriver 对应 (Java 侧 addNewData
    // 调用已被注释, 仅构造), 由 service_loop 直接构造。
    pub diff_speed_sma: Option<SimpleMovingAverage>,
    pub sep_sma: Option<SimpleMovingAverage>,
    pub turnrds_sma: Option<SimpleMovingAverage>,
    pub sum_speed_sma: Option<SimpleMovingAverage>,
    pub calc_speed_sma: Option<SimpleMovingAverage>,
    pub fuel_time_sma: Option<SimpleMovingAverage>,
    pub energy_diff_sma: Option<SimpleMovingAverage>,
    // public static URL urlstate;
    // public static URL urlindicators;
    /// Java `public double loc[]` — null 直到 resetvaria 赋 `new double[2]`;
    /// MapObj.getPlayerLoc 按 &mut [f64;2] 写入 → 定长 [f64; 2] (§1)
    pub loc: Option<[f64; 2]>,
    pub dir: Option<[f64; 2]>,
    pub energy_j_kg: f64,
    pub prev_energy_j_kg: f64,
    pub calc_period: i64,
    // Gravitational constant imported from PhysicsConstants.g
    pub time_stamp: i64,
    pub freq: i64,

    // === API 对象（对应 War Thunder HTTP 端点）===
    pub s_state: Option<State>,     // /state 端点数据
    pub s_indic: Option<Indicators>, // /indicators 端点数据
    pub status_text: Option<String>, // 状态文本
    pub time_text: Option<String>,  // 时间文本

    // === 数值类字段（移除匈牙利前缀）===
    pub total_hp: i32,              // 总马力
    pub total_hp_str: Option<String>, // 总马力字符串
    pub total_hp_eff: i32,          // 有效马力
    pub total_hp_eff_str: Option<String>, // 有效马力字符串
    pub use_mega_hp: bool,          // 是否使用MHp单位
    pub total_thrust: i32,          // 总推力
    pub total_thrust_str: Option<String>, // 总推力字符串
    pub total_fuel: f64,            // 总油量
    pub total_fuel_prev: f64,       // 上次油量（用于计算变化）
    pub low_acc_fuel: bool,         // 低精度燃油警告
    pub total_fuel_str: Option<String>, // 总油量字符串
    pub check_alt: i32,             // 检查高度
    pub fuel_delta: f64,            // 油量变化
    pub fueltime: i64,
    pub fueltime_str: Option<String>, // 油耗时间字符串
    pub not_check_inch: bool,
    // public boolean isFuelpressure;
    pub altper_circlflag: bool,
    pub actual_interval_ms: i64,
    pub althour: f64,
    pub altper_circle: f64,
    pub alt: f64,
    pub altp: f64,
    pub altreg: f64,
    pub iastotascoff: f64,
    pub current_time_ms: i64,
    pub poll_cycle_duration_ms: i64,
    #[allow(dead_code)] // 写者: service_loop 主循环节拍 (Java L1804-1815)
    pub(crate) last_main_loop_time_ms: i64,
    #[allow(dead_code)] // 写者: service_loop 地图轮询节拍 (Java L1830-1832)
    pub(crate) last_map_poll_time_ms: i64,
    #[allow(dead_code)] // 写者: slowcalculate/resetvaria (Java L557/1571)
    pub(crate) fuel_check_mili: i64,
    pub fuel_change: f64,
    #[allow(dead_code)] // 写者: slowcalculate (Java L524)
    pub(crate) fuel_lastchange_mili: i64,
    #[allow(dead_code)] // 写者: slowcalculate (Java L523)
    pub(crate) fuelchange_time: i64,
    #[allow(dead_code)] // Java 全库无读写 (仅声明), 保真保留
    gc_check_mili: i64,
    #[allow(dead_code)] // Java 全库无读写 (仅声明), 保真保留
    slow_check_mili: i64,
    #[allow(dead_code)] // Java 全库无读写 (仅声明), 保真保留
    interval_check_ms: i64,
    /// 外部写者: Controller.java:384 `S.startTime = System.currentTimeMillis()`
    /// (Java 包私有; Controller 若落 vm-data 外的 crate, 届时再上调可见性)
    #[allow(dead_code)] // 写者: Controller 波次
    pub(crate) start_time: i64,
    pub elapsed_time: i64,

    pub noil_temp: f64,
    pub nwater_temp: f64,
    // public int enginenum;
    // public int enginetype;

    pub speedv: f64,
    pub speedvp: f64,
    pub ias_v: f64,
    pub ias_vp: f64,
    pub diffspeed: f64,
    pub acceleration: f64,
    pub sep: f64,

    pub wep_time: i64,

    pub salt: Option<String>,
    pub s_sep: Option<String>,
    pub s_sep_abs: Option<String>,

    pub s_nitro: Option<String>,
    pub s_wep_time: Option<String>,

    // PORT(Java `public Controller c` 不迁移): 环 1 (Controller↔Service) 按 LIFETIMES
    // §4.1 裁决断裂 —— ServiceData 是纯数据快照, 配置读走 Arc<ConfigStore>,
    // 生命周期协作走 service_loop; 保留反向引用 = 重建所有权环, 审查必拒。
    // PORT(Java `private final FocusMonitor focusMonitor` 不迁移): 焦点监控器是
    // 轮询驱动的组件 (tick 由 run() 调), 归 service_loop 线程持有, 非数据快照成员。

    // 对飞机结构有重大影响的警告
    // PORT: Java `Boolean` 装箱 (可 null) → Option<bool> (§1); 初始化器 = false → Some(false)
    pub fatal_warn: Option<bool>,

    // sState转换后
    pub has_wing_sweep_vario: bool,
    pub is_state_jet: bool,
    pub compass_delta: f64,
    pub svalid: Option<String>,
    pub engine_num: i32,
    pub engine_type: Option<String>,
    pub aileron: Option<String>,
    pub elevator: Option<String>,
    pub rudder: Option<String>,
    pub flaps: Option<String>,
    pub gear: Option<String>,
    pub tas: Option<String>,
    pub ias: Option<String>,
    pub m: Option<String>,
    pub aoa: Option<String>,
    pub aos: Option<String>,
    pub ao: Option<String>,
    pub ny: Option<String>,
    pub vy: Option<String>,
    pub wx: Option<String>,
    pub s_n: Option<String>,
    pub throttle: Option<String>,
    pub rpm_throttle: Option<String>,
    pub radiator: Option<String>,
    pub mixture: Option<String>,
    pub compass: Option<String>,
    pub s_acc: Option<String>,
    pub s_turn_rds: Option<String>,
    pub s_wing_sweep: Option<String>,
    pub s_turn_rate: Option<String>,
    pub compressorstage: Option<String>,
    pub magenato: Option<String>,
    pub power: Option<Vec<Option<String>>>,
    pub manifoldpressure: Option<String>,
    /// Java 初始化器 `= "Ata"`
    pub pressure_unit_str: Option<String>,
    pub pressure_pounds: Option<String>,
    pub pressure_inch_hg: Option<String>,
    pub pressure_mm_hg: Option<String>,
    pub watertemp: Option<String>,
    pub oiltemp: Option<String>,
    pub pitch: Option<Vec<Option<String>>>,
    pub thrust: Option<Vec<Option<String>>>,
    pub aclrt: Option<String>,
    pub rel_energy: Option<String>,
    pub cur_load: i32,
    pub cur_load_min_work_time: f64,
    pub efficiency: Option<Vec<Option<String>>>,
    #[allow(dead_code)] // Java 全库无读写 (仅声明), 保真保留
    test_check_mili: i64,
    /// Java `public long loadWorkTimeMili[]` (L172) — 全库无赋值/读取点, null 默认保留;
    /// Java public → 按可见性映射规则给 pub (§1 规则 7)
    pub load_work_time_mili: Option<Vec<i64>>,

    pub ratio: f64,
    pub ratio_1: f64,
    // iIndic
    pub rpm: Option<String>,
    #[allow(dead_code)] // 写者: checkOverheat (service_loop 波次, Java L592)
    pub(crate) cur_w_load: i32,
    #[allow(dead_code)] // 写者: checkOverheat (Java L621)
    pub(crate) cur_o_load: i32,
    /// Java `private double nVy` — 私有但 getVario() 读
    pub(crate) n_vy: f64,
    pub s_horizontal_load: Option<String>,
    pub s_eng_work_time: Option<String>,
    pub s_pitch_up: Option<String>,
    pub s_thurst_percent: Option<String>,
    pub sfuel_percent: Option<String>,
    pub s_avg_eff: Option<String>,
    pub sd_thrust_percent: Option<String>,
    pub s_radio_alt: Option<String>,
    /// Java `Boolean` 装箱, **无初始化器** → null 默认 (isRadioAltitudeValid 的
    /// null 判断是活语义, Java L1990); resetvaria 才赋 false
    pub radio_alt_valid: Option<bool>,
    pub radio_alt: f64,
    pub p_radio_alt: f64,
    pub d_radio_alt: f64,
    pub an: f64,
    pub i_eng_type: i32,
    pub nitrokg: f64,
    pub nitro_consump: f64,
    pub nitro_eng_nr: i32,
    pub s_wep_time_val: i64, // Remaining WEP time in seconds

    /// Optimal compressor stage index for current conditions. -1 = invalid/jet/single-stage
    /// Java 初始化器 `= -1`
    #[allow(dead_code)] // 读者: publishFlightDataEvent (service_loop 波次, Java L459)
    pub(crate) optimal_compressor_stage: i32,
    /// True when actual compressor stage doesn't match optimal (at full throttle)
    /// Java 初始化器 `= false`
    #[allow(dead_code)] // 读者: publishFlightDataEvent (Java L460)
    pub(crate) compressor_stage_mismatch: bool,
    /// Previous actual compressor stage for change detection (0-based, -1 = invalid)
    /// Java 初始化器 `= -1`
    #[allow(dead_code)] // 写者: updateOptimalCompressorStage (Java L1337)
    pub(crate) prev_actual_compressor_stage: i32,
    /// Previous optimal compressor stage for change detection
    /// Java 初始化器 `= -1`
    #[allow(dead_code)] // 写者: updateOptimalCompressorStage (Java L1338)
    pub(crate) prev_optimal_compressor_stage: i32,

    /// Java 包私有 `Boolean portOcupied = false` (装箱 → Option, 初始化器 false)
    #[allow(dead_code)] // 写者: processPollingCycle 端口切换 (Java L1794)
    pub(crate) port_ocupied: Option<bool>,
    #[allow(dead_code)] // 写者: checkEngineJet (Java L489-492)
    pub(crate) check_engine_type: i32,
    #[allow(dead_code)] // 写者: checkEngineJet (Java L495-497)
    pub(crate) check_pitch: i32,
    /// Java 初始化器 `= false`
    pub check_engine_flag: bool,
    // ENGINE_TYPE_* 常量集中声明于 struct 后 (§1 static final → const)
    pub mapinfo: Option<MapInfo>,

    // ---- L219-236 (方法/常量区, 常量见 struct 后) ----
    pub player_live: bool,
    pub s_loc: Option<String>,

    // ---- L651-678 ----
    // 转弯半径和转弯时间计算
    pub turn_rds: f64,
    pub turn_rate: f64,

    pub horizontal_load: f64,
    pub bangle_r: f64,

    pub altmeterp: f64,
    pub altmeter: f64,
    pub thurst_percent: f64,
    #[allow(dead_code)] // 写者: updateEngineState (Java L927-931)
    pub(crate) max_total_thr: i32,
    pub fuel_percent: i32,
    pub avgeff: f64,
    #[allow(dead_code)] // 写者: updateEngineState (Java L930-931)
    pub(crate) max_total_hp: i32,
    #[allow(dead_code)] // 写者: updateSpeed (Java L849), 计算 (Java L934)
    pub(crate) tas_v: f64,
    #[allow(dead_code)] // 写者: updateEngineState (Java L934)
    pub(crate) p_thurst_percent: f64,
    pub t_eng_response: f64,
    pub flap_allow_speed: f64,
    pub flap_allow_angle: f64,
    #[allow(dead_code)] // 写者: checkFlap (Java L1044)
    #[allow(dead_code)] // 写者: checkFlap (Java L1045) / 读者: updateStallSpeed (L1263)
    pub is_downing_flap: bool,
    #[allow(dead_code)] // 写者: checkFlap (Java L1050)
    pub maximum_thr_rpm: f64,
    // double maximumAllowedRPM;
    #[allow(dead_code)] // 写者: getMaximumRPM(fm) 自适应学习 (Java L1085-1095)
    pub(crate) check_maxium_rpm: i64,
    /// Java `public boolean getMaximumRPM` (字段) — 与同名方法 getMaximumRPM(FMHandle)
    /// 构成重载; 方法归 service_loop 波次, 届时命名避让 (如 get_maximum_rpm_learn)
    pub get_maximum_rpm: bool,
    // PORT(Java `public HttpHelper httpClient` 不迁移): IO 机械 (socket + 响应缓冲),
    // 归 service_loop 线程持有, 非数据快照成员。
    pub energy_m: f64,

    // ---- L1180-1184 ----
    pub mach: f64, // 精准mach, 精度高于state.mach, 小于indicators.mach, 不过只有部分飞机有indicators.mach
    pub speed_limit_ratio: f64,
    pub aileron_lock_ratio: f64,
    pub rudder_lock_ratio: f64,
    pub unit_mach_limit_ratio: f64, // 单位马赫数限制比值

    // ---- L1234 ----
    pub stall_speed: f64,

    /// R1 周期 FM 句柄快照 (无 Java 对应字段, 见 struct 级 PORT 注):
    /// service_loop 每周期 `FMManager.current()` 写入; getter 经它读
    /// blkx.nofuelweight (getTotalWeight) / blkx.nitro (hasWep)。
    /// 初始值 = `FMHandle.UNRESOLVED` (对齐 FMManager.current 的 volatile 初值)。
    pub fm: Arc<FMHandle>,

    /// 公式系统一帧求值结果 (公式名→槽号的定位见 CompiledFormulaSet.slots;
    /// 无 Java 对应, 公式系统设计 doc/formula_system_design.md §2 裁决 A1/A2:
    /// Service 线程单点求值, win32 线程经本 RwLock 只读)。
    pub formula_values: vm_core::formula::FormulaResults,
    /// 公式名→结果槽 (formula_step 与 values 同步写; overlay 绑定解析用)
    pub formula_slots: std::sync::Arc<std::collections::HashMap<String, u16>>,
    /// L2 规则本帧触发事件 (formula_step 产出; 消费面 vm-app toast/语音链)
    pub rule_triggers: Vec<vm_core::formula::rules::RuleTriggered>,
    /// 最近一帧变量快照 (formula_step 写; overlay 经 FormulaView 取值)
    pub formula_snapshot: std::sync::Arc<vm_core::formula::VarSnapshot>,
}

/// W7 统一取值视图: 实时直达源头 (不经快照搬运 — 公式值优先, 其余按
/// VarSrc 直取 State/Indicators/Blkx/SessionInputs; 快照仅服务公式求值)
impl vm_core::formula::registry::FormulaView for ServiceData {
    fn var_value(&self, name: &str) -> Option<f64> {
        // 公式优先 (接管语义: 同名公式覆写系统变量)
        if let Some(&slot) = self.formula_slots.get(name) {
            let v = self.formula_values.get(slot);
            if !v.is_nan() {
                return Some(v);
            }
        }
        use vm_core::formula::registry::{registry, VarSrc};
        let vid = registry().lookup(name)?;
        let src = &registry().vars[vid as usize].src;
        let v = match src {
            VarSrc::State(f) => self.s_state.as_ref().map(f)?,
            VarSrc::Indic(f) => self.s_indic.as_ref().map(f)?,
            VarSrc::Blk(f) => self.fm.blkx.as_ref().map(f)?,
            VarSrc::Session(f) => f(&crate::service_loop::session_inputs(self)),
            VarSrc::Const(c) => *c,
            VarSrc::Meta(m) => match m {
                vm_core::formula::registry::MetaVar::IntervalMs => self.actual_interval_ms.max(1) as f64,
                vm_core::formula::registry::MetaVar::Freq => self.freq as f64,
                vm_core::formula::registry::MetaVar::FmLoaded => (self.fm.blkx.is_some()) as u8 as f64,
                _ => 0.0,
            },
        };
        if v.is_nan() { None } else { Some(v) }
    }
}

/// Java `public static final int ENGINE_TYPE_*` (L213-216, DrawFrame.java 外部引用)。
pub const ENGINE_TYPE_PROP: i32 = 0;
pub const ENGINE_TYPE_JET: i32 = 1;
pub const ENGINE_TYPE_TURBOPROP: i32 = 2;
pub const ENGINE_TYPE_UNKNOWN: i32 = -1;

/// Java `public static final String nastring = "-"` (L227)。
/// PORT: indicators.rs 已按 CLASSIFY 裁决内联为私有 NA_STRING (不越文件改, §6);
/// 本处为规范定义, 后续波次统一收敛引用点。
pub const NASTRING: &str = "-";
/// Java `public static final String nullstring = ""` (L228)。
pub const NULLSTRING: &str = "";
/// Java `public static final String pressureUnit = "Ata"` (L236)。
pub const PRESSURE_UNIT: &str = "Ata";

impl Default for ServiceData {
    /// 对应 Java 字段声明默认值: 隐式初始化 (数值 0 / boolean false / 引用 null, §2.10)
    /// + 显式初始化器 (pressureUnitStr="Ata" / fatalWarn=false /
    ///   optimalCompressorStage 族 -1 / portOcupied=false / checkEngineFlag=false)。
    ///
    /// **service_loop 波次验收义务**: Default 是"声明态"而非"构造后态"。Java 构造器
    /// `Service(xc)` (L1678-1703) 无条件跑 clearvaria→resetvaria, 真实初值还包括:
    /// `freq=serviceLoopIntervalMs` / `ratio=freq/1000f` / `ratio_1=1f-ratio` /
    /// `sState`/`sIndic` = new+init / `mapinfo` = new / `power`·`pitch`·`thrust`·
    /// `efficiency` = vec![None; State::MAX_ENG_NUM] / `FuelCheckMili`·
    /// `lastMapPollTimeMs`·`lastMainLoopTimeMs` = 构造时刻; resetvaria (L1526-1660)
    /// 另置 `fueltime=Long.MAX_VALUE` / `maximumThrRPM=1` / `iastotascoff=1` /
    /// `flapAllowSpeed`=`flapAllowAngle`=(f32::MAX as f64, Java Float.MAX_VALUE 拓宽) /
    /// `curLoadMinWorkTime=99999*1000` /
    /// `iEngType=ENGINE_TYPE_UNKNOWN` / `radioAltValid=Some(false)` / `svalid="false"` /
    /// `loc`=`dir`=[0.0;2] / 7 个 SMA 构造 (窗口 1000/freq, fuelTimeSMA=4, 见字段区
    /// PORT 注)。service_loop 构造接线漏任何一项, fueltimeStr 等显示语义会静默漂移。
    fn default() -> Self {
        ServiceData {
            diff_speed_sma: None,
            sep_sma: None,
            turnrds_sma: None,
            sum_speed_sma: None,
            calc_speed_sma: None,
            fuel_time_sma: None,
            energy_diff_sma: None,
            loc: None,
            dir: None,
            energy_j_kg: 0.0,
            prev_energy_j_kg: 0.0,
            calc_period: 0,
            time_stamp: 0,
            freq: 0,
            s_state: None,
            s_indic: None,
            status_text: None,
            time_text: None,
            total_hp: 0,
            total_hp_str: None,
            total_hp_eff: 0,
            total_hp_eff_str: None,
            use_mega_hp: false,
            total_thrust: 0,
            total_thrust_str: None,
            total_fuel: 0.0,
            total_fuel_prev: 0.0,
            low_acc_fuel: false,
            total_fuel_str: None,
            check_alt: 0,
            fuel_delta: 0.0,
            fueltime: 0,
            fueltime_str: None,
            not_check_inch: false,
            altper_circlflag: false,
            actual_interval_ms: 0,
            althour: 0.0,
            altper_circle: 0.0,
            alt: 0.0,
            altp: 0.0,
            altreg: 0.0,
            iastotascoff: 0.0,
            current_time_ms: 0,
            poll_cycle_duration_ms: 0,
            last_main_loop_time_ms: 0,
            last_map_poll_time_ms: 0,
            fuel_check_mili: 0,
            fuel_change: 0.0,
            fuel_lastchange_mili: 0,
            fuelchange_time: 0,
            gc_check_mili: 0,
            slow_check_mili: 0,
            interval_check_ms: 0,
            start_time: 0,
            elapsed_time: 0,
            noil_temp: 0.0,
            nwater_temp: 0.0,
            speedv: 0.0,
            speedvp: 0.0,
            ias_v: 0.0,
            ias_vp: 0.0,
            diffspeed: 0.0,
            acceleration: 0.0,
            sep: 0.0,
            wep_time: 0,
            salt: None,
            s_sep: None,
            s_sep_abs: None,
            s_nitro: None,
            s_wep_time: None,
            fatal_warn: Some(false),
            has_wing_sweep_vario: false,
            is_state_jet: false,
            compass_delta: 0.0,
            svalid: None,
            engine_num: 0,
            engine_type: None,
            aileron: None,
            elevator: None,
            rudder: None,
            flaps: None,
            gear: None,
            tas: None,
            ias: None,
            m: None,
            aoa: None,
            aos: None,
            ao: None,
            ny: None,
            vy: None,
            wx: None,
            s_n: None,
            throttle: None,
            rpm_throttle: None,
            radiator: None,
            mixture: None,
            compass: None,
            s_acc: None,
            s_turn_rds: None,
            s_wing_sweep: None,
            s_turn_rate: None,
            compressorstage: None,
            magenato: None,
            power: None,
            manifoldpressure: None,
            pressure_unit_str: Some(PRESSURE_UNIT.to_string()),
            pressure_pounds: None,
            pressure_inch_hg: None,
            pressure_mm_hg: None,
            watertemp: None,
            oiltemp: None,
            pitch: None,
            thrust: None,
            aclrt: None,
            rel_energy: None,
            cur_load: 0,
            cur_load_min_work_time: 0.0,
            efficiency: None,
            test_check_mili: 0,
            load_work_time_mili: None,
            ratio: 0.0,
            ratio_1: 0.0,
            rpm: None,
            cur_w_load: 0,
            cur_o_load: 0,
            n_vy: 0.0,
            s_horizontal_load: None,
            s_eng_work_time: None,
            s_pitch_up: None,
            s_thurst_percent: None,
            sfuel_percent: None,
            s_avg_eff: None,
            sd_thrust_percent: None,
            s_radio_alt: None,
            radio_alt_valid: None,
            radio_alt: 0.0,
            p_radio_alt: 0.0,
            d_radio_alt: 0.0,
            an: 0.0,
            i_eng_type: 0,
            nitrokg: 0.0,
            nitro_consump: 0.0,
            nitro_eng_nr: 0,
            s_wep_time_val: 0,
            optimal_compressor_stage: -1,
            compressor_stage_mismatch: false,
            prev_actual_compressor_stage: -1,
            prev_optimal_compressor_stage: -1,
            port_ocupied: Some(false),
            check_engine_type: 0,
            check_pitch: 0,
            check_engine_flag: false,
            mapinfo: None,
            player_live: false,
            s_loc: None,
            turn_rds: 0.0,
            turn_rate: 0.0,
            horizontal_load: 0.0,
            bangle_r: 0.0,
            altmeterp: 0.0,
            altmeter: 0.0,
            thurst_percent: 0.0,
            max_total_thr: 0,
            fuel_percent: 0,
            avgeff: 0.0,
            max_total_hp: 0,
            tas_v: 0.0,
            p_thurst_percent: 0.0,
            t_eng_response: 0.0,
            flap_allow_speed: 0.0,
            flap_allow_angle: 0.0,
            is_downing_flap: false,
            maximum_thr_rpm: 0.0,
            check_maxium_rpm: 0,
            get_maximum_rpm: false,
            energy_m: 0.0,
            mach: 0.0,
            speed_limit_ratio: 0.0,
            aileron_lock_ratio: 0.0,
            rudder_lock_ratio: 0.0,
            unit_mach_limit_ratio: 0.0,
            stall_speed: 0.0,
            fm: Arc::new(FMHandle::UNRESOLVED),
            formula_values: Default::default(),
            formula_slots: std::sync::Arc::default(),
            rule_triggers: Vec::new(),
            formula_snapshot: std::sync::Arc::new(vm_core::formula::VarSnapshot::empty(
                vm_core::formula::registry().len(),
            )),
        }
    }
}

impl ServiceData {
    /// Java `public boolean isPlayerLive()` (L232-234) — processPollingCycle 的存活闸门。
    pub fn is_player_live(&self) -> bool {
        self.player_live
    }
}

// --- TelemetrySource Implementation ---

impl TelemetrySource for ServiceData {
    // W7: 71 个 getter 实现整体消解 — 数据面统一走 var_value 桥
    // (快照+公式槽+Session; 快照由 formula_step 每帧写入)。仅保留少数
    // 显示类专用 getter (String/精度, 不适合数值快照通道)。
    fn var_value(&self, name: &str) -> Option<f64> {
        vm_core::formula::registry::FormulaView::var_value(self, name)
    }

    fn get_formula_value(&self, name: &str) -> Option<f64> {
        let slot = self.formula_slots.get(name)?;
        let v = self.formula_values.get(*slot);
        if v.is_nan() { None } else { Some(v) }
    }

    fn get_indic_speed(&self) -> f64 {
        self.s_indic.as_ref().map_or(-65535.0, |i| i.speed)
    }

    fn get_ny_raw(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.ny)
    }

    fn get_manifold_pressure_display_unit(&self) -> String {
        if self.check_alt > 0 { "P/xx".into() } else { "Ata".into() }
    }

    fn get_manifold_pressure_display_precision(&self) -> i32 {
        if self.check_alt > 0 { 1 } else { 2 }
    }
}

// =====================================================================
// Tests — 公共项边界测试 (§5.2 B 类单测; 断言值 = Java 语义逐行推导,
// mock 快照与 state.rs/indicators.rs 的 Java 8 oracle 数据同源)
// =====================================================================
#[cfg(test)]
mod tests;
