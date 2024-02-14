package com.voidmei.ui;

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
import com.voidmei.App;
import com.voidmei.controller;
import com.voidmei.otherService;
import com.voidmei.service;

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
	int blinkTicks=1;
	int blinkCheckTicks=0;
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
				g.setColor(App.colorShadeShape);

				g.drawLine(2 , 2, Width - 2 , Height - 2);
				g.drawLine( Width - 2, 2, 2 , Height - 2);
				g.setStroke(new BasicStroke(3));
				g.setColor(App.colorNum);
				g.drawLine(1, 1,  Width - 1, Height- 1);
				g.drawLine( Width - 1, 1, 1, Height - 1);
				
				
			}
			blinkCheckTicks+=1;
			if (blinkCheckTicks % blinkTicks == 0){
//				App.debugPrint(blinkTicks +"?" + blinkCheckTicks);
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
		int n5 = 5 * HUDFontsize;
//		BasicStroke outBs = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
//		BasicStroke inBs = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		// 油门
		if(throttley >= throttleh) {
			g.setColor(App.colorShadeShape);
			g.setStroke(outBs);
			g.drawLine(kx + barWidth + 1, n5 - throttleh + lineWidth + 2 , kx + barWidth, n5 - throttleh + lineWidth + 2);
			g.setColor(App.colorNum);
			g.setStroke(inBs);
			g.drawLine(kx + barWidth + 1, n5 - throttleh + lineWidth + 2 , kx + barWidth, n5 - throttleh + lineWidth + 2);
		}
		if(throttley >= throttlec) {
			g.setColor(App.colorShadeShape);
			g.setStroke(outBs);
			g.drawLine(kx + barWidth + 1, n5 - throttlec + lineWidth + 2 , (int)(kx + 1.25 * barWidth), n5 - throttlec + lineWidth + 2);
			g.setColor(App.colorNum);
			g.setStroke(inBs);
			g.drawLine(kx + barWidth + 1, n5 - throttlec + lineWidth + 2 , (int)(kx + 1.25 * barWidth), n5 - throttlec + lineWidth + 2);
		}
		if(throttley >= throttlem) {
			g.setColor(App.colorShadeShape);
			g.setStroke(outBs);
			g.drawLine(kx + barWidth + 1, n5 - throttlem + lineWidth + 2 , (int)(kx + 1.5 * barWidth), n5 - throttlem + lineWidth + 2);
			g.setColor(App.colorNum);
			g.setStroke(inBs);
			g.drawLine(kx + barWidth + 1, n5 - throttlem + lineWidth + 2 , (int)(kx + 1.5 * barWidth), n5 - throttlem + lineWidth + 2);
		}
		if(throttley >= throttlew) {
			g.setColor(App.colorShadeShape);
			g.setStroke(outBs);
			g.drawLine(kx + barWidth + 1, n5 - throttlew + lineWidth + 2 , kx + barWidth + barWidth, n5 - throttlew + lineWidth + 2);
			g.setColor(App.colorNum);
			g.setStroke(inBs);
			g.drawLine(kx + barWidth + 1, n5 - throttlew + lineWidth + 2 , kx + barWidth + barWidth, n5 - throttlew + lineWidth + 2);
		}
		uiBaseElem.drawVRect(g, kx, n5 + lineWidth + 2 , barWidth, throttley, 1, throttleColor);

//		if(throttley >= throttlem) {
//			g.setStroke(new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
//			g.setColor(App.colorShadeShape);
//			g.drawLine(kx , n5 - throttley + 2 , kx + barWidth + barWidth, n5 - throttley + 2);
//			g.setStroke(new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
//			g.setColor(App.colorNum);
//			g.drawLine(kx, n5 - throttley + 2, kx + barWidth + barWidth, n5 - throttley + 2);
//		}
		// uiBaseElem.__drawVRect(g, kx, n5 + 2, barWidth, throttley,
		// throttleLineWidth, App.colorNum, throttleColor);
		kx += barWidth;
		
		// 姿态
		int round = (int) (5 * roundCompass);
		int rnd = (int) (HUDFontsize * 0.618f);
		if (drawAttitude) {
			if (!blinkX || (blinkX && !blinkActing)) {
//				g.setStroke(new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
//				g.setColor(App.colorShadeShape);
//				g.drawArc(lineWidth + aosX + round / 2, n5 - HUDFontsize + pitch, round, 2*HUDFontsize, rollDeg, -180);
//				g.drawLine(lineWidth + round, n5, lineWidth + round + aosX, n5 + pitch);
//				
//				
////				g.drawLine(lineWidth + round + aosX, n5+ pitch, lineWidth + round + aosX + HUDFontsize/2, n5 + pitch);
//
//				uiBaseElem.__drawStringShade(g, lineWidth + round + aosX , n5 + pitch - 1, 1, sAttitude, drawFontSmall, App.colorNum);
//				uiBaseElem.__drawStringShade(g, lineWidth + round + aosX - HUDFontsize/2 , n5 + 3 * HUDFontsize/2 + pitch, 1, sAttitudeRoll, drawFontSmall, App.colorNum);
//				
////				uiBaseElem.__drawStringShade(g, lineWidth + round + aosX , n5 + pitch , 1, sAttitudeRoll, drawFontSmall, App.colorNum);
////				sAttitudeRoll
//				g.setStroke(new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
//				if (realSpdPitch < 0) {
//					g.setColor(App.colorUnit);
//				}
//				else
//					g.setColor(App.colorNum);
//				g.drawArc(lineWidth + aosX + round / 2, n5 - HUDFontsize + pitch, round, 2*HUDFontsize, rollDeg, -180);
//				g.drawLine(lineWidth + round, n5, lineWidth + round + aosX, n5 + pitch);
//				
////				g.setColor(App.colorUnit);
////				g.drawLine(lineWidth + round + aosX, n5+ pitch, lineWidth + round + aosX + HUDFontsize/2, n5 + pitch);
		        // 计算小圆形的位置

		        int circleX = lineWidth + aosX + round;
		        int circleY = (int) (n5 - 2.5 * HUDFontsize + rnd/2 - pitch);

		        // 绘制地面和牵引线
		        g.setStroke(outBs);
		        g.setColor(App.colorShadeShape);
		        g.drawLine(circleX - aosX, circleY + pitch, circleX - lineWidth, circleY);
		        g.drawLine(circleX - aosX - rnd/4 , circleY + pitch, circleX - aosX + rnd/4, circleY + pitch);
		        g.setStroke(inBs);
		        g.setColor(App.colorNum);
		        g.drawLine(circleX - aosX, circleY + pitch, circleX - lineWidth, circleY);
		        g.drawLine(circleX - aosX - rnd/4, circleY + pitch, circleX - aosX + rnd/4, circleY + pitch);
		        
		        
		        // 旋转整个组合图形表示横滚角
		        AffineTransform oldTransform = g.getTransform();
		        AffineTransform transform = AffineTransform.getRotateInstance(Math.toRadians(rollDeg), circleX, circleY);
		        
		        g.setTransform(transform);
		        
		        g.setStroke(new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
		        g.setColor(App.colorShadeShape);
		        

		        // 画小圆形
		        g.drawOval(circleX - rnd / 2, circleY - rnd / 2, rnd, rnd);
		        
//		        g.drawLine(circleX - aosX, circleY + pitch, circleX, circleY + pitch);
		        
		        // 画三条支线表示水平和垂直方向
		        g.drawLine(circleX , circleY - rnd/2 - lineWidth - 1, circleX, circleY - rnd);
		        g.drawLine(circleX + rnd/2 + lineWidth + 1, circleY, circleX + rnd, circleY);
		        g.drawLine(circleX - rnd/2 - lineWidth - 1, circleY, circleX - rnd, circleY);
		        
		        g.setStroke(new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
		        g.setColor(App.colorNum);
				
		        g.drawOval(circleX - rnd / 2, circleY - rnd / 2, rnd, rnd);
		        
//		        g.drawLine(circleX - aosX, circleY + pitch, circleX, circleY + pitch);
		        
		        // 画三条支线表示水平和垂直方向
		        g.drawLine(circleX , circleY - rnd/2 - lineWidth, circleX, circleY - rnd);
		        g.drawLine(circleX + rnd/2 + lineWidth, circleY, circleX + rnd, circleY);
		        g.drawLine(circleX - rnd/2 - lineWidth, circleY, circleX - rnd, circleY);
		        
		        g.setTransform(oldTransform);


		        // 画文字
		      if (roundHorizon >= 0) {
		    	  uiBaseElem.__drawStringShade(g, circleX , circleY - 1, 1, sAttitude, drawFontSmall, App.colorNum);
		      }else {
		    	  uiBaseElem.__drawStringShade(g, circleX , circleY - 1, 1, sAttitude, drawFontSmall, App.colorUnit);
		      }
//		        uiBaseElem.__drawStringShade(g, circleX - HUDFontsize / 2, circleY + 3 * HUDFontsize / 2, 1, sAttitudeRoll, drawFontSmall, App.colorNum);
		        
		        
		        // 恢复原始的图形变换
//		        g.setTransform(oldTransform);
			}
		}

		for (int i = 0; i < 5; i++) {

			//
			// g.setColor(App.lblShadeColorMinor);
			// g.drawString(lines[i], x + 1, n + y + 1);
			// // g.drawString(lines[i], x - 1, n + y - 1);
			//// g.drawString(lines[i], x - 1, n + y - 1);
			// g.setColor(App.lblNumColor);
			// if (i == 2 && inAction) {
			// g.setColor(App.warning);
			// }
			// if (i == 0 && warnVne) {
			// g.setColor(App.warning);
			// }
			// g.drawString(lines[i], x, n + y);

			// i = 0画直线提示
			if (i == 0) {

				// g.setColor(App.lblShadeColorMinor);
				// g.drawLine((HUDFontsize * 3) >> 1, 2 + y, rightDraw - 2, 2 +
				// y);
				// g.setColor(App.lblNumColor);
				// g.drawLine(((HUDFontsize * 3) >> 1) - 1, 1 + y, rightDraw -
				// 3, 1 + y);

//				int linex = (HUDFontsize * 3) >> 1;
				int liney = 1 + y;

				// uiBaseElem.drawHRect(g, linex, liney, rightDraw - 2 - linex,
				// 2+lineWidth, 1, App.lblNumColor);
				// uiBaseElem.drawHRect(g, (int)((rightDraw - x) * availableAoA
				// / 30) + x, liney, (int)((rightDraw - x) * (30 - availableAoA)
				// / 30), 2+lineWidth, 1, App.lblNumColor);
				// availableAoA

				uiBaseElem.drawHRect(g, x + (rightDraw - aoaY), liney, aoaY, lineWidth + 3, 1, aoaBarColor);
				// uiBaseElem.drawHBar(g, x + (rightDraw - aoaY), liney, width,
				// height, aoaY, borderwidth, c);b

				uiBaseElem.__drawStringShade(g, x + rightDraw, liney - 1, 1, lineAoA, drawFontSmall, aoaColor);

			}
			if (i == 1) {
				int liney = 1 + y;
				uiBaseElem.__drawStringShade(g, x + rightDraw, n + liney - 1, 1, relEnergy, drawFontSmall, aoaColor);
				
			}
			if ((i == 2 && inAction) || (i == 0 && warnVne) || (i == 1 && warnRH)) {
				uiBaseElem.__drawStringShade(g, x, n + y, 1, lines[i], drawFont, App.colorWarning);
			} else
				uiBaseElem.__drawStringShade(g, x, n + y, 1, lines[i], drawFont, App.colorNum);
			// if( i==1){
			n = n + HUDFontsize;
			// }
		}

		n += 2;

		// g.setColor(App.lblShadeColorMinor);
		// g.drawLine(3, n, 3, n - throttley);
		// g.setColor(App.lblNumColor);
		// g.drawLine(2, n - 1, 2, n - 1 - throttley);

		// uiBaseElem.drawVRect(g, kx, n, barWidth, aoaY, 1, App.warning);
		// uiBaseElem.drawVBar(g, kx, n, barWidth, AoAFuselagePix, aoaY, 1,
		// App.warning);
		// kx += barWidth;
		//

		// uiBaseElem.drawVBarTextNum(g, kx, n, barWidth, HUDFontsize * 5,
		// throttley, 1, App.lblNumColor, "", "T"+throttley, drawFont,
		// drawFont);
		// uiBaseElem.drawVBar(g, kx, n, barWidth, HUDFontsize * 5, throttley,
		// 1, App.lblNumColor);
		if (!drawAttitude) {
			/*
			 * // if (pitch > 0) { // uiBaseElem.drawVRect(g, kx, n, barWidth,
			 * pitch, 1, // App.colorWarning); // } else { //
			 * uiBaseElem.drawVRect(g, kx, n, barWidth, pitch, 1, App.colorNum);
			 * // } // kx += barWidth;
			 */ }

		// uiBaseElem.drawVBar(g, x + rightDraw, n, barWidth, AoAFuselagePix,
		// aoaY, 1, App.warning);
		// AoA
		// g.setColor(App.lblShadeColorMinor);
		// g.drawLine(1, n, 1, n - aoaY);
		// g.setColor(App.warning);
		// g.drawLine(0, n - 1, 0, n - 1 - aoaY);

		// g.setColor(App.lblShadeColorMinor);
		// g.drawLine(0, n-1,0, n-1 - aoaY );
		// g.setColor(App.warning);
		// g.drawLine(1, n, 1, n-aoaY);
		//
		// 燃油量
		// int rightDraw = (int)(HUDFontsize * 4.5f + 1);
		// 引擎健康度
		// g.setColor(App.lblShadeColorMinor);
		// g.drawLine(rightDraw, n, -2 + (rightDraw - OilX), n);
		// g.setColor(App.lblNumColor);
		// g.drawLine(rightDraw - 1, n-1, -2 + (rightDraw - OilX) - 1, n-1);
		//
		// 方位

		// g.setColor(App.lblShadeColorMinor);
		// g.drawLine(rightDraw, n, rightDraw, n - pitch);
		// if (pitch > 0)
		// g.setColor(App.lblNumColor);
		// else
		// g.setColor(App.warning);
		// if (pitch > 0){
		// uiBaseElem.drawVRect(g, rightDraw, n, 5, pitch, 1, App.lblNumColor);
		// }
		// else{
		// uiBaseElem.drawVRect(g, rightDraw, n, 5, pitch, 1, App.warning);
		// }

		// g.drawLine(rightDraw - 1, n - 1, rightDraw - 1, n - 1 - pitch);

		// 画一个半圆

		// 绘制方向
//		n += 2;
		// 2倍半径
		int r = roundCompass;
		n -= 2*HUDFontsize - 2;
		kx += rightDraw + r;
		
		
		g.setStroke(outBs);
		g.setColor(App.colorShadeShape);

		g.drawLine(kx + r + (int)(0.618 * r * Math.sin(compassRads)), n + r - (int)(0.618 * r * Math.cos(compassRads)), kx + r + compassDx, n + r - compassDy);
		g.drawOval(kx, n, r + r, r + r);
//		g.drawArc(kx, n, r + r, r + r, compass - 5, compass + 365 );
		
		// 引擎健康度
		// g.drawArc(2 + 3, n + 3, r + r - 4, r + r - 4, -180, OilX);

		g.setStroke(inBs);
		g.setColor(App.colorNum);
//		g.drawLine(kx + r + r * compassRads, n + r, kx + r + compassDx, n + r - compassDy);
		g.drawLine(kx + r + (int)(0.618 * r *  Math.sin(compassRads)), n + r - (int)(0.618 * r *  Math.cos(compassRads)), kx + r + compassDx, n + r - compassDy);
		g.drawOval(kx, n, r + r, r + r);
//		g.drawArc(kx, n, r + r, r + r, compass - 5, compass + 365);

		// g.setColor(App.warning);
		// g.drawArc(2 + 2, n + 2, r + r - 4, r + r - 4, -180, OilX);

		// g.setColor(App.lblShadeColorMinor);
		// g.drawString(lineHorizon, x + 1, n + y + 1);
		//
		// g.setColor(App.lblNumColor);
		// g.drawString(lineHorizon, x, n + y);

		// g.setColor(App.lblShadeColorMinor);
		// g.drawString(lineCompass, x + 1, n + (r - HUDFontsize / 2) + y + 1);
		//
		// g.setColor(App.lblNumColor);
		// g.drawString(lineCompass, x, n + (r - HUDFontsize / 2) + y);
		uiBaseElem.drawStringShade(g, kx + lineWidth + 3, n + y - (r - HUDFontsize) / 2 , 1, lineCompass, drawFontSmall);
//		uiBaseElem.drawStringShade(g, kx + x, n + (r - HUDFontsize / 2) + y, 1, lineCompass, drawFont);
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
		this.getWebRootPaneUI().setTopBg(App.previewColor);
		this.getWebRootPaneUI().setMiddleBg(App.previewColor);
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
			NumFont = App.defaultNumfontName;
		if (xc.getconfig("crosshairX") != "")
			lx = Integer.parseInt(xc.getconfig("crosshairX"));
		else
			lx = (App.screenWidth - Width) / 2;
		if (xc.getconfig("crosshairY") != "")
			ly = Integer.parseInt(xc.getconfig("crosshairY"));
		else
			ly = (App.screenHeight - Height) / 2;
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
		// App.debugPrint(xc.getconfig("usetexturecrosshair"));
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
			Width = (int) (CrossWidth * 2) - HUDFontsize;
		else {
			Width = (int) (CrossWidth * 2);
		}
		Height = (int) (CrossWidth * 1.5);
		CrossWidthVario = CrossWidth;
		if (lineWidth == 0)
			lineWidth = 1;
		roundCompass = (int) (Math.round(HUDFontsize * 0.8f));
		rightDraw = (int) (HUDFontsize * 3.5f);
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
		lineHorizon = String.format("%3s", "45");
		throttley = 100;
		aoaY = 10;
		disableAoA = false;
		throttleColor = App.colorShadeShape;
		lineAoA = String.format("α%3.0f", 20.0);
		relEnergy = "E114514";
		sAttitude = "";
		sAttitudeRoll="";
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
		aoaColor = App.colorNum;
		aoaBarColor = App.colorNum;
		
		throttlew = (110 * HUDFontsize * 5) / 110;
		throttlem = (100 * HUDFontsize * 5) / 110;
		throttlec = (80 * HUDFontsize * 5) / 110;
		throttleh = (50 * HUDFontsize * 5) / 110;
	
		outBs = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		inBs = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		// App.debugPrint(lx);
		// App.debugPrint(ly);
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
		CrossX = Width / 2;
		CrossY = Height / 2;
		panel = new WebPanel() {

			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				
				  
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw
				g2d.setPaintMode();
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, App.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, App.textAASetting);
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

//		RepaintManager rm = RepaintManager.currentManager(this);
//		boolean b = rm.isDoubleBufferingEnabled();
//		rm.setDoubleBufferingEnabled(false);
//		
		this.add(panel);
		// if (App.debug)setShadeWidth(8);
		// setShowWindowButtons(false);
		// setShowTitleComponent(false);
		// setShowResizeCorner(false);
		// setDefaultCloseOperation(3);
		// setTitle("hud");
		// setAlwaysOnTop(true);
		// root = this.getContentPane();
		// setFocusable(false);
		// setFocusableWindowState(false);// 取消窗口焦点
		// this.setCursor(App.blankCursor);
		// setVisible(true);
		// 1miao 8 ci
		blinkTicks = (int) ((1000/xc.freqService) >> 3);
		if (blinkTicks == 0)blinkTicks = 1;
		
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

	public void updateString() {

		warnVne = false;
		warnRH = false;
		blinkX = xs.fatalWarn;
		int throttle = xs.sState.throttle;
		if (throttle >= 100) {
			throttleColor = App.colorNum;
		} else {
			throttleColor = App.colorNum;
//			if (throttle >= 85) {
//				throttleColor = App.colorLabel;
//			} else {
//				if (throttle >= 50) {
//					throttleColor = App.colorUnit;
//				}
//				// else
//				// throttleColor = App.colorShadeShape;
//			}
		}
		throttley = (throttle * HUDFontsize * 5) / 110;
		

		compass = (int)xs.sIndic.compass;
		compassRads = (double)Math.toRadians(xs.sIndic.compass);
		
//		double compassRads = (double) Math.toRadians(xs.sIndic.compass);
		
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
		// App.debugPrint(-(aviahp+aoa));
		int slideLimit = 4 * HUDFontsize;
		if (xs.sState.AoS != -65535) {
			aosX = (int) (-xs.sState.AoS * slideLimit / 30.0f);
		} else
			aosX = 0;
		rollDeg = (int) (-aviar);
		lineCompass = String.format("%3s", xs.compass);
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
			lines[3] = String.format("%s↑%5s", SLbl, xs.sSEP);
		}
		else {
			lines[3] = String.format("%s↓%5s", SLbl, xs.sSEP);
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

			lines[4] = String.format("L%5s%s", s, compressor);
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
			else
				warnVne = true;
		}

		if (xs.sState.gear > 0) {
			gear = "GEAR";
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

			if (availableAoA < aoaWarningRatio * xc.blkx.NoFlapsWing.AoACritHigh)
				aoaColor = App.colorWarning;
			else {
				aoaColor = App.colorNum;
			}
			if (availableAoA < aoaBarWarningRatio * xc.blkx.NoFlapsWing.AoACritHigh)	
				aoaBarColor  = App.colorWarning;
			else{
				aoaBarColor  = App.colorNum;
			}
			aoaY = (int) ((availableAoA * aoaLength) / maxAvailableAoA);

			if (aoaY > rightDraw)
				aoaY = rightDraw;

			// if (xc.blkx.NoFlapsWing.AoACritHigh - xs.sState.AoA <
			// 0.33f*xc.blkx.NoFlapsWing.AoACritHigh)
			// lines[4] = "A" + String.format("%5.1f",
			// xc.blkx.NoFlapsWing.AoACritHigh - xs.sState.AoA);
			// else
			// lines[4] = "";
			AoAFuselagePix = (int) ((xc.blkx.NoFlapsWing.AoACritHigh - xc.blkx.Fuselage.AoACritHigh) * aoaLength
					/ xc.blkx.NoFlapsWing.AoACritHigh);
		} else {
			AoAFuselagePix = (int) (aoa * aoaLength / 15);
			aoaY = (int) (aoa * aoaLength / 30);
		}
		if (!disableAoA) {
			lineAoA = String.format("α%3.0f", aoa);
//			if(xs.energyJKg > 1000000)
//				relEnergy = String.format("E%6.0f", xs.energyJKg/1000);
//			else
//				relEnergy = String.format("E%5.1f", xs.energyJKg/1000);
			
			relEnergy = String.format("E%5.0f", xs.energyM);
		}
		// 姿态
		sAttitude = "";
		sAttitudeRoll = "";

		roundHorizon = (int) Math.round(-aviahp);
		if (aviahp != -65535) {
			if (roundHorizon > 0)
				sAttitude = String.format("%3d", roundHorizon);
			if (roundHorizon < 0)
				sAttitude = String.format("%3d", -roundHorizon);
		}
		if (aviar != -65535) {
			int roundRoll = (int) Math.round(-aviar);
			if (roundRoll > 0)
				sAttitudeRoll = String.format("\\%3d", roundRoll);
			if (roundRoll < 0)
				sAttitudeRoll = String.format("/%3d", -roundRoll);
		}
		
		/* 雷达高度 */
		if (xs.radioAlt >= 0 && xs.radioAlt < 50) {
			warnRH = true;
		}
	}
	public void drawTick() {

//		updateString();
		root.repaint();
	}

	@Override
	public void run() {

		while (doit) {
			try {
				Thread.sleep(App.threadSleepTime);
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