package parser;

import prog.util.StringHelper;

import prog.Application;

public class State {
	public String valid;
	public boolean flag;

	public static final int maxEngNum = 8;
	public int engineNum;

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

		airbrake = 0;
	}

	public void getEngNum(String buf) {
		for (int i = 0; i < maxEngNum; i++) {
			thrust[i] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust " + i));
			if (thrust[i] != -65535)
				engineNum++;
		}

	}

	public int update(String buf) {
		int i;
		valid = StringHelper.getString(buf, "valid");
		// System.out.println(valid);
		if (valid == null) {
			return -1;
		}
		if (valid.equals("true")) {
			// 无异常的
			flag = true;

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

			radiator = StringHelper.getDataInt(StringHelper.getString(buf, "radiator"));

			RPM = StringHelper.getDataInt(StringHelper.getString(buf, "RPM 1"));

			manifoldpressure = StringHelper.getDataFloat(StringHelper.getString(buf, "manifold pressure 1"));

			mfuel = StringHelper.getDataFloat(StringHelper.getString(buf, "Mfuel"));
			mfuel_1 = StringHelper.getDataFloat(StringHelper.getString(buf, "Mfuel 1"));
			mfuel0 = StringHelper.getDataFloat(StringHelper.getString(buf, "Mfuel0"));

			oiltemp = StringHelper.getDataFloat(StringHelper.getString(buf, "oil temp"));

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
				throttles[i] = StringHelper.getDataInt(StringHelper.getString(buf, "throttle " + (i + 1)));
				power[i] = StringHelper.getDataFloat(StringHelper.getString(buf, "power " + (i + 1)));

				thrust[i] = StringHelper.getDataInt(StringHelper.getString(buf, "thrust " + (i + 1)));
				pitch[i] = StringHelper.getDataFloat(StringHelper.getString(buf, "pitch " + (i + 1)));

				efficiency[i] = StringHelper.getDataInt(StringHelper.getString(buf, "efficiency " + (i + 1)));

				if (thrust[i] == -65535)
					break;

				tmpThrust += thrust[i];
				totalEngineNum += 1;
			}
			engineNum = totalEngineNum;
			totalThr = tmpThrust;
		} else {
			flag = false;
		}
		return 0;
	}
}