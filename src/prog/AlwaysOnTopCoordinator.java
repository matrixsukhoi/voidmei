package prog;

import java.awt.Window;
import java.lang.ref.WeakReference;
import java.util.Iterator;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.atomic.AtomicInteger;
import javax.swing.SwingUtilities;

/**
 * Coordinates alwaysOnTop state across all overlays and dialogs.
 *
 * <p>This coordinator solves the timing problem where dialogs may be triggered
 * before overlays exist, and overlays created later would cover the dialogs.
 *
 * <h3>How it works:</h3>
 * <ul>
 *   <li>{@link #dialogWillShow()} increments pending dialog count and suspends all registered overlays</li>
 *   <li>{@link #dialogDidDismiss()} decrements count; when zero, restores alwaysOnTop for all overlays</li>
 *   <li>{@link #registerOverlay(Window)} registers a new overlay; only sets alwaysOnTop if no pending dialogs</li>
 * </ul>
 *
 * <h3>Thread Safety:</h3>
 * <ul>
 *   <li>pendingDialogs uses AtomicInteger for lock-free counting</li>
 *   <li>overlays uses CopyOnWriteArrayList to avoid ConcurrentModificationException</li>
 *   <li>z-order changes are dispatched to EDT via SwingUtilities.invokeLater()</li>
 * </ul>
 */
public class AlwaysOnTopCoordinator {

    private static final AlwaysOnTopCoordinator INSTANCE = new AlwaysOnTopCoordinator();

    private final AtomicInteger pendingDialogs = new AtomicInteger(0);
    private final CopyOnWriteArrayList<WeakReference<Window>> overlays = new CopyOnWriteArrayList<>();

    /** 标记overlay是否因游戏失焦而被隐藏 */
    private volatile boolean overlaysHidden = false;

    private AlwaysOnTopCoordinator() {
        // Singleton
    }

    /**
     * Gets the singleton instance.
     */
    public static AlwaysOnTopCoordinator getInstance() {
        return INSTANCE;
    }

    /**
     * Registers an overlay window with the coordinator.
     *
     * <p>If no dialogs are pending, sets alwaysOnTop(true) immediately.
     * Otherwise, the overlay will be set to alwaysOnTop when all dialogs close.
     *
     * @param window the overlay window to register
     */
    public void registerOverlay(Window window) {
        if (window == null) {
            return;
        }

        // Add to tracking list
        overlays.add(new WeakReference<>(window));

        // Only set alwaysOnTop if no pending dialogs
        if (pendingDialogs.get() == 0) {
            setAlwaysOnTopOnEDT(window, true);
        } else {
            prog.util.Logger.debug("AlwaysOnTopCoordinator",
                    "Overlay registered but pendingDialogs=" + pendingDialogs.get() + ", deferring alwaysOnTop");
        }
    }

    /**
     * Unregisters an overlay window from the coordinator.
     *
     * <p>Should be called when an overlay is disposed.
     *
     * @param window the overlay window to unregister
     */
    public void unregisterOverlay(Window window) {
        if (window == null) {
            return;
        }

        Iterator<WeakReference<Window>> it = overlays.iterator();
        while (it.hasNext()) {
            WeakReference<Window> ref = it.next();
            Window w = ref.get();
            if (w == null || w == window) {
                overlays.remove(ref);
            }
        }
    }

    /**
     * Called before showing a dialog.
     *
     * <p>Increments pending dialog count and suspends alwaysOnTop for all overlays.
     */
    public void dialogWillShow() {
        int count = pendingDialogs.incrementAndGet();
        prog.util.Logger.debug("AlwaysOnTopCoordinator",
                "dialogWillShow: pendingDialogs now " + count);

        // 清理所有打开的tooltip/popover，防止被dialog覆盖
        ui.replica.ReplicaBuilder.disposeAllPopovers();

        suspendAll();
    }

    /**
     * Called after a dialog is dismissed.
     *
     * <p>Decrements pending dialog count. When count reaches zero, restores
     * alwaysOnTop for all overlays.
     */
    public void dialogDidDismiss() {
        int count = pendingDialogs.decrementAndGet();
        prog.util.Logger.debug("AlwaysOnTopCoordinator",
                "dialogDidDismiss: pendingDialogs now " + count);

        if (count <= 0) {
            // Reset to 0 in case of underflow
            pendingDialogs.compareAndSet(count, 0);
            restoreAll();
        }
    }

    /**
     * Suspends alwaysOnTop for all registered overlays.
     */
    private void suspendAll() {
        cleanupDeadReferences();
        for (WeakReference<Window> ref : overlays) {
            Window w = ref.get();
            if (w != null) {
                setAlwaysOnTopOnEDT(w, false);
            }
        }
    }

    /**
     * Restores alwaysOnTop for all registered overlays.
     */
    private void restoreAll() {
        cleanupDeadReferences();
        for (WeakReference<Window> ref : overlays) {
            Window w = ref.get();
            if (w != null) {
                setAlwaysOnTopOnEDT(w, true);
            }
        }
    }

    /**
     * Sets alwaysOnTop on the EDT to ensure thread safety.
     */
    private void setAlwaysOnTopOnEDT(Window window, boolean value) {
        if (SwingUtilities.isEventDispatchThread()) {
            window.setAlwaysOnTop(value);
        } else {
            SwingUtilities.invokeLater(() -> window.setAlwaysOnTop(value));
        }
    }

    /**
     * Removes dead WeakReferences from the overlay list.
     */
    private void cleanupDeadReferences() {
        overlays.removeIf(ref -> ref.get() == null);
    }

    /**
     * Returns the current pending dialog count (for debugging).
     */
    public int getPendingDialogCount() {
        return pendingDialogs.get();
    }

    /**
     * Returns the number of registered overlays (for debugging).
     */
    public int getRegisteredOverlayCount() {
        cleanupDeadReferences();
        return overlays.size();
    }

    // ========== 游戏失焦时隐藏/显示overlay ==========

    /**
     * 隐藏所有已注册的overlay窗口（不销毁实例）。
     * 用于游戏失焦时自动隐藏HUD。
     */
    public void hideAllOverlays() {
        prog.util.Logger.info("AlwaysOnTopCoordinator", "hideAllOverlays 调用，已注册 overlay 数: " + overlays.size());
        if (overlaysHidden) {
            prog.util.Logger.info("AlwaysOnTopCoordinator", "overlay 已处于隐藏状态，跳过");
            return;
        }
        overlaysHidden = true;
        cleanupDeadReferences();
        for (WeakReference<Window> ref : overlays) {
            Window w = ref.get();
            if (w != null && w.isVisible()) {
                setVisibleOnEDT(w, false);
            }
        }
        prog.util.Logger.info("AlwaysOnTopCoordinator", "所有overlay已隐藏 (游戏失焦)");
    }

    /**
     * 显示所有被隐藏的overlay窗口。
     * 用于游戏重新获得焦点时恢复显示。
     */
    public void showAllOverlays() {
        prog.util.Logger.info("AlwaysOnTopCoordinator", "showAllOverlays 调用，已注册 overlay 数: " + overlays.size());
        if (!overlaysHidden) {
            prog.util.Logger.info("AlwaysOnTopCoordinator", "overlay 已处于显示状态，跳过");
            return;
        }
        overlaysHidden = false;
        cleanupDeadReferences();
        for (WeakReference<Window> ref : overlays) {
            Window w = ref.get();
            if (w != null) {
                setVisibleOnEDT(w, true);
            }
        }
        prog.util.Logger.info("AlwaysOnTopCoordinator", "所有overlay已恢复 (游戏获焦)");
    }

    /**
     * 检查overlay是否因游戏失焦而被隐藏。
     *
     * @return true如果overlay当前被隐藏
     */
    public boolean isOverlaysHidden() {
        return overlaysHidden;
    }

    /**
     * 在EDT线程上设置窗口可见性，确保线程安全。
     */
    private void setVisibleOnEDT(Window window, boolean visible) {
        if (SwingUtilities.isEventDispatchThread()) {
            window.setVisible(visible);
        } else {
            SwingUtilities.invokeLater(() -> window.setVisible(visible));
        }
    }
}
