package ui.window.comparison.logic;

import java.util.HashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import ui.window.comparison.logic.rules.LambdaRule;
import ui.window.comparison.logic.rules.ListIndexRule;
import ui.window.comparison.logic.rules.MultiListIndexRule;
import ui.window.comparison.logic.rules.SimpleRule;

/**
 * Registry of comparison rules for FM properties.
 *
 * Users can add rules by editing the static initializer block.
 * Properties without rules will show as a draw (grey color).
 */
public class ComparisonRules {

    private static final Map<String, ComparisonRule> rules = new HashMap<>();

    // Pattern to extract second number from "A / B" format
    private static final Pattern SLASH_SECOND = Pattern.compile("/\\s*(-?\\d+(\\.\\d+)?)");
    // Pattern to extract both numbers from "A / B" format
    private static final Pattern SLASH_BOTH = Pattern.compile("(-?\\d+(\\.\\d+)?)\\s*/\\s*(-?\\d+(\\.\\d+)?)");

    static {
        // ========== 重量类 ==========
        // 空重: 轻好
        rules.put("空重(kg)", SimpleRule.lowerIsBetter());
        // 燃油: 重好
        rules.put("最大燃油重量(kg)", SimpleRule.higherIsBetter());

        // ========== 速度类 ==========
        // 临界速度 [min, max]: 后面那个数(vne)大好
        rules.put("临界速度(km/h)", new ListIndexRule(1, false));

        // ========== 过载类 ==========
        // 允许过载 [满油+, 满油-], [半油+, 半油-]: 第一个列表最后一项大好
        rules.put("允许过载(满/半油)", new MultiListIndexRule(0, 1, false));

        // ========== 耐热类 ==========
        // 耐热条恢复速率: 大好
        rules.put("平均耐热条恢复速率", SimpleRule.higherIsBetter());

        // ========== 升力类 ==========
        // 最大升力过载 "X / Y(襟)": 第一个数大好
        rules.put("千米最大升力过载", SimpleRule.higherIsBetter());

        // 升力面积因数载荷 "X / Y(襟)": 第一个数大好
        rules.put("主升力面积因数载荷", SimpleRule.higherIsBetter());

        // 翼展效率: 大好
        rules.put("翼展效率", SimpleRule.higherIsBetter());

        // ========== 阻力类 (第二个数小好) ==========
        // 主阻力面积因数及加速度系数 "X / Y": 第二个数小好
        rules.put("主阻力面积因数及加速度系数", new LambdaRule(
            raw -> {
                Matcher m = SLASH_SECOND.matcher(raw);
                return m.find() ? Double.parseDouble(m.group(1)) : null;
            },
            true // lower is better
        ));

        // 诱导阻力因数及加速度系数 "X / Y": 第二个数小好
        rules.put("诱导阻力因数及加速度系数", new LambdaRule(
            raw -> {
                Matcher m = SLASH_SECOND.matcher(raw);
                return m.find() ? Double.parseDouble(m.group(1)) : null;
            },
            true // lower is better
        ));

        // 散热/油冷器阻力系数 "X / Y": 两个数加在一起，总和小好
        rules.put("散热/油冷器阻力系数", new LambdaRule(
            raw -> {
                Matcher m = SLASH_BOTH.matcher(raw);
                if (m.find()) {
                    double a = Double.parseDouble(m.group(1));
                    double b = Double.parseDouble(m.group(3));
                    return a + b;
                }
                return null;
            },
            true // lower is better
        ));
    }

    /**
     * Get the comparison rule for a property name.
     *
     * @param propertyName the property name (e.g., "空重(kg)")
     * @return the rule, or null if no rule is defined (will show as draw)
     */
    public static ComparisonRule get(String propertyName) {
        return rules.get(propertyName);
    }

    /**
     * Check if a rule exists for the given property.
     *
     * @param propertyName the property name
     * @return true if a rule is defined
     */
    public static boolean hasRule(String propertyName) {
        return rules.containsKey(propertyName);
    }
}
