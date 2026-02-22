package ui.overlay;

import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;

import com.alee.extended.label.WebStepLabel;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import prog.Application;
import prog.Controller;
import prog.i18n.Lang;
import prog.Service;
import ui.UIBaseElements;
import ui.base.DraggableOverlay;
import ui.util.SliderHelper;
import prog.config.OverlaySettings;

import prog.event.FlightDataListener;
import prog.event.FlightDataEvent;

public class GearFlapsOverlay extends DraggableOverlay implements FlightDataListener {

    public GearFlapsOverlay() {
        super();
        setTitle("起落襟翼");
    }

    Service xs;
    Controller xc;
    private ui.model.TelemetrySource telemetrySource;
    WebStepLabel s1;
    WebSlider slider;
    Color transParentWhite = Application.colorNum;
    Color transParentWhitePlus = Application.colorNum;
    Color warning = Application.colorWarning;

    Color transparent = new Color(0, 0, 0, 0);
    private Container root;
    // Removed redundant State field
    private WebPanel panel;
    int lx;
    int ly;
    private String FontName;
    private String NumFont;
    private int fontadd;
    private int fontSize;
    private Font fontLabel;
    private Font fontNum;
    private String warnText;
    private Color warnColor;
    private int barWidth;
    private int barHeight;
    private int flapPix;
    private String flapText;
    private int width;
    private int height;

    public void initPreview(Controller xc, OverlaySettings settings) {
        init(xc, null, settings);
        applyPreviewStyle();
        setupDragListeners();
        setVisible(true);
    }

    @Override
    public void saveCurrentPosition() {
        if (overlaySettings != null) {
            overlaySettings.saveWindowPosition(getLocation().x, getLocation().y);
        }
    }

    public void initslider(WebSlider slider1) {
        SliderHelper.configureVerticalProgress(slider1, 0, 100, transParentWhite, transParentWhitePlus);
    }

    public WebLabel createWebLabel(String text) {
        WebLabel l1 = new WebLabel(text);
        l1.setShadeColor(Application.colorShade);
        l1.setDrawShade(true);
        l1.setForeground(Application.colorLabel);
        l1.setFont(new Font(Application.defaultFontName, Font.PLAIN, 12));
        return l1;
    }

    public void reinitConfig() {
        if (overlaySettings != null) {
            FontName = overlaySettings.getFontName();
            NumFont = overlaySettings.getNumFontName();
            fontadd = overlaySettings.getFontSizeAdd();
        } else {
            FontName = Application.defaultFontName;
            NumFont = Application.defaultNumfontName;
            fontadd = 0;
        }

        fontSize = 24 + fontadd;
        fontLabel = new Font(FontName, Font.BOLD, Math.round(fontSize / 2.0f));
        fontNum = new Font(NumFont, Font.BOLD, fontSize);

        barWidth = fontSize >> 1;
        barHeight = 4 * fontSize;

        width = 2 * fontSize;
        height = 5 * fontSize;

        flapPix = barHeight * 50 / 100;
        flapText = String.format("%3d", 50);

        warnText = "";
        warnColor = Application.colorNum;

        int sw = 0;
        if (overlaySettings != null && overlaySettings.getBool("enablegearAndFlapsEdge", false)) {
            sw = 10;
        }

        int totalWidth = width + (int) (4 * fontSize) + (sw * 2);
        int totalHeight = height + (sw * 2);

        if (overlaySettings != null) {
            lx = overlaySettings.getWindowX(totalWidth);
            ly = overlaySettings.getWindowY(totalHeight);
        } else {
            lx = 0;
            ly = 0;
        }

        setShadeWidth(sw);
        setBounds(lx, ly, totalWidth, totalHeight);
    }

    public void init(Controller xc, Service xs, OverlaySettings settings) {
        this.xc = xc;
        this.xs = xs;
        if (xs instanceof ui.model.TelemetrySource) {
            this.telemetrySource = (ui.model.TelemetrySource) xs;
        }
        setOverlaySettings(settings);

        reinitConfig();

        this.setCursor(Application.blankCursor);
        setupTransparentWindow();
        this.setUndecorated(true);
        this.root = this.getContentPane();

        panel = new WebPanel() {
            private static final long serialVersionUID = 1L;

            @Override
            public void paintComponent(Graphics g) {
                Graphics2D g2d = (Graphics2D) g;
                g2d.setPaintMode();
                g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
                g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
                g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
                        RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
                g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);

                int dy = fontSize >> 1;
                // 已经有指示条, 不需要文字了. 暂时注释掉, 不删除.
                // UIBaseElements.__drawLabelBOSType(g2d, width, dy, 1, Application.defaultFont,
                // Application.defaultFont,
                // Application.defaultFont, flapText, Lang.gFlaps, "%", 9);

                dy += barHeight;
                UIBaseElements.drawVBarTextNum(g2d, 0, dy, barWidth, barHeight, flapPix, 1, Application.colorNum, "",
                        "F" + flapText, fontNum, fontLabel);

                if (warnText != null) {
                    g2d.setColor(warnColor);
                    g2d.setFont(fontLabel);
                    g2d.drawString(warnText, width, fontSize);
                }
            }
        };

        panel.setOpaque(false);
        this.add(panel);

        if (xs != null) {
            setVisible(true);
            prog.event.FlightDataBus.getInstance().register(this);
        }
    }

    @Override
    public void onFlightData(prog.event.FlightDataEvent event) {
        javax.swing.SwingUtilities.invokeLater(() -> {
            drawTick();
        });
    }

    @Override
    public void dispose() {
        prog.event.FlightDataBus.getInstance().unregister(this);
        super.dispose();
    }

    long gearCheckMili;

    public void drawTick() {
        if (telemetrySource == null) return;

        // Use TelemetrySource interface for flight data (eliminates Feature Envy)
        int gear = (int) telemetrySource.getGear();
        int flaps = (int) telemetrySource.getFlaps();
        int airbrake = (int) telemetrySource.getAirbrake();

        if (gear >= 0) {
            if (gear == 0) {
                warnText = "";
                warnColor = Application.colorNum;
            } else if (gear == 100) {
                warnText = Lang.gGear;
                warnColor = Application.colorNum;
            } else {
                warnText = Lang.gGearDown;
                warnColor = Application.colorWarning;
            }

            if (airbrake > 0) {
                warnText = warnText + " " + Lang.gBrake;
                warnColor = Application.colorWarning;
            }
        }

        if (flaps >= 0) {
            flapPix = flaps * barHeight / 100;
        } else {
            flapPix = 0;
            flaps = 0;
        }

        flapText = String.format("%3d", flaps);

        root.repaint();
    }

}