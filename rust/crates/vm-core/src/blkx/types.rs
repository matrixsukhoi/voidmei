//! 对应 Java: `src/parser/Blkx.java` L34-660 的内部类区 (D4 拆分: types.rs)。
//! 覆盖 5 个内部类 + Fuel Modification Support 静态函数区:
//! - `FuelModification` (static 内部类, L34-49) + `extractFuelModifications`/
//!   `cutStatic`/`getDoubleFromBlock`/`getBoolFromBlock` (static, L63-218)
//! - `XY` (L222-232) / `engineLoad` (L246-253) / `fm_parts` (L329-350) /
//!   `SweepLevel` (L356-362) — 非静态内部类, 均未引用 Blkx.this 外部状态
//!   (纯语法糖) → 独立 struct, 无需父引用参数 (§1 内部类规则逐个审视结论)。
//!
//! PORT: 本波不含 Blkx 字段区/方法 (L234-241/L255-326/L364-660 的 public 字段与
//! getPartsFm 等) — 属 model.rs/mod.rs 字段波次, 见 mod.rs 骨架注释。

use std::fmt;

// ==================== Fuel Modification Support ====================

/// 对应 Java `public static class FuelModification` (L34-49)。
/// Represents fuel quality modifications extracted from Central file's
/// "modifications" section. These upgrades affect engine power output.
///
/// <p>War Thunder Central files (e.g., flightmodels/yak-3.blkx) contain:
/// <pre>
/// modifications {
///   ussr_fuel_b-100 {
///     effects {
///       addHorsePowers:r = 50
///     }
///   }
/// }
/// </pre>
// PORT: 刻意不 derive PartialEq — Java 无 equals 覆写, 语义只有引用同一性,
// 全库使用点 (FMLoader/FMPowerExtractor/PowerCurveWindow) 均只比较 type 枚举
// 与读数值字段, 从不比较 FuelModification 整体 (FMHandle 同款先例)。
#[derive(Debug, Clone)]
pub struct FuelModification {
    /// Soviet fuel addHorsePowers value (typically 50)
    pub soviet_octane_hp_bonus: f64,
    /// British afterburnerMult from fuel modification
    pub british_afterburner_mult: f64,
    /// British afterburnerCompressorMult from fuel modification
    pub british_afterburner_compressor_mult: f64,
    /// Whether British fuel has invertEnableLogic (means high octane is default)
    pub british_invert_logic: bool,
    /// Fuel modification type
    // PORT: Java 字段名 `type` 是 Rust 关键字 → `r#type` (gauge_marker.rs 先例)
    pub r#type: FuelType,
}

/// 对应 Java 字段初始化器 (`= 0` / `= 1.0` / `= false` / `= FuelType.NONE`)。
impl Default for FuelModification {
    fn default() -> Self {
        FuelModification {
            soviet_octane_hp_bonus: 0.0,
            british_afterburner_mult: 1.0,
            british_afterburner_compressor_mult: 1.0,
            british_invert_logic: false,
            r#type: FuelType::None,
        }
    }
}

impl FuelModification {
    /// 对应 Java `new FuelModification()` — 无参构造, 全字段取初始化器值。
    pub fn new() -> Self {
        Self::default()
    }
}

/// 对应 Java `public enum FuelModification.FuelType` (L46-48)。
// PORT: Java 枚举常量全大写 → Rust 驼峰 (FMStatus 先例); Java 枚举默认
// toString()=常量名 的字符串形态由 Display 保留 (FMLoader 日志
// "Fuel modification detected: " + fuelMod.type 依赖, Java 8 oracle 对拍)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuelType {
    None,
    SovietB95,
    SovietB100,
    British150Octane,
    British100Spitfire,
}

/// 对应 Java 枚举默认 `toString()` = 常量名 (`name()`)。
impl fmt::Display for FuelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FuelType::None => "NONE",
            FuelType::SovietB95 => "SOVIET_B95",
            FuelType::SovietB100 => "SOVIET_B100",
            FuelType::British150Octane => "BRITISH_150_OCTANE",
            FuelType::British100Spitfire => "BRITISH_100_SPITFIRE",
        };
        f.write_str(s)
    }
}

/// 对应 Java `public static FuelModification extractFuelModifications(String centralData)` (L63-127)。
///
/// Extracts fuel quality modifications from a Central file's raw text data.
///
/// <p>Searches for known fuel upgrade keys in the "modifications" section:
/// <ul>
///   <li>Soviet: ussr_fuel_b-95, ussr_fuel_b-100 → addHorsePowers</li>
///   <li>British: 150_octan_fuel, 100_octan_spitfire → afterburnerMult</li>
/// </ul>
///
/// @param centralData raw text content of the Central file
/// @return FuelModification with extracted values, or default (no-op) if none found
// PORT: Java 入参 String 可为 null, 但两处调用方均先行判空 (FMLoader:
// `lookupBlkx.valid && lookupBlkx.data != null`; PowerCurveWindow 读文件失败
// 早退) — Rust 以 &str 收窄为非空语义, null 分支由调用方 Option 层处理,
/// 行为等价 (Java 的 null 与空串同样落到默认返回)。
pub fn extract_fuel_modifications(central_data: &str) -> FuelModification {
    // PORT: Java 局部变量名 `mod` 是 Rust 关键字 → `mod_`
    let mut mod_ = FuelModification::new();
    if central_data.is_empty() {
        return mod_;
    }

    // Find the "modifications" block
    let mods_block = cut_static(central_data, "modifications");
    if mods_block == "null" {
        return mod_;
    }

    // Check for Soviet fuels (ussr_fuel_b-95 or ussr_fuel_b-100)
    let soviet_b100 = cut_static(&mods_block, "ussr_fuel_b-100");
    if soviet_b100 != "null" {
        mod_.r#type = FuelType::SovietB100;
        let effects = cut_static(&soviet_b100, "effects");
        if effects != "null" {
            mod_.soviet_octane_hp_bonus = get_double_from_block(&effects, "addHorsePowers");
        }
        return mod_;
    }

    let soviet_b95 = cut_static(&mods_block, "ussr_fuel_b-95");
    if soviet_b95 != "null" {
        mod_.r#type = FuelType::SovietB95;
        let effects = cut_static(&soviet_b95, "effects");
        if effects != "null" {
            mod_.soviet_octane_hp_bonus = get_double_from_block(&effects, "addHorsePowers");
        }
        return mod_;
    }

    // Check for British fuels (150_octan_fuel or 100_octan_spitfire)
    let british150 = cut_static(&mods_block, "150_octan_fuel");
    if british150 != "null" {
        mod_.r#type = FuelType::British150Octane;
        let effects = cut_static(&british150, "effects");
        if effects != "null" {
            mod_.british_afterburner_mult = get_double_from_block(&effects, "afterburnerMult");
            if mod_.british_afterburner_mult == 0.0 {
                mod_.british_afterburner_mult = 1.0;
            }
            mod_.british_afterburner_compressor_mult =
                get_double_from_block(&effects, "afterburnerCompressorMult");
            if mod_.british_afterburner_compressor_mult == 0.0 {
                mod_.british_afterburner_compressor_mult = 1.0;
            }
        }
        // Check for invertEnableLogic - parse actual boolean value
        mod_.british_invert_logic = get_bool_from_block(&british150, "invertEnableLogic");
        return mod_;
    }

    let british100 = cut_static(&mods_block, "100_octan_spitfire");
    if british100 != "null" {
        mod_.r#type = FuelType::British100Spitfire;
        let effects = cut_static(&british100, "effects");
        if effects != "null" {
            mod_.british_afterburner_mult = get_double_from_block(&effects, "afterburnerMult");
            if mod_.british_afterburner_mult == 0.0 {
                mod_.british_afterburner_mult = 1.0;
            }
            mod_.british_afterburner_compressor_mult =
                get_double_from_block(&effects, "afterburnerCompressorMult");
            if mod_.british_afterburner_compressor_mult == 0.0 {
                mod_.british_afterburner_compressor_mult = 1.0;
            }
        }
        mod_.british_invert_logic = get_bool_from_block(&british100, "invertEnableLogic");
        return mod_;
    }

    mod_
}

/// 对应 Java `private static String cutStatic(String text, String blockLabel)` (L133-155)。
/// Static version of cut() for use in extractFuelModifications().
/// Extracts content between braces of a named block.
// PORT: Java `text.toUpperCase()` 用默认 locale, Rust `to_uppercase()` 固定全映射 —
// 非 Turkish locale 下逐字符一致; Turkish locale 的 'i'→'İ' 漂移属 Java 端
// 历史怪癖 (会令 "modifications" 匹配失败), 不复刻。
// PORT: Java 在大写副本上 indexOf 定位、在原串上 charAt/substring 取段 — 两串
// 长度可能不同 (全映射 ß→SS), Java 以循环边界 (< text.length()) 与 i 越界判断
// 兜住漂移 (parser/CLAUDE.md 防御规则)。Rust 同构: 字节索引在 ASCII 域
// (FM/中央文件键名) 与 Java UTF-16 索引一致; 定界符 '{'/'}' 为 ASCII, 逐字节
// 比较不会误判多字节字符 (UTF-8 自同步, string_helper.rs 先例); 终点取子串经
// get() 边界守卫, 病态非 ASCII 漂移按"未找到"("null")收敛, 对齐 Java 守卫路径。
// PORT: 病态大写变长输入下双方错位扫描的**结果**可能不同 — Java 在 UTF-16 码元
// 域量 bix (ß 1→2 unit 漂移+1), Rust 在 UTF-8 字节域 (ß 2→2 byte 不漂移, 反而
// 更对齐); Java 可产出错位子串继续解析, Rust 统一 get() 收敛 "null"。两侧均不
// panic, ASCII 域完全一致。reader.rs 波次移植成员版 cut() 时同理由适用。
fn cut_static(text: &str, block_label: &str) -> String {
    let upper = text.to_uppercase(); // Java: text.toUpperCase() (两次调用, 结果确定性相同, 合并计算)
    let bix = upper.find(&(block_label.to_uppercase() + " {"));
    // Also try without space: "blockLabel{"
    let bix = bix.or_else(|| upper.find(&(block_label.to_uppercase() + "{")));
    let bix = match bix {
        Some(i) => i,
        None => return "null".to_string(),
    };

    let mut cutleft = bix;
    while cutleft < text.len() && text.as_bytes()[cutleft] != b'{' {
        cutleft += 1;
    }
    if cutleft >= text.len() {
        return "null".to_string();
    }
    cutleft += 1;

    let mut left = 1;
    let mut right = 0;
    let mut i = cutleft;
    while i < text.len() {
        if text.as_bytes()[i] == b'{' {
            left += 1;
        }
        if text.as_bytes()[i] == b'}' {
            right += 1;
        }
        if left == right {
            break;
        }
        i += 1;
    }
    if i >= text.len() {
        return "null".to_string();
    }
    // Java: text.substring(cutleft, i)
    text.get(cutleft..i).unwrap_or("null").to_string()
}

/// 对应 Java `private static double getDoubleFromBlock(String block, String key)` (L161-186)。
/// Extracts a double value from a text block by key name.
/// Handles both "key = value" and "key:r = value" formats.
fn get_double_from_block(block: &str, key: &str) -> f64 {
    // Try key:r = value first (typed format)
    let typed = format!("{key}:r");
    let idx = block.find(&typed).or_else(|| block.find(key));
    let idx = match idx {
        Some(i) => i,
        None => return 0.0,
    };

    // Find the '=' sign
    let eq_idx = match block[idx..].find('=') {
        Some(i) => idx + i,
        None => return 0.0,
    };
    let start = eq_idx + 1; // skip '=' (Java: eqIdx++)

    // Find end of value (newline or end of string)
    let end_idx = block[start..].find('\n').map_or(block.len(), |i| start + i);

    let val_str = block[start..end_idx].trim();
    // Take first part if comma-separated
    // PORT: Java split(",") 丢弃尾部空段 — 此处仅取 parts[0], 行为一致
    let first = val_str.split(',').next().unwrap_or("").trim();
    // PORT: Java try { Double.parseDouble } catch (NumberFormatException) { return 0 }
    // → parse().unwrap_or(0.0) (§2.15); 语法域残余差异: Java 额外接受 '1.5d' 后缀
    // 与十六进制浮点, Rust 拒绝 → 0; 反向 Rust 接受任意大小写 'inf'/'nan'
    // (Java 仅 'Infinity'/'NaN')。域内 (FM 数值) 均不可达。
    // PORT: trim 语义差: Java String.trim 只去 ≤U+0020, Rust str::trim 去 Unicode
    // White_Space (含 NBSP U+00A0) — 值带尾随 NBSP 时 Java 解析失败→0 而 Rust
    // 解析成功。域内数值行全 ASCII 不可达 (CRLF 的 '\r' 两边均正确去除);
    // string_helper.rs 已用同款 Rust trim 近似 Java parseFloat 隐含 trim, 先例一致。
    first.parse::<f64>().unwrap_or(0.0)
}

/// 对应 Java `private static boolean getBoolFromBlock(String block, String key)` (L196-218)。
/// Extracts a boolean value from a text block by key name.
/// Handles "key:b = true/false" format (War Thunder .blkx typed boolean).
///
/// @param block the text block to search within
/// @param key   the key name (e.g., "invertEnableLogic")
/// @return true if the key exists and its value is "true", false otherwise
fn get_bool_from_block(block: &str, key: &str) -> bool {
    // Try key:b = value first (typed boolean format)
    let upper = block.to_uppercase(); // Java: block.toUpperCase() (locale 差异同 cut_static 注)
    let key_typed = format!("{key}:B").to_uppercase();
    let key_plain = key.to_uppercase();

    // PORT: idx 量自大写副本, 可能在原串越界/非字符边界 (大写漂移病态输入) —
    // Java indexOf('=', idx) 越界返回 -1 → false, 此处 get() 越界 None 同路径
    let idx = upper.find(&key_typed).or_else(|| upper.find(&key_plain));
    let idx = match idx {
        Some(i) => i,
        None => return false, // Field absent = false
    };

    // Find the '=' sign after the key
    let eq_idx = match block.get(idx..).and_then(|s| s.find('=')) {
        Some(i) => idx + i,
        None => return false,
    };

    // Find end of value (newline or end of string)
    // Java: block.indexOf('\n', eqIdx) — 自 '=' 位起搜 ('\n' 不可能在 '=' 位, 等价)
    let end_idx = block[eq_idx..]
        .find('\n')
        .map_or(block.len(), |i| eq_idx + i);

    let value = block[eq_idx + 1..end_idx].trim();
    // PORT: Java "true".equalsIgnoreCase(value) 是 Unicode 不区分大小写比较 —
    // 域内值恒为 ASCII "true"/"false", eq_ignore_ascii_case 等价
    // PORT: trim 语义差同 get_double_from_block — 尾随 NBSP 时 Java "true\u{00A0}"
    // ≠ "true" 得 false, Rust trim 掉得 true; 域内不可达, 同款 Rust trim 近似。
    value.eq_ignore_ascii_case("true")
}

// ==================== End Fuel Modification Support ====================

/// 对应 Java `public class XY` (L222-232)。
/// PORT: Java 非静态内部类, 但构造器与字段均未引用 Blkx.this 外部状态
/// (纯语法糖) → 独立 struct, 无父引用参数。
#[derive(Debug, Clone)]
pub struct XY {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub cur: i32,
}

impl XY {
    /// 对应 Java 包私有构造器 `XY(int num)` (L227-231)。
    // PORT: Java int num 为负时 new double[num] 抛 NegativeArraySizeException —
    // usize 在类型层排除负值 (域内 num 恒为非负表格长度);
    // Java 数组零初始化 ↔ vec![0.0; num] (§2.10)。
    pub fn new(num: usize) -> Self {
        XY {
            x: vec![0.0; num],
            y: vec![0.0; num],
            cur: 0,
        }
    }
}

/// 对应 Java `public class engineLoad` (L246-253, 源文件类名即小驼峰)。
/// PORT: 非静态内部类, 六个 double 字段无任何 Blkx 外部引用 → 独立 struct;
/// Java 字段隐式零初始化 → Default 复刻 (§2.10)。类名 engineLoad → EngineLoad (§0.5)。
#[derive(Debug, Clone, Default)]
pub struct EngineLoad {
    pub water_limit: f64,
    pub oil_limit: f64,
    pub work_time: f64,
    pub recover_time: f64,
    pub cur_water_work_time_mili: f64,
    pub cur_oil_work_time_mili: f64,
}

/// 对应 Java `public class fm_parts` (L329-350, 源文件类名即小写下划线)。
/// PORT: 非静态内部类, 未引用 Blkx 外部状态 → 独立 struct;
/// 类名 fm_parts → FmParts (§0.5)。Java 字段隐式零初始化/null → Default;
/// name 未赋值的 null ↔ None (唯一赋值点是 getPartsFm, 后续波次)。
#[derive(Debug, Clone, Default)]
pub struct FmParts {
    pub name: Option<String>,

    pub sq: f64,
    pub cd_min: f64,

    pub cl0: f64,

    pub cl_crit_high: f64,
    pub cl_crit_low: f64,

    pub cl_after_crit: f64,

    pub aoa_crit_high: f64,
    pub aoa_crit_low: f64,

    pub line_cl_coeff: f64,

    // 翼展效率因数，影响诱导阻力，因数越大阻力越小
    // public double oswaldEff;
}

/// 对应 Java `public class SweepLevel` (L356-362)。
/// Represents a single sweep level with its associated aerodynamic data.
/// Used for variable-sweep wing aircraft (e.g., F-14 with 4 sweep levels).
/// PORT: 非静态内部类, 持有的两个 fm_parts 是构造后即赋值的值字段而非
/// Blkx 外部引用 → 独立 struct; noFlaps/fullFlaps 在 `new SweepLevel()` 后、
/// 构造器赋值前为 null ↔ Option<FmParts> + Default (None)。
/// Java 的引用共享 (`NoFlapsWing_V50 = sweepLevels.get(1).noFlaps`) 由后续
/// 字段波次以值克隆承接 — 解析完成后 fm_parts 只读, 无行为差异。
#[derive(Debug, Clone, Default)]
pub struct SweepLevel {
    /// 0.0 ~ 1.0 sweep ratio (from Sweep:r field)
    pub sweep: f64,
    /// VNE limit speed for this sweep level
    pub vne: f64,
    /// MNE limit (Mach) for this sweep level
    pub vne_mach: f64,
    /// No-flaps aerodynamic data
    pub no_flaps: Option<FmParts>,
    /// Full-flaps aerodynamic data
    pub full_flaps: Option<FmParts>,
}

// =====================================================================
// Tests — Java 无内部类独立测试; 公共面按"每个公共项写边界测试"规则补齐,
// 期望值来自 Java 8 oracle 对拍 (§5.1): Blkx.java L34-218 逐字提取为独立
// 可编译类 (该段零外部依赖), OpenJDK 1.8.0_342 实测 dump, 临时文件用完已删。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;

    // ---- Java 8 oracle 对拍: extractFuelModifications 十个输入 ----

    /// oracle: A_empty / B_nomods — 空串与无 modifications 块 → 全默认
    #[test]
    fn java8_oracle_extract_no_fuel() {
        for input in ["", "unit {\n\tfoo {\n\t}\n}\n"] {
            let m = extract_fuel_modifications(input);
            assert_eq!(m.r#type, FuelType::None, "type=NONE ({input:?})");
            assert_eq!(m.soviet_octane_hp_bonus, 0.0);
            assert_eq!(m.british_afterburner_mult, 1.0);
            assert_eq!(m.british_afterburner_compressor_mult, 1.0);
            assert!(!m.british_invert_logic);
        }
    }

    /// oracle: C_b100 / D_b95 / I_b100_bad / J_both — 苏联油料族
    #[test]
    fn java8_oracle_extract_soviet() {
        // yak-3 真实中央文件格式 (data/aces/gamedata/flightmodels/yak-3.blkx L1920-1928)
        let b100 = "modifications {\n\tussr_fuel_b-100 {\n\t\ttier:i = 4\n\t\tmodClass:t = \"lth\"\n\t\tdontDecreaseAirRCost:b = true\n\t\teffects {\n\t\t\taddHorsePowers:r = 50\n\t\t}\n\t}\n}\n";
        let m = extract_fuel_modifications(b100);
        assert_eq!(m.r#type, FuelType::SovietB100, "C_b100 type");
        assert_eq!(m.soviet_octane_hp_bonus, 50.0, "C_b100 hp");
        assert_eq!(m.british_afterburner_mult, 1.0);
        assert!(!m.british_invert_logic);

        let b95 = "modifications {\n\tussr_fuel_b-95 {\n\t\teffects {\n\t\t\taddHorsePowers:r = 30\n\t\t}\n\t}\n}\n";
        let m = extract_fuel_modifications(b95);
        assert_eq!(m.r#type, FuelType::SovietB95, "D_b95 type");
        assert_eq!(m.soviet_octane_hp_bonus, 30.0, "D_b95 hp");

        // 畸形数值 → NumberFormatException catch → 0
        let bad = "modifications {\n\tussr_fuel_b-100 {\n\t\teffects {\n\t\t\taddHorsePowers:r = abc\n\t\t}\n\t}\n}\n";
        let m = extract_fuel_modifications(bad);
        assert_eq!(m.r#type, FuelType::SovietB100, "I_b100_bad type");
        assert_eq!(m.soviet_octane_hp_bonus, 0.0, "I_b100_bad hp=0");

        // b-95 与 b-100 并存 → b-100 优先 (检查顺序)
        let both = "modifications {\n\tussr_fuel_b-95 {\n\t\teffects {\n\t\t\taddHorsePowers:r = 30\n\t\t}\n\t}\n\tussr_fuel_b-100 {\n\t\teffects {\n\t\t\taddHorsePowers:r = 50\n\t\t}\n\t}\n}\n";
        let m = extract_fuel_modifications(both);
        assert_eq!(m.r#type, FuelType::SovietB100, "J_both type");
        assert_eq!(m.soviet_octane_hp_bonus, 50.0, "J_both hp");
    }

    /// oracle: E_b150 / F_b150_zero_inv / G_b100spit / H_b150_noeff — 英国油料族
    #[test]
    fn java8_oracle_extract_british() {
        // spitfire_f24 真实中央文件格式 (datamine: invertEnableLogic:false, 1.42/1.33)
        let b150 = "modifications {\n\tnew_compressor {\n\n\t}\n\t150_octan_fuel {\n\t\tinvertEnableLogic:b = false\n\t\teffects {\n\t\t\tafterburnerMult:r = 1.42\n\t\t\tafterburnerCompressorMult:r = 1.33\n\t\t}\n\t}\n\thispano_universal {\n\n\t}\n}\n";
        let m = extract_fuel_modifications(b150);
        assert_eq!(m.r#type, FuelType::British150Octane, "E_b150 type");
        assert_eq!(m.british_afterburner_mult, 1.42, "E_b150 abm");
        assert_eq!(m.british_afterburner_compressor_mult, 1.33, "E_b150 abcm");
        assert!(!m.british_invert_logic, "E_b150 inv=false");

        // mult 为 0 → 回退 1.0; invertEnableLogic:b = true
        let zero_inv = "modifications {\n\t150_octan_fuel {\n\t\tinvertEnableLogic:b = true\n\t\teffects {\n\t\t\tafterburnerMult:r = 0\n\t\t\tafterburnerCompressorMult:r = 0\n\t\t}\n\t}\n}\n";
        let m = extract_fuel_modifications(zero_inv);
        assert_eq!(m.r#type, FuelType::British150Octane);
        assert_eq!(m.british_afterburner_mult, 1.0, "F abm 0→1.0");
        assert_eq!(m.british_afterburner_compressor_mult, 1.0, "F abcm 0→1.0");
        assert!(m.british_invert_logic, "F inv=true");

        let b100spit = "modifications {\n\t100_octan_spitfire {\n\t\tinvertEnableLogic:b = true\n\t\teffects {\n\t\t\tafterburnerMult:r = 1.1\n\t\t\tafterburnerCompressorMult:r = 1.08\n\t\t}\n\t}\n}\n";
        let m = extract_fuel_modifications(b100spit);
        assert_eq!(m.r#type, FuelType::British100Spitfire, "G type");
        assert_eq!(m.british_afterburner_mult, 1.1);
        assert_eq!(m.british_afterburner_compressor_mult, 1.08);
        assert!(m.british_invert_logic);

        // 无 effects 块 → mult 保持默认 1.0, invertEnableLogic 仍解析 (absent = false)
        let noeff = "modifications {\n\t150_octan_fuel {\n\t}\n}\n";
        let m = extract_fuel_modifications(noeff);
        assert_eq!(m.r#type, FuelType::British150Octane, "H type");
        assert_eq!(m.british_afterburner_mult, 1.0);
        assert_eq!(m.british_afterburner_compressor_mult, 1.0);
        assert!(!m.british_invert_logic, "H inv=false");
    }

    /// oracle: cut|c1~c8 — cutStatic 八个边界 (含大小写不敏感定位/嵌套花括号/未闭合)
    #[test]
    fn java8_oracle_cut_static() {
        assert_eq!(cut_static("a{b}", "a"), "b", "c1");
        assert_eq!(cut_static("a{b}", "zzz"), "null", "c2");
        assert_eq!(cut_static("x { inner { y } tail }", "x"), " inner { y } tail ", "c3");
        assert_eq!(cut_static("modifications{nested { a } }", "modifications"), "nested { a } ", "c4");
        assert_eq!(cut_static("a { b { c", "a"), "null", "c5 未闭合");
        assert_eq!(cut_static("abc", "abc"), "null", "c6 无花括号");
        assert_eq!(cut_static("pre { x } post { y }", "pre"), " x ", "c7 首块即止");
        assert_eq!(cut_static("MODS { q }", "mods"), " q ", "c8 大小写不敏感");
    }

    /// oracle: dbl|d1~d7 — getDoubleFromBlock 七个边界
    #[test]
    fn java8_oracle_get_double_from_block() {
        assert_eq!(get_double_from_block("key:r = 2.5", "key"), 2.5, "d1 typed");
        assert_eq!(get_double_from_block("key = 3.5", "key"), 3.5, "d2 plain");
        assert_eq!(get_double_from_block("key:r = 1.0, 2400", "key"), 1.0, "d3 逗号取首段");
        assert_eq!(get_double_from_block("key no eq", "key"), 0.0, "d4 无等号");
        assert_eq!(get_double_from_block("key:r = xyz", "key"), 0.0, "d5 畸形值");
        assert_eq!(get_double_from_block("key:r = 7", "key"), 7.0, "d6 行尾无换行");
        assert_eq!(get_double_from_block("other:r = 1\nkey:r = 9", "key"), 9.0, "d7 定位后取值");
    }

    /// oracle: bool|b1~b7 — getBoolFromBlock 七个边界
    #[test]
    fn java8_oracle_get_bool_from_block() {
        assert!(get_bool_from_block("k:b = true", "k"), "b1 typed true");
        assert!(get_bool_from_block("k:b = TRUE", "k"), "b2 equalsIgnoreCase");
        assert!(!get_bool_from_block("k:b = false", "k"), "b3");
        assert!(get_bool_from_block("k = true", "k"), "b4 plain key");
        assert!(!get_bool_from_block("nokey", "k"), "b5 键缺失");
        assert!(!get_bool_from_block("k:b = 1", "k"), "b6 非布尔值");
        assert!(!get_bool_from_block("k:b = true", "j"), "b7 他键");
    }

    /// oracle: enum| 五个常量名 — Java 枚举默认 toString()=name()
    #[test]
    fn java8_oracle_fuel_type_display() {
        assert_eq!(FuelType::None.to_string(), "NONE");
        assert_eq!(FuelType::SovietB95.to_string(), "SOVIET_B95");
        assert_eq!(FuelType::SovietB100.to_string(), "SOVIET_B100");
        assert_eq!(FuelType::British150Octane.to_string(), "BRITISH_150_OCTANE");
        assert_eq!(FuelType::British100Spitfire.to_string(), "BRITISH_100_SPITFIRE");
    }

    /// Java 字段初始化器保真: new FuelModification() 的五字段初值
    #[test]
    fn fuel_modification_default_matches_java_initializers() {
        let m = FuelModification::new();
        assert_eq!(m.soviet_octane_hp_bonus, 0.0);
        assert_eq!(m.british_afterburner_mult, 1.0);
        assert_eq!(m.british_afterburner_compressor_mult, 1.0);
        assert!(!m.british_invert_logic);
        assert_eq!(m.r#type, FuelType::None);
        // Default trait 与 new() 同源 (Java 只有一个构造路径)
        assert_eq!(FuelModification::default().british_afterburner_mult, 1.0);
    }

    /// XY::new — 定长零填充数组 + cur=0 (Java 构造器 L227-231)
    #[test]
    fn xy_new_zero_fills_arrays() {
        let xy = XY::new(5);
        assert_eq!(xy.x, vec![0.0; 5]);
        assert_eq!(xy.y, vec![0.0; 5]);
        assert_eq!(xy.cur, 0);
        let empty = XY::new(0);
        assert!(empty.x.is_empty() && empty.y.is_empty(), "num=0 空数组");
    }

    /// EngineLoad — Java 隐式零初始化 (§2.10)
    #[test]
    fn engine_load_default_all_zero() {
        let e = EngineLoad::default();
        assert_eq!(e.water_limit, 0.0);
        assert_eq!(e.oil_limit, 0.0);
        assert_eq!(e.work_time, 0.0);
        assert_eq!(e.recover_time, 0.0);
        assert_eq!(e.cur_water_work_time_mili, 0.0);
        assert_eq!(e.cur_oil_work_time_mili, 0.0);
    }

    /// FmParts — Java 隐式零初始化 + name=null
    #[test]
    fn fm_parts_default_zero_and_unnamed() {
        let p = FmParts::default();
        assert!(p.name.is_none(), "name 未赋值 ≈ Java null");
        assert_eq!(p.sq, 0.0);
        assert_eq!(p.cd_min, 0.0);
        assert_eq!(p.cl0, 0.0);
        assert_eq!(p.cl_crit_high, 0.0);
        assert_eq!(p.cl_crit_low, 0.0);
        assert_eq!(p.cl_after_crit, 0.0);
        assert_eq!(p.aoa_crit_high, 0.0);
        assert_eq!(p.aoa_crit_low, 0.0);
        assert_eq!(p.line_cl_coeff, 0.0);
    }

    /// SweepLevel — Java 隐式零初始化 + noFlaps/fullFlaps=null (构造后赋值前)
    #[test]
    fn sweep_level_default_zero_and_unassigned_parts() {
        let s = SweepLevel::default();
        assert_eq!(s.sweep, 0.0);
        assert_eq!(s.vne, 0.0);
        assert_eq!(s.vne_mach, 0.0);
        assert!(s.no_flaps.is_none(), "noFlaps 未赋值 ≈ Java null");
        assert!(s.full_flaps.is_none(), "fullFlaps 未赋值 ≈ Java null");
    }
}
