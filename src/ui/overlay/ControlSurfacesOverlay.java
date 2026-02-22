package ui.overlay;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;

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
import ui.util.OverlayStyleHelper;
import ui.util.SliderHelper;
import prog.config.OverlaySettings;

public class ControlSurfacesOverlay extends DraggableOverlay implements FlightDataListener {

	public ControlSurfacesOverlay() {
		super();
		setTitle("舵面值");
	}

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
		OverlayStyleHelper.applyTransparentStyle(this);
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
		SliderHelper.configureAttitudeSlider(slider1, -100, 100, Application.colorNum);
	}

	public WebLabel createWebLabel(String text) {
		WebLabel l1 = new WebLabel(text);

		l1.setShadeColor(new Color(0, 0, 0));
		l1.setDrawShade(true);
		return l1;
	}

	int rudderValPix;
	private int width;
	private int height;
	private int locateSize;
	private int strokeSize;

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
				int wsVal = xs.isWingSweepValid() ? (int) (wingSweep * 100) : 0;

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