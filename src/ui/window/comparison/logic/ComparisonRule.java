package ui.window.comparison.logic;

/**
 * Interface for FM comparison rules.
 * Each rule defines how to extract a comparable numeric value from a raw string
 * and whether lower values are considered better.
 */
public interface ComparisonRule {

    /**
     * Extract a comparable numeric value from the raw string.
     *
     * @param rawValue the raw value string (e.g., "4644.0", "[144, 1167]")
     * @return the extracted Double value, or null if extraction fails
     */
    Double extractValue(String rawValue);

    /**
     * @return true if lower values are better, false if higher values are better
     */
    boolean isLowerBetter();
}
