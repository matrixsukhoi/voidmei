//! 对应 Java: `src/ui/util/ReflectBinder.java` (包 ui.util)。
//!
//! Utility class to resolve string targets to zero-GC data accessors.
//!
//! PORT (D7 裁决, 重设计): Java 用 `MethodHandles.publicLookup().findVirtual` 反射
//! 运行时 Service 类拿到 MethodHandle, `bindTo(service)` 闭包捕获实例; Rust 禁反射
//! (PORTING §1 "反射 MethodHandle → match 注册表") → 编译期 match 注册表:
//! - 先例: POC `PowerSource::get` (vm-overlay overlays_field1) 与 PropertyBinder
//!   批七 (renderer_config_helper::property_binder);
//! - `lookup.findVirtual(名)` → [`DoubleAccessor::resolve`] 等按 Java getter 名
//!   精确 match (cfg 的 `:target`/`:unit-source`/`:precision-source` 直达);
//! - `handle.bindTo(service)` + `invoke()` → `accessor.get(&dyn TelemetrySource)`
//!   求值期传入数据源 (消费方 FieldOverlay 系每帧调 get, 等价 Java supplier 的
//!   getAsDouble; Java supplier 的闭包捕获由消费方持有引用承担)。
//!
//! PORT: 注册表封闭于 `TelemetrySource` 接口面 (58 数值 + 1 String + 1 int 方法)。
//! Java 反射打在运行时类 Service 上, 理论可达 Service/Thread 的非接口方法
//! (如 `getId`/`getPriority` 返回 long/int) — 该域 cfg 全表未使用, 封闭注册表
//! 使其恒走"未命中 → 默认值"分支。
//!
//! PORT: `resolveBoolean` 未移植 — 全库无调用点 (消费面仅 resolveDouble/
//! resolveString/resolveInt, 见 FlightInfoOverlay.java:116 / PowerInfoOverlay.java:
//! 133,152,158); 布尔方法面已由 `visibility_expression::call_method` 承接, 且其
//! "未命中静默返回 null" 契约在强类型域无对应物。

use crate::logger;
use crate::ui_model::TelemetrySource;

/// resolve_double 的注册表键: TelemetrySource 全部数值返回方法
/// (double×57 + long×1, 声明序 = Java TelemetrySource.java 接口序)。
/// 键 = Java getter 名 (findVirtual 精确匹配, 大小写敏感)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleAccessor {
    Ias,                    // getIAS
    Tas,                    // getTAS
    Mach,                   // getMach
    AoA,                    // getAoA
    AoS,                    // getAoS
    Ny,                     // getNy
    Vario,                  // getVario
    Altitude,               // getAltitude
    RadioAltitude,          // getRadioAltitude
    Compass,                // getCompass
    Sep,                    // getSEP
    Acceleration,           // getAcceleration
    TurnRate,               // getTurnRate
    TurnRadius,             // getTurnRadius
    RollRate,               // getRollRate
    EnergyJkg,              // getEnergyJKg
    MassFuel,               // getMassFuel
    TotalWeight,            // getTotalWeight
    FuelTimeMili,           // getFuelTimeMili (long → doubleValue)
    Throttle,               // getThrottle
    Rpm,                    // getRPM
    ManifoldPressure,       // getManifoldPressure
    WaterTemp,              // getWaterTemp
    OilTemp,                // getOilTemp
    Pitch,                  // getPitch
    EffHp,                  // getEffHp
    Thrust,                 // getThrust
    HorsePower,             // getHorsePower
    EngineResponse,         // getEngineResponse
    PropEfficiency,         // getPropEfficiency
    WepKg,                  // getWepKg
    WepTime,                // getWepTime
    HeatTolerance,          // getHeatTolerance
    PowerPercent,           // getPowerPercent
    ManifoldPressurePounds, // getManifoldPressurePounds (Imperial)
    ManifoldPressureInchHg, // getManifoldPressureInchHg (Imperial)
    ManifoldPressureDisplay, // getManifoldPressureDisplay
    UnknownMixture,         // getUnknownMixture
    Radiator,               // getRadiator
    CompressorStage,        // getCompressorStage
    FuelPercent,            // getFuelPercent
    RpmThrottle,            // getRPMThrottle
    Gear,                   // getGear
    Flaps,                  // getFlaps
    Airbrake,               // getAirbrake
    Aileron,                // getAileron
    Elevator,               // getElevator
    Rudder,                 // getRudder
    WingSweep,              // getWingSweep
    SpeedLimitRatio,        // getSpeedLimitRatio
    AileronLockRatio,       // getAileronLockRatio
    RudderLockRatio,        // getRudderLockRatio
    UnitMachLimitRatio,     // getUnitMachLimitRatio
    StallSpeed,             // getStallSpeed
    AviahorizonPitch,       // getAviahorizonPitch
    AviahorizonRoll,        // getAviahorizonRoll
    BoosterFuelKg,          // getBoosterFuelKg
    BoosterFuelPercent,     // getBoosterFuelPercent
}

impl DoubleAccessor {
    /// Java `lookup.findVirtual(service.getClass(), methodName, mt)` 的注册表等价物。
    /// PORT: Java 按 returnTypes {double, float, long, int} 依序尝试 — 接口无同名
    /// 异返回类型方法, 类型序退化为名称一次命中。
    pub fn resolve(method_name: &str) -> Option<DoubleAccessor> {
        match method_name {
            "getIAS" => Some(DoubleAccessor::Ias),
            "getTAS" => Some(DoubleAccessor::Tas),
            "getMach" => Some(DoubleAccessor::Mach),
            "getAoA" => Some(DoubleAccessor::AoA),
            "getAoS" => Some(DoubleAccessor::AoS),
            "getNy" => Some(DoubleAccessor::Ny),
            "getVario" => Some(DoubleAccessor::Vario),
            "getAltitude" => Some(DoubleAccessor::Altitude),
            "getRadioAltitude" => Some(DoubleAccessor::RadioAltitude),
            "getCompass" => Some(DoubleAccessor::Compass),
            "getSEP" => Some(DoubleAccessor::Sep),
            "getAcceleration" => Some(DoubleAccessor::Acceleration),
            "getTurnRate" => Some(DoubleAccessor::TurnRate),
            "getTurnRadius" => Some(DoubleAccessor::TurnRadius),
            "getRollRate" => Some(DoubleAccessor::RollRate),
            "getEnergyJKg" => Some(DoubleAccessor::EnergyJkg),
            "getMassFuel" => Some(DoubleAccessor::MassFuel),
            "getTotalWeight" => Some(DoubleAccessor::TotalWeight),
            "getFuelTimeMili" => Some(DoubleAccessor::FuelTimeMili),
            "getThrottle" => Some(DoubleAccessor::Throttle),
            "getRPM" => Some(DoubleAccessor::Rpm),
            "getManifoldPressure" => Some(DoubleAccessor::ManifoldPressure),
            "getWaterTemp" => Some(DoubleAccessor::WaterTemp),
            "getOilTemp" => Some(DoubleAccessor::OilTemp),
            "getPitch" => Some(DoubleAccessor::Pitch),
            "getEffHp" => Some(DoubleAccessor::EffHp),
            "getThrust" => Some(DoubleAccessor::Thrust),
            "getHorsePower" => Some(DoubleAccessor::HorsePower),
            "getEngineResponse" => Some(DoubleAccessor::EngineResponse),
            "getPropEfficiency" => Some(DoubleAccessor::PropEfficiency),
            "getWepKg" => Some(DoubleAccessor::WepKg),
            "getWepTime" => Some(DoubleAccessor::WepTime),
            "getHeatTolerance" => Some(DoubleAccessor::HeatTolerance),
            "getPowerPercent" => Some(DoubleAccessor::PowerPercent),
            "getManifoldPressurePounds" => Some(DoubleAccessor::ManifoldPressurePounds),
            "getManifoldPressureInchHg" => Some(DoubleAccessor::ManifoldPressureInchHg),
            "getManifoldPressureDisplay" => Some(DoubleAccessor::ManifoldPressureDisplay),
            "getUnknownMixture" => Some(DoubleAccessor::UnknownMixture),
            "getRadiator" => Some(DoubleAccessor::Radiator),
            "getCompressorStage" => Some(DoubleAccessor::CompressorStage),
            "getFuelPercent" => Some(DoubleAccessor::FuelPercent),
            "getRPMThrottle" => Some(DoubleAccessor::RpmThrottle),
            "getGear" => Some(DoubleAccessor::Gear),
            "getFlaps" => Some(DoubleAccessor::Flaps),
            "getAirbrake" => Some(DoubleAccessor::Airbrake),
            "getAileron" => Some(DoubleAccessor::Aileron),
            "getElevator" => Some(DoubleAccessor::Elevator),
            "getRudder" => Some(DoubleAccessor::Rudder),
            "getWingSweep" => Some(DoubleAccessor::WingSweep),
            "getSpeedLimitRatio" => Some(DoubleAccessor::SpeedLimitRatio),
            "getAileronLockRatio" => Some(DoubleAccessor::AileronLockRatio),
            "getRudderLockRatio" => Some(DoubleAccessor::RudderLockRatio),
            "getUnitMachLimitRatio" => Some(DoubleAccessor::UnitMachLimitRatio),
            "getStallSpeed" => Some(DoubleAccessor::StallSpeed),
            "getAviahorizonPitch" => Some(DoubleAccessor::AviahorizonPitch),
            "getAviahorizonRoll" => Some(DoubleAccessor::AviahorizonRoll),
            "getBoosterFuelKg" => Some(DoubleAccessor::BoosterFuelKg),
            "getBoosterFuelPercent" => Some(DoubleAccessor::BoosterFuelPercent),
            // Java: NoSuchMethodException → 上层 warn + () -> 0.0
            _ => None,
        }
    }

    /// Java `boundHandle.invoke()` 的类型化分派。
    /// PORT: Java 侧 `catch (Throwable) → 0.0` 与 `instanceof Number` 兜底在
    /// 类型化分派下结构不可达 (方法必然存在且必然返回数值)。
    pub fn get(self, s: &dyn TelemetrySource) -> f64 {
        match self {
            DoubleAccessor::Ias => s.get_ias(),
            DoubleAccessor::Tas => s.get_tas(),
            DoubleAccessor::Mach => s.get_mach(),
            DoubleAccessor::AoA => s.get_aoa(),
            DoubleAccessor::AoS => s.get_aos(),
            DoubleAccessor::Ny => s.get_ny(),
            DoubleAccessor::Vario => s.get_vario(),
            DoubleAccessor::Altitude => s.get_altitude(),
            DoubleAccessor::RadioAltitude => s.get_radio_altitude(),
            DoubleAccessor::Compass => s.get_compass(),
            DoubleAccessor::Sep => s.get_sep(),
            DoubleAccessor::Acceleration => s.get_acceleration(),
            DoubleAccessor::TurnRate => s.get_turn_rate(),
            DoubleAccessor::TurnRadius => s.get_turn_radius(),
            DoubleAccessor::RollRate => s.get_roll_rate(),
            DoubleAccessor::EnergyJkg => s.get_energy_jkg(),
            DoubleAccessor::MassFuel => s.get_mass_fuel(),
            DoubleAccessor::TotalWeight => s.get_total_weight(),
            // Java: ((Number) long值).doubleValue() — long→double 就近舍入, `as f64` 一致
            DoubleAccessor::FuelTimeMili => s.get_fuel_time_mili() as f64,
            DoubleAccessor::Throttle => s.get_throttle(),
            DoubleAccessor::Rpm => s.get_rpm(),
            DoubleAccessor::ManifoldPressure => s.get_manifold_pressure(),
            DoubleAccessor::WaterTemp => s.get_water_temp(),
            DoubleAccessor::OilTemp => s.get_oil_temp(),
            DoubleAccessor::Pitch => s.get_pitch(),
            DoubleAccessor::EffHp => s.get_eff_hp(),
            DoubleAccessor::Thrust => s.get_thrust(),
            DoubleAccessor::HorsePower => s.get_horse_power(),
            DoubleAccessor::EngineResponse => s.get_engine_response(),
            DoubleAccessor::PropEfficiency => s.get_prop_efficiency(),
            DoubleAccessor::WepKg => s.get_wep_kg(),
            DoubleAccessor::WepTime => s.get_wep_time(),
            DoubleAccessor::HeatTolerance => s.get_heat_tolerance(),
            DoubleAccessor::PowerPercent => s.get_power_percent(),
            DoubleAccessor::ManifoldPressurePounds => s.get_manifold_pressure_pounds(),
            DoubleAccessor::ManifoldPressureInchHg => s.get_manifold_pressure_inch_hg(),
            DoubleAccessor::ManifoldPressureDisplay => s.get_manifold_pressure_display(),
            DoubleAccessor::UnknownMixture => s.get_unknown_mixture(),
            DoubleAccessor::Radiator => s.get_radiator(),
            DoubleAccessor::CompressorStage => s.get_compressor_stage(),
            DoubleAccessor::FuelPercent => s.get_fuel_percent(),
            DoubleAccessor::RpmThrottle => s.get_rpm_throttle(),
            DoubleAccessor::Gear => s.get_gear(),
            DoubleAccessor::Flaps => s.get_flaps(),
            DoubleAccessor::Airbrake => s.get_airbrake(),
            DoubleAccessor::Aileron => s.get_aileron(),
            DoubleAccessor::Elevator => s.get_elevator(),
            DoubleAccessor::Rudder => s.get_rudder(),
            DoubleAccessor::WingSweep => s.get_wing_sweep(),
            DoubleAccessor::SpeedLimitRatio => s.get_speed_limit_ratio(),
            DoubleAccessor::AileronLockRatio => s.get_aileron_lock_ratio(),
            DoubleAccessor::RudderLockRatio => s.get_rudder_lock_ratio(),
            DoubleAccessor::UnitMachLimitRatio => s.get_unit_mach_limit_ratio(),
            DoubleAccessor::StallSpeed => s.get_stall_speed(),
            DoubleAccessor::AviahorizonPitch => s.get_aviahorizon_pitch(),
            DoubleAccessor::AviahorizonRoll => s.get_aviahorizon_roll(),
            DoubleAccessor::BoosterFuelKg => s.get_booster_fuel_kg(),
            DoubleAccessor::BoosterFuelPercent => s.get_booster_fuel_percent(),
        }
    }
}

/// resolve_string 的注册表键: TelemetrySource 全部 String 返回方法。
/// cfg 全表仅进气压一条使用 (`:unit-source`, ui_layout.cfg L157)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringAccessor {
    /// "getManifoldPressureDisplayUnit"
    ManifoldPressureDisplayUnit,
}

impl StringAccessor {
    /// Java `findVirtual(..., methodType(String.class))` 等价物
    pub fn resolve(method_name: &str) -> Option<StringAccessor> {
        match method_name {
            "getManifoldPressureDisplayUnit" => Some(StringAccessor::ManifoldPressureDisplayUnit),
            // Java: NoSuchMethodException → 上层 warn + () -> ""
            _ => None,
        }
    }

    /// Java `boundHandle.invokeExact()` 的类型化分派 (Throwable catch → "" 不可达)
    pub fn get(self, s: &dyn TelemetrySource) -> String {
        match self {
            StringAccessor::ManifoldPressureDisplayUnit => s.get_manifold_pressure_display_unit(),
        }
    }
}

/// resolve_int 的注册表键: TelemetrySource 全部 int 返回方法。
/// cfg 全表仅进气压一条使用 (`:precision-source`, ui_layout.cfg L157)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntAccessor {
    /// "getManifoldPressureDisplayPrecision"
    ManifoldPressureDisplayPrecision,
}

impl IntAccessor {
    /// Java `findVirtual(..., methodType(int.class))` 等价物
    pub fn resolve(method_name: &str) -> Option<IntAccessor> {
        match method_name {
            "getManifoldPressureDisplayPrecision" => {
                Some(IntAccessor::ManifoldPressureDisplayPrecision)
            }
            // Java: NoSuchMethodException → 上层 warn + () -> 0
            _ => None,
        }
    }

    /// Java `boundHandle.invoke()` 的类型化分派 (Throwable catch → 0 不可达)
    pub fn get(self, s: &dyn TelemetrySource) -> i32 {
        match self {
            IntAccessor::ManifoldPressureDisplayPrecision => {
                s.get_manifold_pressure_display_precision()
            }
        }
    }
}

/// Java `resolveDouble` 返回的 DoubleSupplier 等价物:
/// 已解析的方法槽 (None = 未命中/空目标, 对应 Java 的 `() -> 0.0` 供应商) + 乘数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoubleBinding {
    pub accessor: Option<DoubleAccessor>,
    /// "getter * N" 的 N (缺省 1.0)
    pub multiplier: f64,
}

impl DoubleBinding {
    /// Java 闭包体: `((Number) invoke()).doubleValue() * finalMultiplier`
    pub fn get(self, s: &dyn TelemetrySource) -> f64 {
        match self.accessor {
            // Java: result instanceof Number → doubleValue() * multiplier
            Some(a) => a.get(s) * self.multiplier,
            // Java: 未命中 / 空 target / invoke 抛错 → return 0.0
            None => 0.0,
        }
    }
}

/// Java `resolveString` 返回的 `Supplier<String>` 等价物 (None → "")。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringBinding {
    pub accessor: Option<StringAccessor>,
}

impl StringBinding {
    pub fn get(self, s: &dyn TelemetrySource) -> String {
        match self.accessor {
            Some(a) => a.get(s),
            // Java: 未命中 / 空 target / invoke 抛错 → return ""
            None => String::new(),
        }
    }
}

/// Java `resolveInt` 返回的 IntSupplier 等价物 (None → 0)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntBinding {
    pub accessor: Option<IntAccessor>,
}

impl IntBinding {
    pub fn get(self, s: &dyn TelemetrySource) -> i32 {
        match self.accessor {
            Some(a) => a.get(s),
            // Java: 未命中 / 空 target / invoke 抛错 → return 0
            None => 0,
        }
    }
}

/// Java `target.split("\\*")` 复刻: 按 '*' 切分并丢弃**全部尾部空串**
/// (JDK String.split limit=0 语义; 前导/中间空串保留, 如 "*" → 空数组)。
fn java_split_star(s: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = s.split('*').collect();
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    parts
}

/// Resolves a target string to a DoubleSupplier.
///
/// Target formats:
/// - "getRPM" -> binds to service.getRPM()
/// - "getWingSweep * 100" -> binds to service.getWingSweep() * 100
///
/// @param target  The target string from config
/// @return Zero-GC DoubleSupplier
///
/// PORT: Java `service == null` 防御分支 (warn "Service is null") 域外不可达 —
/// 两处调用方均以 `service != null` 守卫在前 (FlightInfoOverlay.java:105 /
/// PowerInfoOverlay.java:113), preview 模式不进入绑定; 数据源改为 [`DoubleBinding::get`]
/// 求值期传入, null 无对应物。`target == null` 同理由调用方的
/// `row.property != null && !row.property.isEmpty()` 守卫覆盖 (&str 无 null)。
///
/// PORT: Java findVirtual 阶段的 `IllegalAccessException` (warn "Access denied
/// for method ... Is it public?") 与外层 `catch (Exception)` (warn "Unexpected
/// error binding") 分支在编译期注册表下结构不可达 — 注册表项均为 trait 公有方法,
/// 无访问性/意外失败路径, 正确省略 (invoke 期 `catch (Throwable)`/`instanceof
/// Number` 兜底见 [`DoubleAccessor::get`] 注释)。
pub fn resolve_double(target: &str) -> DoubleBinding {
    // Java: if (target == null || target.isEmpty()) return () -> 0.0;
    if target.is_empty() {
        return DoubleBinding {
            accessor: None,
            multiplier: 1.0,
        };
    }

    let mut method_name = target.trim();
    let mut multiplier = 1.0;

    // Simple arithmetic support (* only for now)
    if target.contains('*') {
        let parts = java_split_star(target);
        if parts.len() == 2 {
            // Java: methodName = parts[0].trim(); (先赋名, 解析失败不回滚)
            method_name = parts[0].trim();
            // Java: multiplier = Double.parseDouble(parts[1].trim());
            // PORT: §2.15 parse 差异域 (visibility_expression::get_value 同款):
            // Java 收 "5f"/"0x1p1" 拒 "inf", Rust 恰反 — 真实 cfg 乘数均为纯小数
            // 字面量 ("100"/"0.001"), 分歧域外。失败 → warn + multiplier 保持 1.0。
            match parts[1].trim().parse::<f64>() {
                Ok(m) => multiplier = m,
                Err(_) => logger::warn(
                    "ReflectBinder",
                    &format!("Invalid multiplier in target: {target}"),
                ),
            }
        }
    }

    let accessor = DoubleAccessor::resolve(method_name);
    if accessor.is_none() {
        // Java: "Method 'X' not found with expected types in <className>"
        // (className = 运行时类; 注册表封闭于接口面, 故固定写 TelemetrySource)
        logger::warn(
            "ReflectBinder",
            &format!("Method '{method_name}' not found with expected types in TelemetrySource"),
        );
    }
    DoubleBinding {
        accessor,
        multiplier,
    }
}

/// Resolves a target string to a String Supplier.
///
/// PORT: 同 resolve_double — service==null 分支域外不可达; `catch (Exception)`
/// 泛化分支 (warn "Could not bind String target") 编译期注册表下结构不可达,
/// 正确省略; 无乘数语法 (Java 原实现即只 `target.trim()` 直查)。
pub fn resolve_string(target: &str) -> StringBinding {
    // Java: if (target == null || target.isEmpty()) return () -> "";
    if target.is_empty() {
        return StringBinding { accessor: None };
    }

    let accessor = StringAccessor::resolve(target.trim());
    if accessor.is_none() {
        logger::warn(
            "ReflectBinder",
            &format!(
                "String Method '{}' NOT found in TelemetrySource",
                target.trim()
            ),
        );
    }
    StringBinding { accessor }
}

/// Resolves a target string to an IntSupplier.
///
/// PORT: 同 resolve_double — service==null 分支域外不可达; `catch (Exception)`
/// 泛化分支 (warn "Error binding int") 编译期注册表下结构不可达, 正确省略;
/// 无乘数语法。
pub fn resolve_int(target: &str) -> IntBinding {
    // Java: if (target == null || target.isEmpty() || service == null) return () -> 0;
    if target.is_empty() {
        return IntBinding { accessor: None };
    }

    let accessor = IntAccessor::resolve(target.trim());
    if accessor.is_none() {
        logger::warn(
            "ReflectBinder",
            &format!(
                "Int method '{}' NOT found in TelemetrySource",
                target.trim()
            ),
        );
    }
    IntBinding { accessor }
}

#[cfg(test)]
mod tests;
