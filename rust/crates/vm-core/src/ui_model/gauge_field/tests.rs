use super::*;

/// 构造: configKey = "disable"+key; 继承字段走 DataField 默认
#[test]
fn constructor_derives_config_key() {
    let gf = GaugeField::new("throttle", "油门", "%", 3, 110, false);
    assert_eq!(gf.base.config_key, "disablethrottle", "configKey = \"disable\" + key");
    assert_eq!(gf.base.key, "throttle");
    assert_eq!(gf.base.label, "油门");
    assert_eq!(gf.base.unit, "%");
    assert!(!gf.base.hide_when_na);
    assert!(!gf.base.hide_when_zero);
    assert!(gf.base.visible);
    assert_eq!(gf.base.current_value, "---");
    assert_eq!(gf.gauge_type, 3);
    assert_eq!(gf.max_value, 110);
    assert_eq!(gf.current_int_value, 0);
    assert!(!gf.is_horizontal);
}

/// 构造: gauge 组件参数 = (label, maxValue, !isHorizontal); markedGauge null
#[test]
fn constructor_gauge_component_params() {
    let gf = GaugeField::new("compressor", "增压器", "", 0, 1, true);
    let gauge = gf.gauge.as_ref().expect("构造即挂 gauge 组件");
    assert_eq!(gauge.label, "增压器");
    assert_eq!(gauge.max_value, 1);
    assert!(!gauge.vertical, "vertical = !isHorizontal = false");
    assert!(gf.marked_gauge.is_none(), "markedGauge 默认 null");

    // 竖条形态: isHorizontal=false → vertical=true
    let gf2 = GaugeField::new("rpm", "转速", "RPM", 1, 3000, false);
    assert!(gf2.gauge.as_ref().unwrap().vertical);
}

/// updateGauge: 整数值 + 显示文本直赋 (不经 %5s 右对齐)
#[test]
fn update_gauge_sets_int_and_raw_text() {
    let mut gf = GaugeField::new("fuel", "燃油", "%", 2, 100, true);
    gf.update_gauge(64, " 64%");
    assert_eq!(gf.current_int_value, 64);
    assert_eq!(gf.base.current_value, " 64%", "displayText 原样落 currentValue");
    // 短文本不被 %5s 补齐 (与 DataField.setValue 行为对照)
    gf.update_gauge(7, "7");
    assert_eq!(gf.base.current_value, "7");
}

/// markedGauge 外部赋值通道 (EngineControlOverlay COMPRESSOR 档位用法)
#[test]
fn marked_gauge_external_assignment() {
    let mut gf = GaugeField::new("compressor", "增压器", "", 0, 1, true);
    gf.marked_gauge = Some(MarkedGaugePlaceholder);
    assert!(gf.marked_gauge.is_some());
}
