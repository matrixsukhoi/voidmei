//! 公式持久化: formulas.cfg (内置只读, 随程序分发) + formulas.user.cfg (用户层)。
//! 格式: S-expr 外壳 (复用 sexp_parser) + 中缀公式体; 合并语义参照 config_manager
//! 双文件 — 同名用户条目覆盖 :expr/:unit/:precision/:desc/:disabled, 其余以模板为准。
//! 设计: doc/formula_system_design.md §11

use super::definition::FormulaDef;
use crate::sexp_parser::{AtomType, SExp, SExpParser};
use std::collections::HashMap;
use std::path::Path;

pub const BUILTIN_FORMULAS_PATH: &str = "./formulas.cfg";
pub const USER_FORMULAS_PATH: &str = "./formulas.user.cfg";

/// 解析公式文件文本 → 定义列表 (容错: 坏条目跳过, 注释性 :desc 保留)
pub fn parse_formulas(src: &str) -> Vec<FormulaDef> {
    let roots = SExpParser::new().parse(src);
    let mut defs = Vec::new();
    for root in &roots {
        let SExp::List(list) = root.as_ref() else { continue };
        for child in &list.children {
            let SExp::List(item) = child.as_ref() else { continue };
            let Some(def) = parse_one(item) else { continue };
            defs.push(def);
        }
    }
    defs
}

/// 解析规则段 ((rule ...) 条目; 与公式同文件共存)
pub fn parse_rules(src: &str) -> Vec<super::rules::RuleDef> {
    let roots = SExpParser::new().parse(src);
    let mut defs = Vec::new();
    for root in &roots {
        let SExp::List(list) = root.as_ref() else { continue };
        for child in &list.children {
            let SExp::List(item) = child.as_ref() else { continue };
            let head = item.children.first().map(|c| c.as_atom().value.clone()).unwrap_or_default();
            if head != "rule" {
                continue;
            }
            let Some(name) = positional_string(item, 1) else { continue };
            let actions = keyword_actions(item);
            defs.push(super::rules::RuleDef {
                name,
                when: keyword_string(item, ":when").unwrap_or_default(),
                hold_ms: keyword_f64(item, ":hold-ms", 0.0),
                cooldown_s: keyword_f64(item, ":cooldown-s", 0.0),
                actions,
                disabled: keyword_int(item, ":disabled", 0) != 0,
            });
        }
    }
    defs
}

/// :actions ((voice "k") (toast "t") (flag "f")) 解析
fn keyword_actions(list: &crate::sexp_parser::SList) -> Vec<super::rules::RuleAction> {
    use super::rules::RuleAction;
    let mut out = Vec::new();
    // 找 :actions 关键字的值 (内层 list of list)
    let mut i = 0;
    while i + 1 < list.children.len() {
        let k = list.children[i].as_atom();
        if k.r#type == AtomType::Keyword && k.value == ":actions" {
            if let SExp::List(inner) = list.children[i + 1].as_ref() {
                for a in &inner.children {
                    let SExp::List(pair) = a.as_ref() else { continue };
                    let kind = pair.children.first().map(|c| c.as_atom().value.clone()).unwrap_or_default();
                    let arg = pair.children.get(1).map(|c| c.as_atom().value.clone()).unwrap_or_default();
                    match kind.as_str() {
                        "voice" if !arg.is_empty() => out.push(RuleAction::Voice(arg)),
                        "toast" if !arg.is_empty() => out.push(RuleAction::Toast(arg)),
                        "flag" if !arg.is_empty() => out.push(RuleAction::Flag(arg)),
                        _ => {}
                    }
                }
            }
            break;
        }
        i += 1;
    }
    out
}

fn keyword_f64(list: &crate::sexp_parser::SList, kw: &str, def: f64) -> f64 {
    keyword_string(list, kw).and_then(|v| v.trim().parse().ok()).unwrap_or(def)
}

fn parse_one(list: &crate::sexp_parser::SList) -> Option<FormulaDef> {
    let head = list.children.first()?.as_atom().get_string();
    if head != "formula" {
        return None;
    }
    let name = positional_string(list, 1)?;
    let expr = keyword_string(list, ":expr").unwrap_or_default();
    Some(FormulaDef {
        name,
        expr,
        unit: keyword_string(list, ":unit").unwrap_or_default(),
        precision: keyword_int(list, ":precision", 0).clamp(0, 9) as u8,
        desc: keyword_string(list, ":desc").unwrap_or_default(),
        disabled: keyword_int(list, ":disabled", 0) != 0,
        builtin: keyword_int(list, ":builtin", 0) != 0,
    })
}

/// 加载合并: 内置模板 + 用户覆盖/新增。
/// 覆盖语义 = **完整定义替换**(编辑器保存时 serialize_user 写全字段),
/// 用户条目缺字段时以缺省值生效 — 不做逐字段继承。
pub fn load_merged(builtin_path: &str, user_path: &str) -> Vec<FormulaDef> {
    let builtin = read_file_lossy(builtin_path);
    let user = read_file_lossy(user_path);
    merge_defs(&parse_formulas(&builtin), &parse_formulas(&user))
}

fn read_file_lossy(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn merge_defs(builtin: &[FormulaDef], user: &[FormulaDef]) -> Vec<FormulaDef> {
    let mut out: Vec<FormulaDef> = builtin.to_vec();
    let mut index: HashMap<String, usize> =
        out.iter().enumerate().map(|(i, d)| (d.name.clone(), i)).collect();
    for u in user {
        match index.get(&u.name) {
            Some(&i) => {
                // 用户覆盖内置: 编辑字段以用户为准, builtin 标志保留
                out[i] = FormulaDef {
                    name: u.name.clone(),
                    expr: u.expr.clone(),
                    unit: u.unit.clone(),
                    precision: u.precision,
                    desc: u.desc.clone(),
                    disabled: u.disabled,
                    builtin: true,
                };
            }
            None => {
                index.insert(u.name.clone(), out.len());
                out.push(FormulaDef { builtin: false, ..u.clone() });
            }
        }
    }
    out
}

/// 序列化用户文件 (调用方决定写哪些 — 用户自定义 + 被改内置)
pub fn serialize_user(defs: &[FormulaDef]) -> String {
    let mut s = String::from(";; VoidMei 用户公式 (自动生成; 内置出厂定义见 formulas.cfg)\n(formulas\n");
    for d in defs {
        s.push_str(&format!(
            "  (formula \"{}\" :expr \"{}\" :unit \"{}\" :precision {} :desc \"{}\"{}{})\n",
            escape(&d.name),
            escape(&d.expr),
            escape(&d.unit),
            d.precision,
            escape(&d.desc),
            if d.disabled { " :disabled 1" } else { "" },
            if d.builtin { " :builtin 1" } else { "" },
        ));
    }
    s.push_str(")\n");
    s
}

pub fn save_user(defs: &[FormulaDef], path: &str) -> std::io::Result<()> {
    if let Some(dir) = Path::new(path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, serialize_user(defs))
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// --- SList 取值小工具 (config_loader 同族函数为私有, 此处自带, 不侵入) ---

fn positional_string(list: &crate::sexp_parser::SList, idx: usize) -> Option<String> {
    let a = list.children.get(idx)?.as_atom();
    if matches!(a.r#type, AtomType::String | AtomType::Symbol | AtomType::Keyword) {
        Some(a.value.clone())
    } else {
        None
    }
}

fn keyword_string(list: &crate::sexp_parser::SList, kw: &str) -> Option<String> {
    let mut i = 0;
    while i + 1 < list.children.len() {
        let k = list.children[i].as_atom();
        if k.r#type == AtomType::Keyword && k.value == kw {
            return Some(list.children[i + 1].as_atom().value.clone());
        }
        i += 1;
    }
    None
}

fn keyword_int(list: &crate::sexp_parser::SList, kw: &str, def: i32) -> i32 {
    keyword_string(list, kw).and_then(|v| v.trim().parse().ok()).unwrap_or(def)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_merge() {
        let builtin = "(formulas\n  (formula \"a\" :expr \"ias*2\" :unit \"x\" :precision 1 :desc \"内\" :builtin 1)\n  (formula \"b\" :expr \"tas\" :builtin 1)\n)\n";
        let user = "(formulas\n  (formula \"a\" :expr \"ias*3\" :unit \"x\" :precision 1 :desc \"内\" :builtin 1)\n  (formula \"c\" :expr \"mach\")\n)\n";
        let merged = merge_defs(&parse_formulas(builtin), &parse_formulas(user));
        assert_eq!(merged.len(), 3);
        let a = merged.iter().find(|d| d.name == "a").unwrap();
        assert_eq!(a.expr, "ias*3");
        assert!(a.builtin, "覆盖内置仍标 builtin");
        let c = merged.iter().find(|d| d.name == "c").unwrap();
        assert!(!c.builtin);
        // 序列化→再解析 roundtrip
        let s = serialize_user(&merged);
        let back = parse_formulas(&s);
        assert_eq!(back.len(), 3);
        assert_eq!(back[0].expr, "ias*3");
        assert_eq!(back[0].precision, 1);
    }

    #[test]
    fn escape_quotes() {
        let d = FormulaDef {
            name: "x\"y".into(),
            expr: "1 \" 2".into(),
            ..Default::default()
        };
        let back = parse_formulas(&serialize_user(&[d]));
        assert_eq!(back[0].name, "x\"y");
        assert_eq!(back[0].expr, "1 \" 2");
    }
}
