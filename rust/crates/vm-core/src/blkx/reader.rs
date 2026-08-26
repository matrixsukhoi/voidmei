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
//! PORT (方法波次边界, 见 mod.rs): 构造器 `doLoad=true` 分支 (L1708-1718) 调用的
//! `getload()` (L855-1590) 属 reader.rs 后续波次 — 本波 parse/parse_str 在读入守卫
//! (空文件/JSON 误喂) 之后即置 `valid=true`, 等价 Java `doLoad=false` 构造
//! (FMLoader.load L71 中央文件只读用法); getload 波次落地后在此补齐
//! `try { getload(); valid=true } catch → valid=false` 语义。
//!
//! PORT (§2.1): Java charAt/indexOf/substring 按 UTF-16 码元计数, 此处一律字节偏移
//! + `as_bytes()` 索引 — 域内 (FM/中央文件) 为纯 ASCII (真机三文件 od/grep 实测),
//!   字节索引与码元索引一致; 定界符 '{'/'}'/'='/'\n' 均为 ASCII, UTF-8 自同步,
//!   逐字节比较不会误判多字节字符 (string_helper.rs / types.rs cut_static 先例)。
//!   `toUpperCase()` 的索引漂移防护 (ß→SS 等病态大写变长输入) 与 Java 的守卫
//!   逐条对应; 终点取子串经 `get()` 边界守卫, 病态漂移按"未找到"哨兵收敛
//!   (types.rs cut_static 同款裁决: Rust UTF-8 字节域 ß 2→2 不漂移, 反而更对齐)。

use super::Blkx;
use crate::lang::Lang;

/// Java `String.trim()`: 剥首尾所有 `<= U+0020` 的字符 (含 \n/\r/\t 与 C0 控制符,
/// **不含** NBSP U+00A0)。
/// PORT: Rust `str::trim` 剥 Unicode White_Space (NBSP 会被剥掉) — 构造器的
/// 空文件/JSON 判定必须用 Java 语义 (oracle ctor_n7: 内容 "\u{A0}{x\n" 的文件
/// Java trim 后非空且不以 '{' 开头 → valid=true)。
fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c as u32) <= 0x20)
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
    // PORT: doLoad=true 的 getload 分支未落地 (模块头注), 当前等价 doLoad=false。
    #[allow(dead_code)] // 具名入口 parse_named 已由 fm_loader 消费; 单参版仅测试消费
    pub fn parse(filepath: &str) -> Result<Blkx, String> {
        // Java 构造器头部的 `fmdata = Lang.noblkx;` (L1673) 在 from_read_data 内统一执行,
        // 此处不再预构造 (曾遗留死赋值: 对象随即被丢弃, 白做一次 default+init_lang)
        let sb = Self::read_to_string(filepath)?;
        let name = std::path::Path::new(filepath)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self::from_read_data(&name, sb)
    }

    /// (path, name) 具名入口 — Java 三参构造器 `Blkx(filepath, name, doLoad)` 的
    /// 调用约定: `readFileName` 由调用方显式给足 (FMLoader L71 中央文件传
    /// `name + ".blk"`, L101 物理文件传 fmfile 相对路径 "fm/xxx.blk"), 不取
    /// 文件名分量 — readFileName 下游进用户可见版本串 (getload L1471)。
    ///
    /// FMLoader 波次 (fm/fm_loader.rs) 的唯一 `new Blkx` 点。
    // PORT: doLoad=true 的 getload 分支未落地 (模块头注), 当前等价 doLoad=false —
    // FMLoader 物理 文件调用点 (Java doLoad=true) 已带 TODO(port) 标注数值字段
    // (engineNum/peakThr/comp* 族) 暂为零值的过渡语义, getload 波次落地后自动补齐
    pub fn parse_named(filepath: &str, name: &str) -> Result<Blkx, String> {
        let sb = Self::read_to_string(filepath)?;
        Self::from_read_data(name, sb)
    }

    /// 构造器共用的文件读入段 (L1675-1692): exists 探测 + readLine 语义拼接 +
    /// 读失败/空文件/JSON 守卫前的 Err 收敛。
    fn read_to_string(filepath: &str) -> Result<String, String> {
        // Java 构造器头部的 `fmdata = Lang.noblkx;` (L1673) 在 from_read_data 内统一执行,
        // 此处不再预构造 (曾遗留死赋值: 对象随即被丢弃, 白做一次 default+init_lang)
        let file = std::path::Path::new(filepath);
        if !file.exists() {
            // Java: else 分支, 静默 valid = false (data/readFileName 保持 null)
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
            let f = std::fs::File::open(file)?; // Java: FileNotFoundException (exists 后消失的 TOCTOU)
            for line in BufReader::new(f).lines() {
                // Java: while ((s = br.readLine()) != null) sb.append(s + "\n");
                sb.push_str(&line?); // Java: IOException
                sb.push('\n');
            }
            // Java: br.close() ↔ Rust Drop
            Ok(())
        })();
        if let Err(e) = read_res {
            // Java: ExceptionHelper.logAndContinue(e, "FM文件读取") 吞错 + readOk=false;
            // crate 内暂无 Logger (model.rs 同款注), Err 串承载同一诊断文本
            return Err(format!("FM文件读取: {e}"));
        }
        Ok(sb)
    }

    /// 供测试/fuzz 的注入入口: 以 `content` 直接充当构造器读入完成的 `data`
    /// (跳过文件 IO 与 readLine 归一化 — 归一化只发生在文件路径 parse: 剥 \r、
    /// 保证行尾 \n; 喂 \n 行界文本时与 `parse` 对同一文件的行为等价)。
    /// PORT: Java 无此构造器 (任务指定的纯函数入口), 守卫逻辑与构造器尾段逐行同源。
    #[allow(dead_code)] // 本波仅测试消费 (fuzz 套件后续波次接入)
    pub fn parse_str(name: &str, content: &str) -> Result<Blkx, String> {
        Self::from_read_data(name, content.to_string())
    }

    /// 构造器尾段 L1694-1726: readFileName/data 赋值 + 空文件/JSON 守卫 + valid 置位。
    /// (Java 的 `!readOk ||` 短路首位由 parse 的提前 return 承接, 结果一致。)
    // PORT: Java 保真 — `this.fmdata = ...` 构造器尾段逐行直译, 不改 struct 字面量
    #[allow(clippy::field_reassign_with_default)]
    fn from_read_data(name: &str, sb: String) -> Result<Blkx, String> {
        let mut b = Blkx::default();
        b.fmdata = Some(Lang::init_lang().noblkx.to_string());
        b.read_file_name = Some(name.to_string());
        b.data = Some(sb);
        let data = b.data.as_deref().unwrap(); // 上一行刚赋值, 恒 Some
        // 防御加固: 读失败或空文件一律判无效, 不允许空 data 带着 valid=true 走后续解析
        if java_trim(data).is_empty() {
            return Err(format!("FM文件读取失败或内容为空: {name}")); // Java: 静默 valid=false
        }
        // 防御加固: 用户误喂 JSON 文件 (拖错文件/version 文件等) 时优雅判无效。
        // Dagor .blk 格式不可能以 '{' 开头, JSON 对象一定以 '{' 开头, 以此快速识别
        if java_trim(data).starts_with('{') {
            // Java: Logger.warn("Blkx", "JSON 格式文件误作 FM 加载, 标记无效: " + name)
            return Err(format!("JSON 格式文件误作 FM 加载, 标记无效: {name}"));
        }
        // Java L1708-1721: if (doLoad) { try { this.getload(); valid = true; } catch ... }
        // else { valid = true; }
        // TODO(port): getload (L855-1590) 属 reader.rs 后续波次 (mod.rs 方法波次边界注),
        // 落地后在此接入其 try/catch→valid 语义; 当前按 doLoad=false 分支置位。
        b.valid = true;
        Ok(b)
    }

    /// 对应 Java `public String getArray(String label)` (L1764-1804) —
    /// 点分标签逐段 cut 后, 收集**所有**匹配行 (含行尾 '\n') 拼接; 无匹配返回 ""。
    #[allow(dead_code)] // 调用方 getplotdata (L1629) 属 reader.rs 后续波次
    pub fn get_array(&self, label: &str) -> String {
        let mut value = String::new();
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
        // Application.debugPrint(text);
        // 第二步获得值
        // PORT: Java label 形参重赋 label.substring(clsbix) ↔ 变量遮蔽;
        // toUpperCase 每轮对剩余全文重算 (Java 同, O(匹配数×剩余长度) 保真保留)
        let mut bix = text.to_uppercase().find(&label.to_uppercase());
        while let Some(mut bi) = bix {
            // 防御加固: 加长度上界——label 匹配处之后到文本末尾都没有 '=' (如匹配到块名/注释/
            // 截断行) 时, 原代码会扫出末尾抛 StringIndexOutOfBoundsException; 越界时放弃剩余
            // 匹配, 返回已积累的 value (与"未找到"时返回空串的语义一致)
            while bi < text.len() && text.as_bytes()[bi] != b'=' {
                bi += 1;
            }
            if bi >= text.len() {
                break;
            }
            bi += 1;
            let mut eix = bi;
            // 防御加固: 末尾无换行符 (init() 直喂的截断文本) 时取到文本末尾, 原代码此处越界
            while eix < text.len() && text.as_bytes()[eix] != b'\n' {
                eix += 1;
            }
            if eix >= text.len() {
                // Java: value = value + text.substring(bix);
                value.push_str(text.get(bi..).unwrap_or(""));
                break;
            }
            // Java: value = value + text.substring(bix, eix + 1); (含行尾 '\n')
            value.push_str(text.get(bi..eix + 1).unwrap_or(""));
            // PORT: Java text = text.substring(eix + 1) 共享底层 ↔ drain 原地移除前缀
            text.drain(..eix + 1);
            bix = text.to_uppercase().find(&label.to_uppercase());
        }
        value
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
        // Java: bix = text.toUpperCase().lastIndexOf(label.toUpperCase());
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
        // Java: value = text.substring(bix, eix); (不含 '\n' — 与 getArray 的 eix+1 相对)
        Some(text.get(bix..eix).unwrap_or("").to_string())
    }

    /// 对应 Java `public String getoneinData(String D, String label)` (L1839-1871) —
    /// 同 getone 但数据源为显式传入的 D (子块文本); 未找到返回哨兵串 "null"。
    #[allow(dead_code)] // 调用方 getload L444/L1073 属 reader.rs 后续波次
    pub fn getonein_data(&self, d: &str, label: &str) -> String {
        let mut text: String = d.to_string();
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
        // Application.debugPrint(label);
        // 第二步获得值
        let mut bix = match text.to_uppercase().find(&label.to_uppercase()) {
            Some(i) => i,
            // Java: if (bix == -1) return "null";
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
        // 防御加固: 行尾无换行时取到文本末尾, 不再扫越界
        while eix < text.len() && text.as_bytes()[eix] != b'\n' {
            eix += 1;
        }
        text.get(bix..eix).unwrap_or("").to_string()
    }

    /// 对应 Java `public String getone(String label)` (L1873-1906) —
    /// 同 getoneinData 但数据源为 self.data, 且定位是**大小写敏感** indexOf
    /// (toUpperCase 版本在源码里已被注释掉, 见下); 未找到返回哨兵串 "null"。
    #[allow(dead_code)] // 调用方 getload (L527+ 多处) 属 reader.rs 后续波次
    pub fn getone(&self, label: &str) -> String {
        // PORT: Java `String text = data` 为 null 时在 indexOf 处 NPE ↔ unwrap panic
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
        // Application.debugPrint(text);
        // 第二步获得值
        // bix = text.toUpperCase().indexOf(label.toUpperCase());
        // Java: bix = text.indexOf(label); — 大小写敏感 (与 getArray/getlastone/
        // getoneinData 的 toUpperCase 定位不同, 源码本意, oracle go_o2 钉死)
        let mut bix = match text.find(label) {
            Some(i) => i,
            // Java: if (bix == -1) return "null";
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
        // 防御加固: 行尾无换行时取到文本末尾, 不再扫越界
        while eix < text.len() && text.as_bytes()[eix] != b'\n' {
            eix += 1;
        }
        text.get(bix..eix).unwrap_or("").to_string()
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
    // Java: int bix = tmp.toUpperCase().indexOf(clslabel.toUpperCase() + " {");
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
    // Java: for (i = bix; i < tmp.length(); i++) { ... if (left != 0 && right != 0
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
    // Java: tmp.substring(cutleft, cutright); — 未闭合时 i==len, 返回余段 (oracle cut_c3)
    tmp.get(cutleft..cutright).unwrap_or("null").to_string()
}

// =====================================================================
// Tests — 期望值来自 Java 8 oracle 对拍 (§5.1): Blkx.java L1665-1906 逐字提取
// (sed 直提 + 构造器改名, 见 build/oracle/blkx_reader/src/parser/BlxReaderOracle.java)
// 在 OpenJDK 1.8.0_342 (与 bin/ 现役 class 同版) 实测 dump; 字符串以 \n/\r/\t
// 转义单行化后逐字断言。真机文件三份 (spitfire_f24 物理/中央 + bf-109e-4 物理)
// od/grep 实测纯 ASCII 无 CR — Java UTF-16 长度/码元和 ↔ Rust 字节长度/字节和等价。
// =====================================================================
#[cfg(test)]
mod tests {
    // PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
    // 不改成 struct 字面量以保持与 Java 测试源逐行对应
    #![allow(clippy::field_reassign_with_default)]

    use super::*;

    fn charsum(s: &str) -> u64 {
        s.bytes().map(|b| b as u64).sum()
    }

    /// 项目内真机 FM 数据根 (cargo 测试 cwd = crate 根, data/ 缺失自动跳过, 对齐
    /// build.py test 语义 — D4 验收注)
    fn fm_root() -> String {
        format!(
            "{}/../../../data/aces/gamedata/flightmodels",
            env!("CARGO_MANIFEST_DIR")
        )
    }

    // ---- oracle: cut_c1~c8 — 块提取/嵌套/大小写/未闭合/无空格块头 ----
    #[test]
    fn java8_oracle_cut() {
        assert_eq!(
            cut("unit {\n\tWing {\n\t\tsweep:r = 25\n\t}\n}\n", "Wing"),
            "\n\t\tsweep:r = 25\n\t",
            "c1 嵌套块"
        );
        assert_eq!(cut("unit {\n}\n", "Wing"), "null", "c2 未找到");
        assert_eq!(cut("a { b { c", "a"), " b { c", "c3 未闭合返回余段");
        assert_eq!(cut("MODS { q }", "mods"), " q ", "c4 大小写不敏感");
        assert_eq!(cut("mods{q}", "mods"), "null", "c5 成员版无无空格回退");
        assert_eq!(cut("x { i { y } t }", "x"), " i { y } t ", "c6 嵌套平衡");
        assert_eq!(cut("a { b } c }", "a"), " b ", "c7 首个配对即止");
        assert_eq!(cut("", "a"), "null", "c8 空文本");
    }

    // ---- oracle: ga_g1~g7 — 多行累积/末行无换行/未找到/无等号/点分路径/大小写 ----
    #[test]
    fn java8_oracle_get_array() {
        let mut b = Blkx::default();
        b.data = Some("t1 {\n k:r = 1\n k:r = 2\n}\n".to_string());
        assert_eq!(b.get_array("t1.k"), " 1\n 2\n", "g1 多行含行尾\\n");
        b.data = Some("t1 {\n k:r = 1\n k:r = 2".to_string());
        assert_eq!(b.get_array("t1.k"), " 1\n 2", "g2 末行无换行不带\\n");
        b.data = Some("nothing here".to_string());
        assert_eq!(b.get_array("t1.k"), "", "g3 未找到");
        b.data = Some("t1 {\n k noeq\n}\n".to_string());
        assert_eq!(b.get_array("t1.k"), "", "g4 匹配但无等号");
        b.data = Some("A {\n B {\n  v:r = 7\n }\n}\n".to_string());
        assert_eq!(b.get_array("A.B.v"), " 7\n", "g5 两级点分");
        b.data = Some("t1 {\n K:r = 9\n}\n".to_string());
        assert_eq!(b.get_array("t1.k"), " 9\n", "g6 toUpperCase 定位");
        b.data = Some("k:t = \"x\"\nk:t = \"y\"\n".to_string());
        assert_eq!(b.get_array("k"), " \"x\"\n \"y\"\n", "g7 顶层多值");
    }

    // ---- oracle: glo_l1~l7 — 末次匹配/None 语义/无换行取到末尾/大小写 ----
    #[test]
    fn java8_oracle_getlastone() {
        let mut b = Blkx::default();
        b.data = Some("fmFile:t = \"fm/spitfire_f24.blk\"\n".to_string());
        assert_eq!(
            b.getlastone("fmfile"),
            Some(" \"fm/spitfire_f24.blk\"".to_string()),
            "l1 大小写不敏感, 值含前导空格与引号 (FMLoader L87 自行去壳)"
        );
        b.data = Some("a {\n b:r = 1\n}\nc:r = 2\n".to_string());
        assert_eq!(b.getlastone("c"), Some(" 2".to_string()), "l2");
        assert_eq!(b.getlastone("zzz"), None, "l3 未找到→null");
        b.data = Some("k no eq sign".to_string());
        assert_eq!(b.getlastone("k"), None, "l4 无等号→null");
        b.data = Some("k:r = 5".to_string());
        assert_eq!(b.getlastone("k"), Some(" 5".to_string()), "l5 无换行取到末尾");
        b.data = Some("K:r = 1\nk:r = 2\n".to_string());
        assert_eq!(b.getlastone("k"), Some(" 2".to_string()), "l6 rfind 取末次");
        b.data = Some("A {\n B {\n  v:r = 7\n }\n}\n".to_string());
        assert_eq!(b.getlastone("A.B.v"), Some(" 7".to_string()), "l7 点分+cut");
    }

    // ---- oracle: god_d1~d5 — 显式数据源定位/哨兵/点分/大小写 ----
    #[test]
    fn java8_oracle_getonein_data() {
        let b = Blkx::default();
        assert_eq!(b.getonein_data("blk {\n key:r = 5\n}\n", "key"), " 5", "d1");
        assert_eq!(b.getonein_data("blk {\n key:r = 5\n}\n", "zzz"), "null", "d2");
        assert_eq!(b.getonein_data("A {\n B {\n  v:r = 7\n }\n}\n", "A.B.v"), " 7", "d3");
        assert_eq!(b.getonein_data("k no eq\n", "k"), "null", "d4 无等号");
        assert_eq!(b.getonein_data("K:r = 1\n", "k"), " 1", "d5 大小写不敏感");
    }

    // ---- oracle: go_o1~o6 — getone 大小写敏感是源码本意 (toUpperCase 行已注释) ----
    #[test]
    fn java8_oracle_getone() {
        let mut b = Blkx::default();
        b.data = Some("Wingspan:r = 11.3\n".to_string());
        assert_eq!(b.getone("Wingspan"), " 11.3", "o1");
        assert_eq!(b.getone("wingspan"), "null", "o2 大小写敏感");
        b.data = Some("A {\n B {\n  v:r = 7\n }\n}\n".to_string());
        assert_eq!(b.getone("A.B.v"), " 7", "o3");
        assert_eq!(b.getone("A.b.v"), " 7", "o4 cut 环节大小写不敏感");
        b.data = Some("k:r = 5".to_string());
        assert_eq!(b.getone("k"), " 5", "o5 无换行");
        b.data = Some("k no eq\n".to_string());
        assert_eq!(b.getone("k"), "null", "o6 无等号");
    }

    // ---- oracle: ctor_n2~n5/n7/n8 — 构造器守卫 (parse_str 承接, content 即读入后 data) ----
    #[test]
    fn java8_oracle_parse_str_guards() {
        // n2/n3: 空与纯空白 → valid=false
        assert!(Blkx::parse_str("empty.blk", "").is_err(), "n2 空");
        assert!(Blkx::parse_str("ws.blk", "   \n\t \n").is_err(), "n3 纯空白");
        // n4: JSON 误喂 → valid=false (Err 文本对齐 Java warn 日志)
        let e = Blkx::parse_str("json.blk", "{\n  \"a\": 1\n}\n").unwrap_err();
        assert!(e.contains("JSON 格式文件误作 FM 加载"), "n4 Err 文本: {e}");
        // n5: 正常文件 → valid=true, data/readFileName/fmdata 齐
        let b = Blkx::parse_str("good.blk", "unit {\n\tWing {\n\t\tsweep:r = 25\n\t}\n}\n").unwrap();
        assert!(b.valid, "n5 valid");
        assert_eq!(b.data.as_deref(), Some("unit {\n\tWing {\n\t\tsweep:r = 25\n\t}\n}\n"));
        assert_eq!(b.read_file_name.as_deref(), Some("good.blk"));
        assert_eq!(b.fmdata.as_deref(), Some("找不到blkx文件\n请使用最新WT拆包aces.vromfs.bin"));
        // n7: Java trim 只剥 <= U+0020 — NBSP 保留 → 非空且非 JSON → valid=true
        assert!(Blkx::parse_str("nbsp.blk", "\u{00A0}{x\n").is_ok(), "n7 NBSP 保真");
        // n8: 前导 ASCII 空白后的 JSON → trim 后以 { 开头 → valid=false
        assert!(Blkx::parse_str("wsjson.blk", "  {\n\"a\": 1\n}\n").is_err(), "n8");
    }

    /// oracle: ctor_n1/n6 — 文件路径构造 (missing / 无行尾换行补 \n)
    #[test]
    fn parse_missing_and_readline_join() {
        // n1: 文件不存在 → Err (Java 静默 valid=false)
        let missing = format!("{}/blkx_test_missing_{}.blk", std::env::temp_dir().display(), line!());
        assert!(Blkx::parse(&missing).is_err(), "n1 不存在");

        // n6: 文件末行无换行 → readLine 语义补行尾 \n (oracle: "a:r = 1" → len8:sum453)
        let p = std::env::temp_dir().join(format!("blkx_test_nonl_{}.blk", line!()));
        std::fs::write(&p, b"a:r = 1").unwrap();
        let b = Blkx::parse(p.to_str().unwrap()).unwrap();
        assert!(b.valid, "n6 valid");
        assert_eq!(b.data.as_deref(), Some("a:r = 1\n"), "readLine 补 \\n");
        assert_eq!(b.data.as_deref().unwrap().len(), 8);
        assert_eq!(charsum(b.data.as_deref().unwrap()), 453, "oracle sum");
        assert_eq!(
            b.read_file_name.as_deref(),
            Some(p.file_name().unwrap().to_str().unwrap()),
            "单参入口 name 取文件名分量"
        );
        std::fs::remove_file(&p).ok();

        // n4 等价: JSON 文件走 parse → Err
        let pj = std::env::temp_dir().join(format!("blkx_test_json_{}.blk", line!()));
        std::fs::write(&pj, b"{\n  \"a\": 1\n}\n").unwrap();
        assert!(Blkx::parse(pj.to_str().unwrap()).is_err(), "JSON 误喂");
        std::fs::remove_file(&pj).ok();
    }

    /// oracle: real_* — 真机三文件全链路 (构造器 + 原语), data/ 缺失自动跳过
    #[test]
    fn parse_real_fm_files() {
        let root = fm_root();
        let phys_path = format!("{root}/fm/spitfire_f24.blkx");
        if !std::path::Path::new(&phys_path).exists() {
            return; // data/ 未解包 (D4: 对齐 build.py 跳过语义)
        }

        // 物理 FM: ctor_real_phys (len18559:sum1376835) + getone (real_go/go2)
        let phys = Blkx::parse(&phys_path).unwrap();
        assert!(phys.valid, "物理文件 valid");
        let data = phys.data.as_deref().unwrap();
        assert_eq!(data.len(), 18559, "real_phys len");
        assert_eq!(charsum(data), 1376835, "real_phys charsum");
        assert!(data.starts_with("AileronEffectiveSpeed:r = 482\nRudderEffe"), "real_phys head");
        assert!(data.ends_with(" {\n\n\t}\n\tIAS {\n\n\t}\n}\n"), "real_phys tail (readLine 补 \\n)");
        assert_eq!(phys.getone("Wingspan"), " 11.3", "real_go");
        assert_eq!(phys.getone("WingTaperRatio"), " 2", "real_go2");
        // real_ga_empty: spitfire 的 PASSPORT.ALT 为空块 → getArray 空串
        assert_eq!(phys.get_array("PASSPORT.ALT.minClimbTimeWep"), "", "real_ga_empty");

        // 中央文件: ctor_real_central (len46687:sum3233530) + getlastone("fmfile")
        let central_path = format!("{root}/spitfire_f24.blkx");
        let central = Blkx::parse(&central_path).unwrap();
        assert!(central.valid, "中央文件 valid");
        let cdata = central.data.as_deref().unwrap();
        assert_eq!(cdata.len(), 46687, "real_central len");
        assert_eq!(charsum(cdata), 3233530, "real_central charsum");
        assert!(cdata.starts_with("model:t = \"spitfire_f22\"\nfmFile:t = \"fm/"), "real_central head");
        assert_eq!(
            central.getlastone("fmfile"),
            Some(" \"fm/spitfire_f24.blk\"".to_string()),
            "real_glo (FMLoader L84 同款调用)"
        );

        // bf-109e-4: PASSPORT 曲线全链路 (cut×2 + 多行累积, real_ga_bf/bf2)
        let bf = Blkx::parse(&format!("{root}/fm/bf-109e-4.blkx")).unwrap();
        assert!(bf.valid);
        let ga = bf.get_array("PASSPORT.ALT.minClimbTimeWep");
        assert_eq!(ga.len(), 32, "real_ga_bf len");
        assert_eq!(charsum(&ga), 1342, "real_ga_bf sum");
        assert_eq!(ga, " 0, 0\n 1000, 137.4\n 2000, 271.4\n", "real_ga_bf 全文");
        assert_eq!(bf.get_array("PASSPORT.ALT.maxSpeedNom").len(), 95, "real_ga_bf2");
    }

    /// NPE 保真: 未 init/load 的对象 (data=None) 上调用原语 → Java NPE ↔ panic (§1)
    #[test]
    #[should_panic]
    fn getone_on_null_data_panics_like_npe() {
        let b = Blkx::default();
        let _ = b.getone("k");
    }

    #[test]
    #[should_panic]
    fn get_array_on_null_data_panics_like_npe() {
        let b = Blkx::default();
        let _ = b.get_array("k");
    }

    #[test]
    #[should_panic]
    fn getlastone_on_null_data_panics_like_npe() {
        let b = Blkx::default();
        let _ = b.getlastone("k").unwrap();
    }

    /// java_trim 与 Java String.trim 同语义 (<= U+0020, 不含 NBSP)
    #[test]
    fn java_trim_matches_java_semantics() {
        assert_eq!(java_trim("  a\n\t"), "a");
        assert_eq!(java_trim("\u{0001}\u{0020}x\u{001F}"), "x");
        assert_eq!(java_trim("\u{00A0}{x"), "\u{00A0}{x", "NBSP 不剥 (Rust trim 会)");
        assert_eq!(java_trim(""), "");
        assert_eq!(java_trim("   "), "");
    }
}
