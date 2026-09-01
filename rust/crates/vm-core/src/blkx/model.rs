//! 对应 Java: `src/parser/Blkx.java` 的 getter/计算方法区 (D4 拆分: model.rs)。
//!
//! 本波覆盖 (字段区见 mod.rs `Blkx`):
//! - L523-631 中不依赖原始文本抽取原语的部分: `findmaxWaterLoad`/`findmaxOilLoad`/
//!   `getVersion`
//! - L676-1660 的纯字段计算面: `getAoAHighVWing`/`getAoALowVWing`/`getVNEVWing`/
//!   `getMNEVWing`/`getMaxAllowGloadForWeight`/`subSt`/`getperformancedata`/`init`
//! - L1978-2028 (D4 行号表外的字段态方法, getload 依赖面提前落地):
//!   `finalizeLoading`/`calculatePeakThrust`/`peakThrust`
//!
//! PORT: 依赖 getone/cut/getArray 等 reader.rs 波次原语的方法 (getPartsFm/
//! extractRpmFromThrottleAuto/getEngineLoad/showEngineLoad/WritePartsFm/getdoubles/
//! getdouble/getdouble_exc/initEngineLoad/getload/transUnit/getAllplotdata/
//! getplotdata) 不在本波, 见 mod.rs 模块级波次边界注。

use super::types::EngineLoad;
use super::Blkx;
use crate::g;
use crate::lang::Lang;

/// 通用 sweep 插值承接说明 (对应 Java 私有方法 `interpolateSweepDouble`, L718-737):
/// 原码将 sweepLevels 逐元素拷入临时数组后做区间线性插值。按项目规约
/// (CLAUDE.md "Use Interpolation for all interpolation / Never duplicate") 与任务指令,
/// 本文件统一改经 [`crate::interpolation::interp_sweep_level`] (该函数即 Java
/// `Interpolation.interpSweepLevel` 的一比一翻译), 区间比较与线性公式逐句同构。
/// 语义差 (病态域, 真机 FM 不可达, 两处标注):
/// 1. 原码 `range == 0` 精确判零返回 values[i], lerp 为 `|Δ| < 1e-9` 提前返回 y0 —
///    相邻档位 sweep 差落在 (0, 1e-9) 时两实现对分叉 (真机档位 0.0/0.25/0.5/1.0);
/// 2. 原码 count==0 时 values[0] 抛 AIOOBE; interp_sweep_level 空表返回默认值 —
///    各 getter 的 `null || size() <= 1` 守卫先于调用, 空表路径不可达。
///
/// 求值时机保真: AoA 两 getter 以 (sweep, 值) 对的急切构造复刻 Java 预拷全表
/// (任何档位 noFlaps 为 null 即 NPE); vne/mach 提取器无 null 路径, 闭包直取等价。
impl Blkx {
    /* 计算可变翼 */
    /// 对应 Java `public double getAoAHighVWing(double vwing, int flaps_percent)` (L740-757)。
    pub fn get_aoa_high_v_wing(&self, vwing: f64, flaps_percent: i32) -> f64 {
        if vwing == 0.0 {
            /* 计算flaps */
            // PORT: Java 直接解引用 NoFlapsWing/FullFlapsWing (未加载时 NPE) —
            // unwrap panic 复刻同一崩溃语义 (§1 RuntimeException→panic)
            let no_flaps = self.no_flaps_wing.as_ref().unwrap();
            let full_flaps = self.full_flaps_wing.as_ref().unwrap();
            return no_flaps.aoa_crit_high
                + (full_flaps.aoa_crit_high - no_flaps.aoa_crit_high) * flaps_percent as f64 / 100.0;
        }
        if self.sweep_levels.as_ref().is_none_or(|l| l.len() <= 1) {
            return self.no_flaps_wing.as_ref().unwrap().aoa_crit_high;
        }
        // Java L750-755: values[i]=noFlaps.AoACritHigh / sweeps[i]=sweep 预拷两表
        let levels = self.sweep_levels.as_deref().unwrap();
        let pairs: Vec<(f64, f64)> = levels
            .iter()
            .map(|l| (l.sweep, l.no_flaps.as_ref().unwrap().aoa_crit_high))
            .collect();
        crate::interpolation::interp_sweep_level(
            vwing,
            Some(&pairs),
            |p| p.1,
            |p| p.0,
            0.0, // default 仅 levels null/空时生效 — 上方守卫后不可达
        )
    }

    /// 对应 Java `public double getAoALowVWing(double vwing, int flaps_percent)` (L759-771)。
    /// PORT: Java 形参 flaps_percent 保留但未用 (无 vwing==0 襟翼混合分支, 与 High 版
    /// 的不对称是源码本意) — Rust 以 `_` 前缀消未用告警, 签名保真。
    pub fn get_aoa_low_v_wing(&self, vwing: f64, _flaps_percent: i32) -> f64 {
        if self.sweep_levels.as_ref().is_none_or(|l| l.len() <= 1) {
            return self.no_flaps_wing.as_ref().unwrap().aoa_crit_low;
        }
        // Java L763-768: values[i]=noFlaps.AoACritLow / sweeps[i]=sweep 预拷两表
        let levels = self.sweep_levels.as_deref().unwrap();
        let pairs: Vec<(f64, f64)> = levels
            .iter()
            .map(|l| (l.sweep, l.no_flaps.as_ref().unwrap().aoa_crit_low))
            .collect();
        crate::interpolation::interp_sweep_level(
            vwing,
            Some(&pairs),
            |p| p.1,
            |p| p.0,
            0.0, // default 仅 levels null/空时生效 — 上方守卫后不可达
        )
    }

    /// 对应 Java `public double getVNEVWing(double vwing)` (L773-785)。
    pub fn get_vne_v_wing(&self, vwing: f64) -> f64 {
        if self.sweep_levels.as_ref().is_none_or(|l| l.len() <= 1) {
            return self.vne;
        }
        crate::interpolation::interp_sweep_level(
            vwing,
            self.sweep_levels.as_deref(),
            |l| l.vne,
            |l| l.sweep,
            self.vne,
        )
    }

    /// 对应 Java `public double getMNEVWing(double vwing)` (L787-799)。
    pub fn get_mne_v_wing(&self, vwing: f64) -> f64 {
        if self.sweep_levels.as_ref().is_none_or(|l| l.len() <= 1) {
            return self.vne_mach;
        }
        crate::interpolation::interp_sweep_level(
            vwing,
            self.sweep_levels.as_deref(),
            |l| l.vne_mach,
            |l| l.sweep,
            self.vne_mach,
        )
    }

    /// 对应 Java `public double[] getMaxAllowGloadForWeight(double currentWeight)` (L808-815)。
    /// Calculates the maximum allowable G-load range based on current aircraft weight.
    /// As fuel burns off, the aircraft can sustain higher G-loads within structural limits.
    ///
    /// @param currentWeight Current total weight in kg (typically nofuelweight + mfuel)
    /// @return double[2]: [0]=negative G limit (e.g., -4.5), [1]=positive G limit (e.g., +11.2)
    // PORT: Java 回退分支原样返回字段引用 (可能为 null, 调用方 VoiceWarning L839 先行
    // 判 rawWingCritOverload != null 才调用) → Option<[f64; 2]> 透传 None (§1 null→Option)
    pub fn get_max_allow_gload_for_weight(&self, current_weight: f64) -> Option<[f64; 2]> {
        if self.raw_wing_crit_overload.is_none() || current_weight <= 0.0 {
            return self.max_allow_gload; // Fallback to static values
        }
        let raw = self.raw_wing_crit_overload.unwrap();
        let negative_g = 1.2 * (2.0 * raw[0] / (g * current_weight) + 1.0);
        let positive_g = 1.2 * (2.0 * raw[1] / (g * current_weight) - 1.0);
        Some([negative_g, positive_g])
    }

    /// 对应 Java `public int findmaxWaterLoad(engineLoad[] eL, double water)` (L581-591)。
    // PORT: eL 短于 max_eng_load 时 Java AIOOBE ↔ 切片索引 panic 同构
    pub fn findmax_water_load(&self, e_l: &[EngineLoad], water: f64) -> i32 {
        for i in 0..self.max_eng_load {
            // 大于还是小于等于呢？
            if water < e_l[i as usize].water_limit {
                return i;
            }
            // if (Math.round(water) < eL[i].WaterLimit)
            // return i;
        }

        self.max_eng_load
    }

    /// 对应 Java `public int findmaxOilLoad(engineLoad[] eL, double oil)` (L593-603)。
    pub fn findmax_oil_load(&self, e_l: &[EngineLoad], oil: f64) -> i32 {
        for i in 0..self.max_eng_load {
            // 大于还是小于等于呢？
            if oil < e_l[i as usize].oil_limit {
                return i;
            }
            // if (Math.round(oil) < eL[i].OilLimit)
            // return i;
        }

        self.max_eng_load
    }

    /// 对应 Java `public String getVersion()` (L605-631) — 读 `./data/aces/version`。
    // PORT: Java FileReader 用平台默认字符集 (中文 Windows=GBK) 读完全程; Rust
    // BufReader::lines() 为 strict UTF-8, 非法字节产出 Err → break 保留半程 sb —
    // 版本文件为 ASCII 版本号, 域内等价。行语义: readLine 以 \n/\r/\r\n 为行界,
    // lines() 仅按 \n 切并剥行尾单个 \r (单独 \r 不终止行) — CRLF 文件等价。
    // IO 失败 ExceptionHelper.logAndContinue 吞掉保留半程 sb ↔ Err 分支中断循环
    // (crate 内暂无 Logger, 吞错语义一致, 见 mod.rs 波次注)
    pub fn get_version(&self) -> Option<String> {
        let file = std::path::Path::new("./data/aces/version");
        let mut tmp_data: Option<String> = None;
        if file.exists() {
            let mut sb = String::new();
            if let Ok(f) = std::fs::File::open(file) {
                use std::io::{BufRead, BufReader};
                for line in BufReader::new(f).lines() {
                    match line {
                        Ok(s) => {
                            sb.push_str(&s);
                            sb.push('\n');
                        }
                        // PORT: Java IOException catch → logAndContinue, sb 保留半程
                        Err(_) => break,
                    }
                }
            }
            // PORT: Java 打开失败 (FileNotFoundException) 同被吞掉, sb 保持 ""
            tmp_data = Some(sb);
        } else {
        }
        tmp_data
    }

    /// 对应 Java `public void getperformancedata(String t)` (L1581-1583, 空方法)。
    pub fn getperformancedata(&self, _t: &str) {}

    /// 对应 Java `public String subSt(String t)` (L1585-1588) — 剥首尾各一字符。
    // PORT: Java substring(1, length-1) 按 UTF-16 码元; 域内输入为 FM 行值
    // (PASSPORT.UNITSYSTEM 等, ASCII) — 字节切片等价 (§2.1); len<2 时 Java 抛
    // StringIndexOutOfBoundsException ↔ 切片范围 panic 同构
    pub fn sub_st(&self, t: &str) -> String {
        t[1..t.len() - 1].to_string()
    }

    /// 对应 Java `public void init(String t)` (L1660-1663)。
    // PORT: Java `fmdata = Lang.noblkx` 读启动期 initLang() 已覆写的静态字段;
    // Rust 无全局 Lang 状态 (§2.9 AppState 属后续波次), 以 init_lang() 静态表
    // 快照现取 — lang/table.rs 与 cur.properties 逐键 oracle 对拍, 值一致
    pub fn init(&mut self, t: &str) {
        self.data = Some(t.to_string());
        self.fmdata = Some(Lang::init_lang().noblkx.to_string());
    }

    /// 对应 Java `public void finalizeLoading()` (L1978-1981)。
    /// Releases the large raw data string after parsing is complete to save memory.
    pub fn finalize_loading(&mut self) {
        self.data = None;
        // We keep fmdata as it is used by FMDataOverlay
    }

    /// 对应 Java `private double calculatePeakThrust(double[][] table)` (L2007-2019)。
    /// 遍历推力表找全局最大值
    /// @param table 推力表 [altitude][velocity]
    /// @return 峰值推力(kgf)
    // PORT: Java private → pub(super) (blkx 模块树内可见: getload 在 reader 波次
    // 落地为本模块树的兄弟 impl, 对应"类内可见"); Java double[][] 传 null →
    // Option<&[Vec<f64>]>; 内层行短于 vel_thr_num 时 Java AIOOBE ↔ 索引 panic
    #[allow(dead_code)] // 唯一调用方 getload (L983-984) 在 reader 波次
    pub(super) fn calculate_peak_thrust(&self, table: Option<&[Vec<f64>]>) -> f64 {
        if table.is_none() || self.alt_thr_num == 0 || self.vel_thr_num == 0 {
            return 0.0;
        }
        let table = table.unwrap();

        let mut peak = 0.0f64;
        for i in 0..self.alt_thr_num {
            for j in 0..self.vel_thr_num {
                if table[i as usize][j as usize] > peak {
                    peak = table[i as usize][j as usize];
                }
            }
        }
        peak
    }

    /// 对应 Java `public double peakThrust(boolean isAfterburner)` (L2026-2028)。
    /// 获取峰值推力
    /// @param isAfterburner true=加力推力，false=军用推力
    /// @return 峰值推力(kgf)
    pub fn peak_thrust(&self, is_afterburner: bool) -> f64 {
        if is_afterburner {
            self.peak_thr_aft
        } else {
            self.peak_thr_mil
        }
    }
}

// =====================================================================
// Tests — 期望值来自 Java 8 oracle 对拍 (§5.1): build/oracle/BlkxModelOracle.java
// 逐字提取的运行脚本在 OpenJDK 1.8.0_342 (与 bin/ 现役 class 同版) 实测 dump,
// double 以 doubleToLongBits 十六进制输出, Rust 侧 to_bits() 逐位断言。
// =====================================================================
#[cfg(test)]
mod tests;
