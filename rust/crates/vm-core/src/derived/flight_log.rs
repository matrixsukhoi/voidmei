//! 飞行数据 CSV 日志记录 (文件 IO + 记录)。
//!
//! 每次 logTick 由 Service 轮询线程驱动, 写一行遥测快照;
//! 换机/退出时 close() 落盘三份分析 CSV (climb/roll/ny)。
//!
//! 依赖注入面 (Service/Controller 落 vm-data, 本 crate 不可见):
//! - Service → 每次调用注入快照 [`FlightLogSnapshot`] (Service 轮询线程实时读字段的
//!   该时刻值); vm-data 侧的快照构造面 = `vm_data::service_loop::flight_log_snapshot`;
//! - FlightAnalyzer → 直接持有同 crate 具体类型 [`FlightAnalyzer`] (pub 字段面 +
//!   pub(crate) 方法), 并注入 `Arc<dyn AnalyzerService>` 令 analyze 活读 Service
//!   字段 (快照会把 time[]/eff/sep 冻结在 init 时刻, 见彼处论证);
//! - Controller → [`ControllerLogSink`] 最小接口 (唯一用途 `xc.logon = false`);
//! - 通知出口 → [`NotifySink`] 回调注入 (FlightAnalyzer.notify 同类型, 共用同一 sink)。
//!
//! 行格式保真: CSV 数值列的文本 = Java `String.concat` 的隐式 toString
//! (Double.toString / Float.toString / Integer.toString), 见 [`java_double_to_string`]。

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{Datelike, Local, Timelike};

use crate::base::java_compat::java_double_to_string;
use crate::base::logger::warn_default;
use crate::base::physics_constants::g;
use crate::config::config_api::ConfigProvider;
use crate::derived::flight_analyzer::{AnalyzerService, FlightAnalyzer, MAX_IAS_STAGE};
use crate::lang::Lang;

// ============================================================================
// 依赖面 (crate 边界注入, 见模块头)
// ============================================================================

/// 通知回调注入面。参数即通知文本 (Lang.xxx)。
pub type NotifySink = Arc<dyn Fn(&str) + Send + Sync>;

/// FlightLog 对 Controller 的依赖面: 唯一用途是 init 失败路径的 `xc.logon = false`。
/// Rust 侧实现应为原子写 (跨线程)。
pub trait ControllerLogSink: Send + Sync {
    /// 写日志开关位
    fn set_logon(&self, logon: bool);
}

/// FlightLog 的跨线程共享槽: Service 轮询线程每轮 `log_tick()`,
/// 主线程 init/close 换入换出。
/// 外层 Mutex 保护 Some/None 切换, 内层 Mutex 串行化
/// tick/close 对实例的独占访问; 两侧均**不嵌套持锁** (先取 Arc 后释放外层), 无死锁。
pub type FlightLogSlot = Arc<Mutex<Option<Arc<Mutex<FlightLog>>>>>;

/// logTick 时刻 Service 字段的快照 (Service 落 vm-data, 本 crate 以值注入)。
#[derive(Debug, Clone, Default)]
pub struct FlightLogSnapshot {
    // ---- writeData 使用 (writeData 各列) ----
    pub elapsed_time: i64,
    pub throttle: String,
    pub ias: String,
    pub tas: String,
    /// 马赫数显示串
    pub mach: String,
    /// 高度显示串 (字段名沿 Java 原名 salt)
    pub salt: String,
    pub watertemp: String,
    pub oiltemp: String,
    pub vy: String,
    /// SEP 显示串
    pub s_sep: String,
    pub ny: f64,
    /// 滚转率显示串
    pub wx: String,
    pub total_hp_str: String,
    /// 效率串 (Java efficiency[] 的首元素)
    pub efficiency_0: String,
    pub total_hp_eff_str: String,
    pub rpm: String,
    pub total_thrust: i32,
    pub acceleration: f64,
    pub rpm_throttle: String,
    /// 螺距串 (Java pitch[] 的首元素)
    pub pitch_0: String,
    pub radiator: String,
    pub mixture: String,
    pub compressorstage: i32,
    /// 磁电机开关 (字段名沿 Java 原文拼写 magneto, 非 magneto)
    pub magneto: i32,
    pub manifoldpressure: String,
    pub flaps: String,
    pub elevator: i32,
    pub aileron: i32,
    pub rudder: i32,
    /// 攻角显示串
    pub aoa: String,
    /// 侧滑角显示串
    pub aos: String,
    // ---- analyzeData 使用 ----
    pub alt: f64,
    /// 高度检查计数 (启动稳定判定)
    pub check_alt: i32,
    pub ias_v: f64,
    pub sep: f64,
    /// 滚转率数值
    pub state_wx: f64,
    // ---- init 使用 ----
    /// 载具机型名
    pub indic_type: String,
}

/// "Stream closed" 错误的复刻: writer 句柄缺失时惰性, 首次 write/flush 报
/// "Stream closed" IO 错误 (而非 panic) — init 文件打开失败路径依赖此行为。
fn stream_closed() -> io::Error {
    io::Error::other("Stream closed")
}

// ============================================================================
// FlightLog
// ============================================================================

/// 飞行日志记录器
pub struct FlightLog {
    /// 写触发 (Controller.writeDown 置 true)。跨线程共享 → AtomicBool。
    pub doit: Arc<AtomicBool>,
    /// 生命周期开关 (close() 置 false 通知停写)
    pub logon: Arc<AtomicBool>,
    /// 创建/截断目标文件后即被弃置的句柄 (对齐 Java 从不 close 的行为;
    /// Rust 侧 Drop 时关闭)。
    pub results_file: Option<File>,
    /// append 句柄 (writeLabel 经它落盘)。
    /// 与 csv_writter 是同路径的两个独立句柄 (Java 侧两 writer 包裹同一对象,
    /// append 模式下字节流等价; 标签先行 flush 后数据句柄才建, 顺序保持)。
    pub csv: Option<File>,
    pub file_name: String,
    /// 分析器 (直接持有具体类型, 见模块头)
    pub f_a: Option<FlightAnalyzer>,
    /// Controller 最小接口注入 (唯一用途 `xc.logon = false`)
    xc: Option<Arc<dyn ControllerLogSink>>,
    // Service 字段不落地 — 快照按调用注入 (见模块头);
    // 时间戳仅 init 用于文件名 (chrono::Local)。
    /// 首帧分析标志 (true = 尚未构造 FlightAnalyzer)
    first_analyze: bool,
    climb_name: String,
    /// 死字段: 从未赋值/读取, 有意保真保留
    #[allow(dead_code)]
    maneuver_name: String,
    roll_name: String,
    load_name: String,
    /// 数据行 writer。None 表示 init 文件打开失败态 (write/flush 报
    /// "Stream closed", 见 stream_closed)。
    csv_writter: Option<BufWriter<File>>,
    write_time: i64,
    config: Option<Arc<dyn ConfigProvider + Send + Sync>>,
    /// init 时取 `Lang::init_lang()` 当前值 (Java 为启动即装载的静态字段)。
    lang: Lang,
    /// 通知注入回调 (None = 未注入, 静默)
    notify: Option<NotifySink>,
    /// FlightAnalyzer 依赖的 Service 活读面 (analyze 每次调用活读 7 字段,
    /// 故注入 Arc 而非快照 — 见模块头)
    analyze_service: Option<Arc<dyn AnalyzerService + Send + Sync>>,
}

/// 对应 `new FlightLog()` — 引用 None / boolean false / long 0 / first_analyze=true
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

    /// 通知回调便捷调用
    fn notify_show(&self, text: &str) {
        if let Some(n) = &self.notify {
            n(text);
        }
    }

    /// 写 CSV 表头行
    fn write_label(txt: Option<&mut File>, lang: &Lang) -> io::Result<()> {
        let Some(txt) = txt else {
            // → "Stream closed" — init 文件打开失败路径即走这里
            return Err(stream_closed());
        };
        let mut bw = BufWriter::new(txt);
        // l1..l31 按序写出 (顺序即 CSV 列序; 平铺收敛为数组循环, 字节流逐次等价)
        let labels = [
            &lang.l1, &lang.l2, &lang.l3, &lang.l4, &lang.l5, &lang.l6, &lang.l7, &lang.l8,
            &lang.l9, &lang.l10, &lang.l11, &lang.l12, &lang.l13, &lang.l14, &lang.l15, &lang.l16,
            &lang.l17, &lang.l18, &lang.l19, &lang.l20, &lang.l21, &lang.l22, &lang.l23, &lang.l24,
            &lang.l25, &lang.l26, &lang.l27, &lang.l28, &lang.l29, &lang.l30, &lang.l31,
        ];
        for label in labels {
            bw.write_all(label.as_bytes())?;
        }

        bw.write_all(
            b"
",
        )?;
        bw.flush()?;
        Ok(())
    }

    /// 写一行遥测数据 (尾注 // N 为 CSV 列序)
    #[allow(unused_assignments)]
    fn write_data(bw: Option<&mut BufWriter<File>>, xs: &FlightLogSnapshot) -> io::Result<()> {
        let Some(bw) = bw else {
            return Err(stream_closed());
        };
        let mut tmp = String::new();

        write!(
            bw,
            "{},",
            java_double_to_string(xs.elapsed_time as f64 / 60000.0) // 波21: f32 复刻退役并归 double 版
        )?; // 1

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

        write!(bw, "{},", xs.magneto)?; // 23

        write!(bw, "{},", xs.manifoldpressure)?; // 24

        write!(bw, "{},", xs.flaps)?; // 25

        write!(bw, "{},", xs.elevator)?; // 26

        write!(bw, "{},", xs.aileron)?; // 27

        write!(bw, "{},", xs.rudder)?; // 28

        write!(bw, "{},", xs.aoa)?; // 29

        write!(bw, "{},", xs.aos)?; // 30

        bw.write_all(b"\n")?;
        Ok(())
    }

    // 进行数据分析
    fn analyze_data(&mut self, xs: &FlightLogSnapshot) {
        let stage = (xs.alt as i32) / 100;
        if xs.check_alt.wrapping_abs() > 10 {
            if self.first_analyze {
                // 第一次分析，先取当前高度
                // notify 与 FlightLog 共用同一 sink
                let mut fa = FlightAnalyzer::default();
                fa.notify = self.notify.clone();
                let src = self.analyze_service.clone().expect(
                    "PORT: 注入缺失 — init 需要 AnalyzerService (Java: Log.init 传 Service xs)",
                );
                fa.init(stage, src, self.config.clone());
                self.f_a = Some(fa);
                self.first_analyze = false;
            } else {
                // 开始分析
                let fa = self
                    .f_a
                    .as_mut()
                    .expect("PORT: Java 不变量 — firstAnalyze==false ⇒ fA!=null");
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

    /// 写爬升 CSV 表头
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
        Ok(())
    }

    /// 写爬升 CSV 数据行
    fn write_climb_data(txt: &mut File, f_a: Option<&FlightAnalyzer>) -> io::Result<()> {
        let mut bw = BufWriter::new(txt);
        // fA == null (全程未触发高度分析) 时 Java 抛 NPE 逃逸 catch → 此处 panic 对应
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
        }
        bw.flush()?;
        Ok(())
    }

    /// save_climb_data/save_roll_data/save_ny_data 三胞胎的共用骨架:
    /// 打开 CSV → 写表头 → 写数据; 失败路径逐一对齐 (创建失败 notify+warn+return,
    /// 写失败按 warn_tag 区分)。
    fn save_analysis_csv(
        &mut self,
        path: &str,
        label_fn: fn(&mut File, &Lang) -> io::Result<()>,
        data_fn: fn(&mut File, Option<&FlightAnalyzer>) -> io::Result<()>,
        warn_tag: &str,
    ) {
        let mut tcsv = match OpenOptions::new().append(true).create(true).open(path) {
            Ok(f) => f,
            Err(e) => {
                self.notify_show(self.lang.lfail_create);
                warn_default(&format!("文件创建失败: {e}"));
                return;
            }
        };
        // Application.debugPrint("打开文件成功");

        let res = (|| {
            label_fn(&mut tcsv, &self.lang)?;
            data_fn(&mut tcsv, self.f_a.as_ref())?;
            // tcsv.close() — 句柄离开作用域即关闭
            Ok::<(), io::Error>(())
        })();
        if let Err(e) = res {
            warn_default(&format!("写入{}数据失败: {e}", warn_tag));
        }
    }

    /// 对应 Java: `public void saveClimbData()`
    pub fn save_climb_data(&mut self) {
        // Application.debugPrint("climbdata save to "+ climbName);
        // 借用拆分: 路径先克隆, 供 &mut self 的共用骨架打开
        let path = self.climb_name.clone();
        self.save_analysis_csv(
            &path,
            FlightLog::write_climb_label,
            FlightLog::write_climb_data,
            "爬升",
        );
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
        let path = self.roll_name.clone();
        self.save_analysis_csv(
            &path,
            FlightLog::write_roll_label,
            FlightLog::write_roll_data,
            "滚转",
        );
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
        let path = self.load_name.clone();
        self.save_analysis_csv(
            &path,
            FlightLog::write_ny_label,
            FlightLog::write_ny_data,
            "过载",
        );
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
        self.climb_name =
            format!("records/{name}_{month}_{date}_{hour}.{minute}.{second}_climb.csv");
        self.roll_name = format!("records/{name}_{month}_{date}_{hour}.{minute}.{second}_roll.csv");
        self.load_name = format!("records/{name}_{month}_{date}_{hour}.{minute}.{second}_ny.csv");

        self.results_file = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.file_name)
        {
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
        self.csv = match OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.file_name)
        {
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
        self.csv_writter = match OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.file_name)
        {
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

    /// 对应 Java: `public void logTick()` (Service 轮询线程直调)
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
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests;
