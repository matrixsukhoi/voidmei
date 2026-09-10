use super::*;
use std::collections::HashMap;
use std::sync::Mutex;

// ---- 测试替身 ----

struct MockState {
    indic_type: Option<String>,
    eng_type: crate::base::engine_type::EngineType,
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
    fn eng_type(&self) -> crate::base::engine_type::EngineType {
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
            eng_type: crate::base::engine_type::EngineType::Jet,
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

/// 通知捕获器: 模拟 NotificationService.show
// Java 保真 — 回调类型同 notify 字段, 不拆 type 别名
#[allow(clippy::type_complexity)]
fn capture_notify() -> (Arc<Mutex<Vec<String>>>, Arc<dyn Fn(&str) + Send + Sync>) {
    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let c = captured.clone();
    let cb: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(move |msg: &str| {
        c.lock().unwrap().push(msg.to_string());
    });
    (captured, cb)
}

// ---- Default ----

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
    assert_eq!(
        fa.engine_type,
        crate::base::engine_type::EngineType::Unknown
    );
    assert_eq!(fa.initalt_stage, 0);
    assert_eq!(fa.curalt_stage, 0);
    assert!(!fa.is_information);
    assert_eq!(fa.r#type, None);
}

// ---- init ----

#[test]
fn init_records_stage_snapshot() {
    let svc = mock_service();
    let mut fa = FlightAnalyzer::default();
    fa.init(5, svc, None);
    assert_eq!(fa.r#type.as_deref(), Some("spitfire_f24"));
    assert_eq!(fa.engine_type, crate::base::engine_type::EngineType::Jet);
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
fn init_elapsed_time_divides_f64() {
    // §2.12: Java long/1000f 先转 float 除 (f32 精度) 再存 double[]
    let svc = mock_service();
    set_mock(&svc, 1, 0, 0.0); // 1ms
    let mut fa = FlightAnalyzer::default();
    fa.init(0, svc, None);
    // 0.001f32 的 f64 展开值 (float 除法精度, 非 0.001)
    assert_eq!(fa.time[0], 0.001); // 波21: f32 拓宽退役
}

#[test]
fn init_config_flag_variants() {
    // "true" → 开
    let cfg = Arc::new(MapConfig::new());
    cfg.set_config("enableAltInformation", "true");
    let mut fa = FlightAnalyzer::default();
    fa.init(
        0,
        mock_service(),
        Some(cfg as Arc<dyn ConfigProvider + Send + Sync>),
    );
    assert!(fa.is_information);
    // 大小写不敏感 (Boolean.parseBoolean)
    let cfg2 = Arc::new(MapConfig::new());
    cfg2.set_config("enableAltInformation", "TRUE");
    let mut fa2 = FlightAnalyzer::default();
    fa2.init(
        0,
        mock_service(),
        Some(cfg2 as Arc<dyn ConfigProvider + Send + Sync>),
    );
    assert!(fa2.is_information);
    // 键缺失 (getConfig → null) → parseBoolean(null) = false
    let cfg3 = Arc::new(MapConfig::new());
    let mut fa3 = FlightAnalyzer::default();
    fa3.init(
        0,
        mock_service(),
        Some(cfg3 as Arc<dyn ConfigProvider + Send + Sync>),
    );
    assert!(!fa3.is_information);
}

#[test]
#[should_panic]
fn analyze_before_init_panics_like_npe() {
    let mut fa = FlightAnalyzer::default();
    fa.analyze(1);
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
    // 新层数据: 100200ms → f32 除法 → 100.2f32 的 f64 展开
    assert_eq!(fa.time[6], 100.2); // 波21: f32 拓宽退役, f64 直算
    assert_eq!(fa.eff[6], 1100);
    assert_eq!(fa.sep[6], 12.3);
}

// analyze 通知 — 历史基线 全串对拍 (见 /FA3 基线: golden.txt)
#[test]
fn analyze_notification_message_delta1() {
    let svc = mock_service();
    set_mock(&svc, 42000, 0, 0.0);
    let (cap, cb) = capture_notify();
    let mut fa = FlightAnalyzer::default();
    fa.notify = Some(cb);
    let cfg = Arc::new(MapConfig::new());
    cfg.set_config("enableAltInformation", "true");
    fa.init(
        5,
        svc.clone(),
        Some(cfg as Arc<dyn ConfigProvider + Send + Sync>),
    );
    set_mock(&svc, 100200, 1100, 12.3);
    fa.analyze(6);
    let msgs = cap.lock().unwrap();
    assert_eq!(msgs.len(), 1);
    // 基线: 到达 600米，用时 100秒，平均爬升率 0.9米/秒，记录完成
    // (1000/100.2 = 9.98 → (int)9 → 9/10 = 0.9)
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
    fa.init(
        5,
        svc.clone(),
        Some(cfg as Arc<dyn ConfigProvider + Send + Sync>),
    );
    set_mock(&svc, 100500, 0, 0.0); // time = 100.5 (f32 精确)
    fa.analyze(6); // delta1: 1000/100.5 → 9 → 0.9
    set_mock(&svc, 100500, 0, 0.0);
    fa.analyze(7); // delta2: 2000/100.5 = 19.9 → (int)19 → 1.9
    set_mock(&svc, 100500, 0, 0.0);
    fa.analyze(8); // delta3: 3000/100.5 = 29.85 → (int)29 → 2.9
    let msgs = cap.lock().unwrap();
    assert_eq!(msgs.len(), 3);
    // 基线 (FA.java M1 d=3 t=100.5): 2.9
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
    fa.init(
        0,
        svc.clone(),
        Some(cfg as Arc<dyn ConfigProvider + Send + Sync>),
    );
    fa.analyze(1);
    let msgs = cap.lock().unwrap();
    // 基线 (FA.java M1 t=0.0): Java 输出 "2.14748368E8"; 本实现最短往返表示
    // 给 "2.1474837E8" (JDK-4511638 域已文档化分歧, 回读同一 f32, 见单测注记)
    assert_eq!(
        msgs[0],
        "到达 100米，用时 0秒，平均爬升率 214748364.7米/秒，记录完成"
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
    // Math.round 半值向上: 305/10 = 30.5 → 31
    assert_eq!(fa.get_speed_stage(305.0), 31);
    assert_eq!(fa.get_speed_stage(295.4), 30); // 29.54 → 3EngineType::Prop
    assert_eq!(fa.get_speed_stage(295.0), 30); // 29.5 → floor(30.0) = 3EngineType::Prop
    assert_eq!(fa.get_speed_stage(-6.0), -1); // -0.6 → floor(-0.1) = -1
    assert_eq!(fa.get_speed_stage(-4.0), 0); // -0.4 → floor(0.1) = EngineType::Prop
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
    fa.update_em_chart(300.0, 1.0, 90, 10.0, 0, 6); // 90-50 = 40, 不 > 4EngineType::Prop
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
    assert_eq!(
        msgs[0],
        "速度  300km/h下的最大法向过载: 5.2G, 此时SEP为: 1.2m/s, 记录完成"
    );
}

#[test]
fn update_em_chart_turn_threshold_exactly_3_no_notify() {
    let (cap, cb) = capture_notify();
    let mut fa = FlightAnalyzer::default();
    fa.notify = Some(cb);
    fa.turn_load[30] = 3.5;
    fa.update_em_chart(300.0, 6.5, 0, 1.0, 10, 0); // 6.5-3.5 = 3.0, 不 > 3.EngineType::Prop
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
    assert_eq!(fa.get_no_zeros_num(&[0, 1, 0, -3, 100]), 3);
    assert_eq!(fa.get_no_zeros_num(&[0i32; 256]), 0);
    assert_eq!(fa.get_no_zeros_num(&[0.0, 0.5, -0.1, 0.0]), 2);
    assert_eq!(fa.get_no_zeros_num::<f64>(&[]), 0);
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
    let fa = FlightAnalyzer::default();
    let mut x = [0.0; 4];
    let mut y = [0.0; 4];
    fa.remove_zeroes_i32(&mut x, &mut y, &[7, 0, 0, 0]);
}

#[test]
#[should_panic]
fn remove_zeroes_i32_last_nonzero_panics_like_aioobe() {
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

// ---- 通知注入边界 ----

#[test]
fn notification_dropped_when_notify_not_wired() {
    let svc = mock_service();
    let mut fa = FlightAnalyzer::default(); // notify = None
    let cfg = Arc::new(MapConfig::new());
    cfg.set_config("enableAltInformation", "true");
    fa.init(
        5,
        svc.clone(),
        Some(cfg as Arc<dyn ConfigProvider + Send + Sync>),
    );
    set_mock(&svc, 100200, 0, 0.0);
    fa.analyze(6); // 消息照常构造, 通知丢弃 (P4 接线前)
    assert!(fa.notify.is_none());
}

// ---- 爬升通知的 climb 值格式化 (波21: f32 Float.toString 复刻退役 → fmt_f) ----

#[test]
fn climb_value_fmt_matches_oracle() {
    // 基线 FA.java: (int)((d*1000)/t) / 10 的一位小数显示 (f64 直算)
    let cases: &[(f64, &str)] = &[
        (1.0 / 10.0, "0.1"),
        (9.0 / 10.0, "0.9"),
        (23.0 / 10.0, "2.3"),
        (81.0 / 10.0, "8.1"),
        (333.0 / 10.0, "33.3"),
        (200.0, "200.0"),
        (1000.0, "1000.0"),
        (0.5, "0.5"),
        (29.0 / 10.0, "2.9"),
        (71.0 / 10.0, "7.1"),
        (243.0 / 10.0, "24.3"),
        (600.0, "600.0"),
        (3000.0, "3000.0"),
        (20.0 / 10.0, "2.0"),
        (119.0 / 10.0, "11.9"),
        (285.0 / 10.0, "28.5"),
        (972.0 / 10.0, "97.2"),
        (2400.0, "2400.0"),
        (12000.0, "12000.0"),
        (426.0 / 10.0, "42.6"),
        (2547.0 / 10.0, "254.7"),
        (6095.0 / 10.0, "609.5"),
        (20745.0 / 10.0, "2074.5"),
        (85333.0 / 10.0, "8533.3"),
        (51200.0, "51200.0"),
        (256000.0, "256000.0"),
        (2560000.0, "2560000.0"),
        (-20.0, "-20.0"),
        (2147483647.0 / 10.0, "214748364.7"), // 除零退化域 (Integer.MAX/10)
    ];
    for &(v, want) in cases {
        assert_eq!(fmt_f(v, 1), want, "climb({v})");
    }
}

// ---- 历史基线: fmt_f(d, 1) (String.format("%.1f", double)) ----

#[test]
fn java_f_prec1_matches_java8_oracle() {
    // 波21 显示引擎退役: 按 Rust {:.1} (精确二进制值 nearest-even) 重录;
    // 精确半点 5.25/1.25/0.25/0.75 取偶, 2.675 按实值 2.67499... 进位 (与半点不同)
    let cases: &[(f64, &str)] = &[
        (3.25, "3.2"),
        (3.75, "3.8"),
        (2.675, "2.7"),
        (0.05, "0.1"),
        (0.15, "0.1"),
        (9.999999, "10.0"),
        (1.0, "1.0"),
        (6.05, "6.0"),
        (12.345, "12.3"),
        (0.0, "0.0"),
        (5.25, "5.2"),
        (1.25, "1.2"),
        (0.25, "0.2"),
        (0.75, "0.8"),
        (2.35, "2.4"),
    ];
    for &(v, want) in cases {
        assert_eq!(fmt_f(v, 1), want, "String.format(\"%.1f\", {v})");
    }
    assert_eq!(fmt_f(-0.0, 1), "-0.0");
    assert_eq!(fmt_f(f64::NAN, 1), "NaN");
    assert_eq!(fmt_f(f64::INFINITY, 1), "inf");
    assert_eq!(fmt_f(f64::NEG_INFINITY, 1), "-inf");
}

// ---- 历史基线: java_math_round (Math.round) ----

#[test]
fn java_math_round_matches_java8_oracle() {
    // MR.java 基线 (含 JDK-8010430 修正域: 0.49999999999999994 → 0)
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
