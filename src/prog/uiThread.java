package prog;

import java.awt.AWTException;

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

	public uiThread(controller xc) {
		c = xc;
		doit = Boolean.TRUE;
	}

	@Override
	public void run() {

		while (doit) {
			// 每多少秒更新一次ui
			try {
				Thread.sleep(app.threadSleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}

			long stime = c.S.SystemTime;

			// 刷新时间
			if (stime - HCheckMili >= c.freqService) {
				HCheckMili = stime;

				if (c.H != null) {
					c.H.drawTick();
				}
			}
			if (stime - FCheckMili >= c.freqFlightInfo) {
				// 飞行信息
				FCheckMili = stime;
				if (c.FL != null) {
					c.FL.drawTick();
				}
			}

			if (stime - ECheckMili >= c.freqEngineInfo) {
				ECheckMili = stime;
				if (c.F != null) {
					c.F.drawTick();
				}

				if (c.FI != null) {
					c.FI.drawTick();
				}
			}

			if (stime - ACheckMili >= c.freqAltitude) {
				ACheckMili = stime;
				if (c.aI != null) {
					if (c.S.sState != null && c.S.sIndic != null)
						c.aI.drawTick();
				}
			}
			
			if (stime - GCheckMili >= c.freqGearAndFlap) {
				GCheckMili = stime;
				if (c.fS != null) {
					c.fS.drawTick();
				}
			}
			if (stime - SCheckMili >= c.freqStickValue) {
				SCheckMili = stime;
				if (c.sV != null) {
					c.sV.drawTick();
				}
			}
			
			// 20秒回收一次内存
			if (stime - GCCheckMili > app.gcSeconds * 1000) {
//				System.out.println("内存回收");
				GCCheckMili = stime;
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
