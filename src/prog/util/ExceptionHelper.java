package prog.util;

/**
 * 统一异常处理工具类
 * 替代散落在代码中的空 catch 块和 printStackTrace()
 *
 * 设计原则：
 * - 保持与原有空 catch 块的行为一致（不改变控制流）
 * - 提供可选的日志记录功能，增加可观测性
 * - 线程安全，可在任意线程中使用
 */
public class ExceptionHelper {

    /**
     * 记录异常但不中断流程（用于非关键操作）
     * 控制流不变，只是增加可观测性
     *
     * @param e 捕获的异常
     * @param context 上下文描述，用于日志
     */
    public static void logAndContinue(Exception e, String context) {
        Logger.warn(context + ": " + e.getMessage());
        if (Logger.getLevel().compareTo(Logger.Level.DEBUG) <= 0) {
            e.printStackTrace();
        }
    }

    /**
     * 静默忽略 InterruptedException
     * 恢复线程中断状态，这是正确的处理方式
     *
     * @param e 中断异常
     */
    public static void ignore(InterruptedException e) {
        Thread.currentThread().interrupt();
    }

    /**
     * 静默休眠（替代重复的 try-catch Thread.sleep）
     * 如果被中断，恢复中断状态但不抛出异常
     *
     * @param millis 休眠时间（毫秒）
     */
    public static void sleepQuietly(long millis) {
        try {
            Thread.sleep(millis);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        }
    }

    /**
     * 静默休眠（严格保持原有空 catch 块的行为）
     * 中断时不恢复中断状态，与原代码行为完全一致
     *
     * 注意：此方法仅用于需要严格行为一致性的场景
     * 新代码应优先使用 sleepQuietly()
     *
     * @param millis 休眠时间（毫秒）
     */
    public static void sleepQuietlyStrict(long millis) {
        try {
            Thread.sleep(millis);
        } catch (InterruptedException e) {
            // 严格保持原行为：静默忽略，不恢复中断状态
        }
    }

    /**
     * 静默关闭资源（用于 finally 块中的资源清理）
     *
     * @param closeable 可关闭的资源
     */
    public static void closeQuietly(AutoCloseable closeable) {
        if (closeable != null) {
            try {
                closeable.close();
            } catch (Exception e) {
                // 静默忽略关闭异常
            }
        }
    }
}
