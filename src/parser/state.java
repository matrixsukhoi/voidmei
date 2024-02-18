package parser;

import prog.app;

public class state {
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
			thrust[i] = stringHelper.getDataInt(stringHelper.getString(buf, "thrust "+i));
			if (thrust[i] != -65535)
				engineNum++;
		}
//		thrust[0] = stringHelper.getDataInt(stringHelper.getString(buf, "thrust 1"));
//		if (thrust[0] != -65535)
//			engineNum++;
//		thrust[1] = stringHelper.getDataInt(stringHelper.getString(buf, "thrust 2"));
//		if (thrust[1] != -65535)
//			engineNum++;
//		thrust[2] = stringHelper.getDataInt(stringHelper.getString(buf, "thrust 3"));
//		if (thrust[2] != -65535)
//			engineNum++;
//		thrust[3] = stringHelper.getDataInt(stringHelper.getString(buf, "thrust 4"));
//		if (thrust[3] != -65535)
//			engineNum++;
//		thrust[4] = stringHelper.getDataInt(stringHelper.getString(buf, "thrust 5"));
//		if (thrust[4] != -65535)
//			engineNum++;
	}

	public int update(String buf) {
		int i;
		valid = stringHelper.getString(buf, "valid");
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
			aileron = stringHelper.getDataInt(stringHelper.getString(buf, "aileron"));
			elevator = stringHelper.getDataInt(stringHelper.getString(buf, "elevator"));
			rudder = stringHelper.getDataInt(stringHelper.getString(buf, "rudder"));
			flaps = stringHelper.getDataInt(stringHelper.getString(buf, "flaps"));
			airbrake = stringHelper.getDataInt(stringHelper.getString(buf, "airbrake"));
			gear = stringHelper.getDataInt(stringHelper.getString(buf, "gear"));
			TAS = stringHelper.getDataInt(stringHelper.getString(buf, "TAS"));
			IAS = stringHelper.getDataInt(stringHelper.getString(buf, "IAS"));
			M = stringHelper.getDataFloat(stringHelper.getString(buf, "\"M\""));
			heightm = stringHelper.getDataFloat(stringHelper.getString(buf, "H, m"));
			AoA = stringHelper.getDataFloat(stringHelper.getString(buf, "AoA"));
			AoS = stringHelper.getDataFloat(stringHelper.getString(buf, "AoS"));
			Ny = stringHelper.getDataFloat(stringHelper.getString(buf, "Ny"));
			Vy = stringHelper.getDataFloat(stringHelper.getString(buf, "Vy"));
			Wx = stringHelper.getDataFloat(stringHelper.getString(buf, "Wx"));
			throttle = stringHelper.getDataInt(stringHelper.getString(buf, "throttle"));
			RPMthrottle = stringHelper.getDataInt(stringHelper.getString(buf, "RPM throttle"));
//			if (RPMthrottle == -65535)
//				RPMthrottle = 0;
			radiator = stringHelper.getDataInt(stringHelper.getString(buf, "radiator"));
//			if (radiator == -65535)
//				radiator = 0;
			// oilradiator = stringHelper.getDataInt(stringHelper.getString(buf, "oilraditor"));
//			power[0] = stringHelper.getDataFloat(stringHelper.getString(buf, "power 1"));
			RPM = stringHelper.getDataInt(stringHelper.getString(buf, "RPM 1"));

			manifoldpressure = stringHelper.getDataFloat(stringHelper.getString(buf, "manifold"));

			mfuel = stringHelper.getDataFloat(stringHelper.getString(buf, "Mfuel"));
			mfuel_1 = stringHelper.getDataFloat(stringHelper.getString(buf, "Mfuel 1"));
			mfuel0 = stringHelper.getDataFloat(stringHelper.getString(buf, "Mfuel0"));

			oiltemp = stringHelper.getDataFloat(stringHelper.getString(buf, "oil temp"));
//			thrust[0] = stringHelper.getDataInt(stringHelper.getString(buf, "thrust 1"));
//
//			efficiency[0] = stringHelper.getDataFloat(stringHelper.getString(buf, "efficiency 1"));
			// engineNum = 1;
			mixture = stringHelper.getDataInt(stringHelper.getString(buf, "mixture"));
			if (mixture == -65535)
				mixture = -1;
			compressorstage = stringHelper.getDataInt(stringHelper.getString(buf, "compressor stage"));
			if (compressorstage == -65535)
				compressorstage = 0;

			magenato = stringHelper.getDataInt(stringHelper.getString(buf, "magneto"));

			watertemp = stringHelper.getDataFloat(stringHelper.getString(buf, "water temp"));
			
			double tmpThrust = 0;

			int totalEngineNum = 0;
			for (i = 0; i < maxEngNum; i++) {
				// System.out.println(engineType);
				throttles[i] = stringHelper.getDataInt(stringHelper.getString(buf, "throttle " + (i+1)));
				power[i] = stringHelper.getDataFloat(stringHelper.getString(buf, "power " + (i+1)));
//				if(power[i] == -65535)break;
				thrust[i] = stringHelper.getDataInt(stringHelper.getString(buf, "thrust " + (i+1)));
				pitch[i] = stringHelper.getDataFloat(stringHelper.getString(buf, "pitch " + (i+1)));
//				if(pitch[i] == -65535)break;
				efficiency[i] = stringHelper.getDataInt(stringHelper.getString(buf, "efficiency " + (i+1)));
//				if(efficiency[i] == -65535)break;
				// System.out.println(pitch[0]);
//				System.out.println("thrust "+i+" "+thrust[i]);
				if(thrust[i] == -65535) break;
				
				tmpThrust += thrust[i];
				totalEngineNum += 1;
				
			}
			engineNum = totalEngineNum;
			totalThr = tmpThrust;
//			app.debugPrint(String.format("引擎数量%d, 功率%.0f", engineNum, power[0]));
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