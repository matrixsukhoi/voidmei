use super::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicI64;
use std::sync::Mutex;

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
    let _fm_guard = crate::fm::test_support::data_root();
    // 应 panic 测试经 catch_unwind 转发 ⇒ 锁可能被毒化, into_inner 容错
    let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let root: PathBuf =
        std::env::temp_dir().join(format!("vm_flight_log_{tag}_{}", std::process::id()));
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
        MapConfig {
            values: Mutex::new(HashMap::new()),
        }
    }
}
impl ConfigProvider for MapConfig {
    fn get_config(&self, key: &str) -> Option<String> {
        self.values.lock().unwrap().get(key).cloned()
    }
    fn set_config(&self, key: &str, value: &str) {
        self.values
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
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
        Arc::new(RecordingService {
            elapsed: AtomicI64::new(0),
        })
    }
}
impl AnalyzerService for RecordingService {
    fn s_indic_type(&self) -> Option<String> {
        Some("rec-type".into())
    }
    fn eng_type(&self) -> crate::base::engine_type::EngineType {
        crate::base::engine_type::EngineType::Turboprop
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
    (
        Arc::new(move |text: &str| s.lock().unwrap().push(text.to_string())),
        seen,
    )
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
        magneto: 1,
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
        assert_eq!(
            &java_float_to_string(*ms as f32 / 60000.0f32),
            expect,
            "ms = {ms}"
        );
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
            lang.l1,
            lang.l2,
            lang.l3,
            lang.l4,
            lang.l5,
            lang.l6,
            lang.l7,
            lang.l8,
            lang.l9,
            lang.l10,
            lang.l11,
            lang.l12,
            lang.l13,
            lang.l14,
            lang.l15,
            lang.l16,
            lang.l17,
            lang.l18,
            lang.l19,
            lang.l20,
            lang.l21,
            lang.l22,
            lang.l23,
            lang.l24,
            lang.l25,
            lang.l26,
            lang.l27,
            lang.l28,
            lang.l29,
            lang.l30,
            lang.l31
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
        assert!(
            fl.file_name.starts_with("records/BF-109E-4_"),
            "{}",
            fl.file_name
        );
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
        assert!(
            (0..=11).contains(&segs[0]),
            "Calendar.HOUR 为 12 小时制 0..11: {time_part}"
        );
        assert!(
            (0..=59).contains(&segs[1]) && (0..=59).contains(&segs[2]),
            "{time_part}"
        );

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
        assert!(
            fl2.file_name.starts_with("records/Unknown_"),
            "{}",
            fl2.file_name
        );
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
        //       FileWriter 失败 → notify + warn; writeLabel → IOException("Stream closed") → warn
        assert!(
            !logon_flag.0.load(Ordering::SeqCst),
            "xc.logon = false 已透传"
        );
        let texts = seen.lock().unwrap().clone();
        assert_eq!(texts.len(), 2, "两条 lfailCreate 通知: {texts:?}");
        assert!(texts.iter().all(|t| t == "记录文件创建失败"));
        assert!(fl.csv.is_none() && fl.csv_writter.is_none() && fl.results_file.is_none());
        assert!(
            fl.logon.load(Ordering::SeqCst),
            "Java init 尾部无条件 logon=true"
        );
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
        assert_eq!(
            content2.lines().count(),
            2,
            "close() 兜底 flush (首帧 t=0 已 flush 的表头 + 本行)"
        );
    });
}
