//! FM JSON 数据源 — wt_ext_cli `unpack_vromf --format Json --blk_extension json` 产物。
//!
//! JSON 是 blk 树的 1:1 镜像 (wt_blk 库序列化, 无 schema/无版本号/零单位换算):
//! - 嵌套 blk section → 嵌套 object; 空 section → `{}`;
//! - 同名重复键 → merge_fields 折叠为数组 (插入序保持), 如中央文件重复的
//!   `fmfile` → `"fmfile": ["fm/a.blk", "fm/b.blk"]`;
//! - 浮点一律 f32 的 ryu 最短往返表示 (必带小数点, 如 `619.0`);
//! - 键序 = blk 文档序 (serde_json preserve_order, 见 Cargo.toml 注)。
//!
//! # 数值位级对齐 (getdouble 族 = Float.parseFloat 域)
//!
//! get_f64 族数值直读: `Number` → `as_f64` → `as f32` → widen f64。
//! wt_blk 以 ryu 输出 f32 的**最短往返十进制** → serde 解析为最近 f64 →
//! `as f32` 恢复原 f32 — 最短往返串经更细粒度的 f64 中转仍恢复同一最近
//! f32 (double rounding 对 round-trip 最短串安全; 2026-09 对全语料 2832
//! 文件逐数值实测 `串→parse::<f32>` 与 `as_f64→as f32` 双链位级一致)。
//! 整数 (blk `:i` 型) 走精确域; String 数字形态按 parseFloat 保真解析。
//!
//! # 与文本原语的语义对应 (已知松散性差异, 对拍裁决)
//!
//! 文本原语的匹配是**子串**语义: cut 找 `"NAME {"` 的 CI 子串、getone 末段
//! 是 CS 子串、getlastone 是 CI rfind 子串。因此树原语同样做子串匹配
//! (而非精确键名), 忠实复刻而非"修正"—— 块名 `WingPlane {` 会命中 cut("Plane")
//! 这类行为两侧一致。已知不可复刻的差异仅一处: 文本 leaf 搜索命中块名行后
//! 会跨行扫到块内首个 `=` 取值, 树版跳过 object 值继续找真键 (真实语料无此形态)。

use serde_json::Value;

use super::types::{FuelModification, FuelType};

// ==================== 树底层原语 ====================

/// merge 折叠数组的取首规则 — 文本 cut/getone 命中的是首个同名块/行。
fn first_of_merged(v: &Value) -> &Value {
    match v {
        Value::Array(arr) if !arr.is_empty() => &arr[0],
        other => other,
    }
}

/// merge 折叠数组的取末规则 — 文本 getlastone 命中的是最后一个同名行。
fn last_of_merged(v: &Value) -> &Value {
    match v {
        Value::Array(arr) if !arr.is_empty() => &arr[arr.len() - 1],
        other => other,
    }
}

/// section 判据: object, 或**首元素为 object** 的数组 (wt_blk merge_fields 把
/// 同名 section 折叠成的数组)。元素为标量的数组是 p2/p3 多分量**叶子**
/// (`alt0:p2 = ...` 值行, 非 `NAME {` 块) — 文本 cut 要求字面 "NAME {",
/// 值行不是块, 树侧据此区分 (parity 实测: Passport 的 alt0 曲线数组曾被
/// 误判为 section 导致 ALT 寻址落空)。
fn is_section(v: &Value) -> bool {
    match v {
        Value::Object(_) => true,
        Value::Array(arr) => arr
            .first()
            .map(|e| matches!(e, Value::Object(_)))
            .unwrap_or(false),
        _ => false,
    }
}

/// DFS 前序 (文档序) 找 section — 文本 `cut(t, name)` 的 "NAME {" 首个匹配语义。
///
/// 匹配是 CI **后缀** ("NAME {" 要求 NAME 后紧跟 " {" → 键名以 NAME 结尾):
/// "Plane" 命中 "WingPlane" 但 "Fuselage" 不命中 "FuselagePlane" (parity 实测
/// 裁决: a-10a 的 Fuselage/Stab 回退链曾被子串匹配误短路)。值为 section
/// (见 [`is_section`]); 嵌套穿透 (引擎块藏在 Aerodynamics 下也能找到),
/// 对齐文本全文顺序搜索。
pub(crate) fn find_section_ci<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    if let Value::Object(map) = v {
        for (k, val) in map {
            if is_section(val) && k.to_uppercase().ends_with(&name.to_uppercase()) {
                return Some(first_of_merged(val));
            }
            if let Some(found) = find_section_ci(val, name) {
                return Some(found);
            }
        }
    }
    None
}

/// leaf 键名匹配模式 — 文本原语对键名边界的不同要求 (parity 实测裁决):
/// - [`KeyMatch::Contains`]: getone 无类型标记的纯 `find(label)` 子串 (无边界,
///   "Wingspan" 会命中 "MaxWingspan");
/// - [`KeyMatch::Suffix`]: 带 `:` 类型标记的 label ("Vne:"/"Sweep:r") — 冒号
///   紧跟键名 → 键名以剥后 label **结尾** (CS, 同 find 的大小写敏感);
/// - [`KeyMatch::Starts`]: `\n` 前缀 label ("\nWingAngle") — 行首即列 0,
///   键名以 label **开头** (仅根层)。
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum KeyMatch {
    Contains,
    Suffix,
    Starts,
}

fn key_matches(key: &str, label: &str, mode: KeyMatch) -> bool {
    match mode {
        KeyMatch::Contains => key.contains(label),
        KeyMatch::Suffix => key.ends_with(label),
        KeyMatch::Starts => key.starts_with(label),
    }
}

/// 子树 DFS 前序首个 leaf (任意键) — 复刻文本"命中块名行后跨行扫到块内首个
/// `=` 行"的取值行为 (引擎计数 getone("EngineN") 依赖: 块名行无 '=', 原代码
/// 扫进块内首个值行; parity 实测 a-10a 双发被数成单发)。
fn first_leaf_in(v: &Value) -> Option<&Value> {
    match v {
        Value::Object(map) => {
            for (_k, val) in map {
                if !is_section(val) {
                    return Some(val);
                }
                if let Some(found) = first_leaf_in(val) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

/// find_leaf 的模式化实现 (各公开包装的公共骨架)。
fn find_leaf_mode<'a>(v: &'a Value, label: &str, mode: KeyMatch, ci: bool) -> Option<&'a Value> {
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                let hit = if ci {
                    key_matches(&k.to_uppercase(), &label.to_uppercase(), mode)
                } else {
                    key_matches(k, label, mode)
                };
                if hit {
                    if !is_section(val) {
                        return Some(val);
                    }
                    // 键名命中但值是块 → 块名行无 '=', 文本版继续扫到块内首个
                    // '=' 行 (见 first_leaf_in); 空块则继续找下一匹配 (跨块扫描近似)
                    if let Some(first_leaf) = first_leaf_in(val) {
                        return Some(first_leaf);
                    }
                }
                if let Some(found) = find_leaf_mode(val, label, mode, ci) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

/// 子树内 leaf 查找 — 文本 getone 末段 (`text.find(label)`) 的 CS 子串语义。
///
/// DFS 前序第一个「键包含 label 子串 (大小写敏感) 且值为非 section」的键,
/// 返回**原始值** (merge 折叠由消费层按元素形态处理, 见 [`leaf_to_text`])。
/// section 值的键跳过 (块名不是 leaf)。
pub(crate) fn find_leaf_cs<'a>(v: &'a Value, label: &str) -> Option<&'a Value> {
    find_leaf_mode(v, label, KeyMatch::Contains, false)
}

/// 子树内 leaf 查找 (CI 子串, 首个) — getoneinData/getBoolFromBlock 的
/// `toUpperCase find` 定位语义。返回原始值 (同 [`find_leaf_cs`])。
pub(crate) fn find_leaf_ci<'a>(v: &'a Value, label: &str) -> Option<&'a Value> {
    find_leaf_mode(v, label, KeyMatch::Contains, true)
}

/// 子树内 leaf 文档序最后一个 — 文本 getlastone (`CI rfind`) 语义。
///
/// 键 **CI 子串**匹配且值为标量/数组; merge 数组整体算一个条目、值取末元素
/// (数组末元素 ≡ 文本最后出现的同名行)。
pub(crate) fn find_leaf_ci_last<'a>(v: &'a Value, label: &str) -> Option<&'a Value> {
    match v {
        Value::Object(map) => {
            let mut result: Option<&Value> = None;
            for (k, val) in map {
                if !is_section(val) && k.to_uppercase().contains(&label.to_uppercase()) {
                    result = Some(last_of_merged(val));
                }
                // 嵌套内的命中晚于本层, 覆盖 result (文档序最后)
                if let Some(found) = find_leaf_ci_last(val, label) {
                    result = Some(found);
                }
            }
            result
        }
        _ => None,
    }
}

/// CI 全树取文档序最后一个字符串标量 (fm_loader 中央文件分支用, 不经 Blkx)。
/// 返回**无引号**干净串 (JSON 字符串值本无引号; 文本链路的剥引号在 fm_loader)。
pub(crate) fn get_last_string_ci(root: &Value, key: &str) -> Option<String> {
    find_leaf_ci_last(root, key).and_then(value_as_string)
}

/// 标量值 → getone 行值文本形态 (无引号域):
/// String → 原样; Bool → "true"/"false" (BlkText `key:b = true` 行值形态);
/// Number → serde 十进制文本; 其余 (object/array) → None (非 leaf)。
pub(crate) fn value_as_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// JSON 标量 → Java `Float.parseFloat` 域的 f64 (24-bit 尾数, 1.42f != 1.42;
/// 位级论证见模块注)。Number 直读; String 数字形态按文本链 parseFloat 保真;
/// Bool → None (文本链 "true".parse 失败同返 None)。
pub(crate) fn num_f32_domain(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64().map(|d| d as f32 as f64),
        Value::String(s) => s.trim().parse::<f32>().ok().map(|f| f as f64),
        _ => None,
    }
}

/// leaf → **首分量**数值 (f32 域)。等价旧文本化协议 "leaf_to_text → split(',')
/// 首段 → parseFloat" 的取首语义, 免除字符串往返: p2/p3 数组取首元素、
/// 嵌套数组 (merge 曲线) 取首 pair 首元素、标量 merge 数组取首元素。
fn first_number_f32(v: &Value) -> Option<f64> {
    match v {
        Value::Array(arr) => match arr.first()? {
            Value::Array(pair) => pair.first().and_then(num_f32_domain),
            e => num_f32_domain(e),
        },
        e => num_f32_domain(e),
    }
}

// ==================== 中央文件燃油修正 (树版) ====================
// 对应 types.rs extract_fuel_modifications (文本版): 键匹配对齐 cut_static (CI
// 子串) 与 getDoubleFromBlock/getBoolFromBlock; 数值是 Double.parseDouble 域
// (f64 直取, **不**收窄 f32 — 与 getdouble 族的 Float 域不同, types.rs 注)。

/// 从中央文件 JSON 树提取燃油品质改装修正 (ussr_fuel_b-95/b-100 →
/// addHorsePowers; 150_octan_fuel/100_octan_spitfire → afterburnerMult 等)。
pub fn extract_fuel_modifications_json(central: &Value) -> FuelModification {
    let mut mod_ = FuelModification::new();

    // modifications 块 (CI 子串, 对齐 cut_static)
    let mods = match find_section_ci(central, "modifications") {
        Some(m) => m,
        None => return mod_,
    };

    // 苏联油: b-100 优先 (互斥 return, 顺序保真)
    if let Some(b100) = find_section_ci(mods, "ussr_fuel_b-100") {
        mod_.r#type = FuelType::SovietB100;
        if let Some(effects) = find_section_ci(b100, "effects") {
            mod_.soviet_octane_hp_bonus = block_f64(effects, "addHorsePowers");
        }
        return mod_;
    }
    if let Some(b95) = find_section_ci(mods, "ussr_fuel_b-95") {
        mod_.r#type = FuelType::SovietB95;
        if let Some(effects) = find_section_ci(b95, "effects") {
            mod_.soviet_octane_hp_bonus = block_f64(effects, "addHorsePowers");
        }
        return mod_;
    }

    // 英国油: 150 辛烷优先 (互斥 return, 顺序保真)
    if let Some(b150) = find_section_ci(mods, "150_octan_fuel") {
        mod_.r#type = FuelType::British150Octane;
        fill_british(&mut mod_, b150);
        return mod_;
    }
    if let Some(b100) = find_section_ci(mods, "100_octan_spitfire") {
        mod_.r#type = FuelType::British100Spitfire;
        fill_british(&mut mod_, b100);
        return mod_;
    }

    mod_
}

/// 英国油两种变体的公共填充 (0 值回退 1.0 + invertEnableLogic)。
fn fill_british(mod_: &mut FuelModification, fuel: &Value) {
    if let Some(effects) = find_section_ci(fuel, "effects") {
        mod_.british_afterburner_mult = block_f64(effects, "afterburnerMult");
        if mod_.british_afterburner_mult == 0.0 {
            mod_.british_afterburner_mult = 1.0;
        }
        mod_.british_afterburner_compressor_mult = block_f64(effects, "afterburnerCompressorMult");
        if mod_.british_afterburner_compressor_mult == 0.0 {
            mod_.british_afterburner_compressor_mult = 1.0;
        }
    }
    // invertEnableLogic 在 fuel 块级 (非 effects 内), 对齐文本版查找范围
    mod_.british_invert_logic = block_bool(fuel, "invertEnableLogic");
}

/// getDoubleFromBlock 的树版: effects 块内 CS 子串键 → f64 直取 (Double 域),
/// 缺席/非数值 → 0.0。
fn block_f64(effects: &Value, key: &str) -> f64 {
    find_leaf_cs(effects, key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

/// getBoolFromBlock 的树版: 块内 CI 子串键, 值为 Bool 直取; 其他标量按
/// "true" 忽略大小写比较 (对齐文本 equalsIgnoreCase)。
fn block_bool(block: &Value, key: &str) -> bool {
    match find_leaf_ci(block, key).and_then(value_as_string) {
        Some(s) => s.eq_ignore_ascii_case("true"),
        None => false,
    }
}

// ==================== JsonSrc — BlkSource 的 JSON 后端 ====================

/// JSON 后端: 寻址层把点分标签解析成"值文本", 数值解析走 trait 默认方法
/// (与文本后端单源共享)。
pub(crate) struct JsonSrc {
    root: Value,
}

impl JsonSrc {
    /// 解析 JSON 文本 (serde 失败 → Err, 守卫语义见 parse_named_opts_json)。
    pub fn parse_str(content: &str) -> Result<Self, String> {
        let root: Value =
            serde_json::from_str(content).map_err(|e| format!("JSON 解析失败: {e}"))?;
        Ok(JsonSrc { root })
    }
}

/// 末段标签的怪癖整形 — 文本版 label 可携带 BlkText 形态标记, JSON 键是干净的:
/// - `"Vne:"`/`"Sweep:r"` 冒号类型后缀 → 剥到冒号前 + **后缀匹配** (键名以剥后
///   label 结尾 — 冒号紧跟键名, "VneControl" 不可冒充 "Vne");
/// - `"\nWingAngle"` 前导换行 (文本行首=列 0, 嵌套块行必缩进) → **仅根层** +
///   前缀匹配;
/// - `" RPMMax"` 前导空格 (全语料 tab 缩进, 恒不命中) → 原样保留, 键永不存在
///   (位级保真: 与文本版同取 0/未找到);
/// - 其余 → 子串匹配 (无边界, 对齐 find(label))。
fn shape_leaf_label(label: &str) -> (&str, bool, KeyMatch) {
    if let Some(stripped) = label.strip_prefix('\n') {
        return (stripped, true, KeyMatch::Starts);
    }
    if let Some(pos) = label.find(':') {
        return (&label[..pos], false, KeyMatch::Suffix);
    }
    (label, false, KeyMatch::Contains)
}

/// 标量/数组值 → getone **首行**值文本形态 (文本化协议, 见 BlkSource trait 注)。
/// 数组三形态按元素类型区分 (wt_blk 序列化契约, parity 实测裁决):
/// - 元素为 Number → blk p2/p3 多分量**值行** → join(", ") (≡ 文本 `k:p2 = a, b`);
/// - 元素为 Array → 同名 p2 **merge 多行** (PASSPORT 曲线) → 首行 = 首 pair join;
/// - 元素为 String/Bool → 同名标量** merge 多行** (fmFile/gyroSight) → 首元素。
fn leaf_to_text(v: &Value) -> Option<String> {
    match v {
        Value::Array(arr) if !arr.is_empty() => match &arr[0] {
            Value::Array(pair) => Some(
                pair.iter()
                    .map(numberish_to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            Value::String(s) => Some(s.clone()),
            Value::Bool(b) => Some(b.to_string()),
            _ => Some(
                arr.iter()
                    .map(numberish_to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        },
        Value::Array(_) => Some(String::new()),
        other => value_as_string(other),
    }
}

/// 数组元素只可能是标量 (p2/p3 多分量值), 取其文本形态。
fn numberish_to_string(v: &Value) -> String {
    value_as_string(v).unwrap_or_default()
}

impl JsonSrc {
    /// 点分标签的 section 链走位: 前缀段逐段 find_section_ci 下钻,
    /// 返回 (终节点, 末段 label); 链断 → None。
    fn walk_sections<'a, 'b>(&'a self, label: &'b str) -> Option<(&'a Value, &'b str)> {
        let mut node: &Value = &self.root;
        let mut clsbix = 0usize;
        for i in 0..label.len() {
            if label.as_bytes()[i] == b'.' {
                node = find_section_ci(node, &label[clsbix..i])?;
                clsbix = i + 1;
            }
        }
        Some((node, &label[clsbix..]))
    }

    /// getone 寻址: section 链 + 末段 leaf 定位 (怪癖整形 + CS 子串)。
    /// 返回**原始 leaf 值** — 数组三形态由消费方处理 (get_str → leaf_to_text,
    /// get_f64 族 → first_number_f32)。
    fn find_getone_leaf<'a>(&'a self, label: &str) -> Option<&'a Value> {
        let (node, rest) = self.walk_sections(label)?;
        let (last, root_only, mode) = shape_leaf_label(rest);
        if root_only {
            // 仅根层键 (文本 "\n" 前缀 = 列 0 行首, 嵌套块行必缩进不可达)
            match &self.root {
                Value::Object(map) => map
                    .keys()
                    .find(|k| key_matches(k, last, mode))
                    .and_then(|k| map.get(k))
                    .filter(|v| !is_section(v)),
                _ => None,
            }
        } else {
            find_leaf_mode(node, last, mode, false)
        }
    }

    /// getone 等价: 寻址同上, 值文本化 (显示串/Bool 形态/"null" 哨兵)。
    /// 数值抽取请走 get_f64 族 (数值直读, 免字符串往返)。
    pub(crate) fn get_str(&self, label: &str) -> String {
        self.find_getone_leaf(label)
            .and_then(leaf_to_text)
            .unwrap_or_else(|| "null".to_string())
    }

    /// getdouble 等价 (数值直读): 寻址同 getone, 取首分量 (f32 域);
    /// 缺席/解析失败 → 0 (Java catch 路径)。
    pub(crate) fn get_f64(&self, label: &str) -> f64 {
        self.find_getone_leaf(label)
            .and_then(first_number_f32)
            .unwrap_or(0.0)
    }

    /// getdouble_exc 等价: 缺席哨兵 = Float.MAX_VALUE (调用方以
    /// `== f32::MAX as f64` 判截断); 命中但解析失败返回 0 (Java catch 路径)。
    pub(crate) fn get_f64_exc(&self, label: &str) -> f64 {
        match self.find_getone_leaf(label) {
            None => f32::MAX as f64,
            Some(v) => first_number_f32(v).unwrap_or(0.0),
        }
    }

    /// getdoubles 等价 (数值直读): 就地写 `ret[..num]`, 返回 None ↔ Java null:
    /// - `num <= 0` → null;
    /// - 键缺席 → **Some(()) 且 ret 不动** (调用方依赖 "找不到键时保持 0 初值");
    /// - 分量不足/非数值 → None (此时已写入的前缀保留 — 部分写入保真)。
    /// 分量序列 = 旧文本化协议 "leaf_to_text join(', ') 再 split" 的等价拆分
    /// (数组三形态): p2/p3 数组逐元素、嵌套数组 (merge 曲线) 取首 pair、
    /// 标量 merge 数组取首元素。
    pub(crate) fn get_f64s(&self, label: &str, ret: &mut [f64], num: usize) -> Option<()> {
        if num == 0 {
            return None;
        }
        let Some(leaf) = self.find_getone_leaf(label) else {
            return Some(()); // 键缺席: ret 保持初值
        };
        let comps: Vec<&Value> = match leaf {
            Value::Array(arr) if !arr.is_empty() => match &arr[0] {
                Value::Array(pair) => pair.iter().collect(),
                Value::String(_) | Value::Bool(_) => vec![&arr[0]],
                _ => arr.iter().collect(),
            },
            // 空数组 ≡ 空行值 "": 首段 parse 失败 → None (部分写入 0 个)
            Value::Array(_) => Vec::new(),
            other => vec![other],
        };
        for (i, slot) in ret.iter_mut().enumerate().take(num) {
            // 分量越界/非数值 = Java tmp[i] 越界或 parseFloat 抛 → catch null
            *slot = comps.get(i).and_then(|v| num_f32_domain(v))?;
        }
        Some(())
    }

    /// cut 等价: DFS 前序 CI 找 section, 返回子树**引用** (零拷贝)。
    pub(crate) fn section(&self, name: &str) -> BlkSection<'_> {
        match find_section_ci(&self.root, name) {
            Some(v) => BlkSection::Json(v),
            None => BlkSection::Null,
        }
    }
}

/// cut 的返回形态: JSON 子树引用 (未找到 ↔ Null)。
pub(crate) enum BlkSection<'a> {
    Null,
    Json(&'a Value),
}

impl BlkSection<'_> {
    /// 文本版 cut 哨兵 `"null"` 的等价判断。
    pub fn is_null(&self) -> bool {
        matches!(self, BlkSection::Null)
    }

    /// getonein_data: 块内 leaf 搜索 (点分段 section 链 + 末段 CI 定位);
    /// 未找到返回哨兵串 "null"。
    pub fn get_in(&self, label: &str) -> String {
        match self {
            BlkSection::Null => "null".to_string(),
            BlkSection::Json(v) => get_in_json(v, label),
        }
    }
}

/// JSON 子树的 get_in (getonein_data 等价: 点分段 CI section 链 + 末段 CI leaf)。
/// 自由函数形态 (reader.rs 的 BlkSection::get_in 经 super::json::get_in_json 调入)。
pub(crate) fn get_in_json(v: &Value, label: &str) -> String {
    let mut node: &Value = v;
    let mut clsbix = 0usize;
    for i in 0..label.len() {
        if label.as_bytes()[i] == b'.' {
            match find_section_ci(node, &label[clsbix..i]) {
                Some(n) => node = n,
                None => return "null".to_string(),
            }
            clsbix = i + 1;
        }
    }
    find_leaf_ci(node, &label[clsbix..])
        .and_then(leaf_to_text)
        .unwrap_or_else(|| "null".to_string())
}

// ==================== parse 入口 (Java 构造器等价) ====================

use super::FmData;
use crate::lang::Lang;
use crate::logger;

impl FmData {
    /// (path, name) 具名入口 (JSON) — Blkx::parse_named 的 JSON 对应物:
    /// doLoad=true 全量装载 (getload_from)。
    pub fn parse_named_json(filepath: &str, name: &str) -> Result<FmData, String> {
        Self::parse_named_opts_json(filepath, name, true)
    }

    /// Java 三参构造器等价 (JSON): doLoad=false 只读 (中央文件探测)。
    ///
    /// 守卫与文本版互为镜像: 空文件 → Err; 内容不以 '{' 开头 (blkx 文本误喂)
    /// → Err (文本版守卫语义的反转面); serde 解析失败 → Err; doLoad=true 的
    /// getload panic 由 catch_unwind 收敛 Err。
    pub fn parse_named_opts_json(filepath: &str, name: &str, do_load: bool) -> Result<FmData, String> {
        let file = std::path::Path::new(filepath);
        if !file.exists() {
            return Err(format!("FM文件不存在: {filepath}"));
        }
        let content = std::fs::read_to_string(file)
            .map_err(|e| format!("FM文件读取: {e}"))?;
        let src = Self::json_guard_and_load(name, &content)?;
        let mut b = FmData::default();
        b.fmdata = Some(Lang::init_lang().noblkx.to_string());
        b.read_file_name = Some(name.to_string());
        if do_load {
            // 与文本版 from_read_data 同防线: getload 的 panic 收敛 Err
            let load =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.getload_from(&src)));
            match load {
                Ok(()) => b.valid = true,
                Err(payload) => {
                    let msg = if let Some(s) = payload.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
                        (*s).to_string()
                    } else {
                        "null".to_string()
                    };
                    logger::error(
                        "FmData",
                        &format!("FM 解析失败, 标记无效: {name} - {msg}"),
                    );
                    return Err(format!("FM 解析失败, 标记无效: {name} - {msg}"));
                }
            }
        } else {
            b.valid = true;
        }
        Ok(b)
    }

    /// 测试/fuzz 注入入口 (JSON): content 直接充当文件内容。
    #[cfg(test)]
    pub fn parse_str_json(name: &str, content: &str) -> Result<FmData, String> {
        let src = Self::json_guard_and_load(name, content)?;
        let mut b = FmData::default();
        b.fmdata = Some(Lang::init_lang().noblkx.to_string());
        b.read_file_name = Some(name.to_string());
        b.getload_from(&src);
        b.valid = true;
        Ok(b)
    }

    /// JSON 守卫段: 空内容 / 非 '{' 开头 (blkx 文本误喂) / serde 解析失败 → Err。
    /// 空 blk 的两格式固有形态差异在此归一: BlkText 链路输出 0 字节文件
    /// (空文件守卫 → Err), JSON 链路序列化为 `{}` (空根对象) — 同判空对齐
    /// (parity 实测: i_180_event03 的 event 占位 FM)。
    fn json_guard_and_load(name: &str, content: &str) -> Result<JsonSrc, String> {
        if content.trim().is_empty() || content.trim() == "{}" {
            return Err(format!("FM文件读取失败或内容为空: {name}"));
        }
        if !content.trim_start().starts_with('{') {
            return Err(format!("非 JSON 格式文件误作 FM 加载, 标记无效: {name}"));
        }
        JsonSrc::parse_str(content)
    }
}

#[cfg(test)]
mod tests;
