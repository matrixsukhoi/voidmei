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
//! 纯静态工具类 → Rust 模块自由函数 (fm_data_paths.rs 同款先例); "唯一解析点"
//! = [`crate::fm::data::reader`] 全量装载 (blkx→json 迁移: FM 数据源为 JSON)。
//! PORT: 线程模型 — Java 在 FM-Loader 线程调用 → 保持同步函数, 线程由
//! Manager/调用方管 (无自起线程)。

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::fm::data::json::{extract_fuel_modifications_json, get_last_string_ci};
use crate::fm::data::{FmData, FuelModification, FuelType};
use crate::fm::data_paths;
use crate::fm::handle::FMHandle;
use crate::fm::power_extractor::{extract_stages_with_fuel, is_piston_engine};
use crate::base::logger;
use crate::fm::piston_model::peak_wep_power;
use crate::base::exception_helper::panic_message;

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
// (blkx→json 迁移已终态: 文本链已删, 只剩 JSON 链)
fn try_load(name: &str) -> Result<FMHandle, String> {
    try_load_json(name)
}

/// try_load 的 JSON 链 (blkx→json 迁移, 与文本链七步平行):
/// 中央 .json → serde 树直取 fmfile/燃油修正 (无引号剥壳) → 物理文件
/// 剥尾 .blk 拼 .json → parse_named_json (plotdata 已内含) → 派生同文本链。
fn try_load_json(name: &str) -> Result<FMHandle, String> {
    // 1. 中央文件不存在 → 确认机型不在库 → MISSING
    let central = data_paths::central_file(name);
    if !central.exists() {
        return Ok(FMHandle::missing(Some(name.to_string())));
    }

    // 2~4. 中央文件 serde 树: 燃油修正 + fmfile (CI 末次; JSON 字符串值本无
    // 引号, 免文本链的剥引号; 仅剥前导 '/')
    let mut fuel_mod: Option<FuelModification> = None;
    let mut fmfile: Option<String> = None;
    let parsed = std::fs::read_to_string(&central)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    if let Some(root) = parsed.as_ref() {
        let fm = extract_fuel_modifications_json(root);
        if fm.r#type != FuelType::None {
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
        fmfile = get_last_string_ci(root, "fmfile");
        if let Some(f) = fmfile.as_deref() {
            if f.is_empty() {
                return Err(format!("fmFile 值为空串: {name}"));
            }
            // 绝对路径 '/fm/...' → 剥前导斜杠回相对路径
            fmfile = Some(f.strip_prefix('/').unwrap_or(f).to_string());
        }
    }
    if fmfile.is_none() {
        // 中央文件里没写 fmFile → 按目录约定回退
        fmfile = Some(format!("fm/{name}.blk"));
    }
    let mut fmfile = fmfile.unwrap();
    if !fmfile.contains(".blk") {
        fmfile.push_str(".blk");
    }

    // 5. 全量解析物理 FM 文件 (JSON 版映射: 剥尾 .blk 拼 .json, 不再补 x;
    //    display_name 传映射前 fmfile 串 — read_file_name/fmdata 版本行与
    //    文本链逐字节一致, parity 同款 name 协议)
    let physical_name = format!(
        "{}.json",
        fmfile.strip_suffix(".blk").unwrap_or(&fmfile)
    );
    let physical = data_paths::physical_file(&physical_name);
    let fmdata = match FmData::parse_named_json(&physical.to_string_lossy(), &fmfile) {
        Ok(b) => b,
        Err(_) => {
            // 中央文件在库但物理文件缺失/解析失败 → CORRUPT（数据不完整）
            logger::warn("FMLoader", &format!("FM文件不存在或解析失败: {name}"));
            return Ok(FMHandle::corrupt(Some(name.to_string())));
        }
    };

    // 6. 按发动机类型提取派生数据（与文本链一致; finalizeLoading 已随原始
    //    data 串退役 — JSON 链 parse 内完成全部装载）
    if is_piston_engine(Some(&fmdata)) {
        let stages = extract_stages_with_fuel(Some(&fmdata), fuel_mod.as_ref());
        let peak_wep = peak_wep_power(stages.as_deref().unwrap_or(&[])) * fmdata.engine_num as f64;
        Ok(FMHandle::ready(
            Some(name.to_string()),
            Some(fmdata),
            peak_wep,
            0.0,
            stages,
        ))
    } else {
        // 喷气机固定取加力峰值推力 (先取值再移动 blkx)
        let peak = fmdata.peak_thrust();
        Ok(FMHandle::ready(
            Some(name.to_string()),
            Some(fmdata),
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

/// Java `"" + double` 字符串拼接 (Double.toString) 形态 — 整数值带 ".0" 尾
/// (50.0 而非 50), 供日志文本逐字保真。
// PORT: 完整复刻在 base::java_compat::java_double_to_string; 此处按域收窄 —
// sovietOctaneHpBonus 来自 FM 文件 addHorsePowers 行,
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
// data_paths::tests::java_main_sequence 会临时翻转全局 DATA_ROOT
// (testroot/otherroot, Drop 恢复回 "./data")。load 内部 central_file 与
// physical_file 各读一次 DATA_ROOT, "前后双检默认根 + 重试"无法闭合单次
// load 内部的翻转窗口 (双检均通过但结果被污染, 审查 B blocker) —— 改为
// **多根铺数据**: 合成文件铺满 DATA_ROOT 的全部可能取值 (ROOTS), load 在
// 任何时刻读任何根, 命中/缺失判定恒定: 既无 flaky fail, 也无 "错误根下
// 恰同结果" 的假通过窗口。共享串行锁 (crate::fm::test_support) 已备位,
// 本测试挂锁; java_main_sequence 本波次禁改 fm_data_paths.rs 无法接入
// (接入仅一行, 见 test_guard 模块注释), 接入后铺根可退化为单根。
// ⚠ 铺根依赖 java_main_sequence 的字面量根名 (其 Java 对拍期望值, 变更
// 概率极低); 若其改名, 本测试在未铺的新根下 READY 判 MISSING → flaky
// fail (fail loud, 不是假通过)。
// LOAD_COUNT 全局计数 (W-B2 备案): 未来 FMManager 波次异步用例若并行调
// load, 会污染 get_load_count()==N 断言 —— 届时须挂同一把 test_guard 锁。
// =====================================================================
#[cfg(test)]
mod tests;
