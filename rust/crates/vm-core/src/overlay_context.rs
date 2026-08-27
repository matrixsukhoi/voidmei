//! 对应 Java: `src/prog/OverlayContext.java` (一比一翻译)
//!
//! import 映射: `parser.Blkx` → [`crate::blkx::Blkx`];
//! `prog.config.ConfigProvider` → [`crate::config_api::ConfigProvider`];
//! `prog.fm.FMManager` → [`crate::fm::FMManager`];
//! `prog.Controller` / `prog.Service` → **泛型参数 TC/S** (见 [`ControllerRef`] 注)。

use std::sync::Arc;

use crate::blkx::Blkx;
use crate::config_api::ConfigProvider;
use crate::fm::FMManager;

/// `prog.Controller` 在本文件的消费面 trait (依赖桩)。
///
/// PORT: Java 字段 `Controller tc` / `Controller.S` / `Controller.getConfigService()`
/// 的两个依赖方向在本波次均不可达 —— Controller 是 C 类 (CLASSIFY.md: 编排核心,
/// 语义复刻不走逐行, 步骤 16 收口), Service 链落 vm-data (D6) 而 vm-core 不得反向
/// 依赖。故 OverlayContext 以泛型 `<TC, S>` 持柄 (§1 组合替代继承), 本 trait 只
/// 约束本文件实际访问面 (focus_monitor 波次 trait FocusDetector / ui_model 的
/// config_stub 同款先例): 真实 Controller 波次为本 trait 提供实现即可对接
/// build() 回退与两个快速工厂。
pub trait ControllerRef<S> {
    /// 对应 Java public 字段 `Controller.S` (Controller.java:90, 可 null 的 Service 引用)。
    fn service(&self) -> Option<Arc<S>>;
    /// 对应 Java `Controller.getConfigService()` (Controller.java:98, 返回
    /// ConfigurationService —— 即 ConfigProvider 实现的共享引用)。
    fn get_config_service(&self) -> Arc<dyn ConfigProvider>;
}

/// Context object containing all data overlays might need.
/// Eliminates the need to pass multiple parameters to overlay methods.
///
/// 配置访问通过 ConfigProvider 接口而不是 Controller，遵循依赖倒置原则。
// PORT: Java `public final` 引用字段 → pub 字段; 可 null 引用 → Option (§1),
// 共享语义 (多 overlay/线程持同一 Controller/Service/配置服务) → Arc。
// §0.7: pub 字段结构体无法复刻 "私有构造器 + Builder" 的编译期约束,
// Builder 仍是规范构造入口 (调用方约定, handle.rs 同款先例)。
// 纪律注 (LIFETIMES §4 环 2): Java 侧 ctx 是瞬时对象 —— 每次 openAll/
// refreshAllPreviews 新建、仅 ActivationStrategy 消费后即弃。Rust 侧必须保持
// 瞬时: 禁止把 ctx 或其 tc/s 的 Arc 克隆存入长命 overlay 字段 —— Controller
// (持 OverlayManager→overlays) 与 overlay(持 Arc<Controller>) 一旦成环, 无 GC
// 兜底, 每次托盘重建泄漏一整棵 Controller 树 (目标形态: overlay 持
// Arc<ConfigStore> + channel, LIFETIMES §4 首选方案)。
pub struct OverlayContext<TC, S> {
    pub tc: Option<Arc<TC>>,
    /// Java 字段名 `S` (单字母大写) → snake_case `s`
    pub s: Option<Arc<S>>,
    /// Java 字段名与类型同名 (`Blkx Blkx`) → 字段 `blkx`
    // PORT: Java 持 FMManager.current() 句柄内同一 Blkx 实例的引用拷 ——
    // Service 线程对 blkx.engLoad 等会话态的就地改写对持有者可见
    // (fm/fm_manager.rs:46); Rust 为构造时深拷快照 fork, 与 FMManager 内实例
    // 的后续突变互不可见。当前消费面仅 null 检查 + isJet() (grep 实证:
    // ActivationStrategy.java:91 / OverlayManager 全部使用点), 无会话态读取,
    // 保真成立。勿经本字段读 engLoad 会话态 —— 那是 Java 侧可做而此处静默
    // 分叉的用法; 后续波次确需会话态时复议 Option<Arc<Blkx>> (现被
    // fm/handle.rs blkx: Option<Blkx> 所有权形态挡住, 待 reader 波次内部
    // 可变性裁决一并处理)。
    pub blkx: Option<Blkx>,
    pub is_preview_mode: bool,
    /// 配置提供者，用于访问配置而不依赖 Controller
    // PORT: Java 接口引用 → Arc<dyn ConfigProvider> 共享句柄 (LIFETIMES §7
    // `config: Arc<ConfigStore>` 形态)。刻意不加 Send + Sync 约束: 现实现
    // ConfigurationService 经 config_loader::GroupConfig 含 Rc<SExp> (!Send,
    // configuration_service.rs L126 已备案的 Rc→Arc 待裁决项), 加约会把该
    // 接线提前堵死。注意 (审查 B 修正): trait object 未显式加 bound 时自动
    // trait 恒丢失 —— 无论具体实现是否 Send+Sync, 本签名下 OverlayContext
    // 恒 !Send+!Sync, 不存在 "跨线程搬运性随具体实现走"; Rc→Arc 裁决落地后
    // 须回改本签名为 `Arc<dyn ConfigProvider + Send + Sync>` (同 crate
    // flight_log.rs / flight_analyzer.rs 已用该拼写, 接线时统一) 才能解锁
    // ConfigDebounce 后台线程构造/搬运 ctx 的路径 (Java 实际存在: 防抖线程
    // 直接 refreshPreviews, LIFETIMES 审查 ★2)。
    pub config_provider: Option<Arc<dyn ConfigProvider>>,
}

// Java: private OverlayContext(Builder builder) —— 私有构造器由 build() 内的
// 结构体字面量对位 (§0.7: 编译期私有性不可复刻, 构造入口约定走 Builder)。
impl<TC, S> OverlayContext<TC, S> {
    /// Get a boolean config value.
    /// 通过 ConfigProvider 接口访问配置，而不是通过 Controller。
    // PORT: Java `Boolean.parseBoolean(s)` = 忽略大小写整串等于 "true" 才真
    // (null/空串/带空白/其余串均 false; 不 trim, 与 parseFloat 不同, §6);
    // eq_ignore_ascii_case 在 "true" 全 ASCII 域与 equalsIgnoreCase 逐字符等价。
    // configProvider 为 null 时 Java 在 getConfig 调用处 NPE → expect panic (§1)。
    pub fn get_bool(&self, key: &str) -> bool {
        self.config_provider
            .as_ref()
            .expect("OverlayContext.configProvider 为 null (Java NullPointerException)")
            .get_config(key)
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    /// Get a string config value.
    /// 通过 ConfigProvider 接口访问配置，而不是通过 Controller。
    // PORT: Java 返回可 null String → Option<String> (ConfigProvider 契约的
    // null/空串两种 "未设置" 形态原样透传); configProvider 为 null 同样 NPE → panic。
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.config_provider
            .as_ref()
            .expect("OverlayContext.configProvider 为 null (Java NullPointerException)")
            .get_config(key)
    }

    /// Check if this is a jet aircraft.
    // PORT: Java `Blkx != null && Blkx.isJet` 短路 → map_or (None 时不触字段)。
    pub fn is_jet(&self) -> bool {
        self.blkx.as_ref().is_some_and(|b| b.is_jet)
    }

    /// Check if debug mode is enabled.
    // PORT: Java 读全局静态 `Application.debug` (Application.java:60, C 类步骤 16
    // 收口, 本 crate 无对应物)。§2.9 禁在本文件自设 OnceLock 全局 (状态分裂),
    // 故先返回其声明初始值 —— 该字段全库零写入点 (grep 实证: 读者仅
    // Application.java:539 / Controller.java:228,262 / MainForm.java:336 / 本方法),
    // 生产可观测行为恒 false, 与此处返回值一致。
    // 已收口 (终态语义): Application.debug 全库零写入点 → 恒 false 即对齐;
    // env.debug 是 --debug 启动参 (对应 debugLog 而非本字段), 注入反而偏离 Java。
    pub fn is_debug(&self) -> bool {
        false
    }

    /// Create a builder.
    pub fn builder() -> Builder<TC, S> {
        Builder::new()
    }

    /// Quick factory for game mode context.
    /// configProvider 自动从 Controller 的 ConfigService 获取。
    ///
    /// <p>P3 迁移: Blkx 直读 FMManager 当前句柄（EDT 上的纯 volatile 读, 无锁无 IO）。
    /// 旧版经 Controller.getBlkx() 是 JIT 加载器, 可能在 EDT 触发同步文件解析;
    /// 现在加载由 FMManager.identify 后台驱动, 此处仅读结果。非 READY 句柄 blkx=null,
    /// 消费方（ActivationStrategy / isJet() 等）均已 null 容忍。
    // PORT: Java `FMManager.getInstance()` 单例已解散 (fm_manager.rs §2.9 裁决:
    // 显式构造注入) → 追加 fm 参数; `Controller tc` 引用形参 → Arc<TC> 共享柄。
    // current() 的 blkx 为句柄私有值, Java 引用拷贝 ↔ Rust clone (Blkx 深拷,
    // 低频路径: 仅 overlay 开启/刷新时构造, 非 ~10Hz 热点; engLoad 会话态共享
    // 由 FMManager 的 Arc<FMHandle> 层承载, 见 fm_manager.rs ★1 注)。
    pub fn for_game_mode(fm: &FMManager, tc: Arc<TC>) -> OverlayContext<TC, S>
    where
        TC: ControllerRef<S>,
    {
        OverlayContext::builder()
            .controller(Some(Arc::clone(&tc)))
            .service(tc.service())
            .blkx(fm.current().blkx.clone())
            .config_provider(Some(tc.get_config_service()))
            .preview_mode(false)
            .build()
    }

    /// Quick factory for preview mode context.
    /// configProvider 自动从 Controller 的 ConfigService 获取。
    ///
    /// <p>P3 迁移: 同 forGameMode, Blkx 直读 FMManager（非 READY 为 null, 消费方 null 容忍）。
    pub fn for_preview_mode(fm: &FMManager, tc: Arc<TC>) -> OverlayContext<TC, S>
    where
        TC: ControllerRef<S>,
    {
        OverlayContext::builder()
            .controller(Some(Arc::clone(&tc)))
            .service(tc.service())
            .blkx(fm.current().blkx.clone())
            .config_provider(Some(tc.get_config_service()))
            .preview_mode(true)
            .build()
    }
}

/// activation_strategy.rs 预留 TODO(port) 的兑现: OverlayContext 实现其按实际
/// 访问面提取的最小 trait (Java `ActivationStrategy.shouldActivate(OverlayContext)`
/// → Rust `&dyn ActivationContext`), 预设工厂零改动即可消费本上下文。
// PORT: 方法体转发到同名固有方法/字段 —— Rust 方法解析固有 impl 优先于 trait
// impl, 此处 self.get_bool(key) 调的是上方固有方法, 无自递归。
impl<TC, S> crate::activation_strategy::ActivationContext for OverlayContext<TC, S> {
    fn get_bool(&self, key: &str) -> bool {
        self.get_bool(key)
    }
    fn is_debug(&self) -> bool {
        self.is_debug()
    }
    fn is_jet(&self) -> bool {
        self.is_jet()
    }
    fn is_preview_mode(&self) -> bool {
        self.is_preview_mode
    }
    fn has_blkx(&self) -> bool {
        self.blkx.is_some()
    }
}

/// Builder for OverlayContext.
// PORT: Java `public static class Builder` 嵌套类 → 独立 struct; 可 null 引用
// 字段/参数 → Option (§1), 级联 `return this` → `&mut Self` (build 后 builder
// 存活可复用, 对齐 Java)。
pub struct Builder<TC, S> {
    // Java 字段隐式默认 null/false (§2.10) → new() 显式初始化
    pub tc: Option<Arc<TC>>,
    pub s: Option<Arc<S>>,
    // 同 OverlayContext.blkx 字段注: 构造时快照 fork, 勿读/勿依赖会话态。
    pub blkx: Option<Blkx>,
    pub is_preview_mode: bool,
    pub config_provider: Option<Arc<dyn ConfigProvider>>,
}

impl<TC, S> Builder<TC, S> {
    /// Java Builder 的隐式无参构造器 (字段全默认)。
    pub fn new() -> Self {
        Builder {
            tc: None,
            s: None,
            blkx: None,
            is_preview_mode: false,
            config_provider: None,
        }
    }

    /// Java `Builder Controller(Controller tc)`
    pub fn controller(&mut self, tc: Option<Arc<TC>>) -> &mut Self {
        self.tc = tc;
        self
    }

    /// Java `Builder Service(Service S)`
    pub fn service(&mut self, s: Option<Arc<S>>) -> &mut Self {
        self.s = s;
        self
    }

    /// Java `Builder Blkx(Blkx Blkx)`
    pub fn blkx(&mut self, blkx: Option<Blkx>) -> &mut Self {
        self.blkx = blkx;
        self
    }

    /// Java `Builder previewMode(boolean isPreviewMode)`
    pub fn preview_mode(&mut self, is_preview_mode: bool) -> &mut Self {
        self.is_preview_mode = is_preview_mode;
        self
    }

    /// 设置配置提供者，用于访问配置而不依赖 Controller。
    pub fn config_provider(&mut self, config_provider: Option<Arc<dyn ConfigProvider>>) -> &mut Self {
        self.config_provider = config_provider;
        self
    }

    /// Java `public OverlayContext build()` —— 非 static, 写回 this 后经私有构造器
    /// 组装 (builder 存活可复用)。
    // PORT: Java 构造器按引用拷贝 builder 字段 (`this.tc = builder.tc`) ↔ Rust
    // clone (Arc 克隆 O(1); Blkx 深拷, 低频构造路径, 见 for_game_mode 注)。
    pub fn build(&mut self) -> OverlayContext<TC, S>
    where
        TC: ControllerRef<S>,
    {
        // 如果没有显式设置 configProvider，从 Controller 获取
        if self.config_provider.is_none() {
            if let Some(tc) = self.tc.as_ref() {
                self.config_provider = Some(tc.get_config_service());
            }
        }
        OverlayContext {
            tc: self.tc.clone(),
            s: self.s.clone(),
            blkx: self.blkx.clone(),
            is_preview_mode: self.is_preview_mode,
            config_provider: self.config_provider.clone(),
        }
    }
}

impl<TC, S> Default for Builder<TC, S> {
    fn default() -> Self {
        Builder::new()
    }
}

// =====================================================================
// Tests — Java 侧无独立测试文件; 按"每个公共项写边界测试"规则补齐。
// 期望值按 Java 语义手工推算 (Boolean.parseBoolean / null 容忍 / 级联回退),
// 并覆盖 activation_strategy.rs TODO(port) 指定的 ActivationContext 实现语义。
// =====================================================================
#[cfg(test)]
mod tests;
