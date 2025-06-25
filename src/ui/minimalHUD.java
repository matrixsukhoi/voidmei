package ui;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.Image;
import java.awt.RenderingHints;
import java.awt.Toolkit;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import java.awt.geom.AffineTransform;
import java.awt.image.BufferedImage;

import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;

import parser.mapInfo;
import prog.app;
import prog.controller;
import prog.otherService;
import prog.service;

public class minimalHUD extends WebFrame implements Runnable {

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
	int AoAFuselagePix;
	int Vx;
	int compass;
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
	public boolean drawAttitude;
	int blinkTicks = 1;
	int blinkCheckTicks = 0;
	public boolean warnRH;

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

	public Boolean blinkX = false;
	public Boolean blinkActing = false;

	public void drawBlinkX(Graphics2D g) {
		// 高度警告标记
		if (blinkX) {
			if (!blinkActing) {
				//
				g.setStroke(new BasicStroke(5));
				g.setColor(app.colorShadeShape);

				g.drawLine(2, 2, Width - 2, Height - 2);
				g.drawLine(Width - 2, 2, 2, Height - 2);
				g.setStroke(new BasicStroke(3));
				g.setColor(app.colorNum);
				g.drawLine(1, 1, Width - 1, Height - 1);
				g.drawLine(Width - 1, 1, 1, Height - 1);

			}
			blinkCheckTicks += 1;
			if (blinkCheckTicks % blinkTicks == 0) {
				// app.debugPrint(blinkTicks +"?" + blinkCheckTicks);
				blinkActing = !blinkActing;
			}
		}
	}

	public void drawAttitude(Graphics2D g, int dx, int dy, int CrossWidth, double realSpdPitch, double rad) {
		/* 圆圈 */
		/* 2直线 */
		/* 1直线 */

		/* 旋转 */
	}

	public void drawCrossair(Graphics2D g, int dx, int dy, int CrossX, int CrossY, int CrossWidth) {
		int l = 4;

		g.setStroke(new BasicStroke(3));
		g.setColor(new Color(0, 0, 0, 75));

		// 圆圈
		g.drawOval((dx - CrossWidth) / 2, (dy - CrossWidth) / 2, CrossWidth, CrossWidth);
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
		g.drawOval((dx - CrossWidth) / 2, (dy - CrossWidth) / 2, CrossWidth, CrossWidth);
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
	public Color throttleColor;
	public Color aoaColor;

	public Color aoaBarColor;
	public int throttleLineWidth = 1;

	public void drawTextseries(Graphics2D g, int x, int y) {
		int n = 0;
		g.setFont(drawFont);
		int kx = 0;
		// drawStringShade(g, x, n+y, lines[0], drawFont);
		// BasicStroke outBs = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND,
		// BasicStroke.JOIN_ROUND);
		// BasicStroke inBs = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND,
		// BasicStroke.JOIN_ROUND);
		// 油门
		// if(throttley >= throttleh) {
		// g.setColor(app.colorShadeShape);
		// g.setStroke(Bs3);
		// g.drawLine(kx + barWidth, n5 - throttleh + lineWidth + 2 , kx + barWidth, n5
		// - throttleh + lineWidth + 2);
		// g.setColor(app.colorNum);
		// g.setStroke(Bs1);
		// g.drawLine(kx + barWidth, n5 - throttleh + lineWidth + 2 , kx + barWidth, n5
		// - throttleh + lineWidth + 2);
		// }
		// if(throttley >= throttlec) {
		// g.setColor(app.colorShadeShape);
		// g.setStroke(Bs3);
		// g.drawLine(kx + barWidth, n5 - throttlec + lineWidth + 2 , (int)(kx + 1.25 *
		// barWidth), n5 - throttlec + lineWidth + 2);
		// g.setColor(app.colorNum);
		// g.setStroke(Bs1);
		// g.drawLine(kx + barWidth, n5 - throttlec + lineWidth + 2 , (int)(kx + 1.25 *
		// barWidth), n5 - throttlec + lineWidth + 2);
		// }
		// if(throttley >= throttlem) {
		// g.setColor(app.colorShadeShape);
		// g.setStroke(Bs3);
		// g.drawLine(kx + barWidth, n5 - throttlem + lineWidth + 2 , (int)(kx + 1.5 *
		// barWidth), n5 - throttlem + lineWidth + 2);
		// g.setColor(app.colorNum);
		// g.setStroke(Bs1);
		// g.drawLine(kx + barWidth, n5 - throttlem + lineWidth + 2 , (int)(kx + 1.5 *
		// barWidth), n5 - throttlem + lineWidth + 2);
		// }
		// if(throttley >= throttlew) {
		// g.setColor(app.colorShadeShape);
		// g.setStroke(Bs3);
		// g.drawLine(kx + barWidth, n5 - throttlew + lineWidth + 2 , kx + barWidth +
		// barWidth, n5 - throttlew + lineWidth + 2);
		// g.setColor(app.colorNum);
		// g.setStroke(Bs1);
		// g.drawLine(kx + barWidth, n5 - throttlew + lineWidth + 2 , kx + barWidth +
		// barWidth, n5 - throttlew + lineWidth + 2);
		// }
		// uiBaseElem.drawVRect(g, kx, n5 + lineWidth + 2 , barWidth, throttley, 1,
		// throttleColor);
		uiBaseElem.drawVBarTextNumLeft(g, kx + barWidth, n5 + lineWidth + 2, barWidth, throttley_max, throttley, 1,
				app.colorNum, throttleColor, "",
				lineThrottle, drawFontSSmall, drawFontSSmall);

		x += barWidth + 3 * drawFontSSmall.getSize() / 2;
		kx += barWidth + 3 * drawFontSSmall.getSize() / 2;
		// 姿态

		if (drawAttitude && !disableAttitude) {
			if (!blinkX || (blinkX && !blinkActing)) {
				// 计算小圆形的位置

				int circleX = lineWidth - aosX + round;
				int circleY = (int) (n5 - 2.5 * HUDFontsize + rnd / 2 - pitch);
				double rollDegRad = Math.toRadians(rollDeg);
				// 绘制地面和牵引线
				g.setStroke(Bs3);
				g.setColor(app.colorShadeShape);
				g.drawLine(circleX + aosX, circleY + pitch, circleX, circleY);
				g.setStroke(Bs1);
				g.setColor(app.colorLabel);
				g.drawLine(circleX + aosX, circleY + pitch, circleX, circleY);

				// 旋转整个组合图形表示横滚角
				AffineTransform oldTransform = g.getTransform();
				AffineTransform transform = AffineTransform.getRotateInstance(-rollDegRad, circleX, circleY);

				g.setTransform(transform);

				g.setStroke(new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
				g.setColor(app.colorShadeShape);

				int hbs = halfLine;
				// 画三条支线表示水平和垂直方向

				g.drawArc(circleX - hrnd + hbs, circleY - hrnd + hbs, rnd, rnd, -180, 180);
				g.drawLine(circleX + hbs, circleY - hrnd / 2 + hbs, circleX + hbs, circleY - urnd + hbs);
				g.drawLine(circleX + hrnd + hbs, circleY + hbs, circleX + urnd + hbs, circleY + hbs);
				g.drawLine(circleX - hrnd + hbs, circleY + hbs, circleX - urnd + hbs, circleY + hbs);

				g.setStroke(new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
				g.setColor(app.colorNum);

				// 画三条支线表示水平和垂直方向
				g.drawArc(circleX - hrnd + hbs, circleY - hrnd + hbs, rnd, rnd, -180, 180);
				g.drawLine(circleX + hbs, circleY - hrnd / 2 + hbs, circleX + hbs, circleY - urnd + hbs);
				g.drawLine(circleX + hrnd + hbs, circleY + hbs, circleX + urnd + hbs, circleY + hbs);
				g.drawLine(circleX - hrnd + hbs, circleY + hbs, circleX - urnd + hbs, circleY + hbs);

				g.setTransform(oldTransform);

				// 画文字
				if (roundHorizon >= 0) {
					uiBaseElem.__drawStringShade(g, circleX, circleY - 1, 1, sAttitude, drawFontSmall, app.colorNum);
				} else {
					uiBaseElem.__drawStringShade(g, circleX, circleY - 1, 1, sAttitude, drawFontSmall, app.colorUnit);
				}
				// uiBaseElem.__drawStringShade(g, circleX - HUDFontsize / 2, circleY + 3 *
				// HUDFontsize / 2, 1, sAttitudeRoll, drawFontSmall, app.colorNum);

				// 恢复原始的图形变换
				// g.setTransform(oldTransform);
			}
		}

		for (int i = 0; i < 5; i++) {

			// i = 0画直线提示
			if (i == 0) {

				int liney = 1 + y;

				// availableAoA
				uiBaseElem.drawHRect(g, x + (rightDraw - aoaY), liney, aoaY, lineWidth + 3, 1, aoaBarColor);
				uiBaseElem.__drawStringShade(g, x + rightDraw, liney - 1, 1, lineAoA, drawFontSmall, aoaColor);

			}
			if (i == 1) {
				int liney = 1 + y;
				uiBaseElem.__drawStringShade(g, x + rightDraw, n + liney - 1, 1, relEnergy, drawFontSmall, aoaColor);

			}
			if ((i == 2 && inAction) || (i == 0 && warnVne) || (i == 1 && warnRH)) {
				uiBaseElem.__drawStringShade(g, x, n + y, 1, lines[i], drawFont, app.colorWarning);
			} else
				uiBaseElem.__drawStringShade(g, x, n + y, 1, lines[i], drawFont, app.colorNum);

			if (i == 4) {
				// 机动性指标线

				g.setColor(app.colorShadeShape);
				g.setStroke(Bs3);
				g.drawLine(x + rightDraw - maneuverIndexLen10, n + y + halfLine + lineWidth + lineWidth,
						x + rightDraw - maneuverIndexLen10, n + y + halfLine - lineWidth + lineWidth);
				g.setColor(app.colorNum);
				g.setStroke(Bs1);
				g.drawLine(x + rightDraw - maneuverIndexLen10, n + y + halfLine + lineWidth + lineWidth,
						x + rightDraw - maneuverIndexLen10, n + y + halfLine - lineWidth + lineWidth);

				if (maneuverIndex >= 0.1) {
					g.setColor(app.colorShadeShape);
					g.setStroke(Bs3);
					g.drawLine(x + rightDraw - maneuverIndexLen20, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen20, n + y + halfLine - lineWidth + lineWidth);
					g.setColor(app.colorNum);
					g.setStroke(Bs1);
					g.drawLine(x + rightDraw - maneuverIndexLen20, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen20, n + y + halfLine - lineWidth + lineWidth);
				}
				if (maneuverIndex >= 0.2) {
					g.setColor(app.colorShadeShape);
					g.setStroke(Bs3);
					g.drawLine(x + rightDraw - maneuverIndexLen30, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen30, n + y + halfLine - lineWidth + lineWidth);
					g.setColor(app.colorNum);
					g.setStroke(Bs1);
					g.drawLine(x + rightDraw - maneuverIndexLen30, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen30, n + y + halfLine - lineWidth + lineWidth);
				}
				if (maneuverIndex >= 0.3) {
					g.setColor(app.colorShadeShape);
					g.setStroke(Bs3);
					g.drawLine(x + rightDraw - maneuverIndexLen40, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen40, n + y + halfLine - lineWidth + lineWidth);
					g.setColor(app.colorNum);
					g.setStroke(Bs1);
					g.drawLine(x + rightDraw - maneuverIndexLen40, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen40, n + y + halfLine - lineWidth + lineWidth);
				}
				if (maneuverIndex >= 0.4) {
					g.setColor(app.colorShadeShape);
					g.setStroke(Bs3);
					g.drawLine(x + rightDraw - maneuverIndexLen50, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen50, n + y + halfLine - lineWidth + lineWidth);
					g.setColor(app.colorNum);
					g.setStroke(Bs1);
					g.drawLine(x + rightDraw - maneuverIndexLen50, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen50, n + y + halfLine - lineWidth + lineWidth);
				}
				g.setStroke(Bs3);
				g.setColor(app.colorShadeShape);
				g.drawLine(x + rightDraw, n + y + halfLine + lineWidth, x + rightDraw - maneuverIndexLen,
						n + y + halfLine + lineWidth);

				g.setStroke(Bs1);
				g.setColor(app.colorNum);
				g.drawLine(x + rightDraw, n + y + halfLine + lineWidth, x + rightDraw - maneuverIndexLen,
						n + y + halfLine + lineWidth);

			}

			n = n + HUDFontsize;

		}
		n += 2;

		// g.setColor(app.lblShadeColorMinor);
		// g.drawLine(3, n, 3, n - throttley);
		// g.setColor(app.lblNumColor);
		// g.drawLine(2, n - 1, 2, n - 1 - throttley);

		// uiBaseElem.drawVRect(g, kx, n, barWidth, aoaY, 1, app.warning);
		// uiBaseElem.drawVBar(g, kx, n, barWidth, AoAFuselagePix, aoaY, 1,
		// app.warning);
		// kx += barWidth;
		//

		// uiBaseElem.drawVBarTextNum(g, kx, n, barWidth, HUDFontsize * 5,
		// throttley, 1, app.lblNumColor, "", "T"+throttley, drawFont,
		// drawFont);
		// uiBaseElem.drawVBar(g, kx, n, barWidth, HUDFontsize * 5, throttley,
		// 1, app.lblNumColor);
		if (!drawAttitude) {
			/*
			 * // if (pitch > 0) { // uiBaseElem.drawVRect(g, kx, n, barWidth,
			 * pitch, 1, // app.colorWarning); // } else { //
			 * uiBaseElem.drawVRect(g, kx, n, barWidth, pitch, 1, app.colorNum);
			 * // } // kx += barWidth;
			 */ }

		// uiBaseElem.drawVBar(g, x + rightDraw, n, barWidth, AoAFuselagePix,
		// aoaY, 1, app.warning);
		// AoA
		// g.setColor(app.lblShadeColorMinor);
		// g.drawLine(1, n, 1, n - aoaY);
		// g.setColor(app.warning);
		// g.drawLine(0, n - 1, 0, n - 1 - aoaY);

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

		// g.setColor(app.lblShadeColorMinor);
		// g.drawLine(rightDraw, n, rightDraw, n - pitch);
		// if (pitch > 0)
		// g.setColor(app.lblNumColor);
		// else
		// g.setColor(app.warning);
		// if (pitch > 0){
		// uiBaseElem.drawVRect(g, rightDraw, n, 5, pitch, 1, app.lblNumColor);
		// }
		// else{
		// uiBaseElem.drawVRect(g, rightDraw, n, 5, pitch, 1, app.warning);
		// }

		// g.drawLine(rightDraw - 1, n - 1, rightDraw - 1, n - 1 - pitch);

		// 画一个半圆

		// 绘制方向
		// n += 2;
		// 2倍半径
		int r = roundCompass;
		n -= 2 * HUDFontsize - 2;
		kx += rightDraw + r;

		g.setStroke(outBs);
		g.setColor(app.colorShadeShape);

		g.drawLine(kx + r + (int) (0.618 * r * Math.sin(compassRads)),
				n + r - (int) (0.618 * r * Math.cos(compassRads)), kx + r + compassDx, n + r - compassDy);
		g.drawOval(kx, n, r + r, r + r);
		// g.drawArc(kx, n, r + r, r + r, compass - 5, compass + 365 );

		// 引擎健康度
		// g.drawArc(2 + 3, n + 3, r + r - 4, r + r - 4, -180, OilX);

		g.setStroke(inBs);
		g.setColor(app.colorNum);
		// g.drawLine(kx + r + r * compassRads, n + r, kx + r + compassDx, n + r -
		// compassDy);
		g.drawLine(kx + r + (int) (0.618 * r * Math.sin(compassRads)),
				n + r - (int) (0.618 * r * Math.cos(compassRads)), kx + r + compassDx, n + r - compassDy);
		g.drawOval(kx, n, r + r, r + r);
		// g.drawArc(kx, n, r + r, r + r, compass - 5, compass + 365);

		// g.setColor(app.warning);
		// g.drawArc(2 + 2, n + 2, r + r - 4, r + r - 4, -180, OilX);

		// g.setColor(app.lblShadeColorMinor);
		// g.drawString(lineHorizon, x + 1, n + y + 1);
		//
		// g.setColor(app.lblNumColor);
		// g.drawString(lineHorizon, x, n + y);

		// g.setColor(app.lblShadeColorMinor);
		// g.drawString(lineCompass, x + 1, n + (r - HUDFontsize / 2) + y + 1);
		//
		// g.setColor(app.lblNumColor);
		// g.drawString(lineCompass, x, n + (r - HUDFontsize / 2) + y);
		uiBaseElem.drawStringShade(g, kx + lineWidth + 3, n + y - (r - HUDFontsize) / 2, 1, lineCompass, drawFontSmall);
		uiBaseElem.drawStringShade(g, kx + lineWidth + 3, n + y + r + HUDFontSizeSmall / 2, 1, lineLoc, drawFontSmall);

		// uiBaseElem.drawStringShade(g, kx + x, n + (r - HUDFontsize / 2) + y, 1,
		// lineCompass, drawFont);
		n = n + 3 * roundCompass;

		drawBlinkX(g);
		// drawLabelBOSType(g, 0, n, 1, drawFont, drawFont, drawFont, "500",
		// "IAS", "km/h");
		// drawLabelBOSType(g, 0, n, 1,)
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
		this.getWebRootPaneUI().setTopBg(app.previewColor);
		this.getWebRootPaneUI().setMiddleBg(app.previewColor);
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
	private boolean crossOn;
	private int barWidth;
	private int lineWidth;
	private double aoaLength;
	private Font drawFontSmall;
	private boolean disableAoA;
	private Container root;
	private int rollDeg;
	private double aoaWarningRatio;
	private double aoaBarWarningRatio;
	private int HUDFontSizeSmall;
	private String relEnergy;
	private BasicStroke outBs;
	private BasicStroke inBs;
	private BasicStroke Bs3;
	private BasicStroke Bs1;
	private int halfLine;
	private int n5;
	private int round;
	private int rnd;
	private int hrnd;
	private String lineLoc;
	private int urnd;
	private Font drawFontSSmall;

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
			NumFont = app.defaultNumfontName;
		if (xc.getconfig("crosshairX") != "")
			lx = Integer.parseInt(xc.getconfig("crosshairX"));
		else
			lx = (app.screenWidth - Width) / 2;
		if (xc.getconfig("crosshairY") != "")
			ly = Integer.parseInt(xc.getconfig("crosshairY"));
		else
			ly = (app.screenHeight - Height) / 2;
		if (xc.getconfig("crosshairScale") != "")
			CrossWidth = Integer.parseInt(xc.getconfig("crosshairScale"));
		else
			CrossWidth = 70;
		if (CrossWidth == 0)
			CrossWidth = 1;
		if (xc.getconfig("crosshairName") != "")
			crosshairName = xc.getconfig("crosshairName");
		else
			crosshairName = "";
		// app.debugPrint(xc.getconfig("usetexturecrosshair"));
		if (xc.getconfig("displayCrosshair") != "") {
			crossOn = Boolean.parseBoolean(xc.getconfig("displayCrosshair"));

		} else {
			crossOn = false;
		}

		if (xc.getconfig("usetexturecrosshair") != "")
			busetexturecrosshair = Boolean.parseBoolean(xc.getconfig("usetexturecrosshair"));
		else
			busetexturecrosshair = false;

		if (xc.getconfig("drawHUDtext") != "") {
			on = Boolean.parseBoolean(xc.getconfig("drawHUDtext"));

		} else {
			on = true;
		}
		if (xc.getconfig("drawHUDAttitude") != "") {
			drawAttitude = Boolean.parseBoolean(xc.getconfig("drawHUDAttitude"));
		} else {
			drawAttitude = true;
		}

		if (xc.getconfig("miniHUDaoaWarningRatio") != "") {
			aoaWarningRatio = Double.parseDouble(xc.getconfig("miniHUDaoaWarningRatio"));
		} else {
			aoaWarningRatio = 0.25;
		}

		if (xc.getconfig("miniHUDaoaBarWarningRatio") != "") {
			aoaBarWarningRatio = Double.parseDouble(xc.getconfig("miniHUDaoaBarWarningRatio"));
		} else {
			aoaBarWarningRatio = 0;
		}

		HUDFontsize = CrossWidth / 4;
		barWidth = HUDFontsize / 4;
		lineWidth = HUDFontsize / 10;
		if (!crossOn)
			Width = (int) (CrossWidth * 2.25) - HUDFontsize;
		else {
			Width = (int) (CrossWidth * 2.25);
		}
		Height = (int) (CrossWidth * 1.5);
		CrossWidthVario = CrossWidth;
		if (lineWidth == 0)
			lineWidth = 1;
		roundCompass = (int) (Math.round(HUDFontsize * 0.8f));
		rightDraw = (int) (HUDFontsize * 3.5f);

		n5 = 5 * HUDFontsize;
		round = (int) (5 * roundCompass);
		rnd = (int) Math.round(2 * HUDFontsize * 0.618);
		hrnd = (int) Math.round(rnd / 2.0);
		urnd = (int) Math.round(0.618 * rnd);

		if (xc.getconfig("hudMach") != "")
			drawHudMach = Boolean.parseBoolean(c.getconfig("hudMach"));

		if (xc.getconfig("disableHUDSpeedLabel") != "")
			if (Boolean.parseBoolean(c.getconfig("disableHUDSpeedLabel"))) {
				ILbl = "";
			}
		if (xc.getconfig("disableHUDHeightLabel") != "")
			if (Boolean.parseBoolean(c.getconfig("disableHUDHeightLabel"))) {
				HLbl = "";
			}
		if (xc.getconfig("disableHUDSEPLabel") != "")
			if (Boolean.parseBoolean(c.getconfig("disableHUDSEPLabel"))) {
				SLbl = "";
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
		lines[2] = "F" + String.format("%3s", "100");
		lines[2] += "BRK";
		lines[2] += "GEAR";
		// lines[4] = "A" + String.format("%5s", "1.0");
		lineCompass = String.format("%3s", "102");
		lineLoc = "A1";
		lineHorizon = String.format("%3s", "45");
		throttley = 100;
		throttley_max = (int) (HUDFontsize * 4.75);
		lineThrottle = "100";
		aoaY = 10;
		disableAoA = false;
		throttleColor = app.colorShadeShape;
		lineAoA = String.format("α%3.0f", 20.0);
		relEnergy = "E114514";
		sAttitude = "";
		sAttitudeRoll = "";
		if (xc.getconfig("disableHUDAoA") != "") {
			if (Boolean.parseBoolean(c.getconfig("disableHUDAoA"))) {
				lineAoA = "";
				disableAoA = true;
				relEnergy = "";
			}
		}
		aosX = 0;
		rollDeg = 0;
		aoaLength = rightDraw - HUDFontsize / 2;
		if (aoaY > rightDraw)
			aoaY = rightDraw;
		aoaColor = app.colorNum;
		aoaBarColor = app.colorNum;

		throttlew = (110 * HUDFontsize * 5) / 110;
		throttlem = (100 * HUDFontsize * 5) / 110;
		throttlec = (80 * HUDFontsize * 5) / 110;
		throttleh = (50 * HUDFontsize * 5) / 110;

		outBs = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		inBs = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		halfLine = (lineWidth / 2 == 0) ? 1 : (int) Math.round(lineWidth / 2.0f);
		Bs3 = new BasicStroke(halfLine + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		Bs1 = new BasicStroke(halfLine, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		// app.debugPrint(lx);
		// app.debugPrint(ly);
		A = Toolkit.getDefaultToolkit().createImage("image/gunsight/" + crosshairName + ".png");
		C = A.getScaledInstance(CrossWidth * 2, CrossWidth * 2, Image.SCALE_SMOOTH);
		// B=setAlpha("image/gunsight/" + crosshairName + ".png",200);
		// B.getScaledInstance(CrossWidth * 2, CrossWidth * 2,
		// Image.SCALE_SMOOTH);
		// A=makeColorTransparent(A,new Color(0,0,0));
		if (crossOn)
			this.setBounds(lx, ly, Width * 2, Height);
		else
			this.setBounds(lx, ly, Width, Height);
		drawFont = new Font(NumFont, Font.BOLD, HUDFontsize);
		HUDFontSizeSmall = (int) (HUDFontsize * 0.75f);
		drawFontSmall = new Font(NumFont, Font.BOLD, HUDFontSizeSmall);

		drawFontSSmall = new Font(NumFont, Font.BOLD, HUDFontsize / 2);
		CrossX = Width / 2;
		CrossY = Height / 2;
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
					drawTextseries(g2d, HUDFontsize / 2, HUDFontsize);

				}
				if (crossOn) {
					if (busetexturecrosshair) {
						g2d.drawImage(C, Width + CrossX - CrossWidthVario, CrossY - CrossWidthVario,
								CrossWidthVario * 2, CrossWidthVario * 2, this);
					} else {
						drawCrossair(g2d, 2 * Width, 1 * Height, Width + CrossX, CrossY, CrossWidth);
					}
				}
				// g.dispose();
			}
		};
		// initpanel();

		// RepaintManager rm = RepaintManager.currentManager(this);
		// boolean b = rm.isDoubleBufferingEnabled();
		// rm.setDoubleBufferingEnabled(false);
		//
		this.add(panel);
		// if (app.debug)setShadeWidth(8);
		// setShowWindowButtons(false);
		// setShowTitleComponent(false);
		// setShowResizeCorner(false);
		// setDefaultCloseOperation(3);
		// setTitle("hud");
		// setAlwaysOnTop(true);
		// root = this.getContentPane();
		// setFocusable(false);
		// setFocusableWindowState(false);// 取消窗口焦点
		// this.setCursor(app.blankCursor);
		// setVisible(true);
		// 1miao 8 ci
		blinkTicks = (int) ((1000 / xc.freqService) >> 3);
		if (blinkTicks == 0)
			blinkTicks = 1;
		
		setTitle("miniHUD");
		uiWebLafSetting.setWindowOpaque(this);
		root = this.getContentPane();
		// this.createBufferStrategy(2);

	}

	public long hudCheckMili;
	private int compassDx;
	private int compassDy;
	private double availableAoA;
	private String lineAoA;
	private int aosX;

	double realSpdPitch;
	private int throttlem;
	private int throttlec;
	private int throttleh;
	private String sAttitude;
	private String sAttitudeRoll;
	private double compassRads;
	private int roundHorizon;
	private int throttlew;
	private double maneuverIndex;
	private int maneuverIndexLen;
	private int maneuverIndexLen15;
	private int maneuverIndexLen30;
	private int maneuverIndexLen10;
	private int maneuverIndexLen20;
	private int maneuverIndexLen40;
	private int maneuverIndexLen50;
	private int throttley_max;
	private String lineThrottle;
	private boolean disableAttitude;

	public void updateString() {

		warnVne = false;
		warnRH = false;
		blinkX = xs.fatalWarn;
		int throttle = xs.sState.throttle;
		if (throttle > 101) {
			throttleColor = app.colorWarning;
		} else {
			throttleColor = app.colorNum;
		}
		throttley = throttle * throttley_max / 110;

		compass = (int) xs.dCompass;
		compassRads = (double) Math.toRadians(xs.dCompass);

		// double compassRads = (double) Math.toRadians(xs.sIndic.compass);

		compassDx = (int) ((roundCompass * 1.3f) * Math.sin(compassRads));
		compassDy = (int) ((roundCompass * 1.3f) * Math.cos(compassRads));
		double aoa = xs.sState.AoA;
		// lineHorizon = " " + String.format("%5s", xs.sPitchUp);
		double p = xs.curLoadMinWorkTime < xs.fueltime ? xs.curLoadMinWorkTime : xs.fueltime;
		OilX = (int) (p * 360 / 600000);
		if (OilX > 360)
			OilX = 360;
		OilX = OilX - 360;
		// OilX = OilX - 180;
		double aviahp = xs.sIndic.aviahorizon_pitch;
		double aviar = xs.sIndic.aviahorizon_roll;
		// int pitchLimit = HUDFontsize * 5;
		int pitchLimit = HUDFontsize;
		realSpdPitch = -(aviahp + aoa);
		if (aviahp != -65535)
			pitch = (int) ((-aviahp * pitchLimit / 90.0f));
		else
			pitch = 0;
		// app.debugPrint(-(aviahp+aoa));
		int slideLimit = 4 * HUDFontsize;
		if (xs.sState.AoS != -65535) {
			aosX = (int) (-xs.sState.AoS * slideLimit / 30.0f);
		} else
			aosX = 0;
		rollDeg = (int) (-aviar);
		lineCompass = String.format("%3s", xs.compass);
		lineThrottle = String.format("%3s", throttle);
		char map_x = (char) ('A' + (xs.loc[1] * xs.mapinfo.mapStage) + xs.mapinfo.inGameOffset);
		int map_y = (int) (xs.loc[0] * xs.mapinfo.mapStage + xs.mapinfo.inGameOffset + 1);

		lineLoc = String.format("%c%d", map_x, map_y);
		if (drawHudMach)
			lines[0] = String.format("M%5s", xs.M);
		else
			lines[0] = String.format("%s%6s", ILbl, xs.IAS);

		/* 近地告警 */
		if (xs.radioAltValid && xs.radioAlt <= 500)
			lines[1] = HLbl + String.format("R%5s", xs.sRadioAlt);
		else
			lines[1] = HLbl + String.format("%6s", xs.salt);

		if (xs.SEP > 0) {
			lines[3] = String.format("%s↑%4s", SLbl, xs.sSEP);
		} else {
			lines[3] = String.format("%s↓%4s", SLbl, xs.sSEP);
		}
		if (xs.sState.Ny > 1.5f || xs.sState.Ny < -0.5f)
			lines[4] = String.format("G%5s", xs.Ny);
		else {
			// 燃油量和增压器
			String s = xs.sfueltime;
			String compressor;
			switch (xs.sState.compressorstage) {
				case 1:
					compressor = "C";
				case 2:
					compressor = "CC";
				case 3:
					compressor = "CCC";
				default:
					compressor = "";
			}
			if (xs.sState.gear <= 0) {
				lines[4] = String.format("L%5s%s", s, compressor);
			} else {
				lines[4] = String.format("E%5s", xs.sTime);
			}
			// if(xs.sState.compressorstage != 0){
			// lines[3] += "S"
			// +String.format("%d",xs.sState.compressorstage);
			// }
			s = null;
		}
		String brk = "";
		String gear = "";
		inAction = false;
		if (xs.sState.airbrake > 0) {
			brk = "BRK";
			if (xs.sState.airbrake != 100) {
				inAction |= true;
			}
			if (xs.sState.airbrake == 100)
				warnVne = true;
		}

		if (xs.sState.gear > 0) {
			gear = "GEA";
			if (xs.sState.gear != 100)
				inAction |= true;
		}

		if (xs.sState.flaps > 0) {
			lines[2] = String.format("F%3s%s%s", xs.flaps, brk, gear);
		} else {
			if (xs.hasWingSweepVario) {
				lines[2] = String.format("W%3s%s%s", xs.sWingSweep, brk, gear);
			} else {
				lines[2] = String.format("%4s%s%s", "", brk, gear);
			}
		}

		// 襟翼告警
		if (xs.sState.IAS > xs.flapAllowSpeed * 0.95) {
			//
			inAction = true;
		}

		if (xc.blkx != null && xc.blkx.valid) {
			// 机动指标(1 - 空油重/(空油重 + 油重))
			// 指标越低说明机动性越好
			double nfweight = xc.blkx.nofuelweight;
			maneuverIndex = 1 - (nfweight / (nfweight + xs.fTotalFuel));
			maneuverIndexLen = (int) Math.round(maneuverIndex / 0.5 * rightDraw);
			maneuverIndexLen10 = (int) Math.round(0.10 / 0.5 * rightDraw);
			maneuverIndexLen20 = (int) Math.round(0.20 / 0.5 * rightDraw);
			maneuverIndexLen30 = (int) Math.round(0.30 / 0.5 * rightDraw);
			maneuverIndexLen40 = (int) Math.round(0.40 / 0.5 * rightDraw);
			maneuverIndexLen50 = (int) Math.round(0.50 / 0.5 * rightDraw);
			// 速度
			double vwing = 0;
			if (xc.blkx.isVWing) {
				vwing = xs.sIndic.wsweep_indicator;
			}

			if ((xs.IASv >= xc.blkx.getVNEVWing(vwing) * 0.95) || (xs.sState.M >= xc.blkx.getMNEVWing(vwing) * 0.95f)) {
				warnVne = true;
			}

			int flaps = xs.sState.flaps > 0 ? xs.sState.flaps : 0;

			double maxAvailableAoA = xc.blkx.getAoAHighVWing(vwing, flaps);

			availableAoA = maxAvailableAoA - aoa;

			if (availableAoA < aoaWarningRatio * maxAvailableAoA) {
				aoaColor = app.colorWarning;
			} else {
				aoaColor = app.colorNum;
			}
			if (availableAoA < aoaBarWarningRatio * maxAvailableAoA) {
				aoaBarColor = app.colorUnit;
			} else {
				aoaBarColor = app.colorNum;
			}

			aoaY = (int) ((availableAoA * aoaLength) / maxAvailableAoA);

			if (aoaY > rightDraw)
				aoaY = rightDraw;

			AoAFuselagePix = (int) ((xc.blkx.NoFlapsWing.AoACritHigh - xc.blkx.Fuselage.AoACritHigh) * aoaLength
					/ xc.blkx.NoFlapsWing.AoACritHigh);
		} else {
			AoAFuselagePix = (int) (aoa * aoaLength / 15);
			aoaY = (int) (aoa * aoaLength / 30);
		}
		if (!disableAoA) {
			lineAoA = String.format("α%3.0f", aoa);
			// if(xs.energyJKg > 1000000)
			// relEnergy = String.format("E%6.0f", xs.energyJKg/1000);
			// else
			// relEnergy = String.format("E%5.1f", xs.energyJKg/1000);

			relEnergy = String.format("E%5.0f", xs.energyM);
		}
		// 姿态
		sAttitude = "";
		sAttitudeRoll = "";
		disableAttitude = false;
		roundHorizon = (int) Math.round(-aviahp);
		if (aviahp != -65535) {
			if (roundHorizon > 0)
				sAttitude = String.format("%3d", roundHorizon);
			if (roundHorizon < 0)
				sAttitude = String.format("%3d", -roundHorizon);
		} else {
			disableAttitude = true;
		}

		if (aviar != -65535) {
			int roundRoll = (int) Math.round(-aviar);
			if (roundRoll > 0)
				sAttitudeRoll = String.format("\\%3d", roundRoll);
			if (roundRoll < 0)
				sAttitudeRoll = String.format("/%3d", -roundRoll);
		} else {
			disableAttitude = true;
		}

		/* 雷达高度 */
		if (xs.radioAlt >= 0 && xs.radioAlt < 50) {
			warnRH = true;
		}
	}

	public void drawTick() {

		// updateString();
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

			long time = xs.SystemTime;
			if (time - hudCheckMili > xc.freqService) {

				hudCheckMili = xs.SystemTime;
				drawTick();
			}
		}
	}

}