//! /state 遥测快照 (速度/高度/G/舵面/引擎数组)。
//!
//! 波20 serde 化: 原 getString/getDataInt 子串扫描 → serde_json::Value 全等键取数。
//! 键名映射对照真机快照 (script/mock_scenarios/snapshots/plane_bf109f4.json):
//! 手写时代的 needle 是子串前缀 (如 "TAS" 实际命中键 `"TAS, km/h"`、
//! `"throttle"` 命中 `"throttle 1, %"`、`"\"M\""` 是键 `M` 的带引号消歧),
//! serde 全等匹配必须用完整真键名 — 详见 update 内的键名注释。
//!
//! 行为变更 (波20 裁决): 数值 f32 单精度拓宽退役 (f64 直读);
//! `valid` 改真实 bool (JSON 里就是 bool, 原字符串比较是手写解析的产物)。

use serde_json::Value;

use super::{v_f64, v_i32, I_INVALID};

/// 遥测侧每引擎数组容量 (throttles/power/pitch/thrust/efficiency)。
/// 2026-08 全量普查 (TestFMAllBoundaries): 真机 FM 引擎数极值 14 (b_66b, 含助推器块),
/// 原 8 会静默丢第 9+ 引擎数据; 上调至 16 (= Blkx 解析护栏, 见 Blkx.getload)。
/// 下游消费循环均按实际 engineNum 遍历 (数据驱动), 扩容不影响小引擎机型行为
pub const MAX_ENG_NUM: usize = 16;

/// /state 遥测快照。字段顺序与 Java 声明一致。
#[derive(Clone)]
pub struct State {
    /// JSON 真值即 bool; None = 响应缺 valid 键 (触发端口翻转信号)
    pub valid: Option<bool>,
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
    pub magneto: i32,
    /// 波21 定长化: 引擎五数组 [T; 16] (Copy, 帧克隆免堆分配)
    pub power: [f64; MAX_ENG_NUM],
    pub rpm: i32,
    pub manifoldpressure: f64,
    pub watertemp: f64,
    pub oiltemp: f64,
    pub mfuel: f64,
    pub mfuel_1: f64,
    pub mfuel0: f64,
    /// 助推器燃料总量 (kg)，无助推器时为 -65535
    pub mfuel0_1: f64,
    pub pitch: [f64; MAX_ENG_NUM],
    pub thrust: [i32; MAX_ENG_NUM],
    pub efficiency: [f64; MAX_ENG_NUM],
    pub airbrake: i32,
    pub total_thr: f64,
    pub throttles: [i32; MAX_ENG_NUM],
}

impl State {
    /// 对应 Java `new State()`: 标量字段取 Java 默认值, 引擎数组零初始化。
    /// (波21: Java new+init 两段式退役 — 定长数组随构造即有效,
    ///  "未 init 就 update panic" 的 NPE 保真随之消亡)
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
            magneto: 0,
            power: [0.0; MAX_ENG_NUM],
            rpm: 0,
            manifoldpressure: 0.0,
            watertemp: 0.0,
            oiltemp: 0.0,
            mfuel: 0.0,
            mfuel_1: 0.0,
            mfuel0: 0.0,
            mfuel0_1: 0.0,
            pitch: [0.0; MAX_ENG_NUM],
            thrust: [0; MAX_ENG_NUM],
            efficiency: [0.0; MAX_ENG_NUM],
            airbrake: 0,
            total_thr: 0.0,
            throttles: [0; MAX_ENG_NUM],
        }
    }

    /// 解析 /state JSON。返回值是调用方 (vm-data 轮询) 的协议信号:
    /// - `-1`: 响应缺 valid 键 (含空串/畸形 JSON) → 端口翻转;
    /// - `0`: 正常 (valid=true 填字段, valid=false 仅置 flag=false)。
    pub fn update(&mut self, buf: &str) -> i32 {
        // 畸形/空 JSON → Null, 全部取数走缺键分支 (等价手写时代 "找不到键")
        let v: Value = serde_json::from_str(buf).unwrap_or(Value::Null);
        let valid = v.get("valid").and_then(Value::as_bool);
        match valid {
            None => {
                self.valid = None;
                return -1;
            }
            Some(b) => self.valid = Some(b),
        }
        if valid == Some(true) {
            self.flag = true;

            // 舵面族 (快照真键带 ", %"/", deg" 单位后缀; 手写时代 needle 是裸前缀)
            self.aileron = v_i32(&v, "aileron, %");
            self.elevator = v_i32(&v, "elevator, %");
            self.rudder = v_i32(&v, "rudder, %");
            self.flaps = v_i32(&v, "flaps, %");
            self.airbrake = v_i32(&v, "airbrake, %");
            self.gear = v_i32(&v, "gear, %");
            // 速度/高度族 (needle "TAS"/"IAS"/"H, m" → 真键带单位)
            self.tas = v_i32(&v, "TAS, km/h");
            self.ias = v_i32(&v, "IAS, km/h");
            // needle "\"M\"" (带引号防命中 Mfuel) → 全等键 M
            self.m = v_f64(&v, "M");
            self.heightm = v_f64(&v, "H, m");
            self.aoa = v_f64(&v, "AoA, deg");
            self.aos = v_f64(&v, "AoS, deg");
            self.ny = v_f64(&v, "Ny");
            self.vy = v_f64(&v, "Vy, m/s");
            self.wx = v_f64(&v, "Wx, deg/s");
            // 油门族: State.throttle 是单值, 手写 needle "throttle" 首次命中
            // "throttle 1, %" 键名 → 真键即 1 号引擎油门
            self.throttle = v_i32(&v, "throttle 1, %");
            self.rpm_throttle = v_i32(&v, "RPM throttle 1, %");
            if self.rpm_throttle == I_INVALID {
                // 自动桨机型(如P-63)8111不返回该字段, 归一化为-1表示无桨距数据(与mixture约定一致),
                // 防止哨兵值-65535泄漏到UI层撑爆桨距竖条
                self.rpm_throttle = -1;
            }

            self.radiator = v_i32(&v, "radiator 1, %");

            self.rpm = v_i32(&v, "RPM 1");

            self.manifoldpressure = v_f64(&v, "manifold pressure 1, atm");

            self.mfuel = v_f64(&v, "Mfuel, kg");
            self.mfuel_1 = v_f64(&v, "Mfuel 1, kg");
            self.mfuel0 = v_f64(&v, "Mfuel0, kg");
            self.mfuel0_1 = v_f64(&v, "Mfuel0 1, kg"); // 助推器燃料总量

            self.oiltemp = v_f64(&v, "oil temp 1, C");

            self.mixture = v_i32(&v, "mixture 1, %");
            if self.mixture == I_INVALID {
                self.mixture = -1;
            }
            self.compressorstage = v_i32(&v, "compressor stage 1");
            if self.compressorstage == I_INVALID {
                self.compressorstage = 0;
            }

            self.magneto = v_i32(&v, "magneto 1");

            self.watertemp = v_f64(&v, "water temp 1, C");

            let mut tmp_thrust: f64 = 0.0;

            let mut total_engine_num: i32 = 0;
            for i in 0..MAX_ENG_NUM {
                // 引擎号键 1 起始; thrust 缺键即终止 (先写哨兵再 break — 数组
                // 产出契约保留, 下游按 engine_num 遍历不触达残留哨兵)
                self.throttles[i] = v_i32(&v, &format!("throttle {}, %", i + 1));
                self.power[i] = v_f64(&v, &format!("power {}, hp", i + 1));

                self.thrust[i] = v_i32(&v, &format!("thrust {}, kgs", i + 1));
                self.pitch[i] = v_f64(&v, &format!("pitch {}, deg", i + 1));

                // Java `efficiency[i] = getDataInt(...)` — int 拓宽进 double 数组元素
                self.efficiency[i] = v_i32(&v, &format!("efficiency {}, %", i + 1)) as f64;

                if self.thrust[i] == I_INVALID {
                    break;
                }

                // Java `tmpThrust += thrust[i]` — int 累加进 double
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
