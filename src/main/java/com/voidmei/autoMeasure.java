package com.voidmei;

import java.io.IOException;

/* 自动测量各高度的功率、速度和*/
public class autoMeasure implements Runnable {
	service xS;
	boolean doit = false;
	int start;
	private double spd;
	autoMeasure (service S){
		xS = S;
		doit = true;
		start = 0;
		spd = 0;
	}
	/* 找到最大速度，使用逐步递增法，从100开始，持续1s SEP为正则+100，否则不加 */
	double findMaxSpd(double curSpd) throws IOException{
		double SEP = xS.SEP;
		if (Math.abs(SEP) >= 1){
			// if (SEP > 0)
			// 	curSpd += Math.sqrt(2 * SEP * xS.g);
			// else
			// 	curSpd -= Math.sqrt(-2 * SEP * xS.g);
			curSpd += SEP/10 * curSpd;
			xS.httpClient.fmCmdSetSpd(curSpd, App.requestDest);
		}
		return curSpd;
	}

	@Override
	public void run() {
		/* 线程 */
		while (doit) {
			
			try {
				Thread.sleep(100);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}	
			if (xS.sState.gear != 100 || (xS.speedv > 10 && xS.sState.throttle > 0)) {
				// 如果收起落架则关闭break
				/* 测试，开始传送 */
				/* 先上10000米 */
				try {
					if (start == 0){
						xS.httpClient.fmCmdSetAlt(3000, App.requestDest);
						
						// spd = 100;
						xS.httpClient.fmCmdSetSpd(spd, App.requestDest);
						start = 1;	
					}else{
						// Thread.sleep(5000);
						// spd = findMaxSpd(spd);
					}
					
					
				} catch (IOException e) {
					// TODO Auto-generated catch block
					e.printStackTrace();
				}
				// } catch (InterruptedException e) {
				// 	// TODO Auto-generated catch block
				// 	e.printStackTrace();
				// }
			}
			
		}
	}
}
