//! 对应 Java: `src/prog/fm/FMDataPaths.java` (一比一翻译)
//!
//! FM 数据路径的唯一来源（P2 重构）。
//!
//! <p>此前 "./data/aces/gamedata/flightmodels/..." 字符串散落在 Controller.loadFMData、
//! Blkx.getVersion 等多处硬编码；本类集中管理，并为白盒测试提供 {@link #setDataRoot}
//! 注入点（测试可指向临时目录，不依赖真机 data/）。
//!
//! <p><b>扩展名统一小写 ".blkx"</b>：旧代码拼 ".Blkx"（大写 B），仅在 Windows
//! 大小写不敏感的文件系统上碰巧可用；fmdata 解包产物（wt_ext_cli --blk_extension blkx）
//! 与 build.py 均为小写，Linux/CI 下大写拼法会直接找不到文件。这里统一为小写。

use std::path::PathBuf;
use std::sync::RwLock;

/// FM 数据根目录；volatile 供测试运行时注入临时目录
// PORT: Java `private static volatile String dataRoot = "./data"` → 进程级
// RwLock<Option<String>> (None ≡ 默认 "./data"; String 堆分配非 const 可构造,
// 静态初始化只能走 Option —— config_loader::LEGACY_SCREEN_SIZE 注入点同款先例)。
// volatile 的"无锁读+写即时可见" ↔ RwLock 读写锁: 本面只有低频注入 + 路径拼装读,
// 无行为差异; 临界区仅 clone/赋值, 无 panic 路径 → 锁永不会中毒, read/write 的
// unwrap 必不失败 (后续若往临界区加逻辑需复核此前提)。表示层唯一差异: 无法区分
// "从未注入"与"显式设回默认"—— 全库无调用点
// 依赖该区分。LIFETIMES §1.3(c)/§7 的长期方案是 App.fm_data_root 构造时定死,
// 当前批次 crate 尚无 App/Env 容器, 先按原静态语义落地, AppState 波次收编。
static DATA_ROOT: RwLock<Option<String>> = RwLock::new(None);

// PORT: Java `public final class` + `private FMDataPaths() {}` 私有构造器
// (防实例化/防继承的纯静态工具类) → Rust 模块自由函数, 无实例可造, 约束天然成立。

/// FM 数据根目录（默认 "./data"，与程序工作区约定一致）
// PORT: Java volatile 读返回活引用 (零拷贝) ↔ 读锁临界区内 clone 出快照 ——
// 根路径为短字符串且读写皆低频, 无行为差异。
pub fn get_data_root() -> String {
    DATA_ROOT
        .read()
        .unwrap()
        .clone()
        .unwrap_or_else(|| "./data".to_string())
}

/// 注入数据根目录（白盒测试用）。传相对/绝对路径均可，
/// 后续所有路径拼装以最新值为准。
pub fn set_data_root(root: &str) {
    *DATA_ROOT.write().unwrap() = Some(root.to_string());
}

/// flightmodels 目录：&lt;root&gt;/aces/gamedata/flightmodels
// PORT: `new File(parent, child)` 与 `PathBuf::join` 对相对 child 均为
// 分隔符拼接, 语义等价。两处平台差异均不在本类域内:
// ① Java Win32 normalize 会把 child 里的 '/' 折叠为 '\' (Rust 原样保留 '/'),
//   仅影响裸字符串形态, 文件访问两分隔符等价, 消费方测试统一 norm('/');
// ② child 为绝对路径时 join 整体替换 parent 而 Java 按平台规则合并 ——
//   本类 child 恒为相对字面量/机型名, 分支不可达。
pub fn fm_dir() -> PathBuf {
    PathBuf::from(get_data_root()).join("aces/gamedata/flightmodels")
}

/// 中央文件（机型入口文件）路径：
/// &lt;root&gt;/aces/gamedata/flightmodels/&lt;name 小写&gt;.blkx。
/// 机型名做小写规范化（大小写不敏感匹配游戏侧命名）。
// PORT: Java `String.toLowerCase()` 绑定默认 Locale (土耳其语 locale 下 I→ı
// 的变异存在); Rust `to_lowercase` 无 Locale (≡ Locale.ROOT)。机型名域为
// ASCII, 二者逐字符一致 (config_loader.rs 同款先例), 且无 Locale 形态恰为
// "匹配游戏侧小写命名"的规范意图。
pub fn central_file(plane_name: &str) -> PathBuf {
    fm_dir().join(format!("{}.blkx", plane_name.to_lowercase()))
}

/// 物理 FM 文件路径。{@code fmFileWithX} 为中央文件 fmFile 字段解析出的相对路径
/// 再补 "x"（形如 "fm/spitfire_f24.blkx"）——与 FMLoader 的调用约定一致。
pub fn physical_file(fm_file_with_x: &str) -> PathBuf {
    fm_dir().join(fm_file_with_x)
}

/// FM 数据版本文件：&lt;root&gt;/aces/version（Blkx.getVersion 展示用）
pub fn version_file() -> PathBuf {
    PathBuf::from(get_data_root()).join("aces/version")
}

// =====================================================================
// Tests — 对应 Java: test/TestFMDataPaths.java (一比一移植)
//
// 纯字符串断言，无需 data/ 目录存在。
// 运行方式: python script/build.py test fmpaths (Java 侧) / cargo test -p vm-core
// =====================================================================
#[cfg(test)]
mod tests {
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
}
