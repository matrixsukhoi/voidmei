//! 飞行信息字段域 (字段网格已原子化: 出厂页 = 逐行 core.data.field 组件,
//! 见 widgets::data_field)。本模块只剩取数与度量共用面。

use vm_core::formula::registry::FormulaView;

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
    let (var, mult) = vm_core::formula::resolve_target(target)?;
    vm_core::formula::target_value(&var, mult, s)
}

// =====================================================================
// Tests
// =====================================================================
/// 名字可达性检查 (测试面): registry 名 ∪ 公式名 — 守卫测试用它钉死
/// 出厂页全部消费 target 可达, 防 "名字解析断链 → 字段行消失/恒 0" 的
/// live 显示回归。单名制 (W10): 无别名翻译, 查不到即真断链。
#[cfg(test)]
pub(crate) fn canonical_var_name(name: &str) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    let m = MAP.get_or_init(|| {
        let mut m: HashMap<String, String> = HashMap::new();
        let reg = vm_core::formula::registry::registry();
        for v in &reg.vars {
            m.insert(v.name.to_string(), v.name.to_string());
        }
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../formulas.cfg");
        if let Ok(src) = std::fs::read_to_string(path) {
            for d in vm_core::formula::persistence::parse_formulas(&src) {
                m.insert(d.name.clone(), d.name.clone());
            }
        }
        m
    });
    m.get(name).cloned()
}

#[cfg(test)]
mod tests;
