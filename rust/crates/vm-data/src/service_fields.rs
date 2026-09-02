//! Service 数据快照 (Java Service 实例字段区 + 取值视图)。
//! 持有方: service_loop 的 RwLock<ServiceData> — Service 线程内部短锁读写
//! (重构波4); 跨线程读者 (win32 渲染/语音/主线程) 一律走 frame.rs 的
//! FrameStore 不可变帧, 不再接触本锁。
//! 取数唯一接口 = impl FormulaView (var_value 短名; 公式值优先, 其余直抵源头)。
//! 批2 起不存格式化字符串 (显示文本由消费侧就地格式化)。

use std::sync::Arc;

use vm_core::base::calc_helper::SimpleMovingAverage;
use vm_core::fm::FMHandle;
use vm_core::telemetry::parser::{Indicators, MapInfo, State};

/// 字段全集。可见性: Java public → pub, 包私有 → pub(crate) (兄弟模块写入需要)。
/// `fm` 为周期 FM 句柄快照 (R1 纪律: 单周期内全部 FM 派生量来自同一 Blkx 实例,
/// 换机时由 loader 线程原子替换, 本周期取旧句柄平滑过渡)。
pub struct ServiceData {
    // fuelTimeSMA: resetvaria 构造 (窗口 4), slowcalculate 滚动
    // PORT(状态双主边界): Deriver (data/derive.rs) 自持 calc/turnrds/diff/sep 四个
    // SMA 及 an/turn_rds/speedv 族同源状态, 而 FlightValues 不携带 SMA 态 ——
    // service_loop 波次必须裁决唯一状态主人 (Java resetvaria L1591-1597 重建 SMA 的
    // 语义打在主人侧), 防 ServiceData 侧双胞胎永远 None / 两份真相互相漂移;
    pub fuel_time_sma: Option<SimpleMovingAverage>,
    // public static URL urlstate;
    // public static URL urlindicators;
    /// Java `public double loc[]` — null 直到 resetvaria 赋 `new double[2]`;
    /// MapObj.getPlayerLoc 按 &mut [f64;2] 写入 → 定长 [f64; 2] (§1)
    pub loc: Option<[f64; 2]>,
    pub dir: Option<[f64; 2]>,
    pub calc_period: i64,
    // Gravitational constant imported from PhysicsConstants.g
    pub freq: i64,

    // === API 对象（对应 War Thunder HTTP 端点）===
    pub s_state: Option<State>,     // /state 端点数据
    pub s_indic: Option<Indicators>, // /indicators 端点数据

    // === 数值类字段（移除匈牙利前缀）===
    pub total_hp: i32,              // 总马力
    pub total_hp_eff: i32,          // 有效马力
    pub total_thrust: i32,          // 总推力
    pub total_fuel: f64,            // 总油量
    pub total_fuel_prev: f64,       // 上次油量（用于计算变化）
    pub low_acc_fuel: bool,         // 低精度燃油警告
    pub check_alt: i32,             // 检查高度
    pub fueltime: i64,
    pub not_check_inch: bool,
    // public boolean isFuelpressure;
    pub actual_interval_ms: i64,
    pub alt: f64,
    pub altp: f64,
    pub current_time_ms: i64,
    pub poll_cycle_duration_ms: i64,
        pub(crate) last_main_loop_time_ms: i64,
        pub(crate) last_map_poll_time_ms: i64,
        pub fuel_change: f64,
        pub(crate) fuel_lastchange_mili: i64,
        pub(crate) fuelchange_time: i64,
    // (start_time 已归 FrameStore 原子 — Controller openpad 写, 波4)
    pub elapsed_time: i64,

    pub noil_temp: f64,
    pub nwater_temp: f64,
    // public int enginenum;
    // public int enginetype;

    pub wep_time: i64,



    // PORT(Java `public Controller c` 不迁移): 环 1 (Controller↔Service) 按 LIFETIMES
    // §4.1 裁决断裂 —— ServiceData 是纯数据快照, 配置读走 Arc<ConfigStore>,
    // 生命周期协作走 service_loop; 保留反向引用 = 重建所有权环, 审查必拒。
    // PORT(Java `private final FocusMonitor focusMonitor` 不迁移): 焦点监控器是
    // 轮询驱动的组件 (tick 由 run() 调), 归 service_loop 线程持有, 非数据快照成员。

    // (fatal_warn 已归 FrameStore 原子 — VoiceWarning set_fatal_warn 写, 波4)

    // sState转换后
    pub compass_delta: f64,
    pub engine_num: i32,
    pub cur_load_min_work_time: f64,

    pub ratio: f64,
    pub ratio_1: f64,
    // iIndic
    /// Java `private double nVy` — 私有但 getVario() 读
    pub(crate) n_vy: f64,
    pub radio_alt: f64,
    pub p_radio_alt: f64,
    pub d_radio_alt: f64,
    pub i_eng_type: i32,
    pub nitrokg: f64,
    pub nitro_consump: f64,
    pub nitro_eng_nr: i32,
    pub s_wep_time_val: i64, // Remaining WEP time in seconds

    /// Optimal compressor stage index for current conditions. -1 = invalid/jet/single-stage
    /// Java 初始化器 `= -1`
        pub(crate) optimal_compressor_stage: i32,
    /// True when actual compressor stage doesn't match optimal (at full throttle)
    /// Java 初始化器 `= false`
        pub(crate) compressor_stage_mismatch: bool,
    /// Previous actual compressor stage for change detection (0-based, -1 = invalid)
    /// Java 初始化器 `= -1`
        pub(crate) prev_actual_compressor_stage: i32,
    /// Previous optimal compressor stage for change detection
    /// Java 初始化器 `= -1`
        pub(crate) prev_optimal_compressor_stage: i32,

    /// Java 包私有 `Boolean portOcupied = false` (装箱 → Option, 初始化器 false)
        pub(crate) port_ocupied: bool,
        pub(crate) check_engine_type: i32,
        pub(crate) check_pitch: i32,
    /// Java 初始化器 `= false`
    pub check_engine_flag: bool,
    // ENGINE_TYPE_* 常量集中声明于 struct 后 (§1 static final → const)
    pub mapinfo: Option<MapInfo>,

    // ---- L219-236 (方法/常量区, 常量见 struct 后) ----
    pub player_live: bool,

    pub altmeterp: f64,
    pub altmeter: f64,
    pub thurst_percent: f64,
        pub(crate) max_total_thr: i32,
    pub fuel_percent: i32,
    pub avgeff: f64,
        pub(crate) max_total_hp: i32,
            pub(crate) p_thurst_percent: f64,
    pub t_eng_response: f64,
    /// C 级保留: get_maximum_rpm_learn 状态机的存储 (W-C 唯一存留的写回字段)
        pub maximum_thr_rpm: f64,
    // double maximumAllowedRPM;
        pub(crate) check_maxium_rpm: i64,
    /// Java `public boolean getMaximumRPM` (字段) — 与同名方法 getMaximumRPM(FMHandle)
    /// 构成重载; 方法归 service_loop 波次, 届时命名避让 (如 get_maximum_rpm_learn)
    pub get_maximum_rpm: bool,
    // PORT(Java `public HttpHelper httpClient` 不迁移): IO 机械 (socket + 响应缓冲),
    // 归 service_loop 线程持有, 非数据快照成员。

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
            fuel_time_sma: None,
            loc: None,
            dir: None,
            calc_period: 0,
            freq: 0,
            s_state: None,
            s_indic: None,
            total_hp: 0,
            total_hp_eff: 0,
            total_thrust: 0,
            total_fuel: 0.0,
            total_fuel_prev: 0.0,
            low_acc_fuel: false,
            check_alt: 0,
            fueltime: 0,
            not_check_inch: false,
            actual_interval_ms: 0,
            alt: 0.0,
            altp: 0.0,
            current_time_ms: 0,
            poll_cycle_duration_ms: 0,
            last_main_loop_time_ms: 0,
            last_map_poll_time_ms: 0,
            fuel_change: 0.0,
            fuel_lastchange_mili: 0,
            fuelchange_time: 0,
            elapsed_time: 0,
            noil_temp: 0.0,
            nwater_temp: 0.0,
            wep_time: 0,
            compass_delta: 0.0,
            engine_num: 0,
            cur_load_min_work_time: 0.0,
            ratio: 0.0,
            ratio_1: 0.0,
            n_vy: 0.0,
            radio_alt: 0.0,
            p_radio_alt: 0.0,
            d_radio_alt: 0.0,
            i_eng_type: 0,
            nitrokg: 0.0,
            nitro_consump: 0.0,
            nitro_eng_nr: 0,
            s_wep_time_val: 0,
            optimal_compressor_stage: -1,
            compressor_stage_mismatch: false,
            prev_actual_compressor_stage: -1,
            prev_optimal_compressor_stage: -1,
            port_ocupied: false,
            check_engine_type: 0,
            check_pitch: 0,
            check_engine_flag: false,
            mapinfo: None,
            player_live: false,
            altmeterp: 0.0,
            altmeter: 0.0,
            thurst_percent: 0.0,
            max_total_thr: 0,
            fuel_percent: 0,
            avgeff: 0.0,
            max_total_hp: 0,
            p_thurst_percent: 0.0,
            t_eng_response: 0.0,
            maximum_thr_rpm: 0.0,
            check_maxium_rpm: 0,
            get_maximum_rpm: false,
            fm: Arc::new(FMHandle::UNRESOLVED),
            formula_values: Default::default(),
            formula_slots: std::sync::Arc::default(),
            rule_triggers: Vec::new(),
        }
    }
}

impl ServiceData {
    /// Java `public boolean isPlayerLive()` (L232-234) — processPollingCycle 的存活闸门。
    pub fn is_player_live(&self) -> bool {
        self.player_live
    }
}

// --- FormulaView: 唯一取数接口 (批1: TelemetrySource 71 getter 层已删) ---

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
            VarSrc::Blk(f) => self.fm.fmdata.as_ref().map(f)?,
            VarSrc::Session(f) => f(&crate::service_loop::session_inputs(self)),
            VarSrc::Const(c) => *c,
            VarSrc::Meta(m) => match m {
                vm_core::formula::registry::MetaVar::IntervalMs => self.actual_interval_ms.max(1) as f64,
                vm_core::formula::registry::MetaVar::Freq => self.freq as f64,
                vm_core::formula::registry::MetaVar::FmLoaded => (self.fm.fmdata.is_some()) as u8 as f64,
                _ => 0.0,
            },
        };
        if v.is_nan() { None } else { Some(v) }
    }

    fn get_formula_value(&self, name: &str) -> Option<f64> {
        let slot = self.formula_slots.get(name)?;
        let v = self.formula_values.get(*slot);
        if v.is_nan() { None } else { Some(v) }
    }
}

// =====================================================================
// Tests — 公共项边界测试 (§5.2 B 类单测; 断言值 = Java 语义逐行推导,
// mock 快照与 state.rs/indicators.rs 的 Java 8 oracle 数据同源)
// =====================================================================
#[cfg(test)]
mod tests;
