package prog;

import prog.util.FocusDetector;

/**
 * 游戏窗口焦点监控辅助类。
 * 不创建线程，由Service轮询调用tick()方法。
 * 内部实现节流和状态追踪。
 *
 * 使用方式:
 * 1. Service.java中创建实例
 * 2. openpad时调用setEnabled(true)
 * 3. 每次轮询调用tick()
 * 4. closepad时调用setEnabled(false)
 */
public class FocusMonitor {

    /** 焦点检测间隔（毫秒），200ms足够响应用户切换窗口 */
    private static final long CHECK_INTERVAL_MS = 200;

    /** 上次检测时间戳 */
    private long lastCheckTime = 0;

    /** 上次检测到的焦点状态 */
    private boolean lastFocusState = true;

    /** 是否启用焦点监控 */
    private boolean enabled = false;

    /**
     * 启用/禁用焦点监控。
     * 禁用时会恢复所有被隐藏的overlay。
     *
     * @param enabled true启用焦点监控，false禁用
     */
    public void setEnabled(boolean enabled) {
        this.enabled = enabled;
        if (enabled) {
            // 启用时假设有焦点
            lastFocusState = true;
            // 立即检测一次（重置计时器）
            lastCheckTime = 0;
        } else {
            // 禁用时确保overlay可见
            if (AlwaysOnTopCoordinator.getInstance().isOverlaysHidden()) {
                AlwaysOnTopCoordinator.getInstance().showAllOverlays();
            }
        }
    }

    /**
     * 检查焦点监控是否启用。
     *
     * @return true如果启用
     */
    public boolean isEnabled() {
        return enabled;
    }

    /**
     * 由Service线程每次轮询时调用。
     * 内部实现200ms节流，避免过于频繁的进程检测。
     */
    public void tick() {
        if (!enabled) {
            return;
        }

        long now = System.currentTimeMillis();
        if (now - lastCheckTime < CHECK_INTERVAL_MS) {
            return;
        }
        lastCheckTime = now;

        // 检测焦点并响应变化
        boolean hasFocus = FocusDetector.isWarThunderFocused();

        if (hasFocus != lastFocusState) {
            lastFocusState = hasFocus;
            if (hasFocus) {
                AlwaysOnTopCoordinator.getInstance().showAllOverlays();
            } else {
                AlwaysOnTopCoordinator.getInstance().hideAllOverlays();
            }
        }
    }
}
