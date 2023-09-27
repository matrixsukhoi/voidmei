package ui;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;

import java.awt.RenderingHints;
import java.awt.Toolkit;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;

import parser.blkx;
import parser.flightAnalyzer;
import prog.app;
import prog.controller;
import prog.lang;

public class drawFrameSimpl extends WebFrame implements Runnable {
	/**
	 * 
	 */
	public volatile boolean doit = true;
	private static final long serialVersionUID = 6290400898885722422L;
	controller xc;
	flightAnalyzer fA;
	WebPanel panel;
	int pixIndex = 0;
	int Index = 8;
	boolean useBlkx = true;
	double ggx4;
	double ggy4;
	blkx blkx;
	double fY[];
	double fX[];
	void paintAction(Graphics g, blkx fmblk) {
		Graphics2D g2d = (Graphics2D) g;
		// 开始绘图
		// g2d.draw

		g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
		g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);

		// 绘制坐标系
		double[] xn = new double[fmblk.velThrNum];
		for (int i = 0; i < fmblk.velThrNum; i++) {
			xn[i] = fmblk.velocityThr[i];
		}

		double xmin = findMin(xn);
		double xmax = findMax(xn);
		
		// app.debugPrint(xmin+" "+xmax);
		double ymin = findMin(fmblk.maxThrAft[fmblk.altThrNum - 1]);
		double ymax = findMax(fmblk.maxThrAft[0]);
		
//		xmax对齐10
		xmin = (double)(((int)(xmin/10))  *10);
		xmax = (double)((int)(xmax/10)*10);
		ymin = (double)(((int)(ymin/10)) *10);
		ymax = (double)((int)(ymax/10)*10);
		int dwidth = 800;
		int dheight  = 400;
		int xgap = Math.round((((int) xmax + 1 - (int) xmin) / 5) / 5.0f) * 5;
		int ygap = Math.round((((int) ymax + 1 - (int) ymin) / 5) / 5.0f) * 5;
		int pxmin = (int) xmin;
		int pxmax = (int) xmax + xgap;
		int pymin = (int) (ymin / 10) * 10;
		int pymax = (int) (ymax / 10) * 10 + (int) (ygap / 10) * 10;
		double ggx4 = 0;
		double ggy4 = 0;
		if (pxmax - pxmin != 0) {
			ggx4 = (double) dwidth / (double) (pxmax - pxmin);
		}
		if (pymax - pymin != 0) {
			ggy4 = (double) dheight / (double) (pymax - pymin);
		}
		int fontsize = 12;
		int rgbx = (int)(255.0f/(fmblk.altThrNum+1));	
		drawXY(g2d, 50, 50, dwidth, dheight, "推力-真空速曲线", "真空速", "推力", "km/h", "kgf", xmin, xmax, ymin, ymax,
				xgap, ygap, fontsize);
		for (int i = 0; i < fmblk.altThrNum; i++) {
			drawPoint(g2d, 50, 50, dwidth, dheight, ggx4, ggy4, xn, fmblk.maxThrAft[i], pxmin, pymin,
					new Color((i+1) *rgbx , (i+1) *rgbx , (i+1) *rgbx , 250));
			
			drawExample(g2d, dwidth - 40 , 60 + i * fontsize - dheight, dheight, new Color((i+1) *rgbx , (i+1) *rgbx , (i+1) *rgbx , 250),
					String.format("高度%.0fm", fmblk.altitudeThr[i]), fontsize);
		}

		// 绘制点

		// 连接点
	}
	double findMin(double X[]) {
		int i;
		double min = Float.MAX_VALUE;
		for (i = 0; i < X.length; i++) {
			if (X[i] < min)
				min = X[i];
		}
		return min;
	}

	double findMax(double X[]) {
		int i;
		double max = Float.MIN_VALUE;
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
		panel.setBackground(new Color(0,0,0,0));
	}

	void setFrameOpaque() {
		this.getWebRootPaneUI().setMiddleBg(new Color(255, 255, 255, 233));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(255, 255, 255, 233));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(255, 255, 255, 233));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(255, 255, 255, 233));// 外描边透明
		setShadeWidth(0);
	}

	void getdata(String planename) {
		String fmfile;
		String unitSystem;
		int i;
		// 读入fm
		blkx = new blkx("./data/aces/gamedata/flightmodels/" + planename + ".blkx", planename + ".blk");
		fmfile = blkx.getlastone("fmfile");
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
		// app.debugPrint(fmfile);

		// 读入fmfile
		blkx = new blkx("./data/aces/gamedata/flightmodels/fm/" + fmfile + "x", planename + ".blk");
		// app.debugPrint(blkx.data);
		blkx.getAllplotdata();

	}

	void drawXY(Graphics2D g, int x, int y, int dwidth, int dheight, String title, String xName, String yName,
			String xD, String yD, double xmin, double xmax, double ymin, double ymax, int xgap, int ygap, int fontsize) {
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
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, fontsize+6));
		g.drawString(title, x + dwidth / 2, y);
		y = y + 10;// 往下推10
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, fontsize));

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
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, fontsize+4));
		g.drawString(xD, x + dwidth + 5, y + dheight);
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, fontsize));

		g.setStroke(new BasicStroke(3));
		// y轴与箭头
		g.drawLine(x, y + dheight, x, y);
		// y轴刻度
		ii = (int) ((pymax - pymin) / intervalY);
		for (; ii >= 0; ii--) {
			g.setStroke(new BasicStroke(1));
			g.drawLine(x, (int) (y + dheight - ii * intervalY * ggy), x + dwidth,
					(int) (y + dheight - ii * intervalY * ggy));

			g.drawString(String.valueOf(pymin + ii * intervalY), x - 40, (int) (y + dheight - ii * intervalY * ggy));
		}
		// y轴单位
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, fontsize+4));
		g.drawString(yD, x - 5, y - 10);

	}

	void drawPoint(Graphics2D g, int x, int y, int dwidth, int dheight, double ggx, double ggy, double ix[], double iy[],
			int pxmin, int pymin, Color C) {
		g.setStroke(new BasicStroke(1));
		g.setColor(C);
		y = y + 10;// 往下推10
		// 绘点
		int ii = 0;
		for (ii = 0; ii < ix.length; ii++) {
			// app.debugPrint((y + dheight) +" "+(y + dheight
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

	void drawExample(Graphics2D g, int x, int y, int dheight, Color C, String name, int fontsize) {
		g.setStroke(new BasicStroke(1));
		g.setColor(C);
		g.drawLine(x, y + dheight + 40, x + 20, y + dheight + 40);
		g.setColor(new Color(0, 0, 0, 250));
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, fontsize));
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
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 16));
		g.drawString(title, x + dwidth / 2, y);
		y = y + 10;// 往下推10
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 10));
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
			// app.debugPrint("X坐标"+(pmin + ii * intervalX)+"位置"+(int)
			// (movex + x + ii * intervalX * ggx));
			g.drawString(String.valueOf(pmin + ii * intervalX), (int) (movex + x + ii * intervalX * ggx),
					y + dheight + 15);
		}
		// x轴单位
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 14));
		g.drawString(xD, x + dwidth + 5, y + dheight);
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 10));

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
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 14));
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
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 16));
		g.drawString(title, x + dwidth / 2, y);
		y = y + 10;// 往下推10

		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 10));
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
			// app.debugPrint("X坐标"+(pmin + ii * intervalX)+"位置"+(int)
			// (movex + x + ii * intervalX * ggx));
			g.drawString(String.valueOf(pmin + ii * intervalX), (int) (movex + x + ii * intervalX * ggx),
					y + dheight + 15);
		}
		// x轴单位
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 14));
		g.drawString(xD, x + dwidth + 5, y + dheight);
		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 10));

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

		g.setFont(new Font(app.defaultFontName, Font.PLAIN, 14));
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

	public void init(controller c) {
		// 特殊处理
		xc = c;
		blkx = xc.blkx;
		// 获得x和y

		setFrameOpaque();

		this.setBounds(0, Toolkit.getDefaultToolkit().getScreenSize().height - 500, 900, 500);

		panel = new WebPanel() {

			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw

				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);

				// 绘制坐标系
				double[] xn = new double[blkx.velThrNum];
				for (int i = 0; i < blkx.velThrNum; i++) {
					xn[i] = blkx.velocityThr[i];
				}

				double xmin = findMin(xn);
				double xmax = findMax(xn);
				
				// app.debugPrint(xmin+" "+xmax);
				double ymin = findMin(blkx.maxThrAft[blkx.altThrNum - 1]);
				double ymax = findMax(blkx.maxThrAft[0]);
				
//				xmax对齐10
				xmin = (double)(((int)(xmin/10))  *10);
				xmax = (double)((int)(xmax/10)*10);
				ymin = (double)(((int)(ymin/10)) *10);
				ymax = (double)((int)(ymax/10)*10);
				int dwidth = 800;
				int dheight  = 400;
				int xgap = Math.round((((int) xmax + 1 - (int) xmin) / 5) / 5.0f) * 5;
				int ygap = Math.round((((int) ymax + 1 - (int) ymin) / 5) / 5.0f) * 5;
				int pxmin = (int) xmin;
				int pxmax = (int) xmax + xgap;
				int pymin = (int) (ymin / 10) * 10;
				int pymax = (int) (ymax / 10) * 10 + (int) (ygap / 10) * 10;
				double ggx4 = 0;
				double ggy4 = 0;
				if (pxmax - pxmin != 0) {
					ggx4 = (double) dwidth / (double) (pxmax - pxmin);
				}
				if (pymax - pymin != 0) {
					ggy4 = (double) dheight / (double) (pymax - pymin);
				}
				int fontsize = 12;
				int rgbx = (int)(255.0f/(blkx.altThrNum+1));	
				drawXY(g2d, 50, 50, dwidth, dheight, "推力-真空速曲线", "真空速", "推力", "km/h", "kgf", xmin, xmax, ymin, ymax,
						xgap, ygap, fontsize);
				for (int i = 0; i < blkx.altThrNum; i++) {
					drawPoint(g2d, 50, 50, dwidth, dheight, ggx4, ggy4, xn, blkx.maxThrAft[i], pxmin, pymin,
							new Color((i+1) *rgbx , (i+1) *rgbx , (i+1) *rgbx , 250));
					
					drawExample(g2d, dwidth - 40 , 60 + i * fontsize - dheight, dheight, new Color((i+1) *rgbx , (i+1) *rgbx , (i+1) *rgbx , 250),
							String.format("高度%.0fm", blkx.altitudeThr[i]), fontsize);
				}

				// 绘制点

				// 连接点
			}

		};
		initpanel();
		panel.setLayout(null);
		this.add(panel);
		this.setShowMaximizeButton(false);
		setShowWindowButtons(false);

		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(2);
		setTitle(lang.dFTitleHZ);
		setAlwaysOnTop(true);
		
		this.setCursor(app.blankCursor);
		setFocusable(false);
		setFocusableWindowState(false);// 取消窗口焦点
		setVisible(true);
		
	}

	@Override
	public void run() {
		while (doit) {
			repaint();
			try {
				Thread.sleep(1000);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}	
			if (this.xc.S.sState.gear != 100 || (this.xc.S.speedv > 10 && this.xc.S.sState.throttle > 0)) {
				// 如果收起落架则关闭break
				try {
					Thread.sleep(10000);
				} catch (InterruptedException e) {
					// TODO Auto-generated catch block
					e.printStackTrace();
				}	
				break;
			}
		}
		this.dispose();

		System.gc();
	}
}