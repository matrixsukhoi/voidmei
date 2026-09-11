//! data 轮询周期黑盒场景: mini-8111 mock → Service 真线程真 HTTP → Frame 观测。
//! 只经 pub 面 (ServiceConfig/ServiceHandle/FrameStore), 不触碰 crate 内部。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


mod common;

use std::sync::Arc;

use expect_test::{expect, Expect};

use common::{poll_until, MockServer};
use data::service_loop::{start, Service, ServiceConfig};
use kernel::base::bus::flight_data_bus::FlightDataBus;
use kernel::fm::{FmChangedBus, FMManager, FMStatus};

/// 测试 Service 组装: 20ms 高频轮询 + mock 端口; FM 根不注入
/// (CWD 相对 ./data 不存在 → identify 走 MISSING 降级, 正常路径之一)
fn service_for(port: u16, fm: Arc<FMManager>, bus: Arc<FlightDataBus>) -> Service {
    Service::new(
        ServiceConfig {
            service_loop_interval_ms: 20,
            app_port: port,
            ..ServiceConfig::default()
        },
        fm,
        bus,
    )
}

fn setup() -> (MockServer, Arc<FMManager>, Arc<FlightDataBus>) {
    let mock = MockServer::start();
    mock.set_snapshot("plane_p51d");
    let fm = Arc::new(FMManager::new(Arc::new(FmChangedBus::new())));
    (mock, fm, Arc::new(FlightDataBus::new()))
}

/// 等首帧 + 等 FM identify 终态 + 等 map 慢速链路到位 (player_live/loc 来自
/// 10×freq 周期的 /map_obj.json, 首帧时未必就绪)
fn settle(handle: &data::service_loop::ServiceHandle, fm: &FMManager) -> Arc<data::frame::Frame> {
    poll_until("首帧发布", || handle.frames.latest().is_some());
    poll_until("FM identify 终态", || {
        fm.current().status != FMStatus::Loading && fm.current().status != FMStatus::Unresolved
    });
    poll_until("player_live 置位 (map 链路就绪)", || {
        handle.frames.latest().is_some_and(|f| f.player_live)
    });
    handle.frames.latest().unwrap()
}

/// 稳定字段集的帧摘要 (时变字段 frame_seq/current_time/actual_interval 排除)
fn frame_summary(f: &data::frame::Frame) -> String {
    let s = f.s_state.as_ref().expect("s_state 应已解析");
    let i = f.s_indic.as_ref().expect("s_indic 应已解析");
    format!(
        "type={} valid={:?} ias={} heightm={:.1} tas={}\n\
         player_live={} n_vy={:.4}\n\
         fm={:?} status={:?}\n\
         loc={:?} dir={:?}",
        i.r#type.clone().unwrap_or_default(),
        s.valid,
        s.ias,
        s.heightm,
        s.tas,
        f.player_live,
        f.n_vy,
        f.fm.name,
        f.fm.status,
        f.loc.map(|v| [format!("{:.1}", v[0]), format!("{:.1}", v[1])]),
        f.dir.map(|v| [format!("{:.4}", v[0]), format!("{:.4}", v[1])]),
    )
}

fn check(actual: &str, expected: Expect) {
    expected.assert_eq(actual);
}

/// 正常周期: 真机 p51d 快照持续供数 → 帧发布, 关键飞行数据与快照一致,
/// identify 指向 mock 机型 (FM 数据根缺失 → MISSING 降级, 线路不断)
#[test]
fn 轮询_正常周期_帧发布与数据一致() {
    let (mock, fm, bus) = setup();
    let mut handle = start(service_for(mock.port, Arc::clone(&fm), bus));
    let f = settle(&handle, &fm);

    check(&frame_summary(&f), expect![[r#"
        type=P-51D-20_CHINA valid=Some(true) ias=474 heightm=46.0 tas=454
        player_live=true n_vy=-7.3426
        fm=Some("p-51d-20_china") status=Missing
        loc=Some(["0.0", "0.0"]) dir=Some(["0.0000", "0.0000"])"#]]);
    handle.stop();
}

/// 帧序号单调递增 (FrameStore publish 语义)
#[test]
fn 轮询_帧序号单调递增() {
    let (mock, fm, bus) = setup();
    let mut handle = start(service_for(mock.port, Arc::clone(&fm), bus));
    let mut last = settle(&handle, &fm).frame_seq;
    let mut ok = true;
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(60));
        match handle.frames.latest() {
            Some(f) if f.frame_seq > last => last = f.frame_seq,
            Some(_) => ok = false,
            None => ok = false,
        }
    }
    assert!(ok, "帧序号应严格递增 (last={last})");
    handle.stop();
}

/// 断线与恢复: mock 断连 → 帧冻结在最后一帧 (s_state 保留, 等待重连语义),
/// 轮询仍继续 (主备端口翻转打点); 恢复供数 → 帧恢复推进。
/// (Java 对位: 游戏关闭 → "Waiting for game connection" → 游戏重开)
#[test]
fn 轮询_断线冻结与恢复() {
    let (mock, fm, bus) = setup();
    let mut handle = start(service_for(mock.port, Arc::clone(&fm), bus));
    settle(&handle, &fm);

    mock.set_refuse();
    let frozen_seq = handle.frames.latest().unwrap().frame_seq;
    let n1 = mock.request_counts().get("/state").copied().unwrap_or(0);
    std::thread::sleep(std::time::Duration::from_millis(300));
    let seq = handle.frames.latest().unwrap().frame_seq;
    let n2 = mock.request_counts().get("/state").copied().unwrap_or(0);
    assert_eq!(seq, frozen_seq, "断线期帧应冻结 (等待重连, 不清数据)");
    assert!(n2 > n1, "断线期应继续轮询主备端口 (n1={n1} n2={n2})");

    mock.set_snapshot("plane_p51d");
    poll_until("恢复供数后帧恢复推进", || {
        handle
            .frames
            .latest()
            .is_some_and(|f| f.frame_seq > frozen_seq)
    });
    handle.stop();
}

/// 端点 404 (游戏在场但 API 异常): 与断线同语义 — 帧冻结 + 轮询不停
#[test]
fn 轮询_端点404_按失败处理() {
    let (mock, fm, bus) = setup();
    let mut handle = start(service_for(mock.port, Arc::clone(&fm), bus));
    settle(&handle, &fm);

    mock.set_not_found();
    let frozen_seq = handle.frames.latest().unwrap().frame_seq;
    let n1 = mock.request_counts().get("/state").copied().unwrap_or(0);
    std::thread::sleep(std::time::Duration::from_millis(300));
    let seq = handle.frames.latest().unwrap().frame_seq;
    let n2 = mock.request_counts().get("/state").copied().unwrap_or(0);
    assert_eq!(seq, frozen_seq, "404 期帧应冻结");
    assert!(n2 > n1, "404 期应继续轮询 (n1={n1} n2={n2})");
    handle.stop();
}

/// 连接层故障下正常停机: stop() 干净收线 (线程未 panic 逃逸)
#[test]
fn 轮询_断线期_线程存活可停机() {
    let (mock, fm, bus) = setup();
    let mut handle = start(service_for(mock.port, Arc::clone(&fm), bus));
    settle(&handle, &fm);
    mock.set_refuse();
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(handle.stop(), "断线期轮询线程应正常退出");
}

/// 切机型: p51d → bf109f4, identify 目标名跟随, FM_CHANGED 广播
#[test]
fn 轮询_切机型_识别跟随() {
    let (mock, fm, bus) = setup();
    let fm_changed = fm.fm_changed_bus();
    let changed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let c = Arc::clone(&changed);
    let _sub = fm_changed.subscribe(move |_| {
        c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    let mut handle = start(service_for(mock.port, Arc::clone(&fm), bus));
    settle(&handle, &fm);
    assert_eq!(fm.current_target_name().as_deref(), Some("p-51d-20_china"));

    mock.set_snapshot("plane_bf109f4");
    poll_until("目标名切到 bf109", || {
        fm.current_target_name().as_deref() == Some("bf-109f-4")
    });
    assert!(
        changed.load(std::sync::atomic::Ordering::SeqCst) >= 1,
        "换机应广播 FM_CHANGED"
    );
    handle.stop();
}

/// 死亡检测: totalThr=0 (missing_fm 快照) → player_live 翻 false
#[test]
fn 轮询_死亡_撤销player_live() {
    let mock = MockServer::start();
    mock.set_snapshot("plane_missing_fm");
    let fm = Arc::new(FMManager::new(Arc::new(FmChangedBus::new())));
    let mut handle = start(service_for(mock.port, fm, Arc::new(FlightDataBus::new())));
    #[allow(clippy::redundant_clone)]
    poll_until("首帧发布", || handle.frames.latest().is_some());
    poll_until("死亡检测落地", || {
        handle.frames.latest().is_some_and(|f| !f.player_live)
    });
    handle.stop();
}
