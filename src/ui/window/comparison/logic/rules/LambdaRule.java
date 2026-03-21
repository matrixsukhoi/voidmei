package ui.window.comparison.logic.rules;

import java.util.function.Function;

import ui.window.comparison.logic.ComparisonRule;

/**
 * Rule that uses a custom lambda function to extract values.
 * Useful for complex extraction patterns that don't fit the standard rules.
 *
 * Example usage:
 * <pre>
 * new LambdaRule(
 *     raw -> {
 *         Matcher m = Pattern.compile("副翼(\\d+)").matcher(raw);
 *         return m.find() ? Double.parseDouble(m.group(1)) : null;
 *     },
 *     false // higher is better
 * )
 * </pre>
 */
public class LambdaRule implements ComparisonRule {

    private final Function<String, Double> extractor;
    private final boolean lowerIsBetter;

    /**
     * @param extractor function that extracts a Double from the raw value string
     * @param lowerIsBetter true if lower values are better
     */
    public LambdaRule(Function<String, Double> extractor, boolean lowerIsBetter) {
        this.extractor = extractor;
        this.lowerIsBetter = lowerIsBetter;
    }

    @Override
    public Double extractValue(String rawValue) {
        if (rawValue == null || rawValue.isEmpty()) {
            return null;
        }
        try {
            return extractor.apply(rawValue);
        } catch (Exception e) {
            return null;
        }
    }

    @Override
    public boolean isLowerBetter() {
        return lowerIsBetter;
    }
}
