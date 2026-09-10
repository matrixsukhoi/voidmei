use super::*;

/// 语义测试 (对应 Java 行为)
#[test]
fn sma_semantics() {
    let mut s = SimpleMovingAverage::new(3);
    assert_eq!(s.add_new_data(1.0), 1.0);
    assert_eq!(s.add_new_data(2.0), 1.5);
    assert_eq!(s.add_new_data(3.0), 2.0);
    assert!((s.add_new_data(5.0) - (2.0 + (5.0 - 1.0) / 3.0)).abs() < 1e-12);
}
