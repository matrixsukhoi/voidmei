//! 派生度量分析器: 爬升分段统计 (各高度级时间/功率/推力/有效功率/SEP 平均)
//! + EM 图记录 (速度分段的滚转率 / 过载 / SEP 损失)。
//!
//! 依赖倒置: Service 落 vm-data (vm-data → vm-core 单向依赖, 本 crate 反向不可引),
//! 故以 trait [`AnalyzerService`] 暴露 init/analyze 实际读取的 7 个 Service 字段,
//! 由 vm-data 的 service_fields 侧 impl 接线。
//!
//! 活读契约: `analyze()` 每次调用实时读取 elapsedTime/totalHp/totalThrust/
//! totalHpEff/SEP, 故持有 `Arc<dyn AnalyzerService>` 而非快照 —— 按快照适配会使
//! analyze 冻结在 init 时刻的值 (time[] 记录错误时刻、eff/sep 累加失真)。防回归由
//! flight_log.rs 的 analyze_flow 测试锁定 (RecordingService 每读递增)。
//! notify 与 flight_log::NotifySink 同类型, 共用同一 sink。
//!
//! 通知出口: [`FlightAnalyzer::notify`] 注入回调, 未接线 (None) 时通知丢弃。

use std::sync::Arc;

use crate::base::format::fmt_f;
use crate::base::java_compat::java_parse_boolean;
use crate::base::physics_constants::g;
use crate::config::config_api::config_provider::ConfigProvider;
use crate::lang::Lang;

/// FlightAnalyzer 对 Service 的读取面 (依赖倒置, 见模块头说明)。
/// `s_indic_type` 的 `Option` 表达 type 键缺失; `elapsed_time` 单位毫秒。
///
/// impl 侧两条硬性义务:
/// 1. sIndic 缺失时 impl 须 panic (对齐 Java NPE) 或论证 Service 轮询链保证
///    sIndic 恒已初始化 — Option 只表达 "type 键缺失", 不得吞掉 "引用缺失";
/// 2. getter 逐字段调用对应逐字段读取时刻 (init 7 次 / analyze 5 次, 不成组),
///    impl 按 getter 粒度读锁/快照, 禁止假设 7 个 getter 恒在一次成组调用里。
pub trait AnalyzerService: Send + Sync {
    /// 载具机型名 (sIndic 引用缺失时 impl 须 panic, 见 trait 级义务 1)
    fn s_indic_type(&self) -> Option<String>;
    /// 引擎类型
    fn eng_type(&self) -> crate::base::engine_type::EngineType;
    /// 经过时间 (毫秒)
    fn elapsed_time(&self) -> i64;
    fn total_hp(&self) -> i32;
    fn total_thrust(&self) -> i32;
    fn total_hp_eff(&self) -> i32;
    fn sep(&self) -> f64;
}

/// 高度级数上限 (每级 100m)
pub const MAX_ALT_STAGE: i32 = 256;

/// 速度级数上限 (每级 10km/h, 0 表示非法, 0 - 2560km/h)
pub const MAX_IAS_STAGE: i32 = 256;

pub struct FlightAnalyzer {
    pub engine_type: crate::base::engine_type::EngineType,
    /// 机型名 (原始字段名 `type` 为 Rust 关键字; null → None)
    pub r#type: Option<String>,
    pub time: Vec<f64>,  // 从第零层开始
    pub power: Vec<i32>, // 从第一层开始
    pub thrust: Vec<i32>,
    pub eff: Vec<i32>,
    pub sep: Vec<f64>,
    pub initalt_stage: i32,
    pub curalt_stage: i32,
    /// 高度级信息通知开关 (配置 enableAltInformation); 同 crate 可见 (FlightLog 专用)
    pub(crate) is_information: bool,

    /// Service 读取面 — 未 init 即 analyze 时 expect panic (对应 Java NullPointerException)
    xs: Option<Arc<dyn AnalyzerService + Send + Sync>>,
    /// 计数: 10Hz 轮询下 i32 溢出需 ~6.8 年停留在同一高度级, 域内不可达
    count: i32,
    config: Option<Arc<dyn ConfigProvider + Send + Sync>>,

    // ---- EM 图记录 (速度分段) ----
    pub roll_rate: Vec<i32>,
    pub roll_alr: Vec<i32>,

    pub turn_load: Vec<f64>,
    pub turn_elev: Vec<i32>,

    pub sep_loss: Vec<f64>,

    /// 通知注入回调 (见模块头; None 时通知丢弃)
    #[allow(clippy::type_complexity)]
    pub notify: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

// 数值字段 0 / boolean false / 引用 None,
// 两组 stage 数组立即按 256 长度零填充。
impl Default for FlightAnalyzer {
    fn default() -> Self {
        FlightAnalyzer {
            engine_type: crate::base::engine_type::EngineType::Unknown,
            r#type: None,
            time: vec![0.0; MAX_ALT_STAGE as usize], // 从第零层开始
            power: vec![0; MAX_ALT_STAGE as usize],  // 从第一层开始
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
    /// Service 读取面访问: init 前为 None (未 init 即用 → panic, 对应 Java NPE)。
    fn xs(&self) -> &Arc<dyn AnalyzerService + Send + Sync> {
        self.xs
            .as_ref()
            .expect("FlightAnalyzer 未 init 即访问 xs (Java NullPointerException)")
    }

    /// 记录首个高度级快照并复位计数。
    // 唯一调用者 flight_log (Log.init → 首帧 analyzeData)
    pub(crate) fn init(
        &mut self,
        stage: i32,
        st: Arc<dyn AnalyzerService + Send + Sync>,
        config: Option<Arc<dyn ConfigProvider + Send + Sync>>,
    ) {
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
        self.engine_type = xs.eng_type();
        self.initalt_stage = stage;
        self.curalt_stage = self.initalt_stage;
        let idx = self.curalt_stage as usize;
        self.time[idx] = xs.elapsed_time() as f64 / 1000.0; // 波21: f32 复刻退役
        self.power[idx] = xs.total_hp();
        self.thrust[idx] = xs.total_thrust();
        self.eff[idx] = xs.total_hp_eff();
        self.sep[idx] = xs.sep();
    }

    /// 逐帧累积/切换高度级 (同层累加均值分量, 进层落账并开新层)。
    // 唯一调用者 flight_log (logTick → analyzeData 每帧)
    pub(crate) fn analyze(&mut self, stage: i32) {
        let xs = self.xs().clone(); // Arc 浅拷贝, 避免与 &mut self 借用冲突
        self.engine_type = xs.eng_type();
        if stage == self.curalt_stage + 1 {
            let idx = self.curalt_stage as usize;
            self.eff[idx] /= self.count;
            self.sep[idx] /= self.count as f64 * g; // count * g: int 提升为 double
            self.curalt_stage += 1;

            let idx = self.curalt_stage as usize;
            self.time[idx] = xs.elapsed_time() as f64 / 1000.0; // 波21: f32 复刻退役
            self.power[idx] = xs.total_hp();
            self.thrust[idx] = xs.total_thrust();
            self.eff[idx] = xs.total_hp_eff();
            self.sep[idx] = xs.sep();
            self.count = 1;
            if self.is_information {
                let lang = Lang::init_lang();
                // climb = (int)((stage - initalt_stage) * 1000 / time[..]) / 10:
                // 波21: f32 复刻退役, f64 直算 + fmt_f 一位小数
                let climb = ((stage - self.initalt_stage) * 1000) as f64 / self.time[idx];
                let msg = format!(
                    "{}{}{}{}{}{}{}",
                    lang.f_a1,
                    stage * 100,
                    lang.f_a2,
                    self.time[idx] as i32,
                    lang.f_a3,
                    fmt_f(climb as i32 as f64 / 10.0, 1), // Java (int) 截断语义保留 (inf 饱和 i32::MAX)
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
        // Java (int)(long) 双转是低 32 位截断; Rust as i32 饱和, 仅巨值域分歧
        (java_math_round(ias / 10.0) as u32) as i32
    }

    // 使用舵面辅助判断
    // pub: FlightLog 之外的调用方 (如 vm-data) 可直调
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
                self.turn_elev[s] = abs_elev;
                if self.is_information && (g_load - self.turn_load[s] > 3.0) {
                    let lang = Lang::init_lang();
                    self.show(&format!(
                        "{}{}{}{}{}{}{}",
                        lang.f_a_turn1,
                        stage * 10,
                        lang.f_a_turn2,
                        fmt_f((self.turn_load[s] + g_load) / 2.0, 1),
                        lang.f_a_turn3,
                        fmt_f((self.sep_loss[s] + sep) / 2.0, 1),
                        lang.f_a_turn4
                    ));
                }
                self.turn_load[s] = (self.turn_load[s] + g_load) / 2.0;
                self.sep_loss[s] = (self.sep_loss[s] + sep) / 2.0;
            }
        }
    }

    /// 非零元素计数 (波21: Java 重载 _i32/_f64 合一为泛型迭代器形态)。
    pub fn get_no_zeros_num<T: PartialEq + Default + Copy>(&self, arr: &[T]) -> i32 {
        arr.iter().filter(|&&v| v != T::default()).count() as i32
    }

    /// 抽稀: 保留非零样本, y 取三点滑动平均 (int 数组重载)。
    pub fn remove_zeroes_i32(&self, x: &mut [f64], y: &mut [f64], oy: &[i32]) {
        let mut j = 0;
        let mut i = 0usize;
        while i < oy.len() {
            if oy[i] != 0 {
                x[j] = i as f64 * 10.0;
                // 循环自 i=0 起 — oy[0]!=0 时 oy[i-1] 即越界 panic (对齐 Java
                // ArrayIndexOutOfBoundsException 的同条件失败)
                y[j] = (oy[i - 1] + oy[i] + oy[i + 1]) as f64 / 3.0;
                j += 1;
            }
            i += 1;
        }
    }

    /// 抽稀: 保留非零样本, y 取三点滑动平均 (f64 数组重载)。
    pub fn remove_zeroes_f64(&self, x: &mut [f64], y: &mut [f64], oy: &[f64]) {
        let mut j = 0;
        // i 从 1 起止于 len-1 (len=0 时不进循环)
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
        self.remove_zeroes_i32(ias, wx, &self.roll_rate);
    }

    pub fn remove_load_zeroes(&self, ias: &mut [f64], g_: &mut [f64], seploss: &mut [f64]) {
        let mut j = 0;
        // i 从 1 起止于 len-1 (同 remove_zeroes_f64)
        let mut i = 1usize;
        while i + 1 < self.turn_load.len() {
            if self.turn_load[i] != 0.0 {
                ias[j] = i as f64 * 10.0;
                // 参数名沿 Java 的 g; Rust E0530 禁止绑定遮蔽常量 g, 加下划线后缀
                g_[j] = (self.turn_load[i - 1] + self.turn_load[i] + self.turn_load[i + 1]) / 3.0;
                seploss[j] = (self.sep_loss[i - 1] + self.sep_loss[i] + self.sep_loss[i + 1]) / 3.0;
                j += 1;
            }
            i += 1;
        }
    }

    /// 通知出口 (见模块头)。
    fn show(&self, msg: &str) {
        if let Some(notify) = &self.notify {
            notify(msg);
        }
    }
}

/// Java 8 `Math.round(double)` 一比一 (JDK 8 源码位级算法, 非朴素 floor(x+0.5)):
/// JDK 7 起 (JDK-8010430) 对半点邻域做了修正 — 历史基线
/// `Math.round(0.49999999999999994d) == 0`, 而朴素 `(x + 0.5).floor()` 给 1。
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

#[cfg(test)]
mod tests;
