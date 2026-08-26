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
mod tests {
    use super::*;

    /// Java 8 oracle `GMdefault`: builder() 不设任何字段的产物。
    #[test]
    fn builder_defaults_match_java() {
        let d = GaugeMarker::builder().build();
        assert_eq!(d.id, "");
        assert_eq!(d.r#type, MarkerType::LineFull);
        assert_eq!(d.ratio, -1.0);
        assert_eq!(d.color, [255, 0, 0, 255]);
        assert_eq!(d.label, "");
        assert_eq!(d.width_ratio, 0.5);
        assert_eq!(d.side, 0);
        assert!(!d.is_visible(), "ratio=-1 默认隐藏");
    }

    /// Java 类文档示例的 fluent 构造路径。
    #[test]
    fn fluent_build_sets_all_fields() {
        let m = GaugeMarker::builder()
            .id("optimal".to_string())
            .r#type(MarkerType::TickLabeled)
            .ratio(0.5)
            .color([255, 200, 64, 255])
            .label("WEP".to_string())
            .width_ratio(0.25)
            .side(-1)
            .build();
        assert_eq!(m.id, "optimal");
        assert_eq!(m.r#type, MarkerType::TickLabeled);
        assert_eq!(m.ratio, 0.5);
        assert_eq!(m.color, [255, 200, 64, 255]);
        assert_eq!(m.label, "WEP");
        assert_eq!(m.width_ratio, 0.25);
        assert_eq!(m.side, -1);
    }

    /// Java 8 oracle `visible(...)` 边界表: 含 NaN 两端开区间行为。
    #[test]
    fn is_visible_boundaries_match_java() {
        let cases = [
            (0.0, true),
            (1.0, true),
            (-0.0001, false),
            (1.0001, false),
            (-1.0, false),
            (f64::NAN, false), // Java: NaN >= 0.0 为 false
            (0.5, true),
        ];
        for (r, expect) in cases {
            assert_eq!(
                GaugeMarker::builder().ratio(r).build().is_visible(),
                expect,
                "visible({r}) 失配"
            );
        }
    }

    /// Java 8 oracle: `with_same=true`、`with_plus_5e5=true`
    /// (|0.5+0.00005 - 0.5| = 4.999999999999449e-5 < 1e-4 → 同实例)。
    #[test]
    fn with_ratio_returns_self_when_delta_below_epsilon() {
        let m = GaugeMarker::builder().ratio(0.5).build();
        assert!(matches!(m.with_ratio(0.5), Cow::Borrowed(_)));
        assert!(matches!(m.with_ratio(0.5 + 0.00005), Cow::Borrowed(_)));
        // 9e-5 < 1e-4 → 同实例 (oracle with_9e5=true)
        let m0 = GaugeMarker::builder().ratio(0.0).build();
        assert!(matches!(m0.with_ratio(0.00009), Cow::Borrowed(_)));
    }

    /// Java 8 oracle `with_exact_1e4=false`: 严格小于判定, delta == 1e-4 时已换新实例。
    #[test]
    fn with_ratio_new_instance_at_exact_epsilon() {
        let m0 = GaugeMarker::builder().ratio(0.0).build();
        // 0.0 + 0.0001: delta 即 1e-4 的最近双精度值, `< 0.0001` 不成立 → Owned
        assert_eq!(0.0001 - 0.0, 0.0001);
        assert!(matches!(m0.with_ratio(0.0001), Cow::Owned(_)));
    }

    /// Java 8 oracle `new_ratio=0.75 ...`: 换 ratio 后其余字段原样保留
    /// (走 Builder 重建路径, 未显式设置处为 Builder 默认的拷贝源值)。
    #[test]
    fn with_ratio_copies_other_fields() {
        let m = GaugeMarker::builder()
            .id("stall".to_string())
            .r#type(MarkerType::Zone)
            .ratio(0.5)
            .color([255, 0, 0, 200])
            .label("STALL".to_string())
            .width_ratio(0.3)
            .side(1)
            .build();
        let m2 = m.with_ratio(0.75);
        let owned = match m2 {
            Cow::Owned(ref g) => g.clone(),
            Cow::Borrowed(_) => panic!("0.75 vs 0.5 应生成新实例"),
        };
        assert_eq!(owned.ratio, 0.75);
        assert_eq!(owned.id, "stall");
        assert_eq!(owned.r#type, MarkerType::Zone);
        assert_eq!(owned.color, [255, 0, 0, 200]);
        assert_eq!(owned.label, "STALL");
        assert_eq!(owned.width_ratio, 0.3);
        assert_eq!(owned.side, 1);
        // 原实例不受影响 (Java 不可变类语义)
        assert_eq!(m.ratio, 0.5);
    }

    /// 边界: NaN 与任何 ratio 的差都是 NaN, `< 0.0001` 为 false → 必然 Owned。
    #[test]
    fn with_ratio_nan_always_new_instance() {
        let m = GaugeMarker::builder().ratio(0.5).build();
        assert!(matches!(m.with_ratio(f64::NAN), Cow::Owned(_)));
    }
}
