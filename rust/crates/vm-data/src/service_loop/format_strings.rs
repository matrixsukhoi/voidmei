//! 对应 Java: `src/prog/Service.java` 的 `formatDataAsStrings()` (L242-432) ——
//! 全量显示字符串格式化 (约 45 个 String 字段), FlightInfo/EngineInfo 等 overlay
//! 的直接数据源 (Agent C 批次; impl Service 跨文件块)。
//!
//! ## 格式化语义裁决
//!
//! - Java `String.format("%.Nf")` 是对**最短往返十进制**做 HALF_UP (2.675→"2.68"),
//!   与 Rust `{:.N}` 的二进制值半偶舍入双重分歧 → 下方 [`java_f`] 本地复刻
//!   (vm-core hud_calculator.rs 的 java_f 为 pub(crate), 跨 crate 不可见,
//!   vm-overlay minihud.rs "本地拷贝" 同款先例)。
//! - **禁用 `vm_core::format::format`** (FastNumberFormatter, Java UI 快速格式化
//!   语义, 二进制半舍入) —— 与 String.format 是两套语义, 混用必漂移。
//! - 断言值 = Java 8 oracle 对拍 (FmtOracle 复刻方法体逐表达式, 输入字面量与
//!   tests 模块逐字段一致)。
//!
//! PORT(锁纪律 §2.8): 读锁一次性快照全部输入 → 锁外纯格式化 → 短写锁写回
//! (格式化是纯计算, 临界区内无回调无 IO)。

use vm_core::g;

use super::{read_data, write_data, NASTRING, Service};

impl Service {
	/// Formats raw flight data into display strings.
	/// Previously named trans2String() - renamed for clarity.
	/// (以上 javadoc 逐字保留, Java L238-241)
	// PORT(dead_code): 调用方接线归主线 (process_polling_cycle 的 "将数据转换格式"
	// 处, service_loop.rs 的 TODO(port) 顶位), 挂载即用。
	pub(super) fn format_data_as_strings(&mut self) {
		// R1 周期快照: 开头取一次句柄, 方法内一律用局部变量, 杜绝同周期混用两个句柄;
		// R2: blkx 非 null 即 READY, 无 FM 走下方 "-" 降级路径
		let fm = self.fm_manager.current();
		// blkx 消费面 (Java L388 `blkx != null && blkx.nitro != 0`)
		let (blkx_present, blkx_nitro, blkx_nitro_decr) = match fm.blkx.as_ref() {
			Some(b) => (true, b.nitro, b.nitro_decr),
			None => (false, 0.0, 0.0),
		};

		// 数据转换格式
		// sState

		// 读锁一次性快照 (§2.8): sState/sIndic 取本方法消费的标量 (calculate 之后
		// 调用, 非 null 域, unwrap 复刻; pitch[0]/efficiency[0] 空 Vec 索引 panic
		// ↔ Java AIOOBE→run 顶层 catch, 保真)
		let inp = {
			let d = read_data(&self.data);
			let s = d.s_state.as_ref().unwrap();
			let i = d.s_indic.as_ref().unwrap();
			FmtInputs {
				throttle: s.throttle,
				aileron: s.aileron,
				elevator: s.elevator,
				rudder: s.rudder,
				rpm: s.rpm,
				efficiency0: s.efficiency[0],
				manifoldpressure: s.manifoldpressure,
				pitch0: s.pitch[0],
				rpm_throttle: s.rpm_throttle,
				radiator: s.radiator,
				mixture: s.mixture,
				flaps: s.flaps,
				ias: s.ias,
				tas: s.tas,
				wx: s.wx,
				m: s.m,
				ny: s.ny,
				aoa: s.aoa,
				aos: s.aos,
				compressorstage: s.compressorstage,
				wsweep_indicator: i.wsweep_indicator,
				radio_altitude: i.radio_altitude,
				aviahorizon_pitch: i.aviahorizon_pitch,
				elapsed_time: d.elapsed_time,
				fueltime: d.fueltime,
				total_thrust: d.total_thrust,
				total_hp: d.total_hp,
				total_hp_eff: d.total_hp_eff,
				check_alt: d.check_alt,
				nwater_temp: d.nwater_temp,
				noil_temp: d.noil_temp,
				total_fuel: d.total_fuel,
				thurst_percent: d.thurst_percent,
				t_eng_response: d.t_eng_response,
				fuel_percent: d.fuel_percent,
				has_wing_sweep_vario: d.has_wing_sweep_vario,
				radio_alt: d.radio_alt,
				avgeff: d.avgeff,
				n_vy: d.n_vy,
				an: d.an,
				alt: d.alt,
				sep: d.sep,
				energy_j_kg: d.energy_j_kg,
				acceleration: d.acceleration,
				compass_delta: d.compass_delta,
				wep_time: d.wep_time,
				nitro_eng_nr: d.nitro_eng_nr,
				nitrokg: d.nitrokg,
				cur_load_min_work_time: d.cur_load_min_work_time,
				turn_rds: d.turn_rds,
				turn_rate: d.turn_rate,
				horizontal_load: d.horizontal_load,
			}
		};

		// ---- 锁外纯格式化 (语句顺序与 Java L252-429 逐行对应) ----

		// Java: throttle = String.format("%d", sState.throttle); —— %d 即 to_string
		let throttle = inp.throttle.to_string();
		let aileron = inp.aileron.to_string();
		let elevator = inp.elevator.to_string();
		let rudder = inp.rudder.to_string();

		// timeText = String.format("%02d'%02d", elapsedTime / 60000, (elapsedTime / 1000) % 60);
		let time_text = format!(
			"{}'{}",
			java_d0(inp.elapsed_time / 60000, 2),
			java_d0((inp.elapsed_time / 1000) % 60, 2)
		);
		let fueltime_str = if inp.fueltime <= 0 || inp.fueltime > 24 * 3600 * 1000 {
			NASTRING.to_string()
		} else if inp.fueltime / 60000 < 100 {
			// fueltimeStr = String.format("%02d'%02d", fueltime / 60000,
			//     (long) ((fueltime / 1000) % 60 / 10) * 10); —— 秒位向下取整到十位
			format!(
				"{}'{}",
				java_d0(inp.fueltime / 60000, 2),
				java_d0((inp.fueltime / 1000) % 60 / 10 * 10, 2)
			)
		} else {
			// String.format("%.0f", (float) fueltime / 60000) —— float 除法域再拓宽 (§2.12)
			java_f((inp.fueltime as f32 / 60000.0f32) as f64, 0)
		};
		let total_thrust_str = inp.total_thrust.to_string();
		let total_hp_str = if inp.total_hp == 0 {
			NASTRING.to_string()
		} else {
			inp.total_hp.to_string()
		};

		// Java: (int) sState.RPM —— State.RPM 本就是 int, 强转恒等
		let rpm = inp.rpm.to_string();
		let (use_mega_hp, total_hp_eff_str) = if inp.total_hp_eff >= 100000 {
			// totalHpEffStr = String.format("%.2f", totalHpEff / 1000000.0f)
			// —— int/float 的 float 除法域再拓宽 (§2.12)
			(true, java_f((inp.total_hp_eff as f32 / 1000000.0f32) as f64, 2))
		} else {
			(false, inp.total_hp_eff.to_string())
		};
		let efficiency0 = if inp.efficiency0 == 0.0 {
			NASTRING.to_string()
		} else {
			java_f(inp.efficiency0, 0)
		};
		let watertemp = if inp.nwater_temp != -65535.0 {
			java_f(inp.nwater_temp, 0)
		} else {
			NASTRING.to_string()
		};
		let oiltemp = java_f(inp.noil_temp, 0);
		// PORT(pressureMmHg 死字段保真): Java 仅在 manifoldpressure==1 的 else 分支
		// 写 pressureMmHg (全库唯一写点, 无读者) → None = 保持上轮值
		let pressure_mm_hg;
		let (manifoldpressure, pressure_unit_str, pressure_pounds, pressure_inch_hg) =
			if inp.manifoldpressure != 1.0 {
				// pressurePounds = String.format("%+.1f", (sState.manifoldpressure - 1) * 14.696);
				let pressure_pounds = java_f_plus((inp.manifoldpressure - 1.0) * 14.696, 1);
				// pressureInchHg = String.format("P/%.1f''", (sState.manifoldpressure * 760 / 25.4));
				let pressure_inch_hg =
					format!("P/{}''", java_f(inp.manifoldpressure * 760.0 / 25.4, 1));
				if inp.check_alt > 0 {
					// Imperial Mode: Value is Boost (psi), Unit is Manifold (inHg)
					pressure_mm_hg = None;
					(
						pressure_pounds.clone(),
						pressure_inch_hg.clone(),
						pressure_pounds,
						pressure_inch_hg,
					)
				} else {
					// Metric Mode: Value is Ata, Unit is Ata
					pressure_mm_hg = None;
					(
						java_f(inp.manifoldpressure, 2),
						"Ata".to_string(),
						pressure_pounds,
						pressure_inch_hg,
					)
				}
			} else {
				pressure_mm_hg = Some(NASTRING.to_string());
				(
					NASTRING.to_string(),
					"Ata".to_string(),
					NASTRING.to_string(),
					NASTRING.to_string(),
				)
			};
		let total_fuel_str = java_f(inp.total_fuel, 0);
		let pitch0 = if inp.pitch0 != -65535.0 {
			java_f(inp.pitch0, 1)
		} else {
			NASTRING.to_string()
		};
		let rpm_throttle = if inp.rpm_throttle >= 0 {
			inp.rpm_throttle.to_string()
		} else {
			NASTRING.to_string()
		};
		let s_thurst_percent = java_f(inp.thurst_percent, 0);
		let sd_thrust_percent = java_f(inp.t_eng_response, 0);

		let radiator = if inp.radiator >= 0 {
			inp.radiator.to_string()
		} else {
			NASTRING.to_string()
		};

		let mixture = if inp.mixture >= 0 {
			inp.mixture.to_string()
		} else {
			NASTRING.to_string()
		};
		let flaps = inp.flaps.to_string();
		let sfuel_percent = inp.fuel_percent.to_string();
		let s_wing_sweep = if inp.has_wing_sweep_vario {
			// sWingSweep = String.format("%.0f", sIndic.wsweep_indicator * 100.f);
			// —— 100.f 提升为 double (数值即 100.0, §2.12)
			java_f(inp.wsweep_indicator * 100.0, 0)
			// Application.debugPrint(sWingSweep);
		} else {
			NASTRING.to_string()
		};
		let s_radio_alt = if inp.radio_altitude >= 0.0 {
			java_f(inp.radio_alt, 0)
		} else {
			NASTRING.to_string()
		};
		//
		let s_avg_eff = if inp.avgeff == 0.0 {
			NASTRING.to_string()
		} else {
			// String.format("%d", Math.round(avgeff))
			java_round(inp.avgeff).to_string()
		};
		// Application.debugPrint(sWingSweep);
		let vy = java_f(inp.n_vy, 1);
		let s_n = if inp.an.abs() <= 1000.0 {
			java_f(inp.an / g, 1)
		} else {
			NASTRING.to_string()
		};
		let ias = inp.ias.to_string();
		let tas = inp.tas.to_string();
		let salt = java_f(inp.alt, 0);
		let wx = java_f(inp.wx.abs(), 0);
		let m = java_f(inp.m, 2);
		let ny = java_f(inp.ny, 1);

		// SEP取整改善SEP过高时的可读性
		// double SEPAccuracy = (double) ((long) SEP / 50); —— (long) 截断 + 整除后拓宽
		let mut sep_accuracy = ((inp.sep as i64) / 50) as f64;
		sep_accuracy *= 2.5;
		if sep_accuracy == 0.0 {
			sep_accuracy = 1.0;
		}

		// sSEP/sSEPAbs = String.format("%.0f", Math.round(SEP / SEPAccuracy) * SEPAccuracy …)
		let sep_rounded = java_round(inp.sep / sep_accuracy) as f64 * sep_accuracy;
		let s_sep = java_f(sep_rounded, 0);
		let s_sep_abs = java_f(sep_rounded.abs(), 0);
		// 相对能量(v^2/2+g*h)

		let rel_energy = java_f(inp.energy_j_kg, 0);

		let aclrt = java_f(inp.acceleration, 3);
		// Ao=String.format("%.1f",
		// Math.sqrt(sState.AoA*sState.AoA+sState.AoS*sState.AoS));
		let (aoa, aos) = if inp.aoa != -65535.0 {
			(java_f(inp.aoa, 1), java_f(inp.aos, 1))
		} else {
			(NASTRING.to_string(), NASTRING.to_string())
		};
		let compass = java_f(inp.compass_delta, 0);
		let s_pitch_up = java_f(inp.aviahorizon_pitch, 0);

		// PORT(sWepTime/sWepTimeVal 保真): nitro 在而 nitroEngNr==0 时 Java 空分支
		// 不写两者 (L392-395) → None = 保持上轮值; 无 nitro 的 else 分支 sWepTimeVal
		// 同样不重置 (仅 sNitro/sWepTime 归 "-")
		let (s_nitro, s_wep_time, s_wep_time_val) = if blkx_present && blkx_nitro != 0.0 {
			let s_nitro = java_f(inp.nitrokg, 0);
			if inp.nitro_eng_nr == 0 {
				// nitroEngNr = sState.engineNum;
				// sWepTime = nastring;
				(s_nitro, None, None)
			} else {
				// twepTime = (int)(((blkx.nitro / blkx.nitroDecr - wepTime / 1000))
				//     / nitroEngNr); —— wepTime/1000 是 long 整除后才并入 double
				let twep_time = ((blkx_nitro / blkx_nitro_decr - (inp.wep_time / 1000) as f64)
					/ inp.nitro_eng_nr as f64) as i32 as i64;

				// sWepTimeVal = twepTime;
				let s_wep_time = if twep_time / 60 >= 100 {
					// sWepTime = String.format("%3d", twepTime / 60); —— 宽度 3 右对齐
					pad_width((twep_time / 60).to_string(), 3, false)
				} else {
					// sWepTime = String.format("%02d'%02d", twepTime / 60, twepTime % 60);
					// sWepTime = String.format("%.0f", (double) twepTime);
					format!("{}'{}", java_d0(twep_time / 60, 2), java_d0(twep_time % 60, 2))
				};
				(s_nitro, Some(s_wep_time), Some(twep_time))
			}
		} else {
			(NASTRING.to_string(), Some(NASTRING.to_string()), None)
		};

		let s_acc = java_f(inp.acceleration, 1);
		let compressorstage = inp.compressorstage.to_string();
		let s_eng_work_time = if inp.cur_load_min_work_time == 99999000.0 {
			NASTRING.to_string()
		} else {
			// String.format("%.0f", curLoadMinWorkTime / 1000) —— double/int 除法
			java_f(inp.cur_load_min_work_time / 1000.0, 0)
		};

		let s_turn_rds = if inp.turn_rds.abs() < 9999.0 {
			java_f(inp.turn_rds.abs(), 0)
		} else {
			NASTRING.to_string()
		};

		let s_turn_rate = if inp.turn_rate < 999.0 {
			java_f(inp.turn_rate, 1)
		} else {
			NASTRING.to_string()
		};
		let s_horizontal_load = java_f(inp.horizontal_load, 1);

		// ---- 短写锁写回 (赋值顺序与 Java 逐行一致) ----
		{
			let mut d = write_data(&self.data);
			d.throttle = Some(throttle);
			d.aileron = Some(aileron);
			d.elevator = Some(elevator);
			d.rudder = Some(rudder);
			d.time_text = Some(time_text);
			d.fueltime_str = Some(fueltime_str);
			d.total_thrust_str = Some(total_thrust_str);
			d.total_hp_str = Some(total_hp_str);
			d.rpm = Some(rpm);
			d.use_mega_hp = use_mega_hp;
			d.total_hp_eff_str = Some(total_hp_eff_str);
			// efficiency[0] = …; (null 数组 NPE ↔ Option/空 Vec panic, 保真)
			d.efficiency.as_mut().unwrap()[0] = Some(efficiency0);
			d.watertemp = Some(watertemp);
			d.oiltemp = Some(oiltemp);
			d.pressure_pounds = Some(pressure_pounds);
			d.pressure_inch_hg = Some(pressure_inch_hg);
			d.manifoldpressure = Some(manifoldpressure);
			d.pressure_unit_str = Some(pressure_unit_str);
			if let Some(v) = pressure_mm_hg {
				d.pressure_mm_hg = Some(v);
			}
			d.total_fuel_str = Some(total_fuel_str);
			d.pitch.as_mut().unwrap()[0] = Some(pitch0);
			d.rpm_throttle = Some(rpm_throttle);
			d.s_thurst_percent = Some(s_thurst_percent);
			d.sd_thrust_percent = Some(sd_thrust_percent);
			d.radiator = Some(radiator);
			d.mixture = Some(mixture);
			d.flaps = Some(flaps);
			d.sfuel_percent = Some(sfuel_percent);
			d.s_wing_sweep = Some(s_wing_sweep);
			d.s_radio_alt = Some(s_radio_alt);
			d.s_avg_eff = Some(s_avg_eff);
			d.vy = Some(vy);
			d.s_n = Some(s_n);
			d.ias = Some(ias);
			d.tas = Some(tas);
			d.salt = Some(salt);
			d.wx = Some(wx);
			d.m = Some(m);
			d.ny = Some(ny);
			d.s_sep = Some(s_sep);
			d.s_sep_abs = Some(s_sep_abs);
			d.rel_energy = Some(rel_energy);
			d.aclrt = Some(aclrt);
			d.aoa = Some(aoa);
			d.aos = Some(aos);
			d.compass = Some(compass);
			d.s_pitch_up = Some(s_pitch_up);
			d.s_nitro = Some(s_nitro);
			if let Some(v) = s_wep_time {
				d.s_wep_time = Some(v);
			}
			if let Some(v) = s_wep_time_val {
				d.s_wep_time_val = v;
			}
			d.s_acc = Some(s_acc);
			d.compressorstage = Some(compressorstage);
			d.s_eng_work_time = Some(s_eng_work_time);
			d.s_turn_rds = Some(s_turn_rds);
			d.s_turn_rate = Some(s_turn_rate);
			d.s_horizontal_load = Some(s_horizontal_load);
		}

		// Java L431: publishFlightDataEvent(); —— 发布不在此调用:
		// 接线归 process_polling_cycle (service_loop.rs "将数据转换格式" 处的直接
		// publish 调用顶位, 时序等价), 主线统一接线
	}
}

/// 读锁快照: formatDataAsStrings 的全部输入 (§2.8 锁外格式化的值拷贝载体)。
/// sState/sIndic 只取本方法消费的标量字段, 派生量直读 ServiceData。
struct FmtInputs {
	// sState 消费面
	throttle: i32,
	aileron: i32,
	elevator: i32,
	rudder: i32,
	rpm: i32,
	efficiency0: f64,
	manifoldpressure: f64,
	pitch0: f64,
	rpm_throttle: i32,
	radiator: i32,
	mixture: i32,
	flaps: i32,
	ias: i32,
	tas: i32,
	wx: f64,
	m: f64,
	ny: f64,
	aoa: f64,
	aos: f64,
	compressorstage: i32,
	// sIndic 消费面
	wsweep_indicator: f64,
	radio_altitude: f64,
	aviahorizon_pitch: f64,
	// 派生量/计时字段
	elapsed_time: i64,
	fueltime: i64,
	total_thrust: i32,
	total_hp: i32,
	total_hp_eff: i32,
	check_alt: i32,
	nwater_temp: f64,
	noil_temp: f64,
	total_fuel: f64,
	thurst_percent: f64,
	t_eng_response: f64,
	fuel_percent: i32,
	has_wing_sweep_vario: bool,
	radio_alt: f64,
	avgeff: f64,
	n_vy: f64,
	an: f64,
	alt: f64,
	sep: f64,
	energy_j_kg: f64,
	acceleration: f64,
	compass_delta: f64,
	wep_time: i64,
	nitro_eng_nr: i32,
	nitrokg: f64,
	cur_load_min_work_time: f64,
	turn_rds: f64,
	turn_rate: f64,
	horizontal_load: f64,
}

// ------------------------------------------------------------------
// Java printf/Math 语义助手 (java_f/pad_width 为 vm-core hud_calculator.rs
// 的本地拷贝 —— pub(crate) 跨 crate 不可见, vm-overlay minihud.rs 同款先例)
// ------------------------------------------------------------------

/// Java `String.format("%N.Mf", d)` 的数值段 (不含宽度): 对**最短往返十进制**
/// HALF_UP。语义模型与 config_loader::java_format_f4 / flight_analyzer::java_format_f1
/// 同源 (Java 8 oracle 实证, 本模块 tests 的 FmtOracle 全字段对拍):
/// - 2.675 → "2.68" (Rust `{:.2}` 会给 "2.67");
/// - -0.4 → "-0" / -0.04 → "-0.0" (舍入到零仍保留负号);
/// - NaN/Infinity 原样 ("NaN"/"Infinity"/"-Infinity");
/// - `exp10 > 25` 是纯实现切点, 非语义边界: else 支路的 scaled 定点累加在 u128
///   内, 10^308 量级会溢出; 该域最短表示位数 n ≤ 17 < keep, 判定位恒 0, 无舍入,
///   走 digits + 补零的字符串路径;
/// - JDK-4511638 已知分歧 (同 config_loader::java_format_f4 裁决): Java 8 旧 dtoa
///   在大值域 (~1e17 起) 偶发非最短 toString, 而 %f 按**自身 toString 的数字**
///   展开 — 1e23 → "9.999999999999999E22" → "99999999999999990000000", 既非精确
///   二进制 (...91611392) 也非最短展开; Rust `{:e}` 给真最短 "1e23" → 本实现输出
///   "100000000000000000000000"。HUD 值域 (速度/高度/能量 < 10^7) 距该域不可达
///   (Java 8 oracle fuzz 35k 例仅 1e23 一例分歧)。
fn java_f(d: f64, prec: usize) -> String {
	if d.is_nan() {
		return "NaN".to_string();
	}
	if d.is_infinite() {
		return if d > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
	}
	// 负号含 -0.0: Java 舍入到零的负数仍输出 "-0"/"-0.0" (oracle 验证)
	let neg = d.is_sign_negative();
	let a = d.abs();
	// Rust `{:e}` 即最短往返科学计数 (与 Java Double.toString 同一最短表示)
	let sci = format!("{a:e}");
	let epos = sci.find('e').unwrap();
	let exp10: i32 = sci[epos + 1..].parse().unwrap();
	let digits = sci[..epos].replace('.', "");
	let digits = digits.as_bytes();
	let n = digits.len() as i32;

	let mut out = String::new();
	if exp10 > 25 {
		// 巨整数域: digits + 隐含尾零 (+ 小数点补零)
		out.push_str(&sci[..epos].replace('.', ""));
		out.push_str(&"0".repeat((exp10 - n + 1) as usize));
		if prec > 0 {
			out.push('.');
			out.push_str(&"0".repeat(prec));
		}
	} else {
		// 最短表示的 i 号数字 (1-based, place = 10^(exp10-i+1)); 越界补 0
		let digit_at = |i: i32| -> u128 {
			if i < 1 {
				0
			} else {
				let idx = (i - 1) as usize;
				if idx < digits.len() {
					u128::from(digits[idx] - b'0')
				} else {
					0
				}
			}
		};
		// 保留到 10^-prec 位: i ≤ exp10 + 1 + prec; 判定位 = 其后一位
		// (HALF_UP: ≥5 进位, 再后的剩余数字 < 1 单位不影响判定; 进位可级联)
		let keep = exp10 + 1 + prec as i32;
		let mut scaled: u128 = 0;
		if keep > 0 {
			for i in 1..=keep {
				scaled = scaled * 10 + digit_at(i);
			}
		}
		if digit_at(keep + 1) >= 5 {
			scaled += 1;
		}
		let p10 = 10u128.pow(prec as u32);
		let int_part = scaled / p10;
		let frac = scaled % p10;
		out.push_str(&int_part.to_string());
		if prec > 0 {
			out.push('.');
			let fs = frac.to_string();
			for _ in fs.len()..prec {
				out.push('0');
			}
			out.push_str(&fs);
		}
	}
	if neg {
		out.insert(0, '-');
	}
	out
}

/// Java printf 宽度语义: 不足补空格 (默认右对齐, '-' 左对齐), 超宽不截断。
/// 宽度按字符计 (数值/NaN/Infinity 输出纯 ASCII, 与 Java UTF-16 码元计数同值)。
fn pad_width(mut s: String, width: usize, left_align: bool) -> String {
	let len = s.chars().count();
	if len >= width {
		return s;
	}
	let fill = " ".repeat(width - len);
	if left_align {
		s.push_str(&fill);
	} else {
		s.insert_str(0, &fill);
	}
	s
}

/// Java `String.format("%0Nd", v)` (long 域): '0' 标志的零填充, 符号感知
/// (负号后补零, 宽度含符号位; 已超宽不截断)。
fn java_d0(v: i64, width: usize) -> String {
	let s = v.to_string();
	if s.len() >= width {
		return s;
	}
	let (sign, digits) = match s.strip_prefix('-') {
		Some(rest) => ("-", rest),
		None => ("", s.as_str()),
	};
	let fill = "0".repeat(width - s.len());
	format!("{sign}{fill}{digits}")
}

/// Java `String.format("%+.Nf", d)` 的 '+' 标志: 非负值强制带 '+' (含 +0.0)。
/// NaN 不加号 (Java 8 oracle 实测 "+NaN" 不存在, 恒 "NaN")。
fn java_f_plus(d: f64, prec: usize) -> String {
	if d.is_nan() {
		return "NaN".to_string();
	}
	if d.is_infinite() {
		return if d > 0.0 { "+Infinity".to_string() } else { "-Infinity".to_string() };
	}
	if d.is_sign_negative() {
		java_f(d, prec)
	} else {
		format!("+{}", java_f(d, prec))
	}
}

/// Java `Math.round(double)` → long: floor(x + 0.5)。
/// NaN → 0 / 超域饱和到 Long.MAX/MIN —— Rust `as i64` 饱和转型同语义 (§2.2)。
fn java_round(d: f64) -> i64 {
	(d + 0.5).floor() as i64
}

// =====================================================================
// Tests — Java 8 oracle 对拍 (FmtOracle: 方法体格式化表达式逐行复刻,
// 输入字面量与本模块逐字段一致; STATE_MOCK/INDIC_MOCK 为 service_loop/tests.rs
// 本地拷贝, 项目先例)
// =====================================================================
#[cfg(test)]
mod tests {
	use super::*;
	use crate::service_fields::ServiceData;
	use std::path::PathBuf;
	use std::sync::Arc;
	use std::time::{Duration, Instant};
	use vm_core::bus::EventBus;
	use vm_core::flight_data_bus::FlightDataBus;
	use vm_core::fm::fm_data_paths;
	use vm_core::fm::status::FMStatus;
	use vm_core::fm::FMManager;

	use super::super::ServiceConfig;

	/// 真机抓取的 /state 快照 (service_loop/tests.rs 同源拷贝; 冒号后一空格)
	const STATE_MOCK: &str = "{\"valid\": true,\"aileron, %\": -48,\"elevator, %\": 20,\"rudder, %\": -47,\"flaps, %\": 0,\"gear, %\": 0,\"H, m\": 46,\"TAS, km/h\": 454,\"IAS, km/h\": 474,\"M\": 0.39,\"AoA, deg\": -1.6,\"AoS, deg\": -5.9,\"Ny\": 0.35,\"Vy, m/s\": -7.3,\"Wx, deg/s\": -34,\"Mfuel, kg\": 197,\"Mfuel0, kg\": 734,\"throttle 1, %\": 110,\"RPM throttle 1, %\": 100,\"mixture 1, %\": 100,\"radiator 1, %\": 42,\"magneto 1\": 3,\"power 1, hp\": 1597.8,\"RPM 1\": 3001,\"manifold pressure 1, atm\": 2.24,\"water temp 1, C\": 121,\"oil temp 1, C\": 90,\"pitch 1, deg\": 35.5,\"thrust 1, kgs\": 840,\"efficiency 1, %\": 87}";

	fn new_service() -> Service {
		let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
		let bus = Arc::new(FlightDataBus::new());
		Service::new(ServiceConfig::default(), fm, bus)
	}

	/// 场景 A (公制主线): STATE_MOCK 解析 + 手设派生量/计时字段。
	/// 全部字面量与 Java oracle (FmtOracle 场景 A) 逐字段一致。
	fn scenario_a(d: &mut ServiceData) {
		d.s_state.as_mut().unwrap().update(STATE_MOCK);
		// sIndic 消费面三字段手设 (oracle 同字面量; 双精度域非 f32 拓宽)
		{
			let i = d.s_indic.as_mut().unwrap();
			i.aviahorizon_pitch = 0.632352;
			i.wsweep_indicator = 0.6;
			i.radio_altitude = 1000.0;
		}
		d.elapsed_time = 3723456;
		d.fueltime = 150000;
		d.total_thrust = 840;
		d.total_hp = 1597;
		d.total_hp_eff = 1412;
		d.check_alt = 0;
		d.nwater_temp = 121.0;
		d.noil_temp = 90.0;
		d.total_fuel = 197.0;
		d.thurst_percent = 2.5;
		d.t_eng_response = 2.675;
		d.fuel_percent = 26;
		d.has_wing_sweep_vario = false;
		d.radio_alt = 245.7;
		d.avgeff = 88.41577958672511;
		d.n_vy = -7.342558;
		d.an = 24.99;
		d.alt = 46.0;
		d.sep = 1234.5678;
		d.energy_j_kg = 1521.7346938775509;
		d.acceleration = 0.0005;
		d.compass_delta = 164.09729;
		d.cur_load_min_work_time = 15000.0;
		d.turn_rds = -2500.5;
		d.turn_rate = 12.25;
		d.horizontal_load = -1.75;
	}

	/// 场景 A 全字段断言 (Java 8 oracle 值; HALF_UP 判别点:
	/// Ny 0.35f 拓宽域→"0.3" / sThurstPercent 2.5→"3" / turnRate 12.25→"12.3" /
	/// turnRds 2500.5→"2501" / SdThrustPercent 2.675→"3" —— Rust {:.N} 均不同值)
	#[test]
	fn format_strings_scenario_a_metric() {
		let mut svc = new_service();
		{
			let mut d = write_data(&svc.data);
			scenario_a(&mut d);
		}
		svc.format_data_as_strings();
		{
			let d = read_data(&svc.data);
			assert_eq!(d.throttle.as_deref(), Some("110"));
			assert_eq!(d.aileron.as_deref(), Some("-48"));
			assert_eq!(d.elevator.as_deref(), Some("20"));
			assert_eq!(d.rudder.as_deref(), Some("-47"));
			assert_eq!(d.time_text.as_deref(), Some("62'03"));
			assert_eq!(d.fueltime_str.as_deref(), Some("02'30"));
			assert_eq!(d.total_thrust_str.as_deref(), Some("840"));
			assert_eq!(d.total_hp_str.as_deref(), Some("1597"));
			assert_eq!(d.rpm.as_deref(), Some("3001"));
			assert!(!d.use_mega_hp);
			assert_eq!(d.total_hp_eff_str.as_deref(), Some("1412"));
			assert_eq!(d.efficiency.as_ref().unwrap()[0].as_deref(), Some("87"));
			assert_eq!(d.watertemp.as_deref(), Some("121"));
			assert_eq!(d.oiltemp.as_deref(), Some("90"));
			assert_eq!(d.manifoldpressure.as_deref(), Some("2.24"));
			assert_eq!(d.pressure_unit_str.as_deref(), Some("Ata"));
			assert_eq!(d.pressure_pounds.as_deref(), Some("+18.2"));
			assert_eq!(d.pressure_inch_hg.as_deref(), Some("P/67.0''"));
			assert_eq!(d.total_fuel_str.as_deref(), Some("197"));
			assert_eq!(d.pitch.as_ref().unwrap()[0].as_deref(), Some("35.5"));
			assert_eq!(d.rpm_throttle.as_deref(), Some("100"));
			assert_eq!(d.s_thurst_percent.as_deref(), Some("3"));
			assert_eq!(d.sd_thrust_percent.as_deref(), Some("3"));
			assert_eq!(d.radiator.as_deref(), Some("42"));
			assert_eq!(d.mixture.as_deref(), Some("100"));
			assert_eq!(d.flaps.as_deref(), Some("0"));
			assert_eq!(d.sfuel_percent.as_deref(), Some("26"));
			assert_eq!(d.s_wing_sweep.as_deref(), Some("-"));
			assert_eq!(d.s_radio_alt.as_deref(), Some("246"));
			assert_eq!(d.s_avg_eff.as_deref(), Some("88"));
			assert_eq!(d.vy.as_deref(), Some("-7.3"));
			assert_eq!(d.s_n.as_deref(), Some("2.6"));
			assert_eq!(d.ias.as_deref(), Some("474"));
			assert_eq!(d.tas.as_deref(), Some("454"));
			assert_eq!(d.salt.as_deref(), Some("46"));
			assert_eq!(d.wx.as_deref(), Some("34"));
			assert_eq!(d.m.as_deref(), Some("0.39"));
			assert_eq!(d.ny.as_deref(), Some("0.3"));
			assert_eq!(d.s_sep.as_deref(), Some("1260"));
			assert_eq!(d.s_sep_abs.as_deref(), Some("1260"));
			assert_eq!(d.rel_energy.as_deref(), Some("1522"));
			assert_eq!(d.aclrt.as_deref(), Some("0.001"));
			assert_eq!(d.aoa.as_deref(), Some("-1.6"));
			assert_eq!(d.aos.as_deref(), Some("-5.9"));
			assert_eq!(d.compass.as_deref(), Some("164"));
			assert_eq!(d.s_pitch_up.as_deref(), Some("1"));
			assert_eq!(d.s_nitro.as_deref(), Some("-"));
			assert_eq!(d.s_wep_time.as_deref(), Some("-"));
			assert_eq!(d.s_acc.as_deref(), Some("0.0"));
			assert_eq!(d.compressorstage.as_deref(), Some("0"));
			assert_eq!(d.s_eng_work_time.as_deref(), Some("15"));
			assert_eq!(d.s_turn_rds.as_deref(), Some("2501"));
			assert_eq!(d.s_turn_rate.as_deref(), Some("12.3"));
			assert_eq!(d.s_horizontal_load.as_deref(), Some("-1.8"));
		}
	}

	/// 场景 B (英制 + 哨兵电池 + fueltime/manifold 边界), Java 8 oracle 值。
	#[test]
	fn format_strings_scenario_b_imperial_and_na() {
		let mut svc = new_service();
		{
			let mut d = write_data(&svc.data);
			scenario_a(&mut d);
			// B1: 英制 (checkAlt>0) + 全哨兵电池 (oracle 场景 B1 同字面量)
			d.check_alt = 500;
			{
				let s = d.s_state.as_mut().unwrap();
				s.manifoldpressure = 1.85; // 手设 f64 域 (非 STATE_MOCK 的 f32 拓宽)
				s.rpm_throttle = -1;
				s.radiator = -1;
				s.mixture = -1;
				s.pitch[0] = -65535.0;
				s.efficiency[0] = 0.0;
				s.aoa = -65535.0;
			}
			{
				let i = d.s_indic.as_mut().unwrap();
				i.wsweep_indicator = 0.55;
				i.radio_altitude = -65535.0;
				i.aviahorizon_pitch = -0.632352;
			}
			d.elapsed_time = 7380045;
			d.fueltime = 7500000;
			d.total_hp = 0;
			d.total_hp_eff = 1234567;
			d.nwater_temp = -65535.0;
			d.noil_temp = -65535.0;
			d.thurst_percent = 17.5;
			d.t_eng_response = -2.25;
			d.fuel_percent = 0;
			d.has_wing_sweep_vario = true;
			d.avgeff = 0.0;
			d.n_vy = -0.04;
			d.an = 19600.0;
			d.alt = 999.99;
			d.sep = 30.0;
			d.energy_j_kg = 0.4;
			d.acceleration = 9.8765;
			d.compass_delta = 359.97;
			d.cur_load_min_work_time = 99999000.0;
			d.turn_rds = 15000.0;
			d.turn_rate = 999.5;
			d.horizontal_load = 0.25;
			d.total_fuel = 8123.45;
		}
		svc.format_data_as_strings();
		{
			let d = read_data(&svc.data);
			assert_eq!(d.manifoldpressure.as_deref(), Some("+12.5"), "英制: 值=psi");
			assert_eq!(d.pressure_unit_str.as_deref(), Some("P/55.4''"));
			assert_eq!(d.pressure_pounds.as_deref(), Some("+12.5"));
			assert_eq!(d.pressure_inch_hg.as_deref(), Some("P/55.4''"));
			assert_eq!(d.time_text.as_deref(), Some("123'00"));
			assert_eq!(d.fueltime_str.as_deref(), Some("125"), "≥100 分钟走 %.0f");
			assert_eq!(d.total_hp_str.as_deref(), Some("-"));
			assert!(d.use_mega_hp);
			assert_eq!(d.total_hp_eff_str.as_deref(), Some("1.23"), "Mhp 域 float 除法");
			assert_eq!(d.efficiency.as_ref().unwrap()[0].as_deref(), Some("-"));
			assert_eq!(d.watertemp.as_deref(), Some("-"));
			assert_eq!(d.oiltemp.as_deref(), Some("-65535"), "油温无条件格式化");
			assert_eq!(d.pitch.as_ref().unwrap()[0].as_deref(), Some("-"));
			assert_eq!(d.rpm_throttle.as_deref(), Some("-"));
			assert_eq!(d.s_thurst_percent.as_deref(), Some("18"), "17.5 HALF_UP");
			assert_eq!(d.sd_thrust_percent.as_deref(), Some("-2"));
			assert_eq!(d.radiator.as_deref(), Some("-"));
			assert_eq!(d.mixture.as_deref(), Some("-"));
			assert_eq!(d.sfuel_percent.as_deref(), Some("0"));
			assert_eq!(d.s_wing_sweep.as_deref(), Some("55"));
			assert_eq!(d.s_radio_alt.as_deref(), Some("-"));
			assert_eq!(d.s_avg_eff.as_deref(), Some("-"));
			assert_eq!(d.vy.as_deref(), Some("-0.0"), "舍到零的负数保负号");
			assert_eq!(d.s_n.as_deref(), Some("-"), "|An|>1000");
			assert_eq!(d.salt.as_deref(), Some("1000"));
			assert_eq!(d.s_sep.as_deref(), Some("30"), "SEPAccuracy==0 → 1");
			assert_eq!(d.s_sep_abs.as_deref(), Some("30"));
			assert_eq!(d.rel_energy.as_deref(), Some("0"));
			assert_eq!(d.aclrt.as_deref(), Some("9.877"));
			assert_eq!(d.aoa.as_deref(), Some("-"));
			assert_eq!(d.aos.as_deref(), Some("-"));
			assert_eq!(d.compass.as_deref(), Some("360"));
			assert_eq!(d.s_pitch_up.as_deref(), Some("-1"));
			assert_eq!(d.s_acc.as_deref(), Some("9.9"));
			assert_eq!(d.s_eng_work_time.as_deref(), Some("-"), "==99999*1000");
			assert_eq!(d.s_turn_rds.as_deref(), Some("-"));
			assert_eq!(d.s_turn_rate.as_deref(), Some("-"));
			assert_eq!(d.s_horizontal_load.as_deref(), Some("0.3"), "0.25 HALF_UP");
			assert_eq!(d.total_fuel_str.as_deref(), Some("8123"));
		}

		// B2: manifoldpressure==1 → 四压强串全 "-" (Java L303-309 else 分支)
		{
			let mut d = write_data(&svc.data);
			d.s_state.as_mut().unwrap().manifoldpressure = 1.0;
		}
		svc.format_data_as_strings();
		{
			let d = read_data(&svc.data);
			assert_eq!(d.manifoldpressure.as_deref(), Some("-"));
			assert_eq!(d.pressure_unit_str.as_deref(), Some("Ata"));
			assert_eq!(d.pressure_pounds.as_deref(), Some("-"));
			assert_eq!(d.pressure_inch_hg.as_deref(), Some("-"));
			assert_eq!(
				d.pressure_mm_hg.as_deref(),
				Some("-"),
				"pressureMmHg 唯一写点 (else 分支)"
			);
		}

		// F: fueltimeStr 边界 (oracle 场景 F)
		for (fueltime, expect) in [
			(0i64, "-"),
			(86400001, "-"),
			(5999999, "99'50"),
			(6000000, "100"),
		] {
			{
				let mut d = write_data(&svc.data);
				d.fueltime = fueltime;
			}
			svc.format_data_as_strings();
			assert_eq!(
				read_data(&svc.data).fueltime_str.as_deref(),
				Some(expect),
				"fueltime={fueltime}"
			);
		}
	}

	/// 场景 C/D/E: nitro 块 (合成 data root 的 READY 句柄, store_tests.rs 先例;
	/// twepTime/秒位串/sWepTimeVal 保持语义)。Java 8 oracle 场景 C/D 值。
	#[test]
	fn format_strings_nitro_block_wep_time() {
		// DATA_ROOT 全局态串行锁 (见 lib.rs DATA_ROOT_TEST_LOCK 注): 覆盖
		// set_data_root(tmp) → RootCleanup 复位 全程, 与真机管道测试互斥
		let _root_guard =
			crate::DATA_ROOT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
		// 合成 data root: central + 物理 fm (MaxNitro/NitroConsumption 进 blkx)
		let tmp = std::env::temp_dir().join(format!("vm_fmt_nitro_{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&tmp);
		let fm_dir = tmp.join("aces/gamedata/flightmodels");
		let fm_sub = fm_dir.join("fm");
		std::fs::create_dir_all(&fm_sub).unwrap();
		// planec: nitro=300, decr=0.5 (twep=(300/0.5-30)/2=285 → "04'45")
		std::fs::write(
			fm_dir.join("planec.blkx"),
			"model:t = \"planec\"\nfmFile:t = \"fm/planec.blk\"\n",
		)
		.unwrap();
		std::fs::write(
			fm_sub.join("planec.blkx"),
			"synthetic-fm:t = \"planec\"\nMaxNitro:r = 300\nNitroConsumption:r = 0.5\nEmptyMass:r = 1000\nWingspan:r = 11\n",
		)
		.unwrap();
		// planed: nitro=10000 (twep=(20000-0)/2=10000 → 10000/60=166 → "%3d")
		std::fs::write(
			fm_dir.join("planed.blkx"),
			"model:t = \"planed\"\nfmFile:t = \"fm/planed.blk\"\n",
		)
		.unwrap();
		std::fs::write(
			fm_sub.join("planed.blkx"),
			"synthetic-fm:t = \"planed\"\nMaxNitro:r = 10000\nNitroConsumption:r = 0.5\nEmptyMass:r = 1000\nWingspan:r = 11\n",
		)
		.unwrap();
		fm_data_paths::set_data_root(&tmp.to_string_lossy());
		struct RootCleanup(PathBuf);
		impl Drop for RootCleanup {
			fn drop(&mut self) {
				fm_data_paths::set_data_root("./data");
				let _ = std::fs::remove_dir_all(&self.0);
			}
		}
		let _cleanup = RootCleanup(tmp.clone());

		let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
		let mut svc =
			Service::new(ServiceConfig::default(), Arc::clone(&fm), Arc::new(FlightDataBus::new()));

		// C: 识别 planec → 等异步加载 READY (identify 是唯一入口)
		fm.identify(Some("planec"));
		let deadline = Instant::now() + Duration::from_secs(10);
		while fm.current().status != FMStatus::Ready && Instant::now() < deadline {
			std::thread::sleep(Duration::from_millis(20));
		}
		assert_eq!(fm.current().status, FMStatus::Ready, "planec 应加载 (data root 生效)");
		{
			let mut d = write_data(&svc.data);
			d.s_state.as_mut().unwrap().update(STATE_MOCK);
			d.wep_time = 30000;
			d.nitro_eng_nr = 2;
			d.nitrokg = 85.6;
		}
		svc.format_data_as_strings();
		{
			let d = read_data(&svc.data);
			assert_eq!(d.s_nitro.as_deref(), Some("86"));
			assert_eq!(d.s_wep_time.as_deref(), Some("04'45"));
			assert_eq!(d.s_wep_time_val, 285, "sWepTimeVal = twepTime");
		}

		// E: nitro 在而 nitroEngNr==0 → sWepTime/sWepTimeVal 保持上轮 (Java 空分支)
		{
			let mut d = write_data(&svc.data);
			d.nitro_eng_nr = 0;
		}
		svc.format_data_as_strings();
		{
			let d = read_data(&svc.data);
			assert_eq!(d.s_nitro.as_deref(), Some("86"), "sNitro 仍格式化");
			assert_eq!(d.s_wep_time.as_deref(), Some("04'45"), "sWepTime 不动");
			assert_eq!(d.s_wep_time_val, 285, "sWepTimeVal 不动");
		}

		// D: 换 planed (nitro=10000) → twepTime/60 ≥ 100 → "%3d" 右对齐
		fm.identify(Some("planed"));
		// 等待条件必须是"新句柄到位"而非 status==Ready — LOADING 期间 current()
		// 仍返回旧句柄 (planec 本就 Ready), 只看 status 会立即假通过
		let deadline = Instant::now() + Duration::from_secs(10);
		while fm.current().name.as_deref() != Some("planed") && Instant::now() < deadline {
			std::thread::sleep(Duration::from_millis(20));
		}
		assert_eq!(fm.current().name.as_deref(), Some("planed"), "planed 新句柄到位");
		{
			let mut d = write_data(&svc.data);
			d.wep_time = 0;
			d.nitro_eng_nr = 2;
			// ServiceData.fm 是周期快照字段 (calculate 首行经 identify 后写入);
			// 直调方法须手动同步为新句柄, 否则仍按 planec 的 nitro 计算
			// (实得 "05'00" = planec nitro=300 的结果, 生产链无此形态)
			d.fm = fm.current();
		}
		svc.format_data_as_strings();
		{
			let d = read_data(&svc.data);
			assert_eq!(d.s_wep_time.as_deref(), Some("166"));
			assert_eq!(d.s_wep_time_val, 10000);
		}
	}

	/// 助手电池: %0Nd 零填充 (符号感知) 与 %3d 宽度 (Java 8 oracle probes)
	#[test]
	fn helper_pad_semantics() {
		assert_eq!(java_d0(5, 2), "05");
		assert_eq!(java_d0(-5, 2), "-5");
		assert_eq!(java_d0(-5, 3), "-05");
		assert_eq!(java_d0(123, 2), "123", "超宽不截断");
		assert_eq!(pad_width("166".to_string(), 3, false), "166");
		assert_eq!(pad_width("5".to_string(), 3, false), "  5");
		// %+ 电池 (oracle probes)
		assert_eq!(java_f_plus(0.0, 1), "+0.0");
		assert_eq!(java_f_plus(-0.04, 1), "-0.0");
		assert_eq!(java_f_plus(f64::NAN, 1), "NaN", "NaN 不加号");
		assert_eq!(java_f_plus(f64::INFINITY, 1), "+Infinity");
		// HALF_UP 判别 (Rust {:.N} 在同值上分别给 2.67/0.2/0.3/2500/12.2)
		assert_eq!(java_f(2.675, 2), "2.68");
		assert_eq!(java_f(0.25, 1), "0.3");
		assert_eq!(java_f(2500.5, 0), "2501");
		assert_eq!(java_f(12.25, 1), "12.3");
	}
}
