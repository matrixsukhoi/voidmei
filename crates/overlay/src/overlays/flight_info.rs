//! 飞行信息字段域 (字段网格已原子化: 出厂页 = 逐行 core.data.field 组件,
//! 见 widgets::data_field)。本模块只剩取数与度量共用面。

use kernel::formula::registry::FormulaView;

/// numHeight 默认值 (POC main.rs 平移): Java 实测校准 24px BOLD Sarasa = 31,
/// 其余字号 1.25×fontSize 近似 (与实测差 ≤1px, 精确值由对拍脚本 --num-height 注入)
pub fn default_num_height(font_add: i32) -> i32 {
    if font_add == 0 {
        31
    } else {
        ((24 + font_add) as f32 * 1.25).round() as i32
    }
}

/// TelemetrySource → 变量数值 (W2: FlightValues 整包快照消解; W10: 统一
/// 短名制 — 变量名 | 公式名 | "X * N" 乘数, Java getter 名不再进内核取数)
pub fn flight_value(s: &dyn FormulaView, target: &str) -> Option<f64> {
    let (var, mult) = kernel::formula::resolve_target(target)?;
    kernel::formula::target_value(&var, mult, s)
}

