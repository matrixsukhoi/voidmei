//! FM 识别栈黑盒场景 (kernel::fm): 合成 DATA_ROOT 临时根经 pub 的
//! set_data_root 注入, 覆盖 identify 终态机 / 负缓存 (#55 死循环回归锚) /
//! 换机广播 / 名字规范化。
//! 文件内 static 串行锁: DATA_ROOT 是进程级全局, 本二进制内互斥
//! (tests/ 每文件独立进程, 跨文件天然隔离)。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use kernel::fm::{data_paths, loader, FmChangedBus, FMManager, FMStatus};

fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn wait_status(m: &FMManager, want: FMStatus) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if m.current().status == want {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("等待 FM {:?} 超时 (实际 {:?})", want, m.current().status);
}

// ---- 合成数据 (store_tests 母本迁移: 最小 central/物理 JSON) ----

fn write_central(fm_dir: &Path, name: &str) {
    std::fs::write(
        fm_dir.join(format!("{name}.json")),
        format!("{{\"model\": \"{name}\", \"fmFile\": \"fm/{name}.blk\"}}"),
    )
    .unwrap();
}

fn write_physical(fm_sub: &Path, name: &str) {
    std::fs::write(
        fm_sub.join(format!("{name}.json")),
        "{\"synthetic-fm\": \"x\", \"EmptyMass\": 1000.0, \"Wingspan\": 11.0}",
    )
    .unwrap();
}

fn setup_synthetic(root: &Path) {
    let fm_dir = root.join("aces/gamedata/flightmodels");
    let fm_sub = fm_dir.join("fm");
    std::fs::create_dir_all(&fm_sub).unwrap();
    write_central(&fm_dir, "plane1");
    write_physical(&fm_sub, "plane1");
    write_central(&fm_dir, "badplane"); // central 在, 物理缺 → CORRUPT
    // ghost: 不铺 → MISSING
}

/// DATA_ROOT 翻转 + 用毕还原/清理 (Drop 承接, panic 展栈也还原)
struct RootGuard(PathBuf);
impl Drop for RootGuard {
    fn drop(&mut self) {
        data_paths::set_data_root("./data");
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn synthetic_root() -> (PathBuf, RootGuard) {
    let root = std::env::temp_dir().join(format!("voidmei_ktest_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    setup_synthetic(&root);
    data_paths::set_data_root(&root.to_string_lossy());
    (root.clone(), RootGuard(root))
}

fn manager() -> FMManager {
    FMManager::new(Arc::new(FmChangedBus::new()))
}

/// identify 命中: 合成齐全机型 → READY, 携带 fmdata
#[test]
fn identify_命中READY() {
    let _g = lock().lock().unwrap();
    let _root = synthetic_root();
    let m = manager();
    loader::reset_load_count();
    m.identify(Some("plane1"));
    wait_status(&m, FMStatus::Ready);
    assert_eq!(m.current_target_name().as_deref(), Some("plane1"));
    assert!(m.current().fmdata.is_some(), "READY 应携带 fmdata");
    assert!(loader::get_load_count() >= 1);
}

/// #55 死循环核心回归: 缺失机型反复 identify, 磁盘加载只发生一次
/// (负缓存; 旧架构 failedFMName 失效导致每秒 ~20 次解析风暴)
#[test]
fn identify_未命中MISSING负缓存防风暴() {
    let _g = lock().lock().unwrap();
    let _root = synthetic_root();
    let m = manager();
    loader::reset_load_count();
    for _ in 0..1000 {
        m.identify(Some("ghost"));
    }
    wait_status(&m, FMStatus::Missing);
    assert_eq!(loader::get_load_count(), 1, "1000 次 identify 只允许一次磁盘加载");
}

/// central 在库但物理文件缺失 → CORRUPT
#[test]
fn identify_物理缺失CORRUPT() {
    let _g = lock().lock().unwrap();
    let _root = synthetic_root();
    let m = manager();
    m.identify(Some("badplane"));
    wait_status(&m, FMStatus::Corrupt);
}

/// 换机: plane1 READY → ghost, FM_CHANGED 广播至少一次
#[test]
fn 换机_状态切换与FM_CHANGED广播() {
    let _g = lock().lock().unwrap();
    let _root = synthetic_root();
    let m = manager();
    let bus = m.fm_changed_bus();
    let hits = Arc::new(Mutex::new(0u32));
    let h = Arc::clone(&hits);
    let _sub = bus.subscribe(move |_| {
        *h.lock().unwrap() += 1;
    });
    m.identify(Some("plane1"));
    wait_status(&m, FMStatus::Ready);
    m.identify(Some("ghost"));
    wait_status(&m, FMStatus::Missing);
    assert!(
        *hits.lock().unwrap() >= 1,
        "换机 (Ready→Missing) 应广播 FM_CHANGED"
    );
}

/// 机型名规范化: 大写/混合大小写输入收敛为小写目标名
#[test]
fn identify_名字小写规范化() {
    let _g = lock().lock().unwrap();
    let _root = synthetic_root();
    let m = manager();
    m.identify(Some("PLANE1"));
    wait_status(&m, FMStatus::Ready);
    assert_eq!(m.current_target_name().as_deref(), Some("plane1"));
}

/// None/空名 identify: 忽略, 不触发加载
#[test]
fn identify_空名忽略() {
    let _g = lock().lock().unwrap();
    let _root = synthetic_root();
    let m = manager();
    loader::reset_load_count();
    m.identify(None);
    m.identify(Some(""));
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(m.current().status, FMStatus::Unresolved);
    assert_eq!(loader::get_load_count(), 0, "空名不应触发磁盘加载");
}
