package parser;

import java.io.BufferedWriter;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.FileWriter;
import java.io.IOException;
import java.util.Calendar;

import prog.controller;
import prog.lang;
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
	private String climbName;
	private String maneuverName;
	private String rollName;
	private String loadName;
	private BufferedWriter csvWritter;
	private long writeTime;

	void writeLabel(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		bw.write(lang.l1);// 1

		bw.write(lang.l2);// 2

		bw.write(lang.l3);// 3

		bw.write(lang.l4);// 4

		bw.write(lang.l5);// 5

		bw.write(lang.l6);// 6

		bw.write(lang.l7);// 7

		bw.write(lang.l8);// 8

		bw.write(lang.l9);// 9

		bw.write(lang.l10);// 10

		bw.write(lang.l11);// 11

		bw.write(lang.l12);// 12

		bw.write(lang.l13);// 13

		bw.write(lang.l14);// 16

		bw.write(lang.l15);// 14

		bw.write(lang.l16);// 15

		bw.write(lang.l17);// 16

		bw.write(lang.l18);// 17

		bw.write(lang.l19);// 18

		bw.write(lang.l20);// 19

		bw.write(lang.l21);// 20

		bw.write(lang.l22);// 21

		bw.write(lang.l23);// 22

		bw.write(lang.l24);// 23

		bw.write(lang.l25);// 24

		bw.write(lang.l26);// 25

		bw.write(lang.l27);// 26

		bw.write(lang.l28);// 27

		bw.write(lang.l29);// 28

		bw.write(lang.l30);// 29

		bw.write(lang.l31);// 30

		bw.write("\n");
		bw.flush();
//		bw.close();
	}

	void writeData(BufferedWriter bw) throws IOException {
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

		bw.write(xs.sTotalHp + ",");// 13

		bw.write(xs.efficiency[0] + ",");// 16

		bw.write(xs.sTotalHpEff + ",");// 14

		bw.write(xs.rpm + ",");// 15

		bw.write(xs.sTotalThr + ",");// 16

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
//		bw.flush();
//		bw.close();
	}

	// 进行数据分析
	void analyzeData() {
		int stage = (int) xs.alt / 100;
		if (Math.abs(xs.iCheckAlt)>10) {
			if (firstAnalyze) {
				// 第一次分析，先取当前高度
				fA = new flightAnalyzer();
				fA.init(stage, xs);
				firstAnalyze = false;
			} else {
				// 开始分析
				fA.analyze(stage);
				fA.updateEMChart(xs.IASv, xs.sState.Ny, (int)Math.abs(xs.sState.Wx), xs.SEP/9.78f, Math.abs(xs.sState.elevator), Math.abs(xs.sState.aileron));
			}
		}
		
		// 分析速度

	}
	
	void writeClimbLabel(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		// 高度
		bw.write(lang.fAlt +"/m, ");
		
		// 时间
		bw.write("t/s" + ", ");
		
		// 动力
		bw.write(lang.ePower + "/hp, ");
		
		// 推力
		bw.write(lang.eThurst + "/kgf, ");
		
		// SEP
		bw.write(lang.fSEP+"/m/s");
		
		bw.write("\n");
		bw.flush();
//		bw.close();
	}
	void writeClimbData(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		 
		for (int i = 0; i < fA.curaltStage; i++){
			
			bw.write(i * 100 + ", ");
			bw.write(fA.time[i] + ", ");
			bw.write(fA.power[i] + ", ");
			bw.write(fA.thrust[i] + ", ");
			bw.write(fA.sep[i]+"\n");
			
//			bw.write("\n");
			
		}
//		System.out.println(String.format("total %d climb data logged", fA.curaltStage));
		bw.flush();
	}
	
	public void saveClimbData(){
		FileWriter tcsv = null;
//		System.out.println("climbdata save to "+ climbName);
		try {
			tcsv = new FileWriter(climbName, true);
			// System.out.println("打开文件成功");
		} catch (IOException e) {
			controller.notification(lang.lfailCreate);
			// TODO Auto-generated catch block
			e.printStackTrace();
			return;
		}
		
		try {
			writeClimbLabel(tcsv);
			writeClimbData(tcsv);

			tcsv.close();
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
	}
	
	void writeRollLabel(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		// 速度
		bw.write(lang.fIAS +"/km/h, ");

		// 副翼
		bw.write(lang.vAileron + "/%, ");
		
		
		// 滚转率
		bw.write(lang.fWx + "/Deg/s");
		
		
		bw.write("\n");
		bw.flush();
//		bw.close();
	}
	void writeRollData(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		int k = 0;
		for (int i = 0; i < flightAnalyzer.maxIASStage; i++){
			// 速度区间
			if (fA.roll_rate[i] > 0){
				k++;
				bw.write(i * 10 +", ");
				bw.write(fA.roll_alr[i]+", ");
				bw.write(fA.roll_rate[i]+"\n");
			}
//			bw.write("\n");
			
		}
//		System.out.println(String.format("total %d roll data logged", k));
		bw.flush();
	}
	
	public void saveRollData(){
		FileWriter tcsv = null;
//		System.out.println("rolldata save to "+ climbName);
		try {
			tcsv = new FileWriter(rollName, true);
			// System.out.println("打开文件成功");
		} catch (IOException e) {
			controller.notification(lang.lfailCreate);
			// TODO Auto-generated catch block
			e.printStackTrace();
			return;
		}
		
		try {
			writeRollLabel(tcsv);
			writeRollData(tcsv);

			tcsv.close();
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
	}
	
	void writeNyLabel(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		// 速度
		bw.write(lang.fIAS +"/km/h, ");
		
		// 升降舵
		bw.write(lang.vElevator + "/%, ");
		
		// 过载
		bw.write(lang.fGL + "/G, ");
		
		// SEP
		bw.write(lang.fSEP + "/m/s");
		
		
		bw.write("\n");
		bw.flush();
//		bw.close();
	}
	void writeNyData(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		int k = 0;
		for (int i = 0; i < flightAnalyzer.maxIASStage; i++){
			// 速度区间
			if (fA.turn_load[i] > 0){
				k++;
				bw.write(i * 10 +", ");
				bw.write(fA.turn_elev[i]+", ");
				bw.write(fA.turn_load[i]+", ");
				bw.write(fA.sep_loss[i] + "\n");
			}
//			bw.write("\n");
			
		}
//		System.out.println(String.format("total %d roll data logged", k));
		bw.flush();
	}
	
	public void saveNyData(){
		FileWriter tcsv = null;
//		System.out.println("rolldata save to "+ climbName);
		try {
			tcsv = new FileWriter(loadName, true);
			// System.out.println("打开文件成功");
		} catch (IOException e) {
			controller.notification(lang.lfailCreate);
			// TODO Auto-generated catch block
			e.printStackTrace();
			return;
		}
		
		try {
			writeNyLabel(tcsv);
			writeNyData(tcsv);

			tcsv.close();
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
	}
	

	
	public void init(controller tc, service s) {
		xc = tc;
		xs = s;
		doit = false;
		//System.out.println("flightlog初始化了");
		c = Calendar.getInstance();
		c.setTimeInMillis(System.currentTimeMillis());
		String name=s.sIndic.type.toUpperCase();
		if(name=="NO COCKPIT")name="Unknown";
		fileName = "records/"+name+"_" +(c.get(Calendar.MONTH) + 1) + "_" + c.get(Calendar.DATE) + "_" + c.get(Calendar.HOUR)
				+ "." + c.get(Calendar.MINUTE) + "." + c.get(Calendar.SECOND) + ".csv";
		climbName = "records/"+name+"_" +(c.get(Calendar.MONTH) + 1) + "_" + c.get(Calendar.DATE) + "_" + c.get(Calendar.HOUR)
		+ "." + c.get(Calendar.MINUTE) + "." + c.get(Calendar.SECOND) + "_climb.csv";
		rollName = "records/"+name+"_" +(c.get(Calendar.MONTH) + 1) + "_" + c.get(Calendar.DATE) + "_" + c.get(Calendar.HOUR)
		+ "." + c.get(Calendar.MINUTE) + "." + c.get(Calendar.SECOND) + "_roll.csv";
		loadName = "records/"+name+"_" +(c.get(Calendar.MONTH) + 1) + "_" + c.get(Calendar.DATE) + "_" + c.get(Calendar.HOUR)
		+ "." + c.get(Calendar.MINUTE) + "." + c.get(Calendar.SECOND) + "_ny.csv";
		
		try {

			resultsFile = new FileOutputStream(fileName);
			// System.out.println("文件创建成功");
		} catch (FileNotFoundException e) {
			controller.notification(lang.lfailCreate);
			// TODO Auto-generated catch block
			e.printStackTrace();
			xc.logon = false;
		}
		try {
			csv = new FileWriter(fileName, true);
			// System.out.println("打开文件成功");
		} catch (IOException e) {
			controller.notification(lang.lfailCreate);
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
//		csvWritter = 
		try {
			writeLabel(csv);
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		csvWritter = new BufferedWriter(csv);
		
		logon = true;
	}

	public void close() {

		// 保存
		try {
			csvWritter.close();
			csv.close();
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		saveClimbData();
		saveRollData();
		saveNyData();
		logon = false;
	}
	public void logTick(){
		
		try {
//			csv = new FileWriter(fileName, true);
			// System.out.println("开始写入");
			analyzeData();
			writeData(csvWritter);

		} catch (IOException e) {
			// TODO Auto-generated catch block
			//
			controller.notification(lang.lfailWrite);
			e.printStackTrace();
		}
		if (writeTime++ % 1024 == 0){
			try {
				csvWritter.flush();
			} catch (IOException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
		}
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
				logTick();
				doit = false;// 写完后关闭
			}
		}
	}
}