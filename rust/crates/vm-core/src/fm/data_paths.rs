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
/// &lt;root&gt;/aces/gamedata/flightmodels/&lt;name 小写&gt;.json。
/// 机型名做小写规范化（大小写不敏感匹配游戏侧命名）。
// PORT: Java `String.toLowerCase()` 绑定默认 Locale (土耳其语 locale 下 I→ı
// 的变异存在); Rust `to_lowercase` 无 Locale (≡ Locale.ROOT)。机型名域为
// ASCII, 二者逐字符一致 (config_loader.rs 同款先例), 且无 Locale 形态恰为
// "匹配游戏侧小写命名"的规范意图。
// 扩展名 .json: blkx→json 迁移终态 (wt_ext_cli --format Json --blk_extension
// json 产物, build.py fmdatajson 产线; 迁移期全量位级对拍 2832/2832 绿)。
pub fn central_file(plane_name: &str) -> PathBuf {
    fm_dir().join(format!("{}.json", plane_name.to_lowercase()))
}

/// 物理 FM 文件路径。入参为中央文件 fmFile 字段映射后的相对路径
/// （"fm/xxx.blk" 剥尾 .blk 拼 .json, 形如 "fm/spitfire_f24.json"）——
/// 与 FMLoader JSON 链的调用约定一致。
pub fn physical_file(fm_file: &str) -> PathBuf {
    fm_dir().join(fm_file)
}

/// FM 数据版本文件：&lt;root&gt;/aces/version（Blkx.getVersion 展示用）
pub fn version_file() -> PathBuf {
    PathBuf::from(get_data_root()).join("aces/version")
}

/// 扫描 flightmodels 下指定子目录的 *.json 文件名 (去扩展名, 排序去重)。
/// `subdir` = "" 即 flightmodels 根 (中央文件), "fm" = 物理文件子目录。
/// 只收 .json (blkx→json 迁移: data/ 双格式同名并存, 不过滤会每机型重复两项);
/// 目录不存在/不可读 → 空 vec; 排序 = 文件名字节序 (域内 ASCII, 与 Java
/// 自然序逐位一致)。机型列表 (GetFmList / loadPlanes) 的语义收敛点。
pub fn list_fm_names(subdir: &str) -> Vec<String> {
    let dir = if subdir.is_empty() {
        fm_dir()
    } else {
        fm_dir().join(subdir)
    };
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".json") {
                continue;
            }
            // 按最后一个 '.' 剥后缀 (FileUtils 语义)
            if let Some(stripped) = crate::base::file_utils::get_file_name_no_ex(Some(&name)) {
                names.push(stripped.to_string());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

// =====================================================================
// Tests — 对应 Java: test/TestFMDataPaths.java (一比一移植)
//
// 纯字符串断言，无需 data/ 目录存在。
// 运行方式: python script/build.py test fmpaths (Java 侧) / cargo test -p vm-core
// =====================================================================
#[cfg(test)]
mod tests;
