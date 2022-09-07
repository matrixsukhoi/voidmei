package ui;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.Image;
import java.awt.RenderingHints;
import java.awt.Toolkit;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import java.awt.image.BufferedImage;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;

import prog.app;
import prog.controller;
import prog.otherService;
import prog.service;

public class crosshair2 extends WebFrame implements Runnable {

	/**
	 * 
	 */
	public volatile boolean doit = true;
	Boolean on;
	controller xc;
	WebPanel panel;
	int HUDFontsize;
	int Width;
	int Height;
	int CrossX;
	int CrossY;
	int Vx;
	service xs;
	otherService cs;
	Image A;
	Image C;
	BufferedImage B;
	String crosshairName;
	boolean busetexturecrosshair;
	int isDragging;
	int xx;
	int yy;
	private static final long serialVersionUID = -3898679368097973617L;
	String lineCompass;
	String lineHorizon;
	// String line2;
	// String line3;
	String lines[];
	String NumFont;
	Font drawFont;
	int CrossWidth;
	int CrossWidthVario;
	int pitch;
	int rightDraw;
	public int roundCompass;
	public boolean warnVne;
	/*	
	 * public static Image makeColorTransparent(Image im, final Color color) {
	 * ImageFilter filter = new RGBImageFilter() {
	 * 
	 * // the color we are looking for... Alpha bits are set to opaque public
	 * int markerRGB = color.getRGB() | 0xFF000000;
	 * 
	 * @Override public final int filterRGB(int x, int y, int rgb) { if ((rgb |
	 * 0xFF000000) == markerRGB) { // Mark the alpha bits as zero - transparent
	 * return 0x00FFFFFF & rgb; } else { // nothing to do return rgb; } } };
	 * 
	 * ImageProducer ip = new FilteredImageSource(im.getSource(), filter);
	 * return Toolkit.getDefaultToolkit().createImage(ip); }
	 */
	public void setFrameOpaque() {
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		setShadeWidth(0);
	}

	public void initpanel() {
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
	}

	public void drawCrossair(Graphics2D g, int CrossX, int CrossY, int CrossWidth) {
		int l = 4;

		g.setStroke(new BasicStroke(3));
		g.setColor(new Color(0, 0, 0, 75));

		// 圆圈
		g.drawOval((Width - CrossWidth) / 2, (Height - CrossWidth) / 2, CrossWidth, CrossWidth);
		// 横线1
		g.drawLine(CrossX - CrossWidth * l / 4, CrossY, CrossX - CrossWidth / 4, CrossY);
		// 横线2
		g.drawLine(CrossX + CrossWidth / 4, CrossY, CrossX + CrossWidth * l / 4, CrossY);
		// 竖线1
		g.drawLine(CrossX, CrossY - CrossWidth * l / 4, CrossX, CrossY - CrossWidth / 4);
		// 竖线2
		g.drawLine(CrossX, CrossY + CrossWidth / 4, CrossX, CrossY + CrossWidth * l / 4);

		g.setStroke(new BasicStroke(2));
		g.setColor(new Color(255, 215, 8, 255));
		// 圆圈
		g.drawOval((Width - CrossWidth) / 2, (Height - CrossWidth) / 2, CrossWidth, CrossWidth);
		// 横线1
		g.drawLine(CrossX - CrossWidth * l / 4, CrossY, CrossX - CrossWidth / 4, CrossY);
		// 横线2
		g.drawLine(CrossX + CrossWidth / 4, CrossY, CrossX + CrossWidth * l / 4, CrossY);
		// 竖线1
		g.drawLine(CrossX, CrossY - CrossWidth * l / 4, CrossX, CrossY - CrossWidth / 4);
		// 竖线2
		g.drawLine(CrossX, CrossY + CrossWidth / 4, CrossX, CrossY + CrossWidth * l / 4);
	}

	public int throttley = 0;
	public int OilX = 0;
	public int aoaY = 0;
	public boolean inAction = false;
	private boolean drawHudMach = false;

	public void drawTextseries(Graphics2D g, int x, int y) {
		int n = 0;
		g.setFont(drawFont);

		for (int i = 0; i < 5; i++) {
			g.setColor(app.colorShadeShape);
			g.drawString(lines[i], x + 1, n + y + 1);
			// g.drawString(lines[i], x - 1, n + y - 1);

			g.setColor(app.colorNum);
			if (i == 2 && inAction) {
				g.setColor(app.colorWarning);
			}
			if (i == 0 && warnVne){
				g.setColor(app.colorWarning);
			}
			g.drawString(lines[i], x, n + y);
			n = n + HUDFontsize;
			// i = 0画直线提示
			if (i == 0) {
				g.setColor(app.colorShadeShape);
				g.drawLine((HUDFontsize * 3) >> 1, 2 + y, rightDraw - 2,  2+ y);
				g.setColor(app.colorNum);
				g.drawLine(((HUDFontsize * 3) >> 1) - 1, 1+ y, rightDraw - 3, 1+ y);
			}
			// if( i==1){
			// n = n + HUDFontsize;
			// }
		}

		n += 2;

		g.setColor(app.colorShadeShape);
		g.drawLine(3, n, 3, n - throttley);
		g.setColor(app.colorNum);
		g.drawLine(2, n - 1, 2, n - 1 - throttley);

		// AoA
		g.setColor(app.colorShadeShape);
		g.drawLine(1, n, 1, n - aoaY);
		g.setColor(app.colorWarning);
		g.drawLine(0, n - 1, 0, n - 1 - aoaY);

		// g.setColor(app.lblShadeColorMinor);
		// g.drawLine(0, n-1,0, n-1 - aoaY );
		// g.setColor(app.warning);
		// g.drawLine(1, n, 1, n-aoaY);
		//
		// 燃油量
		// int rightDraw = (int)(HUDFontsize * 4.5f + 1);
		// 引擎健康度
		// g.setColor(app.lblShadeColorMinor);
		// g.drawLine(rightDraw, n, -2 + (rightDraw - OilX), n);
		// g.setColor(app.lblNumColor);
		// g.drawLine(rightDraw - 1, n-1, -2 + (rightDraw - OilX) - 1, n-1);
		//
		// 方位

		g.setColor(app.colorShadeShape);
		g.drawLine(rightDraw, n, rightDraw, n - pitch);
		if (pitch > 0)
			g.setColor(app.colorNum);
		else
			g.setColor(app.colorWarning);

		g.drawLine(rightDraw - 1, n - 1, rightDraw - 1, n - 1 - pitch);

		// 绘制方向
		n += 2;

		// 2倍半径
		int r = roundCompass;

		g.setColor(app.colorShadeShape);
		g.drawLine(2 + r + 1, n + r + 1, 2 + r + compassDx + 1, n + r - compassDy + 1);
		g.drawOval(2 + 1, n + 1, r + r, r + r);
		// 引擎健康度
		g.drawArc(2 + 3, n + 3, r + r - 4, r + r - 4, -180, OilX);

		g.setColor(app.colorNum);
		g.drawLine(2 + r, n + r, 2 + r + compassDx, n + r - compassDy);
		g.drawOval(2, n, r + r, r + r);

		g.setColor(app.colorWarning);
		g.drawArc(2 + 2, n + 2, r + r - 4, r + r - 4, -180, OilX);

		// g.setColor(app.lblShadeColorMinor);
		// g.drawString(lineHorizon, x + 1, n + y + 1);
		//
		// g.setColor(app.lblNumColor);
		// g.drawString(lineHorizon, x, n + y);

		g.setColor(app.colorShadeShape);
		g.drawString(lineCompass, x + 1, n + (r - HUDFontsize / 2) + y + 1);

		g.setColor(app.colorNum);
		g.drawString(lineCompass, x, n + (r - HUDFontsize / 2) + y);

		n = n + roundCompass;

		// g.drawLine(0, Height-1, 0, throttley );
		// g.drawLine(0, 0, 0, Height - 1 - throttley);
	}

	// public void drawTextseries2(Graphics2D g, int x, int y) {
	// int n = 0;
	// g.setFont(drawFont);
	//
	// g.setColor(Color.gray);
	// g.drawString(line3, x + 1, n + y + 1);
	// g.setColor(new Color(255, 215, 8, 100));
	// g.drawString(line3, x, n + y);
	//
	// }

	/*
	 * private static BufferedImage setAlpha(String srcImageFile,int alpha) {
	 * 
	 * try { //读取图片 FileInputStream stream = new FileInputStream(new
	 * File(srcImageFile));// 指定要读取的图片
	 * 
	 * // 定义一个字节数组输出流，用于转换数组 ByteArrayOutputStream os = new
	 * ByteArrayOutputStream();
	 * 
	 * byte[] data =new byte[1024];// 定义一个1K大小的数组 while (stream.read(data) !=
	 * -1) { os.write(data); }
	 * 
	 * ImageIcon imageIcon = new ImageIcon(os.toByteArray()); BufferedImage
	 * bufferedImage = new BufferedImage(imageIcon.getIconWidth(),
	 * imageIcon.getIconHeight(), BufferedImage.TYPE_4BYTE_ABGR); Graphics2D g2D
	 * = (Graphics2D) bufferedImage.getGraphics();
	 * g2D.drawImage(imageIcon.getImage(), 0, 0, imageIcon.getImageObserver());
	 * 
	 * //判读透明度是否越界 if (alpha < 0) { alpha = 0; } else if (alpha > 10) { alpha =
	 * 10; }
	 * 
	 * // 循环每一个像素点，改变像素点的Alpha值 for (int j1 = bufferedImage.getMinY(); j1 <
	 * bufferedImage.getHeight(); j1++) { for (int j2 = bufferedImage.getMinX();
	 * j2 < bufferedImage.getWidth(); j2++) { int rgb = bufferedImage.getRGB(j2,
	 * j1); rgb = ((alpha * 255 / 10) << 24) | (rgb & 0x00ffffff);
	 * bufferedImage.setRGB(j2, j1, rgb); } } g2D.drawImage(bufferedImage, 0, 0,
	 * imageIcon.getImageObserver());
	 * 
	 * // 生成图片为PNG
	 * 
	 * return bufferedImage;
	 * 
	 * } catch (Exception e) { e.printStackTrace(); } return null;
	 * 
	 * }
	 */

	public void initPreview(controller c) {
		init(c, null, null);
		// setShadeWidth(10);
		this.setVisible(false);
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 1));
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 1));
		// setFocusableWindowState(true);
		// setFocusable(true);

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
	}

	public String ILbl = "I";
	public String HLbl = "H";
	public String SLbl = "S";

	public void init(controller c, service s, otherService os) {
		int lx;
		int ly;

		xs = s;
		xc = c;
		Vx = 0;
		cs = os;
		setFrameOpaque();

		if (xc.getconfig("MonoNumFont") != "")
			NumFont = xc.getconfig("MonoNumFont");
		else
			NumFont = app.DefaultNumfontName;
		if (xc.getconfig("crosshairX") != "")
			lx = Integer.parseInt(xc.getconfig("crosshairX"));
		else
			lx = (app.ScreenWidth - Width) / 2;
		if (xc.getconfig("crosshairY") != "")
			ly = Integer.parseInt(xc.getconfig("crosshairY"));
		else
			ly = (app.ScreenHeight - Height) / 2;
		if (xc.getconfig("crosshairScale") != "")
			CrossWidth = Integer.parseInt(xc.getconfig("crosshairScale"));
		else
			CrossWidth = 70;

		Width = CrossWidth * 2;
		Height = (int) (CrossWidth * 2);
		CrossWidthVario = CrossWidth;
		HUDFontsize = CrossWidth / 4;
		roundCompass = (int) (Math.round(HUDFontsize * 0.75f));
		rightDraw = (int) (HUDFontsize * 4.0f);
		if (xc.getconfig("hudMach") != "")
			drawHudMach = Boolean.parseBoolean(c.getconfig("hudMach"));

		if (xc.getconfig("drawHUDtext") != "") {
			on = Boolean.parseBoolean(xc.getconfig("drawHUDtext"));
		} else {
			on = true;
		}
		if (xc.getconfig("crosshairName") != "")
			crosshairName = xc.getconfig("crosshairName");
		else
			crosshairName = "";
		// System.out.println(xc.getconfig("usetexturecrosshair"));
		if (xc.getconfig("usetexturecrosshair") != "")
			busetexturecrosshair = Boolean.parseBoolean(xc.getconfig("usetexturecrosshair"));
		else
			busetexturecrosshair = false;

		if (xc.getconfig("disableHUDSpeedLabel") != "")
			if (Boolean.parseBoolean(c.getconfig("disableHUDSpeedLabel"))) {
				ILbl = " ";
			}
		if (xc.getconfig("disableHUDHeightLabel") != "")
			if (Boolean.parseBoolean(c.getconfig("disableHUDHeightLabel"))) {
				HLbl = " ";
			}
		if (xc.getconfig("disableHUDSEPLabel") != "")
			if (Boolean.parseBoolean(c.getconfig("disableHUDSEPLabel"))) {
				SLbl = " ";
			}
		lines = new String[6];
		// for (int i = 0; i < 6; i++) {
		// lines[i] = new String();
		// }
		// lines[0] = ""
		lines[0] = ILbl + String.format("%5s", "360");
		lines[1] = HLbl + String.format("%5s", "1024");
		lines[3] = SLbl + String.format("%5s", "30");
		lines[4] = "G" + String.format("%5s", "2.0");
		lines[2] = "F" + String.format("%5s", "100");
		lines[2] += "BRK";
		lines[2] += "GEAR";
		// lines[4] = "A" + String.format("%5s", "1.0");
		lineCompass = " " + String.format("%5s", "102");
		lineHorizon = " " + String.format("%5s", "45");
		throttley = 100;
		aoaY = 10;
		// System.out.println(lx);
		// System.out.println(ly);
		A = Toolkit.getDefaultToolkit().createImage("image/gunsight/" + crosshairName + ".png");
		C = A.getScaledInstance(CrossWidth * 2, CrossWidth * 2, Image.SCALE_SMOOTH);
		// B=setAlpha("image/gunsight/" + crosshairName + ".png",200);
		// B.getScaledInstance(CrossWidth * 2, CrossWidth * 2,
		// Image.SCALE_SMOOTH);
		// A=makeColorTransparent(A,new Color(0,0,0));
		this.setBounds(lx, ly, Width, Height);
		drawFont = new Font(NumFont, Font.BOLD, HUDFontsize);
		CrossX = Width / 2;
		CrossY = Height / 2;
		panel = new WebPanel() {

			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw
				g2d.setPaintMode();
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				// if (busetexturecrosshair) {
				// g2d.drawImage(C, CrossX - CrossWidthVario, CrossY -
				// CrossWidthVario, CrossWidthVario * 2, CrossWidthVario * 2,
				// this);
				//
				// } else {
				// drawCrossair(g2d, CrossX, CrossY, CrossWidth);
				// }
				// 显示攻角和水平
				if (on) {
					drawTextseries(g2d, 5, 15);
					// drawTextseries2(g2d, 15, 15);
				}
//				g.dispose();
			}
		};
		initpanel();
		this.add(panel);
		// if (app.debug)setShadeWidth(8);
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setTitle("hud");
		setAlwaysOnTop(true);

		setFocusable(false);
		setFocusableWindowState(false);// 取消窗口焦点

		this.setCursor(app.blankCursor);
		setVisible(true);

		// this.createBufferStrategy(2);

	}

	public long hudCheckMili;
	private int compassDx;
	private int compassDy;

	@Override
	public void run() {
		while (doit) {
			try {
				Thread.sleep(app.threadSleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}

			/*
			 * if(cs.distance>400){ CrossWidthVario=CrossWidth;
			 * 
			 * } else { CrossWidthVario=CrossWidth/2+CrossWidth%2; }
			 * if(cs.distance>150){
			 * CrossWidthVario=(int)(CrossWidth*150/cs.distance); }
			 */
			// line1 = "S"+xs.IAS +" S"+(int)(cs.enemyspeed*3.6);
			// line2 = Math.round(cs.AZI)+"点 T"+(int)cs.AOT;

			// line3 = "υ " + xs.IAS;
			if (xs.SystemTime - hudCheckMili > xc.freqService) {
				hudCheckMili = xs.SystemTime;
				
				this.getContentPane().repaint();
				
				
				
				warnVne = false;

				
				throttley = xs.sState.throttle * HUDFontsize * 5 / 110;

				float compassRads = (float) Math.toRadians(xs.sIndic.compass);
				compassDx = (int) (roundCompass * Math.sin(compassRads));
				compassDy = (int) (roundCompass * Math.cos(compassRads));

				// lineHorizon = " " + String.format("%5s", xs.sPitchUp);
				float p = xs.curLoadMinWorkTime < xs.fueltime ? xs.curLoadMinWorkTime : xs.fueltime;
				OilX = (int) (p * 360 / 600000);
				if (OilX > 360)
					OilX = 360;
				OilX = OilX - 360;
				// OilX = OilX - 180;
				float aviahp = -xs.sIndic.aviahorizon_pitch;
				if (aviahp != 65535)
					pitch = (int) (aviahp * HUDFontsize * 5 / 90.0f);
				else
					pitch = 0;
				
				lineCompass = " " + String.format("%5s", xs.compass);
				if (drawHudMach)
					lines[0] = "M" + String.format("%5s", xs.M);
				else
					lines[0] = ILbl + String.format("%5s", xs.IAS);
				lines[1] = HLbl + String.format("%5s", xs.salt);

				String t;
				if (xs.SEP > 0) {
					t = "+" + xs.sSEP;
				} else
					t = xs.sSEP;
				// if(xs.sState.se)
				lines[3] = SLbl + String.format("%5s", t);

				if (xs.sState.Ny > 1.5f || xs.sState.Ny < -0.5f)
					lines[4] = "G" + String.format("%5s", xs.Ny);
				else {
					// 燃油量和增压器
					String s = xs.sfueltime;
					for (int i = 1; i < xs.sState.compressorstage; i++) {
						s += "C";
					}

					lines[4] = "L" + String.format("%5s", s);
					// if(xs.sState.compressorstage != 0){
					// lines[3] += "S"
					// +String.format("%d",xs.sState.compressorstage);
					// }
					s = null;
				}
				if (xs.sState.flaps > 0) {
					lines[2] = "F" + String.format("%5s", xs.flaps);
				} else {
					if (xs.hasWingSweepVario) {
						lines[2] = "W" + String.format("%5s", xs.sWingSweep);
					} else {
						lines[2] = "";
					}
				}
				inAction = false;
				if (xs.sState.airbrake > 0) {
					lines[2] += "BRK";
					if (xs.sState.airbrake != 100) {
						inAction |= true;
					}
				}

				if (xs.sState.gear > 0) {
					lines[2] += "GEAR";
					if (xs.sState.gear != 100)
						inAction |= true;
				}

				if (xc.blkx != null && xc.blkx.valid) {
					// 速度
					
					if(xs.IASv + 50 > xc.blkx.vne){
						warnVne = true;
					}
					
					float availableAoA;
					int flaps = xs.sState.flaps > 0 ? xs.sState.flaps : 0;
					availableAoA = (xc.blkx.NoFlapsWing.AoACritHigh
							+ (xc.blkx.FullFlapsWing.AoACritHigh - xc.blkx.NoFlapsWing.AoACritHigh) * flaps / 100.0f)
							- xs.sState.AoA;

					aoaY = (int) (availableAoA * HUDFontsize * 5 / xc.blkx.NoFlapsWing.AoACritHigh);
					// if (xc.blkx.NoFlapsWing.AoACritHigh - xs.sState.AoA <
					// 0.33f*xc.blkx.NoFlapsWing.AoACritHigh)
					// lines[4] = "A" + String.format("%5.1f",
					// xc.blkx.NoFlapsWing.AoACritHigh - xs.sState.AoA);
					// else
					// lines[4] = "";
				} else {
					aoaY = (int) (xs.sState.AoA * HUDFontsize * 5 / 30);
				}

			}
		}

	}
}