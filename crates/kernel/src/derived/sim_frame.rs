//! 试驾场模拟帧 (流动预览数据): 编辑态把合成数据喂给通用页组件 —
//! 条件显隐/自动补位当场演出, 不靠想象 (设计: 数据在流不在停)。
//!
//! 纯时间函数 (无状态): SimFrame::new(场景, now_ms) → var_value 按短名合成;
//! 覆盖面 = 出厂列表页 (flight-info/power-info) 的 target 集合 + 常用
//! 引擎/襟翼量; 未知名 None (对位 live 期 NoSuchMethod 语义)。

use crate::formula::registry::FormulaView;

/// 模拟场景 (工具条"场景拨杆"的拨杆位)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimScenario {
    /// 正常飞行 (螺旋桨基线, 全量缓变)
    Normal,
    /// 喷气机 (喷气族有效, 螺旋桨族 None)
    Jet,
    /// 起落架放下 (gear 置位 — 条件显隐的演出位)
    GearDown,
    /// 无数据 (全 None — 条件塌缩形态的演出位)
    NoData,
}

impl SimScenario {
    /// 拨杆位名 (IPC 面; 未知名宽容退 Normal)
    pub fn parse(name: &str) -> Self {
        match name {
            "jet" => SimScenario::Jet,
            "gear" => SimScenario::GearDown,
            "nodata" => SimScenario::NoData,
            _ => SimScenario::Normal,
        }
    }
}

/// 模拟帧 (FormulaView 合成实现)
pub struct SimFrame {
    scenario: SimScenario,
    /// 秒 (值域缓变的时间基)
    t: f64,
}

impl SimFrame {
    pub fn new(scenario: SimScenario, now_ms: i64) -> Self {
        SimFrame {
            scenario,
            t: now_ms as f64 / 1000.0,
        }
    }

    /// 喷气场景位
    fn is_jet(&self) -> bool {
        matches!(self.scenario, SimScenario::Jet)
    }
}

impl FormulaView for SimFrame {
    fn var_value(&self, name: &str) -> Option<f64> {
        if matches!(self.scenario, SimScenario::NoData) {
            return None; // 无数据形态: 条件塌缩的演出位
        }
        let t = self.t;
        let gear_down = matches!(self.scenario, SimScenario::GearDown);
        let v = match name {
            // ---- 飞行信息 (flight-info target 集) ----
            "ias" => 500.0 + 40.0 * (t / 7.0).sin(),
            "tas" => 550.0 + 45.0 * (t / 7.0 + 0.3).sin(),
            "mach" => 0.45 + 0.06 * (t / 6.0).sin(),
            "compass" => (270.0 + 30.0 * (t / 20.0).sin() + 360.0) % 360.0,
            "altitude" => 3000.0 + 250.0 * (t / 13.0).sin(),
            "vario" => 15.0 * (t / 5.0).sin(),
            "acceleration" => 3.0 * (t / 4.0).sin(),
            "roll_rate" => 60.0 * (t / 2.5).sin(),
            "ny" => 1.0 + 0.6 * (t / 3.0).sin(),
            "turn_rate" => 12.0 * (t / 6.0).sin(),
            "turn_rds" => 800.0 + 100.0 * (t / 8.0).sin(),
            "aoa" => 8.0 + 4.0 * (t / 4.0).sin(),
            "aos" => 2.0 * (t / 4.0).sin(),
            "radio_altitude" => 2800.0 + 200.0 * (t / 13.0).sin(),
            // ---- 动力信息 (power-info target 集; 螺旋桨族喷气位 None) ----
            "horse_power" | "eff_hp" | "power_percent" => 85.0 + 10.0 * (t / 9.0).sin(),
            "thrust" => {
                if self.is_jet() {
                    8000.0 + 400.0 * (t / 9.0).sin()
                } else {
                    1800.0 + 150.0 * (t / 9.0).sin()
                }
            }
            "rpm" => {
                if self.is_jet() {
                    None
                } else {
                    Some(2700.0 + 150.0 * (t / 9.0).sin())
                }?
            }
            "prop_pitch" | "prop_efficiency" => {
                if self.is_jet() {
                    return None;
                }
                22.0 + 2.0 * (t / 9.0).sin()
            }
            "manifold_pressure_display" | "manifold_pressure" => 1.15 + 0.05 * (t / 9.0).sin(),
            "mass_fuel" => 900.0 - (t % 1800.0) * 0.4, // 油量缓降 (30 分钟见底)
            "total_weight" => 7000.0 - (t % 1800.0) * 0.4,
            "fuel_time_milix0_001" => 1800_000.0 - (t % 1800.0) * 1000.0,
            "wep_kg" => 120.0,
            "wep_time" => 300.0,
            "booster_fuel_kg" => 500.0 - (t % 600.0) * 0.8,
            "booster_fuel_percent" => 80.0 - (t % 600.0) * 0.1,
            "water_temp" => 80.0 + 12.0 * (t / 30.0).sin(),
            "oil_temp" => 85.0 + 10.0 * (t / 35.0).sin(),
            "heat_tolerance" => 120.0,
            "engine_response" => 0.9,
            // ---- 引擎仪表/襟翼/起落架 ----
            "throttle" => 0.7 + 0.1 * (t / 9.0).sin(),
            "pitch" | "mixture" | "radiator" | "compressor" => 0.6 + 0.2 * (t / 11.0).sin(),
            "flaps" => if (t / 40.0).sin() > 0.0 { 1.0 } else { 0.0 },
            "gear" => {
                if gear_down {
                    1.0
                } else {
                    0.0
                }
            }
            "airbrake" => if (t / 50.0).sin() > 0.5 { 1.0 } else { 0.0 },
            "is_imperial" => 0.0,
            // 未知名: None (live NoSuchMethod 同语义)
            _ => return None,
        };
        Some(v)
    }
}
