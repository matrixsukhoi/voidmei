//! 派生量计算: Service.java 公式逐行搬运
//! 调用序与 Java 主循环一致: update_speed → update_climb_rate → update_turn → update_sep

use super::json::{StateRaw, IndicatorsRaw, F_INVALID};
use vm_core::G;

use vm_core::calc_helper::SimpleMovingAverage as Sma;

/// 16 个 getter 的最终值 (FlightInfoOverlay 显示用)
#[derive(Debug, Clone, Copy, Default)]
pub struct FlightValues {
    pub ias: f64,
    pub tas: f64,
    pub mach: f64,
    pub compass: f64,
    pub altitude: f64,
    pub vario: f64,
    pub sep: f64,
    pub acceleration: f64,
    pub roll_rate: f64,
    pub ny: f64,
    pub turn_rate: f64,
    pub turn_radius: f64,
    pub aoa: f64,
    pub aos: f64,
    /// 已 ×100 (cfg 表达式 getWingSweep * 100)
    pub wing_sweep: f64,
    pub radio_altitude: f64,
}

pub struct Deriver {
    // updateSpeed 状态 (Service L840-872)
    speedv: f64,
    speedvp: f64,
    iastotascoff: f64,
    calc_speed_sma: Sma,
    // updateTurn 状态 (L788-838)
    an: f64,
    turn_rds: f64,
    turn_rate: f64,
    turnrds_sma: Sma,
    // updateSEP 状态 (L986-1028)
    diffspeed: f64,
    sep: f64,
    diff_speed_sma: Sma,
    sep_sma: Sma,
}

impl Deriver {
    /// SMA 窗口 = 1000/interval_ms (Java L1587-1591: n = 1000/freq, freq=serviceLoopIntervalMs)
    pub fn new(interval_ms: u64) -> Self {
        let n = (1000 / interval_ms.max(1)) as usize;
        Deriver {
            speedv: 0.0,
            speedvp: 0.0,
            iastotascoff: 1.0,
            calc_speed_sma: Sma::new(n),
            an: 0.0,
            turn_rds: 0.0,
            turn_rate: 0.0,
            turnrds_sma: Sma::new(n),
            diffspeed: 0.0,
            sep: 0.0,
            diff_speed_sma: Sma::new(n),
            sep_sma: Sma::new(n),
        }
    }

    /// 一轮完整计算 (顺序对齐 Java 主循环)
    pub fn step(&mut self, s: &StateRaw, i: &IndicatorsRaw, interval_ms: f64) -> FlightValues {
        // --- updateSpeed (L840-872) ---
        self.speedvp = self.speedv;
        let tas = s.tas;
        let tspeedv = if i.speed != F_INVALID { i.speed } else { s.ias / 3.6 };
        if tspeedv != 0.0 {
            self.iastotascoff = self.calc_speed_sma.add_new_data(tas / (tspeedv * 3.6));
        }
        self.speedv = tspeedv * self.iastotascoff;

        // --- updateClimbRate (L777-786) ---
        let n_vy = if i.vario != F_INVALID { i.vario } else { s.vy };

        // --- updateTurn (L788-838) ---
        if i.aviahorizon_roll != F_INVALID && i.aviahorizon_pitch != F_INVALID {
            // An = g*sqrt(Ny² + 1 - 2Ny·cos(roll)·cos(pitch+AoA))
            let roll = i.aviahorizon_roll.to_radians();
            let pitch_aoa = (i.aviahorizon_pitch + s.aoa).to_radians();
            let inner = s.ny * s.ny + 1.0
                - 2.0 * s.ny * roll.cos() * pitch_aoa.cos();
            self.an = G * inner.max(0.0).sqrt();
        } else {
            self.an = G * s.ny;
        }
        if self.an != 0.0 {
            let sum = self.speedvp + self.speedv;
            self.turn_rds = self.turnrds_sma.add_new_data(sum * sum / (4.0 * self.an));
            self.turn_rate = (self.an / self.turn_rds).max(0.0).sqrt().to_degrees();
        }

        // --- updateSEP (L986-1028) ---
        self.diffspeed = self.diff_speed_sma.add_new_data(self.speedv - self.speedvp);
        let acceleration = self.diffspeed * 1000.0 / interval_ms;
        let sum = self.speedv + self.speedvp;
        self.sep = self.sep_sma.add_new_data(
            (sum * (self.speedv - self.speedvp) * 1000.0) / (2.0 * interval_ms * G) + n_vy,
        );

        // --- mach (L1213-1215): 手动大气模型 ---
        let ias_per_mach = 3.6
            * (1.4 / 1.225 * 101325.0
                * (1.0 - 0.0000225577 * s.height_m).powf(5.25588))
            .sqrt();
        let mach = if ias_per_mach != 0.0 { s.ias / ias_per_mach } else { 0.0 };

        // --- 直通量 ---
        let radio_altitude = if i.radio_altitude == F_INVALID {
            s.height_m // 无效 → 用气压高度 (Java L761-763)
        } else {
            i.radio_altitude
        };
        let wing_sweep = if i.wsweep == F_INVALID { 0.0 } else { i.wsweep * 100.0 };

        FlightValues {
            ias: s.ias,
            tas: s.tas,
            mach,
            compass: i.compass,
            altitude: s.height_m,
            vario: n_vy,
            sep: self.sep,
            acceleration,
            roll_rate: s.wx.abs(),
            ny: self.an / G, // getNy = An/g (L1901)
            turn_rate: self.turn_rate,
            turn_radius: self.turn_rds.abs(),
            aoa: s.aoa,
            aos: s.aos,
            wing_sweep,
            radio_altitude,
        }
    }
}

#[cfg(test)]
mod tests;
