package ui.overlay;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;

import java.awt.event.MouseListener;
import java.awt.event.MouseMotionListener;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import prog.Application;
import prog.Controller;
import prog.i18n.Lang;
import prog.Service;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.event.FlightDataListener;
import ui.UIBaseElements;
import ui.WebLafSettings;
import ui.base.DraggableOverlay;
import prog.config.OverlaySettings;

public class StickValue extends DraggableOverlay implements FlightDataListener {

	/**
	 * 
	 */
	WebLabel label_3;
	WebLabel label_6;
	Controller xc;
	Service xs;
	WebPanel topPanel;
	int lx;
	int ly;
	String NumFont;
	int px;
	int py;
	static private int fontadd;
	private int fontSize;
	private String sElevatorLabel;
	private String sElevatorUnit;
	private Font fontNum;
	private String FontName;
	private Font fontLabel;
	private Font fontUnit;
	// Zero-GC Buffers
	private final char[] bufElevator = new char[8];
	private final char[] bufAileron = new char[8];
	private final char[] bufRudder = new char[8];
	private final char[] bufWingSweep = new char[8];

	private int lenElevator = 0;
	private int lenAileron = 0;
	private int lenRudder = 0;
	private int lenWingSweep = 0;

	private String sAileronLabel;
	private String sAileronUnit;
	private String sRudderLabel;
	private String sRudderUnit;
	private String sWingSweepLabel;
	private String sWingSweepUnit;

	public void setFrameOpaque() {
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		setShadeWidth(0);
	}

	public void init(Controller c, Service s, OverlaySettings settings) {
		this.xc = c;
		this.xs = s;
		setOverlaySettings(settings);

		this.setUndecorated(true);
		reinitConfig();

		this.setCursor(Application.blankCursor);
		setupTransparentWindow();

		// Initial Values (50)
		lenElevator = ui.util.FastNumberFormatter.format(50, bufElevator, 0);
		lenAileron = ui.util.FastNumberFormatter.format(50, bufAileron, 0);
		lenRudder = ui.util.FastNumberFormatter.format(50, bufRudder, 0);
		lenWingSweep = ui.util.FastNumberFormatter.format(50, bufWingSweep, 0);

		sElevatorLabel = Lang.vElevator;
		sElevatorUnit = "%";
		sAileronLabel = Lang.vAileron;
		sAileronUnit = "%";
		sRudderLabel = Lang.vRudder;
		sRudderUnit = "%";
		sWingSweepLabel = Lang.vVarioW;
		sWingSweepUnit = "%";

		setFrameOpaque();

		px = width / 2;
		py = width / 2;

		locateSize = width / 30;
		strokeSize = width / 60;

		topPanel = new WebPanel() {
			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				g2d.setPaintMode();

				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);

				locater(g2d, px, py, width, locateSize, strokeSize);

				int dy = fontSize >> 1;
				UIBaseElements.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel, fontUnit, bufElevator,
						lenElevator,
						sElevatorLabel, sElevatorUnit, 9);
				dy += 1.5 * fontSize;
				UIBaseElements.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel, fontUnit, bufAileron,
						lenAileron,
						sAileronLabel,
						sAileronUnit, 9);
				dy += 1.5 * fontSize;
				UIBaseElements.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel, fontUnit, bufRudder, lenRudder,
						sRudderLabel,
						sRudderUnit, 9);
				dy += 1.5 * fontSize;
				UIBaseElements.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel, fontUnit, bufWingSweep,
						lenWingSweep,
						sWingSweepLabel, sWingSweepUnit, 9);

				UIBaseElements.drawHBarTextNum(g2d, 0, height, width, fontSize >> 1, rudderValPix, 1,
						Application.colorNum,
						sRudderLabel, bufRudder, lenRudder, fontLabel, fontLabel);
			}
		};

		add(topPanel);
		setTitle("StickValue");
		WebLafSettings.setWindowOpaque(this);

		if (s != null) {
			setVisible(true);
			FlightDataBus.getInstance().register(this);
		}
	}

	public void initPreview(Controller c, OverlaySettings settings) {
		init(c, null, settings);
		applyPreviewStyle();
		setupDragListeners();
		this.setCursor(null);
		setVisible(true);
	}

	@Override
	public void saveCurrentPosition() {
		if (overlaySettings != null) {
			overlaySettings.saveWindowPosition(getLocation().x, getLocation().y);
		}
	}

	public void locater(Graphics2D g2d, int x, int y, int r, int width, int stroke) {

		// 绘制边框
		g2d.setStroke(new BasicStroke(1));
		g2d.setColor(Application.colorShadeShape);
		g2d.drawLine(0, 0, 0, r);
		g2d.drawLine(0, 0, r, 0);
		g2d.drawLine(0, r - 1, r - 1, r - 1);
		g2d.drawLine(r - 1, 0, r - 1, r - 1);

		// g2d.setColor(Application.lblShadeColorMinor);

		g2d.setStroke(new BasicStroke(stroke));

		// 绘制影子

		g2d.setColor(Application.colorShadeShape);
		// 横线
		g2d.drawLine(x - width / 2, y, x + width / 2, y);
		// 竖线
		g2d.drawLine(x, y - width / 2, x, y + width / 2);

		g2d.setColor(Application.colorNum);

		// 横线
		g2d.drawLine(x - width / 2 - 1, y - 1, x + width / 2 - 1, y - 1);
		// 竖线
		g2d.drawLine(x - 1, y - width / 2 - 1, x - 1, y + width / 2 - 1);
	}

	public void initslider(WebSlider slider1) {
		slider1.setMinimum(-100);
		slider1.setMaximum(100);
		slider1.setValue(0);
		slider1.setDrawProgress(true);
		slider1.setMinorTickSpacing(25);
		slider1.setMajorTickSpacing(50);
		slider1.setPaintTicks(true);
		slider1.setPaintLabels(true);
		// slider1.setPreferredHeight(120);
		// slider1.setSnapToTicks(true);
		slider1.setProgressShadeWidth(0);
		slider1.setTrackShadeWidth(1);
		// slider1.setDrawThumb(false);
		slider1.setThumbShadeWidth(2);
		slider1.setThumbBgBottom(Application.colorNum);
		slider1.setThumbBgTop(Application.colorNum);
		slider1.setTrackBgBottom(new Color(0, 0, 0, 0));
		slider1.setTrackBgTop(new Color(0, 0, 0, 0));
		slider1.setProgressBorderColor(new Color(0, 0, 0, 0));
		slider1.setProgressTrackBgBottom(new Color(0, 0, 0, 0));
		slider1.setProgressTrackBgTop(new Color(0, 0, 0, 0));
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

		l1.setShadeColor(new Color(0, 0, 0));
		l1.setDrawShade(true);
		return l1;
	}

	int rudderValPix;
	// int rudder; // Removed, using buffers
	private int width;
	private int height;
	// borderwidth, c, lbl, num, lblFont, numFont);
	// UIBaseElements.drawHBarTextNum(g2d, 0, height, width, fontSize >> 1,
	// rudderValPix, 1, Application.lblNumColor, sRudderLabel, sRudder, fontLabel,
	// fontLabel);
	//
	// }
	// };
	private int locateSize;
	private int strokeSize;

	// panel.setBackground(new Color(0,0,0,0));
	// panel.setBounds(0, 0, 350, 264);
	// toppanel.add(panel);

	// slider = new WebSlider();
	// initslider(slider);
	//
	// slider.setBounds(0, 200, 200, 64);
	// toppanel.add(slider);
	//
	// WebPanel panel_1 = new WebPanel();
	// panel_1.setLayout(null);
	// panel_1.setBackground(new Color(0, 0, 0, 0));
	// panel_1.setBounds(200, 0, 128, 64);
	// toppanel.add(panel_1);
	//
	// WebLabel lblX = createWebLabel(language.vAileron);
	// lblX.setVerticalAlignment(SwingConstants.BOTTOM);
	// lblX.setHorizontalAlignment(SwingConstants.LEFT);
	// lblX.setForeground(lblNameColor);
	// lblX.setFont(new Font(Application.DefaultFontName, Font.BOLD, 12));
	// lblX.setBounds(92, 0, 36, 31);
	// panel_1.add(lblX);
	//
	// label_1 = createWebLabel("-50");
	// label_1.setHorizontalAlignment(SwingConstants.RIGHT);
	// label_1.setForeground(lblNumColor);
	// label_1.setFont(new Font(NumFont, Font.BOLD, 28));
	// label_1.setBounds(0, 0, 64, 64);
	// panel_1.add(label_1);
	//
	// WebLabel label_2 = createWebLabel("%");
	// label_2.setVerticalAlignment(SwingConstants.TOP);
	// label_2.setHorizontalAlignment(SwingConstants.LEFT);
	// label_2.setForeground(lblColor);
	// label_2.setFont(new Font(NumFont, Font.PLAIN, 14));
	// label_2.setBounds(92, 33, 36, 31);
	// panel_1.add(label_2);
	//
	// WebPanel panel_2 = new WebPanel();
	// panel_2.setLayout(null);
	// panel_2.setBackground(new Color(0, 0, 0, 0));
	// panel_2.setBounds(200, 64, 128, 64);
	// toppanel.add(panel_2);
	//
	// WebLabel lblY = createWebLabel(language.vElevator);
	// lblY.setVerticalAlignment(SwingConstants.BOTTOM);
	// lblY.setHorizontalAlignment(SwingConstants.LEFT);
	// lblY.setForeground(lblNameColor);
	// lblY.setFont(new Font(Application.DefaultFontName, Font.BOLD, 12));
	// lblY.setBounds(92, 0, 36, 31);
	// panel_2.add(lblY);
	//
	// label_3 = createWebLabel("16");
	// label_3.setHorizontalAlignment(SwingConstants.RIGHT);
	// label_3.setForeground(lblNumColor);
	// label_3.setFont(new Font(NumFont, Font.BOLD, 28));
	// label_3.setBounds(0, 0, 64, 64);
	// panel_2.add(label_3);
	//
	// WebLabel label_4 = createWebLabel("%");
	// label_4.setVerticalAlignment(SwingConstants.TOP);
	// label_4.setHorizontalAlignment(SwingConstants.LEFT);
	// label_4.setForeground(lblColor);
	// label_4.setFont(new Font(NumFont, Font.PLAIN, 14));
	// label_4.setBounds(92, 33, 36, 31);
	// panel_2.add(label_4);
	//
	// // 后掠角
	// WebPanel panel_4 = new WebPanel();
	// panel_4.setLayout(null);
	// panel_4.setBackground(new Color(0, 0, 0, 0));
	// panel_4.setBounds(200, 128, 128, 64);
	// toppanel.add(panel_4);
	//
	// WebLabel lblW = createWebLabel(language.vVarioW);
	// lblW.setVerticalAlignment(SwingConstants.BOTTOM);
	// lblW.setHorizontalAlignment(SwingConstants.LEFT);
	// lblW.setForeground(lblNameColor);
	// lblW.setFont(new Font(Application.DefaultFontName, Font.BOLD, 12));
	// lblW.setBounds(92, 0, 36, 31);
	// panel_4.add(lblW);
	//
	// label_8 = createWebLabel("0");
	// label_8.setHorizontalAlignment(SwingConstants.RIGHT);
	// label_8.setForeground(lblNumColor);
	// label_8.setFont(new Font(NumFont, Font.BOLD, 28));
	// label_8.setBounds(0, 0, 64, 64);
	// panel_4.add(label_8);
	//
	// WebLabel label_9 = createWebLabel("%");
	// label_9.setVerticalAlignment(SwingConstants.TOP);
	// label_9.setHorizontalAlignment(SwingConstants.LEFT);
	// label_9.setForeground(lblColor);
	// label_9.setFont(new Font(NumFont, Font.PLAIN, 14));
	// label_9.setBounds(92, 33, 36, 31);
	// panel_4.add(label_9);
	//
	// WebPanel panel_3 = new WebPanel();
	// panel_3.setLayout(null);
	// panel_3.setBackground(new Color(0, 0, 0, 0));
	// panel_3.setBounds(200, 192, 128, 64);
	// toppanel.add(panel_3);
	//
	// WebLabel lblZ = createWebLabel(language.vRudder);
	// lblZ.setVerticalAlignment(SwingConstants.BOTTOM);
	// lblZ.setHorizontalAlignment(SwingConstants.LEFT);
	// lblZ.setForeground(lblNameColor);
	// lblZ.setFont(new Font(Application.DefaultFontName, Font.BOLD, 12));
	// lblZ.setBounds(92, 0, 36, 31);
	// panel_3.add(lblZ);
	//
	// label_6 = createWebLabel("0");
	// label_6.setHorizontalAlignment(SwingConstants.RIGHT);
	// label_6.setForeground(lblNumColor);
	// label_6.setFont(new Font(NumFont, Font.BOLD, 28));
	// label_6.setBounds(0, 0, 64, 64);
	// panel_3.add(label_6);
	//
	// WebLabel label_7 = createWebLabel("%");
	// label_7.setVerticalAlignment(SwingConstants.TOP);
	// label_7.setHorizontalAlignment(SwingConstants.LEFT);
	// label_7.setForeground(lblColor);
	// label_7.setFont(new Font(NumFont, Font.PLAIN, 14));
	// label_7.setBounds(92, 33, 36, 31);
	// panel_3.add(label_7);
	// }

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
		fontNum = new Font(NumFont, Font.BOLD, fontSize);
		fontLabel = new Font(FontName, Font.BOLD, Math.round(fontSize / 2.0f));
		fontUnit = new Font(NumFont, Font.PLAIN, Math.round(fontSize / 2.0f));

		width = fontSize * 6;
		height = width;
		rudderValPix = (50 + 100) * width / 200;

		int twidth = (int) (width + 4 * fontSize);
		int theight = (int) (height + 1.5 * fontSize);

		int sw = 0;
		if (overlaySettings != null && overlaySettings.getBool("enableAxisEdge", false)) {
			sw = 10;
		}

		int totalWidth = twidth + (sw * 2);
		int totalHeight = theight + (sw * 2);

		if (overlaySettings != null) {
			lx = overlaySettings.getWindowX(totalWidth);
			ly = overlaySettings.getWindowY(totalHeight);
		}
		px = width / 2;
		py = width / 2;

		locateSize = width / 30;
		setShadeWidth(sw);

		this.setBounds(lx, ly, totalWidth, totalHeight);

		repaint();
	}

	@Override
	public void dispose() {
		FlightDataBus.getInstance().unregister(this);
		super.dispose();
	}

	@Override
	public void onFlightData(FlightDataEvent event) {
		javax.swing.SwingUtilities.invokeLater(() -> {
			if (xs != null) {
				// Zero-GC Update
				double aileron = xs.getAileron();
				double elevator = xs.getElevator();
				double rudder = xs.getRudder();
				double wingSweep = xs.getWingSweep();

				int aileronVal = (int) aileron;
				int elevatorVal = (int) elevator;
				int rudderVal = (int) rudder;
				int wsVal = (int) (wingSweep * 100);

				px = (100 + aileronVal) * width / 200;
				py = (100 + elevatorVal) * width / 200;
				rudderValPix = (rudderVal + 100) * width / 200;

				lenAileron = ui.util.FastNumberFormatter.format(aileronVal, bufAileron, 0);
				lenElevator = ui.util.FastNumberFormatter.format(elevatorVal, bufElevator, 0);
				lenRudder = ui.util.FastNumberFormatter.format(rudderVal, bufRudder, 0);
				lenWingSweep = ui.util.FastNumberFormatter.format(wsVal, bufWingSweep, 0);
			}
			this.getContentPane().repaint();
		});
	}

}