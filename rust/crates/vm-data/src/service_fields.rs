//! Service 数据快照 (Java Service 实例字段区 + 取值视图)。
//! 持有方: service_loop 的 RwLock<ServiceData> — Service 线程内部短锁读写
//! (重构波4); 跨线程读者 (渲染线程/语音/主线程) 一律走 frame.rs 的
//! FrameStore 不可变帧, 不再接触本锁。
//! 取数唯一接口 = impl FormulaView (var_value 短名; 公式值优先, 其余直抵源头)。
//! 批2 起不存格式化字符串 (显示文本由消费侧就地格式化)。
//! 波17 F14: 派生标量按语义聚合为 EngineScalars/FuelScalars/AltScalars 三组,
//! ServiceData 与 Frame 同持, 帧拷贝整组搬; 投票/EMA 等私有状态量不入组。

use std::sync::Arc;

use vm_core::base::calc_helper::SimpleMovingAverage;
use vm_core::fm::FMHandle;
use vm_core::telemetry::parser::{Indicators, MapInfo, State};

/// Java `ENGINE_TYPE_*` int 常量的枚举收敛 (波17 F1)。
/// 序列化兼容: `as_i32()` 输出与原常量数值逐一致 — Prop=0 / Jet=1 /
/// Turboprop=2 / Unknown=-1 (AnalyzerService trait 等既有 i32 面走它)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EngineType {
    Unknown,
    Jet,
    Prop,
    Turboprop,
}

impl EngineType {
    /// 原常量数值 (既有 i32 消费面的输出值不变)
    pub fn as_i32(self) -> i32 {
        match self {
            EngineType::Unknown => -1,
            EngineType::Jet => 1,
            EngineType::Prop => 0,
            EngineType::Turboprop => 2,
        }
    }

    /// Java isJetEngine: 仅喷气
    pub fn is_jet(self) -> bool {
        matches!(self, EngineType::Jet)
    }

    /// Java isPropEngine = PROP || TURBOPROP (is_piston 才是仅 PROP;
    /// 曾漏 TURBOPROP 致涡桨机 is_prop_engine 恒 false — 语义随方法收敛)
    pub fn is_prop(self) -> bool {
        matches!(self, EngineType::Prop | EngineType::Turboprop)
    }

    /// Java isPistonEngine: 仅活塞
    pub fn is_piston(self) -> bool {
        matches!(self, EngineType::Prop)
    }

    /// Java isTurbopropEngine: 仅涡桨
    pub fn is_turboprop(self) -> bool {
        matches!(self, EngineType::Turboprop)
    }
}

/// 引擎状态组 (波17 F14): Java calculate 链引擎族方法 (updateEngineState /
/// checkEngineJet / updateTemp / updateWepTime / getMaximumRPM 学习 /
/// updateOptimalCompressorStage) 的产物域。ServiceData 与 Frame 同持,
/// 帧拷贝整组搬; 状态机私有量 (投票计数/EMA 峰值/prev 档位) 不入组不入帧。
#[derive(Clone, Debug)]
pub struct EngineScalars {
    pub total_hp: i32,           // 总马力
    pub total_hp_eff: i32,       // 有效马力
    pub total_thrust: i32,       // 总推力
    pub avgeff: f64,
    /// 推力/功率百分比 (G8: 原 thurst_percent 拼写更正)
    pub thrust_percent: f64,
    pub t_eng_response: f64,
    /// 引擎类型 (F1: 原 i_eng_type i32 字段)
    pub engine_type: EngineType,
    /// 投票状态机收敛旗标 (is_jet/is_prop 族供值守卫; 未收敛时判定全 false)
    pub check_engine_flag: bool,
    pub engine_num: i32,
    pub noil_temp: f64,
    pub nwater_temp: f64,
    pub wep_time: i64,
    /// WEP 剩余时间 (秒)
    pub s_wep_time_val: i64,
    pub nitrokg: f64,
    pub nitro_consump: f64,
    pub nitro_eng_nr: i32,
    /// Optimal compressor stage index for current conditions. -1 = invalid/jet/single-stage
    /// Java 初始化器 `= -1`
    pub optimal_compressor_stage: i32,
    /// True when actual compressor stage doesn't match optimal (at full throttle)
    /// Java 初始化器 `= false`
    pub compressor_stage_mismatch: bool,
    /// C 级保留: get_maximum_rpm_learn 状态机的存储 (W-C 唯一存留的写回字段)
    pub maximum_thr_rpm: f64,
    /// Java `public boolean getMaximumRPM` (字段) — 与同名方法 getMaximumRPM(FMHandle)
    /// 构成重载; 方法归 service_loop 波次, 届时命名避让 (如 get_maximum_rpm_learn)
    pub get_maximum_rpm: bool,
}

impl Default for EngineScalars {
    /// 声明态 = Java 初始化器: 数值 0 / boolean false / optimalCompressorStage = -1。
    /// engine_type 的 Java 声明态 int 0 (数值恰为 Prop) 无任何消费者 (构造链
    /// resetvaria 即置 Unknown), 枚举化后声明态统一取 Unknown。
    fn default() -> Self {
        EngineScalars {
            total_hp: 0,
            total_hp_eff: 0,
            total_thrust: 0,
            avgeff: 0.0,
            thrust_percent: 0.0,
            t_eng_response: 0.0,
            engine_type: EngineType::Unknown,
            check_engine_flag: false,
            engine_num: 0,
            noil_temp: 0.0,
            nwater_temp: 0.0,
            wep_time: 0,
            s_wep_time_val: 0,
            nitrokg: 0.0,
            nitro_consump: 0.0,
            nitro_eng_nr: 0,
            optimal_compressor_stage: -1,
            compressor_stage_mismatch: false,
            maximum_thr_rpm: 0.0,
            get_maximum_rpm: false,
        }
    }
}

/// 燃油组 (波17 F14): updateFuel / slowcalculate 的产物域。
/// 差分输入 totalFuelPrev 与变化率状态量不入组 (Service 私有)。
#[derive(Clone, Debug, Default)]
pub struct FuelScalars {
    pub total_fuel: f64,      // 总油量
    pub low_acc_fuel: bool,   // 低精度燃油警告
    pub fuel_percent: i32,
    /// 剩余油量时间 (fuelTimeSMA 平滑产物)
    pub fueltime: i64,
}

/// 高度组 (波17 F14): updateAlt 写回族 — 气压高度链 (alt/altp/altmeter 族,
/// 含英制检测计数 check_alt) + 无线电高度链 (radio_alt 族)。
#[derive(Clone, Debug, Default)]
pub struct AltScalars {
    pub alt: f64,
    pub altp: f64,
    pub altmeter: f64,
    pub altmeterp: f64,
    /// 英制检测状态机积累计数 (语义见 [`ServiceData::is_imperial`])
    pub check_alt: i32,
    pub radio_alt: f64,
    pub p_radio_alt: f64,
    pub d_radio_alt: f64,
}

/// 字段全集。可见性: Java public → pub, 包私有 → pub(crate) (兄弟模块写入需要)。
/// `fm` 为周期 FM 句柄快照 (R1 纪律: 单周期内全部 FM 派生量来自同一 Blkx 实例,
/// 换机时由 loader 线程原子替换, 本周期取旧句柄平滑过渡)。
pub struct ServiceData {
    // fuelTimeSMA: resetvaria 构造 (窗口 4), slowcalculate 滚动
    // PORT(状态双主边界): Deriver (data/derive.rs) 自持 calc/turnrds/diff/sep 四个
    // SMA 及 an/turn_rds/speedv 族同源状态, 而 FlightValues 不携带 SMA 态 ——
    // service_loop 波次必须裁决唯一状态主人 (resetvaria 重建 SMA 的
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

    // === 派生标量组 (波17 F14; 组定义与分组语义见各 struct doc) ===
    pub engine: EngineScalars,
    pub fuel: FuelScalars,
    pub altm: AltScalars,

    // === 杂项平铺 (无成组语义; 私有状态量不入帧) ===
    /// 上次油量（加油检测差分输入, slowcalculate 追赶写点）
    pub total_fuel_prev: f64,
    /// 英制检测状态机锁死旗标 (|check_alt| > 10000 后停计, resetvaria 复位)
    pub not_check_inch: bool,
    pub actual_interval_ms: i64,
    pub current_time_ms: i64,
    pub poll_cycle_duration_ms: i64,
        pub(crate) last_main_loop_time_ms: i64,
        pub(crate) last_map_poll_time_ms: i64,
    pub fuel_change: f64,
        pub(crate) fuel_lastchange_mili: i64,
        pub(crate) fuelchange_time: i64,
    // (start_time 已归 FrameStore 原子 — Controller openpad 写, 波4)
    pub elapsed_time: i64,
    pub compass_delta: f64,
    pub cur_load_min_work_time: f64,
    pub ratio: f64,
    pub ratio_1: f64,
    /// Java `private double nVy` — 私有但 getVario() 读
        pub(crate) n_vy: f64,
    /// checkEngineJet 投票状态机: 磁电机正负票计数 (收敛后冻结)
        pub(crate) check_engine_type: i32,
    /// checkEngineJet 投票状态机: 桨距有效性正负票计数
        pub(crate) check_pitch: i32,
    /// Java 包私有 `Boolean portOcupied = false` (装箱 → Option, 初始化器 false)
        pub(crate) port_ocupied: bool,
    // EMA 峰值缓存 (updateEngineState 的 thrust_percent 回退分母)
        pub(crate) max_total_thr: i32,
        pub(crate) max_total_hp: i32,
    /// thrust_percent 的上轮值 (t_eng_response 差分输入)
        pub(crate) p_thrust_percent: f64,
    /// getMaximumRPM 学习状态机计数
        pub(crate) check_maxium_rpm: i64,
    /// Previous actual compressor stage for change detection (0-based, -1 = invalid)
    /// Java 初始化器 `= -1`
        pub(crate) prev_actual_compressor_stage: i32,
    /// Previous optimal compressor stage for change detection
    /// Java 初始化器 `= -1`
        pub(crate) prev_optimal_compressor_stage: i32,
    pub mapinfo: Option<MapInfo>,

    pub player_live: bool,

    // PORT(Java `public Controller c` 不迁移): 环 1 (Controller↔Service) 按 LIFETIMES
    // §4.1 裁决断裂 —— ServiceData 是纯数据快照, 配置读走 Arc<ConfigStore>,
    // 生命周期协作走 service_loop; 保留反向引用 = 重建所有权环, 审查必拒。
    // PORT(Java `private final FocusMonitor focusMonitor` 不迁移): 焦点监控器是
    // 轮询驱动的组件 (tick 由 run() 调), 归 service_loop 线程持有, 非数据快照成员。

    // (fatal_warn 已归 FrameStore 原子 — VoiceWarning set_fatal_warn 写, 波4)

    /// R1 周期 FM 句柄快照 (无 Java 对应字段, 见 struct 级 PORT 注):
    /// service_loop 每周期 `FMManager.current()` 写入; getter 经它读
    /// blkx.nofuelweight (getTotalWeight) / blkx.nitro (hasWep)。
    /// 初始值 = `FMHandle.UNRESOLVED` (对齐 FMManager.current 的 volatile 初值)。
    pub fm: Arc<FMHandle>,

    /// 公式系统一帧求值结果 (公式名→槽号的定位见 CompiledFormulaSet.slots;
    /// 无 Java 对应, 公式系统设计 doc/formula_system_design.md §2 裁决 A1/A2:
    /// Service 线程单点求值, 渲染线程经本 RwLock 只读)。
    pub formula_values: vm_core::formula::FormulaResults,
    /// 公式名→结果槽 (formula_step 与 values 同步写; overlay 绑定解析用)
    pub formula_slots: std::sync::Arc<std::collections::HashMap<String, u16>>,
    /// L2 规则本帧触发事件 (formula_step 产出; 消费面 vm-app toast/语音链)
    pub rule_triggers: Vec<vm_core::formula::rules::RuleTriggered>,
}

/// 无效占位串 "-" (原 nastring 常量)。
/// PORT: indicators.rs 已按 CLASSIFY 裁决内联为私有 NA_STRING (不越文件改, §6);
/// 本处为规范定义, 后续波次统一收敛引用点。
pub const NASTRING: &str = "-";

impl Default for ServiceData {
    /// 对应 Java 字段声明默认值: 隐式初始化 (数值 0 / boolean false / 引用 null, §2.10)
    /// + 显式初始化器 (pressureUnitStr="Ata" / fatalWarn=false /
    ///   optimalCompressorStage 族 -1 / portOcupied=false / checkEngineFlag=false)。
    ///
    /// **service_loop 波次验收义务**: Default 是"声明态"而非"构造后态"。Java 构造器
    /// 构造器无条件跑 clearvaria→resetvaria, 真实初值还包括:
    /// `freq=serviceLoopIntervalMs` / `ratio=freq/1000f` / `ratio_1=1f-ratio` /
    /// `sState`/`sIndic` = new+init / `mapinfo` = new / `power`·`pitch`·`thrust`·
    /// `efficiency` = vec![None; State::MAX_ENG_NUM] / `FuelCheckMili`·
    /// `lastMapPollTimeMs`·`lastMainLoopTimeMs` = 构造时刻; resetvaria
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
            engine: EngineScalars::default(),
            fuel: FuelScalars::default(),
            altm: AltScalars::default(),
            total_fuel_prev: 0.0,
            not_check_inch: false,
            actual_interval_ms: 0,
            current_time_ms: 0,
            poll_cycle_duration_ms: 0,
            last_main_loop_time_ms: 0,
            last_map_poll_time_ms: 0,
            fuel_change: 0.0,
            fuel_lastchange_mili: 0,
            fuelchange_time: 0,
            elapsed_time: 0,
            compass_delta: 0.0,
            cur_load_min_work_time: 0.0,
            ratio: 0.0,
            ratio_1: 0.0,
            n_vy: 0.0,
            check_engine_type: 0,
            check_pitch: 0,
            port_ocupied: false,
            max_total_thr: 0,
            max_total_hp: 0,
            p_thrust_percent: 0.0,
            check_maxium_rpm: 0,
            prev_actual_compressor_stage: -1,
            prev_optimal_compressor_stage: -1,
            mapinfo: None,
            player_live: false,
            fm: Arc::new(FMHandle::UNRESOLVED),
            formula_values: Default::default(),
            formula_slots: std::sync::Arc::default(),
            rule_triggers: Vec::new(),
        }
    }
}

impl ServiceData {
    // (isPlayerLive() 单行委托已内联: 唯一调用点直读 pub player_live 字段)

    /// 英制单位判定 (波17 F2: 原散点 `checkAlt > 0` 的统一面)。
    /// check_alt 是 updateAlt 英制检测状态机的积累计数 (Java "人类毒瘤英制飞机" 段):
    /// altmeter 跳变与 2·Vy·interval 的失配按轮 ±actualIntervalMs 计入,
    /// |check_alt| > 10000 后 notCheckInch 置位停计 —— 符号即结论: >0 英制, ≤0 公制。
    pub fn is_imperial(&self) -> bool {
        self.altm.check_alt > 0
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
