package prog;

public class gcThread implements Runnable {
	long GCCheckMili;
	controller c;
	Boolean doit;

	public void init(controller xc) {
		c = xc;
		doit = Boolean.TRUE;
	}

	@Override
	public void run() {

		while (doit) {

			try {
				Thread.sleep(1000);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}

			if (c.S != null) {
				long stime = c.S.SystemTime;

				// 20秒回收一次内存
				if (stime - GCCheckMili > app.gcSeconds * 1000) {
//					app.debugPrint("内存回收");
					GCCheckMili = stime;
					System.gc();
				}
			}

		}
	}
}
