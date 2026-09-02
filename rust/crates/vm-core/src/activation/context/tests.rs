// PORT: Java 保真 — 测试桩 Arc<RefCell<..>> 复刻 Java 引用共享 (单线程测试),
// 非 Send+Sync 是桩实现细节, 不改用 Mutex
#![allow(clippy::arc_with_non_send_sync)]

use super::*;
use crate::activation::strategy::ActivationStrategy;
use crate::base::bus::EventBus;
use std::cell::RefCell;
use std::collections::HashMap;

/// 测试替身: 内存键值表实现 ConfigProvider (config_provider.rs 测试同款)
struct MapConfig {
    values: RefCell<HashMap<String, String>>,
}

impl MapConfig {
    fn new() -> Self {
        MapConfig { values: RefCell::new(HashMap::new()) }
    }
}

impl ConfigProvider for MapConfig {
    fn get_config(&self, key: &str) -> Option<String> {
        self.values.borrow().get(key).cloned()
    }

    fn set_config(&self, key: &str, value: &str) {
        self.values.borrow_mut().insert(key.to_string(), value.to_string());
    }

    fn is_field_disabled(&self, _key: &str) -> bool {
        false
    }
}

/// 测试替身: Service 槽位占位 (真实 Service 在 vm-data, D6)
struct MockService;

/// 测试替身: 实现 ControllerRef 消费面 (真实 Controller 为 C 类步骤 16)
struct MockController {
    config: Arc<MapConfig>,
    service: Option<Arc<MockService>>,
}

impl ControllerRef<MockService> for MockController {
    fn service(&self) -> Option<Arc<MockService>> {
        self.service.clone()
    }
    fn get_config_service(&self) -> Arc<dyn ConfigProvider> {
        self.config.clone()
    }
}

fn mock_controller(k: &str, v: &str, with_service: bool) -> Arc<MockController> {
    let config = Arc::new(MapConfig::new());
    config.set_config(k, v);
    Arc::new(MockController {
        config,
        service: if with_service {
            Some(Arc::new(MockService))
        } else {
            None
        },
    })
}

// -- Builder 默认值 (Java 字段隐式 null/false, §2.10) --
#[test]
fn test_builder_defaults_match_java() {
    let ctx: OverlayContext<MockController, MockService> = Builder::new().build();
    assert!(ctx.tc.is_none(), "tc 默认 null");
    assert!(ctx.s.is_none(), "S 默认 null");
    assert!(ctx.fmdata.is_none(), "FmData 默认 null");
    assert!(!ctx.is_preview_mode, "isPreviewMode 默认 false");
    assert!(ctx.config_provider.is_none(), "configProvider 默认 null");
    // builder() 静态工厂与 Builder::new 等价 (Java OverlayContext.builder())
    let _: Builder<MockController, MockService> = OverlayContext::builder();
}

// -- get_bool = Boolean.parseBoolean(getConfig(key)) 的边界表 --
#[test]
fn test_get_bool_parse_boolean_semantics() {
    // (用例名, 存储值 None=键缺失, 期望)
    let cases: &[(&str, Option<&str>, bool)] = &[
        ("lower-true", Some("true"), true),
        ("upper-true", Some("TRUE"), true), // 忽略大小写
        ("mixed-true", Some("True"), true),
        ("false", Some("false"), false),
        ("empty", Some(""), false),
        ("leading-space", Some(" true"), false), // parseBoolean 不 trim (≠parseFloat, §6)
        ("trailing-space", Some("true "), false),
        ("yes", Some("yes"), false),
        ("one", Some("1"), false),
        ("cjk", Some("真"), false),
        ("missing-key-null", None, false), // parseBoolean(null) = false
    ];
    for (name, stored, expect) in cases {
        let config = Arc::new(MapConfig::new());
        if let Some(v) = stored {
            config.set_config("k", v);
        }
        let ctx: OverlayContext<MockController, MockService> = OverlayContext::builder()
            .config_provider(Some(config))
            .build();
        assert_eq!(ctx.get_bool("k"), *expect, "case {name}");
    }
}

// -- get_string 直通 (null 与值两形态) --
#[test]
fn test_get_string_passthrough() {
    let config = Arc::new(MapConfig::new());
    config.set_config("k", "v");
    let ctx: OverlayContext<MockController, MockService> = OverlayContext::builder()
        .config_provider(Some(config))
        .build();
    assert_eq!(ctx.get_string("k"), Some("v".to_string()));
    assert_eq!(ctx.get_string("nope"), None, "缺键 → null 透传");
}

// -- isJet: null 短路 + 字段跟随 --
#[test]
fn test_is_jet_null_short_circuit() {
    // Blkx == null → false (Java 短路, 无 NPE)
    let ctx: OverlayContext<MockController, MockService> = OverlayContext::builder().build();
    assert!(!ctx.is_jet());

    // Blkx 非 null + isJet=false → false
    let ctx: OverlayContext<MockController, MockService> = OverlayContext::builder()
        .fmdata(Some(FmData::default()))
        .build();
    assert!(!ctx.is_jet());

    // Blkx 非 null + isJet=true → true
    // (Blkx 含 blkx 模块树私有字段, 结构体字面量 FUS 不可用; is_jet 为 pub 可赋值)
    let mut jet_fmdata = FmData::default();
    jet_fmdata.is_jet = true;
    let ctx: OverlayContext<MockController, MockService> = OverlayContext::builder()
        .fmdata(Some(jet_fmdata))
        .build();
    assert!(ctx.is_jet());
}

// -- isDebug: Application.debug 无写入点 → 可观测行为恒 false (PORT 注实证) --
#[test]
fn test_is_debug_constant_false() {
    let ctx: OverlayContext<MockController, MockService> = OverlayContext::builder().build();
    assert!(!ctx.is_debug());
    assert!(
        !ActivationStrategy::debug_only().should_activate(&ctx),
        "debugOnly 预设在 Application.debug=false 下不激活"
    );
}

// -- build() 回退: 无显式 configProvider 时从 Controller 获取 --
#[test]
fn test_build_fallback_pulls_config_from_controller() {
    let tc = mock_controller("enableVoiceWarn", "true", true);
    let ctx = OverlayContext::builder()
        .controller(Some(Arc::clone(&tc)))
        // build() 回退只补 configProvider, S 仍按显式设置 (Java 同: forGameMode
        // 才显式链 .Service(tc.S))
        .service(tc.service())
        .build();
    assert!(ctx.config_provider.is_some(), "回退应写入 configProvider");
    assert!(ctx.get_bool("enableVoiceWarn"), "经回退的配置服务读到 true");
    // 引用同一性: ctx.s 与传入的是同一 Service 实例 (Java 共享引用)
    let svc = tc.service().unwrap();
    assert!(ctx.s.is_some() && Arc::ptr_eq(ctx.s.as_ref().unwrap(), &svc));
}

// -- build() 回退让位: 显式 configProvider 优先 --
#[test]
fn test_build_explicit_provider_wins() {
    let explicit = Arc::new(MapConfig::new());
    explicit.set_config("k", "false");
    let ctx = OverlayContext::builder()
        .controller(Some(mock_controller("k", "true", false)))
        .config_provider(Some(explicit))
        .build();
    assert!(!ctx.get_bool("k"), "显式 configProvider (false) 优先于 Controller 回退 (true)");
}

// -- build() 无 Controller 无回退: configProvider 保持 null, 读配置 NPE --
#[test]
#[should_panic(expected = "configProvider")]
fn test_get_bool_without_provider_panics_like_npe() {
    let ctx: OverlayContext<MockController, MockService> = OverlayContext::builder().build();
    let _ = ctx.get_bool("k");
}

#[test]
#[should_panic(expected = "configProvider")]
fn test_get_string_without_provider_panics_like_npe() {
    let ctx: OverlayContext<MockController, MockService> = OverlayContext::builder().build();
    let _ = ctx.get_string("k");
}

// -- build() 写回 builder 后可二次 build (Java builder 存活语义) --
#[test]
fn test_builder_reusable_after_build() {
    let mut b = OverlayContext::<MockController, MockService>::builder();
    b.controller(Some(mock_controller("k", "true", false)));
    let c1 = b.build();
    let c2 = b.build();
    assert!(c1.get_bool("k"));
    assert!(c2.get_bool("k"), "二次 build 同样携带回退的 configProvider");
}

// -- for_live: 全字段接线 (OverlayManager.java:94 的调用形态) --
#[test]
fn test_for_live_wiring() {
    let fm = FMManager::new(Arc::new(EventBus::new()));
    let tc = mock_controller("showSpeedBar", "TRUE", true);
    let svc = tc.service().unwrap();
    let ctx = OverlayContext::for_live(&fm, Arc::clone(&tc));
    assert!(ctx.tc.is_some() && Arc::ptr_eq(ctx.tc.as_ref().unwrap(), &tc));
    assert!(ctx.s.is_some() && Arc::ptr_eq(ctx.s.as_ref().unwrap(), &svc));
    // 未 identify 的 FMManager → UNRESOLVED 句柄 blkx=null (javadoc: 消费方 null 容忍)
    assert!(ctx.fmdata.is_none());
    assert!(!ctx.is_preview_mode, "游戏模式 previewMode=false");
    assert!(ctx.get_bool("showSpeedBar"), "configProvider 自动取自 getConfigService");
}

// -- for_preview_mode: 仅 previewMode 翻转 (OverlayManager.java:122/203) --
#[test]
fn test_for_preview_mode_wiring() {
    let fm = FMManager::new(Arc::new(EventBus::new()));
    let tc = mock_controller("k", "true", false);
    let ctx = OverlayContext::for_preview_mode(&fm, Arc::clone(&tc));
    assert!(ctx.is_preview_mode, "预览模式 previewMode=true");
    assert!(ctx.fmdata.is_none());
    assert!(ctx.s.is_none(), "tc.S 为 null 时如实透传 null");
    assert!(ctx.get_bool("k"));
}

// -- ActivationContext 实现 (指定语义) --
#[test]
fn test_activation_context_impl_semantics() {
    let config = Arc::new(MapConfig::new());
    config.set_config("enableVoiceWarn", "true");
    let ctx: OverlayContext<MockController, MockService> = OverlayContext::builder()
        .config_provider(Some(config))
        .build();
    // get_bool: 只认忽略大小写的 "true"; 缺键 = parseBoolean(null) = false
    assert!(ActivationStrategy::config("enableVoiceWarn").should_activate(&ctx));
    assert!(!ActivationStrategy::config("noSuchKey").should_activate(&ctx));
    // has_blkx: Blkx 字段 null 检查
    assert!(!ActivationStrategy::fmdata_available().should_activate(&ctx));
    // is_preview_mode 字段 / is_jet 字段 (带配置: 组合链左侧 config 先求值, 无
    // provider 会像 Java 一样 NPE)
    let jet_config = Arc::new(MapConfig::new());
    jet_config.set_config("enableVoiceWarn", "true");
    let mut jet_fmdata = FmData::default();
    jet_fmdata.is_jet = true;
    let mut jet = OverlayContext::<MockController, MockService>::builder();
    jet.fmdata(Some(jet_fmdata)).preview_mode(true).config_provider(Some(jet_config));
    let jet_ctx = jet.build();
    assert!(ActivationStrategy::jet_only().should_activate(&jet_ctx));
    assert!(ActivationStrategy::preview_only().should_activate(&jet_ctx));
    assert!(!ActivationStrategy::live_only().should_activate(&jet_ctx));
    assert!(ActivationStrategy::fmdata_available().should_activate(&jet_ctx));
    // Controller.java:723 使用形态: config(...).and(gameModeOnly()) —— 预览态不激活
    let voice = ActivationStrategy::config("enableVoiceWarn").and(&ActivationStrategy::live_only());
    assert!(voice.should_activate(&ctx), "游戏态 + 配置 true → 激活");
    assert!(!voice.should_activate(&jet_ctx), "预览态一律不激活");
}
