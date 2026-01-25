package ui.overlay;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Font;
import java.awt.Toolkit;
import java.util.List;
import java.util.function.Predicate;
import java.util.function.Supplier;

import com.alee.laf.panel.WebPanel;
import ui.base.DraggableOverlay;
import com.alee.extended.layout.VerticalFlowLayout;
import prog.util.Logger;

import prog.Application;

/**
 * Base overlay class using composition for pluggable rendering strategies.
 */
public class BaseOverlay extends DraggableOverlay {
    private static final long serialVersionUID = 1L;

    public volatile boolean doit = true;
    protected WebPanel dataPanel;
    protected String fontName = "";
    protected Font displayFont;
    protected int fontSize = 16;
    protected float scaleFactor = 1.0f;
    protected int width;
    protected int height;
    protected int alpha = 180;

    protected boolean isPreview = false;
    protected Supplier<List<String>> dataSupplier;
    protected List<String> lastData = null;
    protected String configXKey;
    protected String configYKey;
    protected ConfigBridge configBridge;

    protected Font monoFont;

    // Composition: pluggable renderer
    protected OverlayRenderer renderer;

    public interface ConfigBridge {
        String getConfig(String key);

        void setConfig(String key, String value);
    }

    public BaseOverlay() {
        super();
        prog.util.Logger.info("Overlay",
                "Created instance: " + this.getClass().getSimpleName() + "@" + Integer.toHexString(hashCode()));
        // Default renderer
        this.renderer = new ZebraListRenderer();
    }

    @Override
    public void dispose() {
        prog.util.Logger.info("Overlay",
                "Disposing instance: " + this.getClass().getSimpleName() + "@" + Integer.toHexString(hashCode()));
        super.dispose();
    }

    public void setRenderer(OverlayRenderer renderer) {
        if (renderer != null) {
            this.renderer = renderer;
        }
    }

    protected prog.config.OverlaySettings settings;

    public OverlayRenderer getRenderer() {
        return this.renderer;
    }

    public void setHeaderMatcher(Predicate<String> matcher) {
        if (renderer != null) {
            renderer.setHeaderMatcher(matcher);
        }
    }

    public void setAlpha(int alpha) {
        this.alpha = alpha;
    }

    public void init(prog.config.OverlaySettings settings, Supplier<List<String>> supplier) {
        this.settings = settings;
        this.dataSupplier = supplier;

        // MUST be called before window becomes displayable
        this.setUndecorated(true);

        int screenHeight = Toolkit.getDefaultToolkit().getScreenSize().height;
        scaleFactor = (float) screenHeight / 1440.0f;
        fontSize = Math.round(16 * scaleFactor);
        width = Math.round(Application.defaultFontsize * 36 * scaleFactor);
        height = Application.defaultFontsize * 72;

        this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));
        this.setShadeWidth(0);

        setupFont();
        loadPosition();

        ui.WebLafSettings.setWindowFocus(this);

        dataPanel = new WebPanel();
        dataPanel.setLayout(new VerticalFlowLayout(0, 0));
        dataPanel.setBackground(new Color(20, 20, 20, alpha));

        WebPanel mainPanel = new WebPanel(new BorderLayout());
        mainPanel.setWebColoredBackground(false);
        mainPanel.setBackground(new Color(0, 0, 0, 0));
        mainPanel.add(dataPanel, BorderLayout.CENTER);
        this.add(mainPanel);

        this.getRootPane().setCursor(Application.blankCursor);
        this.getContentPane().setCursor(Application.blankCursor);
        this.getGlassPane().setCursor(Application.blankCursor);

        ui.WebLafSettings.setWindowFocus(this);

        new Thread(this).start();
    }

    public void initPreview(prog.config.OverlaySettings settings, Supplier<List<String>> supplier) {
        this.settings = settings;
        this.dataSupplier = supplier;
        this.isPreview = true;

        this.setUndecorated(true);

        int screenHeight = Toolkit.getDefaultToolkit().getScreenSize().height;
        scaleFactor = (float) screenHeight / 1440.0f;
        fontSize = Math.round(16 * scaleFactor);
        width = Math.round(Application.defaultFontsize * 36 * scaleFactor);
        height = Application.defaultFontsize * 72;

        this.getWebRootPaneUI().setMiddleBg(Application.previewColor);
        this.getWebRootPaneUI().setTopBg(Application.previewColor);
        this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));
        this.setShadeWidth(0);

        setupFont();
        loadPosition();

        ui.WebLafSettings.setWindowFocus(this);

        dataPanel = new WebPanel();
        dataPanel.setLayout(new VerticalFlowLayout(0, 0));
        dataPanel.setBackground(new Color(20, 20, 20, alpha));

        WebPanel mainPanel = new WebPanel(new BorderLayout());
        mainPanel.setWebColoredBackground(false);
        mainPanel.setBackground(new Color(0, 0, 0, 0));
        mainPanel.add(dataPanel, BorderLayout.CENTER);
        this.add(mainPanel);

        setupDragListeners();
        applyPreviewStyle();
        this.setCursor(null);
        setVisible(true);
    }

    protected void setupFont() {
        String fontName = "";
        int fontSizeAdd = 0;

        if (settings != null) {
            fontName = settings.getFontName();
            fontSizeAdd = settings.getFontSizeAdd();
        }

        if (fontName == null || fontName.isEmpty()) {
            fontName = Application.defaultFontName;
        }

        this.displayFont = new Font(fontName, Font.PLAIN, 14 + fontSizeAdd);
        this.monoFont = new Font(Application.defaultNumfontName, Font.PLAIN, 14 + fontSizeAdd);
    }

    protected void loadPosition() {
        int w = getWidth();
        int h = getHeight();
        if (w == 0)
            w = 200; // default
        if (h == 0)
            h = 100;

        int sx = -1, sy = -1;

        if (settings != null) {
            sx = settings.getWindowX(w);
            sy = settings.getWindowY(h);
        }

        if (sx == -1 || sy == -1) {
            this.setLocation(0, 0);
        } else {
            this.setLocation(sx, sy);
        }
    }

    @Override
    public void saveCurrentPosition() {
        if (settings != null) {
            settings.saveWindowPosition(getLocation().x, getLocation().y);
        } else {
            super.saveCurrentPosition();
        }
    }

    protected boolean shouldExit() {
        return false;
    }

    protected int getRefreshInterval() {
        return 200;
    }

    protected boolean isVisibleNow() {
        return true;
    }

    @Override
    public void run() {
        while (doit) {
            if (shouldExit())
                break;

            if (isPreview || isVisibleNow()) {
                List<String> currentData = dataSupplier.get();
                if (currentData != null && !currentData.equals(lastData)) {
                    Logger.info("BaseOverlay", "Data changed (Lines: " + currentData.size() + "). Updating UI.");
                    updateUI(currentData);
                    lastData = currentData;
                }
                this.setVisible(true);
            } else {
                this.setVisible(false);
            }

            try {
                Thread.sleep(getRefreshInterval());
            } catch (InterruptedException e) {
                break;
            }
        }
        this.dispose();
    }

    private void updateUI(List<String> currentData) {
        javax.swing.SwingUtilities.invokeLater(() -> {
            // Delegate rendering to the pluggable renderer
            renderer.render(currentData, dataPanel, displayFont, alpha);
            adjustPosition();
            this.getContentPane().repaint();
        });
    }

    private void adjustPosition() {
        int preferredHeight = dataPanel.getPreferredSize().height;
        int maxHeight = Toolkit.getDefaultToolkit().getScreenSize().height - 40;
        if (preferredHeight > maxHeight)
            preferredHeight = maxHeight;

        if (Math.abs(this.getHeight() - preferredHeight) > 2) {
            int currentX = this.getLocation().x;
            int screenHeight = Toolkit.getDefaultToolkit().getScreenSize().height;
            // Maintain bottom anchor if not dragged
            if (!isPreview || configBridge == null || configBridge.getConfig(configYKey) == null
                    || configBridge.getConfig(configYKey).isEmpty()) {
                this.setBounds(currentX, screenHeight - 10 - preferredHeight, width, preferredHeight);
            } else {
                this.setSize(width, preferredHeight);
            }
        }
    }

    public void stop() {
        this.doit = false;
    }
}
