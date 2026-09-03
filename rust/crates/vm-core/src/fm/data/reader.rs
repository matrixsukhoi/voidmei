//! Blkx 的 getload 全量装载 (D4 拆分: reader.rs)。JSON 数据源直供 —
//! BlkText 文本链已随 blkx→json 迁移退役删除
//! (迁移期 2832 对全量位级对拍验证等价):
//! - `getload_from(&JsonSrc)` — FM 全量装载 + fmdata 摘要串构造
//!   (语句顺序/公式/Java bug 保真逐行直译; panic 由 parse_named_json 的
//!   catch_unwind 收敛 Err)
//! - `get_parts_fm`/`get_engine_load`/`init_engine_load`/
//!   `extract_rpm_from_throttle_auto` — 装载辅助
//!
//! 抽取原语在 json.rs 的 JsonSrc: get_f64 族**数值直读** (Number → as_f64 →
//! as f32 → widen, 免字符串往返)、get_str 文本形态 ("null" 哨兵/Bool 形态)、
//! section 子树引用。
//!
//! 2026-09 死代码清理: Java 遗留死存储 (cl_a/aileron_defl/wx 族/平铺 aoa 族/
//! v50/v100 快照字段/oilload/wtload/tmload 装箱遗留/MIL 推力表等) 与 PASSPORT
//! 曲线链 (loc..loc3/transUnit/getplotdata — Java DrawFrame 消费未迁移至 Rust,
//! Rust 生产零消费) 已删。
//!
//! 波14 拆解: getload_from 按装载阶段提取为 load_* 子函数 (引擎/喷气推力表/
//! 增压器/转速与负载/重量面积/升力系数/fmdata 摘要), 子函数按调用序排列,
//! 语句序零变化。
//!
//! 构造入口在 json.rs: [`Blkx::parse_named_json`] (doLoad=true) /
//! [`Blkx::parse_named_opts_json`] (中央文件只读) / [`Blkx::parse_str_json`]
//! (fuzz 注入)。

use super::json::JsonSrc;
use super::types::{EngineLoad, FmParts, SweepLevel};
use super::{CompressorData, FmData};
use crate::base::physics_constants::g;
use crate::lang::Lang;
use crate::base::logger;
use crate::telemetry::parser::state::MAX_ENG_NUM;

/// [`java_format`] 的实参 (getload fmdata 串构造专用)。
enum FmtArg {
    /// Java `%s` (String.toString 形态)
    S(String),
    /// Java `%d` (int)
    D(i32),
    /// Java `%.Mf` (无宽度域; M 位小数 HALF_UP — crate::base::format 同源语义)
    F(f64, u8),
}

/// Java `String.format(tpl, args...)` 的受限子集 — getload 的 fmdata 摘要串构造。
/// 模板来自 Lang 运行时表 (可被 lang/cur.properties 覆盖),
/// 不能编译期展开, 故运行时扫描 `%` 转换: `%s`/`%d`/`%.Mf`/`%%` (getload 用到的
/// 全部形态; 宽度域未用不支持)。参数耗尽 = Java MissingFormatArgumentException
/// → panic (由 parse_named_opts_json 的 catch_unwind 收敛, 同一防线)。
fn java_format(tpl: &str, args: &[FmtArg]) -> String {
    let mut out = String::new();
    let mut ai = 0usize;
    let cs: Vec<char> = tpl.chars().collect();
    let mut i = 0usize;
    while i < cs.len() {
        let c = cs[i];
        if c != '%' {
            out.push(c);
            i += 1;
            continue;
        }
        // '%' 转换
        if i + 1 >= cs.len() {
            // 尾部孤立 '%' — Java 末尾抛 UnknownFormatConversionException,
            // 域内模板恒以 \n 收尾不可达; 保真 panic
            panic!("java_format: 模板尾孤立 '%': {tpl}");
        }
        let mut j = i + 1;
        // 可选宽度数字 (未用, 跳过保兼容)
        while j < cs.len() && cs[j].is_ascii_digit() {
            j += 1;
        }
        // 可选 .M 精度
        let mut prec: u8 = 6;
        if j < cs.len() && cs[j] == '.' {
            j += 1;
            let mut p: u8 = 0;
            while j < cs.len() && cs[j].is_ascii_digit() {
                p = p.saturating_mul(10).saturating_add(cs[j].to_digit(10).unwrap() as u8);
                j += 1;
            }
            prec = p;
        }
        let conv = cs[j];
        if conv == '%' {
            // %% — 字面百分号, 不消耗实参
            out.push('%');
            i = j + 1;
            continue;
        }
        let arg = args
            .get(ai)
            .unwrap_or_else(|| panic!("java_format: 参数耗尽 (模板 {tpl})"));
        ai += 1;
        match conv {
            's' => {
                if let FmtArg::S(v) = arg {
                    out.push_str(v);
                } else {
                    panic!("java_format: %s 收到非 S 实参 (模板 {tpl})");
                }
            }
            'd' => {
                if let FmtArg::D(v) = arg {
                    out.push_str(&v.to_string());
                } else {
                    panic!("java_format: %d 收到非 D 实参 (模板 {tpl})");
                }
            }
            'f' => {
                if let FmtArg::F(v, _p) = arg {
                    // 最短往返十进制 HALF_UP (java_f, 非 FastNumberFormatter 的
                    // 二进制半舍入 — 2.675 → "2.68" oracle 钉死)
                    out.push_str(&crate::base::format::java_f(*v, prec as usize));
                } else {
                    panic!("java_format: %f 收到非 F 实参 (模板 {tpl})");
                }
            }
            other => panic!("java_format: 不支持的转换 %{other} (模板 {tpl})"),
        }
        i = j + 1;
    }
    out
}

/// load_lift_coeffs 的产出: 升力部件族 + 转动惯量/过载限制原值
/// (fmdata 摘要段与部件落位共用, 编排层传递)。
struct LiftLoad {
    no_flaps_wing: FmParts,
    full_flaps_wing: FmParts,
    /// 摘要串用的 v50/v100 快照 (Java 存字段的引用共享, Rust 无字段消费方)
    no_flaps_wing_v50: FmParts,
    no_flaps_wing_v100: FmParts,
    fuselage: FmParts,
    fin: FmParts,
    stab: FmParts,
    sweep_levels: Vec<SweepLevel>,
    moment_of_inertia: [f64; 3],
    max_allow_gload: [f64; 2],
}

impl FmData {

    // ------------------------------------------------------------------
    // getPartsFm / extractRpmFromThrottleAuto / getEngineLoad / initEngineLoad
    // ------------------------------------------------------------------

    /// 对应 Java `public void getPartsFm(String c, fm_parts p)`。
    pub(crate) fn get_parts_fm(src: &JsonSrc, c: &str, p: &mut FmParts) {
        p.name = Some(c.to_string());
        p.cd_min = src.get_f64(&format!("{c}.CdMin"));
        p.cl0 = src.get_f64(&format!("{c}.Cl0"));
        p.cl_crit_high = src.get_f64(&format!("{c}.ClCritHigh"));
        p.cl_crit_low = src.get_f64(&format!("{c}.ClCritLow"));

        p.cl_after_crit = src.get_f64(&format!("{c}.ClAfterCrit"));
        p.line_cl_coeff = src.get_f64(&format!("{c}.lineClCoeff"));

        p.aoa_crit_high = src.get_f64(&format!("{c}.alphaCritHigh"));
        p.aoa_crit_low = src.get_f64(&format!("{c}.alphaCritLow"));
    }

    /// 对应 Java `private void extractRpmFromThrottleAuto(String hdrString)`。
    /// 形参 hdrString 在 Java 方法体内未被引用 — `_` 前缀保真保留
    /// (get_aoa_low_v_wing 同款先例)。
    fn extract_rpm_from_throttle_auto(&mut self, src: &JsonSrc, _hdr_string: &str) {
        self.military_rpm = 0.0;
        self.wep_rpm = 0.0;

        // Try to find Propellor section within the engine type
        // PORT: Java `cut(data, ...)` — data 为 null 时 cut 处 NPE ↔ unwrap panic
        // (parse_named_opts_json 的 catch_unwind 收敛, §1)
        let mut prop_section = src.section("Propellor");
        if prop_section.is_null() {
            prop_section = src.section("Propeller");
        }

        if !prop_section.is_null() {
            for k in 0..20 {
                let key = format!("ThrottleRPMAuto{k}");
                let val = prop_section.get_in(&key);
                if val == "null" {
                    continue;
                }

                // Parse comma-separated throttle/RPM pairs
                let trimmed = val.trim();
                let parts: Vec<&str> = trimmed.split(',').collect();
                if parts.len() >= 2 {
                    // + NumberFormatException ignored
                    if let (Ok(throttle), Ok(rpm)) = (
                        parts[0].trim().parse::<f64>(),
                        parts[1].trim().parse::<f64>(),
                    ) {
                        if (throttle - 1.0).abs() < 0.01 {
                            self.military_rpm = rpm;
                            if self.wep_rpm <= 0.0 {
                                self.wep_rpm = rpm; // Default WEP = military (Java 注释)
                            }
                        } else if (throttle - 1.1).abs() < 0.01 {
                            self.wep_rpm = rpm;
                        }
                    }
                }
            }
        }

        // Fallback to maxRPM approximation if parsing failed
        if self.military_rpm <= 0.0 && self.wep_rpm <= 0.0 {
            self.wep_rpm = self.max_rpm;
            self.military_rpm = self.max_rpm;
        } else if self.military_rpm <= 0.0 {
            self.military_rpm = self.wep_rpm;
        } else if self.wep_rpm <= 0.0 {
            self.wep_rpm = self.military_rpm;
        }
    }

    /// 对应 Java `public boolean getEngineLoad(engineLoad[] eL, int loadIndex)`
    /// — 读一个 Load 档; WaterLimit/OilLimit 为 0 即该档缺席。
    fn get_engine_load(src: &JsonSrc, el: &mut [EngineLoad], load_index: usize) -> bool {
        let c = format!("Load{load_index}");
        el[load_index].water_limit = src.get_f64(&format!("{c}.WaterTemperature"));
        if el[load_index].water_limit == 0.0 {
            return false;
        }
        el[load_index].oil_limit = src.get_f64(&format!("{c}.OilTemperature"));
        if el[load_index].oil_limit == 0.0 {
            return false;
        }
        el[load_index].work_time = src.get_f64(&format!("{c}.WorkTime"));
        el[load_index].recover_time = src.get_f64(&format!("{c}.RecoverTime"));
        el[load_index].cur_water_work_time_mili = el[load_index].work_time * 1000.0;
        el[load_index].cur_oil_work_time_mili = el[load_index].work_time * 1000.0;
        true
    }

    /// 对应 Java `public void initEngineLoad()`。
    /// `Application.maxEngLoad` = 10 (Java 常量)。
    fn init_engine_load(&mut self, src: &JsonSrc) {
        const APP_MAX_ENG_LOAD: usize = 10; // Application.maxEngLoad
        self.avg_eng_recovery_rate = 0.0;
        let mut eng_load: Vec<EngineLoad> = vec![EngineLoad::default(); APP_MAX_ENG_LOAD];
        self.max_eng_load = 0;
        //       maxEngLoad++)); — 空体 do-while, 后缀自增在条件求值内 (无论成败
        //       都 +1), 循环继续 = getEngineLoad 返回值
        loop {
            let idx = self.max_eng_load as usize;
            // 防御加固 (Java 同款): 畸形 FM 的 Load 块数达数组容量即止, 防越界写
            if idx >= eng_load.len() {
                break;
            }
            let ok = Self::get_engine_load(src, &mut eng_load, idx);
            self.max_eng_load += 1;
            if !ok {
                break;
            }
        }
        // 检视反馈 (Java 同款): 档位数达容量退出时探测下一档, 存在即显式告警截断
        if self.max_eng_load as usize >= eng_load.len()
            && src.get_f64(&format!("Load{}.WaterTemperature", eng_load.len())) != 0.0
        {
            logger::warn(
                "FmData",
                &format!(
                    "发动机负载档位数超过数组容量 {}, Load{}+ 被截断 (如为真实机型请上调 Application.maxEngLoad), FM: {}",
                    eng_load.len(),
                    eng_load.len(),
                    self.read_file_name.clone().unwrap_or_default()
                ),
            );
        }
        self.max_eng_load -= 1;
        eng_load[self.max_eng_load as usize].water_limit = 999.0;
        eng_load[self.max_eng_load as usize].oil_limit = 999.0;

        // PORT(allow needless_range_loop): Java for(int i...) 直译 — i 仅作数组
        // 索引 + 日志参数, 保真保留计数形态
        #[allow(clippy::needless_range_loop)]
        for i in 0..self.max_eng_load as usize {
            if eng_load[i].recover_time != 0.0 {
                self.avg_eng_recovery_rate +=
                    eng_load[i].work_time / eng_load[i].recover_time;
            }
            logger::debug(
                "FmData",
                &format!(
                    "Load{} Water/Oil: [{}, {}] WEP/Rec: [{}, {}]",
                    i,
                    crate::base::format::format(eng_load[i].water_limit, 1),
                    crate::base::format::format(eng_load[i].oil_limit, 1),
                    crate::base::format::format(eng_load[i].work_time, 1),
                    crate::base::format::format(eng_load[i].recover_time, 1)
                ),
            );
        }
        // 防御加固 (Java 同款): 单档位除 0 产生 NaN / 零档位 -0.0 → 一并归 0
        if self.max_eng_load > 1 {
            self.avg_eng_recovery_rate /= (self.max_eng_load - 1) as f64;
        } else {
            self.avg_eng_recovery_rate = 0.0;
        }
        self.eng_load = Some(eng_load);
    }

    // ------------------------------------------------------------------
    // getload — FM 全量数据装载 (doLoad=true 的方法体)
    // ------------------------------------------------------------------

    /// 引擎计数循环 (getload_from 喷气/非喷气两分支逐字同款, 收敛于此):
    /// Engine1.. 逐个探测直到 "null"; 防御加固 (Java 同款): 引擎数上限 =
    /// State.maxEngNum (遥测数组容量, 解析上限=可消费上限), 病态文件 O(n²)
    /// 全串扫描防护, 超限截断显式告警不静默。
    fn count_engines(&self, src: &JsonSrc) -> i32 {
        let mut engine_num = 1;
        while src.get_str(&format!("Engine{}", engine_num)) != "null" {
            engine_num += 1;
            if engine_num >= MAX_ENG_NUM as i32 {
                // 检视反馈 (Java 同款): 超限截断显式告警, 不静默
                if src.get_str(&format!("Engine{}", engine_num)) != "null" {
                    logger::warn(
                        "FmData",
                        &format!(
                            "引擎数超过解析上限 {}, Engine{}+ 被截断 (如为真实机型请上调 State.maxEngNum), FM: {}",
                            MAX_ENG_NUM,
                            engine_num,
                            self.read_file_name.clone().unwrap_or_default()
                        ),
                    );
                }
                break;
            }
        }
        engine_num
    }

    /// 对应 Java `public void getload()` — 翼/引擎/增压器/推力表/
    /// vne/面积/重量族的全量装载 + fmdata 摘要串构造。
    ///
    /// PORT 纪律: 逐行直译, 语句顺序与 Java 一致 (含源码自身的重复段/死存储 —
    /// AFuselage 重复读两遍、Stab/KeelAngle 段误写 WingAngle 的 bug 均保真保留);
    /// 浮点字面量按 §2.12 (1.0f/1000.f 拓宽域, 精确值直书); `(int)` 强转按 §2.2。
    /// panic 语义 (§1): Java 由构造器 catch(Exception) 收敛 valid=false ↔ 本方法
    /// 的 panic 由 parse_named_opts_json 的 catch_unwind 收敛 Err (畸形输入防线)。
    /// 波14 拆解: 方法体按装载阶段提取为下方 load_* 子函数, 调用序即语句序。
    pub(crate) fn getload_from(&mut self, src: &JsonSrc) {
        let start_time = std::time::Instant::now(); // System.currentTimeMillis 计时面

        let hdr_string = self.load_engine_section(src);
        if self.is_jet {
            self.load_jet_thrust_tables(src, &hdr_string);
        } else {
            self.load_compressor(src, &hdr_string);
        }
        self.load_rpm_and_engine_load(src, &hdr_string);

        let (flaps_destruction, flaps_destruction_num) = self.load_areas_and_weights(src);
        let parts = self.load_lift_coeffs(src);

        let s = self.build_fmdata_summary(&parts, &flaps_destruction, flaps_destruction_num);

        // 部件实体落位 (Java: 构造过程中的 new fm_parts 赋值在此集中)
        self.no_flaps_wing = Some(parts.no_flaps_wing);
        self.full_flaps_wing = Some(parts.full_flaps_wing);
        self.sweep_levels = Some(parts.sweep_levels);
        self.fuselage = Some(parts.fuselage);
        self.fin = Some(parts.fin);
        self.stab = Some(parts.stab);

        self.fmdata = Some(s);

        let duration = start_time.elapsed().as_millis() as i64;
        logger::info(
            "FmData",
            &format!(
                "Parsed FM file '{}' in {} ms (Engine Count: {}, Jet: {})",
                self.read_file_name.clone().unwrap_or_default(),
                duration,
                self.engine_num,
                self.is_jet
            ),
        );
    }

    /// 引擎段: 喷气判定 + Engine 头前缀选择 + 引擎计数 + WEP 转速乘数清位
    /// (getload 开头)。返回 hdr_string (喷气/增压器段共用键前缀)。
    fn load_engine_section(&mut self, src: &JsonSrc) -> String {
        self.is_jet = false;

        // 读取推力高度
        let mut hdr_string = "EngineType0.".to_string();
        let res = src.get_str("EngineType0.Main.Type");
        if res.contains("Jet") {
            // 判断喷气
            self.is_jet = true;
            self.engine_num = self.count_engines(src);
        } else {
            if res == "null" {
                hdr_string = "Engine0.".to_string();
                if src.get_str("Engine0.Main.Type").contains("Jet") {
                    self.is_jet = true;
                }
            }
            // 遍历引擎数量（适用于所有非喷气引擎，包括活塞引擎）
            self.engine_num = self.count_engines(src);
        }
        self.engine_rpm_mult_wep = 1.0;
        hdr_string
    }

    /// 喷气引擎段: 推力高度/速度表 + 工作模式表 + AFT 推力表预计算
    /// (getload 的 is_jet 分支体)。
    // PORT(allow needless_range_loop): Java for(int i...) 直译 — i 进 format! 键名
    #[allow(clippy::needless_range_loop)]
    fn load_jet_thrust_tables(&mut self, src: &JsonSrc, hdr_string: &str) {
        self.aftb_coff = src.get_f64(&format!("{hdr_string}Main.AfterburnerBoost"));
        self.thr_max0 = src.get_f64("ThrustMax.ThrustMax0");

        self.alt_thr_num = 0;
        let mut altitude_thr = [0.0f64; 30];
        // Java for(init; cond; i++, altThrNum++) — update 在体后 (break 轮不增)
        for i in 0..30 {
            altitude_thr[i] = src.get_f64_exc(&format!("ThrustMax.Altitude_{i}"));
            if altitude_thr[i] == f32::MAX as f64 {
                altitude_thr[i] = 0.0;
                break;
            }
            self.alt_thr_num += 1;
        }
        self.altitude_thr = Some(altitude_thr);

        // 读取推力速度
        self.vel_thr_num = 0;
        let mut velocity_thr = [0.0f64; 30];
        for i in 0..30 {
            velocity_thr[i] = src.get_f64_exc(&format!("ThrustMax.Velocity_{i}"));
            if velocity_thr[i] == f32::MAX as f64 {
                velocity_thr[i] = 0.0;
                break;
            }
            self.vel_thr_num += 1;
        }
        self.velocity_thr = Some(velocity_thr);

        // 读取发动机工作模式
        self.mode_engine_num = 0;
        let mut mode_engine_mult = [0.0f64; 10];
        let mut mode_engine_rpm_mult = [0.0f64; 10];
        for i in 0..10 {
            mode_engine_mult[i] = src.get_f64_exc(&format!("Main.Mode{i}.ThrustMult"));
            mode_engine_rpm_mult[i] = src.get_f64_exc(&format!("Main.Mode{i}.RPM"));
            if mode_engine_mult[i] == f32::MAX as f64 {
                mode_engine_mult[i] = 0.0;
                mode_engine_rpm_mult[i] = 1.0;
                break;
            }
            self.mode_engine_num += 1;
        }

        let mut engine_mult_wep = 1.0f64;
        if self.mode_engine_num != 0 {
            engine_mult_wep = mode_engine_mult[self.mode_engine_num as usize - 1];
            self.engine_rpm_mult_wep =
                mode_engine_rpm_mult[self.mode_engine_num as usize - 1];
        }

        // 预计算 AFT 推力表 (曲线窗口 + 峰值推力; MIL 表 Rust 无消费方, 已随
        // 2026-09 死代码清理删除 — peak MIL 仅装载日志曾用)
        let alt_n = self.alt_thr_num as usize;
        let vel_n = self.vel_thr_num as usize;
        let mut max_thr_aft: Vec<Vec<f64>> = vec![vec![0.0; vel_n]; alt_n];
        let mut max_thr_aft_coff: Vec<Vec<f64>> = vec![vec![0.0; vel_n]; alt_n];
        for i in 0..alt_n {
            for j in 0..vel_n {
                let thr_coff = src.get_f64(&format!("ThrustMax.ThrustMaxCoeff_{i}_{j}"));
                max_thr_aft_coff[i][j] =
                    src.get_f64(&format!("ThrustMax.ThrAftMaxCoeff_{i}_{j}"));
                if max_thr_aft_coff[i][j] == 0.0 {
                    max_thr_aft_coff[i][j] = 1.0;
                }
                max_thr_aft[i][j] =
                    self.thr_max0 * thr_coff * self.aftb_coff
                        * max_thr_aft_coff[i][j]
                        * engine_mult_wep
                        * self.engine_num as f64;
            }
        }
        // 预计算峰值推力
        self.peak_thr_aft = self.calculate_peak_thrust(Some(&max_thr_aft));
        self.max_thr_aft = Some(max_thr_aft);
        self.max_thr_aft_coff = Some(max_thr_aft_coff);

        logger::info(
            "FmData",
            &format!(
                "Jet Engine Thrust Table loaded ({}x{}), peak AFT={} kgf",
                self.alt_thr_num,
                self.vel_thr_num,
                crate::base::format::format(self.peak_thr_aft, 0)
            ),
        );
    }

    /// 增压器段 (radial inline): 9 组增压器数组 + WAPC 扩展参数 + WEP/功率参数
    /// (getload 的非喷气分支体)。
    // PORT(allow needless_range_loop): Java for(int i...) 直译 — i 进 format! 键名
    #[allow(clippy::needless_range_loop)]
    fn load_compressor(&mut self, src: &JsonSrc, hdr_string: &str) {
        self.aftb_coff = src.get_f64(&format!("{hdr_string}Main.AfterburnerBoost"));
        self.comp_num_steps = src.get_f64("Compressor.NumSteps") as i32;
        self.speed_to_manifold_multiplier =
            src.get_f64("Compressor.SpeedManifoldMultiplier");

        // NegativeArraySizeException → 构造器 catch; as usize 巨量 → Vec
        // 分配 panic 同被 parse_named_opts_json 收敛 (CORRUPT 同语义)
        let n = self.comp_num_steps as usize;
        let mut alt = vec![0.0f64; n];
        let mut power = vec![0.0f64; n];
        let mut boost = vec![0.0f64; n];
        let mut rpm_ratio = vec![0.0f64; n];
        let mut ceil = vec![0.0f64; n];
        let mut ceil_pwr = vec![0.0f64; n];
        let mut has_boost = vec![false; n];
        let mut const_rpm_alt = vec![0.0f64; n];
        let mut const_rpm_power = vec![0.0f64; n];
        for i in 0..n {
            alt[i] = src.get_f64(&format!("Compressor.Altitude{i}"));
            power[i] = src.get_f64(&format!("Compressor.Power{i}"));
            boost[i] = src.get_f64(&format!("Compressor.AfterburnerBoostMul{i}"));
            has_boost[i] =
                src.get_str(&format!("Compressor.AfterburnerBoostMul{i}")) != "null";
            rpm_ratio[i] =
                src.get_f64(&format!("Compressor.PowerConstRPMCurvature{i}"));
            ceil[i] = src.get_f64(&format!("Compressor.Ceiling{i}"));
            ceil_pwr[i] = src.get_f64(&format!("Compressor.PowerAtCeiling{i}"));
            const_rpm_alt[i] =
                src.get_f64(&format!("Compressor.AltitudeConstRPM{i}"));
            const_rpm_power[i] =
                src.get_f64(&format!("Compressor.PowerConstRPM{i}"));
        }
        // 9 组表同批收拢 (波17 F5)
        self.compressor = Some(CompressorData {
            alt,
            power,
            boost,
            rpm_ratio,
            ceil,
            ceil_pwr,
            has_boost: Some(has_boost),
            const_rpm_alt: Some(const_rpm_alt),
            const_rpm_power: Some(const_rpm_power),
        });

        // === Extended WAPC-compatible parameters ===
        self.comp_pressure_at_rpm0 =
            src.get_f64("Compressor.CompressorPressureAtRPM0");
        self.comp_omega_factor_sq =
            src.get_f64("Compressor.CompressorOmegaFactorSq");
        self.has_comp_omega_factor_sq =
            src.get_str("Compressor.CompressorOmegaFactorSq") != "null";

        // ExactAltitudes: explicitly defined in FM file
        let ea_str = src.get_str("Compressor.ExactAltitudes");
        if ea_str != "null" {
            self.explicit_exact_altitudes = Some(ea_str.trim() == "true");
        }

        // Per-stage manifold pressure and afterburner pressure boost
        let mut comp_ata = vec![0.0f64; n];
        let mut comp_afterburner_pressure_boost = vec![0.0f64; n];
        for i in 0..n {
            comp_ata[i] = src.get_f64(&format!("Compressor.ATA{i}"));
            comp_afterburner_pressure_boost[i] =
                src.get_f64(&format!("Compressor.AfterburnerPressureBoost{i}"));
        }
        self.comp_ata = Some(comp_ata);
        self.comp_afterburner_pressure_boost = Some(comp_afterburner_pressure_boost);

        // Iterate all ATA entries (ATA0..ATA9) and take the maximum
        self.military_mp = 0.0;
        for i in 0..10 {
            let ata = src.get_f64(&format!("Compressor.ATA{i}"));
            if ata > self.military_mp {
                self.military_mp = ata;
            }
        }

        // WEP parameters from Main section
        self.throttle_boost = src.get_f64(&format!("{hdr_string}Main.ThrottleBoost"));
        if self.throttle_boost <= 0.0 {
            self.throttle_boost = 1.0;
        }

        self.octane_afterburner_mult =
            src.get_f64(&format!("{hdr_string}Main.OctaneAfterburnerMult"));
        if self.octane_afterburner_mult <= 0.0 {
            self.octane_afterburner_mult = 1.0;
        }

        // WEP manifold pressure (ata)
        self.wep_manifold_pressure = src.get_f64("AfterburnerManifoldPressure");

        // Sea level power from Main.Power
        self.deck_power = src.get_f64(&format!("{hdr_string}Main.Power"));

        // RPM parameters for determineDefaultRpm (BUG 2 fix)
        self.shaft_rpm_max = src.get_f64(&format!("{hdr_string}Main.ShaftRPMMax"));
        self.rpm_nom = src.get_f64(&format!("{hdr_string}Main.RPMNom"));

        // GovernorMaxParam is in the Propeller/Propellor section
        self.governor_max_param = 0.0;
        let mut prop_section_for_gov = src.section("Propellor");
        if prop_section_for_gov.is_null() {
            prop_section_for_gov = src.section("Propeller");
        }
        if !prop_section_for_gov.is_null() {
            let gov_str = prop_section_for_gov.get_in("GovernorMaxParam");
            // null 返回, 前半恒真 — 直译保留判 "null")
            if gov_str != "null" {
                // (f64 域) + NumberFormatException ignored
                if let Some(first) = gov_str.trim().split(',').next() {
                    if let Ok(v) = first.trim().parse::<f64>() {
                        self.governor_max_param = v;
                    }
                }
            }
        }
    }

    /// 转速与引擎负载段: 最大转速 (WEP 乘数修正) + military/WEP RPM 提取 +
    /// 版本号 + 耐久负载档 (getload)。
    fn load_rpm_and_engine_load(&mut self, src: &JsonSrc, hdr_string: &str) {
        // 读取最大转速和最大允许转速 (must be before extractRpmFromThrottleAuto)
        //
        self.max_rpm = src.get_f64("RPMAfterburner");
        let max_rpm_normal = src.get_f64(" RPMMax");
        if self.max_rpm < max_rpm_normal {
            self.max_rpm = max_rpm_normal;
        }

        // 针对幻影2000C mode6 rpm乘数1.01的修复
        self.max_rpm *= self.engine_rpm_mult_wep;

        // Extract military/WEP RPM after maxRPM is available as fallback
        if !self.is_jet && self.comp_num_steps > 0 {
            self.extract_rpm_from_throttle_auto(src, hdr_string);
        }

        self.version = self.get_version();
        self.init_engine_load(src);
    }

    /// 重量/阻力/襟翼限速/面积段: 重量族 + vne/舵面效率 + 襟翼损毁限速表 +
    /// 面积三级回退族 (getload)。
    /// 返回 (襟翼损毁表, 档位数) — fmdata 摘要段按原局部变量复用。
    fn load_areas_and_weights(&mut self, src: &JsonSrc) -> ([[f64; 2]; 6], usize) {
        self.emptyweight = src.get_f64("EmptyMass");
        self.vne = src.get_f64("Vne:");
        if self.vne == 0.0 {
            self.vne = src.get_f64("WingPlane.Strength.VNE");
            if self.vne == 0.0 {
                self.vne = src.get_f64("WingPlaneSweep0.Strength.VNE");
            }
        }

        self.vne_mach = src.get_f64("VneMach");
        if self.vne_mach == 0.0 {
            self.vne_mach = src.get_f64("WingPlane.Strength.MNE");
            if self.vne_mach == 0.0 {
                self.vne_mach = src.get_f64("WingPlaneSweep0.Strength.MNE");
            }
        }

        self.aileron_eff = src.get_f64("AileronEffectiveSpeed");
        self.aileron_power_loss = src.get_f64("AileronPowerLoss");
        self.rudder_eff = src.get_f64("RudderEffectiveSpeed");
        self.rudder_power_loss = src.get_f64("RudderPowerLoss");
        self.elav_eff = src.get_f64("ElevatorsEffectiveSpeed");
        self.elav_power_loss = src.get_f64("ElevatorPowerLoss");
        self.maxfuelweight = src.get_f64("MaxFuelMass0");

        self.nitro_decr = src.get_f64("NitroConsumption");
        self.nitro = src.get_f64("MaxNitro");
        self.oil = src.get_f64("OilMass");

        self.grossweight = self.emptyweight + self.maxfuelweight + self.nitro + self.oil;
        self.halfweight = self.emptyweight + self.maxfuelweight / 2.0 + self.nitro + self.oil;
        self.nofuelweight = self.emptyweight + self.nitro + self.oil;

        self.radiator_cd = src.get_f64("RadiatorCd");
        self.oil_radiator_cd = src.get_f64("OilRadiatorCd");
        self.oswalds_efficiency_number = src.get_f64("OswaldsEfficiencyNumber");

        self.swept_wing_angle = src.get_f64("SweptWingAngle");
        if self.swept_wing_angle == 0.0 {
            self.swept_wing_angle = src.get_f64("WingPlane.SweptAngle");
            if self.swept_wing_angle == 0.0 {
                self.swept_wing_angle = src.get_f64("WingPlaneSweep0.SweptAngle");
            }
        }

        self.critical_speed = src.get_f64("CriticalSpeed");

        // +1 留给 1.25x 襟翼档位插值哨兵行，避免5档襟翼飞机(如F-82E/P-51B/P-51A-36)
        // 数组越界
        let mut flaps_destruction = [[0.0f64; 2]; 6];
        let mut flaps_destruction_num: usize = 0;
        {
            let mut p = 0;
            while p < 5 {
                // 在实参求值内; 键缺席时行保持 0 → [1]==0 → continue (档位不进位)
                let key = format!("FlapsDestructionIndSpeedP{p}");
                p += 1;
                let _ = src.get_f64s(&key, &mut flaps_destruction[flaps_destruction_num], 2);
                if flaps_destruction[flaps_destruction_num][1] == 0.0 {
                    continue;
                }
                flaps_destruction_num += 1;
            }
        }
        if flaps_destruction_num == 0 {
            let mut tmp = [0.0f64; 4];
            let _ = src.get_f64s("FlapsDestructionIndSpeedP", &mut tmp, 4);
            flaps_destruction[0][0] = tmp[0];
            flaps_destruction[0][1] = tmp[1];
            flaps_destruction[1][0] = tmp[2];
            flaps_destruction[1][1] = tmp[3];
            flaps_destruction_num = 2;
        }
        if flaps_destruction_num == 0 {
            flaps_destruction[0][0] = 1.0;
            flaps_destruction[0][1] = src.get_f64("FlapsDestructionIndSpeed");
        }
        // 125襟翼档位插值，辅助运算
        flaps_destruction[flaps_destruction_num][0] = 1.25;
        flaps_destruction[flaps_destruction_num][1] = 0.0;
        self.flaps_destruction_ind_speed = Some(flaps_destruction);
        self.flaps_destruction_num = flaps_destruction_num as i32;

        self.gear_destruction_ind_speed = src.get_f64("GearDestructionIndSpeed");

        // 面积 — 三级回退族: 顶层键 → WingPlane.* → WingPlaneSweep0.*
        // PORT: 宏观直译 (Java 每段 3 行 if, 逐字段展开)
        let fallback3 = |top: &str, plane: &str, sweep0: &str| -> f64 {
            let v = src.get_f64(top);
            if v != 0.0 {
                return v;
            }
            let v = src.get_f64(plane);
            if v != 0.0 {
                return v;
            }
            src.get_f64(sweep0)
        };
        self.a_wing_left_in =
            fallback3("Areas.WingLeftIn", "WingPlane.Areas.LeftIn", "WingPlaneSweep0.Areas.LeftIn");
        self.a_wing_left_mid = fallback3(

            "Areas.WingLeftMid",
            "WingPlane.Areas.LeftMid",
            "WingPlaneSweep0.Areas.LeftMid",
        );
        self.a_wing_left_out = fallback3(

            "Areas.WingLeftOut",
            "WingPlane.Areas.LeftOut",
            "WingPlaneSweep0.Areas.LeftOut",
        );
        self.a_wing_left_cut = fallback3(

            "Areas.WingLeftCut",
            "WingPlane.Areas.LeftCut",
            "WingPlaneSweep0.Areas.LeftCut",
        );
        self.a_wing_right_in = fallback3(

            "Areas.WingRightIn",
            "WingPlane.Areas.RightIn",
            "WingPlaneSweep0.Areas.RightIn",
        );
        self.a_wing_right_mid = fallback3(

            "Areas.WingRightMid",
            "WingPlane.Areas.RightMid",
            "WingPlaneSweep0.Areas.RightMid",
        );
        self.a_wing_right_out = fallback3(

            "Areas.WingRightOut",
            "WingPlane.Areas.RightOut",
            "WingPlaneSweep0.Areas.RightOut",
        );
        self.a_wing_right_cut = fallback3(

            "Areas.WingRightCut",
            "WingPlane.Areas.RightCut",
            "WingPlaneSweep0.Areas.RightCut",
        );
        self.a_aileron = fallback3(

            "Areas.Aileron",
            "WingPlane.Areas.Aileron",
            "WingPlaneSweep0.Areas.Aileron",
        );
        self.a_fuselage = fallback3(
            "Areas.Fuselage",
            "FuselagePlane.Areas.Main",
            "WingPlaneSweep0.Areas.Main",
        );
        // Java 源码将 AFuselage 三级回退段**原样重复了两遍** — 第二遍
        // 读到相同值, 净效果为同一赋值; 保真保留重复调用
        self.a_fuselage = fallback3(

            "Areas.Fuselage",
            "FuselagePlane.Areas.Main",
            "WingPlaneSweep0.Areas.Main",
        );

        (flaps_destruction, flaps_destruction_num)
    }

    /// 升力系数段: FmParts 部件族 (机翼/机身/垂尾/平尾/变后掠翼) + 安装角补偿 +
    /// 升力面积因子/翼载/展弦比/诱导阻力 + 过载限制原值 (getload)。
    // PORT(allow needless_range_loop): Java for(int i...) 直译 — i 进 format! 键名
    #[allow(clippy::needless_range_loop)]
    fn load_lift_coeffs(&mut self, src: &JsonSrc) -> LiftLoad {
        let mut no_flaps_wing = FmParts::default();
        Self::get_parts_fm(src, "NoFlaps", &mut no_flaps_wing);
        if no_flaps_wing.aoa_crit_high == 0.0 {
            Self::get_parts_fm(src, "FlapsPolar0", &mut no_flaps_wing);
        }

        let mut full_flaps_wing = FmParts::default();
        Self::get_parts_fm(src, "FullFlaps", &mut full_flaps_wing);
        if full_flaps_wing.aoa_crit_high == 0.0 {
            Self::get_parts_fm(src, "FlapsPolar1", &mut full_flaps_wing);
        }

        // 可变翼: 动态检测 WingPlaneSweep 数量
        let mut sweep_levels: Vec<SweepLevel> = Vec::new();
        for i in 0..10 {
            let prefix = format!("WingPlaneSweep{i}");
            let block = src.section(&prefix);
            if block.is_null() {
                break;
            }

            let mut level = SweepLevel::default();
            level.sweep = src.get_f64(&format!("{prefix}.Sweep:r"));
            level.vne = src.get_f64(&format!("{prefix}.Strength.VNE"));
            level.vne_mach = src.get_f64(&format!("{prefix}.Strength.MNE"));

            let mut no_flaps = FmParts::default();
            Self::get_parts_fm(src, &format!("{prefix}.NoFlaps"), &mut no_flaps);
            if no_flaps.aoa_crit_high == 0.0 {
                Self::get_parts_fm(src, &format!("{prefix}.FlapsPolar0"), &mut no_flaps);
            }
            level.no_flaps = Some(no_flaps);

            let mut full_flaps = FmParts::default();
            Self::get_parts_fm(src, &format!("{prefix}.FullFlaps"), &mut full_flaps);
            if full_flaps.aoa_crit_high == 0.0 {
                Self::get_parts_fm(src, &format!("{prefix}.FlapsPolar1"), &mut full_flaps);
            }
            level.full_flaps = Some(full_flaps);

            sweep_levels.push(level);
        }
        self.is_v_wing = Some(sweep_levels.len() > 1);

        // 向后兼容: 拼摘要串用的 v50/v100 快照 (Java 存字段的引用共享, Rust 无
        // 字段消费方 — 2026-09 死代码清理后仅局部变量; full_flaps 版 Java 亦死)
        let mut no_flaps_wing_v50 = FmParts::default();
        let mut no_flaps_wing_v100 = FmParts::default();
        if sweep_levels.len() >= 2 {
            no_flaps_wing_v50 = sweep_levels[1].no_flaps.clone().unwrap_or_default();
        }
        if sweep_levels.len() >= 3 {
            let last = sweep_levels.len() - 1;
            no_flaps_wing_v100 = sweep_levels[last].no_flaps.clone().unwrap_or_default();
        }

        let mut fuselage = FmParts::default();
        Self::get_parts_fm(src, "Fuselage", &mut fuselage);
        if fuselage.aoa_crit_high == 0.0 {
            Self::get_parts_fm(src, "FuselagePlane.Polar", &mut fuselage);
        }

        let mut fin = FmParts::default();
        Self::get_parts_fm(src, "Fin", &mut fin);
        if fin.aoa_crit_high == 0.0 {
            Self::get_parts_fm(src, "HorStabPlane.Polar", &mut fin);
        }

        let mut stab = FmParts::default();
        Self::get_parts_fm(src, "Stab", &mut stab);
        if stab.aoa_crit_high == 0.0 {
            Self::get_parts_fm(src, "VerStabPlane.Polar", &mut stab);
        }

        // 获得安装角
        self.wing_angle = src.get_f64("\nWingAngle");
        if self.wing_angle == 0.0 {
            self.wing_angle = src.get_f64("WingPlane. Angle");
            if self.wing_angle == 0.0 {
                self.wing_angle = src.get_f64("WingPlaneSweep0. Angle");
            }
        }

        self.stab_angle = src.get_f64("StabAngle");
        // PORT(Java bug 保真): 本行判据是 WingAngle 而非 StabAngle — VerStabPlane 的
        // 角度会错写进 WingAngle, StabAngle 拿不到回退值; 源码如此, 不修 (§6 上报)
        if self.wing_angle == 0.0 {
            self.wing_angle = src.get_f64("VerStabPlane.Angle");
        }

        self.keel_angle = src.get_f64("KeelAngle");
        // PORT(Java bug 保真): 同上 — 判据 WingAngle 而非 KeelAngle
        if self.wing_angle == 0.0 {
            self.wing_angle = src.get_f64("FuselagePlane.Angle");
        }

        // 计算安装角补偿
        no_flaps_wing.aoa_crit_high -= self.wing_angle;
        no_flaps_wing.aoa_crit_low -= self.wing_angle;
        full_flaps_wing.aoa_crit_high -= self.wing_angle;
        full_flaps_wing.aoa_crit_low -= self.wing_angle;

        fuselage.aoa_crit_high -= self.keel_angle;
        fuselage.aoa_crit_low -= self.keel_angle;

        stab.aoa_crit_high -= self.stab_angle;
        stab.aoa_crit_low -= self.stab_angle;

        let mut moment_of_inertia = [0.0f64; 3];
        let _ = src.get_f64s("MomentOfInertia", &mut moment_of_inertia, 3);
        self.moment_of_inertia = Some(moment_of_inertia);

        // 最大升力面积因子载荷计算(气动升力系数x部件面积除以满油重量）
        // 最大攻角转弯时机身是失速的
        self.fuse_cl_high = fuselage.cl_crit_high * fuselage.line_cl_coeff;
        if fuselage.aoa_crit_high < no_flaps_wing.aoa_crit_high {
            self.fuse_cl_high = fuselage.cl_after_crit * fuselage.line_cl_coeff;
        }

        self.a_wing = self.a_wing_left_in
            + self.a_wing_right_in
            + self.a_wing_left_mid
            + self.a_wing_right_mid
            + self.a_wing_left_out
            + self.a_wing_left_cut
            + self.a_wing_right_out
            + self.a_wing_right_cut
            + self.a_aileron;

        no_flaps_wing.sq = self.a_wing;
        full_flaps_wing.sq = self.a_wing;
        fuselage.sq = self.a_fuselage;

        // NoFlapsWing.AoACritHigh 可能不等于 Fuselage.AoACritHigh
        self.no_flap_wll = self.a_wing * no_flaps_wing.cl_crit_high
            + self.a_fuselage * self.fuse_cl_high
                * (no_flaps_wing.aoa_crit_high / fuselage.aoa_crit_high);
        // 这里用空重; Java: / (emptyweight / 1000.f) — 1000.f 精确
        self.no_flap_wll /= self.emptyweight / 1000.0;

        self.fuse_cl_high = fuselage.cl_crit_high * fuselage.line_cl_coeff;
        if fuselage.aoa_crit_high < full_flaps_wing.aoa_crit_high {
            self.fuse_cl_high = fuselage.cl_after_crit * fuselage.line_cl_coeff;
        }

        // PORT(Java 保真): 分母里是 NoFlapsWing.AoACritHigh (非 FullFlaps) — 源码如此
        self.full_flap_wll = self.a_wing * full_flaps_wing.cl_crit_high
            + self.a_fuselage * self.fuse_cl_high
                * (no_flaps_wing.aoa_crit_high / fuselage.aoa_crit_high);
        self.full_flap_wll /= self.emptyweight / 1000.0;
        // 阻力面积因子计算
        self.cd_s = self.a_wing * no_flaps_wing.cd_min + self.a_fuselage * fuselage.cd_min;

        // 翼展
        self.wingspan = src.get_f64("Wingspan");
        if self.wingspan == 0.0 {
            self.wingspan = src.get_f64("WingPlane.Span");
            if self.wingspan == 0.0 {
                self.wingspan = src.get_f64("WingPlaneSweep0.Span");
            }
        }

        self.aspect_ratio = self.wingspan * self.wingspan / self.a_wing;

        // 诱导阻力还要
        self.ind_cd_f = 1.0 / (std::f64::consts::PI * self.aspect_ratio * self.oswalds_efficiency_number);

        let mut max_allow_gload = [0.0f64; 2];
        let _ = src.get_f64s("WingCritOverload", &mut max_allow_gload, 2);
        if max_allow_gload[0] == 0.0 {
            let _ = src.get_f64s("Strength.CritOverload", &mut max_allow_gload, 2);
        }

        // Save raw values for dynamic G-load calculation before conversion
        self.raw_wing_crit_overload = Some(max_allow_gload);

        LiftLoad {
            no_flaps_wing,
            full_flaps_wing,
            no_flaps_wing_v50,
            no_flaps_wing_v100,
            fuselage,
            fin,
            stab,
            sweep_levels,
            moment_of_inertia,
            max_allow_gload,
        }
    }

    /// fmdata 摘要串构造 (getload 的 String.format 族)。
    /// Lang 依赖隔离在本段; 过载限制在中途换算 1.2 倍余量后写入 self.max_allow_gload
    /// (原语句位置保真)。返回摘要串, 部件落位由编排层执行。
    // PORT(allow needless_range_loop): Java for(int i...) 直译 — i 进 format! 实参
    #[allow(clippy::needless_range_loop)]
    fn build_fmdata_summary(
        &mut self,
        parts: &LiftLoad,
        flaps_destruction: &[[f64; 2]; 6],
        flaps_destruction_num: usize,
    ) -> String {
        let mut max_allow_gload = parts.max_allow_gload;
        let moment_of_inertia = parts.moment_of_inertia;
        let no_flaps_wing = &parts.no_flaps_wing;
        let full_flaps_wing = &parts.full_flaps_wing;

        let lang = Lang::init_lang();
        let mut s = java_format(
            lang.b_fm_version,
            &[
                FmtArg::S(self.read_file_name.clone().unwrap_or_default()),
                FmtArg::S(self.version.clone().unwrap_or_default()),
            ],
        );
        s.push_str(&java_format(
            lang.b_weight,
            &[
                FmtArg::F(self.emptyweight, 1),
                FmtArg::F(self.maxfuelweight, 1),
            ],
        ));
        s.push_str(&java_format(
            lang.b_crit_speed,
            &[
                FmtArg::F(self.critical_speed * 3.6, 0),
                FmtArg::F(self.vne, 0),
            ],
        ));
        s.push_str(&java_format(
            lang.b_allow_load_factor,
            &[
                FmtArg::F(1.2 * (2.0 * max_allow_gload[0] / (g * self.grossweight) + 1.0), 1),
                FmtArg::F(1.2 * (2.0 * max_allow_gload[1] / (g * self.grossweight) - 1.0), 1),
                FmtArg::F(1.2 * (2.0 * max_allow_gload[0] / (g * self.halfweight) + 1.0), 1),
                FmtArg::F(1.2 * (2.0 * max_allow_gload[1] / (g * self.halfweight) - 1.0), 1),
            ],
        ));

        for i in 0..flaps_destruction_num {
            s.push_str(&java_format(
                lang.b_flap_restrict,
                &[
                    FmtArg::D(i as i32),
                    FmtArg::F(flaps_destruction[i][0] * 100.0, 0),
                    FmtArg::F(flaps_destruction[i][1], 0),
                ],
            ));
        }
        s.push_str(&java_format(
            lang.b_eff_speed_and_power_loss,
            &[
                FmtArg::F(self.elav_eff, 0),
                FmtArg::F(self.aileron_eff, 0),
                FmtArg::F(self.rudder_eff, 0),
                FmtArg::F(self.elav_power_loss, 0),
                FmtArg::F(self.aileron_power_loss, 0),
                FmtArg::F(self.rudder_power_loss, 0),
            ],
        ));

        if self.nitro != 0.0 {
            s.push_str(&java_format(
                lang.b_nitro,
                &[
                    FmtArg::F(self.nitro, 1),
                    FmtArg::F(self.nitro / (self.nitro_decr * 60.0), 1),
                ],
            ));
        }

        s.push_str(&java_format(
            lang.b_average_heat_recovery,
            &[FmtArg::F(self.avg_eng_recovery_rate, 1)],
        ));

        s.push_str(&java_format(
            lang.b_max_lift_load350,
            &[
                FmtArg::F((self.no_flap_wll + 1.0) / 2.0, 1),
                FmtArg::F((self.full_flap_wll + 1.0) / 2.0, 1),
            ],
        ));

        // 战雷在过载超限到真正断留了20%的余量
        max_allow_gload[0] = 1.2 * (2.0 * max_allow_gload[0] / (g * self.grossweight) + 1.0);
        max_allow_gload[1] = 1.2 * (2.0 * max_allow_gload[1] / (g * self.grossweight) - 1.0);
        self.max_allow_gload = Some(max_allow_gload);

        // 三轴转动惯量的值的顺序和三舵的要保持一致, 即pitch, roll, yaw
        s.push_str(&java_format(
            lang.b_inertia,
            &[
                FmtArg::F(moment_of_inertia[2], 0),
                FmtArg::F(moment_of_inertia[0], 0),
                FmtArg::F(moment_of_inertia[1], 0),
            ],
        ));

        s.push_str(&java_format(
            lang.b_lift,
            &[
                FmtArg::F(self.a_wing, 1),
                FmtArg::F(self.a_fuselage, 1),
                FmtArg::F(self.no_flap_wll, 2),
                FmtArg::F(self.full_flap_wll, 2),
                FmtArg::F(self.oswalds_efficiency_number, 2),
                FmtArg::F(self.aspect_ratio, 2),
                FmtArg::F(self.swept_wing_angle, 0),
            ],
        ));

        s.push_str(&java_format(
            lang.b_drag,
            &[
                FmtArg::F(self.cd_s, 2),
                FmtArg::F(self.cd_s / (self.halfweight / 1000.0), 2),
                FmtArg::F(self.ind_cd_f, 3),
                FmtArg::F(self.halfweight * self.ind_cd_f, 0),
                FmtArg::F(self.radiator_cd, 0),
                FmtArg::F(self.oil_radiator_cd, 0),
            ],
        ));

        s = Self::write_parts_fm(s, no_flaps_wing, &lang);
        if parts.no_flaps_wing_v50.cl_crit_high != 0.0 {
            s = Self::write_parts_fm(s, &parts.no_flaps_wing_v50, &lang);
        }
        if parts.no_flaps_wing_v100.cl_crit_high != 0.0 {
            s = Self::write_parts_fm(s, &parts.no_flaps_wing_v100, &lang);
        }
        s = Self::write_parts_fm(s, full_flaps_wing, &lang);
        s = Self::write_parts_fm(s, &parts.fuselage, &lang);
        s = Self::write_parts_fm(s, &parts.fin, &lang);
        s = Self::write_parts_fm(s, &parts.stab, &lang);

        s
    }

    /// 对应 Java `public String WritePartsFm(String s, fm_parts p)`。
    /// Lang 形参: Java 读静态字段 → 快照传入 (blkx crate 先例)。
    fn write_parts_fm(s: String, p: &FmParts, lang: &Lang) -> String {
        let mut s = s;
        s.push_str(&java_format(
            lang.b_fm_parts,
            &[FmtArg::S(p.name.clone().unwrap_or_default())],
        ));
        s.push_str(&java_format(lang.b_cd_min, &[FmtArg::F(p.cd_min, 3)]));
        s.push_str(&java_format(lang.b_cl0, &[FmtArg::F(p.cl0, 3)]));
        s.push_str(&java_format(
            lang.b_ao_a_crit,
            &[FmtArg::F(p.aoa_crit_low, 1), FmtArg::F(p.aoa_crit_high, 1)],
        ));
        s.push_str(&java_format(
            lang.b_ao_a_crit_cl,
            &[
                FmtArg::F(p.cl_crit_low, 2),
                FmtArg::F(p.cl_crit_high, 2),
            ],
        ));
        s
    }
}

#[cfg(test)]
mod tests;
