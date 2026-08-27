//! JSON 宽容提取: 对齐 Java StringHelper 子串提取的语义
//! (缺失/非数值 → 哨兵 -65535; 顶层非对象/解析失败 → None 即 valid=false)

use serde_json::Value;

/// Java StringHelper.fInvalid
pub const F_INVALID: f64 = -65535.0;

/// 宽容取数: 缺失/非数值返回哨兵
pub fn num(v: &Value, key: &str) -> f64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(F_INVALID),
        _ => F_INVALID,
    }
}

/// /state 端点 (对应 Java State.java)
pub fn parse_state(raw: &[u8]) -> Option<StateRaw> {
    let v: Value = serde_json::from_slice(raw).ok()?;
    if !v.is_object() {
        return None;
    }
    Some(StateRaw {
        ias: num(&v, "IAS, km/h"),
        tas: num(&v, "TAS, km/h"),
        height_m: num(&v, "H, m"),
        vy: num(&v, "Vy, m/s"),
        wx: num(&v, "Wx, deg/s"),
        aoa: num(&v, "AoA, deg"),
        aos: num(&v, "AoS, deg"),
        ny: num(&v, "Ny"),
    })
}

/// /indicators 端点 (对应 Java Indicators.java, 仅 FlightInfo 所需字段)
pub fn parse_indicators(raw: &[u8]) -> Option<IndicatorsRaw> {
    let v: Value = serde_json::from_slice(raw).ok()?;
    if !v.is_object() {
        return None;
    }
    Some(IndicatorsRaw {
        valid: v.get("valid").and_then(|b| b.as_bool()).unwrap_or(false),
        speed: num(&v, "speed"),
        vario: num(&v, "vario"),
        aviahorizon_roll: num(&v, "aviahorizon_roll"),
        aviahorizon_pitch: num(&v, "aviahorizon_pitch"),
        compass: num(&v, "compass"),
        radio_altitude: num(&v, "radio_altitude"),
        wsweep: num(&v, "wsweep_indicator"),
    })
}

#[derive(Debug, Clone, Copy)]
pub struct StateRaw {
    pub ias: f64,
    pub tas: f64,
    pub height_m: f64,
    pub vy: f64,
    pub wx: f64,
    pub aoa: f64,
    pub aos: f64,
    pub ny: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct IndicatorsRaw {
    pub valid: bool,
    pub speed: f64,
    pub vario: f64,
    pub aviahorizon_roll: f64,
    pub aviahorizon_pitch: f64,
    pub compass: f64,
    pub radio_altitude: f64,
    pub wsweep: f64,
}
