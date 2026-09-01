//! 对应 Java: `src/prog/event/EventPayload.java`
//! Type-safe payload for flight data events.
//! Replaces the untyped Map<String, String> for compile-time safety
//! and zero unnecessary String boxing of boolean/numeric values.
//!
//! Immutable — safe for cross-thread passing between Service and EDT.
//! PORT: Java final class + public final 字段 = Rust pub struct + pub 字段 (§0.7),
//! 不可变性由"构造后只经 &self 访问"保证 (Java 靠 final 引用)。
//! PORT: Java 无 equals 覆写 (引用等值); 此处 derive PartialEq 仅为测试基建
//! (fields.rs 先例), 不改变翻译逻辑。

/// Type-safe payload for flight data events. (radioAltValid 已删: 零消费方)
#[derive(Debug, Clone, PartialEq)]
pub struct EventPayload {
    pub map_grid: String,
    pub fatal_warn: bool,
    pub is_downing_flap: bool,
    pub time_str: String,
    pub is_jet: bool,
    pub engine_check_done: bool,
    /// Optimal compressor stage index (0-based). -1 indicates invalid/jet/single-stage.
    pub optimal_compressor_stage: i32,
    /// True when actual compressor stage doesn't match optimal (at full throttle).
    pub compressor_stage_mismatch: bool,
}

impl EventPayload {
    /// 对应 Java 公有构造器 `EventPayload(String, boolean, ..., boolean)`。
    // PORT: Java 保真 — 参数表逐个对应 Java 构造器形参, 不打包成结构体
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        map_grid: String,
        fatal_warn: bool,
        is_downing_flap: bool,
        time_str: String,
        is_jet: bool,
        engine_check_done: bool,
        optimal_compressor_stage: i32,
        compressor_stage_mismatch: bool,
    ) -> Self {
        EventPayload {
            map_grid,
            fatal_warn,
            is_downing_flap,
            time_str,
            is_jet,
            engine_check_done,
            optimal_compressor_stage,
            compressor_stage_mismatch,
        }
    }

    /// 对应 Java `public static Builder builder()`。
    pub fn builder() -> EventPayloadBuilder {
        // Java: return new Builder(); — 每次调用返回全新 Builder (字段取初始缺省值)
        EventPayloadBuilder::new()
    }
}

/// 对应 Java `EventPayload.Builder` (public static class)。
/// 缺省值逐字段对齐 Java 字段初始化器: mapGrid="--", timeStr="--:--",
/// optimalCompressorStage=-1, 其余 false。
#[derive(Debug, Clone)]
pub struct EventPayloadBuilder {
    map_grid: String,
    fatal_warn: bool,
    is_downing_flap: bool,
    time_str: String,
    is_jet: bool,
    engine_check_done: bool,
    optimal_compressor_stage: i32,
    compressor_stage_mismatch: bool,
}

impl Default for EventPayloadBuilder {
    /// 补齐 Rust 惯用 Default (语义同 `Builder::new()`, Java 侧无此概念)
    fn default() -> Self {
        Self::new()
    }
}

impl EventPayloadBuilder {
    /// 对应 Java Builder 默认构造器 `new Builder()` (public)。
    pub fn new() -> Self {
        EventPayloadBuilder {
            map_grid: "--".to_string(),
            fatal_warn: false,
            is_downing_flap: false,
            time_str: "--:--".to_string(),
            is_jet: false,
            engine_check_done: false,
            optimal_compressor_stage: -1,
            compressor_stage_mismatch: false,
        }
    }

    /// Java: `Builder mapGrid(String v) { this.mapGrid = v; return this; }`
    pub fn map_grid(mut self, v: String) -> Self {
        self.map_grid = v;
        self
    }

    pub fn fatal_warn(mut self, v: bool) -> Self {
        self.fatal_warn = v;
        self
    }

    pub fn is_downing_flap(mut self, v: bool) -> Self {
        self.is_downing_flap = v;
        self
    }

    pub fn time_str(mut self, v: String) -> Self {
        self.time_str = v;
        self
    }

    pub fn is_jet(mut self, v: bool) -> Self {
        self.is_jet = v;
        self
    }

    pub fn engine_check_done(mut self, v: bool) -> Self {
        self.engine_check_done = v;
        self
    }

    pub fn optimal_compressor_stage(mut self, v: i32) -> Self {
        self.optimal_compressor_stage = v;
        self
    }

    pub fn compressor_stage_mismatch(mut self, v: bool) -> Self {
        self.compressor_stage_mismatch = v;
        self
    }

    /// 对应 Java `EventPayload build()` — 只读字段组包, 不修改 Builder。
    pub fn build(&self) -> EventPayload {
        EventPayload::new(
            self.map_grid.clone(),
            self.fatal_warn,
            self.is_downing_flap,
            self.time_str.clone(),
            self.is_jet,
            self.engine_check_done,
            self.optimal_compressor_stage,
            self.compressor_stage_mismatch,
        )
    }
}

#[cfg(test)]
mod tests;
