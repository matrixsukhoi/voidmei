use super::*;
use std::cell::Cell;
use std::collections::HashMap;

/// 测试替身: 按 Java OverlayContext 的访问面语义实现。
/// get_bool 缺键返回 false (对应 getConfig 返回 null →
/// Boolean.parseBoolean(null) = false 的链路); 计数器供短路求值断言。
struct MockCtx {
    debug: bool,
    jet: bool,
    preview_mode: bool,
    blkx: bool,
    bools: HashMap<&'static str, bool>,
    get_bool_calls: Cell<usize>,
}

impl MockCtx {
    fn new() -> Self {
        MockCtx {
            debug: false,
            jet: false,
            preview_mode: false,
            blkx: false,
            bools: HashMap::new(),
            get_bool_calls: Cell::new(0),
        }
    }

    fn with_bool(mut self, key: &'static str, v: bool) -> Self {
        self.bools.insert(key, v);
        self
    }
}

impl ActivationContext for MockCtx {
    fn get_bool(&self, key: &str) -> bool {
        self.get_bool_calls.set(self.get_bool_calls.get() + 1);
        self.bools.get(key).copied().unwrap_or(false)
    }
    fn is_debug(&self) -> bool {
        self.debug
    }
    fn is_jet(&self) -> bool {
        self.jet
    }
    fn is_preview_mode(&self) -> bool {
        self.preview_mode
    }
    fn has_blkx(&self) -> bool {
        self.blkx
    }
}

#[test]
fn test_always_and_never() {
    // 任意上下文下恒真/恒假 (ctx 全 false 字段的边界)
    assert!(ActivationStrategy::always().should_activate(&MockCtx::new()));
    assert!(!ActivationStrategy::never().should_activate(&MockCtx::new()));
    // 全 true 字段同样恒定
    let mut all_on = MockCtx::new();
    all_on.debug = true;
    all_on.jet = true;
    all_on.preview_mode = true;
    all_on.blkx = true;
    assert!(ActivationStrategy::always().should_activate(&all_on));
    assert!(!ActivationStrategy::never().should_activate(&all_on));
}

#[test]
fn test_config_follows_get_bool() {
    // 键命中: 跟随配置值
    let t = MockCtx::new().with_bool("enableVoiceWarn", true);
    let f = MockCtx::new().with_bool("enableVoiceWarn", false);
    assert!(ActivationStrategy::config("enableVoiceWarn").should_activate(&t));
    assert!(!ActivationStrategy::config("enableVoiceWarn").should_activate(&f));
    // 键缺失: Boolean.parseBoolean(null) = false
    assert!(!ActivationStrategy::config("noSuchKey").should_activate(&t));
}

#[test]
fn test_context_flag_presets() {
    // debugOnly / jetOnly / blkxAvailable 逐字段开关
    let mut on = MockCtx::new();
    on.debug = true;
    assert!(ActivationStrategy::debug_only().should_activate(&on));
    assert!(!ActivationStrategy::debug_only().should_activate(&MockCtx::new()));

    on.jet = true;
    assert!(ActivationStrategy::jet_only().should_activate(&on));
    assert!(!ActivationStrategy::jet_only().should_activate(&MockCtx::new()));

    on.blkx = true;
    assert!(ActivationStrategy::blkx_available().should_activate(&on));
    assert!(!ActivationStrategy::blkx_available().should_activate(&MockCtx::new()));

    // previewOnly / gameModeOnly 互为补集 (ctx.isPreviewMode 字段)
    let mut preview = MockCtx::new();
    preview.preview_mode = true;
    assert!(ActivationStrategy::preview_only().should_activate(&preview));
    assert!(!ActivationStrategy::preview_only().should_activate(&MockCtx::new()));
    assert!(!ActivationStrategy::live_only().should_activate(&preview));
    assert!(ActivationStrategy::live_only().should_activate(&MockCtx::new()));
}

#[test]
fn test_and_truth_table() {
    // Java: this.shouldActivate(ctx) && other.shouldActivate(ctx)
    for (a, b, expect) in [(true, true, true), (true, false, false), (false, true, false), (false, false, false)] {
        let left = if a { ActivationStrategy::always() } else { ActivationStrategy::never() };
        let right = if b { ActivationStrategy::always() } else { ActivationStrategy::never() };
        assert_eq!(
            left.and(&right).should_activate(&MockCtx::new()),
            expect,
            "a={} b={}", a, b
        );
    }
}

#[test]
fn test_or_truth_table() {
    // Java: this.shouldActivate(ctx) || other.shouldActivate(ctx)
    for (a, b, expect) in [(true, true, true), (true, false, true), (false, true, true), (false, false, false)] {
        let left = if a { ActivationStrategy::always() } else { ActivationStrategy::never() };
        let right = if b { ActivationStrategy::always() } else { ActivationStrategy::never() };
        assert_eq!(
            left.or(&right).should_activate(&MockCtx::new()),
            expect,
            "a={} b={}", a, b
        );
    }
}

#[test]
fn test_not_flips() {
    // Java: !this.shouldActivate(ctx)
    assert!(!ActivationStrategy::always().not().should_activate(&MockCtx::new()));
    assert!(ActivationStrategy::never().not().should_activate(&MockCtx::new()));
    let t = MockCtx::new().with_bool("k", true);
    assert!(!ActivationStrategy::config("k").not().should_activate(&t));
}

/// Java `&&` 短路: 左 (this) 为 false 时右 (other) 不求值。
#[test]
fn test_and_short_circuits_on_left_false() {
    let ctx = MockCtx::new().with_bool("k", true);
    assert!(!ActivationStrategy::never().and(&ActivationStrategy::config("k")).should_activate(&ctx));
    assert_eq!(ctx.get_bool_calls.get(), 0, "左假时 config 策略不应被求值");
}

/// Java `||` 短路: 左 (this) 为 true 时右 (other) 不求值。
#[test]
fn test_or_short_circuits_on_left_true() {
    let ctx = MockCtx::new().with_bool("k", true);
    assert!(ActivationStrategy::always().or(&ActivationStrategy::config("k")).should_activate(&ctx));
    assert_eq!(ctx.get_bool_calls.get(), 0, "左真时 config 策略不应被求值");
}

/// 左真时右侧确实求值 (且左侧求值在前, 各恰好一次)。
#[test]
fn test_and_evaluates_both_sides_when_left_true() {
    let ctx = MockCtx::new().with_bool("k", true);
    assert!(ActivationStrategy::always().and(&ActivationStrategy::config("k")).should_activate(&ctx));
    assert_eq!(ctx.get_bool_calls.get(), 1);
}

/// Java 语义: a.and(b) 之后 a 自身仍可调用、可再次参与组合 (引用不被消费)。
#[test]
fn test_receiver_reusable_after_combination() {
    let base = ActivationStrategy::config("k");
    let ctx_t = MockCtx::new().with_bool("k", true);
    let ctx_f = MockCtx::new().with_bool("k", false);

    let and_c = base.and(&ActivationStrategy::always());
    let or_c = base.or(&ActivationStrategy::never());
    // 组合产物行为正确
    assert!(and_c.should_activate(&ctx_t) && !and_c.should_activate(&ctx_f));
    assert!(or_c.should_activate(&ctx_t) && !or_c.should_activate(&ctx_f));
    // 原策略未被组合消费, 仍独立可用 (Java this 存活)
    assert!(base.should_activate(&ctx_t));
    assert!(!base.should_activate(&ctx_f));
}

/// Controller.java:723 实际使用形态: config(...).and(gameModeOnly()) ——
/// 预览模式下无论配置真假均不激活。
#[test]
fn test_controller_usage_composition() {
    let voice = ActivationStrategy::config("enableVoiceWarn").and(&ActivationStrategy::live_only());
    let game_on = MockCtx::new().with_bool("enableVoiceWarn", true);
    let game_off = MockCtx::new().with_bool("enableVoiceWarn", false);
    let mut preview_on = MockCtx::new().with_bool("enableVoiceWarn", true);
    preview_on.preview_mode = true;

    assert!(voice.should_activate(&game_on));
    assert!(!voice.should_activate(&game_off));
    assert!(!voice.should_activate(&preview_on));
}

/// OverlayManager.OverlayEntry 存储形态: 策略作为字段存入集合并按序调用;
/// 多层组合 (and→or→not) 语义手推: (a && jet) || debug 再取反。
#[test]
fn test_stored_and_chained_composition() {
    let chained = ActivationStrategy::config("a")
        .and(&ActivationStrategy::jet_only())
        .or(&ActivationStrategy::debug_only())
        .not();

    let mut jet_ctx = MockCtx::new().with_bool("a", true);
    jet_ctx.jet = true;
    let piston_ctx = MockCtx::new().with_bool("a", true);
    let mut off_jet = MockCtx::new(); // a 缺键=false, jet=true
    off_jet.jet = true;

    // a=T, jet=T, debug=F: config=T && jet=T → T; or debugOnly(F) → T; not → F
    assert!(!chained.should_activate(&jet_ctx));
    // a=T, jet=F: and=F; or debug(F)=F; not → T
    assert!(chained.should_activate(&piston_ctx));
    // a=F (缺键), jet=T: and=F; or debug(F)=F; not → T
    assert!(chained.should_activate(&off_jet));
    // 短路侧写: jet=T 时 and 已为真, or 右侧不求值, debug_only 无副作用可数;
    // config("a") 恰好各求值一次
    assert_eq!(jet_ctx.get_bool_calls.get(), 1);
    assert_eq!(piston_ctx.get_bool_calls.get(), 1);
    assert_eq!(off_jet.get_bool_calls.get(), 1);

    // 存储形态 (OverlayEntry.strategy 字段 → Vec 持有, 按序调用)
    let entries: Vec<ActivationStrategy> = vec![
        ActivationStrategy::config("a"),
        ActivationStrategy::live_only(),
        ActivationStrategy::always(),
    ];
    let results: Vec<bool> = entries.iter().map(|s| s.should_activate(&piston_ctx)).collect();
    assert_eq!(results, vec![true, true, true]);
}

/// 跨线程契约 (审查 B): ConfigDebounce 防抖线程携带 OverlayEntry 调用
/// shouldActivate (Controller.java:525-536/573-576 路径), 策略必须
/// Send+Sync —— 编译期静态断言锁死, 防止悄悄退回 Rc 而行为测试仍全绿。
#[test]
fn test_strategy_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ActivationStrategy>();

    // 组合产物 (嵌套闭包, 捕获物仅 Arc 与 String) 跨线程移动后在彼侧调用:
    // a=F: and(always)=F; or(never)=F; not → T
    let chained = ActivationStrategy::config("a")
        .and(&ActivationStrategy::always())
        .or(&ActivationStrategy::never())
        .not();
    let handle = std::thread::spawn(move || {
        let ctx = MockCtx::new().with_bool("a", false);
        chained.should_activate(&ctx)
    });
    assert!(handle.join().unwrap());
}
