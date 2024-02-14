package com.voidmei.parser;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.IOException;

import com.voidmei.App;
import com.voidmei.lang;

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
		public double WaterLimit;
		public double OilLimit;
		public double WorkTime;
		public double RecoverTime;
		public double curWaterWorkTimeMili;
		public double curOilWorkTimeMili;
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

	public double vne;
	public double vne_V50;
	public double vne_V100;
	
	public double vneMach;
	public double vneMach_V50;
	public double vneMach_V100;
	
	public double clmax;
	public double aoaHigh;
	public double aoaLow;
	public double flapAoaHigh;
	public double flapAoaLow;

	public double aoaFuselageHigh;
	public double aoaFuselageLow;

	public double flapClmax;
	public double emptyweight;

	public double[] maxAllowGload;

	public int emptyweightToLoad;
	public double aileronEff;
	public double aileronPowerLoss;
	public double rudderEff;
	public double rudderPowerLoss;
	public double elavEff;
	public double elavPowerLoss;
	public double nitro;

	public double grossweight;

	public double oil;

	public double nitroDecr;
	public double maxfuelweight;

	public double FmCdMin;

	public double WingAngle;
	public double StabAngle;
	public double KeelAngle;

	public double RadiatorCd;
	public double OilRadiatorCd;
	public double AirbrakeCd;
	public double OswaldsEfficiencyNumber;

	public class fm_parts {
		public String name;
		
		public double Sq;
		public double CdMin;

		public double Cl0;

		public double ClCritHigh;
		public double ClCritLow;

		public double ClAfterCrit;

		public double AoACritHigh;
		public double AoACritLow;

		public double lineClCoeff;

		// 翼展效率因数，影响诱导阻力，因数越大阻力越小
		// public double oswaldEff;

	}

	public fm_parts NoFlapsWing;
	public fm_parts NoFlapsWing_V50;
	public fm_parts NoFlapsWing_V100;
	
	public fm_parts FullFlapsWing;
	public fm_parts FullFlapsWing_V50;
	public fm_parts FullFlapsWing_V100;
	
	public Boolean isVWing;
	
	public fm_parts Fuselage;
	public fm_parts Fin;
	public fm_parts Stab;
	public double SweptWingAngle;
	public double WingTaperRatio;
	public double CriticalSpeed;
	public double AWingLeftIn;
	public double AWingLeftMid;
	public double AWingLeftOut;
	public double AWingRightIn;
	public double AWingRightMid;
	public double AWingRightOut;
	public double AFuselage;
	public double AWing;
	public double NoFlapWLL;
	public double FullFlapWLL;
	public double CdS;

	public double[] MomentOfInertia;
	public double AAileron;
	public double Wingspan;
	public double AspectRatio;
	public double indCdF;
	public String version;
	public double avgEngRecoveryRate;
	public int FlapsDestructionNum;
	public double[][] FlapsDestructionIndSpeed;
	private double halfweight;

	// public

	public void getPartsFm(String c, fm_parts p) {
		p.name = c;
		p.CdMin = getdouble(c + ".CdMin");
		p.Cl0 = getdouble(c + ".Cl0");
		p.ClCritHigh = getdouble(c + ".ClCritHigh");
		p.ClCritLow = getdouble(c + ".ClCritLow");

		p.ClAfterCrit = getdouble(c + ".ClAfterCrit");
		p.lineClCoeff = getdouble(c + ".lineClCoeff");

		p.AoACritHigh = getdouble(c + ".alphaCritHigh");
		p.AoACritLow = getdouble(c + ".alphaCritLow");

	}

	public boolean getEngineLoad(engineLoad[] eL, int loadIndex) {
		String c = "Load" + loadIndex;
		// App.debugPrint(c);
		eL[loadIndex].WaterLimit = getdouble(c + ".WaterTemperature");
		// App.debugPrint(eL[loadIndex].WaterLimit);
		if (eL[loadIndex].WaterLimit == 0)
			return Boolean.FALSE;
		eL[loadIndex].OilLimit = getdouble(c + ".OilTemperature");
		if (eL[loadIndex].OilLimit == 0)
			return Boolean.FALSE;
		eL[loadIndex].WorkTime = getdouble(c + ".WorkTime");
		// if(eL[loadIndex].WorkTime == 0)
		eL[loadIndex].RecoverTime = getdouble(c + ".RecoverTime");
		// eL[loadIndex].curWorkTimeMili = eL[loadIndex].WorkTime * 1000;
		eL[loadIndex].curWaterWorkTimeMili = eL[loadIndex].WorkTime * 1000;
		eL[loadIndex].curOilWorkTimeMili = eL[loadIndex].WorkTime * 1000;
		return Boolean.TRUE;
	}

	public void showEngineLoad(engineLoad[] eL, int loadIndex) {
		String c = "Load" + loadIndex;
		c = c.concat("水温/油温限制: [" + eL[loadIndex].WaterLimit + "," + eL[loadIndex].OilLimit + "]\n");
		c = c.concat("加力/恢复时间: [" + eL[loadIndex].WorkTime + "," + eL[loadIndex].RecoverTime + "]\n");
		App.debugPrint(c);
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
		// App.debugPrint("------fm器件 "+p.name+"------");
		// App.debugPrint("零升阻力系数:"+p.CdMin);
		// App.debugPrint("零攻角升力:"+p.Cl0);
		// App.debugPrint("临界攻角:["+p.AoACritLow+","+p.AoACritHigh+"]");
		// App.debugPrint("临界攻角升力系数:["+p.ClCritLow+","+p.ClCritHigh+"]");
		return s;
	}

	public double[] getdoubles(String c, double[] ret, int num) {
		if (num <= 0)
			return null;

		if (!getone(c).equals("null")) {
			String[] tmp = getone(c).split(",");
			for (int i = 0; i < num; i++) {
				try {

					ret[i] = Float.parseFloat(tmp[i]);
				} catch (Exception e) {
					App.debugPrint("getdouble error" + c);
					return null;
				}
			}
		}
		return ret;

	}

	public double getdouble(String c) {
		double ret = 0;
		if (!getone(c).equals("null")) {
			String[] tmp = getone(c).split(",");
			try {
				ret = Float.parseFloat(tmp[0]);
			} catch (Exception e) {
				App.debugPrint("getdouble error" + c);
				return 0;
			}
		}
		return ret;
	}

	public double getdouble_exc(String c) {
		double ret = Float.MAX_VALUE;
		if (!getone(c).equals("null")) {
			String[] tmp = getone(c).split(",");
			try {
				ret = Float.parseFloat(tmp[0]);
			} catch (Exception e) {
				App.debugPrint("getdouble error" + c);
				return 0;
			}
		}
		return ret;
	}

//	public int findmaxLoad(engineLoad[] eL, double water, double oil) {
//		for (int i = 0; i < maxEngLoad; i++) {
//			// 大于还是小于等于呢？
//			if (water < eL[i].WaterLimit && oil < eL[i].OilLimit)
//				return i;
//		}
//
//		return maxEngLoad;
//	}

	public int findmaxWaterLoad(engineLoad[] eL, double water) {
		for (int i = 0; i < maxEngLoad; i++) {
			// 大于还是小于等于呢？
			if (water < eL[i].WaterLimit)
				return i;
//			if (Math.round(water) < eL[i].WaterLimit)
//				return i;
		}

		return maxEngLoad;
	}

	public int findmaxOilLoad(engineLoad[] eL, double oil) {
		for (int i = 0; i < maxEngLoad; i++) {
			// 大于还是小于等于呢？
			if (oil < eL[i].OilLimit)
				return i;
//			if (Math.round(oil) < eL[i].OilLimit)
//				return i;
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

	
	public double altitudeThr[];
	public double velocityThr[];
	public double maxThrCoff[][];
	public double maxThrAftCoff[][];
	public double maxThr[][];
	public double maxThrAft[][];
	public double thrMax0; // 静推力
	public double aftbCoff;
	public int altThrNum;
	public int velThrNum;
	public boolean isJet;
	public int engineNum;

	public int compNumSteps;
	public double compAlt[];
	public double compPower[];

	public double compCeil[];
	public double compCeilPwr[];
	
	public double compBoost[];
	public double compRpmRatio[];
	
	public double modeEngineMult[];
	

	// 冲压系数
	public double speedToManifoldMultiplier;
	private int modeEngineNum;
	private double AWingRightCut;
	private double AWingLeftCut;

	
	public double GearDestructionIndSpeed;
	public double maxRPM;
	public double maxAllowedRPM;
	private double Cl_a;
	private double[] AileronDefl;
	private double Wx100;
	private double Wx_vcoff;
	private double Wx250;
	private double Wx300;
	private double Wx350;
	private double WxMax;
	private double Wx600;
	private double[] modeEngineRPMMult;
	private double engineRPMMultWEP;
	private fm_parts FullFlapsWingS;
	private fm_parts NoFlapsWingS;
	
	/* 计算可变翼 */
	public double getAoAHighVWing(double vwing, int flaps_percent) {
		if (vwing == 0) {
			/* 计算flaps */
			return NoFlapsWing.AoACritHigh
					+ (FullFlapsWing.AoACritHigh - NoFlapsWing.AoACritHigh) * flaps_percent / 100.0f;
			
		}
		/* 针对只有2级的 */
		if(NoFlapsWing_V100.AoACritHigh == 0) {
			return NoFlapsWing.AoACritHigh + (NoFlapsWing_V50.AoACritHigh - NoFlapsWing.AoACritHigh) * (vwing);
		} else {
			if (vwing < 0.5)
				return NoFlapsWing.AoACritHigh + (NoFlapsWing_V50.AoACritHigh - NoFlapsWing.AoACritHigh) * (vwing / 0.5);
			else 
				return NoFlapsWing_V50.AoACritHigh + (NoFlapsWing_V100.AoACritHigh - NoFlapsWing_V50.AoACritHigh) * ((vwing - 0.5) / 0.5);
		}
	}
	public double getAoALowVWing(double vwing, int flaps_percent) {
		if(NoFlapsWing_V100.AoACritLow == 0) {
			return NoFlapsWing.AoACritLow + (NoFlapsWing_V50.AoACritLow - NoFlapsWing.AoACritLow) * (vwing);
		} else {
			if (vwing < 0.5)
				return NoFlapsWing.AoACritLow + (NoFlapsWing_V50.AoACritLow - NoFlapsWing.AoACritLow) * (vwing / 0.5);
			else 
				return NoFlapsWing_V50.AoACritLow + (NoFlapsWing_V100.AoACritLow - NoFlapsWing_V50.AoACritLow) * ((vwing - 0.5) / 0.5);
		}
	}
	
	public double getVNEVWing(double vwing) {
		if (vne_V100 == 0) {
			return (vne + (vne_V50 - vne) * vwing);
		}else {
			if (vwing < 0.5)
				return (vne + (vne_V50 - vne) * (vwing/0.5));
			else 
				return (vne_V50 + (vne_V100 - vne_V50)  * ((vwing - 0.5)/0.5));
		}
	}
	
	public double getMNEVWing(double vwing) {
		if (vneMach_V100 == 0) {
			return (vneMach + (vneMach_V50 - vneMach) * vwing);
		}else {
			if (vwing < 0.5)
				return (vneMach + (vneMach_V50 - vneMach) * (vwing/0.5));
			else 
				return (vneMach_V50 + (vneMach_V100 - vneMach_V50)  * ((vwing - 0.5)/0.5));
		}
	}
	
	
	public void initEngineLoad(){
		avgEngRecoveryRate = 0.0f;
		engLoad = new engineLoad[App.maxEngLoad];
		for (int i = 0; i < App.maxEngLoad; i++) {
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
		avgEngRecoveryRate = avgEngRecoveryRate / (maxEngLoad - 1);
	}
	public void getload() {
		// String Load0 = cut(data, "Load0");
		// App.debugPrint(getone("Load0.WaterTemperature"));
		isJet = false;
		
		// 读取推力高度
		engineNum = 1;
//		App.debugPrint(getone("EngineType0.Main.Type"));
		String hdrString = "EngineType0.";
		String res = getone("EngineType0.Main.Type");
//		App.debugPrint(res);
		if (res.equals("\"Jet\"")) {
			// 判断喷气
			isJet = true;
//			App.debugPrint(getone("Engine"+engineNum));
			while (!getone("Engine"+engineNum).equals("null")){
				engineNum++;
			}
		} else {
			if (res.equals("null")) {
				hdrString = "Engine0.";
				if (getone("Engine0.Main.Type").equals("\"Jet\"")) {
					isJet = true;
				}
				while (!getone("Engine"+engineNum).equals("null")){
					engineNum++;
				}
			}
		}
		App.debugPrint("engineNum" + engineNum);
		engineRPMMultWEP = 1.0f;
		if (isJet) {
			aftbCoff = getdouble(hdrString + "Main.AfterburnerBoost");
			thrMax0 = getdouble("ThrustMax.ThrustMax0");
//			thrMax0 = getdouble(hdrString + "Main.Thrust");
			
			App.debugPrint("engineType: jet, afterburner coeff" + aftbCoff);
			altThrNum = 0;
			altitudeThr = new double[30];
			for (int i = 0; i < 30; i++, altThrNum++) {
				altitudeThr[i] = getdouble_exc("ThrustMax.Altitude_" + i);
				if (altitudeThr[i] == Float.MAX_VALUE){
					altitudeThr[i] = 0;
					break;
				}
				// App.debugPrint(altitudeThr[i]);
			}
			// 读取推力速度
			velThrNum = 0;
			velocityThr = new double[30];

			for (int i = 0; i < 30; i++, velThrNum++) {
				velocityThr[i] = getdouble_exc("ThrustMax.Velocity_" + i);
				if (velocityThr[i] == Float.MAX_VALUE){
					velocityThr[i] = 0;
					break;
				}
				// App.debugPrint(altitudeThr[i]);
			}
			
			// 读取发动机工作模式

			modeEngineNum = 0;
			modeEngineMult = new double[10];
			modeEngineRPMMult = new double[10];
			for (int i = 0; i < 10; i++, modeEngineNum++){
				modeEngineMult[i] = getdouble_exc("Main.Mode"+i+".ThrustMult");
				modeEngineRPMMult[i] =  getdouble_exc("Main.Mode"+i+".RPM");
				if (modeEngineMult[i] == Float.MAX_VALUE){
					modeEngineMult[i] = 0;
					modeEngineRPMMult[i] = 1;
					break;
				}	
			}
			
			
			
			double engineMultWEP = 1.0f;
			
			if (modeEngineNum != 0){
				engineMultWEP = modeEngineMult[modeEngineNum - 1];
				engineRPMMultWEP = modeEngineRPMMult[modeEngineNum - 1];
			}

			
			// 读取推力系数包线
			maxThrCoff = new double[altThrNum][];
			maxThr = new double[altThrNum][];
			maxThrAft = new double[altThrNum][];
			maxThrAftCoff = new double[altThrNum][];
			for (int i = 0; i < altThrNum; i++) {
				maxThrCoff[i] = new double[velThrNum];
				maxThr[i] = new double[velThrNum];
				maxThrAft[i] = new double[velThrNum];
				maxThrAftCoff[i] = new double[velThrNum];
				
				for (int j = 0; j < velThrNum; j++) {
					maxThrCoff[i][j] = getdouble("ThrustMax.ThrustMaxCoeff_" + i + "_" + j);
					maxThrAftCoff[i][j] = getdouble("ThrustMax.ThrAftMaxCoeff_"+ i +"_" + j);
					if(maxThrAftCoff[i][j] == 0){
						maxThrAftCoff[i][j] = 1.0f;
					}
					maxThr[i][j] = thrMax0 * maxThrCoff[i][j] * engineNum;
					maxThrAft[i][j] = thrMax0 * maxThrCoff[i][j] * aftbCoff * maxThrAftCoff[i][j] * engineMultWEP * engineNum;
					App.debugPrint(String.format("[%.0f]%.0f:%.0f kgf", altitudeThr[i], velocityThr[j], maxThrAft[i][j]));
				}
			}
		} else {
			// radial inline
			// 获得增压器工作高度
//			App.debugPrint("not a jet");
			aftbCoff = getdouble(hdrString + "Main.AfterburnerBoost");
//			App.debugPrint(hdrString);
			compNumSteps = (int) getdouble("Compressor.NumSteps");
			speedToManifoldMultiplier = getdouble("Compressor.SpeedManifoldMultiplier");

			compAlt = new double[compNumSteps];
			compBoost = new double[compNumSteps];
			compPower = new double[compNumSteps];
			compRpmRatio = new double[compNumSteps];
			compCeil = new double[compNumSteps];
			compCeilPwr = new double[compNumSteps];;
			
			
			for (int i = 0; i < compNumSteps; i++) {
				compAlt[i] = getdouble("Compressor.Altitude" + i);
				compPower[i] = getdouble("Compressor.Power" + i);
				compBoost[i] = getdouble("Compressor.AfterburnerBoostMul" + i);
				compRpmRatio[i] = getdouble("Compressor.PowerConstRPMCurvature" + i);
				compCeil[i] = getdouble("Compressor.Ceiling" + i);
				compCeilPwr[i] = getdouble("Compressor.PowerAtCeiling"+i);
//				App.debugPrint(String.format("*s%d*:[%.0f]%.0fhp - [%.0f]%.0fhp", i, compAlt[i], compPower[i] * compRpmRatio[i] * aftbCoff, compCeil[i], compCeilPwr[i] * compRpmRatio[i] * aftbCoff));
			}

			//
		}

		// 读取最大转速和最大允许转速
		maxRPM = getdouble("RPMAfterburner");
		double maxRPMNormal = getdouble(" RPMMax");
		if (maxRPM < maxRPMNormal) maxRPM = maxRPMNormal;

		App.debugPrint("RPMMult"+engineRPMMultWEP);
		// 针对幻影2000C mode6 rpm乘数1.01的修复
		maxRPM = maxRPM * engineRPMMultWEP;
		App.debugPrint("RPM"+maxRPM);
		maxAllowedRPM = getdouble("RPMMaxAllowed");
//		App.debugPrint("RPM"+maxAllowedRPM);
		
		// avgEngRecoveryRate = 0.0f;
		version = getVersion();
		initEngineLoad();
		// engLoad = new engineLoad[10];
		// for (int i = 0; i < 10; i++) {
		// 	engLoad[i] = new engineLoad();
		// }

		// maxEngLoad -= 1;
		// engLoad[maxEngLoad].WaterLimit = 999;
		// engLoad[maxEngLoad].OilLimit = 999;

		// for (int i = 0; i < maxEngLoad; i++) {
		// 	if (engLoad[i].RecoverTime != 0)
		// 		avgEngRecoveryRate = avgEngRecoveryRate + engLoad[i].WorkTime / engLoad[i].RecoverTime;
		// 	showEngineLoad(engLoad, i);
		// }
		// App.debugPrint(engLoad[0].WorkTime/engLoad[i].RecoverTime);
		// avgEngRecoveryRate = avgEngRecoveryRate / (maxEngLoad - 1);
		emptyweight = getdouble("EmptyMass");
		vne = getdouble("Vne:");
		if(vne == 0){
			vne = getdouble("WingPlane.Strength.VNE");
			if(vne == 0){
				vne = getdouble("WingPlaneSweep0.Strength.VNE");
			}
		}
		
		vne_V50 = getdouble("WingPlaneSweep1.Strength.VNE");
		vne_V100 = getdouble("WingPlaneSweep2.Strength.VNE");

		vneMach = getdouble("VneMach");
		if(vneMach == 0){
			vneMach = getdouble("WingPlane.Strength.MNE");
			if(vneMach == 0){
				vneMach = getdouble("WingPlaneSweep0.Strength.MNE");
			}
		}
		
		vneMach_V50 = getdouble("WingPlaneSweep1.Strength.MNE");
		vneMach_V100 = getdouble("WingPlaneSweep2.Strength.MNE");
		
		aileronEff = getdouble("AileronEffectiveSpeed");
		aileronPowerLoss = getdouble("AileronPowerLoss");
		rudderEff = getdouble("RudderEffectiveSpeed");
		rudderPowerLoss = getdouble("RudderPowerLoss");
		elavEff = getdouble("ElevatorsEffectiveSpeed");
		elavPowerLoss = getdouble("ElevatorPowerLoss");
		maxfuelweight = getdouble("MaxFuelMass0");

		clmax = getdouble("NoFlaps.ClCritHigh");
		flapClmax = getdouble("FullFlaps.ClCritHigh");

		aoaHigh = getdouble("NoFlaps.alphaCritHigh");
		aoaLow = getdouble("NoFlaps.alphaCritLow");

		flapAoaHigh = getdouble("FullFlaps.alphaCritHigh");
		flapAoaLow = getdouble("FullFlaps.alphaCritLow");

		nitroDecr = getdouble("NitroConsumption");
		nitro = getdouble("MaxNitro");
		oil = getdouble("OilMass");

		App.debugPrint("作战空重" + (emptyweight + oil + nitro));
		grossweight = emptyweight + maxfuelweight + nitro + oil;
		halfweight = emptyweight + maxfuelweight / 2 + nitro + oil;

		RadiatorCd = getdouble("RadiatorCd");
		OilRadiatorCd = getdouble("OilRadiatorCd");
		OswaldsEfficiencyNumber = getdouble("OswaldsEfficiencyNumber");

		SweptWingAngle = getdouble("SweptWingAngle");
		if (SweptWingAngle == 0) {
			SweptWingAngle = getdouble("WingPlane.SweptAngle");
			if (SweptWingAngle == 0) {
				SweptWingAngle = getdouble("WingPlaneSweep0.SweptAngle");
			}
		}

		WingTaperRatio = getdouble("WingTaperRatio");
		if (WingTaperRatio == 0) {
			WingTaperRatio = getdouble("WingPlane.TaperRatio");
			if (WingTaperRatio == 0) {
				WingTaperRatio = getdouble("WingPlaneSweep0.TaperRatio");
			}
		}

		CriticalSpeed = getdouble("CriticalSpeed");

		FlapsDestructionIndSpeed = new double[5][2];
		FlapsDestructionNum = 0;
		{
			int p = 0;
			while (true) {
				getdoubles("FlapsDestructionIndSpeedP" + (p++), FlapsDestructionIndSpeed[FlapsDestructionNum], 2);
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
			FlapsDestructionIndSpeed[0][1] = getdouble("FlapsDestructionIndSpeed");
		}
//		if (FlapsDestructionIndSpeed[0][1] != 0){
//			FlapsDestructionNum++;
//		}
		GearDestructionIndSpeed = getdouble("GearDestructionIndSpeed");
		
		// 面积
		AWingLeftIn = getdouble("Areas.WingLeftIn");
		if (AWingLeftIn == 0)
			AWingLeftIn = getdouble("WingPlane.Areas.LeftIn");
		if (AWingLeftIn == 0)
			AWingLeftIn = getdouble("WingPlaneSweep0.Areas.LeftIn");

		AWingLeftMid = getdouble("Areas.WingLeftMid");
		if (AWingLeftMid == 0)
			AWingLeftMid = getdouble("WingPlane.Areas.LeftMid");
		if (AWingLeftMid == 0)
			AWingLeftMid = getdouble("WingPlaneSweep0.Areas.LeftMid");

		AWingLeftOut = getdouble("Areas.WingLeftOut");
		if (AWingLeftOut == 0)
			AWingLeftOut = getdouble("WingPlane.Areas.LeftOut");
		if (AWingLeftOut == 0)
			AWingLeftOut = getdouble("WingPlaneSweep0.Areas.LeftOut");

		AWingLeftCut = getdouble("Areas.WingLeftCut");
		if (AWingLeftCut == 0)
			AWingLeftCut = getdouble("WingPlane.Areas.LeftCut");
		if (AWingLeftCut == 0)
			AWingLeftCut = getdouble("WingPlaneSweep0.Areas.LeftCut");

		
		AWingRightIn = getdouble("Areas.WingRightIn");
		if (AWingRightIn == 0)
			AWingRightIn = getdouble("WingPlane.Areas.RightIn");
		if (AWingRightIn == 0)
			AWingRightIn = getdouble("WingPlaneSweep0.Areas.RightIn");

		AWingRightMid = getdouble("Areas.WingRightMid");
		if (AWingRightMid == 0)
			AWingRightMid = getdouble("WingPlane.Areas.RightMid");
		if (AWingRightMid == 0)
			AWingRightMid = getdouble("WingPlaneSweep0.Areas.RightMid");

		AWingRightOut = getdouble("Areas.WingRightOut");
		if (AWingRightOut == 0)
			AWingRightOut = getdouble("WingPlane.Areas.RightOut");
		if (AWingRightOut == 0)
			AWingRightOut = getdouble("WingPlaneSweep0.Areas.RightOut");

		AWingRightCut = getdouble("Areas.WingRightCut");
		if (AWingRightCut == 0)
			AWingRightCut = getdouble("WingPlane.Areas.RightCut");
		if (AWingRightCut == 0)
			AWingRightCut = getdouble("WingPlaneSweep0.Areas.RightCut");

		
		AAileron = getdouble("Areas.Aileron");
		if (AAileron == 0)
			AAileron = getdouble("WingPlane.Areas.Aileron");
		if (AAileron == 0)
			AAileron = getdouble("WingPlaneSweep0.Areas.Aileron");

		AFuselage = getdouble("Areas.Fuselage");
		if (AFuselage == 0)
			AFuselage = getdouble("FuselagePlane.Areas.Main");
		if (AFuselage == 0)
			AFuselage = getdouble("WingPlaneSweep0.Areas.Main");
		
		AFuselage = getdouble("Areas.Fuselage");
		if (AFuselage == 0)
			AFuselage = getdouble("FuselagePlane.Areas.Main");
		if (AFuselage == 0)
			AFuselage = getdouble("WingPlaneSweep0.Areas.Main");

		
		
		NoFlapsWing = new fm_parts();
		getPartsFm("NoFlaps", NoFlapsWing);
		if (NoFlapsWing.AoACritHigh == 0) {
			getPartsFm("FlapsPolar0", NoFlapsWing);
		}
		
		/* 可变翼 */
		NoFlapsWing_V50 = new fm_parts();
		getPartsFm("WingPlaneSweep1.NoFlaps", NoFlapsWing_V50);
		if (NoFlapsWing_V50.AoACritHigh == 0) {
			getPartsFm("WingPlaneSweep1.FlapsPolar0", NoFlapsWing_V50);
		}
		
		NoFlapsWing_V100 = new fm_parts();
		getPartsFm("WingPlaneSweep2.NoFlaps", NoFlapsWing_V100);
		if (NoFlapsWing_V100.AoACritHigh == 0) {
			getPartsFm("WingPlaneSweep2.FlapsPolar0", NoFlapsWing_V100);
		}
		
		FullFlapsWing = new fm_parts();
		getPartsFm("FullFlaps", FullFlapsWing);
		if (FullFlapsWing.AoACritHigh == 0) {
			getPartsFm("FlapsPolar1", FullFlapsWing);
		}
		
		FullFlapsWing_V50 = new fm_parts();
		getPartsFm("WingPlaneSweep1.FullFlaps", FullFlapsWing_V50);
		if (FullFlapsWing_V50.AoACritHigh == 0) {
			getPartsFm("WingPlaneSweep1.FlapsPolar1", FullFlapsWing_V50);
		}
		
		/* 可变翼 */
		FullFlapsWing_V100 = new fm_parts();
		getPartsFm("WingPlaneSweep2.FullFlaps", FullFlapsWing_V100);
		if (FullFlapsWing_V100.AoACritHigh == 0) {
			getPartsFm("WingPlaneSweep2.FlapsPolar1", FullFlapsWing_V100);
		}
		
		/* 可变翼判断 */
		isVWing = false;
		if ((NoFlapsWing_V100.AoACritHigh != 0) || (NoFlapsWing_V50.AoACritHigh != 0)) {
			isVWing = true;
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
		WingAngle = getdouble("\nWingAngle");
		if (WingAngle == 0) {
			WingAngle = getdouble("WingPlane. Angle");
			if (WingAngle == 0) {
				WingAngle = getdouble("WingPlaneSweep0. Angle");
			}
		}
//		App.debugPrint("机翼安装角" + WingAngle);

		StabAngle = getdouble("StabAngle");
		if (WingAngle == 0) {
			WingAngle = getdouble("VerStabPlane.Angle");
		}

		KeelAngle = getdouble("KeelAngle");
		if (WingAngle == 0) {
			WingAngle = getdouble("FuselagePlane.Angle");
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

		MomentOfInertia = new double[3];
		getdoubles("MomentOfInertia", MomentOfInertia, 3);

		// 最大升力面积因子载荷计算(气动升力系数x部件面积除以满油重量）
		// 最大攻角转弯时机身是失速的
		double fuseClHigh = Fuselage.ClCritHigh * Fuselage.lineClCoeff;
		if (Fuselage.AoACritHigh < NoFlapsWing.AoACritHigh)
			fuseClHigh = Fuselage.ClAfterCrit * Fuselage.lineClCoeff;

		AWing = AWingLeftIn + AWingRightIn + AWingLeftMid + AWingRightMid + AWingLeftOut + AWingLeftCut + AWingRightOut + AWingRightCut+ AAileron ;
		

		NoFlapsWing.Sq = AWing;
		FullFlapsWing.Sq = AWing;
		Fuselage.Sq = AFuselage;
		
		
		
		NoFlapWLL = AWing * NoFlapsWing.ClCritHigh + AFuselage * fuseClHigh;
		NoFlapWLL = NoFlapWLL / (halfweight / 1000.f);
		
		// App.debugPrint(AWing * NoFlapsWing.ClCritHigh/)
		fuseClHigh = Fuselage.ClCritHigh * Fuselage.lineClCoeff;
		if (Fuselage.AoACritHigh < FullFlapsWing.AoACritHigh)
			fuseClHigh = Fuselage.ClAfterCrit * Fuselage.lineClCoeff;

		FullFlapWLL = AWing * FullFlapsWing.ClCritHigh + AFuselage * fuseClHigh;
		FullFlapWLL = FullFlapWLL / (halfweight / 1000.f);
		// 阻力面积因子计算
		CdS = AWing * NoFlapsWing.CdMin + AFuselage * Fuselage.CdMin;
		// 计算阻力抵消的攻角

		// 翼展
		Wingspan = getdouble("Wingspan");
		if (Wingspan == 0) {
			Wingspan = getdouble("WingPlane.Span");
			if (Wingspan == 0) {
				Wingspan = getdouble("WingPlaneSweep0.Span");
			}
		}

		AspectRatio = Wingspan * Wingspan / AWing;

		// 诱导阻力还要
		indCdF = 1 / (Math.PI * AspectRatio * OswaldsEfficiencyNumber);

		// App.debugPrint(NoFlapWLL+","+FullFlapWLL + ","+CdS);

		// FmCdMin = 0;
		// FmCdMin += NoFlapsWing.CdMin;
		// FmCdMin += Fuselage.CdMin;
		// FmCdMin += Fin.CdMin;
		// FmCdMin += Stab.CdMin;

		maxAllowGload = new double[2];
		getdoubles("WingCritOverload", maxAllowGload, 2);
		if (maxAllowGload[0] == 0) {
			getdoubles("Strength.CritOverload", maxAllowGload, 2);
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
		AileronDefl = new double[2];
		
		if (null == getdoubles("AileronAngles", AileronDefl, 2)){
			getdoubles("Ailerons.AnglesRoll", AileronDefl, 2);
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
////		App.debugPrint(String.format("Wx 250, 300, 350: %.0f, %.0f, %.0f", Wx250, Wx300, Wx350));
//		
//		App.debugPrint("滚转效率 :" + 83.333f* Wx_vcoff);
//		App.debugPrint("Wx_max :" + WxMax);
//		App.debugPrint("Wx_600 :" + Wx600 +", aileronMax:" + ((600 - aileronEff) * aileronPowerLoss));
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
		if (NoFlapsWing_V50.ClCritHigh != 0) s = WritePartsFm(s, NoFlapsWing_V50);
		if (NoFlapsWing_V100.ClCritHigh != 0) s = WritePartsFm(s, NoFlapsWing_V100);
		s = WritePartsFm(s, FullFlapsWing);
		s = WritePartsFm(s, Fuselage);
		s = WritePartsFm(s, Fin);
		s = WritePartsFm(s, Stab);

		fmdata = s;
//		App.debugPrint(s);
//		App.debugPrint(GearDestructionIndSpeed);
		// App.debugPrint(wtload0+" "+oilload0);
		// App.debugPrint(wtload1+" "+oilload1);
		// App.debugPrint(wtload2+" "+oilload2);
		// App.debugPrint(wtload3+" "+oilload3);
		// App.debugPrint(wtload4+" "+oilload4);
		// App.debugPrint(wtload5+" "+oilload5);
		// App.debugPrint(subSt(getone("Load0")));
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
			// App.debugPrint("英制");
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
				// App.debugPrint(loc3.x[i]+" "+loc3.y[i]);
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
			this.getload();

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
		// App.debugPrint(text);
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
			return null;
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
//		App.debugPrint(label);
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
		// App.debugPrint(text);
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