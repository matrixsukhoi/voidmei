package parser;

import prog.Application;

import java.io.BufferedWriter;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.FileWriter;
import java.io.IOException;
import java.util.Calendar;

import prog.Controller;
import prog.i18n.Lang;
import prog.Service;

public class FlightLog implements Runnable {
	public volatile boolean doit;
	public volatile boolean logon;
	public FileOutputStream resultsFile;
	public FileWriter csv;
	public String fileName;
	public FlightAnalyzer fA;
	Controller xc;
	Service xs;
	Calendar c;

	boolean firstAnalyze = true;
	private String climbName;
	private String maneuverName;
	private String rollName;
	private String loadName;
	private BufferedWriter csvWritter;
	private long writeTime;
	private prog.config.ConfigProvider config;

	void writeLabel(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		bw.write(Lang.l1);// 1

		bw.write(Lang.l2);// 2

		bw.write(Lang.l3);// 3

		bw.write(Lang.l4);// 4

		bw.write(Lang.l5);// 5

		bw.write(Lang.l6);// 6

		bw.write(Lang.l7);// 7

		bw.write(Lang.l8);// 8

		bw.write(Lang.l9);// 9

		bw.write(Lang.l10);// 10

		bw.write(Lang.l11);// 11

		bw.write(Lang.l12);// 12

		bw.write(Lang.l13);// 13

		bw.write(Lang.l14);// 16

		bw.write(Lang.l15);// 14

		bw.write(Lang.l16);// 15

		bw.write(Lang.l17);// 16

		bw.write(Lang.l18);// 17

		bw.write(Lang.l19);// 18

		bw.write(Lang.l20);// 19

		bw.write(Lang.l21);// 20

		bw.write(Lang.l22);// 21

		bw.write(Lang.l23);// 22

		bw.write(Lang.l24);// 23

		bw.write(Lang.l25);// 24

		bw.write(Lang.l26);// 25

		bw.write(Lang.l27);// 26

		bw.write(Lang.l28);// 27

		bw.write(Lang.l29);// 28

		bw.write(Lang.l30);// 29

		bw.write(Lang.l31);// 30

		bw.write("\n");
		bw.flush();
		// bw.close();
	}

	void writeData(BufferedWriter bw) throws IOException {
		String tmp = "";

		bw.write(xs.elapsedTime / 60000.0f + ",");// 1

		tmp = xs.throttle;
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

		bw.write(xs.iTotalThr + ",");// 16

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
		// bw.flush();
		// bw.close();
	}

	// 进行数据分析
	void analyzeData() {
		int stage = (int) xs.alt / 100;
		if (Math.abs(xs.iCheckAlt) > 10) {
			if (firstAnalyze) {
				// 第一次分析，先取当前高度
				fA = new FlightAnalyzer();
				fA.init(stage, xs, config);
				firstAnalyze = false;
			} else {
				// 开始分析
				fA.analyze(stage);
				fA.updateEMChart(xs.IASv, xs.sState.Ny, (int) Math.abs(xs.sState.Wx), xs.SEP / 9.78f,
						Math.abs(xs.sState.elevator), Math.abs(xs.sState.aileron));
			}
		}

		// 分析速度

	}

	void writeClimbLabel(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		// 高度
		bw.write(Lang.fAlt + "/m, ");

		// 时间
		bw.write("t/s" + ", ");

		// 动力
		bw.write(Lang.ePower + "/hp, ");

		// 推力
		bw.write(Lang.eThurst + "/kgf, ");

		// SEP
		bw.write(Lang.fSEP + "/m/s");

		bw.write("\n");
		bw.flush();
		// bw.close();
	}

	void writeClimbData(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);

		for (int i = 0; i < fA.curaltStage; i++) {

			bw.write(i * 100 + ", ");
			bw.write(fA.time[i] + ", ");
			bw.write(fA.power[i] + ", ");
			bw.write(fA.thrust[i] + ", ");
			bw.write(fA.sep[i] + "\n");

			// bw.write("\n");

		}
		// Application.debugPrint(String.format("total %d climb data logged",
		// fA.curaltStage));
		bw.flush();
	}

	public void saveClimbData() {
		FileWriter tcsv = null;
		// Application.debugPrint("climbdata save to "+ climbName);
		try {
			tcsv = new FileWriter(climbName, true);
			// Application.debugPrint("打开文件成功");
		} catch (IOException e) {
			ui.util.NotificationService.show(Lang.lfailCreate);
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
		bw.write(Lang.fIAS + "/km/h, ");

		// 副翼
		bw.write(Lang.vAileron + "/%, ");

		// 滚转率
		bw.write(Lang.fWx + "/Deg/s");

		bw.write("\n");
		bw.flush();
		// bw.close();
	}

	void writeRollData(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		int k = 0;
		for (int i = 0; i < FlightAnalyzer.maxIASStage; i++) {
			// 速度区间
			if (fA.roll_rate[i] > 0) {
				k++;
				bw.write(i * 10 + ", ");
				bw.write(fA.roll_alr[i] + ", ");
				bw.write(fA.roll_rate[i] + "\n");
			}
			// bw.write("\n");

		}
		// Application.debugPrint(String.format("total %d roll data logged", k));
		bw.flush();
	}

	public void saveRollData() {
		FileWriter tcsv = null;
		// Application.debugPrint("rolldata save to "+ climbName);
		try {
			tcsv = new FileWriter(rollName, true);
			// Application.debugPrint("打开文件成功");
		} catch (IOException e) {
			ui.util.NotificationService.show(Lang.lfailCreate);
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
		bw.write(Lang.fIAS + "/km/h, ");

		// 升降舵
		bw.write(Lang.vElevator + "/%, ");

		// 过载
		bw.write(Lang.fGL + "/G, ");

		// SEP
		bw.write(Lang.fSEP + "/m/s");

		bw.write("\n");
		bw.flush();
		// bw.close();
	}

	void writeNyData(FileWriter txt) throws IOException {
		BufferedWriter bw = new BufferedWriter(txt);
		int k = 0;
		for (int i = 0; i < FlightAnalyzer.maxIASStage; i++) {
			// 速度区间
			if (fA.turn_load[i] > 0) {
				k++;
				bw.write(i * 10 + ", ");
				bw.write(fA.turn_elev[i] + ", ");
				bw.write(fA.turn_load[i] + ", ");
				bw.write(fA.sep_loss[i] + "\n");
			}
			// bw.write("\n");

		}
		// Application.debugPrint(String.format("total %d roll data logged", k));
		bw.flush();
	}

	public void saveNyData() {
		FileWriter tcsv = null;
		// Application.debugPrint("rolldata save to "+ climbName);
		try {
			tcsv = new FileWriter(loadName, true);
			// Application.debugPrint("打开文件成功");
		} catch (IOException e) {
			ui.util.NotificationService.show(Lang.lfailCreate);
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

	public void init(Controller tc, Service s, prog.config.ConfigProvider config) {
		xc = tc;
		xs = s;
		this.config = config;
		doit = false;
		// Application.debugPrint("flightlog初始化了");
		c = Calendar.getInstance();
		c.setTimeInMillis(System.currentTimeMillis());
		String name = s.sIndic.type.toUpperCase();
		if (name == "NO COCKPIT")
			name = "Unknown";
		fileName = "records/" + name + "_" + (c.get(Calendar.MONTH) + 1) + "_" + c.get(Calendar.DATE) + "_"
				+ c.get(Calendar.HOUR)
				+ "." + c.get(Calendar.MINUTE) + "." + c.get(Calendar.SECOND) + ".csv";
		climbName = "records/" + name + "_" + (c.get(Calendar.MONTH) + 1) + "_" + c.get(Calendar.DATE) + "_"
				+ c.get(Calendar.HOUR)
				+ "." + c.get(Calendar.MINUTE) + "." + c.get(Calendar.SECOND) + "_climb.csv";
		rollName = "records/" + name + "_" + (c.get(Calendar.MONTH) + 1) + "_" + c.get(Calendar.DATE) + "_"
				+ c.get(Calendar.HOUR)
				+ "." + c.get(Calendar.MINUTE) + "." + c.get(Calendar.SECOND) + "_roll.csv";
		loadName = "records/" + name + "_" + (c.get(Calendar.MONTH) + 1) + "_" + c.get(Calendar.DATE) + "_"
				+ c.get(Calendar.HOUR)
				+ "." + c.get(Calendar.MINUTE) + "." + c.get(Calendar.SECOND) + "_ny.csv";

		try {

			resultsFile = new FileOutputStream(fileName);
			// Application.debugPrint("文件创建成功");
		} catch (FileNotFoundException e) {
			ui.util.NotificationService.show(Lang.lfailCreate);
			// TODO Auto-generated catch block
			e.printStackTrace();
			xc.logon = false;
		}
		try {
			csv = new FileWriter(fileName, true);
			// Application.debugPrint("打开文件成功");
		} catch (IOException e) {
			ui.util.NotificationService.show(Lang.lfailCreate);
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		// csvWritter =
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

	public void logTick() {

		try {
			// csv = new FileWriter(fileName, true);
			// Application.debugPrint("开始写入");
			analyzeData();
			writeData(csvWritter);

		} catch (IOException e) {
			// TODO Auto-generated catch block
			//
			ui.util.NotificationService.show(Lang.lfailWrite);
			e.printStackTrace();
		}
		if (writeTime++ % 1024 == 0) {
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
			// Application.debugPrint("flightlog内存溢出测试");
			// Application.debugPrint("执行");
			while (doit) {
				logTick();
				doit = false;// 写完后关闭
			}
		}
	}
}