//! 对应 Java: `src/parser/Blkx.java` L1665-1906 (D4 拆分: reader.rs)。
//! 覆盖构造器解析逻辑与原始文本抽取原语:
//! - 两个构造器 (L1665-1726) → [`Blkx::parse`]/[`Blkx::parse_str`] 纯函数
//!   (Java 构造器以 `valid=false` 表达失败、不外抛; Result 化后 Err 携带等价诊断,
//!   详见函数级注)
//! - `cut` (L1728-1762, 包私有) → 本模块私有自由函数
//! - `getArray`/`getlastone`/`getoneinData`/`getone` (L1764-1906) → `impl Blkx` 方法
//!
//! PORT (D4 裁决, 反射段 L1908-2000 **不迁移**):
//! - `getValue` (L1914): 反射按点路径取字段 — 唯一下游 FormulaEvaluator 归 C 类;
//! - `dumpVariables`/`dumpObject` (L1935/L1945): 反射 dump 调试工具;
//! - `getVariableMap` (L1986): 反射扁平化字段供公式引擎 — 同样只服务 FormulaEvaluator,
//!   FMPowerExtractor 主消费者直读字段不经反射。
//!
//! PORT (方法波次边界, 见 mod.rs): 构造器 `doLoad=true` 分支 (L1708-1718) 已落地 —
//! [`Blkx::parse`]/[`Blkx::parse_named`] (Java 两参构造器) 走 getload 全量装载 +
//! catch_unwind 收敛 (panic → Err ↔ Java valid=false); [`Blx::parse_named_opts`]
//! (三参构造器) 显式 doLoad, FMLoader 中央文件传 false 只读。
//! getAllplotdata 批次已落地: transUnit/getAllplotdata/getplotdata (L1590-1658,
//! fm_loader 接线 + fuzz 腿1 管线恢复, 真机/合成英制 oracle 位级对拍)。
//!
//! PORT (§2.1): Java charAt/indexOf/substring 按 UTF-16 码元计数, 此处一律字节偏移
//! + `as_bytes()` 索引 — 域内 (FM/中央文件) 为纯 ASCII (真机三文件 od/grep 实测),
//!   字节索引与码元索引一致; 定界符 '{'/'}'/'='/'\n' 均为 ASCII, UTF-8 自同步,
//!   逐字节比较不会误判多字节字符 (string_helper.rs / types.rs cut_static 先例)。
//!   `toUpperCase()` 的索引漂移防护 (ß→SS 等病态大写变长输入) 与 Java 的守卫
//!   逐条对应; 终点取子串经 `get()` 边界守卫, 病态漂移按"未找到"哨兵收敛
//!   (types.rs cut_static 同款裁决: Rust UTF-8 字节域 ß 2→2 不漂移, 反而更对齐)。

use super::types::{EngineLoad, FmParts, SweepLevel, XY};
use super::Blkx;
use crate::g;
use crate::lang::Lang;
use crate::logger;
use crate::parser::state::MAX_ENG_NUM;

/// Java `String.trim()`: 剥首尾所有 `<= U+0020` 的字符 (含 \n/\r/\t 与 C0 控制符,
/// **不含** NBSP U+00A0)。
/// PORT: Rust `str::trim` 剥 Unicode White_Space (NBSP 会被剥掉) — 构造器的
/// 空文件/JSON 判定必须用 Java 语义 (oracle ctor_n7: 内容 "\u{A0}{x\n" 的文件
/// Java trim 后非空且不以 '{' 开头 → valid=true)。
fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c as u32) <= 0x20)
}

/// [`java_format`] 的实参 (getload fmdata 串构造专用)。
enum FmtArg {
    /// Java `%s` (String.toString 形态)
    S(String),
    /// Java `%d` (int)
    D(i32),
    /// Java `%.Mf` (无宽度域; M 位小数 HALF_UP — crate::format 同源语义)
    F(f64, u8),
}

/// Java `String.format(tpl, args...)` 的受限子集 — getload 的 fmdata 摘要串构造
/// (Java L1464-1560)。模板来自 Lang 运行时表 (可被 lang/cur.properties 覆盖),
/// 不能编译期展开, 故运行时扫描 `%` 转换: `%s`/`%d`/`%.Mf`/`%%` (getload 用到的
/// 全部形态; 宽度域未用不支持)。参数耗尽 = Java MissingFormatArgumentException
/// → panic (由 from_read_data 的 catch_unwind 收敛, 同一防线)。
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
                    out.push_str(&crate::format::java_f(*v, prec as usize));
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

impl Blkx {
    /// 对应 Java `public Blkx(String filepath, String name)` (L1665-1667) +
    /// `public Blkx(String filepath, String name, boolean doLoad)` (L1669-1726)。
    ///
    /// 失败映射: Java 构造器不外抛, 以 `valid=false` 对象表达失败; Result 化后
    /// 统一收敛为 `Err(诊断串)` — 文件不存在/读入失败/空文件/JSON 误喂各自携带
    /// 与 Java 日志同文的诊断 (缺失与空文件在 Java 是静默路径, 诊断串为本侧补充),
    /// `Ok(blkx)` 恒有 `blkx.valid == true`。FMLoader 波次已按 exists() 前置探测 +
    /// Err 收敛区分 MISSING/CORRUPT (对齐 Java FMLoader L65/L102, 见 fm_loader.rs)。
    ///
    /// PORT: 单参入口的 `name` (Java readFileName) 取路径文件名分量 — Java 五处调用
    /// 无一如此 (FMLoader L71 传 `name+".blk"`、L101 传 `fmfile` 相对路径 "fm/xxx.blk",
    /// FMListRowRenderer L250 / CompactComparisonWindow L477 / PowerCurveWindow L262
    /// 各传逻辑机型名); readFileName 下游进用户可见版本串 (getload L1471), 生产路径
    /// (FMLoader) 一律走具名入口 [`Blkx::parse_named`]。本入口现仅测试消费。
    // PORT: doLoad=true — Java 单测入口 `new Blkx(path, name)` 两参构造器即
    // `this(filepath, name, true)` (L1665-1667), getload 全量装载随构造执行。
    #[allow(dead_code)] // 具名入口 parse_named 已由 fm_loader 消费; 单参版仅测试消费
    pub fn parse(filepath: &str) -> Result<Blkx, String> {
        // Java 构造器头部的 `fmdata = Lang.noblkx;` (L1673) 在 from_read_data 内统一执行,
        // 此处不再预构造 (曾遗留死赋值: 对象随即被丢弃, 白做一次 default+init_lang)
        let sb = Self::read_to_string(filepath)?;
        let name = std::path::Path::new(filepath)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self::from_read_data(&name, sb, true)
    }

    /// (path, name) 具名入口 — Java **两参**构造器 `Blkx(filepath, name)` 的
    /// 调用约定 (= `this(filepath, name, true)`, doLoad=true 全量装载):
    /// `readFileName` 由调用方显式给足 (FMLoader L101 物理文件传 fmfile 相对
    /// 路径 "fm/xxx.blk"), 不取文件名分量 — readFileName 下游进用户可见版本串
    /// (getload L1471)。
    ///
    /// FMLoader 波次 (fm/fm_loader.rs) 物理 FM 的加载点。
    pub fn parse_named(filepath: &str, name: &str) -> Result<Blkx, String> {
        Self::parse_named_opts(filepath, name, true)
    }

    /// Java **三参**构造器 `Blkx(filepath, name, doLoad)` — doLoad=false 的只读
    /// 形态 (FMLoader L71 中央文件: 仅读头部/燃油改装, 不触发全量解析)。
    pub fn parse_named_opts(filepath: &str, name: &str, do_load: bool) -> Result<Blkx, String> {
        let sb = Self::read_to_string(filepath)?;
        Self::from_read_data(name, sb, do_load)
    }

    /// 构造器共用的文件读入段 (L1675-1692): exists 探测 + readLine 语义拼接 +
    /// 读失败/空文件/JSON 守卫前的 Err 收敛。
    fn read_to_string(filepath: &str) -> Result<String, String> {
        // Java 构造器头部的 `fmdata = Lang.noblkx;` (L1673) 在 from_read_data 内统一执行,
        // 此处不再预构造 (曾遗留死赋值: 对象随即被丢弃, 白做一次 default+init_lang)
        let file = std::path::Path::new(filepath);
        if !file.exists() {
            return Err(format!("FM文件不存在: {filepath}"));
        }
        let mut sb = String::new();
        // 防御加固: 标记文件是否完整读入。原代码 IOException 后 data 为空串但 valid 仍置
        // true, 假有效对象会流入后续解析流程 (Service/UI 拿到空 data 的 Blkx 当真 FM 用) —
        // Java 以 readOk 标志落进下方 "!readOk || data.trim().isEmpty()" 守卫, 此处读失败
        // 直接收敛为 Err 提前返回 (短路守卫首位, 结果一致)
        // PORT: Java new BufferedReader(new FileReader(file)) 用平台默认字符集
        // (中文 Windows=GBK, 坏字节解成 '?' 继续解析); Rust BufReader::lines() 为
        // strict UTF-8, 非法字节产出 Err → 按读失败收敛 (域内 FM 文件纯 ASCII, 等价;
        // model.rs get_version 同款裁决)。行语义: readLine 以 \n/\r/\r\n 为行界,
        // lines() 仅按 \n 切并剥行尾单个 \r (单独 \r 不终止行) — 域内无单独 \r。
        let read_res: std::io::Result<()> = (|| {
            use std::io::{BufRead, BufReader};
            let f = std::fs::File::open(file)?;
            for line in BufReader::new(f).lines() {
                sb.push_str(&line?);
                sb.push('\n');
            }
            Ok(())
        })();
        if let Err(e) = read_res {
            // crate 内暂无 Logger (model.rs 同款注), Err 串承载同一诊断文本
            return Err(format!("FM文件读取: {e}"));
        }
        Ok(sb)
    }

    /// 供测试/fuzz 的注入入口: 以 `content` 直接充当构造器读入完成的 `data`
    /// (跳过文件 IO 与 readLine 归一化 — 归一化只发生在文件路径 parse: 剥 \r、
    /// 保证行尾 \n; 喂 \n 行界文本时与 `parse` 对同一文件的行为等价)。
    /// PORT: Java 无此构造器 (任务指定的纯函数入口), 守卫逻辑与构造器尾段逐行同源;
    /// doLoad=false 形态 (等价 Java 三参构造器 false — 中央文件只读/守卫与原语测试面,
    /// 与 parse(true) 的差异: 不跑 getload 全量装载, fuzz 对 getload 的覆盖走 parse)。
    #[allow(dead_code)] // 本波仅测试消费 (fuzz 套件后续波次接入)
    pub fn parse_str(name: &str, content: &str) -> Result<Blkx, String> {
        Self::from_read_data(name, content.to_string(), false)
    }

    /// 构造器尾段 L1694-1726: readFileName/data 赋值 + 空文件/JSON 守卫 + valid 置位。
    /// `do_load` = Java 三参构造器的 doLoad: true → `try { getload(); valid=true }
    /// catch (Exception) { error 日志; fmdata=noblkx; valid=false }` — getload 的
    /// 任何 panic (畸形文件越界/解析错) 不允许外泄 (Java 防御加固原注释), 由
    /// catch_unwind 收敛为 Err ↔ valid=false (FMLoader 收敛 CORRUPT 同语义)。
    /// (Java 的 `!readOk ||` 短路首位由 parse 的提前 return 承接, 结果一致。)
    // PORT: Java 保真 — `this.fmdata = ...` 构造器尾段逐行直译, 不改 struct 字面量
    #[allow(clippy::field_reassign_with_default)]
    fn from_read_data(name: &str, sb: String, do_load: bool) -> Result<Blkx, String> {
        let mut b = Blkx::default();
        b.fmdata = Some(Lang::init_lang().noblkx.to_string());
        b.read_file_name = Some(name.to_string());
        b.data = Some(sb);
        let data = b.data.as_deref().unwrap(); // 上一行刚赋值, 恒 Some
        // 防御加固: 读失败或空文件一律判无效, 不允许空 data 带着 valid=true 走后续解析
        if java_trim(data).is_empty() {
            return Err(format!("FM文件读取失败或内容为空: {name}"));
        }
        // 防御加固: 用户误喂 JSON 文件 (拖错文件/version 文件等) 时优雅判无效。
        // Dagor .blk 格式不可能以 '{' 开头, JSON 对象一定以 '{' 开头, 以此快速识别
        if java_trim(data).starts_with('{') {
            return Err(format!("JSON 格式文件误作 FM 加载, 标记无效: {name}"));
        }
        if do_load {
            //         Logger.error("Blkx", "FM 解析失败, 标记无效: " + name + " - " + e);
            //         fmdata = Lang.noblkx; valid = false; }
            // PORT(§6): panic 展开前的默认 hook 打印 = e.printStackTrace 对应物
            // (service_loop catch_unwind 同款论证); AssertUnwindSafe = Java
            // "半初始化对象不外泄" 的宽松契约
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.getload())) {
                Ok(()) => {
                    b.valid = true;
                }
                Err(payload) => {
                    let msg = if let Some(s) = payload.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
                        (*s).to_string()
                    } else {
                        "null".to_string()
                    };
                    logger::error(
                        "Blkx",
                        &format!("FM 解析失败, 标记无效: {name} - {msg}"),
                    );
                    // 对象, 此行仅为日志面对齐的语义注记)
                    return Err(format!("FM 解析失败, 标记无效: {name} - {msg}"));
                }
            }
        } else {
            b.valid = true;
        }
        Ok(b)
    }

    /// 对应 Java `public String getArray(String label)` (L1764-1804) —
    /// 点分标签逐段 cut 后, 收集**所有**匹配行 (含行尾 '\n') 拼接; 无匹配返回 ""。
    /// (BlkSource 迁移: 主体平移至 TextSrc::get_str_all, 此处薄委托)
    pub fn get_array(&self, label: &str) -> String {
        TextSrc::new(self.data.as_deref().unwrap()).get_str_all(label)
    }

    /// 对应 Java `public String getlastone(String label)` (L1806-1837) —
    /// 点分标签逐段 cut 后, 取**最后一次**匹配的行值 (不含行尾 '\n');
    /// 未找到或无 '=' 时 Java 返回 null → None。
    pub fn getlastone(&self, label: &str) -> Option<String> {
        // PORT: Java `String text = data` 为 null 时在 toUpperCase 处 NPE ↔ unwrap panic
        let mut text: String = self.data.clone().unwrap();
        // 第一步处理
        let mut clsbix = 0usize;
        for i in 0..label.len() {
            if label.as_bytes()[i] == b'.' {
                let cls = &label[clsbix..i];
                text = cut(&text, cls);
                clsbix = i + 1;
            }
        }
        let label = &label[clsbix..];
        // 第二步获得值
        let mut bix = text.to_uppercase().rfind(&label.to_uppercase())?;
        // 防御加固: 无 '=' 时按"未找到"返回 (原代码扫出末尾越界)
        while bix < text.len() && text.as_bytes()[bix] != b'=' {
            bix += 1;
        }
        if bix >= text.len() {
            return None;
        }
        bix += 1;
        let mut eix = bix;
        // 防御加固: 行尾无换行时取到文本末尾 (substring 到 length() 合法), 不再扫越界
        while eix < text.len() && text.as_bytes()[eix] != b'\n' {
            eix += 1;
        }
        Some(text.get(bix..eix).unwrap_or("").to_string())
    }

    /// 对应 Java `public String getoneinData(String D, String label)` (L1839-1871) —
    /// 同 getone 但数据源为显式传入的 D (子块文本); 未找到返回哨兵串 "null"。
    /// (BlkSource 迁移: 主体平移至 TextSrc::get_in_text, 此处薄委托)
    pub fn getonein_data(&self, d: &str, label: &str) -> String {
        TextSrc::get_in_text(d, label)
    }

    /// 对应 Java `public String getone(String label)` (L1873-1906) —
    /// 同 getoneinData 但数据源为 self.data, 且定位是**大小写敏感** indexOf
    /// (toUpperCase 版本在源码里已被注释掉, oracle go_o2 钉死);
    /// 未找到返回哨兵串 "null"。
    /// (BlkSource 迁移: 主体平移至 TextSrc::get_str, 此处薄委托)
    pub fn getone(&self, label: &str) -> String {
        TextSrc::new(self.data.as_deref().unwrap()).get_str(label)
    }

    // ------------------------------------------------------------------
    // getdouble 族 (Java L523-569) — 已随 BlkSource 迁移为 trait 默认方法
    // (get_f64/get_f64_exc/get_f64s): Float.parseFloat 域解析逻辑单源,
    // 文本/JSON 后端共享, 见 reader.rs 尾部 trait 定义。
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // getPartsFm / extractRpmFromThrottleAuto / getEngineLoad / initEngineLoad
    // (Java L408-475 / L817-853)
    // ------------------------------------------------------------------

    /// 对应 Java `public void getPartsFm(String c, fm_parts p)` (L408-418)。
    /// (BlkSource 迁移: 数据源泛型化, 静态形态)
    fn get_parts_fm<S: BlkSource>(src: &S, c: &str, p: &mut FmParts) {
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

    /// 对应 Java `private void extractRpmFromThrottleAuto(String hdrString)`
    /// (L431-475)。形参 hdrString 在 Java 方法体内未被引用 — `_` 前缀保真保留
    /// (get_aoa_low_v_wing 同款先例)。
    /// (BlkSource 迁移: 数据源泛型化)
    fn extract_rpm_from_throttle_auto<S: BlkSource>(&mut self, src: &S, _hdr_string: &str) {
        self.military_rpm = 0.0;
        self.wep_rpm = 0.0;

        // Try to find Propellor section within the engine type
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
    /// (L477-494) — 读一个 Load 档; WaterLimit/OilLimit 为 0 即该档缺席。
    /// (BlkSource 迁移: 数据源泛型化, 静态形态)
    fn get_engine_load<S: BlkSource>(src: &S, el: &mut [EngineLoad], load_index: usize) -> bool {
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

    /// 对应 Java `public void initEngineLoad()` (L817-853)。
    /// `Application.maxEngLoad` = 10 (Java 常量, Application.java:67)。
    /// (BlkSource 迁移: 数据源泛型化)
    fn init_engine_load<S: BlkSource>(&mut self, src: &S) {
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
                "Blkx",
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
                "Blkx",
                &format!(
                    "Load{} Water/Oil: [{}, {}] WEP/Rec: [{}, {}]",
                    i,
                    crate::format::format(eng_load[i].water_limit, 1),
                    crate::format::format(eng_load[i].oil_limit, 1),
                    crate::format::format(eng_load[i].work_time, 1),
                    crate::format::format(eng_load[i].recover_time, 1)
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
    // getload (Java L855-1590) — FM 全量数据装载 (doLoad=true 的方法体)
    // ------------------------------------------------------------------

    /// 对应 Java `public void getload()` (L855-1590) — 翼/引擎/增压器/推力表/
    /// vne/面积/重量族的全量装载 + fmdata 摘要串构造。
    ///
    /// PORT 纪律: 逐行直译, 语句顺序与 Java 一致 (含源码自身的重复段/死存储 —
    /// AFuselage 重复读两遍、Stab/KeelAngle 段误写 WingAngle 的 bug 均保真保留);
    /// 浮点字面量按 §2.12 (1.0f/1000.f 拓宽域, 精确值直书); `(int)` 强转按 §2.2。
    /// panic 语义 (§1): Java 由构造器 catch(Exception) 收敛 valid=false ↔ 本方法
    /// 的 panic 由 from_read_data 的 catch_unwind 收敛 Err (畸形输入防线)。
    // PORT(allow needless_range_loop): 方法体多处 Java for(int i...) 直译 — i 进
    // format! 键名 (ThrustMax.Altitude_{i} 等), 计数形态是本意
    #[allow(clippy::needless_range_loop)]
    pub fn getload(&mut self) {
        // BlkSource 迁移: 文本后端包装。data 本地 clone 断开借用 (TextSrc 借它,
        // getload_from 需 &mut self); data 为 None 时喂空串 — 泛型化前 getone
        // 在此形态下 unwrap panic, TextSrc 空串与 cut/find 全部"未找到"收敛,
        // 行为等价; 生产路径 from_read_data 先行赋值不可达
        let data = self.data.clone().unwrap();
        let src = TextSrc::new(&data);
        self.getload_from(&src);
    }

    /// getload 的数据源泛型体 — 文本/JSON 后端共用同一份字段填充逻辑
    /// (blkx→json 迁移的构造性对拍保证: 分歧只在 BlkSource 寻址层)。
    #[allow(clippy::needless_range_loop)]
    pub(crate) fn getload_from<S: BlkSource>(&mut self, src: &S) {
        let start_time = std::time::Instant::now(); // System.currentTimeMillis 计时面
        self.is_jet = false;

        // 读取推力高度
        self.engine_num = 1;
        let mut hdr_string = "EngineType0.".to_string();
        let res = src.get_str("EngineType0.Main.Type");
        if res.contains("Jet") {
            // 判断喷气
            self.is_jet = true;
            // 防御加固 (Java 同款): 引擎数上限 = State.maxEngNum (遥测数组容量,
            // 解析上限=可消费上限), 病态文件 O(n²) 全串扫描防护
            while src.get_str(&format!("Engine{}", self.engine_num)) != "null" {
                self.engine_num += 1;
                if self.engine_num >= MAX_ENG_NUM as i32 {
                    // 检视反馈 (Java 同款): 超限截断显式告警, 不静默
                    if src.get_str(&format!("Engine{}", self.engine_num)) != "null" {
                        logger::warn(
                            "Blkx",
                            &format!(
                                "引擎数超过解析上限 {}, Engine{}+ 被截断 (如为真实机型请上调 State.maxEngNum), FM: {}",
                                MAX_ENG_NUM,
                                self.engine_num,
                                self.read_file_name.clone().unwrap_or_default()
                            ),
                        );
                    }
                    break;
                }
            }
        } else {
            if res == "null" {
                hdr_string = "Engine0.".to_string();
                if src.get_str("Engine0.Main.Type").contains("Jet") {
                    self.is_jet = true;
                }
            }
            // 遍历引擎数量（适用于所有非喷气引擎，包括活塞引擎）
            while src.get_str(&format!("Engine{}", self.engine_num)) != "null" {
                self.engine_num += 1;
                if self.engine_num >= MAX_ENG_NUM as i32 {
                    if src.get_str(&format!("Engine{}", self.engine_num)) != "null" {
                        logger::warn(
                            "Blkx",
                            &format!(
                                "引擎数超过解析上限 {}, Engine{}+ 被截断 (如为真实机型请上调 State.maxEngNum), FM: {}",
                                MAX_ENG_NUM,
                                self.engine_num,
                                self.read_file_name.clone().unwrap_or_default()
                            ),
                        );
                    }
                    break;
                }
            }
        }
        self.engine_rpm_mult_wep = 1.0;
        if self.is_jet {
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
            self.mode_engine_mult = Some(mode_engine_mult);
            self.mode_engine_rpm_mult = Some(mode_engine_rpm_mult);

            let mut engine_mult_wep = 1.0f64;
            if self.mode_engine_num != 0 {
                engine_mult_wep = mode_engine_mult[self.mode_engine_num as usize - 1];
                self.engine_rpm_mult_wep =
                    mode_engine_rpm_mult[self.mode_engine_num as usize - 1];
            }

            // 读取推力系数包络
            let alt_n = self.alt_thr_num as usize;
            let vel_n = self.vel_thr_num as usize;
            let mut max_thr_coff: Vec<Vec<f64>> = vec![vec![0.0; vel_n]; alt_n];
            let mut max_thr: Vec<Vec<f64>> = vec![vec![0.0; vel_n]; alt_n];
            let mut max_thr_aft: Vec<Vec<f64>> = vec![vec![0.0; vel_n]; alt_n];
            let mut max_thr_aft_coff: Vec<Vec<f64>> = vec![vec![0.0; vel_n]; alt_n];
            for i in 0..alt_n {
                for j in 0..vel_n {
                    max_thr_coff[i][j] =
                        src.get_f64(&format!("ThrustMax.ThrustMaxCoeff_{i}_{j}"));
                    max_thr_aft_coff[i][j] =
                        src.get_f64(&format!("ThrustMax.ThrAftMaxCoeff_{i}_{j}"));
                    if max_thr_aft_coff[i][j] == 0.0 {
                        max_thr_aft_coff[i][j] = 1.0;
                    }
                    max_thr[i][j] =
                        self.thr_max0 * max_thr_coff[i][j] * self.engine_num as f64;
                    max_thr_aft[i][j] = self.thr_max0 * max_thr_coff[i][j] * self.aftb_coff
                        * max_thr_aft_coff[i][j]
                        * engine_mult_wep
                        * self.engine_num as f64;
                }
            }
            // 预计算峰值推力
            self.peak_thr_mil = self.calculate_peak_thrust(Some(&max_thr));
            self.peak_thr_aft = self.calculate_peak_thrust(Some(&max_thr_aft));
            self.max_thr_coff = Some(max_thr_coff);
            self.max_thr = Some(max_thr);
            self.max_thr_aft = Some(max_thr_aft);
            self.max_thr_aft_coff = Some(max_thr_aft_coff);

            logger::info(
                "Blkx",
                &format!(
                    "Jet Engine Thrust Table loaded ({}x{}), peak MIL={} kgf, AFT={} kgf",
                    self.alt_thr_num,
                    self.vel_thr_num,
                    crate::format::format(self.peak_thr_mil, 0),
                    crate::format::format(self.peak_thr_aft, 0)
                ),
            );
        } else {
            // radial inline
            self.aftb_coff = src.get_f64(&format!("{hdr_string}Main.AfterburnerBoost"));
            self.comp_num_steps = src.get_f64("Compressor.NumSteps") as i32;
            self.speed_to_manifold_multiplier =
                src.get_f64("Compressor.SpeedManifoldMultiplier");

            // NegativeArraySizeException → 构造器 catch; as usize 巨量 → Vec
            // 分配 panic 同被 from_read_data 收敛 (CORRUPT 同语义)
            let n = self.comp_num_steps as usize;
            let mut comp_alt = vec![0.0f64; n];
            let mut comp_boost = vec![0.0f64; n];
            let mut has_comp_boost = vec![false; n];
            let mut comp_power = vec![0.0f64; n];
            let mut comp_rpm_ratio = vec![0.0f64; n];
            let mut comp_ceil = vec![0.0f64; n];
            let mut comp_ceil_pwr = vec![0.0f64; n];
            let mut comp_const_rpm_alt = vec![0.0f64; n];
            let mut comp_const_rpm_power = vec![0.0f64; n];
            for i in 0..n {
                comp_alt[i] = src.get_f64(&format!("Compressor.Altitude{i}"));
                comp_power[i] = src.get_f64(&format!("Compressor.Power{i}"));
                comp_boost[i] = src.get_f64(&format!("Compressor.AfterburnerBoostMul{i}"));
                has_comp_boost[i] =
                    src.get_str(&format!("Compressor.AfterburnerBoostMul{i}")) != "null";
                comp_rpm_ratio[i] =
                    src.get_f64(&format!("Compressor.PowerConstRPMCurvature{i}"));
                comp_ceil[i] = src.get_f64(&format!("Compressor.Ceiling{i}"));
                comp_ceil_pwr[i] = src.get_f64(&format!("Compressor.PowerAtCeiling{i}"));
                comp_const_rpm_alt[i] =
                    src.get_f64(&format!("Compressor.AltitudeConstRPM{i}"));
                comp_const_rpm_power[i] =
                    src.get_f64(&format!("Compressor.PowerConstRPM{i}"));
            }
            self.comp_alt = Some(comp_alt);
            self.comp_boost = Some(comp_boost);
            self.has_comp_boost = Some(has_comp_boost);
            self.comp_power = Some(comp_power);
            self.comp_rpm_ratio = Some(comp_rpm_ratio);
            self.comp_ceil = Some(comp_ceil);
            self.comp_ceil_pwr = Some(comp_ceil_pwr);
            self.comp_const_rpm_alt = Some(comp_const_rpm_alt);
            self.comp_const_rpm_power = Some(comp_const_rpm_power);

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
            self.extract_rpm_from_throttle_auto(src, &hdr_string);
        }
        self.max_allowed_rpm = src.get_f64("RPMMaxAllowed");

        self.version = self.get_version();
        self.init_engine_load(src);

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

        self.clmax = src.get_f64("NoFlaps.ClCritHigh");
        self.flap_clmax = src.get_f64("FullFlaps.ClCritHigh");

        self.aoa_high = src.get_f64("NoFlaps.alphaCritHigh");
        self.aoa_low = src.get_f64("NoFlaps.alphaCritLow");

        self.flap_aoa_high = src.get_f64("FullFlaps.alphaCritHigh");
        self.flap_aoa_low = src.get_f64("FullFlaps.alphaCritLow");

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

        self.wing_taper_ratio = src.get_f64("WingTaperRatio");
        if self.wing_taper_ratio == 0.0 {
            self.wing_taper_ratio = src.get_f64("WingPlane.TaperRatio");
            if self.wing_taper_ratio == 0.0 {
                self.wing_taper_ratio = src.get_f64("WingPlaneSweep0.TaperRatio");
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
        // (BlkSource 迁移: 闭包从 &Blkx 改捕 src)
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
        // Java 源码将 AFuselage 三级回退段**原样重复了两遍** (L1252-1261) — 第二遍
        // 读到相同值, 净效果为同一赋值; 保真保留重复调用
        self.a_fuselage = fallback3(
            "Areas.Fuselage",
            "FuselagePlane.Areas.Main",
            "WingPlaneSweep0.Areas.Main",
        );

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

        // 向后兼容: 填充旧字段
        // PORT: Java 引用共享 (V50 = sweepLevels.get(1).noFlaps) → 值克隆
        // (mod.rs 字段区裁决: 解析后只读)
        let mut no_flaps_wing_v50 = FmParts::default();
        let mut no_flaps_wing_v100 = FmParts::default();
        let mut full_flaps_wing_v50 = FmParts::default();
        let mut full_flaps_wing_v100 = FmParts::default();
        if sweep_levels.len() >= 2 {
            no_flaps_wing_v50 = sweep_levels[1].no_flaps.clone().unwrap_or_default();
            full_flaps_wing_v50 = sweep_levels[1].full_flaps.clone().unwrap_or_default();
            self.vne_v50 = sweep_levels[1].vne;
            self.vne_mach_v50 = sweep_levels[1].vne_mach;
        }
        if sweep_levels.len() >= 3 {
            let last = sweep_levels.len() - 1;
            no_flaps_wing_v100 = sweep_levels[last].no_flaps.clone().unwrap_or_default();
            full_flaps_wing_v100 = sweep_levels[last].full_flaps.clone().unwrap_or_default();
            self.vne_v100 = sweep_levels[last].vne;
            self.vne_mach_v100 = sweep_levels[last].vne_mach;
        }

        let mut fuselage = FmParts::default();
        Self::get_parts_fm(src, "Fuselage", &mut fuselage);
        if fuselage.aoa_crit_high == 0.0 {
            Self::get_parts_fm(src, "FuselagePlane.Polar", &mut fuselage);
        }
        self.aoa_fuselage_high = fuselage.aoa_crit_high;
        self.aoa_fuselage_low = fuselage.aoa_crit_low;

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

        // ---- fmdata 摘要串构造 (Java L1464-1560 的 String.format 族) ----
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

        // 计算滚转率

        // 先计算Cla
        // (死存储保真: cl_a 写入后无读取方, Java 亦然 — mod.rs allow(dead_code))
        self.cl_a = (no_flaps_wing.cl_crit_high - no_flaps_wing.cl0)
            / no_flaps_wing.aoa_crit_high;

        // 获得襟翼偏转角度(上偏和下偏)
        let mut aileron_defl = [0.0f64; 2];
        if src
            .get_f64s("AileronAngles", &mut aileron_defl, 2)
            .is_none()
        {
            let _ = src.get_f64s("Ailerons.AnglesRoll", &mut aileron_defl, 2);
        }
        self.aileron_defl = Some(aileron_defl);

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

        s = Self::write_parts_fm(s, &no_flaps_wing, &lang);
        if no_flaps_wing_v50.cl_crit_high != 0.0 {
            s = Self::write_parts_fm(s, &no_flaps_wing_v50, &lang);
        }
        if no_flaps_wing_v100.cl_crit_high != 0.0 {
            s = Self::write_parts_fm(s, &no_flaps_wing_v100, &lang);
        }
        s = Self::write_parts_fm(s, &full_flaps_wing, &lang);
        s = Self::write_parts_fm(s, &fuselage, &lang);
        s = Self::write_parts_fm(s, &fin, &lang);
        s = Self::write_parts_fm(s, &stab, &lang);

        // 部件实体落位 (Java: 构造过程中的 new fm_parts 赋值在此集中)
        self.no_flaps_wing = Some(no_flaps_wing);
        self.full_flaps_wing = Some(full_flaps_wing);
        self.no_flaps_wing_v50 = Some(no_flaps_wing_v50);
        self.no_flaps_wing_v100 = Some(no_flaps_wing_v100);
        self.full_flaps_wing_v50 = Some(full_flaps_wing_v50);
        self.full_flaps_wing_v100 = Some(full_flaps_wing_v100);
        self.sweep_levels = Some(sweep_levels);
        self.fuselage = Some(fuselage);
        self.fin = Some(fin);
        self.stab = Some(stab);

        self.fmdata = Some(s);

        let duration = start_time.elapsed().as_millis() as i64;
        logger::info(
            "Blkx",
            &format!(
                "Parsed FM file '{}' in {} ms (Engine Count: {}, Jet: {})",
                self.read_file_name.clone().unwrap_or_default(),
                duration,
                self.engine_num,
                self.is_jet
            ),
        );
    }

    /// 对应 Java `public String WritePartsFm(String s, fm_parts p)` (L502-520)。
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

    // ------------------------------------------------------------------
    // transUnit / getAllplotdata / getplotdata (Java L1590-1658) —
    // PASSPORT 曲线抽取 + 英制单位换算 (getAllplotdata 批次)
    // ------------------------------------------------------------------

    /// 对应 Java `public void transUnit()` (L1590-1616) — 英制单位系 FM 的
    /// PASSPORT 曲线换算 (高度英尺→米 0.3048 / 速度英里每小时→公里 1.609344)。
    ///
    /// PORT: Java `loc.y[i] * 0.3048f` — float 字面量先取 f32 值再拓宽
    /// double 参与乘法 (24-bit 尾数域, `(0.3048f32 as f64) ≠ 0.3048f64`);
    /// oracle (DumpPlot 腿B, OpenJDK 1.8.0_342): 1000 * 0.3048f =
    /// 304.80000376701355, 321.84 * 1.609344f = 517.9512747573852, 位级一致。
    ///
    /// PORT: loc..loc3 未赋值 (Java null) 时 NPE ↔ unwrap panic — 生产调用点
    /// 是 get_all_plotdata 尾部 (五字段刚赋值, 不可达); 直连时的 panic 由
    /// FMLoader.load 的 catch_unwind 收敛 CORRUPT (§1)。
    #[allow(unused_assignments)]
    #[allow(clippy::needless_range_loop)]
    pub fn trans_unit(&mut self) {
        // BlkSource 迁移: 文本后端包装 (getload 同款本地 clone 断借用)
        let data = self.data.clone().unwrap();
        let src = TextSrc::new(&data);
        self.trans_unit_from(&src);
    }

    /// transUnit 的数据源泛型体 (BlkSource 迁移)。
    #[allow(unused_assignments)]
    #[allow(clippy::needless_range_loop)]
    fn trans_unit_from<S: BlkSource>(&mut self, src: &S) {
        let mut unit_system = "".to_string();
        unit_system = src.get_str("PASSPORT.UNITSYSTEM");
        unit_system = self.sub_st(&unit_system);
        // 字节 find ≡ UTF-16 indexOf, §2.1)。getone 未找到时返回哨兵 "null",
        // sub_st 剥首尾得 "ul" 同样不含 "Imperial" → 空转 (DumpPlot 腿A 钉死;
        // 真机 FM 键名恒小写 camelCase, getone 大小写敏感 → metric 数据不走换算)
        if unit_system.find("Imperial").is_some() {
            // Application.debugPrint("英制");
            // PORT: Java for (int i = 0; i < loc.cur; i++) — cur 在循环体内
            // 不被修改, 绑定一次等价 (i32 计数 → usize 供数组索引);
            // `loc.y[i] = loc.y[i] * 0.3048f` 的赋值形态 → `*=` (clippy
            // manual_assign, 单操作数乘法逐位等价)
            let loc = self.loc.as_mut().unwrap();
            let cur = loc.cur as usize;
            for i in 0..cur {
                loc.y[i] *= 0.3048f32 as f64;
            }
            let loc0 = self.loc0.as_mut().unwrap();
            let cur = loc0.cur as usize;
            for i in 0..cur {
                loc0.y[i] *= 0.3048f32 as f64;
            }
            let loc1 = self.loc1.as_mut().unwrap();
            let cur = loc1.cur as usize;
            for i in 0..cur {
                loc1.y[i] *= 0.3048f32 as f64;
                loc1.x[i] *= 1.609344f32 as f64;
            }
            let loc2 = self.loc2.as_mut().unwrap();
            let cur = loc2.cur as usize;
            for i in 0..cur {
                loc2.y[i] *= 0.3048f32 as f64;
                loc2.x[i] *= 1.609344f32 as f64;
            }
            let loc3 = self.loc3.as_mut().unwrap();
            let cur = loc3.cur as usize;
            for i in 0..cur {
                loc3.y[i] *= 1.609344f32 as f64;
                // Application.debugPrint(loc3.x[i]+" "+loc3.y[i]);
            }
        }
    }

    /// 对应 Java `public void getAllplotdata()` (L1618-1625) — 五条 PASSPORT
    /// 曲线全量抽取 + 单位换算 (FMLoader.load 第 6 步, finalizeLoading 前)。
    pub fn get_all_plotdata(&mut self) {
        // BlkSource 迁移: 文本后端包装 (getload 同款本地 clone 断借用)
        let data = self.data.clone().unwrap();
        let src = TextSrc::new(&data);
        self.get_all_plotdata_from(&src);
    }

    /// getAllplotdata 的数据源泛型体 (BlkSource 迁移)。
    pub(crate) fn get_all_plotdata_from<S: BlkSource>(&mut self, src: &S) {
        self.loc = Some(Self::getplotdata_from(src, "PASSPORT.ALT.minClimbTimeWep"));
        self.loc0 = Some(Self::getplotdata_from(src, "PASSPORT.ALT.minClimbTimeNom"));
        self.loc1 = Some(Self::getplotdata_from(src, "PASSPORT.ALT.maxSpeedWep"));
        self.loc2 = Some(Self::getplotdata_from(src, "PASSPORT.ALT.maxSpeedNom"));
        self.loc3 = Some(Self::getplotdata_from(src, "PASSPORT.IAS.maxRollRateLeft"));
        self.trans_unit_from(src);
    }

    /// 对应 Java `public XY getplotdata(String t)` (L1627-1658) — 抽取点分
    /// 标签曲线块 (getArray 多行累积) 的逐行 (y, x) 对; 无匹配时空表
    /// (cur=0, 数组长度 0)。
    // PORT(allow needless_range_loop): Java for(int i...) 直译 — i 是行段
    // substring 的终点索引, 计数形态是本意
    #[allow(clippy::needless_range_loop)]
    pub fn getplotdata(&self, t: &str) -> XY {
        // BlkSource 迁移: 文本后端包装
        let src = TextSrc::new(self.data.as_deref().unwrap());
        Self::getplotdata_from(&src, t)
    }

    /// getplotdata 的数据源泛型体 (BlkSource 迁移, 静态形态)。
    #[allow(clippy::needless_range_loop)]
    fn getplotdata_from<S: BlkSource>(src: &S, t: &str) -> XY {
        let mut line = 0usize;
        let t = src.get_str_all(t);
        for i in 0..t.len() {
            if t.as_bytes()[i] == b'\n' {
                line += 1;
            }
        }
        let mut lo = XY::new(line);
        let mut bix = 0usize;
        for i in 0..t.len() {
            if t.as_bytes()[i] == b'\n' {
                // 切片 ≡ substring (bix <= i < len 恒合法, 此处无防御加固点)
                let temp = &t[bix..i];
                // PORT: Java split(", ") 丢弃尾部空串, Rust split 保留 — 本方法
                // 只消费 tmp[0]/tmp[1], 尾部空串差异的所有分叉 (Java 长度 <2
                // 跳过 / Rust tmp[1]="" 解析失败丢弃) 最终都不写入数据点, 等价
                let tmp: Vec<&str> = temp.split(", ").collect();
                // 防御加固 (P6 fuzz 发现): 畸形曲线行 (缺逗号/数字混入字符) 原代码
                // 直接 parseDouble 抛异常炸穿调用方 (对比窗口回退路径未包 try)。
                // 改为跳过畸形行 (曲线少一个点), 完好行照常解析——仅曲线块受损的
                // 文件仍可按 READY 用发动机数据
                if tmp.len() >= 2 {
                    // lo.x[lo.cur] = Double.parseDouble(tmp[1].trim()); lo.cur++;
                    // — Double 域 (f64, 与 getdouble 族的 Float 域不同!);
                    // NumberFormatException catch → 丢弃该数据点。
                    // PORT: Java 在 y 写入后 x 解析失败会留下不可观察的脏 y[cur]
                    // (cur 未自增, 消费方只读 [0, cur)), Rust 双成功才写入,
                    // 可观察行为一致; trim 用 java_trim (Java String.trim 语义)
                    if let (Ok(y), Ok(x)) = (
                        java_trim(tmp[0]).parse::<f64>(),
                        java_trim(tmp[1]).parse::<f64>(),
                    ) {
                        let cur = lo.cur as usize;
                        lo.y[cur] = y;
                        lo.x[cur] = x;
                        lo.cur += 1;
                    }
                }
                // 缺逗号的行: 同样跳过 (曲线少一个点)
                bix = i + 1;
            }
        }
        lo
    }
}

/// 对应 Java 包私有成员方法 `String cut(String t, String clslabel)` (L1728-1762) —
/// 提取 `clslabel { ... }` 块的花括号内文本; 未找到返回哨兵串 "null"。
/// PORT: 方法体不读任何实例状态 (t/clslabel 纯入参) → 模块私有自由函数
/// (types.rs cut_static 同款先例); 注意与 cutStatic **不同源**: 成员版无
/// "无空格 label{" 回退 (oracle cut_c5), 括号计数从 bix 起步且带 !=0 联合条件。
fn cut(t: &str, clslabel: &str) -> String {
    let tmp = t;
    let mut i: usize;
    let mut left = 0usize;
    let mut right = 0usize;
    let bix = match tmp.to_uppercase().find(&(clslabel.to_uppercase() + " {")) {
        Some(b) => b,
        None => return "null".to_string(),
    };
    // 防御加固: toUpperCase 可能使特殊 unicode 字符变长 (如 ß→SS), 大写串里量出的索引
    // 不一定落在原串范围内; 索引失效时按"未找到块"返回 null 字符串, 与既有未找到路径一致
    if bix >= tmp.len() {
        return "null".to_string();
    }
    let mut cutleft = bix;
    // 防御加固: 加长度上界——截断文件中块名后没有 '{' 时, 原代码会一直扫出字符串末尾抛
    // StringIndexOutOfBoundsException; 越界按「未找到块」处理
    while cutleft < tmp.len() && tmp.as_bytes()[cutleft] != b'{' {
        cutleft += 1;
    }
    if cutleft >= tmp.len() {
        return "null".to_string();
    }
    cutleft += 1;
    // && left == right) break; } — 计数含块头 '{' (bix 起步, 早于 cutleft)
    i = bix;
    while i < tmp.len() {
        if tmp.as_bytes()[i] == b'{' {
            left += 1;
        }
        if tmp.as_bytes()[i] == b'}' {
            right += 1;
        }
        if left != 0 && right != 0 && left == right {
            break;
        }
        i += 1;
    }
    let cutright = i;
    // 防御加固: 括号不配对/索引错位时 cutright 可能小于 cutleft, substring 会越界,
    // 统一按"未找到块"返回 (正常文件 cutleft <= cutright, 行为不变)
    // PORT: usize 下 cutright 恒 <= len, 首个条件不可达 — 保留判定对齐 Java 语义
    if cutright > tmp.len() || cutleft > cutright {
        return "null".to_string();
    }
    tmp.get(cutleft..cutright).unwrap_or("null").to_string()
}

// =====================================================================
// BlkSource — getload 族的抽取原语抽象 (blkx→json 迁移的唯一分歧面)
//
// 设计: 后端 (TextSrc/JsonSrc) 只负责**寻址** — 把点分标签解析到"值文本";
// 数值解析 (getdouble 族的 Float.parseFloat 域/多值 split) 是 trait 默认方法,
// 单源共享。文本化协议: 多分量值 (JSON 数组 / 文本 p2 行) 统一成 "v0, v1"
// 逗号串, 使 split(',') 逻辑对两后端行为一致 — 位级对拍的构造性保证。
// =====================================================================

/// getload/getplotdata 的数据源原语。语义契约逐条锚定 Blkx 旧文本原语
/// (getone/get_array/cut, 见各后端实现注), 缺席一律返回 "null" 哨兵串。
pub(crate) trait BlkSource {
    /// getone: 点分段 = section 链 (cut, CI), 末段 = 块内 leaf (CS 子串)。
    fn get_str(&self, label: &str) -> String;

    /// get_array: 点分段同上, 末段收集**全部**匹配行 (含行尾 '\n') 拼接
    /// (PASSPORT 曲线 "y, x\n" 行流, getplotdata 的 Double 域解析依赖此形态)。
    fn get_str_all(&self, label: &str) -> String;

    /// cut: 命名块的花括号内视图 (Propellor 回退/WingPlaneSweep 循环用)。
    fn section(&self, name: &str) -> BlkSection;

    /// getdouble (默认实现): `Float.parseFloat` 域 — 首段 trim 后 parse f32
    /// 再拓宽 f64 (1.42f != 1.42, 模块注陷阱 2); 解析失败返回 0。
    fn get_f64(&self, label: &str) -> f64 {
        let mut ret = 0.0;
        let one = self.get_str(label);
        if one != "null" {
            let tmp: Vec<&str> = one.split(',').collect();
            match tmp.first().and_then(|s| s.trim().parse::<f32>().ok()) {
                Some(v) => ret = v as f64,
                None => return 0.0,
            }
        }
        ret
    }

    /// getdouble_exc (默认实现): 缺席哨兵 = Float.MAX_VALUE
    /// (调用方以 `== f32::MAX as f64` 判截断); 解析失败返回 0 (Java catch 路径)。
    fn get_f64_exc(&self, label: &str) -> f64 {
        let mut ret = f32::MAX as f64;
        let one = self.get_str(label);
        if one != "null" {
            let tmp: Vec<&str> = one.split(',').collect();
            match tmp.first().and_then(|s| s.trim().parse::<f32>().ok()) {
                Some(v) => ret = v as f64,
                None => return 0.0,
            }
        }
        ret
    }

    /// getdoubles (默认实现): 就地写 `ret[..num]`, 返回 None ↔ Java 返回 null:
    /// - `num <= 0` → null;
    /// - 键缺席 (get_str "null") → **返回 Some(()) 且 ret 不动** (调用方依赖
    ///   "找不到键时保持 0 初值" 的语义, 如 MomentOfInertia);
    /// - 段数不足/解析失败 → null (Java tmp[i] 越界或 parseFloat 抛 → catch;
    ///   此时已写入的前缀保留 — 部分写入保真)。
    fn get_f64s(&self, label: &str, ret: &mut [f64], num: usize) -> Option<()> {
        if num == 0 {
            return None;
        }
        let one = self.get_str(label);
        if one != "null" {
            let tmp: Vec<&str> = one.split(',').collect();
            for (i, slot) in ret.iter_mut().enumerate().take(num) {
                // 双路径同为 catch → null
                *slot = tmp
                    .get(i)
                    .and_then(|s| s.trim().parse::<f32>().ok())
                    .map(|v| v as f64)?;
            }
        }
        Some(())
    }
}

/// cut 的返回形态: 文本 = 块内字符串 ("null" ↔ Null), JSON = 子树。
#[allow(dead_code)] // Json 变体属阶段 3 接线波次
pub(crate) enum BlkSection {
    Null,
    Text(String),
    Json(serde_json::Value),
}

impl BlkSection {
    /// 文本版 cut 哨兵 `"null"` 的等价判断。
    pub fn is_null(&self) -> bool {
        matches!(self, BlkSection::Null)
    }

    /// getonein_data: 块内 leaf 搜索 (点分段 cut 链 + 末段 CI 定位);
    /// 未找到返回哨兵串 "null"。
    pub fn get_in(&self, label: &str) -> String {
        match self {
            BlkSection::Null => "null".to_string(),
            BlkSection::Text(t) => TextSrc::get_in_text(t, label),
            BlkSection::Json(v) => super::json::get_in_json(v, label),
        }
    }
}

/// 文本后端 — Blkx 旧文本原语 (getone/get_array/cut) 的逐位平移宿主。
/// 持 `&str` 借用 (原实现每次调用 clone 全文, 此处按需 to_string, 行为等价)。
pub(crate) struct TextSrc<'a>(&'a str);

impl<'a> TextSrc<'a> {
    pub fn new(data: &'a str) -> Self {
        TextSrc(data)
    }

    /// getone/getlastone/get_array 共用的第一步: 点分段逐段 cut,
    /// 返回 (剩余文本, 末段标签)。
    fn resolve_path<'l>(&self, label: &'l str) -> (String, &'l str) {
        let mut text = self.0.to_string();
        let mut clsbix = 0usize;
        for i in 0..label.len() {
            if label.as_bytes()[i] == b'.' {
                let cls = &label[clsbix..i];
                text = cut(&text, cls);
                clsbix = i + 1;
            }
        }
        (text, &label[clsbix..])
    }

    /// getonein_data 主体 (Java L1839-1871): 对显式文本块做点分段 cut 链 +
    /// 末段 CI 定位; 未找到返回 "null"。
    pub fn get_in_text(d: &str, label: &str) -> String {
        let mut text: String = d.to_string();
        let mut clsbix = 0usize;
        for i in 0..label.len() {
            if label.as_bytes()[i] == b'.' {
                let cls = &label[clsbix..i];
                text = cut(&text, cls);
                clsbix = i + 1;
            }
        }
        let label = &label[clsbix..];
        let mut bix = match text.to_uppercase().find(&label.to_uppercase()) {
            Some(i) => i,
            None => return "null".to_string(),
        };
        // 防御加固: 无 '=' 时按"未找到"返回 (原代码扫出末尾越界)
        while bix < text.len() && text.as_bytes()[bix] != b'=' {
            bix += 1;
        }
        if bix >= text.len() {
            return "null".to_string();
        }
        bix += 1;
        let mut eix = bix;
        // 防御加固: 末尾无换行符时取到文本末尾 (substring 到 length() 合法), 不再扫越界
        while eix < text.len() && text.as_bytes()[eix] != b'\n' {
            eix += 1;
        }
        text.get(bix..eix).unwrap_or("").to_string()
    }
}

impl<'a> BlkSource for TextSrc<'a> {
    /// getone (Java L1873-1906): 点分段 cut 链 (CI) + 末段 **CS** 子串定位
    /// (toUpperCase 版本在 Java 源码里已被注释掉, oracle go_o2 钉死);
    /// 未找到返回哨兵串 "null"。
    fn get_str(&self, label: &str) -> String {
        let (text, label) = self.resolve_path(label);
        let mut bix = match text.find(label) {
            Some(i) => i,
            None => return "null".to_string(),
        };
        // 防御加固: 无 '=' 时按"未找到"返回 (原代码扫出末尾越界)
        while bix < text.len() && text.as_bytes()[bix] != b'=' {
            bix += 1;
        }
        if bix >= text.len() {
            return "null".to_string();
        }
        bix += 1;
        let mut eix = bix;
        // 防御加固: 行尾无换行时取到文本末尾
        while eix < text.len() && text.as_bytes()[eix] != b'\n' {
            eix += 1;
        }
        text.get(bix..eix).unwrap_or("").to_string()
    }

    /// get_array (Java L1764-1804): 点分段 cut 链 + 收集**所有**匹配行
    /// (含行尾 '\n') 拼接; 无匹配返回 ""。
    #[allow(clippy::while_let_on_iterator)]
    fn get_str_all(&self, label: &str) -> String {
        let mut value = String::new();
        let (mut text, label) = self.resolve_path(label);
        // toUpperCase 每轮对剩余全文重算 (Java 同, O(匹配数×剩余长度) 保真保留)
        let mut bix = text.to_uppercase().find(&label.to_uppercase());
        while let Some(mut bi) = bix {
            // 防御加固: 加长度上界——label 匹配处之后到文本末尾都没有 '=' (如匹配到块名/
            // 注释/截断行) 时, 原代码会扫出末尾抛 StringIndexOutOfBoundsException;
            // 越界时放弃剩余匹配, 返回已积累的 value (与"未找到"时返回空串的语义一致)
            while bi < text.len() && text.as_bytes()[bi] != b'=' {
                bi += 1;
            }
            if bi >= text.len() {
                break;
            }
            bi += 1;
            let mut eix = bi;
            // 防御加固: 末尾无换行符 (init() 直喂的截断文本) 时取到文本末尾
            while eix < text.len() && text.as_bytes()[eix] != b'\n' {
                eix += 1;
            }
            if eix >= text.len() {
                value.push_str(text.get(bi..).unwrap_or(""));
                break;
            }
            value.push_str(text.get(bi..eix + 1).unwrap_or(""));
            // PORT: Java text = text.substring(eix + 1) 共享底层 ↔ drain 原地移除前缀
            text.drain(..eix + 1);
            bix = text.to_uppercase().find(&label.to_uppercase());
        }
        value
    }

    /// cut 包装: 命名块文本 ("null" → Null)。
    fn section(&self, name: &str) -> BlkSection {
        let s = cut(self.0, name);
        if s == "null" {
            BlkSection::Null
        } else {
            BlkSection::Text(s)
        }
    }
}

// =====================================================================
// Tests — 期望值来自 Java 8 oracle 对拍 (§5.1): Blkx.java L1665-1906 逐字提取
// (sed 直提 + 构造器改名, 见 build/oracle/blkx_reader/src/parser/BlxReaderOracle.java)
// 在 OpenJDK 1.8.0_342 (与 bin/ 现役 class 同版) 实测 dump; 字符串以 \n/\r/\t
// 转义单行化后逐字断言。真机文件三份 (spitfire_f24 物理/中央 + bf-109e-4 物理)
// od/grep 实测纯 ASCII 无 CR — Java UTF-16 长度/码元和 ↔ Rust 字节长度/字节和等价。
// =====================================================================
#[cfg(test)]
mod tests;
