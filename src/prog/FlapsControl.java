package prog;

import java.awt.AWTException;
import java.awt.Robot;
import java.awt.event.KeyEvent;

import prog.util.ExceptionHelper;

public class FlapsControl implements Runnable {
	public int flaps;
	public int objflaps;
	Controller xc;
	public Robot A;
	public boolean isDown;
	public volatile boolean isRun;

	double spd;
	int cflapsspd = 300;
	double cflaps = 0.2;
	int lflapsspd = 250;
	double lflaps = 1;

	public void init(Controller c) throws AWTException {
		isRun = true;
		xc = c;
		A = new Robot();
		isDown = false;
		flaps = 0;
		objflaps = 30;

		// Application.debugPrint("初始化完毕");
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
			ExceptionHelper.sleepQuietly(20);
			A.keyRelease(KeyEvent.VK_F);
			isDown = false;
		}
	}

	void flapsdown() {
		if (flaps == 0&&isDown==false) {
			A.keyPress(KeyEvent.VK_F);
			ExceptionHelper.sleepQuietly(20);
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
		while (isRun) {
			ExceptionHelper.sleepQuietly(100);
			updateflaps();
			// 襟翼保护
			protectFlaps();
		}
	}
}