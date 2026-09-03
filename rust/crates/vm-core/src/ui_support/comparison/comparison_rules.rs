//! 对应 Java: `src/ui/window/comparison/logic/ComparisonRules.java`
//! 波21: SLASH_* 手写回溯匹配器退役, 模式原样落 regex crate
//! (leftmost-first + 贪婪次序与 java.util.regex find() 一致)。

use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock};

use regex::Regex;

use crate::ui_support::comparison::comparison_rule::ComparisonRule;
use crate::ui_support::comparison::rules::{
    LambdaRule, ListIndexRule, MultiListIndexRule, SimpleRule, JAVA_WS,
};

/// Registry of comparison rules for FM properties.
///
/// Users can add rules by editing the static initializer block.
/// Properties without rules will show as a draw (grey color).
// Java `private static final Map rules` + 类加载 static 初始化块 →
// std OnceLock (首次访问执行一次, audio/voice_alert_type.rs 同款先例);
// HashMap 仅做 get/containsKey, 无迭代顺序依赖, std HashMap
// 即可。规则实例经 Box::leak 取 &'static —— 与 Java 静态字段"进程生命周期持有"
// 语义一致。注册表要求 Sync, 存 `dyn ComparisonRule + Send + Sync`, 对外经
// 自动 trait 削减协变回 `&'static dyn ComparisonRule`。
pub struct ComparisonRules;

// Pattern to extract second number from "A / B" format
// (ComparisonRules.java SLASH_SECOND 常量原注释, 逐字保留 — )

/// `/\s*(-?\d+(\.\d+)?)` (SLASH_SECOND) 的 find(), 返回组1。
/// 波21: 手写回溯匹配器退役, regex crate 直接承载 (JAVA_WS = ASCII \s)。
fn find_slash_second(s: &str) -> Option<&str> {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&format!(r"/[{}]*(-?[0-9]+(\.[0-9]+)?)", JAVA_WS)).unwrap());
    RE.captures(s).map(|c| c.get(1).unwrap().as_str())
}

// Pattern to extract both numbers from "A / B" format
// (ComparisonRules.java SLASH_BOTH 常量原注释, 逐字保留 — )

/// `(-?\d+(\.\d+)?)\s*/\s*(-?\d+(\.\d+)?)` (SLASH_BOTH) 的 find(), 返回 (组1, 组3)。
/// regex crate 的 leftmost-first + 贪婪回溯次序与 java.util.regex 一致
/// ("12.3.4/5" 类输入靠起点右移命中 "3.4/5", 原手写注释的论证同样适用)。
fn find_slash_both(s: &str) -> Option<(&str, &str)> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"(-?[0-9]+(\.[0-9]+)?)[{}]*/[{}]*(-?[0-9]+(\.[0-9]+)?)",
            JAVA_WS, JAVA_WS
        ))
        .unwrap()
    });
    RE.captures(s)
        .map(|c| (c.get(1).unwrap().as_str(), c.get(3).unwrap().as_str()))
}

fn rules() -> &'static HashMap<&'static str, &'static (dyn ComparisonRule + Send + Sync)> {
    RULES.get_or_init(build_rules)
}

fn leak_rule<R: ComparisonRule + Send + Sync + 'static>(
    r: R,
) -> &'static (dyn ComparisonRule + Send + Sync) {
    Box::leak(Box::new(r))
}

static RULES: OnceLock<HashMap<&'static str, &'static (dyn ComparisonRule + Send + Sync)>> =
    OnceLock::new();

fn build_rules() -> HashMap<&'static str, &'static (dyn ComparisonRule + Send + Sync)> {
    let mut rules = HashMap::new();

    // ========== 重量类 ==========
    // 空重: 轻好
    rules.insert("空重(kg)", leak_rule(SimpleRule::lower_is_better()));
    // 燃油: 重好
    rules.insert(
        "最大燃油重量(kg)",
        leak_rule(SimpleRule::higher_is_better()),
    );

    // ========== 速度类 ==========
    // 临界速度 [min, max]: 后面那个数(vne)大好
    rules.insert("临界速度(km/h)", leak_rule(ListIndexRule::new(1, false)));

    // ========== 过载类 ==========
    // 允许过载 [满油+, 满油-], [半油+, 半油-]: 第一个列表最后一项大好
    rules.insert(
        "允许过载(满/半油)",
        leak_rule(MultiListIndexRule::new(0, 1, false)),
    );

    // ========== 耐热类 ==========
    // 耐热条恢复速率: 大好
    rules.insert(
        "平均耐热条恢复速率",
        leak_rule(SimpleRule::higher_is_better()),
    );

    // ========== 升力类 ==========
    // 最大升力过载 "X / Y(襟)": 第一个数大好
    rules.insert(
        "千米最大升力过载",
        leak_rule(SimpleRule::higher_is_better()),
    );

    // 升力面积因数载荷 "X / Y(襟)": 第一个数大好
    rules.insert(
        "主升力面积因数载荷",
        leak_rule(SimpleRule::higher_is_better()),
    );

    // 翼展效率: 大好
    rules.insert("翼展效率", leak_rule(SimpleRule::higher_is_better()));

    // ========== 阻力类 (第二个数小好) ==========
    // 主阻力面积因数及加速度系数 "X / Y": 第二个数小好
    rules.insert(
        "主阻力面积因数及加速度系数",
        leak_rule(LambdaRule::new(
            Box::new(|raw| {
                // Matcher m = SLASH_SECOND.matcher(raw);
                // return m.find() ? Double.parseDouble(m.group(1)) : null;
                find_slash_second(raw).and_then(|g| g.parse::<f64>().ok())
            }),
            true, // lower is better
        )),
    );

    // 诱导阻力因数及加速度系数 "X / Y": 第二个数小好
    rules.insert(
        "诱导阻力因数及加速度系数",
        leak_rule(LambdaRule::new(
            Box::new(|raw| find_slash_second(raw).and_then(|g| g.parse::<f64>().ok())),
            true, // lower is better
        )),
    );

    // 散热/油冷器阻力系数 "X / Y": 两个数加在一起，总和小好
    rules.insert(
        "散热/油冷器阻力系数",
        leak_rule(LambdaRule::new(
            Box::new(|raw| {
                // Matcher m = SLASH_BOTH.matcher(raw);
                // if (m.find()) { double a = ...; double b = ...; return a + b; }
                // return null;
                match find_slash_both(raw) {
                    Some((a, b)) => {
                        let a = a.parse::<f64>().ok()?;
                        let b = b.parse::<f64>().ok()?;
                        Some(a + b)
                    }
                    None => None,
                }
            }),
            true, // lower is better
        )),
    );

    rules
}

impl ComparisonRules {
    /// Get the comparison rule for a property name.
    ///
    /// - `property_name`: the property name (e.g., "空重(kg)")
    /// 返回: the rule, or null if no rule is defined (will show as draw)
    pub fn get(property_name: &str) -> Option<&'static dyn ComparisonRule> {
        rules()
            .get(property_name)
            .map(|r| *r as &'static dyn ComparisonRule)
    }

    /// Check if a rule exists for the given property.
    ///
    /// - `property_name`: the property name
    /// 返回: true if a rule is defined
    pub fn has_rule(property_name: &str) -> bool {
        rules().contains_key(property_name)
    }
}

// =====================================================================
// Tests — 期望值取自 历史基线 (经 ComparisonRules.get 原类直跑,
// Double.doubleToLongBits 逐位对拍)。
#[cfg(test)]
mod tests;
