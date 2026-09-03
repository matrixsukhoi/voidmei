//! 引擎类型枚举 (波21 自 vm-data service_fields 迁入 — vm-core 内
//! flight_analyzer 等不再需要 i32 当枚举)。

/// Java `ENGINE_TYPE_*` int 常量的枚举收敛 (波17 F1)。
/// 序列化兼容: `as_i32()` 输出与原常量数值逐一致 — Prop=0 / Jet=1 /
/// Turboprop=2 / Unknown=-1。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EngineType {
    Unknown,
    Jet,
    Prop,
    Turboprop,
}

impl EngineType {

    /// Java isJetEngine: 仅喷气
    pub fn is_jet(self) -> bool {
        matches!(self, EngineType::Jet)
    }

    /// Java isPropEngine = PROP || TURBOPROP (is_piston 才是仅 PROP;
    /// 曾漏 TURBOPROP 致涡桨机 is_prop_engine 恒 false — 语义随方法收敛)
    pub fn is_prop(self) -> bool {
        matches!(self, EngineType::Prop | EngineType::Turboprop)
    }

    /// Java isPistonEngine: 仅活塞
    pub fn is_piston(self) -> bool {
        matches!(self, EngineType::Prop)
    }

    /// Java isTurbopropEngine: 仅涡桨
    pub fn is_turboprop(self) -> bool {
        matches!(self, EngineType::Turboprop)
    }
}
