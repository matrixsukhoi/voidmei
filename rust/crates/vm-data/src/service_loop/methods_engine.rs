//! 对应 Java: `src/prog/Service.java` calculate 链的四方法 + Service 版襟翼
//! 允许速度/角度计算: checkWing (L1030-1035) / checkFlap (L1042-1064) /
//! getMaximumRPM (L1071-1099) / updateOptimalCompressorStage (L1276-1339) /
//! getFlapAllowSpeed (L1354-1427) / getFlapAllowAngle (L1443-1508) 及其辅助
//! calcK (L1341-1347) / normFlapAngle (L1429-1436)。
//!
//! PORT(模块边界): impl Service 跨文件块, 方法一律 pub(super); calculate 内的
//! 接线调用与 `mod methods_engine;` 声明归主线波次 (见交付说明)。
//! PORT(同名陷阱): vm-core hud_calculator.rs 另有**私有** get_flap_allow_angle
//! —— 那是 HUDCalculator 版, 与本模块的 Service 版不同源, 互不复用互不可见;
//! 本文件按 Java Service 版逐行直译。

use super::{read_data, write_data, Service};
use vm_core::fm::FMHandle;
use vm_core::piston_power_model::find_optimal_stage_index;
use vm_core::string_helper::F_INVALID;
use vm_core::ui_model::TelemetrySource as _;

impl Service {
    /// 对应 Java `public void checkWing()` (L1030-1035) — 可变翼判断。
    /// calculate 链位置: updateSEP 之后 (Java L1159)。
    pub(super) fn check_wing(&mut self) {
        // 简单字段搬运 → 单写锁临界区完成 (§2.8 简单形态)
        let mut d = write_data(&self.data);
        // Java: if (sIndic.wsweep_indicator != -65535) —— float 与 int 字面量
        // 比较, -65535 提升 float; F_INVALID = -65535.0 (float 域哨兵)。
        // (sIndic 构造器恒建 → unwrap 复刻 Java 的 null 不可达域)
        d.has_wing_sweep_vario = d.s_indic.as_ref().unwrap().wsweep_indicator != F_INVALID;
    }

    /// 襟翼状态判断与允许速度/角度计算。
    ///
    /// @param fm 本周期 FM 句柄快照（R1 下传）
    /// (对应 Java `public void checkFlap(FMHandle fm)` L1042-1064;
    ///  calculate 链位置: checkWing 之后, Java L1162)
    pub(super) fn check_flap(&mut self, fm: &FMHandle) {
        // 读快照→锁外查表插值→短写锁写回 (§2.8): getFlapAllowSpeed/Angle 的
        // 档位循环与插值计算在锁外
        let (flap_prev, flap_check, actual_interval_ms, flaps, ias) = {
            let d = read_data(&self.data);
            let s = d.s_state.as_ref().unwrap();
            (d.flap, d.flap_check, d.actual_interval_ms, s.flaps, s.ias)
        };

        // Java: boolean downflap = false;
        let mut downflap = false;
        // Java: flapp = flap; flap = sState.flaps;
        let flapp = flap_prev;
        let flap = flaps;
        // flapCheck 的中间推进 (Java 字段 +=, 此处局部, 尾部写回)
        let mut flap_check = flap_check;
        // Java: if (flap - flapp > 0) —— int 差比较
        if flap - flapp > 0 {
            downflap = true;
        } else if flap - flapp == 0 {
            // 加计数
            flap_check += actual_interval_ms;

            // 维持1秒稳定
            if flap_check >= 1000 {
                flap_check = 0;
                downflap = false;
            }
        } else {
            // 小于则一定是收
            downflap = false;
        }
        // Java: isDowningFlap = downflap;
        // Java: flapAllowSpeed = getFlapAllowSpeed(sState.flaps, downflap, fm);
        let flap_allow_speed = Self::get_flap_allow_speed(flaps, downflap, fm);
        // Java: flapAllowAngle = getFlapAllowAngle(sState.IAS, downflap, fm);
        let flap_allow_angle = Self::get_flap_allow_angle(ias as f64, downflap, fm);

        let mut d = write_data(&self.data);
        d.flapp = flapp;
        d.flap = flap;
        d.flap_check = flap_check;
        d.is_downing_flap = downflap;
        d.flap_allow_speed = flap_allow_speed;
        d.flap_allow_angle = flap_allow_angle;
    }

    /// 获取最大转速（优先 FM, 无 FM 时自适应学习）。
    ///
    /// PORT(命名避让, service_fields.rs 字段区备注): Java 字段 getMaximumRPM
    /// (boolean) 与本方法构成同名重载; Rust 字段已占 get_maximum_rpm → 方法按
    /// 备案命名 get_maximum_rpm_learn。
    /// @param fm 本周期 FM 句柄快照（R1 下传, Java javadoc 原文）
    /// (对应 Java `public void getMaximumRPM(FMHandle fm)` L1071-1099;
    ///  calculate 链位置: checkFlap 之后, Java L1166)
    pub(super) fn get_maximum_rpm_learn(&mut self, fm: &FMHandle) {
        // 简单状态推进 → 单写锁临界区 (无 IO/回调; s_state 不可变借用拆局部,
        // 对齐 check_engine_jet 形态)
        let mut d = write_data(&self.data);
        // Java: if (!getMaximumRPM) —— 字段读, 非方法自递归
        if !d.get_maximum_rpm {
            // R2 守卫: blkx 非 null 即 READY（等价旧版 null+valid 双判）
            if let Some(blkx) = fm.blkx.as_ref() {
                // FM合法直接取FM
                d.maximum_thr_rpm = blkx.max_rpm;
                // 使用最大允许RPM
                // maximumThrRPM = fm.blkx.maxAllowedRPM;
                d.get_maximum_rpm = true;
            } else {
                // 自适应获得(无FM)

                // 获得最大转速，条件是以最大转速持续约20秒或者桨距
                // Java: 20000 / freq —— int 20000 提升为 long 除法; freq=0 时
                // Java ArithmeticException ↔ Rust 除零 panic (保真, 构造域恒 50)
                if d.check_maxium_rpm < 20000 / d.freq {
                    let (ias, rpm) = {
                        let s = d.s_state.as_ref().unwrap();
                        (s.ias, s.rpm)
                    };
                    // Java: sState.IAS > 50 —— int 比较 (无浮点提升)
                    if ias > 50 {
                        // Java: sState.RPM >= maximumThrRPM —— int 提升 double
                        if rpm as f64 >= d.maximum_thr_rpm {
                            // Java: maximumThrRPM = (ratio_1 * maximumThrRPM)
                            //       + ratio * (sState.RPM)
                            d.maximum_thr_rpm =
                                (d.ratio_1 * d.maximum_thr_rpm) + d.ratio * rpm as f64;
                        }
                        d.check_maxium_rpm += 1;
                    }
                } else {
                    d.get_maximum_rpm = true;
                }
            }
        }
    }

    /// 计算最佳增压器档位。
    ///
    /// Also detects mismatch between actual and optimal stage (at full throttle).
    /// Uses state-change detection to only update mismatch status when actual or optimal changes.
    /// Results are published via FlightDataBus for voice warning.
    /// (以上 javadoc 逐字保留, Java L1270-1273)
    ///
    /// @param fm 本周期 FM 句柄快照（R1 下传, Java javadoc 原文）
    /// (对应 Java `public void updateOptimalCompressorStage(FMHandle fm)`
    ///  L1276-1339; calculate 链尾, Java L1173)
    pub(super) fn update_optimal_compressor_stage(&mut self, fm: &FMHandle) {
        // R1: 从周期句柄直接取增压器参数（不再经 @Deprecated 桥接方法）;
        // 非 READY/喷气机/单级句柄为 null → 走下方无效分支归位
        let stages = if fm.has_fm() { fm.compressor_stages.as_ref() } else { None };

        // Invalid cases: jet, single-stage, or no FM loaded
        // Java: stages == null || stages.length <= 1
        let stages = match stages {
            Some(s) if s.len() > 1 => s,
            _ => {
                let mut d = write_data(&self.data);
                d.optimal_compressor_stage = -1;
                d.compressor_stage_mismatch = false;
                d.prev_actual_compressor_stage = -1;
                d.prev_optimal_compressor_stage = -1;
                return;
            }
        };

        // 读快照→锁外计算→短写锁写回 (§2.8): findOptimalStageIndex 逐档
        // powerAtAltitudeAdvanced 较重, 全程锁外
        let (engine_num, throttles, alt, ias, compressorstage, mismatch_prev, prev_actual, prev_optimal) = {
            let d = read_data(&self.data);
            let s = d.s_state.as_ref().unwrap();
            (
                d.engine_num,
                s.throttles.clone(),
                d.alt,
                // Java: getIAS() = sState != null ? sState.IAS : 0
                d.get_ias(),
                s.compressorstage,
                d.compressor_stage_mismatch,
                d.prev_actual_compressor_stage,
                d.prev_optimal_compressor_stage,
            )
        };

        // Detect WEP mode and full throttle state (any engine throttle >= 100)
        let mut is_wep = false;
        let mut is_full_throttle = false;
        // PORT(allow needless_range_loop): Java for(int i...) 直译 — i 仅索引
        #[allow(clippy::needless_range_loop)]
        for i in 0..engine_num as usize {
            // Java: sState.throttles[i] 越界 (engineNum > throttles 长度) 抛
            // AIOOBE → run 顶层 catch; 索引 panic 同构收敛 (update_wep_time 同注)
            if throttles[i] > 100 {
                is_wep = true;
                is_full_throttle = true;
            } else if throttles[i] >= 100 {
                is_full_throttle = true;
            }
        }

        // Calculate optimal stage
        // Java: PistonPowerModel.findOptimalStageIndex(stages, alt, isWep,
        //       getIAS(), true, 15.0) —— Rust 侧返回 usize (Java int), 收窄
        //       存 i32 字段 (域内 = 档位下标)
        let new_optimal = find_optimal_stage_index(stages, alt, is_wep, ias, true, 15.0) as i32;

        // Get current actual stage (convert from 1-based to 0-based)
        let actual_stage = compressorstage - 1;

        // API didn't return compressor stage (e.g., some aircraft don't report it)
        if actual_stage < 0 {
            let mut d = write_data(&self.data);
            // Java L1305 的 optimalCompressorStage = newOptimal 先于本分支执行,
            // 归位四字段不含它 (保真: 归位后 optimal 保留本轮新算值)
            d.optimal_compressor_stage = new_optimal;
            d.compressor_stage_mismatch = false;
            d.prev_actual_compressor_stage = -1;
            d.prev_optimal_compressor_stage = -1;
            return;
        }

        // If throttle < 100%, don't judge mismatch, force consistent
        if !is_full_throttle {
            let mut d = write_data(&self.data);
            d.optimal_compressor_stage = new_optimal;
            d.compressor_stage_mismatch = false;
            d.prev_actual_compressor_stage = -1;
            d.prev_optimal_compressor_stage = -1;
            return;
        }

        // State-change driven: only re-evaluate mismatch when actual or optimal changes
        let has_change =
            (actual_stage != prev_actual) || (new_optimal != prev_optimal);

        // If no change, preserve previous compressorStageMismatch value
        let mismatch = if has_change {
            // Re-evaluate mismatch on state change
            actual_stage != new_optimal
        } else {
            mismatch_prev
        };

        // Update tracking variables
        let mut d = write_data(&self.data);
        d.optimal_compressor_stage = new_optimal;
        d.compressor_stage_mismatch = mismatch;
        d.prev_actual_compressor_stage = actual_stage;
        d.prev_optimal_compressor_stage = new_optimal;
    }

    /// 对应 Java `double calcK(double x0, double y0, double x1, double y1)`
    /// (L1341-1347) — 两点斜率 (x1==x0 时返回 0, 防除零)。
    pub(super) fn calc_k(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        let mut k = 0.0f64;
        // Java: x1 - x0 != 0 —— NaN 时条件为 true → k = NaN/x (极性保持, §2.12)
        if x1 - x0 != 0.0 {
            k = (y1 - y0) / (x1 - x0);
        }
        k
    }

    /// 对应 Java `public double getFlapAllowSpeed(int flapPercent, Boolean isDowningFlap, FMHandle fm)`
    /// (L1354-1427) — 计算当前襟翼开度下的允许速度。
    ///
    /// PORT(Boolean 装箱 §4): isDowningFlap 域内唯一调用点 checkFlap 传原始
    /// boolean 自动装箱, null 不可达 → bool 直译。
    /// PORT(同名陷阱): 见模块头注 —— 本方法是 Service 版, 非 hud_calculator 版。
    pub(super) fn get_flap_allow_speed(flap_percent: i32, is_downing_flap: bool, fm: &FMHandle) -> f64 {
        // R2 hasFM 守卫: blkx 非 null 即 READY, 无 FM 时无限制（MAX_VALUE）
        // Java: if (flapPercent == 0 || blkx == null) return Double.MAX_VALUE;
        // (Double.MAX_VALUE — resetvaria 初值侧是 Float.MAX_VALUE, 两处字面
        //  在 Java 里刻意不同, 保真区分)
        if flap_percent == 0 || fm.blkx.is_none() {
            return f64::MAX;
        }
        let blkx = fm.blkx.as_ref().unwrap();
        // Java: int FlapsDestructionNum = blkx.FlapsDestructionNum;
        let flaps_destruction_num = blkx.flaps_destruction_num;
        // 找到襟翼档位
        // (doLoad=true 恒 Some; doLoad=false 占位形态 Java 为 null 数组 →
        //  NPE 由 run 顶层 catch 兜住, unwrap panic 同构收敛 §6)
        let table = blkx.flaps_destruction_ind_speed.as_ref().unwrap();
        let mut i: i32 = 0;
        while i < flaps_destruction_num - 1 {
            // 大于
            // Java: flapPercent < blkx.FlapsDestructionIndSpeed[i][0] * 100.0f
            // —— int 提升 double; 100.0f 提升后恰为精确值 100.0 (§2.12 直书)
            if (flap_percent as f64) < table[i as usize][0] * 100.0 {
                break;
            }
            i += 1;
        }
        // Java: i -= 1;
        let i = i - 1;
        // 找到档位了
        // 线性求值
        // 找前面的flap值
        // 没有找到，都小于

        if i == -1 {
            // 下襟翼时直接越级使用下一级
            // (FlapsDestructionNum >= 1 守卫在 num=0 的畸形 FM 域内是活条件:
            //  reader 三个回退全 miss 时 num 可为 0, 见 blkx/reader.rs L1094-1105)
            if is_downing_flap && flaps_destruction_num >= 1 {
                return table[0][1];
            }
            // 襟翼只有0级
            // (Java 注释掉的 if(c.getBlkx().FlapsDestructionNum == 0) 分支)
            f64::MAX
        } else {
            // 下襟翼时直接越级使用
            // (Java 注释掉的 if (isDowningFlap) return ...[i][1]; 分支)

            // 相等
            // Java: flapPercent == blkx.FlapsDestructionIndSpeed[i][0] * 100.0f
            // —— int 提升为 double 后的精确相等 (保真, 不改误差形态)
            if (flap_percent as f64) == table[i as usize][0] * 100.0 {
                // 直接返回速度
                return table[i as usize][1];
            }

            // 否则进行线性插值运算
            // 算斜率
            let x0 = table[i as usize][0] * 100.0;
            let y0 = table[i as usize][1];
            let x1 = table[(i + 1) as usize][0] * 100.0;
            let y1 = table[(i + 1) as usize][1];
            let k = Self::calc_k(x0, y0, x1, y1);

            // 速度等于
            y0 + (flap_percent as f64 - x0) * k
        }
    }

    /// 对应 Java `double normFlapAngle(double t)` (L1429-1436) — 襟翼角度
    /// 归一到 [0, 125]。
    pub(super) fn norm_flap_angle(t: f64) -> f64 {
        if t < 0.0 {
            return 0.0;
        }
        if t < 125.0 {
            t
        } else {
            125.0
        }
    }

    /// 对应 Java `public double getFlapAllowAngle(double ias, Boolean isDowningFlap, FMHandle fm)`
    /// (L1443-1508) — 计算当前速度下的允许襟翼角度。
    ///
    /// PORT(Java bug 保真): isDowningFlap 形参全程未被读 (越级使用分支已被
    /// 注释掉), 保真保留形参 (Rust 未用形参以 _ 前缀消警, fm_power_extractor
    /// 同款约定)。
    /// PORT(同名陷阱): 见模块头注 —— 本方法是 Service 版, 非 hud_calculator 版。
    pub(super) fn get_flap_allow_angle(ias: f64, _is_downing_flap: bool, fm: &FMHandle) -> f64 {
        // R2 hasFM 守卫: blkx 非 null 即 READY, 无 FM 时无限制（125 = normFlapAngle 上限）
        // fm文件无法解析
        if ias == 0.0 || fm.blkx.is_none() {
            return 125.0;
        }
        let blkx = fm.blkx.as_ref().unwrap();
        // 找到襟翼档位 (table 的 unwrap 语义同 get_flap_allow_speed 注)
        let table = blkx.flaps_destruction_ind_speed.as_ref().unwrap();
        let mut i: i32 = 0;
        while i < blkx.flaps_destruction_num - 1 {
            // 大于
            if ias > table[i as usize][1] {
                break;
            }
            i += 1;
        }
        // PORT: 与 getFlapAllowSpeed 的关键差异 —— 此处**无** i -= 1

        // 找到档位了
        // 线性求值
        // 找前面的flap值
        // 没有找到，都小于

        if i == 0 {
            // 下襟翼时直接越级使用下一级
            // (Java 注释掉的 "襟翼只有0级" 分支)
            // (x/y 与 Speed 版互换: 此处按速度查允许 flap 角度)
            let x0 = table[i as usize][1];
            let y0 = table[i as usize][0] * 100.0;
            let x1 = table[(i + 1) as usize][1];
            let y1 = table[(i + 1) as usize][0] * 100.0;
            let k = Self::calc_k(x0, y0, x1, y1);

            let t = y0 + (ias - x0) * k;
            Self::norm_flap_angle(t)
        } else {
            // 下襟翼时直接越级使用
            // (Java 注释掉的 if (isDowningFlap) return ...[i][1]; 分支)

            // 相等
            if ias == table[(i - 1) as usize][1] {
                // 直接返回速度
                return table[(i - 1) as usize][0] * 100.0;
            }

            // 否则进行线性插值运算
            // 算斜率
            let x0 = table[(i - 1) as usize][1];
            let y0 = table[(i - 1) as usize][0] * 100.0;
            let x1 = table[i as usize][1];
            let y1 = table[i as usize][0] * 100.0;
            let k = Self::calc_k(x0, y0, x1, y1);

            // 速度等于
            let t = y0 + (ias - x0) * k;
            Self::norm_flap_angle(t)
        }
    }
}

// =====================================================================
// Tests — 断言值 = Java 8 oracle (javac dump 类) + python 位精确手算;
// mock 快照与 service_loop/tests.rs 同源 (STATE_MOCK/INDIC_MOCK 本地拷贝,
// 跨 cfg 模块引用常量不可行, 项目先例)
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::borrow_interior_mutable_const)] // UNRESOLVED 含 Mutex (见 handle.rs 注)
    use super::*;
    use super::super::ServiceConfig;
    use std::path::Path;
    use std::sync::Arc;
    use vm_core::blkx::Blkx;
    use vm_core::bus::EventBus;
    use vm_core::flight_data_bus::FlightDataBus;
    use vm_core::fm::FMManager;
    use vm_core::piston_power_model::CompressorStageParams;

    /// p51d /indicators 快照 (service_loop/tests.rs 同源数据; 无
    /// wing_sweep_indicator 键 → update 后该字段 = F_INVALID)
    const INDIC_MOCK: &str = "{\"valid\": true, \"army\": \"air\", \"type\": \"p-51d-20_china\", \"speed\": 131.007797, \"vario\": -7.342558, \"aviahorizon_roll\": -40.553505, \"aviahorizon_pitch\": 0.632352, \"compass\": 164.09729}";

    fn new_service() -> Service {
        let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
        let bus = Arc::new(FlightDataBus::new());
        Service::new(ServiceConfig::default(), fm, bus)
    }

    /// 真机 spitfire_f24 的襟翼毁伤表 (hud_calculator/tests.rs spitfire_blkx
    /// 同源: Java getload 实测 [0.5,290]/[1.0,260] + 1.25x 哨兵行)
    fn spitfire_flap_blkx() -> Blkx {
        let mut b = Blkx::default();
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
        let mut b = Blkx::default();
        b.max_rpm = 3000.0;
        FMHandle::ready(Some("mock".to_string()), Some(b), 0.0, 0.0, stages)
    }

    /// update_optimal 的周期输入注入 (engine_num=1 单发)
    fn set_cycle_inputs(svc: &Service, alt: f64, ias: i32, compressorstage: i32, throttle: i32) {
        let mut d = svc.data.write().unwrap();
        d.alt = alt;
        d.engine_num = 1;
        let s = d.s_state.as_mut().unwrap();
        s.ias = ias;
        s.compressorstage = compressorstage;
        s.throttles = vec![throttle];
    }

    // ---------------- checkWing ----------------

    /// wsweep 缺键 (-65535 哨兵) → false; 带读数 → true
    #[test]
    fn check_wing_sets_flag_from_wsweep() {
        let mut svc = new_service();
        // INDIC_MOCK 无 wing_sweep_indicator 键 → get_data_float 缺省 F_INVALID
        svc.data
            .write()
            .unwrap()
            .s_indic
            .as_mut()
            .unwrap()
            .update(INDIC_MOCK);
        svc.check_wing();
        assert!(!svc.data.read().unwrap().has_wing_sweep_vario);

        // 带可变翼读数
        svc.data
            .write()
            .unwrap()
            .s_indic
            .as_mut()
            .unwrap()
            .update(r#"{"valid": true, "wing_sweep_indicator": 25.5}"#);
        svc.check_wing();
        assert!(svc.data.read().unwrap().has_wing_sweep_vario);
    }

    // ---------------- checkFlap ----------------

    /// 襟翼状态机: 增 → downing; 维持/收 → 立即 false; flapCheck 1 秒归零路径
    #[test]
    fn check_flap_downing_state_machine() {
        let mut svc = new_service();
        let unr = FMHandle::UNRESOLVED;
        {
            let mut d = svc.data.write().unwrap();
            d.actual_interval_ms = 50;
            d.flap = 0; // resetvaria 后本就 0, 显式示意快照输入
            d.s_state.as_mut().unwrap().flaps = 50;
        }
        // 增襟翼 0→50
        svc.check_flap(&unr);
        {
            let d = svc.data.read().unwrap();
            assert_eq!((d.flap, d.flapp), (50, 0));
            assert!(d.is_downing_flap);
            // 无 FM: getFlapAllowSpeed → Double.MAX_VALUE / Angle → 125
            // (Java L1359/L1448 的无 FM 早退字面)
            assert_eq!(d.flap_allow_speed, f64::MAX);
            assert_eq!(d.flap_allow_angle, 125.0);
            // 增襟翼分支不动 flapCheck
            assert_eq!(d.flap_check, 0);
        }
        // 维持 50 (== 分支): downflap 保持初始 false (Java 保真: 1 秒稳定
        // 分支只把 false 再赋 false, 不构成观察效果)
        svc.check_flap(&unr);
        {
            let d = svc.data.read().unwrap();
            assert!(!d.is_downing_flap);
            assert_eq!(d.flap_check, 50);
        }
        // 收襟翼 50→30
        svc.data.write().unwrap().s_state.as_mut().unwrap().flaps = 30;
        svc.check_flap(&unr);
        assert!(!svc.data.read().unwrap().is_downing_flap);

        // 维持 30: 预置 950 + 50 = 1000 ≥ 1000 → 归零
        svc.data.write().unwrap().flap_check = 950;
        svc.check_flap(&unr);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.flap_check, 0);
            assert!(!d.is_downing_flap);
        }
    }

    /// 带 FM 表的 checkFlap 集成 (python 位精确 oracle)
    #[test]
    fn check_flap_with_fm_table() {
        let mut svc = new_service();
        {
            let mut d = svc.data.write().unwrap();
            d.actual_interval_ms = 50;
            let s = d.s_state.as_mut().unwrap();
            s.flaps = 60;
            s.ias = 270;
        }
        let fm = FMHandle::ready(
            Some("mock".to_string()),
            Some(spitfire_flap_blkx()),
            0.0,
            0.0,
            None,
        );
        svc.check_flap(&fm);
        let d = svc.data.read().unwrap();
        assert!(d.is_downing_flap);
        // 下襟翼 60% 落 [50,100] 档线性插值:
        // python: 290 + (60-50)*((260-290)/(100-50)) = 284.0
        assert_eq!(d.flap_allow_speed, 284.0);
        // ias=270: python: 50 + (270-290)*((100-50)/(260-290))
        //          = 83.33333333333334
        assert_eq!(d.flap_allow_angle, 83.33333333333334);
    }

    // ---------------- getFlapAllowSpeed / getFlapAllowAngle ----------------

    /// 襟翼允许速度/角度的公式族 (python 位精确 oracle + 分支覆盖)
    #[test]
    fn flap_allow_speed_angle_oracle() {
        let fm = FMHandle::ready(
            Some("mock".to_string()),
            Some(spitfire_flap_blkx()),
            0.0,
            0.0,
            None,
        );
        // 60%: 档间插值 → 284.0
        assert_eq!(Service::get_flap_allow_speed(60, true, &fm), 284.0);
        // 50%: 相等档位 (50 == 0.5*100) → 直接返回首档速度 290.0
        assert_eq!(Service::get_flap_allow_speed(50, false, &fm), 290.0);
        // 30% (i=-1): 下襟翼越级 → 首档速度 290.0; 非下襟翼 → Double.MAX_VALUE
        assert_eq!(Service::get_flap_allow_speed(30, true, &fm), 290.0);
        assert_eq!(Service::get_flap_allow_speed(30, false, &fm), f64::MAX);
        // 120%: 超表外插 (Java 无 clamp): 290 + 70*(-0.6) = 248.0
        assert_eq!(Service::get_flap_allow_speed(120, false, &fm), 248.0);
        // flapPercent=0 早退 (先于 blkx 判定)
        assert_eq!(Service::get_flap_allow_speed(0, true, &fm), f64::MAX);

        // 角度 (x/y 与速度版互换: 按速度查允许 flap 角度)
        // 270: 档间插值 → 83.33333333333334
        assert_eq!(Service::get_flap_allow_angle(270.0, false, &fm), 83.33333333333334);
        // 290: 相等 → 首档角 0.5*100 = 50.0
        assert_eq!(Service::get_flap_allow_angle(290.0, false, &fm), 50.0);
        // 350 (i=0 分支): 50 + 60*(-5/3) = -50 → normFlapAngle → 0
        assert_eq!(Service::get_flap_allow_angle(350.0, false, &fm), 0.0);
        // 100 (低速外插): 50 + (100-290)*(-5/3) = 366.6.. → 封顶 125
        assert_eq!(Service::get_flap_allow_angle(100.0, false, &fm), 125.0);
        // ias=0 早退
        assert_eq!(Service::get_flap_allow_angle(0.0, true, &fm), 125.0);

        // 无 FM (UNRESOLVED)
        let unr = FMHandle::UNRESOLVED;
        assert_eq!(Service::get_flap_allow_speed(50, false, &unr), f64::MAX);
        assert_eq!(Service::get_flap_allow_angle(300.0, false, &unr), 125.0);
    }

    // ---------------- getMaximumRPM ----------------

    /// 无 FM 自适应学习 (python 位精确) / 计满 / FM 直取
    #[test]
    fn get_maximum_rpm_adaptive_learning() {
        let mut svc = new_service();
        let unr = FMHandle::UNRESOLVED;
        // 构造后: maximumThrRPM=1.0 (resetvaria), ratio=f32(50/1000) 拓宽,
        // ratio_1=1-ratio, checkMaxiumRPM=0
        {
            let mut d = svc.data.write().unwrap();
            let s = d.s_state.as_mut().unwrap();
            s.ias = 474;
            s.rpm = 3001;
        }
        svc.get_maximum_rpm_learn(&unr);
        {
            let d = svc.data.read().unwrap();
            // python: 0.9499999992549419*1.0 + 0.05000000074505806*3001
            assert_eq!(d.maximum_thr_rpm, 151.00000223517418);
            assert_eq!(d.check_maxium_rpm, 1);
            assert!(!d.get_maximum_rpm);
        }
        svc.get_maximum_rpm_learn(&unr);
        // python: 0.9499999992549419*151.00000223517418 + 0.05000000074505806*3001
        assert_eq!(svc.data.read().unwrap().maximum_thr_rpm, 293.50000424683094);
        assert_eq!(svc.data.read().unwrap().check_maxium_rpm, 2);

        // IAS <= 50: 不学习不进位 (Java int 比较无浮点提升)
        {
            let mut d = svc.data.write().unwrap();
            d.s_state.as_mut().unwrap().ias = 50;
            d.check_maxium_rpm = 399;
        }
        svc.get_maximum_rpm_learn(&unr);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.check_maxium_rpm, 399);
            assert!(!d.get_maximum_rpm);
        }

        // 计满 (20000/freq = 400): 与 IAS 无关直接置完成
        {
            let mut d = svc.data.write().unwrap();
            d.s_state.as_mut().unwrap().ias = 474;
            d.check_maxium_rpm = 400;
        }
        svc.get_maximum_rpm_learn(&unr);
        assert!(svc.data.read().unwrap().get_maximum_rpm);

        // 置位守卫 (Java `if (!getMaximumRPM)`): 计满置位后方法整体短路 —
        // 即使 FM 到位也不再覆盖学习值 (Java 同语义)
        let fm = stage_fm(None);
        svc.get_maximum_rpm_learn(&fm);
        assert_eq!(
            svc.data.read().unwrap().maximum_thr_rpm,
            293.50000424683094,
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
        assert_eq!(d.maximum_thr_rpm, 3000.0, "blkx.maxRPM 直取");
        assert!(d.get_maximum_rpm, "同轮置位");
    }

    // ---------------- updateOptimalCompressorStage ----------------

    /// 无效分支归位 (无 FM / 单级 / stages 缺席)
    #[test]
    fn update_optimal_compressor_invalid_branches() {
        let mut svc = new_service();
        // 预置非默认值验证归位
        {
            let mut d = svc.data.write().unwrap();
            d.optimal_compressor_stage = 1;
            d.compressor_stage_mismatch = true;
            d.prev_actual_compressor_stage = 1;
            d.prev_optimal_compressor_stage = 1;
        }
        // 无 FM (UNRESOLVED): stages 为 null → 四字段归位
        svc.update_optimal_compressor_stage(&FMHandle::UNRESOLVED);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.optimal_compressor_stage, -1);
            assert!(!d.compressor_stage_mismatch);
            assert_eq!(d.prev_actual_compressor_stage, -1);
            assert_eq!(d.prev_optimal_compressor_stage, -1);
        }
        // 单级 (hasFM 但 stages.len() <= 1): 同归位
        svc.update_optimal_compressor_stage(&stage_fm(Some(vec![CompressorStageParams::default()])));
        assert_eq!(svc.data.read().unwrap().optimal_compressor_stage, -1);
        // blkx 在而 stages 缺席 (喷气/未提取形态): 同归位
        svc.update_optimal_compressor_stage(&stage_fm(None));
        assert_eq!(svc.data.read().unwrap().optimal_compressor_stage, -1);
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
            assert_eq!(d.optimal_compressor_stage, 0);
            // actual (compressorstage 1 → 0-based 0) == optimal 0 → 不失配
            assert!(!d.compressor_stage_mismatch);
            assert_eq!(d.prev_actual_compressor_stage, 0);
            assert_eq!(d.prev_optimal_compressor_stage, 0);
        }

        // 无变化轮: mismatch 保持前值 (人为置 true 验证保持语义)
        svc.data.write().unwrap().compressor_stage_mismatch = true;
        svc.update_optimal_compressor_stage(&fm);
        assert!(svc.data.read().unwrap().compressor_stage_mismatch);
        assert_eq!(svc.data.read().unwrap().prev_actual_compressor_stage, 0);

        // 实际档切到 2 (1-based → 0-based 1) ≠ optimal 0 → 失配
        set_cycle_inputs(&svc, 0.0, 474, 2, 110);
        svc.update_optimal_compressor_stage(&fm);
        assert!(svc.data.read().unwrap().compressor_stage_mismatch);

        // alt=3000 (oracle 档 1) + 非 WEP 满油门 (100): actual 1 == optimal 1 → 解除
        set_cycle_inputs(&svc, 3000.0, 474, 2, 100);
        svc.update_optimal_compressor_stage(&fm);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.optimal_compressor_stage, 1);
            assert!(!d.compressor_stage_mismatch);
        }

        // API 未回报档位 (compressorstage=0 → actual=-1): 三字段归位,
        // optimal 保留本轮新算值 (Java L1305 先写后归位的语序保真)
        set_cycle_inputs(&svc, 0.0, 474, 0, 110);
        svc.update_optimal_compressor_stage(&fm);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.optimal_compressor_stage, 0);
            assert!(!d.compressor_stage_mismatch);
            assert_eq!(d.prev_actual_compressor_stage, -1);
            assert_eq!(d.prev_optimal_compressor_stage, -1);
        }

        // 非满油门 (90): 不判定失配, 三字段归位 (optimal 保留新算值)
        set_cycle_inputs(&svc, 3000.0, 474, 2, 90);
        svc.update_optimal_compressor_stage(&fm);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.optimal_compressor_stage, 1);
            assert!(!d.compressor_stage_mismatch);
            assert_eq!(d.prev_actual_compressor_stage, -1);
        }
    }

    // ---------------- 真机 FM (data/ 缺失跳过, 项目惯例) ----------------

    /// 真机 spitfire_f24 经 FMLoader 管道的数据流 (公式级位精确 oracle 已由
    /// mock 表测试锁定, 此处锁 FMLoader→方法接线; data/ 缺失或加载失败跳过)
    #[test]
    fn real_fmloader_spitfire_pipeline() {
        let root = format!("{}/../../../data", env!("CARGO_MANIFEST_DIR"));
        let central = format!("{root}/aces/gamedata/flightmodels/spitfire_f24.blkx");
        if !Path::new(&central).is_file() {
            println!("SKIP: data/ 真机 FM 缺失");
            return;
        }
        // DATA_ROOT 注入 (fm_data_paths 测试钩子)。原注"进程内并行安全"不成立
        // (workspace 全量并行复现 flake): format_strings 的 nitro 场景并行注入
        // tmp 根/复位会覆盖本注入 — 持串行锁互斥 (见 lib.rs DATA_ROOT_TEST_LOCK)
        let _root_guard =
            crate::DATA_ROOT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        vm_core::fm::fm_data_paths::set_data_root(&root);
        let fm = vm_core::fm::fm_loader::load(Some("spitfire_f24"));
        let Some(blkx) = fm.blkx.as_ref() else {
            println!("SKIP: FMLoader 加载失败 ({})", fm.status);
            return;
        };

        // checkFlap: 真机表直查 (首档精确相等分支, 无插值)
        let table = blkx.flaps_destruction_ind_speed.as_ref().unwrap();
        let num = blkx.flaps_destruction_num;
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
        svc.check_flap(&fm);
        {
            let d = svc.data.read().unwrap();
            assert_eq!(d.flap_allow_speed, table[0][1]);
        }

        // getMaximumRPM 的 FM 直取: maximumThrRPM = blkx.maxRPM
        svc.get_maximum_rpm_learn(&fm);
        assert_eq!(svc.data.read().unwrap().maximum_thr_rpm, blkx.max_rpm);
        assert!(svc.data.read().unwrap().get_maximum_rpm);

        // updateOptimal: 真机 stages 与协作者 find_optimal_stage_index 对拍
        // (锁参数组装 alt/isWep/getIAS()/true/15.0 与写回; 档位功率公式属
        // vm-core piston_power_model 已测域)
        match fm.compressor_stages.as_ref() {
            Some(st) if st.len() > 1 => {
                set_cycle_inputs(&svc, 0.0, 474, 1, 110);
                svc.update_optimal_compressor_stage(&fm);
                let expect = find_optimal_stage_index(st, 0.0, true, 474.0, true, 15.0) as i32;
                let d = svc.data.read().unwrap();
                assert_eq!(d.optimal_compressor_stage, expect);
                assert_eq!(d.prev_optimal_compressor_stage, expect);
            }
            _ => println!("SKIP: 单级/无增压器 (stages 缺席)"),
        }
    }
}
