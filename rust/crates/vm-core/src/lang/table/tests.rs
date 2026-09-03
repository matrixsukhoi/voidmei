use super::*;
use std::collections::BTreeMap;

/// Properties 空白字符集: 空格/制表/换页 (全角空格 U+3000 不算, 原样进值)
fn is_props_ws(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\u{c}'
}

/// `java.util.Properties.load(Reader)` 最小兼容实现 (快照对拍专用):
/// 注释(#/!)与空行跳过、尾反斜杠续行、键分隔符(`=`/`:`/空白)两侧空白跳过、
/// 值尾空白保留、`\t \n \r \f \\ \uXXXX` 转义、重复键后者覆盖 (Hashtable.put)。
/// 局限: 行终止符按 `\n`/`\r\n` 识别 (源文件为 CRLF, 无裸 `\r` 行)。
fn load_java_properties(text: &str) -> BTreeMap<String, String> {
    // 1) 物理行 → 逻辑行: 尾随奇数个反斜杠 = 续行 (丢弃该反斜杠与行终止符, 续行段前导空白丢弃)
    let mut logical: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_entry = false;
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if in_entry {
            cur.push_str(line.trim_start_matches(is_props_ws));
        } else {
            let t = line.trim_start_matches(is_props_ws);
            if t.is_empty() || t.starts_with('#') || t.starts_with('!') {
                continue; // 空行/注释行 (注释不可续行)
            }
            cur.clear();
            cur.push_str(t);
            in_entry = true;
        }
        let trailing_bs = cur.len() - cur.trim_end_matches('\\').len();
        if trailing_bs % 2 == 1 {
            cur.pop(); // 转义行终止符的反斜杠丢弃
        } else {
            logical.push(std::mem::take(&mut cur));
            in_entry = false;
        }
    }

    // 2) 逻辑行拆 (key, value) 再解转义
    let mut map = BTreeMap::new();
    for line in &logical {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        let mut key = String::new();
        // 键到第一个未转义的 空白/'='/':' 为止
        while i < chars.len() {
            let c = chars[i];
            if c == '\\' && i + 1 < chars.len() {
                key.push(c);
                key.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '=' || c == ':' || is_props_ws(c) {
                break;
            }
            key.push(c);
            i += 1;
        }
        while i < chars.len() && is_props_ws(chars[i]) {
            i += 1;
        }
        if i < chars.len() && (chars[i] == '=' || chars[i] == ':') {
            i += 1; // 分隔符
        }
        while i < chars.len() && is_props_ws(chars[i]) {
            i += 1;
        }
        let value: String = chars[i..].iter().collect(); // 值尾空白保留
        map.insert(unescape(&key), unescape(&value));
    }
    map
}

/// Properties 单遍解转义; 未知转义取字符本身 (Java 规范行为)
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some('f') => out.push('\u{c}'),
            Some('u') => {
                let hex: String = it.by_ref().take(4).collect();
                let cp = u32::from_str_radix(&hex, 16)
                    .unwrap_or_else(|_| panic!("Malformed \\uXXXX 转义: \\u{hex}"));
                out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[test]
fn get_value_hit_and_miss() {
    assert_eq!(config_get_value("appName"), "VoidMei");
    assert_eq!(config_get_value("httpHeader"), "\n"); // 文件值 \n → 真实换行 (oracle)
    assert_eq!(config_get_value("eMagneto"), ""); // 存在但值为空
    assert_eq!(config_get_value("__no_such_key__"), ""); // 缺失 → ""
}

#[test]
fn table_size_and_unique_sorted() {
    // 历史基线: cur.properties 加载后共 362 键, 无重复
    // (源文件改动需重新生成本表, 由下方对拍测试强制)
    // 波20 清场: oSkeyWord1/2 (OtherService 专用, 未接线) 已删 → 360
    assert_eq!(LANGUAGE_PROPERTIES.len(), 360);
    let mut seen = std::collections::HashSet::new();
    for (k, _) in LANGUAGE_PROPERTIES {
        assert!(seen.insert(*k), "重复键: {k}");
    }
    assert!(LANGUAGE_PROPERTIES.windows(2).all(|w| w[0].0 < w[1].0));
}

#[test]
fn ideographic_space_alignment_preserved() {
    // 基线: mP1TempNotificationBlank = 36 个 U+3000 (对齐占位串)
    let v = config_get_value("mP1TempNotificationBlank");
    assert_eq!(v.chars().count(), 36);
    assert!(v.chars().all(|c| c == '\u{3000}'));
    // 基线: mP4FMPanelBlank = 19 个 U+3000 + 20 个 ASCII 空格 (尾部空白保留)
    let v = config_get_value("mP4FMPanelBlank");
    assert_eq!(v.chars().count(), 39);
    assert!(v.starts_with("\u{3000}\u{3000}\u{3000}"));
    assert!(v.ends_with("                    "));
}

#[test]
fn properties_escape_semantics() {
    // 基线: 文件值 `...？\\n此操作...` → 字面 反斜杠+n, 不是换行
    assert_eq!(
        config_get_value("mResetConfirmContent"),
        "确定要重置所有配置项吗？\\n此操作不可撤销。"
    );
    // 基线: aboutcontent 以 \n\r 转义结尾 → 真实 CR LF
    assert!(config_get_value("aboutcontent").ends_with("\n\r"));
    // 基线: noblkx 分隔符后的前导空格被 Properties 跳过
    assert_eq!(
        config_get_value("noblkx"),
        "找不到blkx文件\n请使用最新WT拆包aces.vromfs.bin"
    );
}

/// 快照对拍 (两轮审查共同警告的漂移守护): 本表必须等于源文件
/// `lang/cur.properties` 经 Properties 加载后的键值集 —— 源文件改动而
/// 未再生快照时, 此处按键给出差异并失败, 而非静默与 Java 行为漂移。
#[test]
fn table_matches_cur_properties_source() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("lang")
        .join("cur.properties");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {} 失败: {e} — 对拍需要仓库内源文件", path.display()));
    let parsed = load_java_properties(&text);

    let table: BTreeMap<&str, &str> = LANGUAGE_PROPERTIES.iter().copied().collect();
    assert_eq!(table.len(), LANGUAGE_PROPERTIES.len(), "快照存在重复键");

    let mut drift: Vec<String> = Vec::new();
    for (k, tv) in &table {
        match parsed.get(*k) {
            None => drift.push(format!("快照多出键 {k:?} (源文件已无此键)")),
            Some(fv) if fv.as_str() != *tv => {
                drift.push(format!("键 {k:?} 值不一致: 文件={fv:?} 快照={tv:?}"));
            }
            _ => {}
        }
    }
    for k in parsed.keys() {
        if !table.contains_key(k.as_str()) {
            drift.push(format!("源文件多出键 {k:?} (快照未再生)"));
        }
    }
    assert!(
        drift.is_empty(),
        "table.rs 与 lang/cur.properties 漂移:\n{}",
        drift.join("\n")
    );
}
