//! calculate 链引擎族方法: 最大转速 (FM 直取, 无 FM 自适应学习) 与最佳
//! 增压器档位/失配检测。原 Service 版襟翼族 (checkFlap/getFlapAllowSpeed/
//! getFlapAllowAngle) 已 W8 公式化或合一至 vm-core 共享实现 (checkWing 已删:
//! 产物无消费者, registry wing_sweep_valid 直通替代)。
//!
//! PORT(模块边界): impl Service 跨文件块, 方法一律 pub(super); calculate 内的
//! 接线调用统一在 service_loop.rs。
//! PORT(同名陷阱): vm-core fm/data/flap_limits.rs 另有**公共** get_flap_allow_angle
//! —— 那是 HUDCalculator 版, 与本模块的 Service 版不同源, 互不复用互不可见;
//! 本文件按 Java Service 版逐行直译。

use super::Service;
use vm_core::fm::piston_model::find_optimal_stage_index;
use vm_core::fm::FMHandle;

impl Service {
    /// 获取最大转速（优先 FM, 无 FM 时自适应学习）。
    ///
    /// PORT(命名避让, service_fields.rs 字段区备注): Java 字段 getMaximumRPM
    /// (boolean) 与本方法构成 Java 同名重载; 字段已更名 maximum_rpm_learned
    /// (波19), 方法名 get_maximum_rpm_learn 不再撞名。
    /// @param fm 本周期 FM 句柄快照（R1 下传）
    pub(super) fn get_maximum_rpm_learn(&mut self, fm: &FMHandle) {
        // 简单状态推进 → 单写锁临界区 (无 IO/回调; s_state 不可变借用拆局部,
        // 对齐 check_engine_jet 形态)
        self.apply(|d| {
            if !d.engine.maximum_rpm_learned {
                // R2 守卫: blkx 非 null 即 READY（等价旧版 null+valid 双判）
                if let Some(fmdata) = fm.fmdata.as_ref() {
                    // FM合法直接取FM
                    d.engine.maximum_thr_rpm = fmdata.max_rpm;
                    // 使用最大允许RPM
                    // maximumThrRPM = fm.blkx.maxAllowedRPM;
                    d.engine.maximum_rpm_learned = true;
                } else {
                    // 自适应获得(无FM)

                    // 获得最大转速，条件是以最大转速持续约20秒或者桨距
                    // Java ArithmeticException ↔ Rust 除零 panic (保真, 构造域恒 50)
                    if d.check_maximum_rpm < 20000 / d.freq {
                        let (ias, rpm) = {
                            let s = d.s_state.as_ref().unwrap();
                            (s.ias, s.rpm)
                        };
                        if ias > 50 {
                            if rpm as f64 >= d.engine.maximum_thr_rpm {
                                //       + ratio * (sState.RPM)
                                d.engine.maximum_thr_rpm =
                                    (d.ratio_1 * d.engine.maximum_thr_rpm) + d.ratio * rpm as f64;
                            }
                            d.check_maximum_rpm += 1;
                        }
                    } else {
                        d.engine.maximum_rpm_learned = true;
                    }
                }
            }
        });
    }

    /// 计算最佳增压器档位。
    ///
    /// Also detects mismatch between actual and optimal stage (at full throttle).
    /// Uses state-change detection to only update mismatch status when actual or optimal changes.
    /// Results are published via FlightDataBus for voice warning.
    ///
    /// @param fm 本周期 FM 句柄快照（R1 下传）
    /// (calculate 链尾)
    pub(super) fn update_optimal_compressor_stage(&mut self, fm: &FMHandle) {
        // R1: 从周期句柄直接取增压器参数（不再经 @Deprecated 桥接方法）;
        // 非 READY/喷气机/单级句柄为 null → 走下方无效分支归位
        let stages = if fm.has_fm() {
            fm.compressor_stages.as_ref()
        } else {
            None
        };

        // Invalid cases: jet, single-stage, or no FM loaded
        let stages = match stages {
            Some(s) if s.len() > 1 => s,
            _ => {
                self.apply(|d| {
                    d.engine.optimal_compressor_stage = -1;
                    d.engine.compressor_stage_mismatch = false;
                    d.prev_actual_compressor_stage = -1;
                    d.prev_optimal_compressor_stage = -1;
                });
                return;
            }
        };

        // 读快照→锁外计算→短写锁写回 (§2.8): findOptimalStageIndex 逐档
        // powerAtAltitudeAdvanced 较重, 全程锁外
        let (
            engine_num,
            throttles,
            alt,
            ias,
            compressorstage,
            mismatch_prev,
            prev_actual,
            prev_optimal,
        ) = self.with_snapshot(|d| {
            let s = d.s_state.as_ref().unwrap();
            (
                d.engine.engine_num,
                s.throttles.clone(),
                d.altm.alt,
                // (曾误走 trait default 恒 0, 增压器最优档判定失真)
                s.ias as f64,
                s.compressorstage,
                d.engine.compressor_stage_mismatch,
                d.prev_actual_compressor_stage,
                d.prev_optimal_compressor_stage,
            )
        });

        // Detect WEP mode and full throttle state (any engine throttle >= 100)
        let mut is_wep = false;
        let mut is_full_throttle = false;
        // PORT(allow needless_range_loop): Java for(int i...) 直译 — i 仅索引
        #[allow(clippy::needless_range_loop)]
        for i in 0..engine_num as usize {
            // AIOOBE → run 顶层 catch; 索引 panic 同构收敛 (update_wep_time 同注)
            if throttles[i] > 100 {
                is_wep = true;
                is_full_throttle = true;
            } else if throttles[i] >= 100 {
                is_full_throttle = true;
            }
        }

        // Calculate optimal stage
        //       getIAS(), true, 15.0) —— Rust 侧返回 usize (Java int), 收窄
        //       存 i32 字段 (域内 = 档位下标)
        let new_optimal = find_optimal_stage_index(stages, alt, is_wep, ias, true, 15.0) as i32;

        // Get current actual stage (convert from 1-based to 0-based)
        let actual_stage = compressorstage - 1;

        // API didn't return compressor stage (e.g., some aircraft don't report it)
        if actual_stage < 0 {
            self.apply(|d| {
                // optimalCompressorStage = newOptimal 先于本分支执行,
                // 归位四字段不含它 (保真: 归位后 optimal 保留本轮新算值)
                d.engine.optimal_compressor_stage = new_optimal;
                d.engine.compressor_stage_mismatch = false;
                d.prev_actual_compressor_stage = -1;
                d.prev_optimal_compressor_stage = -1;
            });
            return;
        }

        // If throttle < 100%, don't judge mismatch, force consistent
        if !is_full_throttle {
            self.apply(|d| {
                d.engine.optimal_compressor_stage = new_optimal;
                d.engine.compressor_stage_mismatch = false;
                d.prev_actual_compressor_stage = -1;
                d.prev_optimal_compressor_stage = -1;
            });
            return;
        }

        // State-change driven: only re-evaluate mismatch when actual or optimal changes
        let has_change = (actual_stage != prev_actual) || (new_optimal != prev_optimal);

        // If no change, preserve previous compressorStageMismatch value
        let mismatch = if has_change {
            // Re-evaluate mismatch on state change
            actual_stage != new_optimal
        } else {
            mismatch_prev
        };

        // Update tracking variables
        self.apply(|d| {
            d.engine.optimal_compressor_stage = new_optimal;
            d.engine.compressor_stage_mismatch = mismatch;
            d.prev_actual_compressor_stage = actual_stage;
            d.prev_optimal_compressor_stage = new_optimal;
        });
    }

    // (calc_k 随 flap 双胞胎合一移除 — vm-core fm::data::flap_limits::calc_k 共享实现)
    // (getFlapAllowSpeed/getFlapAllowAngle W8 公式化后无生产调用方, 委托臂
    //  已删 — Java oracle 锚定测试直调 vm-core 共享实现, 见下方 tests)
}

// =====================================================================
// Tests — 断言值 = Java 8 oracle (javac dump 类) + python 位精确手算;
// mock 快照与 service_loop/tests.rs 同源 (STATE_MOCK/INDIC_MOCK 本地拷贝,
// 跨 cfg 模块引用常量不可行, 项目先例)
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::borrow_interior_mutable_const)] // UNRESOLVED 含 Mutex (见 handle.rs 注)
    use super::super::{write_data, ServiceConfig};
    use super::*;
    use std::path::Path;
    use std::sync::Arc;
    use vm_core::base::bus::flight_data_bus::FlightDataBus;
    use vm_core::base::bus::EventBus;
    use vm_core::fm::data::FmData;
    use vm_core::fm::piston_model::CompressorStageParams;
    use vm_core::fm::FMManager;
    use vm_core::formula::registry::FormulaView as _; // var_value 取数

    fn new_service() -> Service {
        let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
        let bus = Arc::new(FlightDataBus::new());
        Service::new(ServiceConfig::default(), fm, bus)
    }

    /// 真机 spitfire_f24 的襟翼毁伤表 (hud_calculator/tests.rs spitfire_blkx
    /// 同源: Java getload 实测 [0.5,290]/[1.0,260] + 1.25x 哨兵行)
    fn spitfire_flap_fmdata() -> FmData {
        let mut b = FmData::default();
        // 对齐 READY 句柄生产形态 (R2: blkx 非 null 即 READY → valid 恒真);
        // 合一后的共享实现保留 Java 的 !valid → 125 防御分支 (设计 §7)
        b.valid = true;
        b.flaps_destruction_num = 2;
        let mut rows = [[0.0f64; 2]; 6];
        rows[0] = [0.5, 290.0];
        rows[1] = [1.0, 260.0];
        rows[2] = [1.25, 0.0]; // 1.25x 哨兵行
        b.flaps_destruction_ind_speed = Some(rows);
        b
    }

    /// 两级增压器 (Java 8 oracle dump 用同参数, build/tmp_oracle 用后即删)
    fn two_stage_params() -> Vec<CompressorStageParams> {
        vec![
            CompressorStageParams {
                crit_alt: 600.0,
                crit_power: 850.0,
                deck_power: 750.0,
                ..Default::default()
            },
            CompressorStageParams {
                crit_alt: 4500.0,
                crit_power: 1000.0,
                deck_power: 600.0,
                ..Default::default()
            },
        ]
    }

    fn stage_fm(stages: Option<Vec<CompressorStageParams>>) -> FMHandle {
        let mut b = FmData::default();
        b.max_rpm = 3000.0;
        FMHandle::ready(Some("mock".to_string()), Some(b), 0.0, 0.0, stages)
    }

    /// update_optimal 的周期输入注入 (engine_num=1 单发)
    fn set_cycle_inputs(svc: &Service, alt: f64, ias: i32, compressorstage: i32, throttle: i32) {
        let mut d = svc.data.write().unwrap();
        d.altm.alt = alt;
        d.engine.engine_num = 1;
        let s = d.s_state.as_mut().unwrap();
        s.ias = ias;
        s.compressorstage = compressorstage;
        s.throttles = [throttle, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    }

    // ---------------- checkWing (已删: 产物无消费者, registry wing_sweep_valid 直通替代) ----------------

    // ---------------- checkFlap ----------------

    /// W8: check_flap 状态机公式化 — is_downing_flap 接管公式位级锚定
    /// (变化方向 latch + 1 秒稳定归零, 语义见 formulas.cfg 的 is_downing_flap)
    #[test]
    fn flap_takeover_state_machine() {
        let mut svc = new_service();
        let defs = vm_core::formula::persistence::load_merged(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../../formulas.cfg"),
            "",
        );
        svc.formula.install(&defs);
        // 放下襟翼 (0→50): 变化方向为增 → is_downing = true
        {
            let mut d = write_data(&svc.data);
            d.s_state.as_mut().unwrap().flaps = 50;
            d.actual_interval_ms = 50; // 稳定计时步进 (meta.interval_ms 来源)
        }
        svc.calculate();
        assert!(
            svc.data
                .read()
                .unwrap()
                .var_value("is_downing_flap")
                .unwrap_or(0.0)
                != 0.0,
            "放下中"
        );
        // 持续稳定 1 秒后归 false
        for _ in 0..21 {
            svc.calculate();
        }
        assert!(
            svc.data
                .read()
                .unwrap()
                .var_value("is_downing_flap")
                .unwrap_or(0.0)
                == 0.0,
            "稳定 1s 后归零"
        );
        // 收起 (50→0): 方向为减 → false
        {
            let mut d = write_data(&svc.data);
            d.s_state.as_mut().unwrap().flaps = 0;
        }
        svc.calculate();
        assert!(
            svc.data
                .read()
                .unwrap()
                .var_value("is_downing_flap")
                .unwrap_or(0.0)
                == 0.0,
            "收起"
        );
    }

    // ---------------- getFlapAllowSpeed / getFlapAllowAngle ----------------

    /// 襟翼允许速度/角度的公式族 (python 位精确 oracle + 分支覆盖)
    #[test]
    fn flap_allow_speed_angle_oracle() {
        let fm = FMHandle::ready(
            Some("mock".to_string()),
            Some(spitfire_flap_fmdata()),
            0.0,
            0.0,
            None,
        );
        // 60%: 档间插值 → 284.0
        assert_eq!(
            vm_core::fm::data::get_flap_allow_speed(60, true, fm.fmdata.as_ref()),
            284.0
        );
        // 50%: 相等档位 (50 == 0.5*100) → 直接返回首档速度 290.0
        assert_eq!(
            vm_core::fm::data::get_flap_allow_speed(50, false, fm.fmdata.as_ref()),
            290.0
        );
        // 30% (i=-1): 下襟翼越级 → 首档速度 290.0; 非下襟翼 → Double.MAX_VALUE
        assert_eq!(
            vm_core::fm::data::get_flap_allow_speed(30, true, fm.fmdata.as_ref()),
            290.0
        );
        assert_eq!(
            vm_core::fm::data::get_flap_allow_speed(30, false, fm.fmdata.as_ref()),
            f64::MAX
        );
        // 120%: 超表外插 (Java 无 clamp): 290 + 70*(-0.6) = 248.0
        assert_eq!(
            vm_core::fm::data::get_flap_allow_speed(120, false, fm.fmdata.as_ref()),
            248.0
        );
        // flapPercent=0 早退 (先于 blkx 判定)
        assert_eq!(
            vm_core::fm::data::get_flap_allow_speed(0, true, fm.fmdata.as_ref()),
            f64::MAX
        );

        // 角度 (x/y 与速度版互换: 按速度查允许 flap 角度)
        // 270: 档间插值 → 83.33333333333334
        assert_eq!(
            vm_core::fm::data::get_flap_allow_angle(270.0, false, fm.fmdata.as_ref()),
            83.33333333333334
        );
        // 290: 相等 → 首档角 0.5*100 = 50.0
        assert_eq!(
            vm_core::fm::data::get_flap_allow_angle(290.0, false, fm.fmdata.as_ref()),
            50.0
        );
        // 350 (i=0 分支): 50 + 60*(-5/3) = -50 → normFlapAngle → 0
        assert_eq!(
            vm_core::fm::data::get_flap_allow_angle(350.0, false, fm.fmdata.as_ref()),
            0.0
        );
        // 100 (低速外插): 50 + (100-290)*(-5/3) = 366.6.. → 封顶 125
        assert_eq!(
            vm_core::fm::data::get_flap_allow_angle(100.0, false, fm.fmdata.as_ref()),
            125.0
        );
        // ias=0 早退
        assert_eq!(
            vm_core::fm::data::get_flap_allow_angle(0.0, true, fm.fmdata.as_ref()),
            125.0
        );

        // 无 FM (UNRESOLVED)
        let unr = FMHandle::UNRESOLVED;
        assert_eq!(
            vm_core::fm::data::get_flap_allow_speed(50, false, unr.fmdata.as_ref()),
            f64::MAX
        );
        assert_eq!(
            vm_core::fm::data::get_flap_allow_angle(300.0, false, unr.fmdata.as_ref()),
            125.0
        );
    }

    // ---------------- getMaximumRPM ----------------

    /// 无 FM 自适应学习 (python 位精确) / 计满 / FM 直取
    #[test]
    fn get_maximum_rpm_adaptive_learning() {
        let mut svc = new_service();
        let unr = FMHandle::UNRESOLVED;
        // 构造后: maximumThrRPM=1.0 (resetvaria), ratio=f64 直算 (波21 f32 复刻退役),
        // ratio_1=1-ratio, checkMaximumRPM=0
        {
            let mut d = svc.data.write().unwrap();
            let s = d.s_state.as_mut().unwrap();
            s.ias = 474;
            s.rpm = 3001;
        }
        svc.get_maximum_rpm_learn(&unr);
        {
            let d = svc.data.read().unwrap();
            // 波21 f64 直算: 0.95*1.0 + 0.05*3001 = 151 (精确)
            assert_eq!(d.engine.maximum_thr_rpm, 151.0);
            assert_eq!(d.check_maximum_rpm, 1);
            assert!(!d.engine.maximum_rpm_learned);
        }
        svc.get_maximum_rpm_learn(&unr);
        // 波21 f64 直算: 0.95*151 + 0.05*3001 = 293.5 (精确)
        assert_eq!(
            svc.data.read().unwrap().engine.maximum_thr_rpm,
            293.5
        );
        assert_eq!(svc.data.read().unwrap().check_maximum_rpm, 2);

        // IAS <= 50: 不学习不进位 (Java int 比较无浮点提升)
        {
            let mut d = svc.data.write().unwrap();
            d.s_state.as_mut().unwrap().ias = 50;
            d.check_maximum_rpm = 399;
        }
        svc.get_maximum_rpm_learn(&unr);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.check_maximum_rpm, 399);
            assert!(!d.engine.maximum_rpm_learned);
        }

        // 计满 (20000/freq = 400): 与 IAS 无关直接置完成
        {
            let mut d = svc.data.write().unwrap();
            d.s_state.as_mut().unwrap().ias = 474;
            d.check_maximum_rpm = 400;
        }
        svc.get_maximum_rpm_learn(&unr);
        assert!(svc.data.read().unwrap().engine.maximum_rpm_learned);

        // 置位守卫 (Java `if (!getMaximumRPM)`): 计满置位后方法整体短路 —
        // 即使 FM 到位也不再覆盖学习值 (Java 同语义)
        let fm = stage_fm(None);
        svc.get_maximum_rpm_learn(&fm);
        assert_eq!(
            svc.data.read().unwrap().engine.maximum_thr_rpm,
            293.5,
            "置位后 FM 不覆盖 (方法短路)"
        );
    }

    /// FM 直取正向路径: 未置位 + blkx 在场 → maxRPM 直写 + 同轮置位
    #[test]
    fn get_maximum_rpm_fm_direct() {
        let mut svc = new_service();
        let fm = stage_fm(None);
        svc.get_maximum_rpm_learn(&fm);
        let d = svc.data.read().unwrap();
        assert_eq!(d.engine.maximum_thr_rpm, 3000.0, "fmdata.maxRPM 直取");
        assert!(d.engine.maximum_rpm_learned, "同轮置位");
    }

    // ---------------- updateOptimalCompressorStage ----------------

    /// 无效分支归位 (无 FM / 单级 / stages 缺席)
    #[test]
    fn update_optimal_compressor_invalid_branches() {
        let mut svc = new_service();
        // 预置非默认值验证归位
        {
            let mut d = svc.data.write().unwrap();
            d.engine.optimal_compressor_stage = 1;
            d.engine.compressor_stage_mismatch = true;
            d.prev_actual_compressor_stage = 1;
            d.prev_optimal_compressor_stage = 1;
        }
        // 无 FM (UNRESOLVED): stages 为 null → 四字段归位
        svc.update_optimal_compressor_stage(&FMHandle::UNRESOLVED);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.engine.optimal_compressor_stage, -1);
            assert!(!d.engine.compressor_stage_mismatch);
            assert_eq!(d.prev_actual_compressor_stage, -1);
            assert_eq!(d.prev_optimal_compressor_stage, -1);
        }
        // 单级 (hasFM 但 stages.len() <= 1): 同归位
        svc.update_optimal_compressor_stage(&stage_fm(Some(
            vec![CompressorStageParams::default()],
        )));
        assert_eq!(svc.data.read().unwrap().engine.optimal_compressor_stage, -1);
        // blkx 在而 stages 缺席 (喷气/未提取形态): 同归位
        svc.update_optimal_compressor_stage(&stage_fm(None));
        assert_eq!(svc.data.read().unwrap().engine.optimal_compressor_stage, -1);
    }

    /// 状态机主体 (Java 8 oracle: alt0→档0, alt3000→档1)
    #[test]
    fn update_optimal_compressor_stage_state_machine() {
        let mut svc = new_service();
        let fm = stage_fm(Some(two_stage_params()));

        // WEP + 满油门 (110); alt=0 → Java 8 oracle 档 0
        set_cycle_inputs(&svc, 0.0, 474, 1, 110);
        svc.update_optimal_compressor_stage(&fm);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.engine.optimal_compressor_stage, 0);
            // actual (compressorstage 1 → 0-based 0) == optimal 0 → 不失配
            assert!(!d.engine.compressor_stage_mismatch);
            assert_eq!(d.prev_actual_compressor_stage, 0);
            assert_eq!(d.prev_optimal_compressor_stage, 0);
        }

        // 无变化轮: mismatch 保持前值 (人为置 true 验证保持语义)
        svc.data.write().unwrap().engine.compressor_stage_mismatch = true;
        svc.update_optimal_compressor_stage(&fm);
        assert!(svc.data.read().unwrap().engine.compressor_stage_mismatch);
        assert_eq!(svc.data.read().unwrap().prev_actual_compressor_stage, 0);

        // 实际档切到 2 (1-based → 0-based 1) ≠ optimal 0 → 失配
        set_cycle_inputs(&svc, 0.0, 474, 2, 110);
        svc.update_optimal_compressor_stage(&fm);
        assert!(svc.data.read().unwrap().engine.compressor_stage_mismatch);

        // alt=3000 (oracle 档 1) + 非 WEP 满油门 (100): actual 1 == optimal 1 → 解除
        set_cycle_inputs(&svc, 3000.0, 474, 2, 100);
        svc.update_optimal_compressor_stage(&fm);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.engine.optimal_compressor_stage, 1);
            assert!(!d.engine.compressor_stage_mismatch);
        }

        // API 未回报档位 (compressorstage=0 → actual=-1): 三字段归位,
        // optimal 保留本轮新算值 (先写后归位的语序保真)
        set_cycle_inputs(&svc, 0.0, 474, 0, 110);
        svc.update_optimal_compressor_stage(&fm);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.engine.optimal_compressor_stage, 0);
            assert!(!d.engine.compressor_stage_mismatch);
            assert_eq!(d.prev_actual_compressor_stage, -1);
            assert_eq!(d.prev_optimal_compressor_stage, -1);
        }

        // 非满油门 (90): 不判定失配, 三字段归位 (optimal 保留新算值)
        set_cycle_inputs(&svc, 3000.0, 474, 2, 90);
        svc.update_optimal_compressor_stage(&fm);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.engine.optimal_compressor_stage, 1);
            assert!(!d.engine.compressor_stage_mismatch);
            assert_eq!(d.prev_actual_compressor_stage, -1);
        }
    }

    // ---------------- 真机 FM (data/ 缺失跳过, 项目惯例) ----------------

    /// 真机 spitfire_f24 经 FMLoader 管道的数据流 (公式级位精确 oracle 已由
    /// mock 表测试锁定, 此处锁 FMLoader→方法接线; data/ 缺失或加载失败跳过)
    #[test]
    fn real_fmloader_spitfire_pipeline() {
        let root = format!("{}/../../../data", env!("CARGO_MANIFEST_DIR"));
        let central = format!("{root}/aces/gamedata/flightmodels/spitfire_f24.json");
        if !Path::new(&central).is_file() {
            println!("SKIP: data/ 真机 FM 缺失");
            return;
        }
        // DATA_ROOT 注入 (fm_data_paths 测试钩子)。原注"进程内并行安全"不成立
        // (workspace 全量并行复现 flake): format_strings 的 nitro 场景并行注入
        // tmp 根/复位会覆盖本注入 — 持串行锁互斥 (见 lib.rs DATA_ROOT_TEST_LOCK)
        let _root_guard = crate::DATA_ROOT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        vm_core::fm::data_paths::set_data_root(&root);
        let fm = vm_core::fm::loader::load(Some("spitfire_f24"));
        let Some(fmdata) = fm.fmdata.as_ref() else {
            println!("SKIP: FMLoader 加载失败 ({})", fm.status);
            return;
        };

        // checkFlap: 真机表直查 (首档精确相等分支, 无插值)
        let table = fmdata.flaps_destruction_ind_speed.as_ref().unwrap();
        let num = fmdata.flaps_destruction_num;
        assert!(num >= 1, "真机襟翼表至少 1 档");
        let mut svc = new_service();
        {
            let mut d = svc.data.write().unwrap();
            d.actual_interval_ms = 50;
            let s = d.s_state.as_mut().unwrap();
            // 首档 flapPercent = table[0][0]*100 (0.5 → 50) 走相等分支
            s.flaps = (table[0][0] * 100.0) as i32;
            s.ias = 300;
        }
        // W8: check_flap 已公式化 — fm_flap_allow_speed 直查对拍 (首档相等分支)
        let flap_pct = (table[0][0] * 100.0) as i32;
        let got = vm_core::fm::data::get_flap_allow_speed(flap_pct, false, Some(fmdata));
        assert_eq!(got, table[0][1]);

        // getMaximumRPM 的 FM 直取: maximumThrRPM = blkx.maxRPM
        svc.get_maximum_rpm_learn(&fm);
        assert_eq!(
            svc.data.read().unwrap().engine.maximum_thr_rpm,
            fmdata.max_rpm
        );
        assert!(svc.data.read().unwrap().engine.maximum_rpm_learned);

        // updateOptimal: 真机 stages 与协作者 find_optimal_stage_index 对拍
        // (锁参数组装 alt/isWep/getIAS()/true/15.0 与写回; 档位功率公式属
        // vm-core piston_power_model 已测域)
        match fm.compressor_stages.as_ref() {
            Some(st) if st.len() > 1 => {
                set_cycle_inputs(&svc, 0.0, 474, 1, 110);
                svc.update_optimal_compressor_stage(&fm);
                let expect = find_optimal_stage_index(st, 0.0, true, 474.0, true, 15.0) as i32;
                let d = svc.data.read().unwrap();
                assert_eq!(d.engine.optimal_compressor_stage, expect);
                assert_eq!(d.prev_optimal_compressor_stage, expect);
            }
            _ => println!("SKIP: 单级/无增压器 (stages 缺席)"),
        }
    }
}
