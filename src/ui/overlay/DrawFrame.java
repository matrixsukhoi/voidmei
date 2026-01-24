package ui.overlay;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;

import java.awt.RenderingHints;

import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;

import com.alee.extended.panel.WebButtonGroup;
import com.alee.laf.button.WebButton;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;

import parser.Blkx;
import parser.FlightAnalyzer;
import prog.Application;
import prog.Controller;
import prog.i18n.Lang;
import prog.Service;

public class DrawFrame extends WebFrame implements Runnable {
	/**
	 * 
	 */
	public volatile boolean doit = true;
	private static final long serialVersionUID = 6290400898885722422L;
	Controller xc;
	FlightAnalyzer fA;

	public DrawFrame() {
		super();
		prog.util.Logger.info("Overlay",
				"Created instance: " + this.getClass().getSimpleName() + "@" + Integer.toHexString(hashCode()));
	}

	@Override
	public void dispose() {
		prog.util.Logger.info("Overlay",
				"Disposing instance: " + this.getClass().getSimpleName() + "@" + Integer.toHexString(hashCode()));
		super.dispose();
	}

	WebPanel panel;
	int pixIndex = 0;
	int Index = 8;
	boolean useBlkx = true;
	double ggx4;
	double ggy4;
	Blkx Blkx;
	double fY[];
	double fX[];

	public WebButton createButton(String text) {
		WebButton a = new WebButton(text);
		a.setShadeWidth(1);
		a.setDrawShade(true);
		// a.getWebUI().setInnerShadeColor(new Color(255,255,255,200));
		// a.getWebUI().setInnerShadeWidth(10);
		a.setFont(Application.defaultFontBig);
		a.setTopBgColor(new Color(0, 0, 0, 0));
		a.setBottomBgColor(new Color(0, 0, 0, 0));
		// a.setUndecorated(false);
		// a.setShadeWidth(1);
		a.setBorderPainted(false);

		return a;

	}

	public WebButtonGroup createbuttonGroup() {
		WebButton A = createButton(Lang.dFprev);
		WebButton B = createButton(Lang.dFnext);

		WebButtonGroup G = new WebButtonGroup(true, A, B);
		// G.setBorderColor(new Color(0, 0, 0, 0));
		// G.setButtonsDrawSides(true, true, false,false);
		// A.setPreferredHeight(30);
		A.setPreferredWidth(120);
		// B.setPreferredHeight(30);

		B.setPreferredWidth(120);
		B.setRound(10);
		G.setButtonsDrawSides(false, false, false, true);
		G.setButtonsForeground(new Color(0, 0, 0, 200));
		// G.setButtonsInnerShadeColor(new Color(0,0,0));
		// G.setButtonsInnerShadeWidth(5);
		G.setButtonsShadeWidth(3);
		// G.setButtonsDrawFocus(false);
		A.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				if (pixIndex > 0)
					pixIndex--;
				repaint();
			}
		});
		B.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				if (pixIndex + 1 < Index)
					pixIndex++;
				repaint();
			}
		});
		// G.setPaintSides(false, false, false, false);
		return G;
	}

	int findMin(int X[]) {
		int i;
		int min = Integer.MAX_VALUE;
		for (i = 0; i < X.length; i++) {
			if (X[i] < min)
				min = X[i];
		}
		return min;
	}

	int findMax(int X[]) {
		int i;
		int max = Integer.MIN_VALUE;
		for (i = 0; i < X.length; i++) {
			if (X[i] > max)
				max = X[i];
		}
		return max;
	}

	double findMin(double X[]) {
		int i;
		double min = Double.MAX_VALUE;
		for (i = 0; i < X.length; i++) {
			if (X[i] < min)
				min = X[i];
		}
		return min;
	}

	double findMax(double X[]) {
		int i;
		double max = Double.MIN_VALUE;
		for (i = 0; i < X.length; i++) {
			if (X[i] > max)
				max = X[i];
		}
		return max;
	}

	int searchMin(int X[]) {
		int i;
		int min = 655353535;
		for (i = fA.initaltStage; i <= fA.curaltStage - 1; i++) {
			if (X[i] < min)
				min = X[i];
		}
		return min;
	}

	int searchMax(int X[]) {
		int i;
		int max = -655353535;
		for (i = fA.initaltStage; i <= fA.curaltStage - 1; i++) {
			if (X[i] > max)
				max = X[i];
		}
		return max;
	}

	double searchMin(double X[]) {
		int i;
		double min = 655353535;
		for (i = fA.initaltStage; i <= fA.curaltStage - 1; i++) {
			if (X[i] < min)
				min = X[i];
		}
		return min;
	}

	double searchMax(double X[]) {
		int i;
		double max = -655353535;
		for (i = fA.initaltStage; i <= fA.curaltStage - 1; i++) {
			if (X[i] > max)
				max = X[i];
		}
		return max;
	}

	void initpanel() {
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
	}

	void setFrameOpaque() {
		this.getWebRootPaneUI().setMiddleBg(new Color(255, 255, 255, 255));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(255, 255, 255, 255));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(255, 255, 255, 255));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(255, 255, 255, 255));// 外描边透明
	}

	void getdata(String planename) {
		String fmfile;
		String unitSystem;
		int i;
		// 读入fm
		Blkx = new Blkx("./data/aces/gamedata/flightmodels/" + planename + ".Blkx", planename + ".blk");
		if (Blkx.valid) {
			fmfile = Blkx.getlastone("fmfile");
			fmfile = fmfile.substring(1, fmfile.length() - 1);
			if (fmfile.indexOf("blk") == -1)
				fmfile = fmfile + ".blk";
			for (i = 0; i < fmfile.length(); i++) {
				if (fmfile.charAt(i) == '/')
					break;
			}
			if (i + 1 >= fmfile.length()) {
				fmfile = planename + ".blk";
			} else
				fmfile = fmfile.substring(i + 1);
			// Application.debugPrint(fmfile);

			// 读入fmfile
			Blkx = new Blkx("./data/aces/gamedata/flightmodels/fm/" + fmfile + "x", planename + ".blk");
			// Application.debugPrint(Blkx.data);
			if (Blkx.valid)
				Blkx.getAllplotdata();
		}

	}

	void drawXY(Graphics2D g, int x, int y, int dwidth, int dheight, String title, String xName, String yName,
			String xD, String yD, double xmin, double xmax, double ymin, double ymax, int xgap, int ygap) {
		// 确定画笔
		g.setStroke(new BasicStroke(3));
		g.setColor(new Color(0, 0, 0, 250));
		int pxmin = (int) xmin;
		int pxmax = (int) xmax + xgap;
		int pymin = (int) ymin;
		int pymax = (int) ymax + ygap;
		int intervalX = xgap;
		int intervalY = ygap;
		double ggx = 0;
		double ggy = 0;
		if (intervalX == 0)
			intervalX = 1;
		if (intervalY == 0)
			intervalY = 1;
		if (pxmax - pxmin != 0) {
			ggx = (double) dwidth / (double) (pxmax - pxmin);
		}
		if (pymax - pymin != 0) {
			ggy = (double) dheight / (double) (pymax - pymin);
		}

		// 标题
		g.setFont(new Font(Application.defaultFontName, Font.PLAIN, 16));
		g.drawString(title, x + dwidth / 2, y);
		y = y + 10;// 往下推10
		g.setFont(new Font(Application.defaultNumfontName, Font.PLAIN, 10));

		// x轴与箭头
		g.drawLine(x, y + dheight, x + dwidth, y + dheight);

		int ii = (int) ((pxmax - pxmin) / intervalX);
		for (; ii >= 0; ii--) {
			// 坐标轴刻度
			g.setStroke(new BasicStroke(1));
			g.drawLine((int) (x + ii * intervalX * ggx), y + dheight, (int) (x + ii * intervalX * ggx), y);
			g.drawString(String.valueOf(pxmin + ii * intervalX), (int) (x + ii * intervalX * ggx), y + dheight + 15);
		}
		// x轴单位
		g.setFont(new Font(Application.defaultFontName, Font.PLAIN, 14));
		g.drawString(xD, x + dwidth + 5, y + dheight);
		g.setFont(new Font(Application.defaultNumfontName, Font.PLAIN, 10));

		g.setStroke(new BasicStroke(3));
		// y轴与箭头
		g.drawLine(x, y + dheight, x, y);
		// y轴刻度
		ii = (int) ((pymax - pymin) / intervalY);
		for (; ii >= 0; ii--) {
			g.setStroke(new BasicStroke(1));
			g.drawLine(x, (int) (y + dheight - ii * intervalY * ggy), x + dwidth,
					(int) (y + dheight - ii * intervalY * ggy));

			g.drawString(String.valueOf(pymin + ii * intervalY), x - 30, (int) (y + dheight - ii * intervalY * ggy));
		}
		// y轴单位
		g.setFont(new Font(Application.defaultFontName, Font.PLAIN, 14));
		g.drawString(yD, x - 5, y - 10);

	}

	void drawPoint(Graphics2D g, int x, int y, int dwidth, int dheight, double ggx, double ggy, double ix[],
			double iy[], int pxmin, int pymin, Color C) {
		g.setStroke(new BasicStroke(1));
		g.setColor(C);
		y = y + 10;// 往下推10
		// 绘点
		int ii = 0;
		for (ii = 0; ii < ix.length; ii++) {
			// Application.debugPrint((y + dheight) +" "+(y + dheight
			// -(iy[ii]-pymin) * ggy));
			g.drawOval((int) (x + (ix[ii] - pxmin) * ggx) - 1, (int) (y + dheight - (iy[ii] - pymin) * ggy) - 1, 2, 2);
		}

		// 连线
		g.setStroke(new BasicStroke(1));

		for (ii = 0; ii < ix.length - 1; ii++) {
			g.drawLine((int) (x + (ix[ii] - pxmin) * ggx), (int) (y + dheight - (iy[ii] - pymin) * ggy),
					(int) (x + (ix[ii + 1] - pxmin) * ggx), (int) (y + dheight - (iy[ii + 1] - pymin) * ggy));
		}

	}

	void drawExample(Graphics2D g, int x, int y, int dheight, Color C, String name) {
		g.setStroke(new BasicStroke(1));
		g.setColor(C);
		g.drawLine(x, y + dheight + 40, x + 20, y + dheight + 40);
		g.setColor(new Color(0, 0, 0, 250));
		g.setFont(new Font(Application.defaultNumfontName, Font.PLAIN, 10));
		g.drawString(name, x + 25, y + dheight + 45);
	}

	void drawCoordinates(Graphics2D g, int x, int y, int dwidth, int dheight, String title, String xName, String yName,
			double X[], String xD, String yD) {
		g.setStroke(new BasicStroke(3));
		g.setColor(new Color(0, 0, 0, 250));
		int movex = 0;
		int pmin = (int) searchMin(X);
		int pmax = (int) searchMax(X) + 1;
		double ggx = 0;
		double ggy = 0;
		int intervalX = Math.round(((pmax - pmin) / 10) / 10.0f) * 10;// X轴间距
		int intervalY = Math.round((fA.curaltStage - fA.initaltStage) * 100 / 800.0f) * 100;// Y轴高度间距
		if (pmax - pmin != 0) {
			ggx = (dwidth - movex) / (double) (pmax - pmin);
		}
		ggy = (double) (dheight) / (fA.curaltStage * 100);

		// 标题
		g.setFont(new Font(Application.defaultFontName, Font.PLAIN, 16));
		g.drawString(title, x + dwidth / 2, y);
		y = y + 10;// 往下推10
		g.setFont(new Font(Application.defaultNumfontName, Font.PLAIN, 10));
		// x轴与箭头
		g.drawLine(x, y + dheight, x + dwidth, y + dheight);

		// x轴刻度
		if (intervalX == 0)
			intervalX = 1;
		int ii = (int) ((pmax - pmin) / intervalX);
		for (; ii >= 0; ii--) {
			// 坐标轴刻度
			g.setStroke(new BasicStroke(1));
			g.drawLine((int) (movex + x + ii * intervalX * ggx), y + dheight, (int) (movex + x + ii * intervalX * ggx),
					y);
			// Application.debugPrint("X坐标"+(pmin + ii * intervalX)+"位置"+(int)
			// (movex + x + ii * intervalX * ggx));
			g.drawString(String.valueOf(pmin + ii * intervalX), (int) (movex + x + ii * intervalX * ggx),
					y + dheight + 15);
		}
		// x轴单位
		g.setFont(new Font(Application.defaultFontName, Font.PLAIN, 14));
		g.drawString(xD, x + dwidth + 5, y + dheight);
		g.setFont(new Font(Application.defaultNumfontName, Font.PLAIN, 10));

		g.setStroke(new BasicStroke(3));

		// y轴与箭头
		g.drawLine(x, y + dheight, x, y);
		// g.drawLine(x - 5, y + 5, x, y);
		// g.drawLine(x + 5, y + 5, x, y);
		// y轴刻度
		if (intervalY == 0)
			intervalY = 100;
		ii = (int) (fA.curaltStage * 100 / intervalY);
		for (; ii >= 0; ii--) {
			g.setStroke(new BasicStroke(1));
			g.drawLine(x, (int) (y + dheight - ii * intervalY * ggy), x + dwidth,
					(int) (y + dheight - ii * intervalY * ggy));

			g.drawString(String.valueOf(ii * intervalY), x - 30, (int) (y + dheight - ii * intervalY * ggy));
		}
		// y轴单位
		g.setFont(new Font(Application.defaultFontName, Font.PLAIN, 14));
		g.drawString(yD, x - 5, y - 10);

		// 绘点
		for (ii = fA.initaltStage; ii <= fA.curaltStage - 1; ii++) {
			g.drawOval((int) (movex + x + (X[ii] - pmin) * ggx) - 1, (int) (y + dheight - (ii) * 100 * ggy) - 1, 2, 2);
		}

		// 连线
		g.setStroke(new BasicStroke(1));
		for (ii = fA.initaltStage; ii < fA.curaltStage - 1; ii++) {
			if (Math.abs(X[ii] - X[ii + 1]) > 100) {
				X[ii + 1] = X[ii + 2];
			}
			g.drawLine((int) (movex + x + (X[ii] - pmin) * ggx), (int) (y + dheight - (ii) * 100 * ggy),
					(int) (movex + x + (X[ii + 1] - pmin) * ggx), (int) (y + dheight - (ii + 1) * 100 * ggy));
		}

	}

	void drawCoordinates(Graphics2D g, int x, int y, int dwidth, int dheight, String title, String xName, String yName,
			int X[], String xD, String yD) {
		g.setStroke(new BasicStroke(3));
		g.setColor(new Color(0, 0, 0, 250));
		int movex = 0;
		int pmin = searchMin(X) / 10 * 10;
		int pmax = searchMax(X);
		double ggx = 0;
		double ggy = 0;
		int intervalX = Math.round(((pmax - pmin) / 10) / 10.0f) * 10;// X轴间距
		int intervalY = Math.round((fA.curaltStage - fA.initaltStage) * 100 / 800.0f) * 100;// Y轴高度间距
		if (pmax - pmin != 0) {
			ggx = (dwidth - movex) / (double) (pmax - pmin);
		}
		ggy = (double) (dheight) / (fA.curaltStage * 100);

		// 标题
		g.setFont(new Font(Application.defaultFontName, Font.PLAIN, 16));
		g.drawString(title, x + dwidth / 2, y);
		y = y + 10;// 往下推10

		g.setFont(new Font(Application.defaultNumfontName, Font.PLAIN, 10));
		// x轴与箭头
		g.drawLine(x, y + dheight, x + dwidth, y + dheight);
		// g.drawLine(x + dwidth, y + dheight, x + dwidth - 5, y + dheight -
		// 5);箭头
		// g.drawLine(x + dwidth - 5, y + dheight + 5, x + dwidth, y +
		// dheight);箭头

		// x轴刻度
		if (intervalX == 0)
			intervalX = 1;
		int ii = (int) ((pmax - pmin) / intervalX);
		for (; ii >= 0; ii--) {
			// 坐标轴刻度
			g.setStroke(new BasicStroke(1));
			g.drawLine((int) (movex + x + ii * intervalX * ggx), y + dheight, (int) (movex + x + ii * intervalX * ggx),
					y);
			// Application.debugPrint("X坐标"+(pmin + ii * intervalX)+"位置"+(int)
			// (movex + x + ii * intervalX * ggx));
			g.drawString(String.valueOf(pmin + ii * intervalX), (int) (movex + x + ii * intervalX * ggx),
					y + dheight + 15);
		}
		// x轴单位
		g.setFont(new Font(Application.defaultFontName, Font.PLAIN, 14));
		g.drawString(xD, x + dwidth + 5, y + dheight);
		g.setFont(new Font(Application.defaultNumfontName, Font.PLAIN, 10));

		g.setStroke(new BasicStroke(3));

		// y轴与箭头
		g.drawLine(x, y + dheight, x, y);
		// g.drawLine(x - 5, y + 5, x, y);
		// g.drawLine(x + 5, y + 5, x, y);
		// y轴刻度
		if (intervalY == 0)
			intervalY = 100;
		ii = (int) (fA.curaltStage * 100 / intervalY);
		for (; ii >= 0; ii--) {
			g.setStroke(new BasicStroke(1));
			g.drawLine(x, (int) (y + dheight - ii * intervalY * ggy), x + dwidth,
					(int) (y + dheight - ii * intervalY * ggy));

			g.drawString(String.valueOf(ii * intervalY), x - 30, (int) (y + dheight - ii * intervalY * ggy));
		}
		// y轴单位

		g.setFont(new Font(Application.defaultFontName, Font.PLAIN, 14));
		g.drawString(yD, x - 5, y - 10);

		// 绘点
		for (ii = fA.initaltStage; ii <= fA.curaltStage - 1; ii++) {

			g.drawOval((int) (movex + x + (X[ii] - pmin) * ggx) - 1, (int) (y + dheight - (ii) * 100 * ggy) - 1, 2, 2);
		}

		// 连线
		g.setStroke(new BasicStroke(1));
		for (ii = fA.initaltStage; ii < fA.curaltStage - 1; ii++) {
			if (Math.abs(X[ii] - X[ii + 1]) > 100) {
				X[ii + 1] = X[ii + 2];
			}
			g.drawLine((int) (movex + x + (X[ii] - pmin) * ggx), (int) (y + dheight - (ii) * 100 * ggy),
					(int) (movex + x + (X[ii + 1] - pmin) * ggx), (int) (y + dheight - (ii + 1) * 100 * ggy));
		}

	}

	public void init(Controller c, FlightAnalyzer A) {
		// 特殊处理
		xc = c;
		fA = A;

		fA.time[fA.initaltStage] = 0;
		fA.power[fA.initaltStage] = fA.power[fA.initaltStage + 1] + fA.power[fA.initaltStage + 1]
				- fA.power[fA.initaltStage + 2];
		fA.thrust[fA.initaltStage] = fA.thrust[fA.initaltStage + 1] + fA.thrust[fA.initaltStage + 1]
				- fA.thrust[fA.initaltStage + 2];
		fA.eff[fA.initaltStage] = fA.eff[fA.initaltStage + 1] + fA.eff[fA.initaltStage + 1]
				- fA.eff[fA.initaltStage + 2];
		fA.sep[fA.initaltStage] = fA.sep[fA.initaltStage + 1] + fA.sep[fA.initaltStage + 1]
				- fA.sep[fA.initaltStage + 2];

		fY = new double[fA.curaltStage - fA.initaltStage];
		int xk = fA.initaltStage;
		for (int i = 0; i < fY.length; i++) {
			fY[i] = xk * 100;
			xk++;
		}

		getdata(fA.type);

		setFrameOpaque();

		this.setBounds(0, 0, 1200, 830);

		panel = new WebPanel() {

			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
				// g2d.setRenderingHint(RenderingHints.KEY_RENDERING,
				// RenderingHints.VALUE_RENDER_QUALITY);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);
				// 绘制坐标系
				if (pixIndex == 0)
					drawCoordinates(g2d, 50, 50, 1024, 576, Lang.dFTitle1, Lang.dFTitle1X, Lang.dFTitle1Y,
							fA.time, "s", "m");
				// 绘制刻度
				if (pixIndex == 1) {
					if ((fA.engineType == Service.ENGINE_TYPE_PROP)
							|| fA.engineType == (Service.ENGINE_TYPE_TURBOPROP)) {
						// System.out.println("绘制引擎功率");
						drawCoordinates(g2d, 50, 50, 1024, 576, Lang.dFTitle2, Lang.dFTitle2X,
								Lang.dFTitle2Y, fA.power, "hp", "m");
					} else {
						// System.out.println("绘制引擎推力");
						drawCoordinates(g2d, 50, 50, 1024, 576, Lang.dFTitle3, Lang.dFTitle3X,
								Lang.dFTitle3Y, fA.thrust, "kg", "m");
					}
				}
				if (pixIndex == 2)
					drawCoordinates(g2d, 50, 50, 1024, 576, Lang.dFTitle4, Lang.dFTitle4X, Lang.dFTitle4Y,
							fA.eff, "hp", "m");
				if (pixIndex == 3)
					drawCoordinates(g2d, 50, 50, 1024, 576, Lang.dFTitle5, Lang.dFTitle5X, Lang.dFTitle5Y,
							fA.sep, "m/s", "m");
				if (pixIndex == 5) {
					int num = fA.getNoZerosNum(fA.turn_load);

					double[] iasx = new double[num];
					double[] gy = new double[num];
					double[] gseploss = new double[num];
					fA.removeLoadZeroes(iasx, gy, gseploss);

					double xmin = findMin(iasx);
					double xmax = findMax(iasx);
					// Application.debugPrint(xmin+" "+xmax);
					double ymin = findMin(gy);
					double ymax = findMax(gy);
					int dwidth = 1024;
					int dheight = 576;
					int xgap = Math.round((((int) xmax + 1 - (int) xmin) / 10) / 10.0f) * 10;
					int ygap = 1;
					int pxmin = (int) xmin;
					int pxmax = (int) xmax + xgap;
					int pymin = (int) (ymin / 1.0f) * 1;
					int pymax = (int) (ymax / 1.0f) * 1 + ygap;
					double ggx4 = 0;
					double ggy4 = 0;
					if (pxmax - pxmin != 0) {
						ggx4 = (double) dwidth / (double) (pxmax - pxmin);
					}
					if (pymax - pymin != 0) {
						ggy4 = (double) dheight / (double) (pymax - pymin);
					}

					drawXY(g2d, 50, 50, dwidth, dheight, "示空速-法向过载曲线", "示空速", "法向过载", "km/h", "G", xmin, xmax, ymin,
							ymax, xgap, ygap);
					drawPoint(g2d, 50, 50, dwidth, dheight, ggx4, ggy4, iasx, gy, pxmin, pymin,
							new Color(0, 0, 0, 250));
					drawExample(g2d, 50, 60, dheight, new Color(0, 0, 0, 250), "法向过载");

					// int xk = fA.initaltStage;
					// fX = new double[fA.curaltStage - fA.initaltStage];
					// for (int i = 0; i < fX.length; i++) {
					// fX[i] = fA.time[xk];
					// // Application.debugPrint(fX[i]);
					// xk++;
					// }
					// double xmin = 0;
					// double xmax = findMax(Blkx.loc.x) > findMax(fA.time) ?
					// findMax(Blkx.loc.x) : findMax(fA.time);
					// double ymin = 0;
					// double ymax = findMax(Blkx.loc.y) > findMax(fY) ?
					// findMax(Blkx.loc.y) : findMax(fY);
					// int dwidth = 1024;
					// int dheight = 576;
					// int xgap = Math.round((((int) xmax + 1 - (int) xmin) /
					// 10) / 10.0f) * 10;
					// int ygap = 1000;
					// int pxmin = (int) xmin;
					// int pxmax = (int) xmax + xgap;
					// int pymin = (int) ymin;
					// int pymax = (int) ymax + ygap;
					// double ggx4 = 0;
					// double ggy4 = 0;
					// if (pxmax - pxmin != 0) {
					// ggx4 = (double) dwidth / (double) (pxmax - pxmin);
					// }
					// if (pymax - pymin != 0) {
					// ggy4 = (double) dheight / (double) (pymax - pymin);
					// }
					//
					// drawXY(g2d, 50, 50, dwidth, dheight, "爬升对比", "时间", "高度",
					// "s", "m", xmin, xmax, ymin, ymax, xgap,
					// ygap);
					// drawPoint(g2d, 50, 50, dwidth, dheight, ggx4, ggy4,
					// Blkx.loc.x, Blkx.loc.y, pxmin, pymin,
					// Color.blue);
					// drawExample(g2d, 50, 50, dheight, Color.blue, "FM提取数据");
					// drawPoint(g2d, 50, 50, dwidth, dheight, ggx4, ggy4, fX,
					// fY, pxmin, pymin, Color.red);
					// drawExample(g2d, 50, 60, dheight, Color.red, "试飞数据");
				}
				if (pixIndex == 6) {
					double xmin = findMin(Blkx.loc2.x) < findMin(Blkx.loc1.x) ? findMin(Blkx.loc2.x)
							: findMin(Blkx.loc1.x);
					double xmax = findMax(Blkx.loc1.x) > findMax(Blkx.loc1.x) ? findMax(Blkx.loc1.x)
							: findMax(Blkx.loc1.x);
					// Application.debugPrint(xmin+" "+xmax);
					double ymin = 0;
					double ymax = findMax(Blkx.loc1.y) > findMax(Blkx.loc2.y) ? findMax(Blkx.loc1.y)
							: findMax(Blkx.loc2.y);
					int dwidth = 1024;
					int dheight = 576;
					int xgap = Math.round((((int) xmax + 1 - (int) xmin) / 10) / 10.0f) * 10;
					int ygap = 1000;
					int pxmin = (int) xmin;
					int pxmax = (int) xmax + xgap;
					int pymin = (int) ymin;
					int pymax = (int) ymax + ygap;
					double ggx4 = 0;
					double ggy4 = 0;
					if (pxmax - pxmin != 0) {
						ggx4 = (double) dwidth / (double) (pxmax - pxmin);
					}
					if (pymax - pymin != 0) {
						ggy4 = (double) dheight / (double) (pymax - pymin);
					}

					drawXY(g2d, 50, 50, dwidth, dheight, "速度-高度曲线（FM文件隐藏面板数据）", "速度", "高度", "km/h", "m", xmin, xmax,
							ymin, ymax, xgap, ygap);
					drawPoint(g2d, 50, 50, dwidth, dheight, ggx4, ggy4, Blkx.loc1.x, Blkx.loc1.y, pxmin, pymin,
							Color.red);
					drawExample(g2d, 50, 60, dheight, Color.red, "WEP速度");
					drawPoint(g2d, 50, 50, dwidth, dheight, ggx4, ggy4, Blkx.loc2.x, Blkx.loc2.y, pxmin, pymin,
							Color.blue);
					drawExample(g2d, 50, 50, dheight, Color.blue, "100%油门速度");

				}
				if (pixIndex == 7) {

					int num = fA.getNoZerosNum(fA.roll_rate);

					double[] iasx = new double[num];
					double[] wx = new double[num];

					fA.removeRollRatesZeroes(iasx, wx);

					double xmin = findMin(iasx);
					double xmax = findMax(iasx);
					// Application.debugPrint(xmin+" "+xmax);
					double ymin = findMin(wx);
					double ymax = findMax(wx);
					int dwidth = 1024;
					int dheight = 576;
					int xgap = Math.round((((int) xmax + 1 - (int) xmin) / 10) / 10.0f) * 10;
					int ygap = 5;
					int pxmin = (int) xmin;
					int pxmax = (int) xmax + xgap;
					int pymin = (int) (ymin / 5) * 5;
					int pymax = (int) (ymax / 5) * 5 + ygap;
					double ggx4 = 0;
					double ggy4 = 0;
					if (pxmax - pxmin != 0) {
						ggx4 = (double) dwidth / (double) (pxmax - pxmin);
					}
					if (pymax - pymin != 0) {
						ggy4 = (double) dheight / (double) (pymax - pymin);
					}

					drawXY(g2d, 50, 50, dwidth, dheight, "示速度-滚转率曲线", "示速度", "滚转率", "km/h", "Deg/s", xmin, xmax, ymin,
							ymax, xgap, ygap);
					drawPoint(g2d, 50, 50, dwidth, dheight, ggx4, ggy4, iasx, wx, pxmin, pymin,
							new Color(0, 0, 0, 250));
					drawExample(g2d, 50, 60, dheight, new Color(0, 0, 0, 250), "滚转率");

				}
				// 绘制点

				// 连接点
			}

		};
		initpanel();
		WebButtonGroup G = createbuttonGroup();

		panel.add(G);
		panel.setLayout(null);
		G.setBounds(840, 700, 300, 50);
		this.add(panel);
		this.setShowMaximizeButton(false);
		this.getWebRootPaneUI().getTitleComponent().getComponent(1)
				.setFont(new Font(Application.defaultFont.getName(), Font.PLAIN, 14));// 设置title字体
		this.getWebRootPaneUI().getWindowButtons().setBorderColor(new Color(0, 0, 0, 0));
		// this.getWebRootPaneUI().getWindowButtons().setButtonsDrawBottom(false);
		this.getWebRootPaneUI().getWindowButtons().setButtonsDrawSides(false, false, false, false);
		this.getWebRootPaneUI().getWindowButtons().getWebButton(0).setTopBgColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().getWindowButtons().getWebButton(0).setBottomBgColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().getWindowButtons().getWebButton(1).setTopBgColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().getWindowButtons().getWebButton(1).setBottomBgColor(new Color(0, 0, 0, 0));
		// setShowWindowButtons(false);

		// setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(2);
		setTitle(fA.type + Lang.dFTitleHZ);
		setAlwaysOnTop(true);

		// setFocusable(false);
		// setFocusableWindowState(false);// 取消窗口焦点
		setVisible(true);

	}

	@Override
	public void run() {
		while (doit) {
			// textArea.setText(xp.fmdata);
			this.getContentPane().repaint();
			try {
				Thread.sleep(1000);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}

		}
		this.dispose();
		System.gc();
	}
}