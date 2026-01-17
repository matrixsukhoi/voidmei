package prog.util;

import java.text.SimpleDateFormat;
import java.util.Date;

/**
 * Structured logging utility for the application.
 * Supports log levels, timestamps, and component identifiers.
 */
public class Logger {
    public enum Level {
        DEBUG(0), INFO(1), WARN(2), ERROR(3);

        final int value;

        Level(int value) {
            this.value = value;
        }
    }

    private static Level minLevel = Level.INFO;
    private static final SimpleDateFormat dateFormat = new SimpleDateFormat("HH:mm:ss.SSS");

    public static void setMinLevel(Level level) {
        minLevel = level;
    }

    public static void info(String component, String message) {
        log(Level.INFO, component, message);
    }

    public static void warn(String component, String message) {
        log(Level.WARN, component, message);
    }

    public static void error(String component, String message) {
        log(Level.ERROR, component, message);
    }

    public static void debug(String component, String message) {
        log(Level.DEBUG, component, message);
    }

    /**
     * Specialized logging for event transactions.
     */
    public static void event(String action, String eventName, Object source, Object target) {
        if (minLevel.value <= Level.INFO.value) {
            String msg = String.format("%s: %s -> %s: %s",
                    action,
                    source != null ? source.getClass().getSimpleName() : "Unknown",
                    target != null ? target.toString() : "Global",
                    eventName);
            log(Level.INFO, "EventBus", msg);
        }
    }

    private static void log(Level level, String component, String message) {
        if (level.value >= minLevel.value) {
            String timestamp = dateFormat.format(new Date());
            System.out.printf("[%s] [%-10s] [%-5s] %s%n", timestamp, component, level, message);
        }
    }
}
