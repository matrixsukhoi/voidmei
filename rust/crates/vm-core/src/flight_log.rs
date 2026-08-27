//! 对应 Java: `src/parser/FlightLog.java` (一比一翻译, B 类: 文件 IO + 记录)
//!
//! 飞行数据 CSV 日志记录: 每次 logTick 由 Service 轮询线程驱动 (Service.java:1827),
//! 写一行遥测快照; 换机/退出时 close() 落盘三份分析 CSV (climb/roll/ny)。
//!
//! PORT (crate 边界, D6): Java 直接持有 `Service xs` / `Controller xc` / `FlightAnalyzer fA`
//! 三个跨文件引用 —— Service 落 vm-data (本 crate 不可见), Controller 属后续批次。处理方式:
//! - Service → 每次调用注入快照 [`FlightLogSnapshot`] (Java 在 Service 线程上实时读
//!   xs 公有字段, 快照即该时刻字段值, 语义等价); vm-data 侧的快照构造面 =
//!   `vm_data::service_loop::flight_log_snapshot`;
//! - FlightAnalyzer → **已按 flight_analyzer.rs 模块头的集成合同弃快照 trait**:
//!   直接持有同 crate 具体类型 [`FlightAnalyzer`] (pub 字段面 + pub(crate) 方法),
//!   并注入 `Arc<dyn AnalyzerService>` 令 analyze 活读 Service 字段 (快照合同会把
//!   time[]/eff/sep 冻结在 init 时刻, 见彼处论证);
//! - Controller → [`ControllerLogSink`] 最小接口 (唯一用途 `xc.logon = false`);
//! - NotificationService.show (C 类 UI 静态入口, CLASSIFY 裁决"翻译时须注入回调") →
//!   [`NotifySink`] 回调注入 (FlightAnalyzer.notify 同类型, 共用同一 sink)。
//!
//! 行格式保真: CSV 数值列的文本 = Java `String.concat` 的隐式 toString
//! (Double.toString / Float.toString / Integer.toString), 见 [`java_double_to_string`]。

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{Datelike, Local, Timelike};

use crate::config_api::ConfigProvider;
use crate::flight_analyzer::{AnalyzerService, FlightAnalyzer, MAX_IAS_STAGE};
use crate::g;
use crate::lang::Lang;
use crate::logger::warn_default;

// ============================================================================
// Java Float.toString / Double.toString 复刻 (行格式保真依赖)
// ============================================================================

/// Java `Double.toString(double)` 语义复刻。
///
/// Java 规则: 最短十进制数字 (round-trip 唯一); `10^-3 <= |v| < 10^7` 用小数形式,
/// 其余用科学计数 `d.dddEn`; 整数必带一位小数 ("5.0"); NaN→"NaN", ±∞→"Infinity"。
/// PORT: 数字生成用 Rust `{:e}` (最短 round-trip); Java 8 旧 FloatingDecimal 在个别
/// 极端次正规数 (如 4.9E-324) 会多产一位数字 (Java 19 才改 Ryū), 遥测值域
/// (elapsedTime/60000f、Ny、加速度、SEP) 不可能命中, 接受该偏差。
fn java_double_to_string(v: f64) -> String {
	if v.is_nan() {
		return "NaN".to_string();
	}
	if v.is_infinite() {
		return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
	}
	java_plain_or_exp(&format!("{:e}", v))
}

/// Java `Float.toString(float)` 语义复刻 (规则同 [`java_double_to_string`], f32 最短数字)。
fn java_float_to_string(v: f32) -> String {
	if v.is_nan() {
		return "NaN".to_string();
	}
	if v.is_infinite() {
		return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
	}
	java_plain_or_exp(&format!("{:e}", v))
}

/// 按 Java 规则重排 Rust LowerExp 输出 (`[-]d[.ddd]e[-]n`, 无 '+' 号、指数无前导零)。
/// 科学计数指数 = 首数字的十进制幂次 `lead_exp` (Rust 尾数已规格化, e 值即 lead_exp)。
fn java_plain_or_exp(exp_repr: &str) -> String {
	let (mantissa, exp_str) = exp_repr.split_once('e').expect("LowerExp 必含 'e'");
	let exp: i32 = exp_str.parse().expect("LowerExp 指数必为整数");
	let neg = mantissa.starts_with('-');
	let mantissa = mantissa.trim_start_matches('-');
	let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
	let n = digits.len() as i32;

	let mut out = String::new();
	if neg {
		out.push('-');
	}
	// Java: 10^-3 <= |v| < 10^7 ⇔ -3 <= lead_exp <= 6 → 小数形式
	if (-3..=6).contains(&exp) {
		if exp >= 0 {
			if n <= exp + 1 {
				// 整数部分为全部数字 + 补零, 小数部分至少一位 "0"
				out.push_str(&digits);
				for _ in n..=exp {
					out.push('0');
				}
				out.push_str(".0");
			} else {
				out.push_str(&digits[..(exp + 1) as usize]);
				out.push('.');
				out.push_str(&digits[(exp + 1) as usize..]);
			}
		} else {
			// 0.00ddd 形式
			out.push_str("0.");
			for _ in 0..(-exp - 1) {
				out.push('0');
			}
			out.push_str(&digits);
		}
	} else {
		// 计算机科学记数法: 恰一位非零首数字 + 至少一位小数 + 'E' + 指数
		out.push_str(&digits[..1]);
		out.push('.');
		if n > 1 {
			out.push_str(&digits[1..]);
		} else {
			out.push('0');
		}
		out.push('E');
		out.push_str(&exp.to_string());
	}
	out
}

// ============================================================================
// 依赖面 (crate 边界注入, 见模块头 PORT 注释)
// ============================================================================

/// 对应 Java `ui.util.NotificationService.show(String)` 的注入回调 (C 类 UI 静态入口,
/// CLASSIFY 裁决: 翻译时须注入回调)。参数即通知文本 (Lang.xxx)。
pub type NotifySink = Arc<dyn Fn(&str) + Send + Sync>;

/// FlightLog 对 Controller 的依赖面: Java 字段 `Controller xc` 的唯一用途是
/// init 失败路径的 `xc.logon = false` (FlightLog.java:409, Controller.java:44
/// `public boolean logon`)。PORT: Controller 属后续批次, 按 FocusMonitor/
/// FocusDetector 先例在消费方定义最小接口; Rust 侧实现应为原子写 (跨线程)。
pub trait ControllerLogSink: Send + Sync {
	/// Java: `xc.logon = <bool>` (公有字段直写)
	fn set_logon(&self, logon: bool);
}

/// FlightLog 的跨线程共享槽: Java `Controller.Log` 字段 (Service 轮询线程每轮
/// `c.Log.logTick()`, Controller 主线程 init/close 换入换出) 的 Rust 对位形态。
/// 外层 Mutex 保护 Some/None 切换 (= Java 的 `Log = new/null`), 内层 Mutex 串行化
/// tick/close 对实例的独占访问; 两侧均**不嵌套持锁** (先取 Arc 后释放外层), 无死锁。
pub type FlightLogSlot = Arc<Mutex<Option<Arc<Mutex<FlightLog>>>>>;

/// logTick 时刻 Service 公有字段的快照 (D6: Service 落 vm-data, 本 crate 以值注入;
/// Java 在 Service 线程上实时读 `xs.` 字段, 快照即该时刻字段值, 语义等价)。
/// 字段类型逐一对应 Java 声明 (Service.java / State.java)。
#[derive(Debug, Clone, Default)]
pub struct FlightLogSnapshot {
	// ---- writeData 使用 (writeData 各列) ----
	/// Java: `public long elapsedTime` (Service.java:90)
	pub elapsed_time: i64,
	/// Java: `public String throttle` (Service.java:144)
	pub throttle: String,
	/// Java: `public String IAS` (Service.java:135)
	pub ias: String,
	/// Java: `public String TAS` (Service.java:134)
	pub tas: String,
	/// Java: `public String M` (Service.java:136, 马赫数显示串)
	pub mach: String,
	/// Java: `public String salt` (Service.java:107, 高度显示串)
	pub salt: String,
	/// Java: `public String watertemp` (Service.java:161)
	pub watertemp: String,
	/// Java: `public String oiltemp` (Service.java:162)
	pub oiltemp: String,
	/// Java: `public String Vy` (Service.java:141)
	pub vy: String,
	/// Java: `public String sSEP` (Service.java:108)
	pub s_sep: String,
	/// Java: `public double sState.Ny` (State.java:27)
	pub ny: f64,
	/// Java: `public String Wx` (Service.java:142)
	pub wx: String,
	/// Java: `public String totalHpStr` (Service.java:54)
	pub total_hp_str: String,
	/// Java: `public String efficiency[]` 的 [0] (Service.java:169)
	pub efficiency_0: String,
	/// Java: `public String totalHpEffStr` (Service.java:56)
	pub total_hp_eff_str: String,
	/// Java: `public String rpm` (Service.java:177)
	pub rpm: String,
	/// Java: `public int totalThrust` (Service.java:58)
	pub total_thrust: i32,
	/// Java: `public double acceleration` (Service.java:102)
	pub acceleration: f64,
	/// Java: `public String RPMthrottle` (Service.java:145)
	pub rpm_throttle: String,
	/// Java: `public String pitch[]` 的 [0] (Service.java:163)
	pub pitch_0: String,
	/// Java: `public String radiator` (Service.java:146)
	pub radiator: String,
	/// Java: `public String mixture` (Service.java:147)
	pub mixture: String,
	/// Java: `public int sState.compressorstage` (State.java:35)
	pub compressorstage: i32,
	/// Java: `public int sState.magenato` (State.java:36, 原文拼写如此)
	pub magenato: i32,
	/// Java: `public String manifoldpressure` (Service.java:155)
	pub manifoldpressure: String,
	/// Java: `public String flaps` (Service.java:132)
	pub flaps: String,
	/// Java: `public int sState.elevator` (State.java:17)
	pub elevator: i32,
	/// Java: `public int sState.aileron` (State.java:16)
	pub aileron: i32,
	/// Java: `public int sState.rudder` (State.java:18)
	pub rudder: i32,
	/// Java: `public String AoA` (Service.java:137)
	pub aoa: String,
	/// Java: `public String AoS` (Service.java:138)
	pub aos: String,
	// ---- analyzeData 使用 ----
	/// Java: `public double alt` (Service.java:74)
	pub alt: f64,
	/// Java: `public int checkAlt` (Service.java:64)
	pub check_alt: i32,
	/// Java: `public double IASv` (Service.java:99)
	pub ias_v: f64,
	/// Java: `public double SEP` (Service.java:103)
	pub sep: f64,
	/// Java: `public double sState.Wx` (State.java:29)
	pub state_wx: f64,
	// ---- init 使用 ----
	/// Java: `public String sIndic.type` (Indicators.java:8)
	pub indic_type: String,
}

/// Java `BufferedWriter.ensureOpen()` 的 "Stream closed" IOException 复刻:
/// BufferedWriter 包裹 null writer 时惰性成功, 首次 write/close 抛
/// `IOException("Stream closed")` (非 NPE) — init 文件打开失败路径依赖此语义。
fn stream_closed() -> io::Error {
	io::Error::other("Stream closed")
}

// ============================================================================
// FlightLog
// ============================================================================

/// 对应 Java: `public class FlightLog implements Runnable`
pub struct FlightLog {
	/// Java: `public volatile boolean doit` (写触发, Controller.writeDown 置 true)。
	/// PORT: LIFETIMES §3.2 裁决 volatile doit → Arc<AtomicBool> (跨线程持引用)。
	pub doit: Arc<AtomicBool>,
	/// Java: `public volatile boolean logon` (run 线程退出标志, close() 置 false)
	pub logon: Arc<AtomicBool>,
	/// Java: `public FileOutputStream resultsFile` — 创建/截断目标文件后即被弃置
	/// (Java 从不 close, 句柄泄漏至 GC; 保真保留字段, Drop 时关闭)。
	pub results_file: Option<File>,
	/// Java: `public FileWriter csv` (append 句柄; writeLabel 经它落盘)。
	/// PORT: Java 的 csvWritter 与 csv 包裹**同一** FileWriter 对象; Rust 无共享
	/// 别名句柄, 以同路径第二个 append 句柄代餐 (append 模式下字节流等价,
	/// 标签先行 flush 后数据句柄才建, 顺序保持)。
	pub csv: Option<File>,
	/// Java: `public String fileName`
	pub file_name: String,
	/// Java: `public FlightAnalyzer fA` → 直接持有具体类型 (集成合同, 见模块头 PORT)
	pub f_a: Option<FlightAnalyzer>,
	/// Java: `Controller xc` (唯一用途 `xc.logon = false`) → 最小接口注入
	xc: Option<Arc<dyn ControllerLogSink>>,
	// PORT: Java 字段 `Service xs` 不落地 — D6 边界, 快照按调用注入;
	// PORT: Java 字段 `Calendar c` 不落地 — 仅 init 用于文件名时间戳 (chrono::Local)。
	/// Java: `boolean firstAnalyze = true`
	first_analyze: bool,
	/// Java: `private String climbName`
	climb_name: String,
	/// Java: `private String maneuverName` — 原文件即从未赋值/读取的死字段, 保真保留
	#[allow(dead_code)]
	maneuver_name: String,
	/// Java: `private String rollName`
	roll_name: String,
	/// Java: `private String loadName`
	load_name: String,
	/// Java: `private BufferedWriter csvWritter`。
	/// PORT: None 表示 Java 的 `new BufferedWriter(null)` (init 文件打开失败时的
	/// "null writer", write/flush/close 抛 IOException("Stream closed"))。
	csv_writter: Option<BufWriter<File>>,
	/// Java: `private long writeTime`
	write_time: i64,
	/// Java: `private prog.config.ConfigProvider config`
	config: Option<Arc<dyn ConfigProvider + Send + Sync>>,
	/// PORT: Java 读静态 Lang 字段 (Application 启动即装载); Rust Lang 为实例,
	/// init 时取 `Lang::init_lang()` 当前值 (与"启动即装载"终态一致)。
	lang: Lang,
	/// PORT: NotificationService.show 注入回调 (None = 未注入, 静默)
	notify: Option<NotifySink>,
	/// `new FlightAnalyzer()` 依赖的 Service 活读面 (集成合同: analyze 每次调用
	/// 活读 elapsedTime/totalHp 等 7 字段, 故注入 Arc 而非快照 — 见模块头 PORT)
	analyze_service: Option<Arc<dyn AnalyzerService + Send + Sync>>,
}

/// 对应 Java: `new FlightLog()` — 字段默认值 (PORTING §2.10: 引用 null → Option::None,
/// boolean false, long 0, firstAnalyze=true 为 Java 显式初始化值)
impl Default for FlightLog {
	fn default() -> Self {
		Self::new()
	}
}

impl FlightLog {
	pub fn new() -> Self {
		FlightLog {
			doit: Arc::new(AtomicBool::new(false)),
			logon: Arc::new(AtomicBool::new(false)),
			results_file: None,
			csv: None,
			file_name: String::new(),
			f_a: None,
			xc: None,
			first_analyze: true,
			climb_name: String::new(),
			maneuver_name: String::new(),
			roll_name: String::new(),
			load_name: String::new(),
			csv_writter: None,
			write_time: 0,
			config: None,
			lang: Lang::default(),
			notify: None,
			analyze_service: None,
		}
	}

	/// 通知回调便捷调用 (Java: `ui.util.NotificationService.show(Lang.xxx)`)
	fn notify_show(&self, text: &str) {
		if let Some(n) = &self.notify {
			n(text);
		}
	}

	/// 对应 Java: `void writeLabel(FileWriter txt) throws IOException`
	fn write_label(txt: Option<&mut File>, lang: &Lang) -> io::Result<()> {
		let Some(txt) = txt else {
			// Java: new BufferedWriter(null) 惰性成功, 首 bw.write 触发 ensureOpen
			// → IOException("Stream closed") — init 文件打开失败路径即走这里
			return Err(stream_closed());
		};
		let mut bw = BufWriter::new(txt);
		bw.write_all(lang.l1.as_bytes())?; // 1

		bw.write_all(lang.l2.as_bytes())?; // 2

		bw.write_all(lang.l3.as_bytes())?; // 3

		bw.write_all(lang.l4.as_bytes())?; // 4

		bw.write_all(lang.l5.as_bytes())?; // 5

		bw.write_all(lang.l6.as_bytes())?; // 6

		bw.write_all(lang.l7.as_bytes())?; // 7

		bw.write_all(lang.l8.as_bytes())?; // 8

		bw.write_all(lang.l9.as_bytes())?; // 9

		bw.write_all(lang.l10.as_bytes())?; // 10

		bw.write_all(lang.l11.as_bytes())?; // 11

		bw.write_all(lang.l12.as_bytes())?; // 12

		bw.write_all(lang.l13.as_bytes())?; // 13

		bw.write_all(lang.l14.as_bytes())?; // 16

		bw.write_all(lang.l15.as_bytes())?; // 14

		bw.write_all(lang.l16.as_bytes())?; // 15

		bw.write_all(lang.l17.as_bytes())?; // 16

		bw.write_all(lang.l18.as_bytes())?; // 17

		bw.write_all(lang.l19.as_bytes())?; // 18

		bw.write_all(lang.l20.as_bytes())?; // 19

		bw.write_all(lang.l21.as_bytes())?; // 20

		bw.write_all(lang.l22.as_bytes())?; // 21

		bw.write_all(lang.l23.as_bytes())?; // 22

		bw.write_all(lang.l24.as_bytes())?; // 23

		bw.write_all(lang.l25.as_bytes())?; // 24

		bw.write_all(lang.l26.as_bytes())?; // 25

		bw.write_all(lang.l27.as_bytes())?; // 26

		bw.write_all(lang.l28.as_bytes())?; // 27

		bw.write_all(lang.l29.as_bytes())?; // 28

		bw.write_all(lang.l30.as_bytes())?; // 29

		bw.write_all(lang.l31.as_bytes())?; // 30

		bw.write_all(b"\n")?;
		bw.flush()?;
		// bw.close();
		Ok(())
	}

	/// 对应 Java: `void writeData(BufferedWriter bw) throws IOException`
	#[allow(unused_assignments)] // Java: String tmp = ""; 随即被覆盖 (保真)
	fn write_data(bw: Option<&mut BufWriter<File>>, xs: &FlightLogSnapshot) -> io::Result<()> {
		let Some(bw) = bw else {
			// Java: csvWritter 为 null writer 时 write → IOException("Stream closed")
			return Err(stream_closed());
		};
		let mut tmp = String::new();

		// Java: xs.elapsedTime / 60000.0f — long 提升为 float 后除 (f32 链, §2.12)
		write!(bw, "{},", java_float_to_string(xs.elapsed_time as f32 / 60000.0f32))?; // 1

		tmp = xs.throttle.clone();
		write!(bw, "{tmp},")?; // 2

		write!(bw, "{},", xs.ias)?; // 3

		write!(bw, "{},", xs.tas)?; // 4

		write!(bw, "{},", xs.mach)?; // 5

		write!(bw, "{},", xs.salt)?; // 6

		write!(bw, "{},", xs.watertemp)?; // 7

		write!(bw, "{},", xs.oiltemp)?; // 8

		write!(bw, "{},", xs.vy)?; // 9

		write!(bw, "{},", xs.s_sep)?; // 10

		write!(bw, "{},", java_double_to_string(xs.ny))?; // 11

		write!(bw, "{},", xs.wx)?; // 12

		write!(bw, "{},", xs.total_hp_str)?; // 13

		write!(bw, "{},", xs.efficiency_0)?; // 16

		write!(bw, "{},", xs.total_hp_eff_str)?; // 14

		write!(bw, "{},", xs.rpm)?; // 15

		write!(bw, "{},", xs.total_thrust)?; // 16

		write!(bw, "{},", java_double_to_string(xs.acceleration))?; // 17

		write!(bw, "{},", xs.rpm_throttle)?; // 18

		write!(bw, "{},", xs.pitch_0)?; // 19

		write!(bw, "{},", xs.radiator)?; // 20

		write!(bw, "{},", xs.mixture)?; // 21

		write!(bw, "{},", xs.compressorstage)?; // 22

		write!(bw, "{},", xs.magenato)?; // 23

		write!(bw, "{},", xs.manifoldpressure)?; // 24

		write!(bw, "{},", xs.flaps)?; // 25

		write!(bw, "{},", xs.elevator)?; // 26

		write!(bw, "{},", xs.aileron)?; // 27

		write!(bw, "{},", xs.rudder)?; // 28

		write!(bw, "{},", xs.aoa)?; // 29

		write!(bw, "{},", xs.aos)?; // 30

		bw.write_all(b"\n")?;
		// bw.flush();
		// bw.close();
		Ok(())
	}

	// 进行数据分析
	fn analyze_data(&mut self, xs: &FlightLogSnapshot) {
		// Java: (int) xs.alt / 100 — 强转先于除法, 整数除法向零截断
		let stage = (xs.alt as i32) / 100;
		// Java: Math.abs(int) 对 Integer.MIN_VALUE 回绕为负 (§2.2)
		if xs.check_alt.wrapping_abs() > 10 {
			if self.first_analyze {
				// 第一次分析，先取当前高度
				// Java `new FlightAnalyzer()` — 同 crate 直接构造 (集成合同, 见模块头);
				// notify 与 FlightLog 共用同一 sink (flight_analyzer.rs 模块头合同)
				let mut fa = FlightAnalyzer::default();
				fa.notify = self.notify.clone();
				let src = self
					.analyze_service
					.clone()
					.expect("PORT: 注入缺失 — init 需要 AnalyzerService (Java: Log.init 传 Service xs)");
				fa.init(stage, src, self.config.clone());
				self.f_a = Some(fa);
				self.first_analyze = false;
			} else {
				// 开始分析
				// Java: fA 此处必非 null (firstAnalyze 已翻, fA 与之同生)
				let fa = self.f_a.as_mut().expect("PORT: Java 不变量 — firstAnalyze==false ⇒ fA!=null");
				// Java: (int) Math.abs(xs.sState.Wx) — double abs 后 (int) 截断/饱和,
				// 与 Rust as i32 语义一致
				fa.analyze(stage);
				fa.update_em_chart(
					xs.ias_v,
					xs.ny,
					xs.state_wx.abs() as i32,
					xs.sep / g,
					xs.elevator.wrapping_abs(),
					xs.aileron.wrapping_abs(),
				);
			}
		}

		// 分析速度

	}

	/// 对应 Java: `void writeClimbLabel(FileWriter txt) throws IOException`
	fn write_climb_label(txt: &mut File, lang: &Lang) -> io::Result<()> {
		let mut bw = BufWriter::new(txt);
		// 高度
		write!(bw, "{}/m, ", lang.f_alt)?;

		// 时间
		write!(bw, "t/s, ")?;

		// 动力
		write!(bw, "{}/hp, ", lang.e_power)?;

		// 推力
		write!(bw, "{}/kgf, ", lang.e_thurst)?;

		// SEP
		write!(bw, "{}/m/s", lang.f_sep)?;

		bw.write_all(b"\n")?;
		bw.flush()?;
		// bw.close();
		Ok(())
	}

	/// 对应 Java: `void writeClimbData(FileWriter txt) throws IOException`
	fn write_climb_data(txt: &mut File, f_a: Option<&FlightAnalyzer>) -> io::Result<()> {
		let mut bw = BufWriter::new(txt);
		// Java: fA==null 时 `fA.curaltStage` 抛 NullPointerException (非 IOException,
		// 逃逸 saveClimbData 的 catch 直达 close() 调用方) — §1 映射: panic!
		let f_a = f_a.unwrap_or_else(|| {
			panic!("PORT: Java NPE — fA == null (全程未触发高度分析) 于 fA.curaltStage 抛 NullPointerException")
		});

		for i in 0..f_a.curalt_stage {
			let i = i as usize;
			write!(bw, "{}, ", i * 100)?;
			write!(bw, "{}, ", java_double_to_string(f_a.time[i]))?;
			write!(bw, "{}, ", f_a.power[i])?;
			write!(bw, "{}, ", f_a.thrust[i])?;
			writeln!(bw, "{}", java_double_to_string(f_a.sep[i]))?;

			// bw.write("\n");
		}
		// Application.debugPrint(String.format("total %d climb data logged", fA.curaltStage));
		bw.flush()?;
		Ok(())
	}

	/// 对应 Java: `public void saveClimbData()`
	pub fn save_climb_data(&mut self) {
		// Java: FileWriter tcsv = null;
		// Application.debugPrint("climbdata save to "+ climbName);
		// Java: new FileWriter(climbName, true) — append, 文件不存在则创建
		let mut tcsv = match OpenOptions::new().append(true).create(true).open(&self.climb_name) {
			Ok(f) => f,
			Err(e) => {
				self.notify_show(self.lang.lfail_create);
				warn_default(&format!("文件创建失败: {e}"));
				return;
			}
		};
		// Application.debugPrint("打开文件成功");

		// Java: try { writeClimbLabel; writeClimbData; tcsv.close(); } catch (IOException)
		let res = (|| {
			FlightLog::write_climb_label(&mut tcsv, &self.lang)?;
			FlightLog::write_climb_data(&mut tcsv, self.f_a.as_ref())?;
			// tcsv.close() — 句柄离开作用域即关闭
			Ok::<(), io::Error>(())
		})();
		if let Err(e) = res {
			warn_default(&format!("写入爬升数据失败: {e}"));
		}
	}

	/// 对应 Java: `void writeRollLabel(FileWriter txt) throws IOException`
	fn write_roll_label(txt: &mut File, lang: &Lang) -> io::Result<()> {
		let mut bw = BufWriter::new(txt);
		// 速度
		write!(bw, "{}/km/h, ", lang.f_ias)?;

		// 副翼
		write!(bw, "{}/%, ", lang.v_aileron)?;

		// 滚转率
		write!(bw, "{}/Deg/s", lang.f_wx)?;

		bw.write_all(b"\n")?;
		bw.flush()?;
		// bw.close();
		Ok(())
	}

	/// 对应 Java: `void writeRollData(FileWriter txt) throws IOException`
	#[allow(unused_assignments, unused_variables)] // Java: int k = 0; k++ 后未读 (消费处已被注释)
	fn write_roll_data(txt: &mut File, f_a: Option<&FlightAnalyzer>) -> io::Result<()> {
		let mut bw = BufWriter::new(txt);
		// Java: fA==null 时 `fA.roll_rate[i]` 循环条件读 fA 抛 NPE — 同 writeClimbData
		let f_a = f_a.unwrap_or_else(|| {
			panic!("PORT: Java NPE — fA == null (全程未触发高度分析) 于 fA.roll_rate 抛 NullPointerException")
		});
		let mut k = 0;
		for i in 0..MAX_IAS_STAGE {
			// 速度区间
			let i = i as usize;
			if f_a.roll_rate[i] > 0 {
				k += 1;
				write!(bw, "{}, ", i as i32 * 10)?;
				write!(bw, "{}, ", f_a.roll_alr[i])?;
				writeln!(bw, "{}", f_a.roll_rate[i])?;
			}
			// bw.write("\n");
		}
		// Application.debugPrint(String.format("total %d roll data logged", k));
		bw.flush()?;
		Ok(())
	}

	/// 对应 Java: `public void saveRollData()`
	pub fn save_roll_data(&mut self) {
		// Java: FileWriter tcsv = null;
		// Application.debugPrint("rolldata save to "+ climbName);
		let mut tcsv = match OpenOptions::new().append(true).create(true).open(&self.roll_name) {
			Ok(f) => f,
			Err(e) => {
				self.notify_show(self.lang.lfail_create);
				warn_default(&format!("文件创建失败: {e}"));
				return;
			}
		};
		// Application.debugPrint("打开文件成功");

		let res = (|| {
			FlightLog::write_roll_label(&mut tcsv, &self.lang)?;
			FlightLog::write_roll_data(&mut tcsv, self.f_a.as_ref())?;
			Ok::<(), io::Error>(())
		})();
		if let Err(e) = res {
			warn_default(&format!("写入滚转数据失败: {e}"));
		}
	}

	/// 对应 Java: `void writeNyLabel(FileWriter txt) throws IOException`
	fn write_ny_label(txt: &mut File, lang: &Lang) -> io::Result<()> {
		let mut bw = BufWriter::new(txt);
		// 速度
		write!(bw, "{}/km/h, ", lang.f_ias)?;

		// 升降舵
		write!(bw, "{}/%, ", lang.v_elevator)?;

		// 过载
		write!(bw, "{}/G, ", lang.f_gl)?;

		// SEP
		write!(bw, "{}/m/s", lang.f_sep)?;

		bw.write_all(b"\n")?;
		bw.flush()?;
		// bw.close();
		Ok(())
	}

	/// 对应 Java: `void writeNyData(FileWriter txt) throws IOException`
	#[allow(unused_assignments, unused_variables)] // Java: int k = 0; k++ 后未读 (消费处已被注释)
	fn write_ny_data(txt: &mut File, f_a: Option<&FlightAnalyzer>) -> io::Result<()> {
		let mut bw = BufWriter::new(txt);
		// Java: fA==null 时 `fA.turn_load[i]` 循环条件读 fA 抛 NPE — 同 writeClimbData
		let f_a = f_a.unwrap_or_else(|| {
			panic!("PORT: Java NPE — fA == null (全程未触发高度分析) 于 fA.turn_load 抛 NullPointerException")
		});
		let mut k = 0;
		for i in 0..MAX_IAS_STAGE {
			// 速度区间
			let i = i as usize;
			if f_a.turn_load[i] > 0.0 { // Java: > 0 (int 提升为 double 0.0)
				k += 1;
				write!(bw, "{}, ", i as i32 * 10)?;
				write!(bw, "{}, ", f_a.turn_elev[i])?;
				write!(bw, "{}, ", java_double_to_string(f_a.turn_load[i]))?;
				writeln!(bw, "{}", java_double_to_string(f_a.sep_loss[i]))?;
			}
			// bw.write("\n");
		}
		// Application.debugPrint(String.format("total %d roll data logged", k));
		bw.flush()?;
		Ok(())
	}

	/// 对应 Java: `public void saveNyData()`
	pub fn save_ny_data(&mut self) {
		// Java: FileWriter tcsv = null;
		// Application.debugPrint("rolldata save to "+ climbName);
		let mut tcsv = match OpenOptions::new().append(true).create(true).open(&self.load_name) {
			Ok(f) => f,
			Err(e) => {
				self.notify_show(self.lang.lfail_create);
				warn_default(&format!("文件创建失败: {e}"));
				return;
			}
		};
		// Application.debugPrint("打开文件成功");

		let res = (|| {
			FlightLog::write_ny_label(&mut tcsv, &self.lang)?;
			FlightLog::write_ny_data(&mut tcsv, self.f_a.as_ref())?;
			Ok::<(), io::Error>(())
		})();
		if let Err(e) = res {
			warn_default(&format!("写入过载数据失败: {e}"));
		}
	}

	/// 对应 Java: `public void init(Controller tc, Service s, prog.config.ConfigProvider config)`
	///
	/// PORT (注入参数, 见模块头): `xc` 以 [`ControllerLogSink`] 代餐;
	/// `s` 以 [`FlightLogSnapshot`] 代餐 (仅读 `s.sIndic.type`);
	/// `notify` 为 C 类 UI 注入面; `analyze_service` 是 FlightAnalyzer 集成合同
	/// 要求的 Service 活读面 (取代旧快照工厂, 见模块头 PORT)。
	pub fn init(
		&mut self,
		xc: Arc<dyn ControllerLogSink>,
		s: &FlightLogSnapshot,
		config: Option<Arc<dyn ConfigProvider + Send + Sync>>,
		notify: NotifySink,
		analyze_service: Arc<dyn AnalyzerService + Send + Sync>,
	) {
		self.xc = Some(xc);
		// Java: xs = s; — D6 边界, 不存 Service 引用 (快照按调用注入)
		self.config = config;
		self.notify = Some(notify);
		self.analyze_service = Some(analyze_service);
		self.doit.store(false, Ordering::SeqCst);
		// Application.debugPrint("flightlog初始化了");
		// Java: c = Calendar.getInstance(); c.setTimeInMillis(System.currentTimeMillis());
		// — 时间仅用于文件名。PORT: Calendar.HOUR 是 12 小时制 (0..11, 正午/午夜为 0),
		// 原代码刻意/无意未用 HOUR_OF_DAY — 保真取 hour % 12; 时区 = 本地 (chrono::Local)。
		self.lang = Lang::init_lang();
		let now = Local::now();
		let month = now.month() as i64; // Java: c.get(Calendar.MONTH) + 1 (0 基 + 1)
		let date = now.day() as i64; // Java: c.get(Calendar.DATE)
		let hour = (now.hour() % 12) as i64; // Java: c.get(Calendar.HOUR)
		let minute = now.minute() as i64; // Java: c.get(Calendar.MINUTE)
		let second = now.second() as i64; // Java: c.get(Calendar.SECOND)
		// PORT: Java String.toUpperCase() 取默认 locale (土耳其语 'i'→'İ' 会分叉);
		// Rust to_uppercase 无 locale — 机型名 (ASCII) 两端一致
		let mut name = s.indic_type.to_uppercase();
		// 修复: 原为引用比较(name == "NO COCKPIT")永不为 true, 无座舱视角的飞行记录会以
		// "NO COCKPIT" 命名而非 "Unknown" —— 改用 equals (P4 阶段发现的历史 bug)
		if name == "NO COCKPIT" {
			name = "Unknown".to_string();
		}
		self.file_name = format!("records/{name}_{month}_{date}_{hour}.{minute}.{second}.csv");
		self.climb_name = format!("records/{name}_{month}_{date}_{hour}.{minute}.{second}_climb.csv");
		self.roll_name = format!("records/{name}_{month}_{date}_{hour}.{minute}.{second}_roll.csv");
		self.load_name = format!("records/{name}_{month}_{date}_{hour}.{minute}.{second}_ny.csv");

		// Java: new FileOutputStream(fileName) — 创建/截断 (records/ 不存在则 FileNotFoundException)
		self.results_file = match OpenOptions::new().write(true).create(true).truncate(true).open(&self.file_name) {
			Ok(f) => Some(f),
			Err(e) => {
				self.notify_show(self.lang.lfail_create);
				warn_default(&format!("日志文件创建失败: {e}"));
				// Java: xc.logon = false (Controller.java:44, 非 volatile 跨线程写 — Rust 侧实现取原子)
				if let Some(xc) = &self.xc {
					xc.set_logon(false);
				}
				None
			}
		};
		// Application.debugPrint("文件创建成功");
		// Java: new FileWriter(fileName, true)
		self.csv = match OpenOptions::new().append(true).create(true).open(&self.file_name) {
			Ok(f) => Some(f),
			Err(e) => {
				self.notify_show(self.lang.lfail_create);
				warn_default(&format!("日志文件打开失败: {e}"));
				None
			}
		};
		if let Err(e) = FlightLog::write_label(self.csv.as_mut(), &self.lang) {
			warn_default(&format!("写入标签失败: {e}"));
		}
		// Java: csvWritter = new BufferedWriter(csv) — csv==null 时为 "null writer"
		// (write/flush/close 抛 IOException("Stream closed"), 见 stream_closed)
		// PORT: 独立 append 句柄 (见 csv 字段注释); 打开失败 ⇒ None (null writer 模型)
		self.csv_writter = match OpenOptions::new().append(true).create(true).open(&self.file_name) {
			Ok(f) => Some(BufWriter::new(f)),
			Err(_) => None,
		};

		self.logon.store(true, Ordering::SeqCst);
	}

	/// 对应 Java: `public void close()`
	pub fn close(&mut self) {
		// 保存
		// Java: try { csvWritter.close(); csv.close(); } catch (IOException e) { warn } 语义分解:
		// - csvWritter.close(): BufferedWriter.close() 对 out==null (null writer **或已 close**)
		//   静默返回 (close 不走 ensureOpen); 正常路径 flush 后 finally 置 out=null。
		//   flush 失败 → IOException → csv.close() 被短路跳过。
		// - csv.close(): csv==null 时抛 NPE (非 IOException, 逃逸本方法 —— save*Data 与
		//   logon=false 不再执行); 引用非 null 时幂等静默 (含二次 close)。
		let mut io_err: Option<io::Error> = None;
		if let Some(w) = self.csv_writter.as_mut() {
			if let Err(e) = w.flush() {
				io_err = Some(e);
			}
		}
		// Java finally: flush 失败也置 out=null (后续 flush 走 "Stream closed")
		self.csv_writter = None;
		if io_err.is_none() && self.csv.is_none() {
			panic!("PORT: Java NPE — csv == null (init 文件打开失败) 时 csv.close() 抛 NullPointerException, 逃逸 catch(IOException), save*Data 与 logon=false 不再执行");
		}
		// csv.close() — Rust 句柄留待 Drop (append 且此后无写, 字节面等价;
		// std 默认 share 含 DELETE, 不阻塞外部清理), csv 字段保持 Some 以区分
		// "null 引用" 与 "已关闭引用" (Java 二次 close 幂等静默)
		if let Some(e) = io_err {
			warn_default(&format!("关闭日志文件失败: {e}"));
		}
		self.save_climb_data();
		self.save_roll_data();
		self.save_ny_data();
		self.logon.store(false, Ordering::SeqCst);
	}

	/// 对应 Java: `public void logTick()` (Service 轮询线程直调, Service.java:1827)
	pub fn log_tick(&mut self, xs: &FlightLogSnapshot) {
		// Java: try { analyzeData(); writeData(csvWritter); } catch (IOException e)
		self.analyze_data(xs);
		if let Err(e) = FlightLog::write_data(self.csv_writter.as_mut(), xs) {
			self.notify_show(self.lang.lfail_write);
			warn_default(&format!("写入日志数据失败: {e}"));
		}
		// Java: writeTime++ % 1024 == 0 — 后置自增: 先取原值判断再递增;
		// long 静默回绕 (§2.2, 10Hz 下 2900 万年才触顶, 保真取 wrapping)
		let t = self.write_time;
		self.write_time = self.write_time.wrapping_add(1);
		if t % 1024 == 0 {
			match self.csv_writter.as_mut() {
				Some(w) => {
					if let Err(e) = w.flush() {
						warn_default(&format!("刷新日志缓冲区失败: {e}"));
					}
				}
				None => warn_default(&format!("刷新日志缓冲区失败: {}", stream_closed())),
			}
		}
	}

	/// 对应 Java: `public void run()` (Runnable)。
	/// PORT: 该线程在 Java 中**从未启动** (Controller.java:933 `// Log1.start();` 已注释,
	/// 实际 tick 由 Service 轮询线程直调 logTick) — 保真翻译, 保留备用。
	/// PORT: Java 直接读 xs 公有字段 (与 Service 线程共享实例); D6 边界下以快照源
	/// 闭包代餐。ExceptionHelper.sleepQuietly(5) 的中断→吞掉→重查 logon 语义, 由
	/// 停机标志版 sleep_quietly(&logon) 等价复刻 (§2.13: 提前返回后 while 重查)。
	pub fn run(&mut self, xs_source: &(dyn Fn() -> FlightLogSnapshot + Sync)) {
		while self.logon.load(Ordering::SeqCst) {
			crate::exception_helper::sleep_quietly(&self.logon, 5);
			while self.doit.load(Ordering::SeqCst) {
				self.log_tick(&xs_source());
				self.doit.store(false, Ordering::SeqCst); // 写完后关闭
			}
		}
	}
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;
	use std::path::{Path, PathBuf};
	use std::sync::atomic::AtomicI64;
	use std::sync::Mutex;
	use std::time::Duration;

	/// FlightLog 的 records/ 路径是相对 CWD 的硬编码 (与 Java 一致);
	/// cargo test 并行线程共用进程 CWD → 串行化 + 用完恢复。
	static CWD_LOCK: Mutex<()> = Mutex::new(());

	fn with_temp_cwd(tag: &str, create_records: bool, f: impl FnOnce(&Path)) {
		// 挂 fm::test_guard 串行锁: fm 系测试的 DATA_ROOT 取值是**相对路径**
		// ("./data"/"testroot"/"otherroot", fm_manager.rs ROOTS), 本沙箱的进程级
		// CWD 翻转若与之并发会把它们的 load 铺根漂走 (实测击穿
		// invalidate_clears_negative_cache_entry) — 与 fm 挂锁用例互斥后消除。
		// (config_manager::tests 的 sandbox 有其私有 CWD_LOCK, 不在本 crate 保护面,
		// 备案见其 B4 注释; 本文件不越文件修。)
		let _fm_guard = crate::fm::test_guard::data_root();
		// 应 panic 测试经 catch_unwind 转发 ⇒ 锁可能被毒化, into_inner 容错
		let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
		let root: PathBuf = std::env::temp_dir().join(format!(
			"vm_flight_log_{tag}_{}",
			std::process::id()
		));
		let _ = std::fs::remove_dir_all(&root);
		if create_records {
			std::fs::create_dir_all(root.join("records")).unwrap();
		} else {
			std::fs::create_dir_all(&root).unwrap();
		}
		let old = std::env::current_dir().unwrap();
		std::env::set_current_dir(&root).unwrap();
		let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&root)));
		std::env::set_current_dir(&old).unwrap();
		let _ = std::fs::remove_dir_all(&root);
		drop(_guard); // 先放锁再重抛, 避免 panic 穿过 guard 毒化锁
		drop(_fm_guard);
		if let Err(e) = r {
			std::panic::resume_unwind(e);
		}
	}

	// ---- 测试基建: mock ----

	/// ConfigProvider 最小 mock (键值内存表)
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

	/// ControllerLogSink mock: 记录最后写入值
	struct LogonFlag(AtomicBool);
	impl ControllerLogSink for LogonFlag {
		fn set_logon(&self, logon: bool) {
			self.0.store(logon, Ordering::SeqCst);
		}
	}

	/// AnalyzerService 的活读 mock (FlightAnalyzer 集成合同的 Service 面):
	/// elapsed_time 每读 +1000 — 令 "init 之后 analyze 读到新值" 可观测,
	/// 防快照合同回潮 (快照会把 analyze 冻结在 init 时刻, 见 flight_analyzer.rs 论证)
	struct RecordingService {
		elapsed: AtomicI64,
	}
	impl RecordingService {
		fn shared() -> Arc<Self> {
			Arc::new(RecordingService { elapsed: AtomicI64::new(0) })
		}
	}
	impl AnalyzerService for RecordingService {
		fn s_indic_type(&self) -> Option<String> {
			Some("rec-type".into())
		}
		fn i_eng_type(&self) -> i32 {
			2
		}
		fn elapsed_time(&self) -> i64 {
			// 活读证据: 每次调用值递增
			self.elapsed.fetch_add(1000, Ordering::SeqCst)
		}
		fn total_hp(&self) -> i32 {
			1200
		}
		fn total_thrust(&self) -> i32 {
			3400
		}
		fn total_hp_eff(&self) -> i32 {
			1100
		}
		fn sep(&self) -> f64 {
			49.0
		}
	}

	/// 真实 FlightAnalyzer 直填构造 (Java 保真 — 测试构造器逐字段喂值, 参数表对齐
	/// FlightAnalyzer 的十个保存面字段; 字段全 pub, 免 mock)
	#[allow(clippy::too_many_arguments)]
	fn analyzer_with_data(
		curalt_stage: i32,
		time: Vec<f64>,
		power: Vec<i32>,
		thrust: Vec<i32>,
		sep: Vec<f64>,
		roll_rate: Vec<i32>,
		roll_alr: Vec<i32>,
		turn_load: Vec<f64>,
		turn_elev: Vec<i32>,
		sep_loss: Vec<f64>,
	) -> FlightAnalyzer {
		let mut fa = FlightAnalyzer::default();
		fa.curalt_stage = curalt_stage;
		fa.time = time;
		fa.power = power;
		fa.thrust = thrust;
		fa.sep = sep;
		fa.roll_rate = roll_rate;
		fa.roll_alr = roll_alr;
		fa.turn_load = turn_load;
		fa.turn_elev = turn_elev;
		fa.sep_loss = sep_loss;
		fa
	}

	fn notify_collector() -> (NotifySink, Arc<Mutex<Vec<String>>>) {
		let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
		let s = seen.clone();
		(Arc::new(move |text: &str| s.lock().unwrap().push(text.to_string())), seen)
	}

	/// 前缀数据 + 补零到 256 档 (对齐 FlightAnalyzer 数组定长)
	fn padded<T: Clone>(head: Vec<T>, tail: T) -> Vec<T> {
		let mut v = head;
		v.resize(256, tail);
		v
	}


	/// 带判别值的快照 (各列取值互不相同, 便于校验列序)
	fn sample_snapshot() -> FlightLogSnapshot {
		FlightLogSnapshot {
			elapsed_time: 60000,
			throttle: "50".into(),
			ias: "500".into(),
			tas: "520".into(),
			mach: "0.78".into(),
			salt: "1000".into(),
			watertemp: "85".into(),
			oiltemp: "90".into(),
			vy: "15.2".into(),
			s_sep: "12.3".into(),
			ny: 1.5,
			wx: "3".into(),
			total_hp_str: "1200".into(),
			efficiency_0: "82".into(),
			total_hp_eff_str: "1100".into(),
			rpm: "2800".into(),
			total_thrust: 0,
			acceleration: 0.5,
			rpm_throttle: "100".into(),
			pitch_0: "35".into(),
			radiator: "0".into(),
			mixture: "60".into(),
			compressorstage: 1,
			magenato: 1,
			manifoldpressure: "1.15".into(),
			flaps: "0".into(),
			elevator: -20,
			aileron: 15,
			rudder: -5,
			aoa: "5.1".into(),
			aos: "0.2".into(),
			alt: 1500.0,
			check_alt: 0,
			ias_v: 520.0,
			sep: 49.0,
			state_wx: -140.7,
			indic_type: "bf-109e-4".into(),
		}
	}

	fn init_standard(root: &Path) -> FlightLog {
		let mut fl = FlightLog::new();
		let (notify, _) = notify_collector();
		fl.init(
			Arc::new(LogonFlag(AtomicBool::new(true))),
			&sample_snapshot(),
			Some(Arc::new(MapConfig::new())),
			notify,
			RecordingService::shared(),
		);
		assert!(root.join(&fl.file_name).is_file(), "init 应创建主 CSV");
		fl
	}

	// ---- 1. Java 8 oracle 对拍: Double.toString (javac 1.8.0_342 实测 dump) ----
	#[test]
	fn java_double_to_string_matches_java8_oracle() {
		let cases: &[(f64, &str)] = &[
			(5.0, "5.0"),
			(0.0, "0.0"),
			(-0.0, "-0.0"),
			(123.456, "123.456"),
			(0.001, "0.001"),
			(0.0001, "1.0E-4"),
			(9999999.0, "9999999.0"),
			(10000000.0, "1.0E7"),
			(-42.75, "-42.75"),
			(6.02e23, "6.02E23"),
			(3.0, "3.0"),
			(100.0, "100.0"),
			(0.05, "0.05"),
			(12345.6789, "12345.6789"),
			(0.0012, "0.0012"),
			(0.1 + 0.2, "0.30000000000000004"),
			(1.0 / 3.0, "0.3333333333333333"),
			(1234567.0, "1234567.0"),
			(12345678.0, "1.2345678E7"),
			(123456789.0, "1.23456789E8"),
			(f64::NAN, "NaN"),
			(f64::INFINITY, "Infinity"),
			(f64::NEG_INFINITY, "-Infinity"),
			// 注: 9.999999999999999e22 (=1e23) Java 8 输出 "9.999999999999999E22",
			// Rust 最短 round-trip 为 "1.0E23" — Java 8 旧 FloatingDecimal 非最短数字的
			// 已知偏差类 (见 java_double_to_string 的 PORT 注释), 遥测值域不命中, 不入表
			(1.7976931348623157E308, "1.7976931348623157E308"),
			(-1.5E-8, "-1.5E-8"),
		];
		for (v, expect) in cases {
			assert_eq!(&java_double_to_string(*v), expect, "value = {v:e}");
		}
	}

	// ---- 2. Java 8 oracle 对拍: Float.toString ----
	#[test]
	fn java_float_to_string_matches_java8_oracle() {
		let cases: &[(f32, &str)] = &[
			(0.0, "0.0"),
			(5.5, "5.5"),
			(60000.0, "60000.0"),
			(0.05, "0.05"),
			(1e7, "1.0E7"),
			(1e-4, "1.0E-4"),
			(1234567.0, "1234567.0"),
			// 注: f32 123456790 Java 8 输出 "1.23456792E8" (多一位非最短数字, 同上偏差类)
			(1.0 / 3.0, "0.33333334"),
			(9.999999, "9.999999"),
			(0.001, "0.001"),
			(0.000999, "9.99E-4"),
		];
		for (v, expect) in cases {
			assert_eq!(&java_float_to_string(*v), expect, "value = {v:e}");
		}
	}

	// ---- 3. Java 8 oracle 对拍: elapsedTime / 60000.0f (long→float 提升) ----
	#[test]
	fn elapsed_minutes_matches_java8_oracle() {
		let cases: &[(i64, &str)] = &[
			(0, "0.0"),
			(123456, "2.0576"),
			(3600000, "60.0"),
			(1234567890123, "2.0576132E7"),
			(2951479052, "49191.316"),
		];
		for (ms, expect) in cases {
			assert_eq!(&java_float_to_string(*ms as f32 / 60000.0f32), expect, "ms = {ms}");
		}
	}

	// ---- 4. 表头行格式: Lang.l1..l31 顺序拼接 + 换行 ----
	#[test]
	fn label_row_is_lang_l1_to_l31_concatenation() {
		with_temp_cwd("label", true, |root| {
			let fl = init_standard(root);
			let content = std::fs::read_to_string(&fl.file_name).unwrap();
			let lang = Lang::init_lang();
			let expected = format!(
				"{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}\n",
				lang.l1, lang.l2, lang.l3, lang.l4, lang.l5, lang.l6, lang.l7, lang.l8, lang.l9,
				lang.l10, lang.l11, lang.l12, lang.l13, lang.l14, lang.l15, lang.l16, lang.l17,
				lang.l18, lang.l19, lang.l20, lang.l21, lang.l22, lang.l23, lang.l24, lang.l25,
				lang.l26, lang.l27, lang.l28, lang.l29, lang.l30, lang.l31
			);
			assert_eq!(content, expected);
			// 钉死首尾标签 (防 Lang 字段错位接线静默通过)
			assert!(content.starts_with("时间/s,"));
			assert!(content.contains("桨距角/deg,"));
			assert!(content.contains("侧滑角/deg,"));
		});
	}

	// ---- 5. 数据行列序/分隔 (float/double 列走 Java toString, 已由 oracle 测试锁定) ----
	#[test]
	fn data_row_column_order_and_separators() {
		with_temp_cwd("datarow", true, |root| {
			let mut fl = init_standard(root);
			let snap = sample_snapshot();
			fl.log_tick(&snap);
			let content = std::fs::read_to_string(&fl.file_name).unwrap();
			let lines: Vec<&str> = content.lines().collect();
			assert_eq!(lines.len(), 2, "表头 + 一行数据");
			assert_eq!(
				lines[1],
				"1.0,50,500,520,0.78,1000,85,90,15.2,12.3,1.5,3,1200,82,1100,2800,0,0.5,100,35,0,60,1,1,1.15,0,-20,15,-5,5.1,0.2,"
			);
			assert!(content.ends_with('\n'), "行以换行结尾");
		});
	}

	// ---- 6. 分析链: 首帧 init, 其后 analyze + updateEMChart (效果面断言) ----
	#[test]
	fn analyze_flow_init_then_analyze_and_em_chart() {
		with_temp_cwd("analyze", true, |_root| {
			let mut fl = FlightLog::new();
			let (notify, _) = notify_collector();
			fl.init(
				Arc::new(LogonFlag(AtomicBool::new(true))),
				&sample_snapshot(),
				Some(Arc::new(MapConfig::new())),
				notify,
				RecordingService::shared(),
			);
			let mut snap = sample_snapshot();
			snap.alt = 1500.0; // stage = (int)1500/100 = 15
			snap.check_alt = 15; // |checkAlt| > 10 触发分析
			// 48/9.80≈4.898 < 5.0 — 过载分支的 sep<5 门槛 (49 会得整 5.0 进不去,
			// g=9.80 见 physics_constants)
			snap.sep = 48.0;
			fl.log_tick(&snap); // 首帧: init (initaltStage/curaltStage=15)
			fl.log_tick(&snap); // 其后: analyze 同级累加 + updateEMChart
			fl.log_tick(&snap);
			{
				let fa = fl.f_a.as_ref().expect("首帧 init 应落地 fA");
				// init: stage=15 落地; config 非空透传 (MapConfig 无键 → is_information=false)
				assert_eq!(fa.initalt_stage, 15);
				assert_eq!(fa.curalt_stage, 15, "同级 analyze 不推进 curaltStage");
				assert!(!fa.is_information);
				// 同级 analyze ×2 (Java else 分支): eff[15] = 1100(init) + 2×1100,
				// sep[15] = 49×3 (换级才做 /count 的均值收口)
				assert_eq!(fa.eff[15], 1100 + 2 * 1100, "init 一次 + 两次同级累加");
				assert_eq!(fa.sep[15], 49.0 * 3.0);
				// updateEMChart 参数与 Java 表达式逐一对应:
				// (xs.IASv=520 → speed stage 52, (int)|xs.sState.Wx|=140,
				//  xs.SEP/g≈4.997, |elevator|=20, |aileron|=15)
				// 滚转: abs_alr=15>5, wx=140>10, 15>=0, 140>0 → 记录
				assert_eq!(fa.roll_alr[52], 15);
				assert_eq!(fa.roll_rate[52], 140);
				// 过载: ny=1.5>1.0, SEP/g<5, elev=20>=0 → 记录 (turn_load 均值收敛)
				assert_eq!(fa.turn_elev[52], 20);
				// 两次 tick 均命中 (20 >= turn_elev 恒真) → 均值收敛叠加:
				// turn_load: (0+1.5)/2=0.75 → (0.75+1.5)/2=1.125
				assert_eq!(fa.turn_load[52], 1.5 * (0.5 + 0.25));
				assert_eq!(fa.sep_loss[52], (48.0 / g) * 0.75);
			}
			// 第四 tick 换级 (alt 1600 → stage 16): analyze 的 stage+1 分支
			snap.alt = 1600.0;
			fl.log_tick(&snap);
			let fa = fl.f_a.as_ref().unwrap();
			assert_eq!(fa.curalt_stage, 16, "stage+1 推进");
			// 换级收口: eff[15] /= count (3300/3, int/int 截断), sep[15] /= count*g
			assert_eq!(fa.eff[15], 1100);
			assert_eq!(fa.sep[15], 49.0 * 3.0 / (3.0 * g));
			// 活读证据 (集成合同): time[16] 取 analyze 时刻的 elapsed (RecordingService
			// 每读 +1000, 此刻 >0) — 快照合同会冻结为 init 时刻的 0
			assert!(
				fa.time[16] > 0.0,
				"analyze 活读 elapsed_time: time[16]={}",
				fa.time[16]
			);
			assert_eq!(fa.power[16], 1200);
			assert_eq!(fa.thrust[16], 3400);
		});
	}

	// ---- 7. checkAlt 未过阈值 ⇒ 不触发分析 ----
	#[test]
	fn analyze_skipped_when_check_alt_below_threshold() {
		with_temp_cwd("noanalyze", true, |_root| {
			let mut fl = FlightLog::new();
			let (notify, _) = notify_collector();
			fl.init(
				Arc::new(LogonFlag(AtomicBool::new(true))),
				&sample_snapshot(),
				None,
				notify,
				RecordingService::shared(),
			);
			let snap = sample_snapshot(); // check_alt = 0
			fl.log_tick(&snap);
			assert!(fl.f_a.is_none(), "|checkAlt| <= 10 不进分析分支");
		});
	}

	// ---- 8. saveClimbData: 标签 + 每百米行, append 语义 ----
	#[test]
	fn save_climb_data_label_rows_and_append() {
		with_temp_cwd("climb", true, |_root| {
			let mut fl = FlightLog::new();
			let (notify, _) = notify_collector();
			fl.init(
				Arc::new(LogonFlag(AtomicBool::new(true))),
				&sample_snapshot(),
				None,
				notify,
				RecordingService::shared(),
			);
			fl.f_a = Some(analyzer_with_data(
				2,
				padded(vec![10.0, 20.5], 0.0),
				padded(vec![100, 200], 0),
				padded(vec![300, 400], 0),
				padded(vec![10.5, 11.25], 0.0),
				vec![0; 256],
				vec![0; 256],
				vec![0.0; 256],
				vec![0; 256],
				vec![0.0; 256],
			));
			fl.save_climb_data();
			let content = std::fs::read_to_string(&fl.climb_name).unwrap();
			let lang = Lang::init_lang();
			assert_eq!(
				content,
				format!(
					"{a}/m, t/s, {p}/hp, {t}/kgf, {s}/m/s\n0, 10.0, 100, 300, 10.5\n100, 20.5, 200, 400, 11.25\n",
					a = lang.f_alt,
					p = lang.e_power,
					t = lang.e_thurst,
					s = lang.f_sep
				)
			);
			// 再存一次: FileWriter append ⇒ 内容翻倍
			fl.save_climb_data();
			let again = std::fs::read_to_string(&fl.climb_name).unwrap();
			assert_eq!(again.len(), content.len() * 2, "append 模式追加而非覆盖");
		});
	}

	// ---- 9. saveRollData / saveNyData: >0 过滤 + 10km/h 档 ----
	#[test]
	fn save_roll_and_ny_data_filter_positive() {
		with_temp_cwd("rollny", true, |_root| {
			let mut fl = FlightLog::new();
			let (notify, _) = notify_collector();
			fl.init(
				Arc::new(LogonFlag(AtomicBool::new(true))),
				&sample_snapshot(),
				None,
				notify,
				RecordingService::shared(),
			);
			let mut roll_rate = vec![0; 256];
			let mut roll_alr = vec![0; 256];
			roll_rate[1] = 5;
			roll_alr[1] = 80;
			roll_rate[3] = 7;
			roll_alr[3] = 90;
			let mut turn_load = vec![0.0; 256];
			let mut turn_elev = vec![0; 256];
			let mut sep_loss = vec![0.0; 256];
			turn_load[2] = 3.5;
			turn_elev[2] = 30;
			sep_loss[2] = 1.5;
			turn_load[5] = 6.25;
			turn_elev[5] = 50;
			sep_loss[5] = 2.25;
			fl.f_a = Some(analyzer_with_data(
				0,
				vec![0.0; 256],
				vec![0; 256],
				vec![0; 256],
				vec![0.0; 256],
				roll_rate,
				roll_alr,
				turn_load,
				turn_elev,
				sep_loss,
			));
			fl.save_roll_data();
			fl.save_ny_data();
			let lang = Lang::init_lang();
			let roll = std::fs::read_to_string(&fl.roll_name).unwrap();
			assert_eq!(
				roll,
				format!(
					"{i}/km/h, {a}/%, {w}/Deg/s\n10, 80, 5\n30, 90, 7\n",
					i = lang.f_ias,
					a = lang.v_aileron,
					w = lang.f_wx
				)
			);
			let ny = std::fs::read_to_string(&fl.load_name).unwrap();
			assert_eq!(
				ny,
				format!(
					"{i}/km/h, {e}/%, {gl}/G, {s}/m/s\n20, 30, 3.5, 1.5\n50, 50, 6.25, 2.25\n",
					i = lang.f_ias,
					e = lang.v_elevator,
					gl = lang.f_gl,
					s = lang.f_sep
				)
			);
		});
	}

	// ---- 10. 文件名: 机型大写 / NO COCKPIT→Unknown / 四文件后缀 / 12 小时制 ----
	#[test]
	fn file_names_uppercase_no_cockpit_and_suffixes() {
		with_temp_cwd("names", true, |root| {
			let fl = init_standard(root);
			assert!(fl.file_name.starts_with("records/BF-109E-4_"), "{}", fl.file_name);
			assert!(fl.climb_name.ends_with("_climb.csv"));
			assert!(fl.roll_name.ends_with("_roll.csv"));
			assert!(fl.load_name.ends_with("_ny.csv"));
			// 结构: records/<NAME>_<M>_<D>_<H>.<Min>.<S>.csv — NAME 含 '-' 不含 '_',
			// split('_') 尾三段即 M / D / H.M.S
			let stem = fl
				.file_name
				.trim_start_matches("records/")
				.trim_end_matches(".csv");
			let parts: Vec<&str> = stem.split('_').collect();
			assert!(parts.len() >= 3, "{stem}");
			let month: i64 = parts[parts.len() - 3].parse().expect("M 数字");
			let date: i64 = parts[parts.len() - 2].parse().expect("D 数字");
			assert!((1..=12).contains(&month), "MONTH+1: {stem}");
			assert!((1..=31).contains(&date), "DATE: {stem}");
			let time_part = parts[parts.len() - 1];
			let segs: Vec<i64> = time_part
				.split('.')
				.filter_map(|s| s.parse().ok())
				.collect();
			assert_eq!(segs.len(), 3, "H.M.S: {time_part}");
			assert!((0..=11).contains(&segs[0]), "Calendar.HOUR 为 12 小时制 0..11: {time_part}");
			assert!((0..=59).contains(&segs[1]) && (0..=59).contains(&segs[2]), "{time_part}");

			// NO COCKPIT → Unknown
			let mut fl2 = FlightLog::new();
			let (notify, _) = notify_collector();
			let mut snap = sample_snapshot();
			snap.indic_type = "no cockpit".into();
			fl2.init(
				Arc::new(LogonFlag(AtomicBool::new(true))),
				&snap,
				None,
				notify,
				RecordingService::shared(),
			);
			assert!(fl2.file_name.starts_with("records/Unknown_"), "{}", fl2.file_name);
		});
	}

	// ---- 11. records/ 缺失: 三段失败路径全走 notify+warn, xc.logon=false, 不 panic ----
	#[test]
	fn init_without_records_dir_degrades_like_java() {
		with_temp_cwd("norecords", false, |root| {
			assert!(!root.join("records").exists());
			let logon_flag = Arc::new(LogonFlag(AtomicBool::new(true)));
			let (notify, seen) = notify_collector();
			let mut fl = FlightLog::new();
			fl.init(
				logon_flag.clone(),
				&sample_snapshot(),
				None,
				notify,
				RecordingService::shared(),
			);
			// Java: FileOutputStream 失败 → notify + warn + xc.logon=false;
			//       FileWriter 失败 → notify + warn; writeLabel → IOException("Stream closed") → warn
			assert!(!logon_flag.0.load(Ordering::SeqCst), "xc.logon = false 已透传");
			let texts = seen.lock().unwrap().clone();
			assert_eq!(texts.len(), 2, "两条 lfailCreate 通知: {texts:?}");
			assert!(texts.iter().all(|t| t == "记录文件创建失败"));
			assert!(fl.csv.is_none() && fl.csv_writter.is_none() && fl.results_file.is_none());
			assert!(fl.logon.load(Ordering::SeqCst), "Java init 尾部无条件 logon=true");
			// logTick: null writer → IOException("Stream closed") → lfailWrite 通知
			fl.log_tick(&sample_snapshot());
			let texts = seen.lock().unwrap().clone();
			assert_eq!(texts.len(), 3, "{texts:?}");
			assert_eq!(texts[2], "记录文件写入失败");
			// 此处刻意不 close(): fA==null 时 save*Data 走 Java NPE 路径 (下一条测试)
		});
	}

	// ---- 12. fA==null 时 close() 复刻 Java NPE (逃逸 save*Data 的 IOException catch) ----
	#[test]
	#[should_panic(expected = "fA == null")]
	fn close_without_analyzer_panics_like_java_npe() {
		with_temp_cwd("npe", true, |root| {
			let mut fl = init_standard(root);
			let snap = sample_snapshot(); // check_alt=0 ⇒ fA 保持 null
			fl.log_tick(&snap);
			fl.close(); // saveClimbData → writeClimbData → fA.curaltStage NPE
		});
	}

	// ---- 12b. csv==null (records/ 缺失) 时 close() 复刻 Java NPE 于 csv.close() ----
	#[test]
	#[should_panic(expected = "csv == null")]
	fn close_without_csv_panics_like_java_npe() {
		with_temp_cwd("npe2", false, |_root| {
			let mut fl = FlightLog::new();
			let (notify, _) = notify_collector();
			fl.init(
				Arc::new(LogonFlag(AtomicBool::new(true))),
				&sample_snapshot(),
				None,
				notify,
				RecordingService::shared(),
			);
			fl.close(); // csvWritter.close() 静默 (out==null), csv.close() 抛 NPE
		});
	}

	// ---- 13. 缓冲语义: 非 1024 倍数 tick 不落盘, close() 兜底 flush ----
	#[test]
	fn buffered_rows_flush_on_close() {
		with_temp_cwd("flush", true, |root| {
			let mut fl = init_standard(root);
			fl.write_time = 1; // 跳过 t%1024==0 的 tick 内 flush (白盒: 私有字段)
			fl.log_tick(&sample_snapshot());
			let content = std::fs::read_to_string(&fl.file_name).unwrap();
			assert_eq!(content.lines().count(), 1, "数据仍在 BufferedWriter 内存");
			let mut fl2 = init_standard(root); // 新文件名 (秒级时间戳)
			fl2.write_time = 1;
			fl2.log_tick(&sample_snapshot());
			// close() 内 save*Data 需要 fA (fA==null 是 Java NPE 路径, 由测试 12 单独锁定),
			// 此处给空数据 mock 以聚焦缓冲 flush 语义
			fl2.f_a = Some(analyzer_with_data(
				0,
				vec![0.0; 256],
				vec![0; 256],
				vec![0; 256],
				vec![0.0; 256],
				vec![0; 256],
				vec![0; 256],
				vec![0.0; 256],
				vec![0; 256],
				vec![0.0; 256],
			));
			fl2.close();
			let content2 = std::fs::read_to_string(&fl2.file_name).unwrap();
			assert_eq!(content2.lines().count(), 2, "close() 兜底 flush (首帧 t=0 已 flush 的表头 + 本行)");
		});
	}

	// ---- 14. run(): doit 单发单写, logon=false 退出 (Java 死线程的保真翻译) ----
	#[test]
	fn run_thread_writes_once_per_doit_and_exits_on_logon_false() {
		with_temp_cwd("run", true, |root| {
			let mut fl = FlightLog::new();
			let (notify, _) = notify_collector();
			fl.init(
				Arc::new(LogonFlag(AtomicBool::new(true))),
				&sample_snapshot(),
				None,
				notify,
				RecordingService::shared(),
			);
			let doit = fl.doit.clone();
			let logon = fl.logon.clone();
			let path = root.join(&fl.file_name);
			let snapshot = Arc::new(sample_snapshot());
			// 'static 快照源 (run 的 xs 直读 → 闭包代餐, 见 run 的 PORT 注释)
			let src: &'static (dyn Fn() -> FlightLogSnapshot + Sync) =
				Box::leak(Box::new(move || (*snapshot).clone()));
			let (tx, rx) = std::sync::mpsc::channel::<()>();
			let mut fl = fl;
			let handle = std::thread::spawn(move || {
				fl.run(src);
				tx.send(()).unwrap();
			});
			doit.store(true, Ordering::SeqCst);
			let mut rows = 0usize;
			for _ in 0..400 {
				std::thread::sleep(Duration::from_millis(5));
				if let Ok(c) = std::fs::read_to_string(&path) {
					rows = c.lines().count();
					if rows >= 2 {
						break;
					}
				}
			}
			assert!(rows >= 2, "run 线程消费一次 doit 并写入数据行");
			std::thread::sleep(Duration::from_millis(30));
			assert!(!doit.load(Ordering::SeqCst), "写完后 doit 复位 (写完后关闭)");
			let after = std::fs::read_to_string(&path).unwrap().lines().count();
			assert_eq!(after, rows, "doit 未再置位 ⇒ 不再追加行");
			logon.store(false, Ordering::SeqCst);
			assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok(), "logon=false 后 run 退出");
			handle.join().unwrap();
		});
	}
}
