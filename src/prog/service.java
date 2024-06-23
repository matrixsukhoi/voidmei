package prog;


import parser.blkx.engineLoad;
import parser.flightLog;
import parser.indicators;
import parser.mapObj;
import parser.state;
import parser.stringHelper;

public class service implements Runnable {
	public static calcHelper cH = new calcHelper();
	public calcHelper.simpleMovingAverage diffSpeedSMA;
	public calcHelper.simpleMovingAverage sepSMA;
	public calcHelper.simpleMovingAverage turnrdsSMA;
	public calcHelper.simpleMovingAverage sumSpeedSMA;
	public calcHelper.simpleMovingAverage calcSpeedSMA;
	public calcHelper.simpleMovingAverage fuelTimeSMA;

	public calcHelper.simpleMovingAverage energyDiffSMA;
	// public static URL urlstate;
	// public static URL urlindicators;
	public double loc[];
	public double energyJKg;
	public double pEnergyJKg;
	public long calcPeriod;
	public static String buf;
	public static final double g = 9.80;
	public long timeStamp;
	public long freq;
	public state sState;
	public indicators sIndic;
	public String sStatus;
	public String sTime;
	public int iTotalHp;
	public String sTotalHp;
	public int iTotalHpEff;
	public String sTotalHpEff;
	public boolean bUnitMHp;
	public int iTotalThr;
	public String sTotalThr;
	public double fTotalFuel;
	public double fTotalFuelP;
	public boolean bLowAccFuel;
	public String sTotalFuel;
	public int iCheckAlt;
	public double dfuel;
	public long fueltime;
	public String sfueltime;
	public boolean notCheckInch;
	// public boolean isFuelpressure;
	public boolean altperCirclflag;
	public long intv;
	public double althour;
	public double altperCircle;
	public double alt;
	public double altp;
	public double altreg;
	public double iastotascoff;
	public long SystemTime;
	public long TimeIncrMili;
	long MainCheckMili;
	long MapCheckMili;
	long FuelCheckMili;
	public double fuelChange;
	long FuelLastchangeMili;
	long FuelchangeTime;
	long GCCheckMili;
	long SlowCheckMili;
	long intvCheckMili;
	long startTime;
	public long elapsedTime;

	public double noilTemp;
	public double nwaterTemp;
	// public int enginenum;
	// public int enginetype;

	public double speedv;
	public double speedvp;
	public double IASv;
	public double IASvp;
	public double diffspeed;
	public double acceleration;
	public double SEP;

	public long wepTime;

	public String salt;
	public String sSEP;
	public String sSEPAbs;
	
	public String sNitro;
	public String sWepTime;

	public controller c;

	// 对飞机结构有重大影响的警告
	public Boolean fatalWarn = false;

	// sState转换后
	public boolean hasWingSweepVario;
	public boolean isStateJet;

	public String svalid;
	public int engineNum;
	public String engineType;
	public String aileron;
	public String elevator;
	public String rudder;
	public String flaps;
	public String gear;
	public String TAS;
	public String IAS;
	public String M;
	public String AoA;
	public String AoS;
	public String Ao;
	public String Ny;
	public String Vy;
	public String Wx;
	public String sN;
	public String throttle;
	public String RPMthrottle;
	public String radiator;
	public String mixture;
	public String compass;
	public String sAcc;
	public String sTurnRds;
	public String sWingSweep;
	public String sTurnRate;
	public String compressorstage;
	public String magenato;
	public String power[];
	public String manifoldpressure;
	public String pressurePounds;
	public String pressureInchHg;
	public String pressureMmHg;
	public String watertemp;
	public String oiltemp;
	public String pitch[];
	public String thrust[];
	public String aclrt;
	public String relEnergy;
	public int curLoad;
	public double curLoadMinWorkTime;
	public String efficiency[];
	long testCheckMili;

	public long loadWorkTimeMili[];

	public double ratio;
	public double ratio_1;
	// iIndic
	public String rpm;
	private int curWLoad;
	private int curOLoad;
	private double nVy;
	public String sHorizontalLoad;
	public String sEngWorkTime;
	public String sPitchUp;
	public String sThurstPercent;
	public String sfuelPercent;
	public String sAvgEff;
	public String SdThrustPercent;
	public String sRadioAlt;
	public Boolean radioAltValid;
	public double radioAlt;
	public double pRadioAlt;
	public double dRadioAlt;
	public double An;
	public int iEngType;
	public double nitrokg;
	public double nitroConsump;
	public int nitroEngNr;
	
	Boolean portOcupied = false;
	private int checkEngineType;
	private int checkPitch;
	public boolean checkEngineFlag = false;
	public static final int ENGINE_TYPE_PROP = 0;
	public static final int ENGINE_TYPE_JET = 1;
	public static final int ENGINE_TYPE_TURBOPROP = 2;
	public static final int ENGINE_TYPE_UNKNOWN = -1;

	public String NumtoString(int Num, int arg) {
		return String.format("%0" + arg + "d", Num);
	}

	public String NumtoString(double Num, int arg1, int arg2) {
		return String.format("%-" + arg1 + "." + arg2 + "f", Num);
	}

	public static final String nastring = "-";
	public static final String nullstring = "";
	public boolean playerLive;
	public String sLoc;

	public boolean isPlayerLive() {
		return playerLive;
	}

	public static final String pressureUnit = "Ata";

	public void trans2String() {

		// 数据转换格式
		// sState

		//
		// if (iIndic.fuelpressure == true)
		// isFuelpressure = true;

		// if(sState.throttle <= 100)
		throttle = String.format("%d", sState.throttle);
		// else
		// throttle = "WEP";
		aileron = String.format("%d", sState.aileron);
		elevator = String.format("%d", sState.elevator);
		rudder = String.format("%d", sState.rudder);

		sTime = String.format("%02d'%02d", elapsedTime / 60000,  (elapsedTime / 1000) % 60);
		if (fueltime <= 0 || fueltime > 24 * 3600 * 1000)
			sfueltime = nastring;
		else {
			// if (fueltime < 60 * 1000)
			// sfueltime = String.format(".%d", fueltime / 1000);
			// else

			// sfueltime = String.format("%d:%02d", fueltime / 60000, (int)
			// ((fueltime / 1000) % 60 ));
			if (fueltime / 60000 < 100
					/* && !bLowAccFuel */) {
				sfueltime = String.format("%02d'%02d", fueltime / 60000, (long) ((fueltime / 1000) % 60 / 10) * 10);
				// sfueltime = String.format("%d.%d", fueltime / 60000,
				// (fueltime % 60000) / 6000);
			} else
				sfueltime = String.format("%.0f", (float)fueltime / 60000);

		}
		sTotalThr = String.format("%d", iTotalThr);
		if (iTotalHp == 0)
			sTotalHp = nastring;
		else
			sTotalHp = String.format("%d", iTotalHp);

		rpm = String.format("%d", (int) sState.RPM);
		if (iTotalHpEff >= 100000) {
			bUnitMHp = true;
			sTotalHpEff = String.format("%.2f", iTotalHpEff / 1000000.0f);
		} else {
			bUnitMHp = false;
			sTotalHpEff = String.format("%d", iTotalHpEff);
		}
		if (sState.efficiency[0] == 0)
			efficiency[0] = nastring;
		else
			efficiency[0] = String.format("%.0f", sState.efficiency[0]);
		// if (sState.watertemp == -65535) {
		// // app.debugPrint(iIndic.engine_temperature);
		// watertemp = String.format("%.0f", 0.0);
		// if (iIndic.water_temperature != -65535)
		// watertemp = String.format("%.0f", iIndic.water_temperature);
		// if (iIndic.engine_temperature != -65535)
		// watertemp = String.format("%.0f", iIndic.engine_temperature);
		//
		// } else {
		// watertemp = String.format("%.0f", sState.watertemp);
		// }
		if (nwaterTemp != -65535)
			watertemp = String.format("%.0f", nwaterTemp);
		else
			watertemp = nastring;
		oiltemp = String.format("%.0f", noilTemp);
		if (sState.manifoldpressure != 1) {
			manifoldpressure = String.format("%.2f", sState.manifoldpressure);
//			pressurePounds = String.format("%+d", Math.round((sState.manifoldpressure - 1) * 14.696));
			pressurePounds = String.format("%+.1f", (sState.manifoldpressure - 1) * 14.696);
			// pressurePounds = String.format("%+d",
			// Math.round((sState.manifoldpressure - 1f) * 14.696f),
			// Math.round(sState.manifoldpressure * 760f / 25.4f));
			// Math.round(sState.manifoldpressure * 760f / 25.4f)
			// pressureMmHg = String.format("%d",
			// Math.round(sState.manifoldpressure));
//			pressureInchHg = String.format("P/%d''", Math.round(sState.manifoldpressure * 760 / 25.4));
			pressureInchHg = String.format("P/%.1f''", (sState.manifoldpressure * 760 / 25.4));
		} else {
			manifoldpressure = nastring;
			pressurePounds = nastring;
			pressureMmHg = nastring;
			pressureInchHg = nastring;
		}
		sTotalFuel = String.format("%.0f", fTotalFuel);
		if (sState.pitch[0] != -65535)
			pitch[0] = String.format("%.1f", sState.pitch[0]);
		else
			pitch[0] = nastring;
		if (sState.RPMthrottle >= 0)
			RPMthrottle = String.format("%d", sState.RPMthrottle);
		else {
			RPMthrottle = nastring;
		}
		sThurstPercent = String.format("%.0f", thurstPercent);
		SdThrustPercent = String.format("%.0f", tEngResponse);

		if (sState.radiator >= 0)
			radiator = String.format("%d", sState.radiator);
		else
			radiator = nastring;

		if (sState.mixture >= 0)
			mixture = String.format("%d", sState.mixture);
		else {
			mixture = nastring;
		}
		flaps = String.format("%d", sState.flaps);
		sfuelPercent = String.format("%d", fuelPercent);
		if (hasWingSweepVario) {
			sWingSweep = String.format("%.0f", sIndic.wsweep_indicator * 100.f);
			// app.debugPrint(sWingSweep);
		} else
			sWingSweep = nastring;
		if (sIndic.radio_altitude >= 0) {
			sRadioAlt = String.format("%.0f", radioAlt);
		} else {
			sRadioAlt = nastring;
		}
		//
		if (avgeff == 0)
			sAvgEff = nastring;
		else
			sAvgEff = String.format("%d", Math.round(avgeff));
		// app.debugPrint(sWingSweep);
		Vy = String.format("%.1f", nVy);
		if (Math.abs(An) <= 1000)
			sN = String.format("%.1f", An / g);
		else
			sN = nastring;
		IAS = String.format("%d", sState.IAS);
		TAS = String.format("%d", sState.TAS);
		salt = String.format("%.0f", alt);
		Wx = String.format("%.0f", Math.abs(sState.Wx));
		M = String.format("%.2f", sState.M);
		Ny = String.format("%.1f", sState.Ny);

		// SEP取整改善SEP过高时的可读性
		double SEPAccuracy = (double)((long) SEP / 50);
		SEPAccuracy = SEPAccuracy * 2.5;
		if (SEPAccuracy == 0)
			SEPAccuracy = 1;

		sSEP = String.format("%.0f", Math.round(SEP / SEPAccuracy) * SEPAccuracy);
		sSEPAbs =  String.format("%.0f", Math.abs(Math.round(SEP / SEPAccuracy) * SEPAccuracy));
		// 相对能量(v^2/2+g*h)
		
		relEnergy = String.format("%.0f", energyJKg);
		
		aclrt = String.format("%.3f", acceleration);
		// Ao=String.format("%.1f",
		// Math.sqrt(sState.AoA*sState.AoA+sState.AoS*sState.AoS));
		AoA = String.format("%.1f", sState.AoA);
		AoS = String.format("%.1f", sState.AoS);
		// iIndic
		if (sIndic.compass != -65535)
			compass = String.format("%.0f", sIndic.compass);
		else 
			compass = "UND";
		
		sPitchUp = String.format("%.0f", sIndic.aviahorizon_pitch);
		
		if (c.blkx != null && c.blkx.valid && c.blkx.nitro != 0) {
			
			sNitro = String.format("%.0f", nitrokg);
			long twepTime = 0;
			if (nitroEngNr == 0) {
//				nitroEngNr = sState.engineNum;
//				sWepTime = nastring;
			}
			else {
				twepTime = (int) (((c.blkx.nitro / c.blkx.nitroDecr - wepTime / 1000)) / nitroEngNr) ;
			
				if (twepTime < 0) {
					twepTime = 0;
				}
				if (twepTime / 60 >= 100) {
					sWepTime = String.format("%3d", twepTime / 60);
				} else {
					sWepTime = String.format("%02d'%02d", twepTime / 60, twepTime %
					60);
					// sWepTime = String.format("%.0f", (double) twepTime);
				}
			}

		} else {
			sNitro = nastring;
			sWepTime = nastring;
		}

		sAcc = String.format("%.1f", acceleration);
		compressorstage = String.format("%d", sState.compressorstage);
		if (curLoadMinWorkTime == 99999 * 1000)
			sEngWorkTime = nastring;
		else
			sEngWorkTime = String.format("%.0f", curLoadMinWorkTime / 1000);

		if (Math.abs(turnRds) < 9999)
			sTurnRds = String.format("%.0f", Math.abs(turnRds));
		else
			sTurnRds = nastring;

		if (turnRate < 999)
			sTurnRate = String.format("%.1f", turnRate);
		else
			sTurnRate = nastring;
		sHorizontalLoad = String.format("%.1f", horizontalLoad);
		// app.debugPrint("已加力时间(秒)"+wepTime/1000);
		// app.debugPrint("剩余加力时间(分钟)"+rwepTime);
		
		//loc
		// char[] tmp = new char[8];
		// int stepx = (int)(loc[0] / 0.1);
		// int stepy = (int)(loc[1] / 0.1);

		// tmp[0] = (char) ('A' + stepy);
		// if (stepx < 9){
		// 	tmp[1] = (char) ('1' + stepx);
		// 	tmp[2] = '\0';
		// }
		// else{
		// 	tmp[1] = '1';
		// 	tmp[2] = (char)('0' + (stepx - 9));
		// 	tmp[3] = '\0';
		// }
		// sLoc = new String(tmp);
		// System.out.println("current location is:" + sLoc + "[stepx, stepy]" + stepx + ", " + stepy);
		
	}

	public void checkEngineJet() {

		// TODO:自适应方式获得,由磁电机判断. 只有活塞才有磁电机
		if (!checkEngineFlag) {
			if (sState.magenato < 0) {
				checkEngineType--;
			} else {

				checkEngineType++;
			}
			if (sState.pitch[0] != -65535) {
				checkPitch++;
			} else {
				checkPitch--;
			}
		
			if (Math.abs(checkEngineType) >= 100) {
				checkEngineFlag = true;
				if (checkEngineType >= 0) {
					iEngType = ENGINE_TYPE_PROP;
				} else {
	
					// 涡桨
					if (checkPitch > 0) {
						iEngType = ENGINE_TYPE_TURBOPROP;
						// app.debugPrint("涡桨\n");
					} else
						iEngType = ENGINE_TYPE_JET;
				}
	
				 //app.debugPrint(String.format("自适应判断引擎类型 %d\n", iEngType));
			}
		}
	}


	public void slowcalculate(long dtime) {
		// 计算耗油率及持续时间
		// app.debugPrint(totalfuelp - totalfuel);
		// if (MainCheckMili - FuelCheckMili > 1000) {
	
		
		dfuel = (fTotalFuelP - fTotalFuel) / dtime;

		if (dfuel > 0) {


			FuelchangeTime = MainCheckMili - FuelLastchangeMili;
			FuelLastchangeMili = MainCheckMili;
			fuelChange = fTotalFuelP - fTotalFuel; // 改变1公斤花了多长时间

			if (!bLowAccFuel) {
				// 改用滑动平均
				fueltime = (long) fuelTimeSMA.addNewData(fTotalFuel / dfuel);

			}
			else {
//				/* 已知油量不可能递增，考虑计算精度问题导致油量增多，因此取两者间最小值 */
				long tmpft = (long )fuelTimeSMA.addNewData(fTotalFuel * FuelchangeTime / fuelChange);
				if (fueltime > 0)
					fueltime = fueltime < tmpft ? fueltime : tmpft;
				else
					fueltime = tmpft;
//				app.debugPrint("" + fueltime +" " + tmpft);
			}
			// app.debugPrint(fuelChange);

		} else {
			// 没有变化，使用上次
			if (fuelChange == 0) 
				fueltime = 0;
			else {
				/* 已知油量不可能递增，考虑计算精度问题导致油量增多，因此取两者间最小值 */
				long tmpft = (long )fuelTimeSMA.addNewData(fTotalFuel * FuelchangeTime / fuelChange);
				fueltime = tmpft;
			}

		}

		if (fueltime < 0)
			fueltime = Long.MAX_VALUE;

		FuelCheckMili = MainCheckMili;
		fTotalFuelP = fTotalFuel;
//		prev_throttle = sState.throttle;
		// 计算变化率
//		app.debugPrint("" + fueltime);
	}

	// public engineLoad[] sPL;
	public void checkOverheat() {
		engineLoad[] pL = c.blkx.engLoad;
//		curLoad = c.blkx.findmaxLoad(pL, nwaterTemp, noilTemp);
		// 减去时间
		double minWorkTime = 99999 * 1000;
		/* 关发动机后，温度降到最低load后恢复 */
		Boolean engOff = false;
		if (sState.power[0] == 0 && sState.throttle > 0){
			/* 关发动机 */
			engOff = true;
//			app.debugPrint("监测到引擎关闭");
		}
		// 水冷
		curWLoad = c.blkx.findmaxWaterLoad(pL, nwaterTemp);
		for (int i = 0; i < c.blkx.maxEngLoad; i++) {
			if (i < curWLoad) {
				if (pL[i].WorkTime != 0) {
					pL[i].curWaterWorkTimeMili -= TimeIncrMili;
					if (pL[i].curWaterWorkTimeMili < minWorkTime) {
						minWorkTime = pL[i].curWaterWorkTimeMili;
					}
				}

			} else {
				
				if (engOff){
					// 关闭引擎直接回满
					if (curWLoad == 0 || pL[curWLoad - 1].WorkTime < 0.1) {
//						 app.debugPrint("回复水温耐久条");
						pL[i].curWaterWorkTimeMili = pL[i].WorkTime * 1000;
					}
				}
				else{
					// 大于load且工作时长不满则进行恢复
					if (sState.throttle <= 100) {
						if (pL[i].RecoverTime != 0 && (1000 * pL[i].WorkTime > pL[i].curWaterWorkTimeMili)) {
							pL[i].curWaterWorkTimeMili += (double) TimeIncrMili * pL[i].WorkTime / pL[i].RecoverTime;
						}
					}
				}

			
			}
		}

		// app.debugPrint("当前水工作负载: " + curLoad + "," + minWorkTime);
		// app.debugPrint("水工作负载数组: [");
		// for (int i = 0; i < c.blkx.maxEngLoad; i++) {
		// System.out.print(pL[i].curWaterWorkTimeMili / 1000 + " ");
		// }
		// app.debugPrint("]");

		// 油冷
		curOLoad = c.blkx.findmaxOilLoad(pL, noilTemp);
		for (int i = 0; i < c.blkx.maxEngLoad; i++) {
			if (i < curOLoad) {
				if (pL[i].WorkTime != 0) {
					pL[i].curOilWorkTimeMili -= TimeIncrMili;
					if (pL[i].curOilWorkTimeMili < minWorkTime) {
						minWorkTime = pL[i].curOilWorkTimeMili;
					}
				}

			} else {
				if (engOff){
					// 关闭引擎直接回满
					if (curOLoad ==0 || pL[curOLoad - 1].WorkTime < 0.1) {
//						 app.debugPrint("回复油温耐久条");
						pL[i].curOilWorkTimeMili = pL[i].WorkTime * 1000;
					}
				}
				else{
					// 大于load且工作时长不满则进行恢复
					if (sState.throttle <= 100) {
						if (pL[i].RecoverTime != 0 && (1000 * pL[i].WorkTime > pL[i].curOilWorkTimeMili)) {
							pL[i].curOilWorkTimeMili += (double) TimeIncrMili * pL[i].WorkTime / pL[i].RecoverTime;
						}
					}
				}
			}
		}

		//// app.debugPrint("当前油工作负载: " + curLoad + "," + minWorkTime);
		// app.debugPrint("油工作负载数组: [");
		// for (int i = 0; i < c.blkx.maxEngLoad; i++) {
		// System.out.print(pL[i].curOilWorkTimeMili / 1000 + " ");
		// }
		// app.debugPrint("]");

		curLoadMinWorkTime = minWorkTime;
	}

	// 转弯半径和转弯时间计算
	public double turnRds;
	public double turnRate;

	public double horizontalLoad;
	public double bangleR;

	public double altmeterp;
	public double altmeter;
	public double thurstPercent;
	private int maxTotalThr;
	public int fuelPercent;
	public double avgeff;
	private int maxTotalHp;
	private double TASv;
	private double pThurstPercent;
	public double tEngResponse;
	public double flapAllowSpeed;
	private int flapp;
	private int flap;
	private boolean downflap;
	private long flapCheck;
	double maximumThrRPM;
	// double maximumAllowedRPM;
	private long checkMaxiumRPM;
	public boolean getMaximumRPM;
	public httpHelper httpClient;
	public double energyM;

	public void calculateB() {
		// 计算斜抛角度,基本是正确的
		// double theta_rads = Math.toRadians(iIndic.aviahorizon_pitch);
		// double tantheta = Math.tan(theta_rads);
		// double costheta = Math.cos(theta_rads);
		// double bpow_vcostheta = (speedv * costheta) *(speedv * costheta);
		// double bxoffset = (- tantheta + Math.sqrt(tantheta * tantheta + (alt
		// * 2 * 9.78 / bpow_vcostheta) )) * bpow_vcostheta / 9.78;
		// bangleR = Math.toDegrees(Math.atan(alt/bxoffset));
		//
		// double pow_vcostheta = (speedv * costheta) *(speedv * costheta);
		// double xoffset = (- tantheta + Math.sqrt(tantheta * tantheta + (alt *
		// 2 * 9.78 / pow_vcostheta) )) * pow_vcostheta / 9.78;
		// double angleR = Math.toDegrees(Math.atan(alt/xoffset));

		// app.debugPrint(bangleR +"," + bxoffset);

		// 问题是怎么把斜抛角度映射到屏幕空间上
		// degree to Pixel
		// 求出屏幕空间角度比例FOV,然后像素点映射
	}

	public void updateWepTime() {
		nitroEngNr = 0;

		engineNum = sState.engineNum;
		for (int i = 0; i < engineNum; i++) {
			if (sState.throttles[i] > 100) {
				// 进入Wep状态
				// app.debugPrint(TimeIncrMili);
				wepTime += TimeIncrMili;
				nitroEngNr += 1;
			}
		}
		nitrokg = c.blkx.nitro - (wepTime * nitroConsump) / 1000;
		if (nitrokg < 0)
			nitrokg = 0;
		
	}

	public void updateTemp() {
		noilTemp = sIndic.oilTemp;
		nwaterTemp = sIndic.waterTemp;
		if (noilTemp <= -65534) {
			noilTemp = sState.oiltemp;
		}
		if (nwaterTemp <= -65534) {
			nwaterTemp = sIndic.engine_temperature;
			if (nwaterTemp <= -65534)
				nwaterTemp = sState.watertemp;
		}
	}

	public void updateAlt() {
		altp = alt;
		alt = sState.heightm;

		altmeterp = altmeter;
		altmeter = sIndic.altitude_10k;

		// 人类毒瘤英制飞机
		if (!notCheckInch && Math.abs(sState.Vy) > 0) {
			if ((Math.abs(altmeter - altmeterp) * 1000 > Math.abs(2 * sState.Vy * intv))) {
				iCheckAlt += intv;
			} else {
				iCheckAlt -= intv;
			}
			if (Math.abs(iCheckAlt) > 10000)
				notCheckInch = true;
		}

		// if (checkAlt > 2)
		// alt = alt * 0.3048f;
		// // app.debugPrint(Math.abs(alt - altp)*1000+"?"+Math.abs(2 *
		// // sState.Vy * intv));
		//
		// // 解决熊猫的高度问题
		// alt = alt + altperCircle * altreg;
		// app.debugPrint("checkalt"+checkAlt);

		// 无线电高度
		pRadioAlt = radioAlt;
		// radioAlt = iIndic.radio_altitude;

		if (sIndic.radio_altitude == stringHelper.fInvalid) {
			radioAlt = alt;
			radioAltValid = false;
		}else {
			radioAltValid = true;
			if (iCheckAlt > 0) {
				radioAlt = sIndic.radio_altitude * 0.3048f;
			} else {
				radioAlt = sIndic.radio_altitude;
			}
		}
		dRadioAlt = (ratio_1 * dRadioAlt) + ratio * 1000.0f * (radioAlt - pRadioAlt) / intv;
		// app.debugPrint(dRadioAlt);

	}

	public void updateClimbRate() {
		if (sIndic.vario != -65535) {
			// 如果有爬升率表，使用爬升率表订正Vy
			nVy = sIndic.vario;

		} else {
			nVy = sState.Vy;
		}

	}

	public void updateTurn() {
		// 转弯半径等于speedv*speedv/9.78*G

		// 转弯加速度约等于法向过载与重力过载之和
		double alpha = 0;
		double beta = 0;
//		if (sIndic.aviahorizon_roll != -65535) {
//			alpha = sIndic.aviahorizon_roll;
//		}
//		if (sIndic.aviahorizon_pitch != -65535) {
//			beta = sIndic.aviahorizon_pitch + sState.AoA;
//		}

		if (sIndic.aviahorizon_roll != -65535 && sIndic.aviahorizon_pitch != -65535) {
			// 获得横滚角
			
			An = (double) (g * Math
					.sqrt(sState.Ny * sState.Ny + 1 - 2 * sState.Ny * Math.cos(Math.toRadians(sIndic.aviahorizon_roll))
							* Math.cos(Math.toRadians(sIndic.aviahorizon_pitch + sState.AoA))));
			// An = (double)(g * Math.sqrt(a))
		} else
			An = (g * sState.Ny);
		// app.debugPrint(Math.cos(sIndic.aviahorizon_roll));
		// 计算时取前后两次采样的速度平均值

		// double sumspeed1 = sumSpeedSMA.addNewData(speedv + speedvp);

		if (sIndic.turn != -65535) {

			// double cosa = Math.cos(Math.toRadians(sIndic.aviahorizon_roll));
			// double cosb = Math.cos(Math.toRadians(sIndic.aviahorizon_pitch +
			// sState.AoA)) ;
			// double tAn = (double)(g*
			// Math.sqrt(sState.Ny*sState.Ny * cosa * cosa * cosb * cosb
			// + 1
			// - 2 * sState.Ny * cosa * cosb
			// + Math.abs(sIndic.turn) * Math.abs(sIndic.turn) * (speedvp +
			// speedv) * (speedvp + speedv) / (4*g*g))
			// );
			//
			// System.out.println(tAn - An);
			// An = tAn;
			horizontalLoad = Math.abs(sIndic.turn) * (speedvp + speedv) / (2 * g);
			// horizontalLoad = Math.abs(sIndic.turn) * sumspeed1 / (2 * g);s

		} else {
			horizontalLoad = 0;
		}

		// turnRds = (sumspeed1 * sumspeed1) / (4 * An);
		// turnRds = (speedvp + speedv) * (speedvp + speedv) / (4 * An) ;
		if (An != 0) {
			turnRds = turnrdsSMA.addNewData((speedvp + speedv) * (speedvp + speedv) / (4 * An));
			// 转弯率等于向心加速度除以半径开根号
			turnRate = (double) (Math.toDegrees(Math.sqrt(An / turnRds)));
			// turnRds = turnRds/10 * 10
		}
		else{
			
		}
	}

	public void updateSpeed() {

		speedvp = speedv;
		IASvp = IASv;
		// TASvp = TASv;

		IASv = sState.IAS;

		// vTASp = vTAS;;
		TASv = (double) sState.TAS;
		double tspeedv;
		/* 有速度表使用速度表 */
		if (sIndic.speed != -65535) {
			// 高精度指示空速，需要进行TAS校正
			tspeedv = sIndic.speed;
		}
		else{
			tspeedv = IASv / 3.6;
		}

		if (tspeedv != 0) {
			// iastotascoff = (1000 * ratio_1 * iastotascoff + 1000 * ratio
			// * vTAS / (speedv * 3.6f)) / 1000.0f;
			// 改用滑动平均
			iastotascoff = calcSpeedSMA.addNewData(TASv / (tspeedv * 3.6));
		}
		// iastotascoff = 1+(double) (0.02 * sState.heightm * 3.2808 /
		// 1000);
		speedv = tspeedv * iastotascoff;
		// app.debugPrint("校正TAS:"+ speedv*3.6);
		// 订正后加速度还是会有跳变
		// app.debugPrint("校正TAS:"+ speedv*3.6 + "," + iastotascoff);

	}

	public boolean isEngJet() {
		return iEngType == ENGINE_TYPE_JET;
	}

	public void updateEngineState() {
		int i;


		checkEngineJet();
		if (!isEngJet()) {
			// 活塞机或者涡浆机

			double ttotalhp = 0;
			double ttotalhpeff = 0;
			double ttotalthr = 0;
			for (i = 0; i < engineNum; i++) {
				ttotalthr = ttotalthr + sState.thrust[i];
				// app.debugPrint(sState.engineNum);
				ttotalhp = ttotalhp + sState.power[i];
				ttotalhpeff = ttotalhpeff + sState.thrust[i] * g * speedv / 735;
			}
//			System.out.println(ttotalhp);
			// app.debugPrint(totalhpeff);
//			app.debugPrint(String.format("sevice 引擎数量%d, 功率%.0f", engineNum, ttotalhp));
			iTotalHp = (int) (ttotalhp);
			iTotalHpEff = (int) (ttotalhpeff);
			iTotalThr = (int) (ttotalthr);

			if (iTotalHp != 0)
				avgeff = (double) 100 * iTotalHpEff / iTotalHp;
			else
				avgeff = 0;
		} else {
			// 喷气机
			double ttotalthr = 0;
			for (i = 0; i < engineNum; i++) {
				// app.debugPrint(sState.thrust[0]);
				ttotalthr = ttotalthr + sState.thrust[i];
			}
			// app.debugPrint(totalthr+" "+totalhpeff);
			double ttotalhpeff = ((ttotalthr * g * speedv) / 735);

			iTotalThr = (int) ttotalthr;
			iTotalHpEff = (int) ttotalhpeff;

			avgeff = 0;
		}

		if (maxTotalThr < iTotalThr && sState.throttle >= 100) {
			maxTotalThr = (int) (ratio_1 * maxTotalThr + ratio * iTotalThr);
		}
		if (maxTotalHp < iTotalHpEff && sState.throttle >= 100) {
			maxTotalHp = (int) (ratio_1 * maxTotalHp + ratio * iTotalHpEff);
		}

		pThurstPercent = thurstPercent;
		if (isEngJet() && maxTotalThr != 0) {
			// 喷气机
			if (maxTotalThr != 0) {
				thurstPercent = 100.0f * iTotalThr / maxTotalThr;
			}
		} else {
			if (maxTotalHp != 0) {
				thurstPercent = 100.0f * iTotalHpEff / maxTotalHp;
			}
		}

		tEngResponse = (ratio_1 * tEngResponse) + ratio * (thurstPercent - pThurstPercent) * 1000.0f / intv;

	}

	public void updateFuel() {
		int i;
		if (sIndic.fuelnum != 0) {
			double ttotalfuel = 0;
			bLowAccFuel = Boolean.FALSE;
			/* 修复su-27油箱显示不正确的问题 */
			// for (i = 0; i < sIndic.fuelnum; i++) {
			for (i = 0; i < 1; i++){
				ttotalfuel = ttotalfuel + sIndic.fuel[i];
			}
			fTotalFuel = ttotalfuel;
			
		}
		// app.debugPrint("I"+totalfuel);
		if (fTotalFuel == 0) {
			bLowAccFuel = Boolean.TRUE;
			fTotalFuel = sState.mfuel;
		}
		fuelPercent = (int) (100 * fTotalFuel / sState.mfuel0);

	}

	public void updateSEP() {
		// if (sState.IAS != 0) {
		double diffspeed1 = diffSpeedSMA.addNewData(speedv - speedvp);
		// 这是不是示空速的差?
		// 使用修正
		// diffspeed = (ratio_1 * diffspeed + ratio * (speedv - speedvp));
		diffspeed = diffspeed1;
		// app.debugPrint(diffspeed);
		acceleration = diffspeed * 1000.0 / intv;

		// 三种计算方式

		// 这两种等价
		// SEP = acceleration * (speedvp + speedv) / (2 * g) + nVy;
		// SEP /= 2;

		// SEP = (diffspeed * (speedv + speedvp)) /((2 * intv * g)/ 1000.0f) +
		// nVy;
		// -38.8 4.7 = 28
		// SEP = SEP1;

		// 跳变太大, 没法读数

		// SEP = ((speedv * speedv) - (speedvp * speedvp))*1000.0f/(2 * intv *
		// g) + nVy;

		SEP = sepSMA.addNewData(((speedv + speedvp) * (speedv - speedvp) * 1000) / (2 * intv * g) + nVy);
		// SEP /= 2;

		// } else {
		// acceleration = 0;
		// SEP = 0;
		// }

		// 总能量
		//pEnergyJKg = energyJKg;
		// energyJKg = ((speedv + speedvp) * (speedv + speedvp) / (8 * g) +
		// sState.heightm);
		energyJKg = ((speedv + speedvp) * (speedv + speedvp) / 8 + g * sState.heightm);
		energyM = ((speedv + speedvp) * (speedv + speedvp) / (8*g) + sState.heightm);
		// System.out.println(String.format("%.0f",
		// energyDiffSMA.addNewData((energyJKg - pEnergyJKg)*1000/intv)));
	}

	public void checkWing() {
		if (sIndic.wsweep_indicator != -65535)
			hasWingSweepVario = true;
		else
			hasWingSweepVario = false;
	}

	public void checkFlap() {
		flapp = flap;
		flap = sState.flaps;
		if (flap - flapp > 0) {
			downflap = true;
		} else if (flap == flapp) {
			// 加计数
			flapCheck += intv;

			// 维持1秒稳定
			if (flapCheck >= 1000) {
				flapCheck = 0;
				downflap = false;
			}
		} else {
			// 小于则一定是收
			downflap = false;
		}

		flapAllowSpeed = getFlapAllowSpeed(sState.flaps, downflap);
	}

	public void getMaximumRPM() {
		if (!getMaximumRPM) {
			if (c.blkx != null && c.blkx.valid) {
				// FM合法直接取FM
				maximumThrRPM = c.blkx.maxRPM;
				// 使用最大允许RPM
				// maximumThrRPM = c.blkx.maxAllowedRPM;
				// app.debugPrint(maximumThrRPM);
				getMaximumRPM = true;
			} else {
				// 自适应获得(无FM)

				// 获得最大转速，条件是以最大转速持续约20秒或者桨距
				if (checkMaxiumRPM < 20000 / freq) {
					if (sState.IAS > 50) {
						if (sState.RPM >= maximumThrRPM) {
							// app.debugPrint(sState.RPM
							// +","+maximumThrRPM);
							maximumThrRPM = (ratio_1 * maximumThrRPM) + ratio * (sState.RPM);
						}
						checkMaxiumRPM++;
					}
				} else {
					getMaximumRPM = true;
				}
			}
		}
	}

	public void calculate() {

		// 获得开始时间
		elapsedTime = SystemTime - startTime;

		// 增加wep时间
		updateWepTime();

		// 更新温度，优先使用更精确的
		updateTemp();
		// 检查是否过热，如果过热，计算引擎健康度
		checkOverheat();

		// 更新爬升率
		updateClimbRate();

		// 获得准确高度,需要依赖Vy因此要放到爬升率后面
		updateAlt();

		// 更新速度
		updateSpeed();

		// 更新转弯半径
		updateTurn();

		// app.debugPrint(horizontalLoad);
		// 计算总推力、总功率和总轴功率
		updateEngineState();
		// 计算总油量
		updateFuel();

		// 计算SEP
		updateSEP();

		// 可变翼判断
		checkWing();

		// 襟翼判断
		checkFlap();
		// app.debugPrint(flapAllowSpeed);

		// 获得最大转速
		getMaximumRPM();

		// TODO:升力阻力实时计算
		// TODO:临界速度和马赫数动态计算(考虑可变翼)

		// TODO:可用过载动态计算(油、重量)

	}

	public double getFlapAllowSpeed(int flapPercent, Boolean isDowningFlap) {
		// fm文件无法解析
		if (flapPercent == 0 || c.blkx == null || !c.blkx.valid)
			return Double.MAX_VALUE;

		// 找到襟翼档位
		int i = 0;
		for (; i < c.blkx.FlapsDestructionNum; i++) {
			// 大于
			if (flapPercent < c.blkx.FlapsDestructionIndSpeed[i][0] * 100.0f) {
				break;
			}
		}

		// 找到档位了
		// 线性求值
		// 找前面的flap值
		double x0, x1, y0, y1;
		double k;
		// 没有找到，都小于

		if (i == 0) {
			// 下襟翼时直接越级使用下一级
			if (isDowningFlap) {
				return c.blkx.FlapsDestructionIndSpeed[i][1];
			}
			// 襟翼只有0级
			// if(c.blkx.FlapsDestructionNum == 0){
			// return c.blkx.FlapsDestructionIndSpeed[0][1];
			// }
			return Double.MAX_VALUE;
		} else {
			// 下襟翼时直接越级使用
			// if (isDowningFlap) {
			// return c.blkx.FlapsDestructionIndSpeed[i][1];
			// }

			// 相等
			if (flapPercent == c.blkx.FlapsDestructionIndSpeed[i - 1][0] * 100.0f) {
				// 直接返回速度
				return c.blkx.FlapsDestructionIndSpeed[i - 1][1];
			}

			// 否则进行线性插值运算
			// 算斜率
			x0 = c.blkx.FlapsDestructionIndSpeed[i - 1][0] * 100.0f;
			y0 = c.blkx.FlapsDestructionIndSpeed[i - 1][1];
			x1 = c.blkx.FlapsDestructionIndSpeed[i][0] * 100.0f;
			y1 = c.blkx.FlapsDestructionIndSpeed[i][1];
			if (x1 - x0 != 0) {
				k = (y1 - y0) / (x1 - x0);
			} else {
				k = 0;
			}
			// 速度等于
			// app.debugPrint(x0 + "-" + x1 + ", " + y0 + "-" + y1);
			return y0 + (flapPercent - x0) * k;
		}

	}
	public void resetEngLoad(){
		if(c.blkx != null && c.blkx.valid){
			for (int idx = 0; idx < c.blkx.maxEngLoad; idx++){
				c.blkx.engLoad[idx].curWaterWorkTimeMili = c.blkx.engLoad[idx].WorkTime * 1000;
				c.blkx.engLoad[idx].curOilWorkTimeMili = c.blkx.engLoad[idx].WorkTime * 1000;
			}
		}
	}
	// 重置变量
	public void resetvaria() {
		loc = new double[2];
		radioAltValid = false;
		playerLive = false;
		iEngType = ENGINE_TYPE_UNKNOWN;
		checkMaxiumRPM = 0;
		getMaximumRPM = false;
		dRadioAlt = 0;
		curLoad = 0;
		wepTime = 0;
		energyJKg = 0;
		pEnergyJKg = 0;
		elapsedTime = 0;
		altperCircle = 0;
		iCheckAlt = 0;
		altreg = 0;
		altp = 0;
		alt = 0;
		calcPeriod = 0;
		maximumThrRPM = 1;
		maxTotalThr = 0;
		iastotascoff = 1;
		thurstPercent = 0;
		checkEngineFlag = false;
		checkEngineType = 0;
		fueltime = Long.MAX_VALUE;
		checkPitch = 0;
		fuelPercent = 0;
		maxTotalHp = 0;
		maxTotalThr = 0;
		diffspeed = 0;
		curLoadMinWorkTime = 99999 * 1000;
		/* 刷新引擎工作时间 */
		resetEngLoad();
		// if(c.blkx != null && c.blkx.maxEngLoad != 0)c.blkx.resetEngineLoad();
		FuelCheckMili = System.currentTimeMillis();
		MapCheckMili = FuelCheckMili;
		MainCheckMili = FuelCheckMili;
		notCheckInch = false;
		altperCirclflag = false;
		// isFuelpressure = false;
		notCheckInch = false;
		hasWingSweepVario = false;
		flapAllowSpeed = Float.MAX_VALUE;
		fTotalFuelP = 0;
		isStateJet = false;
		nitrokg = 0;
		nitroConsump = 0;
		nitroEngNr = 0;
		
		calcSpeedSMA = cH.new simpleMovingAverage((int) (1000 / freq));
		diffSpeedSMA = cH.new simpleMovingAverage((int) (1000 / freq));
		sepSMA = cH.new simpleMovingAverage((int) (1000 / freq));
		turnrdsSMA = cH.new simpleMovingAverage((int) (1000 / freq));
		sumSpeedSMA = cH.new simpleMovingAverage((int) (1000 / freq));
		energyDiffSMA = cH.new simpleMovingAverage((int) (1000 / freq));
		fuelTimeSMA = cH.new simpleMovingAverage(4);
		if (c.blkx != null) {
			engineLoad[] pL = c.blkx.engLoad;
			nitrokg = c.blkx.nitro;
			nitroConsump = c.blkx.nitroDecr;
			if (pL != null) {
				for (int i = 0; i < c.blkx.maxEngLoad; i++) {
					pL[i].curWaterWorkTimeMili = pL[i].curWaterWorkTimeMili;
					pL[i].curOilWorkTimeMili = pL[i].curOilWorkTimeMili;
				}
			}
		}
		
	}

	public void clearvaria() {
		// sState = null;
		// iIndic = null;
		resetvaria();
	}

	public void clear() {
		// app.debugPrint("执行清洁");
		clearvaria();

		System.gc();
		sState.init();
		sIndic.init();

	}

	public service(controller xc) {

		c = xc;

		freq = xc.freqService;
		clearvaria();

		ratio = freq / 1000.0f;
		ratio_1 = 1.0f - ratio;
		sState = new state();
		sState.init();
		sIndic = new indicators();
		sIndic.init();
		httpClient = new httpHelper();
		power = new String[state.maxEngNum];
		pitch = new String[state.maxEngNum];
		thrust = new String[state.maxEngNum];
		efficiency = new String[state.maxEngNum];
		FuelCheckMili = System.currentTimeMillis();
		// isFuelpressure = false;

	}

	public void checkState() {
		int conState;
		// 更新时间戳
		timeStamp = SystemTime;
		// app.debugPrint("s:"+httpClient.strState+"s1:"+httpClient.strIndic);
		// 更新state

		c.initStatusBar();
		if (httpClient.strState.length() > 0 && httpClient.strIndic.length() > 0) {
			// 改变状态为连接成功
			// app.debugPrint(sState);
			conState = sState.update(httpClient.strState);

			sIndic.update(httpClient.strIndic);
			c.changeS2();
			if (sState.flag && sIndic.flag) {

				/* 修复录像中没法使用的问题 */
				if ((!sIndic.type.equals("DUMMY_PLANE")) && ((sState.totalThr != 0) || (sState.RPM != 0))) {
					playerLive = true;
				}

				if (isPlayerLive()) {
					c.changeS3();// 打开面板
					if (!c.cur_fmtype.equals(sIndic.type)) {
						// 机型变化
						app.debugPrint("机型变化，重启程序");
						c.S4toS1();
					}
					// speedvp = sState.IAS;
					// 开始计算数据
					calculate();
					
					
					// 检测到加油，重置数据
					if ((Math.abs(speedv) < 10) && (fTotalFuel - fTotalFuelP > 1)) {
//						app.debugPrint("检测到油量增加 " + fTotalFuel + "," + fTotalFuelP);
						app.debugPrint("重新加油，重置变量");
						
						resetvaria();
					}

					
					// 0.5秒一次慢计算
					if (((calcPeriod++) % (500 / freq)) == 0)
						slowcalculate((500 / freq) * freq);

					// 将数据转换格式
					trans2String();

					// 写入文档
					// c.writeDown();

					// 检查死亡
					if (sState.totalThr == 0 && sState.RPM <= 0 && sState.IAS < 10) {
						app.debugPrint("检测到玩家坠毁");
						playerLive = false;
					}
				}
			} else {
				// 状态置为等待游戏开始（状态1）
				// c.changeS2();//连接成功等待游戏开始

				c.S4toS1();
				// app.debugPrint("等待游戏开始");
				try {
					Thread.sleep(500);
				} catch (InterruptedException e) {
					// TODO Auto-generated catch block
					e.printStackTrace();
				}
			}

		} else {
			// 状态置为等待连接中
			conState = -1;
			c.S4toS1();
			app.debugPrint("等待连接中");
		}
		if (conState == -1) {
			// 端口连接可能有问题，切换端口
			// app.debugPrint("切换端口\n");
			portOcupied = !portOcupied;
		}
	}

	@Override
	public void run() {
		long waitMili = app.threadSleepTime;
		// app.debugPrint("" + waitMili);
		while (true) {

			try {
				Thread.sleep(waitMili);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
				return;
			}

			SystemTime = System.currentTimeMillis();
			long diffTime = SystemTime - MainCheckMili;
			if (diffTime >= freq) {

				// 尝试GET数据
				if (!portOcupied)
					httpClient.getReqResult(app.requestDest);
				else
					httpClient.getReqResult(app.requestDestBkp);
				
				intv = (diffTime / freq) * freq;
				TimeIncrMili = diffTime;
				MainCheckMili += intv;
				// MainCheckMili = SystemTime;

				// 检查是否需要改变状态
				checkState();

				// 记录
				if (c.logon) {
					flightLog tempLog = c.Log;
					if (tempLog != null)
						tempLog.logTick();
				}
				// app.debugPrint("?\n");
				// 检查超时
//				 if (MainCheckMili <= (System.currentTimeMillis() - intv)) {
//					 app.debugPrint("deadline Miss, try catch\n" + SystemTime +
//							 "," + MainCheckMili);
//				 }
			}
			long diffTime1 = SystemTime - MapCheckMili;
			if (diffTime1 >= 10 * freq) {
				MapCheckMili = SystemTime;
				if (!portOcupied)
					httpClient.getReqMapObjResult(app.requestDest);
				else
					httpClient.getReqMapObjResult(app.requestDestBkp);
				mapObj.getPlayerLoc(httpClient.strMapObj, loc);
				// mapObj.getAirfieldLoc(httpClient.strMapObj, null);
			}
			

		}
	}
}