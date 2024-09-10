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

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import prog.app;
import prog.controller;
import prog.lang;
import prog.service;

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
		this.setVisible(false);
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
					setVisible(true);
					repaint();
				}
			}
		});
		this.setCursor(null);
		setVisible(true);
		// setFocusableWindowState(true);
		// setFocusable(true);
	}

	String[][] totalString;
	int useNum = 0;

	//
	int idx_ias = Integer.MAX_VALUE;
	int idx_tas = Integer.MAX_VALUE;
	int idx_mach = Integer.MAX_VALUE;
	int idx_height = Integer.MAX_VALUE;
	int idx_vario = Integer.MAX_VALUE;
	int idx_sep = Integer.MAX_VALUE;

	int idx_acc = Integer.MAX_VALUE;
	int idx_wx = Integer.MAX_VALUE;
	int idx_ny = Integer.MAX_VALUE;
	int idx_turn = Integer.MAX_VALUE;
	int idx_rds = Integer.MAX_VALUE;
	int idx_dir = Integer.MAX_VALUE;

	int idx_aoa = Integer.MAX_VALUE;
	int idx_aos = Integer.MAX_VALUE;
	int idx_ws = Integer.MAX_VALUE;

	int idx_rda = Integer.MAX_VALUE;
	
	public Boolean[] totalSwitch;
	private int numHeight;
	private int labelHeight;
	private Container root;

	public void initTextString() {
		totalString = new String[20][];
		String tmp;
		totalSwitch = new Boolean[20];
		for (int i = 0; i < 20; i++) {
			totalString[i] = new String[3];
			totalSwitch[i] = true;
		}

		// IAS
		// 判断是否添加
		tmp = xc.getconfig("disableFlightInfoIAS");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "1200");
			totalString[useNum][1] = String.format("%s", lang.fIAS);
			totalString[useNum][2] = String.format("%s", "Km/h");
			idx_ias = useNum++;
		}
		// TAS
		tmp = xc.getconfig("disableFlightInfoTAS");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "1300");
			totalString[useNum][1] = String.format("%s", lang.fTAS);
			totalString[useNum][2] = String.format("%s", "Km/h");
			idx_tas = useNum++;
		}
		// MACH
		tmp = xc.getconfig("disableFlightInfoMach");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "1.0");
			totalString[useNum][1] = String.format("%s", lang.fMach);
			totalString[useNum][2] = String.format("%s", "Mach");
			idx_mach = useNum++;
		}
		// dir
		tmp = xc.getconfig("disableFlightInfoCompass");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "360");
			totalString[useNum][1] = String.format("%s", lang.fCompass);
			totalString[useNum][2] = String.format("%s", "Deg");
			idx_dir = useNum++;
		}
		// 高度
		tmp = xc.getconfig("disableFlightInfoHeight");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "11451");
			totalString[useNum][1] = String.format("%s", lang.fAlt);
			totalString[useNum][2] = String.format("%s", "M");
			idx_height = useNum++;
		}
		// 雷达高度
		tmp = xc.getconfig("disableFlightInfoRadioAlt");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "1000");
			totalString[useNum][1] = String.format("%s", lang.fRa);
			totalString[useNum][2] = String.format("%s", "M");
			idx_rda = useNum++;
		}

		// 爬升率
		tmp = xc.getconfig("disableFlightInfoVario");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "14");
			totalString[useNum][1] = String.format("%s", lang.fVario);
			totalString[useNum][2] = String.format("%s", "M/s");
			idx_vario = useNum++;
		}
		// SEP
		tmp = xc.getconfig("disableFlightInfoSEP");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "-191");
			totalString[useNum][1] = String.format("%s", lang.fSEP);
			totalString[useNum][2] = String.format("%s", "M/s");
			idx_sep = useNum++;
		}
		// acc
		tmp = xc.getconfig("disableFlightInfoAcc");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "98");
			totalString[useNum][1] = String.format("%s", lang.fAcc);
			totalString[useNum][2] = String.format("%s", "M/s^2");
			idx_acc = useNum++;
		}
		// wx
		tmp = xc.getconfig("disableFlightInfoWx");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "10");
			totalString[useNum][1] = String.format("%s", lang.fWx);
			totalString[useNum][2] = String.format("%s", "Deg/s");
			idx_wx = useNum++;
		}
		// ny
		tmp = xc.getconfig("disableFlightInfoNy");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "8.1");
			totalString[useNum][1] = String.format("%s", lang.fGL);
			totalString[useNum][2] = String.format("%s", "G");
			idx_ny = useNum++;
		}
		// turn
		tmp = xc.getconfig("disableFlightInfoTurn");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "155");
			totalString[useNum][1] = String.format("%s", lang.fTRr);
			totalString[useNum][2] = String.format("%s", "Deg/s");
			idx_turn = useNum++;
		}
		// rds
		tmp = xc.getconfig("disableFlightInfoTurnRadius");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "2003");
			totalString[useNum][1] = String.format("%s", lang.fTR);
			totalString[useNum][2] = String.format("%s", "M");
			idx_rds = useNum++;
		}
		// aoa
		tmp = xc.getconfig("disableFlightInfoAoA");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "1.5");
			totalString[useNum][1] = String.format("%s", lang.fAoA);
			totalString[useNum][2] = String.format("%s", "Deg");
			idx_aoa = useNum++;
		}
		// aos
		tmp = xc.getconfig("disableFlightInfoAoS");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "-0.1");
			totalString[useNum][1] = String.format("%s", lang.fAoS);
			totalString[useNum][2] = String.format("%s", "Deg");
			idx_aos = useNum++;
		}
		// 可变翼
		tmp = xc.getconfig("disableFlightInfoWingSweep");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "100");
			totalString[useNum][1] = String.format("%s", lang.fWs);
			totalString[useNum][2] = String.format("%s", "%");
			idx_ws = useNum++;
		}

	}

	public void __update_num(int idx, String s) {
		if (idx < useNum) {
			totalString[idx][0] = String.format("%5s", s);
//			totalString[idx][0] = s;
		}
	}

	public void updateString() {

		// 跳过雷达高

		if (idx_ws < useNum) {
			if (xs.sWingSweep.equals(service.nastring)) {
				totalSwitch[idx_ws] = false;
			} else {
				totalSwitch[idx_ws] = true;
			}
		}
		if (idx_rda < useNum) {
			if (xs.sRadioAlt.equals(service.nastring)) {
				totalSwitch[idx_rda] = false;
			} else {
				totalSwitch[idx_rda] = true;
			}
		}

		// ias
		__update_num(idx_ias, xs.IAS);

		// TAS
		__update_num(idx_tas, xs.TAS);

		// MACH
		__update_num(idx_mach, xs.M);

		// Height
		__update_num(idx_height, xs.salt);

		// Vario
		__update_num(idx_vario, xs.Vy);

		// SEP
		__update_num(idx_sep, xs.sSEP);

		// dir
		__update_num(idx_dir, xs.compass);
		// acc
		__update_num(idx_acc, xs.sAcc);
		// wx
		__update_num(idx_wx, xs.Wx);
		// ny
//		__update_num(idx_ny, xs.Ny);
		// 使用修正过的过载
		__update_num(idx_ny, xs.sN);
		// turn
		__update_num(idx_turn, xs.sTurnRate);
		// rds
		__update_num(idx_rds, xs.sTurnRds);
//		totalString[idx_rds][2] = "m"+xs.sN;
		// aoa
		__update_num(idx_aoa, xs.AoA);
		// aos
		__update_num(idx_aos, xs.AoS);
		// ws
		__update_num(idx_ws, xs.sWingSweep);

		// 无线电测距高
		__update_num(idx_rda, xs.sRadioAlt);

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

	public void init(controller c, service s) {
		xc = c;
		xs = s;
		int lx;
		int ly;

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

		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明

		// 旧的
		// this.setBounds(lx, ly, 580, 250);

		// 新的3x3

		// setAutoRequestFocus(false);
		// setDefaultCloseOperation(WebFrame.EXIT_ON_CLOSE);
		// setFont(new Font(FontName, Font.PLAIN, 10 + fontadd));
		// // getContentPane().setBackground(lblNumColor);
		// setLayout(null);
		fontsize = 24 + fontadd;
		// 设置字体
		fontNum = new Font(NumFont, Font.BOLD, fontsize);
		fontLabel = new Font(FontName, Font.BOLD, Math.round(fontsize / 2.0f));
		fontUnit = new Font(NumFont, Font.PLAIN, Math.round(fontsize / 2.0f));

		numHeight = getFontMetrics(fontNum).getHeight();
		labelHeight = getFontMetrics(fontLabel).getHeight();

		// numWidth = getFontMetrics(fontNum).getWidths();
		// 列
		if (xc.getconfig("flightInfoColumn") != "")
			columnNum = Integer.parseInt(xc.getconfig("flightInfoColumn"));
		else
			columnNum = 3;

		initTextString();

		int addnum = (useNum % columnNum == 0) ? 0 : 1;
		// app.debugPrint(useNum / columnNum + addnum + 1);
		this.setBounds(lx, ly, (fontsize >> 1) + (int) ((columnNum + 0.5) * 5f * fontsize),
				(int) (numHeight + (useNum / columnNum + addnum + 1) * 1.0f * numHeight));

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
				for (int i = 0; i < useNum; i++) {
					if (!totalSwitch[i]) {
						d++;
						continue;
					}
					int fwitdh = 3 * fontsize;
					uiBaseElem._drawLabelBOSType(g2d, doffset[0], doffset[1], 1, fwitdh, fontNum, fontLabel, fontUnit,
							totalString[i][0], totalString[i][1], totalString[i][2]);
					updateDxDy(i + 1 - d, doffset);
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
				g.dispose();
			}

		};
		initpanel();
		this.add(panel);
//		root = this.getContentPane();
//		setTitle(lang.fTitle);
//		setAlwaysOnTop(true);
//		setFocusableWindowState(false);
//		setFocusable(false);
//
//		// app.debugPrint(this.isAlwaysOnTopSupported());
//		// setAlwaysOnTop(true);
//		setShowWindowButtons(false);
//		setShowTitleComponent(false);
//		setShowResizeCorner(false);
//		this.setCursor(app.blankCursor);
//		setVisible(true);
//		
		uiWebLafSetting.setWindowOpaque(this);
		root = this.getContentPane();
		
		if (xc.getconfig("flightInfoEdge").equals("true")){
			setShadeWidth(10);
		}
		else {
			setShadeWidth(0);
			// this.getRootPane().add(separator1);
			// this.getRootPane().add(separator2);
		}
		// setAlwaysOnTop(true);

	}

	long updateTime = 0;

	public void drawTick() {

		// 更新字符串
//		updateString();

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