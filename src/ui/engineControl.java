package ui;

import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.splitpane.WebSplitPane;

import parser.blkx;
import prog.app;
import prog.controller;
import prog.lang;
import prog.service;

public class engineControl extends WebFrame implements Runnable {
	/**
	 * 
	 */

	int isDragging = 0;
	int xx = 0;
	int yy = 0;
	public volatile boolean doit = true;
	WebSplitPane splitPane;
	public int overheattime;
	private static final long serialVersionUID = 3063042782594625576L;
	public controller xc;
	public service s;
	public blkx p;
	public int wtload1;
	public int oilload1;
	public String status;
	String NumFont;

	long freq;
	long MainCheckMili;
	int WIDTH;
	int HEIGHT;
	int lx;
	int ly;
	int OP;
	Color transParentWhite = app.colorNum;
	Color transParentWhitePlus = app.colorNum;

	Color lblShadeColor = app.colorShade;

	WebPanel panel;

	public String FontName;
	int font1 = 12;
	int font2 = 14;
	int font3 = 10;
	int fontadd = 0;
	private int fontsize;
	private Font fontNum;
	private Font fontLabel;
	private Font fontUnit;
	static Color lblColor = app.colorUnit;
	static Color lblNameColor = app.colorLabel;
	static Color lblNumColor = app.colorNum;

	public void fclose() {
		// this.setVisible(false);

		this.dispose();
		// this.close();
	}

	public void initPreview(controller c) {

		init(c, null, null);
		// setShadeWidth(10);
		this.setVisible(false);
		// this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 50));
		this.getWebRootPaneUI().setMiddleBg(app.previewColor);// 中部透明
		this.getWebRootPaneUI().setTopBg(app.previewColor);// 顶部透明

		panel.addMouseListener(new MouseAdapter() {
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
		panel.addMouseMotionListener(new MouseMotionAdapter() {
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
		// setFocusable(true);
		// setFocusableWindowState(true);

	}

	public int leftUseNum;

	// 像素个数
	public int leftValPix[];
	public int leftValMax[];
	// 每个分别是标签,数字,单位默认是空
	public String leftLblNum[][];

	int lidx_t = Integer.MAX_VALUE; // 油门
	int lidx_p = Integer.MAX_VALUE; // 桨距
	int lidx_m = Integer.MAX_VALUE; // 混合比
	int lidx_r = Integer.MAX_VALUE; // 散热器
	int lidx_c = Integer.MAX_VALUE; // 增压器

	int lidx_f = Integer.MAX_VALUE; // 燃油百分比

	// 绘制油门-桨距-散热器

	public void drawTPMRC(Graphics2D g2d, int x, int y) {

		int dx = 0;
		int dy = fontsize >> 1;

		for (int i = 0; i < leftUseNum; i++) {
			if (isJet && (i == lidx_r || i == lidx_c || i == lidx_m))
				continue;
			if (i == lidx_r || i == lidx_c || i == lidx_m || i == lidx_f) {
				if ((i == lidx_c) && (s != null && s.sState.compressorstage == 0))
					continue;
				if ((i == lidx_m) && (s != null && s.sState.mixture < 0))
					continue;
				// 横着画
				// if(isJet) continue;
				uiBaseElem.drawHBarTextNum(g2d, x, y + dy, 4 * fontsize, fontsize >> 1, leftValPix[i], 1, app.colorNum,
						leftLblNum[i][0], leftLblNum[i][0] + leftLblNum[i][1], fontLabel, fontLabel);
				dy += 1 * fontsize + (fontsize >> 2);
			} else {
				uiBaseElem.drawVBarTextNum(g2d, x + dx, y, fontsize >> 1, 4 * fontsize, leftValPix[i], 1, app.colorNum,
						leftLblNum[i][0], leftLblNum[i][0] + leftLblNum[i][1], fontLabel, fontLabel);
				dx += (5 * fontsize) >> 1;
			}

		}

	}

	public int rowNum;
	public int columnNum;
	private Container root;

	public void initLeftString() {
		leftUseNum = 0;
		leftLblNum = new String[10][];
		leftValPix = new int[10];
		leftValMax = new int[10];
		String tmp;
		for (int i = 0; i < 10; i++)
			leftLblNum[i] = new String[2];

		// 油门
		tmp = xc.getconfig("disableEngineInfoThrottle");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			leftLblNum[leftUseNum][0] = String.format("%s", lang.eThrottle);
			leftLblNum[leftUseNum][1] = String.format("%3s", "105");
			leftValMax[leftUseNum] = 110;
			leftValPix[leftUseNum] = Math.round(105f * 4 * fontsize / 110);
			lidx_t = leftUseNum++;
			columnNum++;
		}

		// 桨距
		tmp = xc.getconfig("disableEngineInfoPitch");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			leftLblNum[leftUseNum][0] = String.format("%s", lang.eProppitch);
			leftLblNum[leftUseNum][1] = String.format("%3s", "50");
			leftValMax[leftUseNum] = 100;
			leftValPix[leftUseNum] = Math.round(50f * 4 * fontsize / 100);
			lidx_p = leftUseNum++;
			columnNum++;
		}

		// 混合比
		tmp = xc.getconfig("disableEngineInfoMixture");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			leftLblNum[leftUseNum][0] = String.format("%s", lang.eMixture);
			leftLblNum[leftUseNum][1] = String.format("%3s", "70");
			leftValMax[leftUseNum] = 120;
			leftValPix[leftUseNum] = Math.round(70f * 4 * fontsize / 120);
			lidx_m = leftUseNum++;
			rowNum++;
		}

		// 散热器
		tmp = xc.getconfig("disableEngineInfoRadiator");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			leftLblNum[leftUseNum][0] = String.format("%s", lang.eRadiator);
			leftLblNum[leftUseNum][1] = String.format("%3s", "42");
			leftValMax[leftUseNum] = 100;
			leftValPix[leftUseNum] = Math.round(42f * 4 * fontsize / 100);
			lidx_r = leftUseNum++;
			rowNum++;
		}

		// 增压器
		tmp = xc.getconfig("disableEngineInfoCompressor");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			leftLblNum[leftUseNum][0] = String.format("%s", lang.eCompressor);
			leftLblNum[leftUseNum][1] = String.format("%3s", "1");
			leftValMax[leftUseNum] = 1;
			leftValPix[leftUseNum] = Math.round(0f * 4 * fontsize / 1);
			lidx_c = leftUseNum++;
			rowNum++;
		}

		tmp = xc.getconfig("disableEngineInfoLFuel");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			leftLblNum[leftUseNum][0] = String.format("%s", lang.eFuelPer);
			leftLblNum[leftUseNum][1] = String.format("%3s", "51");
			leftValMax[leftUseNum] = 100;
			leftValPix[leftUseNum] = Math.round(51f * 4 * fontsize / 100);
			lidx_f = leftUseNum++;
			rowNum++;
		}

	}

	public void init(controller xc, service ts, blkx tp) {
		this.xc = xc;
		this.s = ts;
		this.p = tp;

		overheattime = 0;
		freq = xc.freqEngineInfo;

		if (xc.getconfig("GlobalNumFont") != "")
			NumFont = xc.getconfig("GlobalNumFont");
		else
			NumFont = app.defaultNumfontName;

		if (xc.getconfig("engineInfoFont") != "")
			FontName = xc.getconfig("engineInfoFont");
		else
			FontName = app.defaultFont.getFontName();
		if (xc.getconfig("engineInfoFontadd") != "")
			fontadd = Integer.parseInt(xc.getconfig("engineInfoFontadd"));
		else
			fontadd = 0;
		// app.debugPrint(fontadd);
		if (xc.getconfig("engineControlX") != "")
			lx = Integer.parseInt(xc.getconfig("engineControlX"));
		else
			lx = 0;
		if (xc.getconfig("engineControlY") != "")
			ly = Integer.parseInt(xc.getconfig("engineControlY"));
		else
			ly = 860;

		// setIconImage(Toolkit.getDefaultToolkit().createImage("image/form1.jpg"));

		// 初始化Panel
		// initPanel();
		fontsize = 24 + fontadd;
		// 设置字体
		fontNum = new Font(NumFont, Font.BOLD, fontsize);
		fontLabel = new Font(FontName, Font.BOLD, Math.round(fontsize / 2.0f));
		fontUnit = new Font(NumFont, Font.PLAIN, Math.round(fontsize / 2.0f));

		initLeftString();

		WIDTH = fontsize * 8;
		HEIGHT = (int) ((fontsize * 4 + (fontsize * 9) >> 1) + (rowNum + 1) * (1 * fontsize + (fontsize >> 2)));
		// OP = 100;

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
				// 先画左边的条 4.5f
				drawTPMRC(g2d, 0, (fontsize * 9) >> 1);

				g.dispose();
			}
		};

		// panel.setWebColoredBackground(false);
		// panel.setBackground(new Color(0, 0, 0, 0));

		this.add(panel);

		setSize(WIDTH, HEIGHT);
		setLocation(lx, ly);

		setTitle("engineControl");
		uiWebLafSetting.setWindowOpaque(this);

		jetChecked = false;
		root = getContentPane();

		if (xc.getconfig("engineInfoEdge").equals("true"))
			setShadeWidth(10);// 玻璃效果边框
		else {
			setShadeWidth(0);
			// this.getRootPane().add(separator1);
			// this.getRootPane().add(separator2);
		}
	}

	long engineCheckMili;
	private boolean isJet;
	private boolean jetChecked;

	public void __update_num(int idx, String s, int val) {
		if (idx < leftUseNum) {
			leftLblNum[idx][1] = String.format("%3s", s);
			leftValPix[idx] = Math.round(val * 4 * fontsize / leftValMax[idx]);
		}
	}

	public void updateString() {
		if (jetChecked == false) {
			if (s.checkEngineFlag) {
				if (s.isEngJet()) {
					isJet = true;

					// 修改为推力百分比
					if (lidx_p < leftUseNum)
						leftLblNum[lidx_p][0] = lang.eThurstP;
				}
				jetChecked = true;
			}
		}
		if (isJet) {
		} else {

		}
		
		// 油门
		__update_num(lidx_t, s.throttle, s.sState.throttle);

		// 桨距
		if (!isJet) {
			if (!s.RPMthrottle.equals(service.nastring)) {
				__update_num(lidx_p, s.RPMthrottle, s.sState.RPMthrottle);
			} else
				__update_num(lidx_p, s.RPMthrottle, 0);
		} else
			__update_num(lidx_p, s.sThurstPercent, (int) s.thurstPercent);
		// 混合比
		if (!s.mixture.equals(service.nastring)) {
			__update_num(lidx_m, s.mixture, s.sState.mixture);
		} else {
			__update_num(lidx_m, s.mixture, 0);
		}
		// else{
		// lidx_m = leftUseNum;
		// }
		// 散热器
		if (!s.radiator.equals(service.nastring))
			__update_num(lidx_r, s.radiator, s.sState.radiator);
		else {
			__update_num(lidx_r, s.radiator, 0);
		}

		// 增压器
		if (lidx_c < leftUseNum) {
			if (s.sState.compressorstage - 1 > leftValMax[lidx_c])
				leftValMax[lidx_c] = s.sState.compressorstage - 1;
		}
		__update_num(lidx_c, s.compressorstage, s.sState.compressorstage - 1);

		__update_num(lidx_f, s.sfuelPercent, s.fuelPercent);
	}

	public void drawTick() {

		// 更新字符串
		updateString();

		root.repaint();

	}

	@Override
	public void run() {
		while (doit) {

			try {
				Thread.sleep(app.threadSleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}

			if (s.SystemTime - engineCheckMili > xc.freqService) {
				engineCheckMili = s.SystemTime;
				if (s.sState != null) {
					
					drawTick();

				}
			}
		}
	}
}