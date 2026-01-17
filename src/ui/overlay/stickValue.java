package ui.overlay;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;

import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.awt.event.MouseMotionAdapter;
import java.awt.event.MouseMotionListener;

import javax.swing.SwingConstants;
import javax.swing.text.StyledEditorKit.FontSizeAction;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.slider.WebSlider;

import prog.app;
import prog.controller;
import prog.lang;
import prog.service;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.event.FlightDataListener;
import ui.uiBaseElem;
import ui.uiWebLafSetting;
import java.util.Map;

public class stickValue extends WebFrame implements FlightDataListener {

	/**
	 * 
	 */
	private static final long serialVersionUID = 4231053498040646357L;
	int isDragging;
	int xx;
	int yy;
	WebPanel topPanel;
	controller xc;
	service xs;
	WebSlider slider;
	WebLabel label_1;
	WebLabel label_3;
	WebLabel label_6;
	String NumFont;
	int px;
	int py;
	static Color lblColor = app.colorUnit;
	static Color lblNameColor = app.colorLabel;
	static Color lblNumColor = app.colorNum;

	public volatile boolean doit = true;
	private WebLabel label_8;
	private int fontadd;
	private int fontSize;
	private String sElevator;
	private String sElevatorLabel;
	private String sElevatorUnit;
	private Font fontNum;
	private String FontName;
	private Font fontLabel;
	private Font fontUnit;
	private String sAileron;
	private String sAileronLabel;
	private String sAileronUnit;
	private String sRudder;
	private String sRudderLabel;
	private String sRudderUnit;
	private String sWingSweep;
	private String sWingSweepLabel;
	private String sWingSweepUnit;

	public void setFrameOpaque() {
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		setShadeWidth(0);
	}

	public void initpreview(controller c) {
		init(c, null);

		// setShadeWidth(10);
		// this.setVisible(false);
		this.getWebRootPaneUI().setTopBg(app.previewColor);
		this.getWebRootPaneUI().setMiddleBg(app.previewColor);
		// setFocusableWindowState(true);
		// setFocusable(true);

		addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {
			}

			public void mousePressed(MouseEvent e) {
				isDragging = 1;
				xx = e.getX();
				yy = e.getY();

			}

			public void mouseReleased(MouseEvent e) {
				if (isDragging == 1) {
					isDragging = 0;
				}

			}
		});
		addMouseMotionListener(new MouseMotionAdapter() {
			public void mouseDragged(MouseEvent e) {
				if (isDragging == 1) {
					int left = getLocation().x;
					int top = getLocation().y;
					setLocation(left + e.getX() - xx, top + e.getY() - yy);
					saveCurrentPosition();
					setVisible(true);
					repaint();
				}
			}
		});

		this.setCursor(null);
		setVisible(true);
	}

	public void saveCurrentPosition() {
		xc.setconfig("stickValueX", Integer.toString(getLocation().x));
		xc.setconfig("stickValueY", Integer.toString(getLocation().y));
	}

	public void locater(Graphics2D g2d, int x, int y, int r, int width, int stroke) {

		// 绘制边框
		g2d.setStroke(new BasicStroke(1));
		g2d.setColor(app.colorShadeShape);
		g2d.drawLine(0, 0, 0, r);
		g2d.drawLine(0, 0, r, 0);
		g2d.drawLine(0, r - 1, r - 1, r - 1);
		g2d.drawLine(r - 1, 0, r - 1, r - 1);

		// g2d.setColor(app.lblShadeColorMinor);

		g2d.setStroke(new BasicStroke(stroke));

		// 绘制影子

		g2d.setColor(app.colorShadeShape);
		// 横线
		g2d.drawLine(x - width / 2, y, x + width / 2, y);
		// 竖线
		g2d.drawLine(x, y - width / 2, x, y + width / 2);

		g2d.setColor(app.colorNum);

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
		slider1.setThumbBgBottom(app.colorNum);
		slider1.setThumbBgTop(app.colorNum);
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
	int rudder;
	private int width;
	private int height;
	// public void initpanel(WebPanel toppanel) {
	// toppanel.setLayout(null);
	// px = 100;
	// py = 100;
	//
	// WebPanel panel = new WebPanel() {
	// private static final long serialVersionUID = -9061280572815010060L;
	//
	// public void paintComponent(Graphics g) {
	// Graphics2D g2d = (Graphics2D) g;
	// // 开始绘图
	// // g2d.draw
	// g2d.setPaintMode();
	// g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING,
	// RenderingHints.VALUE_ANTIALIAS_ON);
	// g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING,
	// RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
	// g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
	// RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
	// // g2d.setColor(Color.white);
	// // g2d.fillRect(0, 0, 200, 200);
	// // 绘制十字星
	// locater(g2d, px, py, 6);
	//
	// int dy = fontSize >> 1;
	// uiBaseElem.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel,
	// fontUnit, sElevator, sElevatorLabel, sElevatorUnit,9);
	// dy+=1.5 * fontSize;
	// uiBaseElem.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel,
	// fontUnit, sAileron, sAileronLabel, sAileronUnit,9);
	// dy+=1.5 * fontSize;
	// uiBaseElem.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel,
	// fontUnit, sRudder, sRudderLabel, sRudderUnit,9);
	// dy+=1.5 * fontSize;
	// uiBaseElem.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel,
	// fontUnit, sWingSweep, sWingSweepLabel, sWingSweepUnit,9);
	//
	// // 绘制横条
	//// uiBaseElem.drawHBarTextNum(g2d, x, y, width, height, val_width,
	// borderwidth, c, lbl, num, lblFont, numFont);
	// uiBaseElem.drawHBarTextNum(g2d, 0, height, width, fontSize >> 1,
	// rudderValPix, 1, app.lblNumColor, sRudderLabel, sRudder, fontLabel,
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
	// lblX.setFont(new Font(app.DefaultFontName, Font.BOLD, 12));
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
	// lblY.setFont(new Font(app.DefaultFontName, Font.BOLD, 12));
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
	// lblW.setFont(new Font(app.DefaultFontName, Font.BOLD, 12));
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
	// lblZ.setFont(new Font(app.DefaultFontName, Font.BOLD, 12));
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

	int lx;
	int ly;

	public void reinitConfig() {
		if (xc.getconfig("GlobalNumFont") != "")
			NumFont = xc.getconfig("GlobalNumFont");
		else
			NumFont = app.defaultNumfontName;

		if (xc.getconfig("stickValueX") != "")
			lx = Integer.parseInt(xc.getconfig("stickValueX"));
		else
			lx = 0;
		if (xc.getconfig("stickValueY") != "")
			ly = Integer.parseInt(xc.getconfig("stickValueY"));
		else
			ly = 0;
		if (xc.getconfig("flightInfoFontC") != "")
			FontName = xc.getconfig("flightInfoFontC");
		else
			FontName = app.defaultFont.getFontName();
		if (xc.getconfig("flightInfoFontaddC") != "")
			fontadd = Integer.parseInt(xc.getconfig("flightInfoFontaddC"));
		else
			fontadd = 0;

		fontSize = 24 + fontadd;
		fontNum = new Font(NumFont, Font.BOLD, fontSize);
		fontLabel = new Font(FontName, Font.BOLD, Math.round(fontSize / 2.0f));
		fontUnit = new Font(NumFont, Font.PLAIN, Math.round(fontSize / 2.0f));

		width = fontSize * 6;

		rudderValPix = (50 + 100) * width / 200;
		height = width;

		int twidth = (int) (width + 4 * fontSize);
		int theight = (int) (height + 1.5 * fontSize);

		this.setBounds(lx, ly, twidth, theight);
		px = width / 2;
		py = width / 2;

		locateSize = width / 30;
		if (xc.getconfig("enableAxisEdge").equals("true"))
			setShadeWidth(10);
		else
			setShadeWidth(0);

		this.setBounds(lx, ly, twidth, theight);

		repaint();
	}

	public void init(controller c, service s) {
		xc = c;
		xs = s;

		reinitConfig();

		sElevator = "50";
		sElevatorLabel = lang.vElevator;
		sElevatorUnit = "%";

		sAileron = "50";
		sAileronLabel = lang.vAileron;
		sAileronUnit = "%";

		sRudder = "50";
		sRudderLabel = lang.vRudder;
		sRudderUnit = "%";

		sWingSweep = "50";
		sWingSweepLabel = lang.vVarioW;
		sWingSweepUnit = "%";

		setFrameOpaque();

		int twidth = (int) (width + 4 * fontSize);
		int theight = (int) (height + 1.5 * fontSize);

		this.setBounds(lx, ly, twidth, theight);

		px = width / 2;
		py = width / 2;

		locateSize = width / 30;
		strokeSize = width / 60;

		topPanel = new WebPanel() {
			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw
				g2d.setPaintMode();

				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, app.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, app.textAASetting);
				// g2d.setRenderingHint(RenderingHints.KEY_RENDERING,
				// RenderingHints.VALUE_RENDER_QUALITY);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);

				// g2d.setColor(Color.white);
				// g2d.fillRect(0, 0, 200, 200);
				// 绘制十字星
				locater(g2d, px, py, width, locateSize, strokeSize);

				int dy = fontSize >> 1;
				uiBaseElem.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel, fontUnit, sElevator,
						sElevatorLabel, sElevatorUnit, 9);
				dy += 1.5 * fontSize;
				uiBaseElem.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel, fontUnit, sAileron, sAileronLabel,
						sAileronUnit, 9);
				dy += 1.5 * fontSize;
				uiBaseElem.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel, fontUnit, sRudder, sRudderLabel,
						sRudderUnit, 9);
				dy += 1.5 * fontSize;
				uiBaseElem.__drawLabelBOSType(g2d, width, dy, 1, fontNum, fontLabel, fontUnit, sWingSweep,
						sWingSweepLabel, sWingSweepUnit, 9);

				// 绘制横条
				// uiBaseElem.drawHBarTextNum(g2d, x, y, width, height, val_width, borderwidth,
				// c, lbl, num, lblFont, numFont);
				uiBaseElem.drawHBarTextNum(g2d, 0, height, width, fontSize >> 1, rudderValPix, 1, app.colorNum,
						sRudderLabel, sRudder, fontLabel, fontLabel);

			}
		};
		// topPanel.setWebColoredBackground(false);
		// topPanel.setBackground(new Color(0, 0, 0, 0));

		// initpanel(topPanel);
		add(topPanel);
		setTitle("stickValue");
		uiWebLafSetting.setWindowOpaque(this);

		if (xc.getconfig("enableAxisEdge").equals("true"))
			setShadeWidth(10);

		if (s != null) {
			setVisible(true);
			FlightDataBus.getInstance().register(this);
		}
	}

	@Override
	public void dispose() {
		FlightDataBus.getInstance().unregister(this);
		super.dispose();
	}

	@Override
	public void onFlightData(FlightDataEvent event) {
		javax.swing.SwingUtilities.invokeLater(() -> {
			Map<String, String> data = event.getData();
			// Parse values from event data
			try {
				int aileronVal = data.containsKey("aileron") ? Integer.parseInt(data.get("aileron")) : 0;
				int elevatorVal = data.containsKey("elevator") ? Integer.parseInt(data.get("elevator")) : 0;
				int rudderVal = data.containsKey("rudder") ? Integer.parseInt(data.get("rudder")) : 0;

				px = (100 + aileronVal) * width / 200;
				py = (100 + elevatorVal) * width / 200;

				sElevator = data.getOrDefault("elevator", "0");
				sAileron = data.getOrDefault("aileron", "0");
				sRudder = data.getOrDefault("rudder", "0");
				sWingSweep = data.getOrDefault("ws", "0");

				rudderValPix = (rudderVal + 100) * width / 200;

				this.getContentPane().repaint();
			} catch (NumberFormatException e) {
				// Ignore parsing errors
			}
		});
	}

}