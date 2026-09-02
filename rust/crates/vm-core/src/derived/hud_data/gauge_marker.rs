//! GaugeMarker 的 Rust 移植 (src/ui/component/gauge/GaugeMarker.java)。
//!
//! PORT: java.awt.Color 数据字段 → [u8;4] RGBA (POC 先例);
//!      Builder 默认 Color.RED = (255,0,0,255) (Java 8 oracle 实测)。
//!      Java Color 引用理论上可 null, 本域构造路径只有 Builder 默认 RED /
//!      显式传值, 无 null 使用点 → 非 Option (§2.10 按 0 默认处理保真)。
//! PORT: Java 字段/方法名 `type` 是 Rust 关键字 → `r#type`。
//! PORT: `withRatio` 未变化时 "return this (零分配)" / 变化时新实例的可观测
//!      区分 (Java `==` 身份判定) → `Cow::Borrowed(self)` / `Cow::Owned(new)`。

use std::borrow::Cow;

use super::marker_type::MarkerType;

/// Immutable marker specification for MarkedGauge component.
///
/// Markers are visual indicators placed at specific positions on a gauge,
/// such as limit lines, warning zones, or labeled ticks.
///
/// This class is immutable; use the Builder to create instances and
/// withRatio() for copy-on-write updates (zero-allocation friendly when
/// updating only the ratio).
///
/// Example usage:
/// <pre>
/// GaugeMarker optimalStage = GaugeMarker.builder()
///     .id("optimal")
///     .type(MarkerType.LINE_FULL)
///     .ratio(0.5)
///     .color(Application.colorWarning)
///     .build();
/// </pre>
// PORT: Java 未覆写 equals, `==` 是引用同一性; PartialEq 仅为测试断言设施 (§2.6,
// 详见 hud_data.rs HUDData 同款注释)。
#[derive(Debug, Clone, PartialEq)]
pub struct GaugeMarker {
    /// Unique identifier for this marker (used for dynamic updates).
    pub id: String,

    /// The visual type of this marker.
    pub r#type: MarkerType,

    /// Position ratio on the gauge (0.0 = minimum, 1.0 = maximum).
    ///  Values outside [0, 1] will hide the marker.
    pub ratio: f64,

    /// Color used to render this marker.
    pub color: [u8; 4],

    /// Text label for TICK_LABELED type markers.
    pub label: String,

    /// Width ratio for ZONE and LINE_PARTIAL types (0.0 to 1.0, relative to gauge width).
    pub width_ratio: f32,

    /// Side positioning: -1 = left, 0 = center, 1 = right.
    pub side: i32,
}

impl GaugeMarker {
    /// 对应 Java 私有构造器 `GaugeMarker(Builder builder)`: 逐字段拷贝。
    fn from_builder(builder: &Builder) -> Self {
        GaugeMarker {
            id: builder.id.clone(),
            r#type: builder.r#type,
            ratio: builder.ratio,
            color: builder.color,
            label: builder.label.clone(),
            width_ratio: builder.width_ratio,
            side: builder.side,
        }
    }

    /// Creates a new marker with the same properties but a different ratio.
    /// This is the preferred method for updating marker positions in the render loop
    /// as it avoids allocation when the marker is already at the target ratio.
    ///
    /// @param newRatio The new position ratio
    /// @return A new GaugeMarker with updated ratio, or this if ratio unchanged
    // PORT: Java `return this` → Cow::Borrowed(self); `new Builder()...build()` → Cow::Owned。
    pub fn with_ratio(&self, new_ratio: f64) -> Cow<'_, GaugeMarker> {
        if (new_ratio - self.ratio).abs() < 0.0001 {
            return Cow::Borrowed(self); // No change, return self (zero allocation)
        }
        Cow::Owned(
            Builder {
                id: self.id.clone(),
                r#type: self.r#type,
                ratio: new_ratio,
                color: self.color,
                label: self.label.clone(),
                width_ratio: self.width_ratio,
                side: self.side,
            }
            .build(),
        )
    }

    /// Checks if this marker should be visible (ratio in valid range).
    ///
    /// @return true if ratio is in [0, 1] and marker should be drawn
    pub fn is_visible(&self) -> bool {
        self.ratio >= 0.0 && self.ratio <= 1.0
    }

    /// Creates a new Builder for constructing GaugeMarker instances.
    ///
    /// @return A new Builder instance
    pub fn builder() -> Builder {
        Builder::default() // PORT: Java `new Builder()` 带字段默认值
    }
}

/// Builder for constructing GaugeMarker instances.
/// (对应 Java `public static final class Builder`, 字段 Java 私有 → 保持私有, 仅 fluent setter)
#[derive(Debug, Clone, PartialEq)]
pub struct Builder {
    id: String,
    r#type: MarkerType,
    ratio: f64, // Hidden by default (-1)
    color: [u8; 4],
    label: String,
    width_ratio: f32,
    side: i32,
}

impl Default for Builder {
    /// Java 字段初始化器默认值 (Java 8 oracle 实测):
    /// id="", LINE_FULL, ratio=-1 (Hidden by default), RED, label="", 0.5f, 0。
    fn default() -> Self {
        Builder {
            id: String::new(),
            r#type: MarkerType::LineFull,
            ratio: -1.0, // Hidden by default
            color: [255, 0, 0, 255], // Color.RED
            label: String::new(),
            width_ratio: 0.5,
            side: 0,
        }
    }
}

impl Builder {
    pub fn id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    pub fn r#type(mut self, r#type: MarkerType) -> Self {
        self.r#type = r#type;
        self
    }

    pub fn ratio(mut self, ratio: f64) -> Self {
        self.ratio = ratio;
        self
    }

    pub fn color(mut self, color: [u8; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn label(mut self, label: String) -> Self {
        self.label = label;
        self
    }

    pub fn width_ratio(mut self, width_ratio: f32) -> Self {
        self.width_ratio = width_ratio;
        self
    }

    pub fn side(mut self, side: i32) -> Self {
        self.side = side;
        self
    }

    pub fn build(&self) -> GaugeMarker {
        GaugeMarker::from_builder(self)
    }
}

#[cfg(test)]
mod tests;
