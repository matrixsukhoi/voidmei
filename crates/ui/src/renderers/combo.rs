//! ComboRowRenderer 的纯数据函数 (选项解析 + 颜色格式化)。
//!
//! 写链已收敛至 main_form::write_control (JSON 配置变更, Phase 1)。

/// 准星选项头部项 (Java: combined[0] = "软件渲染准星")
const SOFTWARE_CROSSHAIR: &str = "软件渲染准星";
/// Java: `File("image/gunsight")` — 相对 CWD
const CROSSHAIR_DIR: &str = "image/gunsight";

/// 解析下拉选项 (Java getComboOptions)。current 仅为 _FONTS_ 占位所需。
pub fn resolve_options(source: &str, current: &str) -> Vec<String> {
    match source {
        "_FONTS_" => vec![current.to_string()],
        "_CROSSHAIRS_" => crosshair_options(CROSSHAIR_DIR),
        // optionSource.split(",") — 空串 → [""] (与 Java split 逐位一致)
        _ => source.split(',').map(str::to_string).collect(),
    }
}

/// 目录条目名去扩展名 + 头部"软件渲染准星"; 目录缺失 → 仅头部。
/// dir 参数仅为测试可注入, 生产恒 [`CROSSHAIR_DIR`]。
pub(crate) fn crosshair_options(dir: &str) -> Vec<String> {
    let mut opts = vec![SOFTWARE_CROSSHAIR.to_string()];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(stripped) = kernel::base::file_utils::get_file_name_no_ex(Some(&name)) {
                opts.push(stripped.to_string());
            }
        }
    }
    opts
}

/// 颜色配置存储格式 (旧 ColorHelper.toDecimalString): "R, G, B, A"。
/// (旧 legacy 分键 fontNumR/G/B/A 写入已随 JSON 化退役 — 全库无读取方。)
pub fn format_rgba_decimal(c: &[u8; 4]) -> String {
    format!("{}, {}, {}, {}", c[0], c[1], c[2], c[3])
}
