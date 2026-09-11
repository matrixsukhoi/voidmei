//! calculate 链引擎族方法: 最大转速 (FM 直取, 无 FM 自适应学习) 与最佳
//! 增压器档位/失配检测。原 Service 版襟翼族 (checkFlap/getFlapAllowSpeed/
//! getFlapAllowAngle) 已 W8 公式化或合一至 kernel 共享实现 (checkWing 已删:
//! 产物无消费者, registry wing_sweep_valid 直通替代)。
//!
//! PORT(模块边界): impl Service 跨文件块, 方法一律 pub(super); calculate 内的
//! 接线调用统一在 service_loop.rs。
//! PORT(同名陷阱): kernel fm/data/flap_limits.rs 另有**公共** get_flap_allow_angle
//! —— 那是 HUDCalculator 版, 与本模块的 Service 版不同源, 互不复用互不可见;
//! 本文件按 Java Service 版逐行直译。

use super::Service;
use kernel::fm::piston_model::find_optimal_stage_index;
use kernel::fm::FMHandle;

impl Service {
    /// 获取最大转速（优先 FM, 无 FM 时自适应学习）。
    ///
    /// PORT(命名避让, service_fields.rs 字段区备注): Java 字段 getMaximumRPM
    /// (boolean) 与本方法构成 Java 同名重载; 字段已更名 maximum_rpm_learned
    /// (波19), 方法名 get_maximum_rpm_learn 不再撞名。
    /// - `fm`: 本周期 FM 句柄快照（R1 下传）
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
    /// - `fm`: 本周期 FM 句柄快照（R1 下传）
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

        // 读快照→锁外计算→短写锁写回: findOptimalStageIndex 逐档
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
                s.throttles, // [i32;16] Copy, 免 clone (波21 定长化红利)
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

    // (calc_k 随 flap 双胞胎合一移除 — kernel fm::data::flap_limits::calc_k 共享实现)
    // (getFlapAllowSpeed/getFlapAllowAngle W8 公式化后无生产调用方, 委托臂
    //  已删 — 历史基线 锚定测试直调 kernel 共享实现, 见下方 tests)
}

// =====================================================================
// Tests — 断言值 = 历史基线 (javac dump 类) + python 位精确手算;
// mock 快照与 service_loop/tests.rs 同源 (STATE_MOCK/INDIC_MOCK 本地拷贝,
// 跨 cfg 模块引用常量不可行, 项目先例)
// =====================================================================
