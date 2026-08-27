// PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
// 不改成 struct 字面量以保持与 Java 测试源逐行对应
#![allow(clippy::field_reassign_with_default)]

use super::*;
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicU32;
use std::time::Duration;
use vm_core::bus::EventBus;
use vm_core::fm::status::FMStatus;

/// 真机抓取的 /state 快照 (state.rs / service_fields.rs 测试同源,
/// 断言值 = Java 8 oracle 实测; mock 契约: 冒号后一空格)
const STATE_MOCK: &str = "{\"valid\": true,\"aileron, %\": -48,\"elevator, %\": 20,\"rudder, %\": -47,\"flaps, %\": 0,\"gear, %\": 0,\"H, m\": 46,\"TAS, km/h\": 454,\"IAS, km/h\": 474,\"M\": 0.39,\"AoA, deg\": -1.6,\"AoS, deg\": -5.9,\"Ny\": 0.35,\"Vy, m/s\": -7.3,\"Wx, deg/s\": -34,\"Mfuel, kg\": 197,\"Mfuel0, kg\": 734,\"throttle 1, %\": 110,\"RPM throttle 1, %\": 100,\"mixture 1, %\": 100,\"radiator 1, %\": 42,\"magneto 1\": 3,\"power 1, hp\": 1597.8,\"RPM 1\": 3001,\"manifold pressure 1, atm\": 2.24,\"water temp 1, C\": 121,\"oil temp 1, C\": 90,\"pitch 1, deg\": 35.5,\"thrust 1, kgs\": 840,\"efficiency 1, %\": 87}";

/// p51d /indicators 快照 (s2_preview_live 场景同源数据的手工裁剪版,
/// 保 Deriver 判定所需字段; type/vario/compass 为快照原值)
const INDIC_MOCK: &str = "{\"valid\": true, \"army\": \"air\", \"type\": \"p-51d-20_china\", \"speed\": 131.007797, \"vario\": -7.342558, \"aviahorizon_roll\": -40.553505, \"aviahorizon_pitch\": 0.632352, \"compass\": 164.09729}";

fn new_service() -> Service {
    let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
    let bus = Arc::new(FlightDataBus::new());
    Service::new(ServiceConfig::default(), fm, bus)
}

/// Java 构造器 + resetvaria 接线逐项核对 (service_fields.rs 的
/// "Default 是声明态而非构造后态" 验收义务清单)
#[test]
fn constructor_wiring_matches_java() {
    let svc = new_service();
    let d = svc.data.read().unwrap();
    // 构造器: freq = serviceLoopIntervalMs
    assert_eq!(d.freq, 50);
    // ratio = freq / 1000.0f (float 除法拓宽), ratio_1 = 1.0f - ratio
    let ratio = (50f32 / 1000.0f32) as f64;
    assert_eq!(d.ratio, ratio);
    assert_eq!(d.ratio_1, 1.0 - ratio);
    // mapinfo/sState/sIndic 构造 (构造器段, resetvaria 之后)
    assert!(d.mapinfo.is_some());
    assert!(d.s_state.is_some());
    assert!(d.s_indic.is_some());
    // power/pitch/thrust/efficiency = new String[maxEngNum] (null 填充,
    // 覆盖 resetvaria 的 4 长度 nastring 数组)
    for arr in [&d.power, &d.pitch, &d.thrust, &d.efficiency] {
        assert_eq!(arr.as_ref().unwrap().len(), MAX_ENG_NUM);
        assert!(arr.as_ref().unwrap().iter().all(|e| e.is_none()));
    }
    // resetvaria 关键初值 (Java L1528-1660)
    assert_eq!(d.loc, Some([0.0; 2]));
    assert_eq!(d.dir, Some([0.0; 2]));
    assert_eq!(d.radio_alt_valid, Some(false));
    assert!(!d.player_live);
    assert_eq!(d.i_eng_type, ENGINE_TYPE_UNKNOWN);
    assert_eq!(d.fueltime, i64::MAX, "Long.MAX_VALUE");
    assert_eq!(d.maximum_thr_rpm, 1.0);
    assert_eq!(d.iastotascoff, 1.0);
    // Java: Float.MAX_VALUE 拓宽 double
    assert_eq!(d.flap_allow_speed, f32::MAX as f64);
    assert_eq!(d.flap_allow_angle, f32::MAX as f64);
    assert_eq!(d.cur_load_min_work_time, 99999000.0);
    assert_eq!(d.svalid.as_deref(), Some("false"));
    // 字符串族 = nastring
    assert_eq!(d.fueltime_str.as_deref(), Some("-"));
    assert_eq!(d.ias.as_deref(), Some("-"));
    assert_eq!(d.compass.as_deref(), Some("-"));
    // 仅构造的三个 SMA 有值 (sum/energyDiff/fuelTime); calc/diff/sep/turnrds
    // 四槽保持 None (状态双主裁决: 真人在 Deriver)
    assert!(d.sum_speed_sma.is_some());
    assert!(d.energy_diff_sma.is_some());
    assert!(d.fuel_time_sma.is_some());
    assert!(d.calc_speed_sma.is_none());
    assert!(d.diff_speed_sma.is_none());
    assert!(d.sep_sma.is_none());
    assert!(d.turnrds_sma.is_none());
    // 燃油窗口 (1000/50=20) / fuelTimeSMA 窗口 4
    // (SMA 无窗口查询接口, 以行为验证: sum 首值预热段均值语义)
    // FuelCheckMili/lastMapPollTimeMs/lastMainLoopTimeMs ≈ 构造时刻
    let now = current_time_millis();
    assert!((d.fuel_check_mili - now).abs() < 60_000);
    // PORT: Java 构造序对 fuel_check_mili 是二次独立 System.currentTimeMillis
    // (跨毫秒边界差 1ms 属正常语义, 断言容忍 ±1, 消除 flaky)
    assert!((d.last_map_poll_time_ms - d.fuel_check_mili).abs() <= 1);
    assert!((d.last_main_loop_time_ms - d.fuel_check_mili).abs() <= 1);
    // R2 守卫: fresh manager 的 current = UNRESOLVED → nitro 族归零
    assert_eq!(d.nitrokg, 0.0);
    assert!(d.fm.blkx.is_none());
    // 构造期 publish 已发生 (resetvaria 尾部; mapinfo 此刻仍 null → "--",
    // sState 构造在 resetvaria 后 → state=None) —— 由下方事件测试覆盖
}

/// 构造期事件 (resetvaria 尾部 publish): mapGrid="--"/state=None 载荷窗口
#[test]
fn constructor_publishes_initial_event() {
    let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
    let bus = Arc::new(FlightDataBus::new());
    let seen = Arc::new(std::sync::Mutex::new(Vec::<(String, bool, bool)>::new()));
    let s2 = Arc::clone(&seen);
    let _sub = bus.register(move |e: &FlightDataEvent| {
        let p = e.get_payload();
        s2.lock().unwrap().push((
            p.map_grid.clone(),
            e.get_state().is_some(),
            e.get_indicators().is_some(),
        ));
    });
    let _svc = Service::new(ServiceConfig::default(), fm, Arc::clone(&bus));
    let v = seen.lock().unwrap();
    assert_eq!(v.len(), 1, "构造期恰发布一次初始事件");
    // mapinfo 在 clearvaria 之后才构造 → mapGrid 走 "--" 分支 (Java 同窗口)
    assert_eq!(v[0].0, "--");
    // sState 在 resetvaria 之后才构造 → 载荷 state/indicators = null
    assert!(!v[0].1);
    assert!(!v[0].2);
}

/// 单周期直驱 (不起线程): 解析→playerLive→identify→Deriver→发布 全链
#[test]
fn process_polling_cycle_full_chain() {
    let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
    let bus = Arc::new(FlightDataBus::new());
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let _sub = bus.register(move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
    });
    let mut svc =
        Service::new(ServiceConfig::default(), Arc::clone(&fm), Arc::clone(&bus));
    let base = hits.load(Ordering::SeqCst); // 构造期 1 次

    // 预填 http 响应缓冲 (run 循环里 getReqResult 的产物)
    *svc.http_client.str_state.lock().unwrap() = STATE_MOCK.to_string();
    svc.http_client.str_indic = INDIC_MOCK.to_string();

    svc.process_polling_cycle();

    // sState 解析 (Java 8 oracle 值)
    {
        let d = svc.data.read().unwrap();
        let s = d.s_state.as_ref().unwrap();
        assert_eq!(s.ias, 474);
        assert_eq!(s.rpm, 3001);
        assert_eq!(s.total_thr, 840.0);
        // sIndic.type: toUpperCase + 去引号
        assert_eq!(
            d.s_indic.as_ref().unwrap().r#type.as_deref(),
            Some("P-51D-20_CHINA")
        );
        // totalThr != 0 → playerLive
        assert!(d.player_live);
        // Deriver 写回: vario(indicators 优先) / compass / an
        assert!((d.n_vy - (-7.342558f32 as f64)).abs() < 1e-6);
        assert!((d.compass_delta - 164.09729f32 as f64).abs() < 1e-4);
        assert!(d.an > 0.0, "An = g*sqrt(Ny²+1-2Ny·cos(roll)·cos(pitch+AoA))");
        // R2 hasFM 守卫 (Java updateSpeedRatio L1191-1199): 本轮 identify 的
        // 异步加载尚未完成 → blkx None → 整方法早退, mach 保持初值 0
        // (无 FM 机型不得进 hide-when-zero 显示; 无守卫时的 0.39 是越权计算)
        assert_eq!(d.mach, 0.0, "无 FM 时 updateSpeedRatio 早退, mach 保持 0");
        // updateAlt 写回: alt←H,m; mock 无 radio_altitude 键 → 哨兵 →
        // radioAlt=alt 且 radioAltValid=false (EventPayload 载荷源)
        assert_eq!(d.alt, 46.0);
        assert_eq!(d.radio_alt, 46.0);
        assert_eq!(d.radio_alt_valid, Some(false));
        // mapGrid: loc=[0,0] + mapinfo(构造后仍全 0) → 'A' + 1
    }
    // identify 已建立目标 (规范化小写); loader 线程尝试磁盘加载 (data/ 缺失
    // → MISSING 落负缓存, 不影响本断言)
    assert_eq!(
        fm.current_target_name().as_deref(),
        Some("p-51d-20_china")
    );
    // 事件: 构造 1 次 + 本周期 1 次
    assert_eq!(hits.load(Ordering::SeqCst), base + 1);
    // calcPeriod 后缀自增
    assert_eq!(svc.data.read().unwrap().calc_period, 1);
    // 未翻转端口 (响应有效)
    assert_eq!(svc.data.read().unwrap().port_ocupied, Some(false));
}

/// updateCompass 地图回退 + updateAlt 英制/无线电分支 (calculate 写回段,
/// Java L739-775 / L1101-1113)
#[test]
fn update_compass_fallback_and_update_alt_branches() {
    let mut svc = new_service();
    // indicators: 无 compass 键 (→ 哨兵 -65535 走地图回退), 带 radio_altitude
    // (英尺) / altitude_10k (跳变源); army/头部字段契约同 INDIC_MOCK
    let indic = "{\"valid\": true, \"army\": \"air\", \"type\": \"p-51d-20_china\", \
                     \"speed\": 131.0, \"vario\": -7.3, \"aviahorizon_roll\": -40.5, \
                     \"aviahorizon_pitch\": 0.6, \"radio_altitude\": 1000.0, \
                     \"altitude_10k\": 10.0}";
    {
        let mut d = write_data(&svc.data);
        d.s_state.as_mut().unwrap().update(STATE_MOCK);
        d.s_indic.as_mut().unwrap().update(indic);
        // 地图方向 (run() 的 getPlayerDir 产物): dir[1]<0 分支
        d.dir = Some([0.1, -1.0]);
        // run() 的区间量化产物 (卡顿轮 diffTime=100 → actualIntervalMs=100)
        d.actual_interval_ms = 100;
        // 冻结英制状态机 (notCheckInch=true), checkAlt>0 → 英尺转米分支
        d.not_check_inch = true;
        d.check_alt = 500;
    }
    svc.calculate();
    {
        let d = read_data(&svc.data);
        // compass 哨兵 → 地图回退: (360 - atan(dir0/dir1)·deg) % 360
        let expect = (360.0 - (0.1f64 / -1.0f64).atan().to_degrees()) % 360.0;
        assert!(
            (d.compass_delta - expect).abs() < 1e-9,
            "地图方向回退 (实际 {})",
            d.compass_delta
        );
        // updateAlt: altp←alt(初值 0), alt←H,m; altmeterp←0, altmeter←10
        assert_eq!(d.altp, 0.0);
        assert_eq!(d.alt, 46.0);
        assert_eq!(d.altmeterp, 0.0);
        assert_eq!(d.altmeter, 10.0);
        // radio_altitude 有效 + checkAlt>0 → ×0.3048f (float 提升域, ≠ 0.3048)
        assert_eq!(d.radio_alt_valid, Some(true));
        assert!((d.radio_alt - 1000.0 * (0.3048f32 as f64)).abs() < 1e-9);
        // dRadioAlt = ratio_1*0 + ratio*1000*(radioAlt-0)/100 (freq=50;
        // ratio = freq/1000.0f 的 float 除法拓宽值, 非精确 0.05)
        let ratio = (50f32 / 1000.0f32) as f64;
        let expect_dralt = ratio * 1000.0 * (1000.0 * (0.3048f32 as f64)) / 100.0;
        assert!((d.d_radio_alt - expect_dralt).abs() < 1e-9);
    }

    // 英制状态机活分支: notCheckInch=false + altmeter 跳变量 >> |2·Vy·interval|
    // (|10-0|·1000=10000 > |2·(-7.3)·100|=1460) → checkAlt += actualIntervalMs
    {
        let mut d = write_data(&svc.data);
        d.not_check_inch = false;
        d.check_alt = 0;
        d.altmeter = 0.0;
        d.altmeterp = 0.0;
    }
    svc.calculate();
    {
        let d = read_data(&svc.data);
        assert_eq!(d.check_alt, 100, "checkAlt += actualIntervalMs");
        assert!(!d.not_check_inch, "|100| ≤ 10000 不置 notCheckInch");
        // altmeterp ← 改写前的 altmeter (手工置 0), altmeter ← 本轮解析值 10
        assert_eq!(d.altmeterp, 0.0);
        assert_eq!(d.altmeter, 10.0);
    }
}

/// updateSpeedRatio/updateStallSpeed (Java L1185-1231 / L1236-1266) —
/// MiniHUD 速度比值 bar 的数据源。无 FM 归零 / 有 FM 走 python 位级 oracle
/// (真机 spitfire_f24 blkx 的 getload f32 拓宽域值 + STATE_MOCK ias=474/
/// heightm=46: iasRatio 0.5417… > machRatio 0.4460… 走 ias 分支; 失速
/// flaps 0/50 两档)。data/ 缺失时仅跑无 FM 域 (对齐 build.py 跳过语义)。
#[test]
fn update_speed_ratio_and_stall_speed_oracle() {
    let mut svc = new_service();

    // 无 FM (fresh manager = UNRESOLVED): R2 守卫 → 三比值归零, stall 保持 0
    {
        let mut d = write_data(&svc.data);
        d.s_state.as_mut().unwrap().update(STATE_MOCK);
    }
    svc.calculate();
    {
        let d = read_data(&svc.data);
        assert_eq!(d.speed_limit_ratio, 0.0, "无 FM 比值归零");
        assert_eq!(d.aileron_lock_ratio, 0.0);
        assert_eq!(d.stall_speed, 0.0, "无 FM 失速保持上次值 (初始 0)");
    }

    // 有 FM: 真机 spitfire 全量装载 (getload 波次产物)
    let phys = format!(
        "{}/../../../data/aces/gamedata/flightmodels/fm/spitfire_f24.blkx",
        env!("CARGO_MANIFEST_DIR")
    );
    if !std::path::Path::new(&phys).exists() {
        return; // data/ 未解包
    }
    let blkx = vm_core::blkx::Blkx::parse(&phys).unwrap();
    let fm = FMHandle::ready(Some("spitfire_f24".to_string()), Some(blkx), 0.0, 0.0, None);

    svc.update_speed_ratio(&fm);
    {
        let d = read_data(&svc.data);
        // python oracle (f32 拓宽域公式直算, 位级)
        assert_eq!(d.speed_limit_ratio, 0.5417142857142857, "iasRatio = 474/875");
        assert_eq!(d.aileron_lock_ratio, 0.5508571428571428, "482/875");
        assert_eq!(d.rudder_lock_ratio, 0.45714285714285713, "400/875");
        assert_eq!(
            d.unit_mach_limit_ratio, 1.3962520958006088,
            "iasPerMach/875 (ias 分支)"
        );
    }

    // 失速: flap=0 (STATE_MOCK "flaps, %": 0; mfuel=197)
    svc.update_stall_speed(&fm);
    assert_eq!(
        svc.data.read().unwrap().stall_speed,
        158.26201720161404,
        "flap=0 失速 (python oracle)"
    );
    // flap=50 → 襟翼线性混合
    {
        let mut d = write_data(&svc.data);
        d.s_state.as_mut().unwrap().flaps = 50;
    }
    svc.update_stall_speed(&fm);
    assert_eq!(
        svc.data.read().unwrap().stall_speed,
        143.78318105378034,
        "flap=50 失速 (python oracle)"
    );
}

/// updateWepTime/updateTemp/checkEngineJet/updateEngineState/updateFuel
/// (Java L707-723/L726-737/L484-514/L883-962/L964-984) 全链 — EngineInfo/
/// EngineControl 的功率/动力量/油量/温度数据源。python oracle (f32 域):
/// STATE_MOCK power1=1597.8hp/thrust1=840kgs/throttle1=110 + INDIC_MOCK
/// speed=131.007797 → speedv=126.111… → hpEff=1412/avgeff=88.42。
#[test]
fn engine_state_and_fuel_full_chain() {
    let mut svc = new_service();
    // 预填 http 响应 → 解析 (STATE_MOCK/INDIC_MOCK)
    *svc.http_client.str_state.lock().unwrap() = STATE_MOCK.to_string();
    svc.http_client.str_indic = INDIC_MOCK.to_string();
    svc.process_polling_cycle();
    // 解析后补: 引擎数/油门(>100 进 WEP)/跳过投票状态机 (单轮投票不收敛,
    // 收敛语义由 check_engine_jet_voting_state_machine 单独覆盖)
    {
        let mut d = write_data(&svc.data);
        let s = d.s_state.as_mut().unwrap();
        s.engine_num = 1;
        s.throttles[0] = 110;
        d.check_engine_flag = true;
        d.i_eng_type = ENGINE_TYPE_PROP;
        d.poll_cycle_duration_ms = 50; // run() 轮询的量化产物 (直驱 calculate 需手工模拟)
    }
    svc.calculate();

    {
        let d = read_data(&svc.data);
        // updateEngineState (活塞分支, python oracle)
        assert_eq!(d.total_hp, 1597, "(int) 1597.8");
        assert_eq!(d.total_thrust, 840, "thrust 1 = 840 kgs");
        assert_eq!(d.total_hp_eff, 1412, "840·g·speedv(126.111…)/735 截断");
        assert!((d.avgeff - 88.41577958672511).abs() < 1e-9, "avgeff (实际 {})", d.avgeff);
        // thurst_percent: 无 FM (UNRESOLVED) → peak=0 且 maxTotalHp 已积累但
        // 首轮 pThurst 置换后分支不触发? — maxTotalHp 分支: peakPower=0 且
        // maxTotalHp=70 != 0 → thurstPercent = 100*1597/70 = 2281.4…
        // (Java 同式回退, 无 FM 域的已知大数形态)
        // thurst_percent: 无 FM (UNRESOLVED) → peak=0 走 maxTotalHp 回退 — 两轮
        // EMA: 首轮 max=70, 次轮 max=(int)(0.95*70+0.05*1412)=137 → 100*1597/137
        // (State::update 已从遥测键推断 engineNum=1/throttles[0]=110, 首轮即完整计算)
        assert!((d.thurst_percent - 100.0 * 1597.0 / 137.0).abs() < 1e-9);
        // maxTotal 平滑 (EMA ratio=0.05), 两轮: 42 → (int)(0.95*42+0.05*840)=81
        assert_eq!(d.max_total_thr, 81, "第二轮 EMA (0.95*42+0.05*840)");
        assert_eq!(d.max_total_hp, 137, "(int)(0.95*70+0.05*1412)");
        // updateWepTime: 两轮都进 WEP, 但首轮 pollCycleDurationMs=0 (run() 未跑),
        // 仅第二轮 (手工置 50) 计入 → 50
        assert_eq!(d.wep_time, 50, "次轮 WEP 计时 (首轮 pollCycle=0)");
        assert_eq!(d.nitro_eng_nr, 1);
        assert_eq!(d.nitrokg, 0.0, "R2 守卫: 无 FM nitrokg=0");
        // updateTemp: INDIC_MOCK 无温度键 (哨兵) → 回退 sState
        assert_eq!(d.noil_temp, 90.0, "oil temp 1 = 90");
        assert_eq!(d.nwater_temp, 121.0, "water temp 1 = 121");
        // updateFuel: fuelnum=0 → lowAccFuel + mfuel 回退
        assert_eq!(d.total_fuel, 197.0, "Mfuel 回退");
        assert!(d.low_acc_fuel);
        assert_eq!(d.fuel_percent, 26, "(int)(100*197/734)");
    }

    // 喷气分支: iEngType 翻 JET → totalHp 归 0
    {
        let mut d = write_data(&svc.data);
        d.i_eng_type = ENGINE_TYPE_JET;
    }
    let fm = svc.fm_manager.current(); // UNRESOLVED
    svc.update_engine_state(&fm);
    {
        let d = read_data(&svc.data);
        assert_eq!(d.total_hp, 0, "喷气分支不产马力");
        assert_eq!(d.total_thrust, 840);
        assert_eq!(d.avgeff, 0.0);
    }
}

/// checkEngineJet 投票状态机 (L484-514): 磁电机正/负票 + 桨距票 → 100 票收敛
#[test]
fn check_engine_jet_voting_state_machine() {
    let mut svc = new_service();
    {
        let mut d = write_data(&svc.data);
        let s = d.s_state.as_mut().unwrap();
        // 活塞投票: 磁电机 3 (>=0 正票), 桨距 35.5 有效 (正票)
        s.magenato = 3;
        s.pitch = vec![35.5];
    }
    for _ in 0..99 {
        svc.check_engine_jet();
    }
    {
        let d = read_data(&svc.data);
        assert!(!d.check_engine_flag, "99 票未收敛");
        assert_eq!(d.check_engine_type, 99);
        assert_eq!(d.check_pitch, 99, "桨距有效 (非哨兵) 是正票 (Java: != -65535 → ++)");
    }
    svc.check_engine_jet(); // 第 100 票
    {
        let d = read_data(&svc.data);
        assert!(d.check_engine_flag);
        assert_eq!(d.i_eng_type, ENGINE_TYPE_PROP, "磁电机正票 → 活塞");
    }

    // 涡桨/喷气分流: 磁电机负票 + 桨距正票 → 涡桨; 桨距哨兵 → 喷气
    let mut svc2 = new_service();
    {
        let mut d = write_data(&svc2.data);
        let s = d.s_state.as_mut().unwrap();
        s.magenato = -1;
        s.pitch = vec![35.5];
    }
    for _ in 0..100 {
        svc2.check_engine_jet();
    }
    assert_eq!(
        read_data(&svc2.data).i_eng_type,
        ENGINE_TYPE_TURBOPROP,
        "磁电机负 + 桨距正 → 涡桨"
    );

    let mut svc3 = new_service();
    {
        let mut d = write_data(&svc3.data);
        let s = d.s_state.as_mut().unwrap();
        s.magenato = -1;
        s.pitch = vec![-65535.0];
    }
    for _ in 0..100 {
        svc3.check_engine_jet();
    }
    assert_eq!(
        read_data(&svc3.data).i_eng_type,
        ENGINE_TYPE_JET,
        "磁电机负 + 桨距哨兵 → 喷气"
    );
}

/// 空响应 → conState=-1 → 端口翻转 (Java L1785-1795); 再翻回
#[test]
fn port_flip_on_empty_response() {
    let mut svc = new_service();
    // http 缓冲保持初始 NSTRING ("") → 走等待连接分支
    assert!(svc.http_client.str_indic.is_empty());
    svc.process_polling_cycle();
    assert_eq!(svc.data.read().unwrap().port_ocupied, Some(true));
    svc.process_polling_cycle();
    assert_eq!(svc.data.read().unwrap().port_ocupied, Some(false));
}

/// §6 契约: 顶层 catch_unwind——单轮 panic (Boolean 拆箱 NPE 复刻) 不杀线程,
/// 线程经 stop 正常退出 (join ok = panic 未逃逸 run)。
/// 随机端口假游戏端口供数 (不与 mock_e2e 的 9222 或真机 8111 冲突), 响应按 send_get_fast_buf
/// 的单次 read 契约一次性 write。
#[test]
fn catch_unwind_keeps_thread_alive() {
    // 假游戏端口 (bind :0 随机分配, 无并行冲突)
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => break,
            };
            let mut req = [0u8; 8192];
            let _ = stream.read(&mut req); // 单次读请求 (GET 行即可)
            let req = String::from_utf8_lossy(&req).to_string();
            let body = if req.contains("/indicators") {
                INDIC_MOCK
            } else {
                STATE_MOCK // /state 与 map 端点宽容回同一份
            };
            // 头标签避开 "type"/"valid" 子串 (getString 抢先命中即坏,
            // mock_8111.py 同款契约)
            let resp = format!("HTTP/1.1 200 OK\r\nCache-Control: no-cache\r\n\r\n{body}");
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
    let bus = Arc::new(FlightDataBus::new());
    let mut cfg = ServiceConfig::default();
    cfg.service_loop_interval_ms = 20;
    cfg.app_port = port;
    let svc = Service::new(cfg, fm, bus);
    // 注入拆箱 panic 源: publish 的 fatal_warn.unwrap() (Java Boolean null
    // 拆箱 NPE 的同构物, 由 run 顶层 catch_unwind 兜住)。注入在构造之后
    // (构造期 resetvaria 会写回 Some(false))
    svc.data.write().unwrap().fatal_warn = None;

    let mut handle = start(svc);
    std::thread::sleep(Duration::from_millis(400));
    let t0 = std::time::Instant::now();
    // 线程每轮 publish 必 panic: 若 catch_unwind 缺位, 首轮 panic 即杀线程
    // → join 拿到 Err (stop 返回 false)
    assert!(handle.stop(), "线程应经 Interrupted/恢复检查出口正常退出, 而非被 panic 杀死");
    // 恢复 sleep (1000ms) 因 stop 电平提前醒, 退出不应拖满 1s
    assert!(
        t0.elapsed() < Duration::from_millis(900),
        "stop 应在恢复睡眠内快速退出 (实际 {:?})",
        t0.elapsed()
    );
}

/// 规则 2 e2e: mock_8111.py 起 s2_preview_live, Service 跑 3 秒,
/// 断言事件数>0 且 ServiceData 的 ias/vario 与 mock 快照一致。
/// 白盒测试端口约定 (用户指令): 9222 = Java 备用端口 (appPortBkp) 域,
/// 游戏本地 API 恒占 8111 而 9222 游戏永不监听 — 真机在跑测试也不被挤掉。
#[test]
fn mock_e2e_s2_preview_live() {
    const TEST_PORT: u16 = 9222;
    // 端口占用探测: 其他白盒测试/mock 在跑 → 跳过本测试
    // PORT(探测形态, 真机踩坑): 原 8111 + bind 探测有两重坑 —— (a) bind 探测对
    // 0.0.0.0 通配监听者 (战雷 aces.exe) 假阴性: Windows 允许 127.0.0.1 特定
    // 地址 bind 成功, mock 抢绑失败退出后 Service 连的是游戏, 断言对着静态
    // 快照必炸 (实测真机 IAS 593 ≠ 474); (b) 8111 本就是游戏端口。改 connect
    // 探测 + 9222 后两坑皆除 (探测对任何在场监听者恒真)。
    if TcpStream::connect(("127.0.0.1", TEST_PORT)).is_ok() {
        println!("跳过: 127.0.0.1:{TEST_PORT} 已有监听者 (其他 mock/白盒测试在跑), e2e 让位");
        return;
    }

    // 起 mock (失败也要清理 → KillOnDrop 兜底)
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../script/mock_8111.py");
    let child = Command::new("python")
        .arg(&script)
        .arg("serve")
        .arg("--port")
        .arg(TEST_PORT.to_string())
        .arg("--scenario")
        .arg("s2_preview_live")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("启动 mock_8111.py 失败 (python 不在 PATH?)");
    struct KillOnDrop(Child);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let _mock = KillOnDrop(child);

    // 等 mock 就绪 (最多 10s)
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if TcpStream::connect(("127.0.0.1", TEST_PORT)).is_ok() {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!("mock_8111.py 10s 内未在 {TEST_PORT} 就绪");
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Service 全链 (config: 50ms / 9222 — 白盒端口约定, 见测试头注)
    let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
    let bus = Arc::new(FlightDataBus::new());
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let _sub = bus.register(move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
    });
    let svc = Service::new(
        ServiceConfig {
            app_port: TEST_PORT,
            ..ServiceConfig::default()
        },
        Arc::clone(&fm),
        bus,
    );
    let mut handle = start(svc);

    // 跑 3 秒
    std::thread::sleep(Duration::from_secs(3));

    let n = hits.load(Ordering::SeqCst);
    assert!(n > 0, "3 秒内应收到 FlightDataEvent (实际 {n})");
    {
        let d = handle.data.read().unwrap();
        let s = d.s_state.as_ref().expect("sState 已解析");
        // p51d 快照: IAS 474 km/h
        assert_eq!(s.ias, 474, "IAS 应与 mock 快照一致");
        // vario: indicators.vario = -7.342558 (Float.parseFloat f32 拓宽域)
        assert!(
            (d.n_vy - (-7.342558f32 as f64)).abs() < 1e-6,
            "vario 应与 mock 快照一致 (实际 {})",
            d.n_vy
        );
        assert!(d.player_live, "totalThr=840 → 玩家存活");
        assert_eq!(s.valid.as_deref(), Some("true"));
    }
    // identify 链已建立 (mock 机型 p-51d-20_china)
    assert_eq!(
        fm.current_target_name().as_deref(),
        Some("p-51d-20_china")
    );
    // FM 加载终态: 项目 data/ 有 p51d 则 READY, 无则 MISSING (两者皆合法;
    // 只断言离开 UNRESOLVED)
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while fm.current().status == FMStatus::Loading && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_ne!(fm.current().status, FMStatus::Unresolved);

    handle.stop();
    // _mock Drop 时杀掉 mock 进程
}

// ------------------------------------------------------------------
// FlightLog 接线 (Service.java:1824-1828 的 logTick 调用面)
// ------------------------------------------------------------------

/// ControllerLogSink no-op (本测试不覆盖 init 失败路径, 见 vm-core flight_log 测试)
struct NopSink;
impl vm_core::flight_log::ControllerLogSink for NopSink {
    fn set_logon(&self, _logon: bool) {}
}

/// 槽注入 → 数轮 tick → CSV 行数 (首 tick flush / 其后缓冲, close 兜底);
/// analyze 链活体验证 (checkAlt 过阈值 → fA 落地 + AnalyzerService 活读 ServiceData)
#[test]
fn flight_log_tick_writes_rows_and_close_flushes() {
    // CWD 沙箱: FlightLog 的 records/ 是相对 CWD 的硬编码 (与 Java 一致);
    // 本测试二进制内并行线程共享进程 CWD → 串行化 + 用完恢复 (vm-core
    // flight_log.rs 同款; 与 vm-core 测试是不同进程, 天然互斥)
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let root = std::env::temp_dir().join(format!("vm_svc_fl_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("records")).unwrap();
    let old = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut svc = new_service();
        let slot: FlightLogSlot = Arc::new(std::sync::Mutex::new(None));
        svc.set_flight_log(Arc::clone(&slot));
        // ServiceData 喂判别值 (check_alt 过阈值 → 首帧 analyze 触发 init)
        {
            let mut d = write_data(&svc.data);
            d.elapsed_time = 120000;
            d.check_alt = 15;
            d.alt = 1500.0;
            d.total_thrust = 3400;
            if let Some(s) = d.s_state.as_mut() {
                s.ny = 1.5;
                s.wx = -140.7;
                s.elevator = -20;
                s.aileron = 15;
                s.rudder = -5;
                s.compressorstage = 1;
                s.magenato = 1;
            }
            d.s_indic.as_mut().unwrap().r#type = Some("bf-109e-4".to_string());
        }
        // FlightLog init (表头落盘) 并入槽 = Java openpad 的 Log=new + init
        let mut fl = vm_core::flight_log::FlightLog::new();
        let snap = flight_log_snapshot(&read_data(&svc.data));
        fl.init(
            Arc::new(NopSink),
            &snap,
            None,
            Arc::new(|_| {}),
            Arc::new(ServiceAnalyzerSource::new(Arc::clone(&svc.data))),
        );
        let file_name = fl.file_name.clone();
        *slot.lock().unwrap() = Some(Arc::new(std::sync::Mutex::new(fl)));

        // 3 轮 tick: t=0 命中 %1024==0 flush (表头+首行落盘), t=1/2 留缓冲
        svc.flight_log_tick();
        svc.flight_log_tick();
        svc.flight_log_tick();
        let rows = std::fs::read_to_string(&file_name).unwrap().lines().count();
        assert_eq!(rows, 2, "首 tick flush 一行, 其余在 BufferedWriter 内存");
        // 快照映射: elapsed 120000ms→2.0 分钟列 + String 列为 resetvaria 的 "-" 初值
        let line1 = std::fs::read_to_string(&file_name)
            .unwrap()
            .lines()
            .nth(1)
            .unwrap()
            .to_string();
        assert!(line1.starts_with("2.0,-,"), "elapsed/ias 列映射: {line1}");
        assert_eq!(line1.split(',').count(), 32, "31 列 + 尾随逗号: {line1}");

        // analyze 链活体: fA 落地 (stage 15) + 活读 ServiceData (thrust 3400)
        {
            let g = slot.lock().unwrap();
            let fl = g.as_ref().unwrap().lock().unwrap();
            let fa = fl.f_a.as_ref().expect("checkAlt 过阈值应触发 FlightAnalyzer init");
            assert_eq!(fa.curalt_stage, 15);
            assert_eq!(fa.initalt_stage, 15);
            assert_eq!(fa.thrust[15], 3400, "AnalyzerService 活读 totalThrust");
        }

        // close 兜底 flush + 三份分析 CSV (fA 已落地, 无 Java NPE 路径)
        let fl_arc = slot.lock().unwrap().take().unwrap();
        fl_arc.lock().unwrap().close();
        let rows = std::fs::read_to_string(&file_name).unwrap().lines().count();
        assert_eq!(rows, 4, "close 兜底 flush 表头+3 行");
    }));
    std::env::set_current_dir(old).unwrap();
    let _ = std::fs::remove_dir_all(&root);
    drop(_guard); // 先放锁再重抛, 避免 panic 穿过 guard 毒化锁
    if let Err(e) = r {
        std::panic::resume_unwind(e);
    }
}
