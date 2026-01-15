package ui.base;

import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;

import com.alee.laf.rootpane.WebFrame;
import prog.app;
import ui.model.ConfigProvider;

/**
 * Base class for draggable overlay windows.
 * Provides common functionality for window dragging, position saving, and
 * preview mode.
 */
public abstract class DraggableOverlay extends WebFrame implements Runnable {
    private static final long serialVersionUID = 1L;

    // Thread control
    public volatile boolean doit = true;

    // Config for position saving
    protected ConfigProvider config;
    protected String posXKey;
    protected String posYKey;

    // Dragging state
    private int isDragging;
    private int dragStartX, dragStartY;

    // Refresh timing
    protected long refreshInterval = 100; // Default 100ms

    /**
     * Initialize position keys for saving/loading window position.
     */
    protected void setPositionKeys(String xKey, String yKey) {
        this.posXKey = xKey;
        this.posYKey = yKey;
    }

    /**
     * Load saved position from config.
     * 
     * @return int[2] with {x, y} coordinates, or default values if not saved
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
                // Save position only when drag ends
                saveCurrentPosition();
            }
        });

        addMouseMotionListener(new MouseMotionAdapter() {
            @Override
            public void mouseDragged(MouseEvent e) {
                if (isDragging == 1) {
                    // Use screen coordinates for robust dragging calculation
                    // Window Pos = Mouse Screen Pos - Initial Mouse Offset relative to Window
                    setLocation(e.getXOnScreen() - dragStartX, e.getYOnScreen() - dragStartY);

                    // Only save on release to avoid IO spam, but setVisible/repaint is fine
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
        this.getWebRootPaneUI().setMiddleBg(app.previewColor);
        this.getWebRootPaneUI().setTopBg(app.previewColor);
        this.setCursor(null);
    }

    /**
     * Setup transparent window styling.
     * Makes the window background fully transparent for overlay display.
     */
    protected void setupTransparentWindow() {
        this.getWebRootPaneUI().setMiddleBg(new java.awt.Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setTopBg(new java.awt.Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setBorderColor(new java.awt.Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setInnerBorderColor(new java.awt.Color(0, 0, 0, 0));
        ui.uiWebLafSetting.setWindowOpaque(this);
    }

    /**
     * Set the refresh interval in milliseconds.
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

    /**
     * Template method for update logic. Override in subclasses.
     */
    protected abstract void updateData();

    /**
     * Template method for repaint. Override in subclasses.
     * Public so it can be called from external threads like uiThread.
     */
    public abstract void drawTick();

    @Override
    public void run() {
        long lastUpdate = 0;

        while (doit) {
            try {
                Thread.sleep(app.threadSleepTime);
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
