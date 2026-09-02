//! 对应 Java: `src/prog/fm/FMHandle.java` (一比一翻译)

use std::sync::Mutex;

use crate::fmdata::{FmData, EngineLoad};
use crate::fm::status::FMStatus;
use crate::fm::piston_model::CompressorStageParams;

// PORT: 原 BlkxPlaceholder 零字段占位已按 blkx/mod.rs 字段波次陷阱注 5 的排期
// (构造点波次 = FMLoader 波次) 兑现切换为真实 crate::fmdata::FmData 聚合 struct;
// engLoad 会话态的就地改写已由 eng_load_state 字段承接 (见下)。

/// 不可变的 FM 句柄 —— "当前飞机的 FM 加载结果"的单一真相（P2 重构，取代 Controller 上
/// 分散的 Blkx/loadedFMName/identifiedFMName/failedFMName 四个手动同步的变量）。
///
/// <p>一个句柄完整描述一次加载的结果：机型名、状态、解析好的 {@link parser.Blkx}、
/// 以及由 FM 派生的功率/推力缓存。换机 = 换一个新句柄实例，旧句柄保持不可变，
/// 不存在"半新半旧"的中间态。
///
/// <p><b>共享会话状态说明</b>：{@code blkx.engLoad} 是 Service 线程在飞行过程中就地改写的
/// 共享会话状态（水温/油温计时等）。本类刻意<b>不拷贝</b>这层状态——因为换机必然产生
/// 新的 FMHandle → 新的 Blkx 实例，"换机 = 新实例"的语义天然保证会话状态不会串机，
/// 无需额外防御。
/// PORT(会话态提升): Java 的会话态真身挂在 Blkx.engLoad 上 (句柄仅引用共享);
/// Rust 侧 blkx 经 Arc<FMHandle> 共享只读, 会话态上提为句柄字段
/// {@code eng_load_state} (ready() 从解析产物克隆初始化, blkx 保持不可变) ——
/// "换机 = 新句柄实例" 的不串机保证原样保留。
///
/// <p>构造只经静态工厂，字段全 final，线程安全（volatile 发布由 {@link FMManager} 负责）。
// PORT: Java final 类 + 全 final 字段 → Rust 字段无 mut 即不可变; #[derive(Clone)]
// 对应 Java "引用可自由赋值传递" 的可用性 —— 注意 Java 赋值是 O(1) 共享, Rust Clone
// 是深拷贝。已裁决 (fm_manager.rs "Arc 共享在此销号"): 分发走 Arc<FMHandle>
// 共享, Clone 仅为字段级可用性保留。
// PORT: 刻意不 derive PartialEq —— Java FMHandle 无 equals 覆写, 语义只有引用同一性,
// 且全库无句柄 == 比较使用点 (审查已 grep 确认); 后续批次 (FMManager 等) 勿顺手补
// derive, 以免与 Java 引用语义分叉。
// PORT(会话态提升, blkx/mod.rs 字段波次陷阱注 5 的兑现): Java Blkx.engLoad 是
// "解析产物 + 会话态" 混合体 (getload 的 initEngineLoad 产出, 运行期被 Service
// ~10Hz 就地改写 curWater/OilWorkTimeMili); Rust 侧 blkx 经 Arc<FMHandle> 共享
// 只读, 无法落写 —— 会话态提升到本句柄的 eng_load_state (内部可变性 Mutex),
// blkx 本体保持不可变解析产物。初始化全在 ready() 内完成 (blkx.eng_load 克隆),
// FMLoader/fm_manager 调用方零改动。
// PORT: §0.7 pub 字段结构体无法复刻 "私有构造器 + 仅静态工厂" 的编译期约束,
// 工厂仍是规范构造入口 (调用方约定, 语义不变)。
#[derive(Debug)]
pub struct FMHandle {
    /// 规范化小写机型名（toLowerCase+trim）；UNRESOLVED 时为 null
    pub name: Option<String>,
    /// 加载结果状态
    pub status: FMStatus,
    /// 解析完成的 FM 对象；仅 {@link FMStatus#READY} 时非 null
    pub fmdata: Option<FmData>,
    /// 活塞机 WEP 峰值功率（hp，已乘引擎数）；非活塞/未就绪为 0
    pub peak_wep_power: f64,
    /// 喷气机加力峰值推力（kgf）；活塞机/未就绪为 0
    pub peak_thrust: f64,
    /// 活塞机多级增压器参数；喷气机/未就绪为 null
    pub compressor_stages: Option<Vec<CompressorStageParams>>,
    /// engLoad 会话态（Java Blkx.engLoad 的运行期改写面）：水温/油温耐久计时，
    /// Service 线程 checkOverheat ~10Hz 递减/恢复、resetEngLoad 回满。
    /// 仅 READY 句柄非 None（ready() 从 blkx.eng_load 克隆初始化，档位数 =
    /// blkx.max_eng_load，blkx 不可变故二者恒一致）；锁内纯计算无 IO，
    /// 中毒穿透 unwrap_or_else(into_inner) 对齐 §6 宽松契约。
    pub eng_load_state: Mutex<Option<Vec<EngineLoad>>>,
}

// PORT(会话态 Mutex): std::sync::Mutex 无 Clone → derive(Clone) 改手写 impl ——
// 锁内克隆会话态当前值。上注已声明 Clone 是字段级可用性保留 (分发主径 Arc 共享,
// 全库无按值 clone 使用点), 语义 = 会话态快照随副本独立 (Java 引用赋值共享同一
// Blkx 会话态, 与 Arc 共享同构, 不经此路径)。
impl Clone for FMHandle {
    fn clone(&self) -> Self {
        FMHandle {
            name: self.name.clone(),
            status: self.status,
            fmdata: self.fmdata.clone(),
            peak_wep_power: self.peak_wep_power,
            peak_thrust: self.peak_thrust,
            compressor_stages: self.compressor_stages.clone(),
            eng_load_state: Mutex::new(
                self.eng_load_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone(),
            ),
        }
    }
}

impl FMHandle {
    /// 哨兵句柄：未识别到机型时的初始值。字段值恒为
    /// name=null / status=UNRESOLVED / blkx=null / 功率推力全 0。
    // PORT: Java `public static final FMHandle UNRESOLVED = new FMHandle(null,
    // FMStatus.UNRESOLVED, null, 0, 0, null)` (经私有构造器的单例共享引用) →
    // 关联常量 (§1 static final 常量→const); 值全空且不可变, 常量内联与单例共享
    // 无行为差异 (Java 侧无人对句柄做引用同一性 == 比较)。
    // PORT(allow 借用警告): eng_load_state (Mutex) 使常量含内部可变性 —
    // clippy 建议 static 化, 但 const 的按值内联语义被全库使用点依赖
    // (struct 更新语法等), 保 const 形态
    #[allow(clippy::declare_interior_mutable_const, clippy::borrow_interior_mutable_const)]
    pub const UNRESOLVED: FMHandle = FMHandle {
        name: None,
        status: FMStatus::Unresolved,
        fmdata: None,
        peak_wep_power: 0.0,
        peak_thrust: 0.0,
        compressor_stages: None,
        // 无 blkx 即无会话态 (常量语义: 每次 mention 皆新值, Mutex 不可共享误用)
        eng_load_state: Mutex::new(None),
    };

    /// 加载成功句柄（仅 READY 允许携带 blkx）
    // PORT: Java 引用类型参数 (String/Blkx/数组) 隐式可传 null → 显式 Option (§1)
    pub fn ready(
        name: Option<String>,
        fmdata: Option<FmData>,
        peak_wep_power: f64,
        peak_thrust: f64,
        compressor_stages: Option<Vec<CompressorStageParams>>,
    ) -> FMHandle {
        // 会话态初始化 (会话态提升的唯一种子点): 从解析产物克隆, blkx 本体
        // 保持不可变; blkx=None 或 eng_load=None (initEngineLoad 未产出) → None
        // (先取后 move, struct 字面量内 blkx 已被字段吃掉)
        let eng_load_state = Mutex::new(fmdata.as_ref().and_then(|b| b.eng_load.clone()));
        FMHandle {
            name,
            status: FMStatus::Ready,
            fmdata,
            peak_wep_power,
            peak_thrust,
            compressor_stages,
            eng_load_state,
        }
    }

    /// 中央文件确认不存在
    pub fn missing(name: Option<String>) -> FMHandle {
        FMHandle {
            name,
            status: FMStatus::Missing,
            fmdata: None,
            peak_wep_power: 0.0,
            peak_thrust: 0.0,
            compressor_stages: None,
            eng_load_state: Mutex::new(None),
        }
    }

    /// 非飞机载具（陆战坦克/军舰等，type 带路径前缀如 "tankmodels/..."）。
    /// FM 不适用而非数据缺失：不进负缓存、不触发缺失 toast（见 {@link #isMissingLike()}）。
    pub fn not_aircraft(name: Option<String>) -> FMHandle {
        FMHandle {
            name,
            status: FMStatus::NotAircraft,
            fmdata: None,
            peak_wep_power: 0.0,
            peak_thrust: 0.0,
            compressor_stages: None,
            eng_load_state: Mutex::new(None),
        }
    }

    /// 存在但解析失败（物理文件缺失 / 解析异常）
    pub fn corrupt(name: Option<String>) -> FMHandle {
        FMHandle {
            name,
            status: FMStatus::Corrupt,
            fmdata: None,
            peak_wep_power: 0.0,
            peak_thrust: 0.0,
            compressor_stages: None,
            eng_load_state: Mutex::new(None),
        }
    }

    /// 是否持有可用的 FM 数据。
    /// 注意不要直接判 {@code status == READY} 以外的字段——blkx 为 null 的句柄
    /// （UNRESOLVED/LOADING/MISSING/CORRUPT）对调用方一律视为"无 FM"。
    pub fn has_fm(&self) -> bool {
        self.status == FMStatus::Ready && self.fmdata.is_some()
    }

    /// 是否属于"缺失类"状态（MISSING 或 CORRUPT）。
    /// 这类结果会进 {@link FMManager} 的负缓存，是 issue #55 死循环的根治点；
    /// Controller 也以本方法为闸门弹缺失 toast。
    /// 注意 NOT_AIRCRAFT 刻意不在其中——坦克/军舰不是数据缺失，不该被当飞机提示。
    pub fn is_missing_like(&self) -> bool {
        self.status == FMStatus::Missing || self.status == FMStatus::Corrupt
    }
}

/// 对应 Java `toString()` 覆写: `"FMHandle[" + status + " " + name + "]"`。
// PORT: Java 字符串拼接把 null 引用转为 "null" (JLS 字符串转换), 枚举拼接调
// 默认 toString()=常量名 —— Option::unwrap_or("null") + FMStatus 的 Display 复刻。
// Java 8 oracle 实测对拍见 tests::java8_oracle_tostring。
impl std::fmt::Display for FMHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FMHandle[{} {}]",
            self.status,
            self.name.as_deref().unwrap_or("null")
        )
    }
}

// =====================================================================
// Tests — 对应 Java: test/TestFMHandle.java (一比一移植)
// =====================================================================
#[cfg(test)]
mod tests;
