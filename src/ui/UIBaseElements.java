package ui;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;
import java.awt.Shape;
import java.awt.font.FontRenderContext;
import java.awt.font.GlyphVector;
import java.awt.geom.AffineTransform;

import prog.Application;

public class UIBaseElements {
	static AffineTransform tx = new AffineTransform();

	// public BasiStroke roundstroke = new BasicStroke(1, BasicStroke.CAP_ROUND,
	// BasicStroke.JOIN_ROUND);
	// 绘制库
	public static void __drawStringShade(Graphics2D g2d, int x, int y, int shadeWidth, String s, Font f, Color c) {
		g2d.setFont(f);

		g2d.setStroke(new BasicStroke(shadeWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));

		// drawshade
		if (Application.drawFontShape) {
			FontRenderContext frc = g2d.getFontRenderContext();
			GlyphVector glyphVector = f.createGlyphVector(frc, s);
			// get the shape object
			Shape textShape = glyphVector.getOutline();
			AffineTransform tx = new AffineTransform();
			tx.translate(x, y);
			Shape newShape = tx.createTransformedShape(textShape);

			g2d.setColor(Application.colorShadeShape);
			g2d.draw(newShape);
		} else {
			g2d.setColor(Application.colorShadeShape);
			g2d.drawString(s, x + 1, y + 1);
		}

		g2d.setColor(c);
		g2d.drawString(s, x, y);
	}

	public static void drawStringShade(Graphics2D g2d, int x, int y, int shadeWidth, String s, Font f) {
		__drawStringShade(g2d, x, y, shadeWidth, s, f, Application.colorNum);
	}

	public static void __drawVRect(Graphics2D g2d, int x, int y, int width, int height, int borderwidth, Color c,
			Color Frame) {
		g2d.setStroke(new BasicStroke(borderwidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
		// 外边框
		g2d.setColor(Frame);

		if (height >= 0) {
			g2d.drawRect(x, y - height, width - 1, height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth, y + borderwidth - height, width - 2 * borderwidth, height - 2 * borderwidth);
		} else {
			g2d.drawRect(x, y, width - 1, -height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth, y + borderwidth, width - 2 * borderwidth, -height - 2 * borderwidth);
		}
		// 内部条
	}

	// 横向还是竖向
	public static void drawVRect(Graphics2D g2d, int x, int y, int width, int height, int borderwidth, Color c) {
		g2d.setStroke(new BasicStroke(borderwidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
		// 外边框
		g2d.setColor(Application.colorShadeShape);

		if (height >= 0) {
			g2d.drawRect(x, y - height, width - 1, height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth, y + borderwidth - height, width - 2 * borderwidth, height - 2 * borderwidth);
		} else {
			g2d.drawRect(x, y, width - 1, -height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth, y + borderwidth, width - 2 * borderwidth, -height - 2 * borderwidth);
		}
		// 内部条
	}

	public static void drawHRect(Graphics2D g2d, int x, int y, int width, int height, int borderwidth, Color c) {
		g2d.setStroke(new BasicStroke(borderwidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
		// 外边框
		g2d.setColor(Application.colorShadeShape);

		if (width >= 0) {
			g2d.drawRect(x, y, width - 1, height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth, y + borderwidth, width - 2 * borderwidth, height - 2 * borderwidth);
		} else {
			g2d.drawRect(x + width, y, -width - 1, height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth + width, y + borderwidth, -width - 2 * borderwidth, height - 2 * borderwidth);
		}
		// 内部条
	}

	// 竖条
	public static void drawVBar(Graphics2D g2d, int x, int y, int width, int height, int val_height, int borderwidth,
			Color c) {
		g2d.setStroke(new BasicStroke(borderwidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
		// 外边框
		g2d.setColor(Application.colorShadeShape);
		// if (val_height > height) val_height = height;
		if (val_height >= 0) {
			g2d.drawRect(x, y - height, width - 1, height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth, y + borderwidth - val_height, width - 2 * borderwidth,
					val_height - 2 * borderwidth);
		} else {
			g2d.drawRect(x, y, width - 1, -height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth, y + borderwidth, width - 2 * borderwidth, -val_height - 2 * borderwidth);
		}
		// 内部条
	}

	// 竖条加标签系列(油门显示)
	public static void drawVBarText(Graphics2D g2d, int x, int y, int width, int height, int val_height,
			int borderwidth, Color c, String lbl, Font lblFont) {
		drawVBar(g2d, x, y, width, height, val_height, borderwidth, c);
		// 标签
		// __drawStringShade(g2d, x - width, y - height, 1, lbl, lblFont,
		// Application.lblNameColor);
	}

	// 数字
	public static void drawVBarTextNum(Graphics2D g2d, int x, int y, int width, int height, int val_height,
			int borderwidth, Color c, String lbl, String num, Font lblFont, Font numFont) {
		if (val_height > height)
			val_height = height;
		drawVBarText(g2d, x, y, width, height, val_height, borderwidth, c, lbl, lblFont);
		// 直线
		drawHRect(g2d, x, y - val_height - 1, width + 3 * numFont.getSize(), 3, 1, Application.colorLabel);
		// 数字
		__drawStringShade(g2d, x + width, y - val_height - 2, 1, num, numFont, Application.colorLabel);
	}

	public static void drawVBarTextNumLeft(Graphics2D g2d, int x, int y, int width, int height, int val_height,
			int borderwidth, Color c, Color fontc, String lbl, String num, Font lblFont, Font numFont) {
		if (val_height > height)
			val_height = height;
		drawVBarText(g2d, x + width + 3 * numFont.getSize() / 2, y, width, height, val_height, borderwidth, c, lbl,
				lblFont);
		// 直线
		drawHRect(g2d, x, y - val_height - 1, 2 * width + 3 * numFont.getSize() / 2, 3, 1, fontc);
		// 数字
		__drawStringShade(g2d, x, y - val_height - 2, 1, num, numFont, fontc);
	}

	// 横条系列
	public static void drawHBar(Graphics2D g2d, int x, int y, int width, int height, int val_width, int borderwidth,
			Color c) {
		g2d.setStroke(new BasicStroke(borderwidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
		// 外边框
		g2d.setColor(Application.colorShadeShape);
		// if (val_width > width) val_width = width;
		if (val_width >= 0) {
			g2d.drawRect(x, y, width - 1, height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth, y + borderwidth, val_width - 2 * borderwidth, height - 2 * borderwidth);
		} else {
			g2d.drawRect(x - width, y, -width - 1, height - 1);
			g2d.setColor(c);
			g2d.fillRect(x + borderwidth - width, y + borderwidth, -val_width - 2 * borderwidth,
					height - 2 * borderwidth);
		}
		// 内部条
	}

	// 竖条加标签系列(油门显示)
	public static void drawHBarText(Graphics2D g2d, int x, int y, int width, int height, int val_width, int borderwidth,
			Color c, String lbl, Font lblFont) {
		drawHBar(g2d, x, y, width, height, val_width, borderwidth, c);
		// 标签
		// __drawStringShade(g2d, x - width, y - height, 1, lbl, lblFont,
		// Application.lblNameColor);
	}

	public static void drawHBarTextNum(Graphics2D g2d, int x, int y, int width, int height, int val_width,
			int borderwidth, Color c, String lbl, String num, Font lblFont, Font numFont) {
		if (val_width > width)
			val_width = width;
		drawHBarText(g2d, x, y, width, height, val_width, borderwidth, c, lbl, lblFont);
		// 直线
		drawVRect(g2d, x + val_width - 2, y, 3, -height - 1 * numFont.getSize(), 1, Application.colorLabel);
		// 数字
		__drawStringShade(g2d, x + val_width, y + height + 1 * numFont.getSize(), 1, num, numFont, Application.colorLabel);
	}

	public static void _drawLabelBOSType(Graphics2D g2d, int x_offset, int y_offset, int shadeWidth, int lwidth,
			Font num,
			Font label, Font unit, String sNum, String sLabel, String sUnit) {

		// y偏移式加下底边再减去自己字体大小的一半
		__drawStringShade(g2d, x_offset, (y_offset + y_offset + label.getSize() + unit.getSize()) >> 1, shadeWidth,
				sNum, num, Application.colorNum);

		// 标签名
		__drawStringShade(g2d, x_offset + lwidth, y_offset, shadeWidth, sLabel, label, Application.colorLabel);
		// 单位名
		__drawStringShade(g2d, x_offset + lwidth, y_offset + label.getSize(), shadeWidth, sUnit, unit, Application.colorUnit);
	}

	// BOS 类型的标签
	public static void drawLabelBOSType(Graphics2D g2d, int x_offset, int y_offset, int shadeWidth, Font num,
			Font label, Font unit, String sNum, String sLabel, String sUnit) {

		// 数字
		int lwidth = (13 * num.getSize()) >> 2;
		_drawLabelBOSType(g2d, x_offset, y_offset, shadeWidth, lwidth, num, label, unit, sNum, sLabel, sUnit);
	}

	public static void __drawLabelBOSType(Graphics2D g2d, int x_offset, int y_offset, int shadeWidth, Font num,
			Font label, Font unit, String sNum, String sLabel, String sUnit, int lwwidth) {

		// 数字
		int lwidth = (lwwidth * num.getSize()) >> 2;
		// y偏移式加下底边再减去自己字体大小的一半
		__drawStringShade(g2d, x_offset, (y_offset + y_offset + label.getSize() + unit.getSize()) >> 1, shadeWidth,
				sNum, num, Application.colorNum);

		// 标签名
		__drawStringShade(g2d, x_offset + lwidth, y_offset, shadeWidth, sLabel, label, Application.colorLabel);
		// 单位名
		__drawStringShade(g2d, x_offset + lwidth, y_offset + label.getSize(), shadeWidth, sUnit, unit, Application.colorUnit);
	}

}
