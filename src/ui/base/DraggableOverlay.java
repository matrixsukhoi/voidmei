package ui.base;

import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;

import com.alee.laf.rootpane.WebFrame;
import prog.Application;
import prog.config.ConfigProvider;

/**
 * Base class for draggable overlay windows.
 * Provides common functionality for window dragging, position saving, and
 * preview mode.
 * 
 * Subclasses can be either:
 * - Polling-based: Override run(), updateData(), drawTick()
 * - Event-driven: Implement PoolListener and subscribe to AttributePool
 */
public abstract class DraggableOverlay extends WebFrame implements Runnable {
    private static final long serialVersionUID = 1L;

    // Thread control
    public volatile boolean doit = true;

    public DraggableOverlay() {
        super();
        prog.util.Logger.info("Overlay",
                "Created instance: " + this.getClass().getSimpleName() + "@" + Integer.toHexString(hashCode()));
    }

    @Override
    public void dispose() {
        prog.util.Logger.info("Overlay",
                "Disposing instance: " + this.getClass().getSimpleName() + "@" + Integer.toHexString(hashCode()));
        super.dispose();
    }

    // Config for position saving
    protected ConfigProvider config;
    protected String posXKey;
    protected String posYKey;

    // Direct section settings
    protected prog.config.OverlaySettings overlaySettings;

    public void setOverlaySettings(prog.config.OverlaySettings settings) {
        this.overlaySettings = settings;
    }

    // Dragging State
    private int isDragging;
    private int dragStartX, dragStartY;

    // Refresh timing (for polling mode)
    protected long refreshInterval = 100;

    /**
     * Initialize position keys for saving/loading window position.
     */
    protected void setPositionKeys(String xKey, String yKey) {
        this.posXKey = xKey;
        this.posYKey = yKey;
    }

    /**
     * Load saved position from config.
     */
    protected int[] loadPosition(int defaultX, int defaultY) {
        int x = defaultX;
        int y = defaultY;

        if (config != null && posXKey != null && posYKey != null) {
            String savedX = config.getConfig(posXKey);
            String savedY = config.getConfig(posYKey);

            if (savedX != null && !savedX.isEmpty()) {
                try {
                    x = Integer.parseInt(savedX);
                } catch (NumberFormatException e) {
                }
            }
            if (savedY != null && !savedY.isEmpty()) {
                try {
                    y = Integer.parseInt(savedY);
                } catch (NumberFormatException e) {
                }
            }
        }

        return new int[] { x, y };
    }

    /**
     * Save current window position to config.
     */
    public void saveCurrentPosition() {
        if (config != null && posXKey != null && posYKey != null) {
            config.setConfig(posXKey, Integer.toString(getLocation().x));
            config.setConfig(posYKey, Integer.toString(getLocation().y));
        }
    }

    /**
     * Setup drag listeners for preview mode.
     */
    protected void setupDragListeners() {
        addMouseListener(new MouseAdapter() {
            @Override
            public void mousePressed(MouseEvent e) {
                isDragging = 1;
                dragStartX = e.getX();
                dragStartY = e.getY();
            }

            @Override
            public void mouseReleased(MouseEvent e) {
                isDragging = 0;
                saveCurrentPosition();
            }
        });

        addMouseMotionListener(new MouseMotionAdapter() {
            @Override
            public void mouseDragged(MouseEvent e) {
                if (isDragging == 1) {
                    setLocation(e.getXOnScreen() - dragStartX, e.getYOnScreen() - dragStartY);
                    setVisible(true);
                    repaint();
                }
            }
        });
    }

    /**
     * Apply preview mode styling.
     */
    protected void applyPreviewStyle() {
        this.getWebRootPaneUI().setMiddleBg(Application.previewColor);
        this.getWebRootPaneUI().setTopBg(Application.previewColor);
        this.setCursor(null);
    }

    /**
     * Setup transparent window styling.
     */
    protected void setupTransparentWindow() {
        this.getWebRootPaneUI().setMiddleBg(new java.awt.Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setTopBg(new java.awt.Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setBorderColor(new java.awt.Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setInnerBorderColor(new java.awt.Color(0, 0, 0, 0));
        ui.WebLafSettings.setWindowOpaque(this);
    }

    /**
     * Set the refresh interval for polling mode.
     */
    public void setRefreshInterval(long interval) {
        this.refreshInterval = interval;
    }

    /**
     * Helper to get config value with default.
     */
    protected String getConfigOrDefault(String key, String defaultValue) {
        if (config == null)
            return defaultValue;
        String value = config.getConfig(key);
        return (value != null && !value.isEmpty()) ? value : defaultValue;
    }

    protected int getConfigIntOrDefault(String key, int defaultValue) {
        String value = getConfigOrDefault(key, null);
        if (value == null)
            return defaultValue;
        try {
            return Integer.parseInt(value);
        } catch (NumberFormatException e) {
            return defaultValue;
        }
    }

    // --- Polling Mode Hooks (Override for polling, no-op for event-driven) ---

    /**
     * Update data from source. Override for polling-based updates.
     * Event-driven subclasses can leave this as no-op.
     */
    protected void updateData() {
        // No-op by default
    }

    /**
     * Repaint the overlay. Override for custom repaint logic.
     * Called by UIThread for compatibility.
     */
    public void drawTick() {
        // No-op by default
    }

    /**
     * Polling loop. Override if custom polling behavior is needed.
     * Event-driven subclasses can leave doit=false to skip polling.
     */
    @Override
    public void run() {
        long lastUpdate = 0;

        while (doit) {
            try {
                Thread.sleep(Application.threadSleepTime);
            } catch (InterruptedException e) {
                e.printStackTrace();
            }

            long now = System.currentTimeMillis();
            if (now - lastUpdate > refreshInterval) {
                lastUpdate = now;
                updateData();
                drawTick();
            }
        }
    }
}
