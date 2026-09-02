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

use crate::config::config_api::ConfigProvider;
use crate::derived::flight_analyzer::{AnalyzerService, FlightAnalyzer, MAX_IAS_STAGE};
use crate::base::physics_constants::g;
use crate::lang::Lang;
use crate::base::logger::warn_default;

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
	#[allow(unused_assignments)]
	fn write_data(bw: Option<&mut BufWriter<File>>, xs: &FlightLogSnapshot) -> io::Result<()> {
		let Some(bw) = bw else {
			return Err(stream_closed());
		};
		let mut tmp = String::new();

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
		let stage = (xs.alt as i32) / 100;
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
				let fa = self.f_a.as_mut().expect("PORT: Java 不变量 — firstAnalyze==false ⇒ fA!=null");
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
		// Application.debugPrint("climbdata save to "+ climbName);
		let mut tcsv = match OpenOptions::new().append(true).create(true).open(&self.climb_name) {
			Ok(f) => f,
			Err(e) => {
				self.notify_show(self.lang.lfail_create);
				warn_default(&format!("文件创建失败: {e}"));
				return;
			}
		};
		// Application.debugPrint("打开文件成功");

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
	#[allow(unused_assignments, unused_variables)]
	fn write_roll_data(txt: &mut File, f_a: Option<&FlightAnalyzer>) -> io::Result<()> {
		let mut bw = BufWriter::new(txt);
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
	#[allow(unused_assignments, unused_variables)]
	fn write_ny_data(txt: &mut File, f_a: Option<&FlightAnalyzer>) -> io::Result<()> {
		let mut bw = BufWriter::new(txt);
		let f_a = f_a.unwrap_or_else(|| {
			panic!("PORT: Java NPE — fA == null (全程未触发高度分析) 于 fA.turn_load 抛 NullPointerException")
		});
		let mut k = 0;
		for i in 0..MAX_IAS_STAGE {
			// 速度区间
			let i = i as usize;
			if f_a.turn_load[i] > 0.0 {
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
		self.config = config;
		self.notify = Some(notify);
		self.analyze_service = Some(analyze_service);
		self.doit.store(false, Ordering::SeqCst);
		// Application.debugPrint("flightlog初始化了");
		// — 时间仅用于文件名。PORT: Calendar.HOUR 是 12 小时制 (0..11, 正午/午夜为 0),
		// 原代码刻意/无意未用 HOUR_OF_DAY — 保真取 hour % 12; 时区 = 本地 (chrono::Local)。
		self.lang = Lang::init_lang();
		let now = Local::now();
		let month = now.month() as i64;
		let date = now.day() as i64;
		let hour = (now.hour() % 12) as i64;
		let minute = now.minute() as i64;
		let second = now.second() as i64;
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

		self.results_file = match OpenOptions::new().write(true).create(true).truncate(true).open(&self.file_name) {
			Ok(f) => Some(f),
			Err(e) => {
				self.notify_show(self.lang.lfail_create);
				warn_default(&format!("日志文件创建失败: {e}"));
				if let Some(xc) = &self.xc {
					xc.set_logon(false);
				}
				None
			}
		};
		// Application.debugPrint("文件创建成功");
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
		self.analyze_data(xs);
		if let Err(e) = FlightLog::write_data(self.csv_writter.as_mut(), xs) {
			self.notify_show(self.lang.lfail_write);
			warn_default(&format!("写入日志数据失败: {e}"));
		}
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
	/// 运行极性版 sleep_while_run(&logon) 等价复刻 (§2.13: logon 是 true=运行,
	/// 直传 stop 语义的 sleep_quietly 会立即返回 → 热自旋; 备案收口修复)。
	pub fn run(&mut self, xs_source: &(dyn Fn() -> FlightLogSnapshot + Sync)) {
		while self.logon.load(Ordering::SeqCst) {
			crate::base::exception_helper::sleep_while_run(&self.logon, 5);
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
mod tests;
