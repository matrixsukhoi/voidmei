//! /indicators 座舱仪表遥测快照 (舵面/油门/引擎/燃油)。
//!
//! 波20 serde 化: 原 getString/getDataFloat 子串扫描 → serde_json::Value 全等键取数。
//! 键名对照真机快照 (script/mock_scenarios/snapshots/plane_p51d.json):
//! - pedals: 快照只有 pedals1~4 (无裸 pedals 键), 手写 needle "pedals" 是子串
//!   命中 pedals1 — serde 版显式映射 "pedals1" (值不变, 来源从巧合变明确);
//! - gears_lamp/prop_pitch_hour/prop_pitch_min: 手写时代就恒缺键 (真键是
//!   gear_lamp_down 族/prop_pitch), 全等不中 → 哨兵, 行为不变;
//! - fuel[0]: 快照无裸 fuel 键 → 恒哨兵归 0, fuelnum 基准恒 1 (行为不变)。
//!
//! 行为变更 (波20 裁决): valid 改真实 bool; type 加工链退役 "去引号" 步骤
//! (serde 给裸串); army=="tank" 死分支 (字符串值带引号永不等于 "tank") 随
//! army 字段一并删除 — army 仅解析从未消费。

use serde_json::Value;

use super::{v_f64, F_INVALID};

/// /indicators 遥测快照。字段顺序与 Java 声明一致。
#[derive(Clone)]
pub struct Indicators {
    /// JSON 真值即 bool; init 前为 None
    pub valid: Option<bool>,
    pub r#type: Option<String>,
    pub stype: Option<String>,
    pub flag: bool,
    //	public boolean fuelpressure;
    pub speed: f64,
    pub pedals: f64,
    pub stick_elevator: f64,
    pub stick_ailerons: f64,
    pub altitude_hour: f64,
    pub altitude_min: f64,
    pub altitude_10k: f64,
    pub bank: f64,
    pub turn: f64,
    pub compass: f64,
    pub clock_hour: f64,
    pub clock_min: f64,
    pub clock_sec: f64,
    pub manifold_pressure: f64,
    pub rpm: f64,
    pub oil_pressure: f64,
    /// Java 字段 water_temperature 在 update 中从未赋值 (赋值行已注释), 恒为 0.0
    pub water_temperature: f64,
    pub engine_temperature: f64,
    pub mixture: f64,
    /// PORT: Java `public double fuel[]` 裸声明, 未 init 的 update 在 `fuel[0]=..` 即
    /// NPE; Rust [f64;5] 在 new() 即有效, 静默写 0.0 域 — 生产路径 Service 恒先 init,
    /// 不可达偏差 (state.rs 引擎数组用空 Vec 对齐 panic, 二者策略异, 保留各自形状)
    pub fuel: [f64; 5],
    pub fuel_pressure: f64,
    pub oxygen: f64,
    pub gears_lamp: f64,
    pub flaps: f64,
    pub trimmer: f64,
    pub throttle: f64,
    pub weapon1: f64,
    pub weapon2: f64,
    pub weapon3: f64,
    pub prop_pitch_hour: f64,
    pub prop_pitch_min: f64,
    pub ammo_counter1: f64,
    pub ammo_counter2: f64,
    pub ammo_counter3: f64,
    pub oil_temp: f64,
    pub water_temp: f64,
    pub fuelnum: i32,
    pub vario: f64,
    pub aviahorizon_pitch: f64,
    pub aviahorizon_roll: f64,
    pub wsweep_indicator: f64,
    pub radio_altitude: f64,
    /// 只有仪表盘上有mach仪表的飞机才有这个mach
    pub mach: f64,
}

impl Indicators {
    /// 对应 Java `new Indicators()`: 标量字段取 Java 默认值
    pub fn new() -> Self {
        Indicators {
            valid: None,
            r#type: None,
            stype: None,
            flag: false,
            speed: 0.0,
            pedals: 0.0,
            stick_elevator: 0.0,
            stick_ailerons: 0.0,
            altitude_hour: 0.0,
            altitude_min: 0.0,
            altitude_10k: 0.0,
            bank: 0.0,
            turn: 0.0,
            compass: 0.0,
            clock_hour: 0.0,
            clock_min: 0.0,
            clock_sec: 0.0,
            manifold_pressure: 0.0,
            rpm: 0.0,
            oil_pressure: 0.0,
            water_temperature: 0.0,
            engine_temperature: 0.0,
            mixture: 0.0,
            fuel: [0.0; 5],
            fuel_pressure: 0.0,
            oxygen: 0.0,
            gears_lamp: 0.0,
            flaps: 0.0,
            trimmer: 0.0,
            throttle: 0.0,
            weapon1: 0.0,
            weapon2: 0.0,
            weapon3: 0.0,
            prop_pitch_hour: 0.0,
            prop_pitch_min: 0.0,
            ammo_counter1: 0.0,
            ammo_counter2: 0.0,
            ammo_counter3: 0.0,
            oil_temp: 0.0,
            water_temp: 0.0,
            fuelnum: 0,
            vario: 0.0,
            aviahorizon_pitch: 0.0,
            aviahorizon_roll: 0.0,
            wsweep_indicator: 0.0,
            radio_altitude: 0.0,
            mach: 0.0,
        }
    }

    pub fn init(&mut self) {
        self.valid = Some(false);
        self.fuelnum = 0;
        self.fuel = [0.0; 5];
        self.flag = false;
        //		fuelpressure=false;
        self.mach = 0.0;
    }

    pub fn update(&mut self, buf: &str) {
        // 畸形/空 JSON → Null, 全部取数走缺键分支 (等价手写时代 "找不到键")
        let v: Value = serde_json::from_str(buf).unwrap_or(Value::Null);
        self.valid = v.get("valid").and_then(Value::as_bool);
        if self.valid == Some(true) {
            self.flag = true;
            // type 加工链: to_uppercase (下游 DUMMY_PLANE/FM 识别依赖大写) + stype 截 8;
            // 手写时代的 "去首尾引号" 步骤退役 (serde 给裸串)。
            // 缺键 (畸形响应) → type="" 且 stype 不赋值保持 None (对齐手写时代缺失分支)
            match v.get("type").and_then(Value::as_str) {
                None => self.r#type = Some(String::new()),
                Some(t) => {
                    // PORT: toUpperCase() 默认 Locale (tr 语料 'i'→'İ' 差异) —
                    // Rust to_uppercase 与 locale 无关; 域内机型名 ASCII, 无行为差
                    let up = t.to_uppercase();
                    self.stype = Some(if up.chars().count() > 9 {
                        // PORT: substring(0, 8) — 前 8 个 UTF-16 码元 ≈ BMP 前 8 字符
                        up.chars().take(8).collect()
                    } else {
                        up.clone()
                    });
                    self.r#type = Some(up);
                }
            }

            self.speed = v_f64(&v, "speed");
            // 快照无裸 pedals 键 (只有 pedals1~4): 显式取 pedals1
            self.pedals = v_f64(&v, "pedals1");
            self.stick_elevator = v_f64(&v, "stick_elevator");
            self.stick_ailerons = v_f64(&v, "stick_ailerons");
            self.altitude_hour = v_f64(&v, "altitude_hour");
            self.altitude_min = v_f64(&v, "altitude_min");
            self.altitude_10k = v_f64(&v, "altitude_10k");
            self.bank = v_f64(&v, "bank");
            self.turn = v_f64(&v, "turn");
            self.compass = v_f64(&v, "compass");
            self.clock_hour = v_f64(&v, "clock_hour");
            self.clock_min = v_f64(&v, "clock_min");
            self.clock_sec = v_f64(&v, "clock_sec");
            self.manifold_pressure = v_f64(&v, "manifold_pressure");
            self.rpm = v_f64(&v, "rpm");
            self.wsweep_indicator = v_f64(&v, "wing_sweep_indicator");
            //			Application.debugPrint(wsweep_indicator);
            self.oil_pressure = v_f64(&v, "oil_pressure");
            //			water_temperature=StringHelper.getDatadouble(StringHelper.getString(buf, "water_temperature"));
            self.engine_temperature = v_f64(&v, "head_temperature");
            self.mixture = v_f64(&v, "mixture");

            // 防止读到油压: 裸 fuel 键真机不存在 → fuel[0] 恒哨兵 (后续归 0),
            // fuelnum 以 1 为基准 (手写时代同此, 语义保留)
            self.fuel[0] = v_f64(&v, "fuel");
            self.fuelnum = 1;
            for i in 1..5 {
                self.fuel[i] = v_f64(&v, &format!("fuel{}", i));
                if self.fuel[i] == F_INVALID {
                    self.fuel[i] = 0.0;
                } else {
                    self.fuelnum += 1;
                }
            }
            //			fuel[0]=StringHelper.getDatadouble(StringHelper.getString(buf, "fuel1"));
            //			if (fuel[0] == -65535){
            //				fuel[0]=StringHelper.getDatadouble(StringHelper.getString(buf, "fuel"));
            //				if
            //			}
            self.aviahorizon_pitch = v_f64(&v, "aviahorizon_pitch");
            self.aviahorizon_roll = v_f64(&v, "aviahorizon_roll");
            self.radio_altitude = v_f64(&v, "radio_altitude");
            self.oil_temp = v_f64(&v, "oil_temperature");
            self.water_temp = v_f64(&v, "water_temperature");
            if self.fuel[0] == F_INVALID {
                self.fuel[0] = 0.0;
                //				fuel[0]=StringHelper.getDatadouble(StringHelper.getString(buf, "fuel_pressure"))*10;
                //				fuelpressure=true;
            }
            //			else{
            //				fuelpressure=false;
            //			}
            //			fuelpressure=false;

            self.fuel_pressure = v_f64(&v, "fuel_pressure");
            self.oxygen = v_f64(&v, "oxygen");
            self.gears_lamp = v_f64(&v, "gears_lamp");
            self.flaps = v_f64(&v, "flaps");
            self.vario = v_f64(&v, "vario");
            self.trimmer = v_f64(&v, "trimmer");
            self.throttle = v_f64(&v, "throttle");
            self.weapon1 = v_f64(&v, "weapon1");
            self.weapon2 = v_f64(&v, "weapon2");
            self.weapon3 = v_f64(&v, "weapon3");
            self.prop_pitch_hour = v_f64(&v, "prop_pitch_hour");
            self.prop_pitch_min = v_f64(&v, "prop_pitch_min");
            self.ammo_counter1 = v_f64(&v, "ammo_counter1");
            self.ammo_counter2 = v_f64(&v, "ammo_counter2");
            self.ammo_counter3 = v_f64(&v, "ammo_counter3");
            self.mach = v_f64(&v, "mach");
        } else {
            self.r#type = Some("No Cockpit".to_string());
            self.stype = Some("NoCockpit".to_string());

            self.flag = false;
        }
    }
}

impl Default for Indicators {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
