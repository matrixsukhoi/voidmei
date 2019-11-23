package parser;

public class state {
	public String valid;
	public boolean flag;
	public boolean engineAlive=false;

	
	public volatile int engineNum;
	public volatile int engineType;
	public volatile int aileron;
	public volatile int elevator;
	public volatile int rudder;
	public volatile int flaps;
	public volatile int gear;
	public volatile int TAS;
	public volatile int IAS;
	public volatile float M;
	public volatile float AoA;
	public volatile float AoS;
	public volatile float Ny;
	public volatile float Vy;
	public volatile float Wx;
	public volatile int throttle;
	public volatile int RPMthrottle;
	public volatile int radiator;
	public volatile int mixture;
	public volatile int compressorstage;
	public volatile int magenato;
	public volatile float power[];
	public volatile int RPM;
	public volatile float manifoldpressure;
	public volatile float watertemp;
	public volatile float oiltemp;
	public volatile float pitch[];
	public volatile int thrust[];
	public volatile float efficiency[];
	public volatile int airbrake;

	// 临时变量
	int i;

	public String getString(String R, String S) {
		int bix;
		int eix;
		bix = R.indexOf(S);
		if (bix >= 0) {
			eix = bix;
			while (R.charAt(eix) != ':') {
				eix++;
			}
			eix++;
			bix = eix + 1;
			while (R.charAt(eix) != ',' && R.charAt(eix) != '}') {
				eix++;
				if (eix == R.length() + 1)
					break;
			}
			return R.substring(bix, eix);
		} else
			return null;
	}

	public float getDatafloat(String sdata) {
		if (sdata != null)
			return Float.parseFloat(sdata);
		else
			return -65535;
	}

	public int getDateint(String sdata) {
		if (sdata != null)
			return Integer.parseInt(sdata);
		else
			return -65535;
	}

	public void init() {
		//System.out.println("state初始化了");
		valid = "false";
		power = new float[5];
		pitch = new float[5];
		thrust = new int[5];
		efficiency = new float[5];
		engineNum = 0;
		engineType = -1;
	}

	public void update(String buf) {
		valid = getString(buf, "valid");
		// System.out.println(valid);
		flag = false;
		if (valid.equals("true")) {
			// 无异常的
			flag = true;
			if (engineNum == 0&&engineAlive) {
				thrust[0] = getDateint(getString(buf, "thrust 1"));
				if (thrust[0] != -65535)
					engineNum++;
				thrust[1] = getDateint(getString(buf, "thrust 2"));
				if (thrust[1] != -65535)
					engineNum++;
				thrust[2] = getDateint(getString(buf, "thrust 3"));
				if (thrust[2] != -65535)
					engineNum++;
				thrust[3] = getDateint(getString(buf, "thrust 4"));
				if (thrust[3] != -65535)
					engineNum++;
				thrust[4] = getDateint(getString(buf, "thrust 5"));
				if (thrust[4] != -65535)
					engineNum++;
			}
			aileron = getDateint(getString(buf, "aileron"));
			elevator = getDateint(getString(buf, "elevator"));
			rudder = getDateint(getString(buf, "rudder"));
			flaps = getDateint(getString(buf, "flaps"));
			airbrake=getDateint(getString(buf, "airbrake"));
			gear = getDateint(getString(buf, "gear"));
			TAS = getDateint(getString(buf, "TAS"));
			IAS = getDateint(getString(buf, "IAS"));
			M = getDatafloat(getString(buf, "M"));
			AoA = getDatafloat(getString(buf, "AoA"));
			AoS = getDatafloat(getString(buf, "AoS"));
			Ny = getDatafloat(getString(buf, "Ny"));
			Vy = getDatafloat(getString(buf, "Vy"));
			Wx = getDatafloat(getString(buf, "Wx"));
			throttle = getDateint(getString(buf, "throttle"));
			RPMthrottle = getDateint(getString(buf, "RPM throttle"));
			radiator = getDateint(getString(buf, "radiator"));
			power[0] = getDatafloat(getString(buf, "power 1"));
			RPM = getDateint(getString(buf, "RPM 1"));
			manifoldpressure = getDatafloat(getString(buf, "manifold"));

			oiltemp = getDatafloat(getString(buf, "oil temp"));
			thrust[0] = getDateint(getString(buf, "thrust 1"));

			efficiency[0] = getDatafloat(getString(buf, "efficiency 1"));
			// engineNum = 1;
			mixture = getDateint(getString(buf, "mixture"));
			compressorstage = getDateint(getString(buf, "compressor stage"));
			magenato = getDateint(getString(buf, "magneto"));

			watertemp = getDatafloat(getString(buf, "water temp"));

			for (i = 0; i < engineNum; i++) {
				// System.out.println(engineType);
				
				power[i] = getDatafloat(getString(buf, "power " + (i + 1)));
				thrust[i] = getDateint(getString(buf, "thrust " + (i + 1)));
				pitch[i] = getDatafloat(getString(buf, "pitch " + (i + 1)));
				efficiency[i] = getDateint(getString(buf, "efficiency " + (i + 1)));
				//System.out.println("power "+i+" "+power[i]);
				// System.out.println(efficiency[0]);
			}
			//System.out.println(power[0]);
			if(thrust[0]!=0||Vy!=0)engineAlive=true;
			if (power[0] == 0 && thrust[0]!=0){
				engineType = 1;
			}
			else
				engineType = 0;

		}

	}
}