//! 对应 Java: `src/ui/model/GaugeField.java` (一比一翻译)

use crate::ui_model::data_field::DataField;

/// PORT: Java `ui.component.LinearGauge` / `ui.component.LabeledLinearGauge` 是
/// Swing 可视组件 (C 类, 归 vm-overlay 批次), vm-core 无渲染层 —— 先以占位类型
/// 顶住字段位 (fm::handle::BlkxPlaceholder 先例)。构造参数 (label, maxValue,
/// vertical) 是数据侧状态, 留存供 vm-overlay 批次消费; 渲染行为缺失。
// 已收口 (架构裁决): 视觉组件归 vm-overlay renderers.rs 侧缓存持有 (Java 组件
// 存活于本字段 → Rust 缓存+失效钩子), 数据侧不回流组件引用; 本占位零消费。
#[derive(Debug, Clone)]
pub struct LinearGaugePlaceholder {
    /// LabeledLinearGauge 构造参数: 显示标签
    pub label: String,
    /// LabeledLinearGauge 构造参数: 量程最大值
    pub max_value: i32,
    /// LabeledLinearGauge 构造参数: `!isHorizontal` (竖条取向)
    pub vertical: bool,
}

impl LinearGaugePlaceholder {
    /// 对应 Java `new ui.component.LabeledLinearGauge(label, maxValue, !isHorizontal)`
    pub fn new(label: &str, max_value: i32, vertical: bool) -> Self {
        LinearGaugePlaceholder {
            label: label.to_string(),
            max_value,
            vertical,
        }
    }
}

/// PORT: Java `ui.component.gauge.MarkedGauge` 同为 Swing 可视组件 (C 类)。
/// 其状态 (label/maxValue/barStyle/marker 表) 由 EngineControlOverlay 的
/// builder 链设置 (C 类消费点, 不在本批), 占位类型零字段。
// 已收口: 同 LinearGaugePlaceholder — 视觉面归 renderers.rs 缓存, 占位零消费。
#[derive(Debug, Clone, Default)]
pub struct MarkedGaugePlaceholder;

/// Data model for a linear gauge field.
/// Extends DataField to add gauge-specific properties like max value and
/// orientation.
// PORT: Java `extends DataField` → 组合 (§1 禁强行造继承)。注意多态使用点**存在**:
// ui.renderer.LinearGaugeRenderer (C 类) 的 render()/calculatePreferredSize()
// 遍历 List<DataField> 做 `instanceof GaugeField` 向下转型 (LinearGaugeRenderer
// .java:31/35/60/62)。本批组合无行为损失 (该 renderer 未译), 但 C 批移植
/// List<DataField> 时需引入 enum/注册表式判别通道替代 instanceof; 继承字段经
/// `base` 通道访问。
pub struct GaugeField {
    /// 继承自 DataField 的全部字段 (key/label/unit/configKey/hideWhenNA/
    /// hideWhenZero/visible/currentValue/零GC管线字段)
    pub base: DataField,

    pub gauge_type: i32,
    pub max_value: i32,
    pub current_int_value: i32,
    pub is_horizontal: bool,

    // Reference to the visual component for direct updates
    // PORT: Java 构造器赋值后恒非 null (updateGauge 的 null 判断是防御式) →
    // Option 承接字段位的可空语义, 构造时置 Some
    pub gauge: Option<LinearGaugePlaceholder>,

    // Optional MarkedGauge for gauges that need markers (e.g., compressor with optimal stage indicator)
    // PORT: Java 默认 null, 由 EngineControlOverlay 对 COMPRESSOR 档位外部赋值
    pub marked_gauge: Option<MarkedGaugePlaceholder>,
}

impl GaugeField {
    pub fn new(
        key: &str,
        label: &str,
        unit: &str,
        gauge_type: i32,
        max_value: i32,
        is_horizontal: bool,
    ) -> GaugeField {
        // Use key-based configKey, hideWhenNA=false, hideWhenZero=false
        // PORT: Java `"disable" + key` 字符串拼接
        let base = DataField::new(key, label, unit, &format!("disable{}", key), false, false);
        GaugeField {
            base,
            gauge_type,
            max_value,
            current_int_value: 0,
            is_horizontal,
            gauge: Some(LinearGaugePlaceholder::new(label, max_value, !is_horizontal)),
            marked_gauge: None,
        }
    }

    pub fn update_gauge(&mut self, value: i32, display_text: &str) {
        self.current_int_value = value;
        // PORT: Java 直赋 currentValue (不经 setValue 的 %5s 右对齐) —— 原样
        self.base.current_value = display_text.to_string();
        // PORT: Java `if (gauge != null) gauge.update(value, displayText);` ——
        // gauge 是 Swing 可视组件 (C 类, vm-overlay 批次), 占位类型无副作用可调;
        // 数据侧赋值已完整保留, null 判断无可观察行为 (构造即 Some)。
        // 已收口: 联动由 renderers.rs 侧组件缓存每帧 update 同步 (架构裁决)。
    }
}

#[cfg(test)]
mod tests;
