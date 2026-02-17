package ui.window.comparison.logic.rules;

import java.util.regex.Matcher;
import java.util.regex.Pattern;

import ui.window.comparison.logic.ComparisonRule;

/**
 * Simple rule that extracts the first number from the value string.
 * Skips array/list values (starting with '[').
 */
public class SimpleRule implements ComparisonRule {

    private static final Pattern NUMBER_PATTERN = Pattern.compile("(-?\\d+(\\.\\d+)?)");

    private final boolean lowerIsBetter;

    public SimpleRule(boolean lowerIsBetter) {
        this.lowerIsBetter = lowerIsBetter;
    }

    /**
     * Create a rule where lower values are better (e.g., weight, drag)
     */
    public static SimpleRule lowerIsBetter() {
        return new SimpleRule(true);
    }

    /**
     * Create a rule where higher values are better (e.g., speed, thrust)
     */
    public static SimpleRule higherIsBetter() {
        return new SimpleRule(false);
    }

    @Override
    public Double extractValue(String rawValue) {
        if (rawValue == null || rawValue.isEmpty()) {
            return null;
        }
        if (rawValue.startsWith("[")) {
            return null; // Skip array values - use ListIndexRule for these
        }

        try {
            Matcher m = NUMBER_PATTERN.matcher(rawValue);
            if (m.find()) {
                return Double.parseDouble(m.group(1));
            }
        } catch (Exception e) {
            // ignore
        }
        return null;
    }

    @Override
    public boolean isLowerBetter() {
        return lowerIsBetter;
    }
}
