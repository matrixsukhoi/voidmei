package ui.overlay;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Font;
import java.awt.Toolkit;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import java.util.List;
import java.util.function.Predicate;
import java.util.function.Supplier;

import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.extended.layout.VerticalFlowLayout;

import prog.app;

/**
 * Base overlay class using composition for pluggable rendering strategies.
 */
public class BaseOverlay extends WebFrame implements Runnable {
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
    protected int isDragging = 0;
    protected int mouseX, mouseY;

    protected Supplier<List<String>> dataSupplier;
    protected List<String> lastData = null;
    protected String configXKey;
    protected String configYKey;
    protected ConfigBridge configBridge;

    // Composition: pluggable renderer
    protected OverlayRenderer renderer;

    public interface ConfigBridge {
        String getConfig(String key);

        void setConfig(String key, String value);
    }

    public BaseOverlay() {
        super();
        // Default renderer
        this.renderer = new ZebraListRenderer();
    }

    public void setRenderer(OverlayRenderer renderer) {
        if (renderer != null) {
            this.renderer = renderer;
        }
    }

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

    public void init(ConfigBridge bridge, String xKey, String yKey, Supplier<List<String>> supplier) {
        this.configBridge = bridge;
        this.configXKey = xKey;
        this.configYKey = yKey;
        this.dataSupplier = supplier;

        // MUST be called before window becomes displayable
        this.setUndecorated(true);

        int screenHeight = Toolkit.getDefaultToolkit().getScreenSize().height;
        scaleFactor = (float) screenHeight / 1440.0f;
        fontSize = Math.round(16 * scaleFactor);
        width = Math.round(app.defaultFontsize * 36 * scaleFactor);
        height = app.defaultFontsize * 72;

        int initialX = Toolkit.getDefaultToolkit().getScreenSize().width - width;
        int initialY = Toolkit.getDefaultToolkit().getScreenSize().height - 10 - height;

        if (bridge != null) {
            String savedX = bridge.getConfig(xKey);
            String savedY = bridge.getConfig(yKey);
            if (savedX != null && !savedX.isEmpty() && savedY != null && !savedY.isEmpty()) {
                try {
                    initialX = Integer.parseInt(savedX);
                    initialY = Integer.parseInt(savedY);
                } catch (NumberFormatException e) {
                    // Use defaults
                }
            }
        }

        this.setBounds(initialX, initialY, width, height);

        this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));
        this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));
        this.setShadeWidth(0);

        setupFont();

        dataPanel = new WebPanel();
        dataPanel.setLayout(new VerticalFlowLayout(0, 0));
        dataPanel.setBackground(new Color(20, 20, 20, alpha));

        WebPanel mainPanel = new WebPanel(new BorderLayout());
        mainPanel.setWebColoredBackground(false);
        mainPanel.setBackground(new Color(0, 0, 0, 0));
        mainPanel.add(dataPanel, BorderLayout.CENTER);
        this.add(mainPanel);

        this.getRootPane().setCursor(app.blankCursor);
        this.getContentPane().setCursor(app.blankCursor);
        this.getGlassPane().setCursor(app.blankCursor);

        ui.uiWebLafSetting.setWindowFocus(this);
    }

    public void initPreview(ConfigBridge bridge, String xKey, String yKey, Supplier<List<String>> supplier) {
        isPreview = true;
        init(bridge, xKey, yKey, supplier);
        this.getWebRootPaneUI().setMiddleBg(app.previewColor);
        this.getWebRootPaneUI().setTopBg(app.previewColor);

        addMouseListener(new MouseAdapter() {
            @Override
            public void mousePressed(MouseEvent e) {
                isDragging = 1;
                mouseX = e.getX();
                mouseY = e.getY();
            }

            @Override
            public void mouseReleased(MouseEvent e) {
                isDragging = 0;
            }
        });

        addMouseMotionListener(new MouseMotionAdapter() {
            @Override
            public void mouseDragged(MouseEvent e) {
                if (isDragging == 1) {
                    int left = getLocation().x;
                    int top = getLocation().y;
                    setLocation(left + e.getX() - mouseX, top + e.getY() - mouseY);
                    saveCurrentPosition();
                    setVisible(true);
                    repaint();
                }
            }
        });
        this.setCursor(null);
        setVisible(true);
    }

    protected void setupFont() {
        if (configBridge != null) {
            String mono = configBridge.getConfig("MonoNumFont");
            String flight = configBridge.getConfig("flightInfoFontC");
            if (mono != null && !mono.isEmpty()) {
                fontName = mono;
            } else if (flight != null && !flight.isEmpty()) {
                fontName = flight;
            }
        }

        if (!fontName.isEmpty()) {
            displayFont = new Font(fontName, Font.PLAIN, fontSize);
        } else {
            displayFont = new Font(app.defaultNumfontName, Font.PLAIN, fontSize);
        }
    }

    public void saveCurrentPosition() {
        if (configBridge != null) {
            configBridge.setConfig(configXKey, Integer.toString(this.getLocation().x));
            configBridge.setConfig(configYKey, Integer.toString(this.getLocation().y));
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
