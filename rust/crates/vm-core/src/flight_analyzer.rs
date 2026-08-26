//! 对应 Java: `src/parser/FlightAnalyzer.java` (一比一翻译)
//! 派生度量分析器: 爬升分段统计 (各高度级时间/功率/推力/有效功率/SEP 平均)
//! + EM 图记录 (速度分段的滚转率 / 过载 / SEP 损失)。
//!
//! PORT (D6 依赖倒置): Java 持有 `Service xs` 共享引用, 而 Service 链落 vm-data
//! (vm-data → vm-core 单向依赖, 本 crate 反向不可引) —— 此处以 trait
//! [`AnalyzerService`] 暴露 init/analyze 实际读取的 7 个 Service 字段
//! (sIndic.type / iEngType / elapsedTime / totalHp / totalThrust / totalHpEff / SEP),
//! vm-data 的 service_fields 落地时 `impl AnalyzerService for Service` 接线
//! (Java 侧为 public 字段直读, 收敛为 getter 是 crate 边界所需, 非语义变更)。
//!
//! PORT (FlightLog 接线合同, 集成期): Java `analyze()` 每次调用**活读**
//! xs.elapsedTime/totalHp/totalThrust/totalHpEff/SEP (FlightAnalyzer.java:55-59,65-66),
//! 故 retained `Arc<dyn AnalyzerService>` 是唯一正确建模 —— `FlightLog` 集成时
//! 应弃其 `FlightAnalyzerApi` 快照合同、直接持有本具体类型 (pub 字段面 + 包私有
//! init/analyze 的 pub(crate) 同 crate 可见), 并在构造面携带
//! `Arc<dyn AnalyzerService>` (flight_log::FlightLogSnapshot 已含全部 7 个字段,
//! 可由 vm-data 构造); 按快照合同适配会使 analyze 冻结在 init 时刻的值 (time[]
//! 记录错误时刻、eff/sep 累加失真)。notify 两侧类型一致
//! (`Arc<dyn Fn(&str) + Send + Sync>` = flight_log::NotifySink), 可共用同一 sink。
//!
//! PORT (CLASSIFY 裁决"注入回调"): `ui.util.NotificationService.show(String)` 是
//! C 类 UI 静态入口 —— 本译以 [`FlightAnalyzer::notify`] 字段注入, 未接线 (None)
//! 时通知丢弃, P4 NotificationService 落地后由调用方 (FlightLog/Controller) 接上。
//! PORT: `Application.debugPrint(t)` = `Logger.info("Legacy", t)` (Application.java:213)。

use std::sync::Arc;

use crate::config_api::config_provider::ConfigProvider;
use crate::lang::lang::Lang;
use crate::logger;
use crate::physics_constants::g;

/// FlightAnalyzer 对 Service 的读取面 (PORT: D6 依赖倒置, 见模块头说明)。
/// 方法名 = Java 字段名 (§0 规则 7 的 crate 边界变体)。
/// `s_indic_type` 的 `Option` 对应 Java `sIndic.type` 可为 null (键缺失);
/// `elapsed_time` 对应 Java `long elapsedTime` (毫秒)。
///
/// impl 侧两条硬性义务 (接线合同, vm-data service_fields 落地时遵守):
/// 1. Java `xs.sIndic` 为 null 时 `init` 首行 `xs.sIndic.type` 直接 NPE — trait 的
///    Option 只表达 "type 键缺失", **不得**用它吞掉 "sIndic 引用缺失": impl 须在
///    sIndic 缺失时 panic (对齐 NPE) 或论证 Service 轮询链保证 sIndic 恒已初始化;
/// 2. getter 逐字段调用对应 Java 逐字段读取时刻 (init 7 次 / analyze 5 次, 不成组),
///    impl 按 getter 粒度读锁/快照, 禁止假设 7 个 getter 恒在一次成组调用里。
pub trait AnalyzerService: Send + Sync {
    /// Java: `xs.sIndic.type` (sIndic 引用缺失时 impl 须 panic, 见 trait 级义务 1)
    fn s_indic_type(&self) -> Option<String>;
    /// Java: `xs.iEngType`
    fn i_eng_type(&self) -> i32;
    /// Java: `xs.elapsedTime`
    fn elapsed_time(&self) -> i64;
    /// Java: `xs.totalHp`
    fn total_hp(&self) -> i32;
    /// Java: `xs.totalThrust`
    fn total_thrust(&self) -> i32;
    /// Java: `xs.totalHpEff`
    fn total_hp_eff(&self) -> i32;
    /// Java: `xs.SEP`
    fn sep(&self) -> f64;
}

/// Java: `public static final int maxAltStage = 256;`
/// (Java 声明位于 type 与 time 字段之间, const 语法上移出 struct)
pub const MAX_ALT_STAGE: i32 = 256;

/// Java: `public static final int maxIASStage = 256;` (获得速度区间, 0 表示非法, 0 - 2560km/h)
pub const MAX_IAS_STAGE: i32 = 256;

pub struct FlightAnalyzer {
    pub engine_type: i32,
    /// Java: `public String type` (Rust 关键字, 原始名 `type`; null → None)
    pub r#type: Option<String>,
    pub time: Vec<f64>,  // 从第零层开始
    pub power: Vec<i32>, // 从第一层开始
    pub thrust: Vec<i32>,
    pub eff: Vec<i32>,
    pub sep: Vec<f64>,
    pub initalt_stage: i32,
    pub curalt_stage: i32,
    /// Java: 包私有 `boolean isInformation` → 同 crate 可见 (FlightLog 同包兄弟模块)
    pub(crate) is_information: bool,

    /// Java: `Service xs` — 未 init 即 analyze 在 Java 是 NullPointerException,
    /// 此处 Option + expect panic 对应 (§1: 非受检异常 → panic!)。
    xs: Option<Arc<dyn AnalyzerService + Send + Sync>>,
    /// 计数: 10Hz 轮询下 i32 溢出需 ~6.8 年停留在同一高度级, 域内不可达
    count: i32,
    config: Option<Arc<dyn ConfigProvider + Send + Sync>>,

    // ---- 第二字段区 (Java 声明于 maxIASStage 之后) ----
    pub roll_rate: Vec<i32>,
    pub roll_alr: Vec<i32>,

    pub turn_load: Vec<f64>,
    pub turn_elev: Vec<i32>,

    pub sep_loss: Vec<f64>,

    /// PORT: `ui.util.NotificationService.show` 的注入回调 (C 类, 见模块头)。
    // PORT: Java 保真 — NotificationService.show 引用形态, 不拆 type 别名
    #[allow(clippy::type_complexity)]
    pub notify: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

// PORT: Java `new FlightAnalyzer()` — 数值字段 0 / boolean false / 引用 null,
// 五个 maxAltStage 数组立即按 256 长度零填充 (§2.10 隐式初始化显式化)。
impl Default for FlightAnalyzer {
    fn default() -> Self {
        FlightAnalyzer {
            engine_type: 0,
            r#type: None,
            time: vec![0.0; MAX_ALT_STAGE as usize],  // 从第零层开始
            power: vec![0; MAX_ALT_STAGE as usize],   // 从第一层开始
            thrust: vec![0; MAX_ALT_STAGE as usize],
            eff: vec![0; MAX_ALT_STAGE as usize],
            sep: vec![0.0; MAX_ALT_STAGE as usize],
            initalt_stage: 0,
            curalt_stage: 0,
            is_information: false,
            xs: None,
            count: 0,
            config: None,
            roll_rate: vec![0; MAX_IAS_STAGE as usize],
            roll_alr: vec![0; MAX_IAS_STAGE as usize],
            turn_load: vec![0.0; MAX_IAS_STAGE as usize],
            turn_elev: vec![0; MAX_IAS_STAGE as usize],
            sep_loss: vec![0.0; MAX_IAS_STAGE as usize],
            notify: None,
        }
    }
}

impl FlightAnalyzer {
    /// Java `xs` 字段访问: init 前为 null (Java NPE ↔ Rust expect panic)。
    fn xs(&self) -> &Arc<dyn AnalyzerService + Send + Sync> {
        self.xs.as_ref().expect("FlightAnalyzer 未 init 即访问 xs (Java NullPointerException)")
    }

    /// Java: `void init(int stage, Service st, prog.config.ConfigProvider config)` (包私有)。
    // 唯一调用者 flight_log (parser 同包) 集成期才接线到本具体类型 (见模块头接线合同),
    // 接线前非测试构建无调用者 — allow 随接线移除
    #[allow(dead_code)]
    pub(crate) fn init(
        &mut self,
        stage: i32,
        st: Arc<dyn AnalyzerService + Send + Sync>,
        config: Option<Arc<dyn ConfigProvider + Send + Sync>>,
    ) {
        // Application.debugPrint("analyzer初始化了");
        self.xs = Some(st);
        self.config = config;
        self.count = 1;

        // Java: String enableAltInfo = config != null ? config.getConfig("enableAltInformation") : "false";
        let enable_alt_info = match &self.config {
            Some(c) => c.get_config("enableAltInformation"),
            None => Some("false".to_string()),
        };
        // Java: Boolean.parseBoolean — null/非 "true" 一律 false (不抛 NPE)
        self.is_information = java_parse_boolean(enable_alt_info.as_deref().unwrap_or(""));

        let xs = self.xs().clone();
        self.r#type = xs.s_indic_type();
        self.engine_type = xs.i_eng_type();
        self.initalt_stage = stage;
        self.curalt_stage = self.initalt_stage;
        let idx = self.curalt_stage as usize;
        // Java: time[..] = (xs.elapsedTime / 1000f) — long/float 先转 float 再除 (§2.12), 存入 double[] 再加宽
        self.time[idx] = (xs.elapsed_time() as f32 / 1000.0f32) as f64;
        self.power[idx] = xs.total_hp();
        self.thrust[idx] = xs.total_thrust();
        self.eff[idx] = xs.total_hp_eff();
        self.sep[idx] = xs.sep();
        // Application.debugPrint("已经记录stage"+curaltStage+"时间戳"+time[curaltStage]+"功率"+power[curaltStage]+"实功率"+eff[curaltStage]+"SEP"+sep[curaltStage]);
    }

    /// Java: `void analyze(int stage)` (包私有)。
    // 同 init: flight_log 集成期接线, 接线前非测试构建无调用者
    #[allow(dead_code)]
    pub(crate) fn analyze(&mut self, stage: i32) {
        let xs = self.xs().clone(); // Arc 浅拷贝, 避免与 &mut self 借用冲突
        self.engine_type = xs.i_eng_type();
        if stage == self.curalt_stage + 1 {
            let idx = self.curalt_stage as usize;
            self.eff[idx] /= self.count; // Java int/int 截断除 (§2.4)
            self.sep[idx] /= self.count as f64 * g; // count * g: int 提升为 double
            // Application.debugPrint("已经记录stage"+curaltStage+"时间戳"+time[curaltStage]+"功率"+power[curaltStage]+"推力"+thrust[curaltStage]+"实功率"+eff[curaltStage]+"SEP"+sep[curaltStage]);
            self.curalt_stage += 1;

            let idx = self.curalt_stage as usize;
            self.time[idx] = (xs.elapsed_time() as f32 / 1000.0f32) as f64;
            self.power[idx] = xs.total_hp();
            self.thrust[idx] = xs.total_thrust();
            self.eff[idx] = xs.total_hp_eff();
            self.sep[idx] = xs.sep();
            self.count = 1;
            if self.is_information {
                let lang = Lang::init_lang();
                // Java: Lang.fA1 + stage * 100 + Lang.fA2 + (int) time[..] + Lang.fA3
                //       + (int) ((stage - initaltStage) * 1000 / time[..]) / 10.0f + Lang.fA4
                // — (int) X / 10.0f 是 int 除 float 得 float, 字符串拼接走 Float.toString
                let climb = ((stage - self.initalt_stage) * 1000) as f64 / self.time[idx];
                let msg = format!(
                    "{}{}{}{}{}{}{}",
                    lang.f_a1,
                    stage * 100,
                    lang.f_a2,
                    self.time[idx] as i32, // Java (int) double: NaN→0/饱和, Rust as i32 同语义
                    lang.f_a3,
                    java_float_to_string(climb as i32 as f32 / 10.0f32),
                    lang.f_a4
                );
                self.show(&msg);
            }
        } else {
            let idx = self.curalt_stage as usize;
            self.eff[idx] += xs.total_hp_eff();
            self.sep[idx] += xs.sep();
            self.count += 1;
        }
    }

    // 获得速度阶段
    pub fn get_speed_stage(&self, ias: f64) -> i32 {
        // Java: (int) Math.round(ias / 10.0f) — 10.0f 按二元数值提升转 double (精确值 10.0)
        // (int)(long) 双转: Java long→int 截断低 32 位 (§2.2); Rust as i32 饱和, 仅巨值域分歧
        (java_math_round(ias / 10.0) as u32) as i32
    }

    // 使用舵面辅助判断
    // Java 为 public (FlightAnalyzer.java:89), 一比一 pub — vm-data 经 FlightLog 之外直调时可用
    pub fn update_em_chart(
        &mut self,
        ias: f64,
        g_load: f64,
        wx: i32,
        sep: f64,
        abs_elev: i32,
        abs_alr: i32,
    ) {
        let stage = self.get_speed_stage(ias);
        if (0..MAX_IAS_STAGE).contains(&stage) {
            let s = stage as usize;
            // 如果当前roll_rate比记录值高则更新
            // 合法roll_rate校验问题，检查两边线性插值，或是多数据叠加才能生效?
            // 如果当前舵面值大于等于则记录
            if abs_alr > 5 && wx > 10 && abs_alr >= self.roll_alr[s] && wx > self.roll_rate[s] {
                self.roll_alr[s] = abs_alr;

                // Java: wx - roll_rate[..] 用的是尚未更新的旧值 (赋值在其后), 顺序保真
                if self.is_information && (wx - self.roll_rate[s] > 40) {
                    let lang = Lang::init_lang();
                    self.show(&format!(
                        "{}{}{}{}{}",
                        lang.f_a_roll1,
                        stage * 10,
                        lang.f_a_roll2,
                        wx,
                        lang.f_a_roll3
                    ));
                }

                self.roll_rate[s] = wx;
            }

            // Java: g_load > 1.0f / 3.0f 字面量按 double 比较 (float 字面量精确提升)
            if g_load > 1.0 && sep < 5.0 && abs_elev >= self.turn_elev[s] {
                // if (g_load > turn_load[stage] ) {
                self.turn_elev[s] = abs_elev;
                if self.is_information && (g_load - self.turn_load[s] > 3.0) {
                    let lang = Lang::init_lang();
                    // Java: String.format("%.1f", ...) ×2 — HALF_UP on 最短往返十进制 (见 java_format_f1)
                    self.show(&format!(
                        "{}{}{}{}{}{}{}",
                        lang.f_a_turn1,
                        stage * 10,
                        lang.f_a_turn2,
                        java_format_f1((self.turn_load[s] + g_load) / 2.0),
                        lang.f_a_turn3,
                        java_format_f1((self.sep_loss[s] + sep) / 2.0),
                        lang.f_a_turn4
                    ));
                }
                self.turn_load[s] = (self.turn_load[s] + g_load) / 2.0;
                self.sep_loss[s] = (self.sep_loss[s] + sep) / 2.0;
                // }
                // showAllEMChart();
            }
        }
    }

    /// Java: `public int getNoZerosNum(int[] arr)` (重载 → _i32 后缀)。
    pub fn get_no_zeros_num_i32(&self, arr: &[i32]) -> i32 {
        let mut ret = 0;
        let mut i = 0usize;
        while i < arr.len() {
            if arr[i] != 0 {
                ret += 1;
            }
            i += 1;
        }
        ret
    }

    /// Java: `public int getNoZerosNum(double[] arr)` (重载 → _f64 后缀)。
    pub fn get_no_zeros_num_f64(&self, arr: &[f64]) -> i32 {
        let mut ret = 0;
        let mut i = 0usize;
        while i < arr.len() {
            if arr[i] != 0.0 {
                ret += 1;
            }
            i += 1;
        }
        ret
    }

    /// Java: `public void removeZeroes(double[] x, double[] y, int[] oy)` (重载 → _i32 后缀)。
    pub fn remove_zeroes_i32(&self, x: &mut [f64], y: &mut [f64], oy: &[i32]) {
        let mut j = 0;
        let mut i = 0usize;
        while i < oy.len() {
            if oy[i] != 0 {
                x[j] = i as f64 * 10.0;
                // PORT: Java 循环自 i=0 起 — oy[0]!=0 时 oy[i-1] 即 ArrayIndexOutOfBoundsException;
                // Rust usize 下溢 (debug) / 回绕后越界 (release) 在同条件 panic, 行为对应
                y[j] = (oy[i - 1] + oy[i] + oy[i + 1]) as f64 / 3.0;
                j += 1;
            }
            i += 1;
        }
    }

    /// Java: `public void removeZeroes(double[] x, double[] y, double[] oy)` (重载 → _f64 后缀)。
    pub fn remove_zeroes_f64(&self, x: &mut [f64], y: &mut [f64], oy: &[f64]) {
        let mut j = 0;
        // PORT: Java for(i=1; i<oy.length-1; i++) ↔ i+1 < len (len=0 时 Java 判 1<-1 不进循环, 同)
        let mut i = 1usize;
        while i + 1 < oy.len() {
            if oy[i] != 0.0 {
                x[j] = i as f64 * 10.0;
                y[j] = (oy[i - 1] + oy[i] + oy[i + 1]) / 3.0;
                j += 1;
            }
            i += 1;
        }
    }

    pub fn remove_roll_rates_zeroes(&self, ias: &mut [f64], wx: &mut [f64]) {
        // int j = 0;
        self.remove_zeroes_i32(ias, wx, &self.roll_rate);
    }

    pub fn remove_load_zeroes(&self, ias: &mut [f64], g_: &mut [f64], seploss: &mut [f64]) {
        let mut j = 0;
        // PORT: Java for(i=1; i<turn_load.length-1; i++) ↔ i+1 < len (同 remove_zeroes_f64)
        let mut i = 1usize;
        while i + 1 < self.turn_load.len() {
            if self.turn_load[i] != 0.0 {
                ias[j] = i as f64 * 10.0;
                // g[j] = (double) turn_load[i];
                // seploss[j] = (double) sep_loss[i];
                // PORT: 参数名 Java 为 g (遮蔽 PhysicsConstants 的 static import g);
                // Rust E0530 禁止绑定遮蔽常量, 加下划线后缀
                g_[j] = (self.turn_load[i - 1] + self.turn_load[i] + self.turn_load[i + 1]) / 3.0;
                seploss[j] =
                    (self.sep_loss[i - 1] + self.sep_loss[i] + self.sep_loss[i + 1]) / 3.0;
                j += 1;
            }
            i += 1;
        }
    }

    pub fn show_all_em_chart(&self) {
        logger::info("Legacy", "roll rate:"); // Application.debugPrint → Logger.info("Legacy", t)
        let mut i = 0;
        while i < 256 {
            print!("{},", self.roll_rate[i]);
            i += 1;
        }

        // Application.debugPrint("turn:");
        // for(int i = 0; i < 256; i++){
        // System.out.print(turn_load[i]+",");
        // }
        // for(int i = 0; i < 256; i++){
        // System.out.print(sep_loss[i]+",");
        // }
    }

    /// PORT: `ui.util.NotificationService.show(String)` 的注入位 (C 类, 见模块头)。
    fn show(&self, msg: &str) {
        if let Some(notify) = &self.notify {
            notify(msg);
        }
    }
}

/// Java `Boolean.parseBoolean` — 本地同构副本
/// (configuration_service.rs 的 java_parse_boolean 为私有未导出, 待其导出后收敛)。
fn java_parse_boolean(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
}

/// Java 8 `Math.round(double)` 一比一 (JDK 8 源码位级算法, 非朴素 floor(x+0.5)):
/// JDK 7 起 (JDK-8010430) 对半点邻域做了修正 — Java 8 oracle 实测
/// `Math.round(0.49999999999999994d) == 0`, 而朴素 `(x + 0.5).floor()` 给 1
/// (§2.3 先例形式的已知边界分歧; §6 以 Java 8 oracle 实测为准, 故按源码移植)。
/// NaN → 0, ±Inf → Long.MAX/MIN (else 支 `a as i64` 与 Java (long) 转换语义一致)。
fn java_math_round(a: f64) -> i64 {
    let long_bits = a.to_bits();
    let biased_exp = ((long_bits & 0x7FF0_0000_0000_0000) >> 52) as i64;
    // shift = (SIGNIFICAND_WIDTH - 2 + EXP_BIAS) - biasedExp = 1074 - biasedExp
    let shift = 1074 - biased_exp;
    if (shift & !63) == 0 {
        // shift >= 0 && shift < 64: a 是有限数且 pow(2,-64) <= ulp(a) < 1
        // r = 有效数字尾 + 隐含 1 (即 a / ulp(a), 带符号)
        let mut r = ((long_bits & 0x000F_FFFF_FFFF_FFFF) | 0x0010_0000_0000_0000) as i64;
        if (long_bits as i64) < 0 {
            r = -r;
        }
        // (r >> shift) = floor(a * 2); +1 后 >> 1 = floor(a + 1/2) (全程精确, 无浮点舍入)
        ((r >> shift) + 1) >> 1
    } else {
        // |a| < 2^-64 量级 / a 已是数学整数 / Inf / NaN
        a as i64
    }
}

/// Java 8 `Float.toString(float)` 一比一 (analyze 通知里 `(int)X / 10.0f` 的字符串拼接):
/// 10^-3 ≤ |f| < 10^7 → 十进制平原式恒至少一位小数 ("12.0"); 否则 "D.DDDE±x"
/// ('E' 后仅负指数带 '-'); 最短可区分数字串; NaN/±0/±Inf 特判。
/// PORT: 数字串取 Rust `{:e}` 最短往返表示 — 与 Java FloatingDecimal 在
/// JDK-4511638 域 (极罕见多位尾数) 外逐位一致 (config_loader.rs 私有同名
/// java_double_to_string 的 f32 同构副本, 待其导出后收敛)。
fn java_float_to_string(f: f32) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0.0".to_string() } else { "0.0".to_string() };
    }
    if f.is_infinite() {
        return if f > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let neg = f.is_sign_negative();
    let a = f.abs();
    // "{:e}" → "D.DDDe±n"; a > 0 有限恒此形态 (最短往返数字, 无尾随零)
    let sci = format!("{:e}", a);
    let epos = sci.find('e').unwrap();
    let mant = &sci[..epos];
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let mut s = String::new();
    if (-3..=6).contains(&exp10) {
        // 平原式
        if exp10 >= 0 {
            let ip = exp10 as usize + 1; // 整数部分位数
            if digits.len() > ip {
                s.push_str(&digits[..ip]);
                s.push('.');
                s.push_str(&digits[ip..]);
            } else {
                s.push_str(&digits);
                s.push_str(&"0".repeat(ip - digits.len()));
                s.push_str(".0"); // 恒至少一位小数
            }
        } else {
            s.push_str("0.");
            s.push_str(&"0".repeat((-exp10 - 1) as usize));
            s.push_str(&digits);
        }
    } else {
        // 科学计数
        s.push_str(&digits[..1]);
        s.push('.');
        if digits.len() > 1 {
            s.push_str(&digits[1..]);
        } else {
            s.push('0');
        }
        s.push('E');
        s.push_str(&exp10.to_string());
    }
    if neg {
        s.insert(0, '-');
    }
    s
}

/// Java `String.format("%.1f", d)` 一比一 (updateEMChart 通知)。
/// 语义模型 (config_loader.rs java_format_f4 同源, Java 8 oracle 实证): 等价
/// `new BigDecimal(Double.toString(d)).setScale(1, HALF_UP)` — 对**最短往返十进制
/// 表示**做 HALF_UP (2.675 → "2.7"), 而非精确二进制值展开; Rust `{:.1}` 是对
/// 精确值的半偶舍入, 双重分歧 (5.25 → Java "5.3" vs Rust "5.2")。
/// NaN/Infinity 原样; -0.0 → "-0.0"; 巨整数域 (exp10 > 25) 全整数输出 ".0"。
fn java_format_f1(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let neg = d.is_sign_negative(); // 含 -0.0 → "-0.0" (Java 亦然)
    let a = d.abs();
    let sci = format!("{:e}", a);
    let epos = sci.find('e').unwrap();
    let mant = &sci[..epos];
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits = mant.replace('.', "");
    let digits = digits.as_bytes();
    let n = digits.len() as i32;

    let mut out = String::new();
    if exp10 > 25 {
        // 巨整数域 (10^26 以上 double 间距 > 1, 恒无有效小数): digits + 隐含尾零 + ".0"
        out.push_str(&sci[..epos].replace('.', ""));
        out.push_str(&"0".repeat((exp10 - n + 1) as usize));
        out.push_str(".0");
    } else {
        // 最短表示的 i 号数字 (1-based, place = 10^(exp10-i+1)); 越界补 0
        let digit_at = |i: i32| -> u128 {
            if i < 1 {
                0
            } else {
                let idx = (i - 1) as usize;
                if idx < digits.len() {
                    u128::from(digits[idx] - b'0')
                } else {
                    0
                }
            }
        };
        // 保留到 10^-1 位: i ≤ exp10 + 2; 判定位 = 其后一位 (HALF_UP: ≥5 进位,
        // 再后的剩余数字 < 1 单位不影响判定)
        let keep = exp10 + 2;
        let mut scaled: u128 = 0; // = (整数 + 1 位小数) 的 10 倍
        if keep > 0 {
            for i in 1..=keep {
                scaled = scaled * 10 + digit_at(i);
            }
        }
        if digit_at(keep + 1) >= 5 {
            scaled += 1; // HALF_UP (含精确 .5 进位; 进位可级联到整数部分)
        }
        let int_part = scaled / 10;
        let frac1 = scaled % 10;
        out.push_str(&format!("{int_part}.{frac1}"));
    }
    if neg {
        out.insert(0, '-');
    }
    out
}

#[cfg(test)]
mod tests {
    // PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
    // 不改成 struct 字面量以保持与 Java 测试源逐行对应
    #![allow(clippy::field_reassign_with_default)]

    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // ---- 测试替身 ----

    struct MockState {
        indic_type: Option<String>,
        eng_type: i32,
        elapsed_ms: i64,
        total_hp: i32,
        total_thrust: i32,
        total_hp_eff: i32,
        sep: f64,
    }

    /// 可变 Service 替身: init 与 analyze 各时刻读到的值不同 (Java 里 Service 字段
    /// 由轮询线程持续改写), 用 Mutex 内部可变性模拟。
    struct MockService {
        state: Mutex<MockState>,
    }

    impl AnalyzerService for MockService {
        fn s_indic_type(&self) -> Option<String> {
            self.state.lock().unwrap().indic_type.clone()
        }
        fn i_eng_type(&self) -> i32 {
            self.state.lock().unwrap().eng_type
        }
        fn elapsed_time(&self) -> i64 {
            self.state.lock().unwrap().elapsed_ms
        }
        fn total_hp(&self) -> i32 {
            self.state.lock().unwrap().total_hp
        }
        fn total_thrust(&self) -> i32 {
            self.state.lock().unwrap().total_thrust
        }
        fn total_hp_eff(&self) -> i32 {
            self.state.lock().unwrap().total_hp_eff
        }
        fn sep(&self) -> f64 {
            self.state.lock().unwrap().sep
        }
    }

    fn mock_service() -> Arc<MockService> {
        Arc::new(MockService {
            state: Mutex::new(MockState {
                indic_type: Some("spitfire_f24".to_string()),
                eng_type: 1,
                elapsed_ms: 42000,
                total_hp: 2050,
                total_thrust: 0,
                total_hp_eff: 1800,
                sep: 10.5,
            }),
        })
    }

    fn set_mock(svc: &MockService, elapsed_ms: i64, hp_eff: i32, sep: f64) {
        let mut st = svc.state.lock().unwrap();
        st.elapsed_ms = elapsed_ms;
        st.total_hp_eff = hp_eff;
        st.sep = sep;
    }

    /// ConfigProvider 内存替身 (config_api::config_provider 测试同款最小实现;
    /// 内部 Mutex 以满足 Arc<dyn ConfigProvider + Send + Sync> 的跨线程共享形态)
    struct MapConfig {
        values: Mutex<HashMap<String, String>>,
    }

    impl MapConfig {
        fn new() -> Self {
            MapConfig { values: Mutex::new(HashMap::new()) }
        }
    }

    impl ConfigProvider for MapConfig {
        fn get_config(&self, key: &str) -> Option<String> {
            self.values.lock().unwrap().get(key).cloned()
        }
        fn set_config(&self, key: &str, value: &str) {
            self.values.lock().unwrap().insert(key.to_string(), value.to_string());
        }
        fn is_field_disabled(&self, _key: &str) -> bool {
            false
        }
    }

    /// 通知捕获器: 模拟 NotificationService.show
    // PORT: Java 保真 — 回调类型同 notify 字段, 不拆 type 别名
    #[allow(clippy::type_complexity)]
    fn capture_notify() -> (Arc<Mutex<Vec<String>>>, Arc<dyn Fn(&str) + Send + Sync>) {
        let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let c = captured.clone();
        let cb: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(move |msg: &str| {
            c.lock().unwrap().push(msg.to_string());
        });
        (captured, cb)
    }

    // ---- Default (§2.10) ----

    #[test]
    fn default_arrays_full_length_zeroed_and_fields_null() {
        let fa = FlightAnalyzer::default();
        assert_eq!(fa.time.len(), 256);
        assert_eq!(fa.power.len(), 256);
        assert_eq!(fa.thrust.len(), 256);
        assert_eq!(fa.eff.len(), 256);
        assert_eq!(fa.sep.len(), 256);
        assert_eq!(fa.roll_rate.len(), 256);
        assert_eq!(fa.roll_alr.len(), 256);
        assert_eq!(fa.turn_load.len(), 256);
        assert_eq!(fa.turn_elev.len(), 256);
        assert_eq!(fa.sep_loss.len(), 256);
        assert!(fa.time.iter().all(|&v| v == 0.0));
        assert!(fa.power.iter().all(|&v| v == 0));
        assert!(fa.turn_load.iter().all(|&v| v == 0.0));
        assert_eq!(fa.engine_type, 0);
        assert_eq!(fa.initalt_stage, 0);
        assert_eq!(fa.curalt_stage, 0);
        assert!(!fa.is_information);
        assert_eq!(fa.r#type, None); // Java null
    }

    // ---- init ----

    #[test]
    fn init_records_stage_snapshot() {
        let svc = mock_service();
        let mut fa = FlightAnalyzer::default();
        fa.init(5, svc, None);
        assert_eq!(fa.r#type.as_deref(), Some("spitfire_f24"));
        assert_eq!(fa.engine_type, 1);
        assert_eq!(fa.initalt_stage, 5);
        assert_eq!(fa.curalt_stage, 5);
        assert_eq!(fa.count, 1);
        assert_eq!(fa.time[5], 42.0); // 42000ms / 1000f
        assert_eq!(fa.power[5], 2050);
        assert_eq!(fa.thrust[5], 0);
        assert_eq!(fa.eff[5], 1800);
        assert_eq!(fa.sep[5], 10.5);
        // 其余层未动
        assert_eq!(fa.time[0], 0.0);
        assert_eq!(fa.eff[6], 0);
        // config == null → isInformation = false (Java 三目走 "false" 分支)
        assert!(!fa.is_information);
    }

    #[test]
    fn init_elapsed_time_divides_in_f32_then_widens() {
        // §2.12: Java long/1000f 先转 float 除 (f32 精度) 再存 double[]
        let svc = mock_service();
        set_mock(&svc, 1, 0, 0.0); // 1ms
        let mut fa = FlightAnalyzer::default();
        fa.init(0, svc, None);
        // 0.001f32 的 f64 展开值 (float 除法精度, 非 0.001)
        assert_eq!(fa.time[0], 0.0010000000474974513);
    }

    #[test]
    fn init_config_flag_variants() {
        // "true" → 开
        let cfg = Arc::new(MapConfig::new());
        cfg.set_config("enableAltInformation", "true");
        let mut fa = FlightAnalyzer::default();
        fa.init(0, mock_service(), Some(cfg as Arc<dyn ConfigProvider + Send + Sync>));
        assert!(fa.is_information);
        // 大小写不敏感 (Boolean.parseBoolean)
        let cfg2 = Arc::new(MapConfig::new());
        cfg2.set_config("enableAltInformation", "TRUE");
        let mut fa2 = FlightAnalyzer::default();
        fa2.init(0, mock_service(), Some(cfg2 as Arc<dyn ConfigProvider + Send + Sync>));
        assert!(fa2.is_information);
        // 键缺失 (getConfig → null) → parseBoolean(null) = false
        let cfg3 = Arc::new(MapConfig::new());
        let mut fa3 = FlightAnalyzer::default();
        fa3.init(0, mock_service(), Some(cfg3 as Arc<dyn ConfigProvider + Send + Sync>));
        assert!(!fa3.is_information);
    }

    #[test]
    #[should_panic]
    fn analyze_before_init_panics_like_npe() {
        let mut fa = FlightAnalyzer::default();
        fa.analyze(1); // Java: xs.iEngType → NullPointerException
    }

    // ---- analyze ----

    #[test]
    fn analyze_same_stage_accumulates() {
        let svc = mock_service();
        let mut fa = FlightAnalyzer::default();
        fa.init(5, svc.clone(), None);
        set_mock(&svc, 42000, 1400, 11.0);
        fa.analyze(5); // 5 != 6 → 累加分支
        set_mock(&svc, 42000, 1300, 9.0);
        fa.analyze(5);
        assert_eq!(fa.curalt_stage, 5);
        assert_eq!(fa.eff[5], 1800 + 1400 + 1300);
        assert_eq!(fa.sep[5], 10.5 + 11.0 + 9.0);
        assert_eq!(fa.count, 3);
    }

    #[test]
    fn analyze_next_stage_finalizes_average_and_records() {
        let svc = mock_service();
        let mut fa = FlightAnalyzer::default();
        fa.init(5, svc.clone(), None); // eff[5]=1800 sep[5]=10.5 count=1
        set_mock(&svc, 42000, 1400, 11.0);
        fa.analyze(5); // eff[5]=3200 sep[5]=21.5 count=2
        set_mock(&svc, 42000, 1300, 9.0);
        fa.analyze(5); // eff[5]=4500 sep[5]=30.5 count=3
        set_mock(&svc, 100200, 1100, 12.3);
        fa.analyze(6);
        // 终结平均: eff[5] = 4500/3 (int 截断除); sep[5] = 30.5/(3*9.80)
        assert_eq!(fa.eff[5], 4500 / 3);
        assert_eq!(fa.sep[5], 30.5 / (3.0 * g));
        assert_eq!(fa.curalt_stage, 6);
        assert_eq!(fa.count, 1);
        // 新层数据: 100200ms → f32 除法 → 100.2f32 的 f64 展开 (§2.12)
        assert_eq!(fa.time[6], 100.19999694824219);
        assert_eq!(fa.eff[6], 1100);
        assert_eq!(fa.sep[6], 12.3);
    }

    // analyze 通知 — Java 8 oracle 全串对拍 (见 /FA3 oracle: golden.txt)
    #[test]
    fn analyze_notification_message_delta1() {
        let svc = mock_service();
        set_mock(&svc, 42000, 0, 0.0);
        let (cap, cb) = capture_notify();
        let mut fa = FlightAnalyzer::default();
        fa.notify = Some(cb);
        let cfg = Arc::new(MapConfig::new());
        cfg.set_config("enableAltInformation", "true");
        fa.init(5, svc.clone(), Some(cfg as Arc<dyn ConfigProvider + Send + Sync>));
        set_mock(&svc, 100200, 1100, 12.3);
        fa.analyze(6);
        let msgs = cap.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        // oracle: 到达 600米，用时 100秒，平均爬升率 0.9米/秒，记录完成
        // (1000/(f32 100.2 → f64 100.19999694824219) = 9.98 → (int)9 → 9/10.0f = 0.9)
        assert_eq!(
            msgs[0],
            "到达 600米，用时 100秒，平均爬升率 0.9米/秒，记录完成"
        );
    }

    #[test]
    fn analyze_notification_message_delta3() {
        let svc = mock_service();
        set_mock(&svc, 42000, 0, 0.0);
        let (cap, cb) = capture_notify();
        let mut fa = FlightAnalyzer::default();
        fa.notify = Some(cb);
        let cfg = Arc::new(MapConfig::new());
        cfg.set_config("enableAltInformation", "true");
        fa.init(5, svc.clone(), Some(cfg as Arc<dyn ConfigProvider + Send + Sync>));
        set_mock(&svc, 100500, 0, 0.0); // time = 100.5 (f32 精确)
        fa.analyze(6); // delta1: 1000/100.5 → 9 → 0.9
        set_mock(&svc, 100500, 0, 0.0);
        fa.analyze(7); // delta2: 2000/100.5 = 19.9 → (int)19 → 1.9
        set_mock(&svc, 100500, 0, 0.0);
        fa.analyze(8); // delta3: 3000/100.5 = 29.85 → (int)29 → 2.9
        let msgs = cap.lock().unwrap();
        assert_eq!(msgs.len(), 3);
        // oracle (FA.java M1 d=3 t=100.5): 2.9
        assert!(msgs[2].starts_with("到达 800米，用时 100秒，平均爬升率 2.9"));
        assert!(msgs[2].ends_with("米/秒，记录完成"));
        assert!(msgs[1].contains("爬升率 1.9"));
    }

    #[test]
    fn analyze_notification_zero_time_float_inf_domain() {
        // time = 0 → (int)(1000/0.0)=Integer.MAX_VALUE → /10.0f → Float.toString
        let svc = mock_service();
        set_mock(&svc, 0, 0, 0.0);
        let (cap, cb) = capture_notify();
        let mut fa = FlightAnalyzer::default();
        fa.notify = Some(cb);
        let cfg = Arc::new(MapConfig::new());
        cfg.set_config("enableAltInformation", "true");
        fa.init(0, svc.clone(), Some(cfg as Arc<dyn ConfigProvider + Send + Sync>));
        fa.analyze(1);
        let msgs = cap.lock().unwrap();
        // oracle (FA.java M1 t=0.0): Java 输出 "2.14748368E8"; 本实现最短往返表示
        // 给 "2.1474837E8" (JDK-4511638 域已文档化分歧, 回读同一 f32, 见单测注记)
        assert_eq!(
            msgs[0],
            "到达 100米，用时 0秒，平均爬升率 2.1474837E8米/秒，记录完成"
        );
    }

    #[test]
    fn analyze_notification_suppressed_without_flag() {
        let svc = mock_service();
        let (cap, cb) = capture_notify();
        let mut fa = FlightAnalyzer::default();
        fa.notify = Some(cb);
        fa.init(5, svc.clone(), None); // isInformation = false
        set_mock(&svc, 100200, 0, 0.0);
        fa.analyze(6);
        assert!(cap.lock().unwrap().is_empty());
    }

    // ---- getSpeedStage ----

    #[test]
    fn get_speed_stage_boundaries() {
        let fa = FlightAnalyzer::default();
        assert_eq!(fa.get_speed_stage(0.0), 0);
        assert_eq!(fa.get_speed_stage(300.0), 30);
        // Math.round 半值向上: 305/10 = 30.5 → 31 (§2.3)
        assert_eq!(fa.get_speed_stage(305.0), 31);
        assert_eq!(fa.get_speed_stage(295.4), 30); // 29.54 → 30
        assert_eq!(fa.get_speed_stage(295.0), 30); // 29.5 → floor(30.0) = 30
        assert_eq!(fa.get_speed_stage(-6.0), -1); // -0.6 → floor(-0.1) = -1
        assert_eq!(fa.get_speed_stage(-4.0), 0); // -0.4 → floor(0.1) = 0
        assert_eq!(fa.get_speed_stage(2559.9), 256); // 255.99 → 256 (调用方靠 <256 守卫)
        assert_eq!(fa.get_speed_stage(f64::NAN), 0);
    }

    // ---- updateEMChart (滚转) ----

    #[test]
    fn update_em_chart_roll_updates_and_notifies_with_old_rate() {
        let (cap, cb) = capture_notify();
        let mut fa = FlightAnalyzer::default();
        fa.notify = Some(cb);
        fa.is_information = true;
        fa.roll_rate[30] = 50; // 旧记录
        fa.update_em_chart(300.0, 1.0, 100, 10.0, 0, 6);
        assert_eq!(fa.roll_rate[30], 100);
        assert_eq!(fa.roll_alr[30], 6);
        let msgs = cap.lock().unwrap();
        assert_eq!(msgs.len(), 1); // wx(100) - 旧值(50) = 50 > 40 → 通知
        assert_eq!(msgs[0], "速度  300km/h下的最大滚转率: 100度/秒,记录完成");
    }

    #[test]
    fn update_em_chart_roll_threshold_exactly_40_no_notify() {
        let (cap, cb) = capture_notify();
        let mut fa = FlightAnalyzer::default();
        fa.notify = Some(cb);
        fa.roll_rate[30] = 50;
        fa.update_em_chart(300.0, 1.0, 90, 10.0, 0, 6); // 90-50 = 40, 不 > 40
        assert_eq!(fa.roll_rate[30], 90); // 值仍更新
        assert!(cap.lock().unwrap().is_empty());
    }

    #[test]
    fn update_em_chart_roll_gates() {
        let mut fa = FlightAnalyzer::default();
        // abs_alr > 5 失败 (== 5)
        fa.update_em_chart(300.0, 1.0, 100, 10.0, 0, 5);
        assert_eq!(fa.roll_rate[30], 0);
        // wx > 10 失败 (== 10)
        fa.update_em_chart(300.0, 1.0, 10, 10.0, 0, 6);
        assert_eq!(fa.roll_rate[30], 0);
        // abs_alr >= roll_alr 失败
        fa.roll_alr[30] = 80;
        fa.update_em_chart(300.0, 1.0, 100, 10.0, 0, 79);
        assert_eq!(fa.roll_rate[30], 0);
        // wx > roll_rate 失败 (相等)
        fa.roll_alr[30] = 0;
        fa.roll_rate[30] = 100;
        fa.update_em_chart(300.0, 1.0, 100, 10.0, 0, 6);
        assert_eq!(fa.roll_rate[30], 100);
    }

    // ---- updateEMChart (盘旋/过载) ----

    #[test]
    fn update_em_chart_turn_updates_and_notifies_half_up() {
        let (cap, cb) = capture_notify();
        let mut fa = FlightAnalyzer::default();
        fa.notify = Some(cb);
        fa.is_information = true;
        fa.turn_load[30] = 3.5;
        fa.sep_loss[30] = 1.5;
        // g_load=7.0: 7.0-3.5=3.5 > 3.0 → 通知; (3.5+7.0)/2=5.25 → %.1f HALF_UP "5.3"
        fa.update_em_chart(300.0, 7.0, 0, 1.0, 10, 0);
        assert_eq!(fa.turn_elev[30], 10);
        assert_eq!(fa.turn_load[30], (3.5 + 7.0) / 2.0);
        assert_eq!(fa.sep_loss[30], (1.5 + 1.0) / 2.0);
        let msgs = cap.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        // 5.25/1.25 是精确半点: Java HALF_UP → 5.3/1.3, Rust {:.1} 半偶会给 5.2/1.2
        assert_eq!(msgs[0], "速度  300km/h下的最大法向过载: 5.3G, 此时SEP为: 1.3m/s, 记录完成");
    }

    #[test]
    fn update_em_chart_turn_threshold_exactly_3_no_notify() {
        let (cap, cb) = capture_notify();
        let mut fa = FlightAnalyzer::default();
        fa.notify = Some(cb);
        fa.turn_load[30] = 3.5;
        fa.update_em_chart(300.0, 6.5, 0, 1.0, 10, 0); // 6.5-3.5 = 3.0, 不 > 3.0
        assert_eq!(fa.turn_load[30], 5.0); // 平均照常记录
        assert!(cap.lock().unwrap().is_empty());
    }

    #[test]
    fn update_em_chart_turn_gates() {
        let mut fa = FlightAnalyzer::default();
        // g_load > 1.0 失败 (== 1.0)
        fa.update_em_chart(300.0, 1.0, 0, 1.0, 10, 0);
        assert_eq!(fa.turn_load[30], 0.0);
        // sep < 5 失败 (== 5)
        fa.update_em_chart(300.0, 7.0, 0, 5.0, 10, 0);
        assert_eq!(fa.turn_load[30], 0.0);
        // abs_elev >= turn_elev 失败
        fa.turn_elev[30] = 20;
        fa.update_em_chart(300.0, 7.0, 0, 1.0, 10, 0);
        assert_eq!(fa.turn_load[30], 0.0);
    }

    #[test]
    fn update_em_chart_stage_out_of_range_ignored() {
        let mut fa = FlightAnalyzer::default();
        fa.update_em_chart(2560.0, 9.0, 300, 1.0, 50, 90); // stage 256 → 忽略
        fa.update_em_chart(-6.0, 9.0, 300, 1.0, 50, 90); // stage -1 → 忽略
        assert!(fa.roll_rate.iter().all(|&v| v == 0));
        assert!(fa.turn_load.iter().all(|&v| v == 0.0));
    }

    // ---- getNoZerosNum ----

    #[test]
    fn get_no_zeros_num_counts_nonzero() {
        let fa = FlightAnalyzer::default();
        assert_eq!(fa.get_no_zeros_num_i32(&[0, 1, 0, -3, 100]), 3);
        assert_eq!(fa.get_no_zeros_num_i32(&[0; 256]), 0);
        assert_eq!(fa.get_no_zeros_num_f64(&[0.0, 0.5, -0.1, 0.0]), 2);
        assert_eq!(fa.get_no_zeros_num_f64(&[]), 0);
    }

    // ---- removeZeroes / removeRollRatesZeroes / removeLoadZeroes ----

    #[test]
    fn remove_zeroes_i32_smooths_three_point() {
        let fa = FlightAnalyzer::default();
        let mut x = [0.0; 8];
        let mut y = [0.0; 8];
        let oy = [0, 10, 20, 0, 40, 50, 0, 0];
        fa.remove_zeroes_i32(&mut x, &mut y, &oy);
        // 非零: i=1,2,4,5 → j=0..3; y = (oy[i-1]+oy[i]+oy[i+1])/3
        assert_eq!(&x[..4], &[10.0, 20.0, 40.0, 50.0]);
        assert_eq!(y[0], (10 + 20) as f64 / 3.0);
        assert_eq!(y[1], (10 + 20) as f64 / 3.0);
        assert_eq!(y[2], (40 + 50) as f64 / 3.0);
        assert_eq!(y[3], (40 + 50) as f64 / 3.0);
        assert_eq!(y[4], 0.0); // 未写入区不动
    }

    #[test]
    #[should_panic]
    fn remove_zeroes_i32_first_nonzero_panics_like_aioobe() {
        // Java: i=0 且 oy[0]!=0 → oy[-1] ArrayIndexOutOfBoundsException
        let fa = FlightAnalyzer::default();
        let mut x = [0.0; 4];
        let mut y = [0.0; 4];
        fa.remove_zeroes_i32(&mut x, &mut y, &[7, 0, 0, 0]);
    }

    #[test]
    #[should_panic]
    fn remove_zeroes_i32_last_nonzero_panics_like_aioobe() {
        // Java: i=len-1 且非零 → oy[len] 越界
        let fa = FlightAnalyzer::default();
        let mut x = [0.0; 4];
        let mut y = [0.0; 4];
        fa.remove_zeroes_i32(&mut x, &mut y, &[0, 0, 0, 7]);
    }

    #[test]
    fn remove_zeroes_f64_skips_boundary_indices() {
        let fa = FlightAnalyzer::default();
        let mut x = [0.0; 8];
        let mut y = [0.0; 8];
        let oy = [9.0, 10.0, 20.0, 0.0, 40.0, 50.0, 60.0, 70.0];
        fa.remove_zeroes_f64(&mut x, &mut y, &oy);
        // 循环 1..len-1: 非零 i=1,2,4,5,6 → j=0..4; i=0 与 i=7 永不访问 (无越界)
        assert_eq!(&x[..5], &[10.0, 20.0, 40.0, 50.0, 60.0]);
        assert_eq!(y[0], (9.0 + 10.0 + 20.0) / 3.0);
        assert_eq!(y[4], (50.0 + 60.0 + 70.0) / 3.0); // i=6: oy[5..7]
    }

    #[test]
    fn remove_roll_rates_zeroes_end_to_end() {
        let mut fa = FlightAnalyzer::default();
        fa.roll_rate = vec![0, 0, 10, 0, 0, 0, 0, 0];
        let mut ias = [0.0; 8];
        let mut wx = [0.0; 8];
        fa.remove_roll_rates_zeroes(&mut ias, &mut wx);
        assert_eq!(ias[0], 20.0);
        assert_eq!(wx[0], 10_f64 / 3.0);
    }

    #[test]
    fn remove_load_zeroes_end_to_end() {
        let mut fa = FlightAnalyzer::default();
        // 仅 i=2 非零 (i=1..len-2 扫描, i=1/3 非零会再占 j 槽位)
        fa.turn_load = vec![0.0, 0.0, 6.0, 0.0, 0.0];
        fa.sep_loss = vec![0.0, 0.0, 4.0, 0.0, 0.0];
        let mut ias = [0.0; 8];
        let mut g_ = [0.0; 8];
        let mut seploss = [0.0; 8];
        fa.remove_load_zeroes(&mut ias, &mut g_, &mut seploss);
        assert_eq!(ias[0], 20.0);
        assert_eq!(g_[0], (0.0 + 6.0 + 0.0) / 3.0);
        assert_eq!(seploss[0], (0.0 + 4.0 + 0.0) / 3.0);
        assert_eq!(ias[1], 0.0); // 其余层为 0 不写
    }

    // ---- showAllEMChart ----

    #[test]
    fn show_all_em_chart_smoke() {
        let fa = FlightAnalyzer::default();
        fa.show_all_em_chart(); // 不 panic (输出被 cargo test 捕获)
    }

    // ---- 通知注入边界 ----

    #[test]
    fn notification_dropped_when_notify_not_wired() {
        let svc = mock_service();
        let mut fa = FlightAnalyzer::default(); // notify = None
        let cfg = Arc::new(MapConfig::new());
        cfg.set_config("enableAltInformation", "true");
        fa.init(5, svc.clone(), Some(cfg as Arc<dyn ConfigProvider + Send + Sync>));
        set_mock(&svc, 100200, 0, 0.0);
        fa.analyze(6); // 消息照常构造, 通知丢弃 (P4 接线前)
        assert!(fa.notify.is_none());
    }

    // ---- Java 8 oracle: java_float_to_string (Float.toString) ----

    #[test]
    fn java_float_to_string_matches_java8_oracle() {
        // oracle FA.java: (int)((d*1000)/t) / 10.0f 的 Float.toString 输出
        let cases: &[(f32, &str)] = &[
            (1.0f32 / 10.0f32, "0.1"),             // d=1 t=600
            (9.0f32 / 10.0f32, "0.9"),             // d=1 t=100.5
            (1.0f32 / 10.0f32, "0.1"),             // d=1 t=1000
            (23.0f32 / 10.0f32, "2.3"),            // d=1 t=42
            (81.0f32 / 10.0f32, "8.1"),            // d=1 t=12.34
            (333.0f32 / 10.0f32, "33.3"),          // d=1 t=3
            (200.0f32, "200.0"),                   // d=1 t=0.5
            (1000.0f32, "1000.0"),                 // d=1 t=0.1
            (0.5f32, "0.5"),                       // d=3 t=600: (int)5 / 10.0f
            (29.0f32 / 10.0f32, "2.9"),            // d=3 t=100.5
            (71.0f32 / 10.0f32, "7.1"),            // d=3 t=42
            (243.0f32 / 10.0f32, "24.3"),          // d=3 t=12.34
            (600.0f32, "600.0"),                   // d=3 t=0.5
            (3000.0f32, "3000.0"),                 // d=3 t=0.1
            (20.0f32 / 10.0f32, "2.0"),            // d=12 t=600: (int)20 / 10.0f
            (119.0f32 / 10.0f32, "11.9"),          // d=12 t=100.5
            (285.0f32 / 10.0f32, "28.5"),          // d=12 t=42
            (972.0f32 / 10.0f32, "97.2"),          // d=12 t=12.34
            (2400.0f32, "2400.0"),                 // d=12 t=0.5
            (12000.0f32, "12000.0"),               // d=12 t=0.1
            (426.0f32 / 10.0f32, "42.6"),          // d=256 t=600
            (2547.0f32 / 10.0f32, "254.7"),        // d=256 t=100.5
            (6095.0f32 / 10.0f32, "609.5"),        // d=256 t=42
            (20745.0f32 / 10.0f32, "2074.5"),      // d=256 t=12.34
            (85333.0f32 / 10.0f32, "8533.3"),      // d=256 t=3
            (51200.0f32, "51200.0"),               // d=256 t=0.5
            (256000.0f32, "256000.0"),             // d=256 t=0.1
            (2560000.0f32, "2560000.0"),           // 平原式尾零补齐探针 (10^6 域, < 10^7)
            (-20.0f32, "-20.0"),                   // d=1 t=-5
            // t=0 除零域: (int)+Inf = Integer.MAX_VALUE → /10.0f = 214748368.0f32。
            // Java oracle 输出 "2.14748368E8" (9 位, JDK-4511638 域非最短表示);
            // 本实现最短往返 8 位 "2.1474837E8" (回读同一 f32) — 与 config_loader
            // java_double_to_string 对该域的已文档化分歧同一先例, 仅除零退化通知可达
            (2147483647.0f32 / 10.0f32, "2.1474837E8"),
        ];
        for &(v, want) in cases {
            assert_eq!(java_float_to_string(v), want, "Float.toString({v})");
        }
        // 特殊值
        assert_eq!(java_float_to_string(0.0f32), "0.0");
        assert_eq!(java_float_to_string(-0.0f32), "-0.0");
        assert_eq!(java_float_to_string(f32::NAN), "NaN");
        assert_eq!(java_float_to_string(f32::INFINITY), "Infinity");
        assert_eq!(java_float_to_string(f32::NEG_INFINITY), "-Infinity");
        // 平原/科学分界: 10^7 上沿科学, 10^-3 下沿科学
        assert_eq!(java_float_to_string(9999999.0f32), "9999999.0");
        assert_eq!(java_float_to_string(0.001f32), "0.001");
        assert_eq!(java_float_to_string(0.0001f32), "1.0E-4");
    }

    // ---- Java 8 oracle: java_format_f1 (String.format("%.1f", double)) ----

    #[test]
    fn java_format_f1_matches_java8_oracle() {
        // MR.java oracle: 精确半点 5.25/1.25/0.25/0.75 → HALF_UP (Rust {:.1} 半偶会
        // 给 5.2/1.2/0.2/0.8, 双重分歧点钉死)
        let cases: &[(f64, &str)] = &[
            (3.25, "3.3"),
            (3.75, "3.8"),
            (2.675, "2.7"),   // 最短往返 "2.675" HALF_UP (精确二进制是 2.67499...)
            (0.05, "0.1"),
            (0.15, "0.2"),
            (9.999999, "10.0"), // 进位级联到整数
            (1.0, "1.0"),
            (6.05, "6.1"),
            (12.345, "12.3"),
            (0.0, "0.0"),
            (5.25, "5.3"),
            (1.25, "1.3"),
            (0.25, "0.3"),
            (0.75, "0.8"),
            (2.35, "2.4"),
        ];
        for &(v, want) in cases {
            assert_eq!(java_format_f1(v), want, "String.format(\"%.1f\", {v})");
        }
        assert_eq!(java_format_f1(-0.0), "-0.0");
        assert_eq!(java_format_f1(f64::NAN), "NaN");
        assert_eq!(java_format_f1(f64::INFINITY), "Infinity");
        assert_eq!(java_format_f1(f64::NEG_INFINITY), "-Infinity");
    }

    // ---- Java 8 oracle: java_math_round (Math.round) ----

    #[test]
    fn java_math_round_matches_java8_oracle() {
        // MR.java oracle (含 JDK-8010430 修正域: 0.49999999999999994 → 0)
        let cases: &[(f64, i64)] = &[
            (0.5, 1),
            (2.5, 3),
            (-2.5, -2),
            (30.5, 31),
            (0.49999999999999994, 0), // 朴素 floor(x+0.5) 给 1 — 分歧点钉死
            (29.54, 30),
            (-0.6, -1),
            (255.99, 256),
            (2559.9, 2560),
            (3000.0, 3000),
        ];
        for &(v, want) in cases {
            assert_eq!(java_math_round(v), want, "Math.round({v})");
        }
        assert_eq!(java_math_round(f64::NAN), 0);
        assert_eq!(java_math_round(f64::INFINITY), i64::MAX);
        assert_eq!(java_math_round(f64::NEG_INFINITY), i64::MIN);
    }

    // ---- 跨线程共享形态 (Arc<dyn ... + Send + Sync>) ----

    #[test]
    fn analyzer_service_trait_object_safe_and_send() {
        fn assert_send_sync<T: Send + Sync>(_: &T) {}
        let svc: Arc<dyn AnalyzerService + Send + Sync> = mock_service();
        assert_send_sync(&svc);
        let mut fa = FlightAnalyzer::default();
        fa.init(1, svc, None);
        assert_eq!(fa.r#type.as_deref(), Some("spitfire_f24"));
    }
}
