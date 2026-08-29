//! 对应 Java: `src/ui/model/TelemetrySource.java` (一比一翻译)
//!
//! PORT: Java 接口 (60+ 原始类型 getter, 零分配契约) → trait;
//! 实现方 (Service) 后续批次落地。getFuelTimeMili 返回 Java long → i64。

/// Interface for providing raw telemetry data without object allocation.
/// This allows UI components to pull data directly as primitives.
pub trait TelemetrySource {
    // Flight Data
    fn get_ias(&self) -> f64 {
        0.0
    }

    fn get_tas(&self) -> f64 {
        0.0
    }

    fn get_mach(&self) -> f64 {
        0.0
    }

    fn get_aoa(&self) -> f64 {
        0.0
    }

    fn get_aos(&self) -> f64 {
        0.0
    }

    fn get_ny(&self) -> f64 {
        0.0
    } // G-Force

    fn get_vario(&self) -> f64 {
        0.0
    } // Climb Rate

    // Altitude & Position
    fn get_altitude(&self) -> f64 {
        0.0
    }

    fn get_radio_altitude(&self) -> f64 {
        0.0
    }

    fn is_radio_altitude_valid(&self) -> bool {
        false
    }

    fn get_compass(&self) -> f64 {
        0.0
    }

    // Performance
    fn get_sep(&self) -> f64 {
        0.0
    }

    fn get_acceleration(&self) -> f64 {
        0.0
    }

    fn get_turn_rate(&self) -> f64 {
        0.0
    }

    fn get_turn_radius(&self) -> f64 {
        0.0
    }

    /// 判断回转半径是否有效（<= 9999m）
    /// 回转半径过大时（如直飞或缓慢转弯）返回 false，隐藏该数据行
    fn is_turn_radius_valid(&self) -> bool {
        false
    }

    fn get_roll_rate(&self) -> f64 {
        0.0
    } // Wx

    fn get_energy_jkg(&self) -> f64 {
        0.0
    } // Specific Energy

    // Aircraft State
    fn get_mass_fuel(&self) -> f64 {
        0.0
    }

    /// Get total aircraft weight (nofuelweight + current fuel).
    /// @return Total weight in kg, or 0 if FM data unavailable
    fn get_total_weight(&self) -> f64 {
        0.0
    }

    fn get_fuel_time_mili(&self) -> i64 {
        0
    }

    fn get_throttle(&self) -> f64 {
        0.0
    }

    fn get_rpm(&self) -> f64 {
        0.0
    }

    fn get_manifold_pressure(&self) -> f64 {
        0.0
    }

    fn get_water_temp(&self) -> f64 {
        0.0
    }

    fn get_oil_temp(&self) -> f64 {
        0.0
    }

    fn get_pitch(&self) -> f64 {
        0.0
    }

    fn get_eff_hp(&self) -> f64 {
        0.0
    }

    fn get_thrust(&self) -> f64 {
        0.0
    }

    fn get_horse_power(&self) -> f64 {
        0.0
    }

    fn get_engine_response(&self) -> f64 {
        0.0
    }

    fn get_prop_efficiency(&self) -> f64 {
        0.0
    }

    fn get_wep_kg(&self) -> f64 {
        0.0
    }

    fn get_wep_time(&self) -> f64 {
        0.0
    }

    fn get_heat_tolerance(&self) -> f64 {
        0.0
    }

    fn get_power_percent(&self) -> f64 {
        0.0
    }

    fn get_manifold_pressure_pounds(&self) -> f64 {
        0.0
    } // Imperial

    fn get_manifold_pressure_inch_hg(&self) -> f64 {
        0.0
    } // Imperial

    /// Get manifold pressure display value (Ata for metric, psi for imperial).
    fn get_manifold_pressure_display(&self) -> f64 {
        0.0
    }

    /// Get manifold pressure display unit.
    /// Returns "Ata" for metric, "P/XX.X''" (with live inHg) for imperial.
    // PORT: 接口级 "without object allocation" 注释对 String 返回方法本就不成立 ——
    /// Java 原接口此处即返回 String (实现走 String.format), 每调用分配继承自 Java,
    /// 非本译引入的偏差。
    fn get_manifold_pressure_display_unit(&self) -> String {
        "Ata".into()
    }

    /// Get manifold pressure display precision.
    /// Returns 2 for metric (Ata), 1 for imperial (psi).
    fn get_manifold_pressure_display_precision(&self) -> i32 {
        2
    }

    // Engine Control
    fn get_unknown_mixture(&self) -> f64 {
        0.0
    } // For mixture state

    fn get_radiator(&self) -> f64 {
        0.0
    }

    fn get_compressor_stage(&self) -> f64 {
        0.0
    }

    fn get_fuel_percent(&self) -> f64 {
        0.0
    }

    fn get_rpm_throttle(&self) -> f64 {
        0.0
    }

    // Component State (0.0 - 1.0 or percent)
    fn get_gear(&self) -> f64 {
        0.0
    }

    fn get_flaps(&self) -> f64 {
        0.0
    }

    fn get_airbrake(&self) -> f64 {
        0.0
    }

    fn get_aileron(&self) -> f64 {
        0.0
    }

    fn get_elevator(&self) -> f64 {
        0.0
    }

    fn get_rudder(&self) -> f64 {
        0.0
    }

    fn get_wing_sweep(&self) -> f64 {
        0.0
    }

    fn is_wing_sweep_valid(&self) -> bool {
        false
    }

    // Speed Indicator & Limits
    fn get_speed_limit_ratio(&self) -> f64 {
        0.0
    }

    fn get_aileron_lock_ratio(&self) -> f64 {
        0.0
    }

    fn get_rudder_lock_ratio(&self) -> f64 {
        0.0
    }

    fn get_unit_mach_limit_ratio(&self) -> f64 {
        0.0
    }

    fn get_stall_speed(&self) -> f64 {
        0.0
    }

    fn is_imperial(&self) -> bool {
        false
    }

    // Attitude Indicator Data
    /// Get aviahorizon pitch (degrees).
    /// Used by AttitudeOverlay for artificial horizon display.
    fn get_aviahorizon_pitch(&self) -> f64 {
        0.0
    }

    /// Get aviahorizon roll (degrees).
    /// Used by AttitudeOverlay for artificial horizon rotation.
    fn get_aviahorizon_roll(&self) -> f64 {
        0.0
    }

    // === 引擎类型与飞机特性判断（用于 :visible-when 表达式）===

    /// 判断是否为喷气发动机（包括涡轮喷气、涡轮风扇）
    /// 需要等待引擎类型检测完成（约5秒）才能返回准确值
    /// @return true 如果是喷气机，false 如果是活塞/涡桨或未确定
    fn is_jet_engine(&self) -> bool {
        false
    }

    /// 判断是否为螺旋桨发动机（活塞或涡桨）
    /// 需要等待引擎类型检测完成（约5秒）才能返回准确值
    /// @return true 如果是活塞机或涡桨机，false 如果是喷气机或未确定
    fn is_prop_engine(&self) -> bool {
        false
    }

    /// 判断是否为活塞发动机（不包括涡桨）
    /// 需要等待引擎类型检测完成（约5秒）才能返回准确值
    /// @return true 如果是活塞机，false 如果是涡桨/喷气机或未确定
    fn is_piston_engine(&self) -> bool {
        false
    }

    /// 判断是否为涡轮螺旋桨发动机
    /// 需要等待引擎类型检测完成（约5秒）才能返回准确值
    /// @return true 如果是涡桨机，false 如果是活塞/喷气机或未确定
    fn is_turboprop_engine(&self) -> bool {
        false
    }

    /// 判断引擎类型检测是否完成
    /// 游戏启动后约5秒完成检测
    /// @return true 如果检测完成，false 如果仍在检测中
    fn is_engine_check_done(&self) -> bool {
        false
    }

    /// 判断飞机是否有加力系统（WEP/水喷射/氧化亚氮）
    /// 依赖于 FM 数据的加载
    /// @return true 如果有加力系统，false 如果没有或 FM 不可用
    fn has_wep(&self) -> bool {
        false
    }

    // === 火箭助推器 (Issue #52) ===

    /// 获取火箭助推器当前剩余燃料质量 (kg)
    /// 无助推器时返回 0
    fn get_booster_fuel_kg(&self) -> f64 {
        0.0
    }

    /// 获取火箭助推器剩余燃料百分比 (0-100)
    /// 计算公式: 100 * mfuel_1 / mfuel0_1
    /// 无助推器时返回 0
    fn get_booster_fuel_percent(&self) -> f64 {
        0.0
    }

    /// 判断飞机是否有火箭助推器系统
    /// 通过检查 API 返回的 Mfuel 1 和 Mfuel0 1 是否有效（> 0）
    /// @return true 如果有助推器系统，false 如果没有
    fn has_booster(&self) -> bool {
        false
    }

    /// /state 的原始过载 (state.ny 直通, 无 Java getter 对应 — W2 公式供值;
    /// 注意 get_ny 是派生量 an/g, an 被公式接管后二者语义分离)
    fn get_ny_raw(&self) -> f64 {
        0.0
    }

    /// /indicators 的校正速度 (座舱仪表 speed; 无 Java TelemetrySource 对应,
    /// W2 Deriver 消解为公式供值 — Deriver 独占消费的原始直通, F_INVALID 哨兵)
    fn get_indic_speed(&self) -> f64 {
        -65535.0
    }

    /// 公式系统取值 (无 Java 对应, doc/formula_system_design.md §8):
    /// 按公式名查最近一帧求值结果; 默认 None = 实现方未接公式系统。
    /// NaN (invalid/缺数据) 返回 None — 上层走 na/hide-when-zero 降级。
    fn get_formula_value(&self, _name: &str) -> Option<f64> {
        None
    }

    /// W6 统一取值桥: 快照变量/会话量/公式值 按名字统一取
    /// (ServiceData 实现 = FormulaView; target_value 的求值面)
    fn var_value(&self, _name: &str) -> Option<f64> {
        None
    }
}

#[cfg(test)]
mod tests;
