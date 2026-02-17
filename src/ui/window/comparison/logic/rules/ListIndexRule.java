package ui.window.comparison.logic.rules;

import java.util.regex.Matcher;
import java.util.regex.Pattern;

import ui.window.comparison.logic.ComparisonRule;

/**
 * Rule that extracts a value from a specific index in a list/array format.
 *
 * Example: "[144, 1167]" with index=1 extracts 1167
 */
public class ListIndexRule implements ComparisonRule {

    // Matches numbers (including negative and decimal) within brackets
    private static final Pattern LIST_PATTERN = Pattern.compile("\\[([^\\]]+)\\]");
    private static final Pattern NUMBER_PATTERN = Pattern.compile("(-?\\d+(\\.\\d+)?)");

    private final int index;
    private final boolean lowerIsBetter;

    /**
     * @param index the 0-based index to extract from the list
     * @param lowerIsBetter true if lower values are better
     */
    public ListIndexRule(int index, boolean lowerIsBetter) {
        this.index = index;
        this.lowerIsBetter = lowerIsBetter;
    }

    @Override
    public Double extractValue(String rawValue) {
        if (rawValue == null || rawValue.isEmpty()) {
            return null;
        }

        try {
            Matcher listMatcher = LIST_PATTERN.matcher(rawValue);
            if (listMatcher.find()) {
                String listContent = listMatcher.group(1);
                String[] parts = listContent.split(",");

                if (index >= 0 && index < parts.length) {
                    String part = parts[index].trim();
                    Matcher numMatcher = NUMBER_PATTERN.matcher(part);
                    if (numMatcher.find()) {
                        return Double.parseDouble(numMatcher.group(1));
                    }
                }
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
