package prog;

import prog.util.HttpHelper;

import parser.Blkx.engineLoad;
import parser.FlightLog;
import parser.Indicators;
import parser.MapInfo;
import parser.MapObj;
import parser.State;
import prog.util.StringHelper;
import prog.util.CalcHelper;
import java.util.HashMap;
import java.util.Map;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;

public class Service implements Runnable, ui.model.TelemetrySource {
	public static CalcHelper cH = new CalcHelper();
	public CalcHelper.SimpleMovingAverage diffSpeedSMA;
	public CalcHelper.SimpleMovingAverage sepSMA;
	public CalcHelper.SimpleMovingAverage turnrdsSMA;
	public CalcHelper.SimpleMovingAverage sumSpeedSMA;
	public CalcHelper.SimpleMovingAverage calcSpeedSMA;
	public CalcHelper.SimpleMovingAverage fuelTimeSMA;

	public CalcHelper.SimpleMovingAverage energyDiffSMA;
	// public static URL urlstate;
	// public static URL urlindicators;
	public double loc[];
	public double dir[];
	public double energyJKg;
	public double pEnergyJKg;
	public long calcPeriod;
	public static String buf;
	public static final double g = 9.80;
	public long timeStamp;
	public long freq;
	public State sState;
	public Indicators sIndic;
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

	public Controller c;

	// 对飞机结构有重大影响的警告
	public Boolean fatalWarn = false;

	// sState转换后
	public boolean hasWingSweepVario;
	public boolean isStateJet;
	public double dCompass;
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
	public String pressureUnitStr = "Ata";
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
	public MapInfo mapinfo;

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

		sTime = String.format("%02d'%02d", elapsedTime / 60000, (elapsedTime / 1000) % 60);
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
				sfueltime = String.format("%.0f", (float) fueltime / 60000);

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
		// // Application.debugPrint(iIndic.engine_temperature);
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
			pressurePounds = String.format("%+.1f", (sState.manifoldpressure - 1) * 14.696);
			pressureInchHg = String.format("P/%.1f''", (sState.manifoldpressure * 760 / 25.4));

			if (this.iCheckAlt > 0) {
				// Imperial Mode: Value is Boost (psi), Unit is Manifold (inHg)
				this.manifoldpressure = pressurePounds;
				this.pressureUnitStr = pressureInchHg;
			} else {
				// Metric Mode: Value is Ata, Unit is Ata
				this.manifoldpressure = String.format("%.2f", sState.manifoldpressure);
				this.pressureUnitStr = "Ata";
			}
		} else {
			manifoldpressure = nastring;
			pressurePounds = nastring;
			pressureMmHg = nastring;
			pressureInchHg = nastring;
			this.pressureUnitStr = "Ata";
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
			// Application.debugPrint(sWingSweep);
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
		// Application.debugPrint(sWingSweep);
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
		double SEPAccuracy = (double) ((long) SEP / 50);
		SEPAccuracy = SEPAccuracy * 2.5;
		if (SEPAccuracy == 0)
			SEPAccuracy = 1;

		sSEP = String.format("%.0f", Math.round(SEP / SEPAccuracy) * SEPAccuracy);
		sSEPAbs = String.format("%.0f", Math.abs(Math.round(SEP / SEPAccuracy) * SEPAccuracy));
		// 相对能量(v^2/2+g*h)

		relEnergy = String.format("%.0f", energyJKg);

		aclrt = String.format("%.3f", acceleration);
		// Ao=String.format("%.1f",
		// Math.sqrt(sState.AoA*sState.AoA+sState.AoS*sState.AoS));
		if (sState.AoA != -65535) {
			AoA = String.format("%.1f", sState.AoA);
			AoS = String.format("%.1f", sState.AoS);
		} else {
			AoA = nastring;
			AoS = nastring;
		}
		compass = String.format("%.0f", dCompass);
		sPitchUp = String.format("%.0f", sIndic.aviahorizon_pitch);

		if (c.getBlkx() != null && c.getBlkx().valid && c.getBlkx().nitro != 0) {

			sNitro = String.format("%.0f", nitrokg);
			long twepTime = 0;
			if (nitroEngNr == 0) {
				// nitroEngNr = sState.engineNum;
				// sWepTime = nastring;
			} else {
				twepTime = (int) (((c.getBlkx().nitro / c.getBlkx().nitroDecr - wepTime / 1000)) / nitroEngNr);

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
		// Application.debugPrint("已加力时间(秒)"+wepTime/1000);
		// Application.debugPrint("剩余加力时间(分钟)"+rwepTime);

		// loc
		// char[] tmp = new char[8];
		// int stepx = (int)(loc[0] / 0.1);
		// int stepy = (int)(loc[1] / 0.1);

		// tmp[0] = (char) ('A' + stepy);
		// if (stepx < 9){
		// tmp[1] = (char) ('1' + stepx);
		// tmp[2] = '\0';
		// }
		// else{
		// tmp[1] = '1';
		// tmp[2] = (char)('0' + (stepx - 9));
		// tmp[3] = '\0';
		// }
		// sLoc = new String(tmp);
		// System.out.println("current location is:" + sLoc + "[stepx, stepy]" + stepx +
		// ", " + stepy);

		updateGlobalPool();
	}

	private void updateGlobalPool() {
		// 1. Build Data Snapshot
		Map<String, String> data = new HashMap<>();

		// Push Standard Flight Data (Strings formatted in trans2String)
		data.put("TAS", TAS);
		data.put("IAS", IAS);
		data.put("Mach", M);
		data.put("AoA", AoA);
		data.put("AoS", AoS);
		data.put("Ny", Ny); // G-Load raw
		data.put("G", sN); // G-Load formatted
		data.put("Wx", Wx);

		data.put("Altitude", salt);
		data.put("RadioAltitude", sRadioAlt);
		data.put("Vario", Vy);

		data.put("Compass", compass);

		data.put("throttle", throttle);
		data.put("RPM", rpm);
		data.put("manifold_pressure", manifoldpressure);
		data.put("water_temp", watertemp);
		data.put("oil_temp", oiltemp);
		data.put("pitch", pitch != null && pitch.length > 0 ? pitch[0] : "N/A");

		data.put("fuel", sTotalFuel);
		data.put("fuel_time", sfueltime);

		data.put("SEP", sSEP);
		data.put("SEP_abs", sSEPAbs);
		data.put("acceleration", sAcc);

		data.put("turn_rate", sTurnRate);
		data.put("turn_radius", sTurnRds);

		data.put("wing_sweep", sWingSweep);
		data.put("flaps", flaps);
		data.put("gear", gear);
		data.put("aileron", aileron);
		data.put("elevator", elevator);
		data.put("rudder", rudder);

		data.put("valid", svalid);

		// Push FlightInfoConfig Compatible Keys
		data.put("ias", IAS);
		data.put("tas", TAS);
		data.put("mach", M);
		data.put("dir", compass);
		data.put("height", salt);
		data.put("rda", sRadioAlt);
		data.put("vario", Vy);
		data.put("sep", sSEP);
		data.put("acc", sAcc);
		data.put("wx", Wx);
		data.put("ny", sN);
		data.put("turn", sTurnRate);
		data.put("rds", sTurnRds);
		data.put("aoa", AoA);
		data.put("aos", AoS);
		data.put("aoa", AoA);
		data.put("aos", AoS);
		data.put("ws", sWingSweep);

		// Event-Driven Raw Data Support (Double values as Strings)
		data.put("alt_val", String.valueOf(alt));
		data.put("sep_val", String.valueOf(SEP));
		data.put("compass_val", String.valueOf(dCompass));
		data.put("ias_val", String.valueOf(IASv));

		// Push EngineInfoConfig Compatible Keys
		data.put("hp", sTotalHp);
		data.put("thrust", sTotalThr);
		data.put("eff_eta", sAvgEff);
		data.put("eff_hp", sTotalHpEff);
		data.put("eff_hp", sTotalHpEff);
		data.put("pressure", manifoldpressure);
		data.put("pressure_unit", pressureUnitStr);
		data.put("power_percent", sThurstPercent);
		data.put("fuel_kg", sTotalFuel);
		// fuel_time already exists
		data.put("wep", sNitro);
		data.put("wep_time", sWepTime);
		data.put("temp", watertemp);
		// oil_temp already exists
		data.put("heat_time", sEngWorkTime);
		data.put("response", SdThrustPercent);

		// Push EngineControl Compatible Keys
		data.put("mixture", mixture);
		data.put("radiator", radiator);
		data.put("compressor", compressorstage);
		data.put("fuel_percent", sfuelPercent);
		data.put("rpm_throttle", RPMthrottle);
		data.put("thrust_percent", sThurstPercent);
		data.put("is_jet", String.valueOf(iEngType == ENGINE_TYPE_JET));
		data.put("engine_check_done", String.valueOf(checkEngineFlag));
		if (sState != null) {
			data.put("throttle_int", String.valueOf(sState.throttle));
			data.put("mixture_int", String.valueOf(sState.mixture));
			data.put("radiator_int", String.valueOf(sState.radiator));
			data.put("rpm_throttle_int", String.valueOf(sState.RPMthrottle));
			data.put("compressor_int", String.valueOf(sState.compressorstage));
			// MinimalHUD state keys
			data.put("airbrake_int", String.valueOf(sState.airbrake));
			data.put("gear_int", String.valueOf(sState.gear));
			data.put("flaps_int", String.valueOf(sState.flaps));
			data.put("AoA_f", String.valueOf(sState.AoA));
			data.put("AoS_f", String.valueOf(sState.AoS));
			data.put("Ny_f", String.valueOf(sState.Ny));
			data.put("IAS_f", String.valueOf(sState.IAS));
			data.put("M_f", String.valueOf(sState.M));
		}
		data.put("fuel_percent_int", String.valueOf(fuelPercent));
		data.put("thrust_percent_int", String.valueOf((int) thurstPercent));

		// MinimalHUD attitude & warning keys (from sIndic)
		if (sIndic != null) {
			data.put("aviahorizon_pitch", String.valueOf(sIndic.aviahorizon_pitch));
			data.put("aviahorizon_roll", String.valueOf(sIndic.aviahorizon_roll));
			data.put("compass_f", String.valueOf(sIndic.compass));
		}
		data.put("energyM", String.valueOf(energyM));
		data.put("fatalWarn", String.valueOf(fatalWarn));
		data.put("radioAlt_f", String.valueOf(radioAlt));
		data.put("radioAltValid", String.valueOf(radioAltValid));
		data.put("isDowningFlap", String.valueOf(isDowningFlap));
		data.put("timeStr", sfueltime);

		// Map Grid Calculation
		if (loc != null && mapinfo != null) {
			char map_x = (char) ('A' + (loc[1] * mapinfo.mapStage) + mapinfo.inGameOffset);
			int map_y = (int) (loc[0] * mapinfo.mapStage + mapinfo.inGameOffset + 1);
			data.put("mapGrid", String.format("%c%d", map_x, map_y));
		} else {
			data.put("mapGrid", "--");
		}

		// 2. Publish Event (Data Plane)
		// 2. Publish Event (Data Plane)
		FlightDataBus.getInstance().publish(new FlightDataEvent(data, sState, sIndic));

		// 3. Legacy Support (Sync to GlobalPool)
		if (c != null && c.globalPool != null) {
			c.globalPool.beginBatch();
			for (Map.Entry<String, String> entry : data.entrySet()) {
				c.globalPool.put(entry.getKey(), entry.getValue());
			}
			// Push Raw Objects for advanced access (not in Map)
			c.globalPool.put("State", sState);
			c.globalPool.put("Indicators", sIndic);
			c.globalPool.commitBatch();
		}
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
						// Application.debugPrint("涡桨\n");
					} else
						iEngType = ENGINE_TYPE_JET;
				}

				// Application.debugPrint(String.format("自适应判断引擎类型 %d\n", iEngType));
			}
		}
	}

	public void slowcalculate(long dtime) {
		// 计算耗油率及持续时间
		// Application.debugPrint(totalfuelp - totalfuel);
		// if (MainCheckMili - FuelCheckMili > 1000) {

		dfuel = (fTotalFuelP - fTotalFuel) / dtime;

		if (dfuel > 0) {

			FuelchangeTime = MainCheckMili - FuelLastchangeMili;
			FuelLastchangeMili = MainCheckMili;
			fuelChange = fTotalFuelP - fTotalFuel; // 改变1公斤花了多长时间

			if (!bLowAccFuel) {
				// 改用滑动平均
				fueltime = (long) fuelTimeSMA.addNewData(fTotalFuel / dfuel);

			} else {
				// /* 已知油量不可能递增，考虑计算精度问题导致油量增多，因此取两者间最小值 */
				long tmpft = (long) fuelTimeSMA.addNewData(fTotalFuel * FuelchangeTime / fuelChange);
				if (fueltime > 0)
					fueltime = fueltime < tmpft ? fueltime : tmpft;
				else
					fueltime = tmpft;
				// Application.debugPrint("" + fueltime +" " + tmpft);
			}
			// Application.debugPrint(fuelChange);

		} else {
			// 没有变化，使用上次
			if (fuelChange == 0)
				fueltime = 0;
			else {
				/* 已知油量不可能递增，考虑计算精度问题导致油量增多，因此取两者间最小值 */
				long tmpft = (long) fuelTimeSMA.addNewData(fTotalFuel * FuelchangeTime / fuelChange);
				fueltime = tmpft;
			}

		}

		if (fueltime < 0)
			fueltime = Long.MAX_VALUE;

		FuelCheckMili = MainCheckMili;
		fTotalFuelP = fTotalFuel;
		// prev_throttle = sState.throttle;
		// 计算变化率
		// Application.debugPrint("" + fueltime);
	}

	// public engineLoad[] sPL;
	public void checkOverheat() {
		engineLoad[] pL = c.getBlkx().engLoad;
		// curLoad = c.getBlkx().findmaxLoad(pL, nwaterTemp, noilTemp);
		// 减去时间
		double minWorkTime = 99999 * 1000;
		/* 关发动机后，温度降到最低load后恢复 */
		Boolean engOff = false;
		if (sState.power[0] == 0 && sState.throttle > 0) {
			/* 关发动机 */
			engOff = true;
			// Application.debugPrint("监测到引擎关闭");
		}
		// 水冷
		curWLoad = c.getBlkx().findmaxWaterLoad(pL, nwaterTemp);
		for (int i = 0; i < c.getBlkx().maxEngLoad; i++) {
			if (i < curWLoad) {
				if (pL[i].WorkTime != 0) {
					pL[i].curWaterWorkTimeMili -= TimeIncrMili;
					if (pL[i].curWaterWorkTimeMili < minWorkTime) {
						minWorkTime = pL[i].curWaterWorkTimeMili;
					}
				}

			} else {

				if (engOff) {
					// 关闭引擎直接回满
					if (curWLoad == 0 || pL[curWLoad - 1].WorkTime < 0.1) {
						// Application.debugPrint("回复水温耐久条");
						pL[i].curWaterWorkTimeMili = pL[i].WorkTime * 1000;
					}
				} else {
					// 大于load且工作时长不满则进行恢复
					if (sState.throttle <= 100) {
						if (pL[i].RecoverTime != 0 && (1000 * pL[i].WorkTime > pL[i].curWaterWorkTimeMili)) {
							pL[i].curWaterWorkTimeMili += (double) TimeIncrMili * pL[i].WorkTime / pL[i].RecoverTime;
						}
					}
				}

			}
		}

		// Application.debugPrint("当前水工作负载: " + curLoad + "," + minWorkTime);
		// Application.debugPrint("水工作负载数组: [");
		// for (int i = 0; i < c.getBlkx().maxEngLoad; i++) {
		// System.out.print(pL[i].curWaterWorkTimeMili / 1000 + " ");
		// }
		// Application.debugPrint("]");

		// 油冷
		curOLoad = c.getBlkx().findmaxOilLoad(pL, noilTemp);
		for (int i = 0; i < c.getBlkx().maxEngLoad; i++) {
			if (i < curOLoad) {
				if (pL[i].WorkTime != 0) {
					pL[i].curOilWorkTimeMili -= TimeIncrMili;
					if (pL[i].curOilWorkTimeMili < minWorkTime) {
						minWorkTime = pL[i].curOilWorkTimeMili;
					}
				}

			} else {
				if (engOff) {
					// 关闭引擎直接回满
					if (curOLoad == 0 || pL[curOLoad - 1].WorkTime < 0.1) {
						// Application.debugPrint("回复油温耐久条");
						pL[i].curOilWorkTimeMili = pL[i].WorkTime * 1000;
					}
				} else {
					// 大于load且工作时长不满则进行恢复
					if (sState.throttle <= 100) {
						if (pL[i].RecoverTime != 0 && (1000 * pL[i].WorkTime > pL[i].curOilWorkTimeMili)) {
							pL[i].curOilWorkTimeMili += (double) TimeIncrMili * pL[i].WorkTime / pL[i].RecoverTime;
						}
					}
				}
			}
		}

		//// Application.debugPrint("当前油工作负载: " + curLoad + "," + minWorkTime);
		// Application.debugPrint("油工作负载数组: [");
		// for (int i = 0; i < c.getBlkx().maxEngLoad; i++) {
		// System.out.print(pL[i].curOilWorkTimeMili / 1000 + " ");
		// }
		// Application.debugPrint("]");

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
	public double flapAllowAngle;
	private int flapp;
	private int flap;
	public boolean isDowningFlap;
	private long flapCheck;
	public double maximumThrRPM;
	// double maximumAllowedRPM;
	private long checkMaxiumRPM;
	public boolean getMaximumRPM;
	public HttpHelper httpClient;
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

		// Application.debugPrint(bangleR +"," + bxoffset);

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
				// Application.debugPrint(TimeIncrMili);
				wepTime += TimeIncrMili;
				nitroEngNr += 1;
			}
		}
		nitrokg = c.getBlkx().nitro - (wepTime * nitroConsump) / 1000;
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
		// // Application.debugPrint(Math.abs(alt - altp)*1000+"?"+Math.abs(2 *
		// // sState.Vy * intv));
		//
		// // 解决熊猫的高度问题
		// alt = alt + altperCircle * altreg;
		// Application.debugPrint("checkalt"+checkAlt);

		// 无线电高度
		pRadioAlt = radioAlt;
		// radioAlt = iIndic.radio_altitude;

		if (sIndic.radio_altitude == StringHelper.fInvalid) {
			radioAlt = alt;
			radioAltValid = false;
		} else {
			radioAltValid = true;
			if (iCheckAlt > 0) {
				radioAlt = sIndic.radio_altitude * 0.3048f;
			} else {
				radioAlt = sIndic.radio_altitude;
			}
		}
		dRadioAlt = (ratio_1 * dRadioAlt) + ratio * 1000.0f * (radioAlt - pRadioAlt) / intv;
		// Application.debugPrint(dRadioAlt);

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
		// if (sIndic.aviahorizon_roll != -65535) {
		// alpha = sIndic.aviahorizon_roll;
		// }
		// if (sIndic.aviahorizon_pitch != -65535) {
		// beta = sIndic.aviahorizon_pitch + sState.AoA;
		// }

		if (sIndic.aviahorizon_roll != -65535 && sIndic.aviahorizon_pitch != -65535) {
			// 获得横滚角

			An = (double) (g * Math
					.sqrt(sState.Ny * sState.Ny + 1 - 2 * sState.Ny * Math.cos(Math.toRadians(sIndic.aviahorizon_roll))
							* Math.cos(Math.toRadians(sIndic.aviahorizon_pitch + sState.AoA))));
			// An = (double)(g * Math.sqrt(a))
		} else
			An = (g * sState.Ny);
		// Application.debugPrint(Math.cos(sIndic.aviahorizon_roll));
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
		} else {

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
		} else {
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
		// Application.debugPrint("校正TAS:"+ speedv*3.6);
		// 订正后加速度还是会有跳变
		// Application.debugPrint("校正TAS:"+ speedv*3.6 + "," + iastotascoff);

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
				// Application.debugPrint(sState.engineNum);
				ttotalhp = ttotalhp + sState.power[i];
				ttotalhpeff = ttotalhpeff + sState.thrust[i] * g * speedv / 735;
			}
			// System.out.println(ttotalhp);
			// Application.debugPrint(totalhpeff);
			// Application.debugPrint(String.format("sevice 引擎数量%d, 功率%.0f", engineNum,
			// ttotalhp));
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
				// Application.debugPrint(sState.thrust[0]);
				ttotalthr = ttotalthr + sState.thrust[i];
			}
			// Application.debugPrint(totalthr+" "+totalhpeff);
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
			for (i = 0; i < 1; i++) {
				ttotalfuel = ttotalfuel + sIndic.fuel[i];
			}
			fTotalFuel = ttotalfuel;

		}
		// Application.debugPrint("I"+totalfuel);
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
		// Application.debugPrint(diffspeed);
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
		// pEnergyJKg = energyJKg;
		// energyJKg = ((speedv + speedvp) * (speedv + speedvp) / (8 * g) +
		// sState.heightm);
		energyJKg = ((speedv + speedvp) * (speedv + speedvp) / 8 + g * sState.heightm);
		energyM = ((speedv + speedvp) * (speedv + speedvp) / (8 * g) + sState.heightm);
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
		boolean downflap = false;
		flapp = flap;
		flap = sState.flaps;
		if (flap - flapp > 0) {
			downflap = true;
		} else if (flap - flapp == 0) {
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
		isDowningFlap = downflap;
		flapAllowSpeed = getFlapAllowSpeed(sState.flaps, downflap);
		flapAllowAngle = getFlapAllowAngle(sState.IAS, downflap);
	}

	public void getMaximumRPM() {
		if (!getMaximumRPM) {
			if (c.getBlkx() != null && c.getBlkx().valid) {
				// FM合法直接取FM
				maximumThrRPM = c.getBlkx().maxRPM;
				// 使用最大允许RPM
				// maximumThrRPM = c.getBlkx().maxAllowedRPM;
				// Application.debugPrint(maximumThrRPM);
				getMaximumRPM = true;
			} else {
				// 自适应获得(无FM)

				// 获得最大转速，条件是以最大转速持续约20秒或者桨距
				if (checkMaxiumRPM < 20000 / freq) {
					if (sState.IAS > 50) {
						if (sState.RPM >= maximumThrRPM) {
							// Application.debugPrint(sState.RPM
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

	public void updateCompass() {
		if (sIndic.compass != -65535) {
			// 如果有仪表罗盘，读取仪表罗表盘数据
			dCompass = sIndic.compass;
		} else {
			// 否则读取地图中的方向数据
			if (dir[1] < 0) {
				dCompass = (360 - Math.toDegrees(Math.atan(dir[0] / dir[1]))) % 360;
			} else {
				dCompass = (180 - Math.toDegrees(Math.atan(dir[0] / dir[1])));
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

		// 更新方向
		updateCompass();

		// 更新爬升率
		updateClimbRate();

		// 获得准确高度,需要依赖Vy因此要放到爬升率后面
		updateAlt();

		// 更新速度
		updateSpeed();

		// 更新转弯半径
		updateTurn();

		// Application.debugPrint(horizontalLoad);
		// 计算总推力、总功率和总实功率
		updateEngineState();
		// 计算总油量
		updateFuel();

		// 计算SEP
		updateSEP();

		// 可变翼判断
		checkWing();

		// 襟翼判断
		checkFlap();
		// Application.debugPrint(flapAllowSpeed);

		// 获得最大转速
		getMaximumRPM();

		// TODO:升力阻力实时计算
		// TODO:临界速度和马赫数动态计算(考虑可变翼)

		// TODO:可用过载动态计算(油、重量)

	}

	double calcK(double x0, double y0, double x1, double y1) {
		double k = 0;
		if (x1 - x0 != 0) {
			k = (y1 - y0) / (x1 - x0);
		}
		return k;
	}

	public double getFlapAllowSpeed(int flapPercent, Boolean isDowningFlap) {
		// fm文件无法解析
		if (flapPercent == 0 || c.getBlkx() == null || !c.getBlkx().valid)
			return Double.MAX_VALUE;

		int FlapsDestructionNum = c.getBlkx().FlapsDestructionNum;
		// 找到襟翼档位
		int i = 0;
		for (; i < FlapsDestructionNum - 1; i++) {
			// 大于
			if (flapPercent < c.getBlkx().FlapsDestructionIndSpeed[i][0] * 100.0f) {
				break;
			}
		}

		i -= 1;
		// 找到档位了
		// 线性求值
		// 找前面的flap值
		double x0, x1, y0, y1;
		double k;
		// 没有找到，都小于

		if (i == -1) {
			// 下襟翼时直接越级使用下一级
			if (isDowningFlap && FlapsDestructionNum >= 1) {

				// x0 = c.getBlkx().FlapsDestructionIndSpeed[0][0] * 100.0f;
				// y0 = c.getBlkx().FlapsDestructionIndSpeed[0][1];
				// x1 = c.getBlkx().FlapsDestructionIndSpeed[1][0] * 100.0f;
				// y1 = c.getBlkx().FlapsDestructionIndSpeed[1][1];
				// k = this.calcK(x0, y0, x1, y1);

				// // Application.debugPrint(x0 + "-" + x1 + ", " + y0 + "-" + y1 + " k " + k +
				// " : F" +
				// flapPercent + "D "
				// // + (flapPercent - x0) * k + " L" + (y0 + (flapPercent - x0) * k));
				// Application.debugPrint("limit " + ( y0 + (flapPercent - x0) * k));
				// return y0 + (flapPercent - x0) * k;
				return c.getBlkx().FlapsDestructionIndSpeed[0][1];
			}
			// 襟翼只有0级
			// if(c.getBlkx().FlapsDestructionNum == 0){
			// return c.getBlkx().FlapsDestructionIndSpeed[0][1];
			// }
			return Double.MAX_VALUE;
		} else {
			// 下襟翼时直接越级使用
			// if (isDowningFlap) {
			// return c.getBlkx().FlapsDestructionIndSpeed[i][1];
			// }

			// 相等
			if (flapPercent == c.getBlkx().FlapsDestructionIndSpeed[i][0] * 100.0f) {
				// 直接返回速度
				return c.getBlkx().FlapsDestructionIndSpeed[i][1];
			}

			// 否则进行线性插值运算
			// 算斜率
			x0 = c.getBlkx().FlapsDestructionIndSpeed[i][0] * 100.0f;
			y0 = c.getBlkx().FlapsDestructionIndSpeed[i][1];
			x1 = c.getBlkx().FlapsDestructionIndSpeed[i + 1][0] * 100.0f;
			y1 = c.getBlkx().FlapsDestructionIndSpeed[i + 1][1];
			k = this.calcK(x0, y0, x1, y1);

			// 速度等于
			// Application.debugPrint(x0 + "-" + x1 + ", " + y0 + "-" + y1);
			return y0 + (flapPercent - x0) * k;
		}

	}

	double normFlapAngle(double t) {
		if (t < 0)
			return 0;
		if (t < 125)
			return t;
		else
			return 125;
	}

	public double getFlapAllowAngle(double ias, Boolean isDowningFlap) {
		// fm文件无法解析
		if (ias == 0 || c.getBlkx() == null || !c.getBlkx().valid)
			return 125;

		// 找到襟翼档位
		int i = 0;
		for (; i < c.getBlkx().FlapsDestructionNum - 1; i++) {
			// 大于
			if (ias > c.getBlkx().FlapsDestructionIndSpeed[i][1]) {
				break;
			}
		}

		// 找到档位了
		// 线性求值
		// 找前面的flap值
		double x0, x1, y0, y1, t;
		double k;
		// 没有找到，都小于

		if (i == 0) {
			// 下襟翼时直接越级使用下一级

			x0 = c.getBlkx().FlapsDestructionIndSpeed[i][1];
			y0 = c.getBlkx().FlapsDestructionIndSpeed[i][0] * 100.0f;
			x1 = c.getBlkx().FlapsDestructionIndSpeed[i + 1][1];
			y1 = c.getBlkx().FlapsDestructionIndSpeed[i + 1][0] * 100.0f;
			k = this.calcK(x0, y0, x1, y1);

			t = y0 + (ias - x0) * k;
			return normFlapAngle(t);

			// 襟翼只有0级
			// if(c.getBlkx().FlapsDestructionNum == 0){
			// return c.getBlkx().FlapsDestructionIndSpeed[0][1];
			// }

		} else {
			// 下襟翼时直接越级使用
			// if (isDowningFlap) {
			// return c.getBlkx().FlapsDestructionIndSpeed[i][1];
			// }

			// 相等
			if (ias == c.getBlkx().FlapsDestructionIndSpeed[i - 1][1]) {
				// 直接返回速度
				return c.getBlkx().FlapsDestructionIndSpeed[i - 1][0] * 100.0f;
			}

			// 否则进行线性插值运算
			// 算斜率
			x0 = c.getBlkx().FlapsDestructionIndSpeed[i - 1][1];
			y0 = c.getBlkx().FlapsDestructionIndSpeed[i - 1][0] * 100.0f;
			x1 = c.getBlkx().FlapsDestructionIndSpeed[i][1];
			y1 = c.getBlkx().FlapsDestructionIndSpeed[i][0] * 100.0f;
			k = this.calcK(x0, y0, x1, y1);

			// 速度等于
			// Application.debugPrint(x0 + "-" + x1 + ", " + y0 + "-" + y1);
			t = y0 + (ias - x0) * k;
			return normFlapAngle(t);
		}
	}

	public void resetEngLoad() {
		if (c.getBlkx() != null && c.getBlkx().valid) {
			for (int idx = 0; idx < c.getBlkx().maxEngLoad; idx++) {
				c.getBlkx().engLoad[idx].curWaterWorkTimeMili = c.getBlkx().engLoad[idx].WorkTime * 1000;
				c.getBlkx().engLoad[idx].curOilWorkTimeMili = c.getBlkx().engLoad[idx].WorkTime * 1000;
			}
		}
	}

	// 重置变量
	public void resetvaria() {
		loc = new double[2];
		dir = new double[2];
		radioAltValid = false;
		playerLive = false;
		iEngType = ENGINE_TYPE_UNKNOWN;
		checkMaxiumRPM = 0;
		dCompass = 0;
		flapCheck = 0;
		isDowningFlap = false;
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
		// if(c.getBlkx() != null && c.getBlkx().maxEngLoad !=
		// 0)c.getBlkx().resetEngineLoad();
		FuelCheckMili = System.currentTimeMillis();
		MapCheckMili = FuelCheckMili;
		MainCheckMili = FuelCheckMili;
		notCheckInch = false;
		altperCirclflag = false;
		// isFuelpressure = false;
		notCheckInch = false;
		hasWingSweepVario = false;
		flapAllowSpeed = Float.MAX_VALUE;
		flapAllowAngle = Float.MAX_VALUE;
		fTotalFuelP = 0;
		isStateJet = false;
		nitrokg = 0;
		nitroConsump = 0;
		nitroEngNr = 0;

		calcSpeedSMA = cH.new SimpleMovingAverage((int) (1000 / freq));
		diffSpeedSMA = cH.new SimpleMovingAverage((int) (1000 / freq));
		sepSMA = cH.new SimpleMovingAverage((int) (1000 / freq));
		turnrdsSMA = cH.new SimpleMovingAverage((int) (1000 / freq));
		sumSpeedSMA = cH.new SimpleMovingAverage((int) (1000 / freq));
		energyDiffSMA = cH.new SimpleMovingAverage((int) (1000 / freq));
		fuelTimeSMA = cH.new SimpleMovingAverage(4);
		if (c.getBlkx() != null) {
			engineLoad[] pL = c.getBlkx().engLoad;
			nitrokg = c.getBlkx().nitro;
			nitroConsump = c.getBlkx().nitroDecr;
			if (pL != null) {
				for (int i = 0; i < c.getBlkx().maxEngLoad; i++) {
					pL[i].curWaterWorkTimeMili = pL[i].curWaterWorkTimeMili;
					pL[i].curOilWorkTimeMili = pL[i].curOilWorkTimeMili;
				}
			}
		}

		// Initialize Strings to Defaults
		sTotalHp = nastring;
		sTotalThr = nastring;
		rpm = nastring;
		sTotalHpEff = nastring;
		pressureInchHg = nastring;
		manifoldpressure = nastring;
		watertemp = nastring;
		oiltemp = nastring;
		sTotalFuel = nastring;
		sfueltime = nastring;
		sNitro = nastring;
		sWepTime = nastring;
		sEngWorkTime = nastring;
		SdThrustPercent = nastring;
		sThurstPercent = nastring;
		sAvgEff = nastring;

		TAS = nastring;
		IAS = nastring;
		M = nastring;
		AoA = nastring;
		AoS = nastring;
		Ny = nastring;
		sN = nastring;
		Wx = nastring;
		salt = nastring;
		sRadioAlt = nastring;
		Vy = nastring;
		compass = nastring;
		throttle = nastring;
		sSEP = nastring;
		sSEPAbs = nastring;
		sAcc = nastring;
		sTurnRate = nastring;
		sTurnRds = nastring;
		sWingSweep = nastring;
		flaps = nastring;
		gear = nastring;
		aileron = nastring;
		elevator = nastring;
		rudder = nastring;
		svalid = "false";

		efficiency = new String[4];
		for (int i = 0; i < 4; i++)
			efficiency[i] = nastring;
		pitch = new String[4];
		for (int i = 0; i < 4; i++)
			pitch[i] = nastring;

		// Publish initial state immediately
		updateGlobalPool();
	}

	public void clearvaria() {
		// sState = null;
		// iIndic = null;
		resetvaria();
	}

	public void clear() {
		// Application.debugPrint("执行清洁");
		clearvaria();

		System.gc();
		sState.init();
		sIndic.init();

	}

	public Service(Controller xc) {

		c = xc;

		freq = xc.freqService;
		clearvaria();
		mapinfo = new MapInfo();
		ratio = freq / 1000.0f;
		ratio_1 = 1.0f - ratio;
		sState = new State();
		sState.init();
		sIndic = new Indicators();
		sIndic.init();
		httpClient = new HttpHelper();
		power = new String[State.maxEngNum];
		pitch = new String[State.maxEngNum];
		thrust = new String[State.maxEngNum];
		efficiency = new String[State.maxEngNum];
		FuelCheckMili = System.currentTimeMillis();
		// isFuelpressure = false;

	}

	public void checkState() {
		int conState;
		// 更新时间戳
		timeStamp = SystemTime;
		// Application.debugPrint("s:"+httpClient.strState+"s1:"+httpClient.strIndic);
		// 更新state

		c.initStatusBar();
		if (httpClient.strState.length() > 0 && httpClient.strIndic.length() > 0) {
			// 改变状态为连接成功
			// Application.debugPrint(sState);
			conState = sState.update(httpClient.strState);

			sIndic.update(httpClient.strIndic);
			c.changeS2();
			if (sState.flag && sIndic.flag) {

				/* 修复录像中没法使用的问题 */
				if ((!sIndic.type.equals("DUMMY_PLANE")) && ((sState.totalThr != 0) || (sState.RPM != 0))) {
					if (playerLive == false) {
						if (!portOcupied)
							httpClient.getReqMapInfoResult(Application.requestDest);
						else
							httpClient.getReqMapInfoResult(Application.requestDestBkp);
						mapinfo.update(httpClient.strMapInfo);
					}
					playerLive = true;
					// Application.debugPrint("grid_zero: " + mapinfo.grid_zeroX + ", " +
					// mapinfo.grid_zeroY);
				}

				if (isPlayerLive()) {
					// 读取map info

					c.changeS3();// 打开面板
					if (c.cur_fmtype != null && !c.cur_fmtype.equals(sIndic.type)) {
						prog.util.Logger.info("Service",
								"Aircraft type changed to: " + sIndic.type + ". Restarting Controller.");
						c.S4toS1();
					}
					// speedvp = sState.IAS;
					// 开始计算数据
					calculate();

					// 检测到加油，重置数据
					if ((Math.abs(speedv) < 10) && (fTotalFuel - fTotalFuelP > 1)) {
						prog.util.Logger.info("Service",
								String.format(
										"Refueling detected (Fuel: %.1f -> %.1f). Resetting simulation variables.",
										fTotalFuelP, fTotalFuel));

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
						prog.util.Logger.warn("Service", "Player crash/stop detected. Simulation state invalidated.");
						playerLive = false;
					}
				}
			} else {
				// 状态置为等待游戏开始（状态1）
				// c.changeS2();//连接成功等待游戏开始

				c.S4toS1();
				// Application.debugPrint("等待游戏开始");
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
			prog.util.Logger.debug("Service", "Waiting for game connection (8111/9222)...");
		}
		if (conState == -1) {
			// 端口连接可能有问题，切换端口
			// Application.debugPrint("切换端口\n");
			portOcupied = !portOcupied;
		}
	}

	@Override
	public void run() {
		// Application.debugPrint("" + waitMili);
		while (true) {

			SystemTime = System.currentTimeMillis();
			long diffTime = SystemTime - MainCheckMili;
			if (diffTime >= freq) {

				// 尝试GET数据
				if (!portOcupied)
					httpClient.getReqResult(Application.requestDest);
				else
					httpClient.getReqResult(Application.requestDestBkp);

				intv = (diffTime / freq) * freq;
				TimeIncrMili = diffTime;
				MainCheckMili += intv;
				// MainCheckMili = SystemTime;

				// 检查是否需要改变状态
				checkState();

				// 记录
				if (c.logon) {
					FlightLog tempLog = c.Log;
					if (tempLog != null)
						tempLog.logTick();
				}
				// Application.debugPrint("?\n");
				// 检查超时
				// if (MainCheckMili <= (System.currentTimeMillis() - intv)) {
				// Application.debugPrint("deadline Miss, try catch\n" + SystemTime +
				// "," + MainCheckMili);
				// }
			}
			long diffTime1 = SystemTime - MapCheckMili;
			if (diffTime1 >= 10 * freq) {
				MapCheckMili = SystemTime;
				if (!portOcupied)
					httpClient.getReqMapObjResult(Application.requestDest);
				else
					httpClient.getReqMapObjResult(Application.requestDestBkp);
				MapObj.getPlayerLoc(httpClient.strMapObj, loc);
				MapObj.getPlayerDir(httpClient.strMapObj, dir);
				// MapObj.getAirfieldLoc(httpClient.strMapObj, null);
			}

			try {
				long sleeptime = SystemTime + freq - System.currentTimeMillis();
				if (sleeptime > 0)
					Thread.sleep(sleeptime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
				return;
			}
		}
	}
	// --- TelemetrySource Implementation ---

	@Override
	public double getIAS() {
		return sState != null ? sState.IAS : 0;
	}

	@Override
	public double getTAS() {
		return sState != null ? sState.TAS : 0;
	}

	@Override
	public double getMach() {
		return sState != null ? sState.M : 0;
	}

	@Override
	public double getAoA() {
		return sState != null ? sState.AoA : 0;
	}

	@Override
	public double getAoS() {
		return sState != null ? sState.AoS : 0;
	}

	@Override
	public double getNy() {
		return sState != null ? sState.Ny : 0;
	}

	@Override
	public double getVario() {
		return nVy;
	}

	@Override
	public double getHorsePower() {
		return iTotalHp;
	}

	@Override
	public double getEngineResponse() {
		return tEngResponse;
	}

	@Override
	public double getPropEfficiency() {
		return avgeff;
	}

	@Override
	public double getManifoldPressurePounds() {
		return sState != null ? (sState.manifoldpressure - 1) * 14.696 : 0;
	}

	@Override
	public double getManifoldPressureInchHg() {
		return sState != null ? (sState.manifoldpressure * 760 / 25.4) : 0;
	}

	@Override
	public double getUnknownMixture() {
		return sState != null ? sState.mixture : 0;
	}

	@Override
	public double getRadiator() {
		return sState != null ? sState.radiator : 0;
	}

	@Override
	public double getCompressorStage() {
		return sState != null ? sState.compressorstage : 0;
	}

	@Override
	public double getFuelPercent() {
		return fuelPercent;
	}

	@Override
	public double getRPMThrottle() {
		return sState != null ? sState.RPMthrottle : 0;
	}

	@Override
	public double getThrustPercent() {
		return thurstPercent;
	}

	@Override
	public double getAltitude() {
		return alt;
	}

	@Override
	public double getRadioAltitude() {
		return radioAlt;
	}

	@Override
	public boolean isRadioAltitudeValid() {
		return radioAltValid != null && radioAltValid;
	}

	@Override
	public double getCompass() {
		return dCompass;
	}

	@Override
	public double getSEP() {
		return SEP;
	}

	@Override
	public double getAcceleration() {
		return acceleration;
	}

	@Override
	public double getTurnRate() {
		return turnRate;
	}

	@Override
	public double getTurnRadius() {
		return turnRds;
	}

	@Override
	public double getRollRate() {
		return sState != null ? sState.Wx : 0;
	}

	@Override
	public double getMassFuel() {
		return fTotalFuel;
	}

	@Override
	public long getFuelTimeMili() {
		return fueltime;
	}

	@Override
	public double getThrottle() {
		return sState != null ? sState.throttle : 0;
	}

	@Override
	public double getRPM() {
		return sState != null ? sState.RPM : 0;
	}

	@Override
	public double getManifoldPressure() {
		return sState != null ? sState.manifoldpressure : 0;
	}

	@Override
	public double getWaterTemp() {
		return nwaterTemp;
	}

	@Override
	public double getOilTemp() {
		return noilTemp;
	}

	@Override
	public double getPitch() {
		return sState != null ? sState.pitch[0] : 0;
	}

	@Override
	public double getThrust() {
		return sState != null ? sState.thrust[0] : 0;
	}

	@Override
	public double getGear() {
		return sState != null ? sState.gear : 0;
	}

	@Override
	public double getFlaps() {
		return sState != null ? sState.flaps : 0;
	}

	@Override
	public double getAirbrake() {
		return sState != null ? sState.airbrake : 0;
	}

	@Override
	public double getAileron() {
		return sState != null ? sState.aileron : 0;
	}

	@Override
	public double getElevator() {
		return sState != null ? sState.elevator : 0;
	}

	@Override
	public double getRudder() {
		return sState != null ? sState.rudder : 0;
	}

	@Override
	public double getWingSweep() {
		return sIndic != null ? sIndic.wsweep_indicator : 0;
	}

}