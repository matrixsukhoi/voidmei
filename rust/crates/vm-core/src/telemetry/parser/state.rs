//! State 的 Rust 移植 (src/parser/State.java)
//! /state 遥测 JSON 的子串提取解析 (速度/高度/G/舵面/引擎数组)。
//!
//! PORT: Java 类 → pub struct + pub 字段 (§0.7, 不造 getter);
//! getString/getDataInt/getDataFloat 接已译 crate::string_helper,
//! -65535 哨兵引用其 I_INVALID 定义。

use crate::string_helper::{get_data_float, get_data_int, get_string, I_INVALID};

/// 遥测侧每引擎数组容量 (throttles/power/pitch/thrust/efficiency)。
/// 2026-08 全量普查 (TestFMAllBoundaries): 真机 FM 引擎数极值 14 (b_66b, 含助推器块),
/// 原 8 会静默丢第 9+ 引擎数据; 上调至 16 (= Blkx 解析护栏, 见 Blkx.getload)。
/// 下游消费循环均按实际 engineNum 遍历 (数据驱动), 扩容不影响小引擎机型行为
// PORT: Java `public static final int maxEngNum`; 仅用作数组长度/循环上界 → usize
pub const MAX_ENG_NUM: usize = 16;

/// /state 遥测快照。字段顺序与 Java 声明一致。
#[derive(Clone)]
pub struct State {
    /// Java `String valid` — getString 可返回 null (键缺失) → Option
    pub valid: Option<String>,
    pub flag: bool,
    pub engine_num: i32,
    pub aileron: i32,
    pub elevator: i32,
    pub rudder: i32,
    pub flaps: i32,
    pub gear: i32,
    pub tas: i32,
    pub ias: i32,
    pub m: f64,
    pub aoa: f64,
    pub heightm: f64,
    pub aos: f64,
    pub ny: f64,
    pub vy: f64,
    pub wx: f64,
    pub throttle: i32,
    pub rpm_throttle: i32,
    pub radiator: i32,
    pub oilradiator: i32,
    pub mixture: i32,
    pub compressorstage: i32,
    pub magenato: i32,
    pub power: Vec<f64>,
    pub rpm: i32,
    pub manifoldpressure: f64,
    pub watertemp: f64,
    pub oiltemp: f64,
    pub mfuel: f64,
    pub mfuel_1: f64,
    pub mfuel0: f64,
    /// 助推器燃料总量 (kg)，无助推器时为 -65535
    pub mfuel0_1: f64,
    pub pitch: Vec<f64>,
    pub thrust: Vec<i32>,
    pub efficiency: Vec<f64>,
    pub airbrake: i32,
    pub total_thr: f64,
    pub throttles: Vec<i32>,
}

impl State {
    /// 对应 Java `new State()`: 标量字段取 Java 默认值 (§2.10),
    /// 引擎数组保持空 (≈ Java null) — 未 init 就 update/getEngNum 会像 Java 一样抛错。
    pub fn new() -> Self {
        State {
            valid: None,
            flag: false,
            engine_num: 0,
            aileron: 0,
            elevator: 0,
            rudder: 0,
            flaps: 0,
            gear: 0,
            tas: 0,
            ias: 0,
            m: 0.0,
            aoa: 0.0,
            heightm: 0.0,
            aos: 0.0,
            ny: 0.0,
            vy: 0.0,
            wx: 0.0,
            throttle: 0,
            rpm_throttle: 0,
            radiator: 0,
            oilradiator: 0,
            mixture: 0,
            compressorstage: 0,
            magenato: 0,
            power: Vec::new(),
            rpm: 0,
            manifoldpressure: 0.0,
            watertemp: 0.0,
            oiltemp: 0.0,
            mfuel: 0.0,
            mfuel_1: 0.0,
            mfuel0: 0.0,
            mfuel0_1: 0.0,
            pitch: Vec::new(),
            thrust: Vec::new(),
            efficiency: Vec::new(),
            airbrake: 0,
            total_thr: 0.0,
            throttles: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        // System.out.println("state初始化了");
        self.valid = Some("false".to_string());
        self.throttles = vec![0; MAX_ENG_NUM];
        self.power = vec![0.0; MAX_ENG_NUM];
        self.pitch = vec![0.0; MAX_ENG_NUM];
        self.thrust = vec![0; MAX_ENG_NUM];
        self.efficiency = vec![0.0; MAX_ENG_NUM];
        self.engine_num = 0;
        self.airbrake = 0;
    }

    pub fn get_eng_num(&mut self, buf: &str) {
        for i in 0..MAX_ENG_NUM {
            self.thrust[i] = get_data_int(get_string(buf, &format!("thrust {}", i)));
            if self.thrust[i] != I_INVALID {
                self.engine_num += 1;
            }
        }
    }

    pub fn update(&mut self, buf: &str) -> i32 {
        self.valid = get_string(buf, "valid").map(str::to_string);
        // System.out.println(valid);
        if self.valid.is_none() {
            return -1;
        }
        if self.valid.as_deref() == Some("true") {
            // 无异常的
            self.flag = true;

            self.aileron = get_data_int(get_string(buf, "aileron"));
            self.elevator = get_data_int(get_string(buf, "elevator"));
            self.rudder = get_data_int(get_string(buf, "rudder"));
            self.flaps = get_data_int(get_string(buf, "flaps"));
            self.airbrake = get_data_int(get_string(buf, "airbrake"));
            self.gear = get_data_int(get_string(buf, "gear"));
            self.tas = get_data_int(get_string(buf, "TAS"));
            self.ias = get_data_int(get_string(buf, "IAS"));
            self.m = get_data_float(get_string(buf, "\"M\""));
            self.heightm = get_data_float(get_string(buf, "H, m"));
            self.aoa = get_data_float(get_string(buf, "AoA"));
            self.aos = get_data_float(get_string(buf, "AoS"));
            self.ny = get_data_float(get_string(buf, "Ny"));
            self.vy = get_data_float(get_string(buf, "Vy"));
            self.wx = get_data_float(get_string(buf, "Wx"));
            self.throttle = get_data_int(get_string(buf, "throttle"));
            self.rpm_throttle = get_data_int(get_string(buf, "RPM throttle"));
            if self.rpm_throttle == I_INVALID {
                // 自动桨机型(如P-63)8111不返回该字段, 归一化为-1表示无桨距数据(与mixture约定一致),
                // 防止哨兵值-65535泄漏到UI层撑爆桨距竖条
                self.rpm_throttle = -1;
            }

            self.radiator = get_data_int(get_string(buf, "radiator"));

            self.rpm = get_data_int(get_string(buf, "RPM 1"));

            self.manifoldpressure = get_data_float(get_string(buf, "manifold pressure 1"));

            self.mfuel = get_data_float(get_string(buf, "Mfuel"));
            self.mfuel_1 = get_data_float(get_string(buf, "Mfuel 1"));
            self.mfuel0 = get_data_float(get_string(buf, "Mfuel0"));
            self.mfuel0_1 = get_data_float(get_string(buf, "Mfuel0 1")); // 助推器燃料总量

            self.oiltemp = get_data_float(get_string(buf, "oil temp"));

            // engineNum = 1;
            self.mixture = get_data_int(get_string(buf, "mixture"));
            if self.mixture == I_INVALID {
                self.mixture = -1;
            }
            self.compressorstage = get_data_int(get_string(buf, "compressor stage"));
            if self.compressorstage == I_INVALID {
                self.compressorstage = 0;
            }

            self.magenato = get_data_int(get_string(buf, "magneto"));

            self.watertemp = get_data_float(get_string(buf, "water temp"));

            let mut tmp_thrust: f64 = 0.0;

            let mut total_engine_num: i32 = 0;
            for i in 0..MAX_ENG_NUM {
                // System.out.println(engineType);
                self.throttles[i] = get_data_int(get_string(buf, &format!("throttle {}", i + 1)));
                self.power[i] = get_data_float(get_string(buf, &format!("power {}", i + 1)));

                self.thrust[i] = get_data_int(get_string(buf, &format!("thrust {}", i + 1)));
                self.pitch[i] = get_data_float(get_string(buf, &format!("pitch {}", i + 1)));

                // PORT: Java `efficiency[i] = getDataInt(...)` — int 拓宽进 double 数组元素
                self.efficiency[i] = get_data_int(get_string(buf, &format!("efficiency {}", i + 1))) as f64;

                if self.thrust[i] == I_INVALID {
                    break;
                }

                // PORT: Java `tmpThrust += thrust[i]` — int 累加进 double
                tmp_thrust += self.thrust[i] as f64;
                total_engine_num += 1;
            }
            self.engine_num = total_engine_num;
            self.total_thr = tmp_thrust;
        } else {
            self.flag = false;
        }
        0
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
