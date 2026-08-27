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
