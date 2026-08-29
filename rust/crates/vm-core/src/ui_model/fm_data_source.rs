//! 对应 Java: `src/ui/model/FMDataSource.java` (一比一翻译)
//!
//! PORT: Java 接口 → trait; Java 方法名 `getNoFlapsWing_CdMin` 族
//! (驼峰+下划线混排) → `get_no_flaps_wing_cd_min` (§5 camelCase→snake_case,
//! 下划线吞并进词边界); `getFlap0Speed` 数字黏附前词 → `get_flap0_speed`。

/// Interface for providing FM (Flight Model) data for display in overlays.
/// Designed for use with ReflectBinder for zero-GC data access.
///
/// <p>This interface abstracts Blkx data access, allowing overlays to bind
/// to getter methods via reflection without directly depending on Blkx.
pub trait FMDataSource {
    // ==================== Basic Info ====================

    /// Get FM file name and version string
    fn get_fm_version(&self) -> String;

    /// Get empty aircraft weight (kg)
    fn get_empty_weight(&self) -> f64;

    /// Get maximum fuel weight (kg)
    fn get_max_fuel_weight(&self) -> f64;

    // ==================== Speed Limits ====================

    /// Get critical speed (km/h) - compressibility limit
    fn get_critical_speed(&self) -> f64;

    /// Get VNE - never exceed speed (km/h)
    fn get_vne(&self) -> f64;

    /// Get MNE - never exceed Mach number
    fn get_vne_mach(&self) -> f64;

    // ==================== G-Load Limits ====================

    /// Get positive G limit at full fuel
    fn get_full_fuel_pos_g(&self) -> f64;

    /// Get negative G limit at full fuel
    fn get_full_fuel_neg_g(&self) -> f64;

    /// Get positive G limit at half fuel
    fn get_half_fuel_pos_g(&self) -> f64;

    /// Get negative G limit at half fuel
    fn get_half_fuel_neg_g(&self) -> f64;

    // ==================== Control Surface Effectiveness ====================

    /// Get elevator effective speed (km/h)
    fn get_elevator_eff_speed(&self) -> f64;

    /// Get aileron effective speed (km/h)
    fn get_aileron_eff_speed(&self) -> f64;

    /// Get rudder effective speed (km/h)
    fn get_rudder_eff_speed(&self) -> f64;

    /// Get elevator power loss factor
    fn get_elevator_power_loss(&self) -> f64;

    /// Get aileron power loss factor
    fn get_aileron_power_loss(&self) -> f64;

    /// Get rudder power loss factor
    fn get_rudder_power_loss(&self) -> f64;

    // ==================== WEP/Nitro System ====================

    /// Get nitro amount (kg)
    fn get_nitro_amount(&self) -> f64;

    /// Get nitro duration (seconds)
    fn get_nitro_time(&self) -> f64;

    /// Check if nitro is available (for hide-when-zero)
    fn is_nitro_amount_valid(&self) -> bool;

    // ==================== Heat Management ====================

    /// Get average engine heat recovery rate
    fn get_avg_eng_recovery_rate(&self) -> f64;

    // ==================== Lift Performance ====================

    /// Get wing loading limit without flaps
    fn get_no_flap_wing_load(&self) -> f64;

    /// Get wing loading limit with full flaps
    fn get_full_flap_wing_load(&self) -> f64;

    // ==================== Inertia ====================

    /// Get pitch moment of inertia (kg*m^2)
    fn get_moi_pitch(&self) -> f64;

    /// Get roll moment of inertia (kg*m^2)
    fn get_moi_roll(&self) -> f64;

    /// Get yaw moment of inertia (kg*m^2)
    fn get_moi_yaw(&self) -> f64;

    // ==================== Wing Geometry ====================

    /// Get wing area (m^2)
    fn get_wing_area(&self) -> f64;

    /// Get fuselage area (m^2)
    fn get_fuselage_area(&self) -> f64;

    /// Get Oswald's efficiency number
    fn get_oswalds_efficiency(&self) -> f64;

    /// Get aspect ratio
    fn get_aspect_ratio(&self) -> f64;

    /// Get swept wing angle (degrees)
    fn get_swept_wing_angle(&self) -> f64;

    // ==================== Drag Parameters ====================

    /// Get drag area coefficient (CdS)
    fn get_cd_s(&self) -> f64;

    /// Get induced drag factor
    fn get_ind_cd_f(&self) -> f64;

    /// Get radiator drag coefficient
    fn get_radiator_cd(&self) -> f64;

    /// Get oil radiator drag coefficient
    fn get_oil_radiator_cd(&self) -> f64;

    // ==================== No-Flaps Wing (fm_parts) ====================

    /// Get NoFlapsWing CdMin (Java: getNoFlapsWing_CdMin)
    fn get_no_flaps_wing_cd_min(&self) -> f64;

    /// Get NoFlapsWing Cl0 (zero-angle lift coefficient)
    fn get_no_flaps_wing_cl0(&self) -> f64;

    /// Get NoFlapsWing critical AoA high (degrees)
    fn get_no_flaps_wing_aoa_crit_high(&self) -> f64;

    /// Get NoFlapsWing critical AoA low (degrees)
    fn get_no_flaps_wing_aoa_crit_low(&self) -> f64;

    /// Get NoFlapsWing ClCritHigh
    fn get_no_flaps_wing_cl_crit_high(&self) -> f64;

    /// Get NoFlapsWing ClCritLow
    fn get_no_flaps_wing_cl_crit_low(&self) -> f64;

    // ==================== Full-Flaps Wing (fm_parts) ====================

    /// Get FullFlapsWing CdMin
    fn get_full_flaps_wing_cd_min(&self) -> f64;

    /// Get FullFlapsWing Cl0
    fn get_full_flaps_wing_cl0(&self) -> f64;

    /// Get FullFlapsWing critical AoA high (degrees)
    fn get_full_flaps_wing_aoa_crit_high(&self) -> f64;

    /// Get FullFlapsWing critical AoA low (degrees)
    fn get_full_flaps_wing_aoa_crit_low(&self) -> f64;

    // ==================== Other fm_parts ====================

    /// Get Fuselage CdMin
    fn get_fuselage_cd_min(&self) -> f64;

    /// 机身最大升力因数 (W3 stall_speed 公式化供值, 无 Java getter 对应)
    fn get_fuse_cl_high(&self) -> f64 {
        0.0
    }

    /// 机身临界迎角上限 (同上)
    fn get_fuselage_aoa_crit_high(&self) -> f64 {
        0.0
    }

    /// 满襟翼临界升力系数上限 (W3 stall 公式化; 原 trait 缺失补齐)
    fn get_full_flaps_wing_cl_crit_high(&self) -> f64 {
        0.0
    }

    /// 满襟翼临界升力系数下限 (同上)
    fn get_full_flaps_wing_cl_crit_low(&self) -> f64 {
        0.0
    }

    /// Get Fin CdMin
    fn get_fin_cd_min(&self) -> f64;

    /// Get Stab CdMin
    fn get_stab_cd_min(&self) -> f64;

    // ==================== Flap Speed Limits ====================

    /// Get flap position 0 speed limit (km/h)
    fn get_flap0_speed(&self) -> f64;

    /// Get flap position 1 speed limit (km/h)
    fn get_flap1_speed(&self) -> f64;

    /// Get flap position 2 speed limit (km/h)
    fn get_flap2_speed(&self) -> f64;

    /// Get flap position 3 speed limit (km/h)
    fn get_flap3_speed(&self) -> f64;

    /// Check if flap 0 speed is valid (for hide-when-zero)
    fn is_flap0_speed_valid(&self) -> bool;

    /// Check if flap 1 speed is valid (for hide-when-zero)
    fn is_flap1_speed_valid(&self) -> bool;

    /// Check if flap 2 speed is valid (for hide-when-zero)
    fn is_flap2_speed_valid(&self) -> bool;

    /// Check if flap 3 speed is valid (for hide-when-zero)
    fn is_flap3_speed_valid(&self) -> bool;

    // ==================== Gear ====================

    /// Get gear destruction speed (km/h)
    fn get_gear_destruction_speed(&self) -> f64;

    // ==================== Engine Info ====================

    /// Check if aircraft is jet-powered
    fn is_jet(&self) -> bool;

    /// Get number of engines
    fn get_engine_num(&self) -> i32;
}
