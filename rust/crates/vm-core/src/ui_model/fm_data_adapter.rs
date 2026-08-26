//! 对应 Java: `src/ui/model/FMDataAdapter.java` (一比一翻译)

use crate::physics_constants::g;
use crate::ui_model::fm_data_source::FMDataSource;
use std::sync::{Arc, RwLock};

/// PORT: Java `parser.Blkx` 属后续翻译批次 (CLASSIFY 豁免项), 本 crate 尚无对应物
/// (fm::handle 的 BlkxPlaceholder 是零字段"存在性"占位, 供 FMHandle 用)。
/// FMDataAdapter 逐字段读取 Blkx 数值做公式计算 —— 为让公式与分支可测试,
/// 此处定义"FMDataAdapter 消费面子集"数据快照占位 (字段名/类型对照 Blkx.java
/// 公有字段声明, 仅含被本适配器读取的子集)。
// TODO(port): 真实 crate::parser::Blkx 落地时删除本占位类型, `blkx` 字段与
// set_blkx/get_blkx 签名切换为 Arc<Blkx> (§0.4 标记)。届时 crate 内共两处
// BlkxPlaceholder 需一并收编: fm::handle::BlkxPlaceholder (零字段存在性占位,
// 供 FMHandle) 与本处 (41 字段消费面快照), 避免留下第二真相源。
#[derive(Debug, Clone)]
pub struct BlkxPlaceholder {
    pub read_file_name: Option<String>,
    pub version: Option<String>,
    pub emptyweight: f64,
    pub maxfuelweight: f64,
    /// 压缩性临界速度 (m/s; getCriticalSpeed 换算 km/h)
    pub critical_speed: f64,
    pub vne: f64,
    pub vne_mach: f64,
    /// 翼临界过载力矩原始值 [neg, pos]
    pub raw_wing_crit_overload: Option<Vec<f64>>,
    pub grossweight: f64,
    pub halfweight: f64,
    /// Java 原拼写即 elav (elevator 缺字母), 保真保留
    pub elav_eff: f64,
    pub aileron_eff: f64,
    pub rudder_eff: f64,
    pub elav_power_loss: f64,
    pub aileron_power_loss: f64,
    pub rudder_power_loss: f64,
    pub nitro: f64,
    pub nitro_decr: f64,
    pub avg_eng_recovery_rate: f64,
    pub no_flap_wll: f64,
    pub full_flap_wll: f64,
    pub moment_of_inertia: Option<Vec<f64>>,
    pub a_wing: f64,
    pub a_fuselage: f64,
    pub oswalds_efficiency_number: f64,
    pub aspect_ratio: f64,
    pub swept_wing_angle: f64,
    pub cd_s: f64,
    pub ind_cd_f: f64,
    pub radiator_cd: f64,
    pub oil_radiator_cd: f64,
    pub no_flaps_wing: Option<FmPartsPlaceholder>,
    pub full_flaps_wing: Option<FmPartsPlaceholder>,
    pub fuselage: Option<FmPartsPlaceholder>,
    pub fin: Option<FmPartsPlaceholder>,
    pub stab: Option<FmPartsPlaceholder>,
    pub flaps_destruction_num: i32,
    /// 襟翼档位破坏指示速度表 [档位][1=速度]
    pub flaps_destruction_ind_speed: Option<Vec<Vec<f64>>>,
    pub gear_destruction_ind_speed: f64,
    pub is_jet: bool,
    pub engine_num: i32,
}

/// Java `Blkx.fm_parts` 内部类的消费面子集 (CdMin/Cl0/AoA/ClCrit 族)。
// TODO(port): 同 BlkxPlaceholder, 真实 fm_parts 落地时删除。
#[derive(Debug, Clone)]
pub struct FmPartsPlaceholder {
    pub cd_min: f64,
    pub cl0: f64,
    pub cl_crit_high: f64,
    pub cl_crit_low: f64,
    pub aoa_crit_high: f64,
    pub aoa_crit_low: f64,
}

/// Java 字段声明默认值 (数值 0 / 引用 null / boolean false)
impl Default for BlkxPlaceholder {
    fn default() -> Self {
        BlkxPlaceholder {
            read_file_name: None,
            version: None,
            emptyweight: 0.0,
            maxfuelweight: 0.0,
            critical_speed: 0.0,
            vne: 0.0,
            vne_mach: 0.0,
            raw_wing_crit_overload: None,
            grossweight: 0.0,
            halfweight: 0.0,
            elav_eff: 0.0,
            aileron_eff: 0.0,
            rudder_eff: 0.0,
            elav_power_loss: 0.0,
            aileron_power_loss: 0.0,
            rudder_power_loss: 0.0,
            nitro: 0.0,
            nitro_decr: 0.0,
            avg_eng_recovery_rate: 0.0,
            no_flap_wll: 0.0,
            full_flap_wll: 0.0,
            moment_of_inertia: None,
            a_wing: 0.0,
            a_fuselage: 0.0,
            oswalds_efficiency_number: 0.0,
            aspect_ratio: 0.0,
            swept_wing_angle: 0.0,
            cd_s: 0.0,
            ind_cd_f: 0.0,
            radiator_cd: 0.0,
            oil_radiator_cd: 0.0,
            no_flaps_wing: None,
            full_flaps_wing: None,
            fuselage: None,
            fin: None,
            stab: None,
            flaps_destruction_num: 0,
            flaps_destruction_ind_speed: None,
            gear_destruction_ind_speed: 0.0,
            is_jet: false,
            engine_num: 0,
        }
    }
}

impl FmPartsPlaceholder {
    pub fn new(cd_min: f64, cl0: f64, cl_crit_high: f64, cl_crit_low: f64, aoa_crit_high: f64, aoa_crit_low: f64) -> Self {
        FmPartsPlaceholder {
            cd_min,
            cl0,
            cl_crit_high,
            cl_crit_low,
            aoa_crit_high,
            aoa_crit_low,
        }
    }
}

/// Adapter that wraps a Blkx instance and implements FMDataSource.
/// Provides zero-allocation access to FM data for overlay display.
///
/// <p>This adapter allows FMUnpackedDataOverlay to use ReflectBinder
/// for dynamic field binding without directly depending on Blkx structure.
pub struct FMDataAdapter {
    /// P3: volatile 保证跨线程可见性 —— FM_CHANGED 订阅在 FM-Loader 后台线程写入
    /// (reloadFMData), BaseOverlay.run() 线程周期读取; 实例引用原子替换, 无需锁。
    // PORT: Java volatile 引用 → RwLock<Option<Arc<..>>> 原子替换 (LIFETIMES 3.2
    // 推荐 ArcSwap, 但 workspace 未引入该 crate, std 退而用 RwLock; 单写多读、
    // 读侧仅解引用取值, 语义等价)。
    blkx: RwLock<Option<Arc<BlkxPlaceholder>>>,
}

impl Default for FMDataAdapter {
    fn default() -> Self {
        FMDataAdapter::new()
    }
}

impl FMDataAdapter {
    /// Java 隐式无参构造器: blkx 字段默认 null
    pub fn new() -> FMDataAdapter {
        FMDataAdapter {
            blkx: RwLock::new(None),
        }
    }

    /// Set the Blkx instance to read data from.
    /// @param blkx The flight model data, or null if not loaded
    pub fn set_blkx(&self, blkx: Option<Arc<BlkxPlaceholder>>) {
        *self.blkx.write().unwrap() = blkx;
    }

    /// Get the current Blkx instance.
    // PORT: Java 返回引用 (null 可能) → Option<Arc<..>> (Arc 克隆 = 引用传递)
    pub fn get_blkx(&self) -> Option<Arc<BlkxPlaceholder>> {
        self.blkx.read().unwrap().clone()
    }
}

impl FMDataSource for FMDataAdapter {
    // ==================== Basic Info ====================

    fn get_fm_version(&self) -> String {
        // PORT: 锁内只取 Arc 副本即释放, 格式化在锁外 (§2.8 锁粒度纪律,
        // Java volatile 读本身无锁, 不应在持锁期做分配)
        let blkx = self.blkx.read().unwrap().clone();
        let blkx = match blkx {
            Some(b) => b,
            None => return String::new(),
        };
        let name = blkx.read_file_name.as_deref().unwrap_or("N/A");
        let ver = blkx.version.as_deref().unwrap_or("N/A");
        // PORT: Java 原样 —— ver 计算后未使用, 返回值仅取 name (死变量逐行保留)
        let _ = ver;
        name.to_string()
    }

    fn get_empty_weight(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.emptyweight,
            None => 0.0,
        }
    }

    fn get_max_fuel_weight(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.maxfuelweight,
            None => 0.0,
        }
    }

    // ==================== Speed Limits ====================

    fn get_critical_speed(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            // m/s → km/h
            Some(b) => b.critical_speed * 3.6,
            None => 0.0,
        }
    }

    fn get_vne(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.vne,
            None => 0.0,
        }
    }

    fn get_vne_mach(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.vne_mach,
            None => 0.0,
        }
    }

    // ==================== G-Load Limits ====================

    fn get_full_fuel_pos_g(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        let raw = match b.raw_wing_crit_overload.as_ref() {
            Some(r) => r,
            None => return 0.0,
        };
        // PORT: Java rawWingCritOverload[1] 越界抛 AIOOBE → Rust 索引 panic, 同为硬失败
        1.2 * (2.0 * raw[1] / (g * b.grossweight) - 1.0)
    }

    fn get_full_fuel_neg_g(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        let raw = match b.raw_wing_crit_overload.as_ref() {
            Some(r) => r,
            None => return 0.0,
        };
        1.2 * (2.0 * raw[0] / (g * b.grossweight) + 1.0)
    }

    fn get_half_fuel_pos_g(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        let raw = match b.raw_wing_crit_overload.as_ref() {
            Some(r) => r,
            None => return 0.0,
        };
        1.2 * (2.0 * raw[1] / (g * b.halfweight) - 1.0)
    }

    fn get_half_fuel_neg_g(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        let raw = match b.raw_wing_crit_overload.as_ref() {
            Some(r) => r,
            None => return 0.0,
        };
        1.2 * (2.0 * raw[0] / (g * b.halfweight) + 1.0)
    }

    // ==================== Control Surface Effectiveness ====================

    fn get_elevator_eff_speed(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.elav_eff,
            None => 0.0,
        }
    }

    fn get_aileron_eff_speed(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.aileron_eff,
            None => 0.0,
        }
    }

    fn get_rudder_eff_speed(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.rudder_eff,
            None => 0.0,
        }
    }

    fn get_elevator_power_loss(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.elav_power_loss,
            None => 0.0,
        }
    }

    fn get_aileron_power_loss(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.aileron_power_loss,
            None => 0.0,
        }
    }

    fn get_rudder_power_loss(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.rudder_power_loss,
            None => 0.0,
        }
    }

    // ==================== WEP/Nitro System ====================

    fn get_nitro_amount(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.nitro,
            None => 0.0,
        }
    }

    fn get_nitro_time(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        if b.nitro_decr <= 0.0 {
            return 0.0;
        }
        b.nitro / (b.nitro_decr * 60.0)
    }

    fn is_nitro_amount_valid(&self) -> bool {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.nitro > 0.0,
            None => false,
        }
    }

    // ==================== Heat Management ====================

    fn get_avg_eng_recovery_rate(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.avg_eng_recovery_rate,
            None => 0.0,
        }
    }

    // ==================== Lift Performance ====================

    fn get_no_flap_wing_load(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.no_flap_wll,
            None => 0.0,
        }
    }

    fn get_full_flap_wing_load(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.full_flap_wll,
            None => 0.0,
        }
    }

    // ==================== Inertia ====================

    fn get_moi_pitch(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        match b.moment_of_inertia.as_ref() {
            // Java MomentOfInertia.length < 3 → 0
            Some(moi) if moi.len() >= 3 => moi[2],
            _ => 0.0,
        }
    }

    fn get_moi_roll(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        match b.moment_of_inertia.as_ref() {
            // Java MomentOfInertia.length < 1 → 0
            Some(moi) if !moi.is_empty() => moi[0],
            _ => 0.0,
        }
    }

    fn get_moi_yaw(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        match b.moment_of_inertia.as_ref() {
            // Java MomentOfInertia.length < 2 → 0
            Some(moi) if moi.len() >= 2 => moi[1],
            _ => 0.0,
        }
    }

    // ==================== Wing Geometry ====================

    fn get_wing_area(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.a_wing,
            None => 0.0,
        }
    }

    fn get_fuselage_area(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.a_fuselage,
            None => 0.0,
        }
    }

    fn get_oswalds_efficiency(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.oswalds_efficiency_number,
            None => 0.0,
        }
    }

    fn get_aspect_ratio(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.aspect_ratio,
            None => 0.0,
        }
    }

    fn get_swept_wing_angle(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.swept_wing_angle,
            None => 0.0,
        }
    }

    // ==================== Drag Parameters ====================

    fn get_cd_s(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.cd_s,
            None => 0.0,
        }
    }

    fn get_ind_cd_f(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.ind_cd_f,
            None => 0.0,
        }
    }

    fn get_radiator_cd(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.radiator_cd,
            None => 0.0,
        }
    }

    fn get_oil_radiator_cd(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.oil_radiator_cd,
            None => 0.0,
        }
    }

    // ==================== No-Flaps Wing (fm_parts) ====================

    fn get_no_flaps_wing_cd_min(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.no_flaps_wing.is_some() => b.no_flaps_wing.as_ref().unwrap().cd_min,
            _ => 0.0,
        }
    }

    fn get_no_flaps_wing_cl0(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.no_flaps_wing.is_some() => b.no_flaps_wing.as_ref().unwrap().cl0,
            _ => 0.0,
        }
    }

    fn get_no_flaps_wing_aoa_crit_high(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.no_flaps_wing.is_some() => b.no_flaps_wing.as_ref().unwrap().aoa_crit_high,
            _ => 0.0,
        }
    }

    fn get_no_flaps_wing_aoa_crit_low(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.no_flaps_wing.is_some() => b.no_flaps_wing.as_ref().unwrap().aoa_crit_low,
            _ => 0.0,
        }
    }

    fn get_no_flaps_wing_cl_crit_high(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.no_flaps_wing.is_some() => b.no_flaps_wing.as_ref().unwrap().cl_crit_high,
            _ => 0.0,
        }
    }

    fn get_no_flaps_wing_cl_crit_low(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.no_flaps_wing.is_some() => b.no_flaps_wing.as_ref().unwrap().cl_crit_low,
            _ => 0.0,
        }
    }

    // ==================== Full-Flaps Wing (fm_parts) ====================

    fn get_full_flaps_wing_cd_min(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.full_flaps_wing.is_some() => b.full_flaps_wing.as_ref().unwrap().cd_min,
            _ => 0.0,
        }
    }

    fn get_full_flaps_wing_cl0(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.full_flaps_wing.is_some() => b.full_flaps_wing.as_ref().unwrap().cl0,
            _ => 0.0,
        }
    }

    fn get_full_flaps_wing_aoa_crit_high(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.full_flaps_wing.is_some() => b.full_flaps_wing.as_ref().unwrap().aoa_crit_high,
            _ => 0.0,
        }
    }

    fn get_full_flaps_wing_aoa_crit_low(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.full_flaps_wing.is_some() => b.full_flaps_wing.as_ref().unwrap().aoa_crit_low,
            _ => 0.0,
        }
    }

    // ==================== Other fm_parts ====================

    fn get_fuselage_cd_min(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.fuselage.is_some() => b.fuselage.as_ref().unwrap().cd_min,
            _ => 0.0,
        }
    }

    fn get_fin_cd_min(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.fin.is_some() => b.fin.as_ref().unwrap().cd_min,
            _ => 0.0,
        }
    }

    fn get_stab_cd_min(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) if b.stab.is_some() => b.stab.as_ref().unwrap().cd_min,
            _ => 0.0,
        }
    }

    // ==================== Flap Speed Limits ====================

    fn get_flap0_speed(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        let speeds = match b.flaps_destruction_ind_speed.as_ref() {
            Some(s) => s,
            None => return 0.0,
        };
        if b.flaps_destruction_num > 0 {
            // PORT: Java 越界 ([0] 不存在或内层 <2) 抛 AIOOBE → Rust panic, 同为硬失败
            return speeds[0][1];
        }
        0.0
    }

    fn get_flap1_speed(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        let speeds = match b.flaps_destruction_ind_speed.as_ref() {
            Some(s) => s,
            None => return 0.0,
        };
        if b.flaps_destruction_num > 1 {
            return speeds[1][1];
        }
        0.0
    }

    fn get_flap2_speed(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        let speeds = match b.flaps_destruction_ind_speed.as_ref() {
            Some(s) => s,
            None => return 0.0,
        };
        if b.flaps_destruction_num > 2 {
            return speeds[2][1];
        }
        0.0
    }

    fn get_flap3_speed(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        let b = match guard.as_ref() {
            Some(b) => b,
            None => return 0.0,
        };
        let speeds = match b.flaps_destruction_ind_speed.as_ref() {
            Some(s) => s,
            None => return 0.0,
        };
        if b.flaps_destruction_num > 3 {
            return speeds[3][1];
        }
        0.0
    }

    fn is_flap0_speed_valid(&self) -> bool {
        self.get_flap0_speed() > 0.0
    }

    fn is_flap1_speed_valid(&self) -> bool {
        self.get_flap1_speed() > 0.0
    }

    fn is_flap2_speed_valid(&self) -> bool {
        self.get_flap2_speed() > 0.0
    }

    fn is_flap3_speed_valid(&self) -> bool {
        self.get_flap3_speed() > 0.0
    }

    // ==================== Gear ====================

    fn get_gear_destruction_speed(&self) -> f64 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.gear_destruction_ind_speed,
            None => 0.0,
        }
    }

    // ==================== Engine Info ====================

    fn is_jet(&self) -> bool {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.is_jet,
            None => false,
        }
    }

    fn get_engine_num(&self) -> i32 {
        let guard = self.blkx.read().unwrap();
        match guard.as_ref() {
            Some(b) => b.engine_num,
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    // PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
    // 不改成 struct 字面量以保持与 Java 测试源逐行对应
    #![allow(clippy::field_reassign_with_default)]

    use super::*;

    /// Java 空句柄语义: blkx=null 时全部 getter 返回 0/""/false
    #[test]
    fn null_blkx_returns_defaults() {
        let adapter = FMDataAdapter::new();
        assert_eq!(adapter.get_fm_version(), "");
        assert_eq!(adapter.get_empty_weight(), 0.0);
        assert_eq!(adapter.get_max_fuel_weight(), 0.0);
        assert_eq!(adapter.get_critical_speed(), 0.0);
        assert_eq!(adapter.get_vne(), 0.0);
        assert_eq!(adapter.get_vne_mach(), 0.0);
        assert_eq!(adapter.get_full_fuel_pos_g(), 0.0);
        assert_eq!(adapter.get_full_fuel_neg_g(), 0.0);
        assert_eq!(adapter.get_half_fuel_pos_g(), 0.0);
        assert_eq!(adapter.get_half_fuel_neg_g(), 0.0);
        assert_eq!(adapter.get_elevator_eff_speed(), 0.0);
        assert_eq!(adapter.get_aileron_eff_speed(), 0.0);
        assert_eq!(adapter.get_rudder_eff_speed(), 0.0);
        assert_eq!(adapter.get_elevator_power_loss(), 0.0);
        assert_eq!(adapter.get_aileron_power_loss(), 0.0);
        assert_eq!(adapter.get_rudder_power_loss(), 0.0);
        assert_eq!(adapter.get_nitro_amount(), 0.0);
        assert_eq!(adapter.get_nitro_time(), 0.0);
        assert!(!adapter.is_nitro_amount_valid());
        assert_eq!(adapter.get_avg_eng_recovery_rate(), 0.0);
        assert_eq!(adapter.get_no_flap_wing_load(), 0.0);
        assert_eq!(adapter.get_full_flap_wing_load(), 0.0);
        assert_eq!(adapter.get_moi_pitch(), 0.0);
        assert_eq!(adapter.get_moi_roll(), 0.0);
        assert_eq!(adapter.get_moi_yaw(), 0.0);
        assert_eq!(adapter.get_wing_area(), 0.0);
        assert_eq!(adapter.get_fuselage_area(), 0.0);
        assert_eq!(adapter.get_oswalds_efficiency(), 0.0);
        assert_eq!(adapter.get_aspect_ratio(), 0.0);
        assert_eq!(adapter.get_swept_wing_angle(), 0.0);
        assert_eq!(adapter.get_cd_s(), 0.0);
        assert_eq!(adapter.get_ind_cd_f(), 0.0);
        assert_eq!(adapter.get_radiator_cd(), 0.0);
        assert_eq!(adapter.get_oil_radiator_cd(), 0.0);
        assert_eq!(adapter.get_no_flaps_wing_cd_min(), 0.0);
        assert_eq!(adapter.get_no_flaps_wing_cl0(), 0.0);
        assert_eq!(adapter.get_no_flaps_wing_aoa_crit_high(), 0.0);
        assert_eq!(adapter.get_no_flaps_wing_aoa_crit_low(), 0.0);
        assert_eq!(adapter.get_no_flaps_wing_cl_crit_high(), 0.0);
        assert_eq!(adapter.get_no_flaps_wing_cl_crit_low(), 0.0);
        assert_eq!(adapter.get_full_flaps_wing_cd_min(), 0.0);
        assert_eq!(adapter.get_full_flaps_wing_cl0(), 0.0);
        assert_eq!(adapter.get_full_flaps_wing_aoa_crit_high(), 0.0);
        assert_eq!(adapter.get_full_flaps_wing_aoa_crit_low(), 0.0);
        assert_eq!(adapter.get_fuselage_cd_min(), 0.0);
        assert_eq!(adapter.get_fin_cd_min(), 0.0);
        assert_eq!(adapter.get_stab_cd_min(), 0.0);
        assert_eq!(adapter.get_flap0_speed(), 0.0);
        assert_eq!(adapter.get_flap1_speed(), 0.0);
        assert_eq!(adapter.get_flap2_speed(), 0.0);
        assert_eq!(adapter.get_flap3_speed(), 0.0);
        assert!(!adapter.is_flap0_speed_valid());
        assert!(!adapter.is_flap1_speed_valid());
        assert!(!adapter.is_flap2_speed_valid());
        assert!(!adapter.is_flap3_speed_valid());
        assert_eq!(adapter.get_gear_destruction_speed(), 0.0);
        assert!(!adapter.is_jet());
        assert_eq!(adapter.get_engine_num(), 0);
        assert!(adapter.get_blkx().is_none());
    }

    /// getFmVersion: 返回 readFileName; 空时兜底 "N/A"; ver 为 Java 死变量
    #[test]
    fn fm_version_returns_file_name() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.read_file_name = Some("spitfire_mk9.blkx".to_string());
        b.version = Some("2.57.1.35".to_string());
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_fm_version(), "spitfire_mk9.blkx");

        let mut b = BlkxPlaceholder::default();
        b.read_file_name = None;
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_fm_version(), "N/A", "readFileName null → N/A");
    }

    /// 标量直通 getter (blkx 非 null 分支)
    #[test]
    fn scalar_passthrough_getters() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.emptyweight = 2987.0;
        b.maxfuelweight = 412.0;
        b.vne = 790.0;
        b.vne_mach = 0.85;
        b.elav_eff = 580.0;
        b.aileron_eff = 640.0;
        b.rudder_eff = 700.0;
        b.elav_power_loss = 0.12;
        b.aileron_power_loss = 0.08;
        b.rudder_power_loss = 0.05;
        b.avg_eng_recovery_rate = 1.5;
        b.no_flap_wll = 185.5;
        b.full_flap_wll = 132.3;
        b.a_wing = 22.5;
        b.a_fuselage = 7.8;
        b.oswalds_efficiency_number = 0.75;
        b.aspect_ratio = 5.9;
        b.swept_wing_angle = 0.0;
        b.cd_s = 0.38;
        b.ind_cd_f = 0.055;
        b.radiator_cd = 0.02;
        b.oil_radiator_cd = 0.012;
        b.gear_destruction_ind_speed = 320.0;
        b.is_jet = true;
        b.engine_num = 2;
        adapter.set_blkx(Some(Arc::new(b)));

        assert_eq!(adapter.get_empty_weight(), 2987.0);
        assert_eq!(adapter.get_max_fuel_weight(), 412.0);
        assert_eq!(adapter.get_vne(), 790.0);
        assert_eq!(adapter.get_vne_mach(), 0.85);
        assert_eq!(adapter.get_elevator_eff_speed(), 580.0);
        assert_eq!(adapter.get_aileron_eff_speed(), 640.0);
        assert_eq!(adapter.get_rudder_eff_speed(), 700.0);
        assert_eq!(adapter.get_elevator_power_loss(), 0.12);
        assert_eq!(adapter.get_aileron_power_loss(), 0.08);
        assert_eq!(adapter.get_rudder_power_loss(), 0.05);
        assert_eq!(adapter.get_avg_eng_recovery_rate(), 1.5);
        assert_eq!(adapter.get_no_flap_wing_load(), 185.5);
        assert_eq!(adapter.get_full_flap_wing_load(), 132.3);
        assert_eq!(adapter.get_wing_area(), 22.5);
        assert_eq!(adapter.get_fuselage_area(), 7.8);
        assert_eq!(adapter.get_oswalds_efficiency(), 0.75);
        assert_eq!(adapter.get_aspect_ratio(), 5.9);
        assert_eq!(adapter.get_swept_wing_angle(), 0.0);
        assert_eq!(adapter.get_cd_s(), 0.38);
        assert_eq!(adapter.get_ind_cd_f(), 0.055);
        assert_eq!(adapter.get_radiator_cd(), 0.02);
        assert_eq!(adapter.get_oil_radiator_cd(), 0.012);
        assert_eq!(adapter.get_gear_destruction_speed(), 320.0);
        assert!(adapter.is_jet());
        assert_eq!(adapter.get_engine_num(), 2);
    }

    /// getCriticalSpeed: m/s → km/h (×3.6)
    #[test]
    fn critical_speed_converts_to_kmh() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.critical_speed = 250.0;
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_critical_speed(), 900.0);
    }

    /// G 过载公式 (Java: 1.2 * (2*W/(g*weight) ∓ 1)):
    /// raw=[1.5e6, 2.4e6], gross=6000, half=4500, g=9.80 —— 期望值为
    /// Python IEEE double 逐步复算的 oracle dump
    #[test]
    fn g_load_formulas_oracle() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.raw_wing_crit_overload = Some(vec![1_500_000.0, 2_400_000.0]);
        b.grossweight = 6000.0;
        b.halfweight = 4500.0;
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_full_fuel_pos_g(), 96.75918367346937);
        assert_eq!(adapter.get_full_fuel_neg_g(), 62.42448979591836);
        assert_eq!(adapter.get_half_fuel_pos_g(), 129.41224489795917);
        assert_eq!(adapter.get_half_fuel_neg_g(), 82.83265306122448);
    }

    /// rawWingCritOverload=null → 0 (null 守卫)
    #[test]
    fn g_load_null_overload_returns_zero() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.grossweight = 6000.0;
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_full_fuel_pos_g(), 0.0);
        assert_eq!(adapter.get_full_fuel_neg_g(), 0.0);
        assert_eq!(adapter.get_half_fuel_pos_g(), 0.0);
        assert_eq!(adapter.get_half_fuel_neg_g(), 0.0);
    }

    /// WEP/氧化亚氮: nitroTime = nitro / (nitroDecr * 60); nitroDecr<=0 → 0
    #[test]
    fn nitro_time_formula_and_guard() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.nitro = 120.0;
        b.nitro_decr = 0.4;
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_nitro_amount(), 120.0);
        assert_eq!(adapter.get_nitro_time(), 5.0, "120/(0.4*60)=5");
        assert!(adapter.is_nitro_amount_valid());

        // decr = 0: 除零守卫
        let mut b = BlkxPlaceholder::default();
        b.nitro = 120.0;
        b.nitro_decr = 0.0;
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_nitro_time(), 0.0);
        // nitro = 0: 无效 (hide-when-zero)
        let mut b = BlkxPlaceholder::default();
        b.nitro = 0.0;
        b.nitro_decr = 0.4;
        adapter.set_blkx(Some(Arc::new(b)));
        assert!(!adapter.is_nitro_amount_valid());
        // 负 nitro 同样无效
        let mut b = BlkxPlaceholder::default();
        b.nitro = -1.0;
        adapter.set_blkx(Some(Arc::new(b)));
        assert!(!adapter.is_nitro_amount_valid());
    }

    /// 转动惯量: pitch=[2] roll=[0] yaw=[1]; 长度不足守卫; null 守卫
    #[test]
    fn moment_of_inertia_indices_and_guards() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.moment_of_inertia = Some(vec![1000.0, 2000.0, 3000.0]);
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_moi_pitch(), 3000.0);
        assert_eq!(adapter.get_moi_roll(), 1000.0);
        assert_eq!(adapter.get_moi_yaw(), 2000.0);

        // 长度 2: pitch(<3) → 0, yaw(>=2) 正常, roll(>=1) 正常
        let mut b = BlkxPlaceholder::default();
        b.moment_of_inertia = Some(vec![10.0, 20.0]);
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_moi_pitch(), 0.0);
        assert_eq!(adapter.get_moi_roll(), 10.0);
        assert_eq!(adapter.get_moi_yaw(), 20.0);

        // 空数组: 全部 0 (roll 的 <1 守卫)
        let mut b = BlkxPlaceholder::default();
        b.moment_of_inertia = Some(vec![]);
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_moi_roll(), 0.0);

        // null: 全部 0
        let b = BlkxPlaceholder::default();
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_moi_pitch(), 0.0);
        assert_eq!(adapter.get_moi_roll(), 0.0);
        assert_eq!(adapter.get_moi_yaw(), 0.0);
    }

    /// fm_parts (NoFlapsWing/FullFlapsWing/Fuselage/Fin/Stab) 转发与 null 守卫
    #[test]
    fn fm_parts_getters() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.no_flaps_wing = Some(FmPartsPlaceholder::new(0.023, 0.21, 1.65, -0.98, 16.5, -14.2));
        b.full_flaps_wing = Some(FmPartsPlaceholder::new(0.081, 0.85, 2.4, -1.1, 13.0, -12.0));
        b.fuselage = Some(FmPartsPlaceholder::new(0.115, 0.0, 0.0, 0.0, 0.0, 0.0));
        b.fin = Some(FmPartsPlaceholder::new(0.022, 0.0, 0.0, 0.0, 0.0, 0.0));
        b.stab = Some(FmPartsPlaceholder::new(0.018, 0.0, 0.0, 0.0, 0.0, 0.0));
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_no_flaps_wing_cd_min(), 0.023);
        assert_eq!(adapter.get_no_flaps_wing_cl0(), 0.21);
        assert_eq!(adapter.get_no_flaps_wing_cl_crit_high(), 1.65);
        assert_eq!(adapter.get_no_flaps_wing_cl_crit_low(), -0.98);
        assert_eq!(adapter.get_no_flaps_wing_aoa_crit_high(), 16.5);
        assert_eq!(adapter.get_no_flaps_wing_aoa_crit_low(), -14.2);
        assert_eq!(adapter.get_full_flaps_wing_cd_min(), 0.081);
        assert_eq!(adapter.get_full_flaps_wing_cl0(), 0.85);
        assert_eq!(adapter.get_full_flaps_wing_aoa_crit_high(), 13.0);
        assert_eq!(adapter.get_full_flaps_wing_aoa_crit_low(), -12.0);
        assert_eq!(adapter.get_fuselage_cd_min(), 0.115);
        assert_eq!(adapter.get_fin_cd_min(), 0.022);
        assert_eq!(adapter.get_stab_cd_min(), 0.018);

        // 全 null → 0
        adapter.set_blkx(Some(Arc::new(BlkxPlaceholder::default())));
        assert_eq!(adapter.get_no_flaps_wing_cd_min(), 0.0);
        assert_eq!(adapter.get_full_flaps_wing_cl0(), 0.0);
        assert_eq!(adapter.get_fuselage_cd_min(), 0.0);
        assert_eq!(adapter.get_fin_cd_min(), 0.0);
        assert_eq!(adapter.get_stab_cd_min(), 0.0);
    }

    /// 襟翼档位速度: FlapsDestructionNum 是档位数闸门 (非数组长度)
    #[test]
    fn flap_speeds_gated_by_destruction_num() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.flaps_destruction_num = 2;
        b.flaps_destruction_ind_speed = Some(vec![vec![0.0, 350.0], vec![0.0, 280.0], vec![0.0, 240.0]]);
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_flap0_speed(), 350.0);
        assert_eq!(adapter.get_flap1_speed(), 280.0);
        assert_eq!(adapter.get_flap2_speed(), 0.0, "num=2 不放开第 3 档");
        assert_eq!(adapter.get_flap3_speed(), 0.0);
        assert!(adapter.is_flap0_speed_valid());
        assert!(adapter.is_flap1_speed_valid());
        assert!(!adapter.is_flap2_speed_valid());
        assert!(!adapter.is_flap3_speed_valid());

        // num=0: 即便表有数据也全 0
        let mut b = BlkxPlaceholder::default();
        b.flaps_destruction_num = 0;
        b.flaps_destruction_ind_speed = Some(vec![vec![0.0, 350.0]]);
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_flap0_speed(), 0.0);
        assert!(!adapter.is_flap0_speed_valid());

        // 速度 0: valid=false (hide-when-zero 契约)
        let mut b = BlkxPlaceholder::default();
        b.flaps_destruction_num = 1;
        b.flaps_destruction_ind_speed = Some(vec![vec![0.0, 0.0]]);
        adapter.set_blkx(Some(Arc::new(b)));
        assert!(!adapter.is_flap0_speed_valid());

        // 表 null: 全 0
        let mut b = BlkxPlaceholder::default();
        b.flaps_destruction_num = 4;
        adapter.set_blkx(Some(Arc::new(b)));
        assert_eq!(adapter.get_flap0_speed(), 0.0);
    }

    /// set/get_blkx 引用语义 + 换机替换 (volatile 原子替换语义)
    #[test]
    fn set_and_get_blkx_reference() {
        let adapter = FMDataAdapter::new();
        let mut b = BlkxPlaceholder::default();
        b.emptyweight = 1234.0;
        let shared = Arc::new(b);
        adapter.set_blkx(Some(Arc::clone(&shared)));
        // 返回同一实例 (Arc 指针相等 = Java 引用相等)
        let got = adapter.get_blkx().unwrap();
        assert!(Arc::ptr_eq(&got, &shared));
        assert_eq!(adapter.get_empty_weight(), 1234.0);

        // 换机: 整体替换为新实例
        let mut b2 = BlkxPlaceholder::default();
        b2.emptyweight = 5678.0;
        adapter.set_blkx(Some(Arc::new(b2)));
        assert_eq!(adapter.get_empty_weight(), 5678.0);
        // 清空 (Java setBlkx(null))
        adapter.set_blkx(None);
        assert!(adapter.get_blkx().is_none());
        assert_eq!(adapter.get_empty_weight(), 0.0);
    }

    /// P3 跨线程可见性: 后台线程 setBlkx → 读取线程立即可见
    /// (LIFETIMES: FM-Loader 线程写 / BaseOverlay.run() 线程读)
    #[test]
    fn cross_thread_blkx_replacement_is_visible() {
        let adapter = Arc::new(FMDataAdapter::new());
        let writer = Arc::clone(&adapter);
        let handle = std::thread::spawn(move || {
            let mut b = BlkxPlaceholder::default();
            b.emptyweight = 4242.0;
            b.is_jet = true;
            writer.set_blkx(Some(Arc::new(b)));
        });
        handle.join().unwrap();
        assert_eq!(adapter.get_empty_weight(), 4242.0);
        assert!(adapter.is_jet());
    }

    /// trait 对象分发: FMUnpackedDataOverlay 经 FMDataSource 接口消费适配器
    #[test]
    fn fm_data_source_object_dispatch() {
        let adapter = FMDataAdapter::new();
        let src: Box<dyn FMDataSource> = Box::new(adapter);
        assert_eq!(src.get_fm_version(), "");
        assert_eq!(src.get_engine_num(), 0);
    }
}
