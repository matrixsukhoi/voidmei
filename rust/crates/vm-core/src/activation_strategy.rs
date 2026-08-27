//! 对应 Java: `src/prog/ActivationStrategy.java` (一比一翻译)
//!
//! overlay 激活条件谓词组合。Java `@FunctionalInterface` (唯一抽象方法
//! `shouldActivate` + default 组合方法 and/or/not + 静态预设工厂) → 持闭包的
//! 结构体 (§1 匿名类/lambda → 闭包)。使用点 (Controller/OverlayManager) 全部
//! 是内联 lambda 构造 + 按引用存储后调用, 闭包承载零成本; 组合方法取 `Arc`
//! 共享子闭包, 完整保留 Java "a.and(b) 后 a/b 均仍可用" 的引用语义。

use std::sync::Arc;

/// PORT: Java 参数类型 `prog.OverlayContext` 属后续翻译批次 (CLASSIFY 步骤 14,
/// B 类), 本 crate 尚无对应物 —— 此处按 ActivationStrategy 的实际访问面
/// (getBool/isDebug/isJet/isPreviewMode 字段/Blkx 字段 null 检查) 提取最小 trait。
// 已收口: overlay_context.rs 的 OverlayContext impl 即本 trait 的生产实现
// (get_bool = Boolean.parseBoolean(configProvider.getConfig(key)) —— 只认
// 不区分大小写的 "true", null/其余串均 false; has_blkx = self.Blkx.is_some()),
// 组合逻辑零改动。
pub trait ActivationContext {
    /// 对应 Java `OverlayContext.getBool(String)`。
    fn get_bool(&self, key: &str) -> bool;
    /// 对应 Java `OverlayContext.isDebug()`。
    fn is_debug(&self) -> bool;
    /// 对应 Java `OverlayContext.isJet()`。
    fn is_jet(&self) -> bool;
    /// 对应 Java public 字段 `OverlayContext.isPreviewMode` (字段访问 → trait 方法)。
    fn is_preview_mode(&self) -> bool;
    /// 对应 Java public 字段 null 检查 `ctx.Blkx != null`。
    fn has_blkx(&self) -> bool;
}

/// Strategy interface for determining if an overlay should be activated.
/// Replaces hardcoded conditions scattered throughout Controller.
// PORT: Java 函数式接口 → 包装 `Arc<dyn Fn>` 的结构体; shouldActivate 是唯一
// 抽象方法, 预设工厂直接产闭包。Arc 而非 Box: and/or/not 组合后原策略仍可用
// (Java 引用语义), 且 Arc 浅克隆 = Java 引用赋值 (Clone 派生即此意)。
// 刻意不 derive PartialEq —— Java 函数式接口无 equals 语义, 比较只有引用
// 同一性, 闭包无对应 (fm/handle.rs 同款裁决)。
// Arc + Send + Sync (而非 Rc): shouldActivate 的调用线程不止 EDT ——
// Controller.java:525-536/573-576 的 configDebouncer (单线程 daemon
// "ConfigDebounce") 防抖任务在后台线程调 overlayManager.refreshAllPreviews()
// → entry.refreshPreview(ctx) → strategy.shouldActivate(ctx)
// (OverlayManager.java:115/124/314-315/291); Controller.java:855 的
// refreshPreviews 亦注明只在后台线程调用 (LIFETIMES.md 线程清单含
// ConfigDebounce)。Send/Sync 只约束闭包捕获物 (String 与嵌套策略), 不会
// 约束按引用传入的 ActivationContext, 对步骤 14 的 OverlayContext 无泄漏。
#[derive(Clone)]
pub struct ActivationStrategy {
    // PORT: Java 保真 — `Predicate<ActivationContext>` 的 Rust 对应形态,
    // trait object + Send/Sync 约束为一体签名, 不拆 type 别名
    #[allow(clippy::type_complexity)]
    f: Arc<dyn Fn(&dyn ActivationContext) -> bool + Send + Sync>,
}

impl ActivationStrategy {
    /// Determine if the overlay should be activated based on context.
    // PORT: Java `OverlayContext ctx` 按引用传入 → `&dyn ActivationContext` (见 trait 注释)。
    pub fn should_activate(&self, ctx: &dyn ActivationContext) -> bool {
        (self.f)(ctx)
    }

    /// Combine this strategy with another using AND logic.
    // PORT: Java `a.and(b)` 后 a/b 均存活 → &self + &ActivationStrategy, Arc 克隆
    // 共享子闭包; `&&` 短路求值两语言一致且左操作数 (this) 先求值。
    pub fn and(&self, other: &ActivationStrategy) -> ActivationStrategy {
        let this = Arc::clone(&self.f);
        let other = Arc::clone(&other.f);
        ActivationStrategy {
            f: Arc::new(move |ctx| this(ctx) && other(ctx)),
        }
    }

    /// Combine this strategy with another using OR logic.
    // PORT: 同 and, `||` 短路求值左操作数 (this) 先求值。
    pub fn or(&self, other: &ActivationStrategy) -> ActivationStrategy {
        let this = Arc::clone(&self.f);
        let other = Arc::clone(&other.f);
        ActivationStrategy {
            f: Arc::new(move |ctx| this(ctx) || other(ctx)),
        }
    }

    /// Negate this strategy.
    pub fn not(&self) -> ActivationStrategy {
        let this = Arc::clone(&self.f);
        ActivationStrategy {
            f: Arc::new(move |ctx| !this(ctx)),
        }
    }

    // ========== Preset Strategies ==========

    /// Always activate.
    pub fn always() -> ActivationStrategy {
        ActivationStrategy {
            f: Arc::new(|_ctx| true),
        }
    }

    /// Never activate.
    pub fn never() -> ActivationStrategy {
        ActivationStrategy {
            f: Arc::new(|_ctx| false),
        }
    }

    /// Activate based on a boolean config key.
    // PORT: Java String 参数 → &str (cfg 键纯 ASCII, §2.1), 闭包持有 String。
    pub fn config(config_key: &str) -> ActivationStrategy {
        let key = config_key.to_string();
        ActivationStrategy {
            f: Arc::new(move |ctx| ctx.get_bool(&key)),
        }
    }

    /// Activate only in debug mode.
    pub fn debug_only() -> ActivationStrategy {
        ActivationStrategy {
            f: Arc::new(|ctx| ctx.is_debug()),
        }
    }

    /// Activate only for jet aircraft.
    pub fn jet_only() -> ActivationStrategy {
        ActivationStrategy {
            f: Arc::new(|ctx| ctx.is_jet()),
        }
    }

    /// Activate only in preview mode.
    pub fn preview_only() -> ActivationStrategy {
        ActivationStrategy {
            f: Arc::new(|ctx| ctx.is_preview_mode()),
        }
    }

    /// Activate only in live mode (not preview). 术语对齐: live=接入真实遥测数据
    /// (对位 Java isPreviewMode=false; 旧名 game_mode_only, 业界 preview↔live 对仗)
    pub fn live_only() -> ActivationStrategy {
        ActivationStrategy {
            f: Arc::new(|ctx| !ctx.is_preview_mode()),
        }
    }

    /// Activate when Blkx data is available.
    pub fn blkx_available() -> ActivationStrategy {
        ActivationStrategy {
            f: Arc::new(|ctx| ctx.has_blkx()),
        }
    }
}

// =====================================================================
// Tests — Java 侧无独立测试文件; 按"每个公共函数写边界测试"规则补齐。
// 期望值按 Java 语义手工推算: && / || 短路求值 (JLS 15.23/15.24, 左先右后)、
// OverlayContext.getBool 缺键 = Boolean.parseBoolean(null) = false。
// =====================================================================
#[cfg(test)]
mod tests;
