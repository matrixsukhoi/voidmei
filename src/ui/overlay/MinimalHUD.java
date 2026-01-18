package ui.overlay;

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
import java.util.Map;

import com.alee.laf.panel.WebPanel;

import prog.Application;
import prog.Controller;
import prog.OtherService;
import prog.Service;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.event.FlightDataListener;
import ui.UIBaseElements;
import ui.WebLafSettings;
import ui.base.DraggableOverlay;

/**
 * MinimalHUD overlay for displaying compact flight information.
 * Being migrated to event-driven architecture.
 */
public class MinimalHUD extends DraggableOverlay implements FlightDataListener {

	/**
	 * 
	 */
	public volatile boolean doit = true;
	Boolean on;
	Controller xc;
	WebPanel panel;
	int HUDFontsize;
	int Width;
	int Height;
	int CrossX;
	int CrossY;
	int AoAFuselagePix;
	int Vx;
	int compass;
	Service xs;
	OtherService cs;
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

	// Reusable UI Components (high-performance, cached strokes)
	private ui.component.CrosshairGauge crosshairGauge;
	private ui.component.FlapAngleBar flapAngleBar;
	private ui.component.WarningOverlay warningOverlay;
	private ui.component.CompassGauge compassGauge;

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
		// 高度警告标记 - now using reusable component
		if (blinkX) {
			if (warningOverlay != null) {
				warningOverlay.draw(g, 0, 0, Width, Height, blinkActing);
			}
			blinkCheckTicks += 1;
			if (blinkCheckTicks % blinkTicks == 0) {
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
		// Now using reusable CrosshairGauge component with cached strokes
		if (crosshairGauge != null) {
			crosshairGauge.draw(g, CrossX, CrossY, CrossWidth);
		}
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

	public void drawFlapAngleBar(Graphics2D g, int x, int y) {
		// 绘制襟翼角度文本
		int strWidth = g.getFontMetrics(drawFontSmall).stringWidth(lineFlapAngle);
		int strX = x + (Width - x - HUDFontsize / 2 - strWidth) / 2;
		UIBaseElements.__drawStringShade(g, strX, y, 1, lineFlapAngle, drawFontSmall, Application.colorNum);

		// 横条参数
		int barY = y + HUDFontSizeSmall / 4;
		int barHeight = lineWidth + 2;
		// 让横条填满右侧空间 (Width 是总宽, x 是起始x坐标, 预留一点右边距)
		int barTotalWidth = Width - x - HUDFontsize / 2;

		// 计算三色区域宽度 (0-125范围映射到barTotalWidth)
		int blueWidth = (int) (flapA * barTotalWidth / 125.0);
		int greenWidth = (int) ((flapAllowA - flapA) * barTotalWidth / 125.0);
		int redWidth = barTotalWidth - blueWidth - greenWidth;

		// 画刻度线
		g.setColor(Application.colorLabel);
		g.setStroke(new BasicStroke(2));

		int[] ticks = { 20, 33, 60, 100 };
		for (int t : ticks) {
			int tx = x + (int) (t * barTotalWidth / 125.0);
			// 100处刻度更长 (延伸 barHeight), 其他较短 (延伸 1/4)
			int ext = (t == 100) ? barHeight : barHeight / 4;

			// 绘制白色本体
			g.setColor(Application.colorLabel);
			g.setStroke(new BasicStroke(2));
			g.drawLine(tx, barY - ext - 4, tx, barY);
		}

		// 绘制蓝色区域 (0 → flapA)
		if (blueWidth > 0) {
			g.setColor(Application.colorShadeShape);
			g.fillRect(x, barY, blueWidth, barHeight);
		}

		// 绘制绿色区域 (flapA → flapAllowA)
		if (greenWidth > 0) {
			g.setColor(Application.colorNum);
			g.fillRect(x + blueWidth, barY, greenWidth, barHeight);
		}

		// 绘制红色区域 (flapAllowA → 125)
		if (redWidth > 0) {
			g.setColor(Application.colorWarning);
			g.fillRect(x + blueWidth + greenWidth, barY, redWidth, barHeight);
		}

	}

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
		// g.setColor(Application.colorShadeShape);
		// g.setStroke(Bs3);
		// g.drawLine(kx + barWidth, n5 - throttleh + lineWidth + 2 , kx + barWidth, n5
		// - throttleh + lineWidth + 2);
		// g.setColor(Application.colorNum);
		// g.setStroke(Bs1);
		// g.drawLine(kx + barWidth, n5 - throttleh + lineWidth + 2 , kx + barWidth, n5
		// - throttleh + lineWidth + 2);
		// }
		// if(throttley >= throttlec) {
		// g.setColor(Application.colorShadeShape);
		// g.setStroke(Bs3);
		// g.drawLine(kx + barWidth, n5 - throttlec + lineWidth + 2 , (int)(kx + 1.25 *
		// barWidth), n5 - throttlec + lineWidth + 2);
		// g.setColor(Application.colorNum);
		// g.setStroke(Bs1);
		// g.drawLine(kx + barWidth, n5 - throttlec + lineWidth + 2 , (int)(kx + 1.25 *
		// barWidth), n5 - throttlec + lineWidth + 2);
		// }
		// if(throttley >= throttlem) {
		// g.setColor(Application.colorShadeShape);
		// g.setStroke(Bs3);
		// g.drawLine(kx + barWidth, n5 - throttlem + lineWidth + 2 , (int)(kx + 1.5 *
		// barWidth), n5 - throttlem + lineWidth + 2);
		// g.setColor(Application.colorNum);
		// g.setStroke(Bs1);
		// g.drawLine(kx + barWidth, n5 - throttlem + lineWidth + 2 , (int)(kx + 1.5 *
		// barWidth), n5 - throttlem + lineWidth + 2);
		// }
		// if(throttley >= throttlew) {
		// g.setColor(Application.colorShadeShape);
		// g.setStroke(Bs3);
		// g.drawLine(kx + barWidth, n5 - throttlew + lineWidth + 2 , kx + barWidth +
		// barWidth, n5 - throttlew + lineWidth + 2);
		// g.setColor(Application.colorNum);
		// g.setStroke(Bs1);
		// g.drawLine(kx + barWidth, n5 - throttlew + lineWidth + 2 , kx + barWidth +
		// barWidth, n5 - throttlew + lineWidth + 2);
		// }
		// UIBaseElements.drawVRect(g, kx, n5 + lineWidth + 2 , barWidth, throttley, 1,
		// throttleColor);
		int yOffset = y - HUDFontsize;
		UIBaseElements.drawVBarTextNumLeft(g, kx + barWidth, n5 + yOffset + lineWidth + 2, barWidth, throttley_max,
				throttley, 1,
				Application.colorNum, throttleColor, "",
				lineThrottle, drawFontSSmall, drawFontSSmall);

		x += barWidth + 3 * drawFontSSmall.getSize() / 2;
		kx += barWidth + 3 * drawFontSSmall.getSize() / 2;
		// 姿态

		if (drawAttitude && !disableAttitude) {
			if (!blinkX || (blinkX && !blinkActing)) {
				// 计算小圆形的位置

				int circleX = lineWidth - aosX + round;
				int circleY = (int) (n5 + yOffset - 2.5 * HUDFontsize + rnd / 2 - pitch);
				double rollDegRad = Math.toRadians(rollDeg);
				// 绘制地面和牵引线
				g.setStroke(Bs3);
				g.setColor(Application.colorShadeShape);
				g.drawLine(circleX + aosX, circleY + pitch, circleX, circleY);
				g.setStroke(Bs1);
				g.setColor(Application.colorLabel);
				g.drawLine(circleX + aosX, circleY + pitch, circleX, circleY);

				// 旋转整个组合图形表示横滚角
				AffineTransform oldTransform = g.getTransform();
				AffineTransform transform = AffineTransform.getRotateInstance(-rollDegRad, circleX, circleY);

				g.setTransform(transform);

				g.setStroke(new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
				g.setColor(Application.colorShadeShape);

				int hbs = halfLine;
				// 画三条支线表示水平和垂直方向

				g.drawArc(circleX - hrnd + hbs, circleY - hrnd + hbs, rnd, rnd, -180, 180);
				g.drawLine(circleX + hbs, circleY - hrnd / 2 + hbs, circleX + hbs, circleY - urnd + hbs);
				g.drawLine(circleX + hrnd + hbs, circleY + hbs, circleX + urnd + hbs, circleY + hbs);
				g.drawLine(circleX - hrnd + hbs, circleY + hbs, circleX - urnd + hbs, circleY + hbs);

				g.setStroke(new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
				g.setColor(Application.colorNum);

				// 画三条支线表示水平和垂直方向
				g.drawArc(circleX - hrnd + hbs, circleY - hrnd + hbs, rnd, rnd, -180, 180);
				g.drawLine(circleX + hbs, circleY - hrnd / 2 + hbs, circleX + hbs, circleY - urnd + hbs);
				g.drawLine(circleX + hrnd + hbs, circleY + hbs, circleX + urnd + hbs, circleY + hbs);
				g.drawLine(circleX - hrnd + hbs, circleY + hbs, circleX - urnd + hbs, circleY + hbs);

				g.setTransform(oldTransform);

				// 画文字
				if (roundHorizon >= 0) {
					UIBaseElements.__drawStringShade(g, circleX, circleY - 1, 1, sAttitude, drawFontSmall,
							Application.colorNum);
				} else {
					UIBaseElements.__drawStringShade(g, circleX, circleY - 1, 1, sAttitude, drawFontSmall,
							Application.colorUnit);
				}
				// UIBaseElements.__drawStringShade(g, circleX - HUDFontsize / 2, circleY + 3 *
				// HUDFontsize / 2, 1, sAttitudeRoll, drawFontSmall, Application.colorNum);

				// 恢复原始的图形变换
				// g.setTransform(oldTransform);
			}
		}

		for (int i = 0; i < 5; i++) {

			// i = 0画直线提示
			if (i == 0) {

				int liney = 1 + y;

				// availableAoA
				UIBaseElements.drawHRect(g, x + (rightDraw - aoaY), liney, aoaY, lineWidth + 3, 1, aoaBarColor);
				UIBaseElements.__drawStringShade(g, x + rightDraw, liney - 1, 1, lineAoA, drawFontSmall, aoaColor);

			}
			if (i == 1) {
				int liney = 1 + y;
				UIBaseElements.__drawStringShade(g, x + rightDraw, n + liney - 1, 1, relEnergy, drawFontSmall,
						aoaColor);

			}
			if ((i == 2 && inAction) || (i == 0 && warnVne) || (i == 1 && warnRH)) {
				UIBaseElements.__drawStringShade(g, x, n + y, 1, lines[i], drawFont, Application.colorWarning);
			} else
				UIBaseElements.__drawStringShade(g, x, n + y, 1, lines[i], drawFont, Application.colorNum);

			if (i == 4) {
				// 机动性指标线

				g.setColor(Application.colorShadeShape);
				g.setStroke(Bs3);
				g.drawLine(x + rightDraw - maneuverIndexLen10, n + y + halfLine + lineWidth + lineWidth,
						x + rightDraw - maneuverIndexLen10, n + y + halfLine - lineWidth + lineWidth);
				g.setColor(Application.colorNum);
				g.setStroke(Bs1);
				g.drawLine(x + rightDraw - maneuverIndexLen10, n + y + halfLine + lineWidth + lineWidth,
						x + rightDraw - maneuverIndexLen10, n + y + halfLine - lineWidth + lineWidth);

				if (maneuverIndex >= 0.1) {
					g.setColor(Application.colorShadeShape);
					g.setStroke(Bs3);
					g.drawLine(x + rightDraw - maneuverIndexLen20, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen20, n + y + halfLine - lineWidth + lineWidth);
					g.setColor(Application.colorNum);
					g.setStroke(Bs1);
					g.drawLine(x + rightDraw - maneuverIndexLen20, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen20, n + y + halfLine - lineWidth + lineWidth);
				}
				if (maneuverIndex >= 0.2) {
					g.setColor(Application.colorShadeShape);
					g.setStroke(Bs3);
					g.drawLine(x + rightDraw - maneuverIndexLen30, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen30, n + y + halfLine - lineWidth + lineWidth);
					g.setColor(Application.colorNum);
					g.setStroke(Bs1);
					g.drawLine(x + rightDraw - maneuverIndexLen30, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen30, n + y + halfLine - lineWidth + lineWidth);
				}
				if (maneuverIndex >= 0.3) {
					g.setColor(Application.colorShadeShape);
					g.setStroke(Bs3);
					g.drawLine(x + rightDraw - maneuverIndexLen40, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen40, n + y + halfLine - lineWidth + lineWidth);
					g.setColor(Application.colorNum);
					g.setStroke(Bs1);
					g.drawLine(x + rightDraw - maneuverIndexLen40, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen40, n + y + halfLine - lineWidth + lineWidth);
				}
				if (maneuverIndex >= 0.4) {
					g.setColor(Application.colorShadeShape);
					g.setStroke(Bs3);
					g.drawLine(x + rightDraw - maneuverIndexLen50, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen50, n + y + halfLine - lineWidth + lineWidth);
					g.setColor(Application.colorNum);
					g.setStroke(Bs1);
					g.drawLine(x + rightDraw - maneuverIndexLen50, n + y + halfLine + lineWidth + lineWidth,
							x + rightDraw - maneuverIndexLen50, n + y + halfLine - lineWidth + lineWidth);
				}
				g.setStroke(Bs3);
				g.setColor(Application.colorShadeShape);
				g.drawLine(x + rightDraw, n + y + halfLine + lineWidth, x + rightDraw - maneuverIndexLen,
						n + y + halfLine + lineWidth);

				g.setStroke(Bs1);
				g.setColor(Application.colorNum);
				g.drawLine(x + rightDraw, n + y + halfLine + lineWidth, x + rightDraw - maneuverIndexLen,
						n + y + halfLine + lineWidth);

			}

			n = n + HUDFontsize;

		}
		n += 2;

		// g.setColor(Application.lblShadeColorMinor);
		// g.drawLine(3, n, 3, n - throttley);
		// g.setColor(Application.lblNumColor);
		// g.drawLine(2, n - 1, 2, n - 1 - throttley);

		// UIBaseElements.drawVRect(g, kx, n, barWidth, aoaY, 1, Application.warning);
		// UIBaseElements.drawVBar(g, kx, n, barWidth, AoAFuselagePix, aoaY, 1,
		// Application.warning);
		// kx += barWidth;
		//

		// UIBaseElements.drawVBarTextNum(g, kx, n, barWidth, HUDFontsize * 5,
		// throttley, 1, Application.lblNumColor, "", "T"+throttley, drawFont,
		// drawFont);
		// UIBaseElements.drawVBar(g, kx, n, barWidth, HUDFontsize * 5, throttley,
		// 1, Application.lblNumColor);
		if (!drawAttitude) {
			/*
			 * // if (pitch > 0) { // UIBaseElements.drawVRect(g, kx, n, barWidth,
			 * pitch, 1, // Application.colorWarning); // } else { //
			 * UIBaseElements.drawVRect(g, kx, n, barWidth, pitch, 1, Application.colorNum);
			 * // } // kx += barWidth;
			 */ }

		// UIBaseElements.drawVBar(g, x + rightDraw, n, barWidth, AoAFuselagePix,
		// aoaY, 1, Application.warning);
		// AoA
		// g.setColor(Application.lblShadeColorMinor);
		// g.drawLine(1, n, 1, n - aoaY);
		// g.setColor(Application.warning);
		// g.drawLine(0, n - 1, 0, n - 1 - aoaY);

		// g.setColor(Application.lblShadeColorMinor);
		// g.drawLine(0, n-1,0, n-1 - aoaY );
		// g.setColor(Application.warning);
		// g.drawLine(1, n, 1, n-aoaY);
		//
		// 燃油量
		// int rightDraw = (int)(HUDFontsize * 4.5f + 1);
		// 引擎健康度
		// g.setColor(Application.lblShadeColorMinor);
		// g.drawLine(rightDraw, n, -2 + (rightDraw - OilX), n);
		// g.setColor(Application.lblNumColor);
		// g.drawLine(rightDraw - 1, n-1, -2 + (rightDraw - OilX) - 1, n-1);
		//
		// 方位

		// g.setColor(Application.lblShadeColorMinor);
		// g.drawLine(rightDraw, n, rightDraw, n - pitch);
		// if (pitch > 0)
		// g.setColor(Application.lblNumColor);
		// else
		// g.setColor(Application.warning);
		// if (pitch > 0){
		// UIBaseElements.drawVRect(g, rightDraw, n, 5, pitch, 1,
		// Application.lblNumColor);
		// }
		// else{
		// UIBaseElements.drawVRect(g, rightDraw, n, 5, pitch, 1, Application.warning);
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
		g.setColor(Application.colorShadeShape);

		g.drawLine(kx + r + (int) (0.618 * r * Math.sin(compassRads)),
				n + yOffset + r - (int) (0.618 * r * Math.cos(compassRads)), kx + r + compassDx,
				n + yOffset + r - compassDy);
		g.drawOval(kx, n + yOffset, r + r, r + r);
		// g.drawArc(kx, n, r + r, r + r, compass - 5, compass + 365 );

		// 引擎健康度
		// g.drawArc(2 + 3, n + 3, r + r - 4, r + r - 4, -180, OilX);

		g.setStroke(inBs);
		g.setColor(Application.colorNum);
		// g.drawLine(kx + r + r * compassRads, n + r, kx + r + compassDx, n + r -
		// compassDy);
		g.drawLine(kx + r + (int) (0.618 * r * Math.sin(compassRads)),
				n + yOffset + r - (int) (0.618 * r * Math.cos(compassRads)), kx + r + compassDx,
				n + yOffset + r - compassDy);
		g.drawOval(kx, n + yOffset, r + r, r + r);
		// g.drawArc(kx, n, r + r, r + r, compass - 5, compass + 365);

		// g.setColor(Application.warning);
		// g.drawArc(2 + 2, n + 2, r + r - 4, r + r - 4, -180, OilX);

		// g.setColor(Application.lblShadeColorMinor);
		// g.drawString(lineHorizon, x + 1, n + y + 1);
		//
		// g.setColor(Application.lblNumColor);
		// g.drawString(lineHorizon, x, n + y);

		// g.setColor(Application.lblShadeColorMinor);
		// g.drawString(lineCompass, x + 1, n + (r - HUDFontsize / 2) + y + 1);
		//
		// g.setColor(Application.lblNumColor);
		// g.drawString(lineCompass, x, n + (r - HUDFontsize / 2) + y);
		UIBaseElements.drawStringShade(g, kx + lineWidth + 3, n + y - (r - HUDFontsize) / 2, 1, lineCompass,
				drawFontSmall);
		UIBaseElements.drawStringShade(g, kx + lineWidth + 3, n + y + r + HUDFontSizeSmall / 2, 1, lineLoc,
				drawFontSmall);

		// UIBaseElements.drawStringShade(g, kx + x, n + (r - HUDFontsize / 2) + y, 1,
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

	public void initPreview(Controller c) {
		init(c, null, null);
		// setShadeWidth(10);
		// this.setVisible(false);
		this.getWebRootPaneUI().setTopBg(Application.previewColor);
		this.getWebRootPaneUI().setMiddleBg(Application.previewColor);
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
		xc.setconfig("crosshairX", Integer.toString(getLocation().x));
		xc.setconfig("crosshairY", Integer.toString(getLocation().y));
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

	// 襟翼角度
	private double flapA;
	private double flapAllowA;
	private String lineFlapAngle;
	private boolean enableFlapAngleBar;
	int lx;
	int ly;

	public void reinitConfig() {

		if (xc.getconfig("MonoNumFont") != "")
			NumFont = xc.getconfig("MonoNumFont");
		else
			NumFont = Application.defaultNumfontName;
		if (xc.getconfig("crosshairX") != "")
			lx = Integer.parseInt(xc.getconfig("crosshairX"));
		else
			lx = (Application.screenWidth - Width) / 2;
		if (xc.getconfig("crosshairY") != "")
			ly = Integer.parseInt(xc.getconfig("crosshairY"));
		else
			ly = (Application.screenHeight - Height) / 2;
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
		// Application.debugPrint(xc.getconfig("usetexturecrosshair"));
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

		if (xc.getconfig("enableFlapAngleBar") != "") {
			enableFlapAngleBar = Boolean.parseBoolean(xc.getconfig("enableFlapAngleBar"));
		} else {
			enableFlapAngleBar = true;
		}

		HUDFontsize = CrossWidth / 4;
		barWidth = HUDFontsize / 4;
		lineWidth = HUDFontsize / 10;
		if (!crossOn)
			Width = (int) (CrossWidth * 2.25) - HUDFontsize;
		else {
			Width = (int) (CrossWidth * 2.25);
		}
		Height = (int) (CrossWidth * 1.5) + (int) (HUDFontsize * 3.5);
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
		hrnd = (int) Math.round(rnd / 2.0);

		if (xc.getconfig("hudMach") != "")
			drawHudMach = Boolean.parseBoolean(xc.getconfig("hudMach"));

		if (xc.getconfig("disableHUDSpeedLabel") != "") {
			if (Boolean.parseBoolean(xc.getconfig("disableHUDSpeedLabel"))) {
				ILbl = "";
			} else {
				ILbl = "SPD";
			}
		}
		if (xc.getconfig("disableHUDHeightLabel") != "") {
			if (Boolean.parseBoolean(xc.getconfig("disableHUDHeightLabel"))) {
				HLbl = "";
			} else {
				HLbl = "ALT";
			}
		}
		if (xc.getconfig("disableHUDSEPLabel") != "") {
			if (Boolean.parseBoolean(xc.getconfig("disableHUDSEPLabel"))) {
				SLbl = "";
			} else {
				SLbl = "SEP";
			}
		}
		if (xc.getconfig("disableHUDAoA") != "") {
			if (Boolean.parseBoolean(xc.getconfig("disableHUDAoA"))) {
				lineAoA = "";
				disableAoA = true;
				relEnergy = "";
			} else {
				disableAoA = false;
			}
		}

		aoaLength = rightDraw - HUDFontsize / 2;
		if (aoaY > rightDraw)
			aoaY = rightDraw;

		throttlew = (110 * HUDFontsize * 5) / 110;
		throttlem = (100 * HUDFontsize * 5) / 110;
		throttlec = (80 * HUDFontsize * 5) / 110;
		throttleh = (50 * HUDFontsize * 5) / 110;

		outBs = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		inBs = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		halfLine = (lineWidth / 2 == 0) ? 1 : (int) Math.round(lineWidth / 2.0f);
		Bs3 = new BasicStroke(halfLine + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		Bs1 = new BasicStroke(halfLine, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);

		A = Toolkit.getDefaultToolkit().createImage("image/gunsight/" + crosshairName + ".png");
		C = A.getScaledInstance(CrossWidth * 2, CrossWidth * 2, Image.SCALE_SMOOTH);

		if (crossOn)
			this.setBounds(lx, ly, Width * 2, Height);
		else
			this.setBounds(lx, ly, Width, Height);

		drawFont = new Font(NumFont, Font.BOLD, HUDFontsize);
		HUDFontSizeSmall = (int) (HUDFontsize * 0.75f);
		drawFontSmall = new Font(NumFont, Font.BOLD, HUDFontSizeSmall);
		drawFontSSmall = new Font(NumFont, Font.BOLD, HUDFontsize / 2);

		repaint();
	}

	public void init(Controller c, Service s, OtherService os) {
		xs = s;
		xc = c;
		Vx = 0;
		cs = os;
		setFrameOpaque();

		reinitConfig();

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
		throttleColor = Application.colorShadeShape;
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
		aoaColor = Application.colorNum;
		aoaBarColor = Application.colorNum;

		throttlew = (110 * HUDFontsize * 5) / 110;
		throttlem = (100 * HUDFontsize * 5) / 110;
		throttlec = (80 * HUDFontsize * 5) / 110;
		throttleh = (50 * HUDFontsize * 5) / 110;

		outBs = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		inBs = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		halfLine = (lineWidth / 2 == 0) ? 1 : (int) Math.round(lineWidth / 2.0f);
		Bs3 = new BasicStroke(halfLine + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		Bs1 = new BasicStroke(halfLine, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		// Application.debugPrint(lx);
		// Application.debugPrint(ly);
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

		flapA = 20.0;
		flapAllowA = 100.0;
		lineFlapAngle = String.format("%3.0f/%3.0f", flapA, flapAllowA);

		// Initialize reusable UI components (high-performance)
		crosshairGauge = new ui.component.CrosshairGauge();
		flapAngleBar = new ui.component.FlapAngleBar();
		warningOverlay = new ui.component.WarningOverlay();
		compassGauge = new ui.component.CompassGauge(roundCompass);

		panel = new WebPanel() {

			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {

				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw
				g2d.setPaintMode();
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
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
					// 绘制襟翼角度
					// 显示在顶部, 并向右偏移以避开左侧油门条
					if (enableFlapAngleBar) {
						int flapXOffset = barWidth + 3 * drawFontSSmall.getSize() / 2;
						drawFlapAngleBar(g2d, HUDFontsize / 2 + flapXOffset, (int) (HUDFontsize * 1.2));
					}

					drawTextseries(g2d, HUDFontsize / 2, (int) (HUDFontsize * 2.5));

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
		// if (Application.debug)setShadeWidth(8);
		// setShowWindowButtons(false);
		// setShowTitleComponent(false);
		// setShowResizeCorner(false);
		// setDefaultCloseOperation(3);
		// setTitle("hud");
		// setAlwaysOnTop(true);
		// root = this.getContentPane();
		// setFocusable(false);
		// setFocusableWindowState(false);// 取消窗口焦点
		// this.setCursor(Application.blankCursor);
		// setVisible(true);
		// 1miao 8 ci
		blinkTicks = (int) ((1000 / xc.freqService) >> 3);
		if (blinkTicks == 0)
			blinkTicks = 1;

		// Load refresh interval from config
		refreshInterval = (long) (xc.freqService * 1.0); // Match freqService

		setTitle("miniHUD");
		WebLafSettings.setWindowOpaque(this);
		root = this.getContentPane();
		// this.createBufferStrategy(2);

		// Subscribe to events for game mode
		if (s != null) {
			subscribeToEvents();
			setVisible(true);
		}

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
			throttleColor = Application.colorWarning;
		} else {
			throttleColor = Application.colorNum;
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
		// Application.debugPrint(-(aviahp+aoa));
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

		parser.Blkx b = xc.getBlkx();
		if (b != null && b.valid) {
			// 机动指标(1 - 空油重/(空油重 + 油重))
			// 指标越低说明机动性越好
			double nfweight = b.nofuelweight;
			maneuverIndex = 1 - (nfweight / (nfweight + xs.fTotalFuel));
			maneuverIndexLen = (int) Math.round(maneuverIndex / 0.5 * rightDraw);
			maneuverIndexLen10 = (int) Math.round(0.10 / 0.5 * rightDraw);
			maneuverIndexLen20 = (int) Math.round(0.20 / 0.5 * rightDraw);
			maneuverIndexLen30 = (int) Math.round(0.30 / 0.5 * rightDraw);
			maneuverIndexLen40 = (int) Math.round(0.40 / 0.5 * rightDraw);
			maneuverIndexLen50 = (int) Math.round(0.50 / 0.5 * rightDraw);
			// 速度
			double vwing = 0;
			if (b.isVWing) {
				vwing = xs.sIndic.wsweep_indicator;
			}

			if ((xs.IASv >= b.getVNEVWing(vwing) * 0.95) || (xs.sState.M >= b.getMNEVWing(vwing) * 0.95f)) {
				warnVne = true;
			}

			int flaps = xs.sState.flaps > 0 ? xs.sState.flaps : 0;

			double maxAvailableAoA = b.getAoAHighVWing(vwing, flaps);

			availableAoA = maxAvailableAoA - aoa;

			if (availableAoA < aoaWarningRatio * maxAvailableAoA) {
				aoaColor = Application.colorWarning;
			} else {
				aoaColor = Application.colorNum;
			}
			if (availableAoA < aoaBarWarningRatio * maxAvailableAoA) {
				aoaBarColor = Application.colorUnit;
			} else {
				aoaBarColor = Application.colorNum;
			}

			aoaY = (int) ((availableAoA * aoaLength) / maxAvailableAoA);

			if (aoaY > rightDraw)
				aoaY = rightDraw;

			AoAFuselagePix = (int) ((b.NoFlapsWing.AoACritHigh - b.Fuselage.AoACritHigh) * aoaLength
					/ b.NoFlapsWing.AoACritHigh);
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

		// 襟翼角度显示
		flapA = xs.sState.flaps;
		flapAllowA = xs.getFlapAllowAngle(xs.sState.IAS, xs.isDowningFlap);
		lineFlapAngle = String.format("%3.0f/%3.0f", flapA, flapAllowA);
	}

	public void drawTick() {

		// updateString();
		root.repaint();
	}

	// --- Event-Driven Update ---

	// Throttling for refresh rate
	private static final long DEFAULT_REFRESH_INTERVAL = 100; // ms
	private long refreshInterval = DEFAULT_REFRESH_INTERVAL;
	private long lastRefreshTime = 0;

	@Override
	public void onFlightData(FlightDataEvent event) {
		// Throttle updates based on configured refresh interval
		long now = System.currentTimeMillis();
		if (now - lastRefreshTime < refreshInterval) {
			return; // Skip this update, too soon
		}
		lastRefreshTime = now;

		javax.swing.SwingUtilities.invokeLater(() -> {
			updateFromEvent(event);
			if (root != null)
				root.repaint();
		});
	}

	/**
	 * Update HUD state from FlightDataEvent.
	 * 
	 * NOTE: Phase 2 Partial Migration
	 * The event-driven architecture is in place (updates triggered by
	 * FlightDataEvent),
	 * but data is still read from Service (xs) fields for now because:
	 * 1. updateString() has complex dependencies on Blkx flight model data
	 * 2. Map coordinate calculations require Service.mapinfo
	 * 3. Many calculated warning thresholds depend on Blkx.getVNE/getAoA methods
	 * 
	 * Future work: Move calculations to Service and expose via FlightDataEvent
	 * keys.
	 */
	private void updateFromEvent(FlightDataEvent event) {
		Map<String, String> data = event.getData();

		// Read simple values from event where available
		String fatalWarnStr = data.get("fatalWarn");
		if (fatalWarnStr != null) {
			blinkX = Boolean.parseBoolean(fatalWarnStr);
		}

		// Delegate complex calculations to legacy method
		// which still reads from xs (Service) for Blkx-dependent logic
		if (xs != null) {
			updateString();
		}
	}

	/**
	 * Subscribe to flight data events.
	 */
	public void subscribeToEvents() {
		FlightDataBus.getInstance().register(this);
	}

	/**
	 * Unsubscribe from flight data events.
	 */
	public void unsubscribeFromEvents() {
		FlightDataBus.getInstance().unregister(this);
	}

	@Override
	public void run() {
		// Event-driven - no polling needed
		// Kept for compatibility with DraggableOverlay interface
	}

	@Override
	public void dispose() {
		unsubscribeFromEvents();
		super.dispose();
	}

}
