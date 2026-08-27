//! 对应 Java: `src/prog/fm/FMStatus.java` (一比一翻译)
//! PORT: Java 枚举常量全大写 (UNRESOLVED/NOT_AIRCRAFT) → Rust 驼峰
//! (Unresolved/NotAircraft), 语义不变 (sexp_parser.rs 同款先例);
//! Java 枚举默认 toString()=常量名 的字符串形态由 Display 保留 (Java 8 oracle 对拍)。

use std::fmt;

/// FM（飞行数据包）加载状态机的六种状态（P2 单一真相源架构，issue #55 死循环重构）。
///
/// <p>状态语义：
/// <ul>
///   <li>{@link #UNRESOLVED} —— 尚未识别到机型（没有 live 数据也没有配置默认机），
///       一切从这里开始。</li>
///   <li>{@link #LOADING} —— 后台线程正在加载中；期间 {@link FMManager#current()}
///       仍返回旧句柄（平滑过渡），本状态只表达"有任务在途"。</li>
///   <li>{@link #READY} —— FM 解析成功，{@link FMHandle#blkx} 可用。</li>
///   <li>{@link #MISSING} —— 中央文件（&lt;dataRoot&gt;/aces/gamedata/flightmodels/&lt;name&gt;.blkx）
///       不存在，确认该机型不在数据库中。</li>
///   <li>{@link #CORRUPT} —— 中央文件存在但后续解析失败（物理 fm 文件缺失 / 构造异常 /
///       getAllplotdata 抛错等）。</li>
///   <li>{@link #NOT_AIRCRAFT} —— 识别到的是非飞机载具（陆战坦克/军舰等，type 带
///       "tankmodels/" 之类路径前缀）。FM 数据库只有 flightmodels，这类目标不是
///       "数据缺失"而是"根本不适用"：不发加载任务、不进负缓存、不弹缺失 toast，
///       HUD 端按 hasFM()=false 正常降级。</li>
/// </ul>
///
/// <p>{@link #MISSING} 与 {@link #CORRUPT} 统称 "missing-like"（见
/// {@link FMHandle#isMissingLike()}），二者都会进入 {@link FMManager} 的负缓存，
/// 杜绝旧架构"每次轮询都重试坏机型"的风暴。{@link #NOT_AIRCRAFT} 刻意不属于
/// missing-like：无数据问题、无需重试、不该打扰用户。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FMStatus {
    Unresolved,
    Loading,
    Ready,
    Missing,
    Corrupt,
    NotAircraft,
}

/// 对应 Java 枚举默认 `toString()` = 常量名 (`name()`)。
/// `FMHandle.toString()` 的字符串拼接依赖此形态。
// PORT: Java 8 oracle 实测 (build/oracle, 临时文件已删): 六态 toString/name 均
// 为声明名, NOT_AIRCRAFT 含下划线
impl fmt::Display for FMStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FMStatus::Unresolved => "UNRESOLVED",
            FMStatus::Loading => "LOADING",
            FMStatus::Ready => "READY",
            FMStatus::Missing => "MISSING",
            FMStatus::Corrupt => "CORRUPT",
            FMStatus::NotAircraft => "NOT_AIRCRAFT",
        };
        f.write_str(s)
    }
}

// =====================================================================
// Tests — FMStatus 无 Java 独立测试文件; 公共面 (Display=Java toString 形态)
// 按"每个公共函数写边界测试"规则补齐。六态互异由 Rust enum 判别式唯一性 +
// 派生 PartialEq 编译期保证, 不写空转测试 (§5)。
// =====================================================================
#[cfg(test)]
mod tests;
