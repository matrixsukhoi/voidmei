package ui.util;

/**
 * Utility for formatting numbers directly into char buffers without String
 * allocation.
 * Optimized for high-frequency UI rendering.
 */
public class FastNumberFormatter {

    private static final char[] DIGITS = "0123456789".toCharArray();

    /**
     * Formats a double value into the provided buffer.
     * 
     * @param value     The value to format
     * @param buffer    The destination buffer
     * @param precision Number of decimal places (0-5)
     * @return The number of characters written
     */
    public static int format(double value, char[] buffer, int precision) {
        if (Double.isNaN(value)) {
            buffer[0] = 'N';
            buffer[1] = '/';
            buffer[2] = 'A';
            return 3;
        }

        int pos = 0;
        if (value < 0) {
            double threshold = 0.5;
            for (int i = 0; i < precision; i++)
                threshold /= 10.0;
            if (value <= -threshold) {
                buffer[pos++] = '-';
            }
            value = -value;
        }

        double scale = 1.0;
        for (int i = 0; i < precision; i++)
            scale *= 10.0;

        long integral = (long) value;
        double fractional = value - integral;

        long scaledFraction = Math.round(fractional * scale);
        if (scaledFraction >= scale) {
            integral += 1;
            scaledFraction = 0;
        }

        if (integral == 0) {
            buffer[pos++] = '0';
        } else {
            long v = integral;
            long temp = v;
            int len = 0;
            while (temp > 0) {
                temp /= 10;
                len++;
            }
            pos += len;
            int cursor = pos - 1;
            while (v > 0) {
                buffer[cursor--] = DIGITS[(int) (v % 10)];
                v /= 10;
            }
        }

        if (precision > 0) {
            buffer[pos++] = '.';
            int fracEnd = pos + precision;
            long rem = scaledFraction;
            for (int i = 0; i < precision; i++) {
                buffer[fracEnd - 1 - i] = DIGITS[(int) (rem % 10)];
                rem /= 10;
            }
            pos += precision;
        }

        return pos;
    }

    /**
     * Formats an integer value into the provided buffer.
     */
    public static int format(long value, char[] buffer) {
        int pos = 0;
        if (value < 0) {
            buffer[pos++] = '-';
            value = -value;
        }
        if (value == 0) {
            buffer[pos++] = '0';
            return pos;
        }

        long v = value;
        int len = 0;
        long temp = v;
        while (temp > 0) {
            temp /= 10;
            len++;
        }

        pos += len;
        int cursor = pos - 1;
        while (v > 0) {
            buffer[cursor--] = DIGITS[(int) (v % 10)];
            v /= 10;
        }
        return pos;
    }
}
