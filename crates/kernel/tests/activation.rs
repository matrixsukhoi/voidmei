//! overlay 激活策略黑盒场景 (kernel::activation)。
//! ActivationStrategy 预设工厂 + and/or/not 组合 (Java @FunctionalInterface
//! 的谓词组合语义) + OverlayContext 对 ActivationContext 的适配面。
#![allow(non_snake_case)] // 测试名 = 模块前缀英文 + 中文场景 (风格约定, 豁免蛇形)

use std::collections::HashMap;

use expect_test::expect;
use kernel::activation::strategy::{ActivationContext, ActivationStrategy};
use kernel::activation::context::OverlayContext;
use kernel::config::config_api::ConfigProvider;

// ---- 测试用上下文 (trait 的最小 mock: 四个谓词源可控) ----

struct Ctx {
    cfg: HashMap<&'static str, bool>,
    jet: bool,
    preview: bool,
    fmdata: bool,
}

impl Ctx {
    fn new(jet: bool, preview: bool, fmdata: bool) -> Self {
        Ctx {
            cfg: HashMap::new(),
            jet,
            preview,
            fmdata,
        }
    }
}

impl ActivationContext for Ctx {
    fn get_bool(&self, key: &str) -> bool {
        self.cfg.get(key).copied().unwrap_or(false)
    }
    fn is_debug(&self) -> bool {
        false // Application.debug 全库零写入点 → 恒 false (源码注释裁决)
    }
    fn is_jet(&self) -> bool {
        self.jet
    }
    fn is_preview_mode(&self) -> bool {
        self.preview
    }
    fn has_fmdata(&self) -> bool {
        self.fmdata
    }
}

/// 预设工厂 × 上下文变体的组合判定表 (preview/live 差异 + is_jet + FM 在场)
#[test]
fn 预设工厂_组合判定表() {
    let live_jet_fm = Ctx::new(true, false, true);
    let preview_prop = Ctx::new(false, true, false);
    let rows: Vec<(&str, &Ctx, bool)> = vec![
        ("always", &live_jet_fm, ActivationStrategy::always().should_activate(&live_jet_fm)),
        ("never", &live_jet_fm, ActivationStrategy::never().should_activate(&live_jet_fm)),
        (
            "live_only @ live",
            &live_jet_fm,
            ActivationStrategy::live_only().should_activate(&live_jet_fm),
        ),
        (
            "live_only @ preview",
            &preview_prop,
            ActivationStrategy::live_only().should_activate(&preview_prop),
        ),
        (
            "preview_only @ preview",
            &preview_prop,
            ActivationStrategy::preview_only().should_activate(&preview_prop),
        ),
        (
            "preview_only @ live",
            &live_jet_fm,
            ActivationStrategy::preview_only().should_activate(&live_jet_fm),
        ),
        (
            "jet_only @ jet",
            &live_jet_fm,
            ActivationStrategy::jet_only().should_activate(&live_jet_fm),
        ),
        (
            "jet_only @ prop",
            &preview_prop,
            ActivationStrategy::jet_only().should_activate(&preview_prop),
        ),
        (
            "fmdata_available @ fm",
            &live_jet_fm,
            ActivationStrategy::fmdata_available().should_activate(&live_jet_fm),
        ),
        (
            "fmdata_available @ no-fm",
            &preview_prop,
            ActivationStrategy::fmdata_available().should_activate(&preview_prop),
        ),
        (
            "debug_only (恒 false)",
            &live_jet_fm,
            ActivationStrategy::debug_only().should_activate(&live_jet_fm),
        ),
    ];
    let actual = rows
        .iter()
        .map(|(name, _, v)| format!("{name} = {v}"))
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        always = true
        never = false
        live_only @ live = true
        live_only @ preview = false
        preview_only @ preview = true
        preview_only @ live = false
        jet_only @ jet = true
        jet_only @ prop = false
        fmdata_available @ fm = true
        fmdata_available @ no-fm = false
        debug_only (恒 false) = false"#]]
    .assert_eq(&actual);
}

/// config 键策略 + and/or/not 组合律 (短路求值; 组合后原策略仍可用 = Arc 共享语义)
#[test]
fn 组合律_and_or_not与config键() {
    let mut ctx = Ctx::new(true, false, true);
    ctx.cfg.insert("crosshairSwitch", true);

    let cfg_on = ActivationStrategy::config("crosshairSwitch");
    let cfg_off = ActivationStrategy::config("unknownSwitch");
    let jet = ActivationStrategy::jet_only();

    // and: 双真才真
    assert!(cfg_on.and(&jet).should_activate(&ctx));
    assert!(!cfg_off.and(&jet).should_activate(&ctx)); // config false 短路
    // or: 一真即真
    assert!(cfg_off.or(&jet).should_activate(&ctx));
    assert!(!cfg_off.or(&ActivationStrategy::preview_only()).should_activate(&ctx));
    // not: 取反
    assert!(!cfg_on.not().should_activate(&ctx));
    assert!(cfg_off.not().should_activate(&ctx));

    // 组合后原策略仍可用 (Java "a.and(b) 后 a/b 均仍可用" 的引用语义)
    assert!(cfg_on.should_activate(&ctx));
    assert!(jet.should_activate(&ctx));

    // config 键读取走 ctx.get_bool (缺省键 → false)
    assert!(!ActivationStrategy::config("missing").should_activate(&ctx));
}

/// OverlayContext 适配面: get_bool 只认忽略大小写的整串 "true";
/// is_jet 需 FM 在场; is_debug 恒 false (真值钉板)
#[test]
fn overlay_context_适配与get_bool语义() {
    // ConfigProvider 最小桩: "k_true"→"true", "k_yes"→"yes", 其余 ""
    struct P;
    impl ConfigProvider for P {
        fn get_config(&self, key: &str) -> Option<String> {
            match key {
                "k_true" => Some("true".into()),
                "k_yes" => Some("yes".into()),
                "k_TRUE_case" => Some("TRUE".into()),
                _ => Some(String::new()),
            }
        }
        fn set_config(&self, _key: &str, _value: &str) {}
        fn is_field_disabled(&self, _key: &str) -> bool {
            false
        }
    }

    let mut fmdata = kernel::fm::data::FmData::default();
    fmdata.is_jet = true;
    // 字段全 pub, 直接字面量构造 (build() 需要 ControllerRef 泛型约束, 测试无 Controller)
    let ctx = OverlayContext::<(), ()> {
        tc: None,
        s: None,
        fmdata: Some(fmdata),
        is_preview_mode: false,
        config_provider: Some(std::sync::Arc::new(P)),
    };

    let actual = [
        // Boolean.parseBoolean 语义: 只认 "true" (忽略大小写), "yes"/空 → false
        ActivationStrategy::config("k_true").should_activate(&ctx).to_string(),
        ActivationStrategy::config("k_TRUE_case").should_activate(&ctx).to_string(),
        ActivationStrategy::config("k_yes").should_activate(&ctx).to_string(),
        ActivationStrategy::config("k_missing").should_activate(&ctx).to_string(),
        ActivationStrategy::jet_only().should_activate(&ctx).to_string(),
        ActivationStrategy::fmdata_available().should_activate(&ctx).to_string(),
        ActivationStrategy::live_only().should_activate(&ctx).to_string(),
        ActivationStrategy::debug_only().should_activate(&ctx).to_string(),
    ]
    .join("|");
    expect!["true|true|false|false|true|true|true|false"].assert_eq(&actual);
}
