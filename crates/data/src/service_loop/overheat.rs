//! 引擎过热/耐久度检查 (check_overheat) + 耐久计时重置 (reset_eng_load)
//! (impl Service 跨文件块, 方法 pub(super); calculate 链 update_temp 之后接线)。
//!
//! 会话态走向 (handle.rs "会话态提升" 裁决): Java 就地改写 `blkx.engLoad[i]`,
//! Rust 改写 `fm.eng_load_state` 的 Mutex 锁内副本 (blkx 保持不可变解析产物)。
//! 锁纪律: 锁内纯计算无 IO, 且与 ServiceData 的 RwLock **不嵌套** —— 输入先经
//! with_snapshot 快照、锁内算完释放、结果再经 apply 写回 (单向 data→session
//! 取值序, 杜绝 ABBA)。
use super::Service;
use kernel::fm::FMHandle;

impl Service {
    /// 引擎过热/耐久度检查。
    ///
    /// - `fm`: 本周期 FM 句柄快照（R1 下传, 单周期内同一 Blkx 实例）
    pub(super) fn check_overheat(&mut self, fm: &FMHandle) {
        // 输入快照 (锁外取, §2.8): sState.power[0]/throttle + 温度/轮询周期。
        // power 空数组索引 panic = Java AIOOBE 同构 (run 顶层 catch_unwind 兜住)
        let (power0, throttle, poll_cycle_duration_ms, nwater_temp, noil_temp) = self
            .with_snapshot(|d| {
                let s = d.s_state.as_ref().unwrap();
                (
                    s.power[0],
                    s.throttle,
                    d.poll_cycle_duration_ms,
                    d.engine.nwater_temp,
                    d.engine.noil_temp,
                )
            });
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
            let mut session = fm.eng_load_state.lock().unwrap_or_else(|e| e.into_inner());
            let (fmdata, p_l) = match (fm.fmdata.as_ref(), session.as_deref_mut()) {
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
            let cur_w_load = fmdata.findmax_water_load(p_l, nwater_temp);
            for i in 0..fmdata.max_eng_load {
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
            let cur_o_load = fmdata.findmax_oil_load(p_l, noil_temp);
            for i in 0..fmdata.max_eng_load {
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

        self.apply(|d| match outcome {
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
        });
    }

    /// 重置引擎耐久计时（engLoad 为共享会话状态, 就地改写语义见 FMHandle javadoc 声明,
    /// "换机 = 新 Blx 实例" 天然保证会话态不串机, 此处保持就地改写不变）。
    ///
    /// - `fm`: 本周期 FM 句柄快照（R1 下传）
    // PORT(形态): Java 为实例方法 resetEngLoad(FMHandle fm),
    // 方法体不触碰任何 Service 实例字段 → 关联函数形态, 与 reset_varia 的既有
    // 调用点 `Self::reset_eng_load(&fm)` 零改动衔接。
    // PORT(会话态提升): 改写目标从 blkx.engLoad 换成 fm.eng_load_state (blkx 本体
    // 保持不可变解析产物); 锁内纯赋值无 IO。
    pub(super) fn reset_eng_load(fm: &FMHandle) {
        // R2 hasFM 守卫: blkx 非 null 即 READY, 无 FM 时无耐久数据可重置
        if let Some(fmdata) = &fm.fmdata {
            let mut session = fm.eng_load_state.lock().unwrap_or_else(|e| e.into_inner());
            // 畸形 FM 在 Java 裸索引即 NPE (resetvaria 调用域由 run 顶层 catch 兜住),
            // expect panic 同构
            let p_l = session
                .as_deref_mut()
                .expect("PORT: Java NPE — fmdata.engLoad 为 null");
            for idx in 0..fmdata.max_eng_load {
                p_l[idx as usize].cur_water_work_time_mili = p_l[idx as usize].work_time * 1000.0;
                p_l[idx as usize].cur_oil_work_time_mili = p_l[idx as usize].work_time * 1000.0;
            }
        }
    }
}

// =====================================================================
// Tests — 覆盖 checkOverheat 三场景 + R2 守卫 + resetEngLoad (Java 无独立
// 测试, 按批次验收要求补齐; 断言值 = 公式直算)
// =====================================================================
