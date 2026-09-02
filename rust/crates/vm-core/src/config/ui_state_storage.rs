//! prog.util.UIStateStorage 依赖桩 — **不是** UIStateStorage 的翻译。
//! (波16 E5 自 config_manager.rs 尾部抽出, md5.rs 抽出同款先例;
//! ui_model/config_stub.rs 桩先例; 仅覆盖 ConfigManager 消费面
//! loadTemplateHash/saveTemplateHash 两个方法)
//!
//! TODO(ui_state_storage): 真实现落地后删除本文件, config_manager 的
//! initialize() 两处调用切换到 `crate::ui_state_storage::{load_template_hash, save_template_hash}`。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::base::logger;

/// Java UIStateStorage.APP_NAME / STATE_FILE / KEY_TEMPLATE_HASH 原值
pub(crate) const UI_STATE_APP_NAME: &str = "voidmei";
const UI_STATE_FILE: &str = "ui_state.properties";
const UI_STATE_KEY_TEMPLATE_HASH: &str = "templateConfigHash";

/// 测试注入点: ui_state 目录覆盖 (Java 无此面 — 否则单测会写穿开发者真实
/// %APPDATA%/voidmei)。对齐 config_loader::set_legacy_screen_size 注入先例。
#[cfg(test)]
static UI_STATE_DIR_OVERRIDE: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

/// 测试注入 ui_state 目录 (Drop 守卫型沙箱配套)
#[cfg(test)]
pub(crate) fn set_ui_state_dir_override(dir: Option<PathBuf>) {
    *UI_STATE_DIR_OVERRIDE.write().unwrap() = dir;
}

/// Java UIStateStorage.getConfigDir() 的路径规则:
/// Windows: %APPDATA%\\voidmei (无 APPDATA 则 ~\\.voidmei);
/// Linux: $XDG_CONFIG_HOME/voidmei (空/缺省则 ~/.config/voidmei);
/// 其余 (macOS): ~/.voidmei。
/// PORT: Java `System.getProperty("os.name")` 运行期判定 ↔ Rust cfg! 编译期目标
/// 三平台二进制等价; `user.home` ↔ USERPROFILE/HOME 环境变量。
/// PORT: Java getenv 判 null (空串可用) ↔ env::var().ok() 只滤未设置; XDG 的
/// `!= null && !isEmpty()` ↔ ok().filter非空 — 两处判定严格对齐 Java。
pub(crate) fn ui_state_config_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(d) = UI_STATE_DIR_OVERRIDE.read().unwrap().as_ref() {
        return d.clone();
    }

    let user_home = || -> String {
        let v = if cfg!(windows) { env::var("USERPROFILE") } else { env::var("HOME") };
        v.unwrap_or_else(|_| ".".to_string())
    };
    // PORT: Java 是字符串拼接 `base + File.separator + tail` — 基座为空串时
    // (如 APPDATA="") 得 "\voidmei" (当前盘根的绝对路径); PathBuf::from("").join(tail)
    // 会折叠成相对路径 voidmei, 故同样以拼接复刻
    let join = |base: String, tail: String| -> String {
        format!("{base}{}{tail}", std::path::MAIN_SEPARATOR)
    };

    if cfg!(windows) {
        if let Ok(app_data) = env::var("APPDATA") {
            return PathBuf::from(join(app_data, UI_STATE_APP_NAME.to_string()));
        }
        PathBuf::from(join(user_home(), format!(".{UI_STATE_APP_NAME}")))
    } else if cfg!(target_os = "linux") {
        if let Some(xdg) = env::var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty()) {
            return PathBuf::from(join(xdg, UI_STATE_APP_NAME.to_string()));
        }
        PathBuf::from(join(join(user_home(), ".config".to_string()), UI_STATE_APP_NAME.to_string()))
    } else {
        // macOS or others
        PathBuf::from(join(user_home(), format!(".{UI_STATE_APP_NAME}")))
    }
}

/// Java UIStateStorage.getConfigFile(): 目录不存在则 mkdirs (读路径同样建目录,
/// 原行为如此), 返回 <dir>/ui_state.properties。
pub(crate) fn ui_state_config_file() -> PathBuf {
    let dir = ui_state_config_dir();
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir.join(UI_STATE_FILE)
}

/// java.util.Properties.load 的兼容解析 (桩自用):
/// '#'/'!' 注释行、空行跳过; key 以未转义空白(' '/'\t'/'\f')/':'/'=' 终止; 值前
/// 分隔符与空白剥离; 行尾奇数 '\\' 续行 (续行首白空间丢弃, Properties.load 规范)。
/// \\n\\t\\r\\f 与 \\uXXXX 反转义 (JDK 常规转义面)。按字节索引切分 (键 ASCII 域安全)。
fn ui_state_parse_properties(text: &str) -> Vec<(String, String)> {
    let physical: Vec<&str> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();

    let count_trailing_backslashes = |s: &str| -> usize {
        s.bytes().rev().take_while(|&b| b == b'\\').count()
    };

    let unescape = |s: &str| -> String {
        let mut out = String::new();
        let mut it = s.chars();
        while let Some(c) = it.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('f') => out.push('\u{c}'),
                Some('u') => {
                    // \uXXXX; Java 对非 4 位 hex 抛 IllegalArgumentException → 外层
                    // catch 返回 null, 桩从简取字面 'u' (域内键值均 ASCII, 不可达)
                    let hex: String = it.clone().take(4).collect();
                    if hex.len() == 4 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                        for _ in 0..4 {
                            it.next();
                        }
                        let cp = u32::from_str_radix(&hex, 16).unwrap();
                        // Java String 可存孤立代理项, Rust char 不能 — 以 U+FFFD 顶替
                        out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                    } else {
                        out.push('u');
                    }
                }
                Some(other) => out.push(other), // \\ → \, \: → : (Java: 保留字符原样)
                None => out.push('\\'),         // 行尾悬挂反斜杠 (域内不可达)
            }
        }
        out
    };

    let mut entries = Vec::new();
    let mut i = 0;
    while i < physical.len() {
        let mut logical = physical[i].to_string();
        i += 1;
        // 续行: 行尾奇数个 '\', 下一自然行首白空间 (' ','\t','\f') 被丢弃不拼接
        while count_trailing_backslashes(&logical) % 2 == 1 {
            logical.pop();
            if i < physical.len() {
                logical.push_str(physical[i].trim_start_matches([' ', '\t', '\u{c}']));
                i += 1;
            } else {
                break;
            }
        }

        let line = logical.trim_start_matches(|c: char| (c as u32) <= 0x20);
        if line.is_empty() {
            continue;
        }
        let first = line.as_bytes()[0];
        if first == b'#' || first == b'!' {
            continue;
        }

        let b = line.as_bytes();
        let mut j = 0;
        let mut key_end = b.len();
        while j < b.len() {
            let c = b[j];
            if c == b'\\' {
                j += 2; // 转义对不参与分隔判定
                continue;
            }
            if c == b' ' || c == b'\t' || c == 0x0c || c == b':' || c == b'=' {
                key_end = j;
                break;
            }
            j += 1;
        }
        let raw_key = &line[..key_end];

        let mut v = key_end;
        while v < b.len() && (b[v] == b' ' || b[v] == b'\t' || b[v] == 0x0c) {
            v += 1;
        }
        if v < b.len() && (b[v] == b':' || b[v] == b'=') {
            v += 1;
            while v < b.len() && (b[v] == b' ' || b[v] == b'\t' || b[v] == 0x0c) {
                v += 1;
            }
        }
        let raw_val = &line[v..];

        entries.push((unescape(raw_key), unescape(raw_val)));
    }
    entries
}

/// Java Properties.load 按 ISO-8859-1 逐字节读 (任何字节序均合法); fs::read_to_string
/// 的严格 UTF-8 校验会把含原始高位字节的文件打成 Err → 误触合并/重写丢他键。
/// 这里以 Latin-1 解码 (b → char, 无损), 仅 IO 失败走 Err — 对齐 Java 读面。
pub(crate) fn ui_state_read_properties(path: &Path) -> std::io::Result<Vec<(String, String)>> {
    let bytes = fs::read(path)?;
    let text: String = bytes.iter().map(|&b| b as char).collect();
    Ok(ui_state_parse_properties(&text))
}

/// java.util.Properties.store 的 saveConvert 对齐: '\\' 与 \t\n\r\f 助记符,
/// <0x20 / >0x7E 转 \\uXXXX (JDK toHex 大写十六进制), "=:# 与空格加反斜杠前缀。
/// 保证 Latin-1 域内字节经写-读往返无损, Java 端 load 语义等价。
fn ui_state_escape_store(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{c}' => out.push_str("\\f"),
            _ if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            '=' | ':' | '#' | ' ' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Java UIStateStorage.loadTemplateHash(): 文件存在则读 properties 返回
/// templateConfigHash (键缺失 → None); 文件不存在 → None。
pub(crate) fn ui_state_load_template_hash() -> Option<String> {
    let file = ui_state_config_file();
    if file.exists() {
        // catch(Exception) → Logger.info("UIStateStorage", ...) 后返回 null
        match ui_state_read_properties(&file) {
            Ok(entries) => {
                for (k, v) in entries {
                    if k == UI_STATE_KEY_TEMPLATE_HASH {
                        return Some(v);
                    }
                }
            }
            Err(e) => {
                logger::info("UIStateStorage", &format!("Failed to load template hash: {e}"));
            }
        }
    }
    None
}

/// Java UIStateStorage.saveTemplateHash(hash): 载入既有键保留他键 → set → store。
/// PORT: Java Properties.setProperty(key, null) 抛 NullPointerException 且
/// ConfigManager 无 catch → None 入参以 panic 复刻 (与 Java 同为调用方崩溃面)。
/// 写出格式 `key=value` 行, 非 ASCII 以 \\uXXXX 转义; Java store 额外写 #日期 行且
/// 行分隔符随平台 — 桩不写日期行/固定 \n (双向 load 语义等价)。
pub(crate) fn ui_state_save_template_hash(hash: Option<&str>) {
    let hash = match hash {
        Some(h) => h,
        None => panic!("java.lang.NullPointerException: Properties.setProperty null value"),
    };

    let file = ui_state_config_file();

    // Load existing to preserve other keys
    // (Java: 载入失败 catch(IOException) 静默忽略 ↔ unwrap_or_default)
    let mut entries = ui_state_read_properties(&file).unwrap_or_default();

    if let Some(e) = entries.iter_mut().find(|(k, _)| k == UI_STATE_KEY_TEMPLATE_HASH) {
        e.1 = hash.to_string();
    } else {
        entries.push((UI_STATE_KEY_TEMPLATE_HASH.to_string(), hash.to_string()));
    }

    let mut out = String::from("#UI State for VoidMei\n");
    for (k, v) in &entries {
        out.push_str(&format!(
            "{}={}\n",
            ui_state_escape_store(k),
            ui_state_escape_store(v)
        ));
    }
    if let Err(e) = fs::write(&file, out) {
        logger::info("UIStateStorage", &format!("Failed to save template hash: {e}"));
    }
}
