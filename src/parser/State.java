package parser;
import prog.util.StringHelper;

import prog.Application;

public class State {
	public String valid;
	public boolean flag;
//	public boolean engineAlive = false;

	public static final int maxEngNum = 8;
	public int engineNum;
//	public int isEngineJet;
	public int aileron;
	public int elevator;
	public int rudder;
	public int flaps;
	public int gear;
	public int TAS;
	public int IAS;
	public double M;
	public double AoA;
	public double heightm;
	public double AoS;
	public double Ny;
	public double Vy;
	public double Wx;
	public int throttle;
	public int RPMthrottle;
	public int radiator;
	public int oilradiator;
	public int mixture;
	public int compressorstage;
	public int magenato;
	public double power[];
	public int RPM;
	public double manifoldpressure;
	public double watertemp;
	public double oiltemp;
	public double mfuel;
	public double mfuel_1;
	public double mfuel0;
	public double pitch[];
	public int thrust[];
	public double efficiency[];
	public int airbrake;
	public double totalThr;
	public int throttles[];
	// 临时变量

	public void init() {
		// System.out.println("state初始化了");
		valid = "false";
		throttles = new int[maxEngNum];
		power = new double[maxEngNum];
		pitch = new double[maxEngNum];
		thrust = new int[maxEngNum];
		efficiency = new double[maxEngNum];
		engineNum = 0;
//		isEngineJet = -1;
//		engineAlive = false;
		airbrake = 0;
	}

	public void getEngNum(String buf) {
		for (int i = 0; i < maxEngNum; i++) {
			thrust[i] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust "+i));
			if (thrust[i] != -65535)
				engineNum++;
		}
//		thrust[0] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust 1"));
//		if (thrust[0] != -65535)
//			engineNum++;
//		thrust[1] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust 2"));
//		if (thrust[1] != -65535)
//			engineNum++;
//		thrust[2] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust 3"));
//		if (thrust[2] != -65535)
//			engineNum++;
//		thrust[3] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust 4"));
//		if (thrust[3] != -65535)
//			engineNum++;
//		thrust[4] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust 5"));
//		if (thrust[4] != -65535)
//			engineNum++;
	}

	public int update(String buf) {
		int i;
		valid = StringHelper.getString(buf, "valid");
		// System.out.println(valid);
		if (valid == null){
			return -1;
		}
		if (valid.equals("true")) {
			// 无异常的
			flag = true;
//			if (engineNum == 0) {
//				getEngNum(buf);
//			}
//			 System.out.println(engineNum);
			aileron = StringHelper.getDataInt(StringHelper.getString(buf, "aileron"));
			elevator = StringHelper.getDataInt(StringHelper.getString(buf, "elevator"));
			rudder = StringHelper.getDataInt(StringHelper.getString(buf, "rudder"));
			flaps = StringHelper.getDataInt(StringHelper.getString(buf, "flaps"));
			airbrake = StringHelper.getDataInt(StringHelper.getString(buf, "airbrake"));
			gear = StringHelper.getDataInt(StringHelper.getString(buf, "gear"));
			TAS = StringHelper.getDataInt(StringHelper.getString(buf, "TAS"));
			IAS = StringHelper.getDataInt(StringHelper.getString(buf, "IAS"));
			M = StringHelper.getDataFloat(StringHelper.getString(buf, "\"M\""));
			heightm = StringHelper.getDataFloat(StringHelper.getString(buf, "H, m"));
			AoA = StringHelper.getDataFloat(StringHelper.getString(buf, "AoA"));
			AoS = StringHelper.getDataFloat(StringHelper.getString(buf, "AoS"));
			Ny = StringHelper.getDataFloat(StringHelper.getString(buf, "Ny"));
			Vy = StringHelper.getDataFloat(StringHelper.getString(buf, "Vy"));
			Wx = StringHelper.getDataFloat(StringHelper.getString(buf, "Wx"));
			throttle = StringHelper.getDataInt(StringHelper.getString(buf, "throttle"));
			RPMthrottle = StringHelper.getDataInt(StringHelper.getString(buf, "RPM throttle"));
//			if (RPMthrottle == -65535)
//				RPMthrottle = 0;
			radiator = StringHelper.getDataInt(StringHelper.getString(buf, "radiator"));
//			if (radiator == -65535)
//				radiator = 0;
			// oilradiator = StringHelper.getDataInt(StringHelper.getString(buf, "oilraditor"));
//			power[0] = StringHelper.getDataFloat(StringHelper.getString(buf, "power 1"));
			RPM = StringHelper.getDataInt(StringHelper.getString(buf, "RPM 1"));

			manifoldpressure = StringHelper.getDataFloat(StringHelper.getString(buf, "manifold"));

			mfuel = StringHelper.getDataFloat(StringHelper.getString(buf, "Mfuel"));
			mfuel_1 = StringHelper.getDataFloat(StringHelper.getString(buf, "Mfuel 1"));
			mfuel0 = StringHelper.getDataFloat(StringHelper.getString(buf, "Mfuel0"));

			oiltemp = StringHelper.getDataFloat(StringHelper.getString(buf, "oil temp"));
//			thrust[0] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust 1"));
//
//			efficiency[0] = StringHelper.getDataFloat(StringHelper.getString(buf, "efficiency 1"));
			// engineNum = 1;
			mixture = StringHelper.getDataInt(StringHelper.getString(buf, "mixture"));
			if (mixture == -65535)
				mixture = -1;
			compressorstage = StringHelper.getDataInt(StringHelper.getString(buf, "compressor stage"));
			if (compressorstage == -65535)
				compressorstage = 0;

			magenato = StringHelper.getDataInt(StringHelper.getString(buf, "magneto"));

			watertemp = StringHelper.getDataFloat(StringHelper.getString(buf, "water temp"));
			
			double tmpThrust = 0;

			int totalEngineNum = 0;
			for (i = 0; i < maxEngNum; i++) {
				// System.out.println(engineType);
				throttles[i] = StringHelper.getDataInt(StringHelper.getString(buf, "throttle " + (i+1)));
				power[i] = StringHelper.getDataFloat(StringHelper.getString(buf, "power " + (i+1)));
//				if(power[i] == -65535)break;
				thrust[i] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust " + (i+1)));
				pitch[i] = StringHelper.getDataFloat(StringHelper.getString(buf, "pitch " + (i+1)));
//				if(pitch[i] == -65535)break;
				efficiency[i] = StringHelper.getDataInt(StringHelper.getString(buf, "efficiency " + (i+1)));
//				if(efficiency[i] == -65535)break;
				// System.out.println(pitch[0]);
//				System.out.println("thrust "+i+" "+thrust[i]);
				if(thrust[i] == -65535) break;
				
				tmpThrust += thrust[i];
				totalEngineNum += 1;
				
			}
			engineNum = totalEngineNum;
			totalThr = tmpThrust;
//			Application.debugPrint(String.format("引擎数量%d, 功率%.0f", engineNum, power[0]));
//			if (thrust[0] != 0){
//				engineAlive = true;
//			}
//			else{
//				engineAlive = false;
//			}
//			
//			else{
//				// 推力为空且
//			}
//			if (thrust[0] != 0 || Vy != 0)
//				engineAlive = true;
//			else{
//				// dead逻辑
//				if (gear >= 0 && IAS < 10 && Vy < 1)
//					engineAlive = false;				
//			}


//			if (pitch[0] <= 0 && efficiency[0] <= 0 && power[0] == 0 && thrust[0] != 0) {
//				isEngineJet = 1;
//			} else
//				isEngineJet = 0;
			 
//			if(magenato >= 0){
//				isEngineJet = 0;
//			}
//			else{
//				isEngineJet = 1;
//			}
			
		} else {
			flag = false;
		}
		return 0;
	}
}