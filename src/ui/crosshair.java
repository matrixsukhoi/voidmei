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

public class crosshair extends WebFrame implements Runnable {

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
	String line1;
	String line2;
	String line3;
	String NumFont;
	int CrossWidth;
	int CrossWidthVario;
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

	public void drawTextseries(Graphics2D g, int x, int y) {
		if (app.debug) {
			int n = 0;
			g.setFont(new Font(NumFont, Font.PLAIN, HUDFontsize));

			g.setColor(Color.gray);
			g.drawString(line1, x + 1, n + y + 1);
			g.setColor(new Color(255, 215, 8, 100));
			g.drawString(line1, x, n + y);

			n = n + HUDFontsize;

			g.setColor(Color.gray);
			g.drawString(line2, x + 1, n + y + 1);
			g.setColor(new Color(255, 215, 8, 100));
			g.drawString(line2, x, n + y);
		}

	}

	public void drawTextseries2(Graphics2D g, int x, int y) {
		int n = 0;
		g.setFont(new Font(NumFont, Font.PLAIN, 14));

		g.setColor(Color.gray);
		g.drawString(line3, x + 1, n + y + 1);
		g.setColor(new Color(255, 215, 8, 100));
		g.drawString(line3, x, n + y);

	}

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
		setShadeWidth(10);
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
		setVisible(true);
	}

	public void init(controller c, service s, otherService os) {
		int lx;
		int ly;
		
		xs = s;
		xc = c;
		Vx = 0;
		cs = os;
		setFrameOpaque();

		line1 = "a 13.2";
		line2 = "g 6.1";
		line3 = "υ 423";

		if (xc.getconfig("GlobalNumFont") != "")
			NumFont = xc.getconfig("GlobalNumFont");
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
		CrossWidthVario=CrossWidth;
		HUDFontsize = CrossWidth / 5;

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

		Width = CrossWidth * 2;
		Height = CrossWidth * 2;
		// System.out.println(lx);
		// System.out.println(ly);
		A = Toolkit.getDefaultToolkit().createImage("image/gunsight/" + crosshairName + ".png");
		C = A.getScaledInstance(CrossWidth * 2, CrossWidth * 2, Image.SCALE_SMOOTH);
		// B=setAlpha("image/gunsight/" + crosshairName + ".png",200);
		// B.getScaledInstance(CrossWidth * 2, CrossWidth * 2,
		// Image.SCALE_SMOOTH);
		// A=makeColorTransparent(A,new Color(0,0,0));
		this.setBounds(lx, ly, Width, Height);

		CrossX = Width / 2;
		CrossY = Height / 2;
		panel = new WebPanel() {

			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw

				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
				if (busetexturecrosshair) {
					g2d.drawImage(C, CrossX - CrossWidthVario, CrossY - CrossWidthVario, CrossWidthVario * 2, CrossWidthVario * 2, this);

				} else {
					drawCrossair(g2d, CrossX, CrossY, CrossWidth);
				}
				// 显示攻角和水平
				if (on) {
					drawTextseries(g2d, 5, 15);
					// drawTextseries2(g2d, 15, 15);
				}
			}
		};
		initpanel();
		this.add(panel);

		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setTitle("hud");
		setAlwaysOnTop(true);

		setFocusable(false);
		setFocusableWindowState(false);// 取消窗口焦点
		setVisible(true);

	}

	@Override
	public void run() {
		while (doit) {
			try {
				Thread.sleep(200);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			/*
			if(cs.distance>400){
				CrossWidthVario=CrossWidth;
				
			}
			else {
				CrossWidthVario=CrossWidth/2+CrossWidth%2;
			}
			if(cs.distance>150){
				CrossWidthVario=(int)(CrossWidth*150/cs.distance);
			}
			*/
			line1 = "S"+xs.IAS +" S"+(int)(cs.enemyspeed*3.6);
			line2 = Math.round(cs.AZI)+"点 T"+(int)cs.AOT;

			line3 = "υ " + xs.IAS;

			repaint();
		}

	}
}