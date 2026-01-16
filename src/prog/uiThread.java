package prog;

import ui.minimalHUD;
import ui.attitudeIndicator;
import ui.gearAndFlaps;

public class uiThread implements Runnable {
	long CheckMili;
	controller c;
	Boolean doit;
	private long HCheckMili;
	private long ACheckMili;
	private long GCheckMili;
	private int drawTickNr;

	public uiThread(controller xc) {
		c = xc;
		doit = Boolean.TRUE;
		drawTickNr = 0;
	}

	@Override
	public void run() {
		Boolean repaintH;
		long stime;
		while (doit) {
			// 每多少秒更新一次ui
			try {
				Thread.sleep(app.threadSleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			repaintH = false;
			if (c.S == null)
				continue;
			stime = c.S.SystemTime;

			// 刷新时间
			if (stime - HCheckMili >= c.freqService) {
				HCheckMili = stime;

				/* 刷新字符串 */
				minimalHUD H = (minimalHUD) c.overlayManager.get("crosshairSwitch");
				if (H != null) {
					H.updateString();
					repaintH = true;
					drawTickNr++;
				}
			}
			/*
			 * Refactored to Event-Driven
			 * if (stime - FCheckMili >= c.freqFlightInfo) {
			 * // 飞行信息
			 * FCheckMili = stime;
			 * flightInfo FL = (flightInfo) c.overlayManager.get("flightInfoSwitch");
			 * if (FL != null) {
			 * FL.updateString();
			 * repaintFL = true;
			 * drawTickNr++;
			 * }
			 * }
			 */

			/*
			 * Refactored to Event-Driven
			 * if (stime - ECheckMili >= c.freqEngineInfo) {
			 * ECheckMili = stime;
			 * engineControl F = (engineControl)
			 * c.overlayManager.get("enableEngineControl");
			 * if (F != null) {
			 * F.updateString();
			 * repaintF = true;
			 * drawTickNr++;
			 * }
			 * 
			 * engineInfo FI = (engineInfo) c.overlayManager.get("engineInfoSwitch");
			 * if (FI != null) {
			 * FI.updateString();
			 * repaintFI = true;
			 * drawTickNr++;
			 * }
			 * }
			 */

			// 立即刷新，提升实时性
			// Toolkit.getDefaultToolkit().sync();
			if (repaintH) {
				minimalHUD H = (minimalHUD) c.overlayManager.get("crosshairSwitch");
				if (H != null)
					H.drawTick();
			}

			if (stime - ACheckMili >= c.freqAltitude) {
				ACheckMili = stime;
				attitudeIndicator aI = (attitudeIndicator) c.overlayManager.get("enableAttitudeIndicator");
				if (aI != null) {
					if (c.S.sState != null && c.S.sIndic != null) {
						aI.drawTick();
						drawTickNr++;
					}
				}
			}

			if (stime - GCheckMili >= c.freqGearAndFlap) {
				GCheckMili = stime;
				gearAndFlaps fS = (gearAndFlaps) c.overlayManager.get("enablegearAndFlaps");
				if (fS != null) {
					drawTickNr++;
					fS.drawTick();
				}
			}
			/*
			 * Refactored to Event-Driven
			 * if (stime - SCheckMili >= c.freqStickValue) {
			 * SCheckMili = stime;
			 * stickValue sV = (stickValue) c.overlayManager.get("enableAxis");
			 * if (sV != null) {
			 * drawTickNr++;
			 * sV.drawTick();
			 * }
			 * }
			 */

			// Dynamic Overlays now self-refresh via ListOverlay.run()
			// No external polling needed
			// 10秒回收一次内存
			// if (stime - GCCheckMili > app.gcSeconds * 1000) {
			//// app.debugPrint("内存回收");
			// GCCheckMili = stime;
			// System.gc();
			// }
			// System.gc();
			// 8 * 4096次回收一次内存

			// Memory is now managed by the JVM; no manual GC needed
			if (drawTickNr >= 0x400) {
				drawTickNr = 0;
			}

		}
	}
}
