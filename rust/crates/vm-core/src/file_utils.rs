//! FileUtils 的 Rust 移植 (src/prog/util/FileUtils.java)
//! 文件名去扩展名工具 (ui layout 文件下拉渲染器/机型对比窗口的数据源)。
//!
//! PORT: Java 类仅含 static 方法 → Rust 模块自由函数 (string_helper 先例)。
//! PORT: §2.1 — Java lastIndexOf/substring 按 UTF-16 码元索引, Rust 按字节;
//! '.' 为 ASCII 且不可能出现在 UTF-8 多字节序列内部 (自同步), rfind 的切点
//! 必落在字符边界且与 Java 码元切点是同一字符位置, substring(0, dot) 结果
//! 一致 (Java 8 oracle 实测 "文件.tar" → "文件")。
//! PORT: Java String[]/String 可为 null (本类两个方法都有 null 分支) →
//! Option<&str> 逐位对应, null-in/null-out 保真。

/// 对应 Java `getFilelistNameNoEx(String[] list)`:
/// null → 空 String[0]; 否则逐元素调 get_file_name_no_ex。
pub fn get_filelist_name_no_ex<'a>(list: Option<&'a [Option<&'a str>]>) -> Vec<Option<&'a str>> {
    let list = match list {
        Some(l) => l,
        // Java: if (list == null) return new String[0];
        None => return Vec::new(),
    };
    // Java: String[] a = new String[list.length];
    let mut a: Vec<Option<&str>> = Vec::with_capacity(list.len());
    // Java: for (int i = 0; i < list.length; i++) a[i] = getFileNameNoEx(list[i]);
    for item in list {
        a.push(get_file_name_no_ex(*item));
    }
    a
}

/// 对应 Java `getFileNameNoEx(String filename)`:
/// 截掉最后一个 '.' 及其后的部分; null/空串/无点时原样返回。
pub fn get_file_name_no_ex(filename: Option<&str>) -> Option<&str> {
    // Java: if ((filename != null) && (filename.length() > 0))
    if let Some(filename) = filename {
        if !filename.is_empty() {
            // Java: int dot = filename.lastIndexOf('.');
            if let Some(dot) = filename.rfind('.') {
                // Java: if ((dot > -1) && (dot < (filename.length()))) —
                // rfind 命中即等价 dot > -1; 命中索引必 < len (两语言下均恒真), 保真保留
                if dot < filename.len() {
                    // Java: return filename.substring(0, dot);
                    return Some(&filename[..dot]);
                }
            }
        }
    }
    filename // Java: return filename; — null/空串/无点原样返回
}

#[cfg(test)]
mod tests;
