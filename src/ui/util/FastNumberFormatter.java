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

    /**
     * Formats a time in seconds into MM'SS format.
     * 
     * @param value  Time in seconds
     * @param buffer Destination buffer
     * @return Number of characters written
     */
    public static int formatTime(double value, char[] buffer) {
        if (Double.isNaN(value) || value < 0) {
            buffer[0] = '-';
            buffer[1] = '-';
            buffer[2] = '\'';
            buffer[3] = '-';
            buffer[4] = '-';
            return 5;
        }

        int totalSeconds = (int) value;
        int minutes = totalSeconds / 60;
        int seconds = totalSeconds % 60;

        int pos = 0;
        // Minutes (at least 2 digits)
        if (minutes < 10) {
            buffer[pos++] = '0';
            buffer[pos++] = DIGITS[minutes];
        } else {
            // Support up to 999 minutes for long fuel times
            if (minutes >= 100) {
                if (minutes >= 1000) {
                    // Overflow
                    buffer[pos++] = '9';
                    buffer[pos++] = '9';
                    buffer[pos++] = '9';
                } else {
                    buffer[pos++] = DIGITS[minutes / 100];
                    buffer[pos++] = DIGITS[(minutes / 10) % 10];
                    buffer[pos++] = DIGITS[minutes % 10];
                }
            } else {
                buffer[pos++] = DIGITS[minutes / 10];
                buffer[pos++] = DIGITS[minutes % 10];
            }
        }

        buffer[pos++] = '\'';

        // Seconds (always 2 digits)
        buffer[pos++] = DIGITS[seconds / 10];
        buffer[pos++] = DIGITS[seconds % 10];

        return pos;
    }
}
