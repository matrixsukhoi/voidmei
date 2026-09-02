//! 对应 Java: `test/TestFMStore.java` (一比一移植)
//!
//! FMManager/FMLoader 白盒测试 —— issue #55 死循环回归（P2 单一真相源架构）
//!
//! 在 DATA_ROOT 临时根下合成最小中央/物理 blkx 文件（不依赖真机 data/，CI 可跑）：
//!   plane1/plane2 —— central + physical 齐全，可加载到 READY
//!   badplane     —— 只有 central（物理文件缺失）→ CORRUPT
//!   ghost        —— 什么都不放 → MISSING
//!
//! 核心回归点：缺失机型反复 identify 不再触发磁盘加载（负缓存），
//! 取代旧架构 failedFMName 手动同步失效导致的每秒 ~20 次解析风暴。
//!
//! 运行方式: python script/build.py test fmstore (Java 侧) / cargo test -p vm-core
//!
//! PORT 移植要点:
//! - TestFMHandle.java 的断言已全量移植于 `fm/handle.rs` 的 tests 模块 (同批
//!   已译产物, 六个用例逐条核对无缺漏), 本文件不重复移植 (§5 不为凑覆盖写空转
//!   测试); TestFMStore 的 FMLoader 同步面亦已移植于 `fm/fm_loader.rs` tests。
//! - 数据根方案 = **Java 原文直译**: `Files.createTempDirectory("voidmei_fmtest")`
//!   + `FMDataPaths.setDataRoot(tmpRoot.toString())` (TestFMStore.java L40-43,
//!     finally L57-59 还原 "./data" + rmtree)。临时根为**绝对路径**, 对进程级
//!     CWD 的任意翻转天然免疫 —— 这是审查 A blocker B1 的修复: 前一版"多根铺
//!     数据"方案把 ROOTS 铺在**相对路径** (`./data`/`testroot`/`otherroot`) 上,
//!     而同测试二进制内 config_manager::tests::sandbox() 持其**私有** CWD_LOCK
//!     后 `env::set_current_dir(临时目录)` —— CWD 是进程级的, 该锁与
//!     fm::test_guard 不互斥, 相对路径的铺数据/落盘/load 解析全部随瞬时 CWD
//!     漂移 (实测形态 A: create_dir_all/write 撞上 sandbox 并发建删的目录树,
//!     Os 183/3 panic; 形态 B: load 的 central miss → MISSING 进负缓存 → 断言
//!     失败)。Java 原版对 DATA_ROOT 与 CWD 双免疫 (独立 JVM + 绝对临时根),
//!     直译即恢复行为保真。
//! - 本测试全程持 test_guard 串行锁: 与 fm_loader::tests / fm_manager::tests
//!   的挂锁用例互斥 (LOAD_COUNT 计数与 DATA_ROOT 互不污染; 锁释放前 DATA_ROOT
//!   已还原回 "./data", 挂锁方看到的恒为干净初态)。
//! - PORT(残余竞态备案, §6 只标注不越文件修): fm_data_paths::java_main_sequence
//!   是全库唯一未挂 test_guard 的 DATA_ROOT 翻转源 (接入 = 其 #[test] 体首行
//!   `let _g = crate::fm::test_guard::data_root();` 一行, 落点在 fm_data_paths.rs,
//!   本波次文件约束外, 已上报主 agent)。其 string 断言窗口 (亚毫秒) 若与本测试
//!   set_data_root 生效窗口重叠会互相击穿; 现依赖实践裕度: 测试注册序
//!   (fm::fm_data_paths 先于 fm::store_tests, harness 按注册序派发) + 其亚毫秒
//!   时长 + 本测试 setup 的毫秒级前置。接入一行后彻底消除。
//! - 机型名保留 Java 字面量 plane1/plane2/badplane/ghost: 数据落在本测试私有的
//!   绝对临时根内, 无共享根碰撞面, 不需 fm_loader.rs 铺共享根时的 zzfmload_
//!   防碰撞前缀 (fm_loader.rs 铺 crate 相对根才需防真机 data/ 误覆盖)。
//! - Java `FMManager.getInstance()` 单例 → 测试自建 manager 实例 (fm_manager
//!   非单例 API, W3 批次裁决): 单实例贯穿全部用例 + 各用例首 `reset()`
//!   复刻 Java 单例生命周期下的用例隔离方式。FM_CHANGED 广播通道
//!   `FmChangedBus` 构造注入 (TestFMStore 无事件面断言, 零订阅即可)。
//! - Java `Thread.sleep(300/200)` ("给潜在误发任务留出现形时间") →
//!   `thread::sleep` 同值保留 (§2.13 中断语义不适用于测试线程)。
//! - Java `check(boolean, String)` 计数式软断言 (全部执行完才判失败) →
//!   assert! 宏 (失败即 panic), 描述文本逐字保留 (handle.rs 先例)。
//!
//! PORT(与 fm_manager.rs tests 的分工): fm_manager 波次 (W3 并行) 一度在其
//! 文件内重复移植了 TestFMStore 全套用例, 其审查/修复轮已收缩回 manager 私有
//! 面 (真机 data/ 断言链 + 负缓存事件派发 + invalidate/null 边界) —— 本文件是
//! wf-p3-batch5 为 TestFMStore 指定的落点 (item fm_store_tests 的 out),
//! 二者不再重叠。跨文件备案 (§6 只标注不越文件修): fm_manager.rs tests 的
//! identify_null_and_empty_are_ignored 断言 `get_load_count() == 0` 但既不
//! reset_load_count() 也不挂 test_guard 锁 —— `--test-threads=1` 顺序执行时
//! 被先行用例的 LOAD_COUNT 残留确定性击穿 (实测复现一次), 修复属 fm_manager
//! 波次。另: fm_manager.rs tests 的三个铺根用例仍用 crate 相对根, 存在同款
//! CWD 翻转敞口 (审查 A B1 尾注), 修复亦属 fm_manager 波次 (换绝对临时根或
//! 挂锁, 本文件方案可直接参考)。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::bus::EventBus;
use crate::fm::fm_data_paths;
use crate::fm::fm_loader;
use crate::fm::fm_manager::FMManager;
use crate::fm::status::FMStatus;

/// 轮询等待的超时上限（合成文件很小，正常毫秒级完成；10s 是宽松上界）
const WAIT_TIMEOUT_MS: u64 = 10_000;

/// Java check(boolean, String) 计数式断言 → assert! 宏, 描述逐字保留
fn check(cond: bool, desc: &str) {
    assert!(cond, "FAIL: {desc}");
}

/// 轮询等待条件成立（20ms 间隔），超时返回最后一次求值结果
fn wait_for<F: Fn() -> bool>(cond: F) -> bool {
    let deadline = Instant::now() + Duration::from_millis(WAIT_TIMEOUT_MS);
    let mut v = cond();
    while !v && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
        v = cond();
    }
    v
}

/// 构造被测 manager。Java `FMManager.getInstance()` 饿汉单例 → 非单例实例
/// (W3 裁决: 调用方持有 struct, AppState 收编); FM_CHANGED 广播经
/// `FmChangedBus` (构造注入, 非全局) —— TestFMStore 无事件面断言,
/// 注入零订阅总线即可。
fn new_manager() -> FMManager {
    FMManager::new(Arc::new(EventBus::new()))
}

// ==================== 合成数据 (Java 原文: 私有临时根) ====================

/// Java `Files.createTempDirectory("voidmei_fmtest")`: 前缀 + 唯一后缀的
/// **绝对**临时目录 (resolve 到系统 temp, 与 CWD 无关)。
/// PORT: std 无 create_temp_dir (禁新增 tempfile 依赖), 以 pid + 原子计数
/// 复刻唯一性; 先清可能的崩溃残留 (Java 随机名无此态, 防御性多余动作)
fn create_temp_root() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("voidmei_fmtest{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 双格式同铺 (blkx→json 迁移: 默认格式已翻 Json, 合成数据双铺兼容两条链)
fn write_both(dir: &Path, name: &str, blkx_text: &str, json_text: &str) {
    std::fs::write(dir.join(format!("{name}.blkx")), blkx_text).unwrap();
    std::fs::write(dir.join(format!("{name}.json")), json_text).unwrap();
}

/// 最小中央文件 —— 只需 getlastone("fmfile")/get_last_string_ci("fmfile") 能命中
/// （参考真机文件头 fmFile:t = "fm/xxx.blk"）
fn write_central(fm_dir: &Path, name: &str) {
    write_both(
        fm_dir,
        name,
        &format!("model:t = \"{name}\"\nfmFile:t = \"fm/{name}.blk\"\n"),
        &format!("{{\"model\": \"{name}\", \"fmFile\": \"fm/{name}.blk\"}}"),
    );
}

/// 最小物理 FM —— 非空且不以 '{' 开头即可全量解析：
/// getload 对缺失字段全部按 0 处理（无 Jet/Compressor 块 → 按喷气形态、compNumSteps=0，
/// extractStages 返回 null、peakThrust=0），最终 valid=true → READY。
fn write_physical(fm_sub: &Path, name: &str) {
    write_both(
        fm_sub,
        name,
        "synthetic-fm:t = \"x\"\nEmptyMass:r = 1000\nWingspan:r = 11\n",
        "{\"synthetic-fm\": \"x\", \"EmptyMass\": 1000.0, \"Wingspan\": 11.0}",
    );
}

fn setup_synthetic_data(tmp_root: &Path) {
    let fm_dir = tmp_root.join("aces/gamedata/flightmodels");
    let fm_sub = fm_dir.join("fm");
    std::fs::create_dir_all(&fm_sub).unwrap();

    // 可加载机型: central 指向 fm/<name>.blk, 物理文件存在
    write_central(&fm_dir, "plane1");
    write_central(&fm_dir, "plane2");
    write_physical(&fm_sub, "plane1");
    write_physical(&fm_sub, "plane2");

    // CORRUPT 机型: central 在库但物理文件缺失
    write_central(&fm_dir, "badplane");

    // ghost: 什么都不写 → MISSING
}

/// Java finally 三步的 Drop 承接 (断言 panic 展栈时也执行):
/// `setDataRoot("./data")` 还原全局 → (reset 由测试尾显式调用) → `rmtree(tmpRoot)`。
/// rmtree 删除失败静默 (Java `f.delete()` 返回值未用, 同为无提示尽力删)
struct RootCleanup {
    root: PathBuf,
}
impl Drop for RootCleanup {
    fn drop(&mut self) {
        fm_data_paths::set_data_root("./data");
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

// ==================== 测试用例 ====================
// (identify 的 null/空名边界守卫不在此重复: fm_manager.rs tests 的
//  identify_null_and_empty_are_ignored 已覆盖, §5 不为凑覆盖写空转测试)

/**
 * 用例① identify 去重：同 target 调 1000 次，加载任务只执行一次。
 * （旧架构等价场景：getBlkx 每 50ms 被调 → 每次都进 loadFMData）
 */
fn test_identify_dedup(m: &mut FMManager) {
    // -- 用例① identify 去重 --
    m.reset();
    fm_loader::reset_load_count();

    for _ in 0..1000 {
        m.identify(Some("plane1"));
    }

    let ok = wait_for(|| {
        m.current().status == FMStatus::Ready && m.current().name.as_deref() == Some("plane1")
    });
    check(ok, "1000 次 identify 后应到达 READY(plane1)");
    check(
        fm_loader::get_load_count() == 1,
        &format!(
            "FMLoader.load 只应执行 1 次 (实际 {})",
            fm_loader::get_load_count()
        ),
    );
    // PORT: Java `m.current().blkx != null` 引用非空判 ↔ Option::is_some
    check(m.current().has_fm() && m.current().blkx.is_some(), "READY 句柄应携带 blkx");
    check(
        m.current_target_name().as_deref() == Some("plane1"),
        "目标名应规范化为小写 plane1",
    );
}

/**
 * 用例② 死循环核心回归：identify 不存在的机型 1000 次 ——
 * 第一次加载落 MISSING 进负缓存后，其余 999 次零磁盘加载。
 */
fn test_negative_cache_no_storm(m: &mut FMManager) {
    // -- 用例② 负缓存防风暴 (核心死循环回归) --
    m.reset();
    fm_loader::reset_load_count();

    m.identify(Some("ghost")); // 第一次: 发任务 → FMLoader.load → MISSING → 负缓存
    let ok = wait_for(|| m.current().status == FMStatus::Missing && !m.is_loading());
    check(ok, "首次 identify(ghost) 后应落定 MISSING");
    check(fm_loader::get_load_count() == 1, "ghost 只应真正加载 1 次");

    let after_first = fm_loader::get_load_count();
    for _ in 0..999 {
        m.identify(Some("ghost")); // 全部应被负缓存拦截
    }
    // 给潜在误发任务留出现形时间
    std::thread::sleep(Duration::from_millis(300));
    check(
        fm_loader::get_load_count() == after_first,
        &format!(
            "后续 999 次 identify 不应产生新加载 (期望 {after_first}, 实际 {})",
            fm_loader::get_load_count()
        ),
    );
    check(m.current().status == FMStatus::Missing, "状态应稳定停留在 MISSING");
    check(!m.is_loading(), "不应有在途任务");
}

/** 用例②b CORRUPT 同样进负缓存（central 在库但物理文件缺失） */
fn test_corrupt_also_cached(m: &mut FMManager) {
    // -- 用例②b CORRUPT 也进负缓存 --
    m.reset();
    fm_loader::reset_load_count();

    m.identify(Some("badplane"));
    let ok = wait_for(|| m.current().is_missing_like() && !m.is_loading());
    check(ok, "identify(badplane) 应落定 missing-like (MISSING/CORRUPT)");
    check(m.current().status == FMStatus::Corrupt, "物理文件缺失应为 CORRUPT");

    let after_first = fm_loader::get_load_count();
    for _ in 0..500 {
        m.identify(Some("badplane"));
    }
    std::thread::sleep(Duration::from_millis(200));
    check(
        fm_loader::get_load_count() == after_first,
        "CORRUPT 后续 identify 不应产生新加载",
    );
}

/** 用例③ clearTarget 保留句柄：下次同名 identify 秒开（零加载） */
fn test_clear_target_keeps_handle(m: &mut FMManager) {
    // -- 用例③ clearTarget 保留句柄 --
    m.reset();
    fm_loader::reset_load_count();

    m.identify(Some("plane1"));
    let ok = wait_for(|| m.current().status == FMStatus::Ready);
    check(ok, "前置: plane1 到达 READY");

    m.clear_target();
    check(m.current_target_name().is_none(), "clearTarget 后目标应为 null");
    check(m.current().has_fm(), "clearTarget 后句柄应保留 (下次秒开)");

    let before = fm_loader::get_load_count();
    m.identify(Some("plane1")); // 句柄已在 → 恢复目标即可，零成本
    check(
        m.current_target_name().as_deref() == Some("plane1"),
        "identify 后目标恢复为 plane1",
    );
    check(m.current().has_fm(), "句柄持续可用");
    std::thread::sleep(Duration::from_millis(200));
    check(fm_loader::get_load_count() == before, "秒开路径不应触发重新加载");
}

/**
 * 用例④ 速率护栏与活性：A→B→A 快速切换（60s 护栏窗口内）不得卡死在 B 的句柄上。
 * 护栏只在 current 正是该机型时拦截；目标切走又切回时放行重载（正确性优先于限速）。
 */
fn test_rate_guard_and_liveness(m: &mut FMManager) {
    // -- 用例④ 速率护栏与回切活性 --
    m.reset();
    fm_loader::reset_load_count();

    m.identify(Some("plane1"));
    check(
        wait_for(|| m.current().name.as_deref() == Some("plane1") && m.current().has_fm()),
        "plane1 READY",
    );

    m.identify(Some("plane2"));
    check(
        wait_for(|| m.current().name.as_deref() == Some("plane2") && m.current().has_fm()),
        "plane2 READY",
    );

    m.identify(Some("plane1")); // 60s 内已尝试过 plane1 —— 必须放行, 否则卡死在 plane2
    let ok = wait_for(|| m.current().name.as_deref() == Some("plane1") && m.current().has_fm());
    check(ok, "A→B→A 回切后应回到 READY(plane1), 不得卡在 plane2");
    check(
        fm_loader::get_load_count() == 3,
        &format!(
            "两次首载 + 一次回切重载 = 3 次执行 (实际 {})",
            fm_loader::get_load_count()
        ),
    );
}

/**
 * 用例④b 非飞机载具短路：坦克 type（"tankmodels/..." 路径前缀名）直接落
 * NOT_AIRCRAFT——零磁盘加载、不进负缓存、飞机↔坦克往返行为正确。
 * 回归: 陆战时误把坦克当"FM 缺失的新飞机"弹 toast + 白做磁盘查找。
 */
fn test_not_aircraft_short_circuit(m: &mut FMManager) {
    // -- 用例④b 非飞机载具短路 (陆战坦克) --
    m.reset();
    fm_loader::reset_load_count();

    // 同步落定: 不经过 loader 线程
    m.identify(Some("tankmodels/us_n4a3e8_76_sherman"));
    check(m.current().status == FMStatus::NotAircraft, "坦克应立即落定 NOT_AIRCRAFT");
    check(!m.current().is_missing_like(), "不属于 missing-like (不弹缺失 toast)");
    check(!m.current().has_fm(), "无 FM, HUD 走降级");
    check(fm_loader::get_load_count() == 0, "不应触发任何磁盘加载");
    check(!m.is_loading(), "无在途任务");

    // 重复 identify 同一坦克: 目标去重拦截, 仍零加载
    for _ in 0..100 {
        m.identify(Some("tankmodels/us_n4a3e8_76_sherman"));
    }
    check(fm_loader::get_load_count() == 0, "重复 identify 同一坦克仍零加载");

    // 飞机 → 坦克 → 换坦克 → 回飞机: 往返行为正确
    m.identify(Some("plane1"));
    check(wait_for(|| m.current().has_fm()), "前置: plane1 到达 READY");
    let loads_before = fm_loader::get_load_count();

    m.identify(Some("tankmodels/us_n4a3e8_76_sherman"));
    check(
        m.current().status == FMStatus::NotAircraft && !m.is_loading(),
        "飞机→坦克: 句柄应让位为 NOT_AIRCRAFT",
    );
    m.identify(Some("tankmodels/germ_panther_ii"));
    check(
        m.current().status == FMStatus::NotAircraft
            && m.current().name.as_deref() == Some("tankmodels/germ_panther_ii"),
        "坦克→坦克: 直接换 NOT_AIRCRAFT 句柄",
    );
    check(fm_loader::get_load_count() == loads_before, "坦克切换全程零加载");

    m.identify(Some("plane1"));
    check(wait_for(|| m.current().has_fm()), "坦克→飞机: 应重新加载回 READY(plane1)");
}

/** 用例⑤ reset：清一切（含负缓存），停掉 pending 任务 */
fn test_reset(m: &mut FMManager) {
    // -- 用例⑤ reset --
    // PORT: Java 此用例刻意不带前置 reset —— 承接用例④b 的残留状态
    m.identify(Some("ghost"));
    check(wait_for(|| m.current().status == FMStatus::Missing), "前置: ghost 已进负缓存");

    m.reset();
    check(m.current().status == FMStatus::Unresolved, "reset 后 current 应为 UNRESOLVED");
    check(m.current_target_name().is_none(), "reset 后目标应为 null");
    check(!m.is_loading(), "reset 后无在途任务");

    fm_loader::reset_load_count();
    m.identify(Some("ghost")); // 负缓存已清 → 应重新发任务
    let ok = wait_for(|| m.current().status == FMStatus::Missing);
    check(ok, "reset 清负缓存后 ghost 可重新尝试 (并再次落 MISSING)");
    check(fm_loader::get_load_count() == 1, "reset 后应执行 1 次新加载");
}

/**
 * 用例⑥ 并发 identify：两个线程各 50 次交替识别不同机型，
 * 最终 current 必须与 currentTarget 一致（单线程 loader 串行 + 任务过期校验保证）。
 */
fn test_concurrent_identify(m: &mut FMManager) {
    // -- 用例⑥ 并发 identify 最终一致 --
    m.reset();
    fm_loader::reset_load_count();

    // PORT: scoped thread + Builder 保名 (join 由 scope 收尾保证, 时序等价)
    let mgr: &FMManager = &*m;
    std::thread::scope(|s| {
        let t1 = std::thread::Builder::new()
            .name("identify-plane1".to_string())
            .spawn_scoped(s, || {
                for _ in 0..50 {
                    mgr.identify(Some("plane1"));
                }
            })
            .unwrap();
        let t2 = std::thread::Builder::new()
            .name("identify-plane2".to_string())
            .spawn_scoped(s, || {
                for _ in 0..50 {
                    mgr.identify(Some("plane2"));
                }
            })
            .unwrap();
        t1.join().unwrap();
        t2.join().unwrap();
    });

    // PORT: Java 字符串拼接把 null 引用转为 "null" ↔ unwrap_or("null")
    let ok = wait_for(|| {
        !mgr.is_loading()
            && mgr.current_target_name().is_some()
            && mgr.current_target_name().as_deref() == mgr.current().name.as_deref()
    });
    check(
        ok,
        &format!(
            "任务清空后 current 应与最后 target 一致 (target={}, current={})",
            mgr.current_target_name().as_deref().unwrap_or("null"),
            mgr.current().name.as_deref().unwrap_or("null")
        ),
    );
    let loads = fm_loader::get_load_count();
    check(
        (1..=100).contains(&loads),
        &format!("加载次数应远小于 identify 次数且非零 (实际 {loads})"),
    );

    // 无论最终是谁，句柄必须完整可用
    check(mgr.current().has_fm(), "最终句柄应为 READY 且携带 blkx");
}

// ==================== main (Java main 顺序执行) ====================

// PORT: Java main() 在单 JVM 内顺序执行八个用例 (共享单例 + 全局 DATA_ROOT/
// loadCount), cargo test 并行跑 #[test] 会与之竞态 —— 收敛为单个 #[test]
// 复刻 main() 的顺序执行 (fm_data_paths::java_main_sequence 先例)。
#[test]
fn java_main_sequence() {
    // DATA_ROOT 测试串行锁 (test_guard): 全程持锁, 与 fm_loader::tests /
    // fm_manager::tests 的挂锁用例互斥; Drop 逆序保证锁释放前 DATA_ROOT 已
    // 还原 (挂锁方看到的恒为 "./data" 干净初态)
    let _guard = crate::fm::test_guard::data_root();

    // 对 config_manager sandbox 的进程级 CWD 翻转免疫 (审查 A B1 修复点)
    let tmp_root = create_temp_root();
    // Java finally 的 Drop 承接: panic 展栈时也还原 DATA_ROOT + rmtree
    let _cleanup = RootCleanup { root: tmp_root.clone() };

    setup_synthetic_data(&tmp_root);
    // load 的 central/physical 解析不再随 CWD 漂移
    fm_data_paths::set_data_root(&tmp_root.to_string_lossy());

    // PORT: Rust Lang 为静态表, blkx 构造内部自取 (blkx/model.rs 先例), 无全局 init

    let mut m = new_manager();
    let m = &mut m;

    test_identify_dedup(m);
    test_negative_cache_no_storm(m);
    test_corrupt_also_cached(m);
    test_clear_target_keeps_handle(m);
    test_rate_guard_and_liveness(m);
    test_not_aircraft_short_circuit(m);
    test_reset(m);
    test_concurrent_identify(m);

    // Java finally: setDataRoot("./data") / FMManager.getInstance().reset()
    // (停掉 pending 任务, 防 straggler 加载在清理后现形) / rmtree (Drop 承接)
    fm_data_paths::set_data_root("./data");
    m.reset();
}
