package parser;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.IOException;

import prog.lang;

public class blkx {
	public boolean valid;

	public class XY {
		public double x[];
		public double y[];
		public int cur;

		XY(int num) {
			x = new double[num];
			y = new double[num];
			cur = 0;
		}
	}

	public String data;
	public String readFileName;
	public XY loc;// WEP爬升
	public XY loc0;// NOM爬升
	public XY loc1;// WEP速度
	public XY loc2;// NOM速度
	public XY loc3;// 滚转
	public XY plotdata[];

	// 发动机负载相关
	public String fmdata;

	public class engineLoad {
		public float WaterLimit;
		public float OilLimit;
		public float WorkTime;
		public float RecoverTime;
		public float curWaterWorkTimeMili;
		public float curOilWorkTimeMili;
	}

	public engineLoad[] engLoad;
	public int maxEngLoad;
	public Float oilload0;
	public Float oilload1;
	public Float oilload2;
	public Float oilload3;
	public Float oilload4;
	public Float oilload5;
	public Float wtload0;
	public Float wtload1;
	public Float wtload2;
	public Float wtload3;
	public Float wtload4;
	public Float wtload5;
	public int tmload1;
	public int tmload2;
	public int tmload3;
	public int tmload4;
	public int tmload5;

	public float vne;
	public float clmax;
	public float aoaHigh;
	public float aoaLow;
	public float flapAoaHigh;
	public float flapAoaLow;

	public float aoaFuselageHigh;
	public float aoaFuselageLow;

	public float flapClmax;
	public float emptyweight;

	public float[] maxAllowGload;

	public int emptyweightToLoad;
	public float aileronEff;
	public float aileronPowerLoss;
	public float rudderEff;
	public float rudderPowerLoss;
	public float elavEff;
	public float elavPowerLoss;
	public float nitro;

	public float grossweight;

	public float oil;

	public float nitroDecr;
	public float maxfuelweight;

	public float FmCdMin;

	public float WingAngle;
	public float StabAngle;
	public float KeelAngle;

	public float RadiatorCd;
	public float OilRadiatorCd;
	public float AirbrakeCd;
	public float OswaldsEfficiencyNumber;

	public class fm_parts {
		public String name;
		
		public float Sq;
		public float CdMin;

		public float Cl0;

		public float ClCritHigh;
		public float ClCritLow;

		public float ClAfterCrit;

		public float AoACritHigh;
		public float AoACritLow;

		public float lineClCoeff;

		// 翼展效率因数，影响诱导阻力，因数越大阻力越小
		// public float oswaldEff;

	}

	public fm_parts NoFlapsWing;
	public fm_parts FullFlapsWing;

	public fm_parts Fuselage;
	public fm_parts Fin;
	public fm_parts Stab;
	public float SweptWingAngle;
	public float WingTaperRatio;
	public float CriticalSpeed;
	public float AWingLeftIn;
	public float AWingLeftMid;
	public float AWingLeftOut;
	public float AWingRightIn;
	public float AWingRightMid;
	public float AWingRightOut;
	public float AFuselage;
	public float AWing;
	public float NoFlapWLL;
	public float FullFlapWLL;
	public float CdS;

	public float[] MomentOfInertia;
	public float AAileron;
	public float Wingspan;
	public float AspectRatio;
	public double indCdF;
	public String version;
	public float avgEngRecoveryRate;
	public int FlapsDestructionNum;
	public float[][] FlapsDestructionIndSpeed;
	private float halfweight;

	// public

	public void getPartsFm(String c, fm_parts p) {
		p.name = c;
		p.CdMin = getfloat(c + ".CdMin");
		p.Cl0 = getfloat(c + ".Cl0");
		p.ClCritHigh = getfloat(c + ".ClCritHigh");
		p.ClCritLow = getfloat(c + ".ClCritLow");

		p.ClAfterCrit = getfloat(c + ".ClAfterCrit");
		p.lineClCoeff = getfloat(c + ".lineClCoeff");

		p.AoACritHigh = getfloat(c + ".alphaCritHigh");
		p.AoACritLow = getfloat(c + ".alphaCritLow");

	}

	public boolean getEngineLoad(engineLoad[] eL, int loadIndex) {
		String c = "Load" + loadIndex;
		// System.out.println(c);
		eL[loadIndex].WaterLimit = getfloat(c + ".WaterTemperature");
		// System.out.println(eL[loadIndex].WaterLimit);
		if (eL[loadIndex].WaterLimit == 0)
			return Boolean.FALSE;
		eL[loadIndex].OilLimit = getfloat(c + ".OilTemperature");
		if (eL[loadIndex].OilLimit == 0)
			return Boolean.FALSE;
		eL[loadIndex].WorkTime = getfloat(c + ".WorkTime");
		// if(eL[loadIndex].WorkTime == 0)
		eL[loadIndex].RecoverTime = getfloat(c + ".RecoverTime");
		// eL[loadIndex].curWorkTimeMili = eL[loadIndex].WorkTime * 1000;
		eL[loadIndex].curWaterWorkTimeMili = eL[loadIndex].WorkTime * 1000;
		eL[loadIndex].curOilWorkTimeMili = eL[loadIndex].WorkTime * 1000;
		return Boolean.TRUE;
	}

	public void showEngineLoad(engineLoad[] eL, int loadIndex) {
		String c = "Load" + loadIndex;
		c = c.concat("水温/油温限制: [" + eL[loadIndex].WaterLimit + "," + eL[loadIndex].OilLimit + "]\n");
		c = c.concat("加力/恢复时间: [" + eL[loadIndex].WorkTime + "," + eL[loadIndex].RecoverTime + "]\n");
		System.out.println(c);
	}

	public String WritePartsFm(String s, fm_parts p) {
		s = s.concat(String.format(lang.bFmParts, p.name));
		s = s.concat(String.format(lang.bCdMin, p.CdMin));
		s = s.concat(String.format(lang.bCl0,  p.Cl0));
		s = s.concat(String.format(lang.bAoACrit,  p.AoACritLow, p.AoACritHigh));
		s = s.concat(String.format(lang.bAoACritCl,  p.ClCritLow, p.ClCritHigh));
		
//		s = s.concat("------fm器件 " + p.name + "------\n");
//		s = s.concat("零升阻力系数:" + p.CdMin + "\n");
//		s = s.concat("零攻角升力:" + p.Cl0 + "\n");
//		s = s.concat("临界攻角:[" + p.AoACritLow + ", " + p.AoACritHigh + "]" + "\n");
//		s = s.concat("临界攻角升力系数:[" + p.ClCritLow + ", " + p.ClCritHigh + "]" + "\n");
		//
		// System.out.println("------fm器件 "+p.name+"------");
		// System.out.println("零升阻力系数:"+p.CdMin);
		// System.out.println("零攻角升力:"+p.Cl0);
		// System.out.println("临界攻角:["+p.AoACritLow+","+p.AoACritHigh+"]");
		// System.out.println("临界攻角升力系数:["+p.ClCritLow+","+p.ClCritHigh+"]");
		return s;
	}

	public float[] getfloats(String c, float[] ret, int num) {
		if (num <= 0)
			return null;

		if (!getone(c).equals("null")) {
			String[] tmp = getone(c).split(",");
			for (int i = 0; i < num; i++) {
				try {

					ret[i] = Float.parseFloat(tmp[i]);
				} catch (Exception e) {
					System.out.println("getfloat error" + c);
					return null;
				}
			}
		}
		return ret;

	}

	public float getfloat(String c) {
		float ret = 0;
		if (!getone(c).equals("null")) {
			String[] tmp = getone(c).split(",");
			try {
				ret = Float.parseFloat(tmp[0]);
			} catch (Exception e) {
				System.out.println("getfloat error" + c);
				return 0;
			}
		}
		return ret;
	}

	public float getfloat_exc(String c) {
		float ret = Float.MAX_VALUE;
		if (!getone(c).equals("null")) {
			String[] tmp = getone(c).split(",");
			try {
				ret = Float.parseFloat(tmp[0]);
			} catch (Exception e) {
				System.out.println("getfloat error" + c);
				return 0;
			}
		}
		return ret;
	}

	public int findmaxLoad(engineLoad[] eL, float water, float oil) {
		for (int i = 0; i < maxEngLoad; i++) {
			// 大于还是小于等于呢？
			if (water < eL[i].WaterLimit && oil < eL[i].OilLimit)
				return i;
		}

		return maxEngLoad;
	}

	public int findmaxWaterLoad(engineLoad[] eL, float water) {
		for (int i = 0; i < maxEngLoad; i++) {
			// 大于还是小于等于呢？
			if (water < eL[i].WaterLimit)
				return i;
		}

		return maxEngLoad;
	}

	public int findmaxOilLoad(engineLoad[] eL, float oil) {
		for (int i = 0; i < maxEngLoad; i++) {
			// 大于还是小于等于呢？
			if (oil < eL[i].OilLimit)
				return i;
		}

		return maxEngLoad;
	}

	public String getVersion() {
		File file = new File("./data/aces/version");
		String tmp_data = null;
		if (file.exists()) {
			StringBuilder sb = new StringBuilder();
			String s = "";
			BufferedReader br;
			try {
				br = new BufferedReader(new FileReader(file));
				while ((s = br.readLine()) != null) {
					sb.append(s + "\n");
				}
				br.close();
			} catch (FileNotFoundException e1) {
				// TODO Auto-generated catch block
				e1.printStackTrace();
			} catch (IOException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			tmp_data = sb.toString();
			// valid = true;
		} else {
			// valid = false;
		}
		return tmp_data;
	}

	public float altitudeThr[];
	public float velocityThr[];
	public float maxThrCoff[][];
	public float maxThrAftCoff[][];
	public float maxThr[][];
	public float maxThrAft[][];
	public float thrMax0; // 静推力
	public float aftbCoff;
	public int altThrNum;
	public int velThrNum;
	public boolean isJet;
	public int engineNum;

	public int compNumSteps;
	public float compAlt[];
	public float compPower[];

	public float compCeil[];
	public float compCeilPwr[];
	
	public float compBoost[];
	public float compRpmRatio[];
	
	public float modeEngineMult[];
	

	// 冲压系数
	public float speedToManifoldMultiplier;
	private int modeEngineNum;
	private float AWingRightCut;
	private float AWingLeftCut;
	public float vneMach;
	public float GearDestructionIndSpeed;
	public float maxRPM;
	public float maxAllowedRPM;
	private float Cl_a;
	private float[] AileronDefl;
	private float Wx100;
	private float Wx_vcoff;
	private float Wx250;
	private float Wx300;
	private float Wx350;
	private float WxMax;
	private float Wx600;

	public void getload() {
		// String Load0 = cut(data, "Load0");
		// System.out.println(getone("Load0.WaterTemperature"));
		isJet = false;
		
		// 读取推力高度
		engineNum = 1;
//		System.out.println(getone("EngineType0.Main.Type"));
		String hdrString = "EngineType0.";
		String res = getone("EngineType0.Main.Type");
//		System.out.println(res);
		if (res.equals("\"Jet\"")) {
			// 判断喷气
			isJet = true;
			System.out.println(getone("Engine"+engineNum));
			while (!getone("Engine"+engineNum).equals("null")){
				engineNum++;
			}
		} else {
			if (res.equals("null")) {
				hdrString = "Engine0.";
				if (getone("Engine0.Main.Type").equals("\"Jet\"")) {
					isJet = true;
					while (!getone("Engine"+engineNum).equals("null")){
						engineNum++;
					}
				}
			}
		}
		if (isJet) {
			aftbCoff = getfloat(hdrString + "Main.AfterburnerBoost");
			thrMax0 = getfloat("ThrustMax.ThrustMax0");
//			thrMax0 = getfloat(hdrString + "Main.Thrust");
			
			System.out.println("engineType: jet, afterburner coeff" + aftbCoff);
			altThrNum = 0;
			altitudeThr = new float[30];
			for (int i = 0; i < 30; i++, altThrNum++) {
				altitudeThr[i] = getfloat_exc("ThrustMax.Altitude_" + i);
				if (altitudeThr[i] == Float.MAX_VALUE){
					altitudeThr[i] = 0;
					break;
				}
				// System.out.println(altitudeThr[i]);
			}
			// 读取推力速度
			velThrNum = 0;
			velocityThr = new float[30];

			for (int i = 0; i < 30; i++, velThrNum++) {
				velocityThr[i] = getfloat_exc("ThrustMax.Velocity_" + i);
				if (velocityThr[i] == Float.MAX_VALUE){
					velocityThr[i] = 0;
					break;
				}
				// System.out.println(altitudeThr[i]);
			}
			
			// 读取发动机工作模式

			modeEngineNum = 0;
			modeEngineMult = new float[10];
			for (int i = 0; i < 10; i++, modeEngineNum++){
				modeEngineMult[i] = getfloat_exc("Mode"+i+".ThrustMult");
				if (modeEngineMult[i] == Float.MAX_VALUE){
					modeEngineMult[i] = 0;
					break;
				}	
			}
			
			
			float engineMultWEP = 1.0f;
			
			if (modeEngineNum != 0){
				engineMultWEP = modeEngineMult[modeEngineNum - 1];
			}
			// 读取推力系数包线
			maxThrCoff = new float[altThrNum][];
			maxThr = new float[altThrNum][];
			maxThrAft = new float[altThrNum][];
			maxThrAftCoff = new float[altThrNum][];
			for (int i = 0; i < altThrNum; i++) {
				maxThrCoff[i] = new float[velThrNum];
				maxThr[i] = new float[velThrNum];
				maxThrAft[i] = new float[velThrNum];
				maxThrAftCoff[i] = new float[velThrNum];
				
				for (int j = 0; j < velThrNum; j++) {
					maxThrCoff[i][j] = getfloat("ThrustMax.ThrustMaxCoeff_" + i + "_" + j);
					maxThrAftCoff[i][j] = getfloat("ThrustMax.ThrAftMaxCoeff_"+ i +"_" + j);
					if(maxThrAftCoff[i][j] == 0){
						maxThrAftCoff[i][j] = 1.0f;
					}
					maxThr[i][j] = thrMax0 * maxThrCoff[i][j] * engineNum;
					maxThrAft[i][j] = thrMax0 * maxThrCoff[i][j] * aftbCoff * maxThrAftCoff[i][j] * engineMultWEP * engineNum;
					System.out.println(String.format("[%.0f]%.0f:%.0f kgf", altitudeThr[i], velocityThr[j], maxThrAft[i][j]));
				}
			}
		} else {
			// radial inline
			// 获得增压器工作高度
			System.out.println("not a jet");
			aftbCoff = getfloat(hdrString + "Main.AfterburnerBoost");
			System.out.println(hdrString);
			compNumSteps = (int) getfloat("Compressor.NumSteps");
			speedToManifoldMultiplier = getfloat("Compressor.SpeedManifoldMultiplier");

			compAlt = new float[compNumSteps];
			compBoost = new float[compNumSteps];
			compPower = new float[compNumSteps];
			compRpmRatio = new float[compNumSteps];
			compCeil = new float[compNumSteps];
			compCeilPwr = new float[compNumSteps];;
			
			
			for (int i = 0; i < compNumSteps; i++) {
				compAlt[i] = getfloat("Compressor.Altitude" + i);
				compPower[i] = getfloat("Compressor.Power" + i);
				compBoost[i] = getfloat("Compressor.AfterburnerBoostMul" + i);
				compRpmRatio[i] = getfloat("Compressor.PowerConstRPMCurvature" + i);
				compCeil[i] = getfloat("Compressor.Ceiling" + i);
				compCeilPwr[i] = getfloat("Compressor.PowerAtCeiling"+i);
//				System.out.println(String.format("*s%d*:[%.0f]%.0fhp - [%.0f]%.0fhp", i, compAlt[i], compPower[i] * compRpmRatio[i] * aftbCoff, compCeil[i], compCeilPwr[i] * compRpmRatio[i] * aftbCoff));
			}

			//
		}

		// 读取最大转速和最大允许转速
		maxRPM = getfloat("RPMAfterburner");
		if (maxRPM == 0) maxRPM = getfloat(" RPMMax");
		maxAllowedRPM = getfloat("RPMMaxAllowed");
				
		avgEngRecoveryRate = 0.0f;
		version = getVersion();

		engLoad = new engineLoad[10];
		for (int i = 0; i < 10; i++) {
			engLoad[i] = new engineLoad();
		}
		maxEngLoad = 0;
		do {

		} while (getEngineLoad(engLoad, maxEngLoad++));

		maxEngLoad -= 1;
		engLoad[maxEngLoad].WaterLimit = 999;
		engLoad[maxEngLoad].OilLimit = 999;

		for (int i = 0; i < maxEngLoad; i++) {
			if (engLoad[i].RecoverTime != 0)
				avgEngRecoveryRate = avgEngRecoveryRate + engLoad[i].WorkTime / engLoad[i].RecoverTime;
			showEngineLoad(engLoad, i);
		}
		// System.out.println(engLoad[0].WorkTime/engLoad[i].RecoverTime);
		avgEngRecoveryRate = avgEngRecoveryRate / (maxEngLoad - 1);
		emptyweight = getfloat("EmptyMass");
		vne = getfloat("Vne:");
		if(vne == 0){
			vne = getfloat("WingPlane.Strength.VNE");
			if(vne == 0){
				vne = getfloat("WingPlaneSweep1.Strength.VNE");
			}
		}
		vneMach = getfloat("VneMach");
		if(vneMach == 0){
			vneMach = getfloat("WingPlane.Strength.MNE");
			if(vneMach == 0){
				vneMach = getfloat("WingPlaneSweep1.Strength.MNE");
			}
		}
		
		aileronEff = getfloat("AileronEffectiveSpeed");
		aileronPowerLoss = getfloat("AileronPowerLoss");
		rudderEff = getfloat("RudderEffectiveSpeed");
		rudderPowerLoss = getfloat("RudderPowerLoss");
		elavEff = getfloat("ElevatorsEffectiveSpeed");
		elavPowerLoss = getfloat("ElevatorPowerLoss");
		maxfuelweight = getfloat("MaxFuelMass0");

		clmax = getfloat("NoFlaps.ClCritHigh");
		flapClmax = getfloat("FullFlaps.ClCritHigh");

		aoaHigh = getfloat("NoFlaps.alphaCritHigh");
		aoaLow = getfloat("NoFlaps.alphaCritLow");

		flapAoaHigh = getfloat("FullFlaps.alphaCritHigh");
		flapAoaLow = getfloat("FullFlaps.alphaCritLow");

		nitroDecr = getfloat("NitroConsumption");
		nitro = getfloat("MaxNitro");
		oil = getfloat("OilMass");

		System.out.println("作战空重" + (emptyweight + oil + nitro));
		grossweight = emptyweight + maxfuelweight + nitro + oil;
		halfweight = emptyweight + maxfuelweight / 2 + nitro + oil;

		RadiatorCd = getfloat("RadiatorCd");
		OilRadiatorCd = getfloat("OilRadiatorCd");
		OswaldsEfficiencyNumber = getfloat("OswaldsEfficiencyNumber");

		SweptWingAngle = getfloat("SweptWingAngle");
		if (SweptWingAngle == 0) {
			SweptWingAngle = getfloat("WingPlane.SweptAngle");
			if (SweptWingAngle == 0) {
				SweptWingAngle = getfloat("WingPlaneSweep0.SweptAngle");
			}
		}

		WingTaperRatio = getfloat("WingTaperRatio");
		if (WingTaperRatio == 0) {
			WingTaperRatio = getfloat("WingPlane.TaperRatio");
			if (WingTaperRatio == 0) {
				WingTaperRatio = getfloat("WingPlaneSweep0.TaperRatio");
			}
		}

		CriticalSpeed = getfloat("CriticalSpeed");

		FlapsDestructionIndSpeed = new float[5][2];
		FlapsDestructionNum = 0;
		{
			int p = 0;
			while (true) {
				getfloats("FlapsDestructionIndSpeedP" + (p++), FlapsDestructionIndSpeed[FlapsDestructionNum], 2);
				if (p >= 5)
					break;
				if (FlapsDestructionIndSpeed[FlapsDestructionNum][1] == 0)
					continue;
				// if (p >= 5)
				// break;
				FlapsDestructionNum++;
			}
		}
		if (FlapsDestructionNum == 0){
			FlapsDestructionIndSpeed[0][0] = 1.0f;
			FlapsDestructionIndSpeed[0][1] = getfloat("FlapsDestructionIndSpeed");
		}
//		if (FlapsDestructionIndSpeed[0][1] != 0){
//			FlapsDestructionNum++;
//		}
		GearDestructionIndSpeed = getfloat("GearDestructionIndSpeed");
		
		// 面积
		AWingLeftIn = getfloat("Areas.WingLeftIn");
		if (AWingLeftIn == 0)
			AWingLeftIn = getfloat("WingPlane.Areas.LeftIn");
		if (AWingLeftIn == 0)
			AWingLeftIn = getfloat("WingPlaneSweep0.Areas.LeftIn");

		AWingLeftMid = getfloat("Areas.WingLeftMid");
		if (AWingLeftMid == 0)
			AWingLeftMid = getfloat("WingPlane.Areas.LeftMid");
		if (AWingLeftMid == 0)
			AWingLeftMid = getfloat("WingPlaneSweep0.Areas.LeftMid");

		AWingLeftOut = getfloat("Areas.WingLeftOut");
		if (AWingLeftOut == 0)
			AWingLeftOut = getfloat("WingPlane.Areas.LeftOut");
		if (AWingLeftOut == 0)
			AWingLeftOut = getfloat("WingPlaneSweep0.Areas.LeftOut");

		AWingLeftCut = getfloat("Areas.WingLeftCut");
		if (AWingLeftCut == 0)
			AWingLeftCut = getfloat("WingPlane.Areas.LeftCut");
		if (AWingLeftCut == 0)
			AWingLeftCut = getfloat("WingPlaneSweep0.Areas.LeftCut");

		
		AWingRightIn = getfloat("Areas.WingRightIn");
		if (AWingRightIn == 0)
			AWingRightIn = getfloat("WingPlane.Areas.RightIn");
		if (AWingRightIn == 0)
			AWingRightIn = getfloat("WingPlaneSweep0.Areas.RightIn");

		AWingRightMid = getfloat("Areas.WingRightMid");
		if (AWingRightMid == 0)
			AWingRightMid = getfloat("WingPlane.Areas.RightMid");
		if (AWingRightMid == 0)
			AWingRightMid = getfloat("WingPlaneSweep0.Areas.RightMid");

		AWingRightOut = getfloat("Areas.WingRightOut");
		if (AWingRightOut == 0)
			AWingRightOut = getfloat("WingPlane.Areas.RightOut");
		if (AWingRightOut == 0)
			AWingRightOut = getfloat("WingPlaneSweep0.Areas.RightOut");

		AWingRightCut = getfloat("Areas.WingRightCut");
		if (AWingRightCut == 0)
			AWingRightCut = getfloat("WingPlane.Areas.RightCut");
		if (AWingRightCut == 0)
			AWingRightCut = getfloat("WingPlaneSweep0.Areas.RightCut");

		
		AAileron = getfloat("Areas.Aileron");
		if (AAileron == 0)
			AAileron = getfloat("WingPlane.Areas.Aileron");
		if (AAileron == 0)
			AAileron = getfloat("WingPlaneSweep0.Areas.Aileron");

		AFuselage = getfloat("Areas.Fuselage");
		if (AFuselage == 0)
			AFuselage = getfloat("FuselagePlane.Areas.Main");
		if (AFuselage == 0)
			AFuselage = getfloat("WingPlaneSweep0.Areas.Main");
		
		AFuselage = getfloat("Areas.Fuselage");
		if (AFuselage == 0)
			AFuselage = getfloat("FuselagePlane.Areas.Main");
		if (AFuselage == 0)
			AFuselage = getfloat("WingPlaneSweep0.Areas.Main");

		
		
		NoFlapsWing = new fm_parts();
		getPartsFm("NoFlaps", NoFlapsWing);
		if (NoFlapsWing.AoACritHigh == 0) {
			getPartsFm("FlapsPolar0", NoFlapsWing);
		}

		FullFlapsWing = new fm_parts();
		getPartsFm("FullFlaps", FullFlapsWing);
		if (FullFlapsWing.AoACritHigh == 0) {
			getPartsFm("FlapsPolar1", FullFlapsWing);
		}

		Fuselage = new fm_parts();
		getPartsFm("Fuselage", Fuselage);
		if (Fuselage.AoACritHigh == 0) {
			getPartsFm("FuselagePlane.Polar", Fuselage);
		}
		aoaFuselageHigh = Fuselage.AoACritHigh;
		aoaFuselageLow = Fuselage.AoACritLow;

		Fin = new fm_parts();
		getPartsFm("Fin", Fin);
		if (Fin.AoACritHigh == 0) {
			getPartsFm("HorStabPlane.Polar", Fin);
		}

		Stab = new fm_parts();
		getPartsFm("Stab", Stab);
		if (Stab.AoACritHigh == 0) {
			getPartsFm("VerStabPlane.Polar", Stab);
		}

		// 获得安装角
		WingAngle = getfloat("\nWingAngle");
		if (WingAngle == 0) {
			WingAngle = getfloat("WingPlane. Angle");
			if (WingAngle == 0) {
				WingAngle = getfloat("WingPlaneSweep0. Angle");
			}
		}
		System.out.println("机翼安装角" + WingAngle);

		StabAngle = getfloat("StabAngle");
		if (WingAngle == 0) {
			WingAngle = getfloat("VerStabPlane.Angle");
		}

		KeelAngle = getfloat("KeelAngle");
		if (WingAngle == 0) {
			WingAngle = getfloat("FuselagePlane.Angle");
		}

		// 计算安装角补偿
		NoFlapsWing.AoACritHigh -= WingAngle;
		NoFlapsWing.AoACritLow -= WingAngle;
		FullFlapsWing.AoACritHigh -= WingAngle;
		FullFlapsWing.AoACritLow -= WingAngle;

		Fuselage.AoACritHigh -= KeelAngle;
		Fuselage.AoACritLow -= KeelAngle;

		Stab.AoACritHigh -= StabAngle;
		Stab.AoACritLow -= StabAngle;

		MomentOfInertia = new float[3];
		getfloats("MomentOfInertia", MomentOfInertia, 3);

		// 最大升力面积因子载荷计算(气动升力系数x部件面积除以满油重量）
		// 最大攻角转弯时机身是失速的
		float fuseClHigh = Fuselage.ClCritHigh * Fuselage.lineClCoeff;
		if (Fuselage.AoACritHigh < NoFlapsWing.AoACritHigh)
			fuseClHigh = Fuselage.ClAfterCrit * Fuselage.lineClCoeff;

		AWing = AWingLeftIn + AWingRightIn + AWingLeftMid + AWingRightMid + AWingLeftOut + AWingLeftCut + AWingRightOut + AWingRightCut+ AAileron ;
		

		NoFlapsWing.Sq = AWing;
		FullFlapsWing.Sq = AWing;
		Fuselage.Sq = AFuselage;
		
		
		
		NoFlapWLL = AWing * NoFlapsWing.ClCritHigh + AFuselage * fuseClHigh;
		NoFlapWLL = NoFlapWLL / (halfweight / 1000.f);
		
		// System.out.println(AWing * NoFlapsWing.ClCritHigh/)
		fuseClHigh = Fuselage.ClCritHigh * Fuselage.lineClCoeff;
		if (Fuselage.AoACritHigh < FullFlapsWing.AoACritHigh)
			fuseClHigh = Fuselage.ClAfterCrit * Fuselage.lineClCoeff;

		FullFlapWLL = AWing * FullFlapsWing.ClCritHigh + AFuselage * fuseClHigh;
		FullFlapWLL = FullFlapWLL / (halfweight / 1000.f);
		// 阻力面积因子计算
		CdS = AWing * NoFlapsWing.CdMin + AFuselage * Fuselage.CdMin;
		// 计算阻力抵消的攻角

		// 翼展
		Wingspan = getfloat("Wingspan");
		if (Wingspan == 0) {
			Wingspan = getfloat("WingPlane.Span");
			if (Wingspan == 0) {
				Wingspan = getfloat("WingPlaneSweep0.Span");
			}
		}

		AspectRatio = Wingspan * Wingspan / AWing;

		// 诱导阻力还要
		indCdF = 1 / (Math.PI * AspectRatio * OswaldsEfficiencyNumber);

		// System.out.println(NoFlapWLL+","+FullFlapWLL + ","+CdS);

		// FmCdMin = 0;
		// FmCdMin += NoFlapsWing.CdMin;
		// FmCdMin += Fuselage.CdMin;
		// FmCdMin += Fin.CdMin;
		// FmCdMin += Stab.CdMin;

		maxAllowGload = new float[2];
		getfloats("WingCritOverload", maxAllowGload, 2);
		if (maxAllowGload[0] == 0) {
			getfloats("Strength.CritOverload", maxAllowGload, 2);
		}
		// 减去机身升力承载的重量
		// WingLoad =
		// halfWingLoad =
		// String s = "----------------FM信息------------------\n"
		String s = String.format(lang.bFmVersion, readFileName, version);
		s += (String.format(lang.bWeight, emptyweight, maxfuelweight));
		s += (String.format(lang.bCritSpeed, CriticalSpeed * 3.6, vne));
		s += (String.format(lang.bAllowLoadFactor,  2 * maxAllowGload[0] / (9.78f * grossweight) + 1, 
				2 * maxAllowGload[1] / (9.78f * grossweight) - 1, 
		2 * maxAllowGload[0] / (9.78f * halfweight) + 1, 
		2 * maxAllowGload[1] / (9.78f * halfweight) - 1));
		
		for (int i = 0; i < FlapsDestructionNum; i++) {
//			s += "襟翼限速" + i + ": [" + String.format("%.0f", FlapsDestructionIndSpeed[i][0] * 100) + "%, "
//					+ String.format("%.0f", FlapsDestructionIndSpeed[i][1]) + "]\n";
			s += String.format(lang.bFlapRestrict, i, FlapsDestructionIndSpeed[i][0] * 100, FlapsDestructionIndSpeed[i][1]);
		}
		s += String.format(lang.bEffSpeedAndPowerLoss, elavEff, aileronEff, rudderEff, elavPowerLoss, aileronPowerLoss, rudderPowerLoss);
		
		if (nitro != 0)
			s += String.format(lang.bNitro, nitro, nitro / (nitroDecr * 60));

		s += String.format(lang.bAverageHeatRecovery, avgEngRecoveryRate);
		
		s += String.format(lang.bMaxLiftLoad350, (NoFlapWLL + 1) / 2, (FullFlapWLL + 1) / 2);
		
		maxAllowGload[0] = (2 * maxAllowGload[0] / (9.78f * grossweight) + 1);
		maxAllowGload[1] = (2 * maxAllowGload[1] / (9.78f * grossweight) - 1);
		
		
		
		// 计算滚转率
		
		// 先计算Cla
		Cl_a = (NoFlapsWing.ClCritHigh - NoFlapsWing.Cl0)/(NoFlapsWing.AoACritHigh);
		
		
		// 获得襟翼偏转角度(上偏和下偏)
		AileronDefl = new float[2];
		
		if (null == getfloats("AileronAngles", AileronDefl, 2)){
			getfloats("Ailerons.AnglesRoll", AileronDefl, 2);
		}
//		AAileron
		// 
//		Wx_vcoff = ((Cl_a * (AileronDefl[0]+AileronDefl[1]) + NoFlapsWing.Cl0) * AAileron / (2 * Wingspan * 5f /*c*/)) * 57.3f;
//		// 得到滚转率函数(高速要考虑到线性减少)
//		Wx250= 69.444f * Wx_vcoff;
//		Wx300= 83.333f  * Wx_vcoff;
//		Wx350= 97.222f * Wx_vcoff;
//		// 最大滚转率
//		WxMax = aileronEff/3.6f * Wx_vcoff * 1.0f;
//		// 如果大于最大速度则需要乘以舵面值
////		Wx600 = 600/3.6f * Wx_vcoff * (1.0f - (((600 - aileronEff) * aileronPowerLoss)/100.0f));
//		
////		System.out.println(String.format("Wx 250, 300, 350: %.0f, %.0f, %.0f", Wx250, Wx300, Wx350));
//		
//		System.out.println("滚转效率 :" + 83.333f* Wx_vcoff);
//		System.out.println("Wx_max :" + WxMax);
//		System.out.println("Wx_600 :" + Wx600 +", aileronMax:" + ((600 - aileronEff) * aileronPowerLoss));
//		s += 		
		s += String.format(lang.bInertia, MomentOfInertia[0], MomentOfInertia[1], MomentOfInertia[2]);
		
		s += String.format(lang.bLift, AWing, AFuselage, NoFlapWLL, FullFlapWLL, OswaldsEfficiencyNumber, AspectRatio, SweptWingAngle);
		
		s += String.format(lang.bDrag, CdS, CdS/(halfweight / 1000.f), indCdF, halfweight * indCdF, RadiatorCd, OilRadiatorCd);
//		s += "------------------\n机翼与机身升力面积: [" + String.format("%.1f, %.1f", AWing, AFuselage) + "]\n翼展效率: " + OswaldsEfficiencyNumber + " 展弦比:"
//				+ String.format("%.2f", AspectRatio) + " 后掠角: " + SweptWingAngle + "\n诱导阻力因子: "
//				+ String.format("%.3g", indCdF) + "\n诱导阻力加速度系数: " + String.format("%.0f", halfweight * indCdF) + "(半油)"
//				+ "\n翼身临界升力面积因数载荷: [" + String.format("%.2f", NoFlapWLL) + ", " + String.format("%.2f", FullFlapWLL)
//				+ "]" + "\n翼身阻力面积因数及俯冲系数: " + String.format("%.3f, ", CdS) + String.format("%.3f", CdS/(halfweight / 1000.f))
//				+ "\n散热器/油冷器阻力系数: [" + RadiatorCd + ", " + OilRadiatorCd + "]\n";

		s = WritePartsFm(s, NoFlapsWing);
		s = WritePartsFm(s, FullFlapsWing);
		s = WritePartsFm(s, Fuselage);
		s = WritePartsFm(s, Fin);
		s = WritePartsFm(s, Stab);

		fmdata = s;
//		System.out.println(s);
//		System.out.println(GearDestructionIndSpeed);
		// System.out.println(wtload0+" "+oilload0);
		// System.out.println(wtload1+" "+oilload1);
		// System.out.println(wtload2+" "+oilload2);
		// System.out.println(wtload3+" "+oilload3);
		// System.out.println(wtload4+" "+oilload4);
		// System.out.println(wtload5+" "+oilload5);
		// System.out.println(subSt(getone("Load0")));
	}

	public void getperformancedata(String t) {

	}

	public String subSt(String t) {
		String a = t.substring(1, t.length() - 1);
		return a;
	}

	public void transUnit() {
		String unitSystem = "";
		unitSystem = getone("PASSPORT.UNITSYSTEM");
		unitSystem = subSt(unitSystem);
		if (unitSystem.indexOf("Imperial") != -1) {
			// System.out.println("英制");
			for (int i = 0; i < loc.cur; i++) {
				loc.y[i] = loc.y[i] * 0.3048f;
			}
			for (int i = 0; i < loc0.cur; i++) {
				loc0.y[i] = loc0.y[i] * 0.3048f;
			}
			for (int i = 0; i < loc1.cur; i++) {
				loc1.y[i] = loc1.y[i] * 0.3048f;
				loc1.x[i] = loc1.x[i] * 1.609344f;
			}
			for (int i = 0; i < loc2.cur; i++) {
				loc2.y[i] = loc2.y[i] * 0.3048f;
				loc2.x[i] = loc2.x[i] * 1.609344f;
			}
			for (int i = 0; i < loc3.cur; i++) {
				loc3.y[i] = loc3.y[i] * 1.609344f;
				// System.out.println(loc3.x[i]+" "+loc3.y[i]);
			}

		}
	}

	public void getAllplotdata() {
		loc = getplotdata("PASSPORT.ALT.minClimbTimeWep");
		loc0 = getplotdata("PASSPORT.ALT.minClimbTimeNom");
		loc1 = getplotdata("PASSPORT.ALT.maxSpeedWep");
		loc2 = getplotdata("PASSPORT.ALT.maxSpeedNom");
		loc3 = getplotdata("PASSPORT.IAS.maxRollRateLeft");
		transUnit();
	}

	public XY getplotdata(String t) {
		int line = 0;
		t = getArray(t);
		for (int i = 0; i < t.length(); i++) {
			if (t.charAt(i) == '\n')
				line++;
		}
		XY lo = new XY(line);
		int bix = 0;
		for (int i = 0; i < t.length(); i++) {
			if (t.charAt(i) == '\n') {
				String temp = t.substring(bix, i);
				String[] tmp = temp.split(", ");
				lo.y[lo.cur] = Double.parseDouble(tmp[0]);
				lo.x[lo.cur] = Double.parseDouble(tmp[1]);
				lo.cur++;
				bix = i + 1;
			}
		}
		return lo;
	}

	public void init(String t) {
		data = t;
		fmdata = lang.noblkx;
	}

	public blkx(String filepath, String name) {
		File file = new File(filepath);
		fmdata = lang.noblkx;
		if (file.exists()) {
			StringBuilder sb = new StringBuilder();
			String s = "";
			BufferedReader br;
			try {
				br = new BufferedReader(new FileReader(file));
				while ((s = br.readLine()) != null) {
					sb.append(s + "\n");
				}
				br.close();
			} catch (FileNotFoundException e1) {
				// TODO Auto-generated catch block
				e1.printStackTrace();
			} catch (IOException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			readFileName = name;
			data = sb.toString();
			valid = true;
		} else {
			valid = false;
		}
	}

	String cut(String t, String clslabel) {
		String tmp = t;
		int i = 0;
		int left = 0;
		int right = 0;
		int bix = tmp.toUpperCase().indexOf(clslabel.toUpperCase() + '{');
		if (bix == -1)
			return "null";
		int cutleft = bix;
		while (tmp.charAt(cutleft) != '{')
			cutleft++;
		cutleft++;
		for (i = bix; i < tmp.length(); i++) {
			if (tmp.charAt(i) == '{')
				left++;
			if (tmp.charAt(i) == '}')
				right++;
			if (left != 0 && right != 0 && left == right)
				break;
		}
		int cutright = i;
		return tmp.substring(cutleft, cutright);
	}

	public String getArray(String label) {
		String value = "";
		String text = data;
		// 第一步处理
		int clsbix = 0;
		for (int i = 0; i < label.length(); i++) {
			if (label.charAt(i) == '.') {
				String cls = label.substring(clsbix, i);
				text = cut(text, cls);
				clsbix = i + 1;
			}
		}
		label = label.substring(clsbix);
		// System.out.println(text);
		// 第二步获得值
		int bix = 0;
		int eix = 0;
		bix = text.toUpperCase().indexOf(label.toUpperCase());
		while (bix != -1) {
			while (text.charAt(bix) != '=')
				bix++;
			bix++;
			eix = bix;
			while (text.charAt(eix) != '\n')
				eix++;
			value = value + text.substring(bix, eix + 1);
			text = text.substring(eix + 1);
			bix = text.toUpperCase().indexOf(label.toUpperCase());
		}
		return value;
	}

	public String getlastone(String label) {
		String value = "";
		String text = data;
		// 第一步处理
		int clsbix = 0;
		for (int i = 0; i < label.length(); i++) {
			if (label.charAt(i) == '.') {
				String cls = label.substring(clsbix, i);
				text = cut(text, cls);
				clsbix = i + 1;
			}
		}
		label = label.substring(clsbix);
		// 第二步获得值
		int bix = 0;
		int eix = 0;
		bix = text.toUpperCase().lastIndexOf(label.toUpperCase());
		if (bix == -1)
			return "null";
		while (text.charAt(bix) != '=')
			bix++;
		bix++;
		eix = bix;
		while (text.charAt(eix) != '\n')
			eix++;
		value = text.substring(bix, eix);
		return value;
	}

	public String getoneinData(String D, String label) {
		String value = "";
		String text = D;
		// 第一步处理
		int clsbix = 0;
		for (int i = 0; i < label.length(); i++) {
			if (label.charAt(i) == '.') {
				String cls = label.substring(clsbix, i);
				text = cut(text, cls);
				clsbix = i + 1;
			}
		}
		label = label.substring(clsbix);
//		System.out.println(label);
		// 第二步获得值
		int bix = 0;
		int eix = 0;
		bix = text.toUpperCase().indexOf(label.toUpperCase());
		if (bix == -1)
			return "null";
		while (text.charAt(bix) != '=')
			bix++;
		bix++;
		eix = bix;
		while (text.charAt(eix) != '\n')
			eix++;
		value = text.substring(bix, eix);
		return value;
	}

	public String getone(String label) {
		String value = "";
		String text = data;
		// 第一步处理
		int clsbix = 0;
		for (int i = 0; i < label.length(); i++) {
			if (label.charAt(i) == '.') {
				String cls = label.substring(clsbix, i);
				text = cut(text, cls);
				clsbix = i + 1;
			}
		}
		label = label.substring(clsbix);
		// System.out.println(text);
		// 第二步获得值
		int bix = 0;
		int eix = 0;
//		bix = text.toUpperCase().indexOf(label.toUpperCase());
		bix = text.indexOf(label);
		if (bix == -1)
			return "null";
		while (text.charAt(bix) != '=')
			bix++;
		bix++;
		eix = bix;
		while (text.charAt(eix) != '\n')
			eix++;
		value = text.substring(bix, eix);
		return value;
	}

}