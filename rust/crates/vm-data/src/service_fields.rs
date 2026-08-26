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
use vm_core::string_helper::F_INVALID;
use vm_core::ui_model::TelemetrySource;
use vm_core::{format, g};

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
    pub(crate) flapp: i32,
    #[allow(dead_code)] // 写者: checkFlap (Java L1045) / 读者: updateStallSpeed (L1263)
    pub(crate) flap: i32,
    pub is_downing_flap: bool,
    #[allow(dead_code)] // 写者: checkFlap (Java L1050)
    pub(crate) flap_check: i64,
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
            flapp: 0,
            flap: 0,
            is_downing_flap: false,
            flap_check: 0,
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
    fn get_ias(&self) -> f64 {
        // PORT: Java `sState.IAS` int → double 拓宽
        self.s_state.as_ref().map_or(0.0, |s| s.ias as f64)
    }

    fn get_tas(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.tas as f64)
    }

    fn get_mach(&self) -> f64 {
        self.mach
    }

    fn get_aoa(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.aoa)
    }

    fn get_aos(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.aos)
    }

    fn get_ny(&self) -> f64 {
        self.an / g
    }

    fn get_vario(&self) -> f64 {
        self.n_vy
    }

    fn get_horse_power(&self) -> f64 {
        // PORT: Java `return totalHp` int → double 拓宽
        self.total_hp as f64
    }

    fn get_engine_response(&self) -> f64 {
        self.t_eng_response
    }

    fn get_prop_efficiency(&self) -> f64 {
        self.avgeff
    }

    fn get_manifold_pressure_pounds(&self) -> f64 {
        self.s_state
            .as_ref()
            .map_or(0.0, |s| (s.manifoldpressure - 1.0) * 14.696)
    }

    fn get_manifold_pressure_inch_hg(&self) -> f64 {
        // PORT: Java `sState.manifoldpressure * 760 / 25.4` — int 760 提升为 double
        self.s_state
            .as_ref()
            .map_or(0.0, |s| s.manifoldpressure * 760.0 / 25.4)
    }

    fn get_manifold_pressure_display(&self) -> f64 {
        if self.is_imperial() {
            self.get_manifold_pressure_pounds()
        } else {
            self.get_manifold_pressure()
        }
    }

    fn get_manifold_pressure_display_unit(&self) -> String {
        if self.is_imperial() {
            // PORT: Java `String.format("P/%.1f''", ...)` — Formatter %.1f 为 HALF_UP →
            // crate::format::format(v, 1) 复刻 (Rust {:.1} 是半偶舍入, §2.3, 禁直接用)
            return format!("P/{}''", format::format(self.get_manifold_pressure_inch_hg(), 1));
        }
        "Ata".to_string()
    }

    fn get_manifold_pressure_display_precision(&self) -> i32 {
        if self.is_imperial() {
            1
        } else {
            2
        }
    }

    fn get_unknown_mixture(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.mixture as f64)
    }

    fn get_radiator(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.radiator as f64)
    }

    fn get_compressor_stage(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.compressorstage as f64)
    }

    fn get_fuel_percent(&self) -> f64 {
        self.fuel_percent as f64
    }

    fn get_rpm_throttle(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.rpm_throttle as f64)
    }

    fn get_altitude(&self) -> f64 {
        self.alt
    }

    fn get_radio_altitude(&self) -> f64 {
        self.radio_alt
    }

    fn is_radio_altitude_valid(&self) -> bool {
        // PORT: Java `radioAltValid != null && radioAltValid` — Boolean 装箱 null 判断
        self.radio_alt_valid == Some(true)
    }

    fn get_compass(&self) -> f64 {
        self.compass_delta
    }

    fn get_sep(&self) -> f64 {
        self.sep
    }

    fn get_acceleration(&self) -> f64 {
        self.acceleration
    }

    fn get_turn_rate(&self) -> f64 {
        self.turn_rate
    }

    fn get_turn_radius(&self) -> f64 {
        self.turn_rds.abs()
    }

    /// 判断回转半径是否有效（<= 9999m）
    /// 回转半径过大时（如直飞或缓慢转弯）返回 false，隐藏该数据行
    fn is_turn_radius_valid(&self) -> bool {
        self.turn_rds.abs() <= 9999.0
    }

    fn get_roll_rate(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.wx.abs())
    }

    fn get_mass_fuel(&self) -> f64 {
        self.total_fuel
    }

    /// Get total aircraft weight (nofuelweight + current fuel).
    /// @return Total weight in kg, or 0 if FM data unavailable
    fn get_total_weight(&self) -> f64 {
        // R1 快照读: 单次 volatile 读（可能被 EDT 的 HUDCalculator 回退路径调用, 纯读安全）;
        // R2 守卫: 非 READY 句柄 blkx=null → 返回 0, 走 UI 端 hide-when-zero 隐藏
        // PORT: Java 经 FMManager.getInstance().current() 现读单例 → 读本 struct 的
        // fm 周期快照字段 (LIFETIMES §7, 见 struct 级注)
        match (&self.fm.blkx, &self.s_state) {
            (Some(blkx), Some(s)) => blkx.nofuelweight + s.mfuel,
            _ => 0.0,
        }
    }

    fn get_fuel_time_mili(&self) -> i64 {
        self.fueltime
    }

    fn get_throttle(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.throttle as f64)
    }

    fn get_rpm(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.rpm as f64)
    }

    fn get_manifold_pressure(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.manifoldpressure)
    }

    fn get_water_temp(&self) -> f64 {
        // 水温对所有机型都显示，包括喷气机
        self.nwater_temp
    }

    fn get_oil_temp(&self) -> f64 {
        // 油温对所有机型都显示，包括喷气机
        self.noil_temp
    }

    fn get_pitch(&self) -> f64 {
        // PORT: Java `sState.pitch[0]` — pitch[] 为 null (未 init) 时 Java 抛 NPE,
        // Rust 空 Vec 索引 panic (get_thrust 的 thrust[0] 同理)。注意 panic 兜底边界:
        // §6 的 catch_unwind 只覆盖 service_loop 的 Service 线程 (对齐 Java L1850 顶层
        // catch); 本 getter 若被 EDT 侧 HUDCalculator 回退路径调用, Java NPE 由 AWT
        // 事件派发线程吞掉而 Rust panic 会杀 UI 线程 —— P4/P5 必须为 UI 线程补
        // panic 边界 (此处保真保留 panic, 不在 getter 内吞)
        self.s_state.as_ref().map_or(0.0, |s| s.pitch[0])
    }

    fn get_thrust(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.thrust[0] as f64)
    }

    fn get_gear(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.gear as f64)
    }

    fn get_flaps(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.flaps as f64)
    }

    fn get_airbrake(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.airbrake as f64)
    }

    fn get_aileron(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.aileron as f64)
    }

    fn get_elevator(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.elevator as f64)
    }

    fn get_rudder(&self) -> f64 {
        self.s_state.as_ref().map_or(0.0, |s| s.rudder as f64)
    }

    fn get_wing_sweep(&self) -> f64 {
        // -65535 是 API 无效标记，表示飞机没有可变翼功能
        // 返回 0 使 visible-when (!= value 0) 能正确隐藏此字段
        // PORT: -65535 即 string_helper::F_INVALID 哨兵 (float 域)
        match self.s_indic.as_ref() {
            Some(i) if i.wsweep_indicator != F_INVALID => i.wsweep_indicator,
            _ => 0.0,
        }
    }

    // PORT: Java getEnergyJKg → trait 方法名 get_energy_jkg (字段名 energy_j_kg)
    fn get_energy_jkg(&self) -> f64 {
        self.energy_j_kg
    }

    fn get_eff_hp(&self) -> f64 {
        self.total_hp_eff as f64
    }

    fn get_wep_kg(&self) -> f64 {
        self.nitrokg
    }

    fn get_wep_time(&self) -> f64 {
        // PORT: Java `return sWepTimeVal` long → double 拓宽
        self.s_wep_time_val as f64
    }

    // === 火箭助推器 (Issue #52) ===

    // PORT: Java 保真 — 守卫 `!(mfuel_1 <= 0.0)` 极性不翻 (NaN 穿透, §2.12),
    // 不改写为 partial_cmp 形态
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn get_booster_fuel_kg(&self) -> f64 {
        // 无助推器时 mfuel_1 为 -65535，返回 0
        // PORT: Java 守卫 `mfuel_1 <= 0` — NaN 时 `<=` 为 false, 穿透返回 NaN;
        // 守卫极性翻成 `> 0` 会把 NaN 静默归零, 故 `!(x <= 0.0)` 原样复刻 (§2.12)
        match self.s_state.as_ref() {
            Some(s) if !(s.mfuel_1 <= 0.0) => s.mfuel_1,
            _ => 0.0,
        }
    }

    // PORT: Java 保真 — 守卫 `!(mfuel0_1 <= 0.0)` 极性不翻 (NaN 穿透, §2.12),
    // 不改写为 partial_cmp 形态
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn get_booster_fuel_percent(&self) -> f64 {
        // 计算助推器剩余百分比 = 当前助推燃料 / 助推燃料总量 * 100
        // PORT: 守卫 `!(mfuel0_1 <= 0.0)` 原样复刻 Java `mfuel0_1 <= 0` 的 NaN 穿透
        // (§2.12); Java `Math.min(100, ...)` int 100 提升为 double, 且
        // Math.min(double,double) NaN 传播 — f64::min 会吞 NaN 返 100.0, 手写复刻
        match self.s_state.as_ref() {
            Some(s) if !(s.mfuel0_1 <= 0.0) => {
                let v = 100.0 * s.mfuel_1 / s.mfuel0_1;
                if v.is_nan() {
                    v
                } else {
                    v.min(100.0)
                }
            }
            _ => 0.0,
        }
    }

    fn has_booster(&self) -> bool {
        // mfuel_1 > 0 说明当前有助推器燃料，即有助推器系统
        match self.s_state.as_ref() {
            Some(s) => s.mfuel_1 > 0.0 && s.mfuel0_1 > 0.0,
            None => false,
        }
    }

    fn get_heat_tolerance(&self) -> f64 {
        // 直接返回原始值，UI层通过 :na-when 表达式过滤无效值
        self.cur_load_min_work_time / 1000.0
    }

    fn get_power_percent(&self) -> f64 {
        // PORT: Java Math.min(thurstPercent, 100.0) — NaN 传播 (f64::min 会吞 NaN 返
        // 100.0, 手写复刻); 现域 thurstPercent 除数 peak/maxTotalThr 有非零守卫,
        // 无 NaN 通路, 此处纯保形
        let v = self.thurst_percent;
        if v.is_nan() {
            v
        } else {
            v.min(100.0)
        }
    }

    fn is_imperial(&self) -> bool {
        self.check_alt > 0
    }

    fn is_wing_sweep_valid(&self) -> bool {
        match self.s_indic.as_ref() {
            Some(i) => i.wsweep_indicator != F_INVALID,
            None => false,
        }
    }

    fn get_speed_limit_ratio(&self) -> f64 {
        self.speed_limit_ratio
    }

    fn get_aileron_lock_ratio(&self) -> f64 {
        self.aileron_lock_ratio
    }

    fn get_rudder_lock_ratio(&self) -> f64 {
        self.rudder_lock_ratio
    }

    fn get_unit_mach_limit_ratio(&self) -> f64 {
        self.unit_mach_limit_ratio
    }

    fn get_stall_speed(&self) -> f64 {
        self.stall_speed
    }

    fn get_aviahorizon_pitch(&self) -> f64 {
        self.s_indic.as_ref().map_or(0.0, |i| i.aviahorizon_pitch)
    }

    fn get_aviahorizon_roll(&self) -> f64 {
        self.s_indic.as_ref().map_or(0.0, |i| i.aviahorizon_roll)
    }

    // === 引擎类型与飞机特性判断（用于 :visible-when 表达式）===

    /// 判断是否为喷气发动机
    /// 检测完成前（约5秒）返回 false
    fn is_jet_engine(&self) -> bool {
        self.check_engine_flag && self.i_eng_type == ENGINE_TYPE_JET
    }

    /// 判断是否为螺旋桨发动机（活塞或涡桨）
    /// 检测完成前（约5秒）返回 false
    fn is_prop_engine(&self) -> bool {
        self.check_engine_flag
            && (self.i_eng_type == ENGINE_TYPE_PROP || self.i_eng_type == ENGINE_TYPE_TURBOPROP)
    }

    /// 判断是否为活塞发动机（不包括涡桨）
    /// 检测完成前（约5秒）返回 false
    fn is_piston_engine(&self) -> bool {
        self.check_engine_flag && self.i_eng_type == ENGINE_TYPE_PROP
    }

    /// 判断是否为涡轮螺旋桨发动机
    /// 检测完成前（约5秒）返回 false
    fn is_turboprop_engine(&self) -> bool {
        self.check_engine_flag && self.i_eng_type == ENGINE_TYPE_TURBOPROP
    }

    /// 判断引擎类型检测是否完成
    fn is_engine_check_done(&self) -> bool {
        self.check_engine_flag
    }

    /// 判断飞机是否有加力系统
    /// 检查 FM 数据中的 nitro 值
    fn has_wep(&self) -> bool {
        // R1/R2: 单次 volatile 读; blkx 非 null 即 READY, 无 FM → false
        // PORT: Java 经 FMManager.getInstance().current() 现读单例 → fm 周期快照字段
        self.fm
            .blkx
            .as_ref()
            .is_some_and(|blkx| blkx.nitro > 0.0)
    }
}

// =====================================================================
// Tests — 公共项边界测试 (§5.2 B 类单测; 断言值 = Java 语义逐行推导,
// mock 快照与 state.rs/indicators.rs 的 Java 8 oracle 数据同源)
// =====================================================================
#[cfg(test)]
mod tests {
    // PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
    // 不改成 struct 字面量以保持与 Java 测试源逐行对应
    #![allow(clippy::field_reassign_with_default)]

    use super::*;
    use vm_core::blkx::Blkx;

    /// 真机抓取的 /state 快照 (state.rs 测试同源, 断言值 = Java 8 oracle 实测)
    const STATE_MOCK: &str = "{\"valid\": true,\"aileron, %\": -48,\"elevator, %\": 20,\"rudder, %\": -47,\"flaps, %\": 0,\"gear, %\": 0,\"H, m\": 46,\"TAS, km/h\": 454,\"IAS, km/h\": 474,\"M\": 0.39,\"AoA, deg\": -1.6,\"AoS, deg\": -5.9,\"Ny\": 0.35,\"Vy, m/s\": -7.3,\"Wx, deg/s\": -34,\"Mfuel, kg\": 197,\"Mfuel0, kg\": 734,\"throttle 1, %\": 110,\"RPM throttle 1, %\": 100,\"mixture 1, %\": 100,\"radiator 1, %\": 42,\"magneto 1\": 3,\"power 1, hp\": 1597.8,\"RPM 1\": 3001,\"manifold pressure 1, atm\": 2.24,\"water temp 1, C\": 121,\"oil temp 1, C\": 90,\"pitch 1, deg\": 35.5,\"thrust 1, kgs\": 840,\"efficiency 1, %\": 87}";

    fn mock_state() -> State {
        let mut st = State::new();
        st.init();
        st.update(STATE_MOCK);
        st
    }

    fn mock_indicators() -> Indicators {
        let mut i = Indicators::new();
        // 手工装填 getter 所需字段 (aviahorizon/wsweep), 走 Indicators::new 的 0 默认
        i.aviahorizon_pitch = 12.5;
        i.aviahorizon_roll = -30.0;
        i.wsweep_indicator = 0.55;
        i
    }

    /// Java 字段声明默认值 + 显式初始化器 (§2.10) 逐项核对
    #[test]
    fn default_matches_java_field_initializers() {
        let d = ServiceData::default();
        // 显式初始化器
        assert_eq!(d.pressure_unit_str.as_deref(), Some("Ata"));
        assert_eq!(d.fatal_warn, Some(false));
        assert_eq!(d.optimal_compressor_stage, -1);
        assert!(!d.compressor_stage_mismatch);
        assert_eq!(d.prev_actual_compressor_stage, -1);
        assert_eq!(d.prev_optimal_compressor_stage, -1);
        assert_eq!(d.port_ocupied, Some(false));
        assert!(!d.check_engine_flag);
        // 隐式默认: 引用 null / 数值 0 / boolean false
        assert!(d.s_state.is_none());
        assert!(d.s_indic.is_none());
        assert!(d.mapinfo.is_none());
        assert!(d.loc.is_none());
        assert!(d.dir.is_none());
        assert!(d.diff_speed_sma.is_none());
        assert!(d.fuel_time_sma.is_none());
        assert!(d.radio_alt_valid.is_none(), "Boolean 无初始化器 → null");
        assert_eq!(d.fueltime, 0);
        assert_eq!(d.i_eng_type, 0, "Java int 默认 0 (resetvaria 才置 UNKNOWN=-1)");
        assert_eq!(d.turn_rds, 0.0);
        assert_eq!(d.mach, 0.0);
        assert!(!d.player_live);
        assert!(d.fm.blkx.is_none(), "fm 快照初值 = UNRESOLVED (blkx=null)");
        // 常量 (Java L213-216/227-228/236)
        assert_eq!(ENGINE_TYPE_PROP, 0);
        assert_eq!(ENGINE_TYPE_JET, 1);
        assert_eq!(ENGINE_TYPE_TURBOPROP, 2);
        assert_eq!(ENGINE_TYPE_UNKNOWN, -1);
        assert_eq!(NASTRING, "-");
        assert_eq!(NULLSTRING, "");
        assert_eq!(PRESSURE_UNIT, "Ata");
    }

    /// 默认态 (sState/sIndic null, fm=UNRESOLVED) 下 getter 的降级语义
    /// (Java L1877 起 `sState != null ? ... : 0` 全分支 + FM 守卫)
    #[test]
    fn null_state_getters_return_defaults() {
        let src: Box<dyn TelemetrySource> = Box::new(ServiceData::default());
        assert_eq!(src.get_ias(), 0.0);
        assert_eq!(src.get_tas(), 0.0);
        assert_eq!(src.get_aoa(), 0.0);
        assert_eq!(src.get_aos(), 0.0);
        assert_eq!(src.get_throttle(), 0.0);
        assert_eq!(src.get_rpm(), 0.0);
        assert_eq!(src.get_manifold_pressure(), 0.0);
        assert_eq!(src.get_manifold_pressure_pounds(), 0.0);
        assert_eq!(src.get_manifold_pressure_inch_hg(), 0.0);
        assert_eq!(src.get_unknown_mixture(), 0.0);
        assert_eq!(src.get_radiator(), 0.0);
        assert_eq!(src.get_compressor_stage(), 0.0);
        assert_eq!(src.get_rpm_throttle(), 0.0);
        assert_eq!(src.get_gear(), 0.0);
        assert_eq!(src.get_flaps(), 0.0);
        assert_eq!(src.get_airbrake(), 0.0);
        assert_eq!(src.get_aileron(), 0.0);
        assert_eq!(src.get_elevator(), 0.0);
        assert_eq!(src.get_rudder(), 0.0);
        assert_eq!(src.get_pitch(), 0.0);
        assert_eq!(src.get_thrust(), 0.0);
        assert_eq!(src.get_roll_rate(), 0.0);
        assert_eq!(src.get_aviahorizon_pitch(), 0.0);
        assert_eq!(src.get_aviahorizon_roll(), 0.0);
        assert_eq!(src.get_wing_sweep(), 0.0);
        assert!(!src.is_wing_sweep_valid());
        // FM 守卫: UNRESOLVED 句柄 blkx=null
        assert_eq!(src.get_total_weight(), 0.0);
        assert!(!src.has_wep());
        // 助推器守卫
        assert_eq!(src.get_booster_fuel_kg(), 0.0);
        assert_eq!(src.get_booster_fuel_percent(), 0.0);
        assert!(!src.has_booster());
        // 英制切换默认关 (checkAlt=0)
        assert!(!src.is_imperial());
        assert_eq!(src.get_manifold_pressure_display(), 0.0);
        assert_eq!(src.get_manifold_pressure_display_unit(), "Ata");
        assert_eq!(src.get_manifold_pressure_display_precision(), 2);
        // radioAltValid null → false (Boolean 装箱语义)
        assert!(!src.is_radio_altitude_valid());
        // turnRds=0 → |0| <= 9999 → 有效 (Java 同此)
        assert!(src.is_turn_radius_valid());
        assert_eq!(src.get_turn_radius(), 0.0);
        // 引擎检测未完成 → 类型判断全 false
        assert!(!src.is_jet_engine());
        assert!(!src.is_prop_engine());
        assert!(!src.is_engine_check_done());
        // trait 对象分发 (消费方以 dyn TelemetrySource 引用)
        assert_eq!(src.get_fuel_time_mili(), 0);
    }

    /// 快照直读族: int→double 拓宽 / 纯字段读 / 派生量直读 (Deriver 写入后)
    #[test]
    fn snapshot_reads_widen_and_passthrough() {
        let mut d = ServiceData::default();
        d.s_state = Some(mock_state());
        d.s_indic = Some(mock_indicators());
        // Deriver 写入的派生量 (来源: data/derive.rs FlightValues)
        d.an = 17.34;
        d.n_vy = -7.3;
        d.mach = 0.391;
        d.sep = 12.5;
        d.acceleration = 1.25;
        d.turn_rds = -8000.0;
        d.turn_rate = 4.25;
        d.compass_delta = 164.1;
        d.alt = 46.0;
        d.radio_alt = 120.5;
        d.total_fuel = 197.0;
        d.total_hp = 1597;
        d.total_hp_eff = 1620;
        d.fuel_percent = 26;
        d.t_eng_response = 3.75;
        d.avgeff = 101.5;
        d.nitrokg = 85.0;
        d.s_wep_time_val = 270;
        d.fueltime = 2_700_000;
        d.cur_load_min_work_time = 300_000.0;
        d.energy_j_kg = 1234.5;
        d.noil_temp = 90.0;
        d.nwater_temp = 121.0;
        d.speed_limit_ratio = 0.72;
        d.aileron_lock_ratio = 0.41;
        d.rudder_lock_ratio = 0.33;
        d.unit_mach_limit_ratio = 0.66;
        d.stall_speed = 155.5;

        // int → double 拓宽 (Java State int 字段)
        assert_eq!(d.get_ias(), 474.0);
        assert_eq!(d.get_tas(), 454.0);
        assert_eq!(d.get_throttle(), 110.0);
        assert_eq!(d.get_rpm(), 3001.0);
        assert_eq!(d.get_rpm_throttle(), 100.0);
        assert_eq!(d.get_radiator(), 42.0);
        assert_eq!(d.get_unknown_mixture(), 100.0);
        assert_eq!(d.get_compressor_stage(), 0.0);
        assert_eq!(d.get_gear(), 0.0);
        assert_eq!(d.get_flaps(), 0.0);
        assert_eq!(d.get_airbrake(), -65535.0, "哨兵 int 原样拓宽 (Java 同此)");
        assert_eq!(d.get_aileron(), -48.0);
        assert_eq!(d.get_elevator(), 20.0);
        assert_eq!(d.get_rudder(), -47.0);
        assert_eq!(d.get_thrust(), 840.0);
        assert_eq!(d.get_fuel_percent(), 26.0);
        assert_eq!(d.get_horse_power(), 1597.0);
        assert_eq!(d.get_eff_hp(), 1620.0);
        // double 直读 (Float.parseFloat 拓宽值)
        assert_eq!(d.get_aoa(), -1.6f32 as f64);
        assert_eq!(d.get_aos(), -5.9f32 as f64);
        assert_eq!(d.get_manifold_pressure(), 2.24f32 as f64);
        assert_eq!(d.get_pitch(), 35.5f32 as f64);
        assert_eq!(d.get_roll_rate(), 34.0, "Math.abs(Wx)");
        assert_eq!(d.get_aviahorizon_pitch(), 12.5);
        assert_eq!(d.get_aviahorizon_roll(), -30.0);
        // getNy = An/g (Java L1901-1903)
        assert!((d.get_ny() - 17.34 / g).abs() < 1e-12);
        // 派生量直读
        assert_eq!(d.get_vario(), -7.3);
        assert_eq!(d.get_mach(), 0.391);
        assert_eq!(d.get_sep(), 12.5);
        assert_eq!(d.get_acceleration(), 1.25);
        assert_eq!(d.get_turn_rate(), 4.25);
        assert_eq!(d.get_compass(), 164.1);
        assert_eq!(d.get_altitude(), 46.0);
        assert_eq!(d.get_radio_altitude(), 120.5);
        assert_eq!(d.get_mass_fuel(), 197.0);
        assert_eq!(d.get_engine_response(), 3.75);
        assert_eq!(d.get_prop_efficiency(), 101.5);
        assert_eq!(d.get_wep_kg(), 85.0);
        assert_eq!(d.get_energy_jkg(), 1234.5);
        assert_eq!(d.get_water_temp(), 121.0);
        assert_eq!(d.get_oil_temp(), 90.0);
        assert_eq!(d.get_speed_limit_ratio(), 0.72);
        assert_eq!(d.get_aileron_lock_ratio(), 0.41);
        assert_eq!(d.get_rudder_lock_ratio(), 0.33);
        assert_eq!(d.get_unit_mach_limit_ratio(), 0.66);
        assert_eq!(d.get_stall_speed(), 155.5);
        // long 拓宽 / i64 直读
        assert_eq!(d.get_wep_time(), 270.0, "sWepTimeVal long → double");
        assert_eq!(d.get_fuel_time_mili(), 2_700_000);
        // 派生换算
        assert_eq!(d.get_heat_tolerance(), 300.0, "curLoadMinWorkTime / 1000.0");
        // Math.abs + 9999 边界
        assert_eq!(d.get_turn_radius(), 8000.0, "Math.abs(turnRds)");
        assert!(d.is_turn_radius_valid());
        d.turn_rds = -9999.0;
        assert!(d.is_turn_radius_valid(), "<= 9999 边界含等号");
        d.turn_rds = 9999.1;
        assert!(!d.is_turn_radius_valid());
        assert!(d.is_player_live() == d.player_live);
    }

    /// 英制/公制切换 (checkAlt 符号) 驱动的进气压显示三件套 (Java L1926-1951/2184-2186)
    #[test]
    fn imperial_switch_manifold_display() {
        let mut d = ServiceData::default();
        let mut st = State::new();
        st.manifoldpressure = 2.25;
        d.s_state = Some(st);

        // 公制 (checkAlt <= 0): 值 = Ata 原值, 单位 Ata, 精度 2
        assert!(!d.is_imperial());
        assert_eq!(d.get_manifold_pressure_display(), 2.25);
        assert_eq!(d.get_manifold_pressure_display_unit(), "Ata");
        assert_eq!(d.get_manifold_pressure_display_precision(), 2);

        // 英制 (checkAlt > 0): 值 = Boost(psi), 单位 = P/xx.x'' (live inHg), 精度 1
        d.check_alt = 1;
        assert!(d.is_imperial());
        assert!((d.get_manifold_pressure_pounds() - (2.25 - 1.0) * 14.696).abs() < 1e-12);
        // 2.25 * 760 / 25.4 = 67.322834... → %.1f HALF_UP → 67.3
        assert_eq!(d.get_manifold_pressure_inch_hg(), 2.25 * 760.0 / 25.4);
        assert_eq!(d.get_manifold_pressure_display_unit(), "P/67.3''");
        assert_eq!(d.get_manifold_pressure_display(), (2.25 - 1.0) * 14.696);
        assert_eq!(d.get_manifold_pressure_display_precision(), 1);

        // Float.parseFloat 单精度拓宽值的公式一致性 (mock 快照 2.24f32)
        d.check_alt = 0;
        let mut st2 = mock_state();
        st2.manifoldpressure = 2.24f32 as f64;
        d.s_state = Some(st2);
        d.check_alt = 5;
        let mp = 2.24f32 as f64;
        assert!((d.get_manifold_pressure_pounds() - (mp - 1.0) * 14.696).abs() < 1e-12);
        assert!((d.get_manifold_pressure_inch_hg() - mp * 760.0 / 25.4).abs() < 1e-12);
    }

    /// 引擎类型判断: checkEngineFlag 闸门 + 四型组合 (Java L2235-2272)
    #[test]
    fn engine_type_flags_require_check_done() {
        let mut d = ServiceData::default();
        // 检测未完成: 即便 iEngType 已有值也全 false
        d.i_eng_type = ENGINE_TYPE_JET;
        assert!(!d.check_engine_flag);
        assert!(!d.is_jet_engine());
        assert!(!d.is_prop_engine());
        assert!(!d.is_piston_engine());
        assert!(!d.is_turboprop_engine());
        assert!(!d.is_engine_check_done());

        d.check_engine_flag = true;
        d.i_eng_type = ENGINE_TYPE_JET;
        assert!(d.is_jet_engine());
        assert!(!d.is_prop_engine());
        assert!(!d.is_piston_engine());
        assert!(!d.is_turboprop_engine());

        d.i_eng_type = ENGINE_TYPE_PROP;
        assert!(!d.is_jet_engine());
        assert!(d.is_prop_engine());
        assert!(d.is_piston_engine());
        assert!(!d.is_turboprop_engine());

        d.i_eng_type = ENGINE_TYPE_TURBOPROP;
        assert!(!d.is_jet_engine());
        assert!(d.is_prop_engine());
        assert!(!d.is_piston_engine());
        assert!(d.is_turboprop_engine());

        // UNKNOWN: 既非 jet 也非 prop
        d.i_eng_type = ENGINE_TYPE_UNKNOWN;
        assert!(!d.is_jet_engine() && !d.is_prop_engine() && !d.is_piston_engine());
        assert!(d.is_engine_check_done(), "闸门只看 checkEngineFlag");
    }

    /// radioAltValid 的 Boolean 装箱三态 (null/false/true, Java L1989-1991)
    #[test]
    fn radio_alt_valid_null_semantics() {
        let mut d = ServiceData::default();
        d.radio_alt_valid = None;
        assert!(!d.is_radio_altitude_valid());
        d.radio_alt_valid = Some(false);
        assert!(!d.is_radio_altitude_valid());
        d.radio_alt_valid = Some(true);
        assert!(d.is_radio_altitude_valid());
    }

    /// 可变翼哨兵: -65535 → 0 且无效; 有效值直通 (Java L2121-2128/2189-2191)
    #[test]
    fn wing_sweep_sentinel() {
        let mut d = ServiceData::default();
        let mut i = Indicators::new();
        i.wsweep_indicator = F_INVALID;
        d.s_indic = Some(i);
        assert_eq!(d.get_wing_sweep(), 0.0);
        assert!(!d.is_wing_sweep_valid());

        let mut i2 = Indicators::new();
        i2.wsweep_indicator = 0.55;
        d.s_indic = Some(i2);
        assert_eq!(d.get_wing_sweep(), 0.55);
        assert!(d.is_wing_sweep_valid());
    }

    /// 助推器 (Issue #52): 哨兵归零 / 百分比封顶 / 组合判断 (Java L2153-2170)
    #[test]
    fn booster_sentinels_and_cap() {
        let mut d = ServiceData::default();
        let mut st = State::new();
        st.mfuel_1 = 200.0;
        st.mfuel0_1 = 100.0;
        d.s_state = Some(st);
        assert_eq!(d.get_booster_fuel_kg(), 200.0);
        assert_eq!(d.get_booster_fuel_percent(), 100.0, "Math.min(100, 200) 封顶");
        assert!(d.has_booster());

        // 经 struct 字段原地改 (State 非 Copy, 已移入 s_state)
        if let Some(s) = d.s_state.as_mut() {
            s.mfuel_1 = 25.0;
        }
        assert_eq!(d.get_booster_fuel_percent(), 25.0);

        // mfuel_1 哨兵 (-65535) → kg/hasBooster 归零; percent 守卫只看 mfuel0_1,
        // Java min(100, 100*(-65535)/100) = -65535.0 原样返回 (UI 端配合 hasBooster
        // 隐藏, 保真保留负值泄漏)
        if let Some(s) = d.s_state.as_mut() {
            s.mfuel_1 = -65535.0;
        }
        assert_eq!(d.get_booster_fuel_kg(), 0.0);
        assert_eq!(d.get_booster_fuel_percent(), -65535.0);
        assert!(!d.has_booster());

        // mfuel0_1 哨兵 → percent 归零, hasBooster false (mfuel_1 有效也不算)
        let mut st2 = State::new();
        st2.mfuel_1 = 100.0;
        st2.mfuel0_1 = -65535.0;
        d.s_state = Some(st2);
        assert_eq!(d.get_booster_fuel_kg(), 100.0);
        assert_eq!(d.get_booster_fuel_percent(), 0.0);
        assert!(!d.has_booster());
    }

    /// NaN 穿透语义 (§2.12 原样保持): Java 守卫 `mfuel_1 <= 0` / `mfuel0_1 <= 0` 对
    /// NaN 判 false → 穿透; Math.min(double,double) NaN 传播。现解析层
    /// (get_data_float) 只产哨兵或有效值, NaN 不可达 —— 本测试锁的是守卫极性
    /// 不被未来改回 `> 0.0` (那会把 NaN 静默归零)
    #[test]
    fn nan_passthrough_matches_java_guard_polarity() {
        let mut d = ServiceData::default();
        let mut st = State::new();
        st.mfuel_1 = f64::NAN;
        st.mfuel0_1 = f64::NAN;
        d.s_state = Some(st);
        assert!(d.get_booster_fuel_kg().is_nan(), "NaN <= 0 为 false → 穿透");
        assert!(d.get_booster_fuel_percent().is_nan());
        // hasBooster 的 `mfuel_1 > 0 && mfuel0_1 > 0` 对 NaN 判 false (Java/Rust 同)
        assert!(!d.has_booster());
        // Math.min(thurstPercent, 100.0) NaN 传播 (f64::min 会返 100.0, 已手写复刻)
        d.thurst_percent = f64::NAN;
        assert!(d.get_power_percent().is_nan());
    }

    /// FM 周期快照驱动 get_total_weight / has_wep (Java L2038-2046/2280-2283)
    #[test]
    fn fm_snapshot_drives_total_weight_and_wep() {
        let mut d = ServiceData::default();
        d.s_state = Some(mock_state()); // mfuel = 197
        // UNRESOLVED (默认): blkx=null → 0 / false
        assert_eq!(d.get_total_weight(), 0.0);
        assert!(!d.has_wep());

        // READY + blkx: nofuelweight + mfuel; nitro > 0 → hasWep
        let mut blkx = Blkx::default();
        blkx.nofuelweight = 3000.0;
        blkx.nitro = 120.0;
        d.fm = Arc::new(FMHandle::ready(Some("bf109f-4".into()), Some(blkx), 0.0, 0.0, None));
        assert_eq!(d.get_total_weight(), 3000.0 + 197.0);
        assert!(d.has_wep());

        // nitro = 0 → hasWep false (无加力系统)
        let mut blkx2 = Blkx::default();
        blkx2.nofuelweight = 3000.0;
        blkx2.nitro = 0.0;
        d.fm = Arc::new(FMHandle::ready(Some("bf109f-4".into()), Some(blkx2), 0.0, 0.0, None));
        assert!(!d.has_wep());
        assert_eq!(d.get_total_weight(), 3197.0);

        // blkx 有但 sState null → 0 (守卫的另一半)
        d.s_state = None;
        assert_eq!(d.get_total_weight(), 0.0);
    }

    /// get_power_percent 的 Math.min 封顶 (Java L2179-2181)
    #[test]
    fn power_percent_caps_at_100() {
        let mut d = ServiceData::default();
        d.thurst_percent = 150.0;
        assert_eq!(d.get_power_percent(), 100.0);
        d.thurst_percent = 42.5;
        assert_eq!(d.get_power_percent(), 42.5);
        d.thurst_percent = 0.0;
        assert_eq!(d.get_power_percent(), 0.0);
    }

    /// get_pitch 对未 init 的 State (pitch 数组空) panic — 对应 Java
    /// `sState.pitch[0]` 的 NPE (构造器窗口内不可达, 轮询线程 catch 兜底, §6)
    #[test]
    #[should_panic]
    fn get_pitch_on_uninit_state_panics_like_java_npe() {
        let mut d = ServiceData::default();
        d.s_state = Some(State::new()); // 未 init: pitch 为空 Vec (≈ Java null)
        let _ = d.get_pitch();
    }
}
