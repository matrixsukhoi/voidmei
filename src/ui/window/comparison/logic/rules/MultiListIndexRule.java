package ui.window.comparison.logic.rules;

import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import ui.window.comparison.logic.ComparisonRule;

/**
 * Rule that extracts a value from a specific position in nested lists.
 *
 * Example: "[4.5, 11.2], [5.0, 12.0]" with listIndex=0, itemIndex=1 extracts 11.2
 */
public class MultiListIndexRule implements ComparisonRule {

    // Matches each bracketed list
    private static final Pattern LIST_PATTERN = Pattern.compile("\\[([^\\]]+)\\]");
    private static final Pattern NUMBER_PATTERN = Pattern.compile("(-?\\d+(\\.\\d+)?)");

    private final int listIndex;
    private final int itemIndex;
    private final boolean lowerIsBetter;

    /**
     * @param listIndex the 0-based index of the list to use
     * @param itemIndex the 0-based index within that list
     * @param lowerIsBetter true if lower values are better
     */
    public MultiListIndexRule(int listIndex, int itemIndex, boolean lowerIsBetter) {
        this.listIndex = listIndex;
        this.itemIndex = itemIndex;
        this.lowerIsBetter = lowerIsBetter;
    }

    @Override
    public Double extractValue(String rawValue) {
        if (rawValue == null || rawValue.isEmpty()) {
            return null;
        }

        try {
            // Find all lists
            List<String> lists = new ArrayList<>();
            Matcher listMatcher = LIST_PATTERN.matcher(rawValue);
            while (listMatcher.find()) {
                lists.add(listMatcher.group(1));
            }

            if (listIndex >= 0 && listIndex < lists.size()) {
                String listContent = lists.get(listIndex);
                String[] parts = listContent.split(",");

                if (itemIndex >= 0 && itemIndex < parts.length) {
                    String part = parts[itemIndex].trim();
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
