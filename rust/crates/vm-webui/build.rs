//! tauri 资源构建 (窗口 icon 嵌入; bundle 关闭 — 分发走 build.py, 不用 tauri CLI)
//! + Windows manifest 嵌入腿 (vm-app/build.rs 同款): tauri-plugin-dialog/opener
//! 的 comctl32 subclass import 要求 common-controls v6, deps/ 下哈希名测试 exe
//! 无嵌入即加载期 0xC0000139 (批3 中断收尾实测) — 外部 .manifest 对哈希名
//! exe 不可靠 (SxS 按路径缓存失败激活上下文), 嵌入资源随 exe 自带才稳。
fn main() {
    // 去 tauri_build 自带 manifest (否则其 -bins 的 libresource 与本腿全局
    // manifest.o 在 bin-test 形态资源 ID 24 重复) — 本腿 windres 嵌入为唯一
    // 来源, 覆盖 bin/lib-unittests/example 全部可执行件
    if let Err(e) = tauri_build::try_build(
        tauri_build::Attributes::default()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest()),
    ) {
        panic!("tauri_build 失败: {e}");
    }
    embed_manifest_via_windres();
}

/// rc (资源 ID 1 / 类型 24=RT_MANIFEST) → windres COFF → link-arg, 链入本包
/// 全部可执行件 (bin/test)。windres 缺失时 cargo:warning 降级, 不静默。
fn embed_manifest_via_windres() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("app.manifest");
    println!("cargo:rerun-if-changed={}", src.display());
    let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
    if out_dir.is_empty() {
        return;
    }
    let out_dir = std::path::PathBuf::from(out_dir);
    // rc 内引用相对路径 — 清单拷进 OUT_DIR, windres 以其为 cwd
    let local = out_dir.join("app.manifest");
    if let Err(e) = std::fs::copy(&src, &local) {
        println!("cargo:warning=manifest 拷入 OUT_DIR 失败: {e}");
        return;
    }
    let rc = out_dir.join("manifest.rc");
    if let Err(e) = std::fs::write(&rc, "1 24 \"app.manifest\"
") {
        println!("cargo:warning=manifest.rc 写入失败: {e}");
        return;
    }
    let obj = out_dir.join("manifest.o");
    let ok = std::process::Command::new("windres")
        .arg("--input").arg(&rc)
        .arg("--output").arg(&obj)
        .arg("--input-format=rc")
        .arg("--output-format=coff")
        .current_dir(&out_dir)
        .output();
    match ok {
        Ok(o) if o.status.success() => {
            println!("cargo:rustc-link-arg={}", obj.display());
        }
        Ok(o) => println!(
            "cargo:warning=windres 失败 ({}), manifest 嵌入降级 — 测试 exe 将无法加载: {}",
            o.status, String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => println!(
            "cargo:warning=windres 不可用 ({e}), manifest 嵌入降级 — 测试 exe 将无法加载"
        ),
    }
}
