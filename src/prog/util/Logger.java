package prog.util;

import java.text.SimpleDateFormat;
import java.util.Date;

/**
 * Structured logging utility for the application.
 * Supports log levels, timestamps, and component identifiers.
 *
 * 日志级别说明：
 * - TRACE: 最详细的跟踪信息（仅开发调试使用）
 * - DEBUG: 调试信息
 * - INFO: 一般信息（默认级别）
 * - WARN: 警告信息（不影响运行但需关注）
 * - ERROR: 错误信息（影响功能）
 */
public class Logger {
    public enum Level {
        TRACE(-1), DEBUG(0), INFO(1), WARN(2), ERROR(3);

        final int value;

        Level(int value) {
            this.value = value;
        }
    }

    private static Level currentLevel = Level.INFO;
    private static final SimpleDateFormat dateFormat = new SimpleDateFormat("HH:mm:ss.SSS");

    /** 默认组件名，用于单参数日志方法 */
    private static final String DEFAULT_COMPONENT = "App";

    public static void setMinLevel(Level level) {
        currentLevel = level;
    }

    /**
     * 获取当前日志级别
     * @return 当前日志级别
     */
    public static Level getLevel() {
        return currentLevel;
    }

    // ===== 两参数方法（指定组件） =====

    public static void trace(String component, String message) {
        log(Level.TRACE, component, message);
    }

    public static void debug(String component, String message) {
        log(Level.DEBUG, component, message);
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

    /**
     * 记录错误信息和异常详情
     * 在 DEBUG 级别时打印完整堆栈
     */
    public static void error(String component, String message, Throwable t) {
        log(Level.ERROR, component, message + ": " + t.getMessage());
        if (currentLevel.value <= Level.DEBUG.value) {
            t.printStackTrace();
        }
    }

    // ===== 单参数方法（使用默认组件） =====

    public static void trace(String message) {
        log(Level.TRACE, DEFAULT_COMPONENT, message);
    }

    public static void debug(String message) {
        log(Level.DEBUG, DEFAULT_COMPONENT, message);
    }

    public static void info(String message) {
        log(Level.INFO, DEFAULT_COMPONENT, message);
    }

    public static void warn(String message) {
        log(Level.WARN, DEFAULT_COMPONENT, message);
    }

    public static void error(String message) {
        log(Level.ERROR, DEFAULT_COMPONENT, message);
    }

    /**
     * 记录错误信息和异常详情（单参数版本）
     * 在 DEBUG 级别时打印完整堆栈
     */
    public static void error(String message, Throwable t) {
        log(Level.ERROR, DEFAULT_COMPONENT, message + ": " + t.getMessage());
        if (currentLevel.value <= Level.DEBUG.value) {
            t.printStackTrace();
        }
    }

    /**
     * Specialized logging for event transactions.
     */
    public static void event(String action, String eventName, Object source, Object target) {
        if (currentLevel.value <= Level.INFO.value) {
            String msg = String.format("%s: %s -> %s: %s",
                    action,
                    source != null ? source.getClass().getSimpleName() : "Unknown",
                    target != null ? target.toString() : "Global",
                    eventName);
            log(Level.INFO, "EventBus", msg);
        }
    }

    private static void log(Level level, String component, String message) {
        if (level.value >= currentLevel.value) {
            String timestamp = dateFormat.format(new Date());
            if (level == Level.INFO) {
                System.out.printf("[%s] [%-10s] %s%n", timestamp, component, message);
            } else {
                System.out.printf("[%s] [%-10s] [%-5s] %s%n", timestamp, component, level, message);
            }
        }
    }
}
