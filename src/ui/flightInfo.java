package ui;

import java.awt.Color;
import java.awt.Component;
import java.awt.Container;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.Shape;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.awt.event.MouseMotionAdapter;
import java.awt.event.MouseMotionListener;
import java.awt.font.FontRenderContext;
import java.awt.font.GlyphVector;
import java.awt.geom.AffineTransform;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import prog.app;
import prog.controller;
import prog.lang;
import prog.service;
import ui.model.FlightField;

public class flightInfo extends WebFrame implements Runnable {
	/**
	 * 
	 */
	public volatile boolean doit = true;
	String FontName;
	int fontadd;
	controller xc;
	service xs;
	WebLabel label_1;
	WebLabel label_3;
	WebLabel label_9;
	WebLabel label_10;
	WebLabel label_12;
	WebLabel label_15;
	WebLabel label_17;
	WebLabel label_19;
	WebLabel label_13;
	WebLabel label_21;
	WebLabel label_6;
	WebLabel label_28;
	WebLabel lblGload;
	WebLabel label_25;
	WebLabel label_30;
	String NumFont;

	Color lblColor = app.colorUnit;
	Color lblNameColor = app.colorLabel;
	Color lblNumColor = app.colorNum;
	Color lblShadeColor = app.colorShade;

	// AffineTransform tx = new AffineTransform();
	//
	// // public BasiStroke roundstroke = new BasicStroke(1,
	// BasicStroke.CAP_ROUND,
	// // BasicStroke.JOIN_ROUND);
	// // 绘制库
	// public void __drawStringShade(Graphics2D g2d, int x, int y, int
	// shadeWidth, String s, Font f, Color c) {
	// g2d.setFont(f);
	// g2d.setStroke(new BasicStroke(shadeWidth, BasicStroke.CAP_ROUND,
	// BasicStroke.JOIN_ROUND));
	//
	// // drawshade
	// FontRenderContext frc = g2d.getFontRenderContext();
	// GlyphVector glyphVector = f.createGlyphVector(frc, s);
	// // get the shape object
	// Shape textShape = glyphVector.getOutline();
	//
	// tx = new AffineTransform();
	// tx.translate(x, y);
	// Shape newShape = tx.createTransformedShape(textShape);
	//
	// g2d.setColor(app.lblShadeColorMinor);
	// g2d.draw(newShape);
	//
	// g2d.setColor(c);
	// g2d.drawString(s, x, y);
	//
	// }
	//
	// public void drawStringShade(Graphics2D g2d, int x, int y, int shadeWidth,
	// String s, Font f) {
	// __drawStringShade(g2d, x, y, shadeWidth, s, f, app.lblNumColor);
	// }
	//
	// // 横向还是竖向
	// public void drawVRect(Graphics2D g2d, int x, int y, int width, int
	// height, int borderwidth, Color c) {
	// g2d.setStroke(new BasicStroke(borderwidth, BasicStroke.CAP_ROUND,
	// BasicStroke.JOIN_ROUND));
	// // 外边框
	// g2d.setColor(app.lblShadeColorMinor);
	//
	// if (height >= 0) {
	// g2d.drawRect(x, y - height, width - 1, height - 1);
	// g2d.setColor(c);
	// g2d.fillRect(x + borderwidth, y + borderwidth - height, width - 2 *
	// borderwidth, height - 2 * borderwidth);
	// } else {
	// g2d.drawRect(x, y, width - 1, -height - 1);
	// g2d.setColor(c);
	// g2d.fillRect(x + borderwidth, y + borderwidth, width - 2 * borderwidth,
	// -height - 2 * borderwidth);
	// }
	// // 内部条
	// }
	//
	// public void drawHRect(Graphics2D g2d, int x, int y, int width, int
	// height, int borderwidth, Color c) {
	// g2d.setStroke(new BasicStroke(borderwidth, BasicStroke.CAP_ROUND,
	// BasicStroke.JOIN_ROUND));
	// // 外边框
	// g2d.setColor(app.lblShadeColorMinor);
	//
	// if (width >= 0) {
	// g2d.drawRect(x, y, width - 1, height - 1);
	// g2d.setColor(c);
	// g2d.fillRect(x + borderwidth, y + borderwidth, width - 2 * borderwidth,
	// height - 2 * borderwidth);
	// } else {
	// g2d.drawRect(x - width, y, -width - 1, height - 1);
	// g2d.setColor(c);
	// g2d.fillRect(x + borderwidth - width, y + borderwidth, -width - 2 *
	// borderwidth, height - 2 * borderwidth);
	// }
	// // 内部条
	// }
	//
	// // BOS 类型的标签
	// public void drawLabelBOSType(Graphics2D g2d, int x_offset, int y_offset,
	// int shadeWidth, Font num, Font label,
	// Font unit, String sNum, String sLabel, String sUnit) {
	//
	// // 数字
	// // y偏移式加下底边再减去自己字体大小的一半
	// __drawStringShade(g2d, x_offset, (y_offset + y_offset + label.getSize() +
	// unit.getSize()) >> 1, shadeWidth,
	// sNum, num, app.lblNumColor);
	//
	// // 标签名
	// __drawStringShade(g2d, x_offset + 3 * num.getSize(), y_offset,
	// shadeWidth, sLabel, label, app.lblNameColor);
	// // 单位名
	// __drawStringShade(g2d, x_offset + 3 * num.getSize(), y_offset +
	// label.getSize(), shadeWidth, sUnit, unit,
	// app.lblColor);
	// }

	private static final long serialVersionUID = 6759127498151892589L;
	int isDragging;
	int xx;
	int yy;
	Color Red = lblShadeColor;
	Color White = new Color(255, 255, 255, 255);
	private WebPanel panel;
	private Font fontNum;
	private Font fontLabel;
	private Font fontUnit;
	private int fontsize;
	private int columnNum;
	private int[] doffset;

	public void webLabelRemoveML(WebLabel label) {
		// label.get
		Component[] tmp = label.getComponents();
		for (int i = 0; i < tmp.length; i++) {
			// app.debugPrint("remove compnents" + tmp[i] +
			// "mouseListener");
			MouseListener[] mls = tmp[i].getMouseListeners();
			MouseMotionListener[] mmls = tmp[i].getMouseMotionListeners();
			for (int j = 0; j < mls.length; j++) {
				tmp[i].removeMouseListener(mls[j]);
			}
			for (int j = 0; j < mmls.length; j++) {
				tmp[i].removeMouseMotionListener(mmls[j]);
			}
		}
	}

	public void webPanelRemoveML(WebPanel label) {
		// label.get
		// label.getMouseMotionListeners()
		// Component[] tmp = label.getComponents();

		MouseListener[] mls = label.getMouseListeners();
		MouseMotionListener[] mmls = label.getMouseMotionListeners();
		for (int j = 0; j < mls.length; j++) {
			// app.debugPrint("remove ml" + mls[j]);
			label.removeMouseListener(mls[j]);
		}
		for (int j = 0; j < mmls.length; j++) {
			// app.debugPrint("remove mls" + mmls[j]);
			label.removeMouseMotionListener(mmls[j]);
		}

		Component[] tmp = label.getComponents();
		for (int i = 0; i < tmp.length; i++) {
			// app.debugPrint("remove compnents" + tmp[i] +
			// "mouseListener");
			mls = tmp[i].getMouseListeners();
			mmls = tmp[i].getMouseMotionListeners();
			for (int j = 0; j < mls.length; j++) {
				tmp[i].removeMouseListener(mls[j]);
			}
			for (int j = 0; j < mmls.length; j++) {
				tmp[i].removeMouseMotionListener(mmls[j]);
			}
		}
	}

	public void initPreview(controller c) {
		init(c, null);
		// app.debugPrint("初始化");
		// setShadeWidth(10);
		// this.setVisible(false);
		// this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 50));
		this.getWebRootPaneUI().setMiddleBg(app.previewColor);// 中部透明
		this.getWebRootPaneUI().setTopBg(app.previewColor);// 顶部透明

		addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {
				/*
				 * if(A.tag==0){ if(f.mode==1){ A.setVisible(false);
				 * A.visibletag=0; } }
				 */
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
				/*
				 * if(A.tag==0){ A.setVisible(false); }
				 */
			}
			/*
			 * public void mouseReleased(MouseEvent e){ if(A.tag==0){
			 * A.setVisible(true); } }
			 */
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
		xc.setconfig("flightInfoX", Integer.toString(getLocation().x));
		xc.setconfig("flightInfoY", Integer.toString(getLocation().y));
	}
	// setFocusableWindowState(true);
	// setFocusable(true);
	// } // This closing brace was for initPreview, now it's moved up.

	// Data model: List of flight fields (replaces String[][] totalString)
	List<FlightField> fields = new ArrayList<>();
	Map<String, FlightField> fieldMap = new HashMap<>();

	private int numHeight;
	private int labelHeight;
	private Container root;

	/**
	 * Helper to add a field and register it in the map.
	 */
	private void addField(String key, String label, String unit, String configKey, boolean hideWhenNA) {
		String tmp = xc.getconfig(configKey);
		// If the field is disabled in config, don't add it
		if (tmp != null && !tmp.isEmpty() && Boolean.parseBoolean(tmp)) {
			return;
		}
		FlightField field = new FlightField(key, label, unit, configKey, hideWhenNA);
		field.setValue("---"); // Default placeholder
		fields.add(field);
		fieldMap.put(key, field);
	}

	public void initTextString() {
		fields.clear();
		fieldMap.clear();

		// Add all flight info fields using the helper method
		// Args: key, label, unit, configKey, hideWhenNA
		addField("ias", lang.fIAS, "Km/h", "disableFlightInfoIAS", false);
		addField("tas", lang.fTAS, "Km/h", "disableFlightInfoTAS", false);
		addField("mach", lang.fMach, "Mach", "disableFlightInfoMach", false);
		addField("dir", lang.fCompass, "Deg", "disableFlightInfoCompass", false);
		addField("height", lang.fAlt, "M", "disableFlightInfoHeight", false);
		addField("rda", lang.fRa, "M", "disableFlightInfoRadioAlt", true); // hideWhenNA
		addField("vario", lang.fVario, "M/s", "disableFlightInfoVario", false);
		addField("sep", lang.fSEP, "M/s", "disableFlightInfoSEP", false);
		addField("acc", lang.fAcc, "M/s^2", "disableFlightInfoAcc", false);
		addField("wx", lang.fWx, "Deg/s", "disableFlightInfoWx", false);
		addField("ny", lang.fGL, "G", "disableFlightInfoNy", false);
		addField("turn", lang.fTRr, "Deg/s", "disableFlightInfoTurn", false);
		addField("rds", lang.fTR, "M", "disableFlightInfoTurnRadius", false);
		addField("aoa", lang.fAoA, "Deg", "disableFlightInfoAoA", false);
		addField("aos", lang.fAoS, "Deg", "disableFlightInfoAoS", false);
		addField("ws", lang.fWs, "%", "disableFlightInfoWingSweep", true); // hideWhenNA
	}

	/**
	 * Helper to update a field value by key.
	 */
	private void updateField(String key, String value) {
		FlightField field = fieldMap.get(key);
		if (field != null) {
			field.setValue(value);
			// Handle hideWhenNA logic
			if (field.hideWhenNA) {
				field.visible = !value.equals(service.nastring);
			}
		}
	}

	public void updateString() {
		if (xs == null)
			return;

		// Update all field values from the service
		updateField("ias", xs.IAS);
		updateField("tas", xs.TAS);
		updateField("mach", xs.M);
		updateField("dir", xs.compass);
		updateField("height", xs.salt);
		updateField("rda", xs.sRadioAlt);
		updateField("vario", xs.Vy);
		updateField("sep", xs.sSEP);
		updateField("acc", xs.sAcc);
		updateField("wx", xs.Wx);
		updateField("ny", xs.sN); // 使用修正过的过载
		updateField("turn", xs.sTurnRate);
		updateField("rds", xs.sTurnRds);
		updateField("aoa", xs.AoA);
		updateField("aos", xs.AoS);
		updateField("ws", xs.sWingSweep);
	}

	public void initpanel() {
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
	}

	private void updateDxDy(int num, int[] doffset) {
		// TODO Auto-generated method stub
		if (num % columnNum == 0) {
			doffset[1] += Math.round(1 * numHeight);
			// doffset[0] = 0;;
			doffset[0] = fontsize >> 1;
		} else {
			doffset[0] += 5 * fontsize;
		}

	}

	int lx;
	int ly;

	public void reinitConfig() {
		if (xc.getconfig("GlobalNumFont") != "")
			NumFont = xc.getconfig("GlobalNumFont");
		else
			NumFont = app.defaultNumfontName;

		if (xc.getconfig("flightInfoFontC") != "")
			FontName = xc.getconfig("flightInfoFontC");
		else
			FontName = app.defaultFont.getFontName();
		if (xc.getconfig("flightInfoFontaddC") != "")
			fontadd = Integer.parseInt(xc.getconfig("flightInfoFontaddC"));
		else
			fontadd = 0;
		// app.debugPrint(fontadd);

		if (xc.getconfig("flightInfoX") != "")
			lx = Integer.parseInt(xc.getconfig("flightInfoX"));
		else
			lx = 0;
		if (xc.getconfig("flightInfoY") != "")
			ly = Integer.parseInt(xc.getconfig("flightInfoY"));
		else
			ly = 0;

		fontsize = 24 + fontadd;
		// 设置字体
		fontNum = new Font(NumFont, Font.BOLD, fontsize);
		fontLabel = new Font(FontName, Font.BOLD, Math.round(fontsize / 2.0f));
		fontUnit = new Font(NumFont, Font.PLAIN, Math.round(fontsize / 2.0f));

		numHeight = getFontMetrics(fontNum).getHeight();
		labelHeight = getFontMetrics(fontLabel).getHeight();

		// 列
		if (xc.getconfig("flightInfoColumn") != "")
			columnNum = Integer.parseInt(xc.getconfig("flightInfoColumn"));
		else
			columnNum = 3;

		initTextString();

		int fieldCount = fields.size();
		int addnum = (fieldCount % columnNum == 0) ? 0 : 1;
		// app.debugPrint(fieldCount / columnNum + addnum + 1);
		if (xc.getconfig("flightInfoEdge").equals("true"))
			setShadeWidth(10);
		else
			setShadeWidth(0);

		this.setBounds(lx, ly, (fontsize >> 1) + (int) ((columnNum + 0.5) * 5f * fontsize),
				(int) (numHeight + (fieldCount / columnNum + addnum + 1) * 1.0f * numHeight));

		repaint();
	}

	public void init(controller c, service s) {
		xc = c;
		xs = s;

		reinitConfig();

		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明

		doffset = new int[2];

		panel = new WebPanel() {

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

				doffset[0] = fontsize >> 1;
				doffset[1] = fontsize >> 1;
				int d = 0;
				int visibleIndex = 0;
				for (FlightField field : fields) {
					if (!field.visible) {
						d++;
						continue;
					}
					int fwitdh = 3 * fontsize;
					uiBaseElem._drawLabelBOSType(g2d, doffset[0], doffset[1], 1, fwitdh, fontNum, fontLabel, fontUnit,
							field.currentValue, field.label, field.unit);
					visibleIndex++;
					updateDxDy(visibleIndex, doffset);
				}

				// drawLabelBOSType(g2d, doffset[0], doffset[1], 1, fontNum,
				// fontLabel, fontUnit, sIAS, language.fIAS, "Km/h");
				// updateDxDy(++num, doffset);
				// drawLabelBOSType(g2d, doffset[0], doffset[1], 1, fontNum,
				// fontLabel, fontUnit, sIAS, language.fIAS, "Km/h");
				// updateDxDy(++num, doffset);
				// drawLabelBOSType(g2d, doffset[0], doffset[1], 1, fontNum,
				// fontLabel, fontUnit, sIAS, language.fIAS, "Km/h");
				// updateDxDy(++num, doffset);
				// g.dispose();
			}

		};
		initpanel();
		this.add(panel);
		// root = this.getContentPane();
		// setTitle(lang.fTitle);
		// setAlwaysOnTop(true);
		// setFocusableWindowState(false);
		// setFocusable(false);
		//
		// // app.debugPrint(this.isAlwaysOnTopSupported());
		// // setAlwaysOnTop(true);
		// setShowWindowButtons(false);
		// setShowTitleComponent(false);
		// setShowResizeCorner(false);
		// this.setCursor(app.blankCursor);
		// setVisible(true);
		//
		setTitle("flightInfo");
		uiWebLafSetting.setWindowOpaque(this);
		root = this.getContentPane();

		if (xc.getconfig("flightInfoEdge").equals("true")) {
			setShadeWidth(10);
		} else {
			setShadeWidth(0);
			// this.getRootPane().add(separator1);
			// this.getRootPane().add(separator2);
		}
		// setAlwaysOnTop(true);

		if (s != null)
			setVisible(true);

	}

	long updateTime = 0;

	public void drawTick() {

		// 更新字符串
		// updateString();

		root.repaint();

	}

	@Override
	public void run() {
		// TODO Auto-generated method stub
		while (doit) {
			try {
				Thread.sleep(app.threadSleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			long systemTime = xs.SystemTime;

			if (systemTime - updateTime > xc.freqService) {
				updateTime = systemTime;
				// 绘制本帧
				drawTick();
			}

		}
	}

}