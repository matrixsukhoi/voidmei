use super::*;

/// 边界测试: get_file_name_no_ex (期望值经 历史基线 对拍)
#[test]
fn get_file_name_no_ex_boundaries() {
    // null → null
    assert_eq!(get_file_name_no_ex(None), None);
    // 空串原样返回
    assert_eq!(get_file_name_no_ex(Some("")), Some(""));
    // 无点原样返回
    assert_eq!(get_file_name_no_ex(Some("abc")), Some("abc"));
    // 常规去扩展名
    assert_eq!(get_file_name_no_ex(Some("a.txt")), Some("a"));
    // 点在首位 → 空串
    assert_eq!(get_file_name_no_ex(Some(".hidden")), Some(""));
    // 多个点只截最后一个
    assert_eq!(get_file_name_no_ex(Some("a.b.c")), Some("a.b"));
    // 尾部点
    assert_eq!(get_file_name_no_ex(Some("abc.")), Some("abc"));
    // 不处理路径分隔符, 只认最后一个 '.'
    assert_eq!(
        get_file_name_no_ex(Some("dir/file.name.ext")),
        Some("dir/file.name")
    );
    assert_eq!(
        get_file_name_no_ex(Some("my.file.v1.bin")),
        Some("my.file.v1")
    );
    // CJK: Java 按码元切 / Rust 按字节切, 落在同一字符边界
    assert_eq!(get_file_name_no_ex(Some("文件.tar")), Some("文件"));
    assert_eq!(get_file_name_no_ex(Some("文件名")), Some("文件名"));
}

/// 边界测试: get_filelist_name_no_ex
#[test]
fn get_filelist_name_no_ex_boundaries() {
    // null 数组 → 空数组
    assert!(get_filelist_name_no_ex(None).is_empty());
    // 空数组 → 空数组
    assert!(get_filelist_name_no_ex(Some(&[])).is_empty());
    // 逐元素映射, 保序
    let list = [Some("a.txt"), Some("b"), Some(".hidden")];
    assert_eq!(
        get_filelist_name_no_ex(Some(&list)),
        vec![Some("a"), Some("b"), Some("")]
    );
    // null 元素透传 (null-in/null-out)
    let list2 = [Some("x.bin"), None];
    assert_eq!(get_filelist_name_no_ex(Some(&list2)), vec![Some("x"), None]);
}
