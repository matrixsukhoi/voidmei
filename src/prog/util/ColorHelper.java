package prog.util;

import java.awt.Color;

/**
 * Utility class for color parsing and formatting.
 * Supports both hex (#RRGGBB, #RRGGBBAA) and decimal (R, G, B, A) formats.
 *
 * <p>Usage:</p>
 * <pre>
 * import static prog.util.ColorHelper.*;
 *
 * // Parse any format
 * Color c1 = parseColor("#FF5500AA", Color.WHITE);
 * Color c2 = parseColor("255, 85, 0, 170", Color.WHITE);
 *
 * // Format for display (hex)
 * String hex = toHexString(c1, true);  // "#FF5500AA"
 *
 * // Format for storage (decimal)
 * String dec = toDecimalString(c1);    // "255, 85, 0, 170"
 * </pre>
 */
public class ColorHelper {

    /**
     * Parses a color string in either hex or decimal format.
     *
     * <p>Supported formats:</p>
     * <ul>
     *   <li>Hex RGB: "#RRGGBB" (alpha defaults to 255)</li>
     *   <li>Hex RGBA: "#RRGGBBAA"</li>
     *   <li>Decimal RGB: "R, G, B" (alpha defaults to 255)</li>
     *   <li>Decimal RGBA: "R, G, B, A"</li>
     * </ul>
     *
     * @param text         The color string to parse
     * @param defaultColor Color to return if parsing fails
     * @return Parsed color, or defaultColor if text is null/empty/invalid
     */
    public static Color parseColor(String text, Color defaultColor) {
        if (text == null || text.trim().isEmpty()) {
            return defaultColor;
        }

        String trimmed = text.trim();

        // Try hex format first
        if (trimmed.startsWith("#")) {
            return parseHexColor(trimmed, defaultColor);
        }

        // Try decimal format
        return parseDecimalColor(trimmed, defaultColor);
    }

    /**
     * Parses a hex color string.
     *
     * @param hex          Hex string starting with # (e.g., "#RRGGBB" or "#RRGGBBAA")
     * @param defaultColor Fallback color on parse failure
     * @return Parsed color
     */
    private static Color parseHexColor(String hex, Color defaultColor) {
        try {
            String h = hex.substring(1); // Remove #

            if (h.length() == 6) {
                // #RRGGBB - no alpha
                int r = Integer.parseInt(h.substring(0, 2), 16);
                int g = Integer.parseInt(h.substring(2, 4), 16);
                int b = Integer.parseInt(h.substring(4, 6), 16);
                return new Color(r, g, b);
            } else if (h.length() == 8) {
                // #RRGGBBAA - with alpha
                int r = Integer.parseInt(h.substring(0, 2), 16);
                int g = Integer.parseInt(h.substring(2, 4), 16);
                int b = Integer.parseInt(h.substring(4, 6), 16);
                int a = Integer.parseInt(h.substring(6, 8), 16);
                return new Color(r, g, b, a);
            }
        } catch (Exception e) {
            // Fall through to default
        }
        return defaultColor;
    }

    /**
     * Parses a decimal color string in "R, G, B" or "R, G, B, A" format.
     *
     * @param decimal      Decimal string (e.g., "255, 128, 0" or "255, 128, 0, 200")
     * @param defaultColor Fallback color on parse failure
     * @return Parsed color
     */
    private static Color parseDecimalColor(String decimal, Color defaultColor) {
        try {
            String cleaned = decimal.replaceAll("\\s+", "");
            String[] parts = cleaned.split(",");

            if (parts.length >= 3) {
                int r = Integer.parseInt(parts[0]);
                int g = Integer.parseInt(parts[1]);
                int b = Integer.parseInt(parts[2]);
                int a = (parts.length >= 4) ? Integer.parseInt(parts[3]) : 255;

                // Clamp values to valid range
                r = clamp(r, 0, 255);
                g = clamp(g, 0, 255);
                b = clamp(b, 0, 255);
                a = clamp(a, 0, 255);

                return new Color(r, g, b, a);
            }
        } catch (Exception e) {
            // Fall through to default
        }
        return defaultColor;
    }

    /**
     * Formats a color as a decimal RGBA string for config storage.
     *
     * @param color The color to format
     * @return Decimal string in format "R, G, B, A" (e.g., "255, 128, 0, 200")
     */
    public static String toDecimalString(Color color) {
        if (color == null) {
            return "255, 255, 255, 255";
        }
        return color.getRed() + ", " + color.getGreen() + ", " + color.getBlue() + ", " + color.getAlpha();
    }

    /**
     * Formats a color as a hex string for display.
     *
     * @param color        The color to format
     * @param includeAlpha If true, includes alpha channel (#RRGGBBAA); if false, #RRGGBB
     * @return Hex string starting with # (e.g., "#FF8000" or "#FF8000C8")
     */
    public static String toHexString(Color color, boolean includeAlpha) {
        if (color == null) {
            return includeAlpha ? "#FFFFFFFF" : "#FFFFFF";
        }

        if (includeAlpha) {
            return String.format("#%02X%02X%02X%02X",
                color.getRed(), color.getGreen(), color.getBlue(), color.getAlpha());
        } else {
            return String.format("#%02X%02X%02X",
                color.getRed(), color.getGreen(), color.getBlue());
        }
    }

    /**
     * Detects if a string appears to be in hex format.
     *
     * @param text The text to check
     * @return true if text starts with #
     */
    public static boolean isHexFormat(String text) {
        return text != null && text.trim().startsWith("#");
    }

    /**
     * Clamps an integer value to the specified range.
     */
    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }
}
