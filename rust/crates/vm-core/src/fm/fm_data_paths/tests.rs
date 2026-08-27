use super::*;
use std::path::Path;

/// 路径统一为 '/' 分隔，规避 Windows/Linux 分隔符差异
fn norm(f: &Path) -> String {
    f.to_string_lossy().replace('\\', "/")
}

/// Java assertEquals(expected, actual, desc) 计数式断言 → assert! 宏
/// (失败即 panic), 描述与期望/实际值格式逐字保留
fn assert_equals(expected: &str, actual: &str, desc: &str) {
    // 失败文案 = Java fail() 的 desc 串: desc + " —— 期望: …, 实际: …", 逐字
    assert!(expected == actual, "{desc} —— 期望: {expected}, 实际: {actual}");
}

fn test_default_root() {
    // -- 默认根目录测试 --
    // 程序约定: repo 即工作区, data 在项目根
    assert_equals("./data", &get_data_root(), "默认数据根目录应为 ./data");
}

fn test_central_file_normalization() {
    // -- 中央文件路径与小写规范化测试 --
    assert_equals(
        "./data/aces/gamedata/flightmodels/spitfire_f24.blkx",
        &norm(&central_file("spitfire_f24")),
        "小写机型名直接拼接",
    );

    // 大小写规范化: 任意大小写输入都归一到小写 (匹配游戏侧命名约定)
    assert_equals(
        "./data/aces/gamedata/flightmodels/spitfire_f24.blkx",
        &norm(&central_file("Spitfire_F24")),
        "大写输入应规范化为小写",
    );
    assert_equals(
        "./data/aces/gamedata/flightmodels/spitfire_f24.blkx",
        &norm(&central_file("SPITFIRE_F24")),
        "全大写输入应规范化为小写",
    );

    // 统一小写 .blkx 扩展名 (旧代码 ".Blkx" 仅 Windows 大小写不敏感下碰巧可用)
    let p = norm(&central_file("abc"));
    assert!(
        p.ends_with(".blkx") && !p.ends_with(".Blkx"),
        "扩展名应为小写 .blkx, 实际: {p}"
    );
}

fn test_physical_file() {
    // -- 物理 FM 文件路径测试 --
    // physicalFile 接收带 x 的相对路径 (与 FMLoader 调用约定一致)
    assert_equals(
        "./data/aces/gamedata/flightmodels/fm/spitfire_f24.blkx",
        &norm(&physical_file("fm/spitfire_f24.blkx")),
        "物理文件 = fmDir + 相对路径",
    );
}

fn test_fm_dir_and_version_file() {
    // -- 目录与版本文件路径测试 --
    assert_equals(
        "./data/aces/gamedata/flightmodels",
        &norm(&fm_dir()),
        "fmDir 应为 <root>/aces/gamedata/flightmodels",
    );
    assert_equals(
        "./data/aces/version",
        &norm(&version_file()),
        "versionFile 应为 <root>/aces/version",
    );
}

fn test_set_data_root_injection() {
    // -- setDataRoot 注入测试 --
    set_data_root("testroot");
    assert_equals("testroot", &get_data_root(), "getDataRoot 应返回注入值");
    assert_equals(
        "testroot/aces/gamedata/flightmodels/plane1.blkx",
        &norm(&central_file("Plane1")),
        "注入后所有路径以新根为准",
    );
    assert_equals(
        "testroot/aces/version",
        &norm(&version_file()),
        "注入后 versionFile 跟随新根",
    );

    // 再注入一次验证可重复切换 (测试套件间隔离的基础)
    set_data_root("otherroot");
    assert_equals(
        "otherroot/aces/gamedata/flightmodels/plane1.blkx",
        &norm(&central_file("plane1")),
        "二次注入应生效",
    );
}

/// 复刻 Java main() 的 try/finally: 断言 panic 展栈时仍执行还原,
/// 避免影响同测试二进制内后续逻辑
struct DataRootResetOnDrop;
impl Drop for DataRootResetOnDrop {
    fn drop(&mut self) {
        // 还原默认根目录，避免影响同 JVM 内后续逻辑
        set_data_root("./data");
    }
}

// PORT: Java main() 在单 JVM 内顺序执行五个测试方法 (test_set_data_root_injection
// 依赖前序默认根状态, 且会改写全局); cargo test 默认多线程并行跑 #[test],
// 拆成多个 #[test] 会与全局注入竞态 —— 故收敛为单个 #[test] 复刻 main() 的
// 顺序执行, 五个方法体与断言逐条保留。
// ⚠ 后续新增任何触碰 set_data_root 的测试 (如 blkx/realtests.rs 腿2 TODO(port)
// 计划的临时 data 根注入) 必须并入本 #[test], 或加共享 static Mutex<()> 的
// 手写串行守卫 (serial_test 式, 禁新增依赖) —— cargo 在同测试二进制内并行跑
// #[test], Java 靠"单 JVM 顺序 + finally 还原"规避的 DATA_ROOT 竞态会真实发生。
#[test]
fn java_main_sequence() {
    // DATA_ROOT 测试串行锁 (test_guard, QA 终检接入): 消除与 store_tests
    // set_data_root 生效窗口的互相击穿 (此前依赖注册序裕度, 见 fm/mod.rs
    // test_guard 模块注释的 B1 残余项备案) —— 接入后本用例成为挂锁方,
    // 与 fm_loader/fm_manager/store_tests 的 DATA_ROOT 用例全量互斥
    let _guard = crate::fm::test_guard::data_root();
    let _reset = DataRootResetOnDrop;

    test_default_root();
    test_central_file_normalization();
    test_physical_file();
    test_fm_dir_and_version_file();
    test_set_data_root_injection();
}
