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
//! PORT (FlightLog 接线合同, **已履行**): Java `analyze()` 每次调用**活读**
//! xs.elapsedTime/totalHp/totalThrust/totalHpEff/SEP (FlightAnalyzer.java:55-59,65-66),
//! 故 retained `Arc<dyn AnalyzerService>` 是唯一正确建模 —— `FlightLog` 集成时
//! 应弃其 `FlightAnalyzerApi` 快照合同、直接持有本具体类型 (pub 字段面 + 包私有
//! init/analyze 的 pub(crate) 同 crate 可见), 并在构造面携带
//! `Arc<dyn AnalyzerService>`; 按快照合同适配会使 analyze 冻结在 init 时刻的值 (time[]
//! 记录错误时刻、eff/sep 累加失真)。notify 两侧类型一致
//! (`Arc<dyn Fn(&str) + Send + Sync>` = flight_log::NotifySink), 可共用同一 sink。
//! (落地形态: FlightLog::init 第 5 参注入 Arc<dyn AnalyzerService>, vm-data 提供
//! ServiceData 的 impl 适配器 ServiceAnalyzerSource; 活读防回归由
//! flight_log.rs 的 analyze_flow 测试锁定 — RecordingService 每读递增。)
//!
//! PORT (CLASSIFY 裁决"注入回调"): `ui.util.NotificationService.show(String)` 是
//! C 类 UI 静态入口 —— 本译以 [`FlightAnalyzer::notify`] 字段注入, 未接线 (None)
//! 时通知丢弃, P4 NotificationService 落地后由调用方 (FlightLog/Controller) 接上。
//! PORT: `Application.debugPrint(t)` = `Logger.info("Legacy", t)` (Application.java:213)。

use std::sync::Arc;

use crate::config_api::config_provider::ConfigProvider;
use crate::lang::lang::Lang;
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
    // 唯一调用者 flight_log (parser 同包, Controller.java:332 `Log.init` → 首帧 analyzeData)
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

        let enable_alt_info = match &self.config {
            Some(c) => c.get_config("enableAltInformation"),
            None => Some("false".to_string()),
        };
        self.is_information = java_parse_boolean(enable_alt_info.as_deref().unwrap_or(""));

        let xs = self.xs().clone();
        self.r#type = xs.s_indic_type();
        self.engine_type = xs.i_eng_type();
        self.initalt_stage = stage;
        self.curalt_stage = self.initalt_stage;
        let idx = self.curalt_stage as usize;
        self.time[idx] = (xs.elapsed_time() as f32 / 1000.0f32) as f64;
        self.power[idx] = xs.total_hp();
        self.thrust[idx] = xs.total_thrust();
        self.eff[idx] = xs.total_hp_eff();
        self.sep[idx] = xs.sep();
        // Application.debugPrint("已经记录stage"+curaltStage+"时间戳"+time[curaltStage]+"功率"+power[curaltStage]+"实功率"+eff[curaltStage]+"SEP"+sep[curaltStage]);
    }

    /// Java: `void analyze(int stage)` (包私有)。
    // 唯一调用者 flight_log (parser 同包, logTick → analyzeData 每帧)
    pub(crate) fn analyze(&mut self, stage: i32) {
        let xs = self.xs().clone(); // Arc 浅拷贝, 避免与 &mut self 借用冲突
        self.engine_type = xs.i_eng_type();
        if stage == self.curalt_stage + 1 {
            let idx = self.curalt_stage as usize;
            self.eff[idx] /= self.count;
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
                //       + (int) ((stage - initaltStage) * 1000 / time[..]) / 10.0f + Lang.fA4
                // — (int) X / 10.0f 是 int 除 float 得 float, 字符串拼接走 Float.toString
                let climb = ((stage - self.initalt_stage) * 1000) as f64 / self.time[idx];
                let msg = format!(
                    "{}{}{}{}{}{}{}",
                    lang.f_a1,
                    stage * 100,
                    lang.f_a2,
                    self.time[idx] as i32,
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

            if g_load > 1.0 && sep < 5.0 && abs_elev >= self.turn_elev[s] {
                // if (g_load > turn_load[stage] ) {
                self.turn_elev[s] = abs_elev;
                if self.is_information && (g_load - self.turn_load[s] > 3.0) {
                    let lang = Lang::init_lang();
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
mod tests;
