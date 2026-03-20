package prog;

public class UIThread implements Runnable {
	Controller c;
	Boolean doit;

	public UIThread(Controller xc) {
		c = xc;
		doit = Boolean.TRUE;
	}

	@Override
	public void run() {
		while (doit) {
			// 每多少秒更新一次ui
			try {
				Thread.sleep(Application.threadSleepTime);
			} catch (InterruptedException e) {
				Thread.currentThread().interrupt();
				break;
			}
			if (c.S == null)
				continue;

			// Overlays are now event-driven (implement FlightDataListener)
			// No polling needed here.
		}
	}
}
