//! 对应 Java: `src/prog/fm/FMHandle.java` (一比一翻译)

use crate::blkx::Blkx;
use crate::fm::status::FMStatus;
use crate::piston_power_model::CompressorStageParams;

// PORT: 原 BlkxPlaceholder 零字段占位已按 blkx/mod.rs 字段波次陷阱注 5 的排期
// (构造点波次 = FMLoader 波次) 兑现切换为真实 crate::blkx::Blx 聚合 struct;
// engLoad 会话态的就地改写仍待 reader 波次以内部可变性承接 (同注)。

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
///
/// <p>构造只经静态工厂，字段全 final，线程安全（volatile 发布由 {@link FMManager} 负责）。
// PORT: Java final 类 + 全 final 字段 → Rust 字段无 mut 即不可变; #[derive(Clone)]
// 对应 Java "引用可自由赋值传递" 的可用性 —— 注意 Java 赋值是 O(1) 共享, Rust Clone
// 是深拷贝。已裁决 (fm_manager.rs "Arc 共享在此销号"): 分发走 Arc<FMHandle>
// 共享, Clone 仅为字段级可用性保留。
// PORT: 刻意不 derive PartialEq —— Java FMHandle 无 equals 覆写, 语义只有引用同一性,
// 且全库无句柄 == 比较使用点 (审查已 grep 确认); 后续批次 (FMManager 等) 勿顺手补
// derive, 以免与 Java 引用语义分叉。
// PORT: blkx 已是真身; "engLoad 就地改写" 的会话态语义仍归 getload 波次
// (reader.rs 后续批次, 未译 — 见 realtests canary), 落地时以内部可变性承接。
// PORT: §0.7 pub 字段结构体无法复刻 "私有构造器 + 仅静态工厂" 的编译期约束,
// 工厂仍是规范构造入口 (调用方约定, 语义不变)。
#[derive(Debug, Clone)]
pub struct FMHandle {
    /// 规范化小写机型名（toLowerCase+trim）；UNRESOLVED 时为 null
    pub name: Option<String>,
    /// 加载结果状态
    pub status: FMStatus,
    /// 解析完成的 FM 对象；仅 {@link FMStatus#READY} 时非 null
    pub blkx: Option<Blkx>,
    /// 活塞机 WEP 峰值功率（hp，已乘引擎数）；非活塞/未就绪为 0
    pub peak_wep_power: f64,
    /// 喷气机加力峰值推力（kgf）；活塞机/未就绪为 0
    pub peak_thrust: f64,
    /// 活塞机多级增压器参数；喷气机/未就绪为 null
    pub compressor_stages: Option<Vec<CompressorStageParams>>,
}

impl FMHandle {
    /// 哨兵句柄：未识别到机型时的初始值。字段值恒为
    /// name=null / status=UNRESOLVED / blkx=null / 功率推力全 0。
    // PORT: Java `public static final FMHandle UNRESOLVED = new FMHandle(null,
    // FMStatus.UNRESOLVED, null, 0, 0, null)` (经私有构造器的单例共享引用) →
    // 关联常量 (§1 static final 常量→const); 值全空且不可变, 常量内联与单例共享
    // 无行为差异 (Java 侧无人对句柄做引用同一性 == 比较)。
    pub const UNRESOLVED: FMHandle = FMHandle {
        name: None,
        status: FMStatus::Unresolved,
        blkx: None,
        peak_wep_power: 0.0,
        peak_thrust: 0.0,
        compressor_stages: None,
    };

    /// 加载成功句柄（仅 READY 允许携带 blkx）
    // PORT: Java 引用类型参数 (String/Blkx/数组) 隐式可传 null → 显式 Option (§1)
    pub fn ready(
        name: Option<String>,
        blkx: Option<Blkx>,
        peak_wep_power: f64,
        peak_thrust: f64,
        compressor_stages: Option<Vec<CompressorStageParams>>,
    ) -> FMHandle {
        FMHandle {
            name,
            status: FMStatus::Ready,
            blkx,
            peak_wep_power,
            peak_thrust,
            compressor_stages,
        }
    }

    /// 中央文件确认不存在
    pub fn missing(name: Option<String>) -> FMHandle {
        FMHandle {
            name,
            status: FMStatus::Missing,
            blkx: None,
            peak_wep_power: 0.0,
            peak_thrust: 0.0,
            compressor_stages: None,
        }
    }

    /// 非飞机载具（陆战坦克/军舰等，type 带路径前缀如 "tankmodels/..."）。
    /// FM 不适用而非数据缺失：不进负缓存、不触发缺失 toast（见 {@link #isMissingLike()}）。
    pub fn not_aircraft(name: Option<String>) -> FMHandle {
        FMHandle {
            name,
            status: FMStatus::NotAircraft,
            blkx: None,
            peak_wep_power: 0.0,
            peak_thrust: 0.0,
            compressor_stages: None,
        }
    }

    /// 存在但解析失败（物理文件缺失 / 解析异常）
    pub fn corrupt(name: Option<String>) -> FMHandle {
        FMHandle {
            name,
            status: FMStatus::Corrupt,
            blkx: None,
            peak_wep_power: 0.0,
            peak_thrust: 0.0,
            compressor_stages: None,
        }
    }

    /// 是否持有可用的 FM 数据。
    /// 注意不要直接判 {@code status == READY} 以外的字段——blkx 为 null 的句柄
    /// （UNRESOLVED/LOADING/MISSING/CORRUPT）对调用方一律视为"无 FM"。
    pub fn has_fm(&self) -> bool {
        self.status == FMStatus::Ready && self.blkx.is_some()
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
mod tests {
    use super::*;

    /// Java check(boolean, String) 计数式断言 → assert! 宏 (失败即 panic), 描述逐字保留
    fn check(cond: bool, desc: &str) {
        assert!(cond, "FAIL: {desc}");
    }

    fn test_unresolved_sentinel() {
        // -- UNRESOLVED 哨兵字段值测试 --
        let h = &FMHandle::UNRESOLVED;
        check(h.name.is_none(), "哨兵 name 应为 null");
        check(h.status == FMStatus::Unresolved, "哨兵 status 应为 UNRESOLVED");
        check(h.blkx.is_none(), "哨兵 blkx 应为 null");
        check(h.peak_wep_power == 0.0, "哨兵 peakWepPower 应为 0");
        check(h.peak_thrust == 0.0, "哨兵 peakThrust 应为 0");
        check(h.compressor_stages.is_none(), "哨兵 compressorStages 应为 null");
        check(!h.has_fm(), "哨兵 hasFM 应为 false");
        check(!h.is_missing_like(), "哨兵 isMissingLike 应为 false");
    }

    fn test_ready_handle() {
        // -- READY 句柄语义测试 --
        // dummy Blkx: 路径不存在的文件 → 对象非 null 即可 (hasFM 只看 status 与 blkx 非空)
        // PORT: Java `new parser.Blkx("__no_such_file__.blkx", "dummy")` 构造出的
        // valid=false 空壳对象 ↔ Blkx::default() (字段全默认值)
        let dummy = Blkx::default();
        let stages = [CompressorStageParams::default()];
        let h = FMHandle::ready(
            Some("plane1".to_string()),
            Some(dummy),
            1850.5,
            0.0,
            Some(stages.to_vec()),
        );

        check(h.status == FMStatus::Ready, "ready() 工厂 status 应为 READY");
        check(h.name.as_deref() == Some("plane1"), "name 应保留规范化机型名");
        // PORT: Java `h.blkx == dummy` 引用同一性 → 所有权模型无引用同一性可判,
        // 退化为存在性检查 (值即传入值)
        check(h.blkx.is_some(), "blkx 应携带解析对象");
        check(h.peak_wep_power == 1850.5, "peakWepPower 应保留传入值");
        check(h.peak_thrust == 0.0, "活塞机 peakThrust 应为 0");
        // PORT: Java `h.compressorStages == stages` 引用同一性 → 内容相等
        // (CompressorStageParams 为 Copy 值, 内容比较等价)
        check(
            h.compressor_stages.as_deref() == Some(stages.as_slice()),
            "compressorStages 应保留传入引用",
        );
        check(h.has_fm(), "READY 且 blkx 非空 → hasFM 为 true");
        check(!h.is_missing_like(), "READY 不是 missing-like");

        // 喷气机形态: stages=null, peakThrust>0
        let jet = FMHandle::ready(
            Some("me262".to_string()),
            Some(Blkx::default()),
            0.0,
            1800.0,
            None,
        );
        check(
            jet.has_fm() && jet.compressor_stages.is_none() && jet.peak_thrust == 1800.0,
            "喷气机句柄: stages null / thrust 1800",
        );
    }

    fn test_missing_handle() {
        // -- MISSING 句柄语义测试 --
        let h = FMHandle::missing(Some("ghost".to_string()));
        check(h.status == FMStatus::Missing, "missing() 工厂 status 应为 MISSING");
        check(h.name.as_deref() == Some("ghost"), "name 应为机型名");
        check(h.blkx.is_none(), "MISSING 不携带 blkx");
        check(h.peak_wep_power == 0.0 && h.peak_thrust == 0.0, "MISSING 功率/推力应为 0");
        check(!h.has_fm(), "MISSING hasFM 应为 false");
        check(h.is_missing_like(), "MISSING isMissingLike 应为 true");
    }

    fn test_corrupt_handle() {
        // -- CORRUPT 句柄语义测试 --
        let h = FMHandle::corrupt(Some("badplane".to_string()));
        check(h.status == FMStatus::Corrupt, "corrupt() 工厂 status 应为 CORRUPT");
        check(h.name.as_deref() == Some("badplane"), "name 应为机型名");
        check(h.blkx.is_none(), "CORRUPT 不携带 blkx");
        check(!h.has_fm(), "CORRUPT hasFM 应为 false");
        check(h.is_missing_like(), "CORRUPT isMissingLike 应为 true");
    }

    /// NOT_AIRCRAFT: 非飞机载具（坦克/军舰）——无 FM 但也不是数据缺失，不该弹缺失 toast
    fn test_not_aircraft_handle() {
        // -- NOT_AIRCRAFT 句柄语义测试 (陆战坦克) --
        let h = FMHandle::not_aircraft(Some("tankmodels/us_n4a3e8_76_sherman".to_string()));
        check(h.status == FMStatus::NotAircraft, "notAircraft() 工厂 status 应为 NOT_AIRCRAFT");
        check(
            h.name.as_deref() == Some("tankmodels/us_n4a3e8_76_sherman"),
            "name 应保留原始载具名",
        );
        check(h.blkx.is_none(), "NOT_AIRCRAFT 不携带 blkx");
        check(!h.has_fm(), "NOT_AIRCRAFT hasFM 应为 false (HUD 走降级)");
        check(
            !h.is_missing_like(),
            "NOT_AIRCRAFT 不是 missing-like (不进负缓存/不弹缺失 toast)",
        );
    }

    fn test_missing_like_semantics() {
        // -- isMissingLike 全枚举覆盖测试 --
        check(!FMHandle::UNRESOLVED.is_missing_like(), "UNRESOLVED 不是 missing-like");
        check(FMHandle::missing(Some("x".into())).blkx.is_none(), "MISSING 永不携带 blkx");
        check(FMHandle::corrupt(Some("x".into())).blkx.is_none(), "CORRUPT 永不携带 blkx");
        check(FMHandle::corrupt(Some("x".into())).is_missing_like(), "CORRUPT 属于 missing-like");
        check(FMHandle::missing(Some("x".into())).is_missing_like(), "MISSING 属于 missing-like");
        check(
            !FMHandle::not_aircraft(Some("x/y".into())).is_missing_like(),
            "NOT_AIRCRAFT 不属于 missing-like",
        );
    }

    #[test]
    fn unresolved_sentinel() {
        test_unresolved_sentinel();
    }

    #[test]
    fn ready_handle() {
        test_ready_handle();
    }

    #[test]
    fn missing_handle() {
        test_missing_handle();
    }

    #[test]
    fn corrupt_handle() {
        test_corrupt_handle();
    }

    #[test]
    fn not_aircraft_handle() {
        test_not_aircraft_handle();
    }

    #[test]
    fn missing_like_semantics() {
        test_missing_like_semantics();
    }

    /// 边界补充 (Java 测试未覆盖): status=READY 但 blkx=null 的防御分支 ——
    /// hasFM 必须为 false (javadoc 明示 "对调用方一律视为无 FM")
    #[test]
    fn ready_with_null_blkx_has_no_fm() {
        let h = FMHandle::ready(Some("x".to_string()), None, 100.0, 0.0, None);
        check(!h.has_fm(), "READY 但 blkx 为 null → hasFM 应为 false");
    }

    /// Java 8 oracle 对拍 (§5.1 A 类): toString 五形态 + 六态枚举名, 期望值为
    /// build/oracle/FMHandleOracle.java 在 OpenJDK 1.8.0_342 的实测 dump
    /// (临时文件, 用完已删除)。
    #[test]
    fn java8_oracle_tostring() {
        assert_eq!(FMHandle::UNRESOLVED.to_string(), "FMHandle[UNRESOLVED null]");
        assert_eq!(
            FMHandle::ready(
                Some("plane1".to_string()),
                Some(Blkx::default()),
                1850.5,
                0.0,
                Some(vec![CompressorStageParams::default()]),
            )
            .to_string(),
            "FMHandle[READY plane1]"
        );
        assert_eq!(
            FMHandle::missing(Some("ghost".to_string())).to_string(),
            "FMHandle[MISSING ghost]"
        );
        assert_eq!(
            FMHandle::not_aircraft(Some("tankmodels/us_n4a3e8_76_sherman".to_string())).to_string(),
            "FMHandle[NOT_AIRCRAFT tankmodels/us_n4a3e8_76_sherman]"
        );
        assert_eq!(
            FMHandle::corrupt(Some("badplane".to_string())).to_string(),
            "FMHandle[CORRUPT badplane]"
        );
    }
}
