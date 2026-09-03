//! 对比窗口数据命令域: `ui/window/comparison/CompactComparisonWindow.java` 的
//! Rust 数据面 (displayStructure + dataMap0/1 构造 + 胜负规则接线, vm-core
//! comparison rules 已译)。像素/布局归 web 前端, 本层只出数据。
//!
//! 与 D9 主壳 IPC (commands.rs → mpsc → 主线程 dispatch) 的分工: 本域命令的
//! 计算面只依赖 vm-core (FMLoader/Blkx/对比规则, 全线程安全), **不经主线程
//! dispatcher** — AppShell (!Send) 不被触碰, vm-app 的 form_dispatch 零改动。
//! (功率曲线域 commands_powercurve 与 commands.rs 的 fm_list 同款直算模式。)
//!
//! 直算备案 (审查 W3 — 接受直算, 不下放 blocking 池; 三域原合文件拆分时随
//! comparison 域收存): comparison/power_curve/fm_list 的重计算 (Blkx
//! 全量解析/双曲线采样/目录扫描) 直接跑在命令执行上下文, 双窗并发查询理论可占住
//! async worker。实测两种下放实现均不可用: 引用 `tauri::async_runtime::
//! spawn_blocking` 或手写 std Future 桥 (线程 + waker) 都会向 cargo test 二进制
//! 拖入 comctl32 v6 依赖 (`TaskDialogIndirect`, 无 SxS manifest) → 加载即
//! STATUS_ENTRYPOINT_NOT_FOUND, 测试全灭 (二分定位实锤: 去掉对下放桥的引用即
//! 恢复干净导入表)。解锁路径: 为测试二进制嵌 common-controls v6 manifest
//! (build.rs) 后再启用下放; 当前单窗使用无观测面, 按 reviewer 裁决"备案接受"。
//!
//! 跨域共用小件 ([`normalize_secondary`]/[`comparison_title`]/
//! [`fallback_physical_file`]) 收在本文件: fm1 归一化与物理文件回退的 Java 原型
//! 在两窗构造器同款, comparison 域为宿主, commands_powercurve/web_windows 引用。

use std::collections::HashMap;
use std::path::PathBuf;

use vm_core::base::java_compat::java_trim;
use vm_core::fm::data::FmData;
use vm_core::ui_support::comparison::comparison_rules::ComparisonRules;
use vm_core::fm::data_paths;
use vm_core::fm::loader;
use vm_core::lang::Lang;

use crate::commands::to_json;
use crate::dto::{ComparisonDataDto, ComparisonRowDto, Win};

// =====================================================================
// 跨域共用小件 (commands_powercurve / web_windows 引用)
// =====================================================================

/// fm1 归一化 (单源): None/空串/与 fm0 同名 → None (单机模式)。
/// 原型 = Java 构造器裁决 (fm1 空/==fm0 = 单曲线);
/// 波13 统一 comparison 侧 (此前只判空) — 同名对比无信息量, 并入单机视图,
/// 与窗口 title/query/DTO 三面保持一致。
pub(crate) fn normalize_secondary<'a>(fm0: &str, fm1: Option<&'a str>) -> Option<&'a str> {
    fm1.filter(|s| !s.is_empty() && *s != fm0)
}

/// 对比窗口标题 (CompactComparisonWindow 构造器): DTO title 与
/// web_windows 窗口 title 同源 (波13 收敛, 归一化由调用方先行)。
pub(crate) fn comparison_title(fm0: &str, fm1: Option<&str>) -> String {
    match fm1 {
        Some(n) => format!("Comparison: {fm0} vs {n}"),
        None => format!("Aircraft Data: {fm0}"),
    }
}

/// 名字空间差异回退的物理文件定位: 只查 `fm/<name>.json`, 不存在返回 None
/// (name 原样使用 — Java 拼串不做大小写规范化; blkx/blk 过渡期回落已随
/// blkx→json 迁移终态退役)。
/// 背景: name 是 fm/ 物理文件名（连字符, 如 a-10c）, 中央机型名是下划线
/// （a_10c）——少数不同名机型 FMLoader 判 MISSING, 按物理文件直读。
pub(crate) fn fallback_physical_file(name: &str) -> Option<PathBuf> {
    let f = data_paths::fm_dir().join(format!("fm/{name}.json"));
    if f.exists() { Some(f) } else { None }
}

// =====================================================================
// 结构解析与合并 (initUI 数据段)
// =====================================================================

/// Java `private static class DisplayItem` (isHeader + text)
#[derive(Debug, Clone)]
pub struct DisplayItem {
    pub is_header: bool,
    pub text: String,
}

// Regex to parse: "Property Name: Value [Unit]"
// Example: "空重(kg): 4644.0" -> Prop="空重(kg)", Val=4644.0
// Example: "临界速度(km/h): [144, 1167]" -> Prop="...", Val="[144, 1167]" (Complex)
// (CompactComparisonWindow.java PROP_PATTERN 原注释, 逐字保留 — PORTING.md §0.2)
//
// PORT: `([^:]+):\s*(.*)` 的 matches() (全串匹配) — `[^:]+` 无法跨越 ':', 故
// 第一个冒号即分界且要求 ≥1 个前置字符; `\s*` 吞组2头部空白 + group(2).trim()
// 归并为 java_trim (Java \s ⊂ trim 字符集, §2.1)
fn parse_prop_line(line: &str) -> Option<(String, String)> {
    let pos = line.find(':')?;
    if pos == 0 {
        return None; // ([^:]+) 需至少 1 个字符
    }
    let k = java_trim(&line[..pos]).to_string();
    let v = java_trim(&line[pos + 1..]).to_string();
    Some((k, v))
}

/// Java `findInStructure` —
/// 波14: Java 的 -1 哨兵退役, 未找到 = None (position() 一步到位)。
fn find_in_structure(list: &[DisplayItem], key: &str) -> Option<usize> {
    list.iter().position(|item| !item.is_header && item.text == key)
}

/// initUI 的解析段: lines0 建结构 +
/// dataMap0, lines1 合并独有键进结构、全量进 dataMap1。纯函数 (无 UI/文件)。
fn build_structure(
    lines0: &[String],
    lines1_safe: &[String],
) -> (Vec<DisplayItem>, HashMap<String, String>, HashMap<String, String>) {
    let mut structure: Vec<DisplayItem> = Vec::new();
    let mut map0: HashMap<String, String> = HashMap::new();
    let mut map1: HashMap<String, String> = HashMap::new();

    // 1. Build initial structure from lines0
    for l in lines0 {
        let l = java_trim(l);
        if l.is_empty() {
            continue;
        }
        if l.contains("------") {
            let h = java_trim(&l.replace('-', "")).to_string();
            structure.push(DisplayItem { is_header: true, text: h });
            continue;
        }
        if let Some((k, v)) = parse_prop_line(l) {
            structure.push(DisplayItem { is_header: false, text: k.clone() });
            map0.insert(k, v);
        }
    }

    // 2. Parse lines1 and merge unique keys
    let mut last_match: Option<usize> = None; // 最近命中/插入位 (Java lastMatchIndex, -1 = None)
    // Find where the content starts (skip initial headers if possible, or just
    // merge)
    // Simple merge: scan lines1. If key exists, update index. If not, insert after
    // index.
    // (原注释保留)
    for l in lines1_safe {
        let l = java_trim(l);
        // Skip headers in merge for now to avoid duplications (原注释保留)
        if l.is_empty() || l.contains("------") {
            continue;
        }
        if let Some((k, v)) = parse_prop_line(l) {
            map1.insert(k.clone(), v);

            // Check struct (未命中 → 插入最近命中位之后; None 视作 -1+1=0)
            match find_in_structure(&structure, &k) {
                Some(idx) => last_match = Some(idx),
                None => {
                    // Insert after last match
                    let insert_at = last_match.map_or(0, |i| i + 1);
                    // Java `lastMatchIndex < size-1` 即插入位在表内 → insert, 否则尾部 push
                    if insert_at < structure.len() {
                        structure.insert(insert_at, DisplayItem { is_header: false, text: k });
                    } else {
                        structure.push(DisplayItem { is_header: false, text: k });
                    }
                    last_match = Some(insert_at); // 新键落位 (= Java 的 +=1)
                }
            }
        }
    }

    (structure, map0, map1)
}

// =====================================================================
// 胜负判定 (vm-core comparison rules 接线)
// =====================================================================

/// addComparisonRow 的胜负判定。
/// 入参用**展示串** (缺键已补 "-"), 与 Java 调用点一致 (extractValue("-") 无数字
/// → None → 平局)。
fn row_win(prop: &str, v0: &str, v1: &str, single_mode: bool) -> Win {
    // Determine Winner using rule system
    let mut win = Win::Draw; // 平局缺省
    if let Some(rule) = ComparisonRules::get(prop) {
        if !single_mode {
            let d0 = rule.extract_value(Some(v0));
            let d1 = rule.extract_value(Some(v1));
            if let (Some(d0), Some(d1)) = (d0, d1) {
                if (d0 - d1).abs() > 0.001 {
                    let lower_is_better = rule.is_lower_better();
                    win = if d0 > d1 {
                        if lower_is_better { Win::Right } else { Win::Left }
                    } else if lower_is_better {
                        Win::Left
                    } else {
                        Win::Right
                    };
                }
            }
        }
    }
    // No rule → Draw (grey color)
    win
}

/// determineWinner (buildCopyText 用) —
/// 与 row_win 的差异: 入参是**原始可空值** (null 判在规则前)。
fn winner_name(
    prop: &str,
    v0: Option<&str>,
    v1: Option<&str>,
    fm0_name: &str,
    fm1_name: &str,
) -> Option<String> {
    let rule = ComparisonRules::get(prop)?;
    let (v0, v1) = (v0?, v1?);
    let d0 = rule.extract_value(Some(v0))?;
    let d1 = rule.extract_value(Some(v1))?;
    // Java `Math.abs(d0 - d1) < 0.001` float 比较, 原样保留
    #[allow(clippy::float_cmp)]
    {
        if (d0 - d1).abs() < 0.001 {
            return None;
        }
    }
    let lower_is_better = rule.is_lower_better();
    Some(
        (if d0 > d1 {
            if lower_is_better { fm1_name } else { fm0_name }
        } else if lower_is_better {
            fm0_name
        } else {
            fm1_name
        })
        .to_string(),
    )
}

// =====================================================================
// 数据装配
// =====================================================================

/// Java `loadFmLines`: FMLoader 标准链路
/// 加载, MISSING 时按物理文件直读回退; 返回 fmdata 的非空白行。
pub fn load_fm_lines(name: Option<&str>) -> Vec<String> {
    let name = match name {
        Some(n) if !java_trim(n).is_empty() => n,
        _ => return Vec::new(),
    };
    // P5 收编: 优先 FMLoader 标准链路（机型名 → 中央文件 → 物理文件 → 全量解析）。
    // 本方法只用 fmdata 文本 —— finalizeLoading 只清原始 data 文本，刻意保留 fmdata
    // （FMDataOverlay 依赖），故 READY 句柄的 blkx 直接可用，无需自行解析
    let handle = loader::load(Some(name));
    let fmdata: String = if handle.has_fm() {
        handle.fmdata.and_then(|b| b.fmdata).unwrap_or_default()
    } else {
        // 名字空间差异回退: name 是 fm/ 物理文件名（连字符, 如 a-10c）, 中央机型名
        // 是下划线（a_10c）——少数不同名机型 FMLoader 判 MISSING, 按物理文件直读
        match fallback_physical_file(name) {
            Some(f) => match FmData::parse_named_json(&f.to_string_lossy(), name) {
                Ok(b) => b.fmdata.unwrap_or_default(),
                Err(_) => {
                    // Java 构造器内 catch 产出 valid=false 对象 (fmdata=noblkx),
                    // 行为保真: 以 noblkx 文本顶位 (无冒号 → 解析段自然过滤)
                    Lang::init_lang().noblkx.to_string()
                }
            },
            // fmdata=noblkx (Java 构造器头部赋值) — 与上方解析失败分支
            // 同文本顶位: load_fm_lines 返回 noblkx 两行 (无冒号 → 对比解析段全滤,
            // DTO 可观察结果不变; 直接消费行内容的调用点也与 Java 一致)
            None => Lang::init_lang().noblkx.to_string(),
        }
    };

    // fmdata is string.
    if fmdata.is_empty() {
        return Vec::new();
    }
    fmdata
        .split('\n')
        .filter(|s| !java_trim(s).is_empty())
        .map(String::from)
        .collect()
}

/// buildCopyText: COPY 按钮文本
fn build_copy_text(
    fm0_name: &str,
    fm1_name: &str,
    single_mode: bool,
    structure: &[DisplayItem],
    map0: &HashMap<String, String>,
    map1: &HashMap<String, String>,
) -> String {
    let mut sb = String::new();

    // Header
    if single_mode {
        sb.push_str(&format!("========== Aircraft Data: {fm0_name} ==========\n\n"));
    } else {
        sb.push_str(&format!(
            "========== Comparison: {fm0_name} vs {fm1_name} ==========\n\n"
        ));
    }

    // Body
    for item in structure {
        if item.is_header {
            sb.push_str(&format!("---------- {} ----------\n", item.text));
        } else {
            let k = &item.text;
            let v0 = map0.get(k).map(String::as_str);
            let v1 = map1.get(k).map(String::as_str);

            if single_mode {
                sb.push_str(&format!("{}: {}\n", k, v0.unwrap_or("-")));
            } else {
                sb.push_str(&format!("{}: {} vs {}", k, v0.unwrap_or("-"), v1.unwrap_or("-")));
                if let Some(w) = winner_name(k, v0, v1, fm0_name, fm1_name) {
                    sb.push_str(&format!("  [{w} +]"));
                }
                sb.push('\n');
            }
        }
    }
    sb
}

/// 对比窗口数据装配 (initUI 的数据段)
pub fn comparison_data_impl(fm0_name: &str, fm1_name: Option<&str>) -> ComparisonDataDto {
    // fm1 归一化 (空串/同名 → 单机模式, 波13 统一 — 见 normalize_secondary)
    let fm1_name = normalize_secondary(fm0_name, fm1_name);
    let single_mode = fm1_name.is_none();

    // Get Data
    let lines0 = load_fm_lines(Some(fm0_name));
    let lines1 = load_fm_lines(fm1_name);

    // Parse Data into Maps and Structure
    let lines1_safe = lines1;
    let (structure, map0, map1) = build_structure(&lines0, &lines1_safe);

    // Render (行清单: 标题行/属性行 + 胜负符号)
    let mut rows: Vec<ComparisonRowDto> = Vec::new();
    for item in &structure {
        if item.is_header {
            rows.push(ComparisonRowDto {
                is_header: true,
                text: item.text.clone(),
                value0: None,
                value1: None,
                win: Win::Draw,
                symbol: String::new(),
            });
            continue;
        }
        let k = &item.text;
        let v0 = map0.get(k).map(String::as_str);
        let v1 = map1.get(k).map(String::as_str);
        // If v0 is null, it means it's a key only in FM1.
        // If v1 is null, it means it's a key only in FM0 (or Single Mode).
        // (原注释保留)
        let disp0 = v0.unwrap_or("-").to_string();
        let win = if single_mode {
            Win::Draw // 单机模式无胜负 (Java addComparisonRow 的 !singleMode 守卫)
        } else {
            row_win(k, &disp0, v1.unwrap_or("-"), single_mode)
        };
        let sym = match win {
            Win::Left => "▶",
            Win::Right => "◀",
            Win::Draw => "-",
        };
        rows.push(ComparisonRowDto {
            is_header: false,
            text: k.clone(),
            value0: Some(disp0),
            value1: if single_mode { None } else { Some(v1.unwrap_or("-").to_string()) },
            win,
            symbol: sym.to_string(),
        });
    }

    let title = comparison_title(fm0_name, fm1_name);
    let copy_text = build_copy_text(
        fm0_name,
        fm1_name.unwrap_or(""),
        single_mode,
        &structure,
        &map0,
        &map1,
    );

    ComparisonDataDto {
        fm0_name: fm0_name.to_string(),
        fm1_name: fm1_name.map(str::to_string),
        single_mode,
        title,
        rows,
        copy_text,
    }
}

/// 对比窗口数据 (CompactComparisonWindow: displayStructure + dataMap0/1 + 胜负;
/// 直算不经主线程 — 见模块头分工说明)
#[tauri::command]
pub async fn comparison_data(
    fm0: String,
    fm1: Option<String>,
) -> Result<serde_json::Value, String> {
    to_json(&comparison_data_impl(&fm0, fm1.as_deref()))
}

// =====================================================================
// Tests — 数据面单测: 纯函数 (解析/合并/胜负) 无文件依赖; 真机腿用项目内
// data/ 的 spitfire blkx, data 缺失环境按 realtests 先例 SKIP+真因。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ensure_real_data;

    // ---- 纯函数: PROP_PATTERN 解析 ----

    #[test]
    fn prop_line_冒号分界与列表值() {
        // Java javadoc 示例
        assert_eq!(
            parse_prop_line("空重(kg): 4644.0"),
            Some(("空重(kg)".to_string(), "4644.0".to_string()))
        );
        assert_eq!(
            parse_prop_line("临界速度(km/h): [144, 1167]"),
            Some(("临界速度(km/h)".to_string(), "[144, 1167]".to_string()))
        );
        // 多冒号: [^:]+ 无法跨越 ':', 第一个冒号分界
        assert_eq!(
            parse_prop_line("a: b: c"),
            Some(("a".to_string(), "b: c".to_string()))
        );
        // 冒号前 ≥1 字符 (Java ([^:]+) 空匹配失败)
        assert_eq!(parse_prop_line(": x"), None);
        // 无冒号整行丢弃
        assert_eq!(parse_prop_line("没有冒号的行"), None);
        // 前置空白冒号 → k trim 为空串 (Java 仍入 map, 保真)
        assert_eq!(
            parse_prop_line(" : 1.0"),
            Some(("".to_string(), "1.0".to_string()))
        );
    }

    // ---- 纯函数: 结构合并 ----

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn 结构合并_表头与fm1独有键插入() {
        let l0 = lines(&[
            "------ 基本参数 ------",
            "空重(kg): 1000.0",
            "最大燃油重量(kg): 500.0",
            "翼展效率: 0.9",
        ]);
        let l1 = lines(&[
            "------ 其他 ------", // merge 段跳过表头 (避免重复)
            "空重(kg): 1200.0",
            "独有属性A: 1",
            "翼展效率: 0.8",
            "独有属性B: 2",
        ]);
        let (structure, map0, map1) = build_structure(&l0, &l1);

        // 结构: header + 3 键 + FM1 独有两键按 lastMatch 插入:
        // 独有属性A 出现时最近命中是 空重(kg)(idx1) → 插在 idx2 (最大燃油重量
        // 之前); 独有属性B 出现时最近命中是 翼展效率(已到尾) → 尾部追加
        let texts: Vec<(bool, &str)> = structure
            .iter()
            .map(|d| (d.is_header, d.text.as_str()))
            .collect();
        assert_eq!(
            texts,
            vec![
                (true, "基本参数"),
                (false, "空重(kg)"),
                (false, "独有属性A"),
                (false, "最大燃油重量(kg)"),
                (false, "翼展效率"),
                (false, "独有属性B"),
            ]
        );
        assert_eq!(map0.get("空重(kg)").map(String::as_str), Some("1000.0"));
        assert_eq!(map1.get("空重(kg)").map(String::as_str), Some("1200.0"));
        assert!(!map0.contains_key("独有属性A"));
        assert_eq!(map1.get("独有属性B").map(String::as_str), Some("2"));
    }

    #[test]
    fn 结构合并_fm0为空时fm1键全量追加() {
        // lastMatchIndex 初值 -1: 结构空时 size-1 = -1, -1 < -1 假 → 尾部追加
        let l1 = lines(&["a: 1", "b: 2"]);
        let (structure, map0, map1) = build_structure(&[], &l1);
        assert_eq!(structure.len(), 2);
        assert_eq!(structure[1].text, "b");
        assert!(map0.is_empty());
        assert_eq!(map1.len(), 2);
    }

    // ---- 纯函数: 胜负规则接线 ----

    #[test]
    fn 胜负规则_接线_vm_core规则族() {
        // 空重: lower better — v0 重 → 右胜 (Right)
        assert_eq!(row_win("空重(kg)", "5000.0", "4000.0", false), Win::Right);
        // v0 轻 → 左胜
        assert_eq!(row_win("空重(kg)", "3000.0", "4000.0", false), Win::Left);
        // 最大燃油重量: higher better
        assert_eq!(row_win("最大燃油重量(kg)", "800.0", "500.0", false), Win::Left);
        // 临界速度: ListIndexRule(1) — "[144, 1167]" 取 1167, higher better
        assert_eq!(row_win("临界速度(km/h)", "[144, 1167]", "[144, 1300]", false), Win::Right);
        // 允许过载: MultiListIndexRule(0,1) — "[8.5, -4.2], [10.1, -5.3]" 取 -4.2
        assert_eq!(
            row_win("允许过载(满/半油)", "[8.5, -4.2], [10.1, -5.3]", "[7.0, -3.0], [9.0, -4.0]", false),
            Win::Right
        );
        // 主阻力面积因数: Lambda 取 '/' 后第二个数, lower better
        assert_eq!(row_win("主阻力面积因数及加速度系数", "0.25 / 0.35", "0.20 / 0.30", false), Win::Right);
        // 散热/油冷器: SLASH_BOTH 求和, lower better — 0.5+0.6=1.1 vs 0.4+0.5=0.9
        assert_eq!(row_win("散热/油冷器阻力系数", "0.5 / 0.6", "0.4 / 0.5", false), Win::Right);
        // |d0-d1| <= 0.001 → 平
        assert_eq!(row_win("空重(kg)", "4000.0005", "4000.0", false), Win::Draw);
        // 缺键补 "-" 后 extract 失败 → 平 (Java 调用点形态)
        assert_eq!(row_win("空重(kg)", "-", "4000.0", false), Win::Draw);
        // 无规则属性 → 平
        assert_eq!(row_win("无规则属性", "1.0", "2.0", false), Win::Draw);
        // 单机模式恒平 (Java !singleMode 守卫)
        assert_eq!(row_win("空重(kg)", "5000.0", "4000.0", true), Win::Draw);
    }

    #[test]
    fn copy文本_胜负方名() {
        let winner = winner_name("空重(kg)", Some("5000.0"), Some("4000.0"), "fm0", "fm1");
        assert_eq!(winner.as_deref(), Some("fm1")); // 右侧轻 → fm1 胜
        // 缺键 (v1 None) → None (Java null 判在规则前)
        assert_eq!(winner_name("空重(kg)", Some("5000.0"), None, "fm0", "fm1"), None);
        assert_eq!(winner_name("无规则", Some("1"), Some("2"), "fm0", "fm1"), None);
    }

    // ---- 真机腿 (data/ 缺失 SKIP) ----

    #[test]
    fn 真机_对比_单机模式() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (comparison_data 无数据源)");
            return;
        }
        let dto = comparison_data_impl("spitfire_f24", None);
        assert!(dto.single_mode);
        assert_eq!(dto.fm1_name, None);
        assert_eq!(dto.title, "Aircraft Data: spitfire_f24");
        assert!(!dto.rows.is_empty(), "fmdata 应产出行清单");
        // 表头行存在 ("------" 分节)
        assert!(
            dto.rows.iter().any(|r| r.is_header),
            "fmdata 应含 ------ 分节: {:?}",
            &dto.rows[..5.min(dto.rows.len())]
        );
        // fmdata 首行 = "FM文件: <fmfile> - <版本\n>" — bFmVersion 模板尾无换行,
        // 版本串 (version 文件 readLine 拼接) 自带 \n, 故首行在版本处断行。
        // (旧断言 "首行与空重粘一行" 是 version 读不到的环境假象: get_version
        // 曾硬编码 ./data 相对 cwd, cargo 测试 cwd 下读不到 → 空版本串粘住
        // 下一行; 走 fm_data_paths 后与 Java 生产一致, 空重独立成行)
        let fmfile = dto
            .rows
            .iter()
            .find(|r| !r.is_header && r.text == "FM文件")
            .expect("应含 FM文件 首行");
        assert!(
            fmfile.value0.as_deref().unwrap_or("").contains("fm/spitfire_f24.blk"),
            "FM文件 行值应含物理文件相对路径"
        );
        // 规则属性存在且值为数字 (Blkx.getload 格式化产物)
        let fuel = dto
            .rows
            .iter()
            .find(|r| !r.is_header && r.text == "最大燃油重量(kg)")
            .expect("应含 最大燃油重量(kg) 行");
        assert!(fuel.value0.as_deref().unwrap_or("-").parse::<f64>().is_ok());
        // 单机模式: 全部平局, value1 恒 None
        assert!(dto.rows.iter().all(|r| r.win == Win::Draw));
        assert!(dto.rows.iter().all(|r| r.value1.is_none()));
        // copy 文本单机形态
        assert!(dto.copy_text.starts_with("========== Aircraft Data: spitfire_f24"));
        assert!(dto.copy_text.contains("空重(kg): "));
    }

    #[test]
    fn 真机_对比_双机模式_胜负与合并() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (comparison_data 无数据源)");
            return;
        }
        let dto = comparison_data_impl("spitfire_f24", Some("spitfire_f22"));
        assert!(!dto.single_mode);
        assert_eq!(dto.fm1_name.as_deref(), Some("spitfire_f22"));
        assert_eq!(dto.title, "Comparison: spitfire_f24 vs spitfire_f22");
        // 最大燃油重量(kg) (SimpleRule higher-better): 胜负 = 值比较的确定性推演
        let fuel = dto
            .rows
            .iter()
            .find(|r| !r.is_header && r.text == "最大燃油重量(kg)")
            .expect("应含 最大燃油重量(kg) 行");
        let w0: f64 = fuel.value0.as_deref().unwrap().parse().unwrap();
        let w1: f64 = fuel.value1.as_deref().unwrap().parse().unwrap();
        let expect = if (w0 - w1).abs() > 0.001 {
            if w0 > w1 { Win::Left } else { Win::Right }
        } else {
            Win::Draw
        };
        assert_eq!(
            fuel.win, expect,
            "最大燃油重量 higher-better: v0={w0} v1={w1} win 应为 {expect:?}"
        );
        assert_eq!(
            fuel.symbol,
            match expect {
                Win::Left => "▶",
                Win::Right => "◀",
                Win::Draw => "-",
            }
        );
        // 双机 copy 文本含胜负方名
        assert!(dto.copy_text.contains("vs"));
        assert!(dto.copy_text.contains("spitfire_f2"));
        // 有胜负符号的行集合与 win!=Draw 一致
        for r in &dto.rows {
            if r.is_header {
                continue;
            }
            let expect_sym = match r.win {
                Win::Left => "▶",
                Win::Right => "◀",
                Win::Draw => "-",
            };
            assert_eq!(r.symbol, expect_sym);
        }
    }

    #[test]
    fn load_fm_lines_物理不存在返回noblkx两行() {
        // (Blkx.java:1671), loadFmLines 返回 noblkx 两行 — 与解析失败分支同文本
        let lines = load_fm_lines(Some("voidmei-nonexistent-fm"));
        let expect: Vec<String> = Lang::init_lang()
            .noblkx
            .split('\n')
            .filter(|s| !java_trim(s).is_empty())
            .map(String::from)
            .collect();
        assert_eq!(lines, expect, "物理缺失走 noblkx 文本 (与 Java 一致)");
        assert_eq!(lines.len(), 2);
        // DTO 可观察结果等价: noblkx 行无冒号无 ------ → 解析段全滤, rows 为空
        let dto = comparison_data_impl("voidmei-nonexistent-fm", None);
        assert!(dto.rows.is_empty(), "noblkx 行不产 DTO 行: {:?}", dto.rows);
    }
}
