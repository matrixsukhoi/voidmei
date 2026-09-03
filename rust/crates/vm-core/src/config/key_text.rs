//! jnativehook 键码↔文本映射 (NativeKeyEvent.getKeyText / getKeyCodeFromText)
//! — ConfigLoader 的 :hotkey 装载/写回消费面。
//! (波16 E6 自 config_loader.rs 抽出 — 139 项 VC 码表与配置装载无关,
//! md5.rs 抽出同款先例)

use crate::base::java_compat::java_trim;

/// jnativehook 2.2.2 `NativeKeyEvent.getKeyText` 的 VC 码→文本表 (139 项),
/// bytecode ldc 默认值 + en_US locale 历史基线 全量对拍生成。
/// 原实现经 `Toolkit.getProperty("AWT.xxx", 默认值)` 查 JDK 的 awt.properties —
/// **随 JDK locale 本地化** (zh JDK: 1→"Esc"、54→"未知 keyCode: 0x36"), 本表为
/// 英文 canonical 默认值 (en_US 基线); 中文 JDK 上 Java 侧 hotkey 往返取不到
/// 英文名属环境差异, 非 C 类接线可解 — 已在迁移报告中上报。
fn key_text_table(code: i32) -> Option<&'static str> {
    match code {
        0 => Some("Undefined"),
        1 => Some("Escape"),
        2 => Some("1"),
        3 => Some("2"),
        4 => Some("3"),
        5 => Some("4"),
        6 => Some("5"),
        7 => Some("6"),
        8 => Some("7"),
        9 => Some("8"),
        10 => Some("9"),
        11 => Some("0"),
        12 => Some("Minus"),
        13 => Some("Equals"),
        14 => Some("Backspace"),
        15 => Some("Tab"),
        16 => Some("Q"),
        17 => Some("W"),
        18 => Some("E"),
        19 => Some("R"),
        20 => Some("T"),
        21 => Some("Y"),
        22 => Some("U"),
        23 => Some("I"),
        24 => Some("O"),
        25 => Some("P"),
        26 => Some("Open Bracket"),
        27 => Some("Close Bracket"),
        28 => Some("Enter"),
        29 => Some("Ctrl"),
        30 => Some("A"),
        31 => Some("S"),
        32 => Some("D"),
        33 => Some("F"),
        34 => Some("G"),
        35 => Some("H"),
        36 => Some("J"),
        37 => Some("K"),
        38 => Some("L"),
        39 => Some("Semicolon"),
        40 => Some("Quote"),
        41 => Some("Back Quote"),
        42 => Some("Shift"),
        43 => Some("Back Slash"),
        44 => Some("Z"),
        45 => Some("X"),
        46 => Some("C"),
        47 => Some("V"),
        48 => Some("B"),
        49 => Some("N"),
        50 => Some("M"),
        51 => Some("Comma"),
        52 => Some("Period"),
        53 => Some("Slash"),
        56 => Some("Alt"),
        57 => Some("Space"),
        58 => Some("Caps Lock"),
        59 => Some("F1"),
        60 => Some("F2"),
        61 => Some("F3"),
        62 => Some("F4"),
        63 => Some("F5"),
        64 => Some("F6"),
        65 => Some("F7"),
        66 => Some("F8"),
        67 => Some("F9"),
        68 => Some("F10"),
        69 => Some("Num Lock"),
        70 => Some("Scroll Lock"),
        83 => Some("NumPad ,"),
        87 => Some("F11"),
        88 => Some("F12"),
        91 => Some("F13"),
        92 => Some("F14"),
        93 => Some("F15"),
        99 => Some("F16"),
        100 => Some("F17"),
        101 => Some("F18"),
        102 => Some("F19"),
        103 => Some("F20"),
        104 => Some("F21"),
        105 => Some("F22"),
        106 => Some("F23"),
        107 => Some("F24"),
        112 => Some("Katakana"),
        115 => Some("Underscore"),
        119 => Some("Furigana"),
        121 => Some("Kanji"),
        123 => Some("Hiragana"),
        125 => Some("¥"),
        3639 => Some("Print Screen"),
        3653 => Some("Pause"),
        3655 => Some("Home"),
        3657 => Some("Page Up"),
        3663 => Some("End"),
        3665 => Some("Page Down"),
        3666 => Some("Insert"),
        3667 => Some("Delete"),
        3675 => Some("Meta"),
        3677 => Some("Context Menu"),
        57360 => Some("Previous"),
        57369 => Some("Next"),
        57376 => Some("Mute"),
        57377 => Some("App Calculator"),
        57378 => Some("Play"),
        57380 => Some("Stop"),
        57388 => Some("Eject"),
        57390 => Some("Volume Down"),
        57392 => Some("Volume Up"),
        57394 => Some("Browser Home"),
        57404 => Some("App Music"),
        57416 => Some("Up"),
        57419 => Some("Left"),
        57420 => Some("Clear"),
        57421 => Some("Right"),
        57424 => Some("Down"),
        57438 => Some("Power"),
        57439 => Some("Sleep"),
        57443 => Some("Wake"),
        57444 => Some("App Pictures"),
        57445 => Some("Browser Search"),
        57446 => Some("Browser Favorites"),
        57447 => Some("Browser Refresh"),
        57448 => Some("Stop"),
        57449 => Some("Browser Forward"),
        57450 => Some("Browser Back"),
        57452 => Some("App Mail"),
        57453 => Some("Select"),
        65396 => Some("Sun Open"),
        65397 => Some("Sun Help"),
        65398 => Some("Sun Props"),
        65399 => Some("Sun Front"),
        65400 => Some("Sun Stop"),
        65401 => Some("Sun Again"),
        65402 => Some("Sun Undo"),
        65403 => Some("Sun Cut"),
        65404 => Some("Sun Copy"),
        65405 => Some("Sun Insert"),
        65406 => Some("Sun Find"),
        _ => None,
    }
}

/// Java: `NativeKeyEvent.getKeyText(int keyCode)`
pub fn get_key_text(key_code: i32) -> String {
    if let Some(t) = key_text_table(key_code) {
        return t.to_string();
    }
    // Java default 分支: getProperty("AWT.unknown", "Unknown") + " keyCode: 0x" +
    // Integer.toString(keyCode, 16) — 负数带 '-' 前缀 (有符号幅值), 十六进制小写无补零
    let hex = if key_code < 0 {
        format!("-{:x}", (i64::from(key_code)).unsigned_abs())
    } else {
        format!("{:x}", key_code)
    };
    format!("Unknown keyCode: 0x{hex}")
}

/// Attempts to resolve key code from string (either "P" or "25")
pub(crate) fn get_key_code_from_text(text: Option<&str>) -> i32 {
    let Some(text) = text else {
        return 0;
    };
    if java_trim(text).is_empty() {
        return 0;
    }
    let t = java_trim(text);
    // 1. Try numeric
    // 波21: Integer.parseInt 复刻退役, std parse
    match t.parse::<i32>() {
        Ok(i) => i,
        Err(_) => {
            // 2. Brute force lookup in common JNativeHook VC codes (typically < 256)
            for i in 1..256i32 {
                // Java equalsIgnoreCase — 键名域 ASCII, eq_ignore_ascii_case 等价
                if get_key_text(i).eq_ignore_ascii_case(t) {
                    return i;
                }
            }
            0
        }
    }
}
