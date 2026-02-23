package ui;

import java.awt.Color;

import com.alee.laf.rootpane.WebFrame;

import prog.AlwaysOnTopCoordinator;
import prog.Application;

public class WebLafSettings {

	static final Color transparent = new Color(0, 0, 0, 0);

	public static void setWindowFocus(WebFrame t) {
		t.setShowWindowButtons(false);
		t.setShowTitleComponent(false);
		t.setShowResizeCorner(false);

		t.setDefaultCloseOperation(3);
		t.setIgnoreRepaint(true);
		// t.setRound(100);
		// t.setUndecorated(false);
		// t.setDefaultLookAndFeelDecorated(false);

		// 焦点抢占防护：必须在 registerOverlay() 之前设置焦点属性
		// 否则 setAlwaysOnTop(true) 可能触发焦点事件，导致窗口闪烁
		t.setFocusableWindowState(false);// 取消窗口焦点
		t.setFocusable(false);
		// Register with coordinator instead of direct setAlwaysOnTop
		// This ensures overlays respect pending dialogs
		AlwaysOnTopCoordinator.getInstance().registerOverlay(t);
		t.setCursor(Application.blankCursor);
		// t.setVisible(true);
	}

	public static void setWindowOpaque(WebFrame t) {
		t.getWebRootPaneUI().setMiddleBg(transparent);// 中部透明
		t.getWebRootPaneUI().setTopBg(transparent);// 顶部透明
		t.getWebRootPaneUI().setBorderColor(transparent);// 内描边透明
		t.getWebRootPaneUI().setInnerBorderColor(transparent);// 外描边透明

		setWindowFocus(t);

	}
}
