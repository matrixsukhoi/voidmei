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
        rules.put("emptyWeight", SimpleRule.lowerIsBetter());
        // 燃油: 重好
        rules.put("maxFuelWeight", SimpleRule.higherIsBetter());

        // ========== 速度类 ==========
        // 临界速度 [min, max]: 后面那个数(vne)大好
        rules.put("critSpeed", new ListIndexRule(1, false));

        // ========== 过载类 ==========
        // 允许过载 [满油+, 满油-], [半油+, 半油-]: 第一个列表最后一项大好
        rules.put("allowLoadFactor", new MultiListIndexRule(0, 1, false));

        // ========== 耐热类 ==========
        // 耐热条恢复速率: 大好
        rules.put("avgHeatRecovery", SimpleRule.higherIsBetter());

        // ========== 升力类 ==========
        // 最大升力过载 "X / Y(襟)": 第一个数大好
        rules.put("maxLiftLoad350", SimpleRule.higherIsBetter());

        // 升力面积因数载荷 "X / Y(襟)": 第一个数大好
        rules.put("liftLoadFactor", SimpleRule.higherIsBetter());

        // 翼展效率: 大好
        rules.put("oswaldEfficiency", SimpleRule.higherIsBetter());

        // ========== 阻力类 (第二个数小好) ==========
        // 主阻力面积因数及加速度系数 "X / Y": 第二个数小好
        rules.put("dragAreaFactor", new LambdaRule(
            raw -> {
                Matcher m = SLASH_SECOND.matcher(raw);
                return m.find() ? Double.parseDouble(m.group(1)) : null;
            },
            true // lower is better
        ));

        // 诱导阻力因数及加速度系数 "X / Y": 第二个数小好
        rules.put("inducedDragFactor", new LambdaRule(
            raw -> {
                Matcher m = SLASH_SECOND.matcher(raw);
                return m.find() ? Double.parseDouble(m.group(1)) : null;
            },
            true // lower is better
        ));

        // 散热/油冷器阻力系数 "X / Y": 两个数加在一起，总和小好
        rules.put("radiatorDragCoeff", new LambdaRule(
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
     * 显示名(冒号前段, 随语言变化)经 {@link FmPropKeys} 反查稳定 key 后再查表,
     * 未注册的属性返回 null(显示为平局)。
     *
     * @param propertyName the property display name (e.g., "空重(kg)")
     * @return the rule, or null if no rule is defined (will show as draw)
     */
    public static ComparisonRule get(String propertyName) {
        String key = FmPropKeys.keyOfDisplay(propertyName);
        return key != null ? rules.get(key) : null;
    }

    /**
     * Check if a rule exists for the given property.
     *
     * @param propertyName the property display name
     * @return true if a rule is defined
     */
    public static boolean hasRule(String propertyName) {
        return get(propertyName) != null;
    }
}
