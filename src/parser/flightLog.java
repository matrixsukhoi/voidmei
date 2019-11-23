package parser;

import java.io.BufferedWriter;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.FileWriter;
import java.io.IOException;
import java.util.Calendar;

import prog.controller;
import prog.language;
import prog.service;

public class flightLog implements Runnable {
	public volatile boolean doit;
	public volatile boolean logon;
	public FileOutputStream resultsFile;
	public FileWriter csv;
	public String fileName;
	public flightAnalyzer fA;
	controller xc;
	service xs;
	Calendar c;

	boolean firstAnalyze = true;

	void writeLabel(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		bw.write(language.l1);// 1

		bw.write(language.l2);// 2

		bw.write(language.l3);// 3

		bw.write(language.l4);// 4

		bw.write(language.l5);// 5

		bw.write(language.l6);// 6

		bw.write(language.l7);// 7

		bw.write(language.l8);// 8

		bw.write(language.l9);// 9

		bw.write(language.l10);// 10

		bw.write(language.l11);// 11

		bw.write(language.l12);// 12

		bw.write(language.l13);// 13

		bw.write(language.l14);// 16

		bw.write(language.l15);// 14

		bw.write(language.l16);// 15

		bw.write(language.l17);// 16

		bw.write(language.l18);// 17

		bw.write(language.l19);// 18

		bw.write(language.l20);// 19

		bw.write(language.l21);// 20

		bw.write(language.l22);// 21

		bw.write(language.l23);// 22

		bw.write(language.l24);// 23

		bw.write(language.l25);// 24

		bw.write(language.l26);// 25

		bw.write(language.l27);// 26

		bw.write(language.l28);// 27

		bw.write(language.l29);// 28

		bw.write(language.l30);// 29

		bw.write(language.l31);// 30

		bw.write("\r\n");
		bw.flush();
		bw.close();
	}

	void writeData(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		String tmp = "";

		bw.write(xs.sTime + ",");// 1

		tmp=xs.throttle;
		bw.write(tmp + ",");// 2

		bw.write(xs.IAS + ",");// 3

		bw.write(xs.TAS + ",");// 4

		bw.write(xs.M + ",");// 5

		bw.write(xs.salt + ",");// 6

		bw.write(xs.watertemp + ",");// 7

		bw.write(xs.oiltemp + ",");// 8

		bw.write(xs.Vy + ",");// 9

		bw.write(xs.sSEP + ",");// 10

		bw.write(xs.sState.Ny + ",");// 11

		bw.write(xs.Wx + ",");// 12

		bw.write(xs.stotalhp + ",");// 13

		bw.write(xs.efficiency[0] + ",");// 16

		bw.write(xs.stotalhpeff + ",");// 14

		bw.write(xs.rpm + ",");// 15

		bw.write(xs.stotalthr + ",");// 16

		bw.write(xs.acceleration + ",");// 17

		bw.write(xs.RPMthrottle + ",");// 18

		bw.write(xs.pitch[0] + ",");// 19

		bw.write(xs.radiator + ",");// 20

		bw.write(xs.mixture + ",");// 21

		bw.write(xs.sState.compressorstage + ",");// 22

		bw.write(xs.sState.magenato + ",");// 23

		bw.write(xs.manifoldpressure + ",");// 24

		bw.write(xs.flaps + ",");// 25

		bw.write(xs.sState.elevator + ",");// 26

		bw.write(xs.sState.aileron + ",");// 27

		bw.write(xs.sState.rudder + ",");// 28

		bw.write(xs.AoA + ",");// 29

		bw.write(xs.AoS + ",");// 30

		bw.write("\n");
		bw.flush();
		bw.close();
	}

	// 进行数据分析
	void analyzeData() {
		int stage = (int) xs.alt / 100;
		if (Math.abs(xs.checkAlt)>10) {
			if (firstAnalyze) {
				// 第一次分析，先取当前高度
				fA = new flightAnalyzer();
				fA.init(stage, xs);
				firstAnalyze = false;
			} else {
				// 开始分析
				fA.analyze(stage);
			}
		}

	}

	public void init(controller tc, service s) {
		xc = tc;
		xs = s;
		doit = false;
		//System.out.println("flightlog初始化了");
		c = Calendar.getInstance();
		c.setTimeInMillis(System.currentTimeMillis());
		String name=s.iIndic.type.toUpperCase();
		if(name=="NO COCKPIT")name="Unknown";
		fileName = "records/"+name+"_" +(c.get(Calendar.MONTH) + 1) + "_" + c.get(Calendar.DATE) + "_" + c.get(Calendar.HOUR)
				+ "." + c.get(Calendar.MINUTE) + "." + c.get(Calendar.SECOND) + ".csv";
		try {

			resultsFile = new FileOutputStream(fileName);
			// System.out.println("文件创建成功");
		} catch (FileNotFoundException e) {
			controller.notification(language.lfailCreate);
			// TODO Auto-generated catch block
			e.printStackTrace();
			xc.logon = false;
		}
		try {
			csv = new FileWriter(fileName, true);
			// System.out.println("打开文件成功");
		} catch (IOException e) {
			controller.notification(language.lfailCreate);
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		try {
			writeLabel(csv);
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		logon = true;
	}

	public void close() {

		logon = false;
	}

	@Override
	public void run() {
		// TODO Auto-generated method stub
		while (logon) {
			try {
				Thread.sleep(5);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				// e.printStackTrace();

			}
			//System.out.println("flightlog内存溢出测试");
			// System.out.println("执行");
			while (doit) {
				try {
					csv = new FileWriter(fileName, true);
					// System.out.println("开始写入");
					analyzeData();
					writeData(csv);

				} catch (IOException e) {
					// TODO Auto-generated catch block
					//
					controller.notification(language.lfailWrite);
					e.printStackTrace();
				}
				doit = false;// 写完后关闭
			}
		}
	}
}