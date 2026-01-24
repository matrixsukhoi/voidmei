package ui.overlay;

import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.event.MouseListener;
import java.awt.event.MouseMotionListener;

import javax.swing.SwingConstants;

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
import prog.config.OverlaySettings;

public class GearAndFlaps extends DraggableOverlay {
    Service xs;
    Controller xc;
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
    private int gap;

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

    //
    // slider.addMouseListener(new MouseAdapter() {
    // public void mouseEntered(MouseEvent e) {
    // /*
    // * if(A.tag==0){ if(f.mode==1){ A.setVisible(false);
    // * A.visibletag=0; } }
    // */
    // }
    //
    // public void mousePressed(MouseEvent e) {
    // isDragging = 1;
    // xx = e.getX();
    // yy = e.getY();
    //
    // }
    //
    // public void mouseReleased(MouseEvent e) {
    // if (isDragging == 1) {
    // isDragging = 0;
    // }
    // /*
    // * if(A.tag==0){ A.setVisible(false); }
    // */
    // }
    // /*
    // * public void mouseReleased(MouseEvent e){ if(A.tag==0){
    // * A.setVisible(true); } }
    // */
    // });
    // slider.addMouseMotionListener(new MouseMotionAdapter() {
    // public void mouseDragged(MouseEvent e) {
    // if (isDragging == 1) {
    // int left = getLocation().x;
    // int top = getLocation().y;
    // setLocation(left + e.getX() - xx, top + e.getY() - yy);
    // setVisible(true);
    // repaint();
    // }
    // }
    // });

    public void initslider(WebSlider slider1) {
        slider1.setMinimum(0);
        slider1.setMaximum(100);
        slider1.setValue(0);
        slider1.setDrawProgress(true);
        slider1.setMinorTickSpacing(25);
        slider1.setMajorTickSpacing(50);
        slider1.setOrientation(SwingConstants.VERTICAL);
        // slider1.t(ComponentOrientation.RIGHT_TO_LEFT);
        slider1.setPaintTicks(true);
        slider1.setPaintLabels(false);
        slider1.setDrawThumb(false);

        slider1.setProgressRound(0);
        slider1.setTrackRound(1);
        // slider1.setSharpThumbAngle(true);
        // slider1.setAngledThumb(true);
        // slider1.setThumbAngleLength(5);
        // slider1.setPreferredHeight(120);
        // slider1.setSnapToTicks(true);
        slider1.setProgressShadeWidth(0);
        slider1.setTrackShadeWidth(0);
        // slider1.setDrawThumb(false);
        slider1.setThumbShadeWidth(0);
        slider1.setThumbBgBottom(transparent);
        slider1.setThumbBgTop(transparent);
        slider1.setTrackBgBottom(transparent);
        slider1.setTrackBgTop(transparent);
        slider1.setProgressBorderColor(transparent);
        slider1.setProgressTrackBgBottom(transParentWhitePlus);
        slider1.setProgressTrackBgTop(transParentWhite);
        slider1.setFocusable(false);
        // 取消slider1响应
        MouseListener[] mls = slider1.getMouseListeners();
        MouseMotionListener[] mmls = slider1.getMouseMotionListeners();
        for (int t = 0; t < mls.length; t++) {
            slider1.removeMouseListener(mls[t]);

        }
        for (int t = 0; t < mmls.length; t++) {
            slider1.removeMouseMotionListener(mmls[t]);
        }

    }

    public WebLabel createWebLabel(String text) {
        WebLabel l1 = new WebLabel(text);
        l1.setShadeColor(Application.colorShade);
        l1.setDrawShade(true);
        l1.setForeground(Application.colorLabel);
        l1.setFont(new Font(Application.defaultFontName, Font.PLAIN, 12));
        return l1;
    }

    public void initpanel() {
        /*
         * WebRadioButton rdbtnNewRadioButton = new
         * WebRadioButton("\u8D77\u843D\u67B6"); rdbtnNewRadioButton.setFont(new
         * Font(Application.DefaultFontName,Font.PLAIN,12));
         * rdbtnNewRadioButton.setForeground(Color.WHITE);
         * rdbtnNewRadioButton.getWebUI().setShadeWidth(5);
         * rdbtnNewRadioButton.setHorizontalAlignment(SwingConstants.CENTER);
         * rdbtnNewRadioButton.setBounds(0, 0, 90, 30);
         */

        s1 = new WebStepLabel("Gear");
        // s1.setFont(new Font(Application.DefaultNumfontName,Font.PLAIN,10));

        s1.setBounds(0, 0, 90, 30);
        s1.setFontSize(10);
        s1.setForeground(Application.colorLabel);
        s1.setDrawShade(true);
        s1.setShadeColor(Application.colorShade);
        s1.setBottomBgColor(new Color(0, 0, 0, 0));
        s1.setTopBgColor(new Color(0, 0, 0, 0));
        s1.setFont(Application.defaultFont);
        // s1.setSelected(true);
        s1.setSelectedBgColor(warning);

        getContentPane().add(s1);

        WebPanel panel = new WebPanel();
        panel.setBounds(0, 30, 90, 120);
        getContentPane().add(panel);
        panel.setLayout(null);
        panel.setWebColoredBackground(false);
        panel.setBackground(new Color(0, 0, 0, 0));

        slider = new WebSlider();
        initslider(slider);
        slider.setBounds(0, 30, 90, 90);
        panel.add(slider);

        WebLabel lblNewLabel = createWebLabel(Lang.gFlaps);
        lblNewLabel.setHorizontalAlignment(SwingConstants.CENTER);
        lblNewLabel.setBounds(0, 0, 90, 30);
        panel.add(lblNewLabel);
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

        int totalWidth = width + (sw * 2);
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
                UIBaseElements.__drawLabelBOSType(g2d, width, dy, 1, Application.defaultFont, Application.defaultFont,
                        Application.defaultFont, flapText, Lang.gFlaps, "%", 9);

                dy += barHeight + gap;
                UIBaseElements.drawVBarTextNum(g2d, 0, dy, barWidth, barHeight, flapPix, 1, Application.colorNum, "",
                        "F" + flapText, fontNum, fontLabel);

                if (warnText != null) {
                    g2d.setColor(warnColor);
                    g2d.setFont(fontLabel);
                    g2d.drawString(warnText, width + gap, fontSize);
                }
            }
        };

        panel.setOpaque(false);
        this.add(panel);

        if (xs != null)
            setVisible(true);
    }

    long gearCheckMili;

    public void drawTick() {
        // if (xs.sState != null) {
        if (xs.sState.gear >= 0) {
            if (xs.sState.gear == 0) {
                // s1.setSelected(false);
                // if (xs.sState.airbrake > 0) {
                // warnText= warnText + " " + Lang.gBrake;
                // warnColor = Application.colorWarning;
                // // Application.debugPrint(xs.sState.airbrake);
                // } else {
                warnText = "";
                warnColor = Application.colorNum;
                // Application.debugPrint(xs.sState.airbrake);
                // s1.setText(language.gBrake);
                // }

            } else {
                if (xs.sState.gear == 100) {
                    // s1.setSelected(true);
                    // s1.setText(Lang.gGear);
                    warnText = Lang.gGear;
                    warnColor = Application.colorNum;
                } else {
                    // s1.setSelected(true);
                    warnText = Lang.gGearDown;
                    // s1.setText(Lang.gGearDown);
                    warnColor = Application.colorWarning;
                }
            }
            if (xs.sState.airbrake > 0) {
                // s1.setSelected(true);
                // s1.setText(Lang.gBrake);
                warnText = warnText + " " + Lang.gBrake;
                warnColor = Application.colorWarning;
            }
        }
        int flps = xs.sState.flaps;
        if (flps >= 0)
            flapPix = flps * barHeight / 100;
        else {
            flapPix = 0;
            flps = 0;
        }

        flapText = String.format("%3d", flps);

        // // Application.debugPrint("gearandFlaps执行了");
        // slider.setValue(xs.sState.flaps);

        root.repaint();
        // }
    }

    @Override
    public void run() {
        // TODO Auto-generated method stub
        while (doit) {
            try {
                Thread.sleep(Application.threadSleepTime);
            } catch (InterruptedException e) {
                // TODO Auto-generated catch block
                e.printStackTrace();
            }
            long t = xs.SystemTime;
            if (t - gearCheckMili > 2 * xc.freqService) {
                gearCheckMili = t;
                drawTick();
            }
        }
    }

}