//! 对应 Java: `src/prog/Service.java` 的 checkOverheat (L570-648) / resetEngLoad
//! (L1510-1525) — 引擎过热/耐久度检查 + 耐久计时重置 (impl Service 跨文件块,
//! 方法 pub(super); calculate 接线点见文件尾注)。
//!
//! 会话态走向 (handle.rs "会话态提升" 裁决): Java 就地改写 `blkx.engLoad[i]`,
//! Rust 改写 `fm.eng_load_state` 的 Mutex 锁内副本 (blkx 保持不可变解析产物)。
//! 锁纪律: 锁内纯计算无 IO, 且与 ServiceData 的 RwLock **不嵌套** —— 输入先经
//! read_data 快照、锁内算完释放、结果再经 write_data 写回 (单向 data→session
//! 取值序, 杜绝 ABBA)。
use super::{read_data, write_data, Service};
use vm_core::fm::FMHandle;

impl Service {
    /// 引擎过热/耐久度检查。
    ///
    /// @param fm 本周期 FM 句柄快照（R1 下传, 单周期内同一 Blkx 实例）
    // 接线点: calculate 链 updateTemp 之后 (主线波次, 见文件尾注) — 接线前
    // dead_code 以 allow 静音 (service_fields.rs cur_w_load 同款先例), 接线后无感
    pub(super) fn check_overheat(&mut self, fm: &FMHandle) {
        // 输入快照 (锁外取, §2.8): sState.power[0]/throttle + 温度/轮询周期。
        // power 空数组索引 panic = Java AIOOBE 同构 (run 顶层 catch_unwind 兜住)
        let (power0, throttle, poll_cycle_duration_ms, nwater_temp, noil_temp) = {
            let d = read_data(&self.data);
            let s = d.s_state.as_ref().unwrap();
            (
                s.power[0],
                s.throttle,
                d.poll_cycle_duration_ms,
                d.nwater_temp,
                d.noil_temp,
            )
        };
        /* 关发动机后，温度降到最低load后恢复 */
        let mut eng_off = false;
        if power0 == 0.0 && throttle > 0 {
            /* 关发动机 */
            eng_off = true;
            // Application.debugPrint("监测到引擎关闭");
        }

        // parser.Blkx blkx = fm.blkx;
        // engineLoad[] pL = (blkx != null) ? blkx.engLoad : null;
        // (会话态提升: pL 的真人 = fm.eng_load_state, ready() 从 blkx.eng_load 克隆初始化;
        //  Java 守卫内 `curLoadMinWorkTime = 99999*1000; return;` 的早退写回收口到
        //  闭包外的 write 段统一落 —— 期间无任何读者 (单写者线程), 语义不变)
        let outcome = (|| -> Option<(i32, i32, f64)> {
            let mut session = fm
                .eng_load_state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let (blkx, p_l) = match (fm.blkx.as_ref(), session.as_deref_mut()) {
                (Some(b), Some(p)) => (b, p),
                // R2 hasFM 守卫（P3 修复 NPE 点）: 旧版 Controller.getBlkx() 可能返回
                // invalid 但非 null 的实例, 此处裸调 engLoad 不会炸; P2 桥接后 MISSING/CORRUPT
                // 句柄 blkx 恒为 null, 裸调即 NPE —— 必须先守卫, 无 FM 时走既有降级:
                // curLoadMinWorkTime 置哨兵值 → sEngWorkTime 显示 "-"
                // (pL == null 域: blkx 有但 initEngineLoad 未产出 engLoad 的畸形 FM)
                _ => return None,
            };
            // curLoad = blkx.findmaxLoad(pL, nwaterTemp, noilTemp);
            // 减去时间
            let mut min_work_time = (99999 * 1000) as f64;

            // 水冷
            let cur_w_load = blkx.findmax_water_load(p_l, nwater_temp);
            for i in 0..blkx.max_eng_load {
                if i < cur_w_load {
                    if p_l[i as usize].work_time != 0.0 {
                        p_l[i as usize].cur_water_work_time_mili -= poll_cycle_duration_ms as f64;
                        if p_l[i as usize].cur_water_work_time_mili < min_work_time {
                            min_work_time = p_l[i as usize].cur_water_work_time_mili;
                        }
                    }
                } else if eng_off {
                    // 关闭引擎直接回满
                    if cur_w_load == 0 || p_l[(cur_w_load - 1) as usize].work_time < 0.1 {
                        // Application.debugPrint("回复水温耐久条");
                        p_l[i as usize].cur_water_work_time_mili =
                            p_l[i as usize].work_time * 1000.0;
                    }
                } else {
                    // 大于load且工作时长不满则进行恢复（WEP时也允许恢复）
                    if p_l[i as usize].recover_time != 0.0
                        && (1000.0 * p_l[i as usize].work_time
                            > p_l[i as usize].cur_water_work_time_mili)
                    {
                        p_l[i as usize].cur_water_work_time_mili += poll_cycle_duration_ms as f64
                            * p_l[i as usize].work_time
                            / p_l[i as usize].recover_time;
                    }
                }
            }

            // 油冷
            let cur_o_load = blkx.findmax_oil_load(p_l, noil_temp);
            for i in 0..blkx.max_eng_load {
                if i < cur_o_load {
                    if p_l[i as usize].work_time != 0.0 {
                        p_l[i as usize].cur_oil_work_time_mili -= poll_cycle_duration_ms as f64;
                        if p_l[i as usize].cur_oil_work_time_mili < min_work_time {
                            min_work_time = p_l[i as usize].cur_oil_work_time_mili;
                        }
                    }
                } else if eng_off {
                    // 关闭引擎直接回满
                    if cur_o_load == 0 || p_l[(cur_o_load - 1) as usize].work_time < 0.1 {
                        // Application.debugPrint("回复油温耐久条");
                        p_l[i as usize].cur_oil_work_time_mili = p_l[i as usize].work_time * 1000.0;
                    }
                } else {
                    // 大于load且工作时长不满则进行恢复（WEP时也允许恢复）
                    if p_l[i as usize].recover_time != 0.0
                        && (1000.0 * p_l[i as usize].work_time
                            > p_l[i as usize].cur_oil_work_time_mili)
                    {
                        p_l[i as usize].cur_oil_work_time_mili += poll_cycle_duration_ms as f64
                            * p_l[i as usize].work_time
                            / p_l[i as usize].recover_time;
                    }
                }
            }

            Some((cur_w_load, cur_o_load, min_work_time))
        })(); // —— eng_load_state 锁随闭包结束释放 (写回前必须放下, 锁不嵌套)

        let mut d = write_data(&self.data);
        match outcome {
            Some((_cur_w_load, _cur_o_load, min_work_time)) => {
                // (curWLoad/curOLoad 字段已删: 全库无读者, 引擎载荷态真身
                //  在 FMHandle.eng_load_state, 此处只落 min_work_time)
                // curLoadMinWorkTime = minWorkTime;
                d.cur_load_min_work_time = min_work_time;
            }
            // 守卫降级: curLoadMinWorkTime = 99999 * 1000; return;
            None => {
                d.cur_load_min_work_time = (99999 * 1000) as f64;
            }
        }
    }

    /// 重置引擎耐久计时（engLoad 为共享会话状态, 就地改写语义见 FMHandle javadoc 声明,
    /// "换机 = 新 Blx 实例" 天然保证会话态不串机, 此处保持就地改写不变）。
    ///
    /// @param fm 本周期 FM 句柄快照（R1 下传）
    // PORT(形态): Java 为实例方法 `public void resetEngLoad(FMHandle fm)` (L1516),
    // 方法体不触碰任何 Service 实例字段 → 关联函数形态, 与 reset_varia 的既有
    // 调用点 `Self::reset_eng_load(&fm)` 零改动衔接。
    // PORT(会话态提升): 改写目标从 blkx.engLoad 换成 fm.eng_load_state (blkx 本体
    // 保持不可变解析产物); 锁内纯赋值无 IO。
    pub(super) fn reset_eng_load(fm: &FMHandle) {
        // R2 hasFM 守卫: blkx 非 null 即 READY, 无 FM 时无耐久数据可重置
        if let Some(blkx) = &fm.blkx {
            let mut session = fm
                .eng_load_state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            // 畸形 FM 在 Java 裸索引即 NPE (resetvaria 调用域由 run 顶层 catch 兜住),
            // expect panic 同构
            let p_l = session
                .as_deref_mut()
                .expect("PORT: Java NPE — blkx.engLoad 为 null");
            for idx in 0..blkx.max_eng_load {
                p_l[idx as usize].cur_water_work_time_mili =
                    p_l[idx as usize].work_time * 1000.0;
                p_l[idx as usize].cur_oil_work_time_mili =
                    p_l[idx as usize].work_time * 1000.0;
            }
        }
    }
}

// PORT(接线点, 主线 calculate 波次): Java calculate 链在 updateTemp 之后调用
// checkOverheat (Service.java L1130-1131 附近), Rust 对位 =
// service_loop.rs calculate 内 `self.update_temp();` 之后插 `self.check_overheat(&fm);`。

// =====================================================================
// Tests — 覆盖 checkOverheat 三场景 + R2 守卫 + resetEngLoad (Java 无独立
// 测试, 按批次验收要求补齐; 断言值 = 公式直算)
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use vm_core::blkx::{Blkx, EngineLoad};
    use vm_core::bus::EventBus;
    use vm_core::fm::FMManager;
    use vm_core::flight_data_bus::FlightDataBus;

    use crate::service_loop::ServiceConfig;

    fn new_service() -> Service {
        let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
        let bus = Arc::new(FlightDataBus::new());
        Service::new(ServiceConfig::default(), fm, bus)
    }

    /// 两档 engLoad 的测试 blkx (WaterLimit 80/60, OilLimit 60/50,
    /// WorkTime 300/60, RecoverTime 600/30; cur 初值 = 解析产物形态 WorkTime*1000
    /// 或由参数覆写)。maxEngLoad=2。
    // PORT: Blkx 含 blkx 模块私有字段 → 跨 crate 无法用 struct 字面量 +
    // ..Default::default() (E0451), 走 default() 后逐字段赋值 (tests.rs 同款形态)
    fn test_blkx(
        cur_water0: f64,
        cur_oil0: f64,
        cur_water1: f64,
        cur_oil1: f64,
    ) -> Blkx {
        let mk = |water_limit: f64,
                  oil_limit: f64,
                  work_time: f64,
                  recover_time: f64,
                  cur_water: f64,
                  cur_oil: f64| EngineLoad {
            water_limit,
            oil_limit,
            work_time,
            recover_time,
            cur_water_work_time_mili: cur_water,
            cur_oil_work_time_mili: cur_oil,
        };
        let mut blkx = Blkx::default();
        blkx.eng_load = Some(vec![
            mk(80.0, 60.0, 300.0, 600.0, cur_water0, cur_oil0),
            mk(60.0, 50.0, 60.0, 30.0, cur_water1, cur_oil1),
        ]);
        blkx.max_eng_load = 2;
        blkx
    }

    /// 喂 checkOverheat 的输入面: power[0]/throttle/两温度/轮询周期 50ms
    fn seed_inputs(svc: &mut Service, power0: f64, throttle: i32, nwater: f64, noil: f64) {
        let mut d = write_data(&svc.data);
        let s = d.s_state.as_mut().unwrap();
        s.power = vec![power0];
        s.throttle = throttle;
        d.nwater_temp = nwater;
        d.noil_temp = noil;
        d.poll_cycle_duration_ms = 50; // run() 轮询的量化产物 (直驱需手工模拟)
    }

    fn session(fm: &FMHandle) -> Vec<EngineLoad> {
        fm.eng_load_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .unwrap()
    }

    /// 场景 A 超温递减: 水温 90 ≥ 80/60 两档 → curWLoad=2, 油温 70 ≥ 60/50 →
    /// curOLoad=2, 全档递减 50ms; minWorkTime 汇聚到最小值 (档 1 水/油 59950)
    #[test]
    fn overheat_decrement_when_above_limits() {
        let mut svc = new_service();
        seed_inputs(&mut svc, 1500.0, 100, 90.0, 70.0);
        // 解析产物形态初值: 档0 水/油 300000, 档1 60000
        let blkx = test_blkx(300000.0, 300000.0, 60000.0, 60000.0);
        let fm = FMHandle::ready(Some("test-plane".into()), Some(blkx), 0.0, 0.0, None);

        svc.check_overheat(&fm);

        let p = session(&fm);
        assert_eq!(p[0].cur_water_work_time_mili, 299950.0, "档0 水耐久 -50ms");
        assert_eq!(p[1].cur_water_work_time_mili, 59950.0, "档1 水耐久 -50ms");
        assert_eq!(p[0].cur_oil_work_time_mili, 299950.0, "档0 油耐久 -50ms");
        assert_eq!(p[1].cur_oil_work_time_mili, 59950.0, "档1 油耐久 -50ms");
        let d = read_data(&svc.data);
        assert_eq!(d.cur_load_min_work_time, 59950.0, "minWorkTime 汇聚最小值");
        // blkx 本体保持不可变解析产物 (会话态提升契约)
        assert_eq!(
            fm.blkx.as_ref().unwrap().eng_load.as_ref().unwrap()[1]
                .cur_water_work_time_mili,
            60000.0
        );
    }

    /// 场景 B 降档恢复: 水温 70 < 80 → curWLoad=0 (无档超温), 耐久不满的档恢复
    /// += poll*WorkTime/RecoverTime; 已满的档 (1000*WorkTime > cur 为假) 不动
    #[test]
    fn overheat_recover_when_below_limits() {
        let mut svc = new_service();
        seed_inputs(&mut svc, 1500.0, 100, 70.0, 40.0);
        // 档0 水 200000 (<300000 恢复), 油同; 档1 60000 已满不恢复
        let blkx = test_blkx(200000.0, 200000.0, 60000.0, 60000.0);
        let fm = FMHandle::ready(Some("test-plane".into()), Some(blkx), 0.0, 0.0, None);

        svc.check_overheat(&fm);

        let p = session(&fm);
        // 恢复量 = 50 * 300/600 = 25 (水), 油温 40 < 60 → curOLoad=0 同域恢复
        assert_eq!(p[0].cur_water_work_time_mili, 200025.0);
        assert_eq!(p[0].cur_oil_work_time_mili, 200025.0);
        // 档1 已满: 1000*60 > 60000 为假 → 不恢复不变
        assert_eq!(p[1].cur_water_work_time_mili, 60000.0);
        assert_eq!(p[1].cur_oil_work_time_mili, 60000.0);
        let d = read_data(&svc.data);
        // 无超温档 → minWorkTime 保持哨兵 99999000
        assert_eq!(d.cur_load_min_work_time, 99999000.0);
    }

    /// 场景 C 关机回满: power[0]==0 且 throttle>0 → engOff; curWLoad==0 时
    /// 全档直接回满 WorkTime*1000 (不按 RecoverTime 恢复)
    #[test]
    fn overheat_eng_off_refill() {
        let mut svc = new_service();
        seed_inputs(&mut svc, 0.0, 90, 70.0, 40.0);
        // 耐久已耗损 (水 12345 / 油 23456)
        let blkx = test_blkx(12345.0, 23456.0, 999.0, 888.0);
        let fm = FMHandle::ready(Some("test-plane".into()), Some(blkx), 0.0, 0.0, None);

        svc.check_overheat(&fm);

        let p = session(&fm);
        assert_eq!(p[0].cur_water_work_time_mili, 300000.0, "关机回满 WorkTime*1000");
        assert_eq!(p[0].cur_oil_work_time_mili, 300000.0);
        assert_eq!(p[1].cur_water_work_time_mili, 60000.0);
        assert_eq!(p[1].cur_oil_work_time_mili, 60000.0);
        let d = read_data(&svc.data);
        assert_eq!(d.cur_load_min_work_time, 99999000.0, "回满分支不进 minWorkTime");
    }

    /// 场景 C 分流: engOff 但 curWLoad>0 且上一档 WorkTime>=0.1 → 不回满,
    /// 走恢复分支 (Java L606/L634 的 pL[curLoad-1].WorkTime < 0.1 闸门)
    #[test]
    fn overheat_eng_off_hot_engine_stays_depleting() {
        let mut svc = new_service();
        // 水温 90 → curWLoad=2 (引擎关了但还热), power=0/throttle=90 → engOff
        seed_inputs(&mut svc, 0.0, 90, 90.0, 40.0);
        let blkx = test_blkx(100000.0, 100000.0, 100000.0, 100000.0);
        let fm = FMHandle::ready(Some("test-plane".into()), Some(blkx), 0.0, 0.0, None);

        svc.check_overheat(&fm);

        let p = session(&fm);
        // i < curWLoad (2) → 两档继续递减 50 (WorkTime!=0), 不回满不恢复
        assert_eq!(p[0].cur_water_work_time_mili, 99950.0);
        assert_eq!(p[1].cur_water_work_time_mili, 99950.0);
        // 油温 40 → curOLoad=0 → engOff 回满分支
        assert_eq!(p[0].cur_oil_work_time_mili, 300000.0, "油侧关机回满");
        assert_eq!(p[1].cur_oil_work_time_mili, 60000.0);
        let d = read_data(&svc.data);
        assert_eq!(d.cur_load_min_work_time, 99950.0, "水侧递减汇聚");
    }

    /// R2 守卫: 无 FM (blkx=null) → curLoadMinWorkTime 哨兵 99999*1000 后早退
    #[test]
    fn overheat_guard_without_fm() {
        let mut svc = new_service();
        seed_inputs(&mut svc, 1500.0, 100, 90.0, 70.0);
        let fm = FMHandle::missing(Some("ghost".into()));

        svc.check_overheat(&fm);

        let d = read_data(&svc.data);
        assert_eq!(d.cur_load_min_work_time, 99999000.0, "哨兵 → sEngWorkTime 显示 \"-\"");
        // 会话态未被触碰
        assert!(fm.eng_load_state.lock().unwrap().is_none());
    }

    /// resetEngLoad: 全档 curWater/OilWorkTimeMili 重置 WorkTime*1000
    #[test]
    fn reset_eng_load_restores_full_work_time() {
        let blkx = test_blkx(1.0, 2.0, 3.0, 4.0);
        let fm = FMHandle::ready(Some("test-plane".into()), Some(blkx), 0.0, 0.0, None);
        // 会话态先耗损 (ready 克隆的是解析产物, 直接改写会话态模拟飞行耗损)
        {
            let mut s = fm.eng_load_state.lock().unwrap();
            let p = s.as_deref_mut().unwrap();
            p[0].cur_water_work_time_mili = 111.0;
            p[0].cur_oil_work_time_mili = 222.0;
            p[1].cur_water_work_time_mili = 333.0;
            p[1].cur_oil_work_time_mili = 444.0;
        }

        Service::reset_eng_load(&fm);

        let p = session(&fm);
        assert_eq!(p[0].cur_water_work_time_mili, 300000.0);
        assert_eq!(p[0].cur_oil_work_time_mili, 300000.0);
        assert_eq!(p[1].cur_water_work_time_mili, 60000.0);
        assert_eq!(p[1].cur_oil_work_time_mili, 60000.0);
    }

    /// resetEngLoad R2 守卫: 无 FM → no-op (不 panic)
    #[test]
    fn reset_eng_load_guard_without_fm() {
        let fm = FMHandle::missing(Some("ghost".into()));
        Service::reset_eng_load(&fm); // 不应 panic
    }
}
