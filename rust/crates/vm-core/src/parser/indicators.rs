//! Indicators 的 Rust 移植 (src/parser/Indicators.java)
//! /indicators 座舱仪表遥测 JSON 的子串提取解析 (舵面/油门/引擎/燃油)。
//!
//! PORT: getString/getDataFloat 接已译 crate::string_helper;
//! Java `Service.nastring` 静态常量按 CLASSIFY 裁决内联为本模块常量。

use crate::string_helper::{get_data_float, get_string, F_INVALID};

/// 对应 Java `Service.nastring` (public static final String nastring = "-")
/// PORT: CLASSIFY 裁决 — 引用 Service 静态常量需内联, 不引入未翻译的 prog.Service
const NA_STRING: &str = "-";

/// /indicators 遥测快照。字段顺序与 Java 声明一致。
pub struct Indicators {
    pub valid: Option<String>,
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
    /// Java `private String army` — 保持私有 (测试经同模块子模块访问)
    army: Option<String>,
    /// 只有仪表盘上有mach仪表的飞机才有这个mach
    pub mach: f64,
}

impl Indicators {
    /// 对应 Java `new Indicators()`: 标量字段取 Java 默认值 (§2.10)
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
            army: None,
            mach: 0.0,
        }
    }

    pub fn init(&mut self) {
        //Application.debugPrint("indicator初始化了");
        self.valid = Some(NA_STRING.to_string());
        self.fuelnum = 0;
        self.fuel = [0.0; 5];
        self.flag = false;
        //		fuelpressure=false;
        self.mach = 0.0;
    }

    pub fn update(&mut self, buf: &str) {
        self.valid = get_string(buf, "valid").map(str::to_string);
        self.army = get_string(buf, "army").map(str::to_string);
        // 防御加固: army 字段缺失 (响应截断/畸形 JSON) 时 getString 返回 null,
        // 原代码 army.equals("tank") 会抛 NullPointerException; 字面量前置统一判空
        // PORT: getString 的字符串值含首尾引号 (string_helper 契约), 故 army=="tank"
        // 实际永不成立 (Java 同此, tank 过滤名存实亡) — 保真保留
        if self.valid.as_deref() == Some("true") && self.army.as_deref() != Some("tank") {
            self.flag = true;
            // 防御加固: type 字段缺失时 getString 返回 null, 原代码 .toUpperCase() 会 NPE,
            // 缺失时按空类型处理 (走 type.length()>0 的既有兜底分支)
            let type_raw = get_string(buf, "type");
            self.r#type = Some(match type_raw {
                None => String::new(),
                // PORT: Java toUpperCase() 用默认 Locale (tr 语料 'i'→'İ' 差异),
                // Rust to_uppercase 与 locale 无关; 域内机型名 ASCII, 无行为差
                Some(t) => t.to_uppercase(),
            });

            // 防御加固: getString 提取的 type 值去首尾字符 (原为去空格+引号),
            // 若值只有 0/1 个字符 (畸形响应如 "type": 0), substring(1, -1) 会越界,
            // 长度不足 2 时保持原值跳过去壳, 正常机型名 (>=2 字符) 行为不变
            // PORT: substring(1, len-1) 与 length() 按 UTF-16 码元计; 此处按字符
            // (chars) 计 — BMP 域等价 (引号 ASCII, 切点必在边界; 域内机型名 ASCII)
            if self.r#type.as_deref().is_some_and(|t| t.chars().count() > 1) {
                let t = self.r#type.take().unwrap();
                let n = t.chars().count();
                let inner: String = t.chars().skip(1).take(n - 2).collect();
                if inner.chars().count() > 9 {
                    // PORT: substring(0, 8) — 前 8 个 UTF-16 码元 ≈ BMP 前 8 字符
                    self.stype = Some(inner.chars().take(8).collect());
                } else {
                    self.stype = Some(inner.clone());
                }
                self.r#type = Some(inner);
            }

            self.speed = get_data_float(get_string(buf, "speed"));
            self.pedals = get_data_float(get_string(buf, "pedals"));
            self.stick_elevator = get_data_float(get_string(buf, "stick_elevator"));
            self.stick_ailerons = get_data_float(get_string(buf, "stick_ailerons"));
            self.altitude_hour = get_data_float(get_string(buf, "altitude_hour"));
            self.altitude_min = get_data_float(get_string(buf, "altitude_min"));
            self.altitude_10k = get_data_float(get_string(buf, "altitude_10k"));
            self.bank = get_data_float(get_string(buf, "bank"));
            self.turn = get_data_float(get_string(buf, "turn"));
            self.compass = get_data_float(get_string(buf, "compass"));
            self.clock_hour = get_data_float(get_string(buf, "clock_hour"));
            self.clock_min = get_data_float(get_string(buf, "clock_min"));
            self.clock_sec = get_data_float(get_string(buf, "clock_sec"));
            self.manifold_pressure = get_data_float(get_string(buf, "manifold_pressure"));
            self.rpm = get_data_float(get_string(buf, "rpm"));
            self.wsweep_indicator = get_data_float(get_string(buf, "wing_sweep_indicator"));
            //			Application.debugPrint(wsweep_indicator);
            self.oil_pressure = get_data_float(get_string(buf, "oil_pressure"));
            //			water_temperature=StringHelper.getDatadouble(StringHelper.getString(buf, "water_temperature"));
            self.engine_temperature = get_data_float(get_string(buf, "head_temperature"));
            self.mixture = get_data_float(get_string(buf, "mixture"));

            // 防止读到油压
            self.fuel[0] = get_data_float(get_string(buf, "\"fuel\""));
            self.fuelnum = 1;
            for i in 1..5 {
                self.fuel[i] = get_data_float(get_string(buf, &format!("fuel{}", i)));
                if self.fuel[i] == F_INVALID {
                    self.fuel[i] = 0.0;
                } else {
                    self.fuelnum += 1;
                }
            }
            //			fuel[0]=StringHelper.getDatadouble(StringHelper.getString(buf, "fuel1"));
            //			if (fuel[0] == -65535){
            //				fuel[0] = StringHelper.getDatadouble(StringHelper.getString(buf, "fuel"));
            //				if
            //			}
            self.aviahorizon_pitch = get_data_float(get_string(buf, "aviahorizon_pitch"));
            self.aviahorizon_roll = get_data_float(get_string(buf, "aviahorizon_roll"));
            self.radio_altitude = get_data_float(get_string(buf, "radio_altitude"));
            self.oil_temp = get_data_float(get_string(buf, "oil_temperature"));
            self.water_temp = get_data_float(get_string(buf, "water_temperature"));
            if self.fuel[0] == F_INVALID {
                self.fuel[0] = 0.0;
                //				fuel[0]=StringHelper.getDatadouble(StringHelper.getString(buf, "fuel_pressure"))*10;
                //				fuelpressure=true;
            }
            //			else{
            //				fuelpressure=false;
            //			}
            //			fuelpressure=false;
            //			fuel[1]=StringHelper.getDataFloat(StringHelper.getString(buf, "fuel2"));
            //			fuel[2]=StringHelper.getDataFloat(StringHelper.getString(buf, "fuel3"));
            //			fuel[3]=StringHelper.getDataFloat(StringHelper.getString(buf, "fuel4"));
            //			if(fuelnum==0){
            //				if(fuel[0]!=-65535)fuelnum=fuelnum+1;
            //				if(fuel[1]!=-65535)fuelnum=fuelnum+1;
            //				if(fuel[2]!=-65535)fuelnum=fuelnum+1;
            //				if(fuel[3]!=-65535)fuelnum=fuelnum+1;
            //
            //			}

            self.fuel_pressure = get_data_float(get_string(buf, "fuel_pressure"));
            self.oxygen = get_data_float(get_string(buf, "oxygen"));
            self.gears_lamp = get_data_float(get_string(buf, "gears_lamp"));
            self.flaps = get_data_float(get_string(buf, "flaps"));
            self.vario = get_data_float(get_string(buf, "vario"));
            self.trimmer = get_data_float(get_string(buf, "trimmer"));
            self.throttle = get_data_float(get_string(buf, "throttle"));
            self.weapon1 = get_data_float(get_string(buf, "weapon1"));
            self.weapon2 = get_data_float(get_string(buf, "weapon2"));
            self.weapon3 = get_data_float(get_string(buf, "weapon3"));
            self.prop_pitch_hour = get_data_float(get_string(buf, "prop_pitch_hour"));
            self.prop_pitch_min = get_data_float(get_string(buf, "prop_pitch_min"));
            self.ammo_counter1 = get_data_float(get_string(buf, "ammo_counter1"));
            self.ammo_counter2 = get_data_float(get_string(buf, "ammo_counter2"));
            self.ammo_counter3 = get_data_float(get_string(buf, "ammo_counter3"));
            self.mach = get_data_float(get_string(buf, "mach"));
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
mod tests {
    use super::*;

    /// 真机抓取的 /indicators 快照 (bf109f4, mock 8111 线上格式)。断言值 = Java 8 oracle 实测。
    const IND_MOCK: &str = "{\"valid\": true,\"army\": \"air\",\"type\": \"bf-109f-4\",\"speed\": 131.007797,\"pedals1\": -0.465775,\"pedals2\": -0.465775,\"pedals3\": -0.465775,\"pedals4\": -0.465775,\"stick_elevator\": 0.187344,\"stick_elevator1\": 0.187344,\"stick_ailerons\": -0.477257,\"vario\": -7.342558,\"altitude_hour\": 151.084213,\"altitude_min\": 151.084213,\"altitude_10k\": 151.084213,\"aviahorizon_roll\": -40.553505,\"aviahorizon_pitch\": 0.632352,\"bank\": 8.0,\"turn\": 0.131359,\"compass\": 164.09729,\"compass1\": 164.09729,\"clock_hour\": 2.7,\"clock_min\": 42.0,\"clock_sec\": 6.0,\"manifold_pressure\": 2.243279,\"rpm\": 3001.457031,\"oil_pressure\": 1.0,\"oil_temperature\": 90.373169,\"water_temperature\": 121.324493,\"mixture\": 0.833333,\"carb_temperature\": 0.0,\"fuel1\": 196.846497,\"fuel2\": 0.0,\"fuel_pressure\": 11.0,\"gears\": 0.0,\"gear_lamp_down\": 0.0,\"gear_lamp_up\": 0.0,\"gear_lamp_off\": 0.0,\"flaps\": 0.0,\"throttle\": 1.1,\"weapon1\": 0.0,\"prop_pitch\": 1.0}";

    #[test]
    fn update_full_snapshot_matches_java_oracle() {
        let mut id = Indicators::new();
        id.init();
        id.update(IND_MOCK);
        assert_eq!(id.valid.as_deref(), Some("true"));
        // type 去引号 + toUpperCase; 9 字符不 >9 → stype 同 type (oracle 实测)
        assert_eq!(id.r#type.as_deref(), Some("BF-109F-4"));
        assert_eq!(id.stype.as_deref(), Some("BF-109F-4"));
        assert!(id.flag);
        // "pedals" 键搜索命中 "pedals1" 前缀 (getString 首次子串匹配, Java 同此)
        assert_eq!(id.speed, 131.007_8_f32 as f64);
        assert_eq!(id.pedals, -0.465775f32 as f64);
        assert_eq!(id.stick_elevator, 0.187344f32 as f64);
        assert_eq!(id.stick_ailerons, -0.477257f32 as f64);
        assert_eq!(id.altitude_hour, 151.084_21_f32 as f64);
        assert_eq!(id.bank, 8.0);
        assert_eq!(id.turn, 0.131359f32 as f64);
        assert_eq!(id.compass, 164.09729f32 as f64);
        assert_eq!(id.clock_hour, 2.7f32 as f64);
        assert_eq!(id.clock_min, 42.0);
        assert_eq!(id.clock_sec, 6.0);
        assert_eq!(id.manifold_pressure, 2.243279f32 as f64);
        assert_eq!(id.rpm, 3_001.457_f32 as f64);
        assert_eq!(id.oil_pressure, 1.0);
        // water_temperature 字段从未赋值 (赋值行已注释) → 恒 0.0; head_temperature 缺失 → 哨兵
        assert_eq!(id.water_temperature, 0.0);
        assert_eq!(id.engine_temperature, -65535.0);
        assert_eq!(id.mixture, 0.833333f32 as f64);
        // "\"fuel\"" 带引号键在 payload 中不存在 → fuel[0] 哨兵 → 归 0 (oracle 实测)
        assert_eq!(id.fuel[0], 0.0);
        assert_eq!(id.fuel[1], 196.846_5_f32 as f64);
        assert_eq!(id.fuel[2], 0.0); // fuel2=0.0 有效值 (≠哨兵), 不归零处理前也计入
        assert_eq!(id.fuel[3], 0.0); // 缺失 → 哨兵 → 0
        assert_eq!(id.fuelnum, 3); // 1 (基准) + fuel1 + fuel2
        assert_eq!(id.fuel_pressure, 11.0);
        assert_eq!(id.oxygen, -65535.0);
        assert_eq!(id.gears_lamp, -65535.0);
        assert_eq!(id.flaps, 0.0);
        assert_eq!(id.trimmer, -65535.0);
        assert_eq!(id.throttle, 1.1f32 as f64);
        assert_eq!(id.weapon1, 0.0);
        assert_eq!(id.weapon2, -65535.0);
        assert_eq!(id.weapon3, -65535.0);
        assert_eq!(id.vario, -7.342558f32 as f64);
        assert_eq!(id.aviahorizon_pitch, 0.632352f32 as f64);
        assert_eq!(id.aviahorizon_roll, -40.553505f32 as f64);
        assert_eq!(id.wsweep_indicator, -65535.0);
        assert_eq!(id.radio_altitude, -65535.0);
        assert_eq!(id.mach, -65535.0);
        assert_eq!(id.oil_temp, 90.373_17_f32 as f64);
        assert_eq!(id.water_temp, 121.324_49_f32 as f64);
    }

    #[test]
    fn init_sets_nastring_and_defaults() {
        let mut id = Indicators::new();
        id.init();
        assert_eq!(id.valid.as_deref(), Some("-")); // Service.nastring 内联
        assert_eq!(id.fuelnum, 0);
        assert_eq!(id.fuel, [0.0; 5]);
        assert!(!id.flag);
        assert_eq!(id.mach, 0.0);
    }

    #[test]
    fn update_tank_army_still_parsed_as_aircraft() {
        // getString 返回值含引号 → army == "\"tank\"" ≠ "tank" → 不走 else 分支
        // (Java 同此, oracle 实测 type=TU-4) — tank 过滤名存实亡, 保真保留
        let mut id = Indicators::new();
        id.init();
        id.update("{\"valid\": true, \"army\": \"tank\", \"type\": \"tu-4\"}");
        assert!(id.flag);
        assert_eq!(id.r#type.as_deref(), Some("TU-4"));
    }

    #[test]
    fn update_missing_type_gives_empty_type_null_stype() {
        // 防御分支: type 缺失 → "" ; stype 不赋值保持 null (oracle 实测 stype=null)
        let mut id = Indicators::new();
        id.init();
        id.update("{\"valid\": true, \"speed\": 55.5}");
        assert!(id.flag);
        assert_eq!(id.r#type.as_deref(), Some(""));
        assert!(id.stype.is_none());
        assert_eq!(id.speed, 55.5);
    }

    #[test]
    fn update_long_type_truncates_stype_to_8() {
        let mut id = Indicators::new();
        id.init();
        id.update("{\"valid\": true, \"type\": \"aaaaaaaaaaaaaaaaaaaa\"}");
        assert_eq!(id.r#type.as_deref(), Some("AAAAAAAAAAAAAAAAAAAA"));
        assert_eq!(id.stype.as_deref(), Some("AAAAAAAA")); // >9 → 前 8 字符
    }

    #[test]
    fn update_short_type_keeps_stype_equal() {
        // 去壳后单字符: type="A" (带引号 3 字符 >1 → 进去壳分支), ≤9 → stype=type
        let mut id = Indicators::new();
        id.init();
        id.update("{\"valid\": true, \"type\": \"a\"}");
        assert_eq!(id.r#type.as_deref(), Some("A"));
        assert_eq!(id.stype.as_deref(), Some("A"));
    }

    #[test]
    fn update_invalid_gives_no_cockpit() {
        let mut id = Indicators::new();
        id.init();
        id.update("{\"valid\": false}");
        assert_eq!(id.r#type.as_deref(), Some("No Cockpit"));
        assert_eq!(id.stype.as_deref(), Some("NoCockpit"));
        assert!(!id.flag);
        assert_eq!(id.speed, 0.0);
    }

    #[test]
    fn update_cjk_type_bmp_equivalent() {
        // CJK 机型名 (BMP): 去引号去壳按字符, 与 Java UTF-16 码元语义等价
        let mut id = Indicators::new();
        id.init();
        id.update("{\"valid\": true, \"type\": \"歼-20\"}");
        assert_eq!(id.r#type.as_deref(), Some("歼-20"));
        assert_eq!(id.stype.as_deref(), Some("歼-20"));
    }

    #[test]
    fn update_fuel_sentinel_zeroing() {
        // fuel1 哨兵 → 0 且不计数; fuel2 有效 (含 0.0) → 计数
        let mut id = Indicators::new();
        id.init();
        id.update("{\"valid\": true, \"fuel1\": 12.5, \"fuel2\": 0.0, \"fuel3\": 7.5}");
        assert_eq!(id.fuel[1], 12.5);
        assert_eq!(id.fuel[2], 0.0);
        assert_eq!(id.fuel[3], 7.5);
        assert_eq!(id.fuel[4], 0.0); // 缺失 → 哨兵 → 0
        // 基准 1 + fuel1/fuel2/fuel3 各计数 +1, fuel4 缺失不计数
        assert_eq!(id.fuelnum, 4);
    }
}
