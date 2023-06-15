package prog;

import java.awt.AWTException;
import java.awt.Toolkit;

import parser.flightLog;
import ui.attitudeIndicator;
import ui.drawFrameSimpl;
import ui.engineInfo;
import ui.engineControl;
import ui.flightInfo;
import ui.gearAndFlaps;
import ui.minimalHUD;
import ui.someUsefulData;
import ui.stickValue;

public class uiThread implements Runnable {
	long CheckMili;
	controller c;
	Boolean doit;
	private long HCheckMili;
	private long FCheckMili;
	private long ECheckMili;
	private long ACheckMili;
	private long GCheckMili;
	private long SCheckMili;
	private long GCCheckMili;
	private int drawTickNr;

	public uiThread(controller xc) {
		c = xc;
		doit = Boolean.TRUE;
		drawTickNr = 0;
	}

	@Override
	public void run() {
		Boolean repaintH;
		Boolean repaintFL;
		Boolean repaintF;
		Boolean repaintFI;
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
			repaintFL = false;
			repaintF = false;
			repaintFI = false;
			stime = c.S.SystemTime;
			
			// 刷新时间
			if (stime - HCheckMili >= c.freqService) {
				HCheckMili = stime;
				
				/* 刷新字符串 */
				if (c.H != null) {
					c.H.updateString();
					repaintH = true;
					drawTickNr ++;
				}
			}
			if (stime - FCheckMili >= c.freqFlightInfo) {
				// 飞行信息
				FCheckMili = stime;
				if (c.FL != null) {
					c.FL.updateString();
					repaintFL = true;
					drawTickNr ++;
				}
			}

			if (stime - ECheckMili >= c.freqEngineInfo) {
				ECheckMili = stime;
				if (c.F != null) {
					c.F.updateString();
					repaintF = true;
					drawTickNr ++;
				}

				if (c.FI != null) {
					c.FI.updateString();
					repaintFI = true;
					drawTickNr ++;
				}
			}



			// 立即刷新，提升实时性
//			Toolkit.getDefaultToolkit().sync();
			if (repaintH) c.H.drawTick();
			if (repaintFL) c.FL.drawTick();
			if (repaintF) c.F.drawTick();
			if (repaintFI) c.FI.drawTick();
			
			if (stime - ACheckMili >= c.freqAltitude) {
				ACheckMili = stime;
				if (c.aI != null) {
					if (c.S.sState != null && c.S.sIndic != null) {
						c.aI.drawTick();
						drawTickNr ++;
					}
				}
			}
			
			if (stime - GCheckMili >= c.freqGearAndFlap) {
				GCheckMili = stime;
				if (c.fS != null) {
					drawTickNr ++;
					c.fS.drawTick();
				}
			}
			if (stime - SCheckMili >= c.freqStickValue) {
				SCheckMili = stime;
				if (c.sV != null) {
					drawTickNr ++;
					c.sV.drawTick();
				}
			}
			// 10秒回收一次内存
//			if (stime - GCCheckMili > app.gcSeconds * 1000) {
////				app.debugPrint("内存回收");
//				GCCheckMili = stime;
//				System.gc();
//			}
//			System.gc();
			// 8 * 4096次回收一次内存
			
			if (drawTickNr >= 0x400) {
//				GCCheckMili = stime;
				drawTickNr = 0;
				System.gc();
			}

			// if (Boolean.parseBoolean(getconfig("crosshairSwitch"))) {
			// H = new minimalHUD();
			// H1 = new Thread(H);
			// H.init(this, S, O);
			// H1.start();
			// }
			// if (Boolean.parseBoolean(getconfig("flightInfoSwitch"))) {
			// FL = new flightInfo();
			// FL1 = new Thread(FL);
			// FL.init(this, S);
			// FL1.start();
			// }
			// if (Boolean.parseBoolean(getconfig("enableLogging"))) {
			// if (dF != null) {
			// dF.doit = false;
			// dF = null;
			// }
			// notification(language.cStartlog);
			// Log = new flightLog();
			// Log.init(this, S);
			// Log1 = new Thread(Log);
			// Log1.start();
			// logon = true;
			// }
			//
			// if (Boolean.parseBoolean(getconfig("enableAxis"))) {
			// sV = new stickValue();
			// sV.init(this, S);
			// sV1 = new Thread(sV);
			// sV1.start();
			// }
			//
			// if (Boolean.parseBoolean(getconfig("enableAttitudeIndicator"))) {
			// aI = new attitudeIndicator();
			// aI.init(this, S);
			// aI1 = new Thread(aI);
			// aI1.start();
			// }
			//
			// if (Boolean.parseBoolean(getconfig("enablegearAndFlaps"))) {
			// fS = new gearAndFlaps();
			// fS.init(this, S);
			// fS1 = new Thread(fS);
			// fS1.start();
			//
			// }

		}
	}
}
