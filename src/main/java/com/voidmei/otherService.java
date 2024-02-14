package com.voidmei;

import java.io.BufferedInputStream;
import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.net.SocketAddress;

import com.voidmei.parser.hudMsg;
import com.voidmei.parser.mapInfo;
import com.voidmei.parser.mapObj;

public class otherService implements Runnable {

	String sMapInfo;
	String sMapObj;
	String shudMsg;

	controller xc;
	public mapInfo mapi;
	public mapObj mapo;

	public double distance;
	public double enemyspeed;
	public double AOT;
	public double AZI;

	public int enemycount;
	public int friendcount;

	public int dislmt;
	double pX;
	double pY;
	long SpeedCheckMili;

	hudMsg msg;
	int lastEvt;
	int lastDmg;

	public volatile boolean isRun;
	boolean isgetMsg;
	boolean isgetmapObj;
	boolean isOverheat;
	boolean hisOverheat;
	int check;

	public double angleToclock(double angle) {
		double temp;
		temp = 12 + angle / 30.0f;
		if (temp >= 12)
			temp = temp - 12;
		return temp;
	}

	public double dxdyToangle(double dx, double dy) {
		double tems;
		tems = Math.atan(dy / dx) * 180 / Math.PI;
		if (dy >= 0 && dx <= 0) {
			tems = 180 + tems;
		}
		if (dy <= 0 && dx <= 0) {
			tems = 180 + tems;
		}
		if (dy <= 0 && dx >= 0) {
			tems=360+tems;
		}
		return tems;
	}

	public String sendGet(String host, int port, String path) throws IOException {

		String result = "";
		Socket socket = new Socket();
		SocketAddress dest = new InetSocketAddress(host, port);
		socket.connect(dest);
		OutputStreamWriter streamWriter = new OutputStreamWriter(socket.getOutputStream());
		BufferedWriter bufferedWriter = new BufferedWriter(streamWriter);

		bufferedWriter.write("GET " + path + " HTTP/1.1\r\n");
		bufferedWriter.write("Host: " + host + "\r\n");
		bufferedWriter.write(App.httpHeader);
		bufferedWriter.write("\r\n");
		bufferedWriter.flush();

		BufferedInputStream streamReader = new BufferedInputStream(socket.getInputStream());

		BufferedReader bufferedReader = new BufferedReader(new InputStreamReader(streamReader, "utf-8"));

		String line = null;

		bufferedReader.ready();
		bufferedReader.readLine();

		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		// App.debugPrint(System.currentTimeMillis()-testCheckMili);
		StringBuilder contentBuf = new StringBuilder();
		while ((line = bufferedReader.readLine()) != null) {
			contentBuf.append(line);
		}
		result = contentBuf.toString();

		bufferedReader.close();
		bufferedWriter.close();
		socket.close();
		return result;
	}

	public void init(controller c) {
		isRun = true;
		xc = c;
		pX = 0;
		pY = 0;
		lastEvt = 0;
		lastDmg = 0;
		lastEvt = xc.lastEvt;
		lastDmg = xc.lastDmg;
		isgetMsg = true;
		isgetmapObj = true;
		isOverheat = false;
		hisOverheat = false;
		//
		dislmt = 1200;
		SpeedCheckMili = System.currentTimeMillis();
		mapi = new mapInfo();
		mapi.init();
		if (isgetmapObj) {
			mapo = new mapObj();
			mapo.init();
		}
		if (isgetMsg) {
			msg = new hudMsg();
			msg.init();
		}

		// 初始化地图设置，计算尺寸
		try {
			sMapInfo = sendGet("127.0.0.1", 8111, "/map_info.json");
			mapi.update(sMapInfo);
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}

	}

	public void calculate() {
		// 计算选择目标的水平相对距离及速度及AOT
		double pys;
		double eys;
		if (mapo.slc.type != "") {

			distance = (double) Math.sqrt((mapo.slc.x - mapo.pla.x) * (mapo.slc.x - mapo.pla.x) * mapi.cmapmaxsizeX
					* mapi.cmapmaxsizeX
					+ (mapo.slc.y - mapo.pla.y) * (mapo.slc.y - mapo.pla.y) * mapi.cmapmaxsizeY * mapi.cmapmaxsizeY);
			// App.debugPrint(distance);

			if (mapo.slc.dx != 0 && distance < dislmt) {
				enemycount++;
			}

			enemyspeed = (double) (Math
					.sqrt(((mapo.slc.x - pX) * mapi.cmapmaxsizeX) * ((mapo.slc.x - pX) * mapi.cmapmaxsizeX)
							+ ((mapo.slc.y - pY) * mapi.cmapmaxsizeX) * ((mapo.slc.y - pY) * mapi.cmapmaxsizeY))
					* 1000 / (System.currentTimeMillis() - SpeedCheckMili));
			SpeedCheckMili = System.currentTimeMillis();
			pys = dxdyToangle(mapo.pla.dx, mapo.pla.dy);
			eys = dxdyToangle(mapo.slc.dx, mapo.slc.dy);
			AOT = Math.abs(pys - eys);
			if(AOT>180)AOT=360-AOT;
			// App.debugPrint(enemyspeed*3.6 );
			AZI = angleToclock(dxdyToangle(mapo.slc.x - mapo.pla.x, mapo.slc.y - mapo.pla.y) - pys);
			// App.debugPrint(mapo.slc.dx);
			// App.debugPrint(enemycount);
		}

		// 统计周围敌机数和友机数
		int i;
		for (i = 0; i < mapo.movcur; i++) {
			double sdistance = Math.sqrt(
					(mapo.mov[i].x - mapo.pla.x) * (mapo.mov[i].x - mapo.pla.x) * mapi.cmapmaxsizeX * mapi.cmapmaxsizeX
							+ (mapo.mov[i].y - mapo.pla.y) * (mapo.mov[i].y - mapo.mov[i].y) * mapi.cmapmaxsizeY
									* mapi.cmapmaxsizeY);
			if (sdistance < dislmt && sdistance < mapo.mov[i].distance) {
				if (mapo.mov[i].colorg.getBlue() > 200 || mapo.mov[i].colorg.getGreen() > 200) {
					friendcount++;
					// App.debugPrint((mapo.mov[i].type+"友军"+i+"距离"+sdistance));
				}
				if (mapo.mov[i].colorg.getRed() > 200) {
					enemycount++;
				}
			}
			mapo.mov[i].distance = sdistance;
		}

		// App.debugPrint("周围友机数" + friendcount + " 周围敌机数" + enemycount);
	}

	public void close() {
		this.isRun = false;
	}

	public void judgeOverheat() {
//		xc.judgeEngineload();
		
		/*
		// 初次
		if (!hisOverheat && isOverheat) {
			hisOverheat = true;
			// App.debugPrint("打开过热计时器");
			check = 3;// 六次检测
			xc.startOverheatTime();
		}
		// 更新过热时间
		if (hisOverheat && isOverheat) {
			// App.debugPrint("更新过热时间");
			check = 3;
			xc.updateOverheatTime();

		}
		// 如果不再接受过热消息
		if (!isOverheat) {
			if (hisOverheat) {
				if (check == 0) {
					// App.debugPrint("终结过热计时器");
					xc.endOverheatTime();
					hisOverheat = false;
					check--;

				} else {
					// App.debugPrint("不过热检查次数-1");
					check--;
				}
			}
		}
		*/
	}

	@Override
	public void run() {
		// TODO Auto-generated method stub
		while (isRun) {
			// 500毫秒执行一次
			try {
				Thread.sleep(500);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			// 取得地图数据
			// App.debugPrint("正在处理地图数据");
			enemycount = 0;
			friendcount = 0;
			try {
				if (isgetmapObj)
					sMapObj = sendGet("127.0.0.1", 8111, "/map_obj.json");
				if (isgetMsg)
					shudMsg = sendGet("127.0.0.1", 8111, "/hudmsg?lastEvt=" + lastEvt + "&lastDmg=" + lastDmg);
			} catch (IOException e) {
				// TODO Auto-generated catch block
				// e.printStackTrace();
			}
			// App.debugPrint(sMapObj);
			if (isgetmapObj)
				mapo.update(sMapObj);
			if (isgetMsg) {
				lastDmg = msg.update(shudMsg, lastDmg);
				if (msg.dmg.updated) {
					// App.debugPrint("过热检查" + msg.dmg.msg.indexOf("热") +
					// "过高检查" + msg.dmg.msg.indexOf("温"));
					if (msg.dmg.msg.indexOf(lang.oSkeyWord1) != -1
							|| msg.dmg.msg.indexOf(lang.oSkeyWord2) != -1) {
						isOverheat = true;
						// App.debugPrint("检测到过热标志" + isOverheat);
					}
				} else {

					isOverheat = false;
					// App.debugPrint("检测到不过热标志" + isOverheat);
				}
			}
			// 处理地图数据

			calculate();
			pX = mapo.slc.x;
			pY = mapo.slc.y;

			// 获得HUDMSG消息并通知玩家过热
			judgeOverheat();
			// App.debugPrint("otherService执行了");
		}

	}
}