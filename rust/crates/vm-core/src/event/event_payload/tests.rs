use super::*;

// Java Builder 字段初始化器缺省值 (EventPayload.java:39-47) 逐项核对
#[test]
fn test_builder_defaults() {
    let p = EventPayload::builder().build();
    assert_eq!(p.map_grid, "--");
    assert!(!p.fatal_warn);
    assert!(!p.radio_alt_valid);
    assert!(!p.is_downing_flap);
    assert_eq!(p.time_str, "--:--");
    assert!(!p.is_jet);
    assert!(!p.engine_check_done);
    assert_eq!(p.optimal_compressor_stage, -1);
    assert!(!p.compressor_stage_mismatch);
}

// 每个 setter 覆盖对应缺省值; 链式调用 (Java return this) 语义
#[test]
fn test_builder_setters_override_defaults() {
    let p = EventPayload::builder()
        .map_grid("A1".to_string())
        .fatal_warn(true)
        .radio_alt_valid(true)
        .is_downing_flap(true)
        .time_str("12:34".to_string())
        .is_jet(true)
        .engine_check_done(true)
        .optimal_compressor_stage(2)
        .compressor_stage_mismatch(true)
        .build();
    assert_eq!(p.map_grid, "A1");
    assert!(p.fatal_warn);
    assert!(p.radio_alt_valid);
    assert!(p.is_downing_flap);
    assert_eq!(p.time_str, "12:34");
    assert!(p.is_jet);
    assert!(p.engine_check_done);
    assert_eq!(p.optimal_compressor_stage, 2);
    assert!(p.compressor_stage_mismatch);
}

// 单个 setter 只影响自己的字段 (其余保持缺省)
#[test]
fn test_builder_partial_set() {
    let p = EventPayload::builder().is_jet(true).build();
    assert!(p.is_jet);
    assert_eq!(p.map_grid, "--");
    assert_eq!(p.time_str, "--:--");
    assert_eq!(p.optimal_compressor_stage, -1);
}

// 公有 9 参构造器按参赋值 (与 Builder 无关的独立入口)
#[test]
fn test_constructor_assigns_all_fields() {
    let p = EventPayload::new(
        "B2".to_string(),
        true,
        false,
        true,
        "00:01".to_string(),
        false,
        true,
        3,
        false,
    );
    assert_eq!(p.map_grid, "B2");
    assert!(p.fatal_warn);
    assert!(!p.radio_alt_valid);
    assert!(p.is_downing_flap);
    assert_eq!(p.time_str, "00:01");
    assert!(!p.is_jet);
    assert!(p.engine_check_done);
    assert_eq!(p.optimal_compressor_stage, 3);
    assert!(!p.compressor_stage_mismatch);
}

// Java build() 不消耗 Builder: 同一 Builder 两次 build 产物相等
#[test]
fn test_build_is_repeatable() {
    let b = EventPayload::builder().map_grid("X".to_string());
    assert_eq!(b.build(), b.build());
}

// 每次调用 EventPayload::builder() 返回独立实例, 互不串扰
#[test]
fn test_builder_fresh_instance_each_call() {
    let _a = EventPayload::builder().is_jet(true);
    let b = EventPayload::builder().build();
    assert!(!b.is_jet);
}
