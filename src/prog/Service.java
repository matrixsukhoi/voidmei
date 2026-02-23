package prog;

import prog.util.HttpHelper;
import prog.util.ExceptionHelper;

import parser.Blkx.engineLoad;
import parser.FlightLog;
import parser.Indicators;
import parser.MapInfo;
import parser.MapObj;
import parser.State;
import prog.util.StringHelper;
import prog.util.CalcHelper;
import prog.util.PistonPowerModel;
import static prog.util.PhysicsConstants.g;
import prog.event.EventPayload;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.config.HUDSettings;
import ui.overlay.logic.HUDCalculator;
import ui.overlay.model.HUDData;

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
	public double prevEnergyJKg;
	public long calcPeriod;
	public static String buf;
	// Gravitational constant imported from PhysicsConstants.g
	public long timeStamp;
	public long freq;
	// === API 对象（对应 War Thunder HTTP 端点）===
	public State sState;           // /state 端点数据
	public Indicators sIndic;      // /indicators 端点数据
	public String statusText;      // 状态文本
	public String timeText;        // 时间文本

	// === 数值类字段（移除匈牙利前缀）===
	public int totalHp;            // 总马力
	public String totalHpStr;      // 总马力字符串
	public int totalHpEff;         // 有效马力
	public String totalHpEffStr;   // 有效马力字符串
	public boolean useMegaHp;      // 是否使用MHp单位
	public int totalThrust;        // 总推力
	public String totalThrustStr;  // 总推力字符串
	public double totalFuel;       // 总油量
	public double totalFuelPrev;   // 上次油量（用于计算变化）
	public boolean lowAccFuel;     // 低精度燃油警告
	public String totalFuelStr;    // 总油量字符串
	public int checkAlt;           // 检查高度
	public double fuelDelta;       // 油量变化
	public long fueltime;
	public String fueltimeStr;     // 油耗时间字符串
	public boolean notCheckInch;
	// public boolean isFuelpressure;
	public boolean altperCirclflag;
	public long actualIntervalMs;
	public double althour;
	public double altperCircle;
	public double alt;
	public double altp;
	public double altreg;
	public double iastotascoff;
	public long currentTimeMs;
	public long pollCycleDurationMs;
	long lastMainLoopTimeMs;
	long lastMapPollTimeMs;
	long FuelCheckMili;
	public double fuelChange;
	long FuelLastchangeMili;
	long FuelchangeTime;
	long GCCheckMili;
	long SlowCheckMili;
	long intervalCheckMs;
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

	/** 游戏窗口焦点监控器，用于实现失焦时隐藏overlay */
	private final FocusMonitor focusMonitor = new FocusMonitor();

	// 对飞机结构有重大影响的警告
	public Boolean fatalWarn = false;

	// sState转换后
	public boolean hasWingSweepVario;
	public boolean isStateJet;
	public double compassDelta;
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
	public long sWepTimeVal; // Remaining WEP time in seconds

	/** Optimal compressor stage index for current conditions. -1 = invalid/jet/single-stage */
	private int optimalCompressorStage = -1;
	/** True when actual compressor stage doesn't match optimal (at full throttle) */
	private boolean compressorStageMismatch = false;
	/** Previous actual compressor stage for change detection (0-based, -1 = invalid) */
	private int prevActualCompressorStage = -1;
	/** Previous optimal compressor stage for change detection */
	private int prevOptimalCompressorStage = -1;

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

	/**
	 * Formats raw flight data into display strings.
	 * Previously named trans2String() - renamed for clarity.
	 */
	public void formatDataAsStrings() {

		// 数据转换格式
		// sState

		throttle = String.format("%d", sState.throttle);
		aileron = String.format("%d", sState.aileron);
		elevator = String.format("%d", sState.elevator);
		rudder = String.format("%d", sState.rudder);

		timeText = String.format("%02d'%02d", elapsedTime / 60000, (elapsedTime / 1000) % 60);
		if (fueltime <= 0 || fueltime > 24 * 3600 * 1000)
			fueltimeStr = nastring;
		else {
			if (fueltime / 60000 < 100) {
				fueltimeStr = String.format("%02d'%02d", fueltime / 60000, (long) ((fueltime / 1000) % 60 / 10) * 10);
			} else
				fueltimeStr = String.format("%.0f", (float) fueltime / 60000);

		}
		totalThrustStr = String.format("%d", totalThrust);
		if (totalHp == 0)
			totalHpStr = nastring;
		else
			totalHpStr = String.format("%d", totalHp);

		rpm = String.format("%d", (int) sState.RPM);
		if (totalHpEff >= 100000) {
			useMegaHp = true;
			totalHpEffStr = String.format("%.2f", totalHpEff / 1000000.0f);
		} else {
			useMegaHp = false;
			totalHpEffStr = String.format("%d", totalHpEff);
		}
		if (sState.efficiency[0] == 0)
			efficiency[0] = nastring;
		else
			efficiency[0] = String.format("%.0f", sState.efficiency[0]);
		if (nwaterTemp != -65535)
			watertemp = String.format("%.0f", nwaterTemp);
		else
			watertemp = nastring;
		oiltemp = String.format("%.0f", noilTemp);
		if (sState.manifoldpressure != 1) {
			pressurePounds = String.format("%+.1f", (sState.manifoldpressure - 1) * 14.696);
			pressureInchHg = String.format("P/%.1f''", (sState.manifoldpressure * 760 / 25.4));

			if (this.checkAlt > 0) {
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
		totalFuelStr = String.format("%.0f", totalFuel);
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
		compass = String.format("%.0f", compassDelta);
		sPitchUp = String.format("%.0f", sIndic.aviahorizon_pitch);

		if (c.getBlkx() != null && c.getBlkx().valid && c.getBlkx().nitro != 0) {

			sNitro = String.format("%.0f", nitrokg);
			long twepTime = 0;
			if (nitroEngNr == 0) {
				// nitroEngNr = sState.engineNum;
				// sWepTime = nastring;
			} else {
				twepTime = (int) (((c.getBlkx().nitro / c.getBlkx().nitroDecr - wepTime / 1000)) / nitroEngNr);

				sWepTimeVal = twepTime;
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

		publishFlightDataEvent();
	}

	/**
	 * Publishes flight data to FlightDataBus.
	 * Pre-computes HUDData on Service thread to offload work from EDT.
	 *
	 * @deprecated Method name is legacy - renamed to publishFlightDataEvent() for clarity.
	 */
	private void publishFlightDataEvent() {
		// Build type-safe payload (replaces legacy Map<String, String>)
		String mapGrid;
		if (loc != null && mapinfo != null) {
			char map_x = (char) ('A' + (loc[1] * mapinfo.mapStage) + mapinfo.inGameOffset);
			int map_y = (int) (loc[0] * mapinfo.mapStage + mapinfo.inGameOffset + 1);
			mapGrid = String.format("%c%d", map_x, map_y);
		} else {
			mapGrid = "--";
		}

		EventPayload payload = EventPayload.builder()
			.mapGrid(mapGrid)
			.fatalWarn(fatalWarn)
			.radioAltValid(radioAltValid)
			.isDowningFlap(isDowningFlap)
			.timeStr(fueltimeStr)
			.isJet(iEngType == ENGINE_TYPE_JET)
			.engineCheckDone(checkEngineFlag)
			.optimalCompressorStage(optimalCompressorStage)
			.compressorStageMismatch(compressorStageMismatch)
			.build();

		FlightDataEvent event = new FlightDataEvent(payload, sState, sIndic);

		// Pre-compute HUDData on Service thread (reduces EDT latency by ~40-60ms)
		if (c != null && c.configService != null) {
			try {
				HUDSettings hudSettings = c.configService.getHUDSettings();
				if (hudSettings != null) {
					HUDData hudData = HUDCalculator.calculate(event, this, c.getBlkx(), hudSettings, null);
					event.setHudData(hudData);
				}
			} catch (Exception e) {
				// Calculation failure should not prevent event publishing
				prog.util.Logger.warn("Service", "HUDData pre-calculation failed: " + e.getMessage());
			}
		}

		FlightDataBus.getInstance().publish(event);
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

			}
		}
	}

	public void slowcalculate(long dtime) {
		fuelDelta = (totalFuelPrev - totalFuel) / dtime;

		if (fuelDelta > 0) {

			FuelchangeTime = lastMainLoopTimeMs - FuelLastchangeMili;
			FuelLastchangeMili = lastMainLoopTimeMs;
			fuelChange = totalFuelPrev - totalFuel; // 改变1公斤花了多长时间

			if (!lowAccFuel) {
				// 改用滑动平均
				fueltime = (long) fuelTimeSMA.addNewData(totalFuel / fuelDelta);

			} else {
				// /* 已知油量不可能递增，考虑计算精度问题导致油量增多，因此取两者间最小值 */
				long tmpft = (long) fuelTimeSMA.addNewData(totalFuel * FuelchangeTime / fuelChange);
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
				long tmpft = (long) fuelTimeSMA.addNewData(totalFuel * FuelchangeTime / fuelChange);
				fueltime = tmpft;
			}

		}

		if (fueltime < 0)
			fueltime = Long.MAX_VALUE;

		FuelCheckMili = lastMainLoopTimeMs;
		totalFuelPrev = totalFuel;
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
					pL[i].curWaterWorkTimeMili -= pollCycleDurationMs;
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
							pL[i].curWaterWorkTimeMili += (double) pollCycleDurationMs * pL[i].WorkTime / pL[i].RecoverTime;
						}
					}
				}

			}
		}

		// 油冷
		curOLoad = c.getBlkx().findmaxOilLoad(pL, noilTemp);
		for (int i = 0; i < c.getBlkx().maxEngLoad; i++) {
			if (i < curOLoad) {
				if (pL[i].WorkTime != 0) {
					pL[i].curOilWorkTimeMili -= pollCycleDurationMs;
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
							pL[i].curOilWorkTimeMili += (double) pollCycleDurationMs * pL[i].WorkTime / pL[i].RecoverTime;
						}
					}
				}
			}
		}

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
				// Application.debugPrint(pollCycleDurationMs);
				wepTime += pollCycleDurationMs;
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
			if ((Math.abs(altmeter - altmeterp) * 1000 > Math.abs(2 * sState.Vy * actualIntervalMs))) {
				checkAlt += actualIntervalMs;
			} else {
				checkAlt -= actualIntervalMs;
			}
			if (Math.abs(checkAlt) > 10000)
				notCheckInch = true;
		}

		// 无线电高度
		pRadioAlt = radioAlt;
		// radioAlt = iIndic.radio_altitude;

		if (sIndic.radio_altitude == StringHelper.fInvalid) {
			radioAlt = alt;
			radioAltValid = false;
		} else {
			radioAltValid = true;
			if (checkAlt > 0) {
				radioAlt = sIndic.radio_altitude * 0.3048f;
			} else {
				radioAlt = sIndic.radio_altitude;
			}
		}
		dRadioAlt = (ratio_1 * dRadioAlt) + ratio * 1000.0f * (radioAlt - pRadioAlt) / actualIntervalMs;
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
			totalHp = (int) (ttotalhp);
			totalHpEff = (int) (ttotalhpeff);
			totalThrust = (int) (ttotalthr);

			if (totalHp != 0)
				avgeff = (double) 100 * totalHpEff / totalHp;
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

			totalThrust = (int) ttotalthr;
			totalHpEff = (int) ttotalhpeff;

			avgeff = 0;
		}

		if (maxTotalThr < totalThrust && sState.throttle >= 100) {
			maxTotalThr = (int) (ratio_1 * maxTotalThr + ratio * totalThrust);
		}
		if (maxTotalHp < totalHpEff && sState.throttle >= 100) {
			maxTotalHp = (int) (ratio_1 * maxTotalHp + ratio * totalHpEff);
		}

		pThurstPercent = thurstPercent;

		// Get cached peak values (both are WEP/afterburner mode)
		double peakPower = c.getPeakWepPower();
		double peak = c.getPeakThrust();

		if (isEngJet()) {
			// Jet: current thrust / peak afterburner thrust
			if (peak > 0) {
				thurstPercent = 100.0 * totalThrust / peak;
			} else if (maxTotalThr != 0) {
				// Fallback to old algorithm
				thurstPercent = 100.0 * totalThrust / maxTotalThr;
			}
		} else {
			// Piston: current power / peak WEP power
			if (peakPower > 0) {
				thurstPercent = 100.0 * totalHp / peakPower;
			} else if (maxTotalHp != 0) {
				// Fallback to old algorithm
				thurstPercent = 100.0 * totalHp / maxTotalHp;
			}
		}

		tEngResponse = (ratio_1 * tEngResponse) + ratio * (thurstPercent - pThurstPercent) * 1000.0f / actualIntervalMs;

	}

	public void updateFuel() {
		int i;
		if (sIndic.fuelnum != 0) {
			double ttotalfuel = 0;
			lowAccFuel = Boolean.FALSE;
			/* 修复su-27油箱显示不正确的问题 */
			// for (i = 0; i < sIndic.fuelnum; i++) {
			for (i = 0; i < 1; i++) {
				ttotalfuel = ttotalfuel + sIndic.fuel[i];
			}
			totalFuel = ttotalfuel;

		}
		// Application.debugPrint("I"+totalfuel);
		if (totalFuel == 0) {
			lowAccFuel = Boolean.TRUE;
			totalFuel = sState.mfuel;
		}
		fuelPercent = (int) (100 * totalFuel / sState.mfuel0);

	}

	public void updateSEP() {
		// if (sState.IAS != 0) {
		double diffspeed1 = diffSpeedSMA.addNewData(speedv - speedvp);
		// 这是不是示空速的差?
		// 使用修正
		// diffspeed = (ratio_1 * diffspeed + ratio * (speedv - speedvp));
		diffspeed = diffspeed1;
		// Application.debugPrint(diffspeed);
		acceleration = diffspeed * 1000.0 / actualIntervalMs;

		// 三种计算方式

		// 这两种等价
		// SEP = acceleration * (speedvp + speedv) / (2 * g) + nVy;
		// SEP /= 2;

		// SEP = (diffspeed * (speedv + speedvp)) /((2 * actualIntervalMs * g)/ 1000.0f) +
		// nVy;
		// -38.8 4.7 = 28
		// SEP = SEP1;

		// 跳变太大, 没法读数

		// SEP = ((speedv * speedv) - (speedvp * speedvp))*1000.0f/(2 * actualIntervalMs *
		// g) + nVy;

		SEP = sepSMA.addNewData(((speedv + speedvp) * (speedv - speedvp) * 1000) / (2 * actualIntervalMs * g) + nVy);
		// SEP /= 2;

		// } else {
		// acceleration = 0;
		// SEP = 0;
		// }

		// 总能量
		// prevEnergyJKg = energyJKg;
		// energyJKg = ((speedv + speedvp) * (speedv + speedvp) / (8 * g) +
		// sState.heightm);
		energyJKg = ((speedv + speedvp) * (speedv + speedvp) / 8 + g * sState.heightm);
		energyM = ((speedv + speedvp) * (speedv + speedvp) / (8 * g) + sState.heightm);
		// System.out.println(String.format("%.0f",
		// energyDiffSMA.addNewData((energyJKg - prevEnergyJKg)*1000/actualIntervalMs)));
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
			flapCheck += actualIntervalMs;

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
			compassDelta = sIndic.compass;
		} else {
			// 否则读取地图中的方向数据
			if (dir[1] < 0) {
				compassDelta = (360 - Math.toDegrees(Math.atan(dir[0] / dir[1]))) % 360;
			} else {
				compassDelta = (180 - Math.toDegrees(Math.atan(dir[0] / dir[1])));
			}
		}
	}

	public void calculate() {

		// 获得开始时间
		elapsedTime = currentTimeMs - startTime;

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

		// 计算速度与临界速度比值
		updateSpeedRatio();
		updateStallSpeed();

		// 计算最佳增压器档位
		updateOptimalCompressorStage();

		// TODO:升力阻力实时计算
		// TODO:可用过载动态计算(油、重量)

	}

	public double mach; // 精准mach, 精度高于state.mach, 小于indicators.mach, 不过只有部分飞机有indicators.mach
	public double speedLimitRatio;
	public double aileronLockRatio;
	public double rudderLockRatio;
	public double unitMachLimitRatio; // 单位马赫数限制比值

	public void updateSpeedRatio() {
		if (c.getBlkx() == null || !c.getBlkx().valid) {
			speedLimitRatio = 0.0;
			aileronLockRatio = 0.0;
			rudderLockRatio = 0.0;
			return;
		}

		double wingSweep = 0;
		if (isWingSweepValid()) {
			wingSweep = sIndic.wsweep_indicator;
		}

		double ias = getIAS();
		// double mach = getMach(); 战雷的mach是小数点后两位的, 有很大的误差, 我们根据地球大气模型手动计算
		double iasLimit = c.getBlkx().getVNEVWing(wingSweep);
		double machLimit = c.getBlkx().getMNEVWing(wingSweep);
		double aileronLockSpeed = c.getBlkx().aileronEff;
		double rudderLockSpeed = c.getBlkx().rudderEff;
		
		// 1. 根据地球大气模型计算mach
		double iasPerMach = 3.6 * Math.sqrt(1.4 / 1.225 * 101325 * Math.pow((1 - 0.0000225577 * sState.heightm), 5.25588));
		mach = ias / iasPerMach;

		// 2. 计算速度比值
		double iasRatio = ias / iasLimit;
		double machRatio = mach / machLimit;
		// 3. 计算更大的速度
		if (iasPerMach == 0 || iasRatio >= machRatio) {
			speedLimitRatio = iasRatio;
			aileronLockRatio = aileronLockSpeed / iasLimit;
			rudderLockRatio = rudderLockSpeed / iasLimit;
			unitMachLimitRatio = iasPerMach / iasLimit;
		} else {
			speedLimitRatio = machRatio;
			aileronLockRatio = aileronLockSpeed / (machLimit * iasPerMach);
			rudderLockRatio = rudderLockSpeed / (machLimit * iasPerMach);
			unitMachLimitRatio = 1 / machLimit;
		}
	}

	public double stallSpeed;

	public void updateStallSpeed() {
		if (c.getBlkx() == null || !c.getBlkx().valid) {
			return;
		}

		// 主升力面积因数载荷
		double wingBodyLiftAreaLoad_NoFlap = c.getBlkx().AWing * c.getBlkx().NoFlapsWing.ClCritHigh
				+ c.getBlkx().AFuselage * c.getBlkx().fuseClHigh
						* (c.getBlkx().NoFlapsWing.AoACritHigh / c.getBlkx().Fuselage.AoACritHigh);
		double wingBodyLiftAreaLoad_FullFlap = c.getBlkx().AWing * c.getBlkx().FullFlapsWing.ClCritHigh
				+ c.getBlkx().AFuselage * c.getBlkx().fuseClHigh
						* (c.getBlkx().FullFlapsWing.AoACritHigh / c.getBlkx().Fuselage.AoACritHigh);
		double currentWeight = c.getBlkx().nofuelweight + sState.mfuel;

		// 假设战雷的襟翼是线性的
		// 单位换算: 3.6
		// 单位制混用: 1 / 1.225
		// stallSpeed = 3.6 * Math.sqrt(1 / 1.225 * g * (2 * currentWeight)
		// 		/ ((1 - flap / 100) * wingBodyLiftAreaLoad_NoFlap + (flap / 100) * wingBodyLiftAreaLoad_FullFlap));
		
		double flapFactor = flap / 100.0;
		double totalLiftArea = (1.0 - flapFactor) * wingBodyLiftAreaLoad_NoFlap + flapFactor * wingBodyLiftAreaLoad_FullFlap;
		stallSpeed = 3.6 * Math.sqrt((2.0 * currentWeight * g) / (1.225 * totalLiftArea));
	}

	/**
	 * Calculates the optimal supercharger stage for current altitude and throttle.
	 * Also detects mismatch between actual and optimal stage (at full throttle).
	 * Uses state-change detection to only update mismatch status when actual or optimal changes.
	 * Results are published via FlightDataBus for voice warning.
	 */
	public void updateOptimalCompressorStage() {
		PistonPowerModel.CompressorStageParams[] stages = c.getCompressorStages();

		// Invalid cases: jet, single-stage, or no FM loaded
		if (stages == null || stages.length <= 1) {
			optimalCompressorStage = -1;
			compressorStageMismatch = false;
			prevActualCompressorStage = -1;
			prevOptimalCompressorStage = -1;
			return;
		}

		// Detect WEP mode and full throttle state (any engine throttle >= 100)
		boolean isWep = false;
		boolean isFullThrottle = false;
		for (int i = 0; i < engineNum; i++) {
			if (sState.throttles[i] > 100) {
				isWep = true;
				isFullThrottle = true;
			} else if (sState.throttles[i] >= 100) {
				isFullThrottle = true;
			}
		}

		// Calculate optimal stage
		int newOptimal = PistonPowerModel.findOptimalStageIndex(
			stages, alt, isWep, getIAS(), true, 15.0);
		optimalCompressorStage = newOptimal;

		// Get current actual stage (convert from 1-based to 0-based)
		int actualStage = sState.compressorstage - 1;

		// API didn't return compressor stage (e.g., some aircraft don't report it)
		if (actualStage < 0) {
			compressorStageMismatch = false;
			prevActualCompressorStage = -1;
			prevOptimalCompressorStage = -1;
			return;
		}

		// If throttle < 100%, don't judge mismatch, force consistent
		if (!isFullThrottle) {
			compressorStageMismatch = false;
			prevActualCompressorStage = -1;
			prevOptimalCompressorStage = -1;
			return;
		}

		// State-change driven: only re-evaluate mismatch when actual or optimal changes
		boolean hasChange = (actualStage != prevActualCompressorStage)
						 || (newOptimal != prevOptimalCompressorStage);

		if (hasChange) {
			// Re-evaluate mismatch on state change
			compressorStageMismatch = (actualStage != newOptimal);
		}
		// If no change, preserve previous compressorStageMismatch value

		// Update tracking variables
		prevActualCompressorStage = actualStage;
		prevOptimalCompressorStage = newOptimal;
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
		compassDelta = 0;
		flapCheck = 0;
		isDowningFlap = false;
		getMaximumRPM = false;
		dRadioAlt = 0;
		curLoad = 0;
		wepTime = 0;
		energyJKg = 0;
		prevEnergyJKg = 0;
		elapsedTime = 0;
		altperCircle = 0;
		checkAlt = 0;
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
		lastMapPollTimeMs = FuelCheckMili;
		lastMainLoopTimeMs = FuelCheckMili;
		notCheckInch = false;
		altperCirclflag = false;
		// isFuelpressure = false;
		notCheckInch = false;
		hasWingSweepVario = false;
		flapAllowSpeed = Float.MAX_VALUE;
		flapAllowAngle = Float.MAX_VALUE;
		totalFuelPrev = 0;
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
		totalHpStr = nastring;
		totalThrustStr = nastring;
		rpm = nastring;
		totalHpEffStr = nastring;
		pressureInchHg = nastring;
		manifoldpressure = nastring;
		watertemp = nastring;
		oiltemp = nastring;
		totalFuelStr = nastring;
		fueltimeStr = nastring;
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
		publishFlightDataEvent();
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

		freq = xc.serviceLoopIntervalMs;
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

	/**
	 * Processes one polling cycle: updates state, calculates data, and publishes events.
	 * Previously named checkState() - renamed for clarity.
	 */
	public void processPollingCycle() {
		int conState;
		// 更新时间戳
		timeStamp = currentTimeMs;
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
					if ((Math.abs(speedv) < 10) && (totalFuel - totalFuelPrev > 1)) {
						prog.util.Logger.info("Service",
								String.format(
										"Refueling detected (Fuel: %.1f -> %.1f). Resetting simulation variables.",
										totalFuelPrev, totalFuel));

						resetvaria();
					}

					// 0.5秒一次慢计算
					if (((calcPeriod++) % (500 / freq)) == 0)
						slowcalculate((500 / freq) * freq);

					// 将数据转换格式
					formatDataAsStrings();

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
				// 等待游戏开始
				ExceptionHelper.sleepQuietly(500);
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
		// Main polling loop with exception recovery
		while (true) {
			try {
				currentTimeMs = System.currentTimeMillis();
				long diffTime = currentTimeMs - lastMainLoopTimeMs;
				if (diffTime >= freq) {

					// 尝试GET数据
					if (!portOcupied)
						httpClient.getReqResult(Application.requestDest);
					else
						httpClient.getReqResult(Application.requestDestBkp);

					actualIntervalMs = (diffTime / freq) * freq;
					pollCycleDurationMs = actualIntervalMs;
					lastMainLoopTimeMs += actualIntervalMs;

					// 检查是否需要改变状态
					processPollingCycle();

					// 焦点监控（内部有200ms节流）
					focusMonitor.tick();

					// 记录
					if (c.logon) {
						FlightLog tempLog = c.Log;
						if (tempLog != null)
							tempLog.logTick();
					}
				}
				long diffTime1 = currentTimeMs - lastMapPollTimeMs;
				if (diffTime1 >= 10 * freq) {
					lastMapPollTimeMs = currentTimeMs;
					if (!portOcupied)
						httpClient.getReqMapObjResult(Application.requestDest);
					else
						httpClient.getReqMapObjResult(Application.requestDestBkp);
					MapObj.getPlayerLoc(httpClient.strMapObj, loc);
					MapObj.getPlayerDir(httpClient.strMapObj, dir);
				}

				long sleeptime = currentTimeMs + freq - System.currentTimeMillis();
				if (sleeptime > 0) {
					Thread.sleep(sleeptime);
				}

			} catch (InterruptedException e) {
				// Thread was interrupted - exit the loop gracefully
				prog.util.Logger.info("Service", "Service thread interrupted, exiting...");
				break;  // Exit the while(true) loop
			} catch (Exception e) {
				// Unexpected error - log and recover after short delay
				prog.util.Logger.error("Service", "Service error: " + e.getClass().getSimpleName() + " at " +
					(e.getStackTrace().length > 0 ? e.getStackTrace()[0] : "unknown"));
				e.printStackTrace();
				try {
					Thread.sleep(1000);
				} catch (InterruptedException ignored) {
					// Ignore interrupt during recovery sleep
				}
			}
		}
	}

	/**
	 * 获取焦点监控器实例，供Controller在openpad/closepad时启用/禁用。
	 *
	 * @return FocusMonitor实例
	 */
	public FocusMonitor getFocusMonitor() {
		return focusMonitor;
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
		return mach;
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
		return An / g;
	}

	@Override
	public double getVario() {
		return nVy;
	}

	@Override
	public double getHorsePower() {
		return totalHp;
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
		if (iEngType == ENGINE_TYPE_JET)
			return 0;
		return sState != null ? (sState.manifoldpressure - 1) * 14.696 : 0;
	}

	@Override
	public double getManifoldPressureInchHg() {
		if (iEngType == ENGINE_TYPE_JET)
			return 0;
		return sState != null ? (sState.manifoldpressure * 760 / 25.4) : 0;
	}

	@Override
	public double getManifoldPressureDisplay() {
		if (iEngType == ENGINE_TYPE_JET)
			return 0;
		return isImperial() ? getManifoldPressurePounds() : getManifoldPressure();
	}

	@Override
	public String getManifoldPressureDisplayUnit() {
		if (iEngType == ENGINE_TYPE_JET)
			return "";
		if (isImperial()) {
			return String.format("P/%.1f''", getManifoldPressureInchHg());
		}
		return "Ata";
	}

	@Override
	public int getManifoldPressureDisplayPrecision() {
		return isImperial() ? 1 : 2;
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
		return compassDelta;
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
		return Math.abs(turnRds);
	}

	@Override
	public double getRollRate() {
		return sState != null ? Math.abs(sState.Wx) : 0;
	}

	@Override
	public double getMassFuel() {
		return totalFuel;
	}

	@Override
	public double getTotalWeight() {
		if (c == null || c.getBlkx() == null || !c.getBlkx().valid || sState == null) {
			return 0;
		}
		return c.getBlkx().nofuelweight + sState.mfuel;
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
		if (iEngType == ENGINE_TYPE_JET)
			return 0;
		return sState != null ? sState.manifoldpressure : 0;
	}

	@Override
	public boolean isManifoldPressureValid() {
		if (iEngType == ENGINE_TYPE_JET)
			return false;
		return sState != null && sState.manifoldpressure != -65535 && sState.manifoldpressure > 0.01;
	}

	@Override
	public double getWaterTemp() {
		if (iEngType == ENGINE_TYPE_JET)
			return 0;
		return nwaterTemp;
	}

	@Override
	public double getOilTemp() {
		if (iEngType == ENGINE_TYPE_JET)
			return 0;
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

	@Override
	public double getEnergyJKg() {
		return energyJKg;
	}

	@Override
	public double getEffHp() {
		return totalHpEff;
	}

	@Override
	public double getWepKg() {
		return nitrokg;
	}

	@Override
	public double getWepTime() {
		return sWepTimeVal;
	}

	@Override
	public double getHeatTolerance() {
		double val = curLoadMinWorkTime / 1000.0;
		if (val > 90000)
			return 0;
		return val;
	}

	@Override
	public double getPowerPercent() {
		return Math.min(thurstPercent, 100.0);
	}

	@Override
	public boolean isImperial() {
		return checkAlt > 0;
	}

	@Override
	public boolean isWingSweepValid() {
		return sIndic != null && sIndic.wsweep_indicator != -65535;
	}

	@Override
	public double getSpeedLimitRatio() {
		return speedLimitRatio;
	}

	@Override
	public double getAileronLockRatio() {
		return aileronLockRatio;
	}

	@Override
	public double getRudderLockRatio() {
		return rudderLockRatio;
	}

	@Override
	public double getUnitMachLimitRatio() {
		return unitMachLimitRatio;
	}

	@Override
	public double getStallSpeed() {
		return stallSpeed;
	}

	@Override
	public double getAviahorizonPitch() {
		return sIndic != null ? sIndic.aviahorizon_pitch : 0;
	}

	@Override
	public double getAviahorizonRoll() {
		return sIndic != null ? sIndic.aviahorizon_roll : 0;
	}
}
