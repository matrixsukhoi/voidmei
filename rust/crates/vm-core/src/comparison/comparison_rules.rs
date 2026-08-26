//! 对应 Java: `src/ui/window/comparison/logic/ComparisonRules.java` (一比一翻译)

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::comparison::comparison_rule::ComparisonRule;
use crate::comparison::rules::{try_number_ends, LambdaRule, ListIndexRule, MultiListIndexRule, SimpleRule};
use crate::parser::char_len_at;

/// Registry of comparison rules for FM properties.
///
/// Users can add rules by editing the static initializer block.
/// Properties without rules will show as a draw (grey color).
// PORT: Java `private static final Map rules` + 类加载 static 初始化块 →
// std OnceLock (首次访问执行一次, audio/voice_alert_type.rs 同款先例);
// HashMap 仅做 get/containsKey, 无迭代顺序依赖 (PORTING.md §2.5), std HashMap
// 即可。规则实例经 Box::leak 取 &'static —— 与 Java 静态字段"进程生命周期持有"
// 语义一致。注册表要求 Sync, 存 `dyn ComparisonRule + Send + Sync`, 对外经
// 自动 trait 削减协变回 `&'static dyn ComparisonRule`。
pub struct ComparisonRules;

/// Java 正则 `\s`: [ \t\n\x0B\x0C\r] (ASCII 定义)
fn is_java_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

// Pattern to extract second number from "A / B" format
// (ComparisonRules.java SLASH_SECOND 常量原注释, 逐字保留 — PORTING.md §0.2)

/// `/\s*(-?\d+(\.\d+)?)` (SLASH_SECOND) 的 `Matcher.find()`, 返回组1。
/// '/' 非空白且数字首字符非空白 → `\s*` 贪婪无有效回溯, 确定性。
fn find_slash_second(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    let mut i = 0usize;
    while i < s.len() {
        if b[i] == b'/' {
            let mut p = i + 1;
            while p < s.len() && is_java_ws(b[p]) {
                p += char_len_at(s, p);
            }
            if let Some((_, end)) = try_number_ends(s, p) {
                return Some(&s[p..end]);
            }
        }
        i += char_len_at(s, i);
    }
    None
}

// Pattern to extract both numbers from "A / B" format
// (ComparisonRules.java SLASH_BOTH 常量原注释, 逐字保留 — PORTING.md §0.2)

/// `(-?\d+(\.\d+)?)\s*/\s*(-?\d+(\.\d+)?)` (SLASH_BOTH) 的 `Matcher.find()`,
/// 返回 (组1, 组3)。尝试序与 java.util.regex 一致: 先按贪婪吞小数段, 再重试
/// `(\.\d+)?` 放弃小数段的分支。
///
/// 注意: 该"放弃小数段"分支在本模式中**结构性恒败** —— 有小数段时 int_end 处
/// 必是 '.', 而 '.' 既非 `\s` 也非 '/', `\s*/\s*` 无法在其上起步 (Java 正则的
/// 对应回溯同样恒败, 双方行为一致; 分支保留以镜像回溯次序)。因此形如
/// "12.3.4/5" 的输入实际靠 find() 起点右移命中: 起点推进到内层数字 '3' 后
/// 匹配 "3.4/5" → 3.4+5=8.4 (leftmost-first 起点推进语义, 非回溯)。
fn find_slash_both(s: &str) -> Option<(&str, &str)> {
    let mut i = 0usize;
    while i < s.len() {
        if let Some((int_end, full_end)) = try_number_ends(s, i) {
            // 贪婪: 带小数段
            if let Some(second) = slash_suffix_number(s, full_end) {
                return Some((&s[i..full_end], second));
            }
            // 回溯: `(\.\d+)?` 不吞 (结构性恒败: int_end 处是 '.', 非 `\s` 非 '/',
            // 镜像 Java 回溯次序保留)
            if full_end != int_end {
                if let Some(second) = slash_suffix_number(s, int_end) {
                    return Some((&s[i..int_end], second));
                }
            }
        }
        i += char_len_at(s, i);
    }
    None
}

/// `\s*/\s*(num)` 从 pos 起的匹配, 成功返回第二个数字 (组4) 的文本。
/// 两处 `\s*` 的后继原子 ('/' 与数字首字符) 均非空白, 贪婪即终态。
fn slash_suffix_number(s: &str, pos: usize) -> Option<&str> {
    let b = s.as_bytes();
    let mut p = pos;
    while p < s.len() && is_java_ws(b[p]) {
        p += char_len_at(s, p);
    }
    if p < s.len() && b[p] == b'/' {
        p += 1;
        while p < s.len() && is_java_ws(b[p]) {
            p += char_len_at(s, p);
        }
        if let Some((_, end)) = try_number_ends(s, p) {
            return Some(&s[p..end]);
        }
    }
    None
}

fn rules() -> &'static HashMap<&'static str, &'static (dyn ComparisonRule + Send + Sync)> {
    RULES.get_or_init(build_rules)
}

fn leak_rule<R: ComparisonRule + Send + Sync + 'static>(r: R) -> &'static (dyn ComparisonRule + Send + Sync) {
    Box::leak(Box::new(r))
}

// Java: private static final Map<String, ComparisonRule> rules = new HashMap<>();
static RULES: OnceLock<HashMap<&'static str, &'static (dyn ComparisonRule + Send + Sync)>> =
    OnceLock::new();

fn build_rules() -> HashMap<&'static str, &'static (dyn ComparisonRule + Send + Sync)> {
    let mut rules = HashMap::new();

    // ========== 重量类 ==========
    // 空重: 轻好
    rules.insert("空重(kg)", leak_rule(SimpleRule::lower_is_better()));
    // 燃油: 重好
    rules.insert("最大燃油重量(kg)", leak_rule(SimpleRule::higher_is_better()));

    // ========== 速度类 ==========
    // 临界速度 [min, max]: 后面那个数(vne)大好
    rules.insert("临界速度(km/h)", leak_rule(ListIndexRule::new(1, false)));

    // ========== 过载类 ==========
    // 允许过载 [满油+, 满油-], [半油+, 半油-]: 第一个列表最后一项大好
    rules.insert("允许过载(满/半油)", leak_rule(MultiListIndexRule::new(0, 1, false)));

    // ========== 耐热类 ==========
    // 耐热条恢复速率: 大好
    rules.insert("平均耐热条恢复速率", leak_rule(SimpleRule::higher_is_better()));

    // ========== 升力类 ==========
    // 最大升力过载 "X / Y(襟)": 第一个数大好
    rules.insert("千米最大升力过载", leak_rule(SimpleRule::higher_is_better()));

    // 升力面积因数载荷 "X / Y(襟)": 第一个数大好
    rules.insert("主升力面积因数载荷", leak_rule(SimpleRule::higher_is_better()));

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
            Box::new(|raw| {
                find_slash_second(raw).and_then(|g| g.parse::<f64>().ok())
            }),
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
    /// @param property_name the property name (e.g., "空重(kg)")
    /// @return the rule, or null if no rule is defined (will show as draw)
    pub fn get(property_name: &str) -> Option<&'static dyn ComparisonRule> {
        rules().get(property_name).map(|r| *r as &'static dyn ComparisonRule)
    }

    /// Check if a rule exists for the given property.
    ///
    /// @param property_name the property name
    /// @return true if a rule is defined
    pub fn has_rule(property_name: &str) -> bool {
        rules().contains_key(property_name)
    }
}

// =====================================================================
// Tests — 期望值取自 Java 8 oracle 实测 (经 ComparisonRules.get 原类直跑,
// Double.doubleToLongBits 逐位对拍)。
#[cfg(test)]
mod tests {
    use super::*;

    fn extract(prop: &str, raw: Option<&str>) -> Option<f64> {
        ComparisonRules::get(prop).expect("rule registered").extract_value(raw)
    }

    fn bits(v: Option<f64>) -> Option<u64> {
        v.map(|x| x.to_bits())
    }

    #[test]
    fn all_registered_keys_present_with_direction() {
        // 11 条规则齐全 + isLowerBetter 方向 (Java static 块逐条对照)
        let expect = [
            ("空重(kg)", true),
            ("最大燃油重量(kg)", false),
            ("临界速度(km/h)", false),
            ("允许过载(满/半油)", false),
            ("平均耐热条恢复速率", false),
            ("千米最大升力过载", false),
            ("主升力面积因数载荷", false),
            ("翼展效率", false),
            ("主阻力面积因数及加速度系数", true),
            ("诱导阻力因数及加速度系数", true),
            ("散热/油冷器阻力系数", true),
        ];
        for (key, lower) in expect {
            let r = ComparisonRules::get(key).unwrap_or_else(|| panic!("{key} 未注册"));
            assert_eq!(r.is_lower_better(), lower, "{key}");
        }
    }

    #[test]
    fn missing_property_has_no_rule() {
        // oracle: REG 不存在的属性 NORULE; HAS foo false; HAS 散热/油冷器阻力系数 true
        assert!(ComparisonRules::get("不存在的属性").is_none());
        assert!(!ComparisonRules::has_rule("foo"));
        assert!(ComparisonRules::has_rule("空重(kg)"));
        assert!(ComparisonRules::has_rule("散热/油冷器阻力系数")); // 键本身含 '/'
    }

    #[test]
    fn builtin_rules_extract_via_registry() {
        // oracle: REG 空重 4644.0; [1,2]→null; 最大燃油 705.0; 临界速度 1167.0;
        // 允许过载 (0,1)→-4.2; 耐热 0.87; 千米升力 12.5; 主升力 123.4; 翼展 0.95
        assert_eq!(bits(extract("空重(kg)", Some("4644.0"))), Some(4661828146700484608));
        assert_eq!(extract("空重(kg)", Some("[1,2]")), None);
        assert_eq!(bits(extract("最大燃油重量(kg)", Some("705.0"))), Some(4649412461399638016));
        assert_eq!(bits(extract("临界速度(km/h)", Some("[144, 1167]"))), Some(4652847335724810240));
        assert_eq!(
            bits(extract("允许过载(满/半油)", Some("[8.5, -4.2], [10.1, -5.3]"))),
            Some((-4606957238818648883_i64) as u64)
        );
        assert_eq!(bits(extract("平均耐热条恢复速率", Some("0.87"))), Some(4606011482896901079));
        assert_eq!(bits(extract("千米最大升力过载", Some("12.5 / 13.0"))), Some(4623226492472524800));
        assert_eq!(bits(extract("主升力面积因数载荷", Some("123.4"))), Some(4638383919968393626));
        assert_eq!(bits(extract("翼展效率", Some("0.95"))), Some(4606732058837280358));
        // null 原始值 → null
        assert_eq!(extract("空重(kg)", None), None);
    }

    #[test]
    fn slash_second_rule_extracts_number_after_slash() {
        // oracle: 0.25 / 0.35 → 0.35 (空格可有可无); 1 / 2 / 3 → 2 (首个 '/');
        // -1.5 / -2.5 → -2.5; abc / 0.5 → 0.5 ('/' 前内容不参与)
        let prop = "主阻力面积因数及加速度系数";
        assert_eq!(bits(extract(prop, Some("0.25 / 0.35"))), Some(4599976659396224614));
        assert_eq!(bits(extract(prop, Some("0.25/0.35"))), Some(4599976659396224614));
        assert_eq!(bits(extract(prop, Some("1 / 2 / 3"))), Some(4611686018427387904));
        assert_eq!(bits(extract(prop, Some("-1.5 / -2.5"))), Some((-4610560118520545280_i64) as u64));
        assert_eq!(bits(extract(prop, Some("abc / 0.5"))), Some(4602678819172646912));
        assert_eq!(bits(extract(prop, Some("a/3.5"))), Some(4615063718147915776));
    }

    #[test]
    fn slash_second_rule_no_match_returns_none() {
        // oracle: "/" (后无数字) / "0.25 / " (尾无数字) / "1.5/abc" ('/' 后非数字)
        // / "x /y" ('/' 后零空白紧跟非数字) → null
        let prop = "主阻力面积因数及加速度系数";
        assert_eq!(extract(prop, Some("/")), None);
        assert_eq!(extract(prop, Some("0.25 / ")), None);
        assert_eq!(extract(prop, Some("1.5/abc")), None);
        assert_eq!(extract(prop, Some("x /y")), None);
    }

    #[test]
    fn slash_both_rule_sums_two_numbers() {
        // oracle: 0.1 / 0.2 → 0.30000000000000004 (逐位); 1/2 → 3.0;
        // 3.0 / 4.0 / 5.0 → 7.0 (取前两个数); a 1 / 2 b → 3.0; "  0.5/0.6  " → 1.1
        let prop = "散热/油冷器阻力系数";
        assert_eq!(bits(extract(prop, Some("0.1 / 0.2"))), Some(4599075939470750516));
        assert_eq!(bits(extract(prop, Some("1/2"))), Some(4613937818241073152));
        assert_eq!(bits(extract(prop, Some("3.0 / 4.0 / 5.0"))), Some(4619567317775286272));
        assert_eq!(bits(extract(prop, Some("a 1 / 2 b"))), Some(4613937818241073152));
        assert_eq!(bits(extract(prop, Some("  0.5/0.6  "))), Some(4607632778762754458));
    }

    #[test]
    fn slash_both_rule_backtracks_fraction_part() {
        // oracle: "12.3.4/5" → 3.4 + 5 = 8.4 — 起点处贪婪吞 "12.3" 后 '.' 令
        // `\s*/` 失败, 放弃小数段的回溯同样恒败 ('.' 非 `\s` 非 '/'), 由
        // find() 起点右移至内层数字 '3' 命中 "3.4/5" (leftmost-first 推进)
        assert_eq!(
            bits(extract("散热/油冷器阻力系数", Some("12.3.4/5"))),
            Some(4620918397663497421)
        );
    }

    #[test]
    fn slash_both_rule_no_match_returns_none() {
        // oracle: "1.5/abc" ('/' 后无数字) / "仅/中文" (两侧均无数字) → null
        let prop = "散热/油冷器阻力系数";
        assert_eq!(extract(prop, Some("1.5/abc")), None);
        assert_eq!(extract(prop, Some("仅/中文")), None);
    }

    #[test]
    fn slash_rules_null_and_empty_return_none() {
        // oracle: 空串/null → LambdaRule 前置守卫返回 null
        assert_eq!(extract("散热/油冷器阻力系数", Some("")), None);
        assert_eq!(extract("散热/油冷器阻力系数", None), None);
        // 诱导阻力 (SLASH_SECOND 同款 lambda): 0.10 / 0.20 → 0.2
        assert_eq!(bits(extract("诱导阻力因数及加速度系数", Some("0.10 / 0.20"))), Some(4596373779694328218));
    }
}
