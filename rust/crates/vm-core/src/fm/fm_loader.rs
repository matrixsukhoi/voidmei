//! 对应 Java: `src/prog/fm/FMLoader.java` (一比一翻译)
//!
//! 纯静态的 FM 加载器（P2 重构）—— 项目内未来唯一 new Blkx 的地方。
//!
//! <p>逻辑自旧 Controller.loadFMData 原样迁移：中央文件只读文本 → 提取燃油改装修正
//! 与 fmFile 字段 → 解析物理 FM 文件 → 全量解析（getAllplotdata + finalizeLoading）→
//! 按发动机类型提取增压器参数或峰值推力 → 产出不可变 {@link FMHandle}。
//!
//! <p>与旧实现的差异（均为死循环重构的关键）：
//! <ul>
//!   <li><b>全程 try{...}catch(Throwable)</b>：旧代码只 try 了物理文件构造，getAllplotdata/
//!       finalizeLoading 在 try 之外（P1 核验发现的第二条循环路径——那里抛异常会直接炸出
//!       loadFMData，失败状态记录不上，调用方下一轮又重试）。现在任何 Throwable（含 OOM，
//!       记日志后）一律收敛为 CORRUPT 句柄，进入 {@link FMManager} 负缓存，永不再试。</li>
//!   <li><b>不再 System.gc()</b>：loader 是低频后台线程，显式 gc 只是"建议"且在旧架构的
//!       每秒多次重载风暴下反而放大停顿；大 FM 结构的回收交给 JVM 自行决策。</li>
//!   <li><b>不持有任何状态</b>：失败记录（旧 failedFMName）由 FMManager 的负缓存承担，
//!       本类无副作用、可任意重入。</li>
//! </ul>
//!
//! PORT: Java `public final class FMLoader` + `private FMLoader() {}` 私有构造器的
//! 纯静态工具类 → Rust 模块自由函数 (fm_data_paths.rs 同款先例); "唯一 new Blkx 点"
//! = [`crate::blkx::Blkx::parse_named`] (blkx::reader 家族, 该波次补的具名入口)。
//! PORT: 线程模型 — Java 在 FM-Loader 线程调用 → 保持同步函数, 线程由
//! Manager/调用方管 (无自起线程)。

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::blkx::{extract_fuel_modifications, Blkx, FuelModification, FuelType};
use crate::fm::fm_data_paths;
use crate::fm::handle::FMHandle;
use crate::fm_power_extractor::{extract_stages_with_fuel, is_piston_engine};
use crate::logger;
use crate::piston_power_model::peak_wep_power;

/// 白盒测试计数器：FMLoader.load 真正执行（进入加载流程）的次数
// PORT: Java `private static volatile long loadCount` → AtomicU64 (§1 volatile →
// AtomicXxx; LIFETIMES §1.3 裁决 "测试用 AtomicU64 注入或删")。Java 的 `loadCount++`
// 是 volatile 复合赋值 (非原子, 并发下本就是竞态噪声计数), fetch_add 为其正确超集
static LOAD_COUNT: AtomicU64 = AtomicU64::new(0);

/// 白盒测试用：读取 load 执行计数
pub fn get_load_count() -> u64 {
    LOAD_COUNT.load(Ordering::Relaxed)
}

/// 白盒测试用：清零计数
pub fn reset_load_count() {
    LOAD_COUNT.store(0, Ordering::Relaxed)
}

/// 加载指定机型的 FM 数据。任何一步失败都返回 MISSING/CORRUPT 句柄，绝不抛出、
/// 绝不返回 null。
///
/// @param planeName 机型名（任意大小写/空白，内部规范化）
/// @return 加载结果句柄；name 为空时返回 UNRESOLVED
// PORT: Java `String` 可 null 入参 → Option<&str> (§1)。
// PORT: Java catch(Throwable) (含 OOM/NPE/StringIndexOutOfBounds) → 双通道收敛:
// ① 常规失败走 try_load 的 Result Err; ② panic (blkx 原语在 data=None 上 unwrap
// 对应 Java NPE, reader.rs NPE 保真注) 经 catch_unwind 兜底 —— 两者统一记
// ERROR 日志后收敛 CORRUPT, 不允许炸穿 loader 线程导致任务队列停摆
pub fn load(plane_name: Option<&str>) -> FMHandle {
    // 空名直接 UNRESOLVED（与 FMManager.identify 的空值守卫双保险）
    let plane_name = match plane_name {
        None | Some("") => return FMHandle::UNRESOLVED,
        Some(p) => p,
    };
    // PORT: Java `planeName.toLowerCase().trim()` — toLowerCase 绑定默认 Locale,
    // 机型名域为 ASCII, Rust to_lowercase (≡ Locale.ROOT) 逐字符一致
    // (fm_data_paths.rs 同款先例); trim 差异同类: Java String.trim 只剥
    // <= U+0020 的 C0 控制符, Rust str::trim 剥 Unicode White_Space (含 NBSP
    // U+00A0) —— 机型名域为游戏 API type 字段 (ASCII), 差异不可达
    // (blkx/reader.rs java_trim 同域声明先例)
    let name = plane_name.to_lowercase().trim().to_string();
    LOAD_COUNT.fetch_add(1, Ordering::Relaxed);

    // 全程兜底：见模块 doc——任何异常（含 getAllplotdata/finalizeLoading 阶段）
    // 都收敛为 CORRUPT，交给 FMManager 负缓存，杜绝重试风暴
    match catch_unwind(AssertUnwindSafe(|| try_load(&name))) {
        Ok(Ok(handle)) => handle,
        Ok(Err(t)) => {
            // OOM 也一并捕获（记 ERROR 便于排查）：不允许异常炸穿 loader 线程导致任务队列停摆，
            // 统一收敛为 CORRUPT 句柄进负缓存
            // PORT: Java 3 参 error 的 "message + ": " + getMessage()" 双拼形态
            // (消息串已含 t 的 toString, 再拼一次 getMessage) 由 error_with_throwable
            // 复刻; 消息本体 = `"FM加载异常(" + name + "): " + t` 逐字
            logger::error_with_throwable(
                "FMLoader",
                &format!("FM加载异常({name}): {t}"),
                &LoadThrowable(t),
            );
            FMHandle::corrupt(Some(name))
        }
        Err(panic_payload) => {
            // panic 载荷 (unwrap 越界等) ≈ Java 运行时异常的 toString 形态
            // PORT: 可见输出差异备案 — Rust 默认 panic hook 在 catch_unwind 捕获前
            // 已把 "thread ... panicked at ..." 打到 stderr (不受日志级别控制),
            // Java catch(Throwable) 只有 Logger.error 单通道; 句柄契约 (CORRUPT
            // 收敛) 不受影响。不在库内 set_hook 全局压制 (会伤测试输出), 留待
            // App 组装层统一处置 (B 审查建议)
            let t = panic_message(panic_payload.as_ref());
            logger::error_with_throwable(
                "FMLoader",
                &format!("FM加载异常({name}): {t}"),
                &LoadThrowable(t),
            );
            FMHandle::corrupt(Some(name))
        }
    }
}

/// Java `load` 的 try 块主体 — Err ≡ 会落入 catch(Throwable) 的异常路径。
// PORT: Java catch 块收敛 CORRUPT 由外层 load 统一执行, 本函数只报错不处置
fn try_load(name: &str) -> Result<FMHandle, String> {
    // 1. 中央文件不存在 → 确认机型不在库 → MISSING
    let central = fm_data_paths::central_file(name);
    if !central.exists() {
        return Ok(FMHandle::missing(Some(name.to_string())));
    }

    // 2. 只读解析中央文件（doLoad=false，不触发全量 FM 解析）
    // PORT: Java 构造器失败不外抛 (产出 valid=false 对象) ↔ parse_named 返回 Err;
    // 下游 `lookupBlkx.valid && lookupBlkx.data != null` 双条件在 Result 化后坍缩为
    // Option 的 is_some (reader.rs 契约: Ok 恒 valid=true 且 data 非 None),
    // `.ok()` 即 "valid=false 时整块跳过"
    let lookup_blkx =
        Blkx::parse_named_opts(&central.to_string_lossy(), &format!("{name}.blk"), false).ok();

    // 3. 提取燃油改装修正（中央文件专属信息，物理文件里没有）
    let mut fuel_mod: Option<FuelModification> = None;
    let mut fmfile: Option<String> = None;
    if let Some(lookup) = lookup_blkx.as_ref() {
        // data 非 None 由 reader 契约保证 (见上 PORT 注), unwrap 恒成功
        let data = lookup.data.as_deref().unwrap();
        let fm = extract_fuel_modifications(data);
        if fm.r#type != FuelType::None {
            // PORT: Java `"…(HP bonus=" + fuelMod.sovietOctaneHpBonus + ")"` 的
            // Double.toString 拼接形态 (整数值带 ".0" 尾) 由 java_double_str 复刻
            logger::info(
                "FMLoader",
                &format!(
                    "Fuel modification detected: {} (HP bonus={})",
                    fm.r#type,
                    java_double_str(fm.soviet_octane_hp_bonus)
                ),
            );
        }
        fuel_mod = Some(fm);

        // 4. 从中央文件取物理 FM 文件相对路径（fmFile:t = "fm/xxx.blk"）
        fmfile = lookup.getlastone("fmfile");
        if let Some(f) = fmfile.as_deref() {
            // 剥首尾引号并去前导 '/'
            // PORT: Java `substring(indexOf("\"") + 1, length() - 1)` — indexOf 无引号
            // 时 -1 → 起点 0 (剥末字符); 越界 (空串 / 起点越过终点) Java 抛
            // StringIndexOutOfBoundsException → catch(Throwable) → CORRUPT, 此处
            // 以 Err 复刻。§2.1: 域内 fmFile 值为 ASCII 路径, 字节偏移 ≡ UTF-16
            // 码元偏移
            if f.is_empty() {
                // Java: length()-1 = -1 → substring 越界异常
                return Err(format!("fmFile 值为空串: {name}"));
            }
            let start = match f.find('"') {
                Some(i) => i + 1,
                None => 0,
            };
            let end = f.len() - 1;
            if start > end {
                return Err(format!("fmFile 引号越界: {f}"));
            }
            let stripped = f[start..end].to_string();
            if stripped.is_empty() {
                // Java: charAt(0) 越界异常
                return Err(format!("fmFile 剥引号后为空: {f}"));
            }
            fmfile = if stripped.as_bytes()[0] == b'/' {
                Some(stripped[1..].to_string())
            } else {
                Some(stripped)
            };
        }
    }
    if fmfile.is_none() {
        // 中央文件里没写 fmFile → 按目录约定回退
        fmfile = Some(format!("fm/{name}.blk"));
    }
    let mut fmfile = fmfile.unwrap();
    if !fmfile.contains(".blk") {
        // Java: if (-1 == fmfile.indexOf(".blk")) fmfile += ".blk";
        fmfile.push_str(".blk");
    }

    // 5. 全量解析物理 FM 文件（物理文件 = fmfile + "x"，即 .blkx）
    // parse_named = Java 两参构造器 (doLoad=true): getload 全量装载 —
    // engineNum/peakThr/comp*/翼数据/vne 族齐备; getload 内 panic (畸形文件)
    // 由构造器 catch_unwind 收敛 Err → 此处 CORRUPT (Java valid=false 同位)
    let physical = fm_data_paths::physical_file(&format!("{fmfile}x"));
    let mut blkx = match Blkx::parse_named(&physical.to_string_lossy(), &fmfile) {
        Ok(b) => b,
        Err(_) => {
            // 中央文件在库但物理文件缺失/解析失败 → CORRUPT（数据不完整）
            logger::warn("FMLoader", &format!("FM文件不存在或解析失败: {name}"));
            return Ok(FMHandle::corrupt(Some(name.to_string())));
        }
    };

    // 6. plot 数据解析同样可能抛异常，必须留在 try 内（第二条循环路径）
    // TODO(port): getAllplotdata (Blkx.java L1618) 属 reader.rs 后续波次
    // (getplotdata/transUnit 未译, 见 blkx/mod.rs 方法波次边界清单); 落地后在此
    // 接入, 其异常路径由本函数 Result 通道承接
    blkx.finalize_loading();

    // 7. 按发动机类型提取派生数据（与旧 loadFMData 一致）
    if is_piston_engine(Some(&blkx)) {
        let stages = extract_stages_with_fuel(Some(&blkx), fuel_mod.as_ref());
        // 多发飞机乘引擎数（与喷气推力计算口径一致）
        // PORT: Java double * int 提升 double → as f64 (§2.4)
        // PORT: Java extractStages 意外返回 null 时 peakWepPower(null) 首行
        // `if (stages == null || stages.length == 0) return 0` 守卫返回 0,
        // 不抛 NPE —— 产出 READY + stages=null/peakWep=0 的降级句柄 (PistonPowerModel
        // L390-392)。None→空切片走 peak_wep_power 同一提前返回, 两语言失败模式
        // 一致; 不变量: is_piston_engine 已保证 comp_num_steps>0 → extract_stages
        // 必 Some, None 分支当前不可达 (谓词互补, fm_power_extractor.rs)
        let peak_wep = peak_wep_power(stages.as_deref().unwrap_or(&[])) * blkx.engine_num as f64;
        Ok(FMHandle::ready(
            Some(name.to_string()),
            Some(blkx),
            peak_wep,
            0.0,
            stages,
        ))
    } else {
        // 喷气机固定取加力峰值推力
        // PORT: 参数求值顺序 — 先取值再移动 blkx (Rust 实参左到右求值会先移走)
        let peak = blkx.peak_thrust(true);
        Ok(FMHandle::ready(
            Some(name.to_string()),
            Some(blkx),
            0.0,
            peak,
            None,
        ))
    }
}

/// catch(Throwable) 通道的载荷 — 携带诊断串以喂给
/// [`logger::error_with_throwable`] (复刻 Java "message: getMessage" 双拼 +
/// DEBUG 级 printStackTrace 形态)。
// PORT: Rust 错误域 (parse 的 Err<String> / panic 载荷) 无 Java 形堆栈,
/// Display ≡ getMessage, Debug repr 顶 printStackTrace 位 (logger.rs 同款注)
struct LoadThrowable(String);

// std::error::Error 的 Debug bound (打印 `{t:?}` 时输出同 Display 形态即可)
impl std::fmt::Debug for LoadThrowable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for LoadThrowable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LoadThrowable {}

/// panic 载荷提取 (catch_unwind 的 `Box<dyn Any + Send>`)
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

/// Java `"" + double` 字符串拼接 (Double.toString) 形态 — 整数值带 ".0" 尾
/// (50.0 而非 50), 供日志文本逐字保真。
// PORT: 完整复刻在 configuration_service::java_double_to_string (私有, 不越文件
// 引用); 此处按域收窄 — sovietOctaneHpBonus 来自 FM 文件 addHorsePowers 行,
/// 为小整数或短小数, `{:.1}`(整数)/`{}`(小数) 两分支与 Java 一一对应
fn java_double_str(d: f64) -> String {
    if d.is_finite() && d == d.trunc() && d.abs() < 1e16 {
        format!("{d:.1}")
    } else {
        format!("{d}")
    }
}

// =====================================================================
// Tests — 对应 Java: test/TestFMStore.java 的 FMLoader 面 (FMManager 异步
// 用例 ①~⑥ 属 FMManager 波次); 合成数据方案逐字移植 (不依赖真机 data/)。
// 另补边界: UNRESOLVED 计数、fmFile 回退/无后缀/燃油改装分支。
//
// 数据根策略 (PORT): cargo test 在同测试二进制内并行跑 #[test],
// fm_data_paths::tests::java_main_sequence 会临时翻转全局 DATA_ROOT
// (testroot/otherroot, Drop 恢复回 "./data")。load 内部 central_file 与
// physical_file 各读一次 DATA_ROOT, "前后双检默认根 + 重试"无法闭合单次
// load 内部的翻转窗口 (双检均通过但结果被污染, 审查 B blocker) —— 改为
// **多根铺数据**: 合成文件铺满 DATA_ROOT 的全部可能取值 (ROOTS), load 在
// 任何时刻读任何根, 命中/缺失判定恒定: 既无 flaky fail, 也无 "错误根下
// 恰同结果" 的假通过窗口。共享串行锁 (crate::fm::test_guard) 已备位,
// 本测试挂锁; java_main_sequence 本波次禁改 fm_data_paths.rs 无法接入
// (接入仅一行, 见 test_guard 模块注释), 接入后铺根可退化为单根。
// ⚠ 铺根依赖 java_main_sequence 的字面量根名 (其 Java 对拍期望值, 变更
// 概率极低); 若其改名, 本测试在未铺的新根下 READY 判 MISSING → flaky
// fail (fail loud, 不是假通过)。
// LOAD_COUNT 全局计数 (W-B2 备案): 未来 FMManager 波次异步用例若并行调
// load, 会污染 get_load_count()==N 断言 —— 届时须挂同一把 test_guard 锁。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::status::FMStatus;

    /// Java check(boolean, String) 计数式断言 → assert! 宏, 描述逐字保留
    fn check(cond: bool, desc: &str) {
        assert!(cond, "FAIL: {desc}");
    }

    // ---- 合成数据 (TestFMStore.setupSyntheticData 一比一移植) ----

    /// DATA_ROOT 的全部可能取值: 默认根 + java_main_sequence 临时注入的根
    /// (见模块注释; load 读取时任取其一/中途切换均有效)
    const ROOTS: [&str; 3] = ["./data", "testroot", "otherroot"];

    /// 各候选根下的 flightmodels 目录 (中央文件所在)
    fn fm_dir_of(root: &str) -> String {
        format!("{root}/aces/gamedata/flightmodels")
    }

    /// 机型名统一 zzfmload_ 前缀: 各根下绝不与真机 FM / 其他测试文件重名
    fn write_central(root: &str, name: &str) {
        // 最小中央文件 —— 只需 getlastone("fmfile") 能命中（参考真机文件头 fmFile:t = "fm/xxx.blk"）
        let content = format!("model:t = \"{name}\"\nfmFile:t = \"fm/{name}.blk\"\n");
        std::fs::write(format!("{}/{name}.blkx", fm_dir_of(root)), content).unwrap();
    }

    /// 中央文件无 fmFile 行 → 触发目录约定回退
    fn write_central_no_fmfile(root: &str, name: &str) {
        let content = format!("model:t = \"{name}\"\n");
        std::fs::write(format!("{}/{name}.blkx", fm_dir_of(root)), content).unwrap();
    }

    /// 中央文件 fmFile 值不带 .blk 后缀 → 触发补后缀分支
    fn write_central_noext(root: &str, name: &str) {
        let content = format!("model:t = \"{name}\"\nfmFile:t = \"fm/{name}\"\n");
        std::fs::write(format!("{}/{name}.blkx", fm_dir_of(root)), content).unwrap();
    }

    /// 中央文件带苏联燃油改装块 → 触发 extractFuelModifications + info 日志分支
    fn write_central_fuel(root: &str, name: &str) {
        let content = format!(
            "model:t = \"{name}\"\nfmFile:t = \"fm/{name}.blk\"\nmodifications {{\n\tussr_fuel_b-100 {{\n\t\teffects {{\n\t\t\taddHorsePowers:r = 50\n\t\t}}\n\t}}\n}}\n"
        );
        std::fs::write(format!("{}/{name}.blkx", fm_dir_of(root)), content).unwrap();
    }

    /// 最小物理 FM —— 非空且不以 '{' 开头即可全量解析：
    /// getload 对缺失字段全部按 0 处理（无 Jet/Compressor 块 → 按喷气形态、compNumSteps=0，
    /// extractStages 返回 null、peakThrust=0），最终 valid=true → READY。
    fn write_physical(root: &str, name: &str) {
        let content = format!("synthetic-fm:t = \"{name}\"\nEmptyMass:r = 1000\nWingspan:r = 11\n");
        std::fs::write(format!("{}/fm/{name}.blkx", fm_dir_of(root)), content).unwrap();
    }

    fn setup_synthetic_data() {
        for root in ROOTS {
            std::fs::create_dir_all(format!("{}/fm", fm_dir_of(root))).unwrap();

            // 可加载机型: central 指向 fm/<name>.blk, 物理文件存在
            write_central(root, "zzfmload_plane1");
            write_physical(root, "zzfmload_plane1");
            // fmFile 回退: central 无 fmFile → fm/<name>.blk 约定
            write_central_no_fmfile(root, "zzfmload_fb");
            write_physical(root, "zzfmload_fb");
            // fmFile 无 .blk 后缀: 剥引号后补 ".blk"
            write_central_noext(root, "zzfmload_nb");
            write_physical(root, "zzfmload_nb");
            // 燃油改装: soviet b-100 addHorsePowers=50
            write_central_fuel(root, "zzfmload_fuel");
            write_physical(root, "zzfmload_fuel");

            // CORRUPT 机型: central 在库但物理文件缺失
            write_central(root, "zzfmload_badplane");

            // ghost: 什么都不写 → MISSING
        }
    }

    /// 清理: 只删本测试落盘的文件; 目录仅在其为空时移除 (绝不触动既有 data/ 内容)
    fn cleanup_synthetic_data() {
        for root in ROOTS {
            for name in [
                "zzfmload_plane1",
                "zzfmload_fb",
                "zzfmload_nb",
                "zzfmload_fuel",
                "zzfmload_badplane",
            ] {
                let _ = std::fs::remove_file(format!("{}/{name}.blkx", fm_dir_of(root)));
                let _ = std::fs::remove_file(format!("{}/fm/{name}.blkx", fm_dir_of(root)));
            }
            // 自内向外逐层 prune 空目录 (remove_dir 对非空目录失败即止)
            for dir in [
                format!("{}/fm", fm_dir_of(root)),
                fm_dir_of(root),
                format!("{root}/aces/gamedata"),
                format!("{root}/aces"),
                root.to_string(),
            ] {
                let _ = std::fs::remove_dir(dir);
            }
        }
    }

    /// Drop 兜底清理 (断言 panic 展栈时也还原 cwd 下的合成文件)
    struct CleanupOnDrop;
    impl Drop for CleanupOnDrop {
        fn drop(&mut self) {
            cleanup_synthetic_data();
        }
    }

    /// TestFMStore 的 FMLoader 面 + 边界补充, 一次顺序执行
    /// (DATA_ROOT 竞态已由多根铺数据免疫, 见模块注释 —— 直接 load, 无需重试)
    fn run_cases() {
        reset_load_count();

        // -- 空名守卫: null / "" → UNRESOLVED 且不计入 loadCount --
        let h = load(None);
        check(h.status == FMStatus::Unresolved && h.name.is_none(), "null → UNRESOLVED");
        let h = load(Some(""));
        check(h.status == FMStatus::Unresolved, "空串 → UNRESOLVED");
        check(get_load_count() == 0, "空名不进入加载流程 (loadCount 不增)");

        // -- READY: central + physical 齐全 (大小写规范化) --
        let h = load(Some("ZZFMLOAD_PLANE1"));
        check(h.status == FMStatus::Ready, "合成齐全机型应 READY");
        check(h.name.as_deref() == Some("zzfmload_plane1"), "机型名规范化为小写");
        check(h.has_fm() && h.blkx.is_some(), "READY 句柄应携带 blkx");
        // readFileName 传参链锁死 (物理侧; 消费者 ui_model/fm_data_adapter.rs
        // get_fm_version —— 中央侧 name+".blk" 进 getload 版本串, 波次未落地
        // 暂无观察点)
        check(
            h.blkx.as_ref().unwrap().read_file_name.as_deref() == Some("fm/zzfmload_plane1.blk"),
            "物理文件 readFileName = fmfile 相对路径 (Java L101)",
        );
        // PORT: getload 未落地 (try_load 步骤5 TODO) — 数值字段暂为 0,
        // getload 波次落地后此断言需更新为真实喷气/活塞口径
        check(h.peak_wep_power == 0.0 && h.peak_thrust == 0.0, "getload 未落地: 功率/推力暂为 0");
        check(h.compressor_stages.is_none(), "无 Compressor 块 → stages 为 None");
        check(h.blkx.as_ref().unwrap().data.is_none(), "finalizeLoading 后 data 应释放");
        check(get_load_count() == 1, "READY 路径 loadCount=1");

        // -- CORRUPT: central 在库但物理文件缺失 (TestFMStore badplane) --
        let h = load(Some("zzfmload_badplane"));
        check(h.status == FMStatus::Corrupt, "物理文件缺失应为 CORRUPT");
        check(h.is_missing_like() && !h.has_fm(), "CORRUPT 属 missing-like 且无 FM");

        // -- MISSING: 什么都不放 (TestFMStore ghost) --
        let h = load(Some("zzfmload_ghost"));
        check(h.status == FMStatus::Missing, "不在库机型应为 MISSING");
        check(h.is_missing_like(), "MISSING 属 missing-like");
        check(h.name.as_deref() == Some("zzfmload_ghost"), "MISSING 保留机型名");

        // -- fmFile 回退: central 未写 fmFile → fm/<name>.blk 约定 --
        let h = load(Some("zzfmload_fb"));
        check(h.status == FMStatus::Ready, "目录约定回退应命中物理文件");

        // -- fmFile 无 .blk 后缀 → 剥引号后补 ".blk" --
        let h = load(Some("zzfmload_nb"));
        check(h.status == FMStatus::Ready, "无后缀 fmFile 补 .blk 后应命中");

        // -- 燃油改装分支: soviet b-100 检出 (info 日志) 且不阻断加载 --
        let h = load(Some("zzfmload_fuel"));
        check(h.status == FMStatus::Ready, "带燃油改装的中央文件仍应 READY");

        check(get_load_count() == 6, "六次有效加载 (READY x4 + CORRUPT + MISSING)");
    }

    #[test]
    fn loader_contract_synthetic() {
        // DATA_ROOT 测试串行锁 (test_guard): 与未来接入该锁的 DATA_ROOT 相关
        // 测试互斥 (java_main_sequence 未接入期间的翻转免疫靠多根铺数据)
        let _guard = crate::fm::test_guard::data_root();
        let _cleanup = CleanupOnDrop;
        setup_synthetic_data();
        run_cases();
    }

    /// 边界: java_double_str 复刻 Java Double.toString 拼接形态 (日志文本保真)
    #[test]
    fn java_double_str_matches_java_concat() {
        assert_eq!(java_double_str(50.0), "50.0");
        assert_eq!(java_double_str(0.0), "0.0");
        assert_eq!(java_double_str(1.5), "1.5");
        assert_eq!(java_double_str(-0.0), "-0.0");
    }

    /// panic 载荷提取的边界形态
    #[test]
    fn panic_message_payload_kinds() {
        let p: Box<dyn std::any::Any + Send> = Box::new("boom");
        assert_eq!(panic_message(p.as_ref()), "boom", "&str 载荷");
        let p: Box<dyn std::any::Any + Send> = Box::new(String::from("bang"));
        assert_eq!(panic_message(p.as_ref()), "bang", "String 载荷");
        let p: Box<dyn std::any::Any + Send> = Box::new(42i32);
        assert_eq!(panic_message(p.as_ref()), "unknown panic payload", "非字符串载荷");
    }
}
