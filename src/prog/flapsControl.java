package prog;

import java.awt.AWTException;
import java.awt.Robot;
import java.awt.event.KeyEvent;

public class flapsControl implements Runnable {
	public int flaps;
	public int objflaps;
	controller xc;
	public Robot A;
	public boolean isDown;
	public volatile boolean isRun;

	double spd;
	int cflapsspd = 300;
	double cflaps = 0.2;
	int lflapsspd = 250;
	double lflaps = 1;

	public void init(controller c) throws AWTException {
		isRun = true;
		xc = c;
		A = new Robot();
		isDown = false;
		flaps = 0;
		objflaps = 30;

		// System.out.println("初始化完毕");
	}

	public void updateflaps() {
		flaps = xc.S.sState.flaps;
		spd = xc.S.speedv * 3.6;
		if (spd > lflapsspd)
			objflaps = 20;
		else
			objflaps = 100;
		if (flaps < cflaps * 100)
			isDown = true;
	}

	public void changeobjflaps(int obj) {
		objflaps = obj;
	}

	public void close() {
		isRun = false;
	}

	void flapsup() {
		if (isDown) {
			A.keyPress(KeyEvent.VK_F);
			try {
				Thread.sleep(20);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			A.keyRelease(KeyEvent.VK_F);
			isDown = false;
		}
	}

	void flapsdown() {
		if (flaps == 0&&isDown==false) {
			A.keyPress(KeyEvent.VK_F);
			try {
				Thread.sleep(20);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			A.keyRelease(KeyEvent.VK_F);
			isDown = true;
		}
	}

	public void protectFlaps() {
		if (spd > cflapsspd && flaps > cflaps * 100) {
			flapsup();
		}
		/*if (spd <= cflapsspd && spd > lflapsspd) {
			isDown=false;
			flapsdown();
		}
		*/

	}

	@Override
	public void run() {
		// TODO Auto-generated method stub
		// System.out.println(flaps+" "+isRun);
		while (isRun) {

			try {
				Thread.sleep(100);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			updateflaps();
			// System.out.println(flaps);
			// 自动襟翼
			/*
			 * if (flaps > 0) { //System.out.println(spd+" "+objflaps); if
			 * (flaps > objflaps&&isDown) { //System.out.println("襟翼收回");
			 * A.keyPress(KeyEvent.VK_CAPS_LOCK); try { Thread.sleep(20); }
			 * catch (InterruptedException e) { // TODO Auto-generated catch
			 * block e.printStackTrace(); } A.keyRelease(KeyEvent.VK_CAPS_LOCK);
			 * isDown = false; } if (flaps < objflaps&&! isDown ) {
			 * //System.out.println("襟翼放下"); A.keyPress(KeyEvent.VK_TAB); try {
			 * Thread.sleep(20); } catch (InterruptedException e) { // TODO
			 * Auto-generated catch block e.printStackTrace(); }
			 * A.keyRelease(KeyEvent.VK_TAB); isDown = true; } }
			 */
			// 襟翼保护
			protectFlaps();
		}

	}
}