package prog;

public class UIThread implements Runnable {
	Controller c;
	Boolean doit;
	private int drawTickNr;

	public UIThread(Controller xc) {
		c = xc;
		doit = Boolean.TRUE;
		drawTickNr = 0;
	}

	@Override
	public void run() {
		while (doit) {
			// 每多少秒更新一次ui
			try {
				Thread.sleep(Application.threadSleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			if (c.S == null)
				continue;

			// Overlays are now event-driven (implement FlightDataListener)
			// No polling needed here.

			// Memory is now managed by the JVM; no manual GC needed
			if (drawTickNr >= 0x400) {
				drawTickNr = 0;
			}
		}
	}
}
