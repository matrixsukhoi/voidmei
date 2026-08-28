//! vm-app bin 的 Windows manifest (D9): tauri (vm-webui) 的 comctl32 subclass 系
//! import 要求 common-controls v6 — 无 manifest 时 loader 按 v5 解析, 加载期即
//! 0xC0000139 (vm-app-probe 实证)。
//!
//! 两条落地腿 (语音子系统装配批补齐第 2 腿):
//! 1. **嵌入腿 (本批新增, 主修复)**: windres 把 `1 24 "app.manifest"` 编成 COFF
//!    对象, 经 `cargo:rustc-link-arg` 链入本包全部可执行件 — **含 deps/ 下哈希名
//!    测试 exe**。外部同名 .manifest 文件方案对测试 exe 不可靠 (实测: 同字节 exe
//!    换名后 beside-manifest 生效, 原哈希名不生效 — SxS 按路径缓存失败的激活
//!    上下文, 首次无清单加载后补文件不回溯), 嵌入资源使激活上下文随 exe 自带,
//!    不受外部文件/缓存影响。windres 缺失时降级回外部拷贝腿并 cargo:warning
//!    (测试 exe 将无法加载, 不静默)。
//! 2. **外部拷贝腿 (D9 原方案, 保留)**: 把 app.manifest 拷到 target 下 voidmei.exe
//!    旁 (Windows loader 对 exe 同名 .manifest 的外部清单原生支持, 与嵌入等价;
//!    两者共存时内容相同无行为差)。
fn main() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("app.manifest");
    println!("cargo:rerun-if-changed={}", src.display());
    // OUT_DIR = target/<profile>/build/vm-app-<hash>/out; 上溯 4 级到 target/<profile>
    let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
    let profile_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| "target/debug".into());
    let dst = profile_dir.join("voidmei.exe.manifest");
    if let Err(e) = std::fs::copy(&src, &dst) {
        println!("cargo:warning=manifest 拷贝失败 {}: {e}", dst.display());
    }
    embed_manifest_via_windres(&src);
}

/// 嵌入腿: rc (资源 ID 1 / 类型 24=RT_MANIFEST) → windres COFF → link-arg。
/// 成功即对后续全部链接生效 (bin/test/example); 失败降级 (见模块头)。
fn embed_manifest_via_windres(manifest_src: &std::path::Path) {
    let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
    if out_dir.is_empty() {
        return;
    }
    let out_dir = std::path::PathBuf::from(out_dir);
    // rc 内引用相对路径 — 清单拷进 OUT_DIR, windres 以其为 cwd
    let local_manifest = out_dir.join("app.manifest");
    if let Err(e) = std::fs::copy(manifest_src, &local_manifest) {
        println!("cargo:warning=manifest 拷入 OUT_DIR 失败: {e}");
        return;
    }
    let rc = out_dir.join("manifest.rc");
    if let Err(e) = std::fs::write(&rc, "1 24 \"app.manifest\"\n") {
        println!("cargo:warning=manifest.rc 写入失败: {e}");
        return;
    }
    let obj = out_dir.join("manifest.o");
    let ok = std::process::Command::new("windres")
        // --input-format=rc --output-format=coff (默认即 rc/coff, 显式声明)
        .arg("--input")
        .arg(&rc)
        .arg("--output")
        .arg(&obj)
        .arg("--input-format=rc")
        .arg("--output-format=coff")
        .current_dir(&out_dir)
        .output();
    match ok {
        Ok(o) if o.status.success() => {
            // rustc-link-arg: 作用于本包全部可执行件链接 (bin/test/example)
            println!("cargo:rustc-link-arg={}", obj.display());
        }
        Ok(o) => {
            println!(
                "cargo:warning=windres 失败 ({}), manifest 嵌入腿降级 — 测试 exe 将无法加载 (comctl32 v6 缺失): {}",
                o.status,
                String::from_utf8_lossy(&o.stderr)
            );
        }
        Err(e) => {
            println!(
                "cargo:warning=windres 不可用 ({e}), manifest 嵌入腿降级 — 测试 exe 将无法加载 (comctl32 v6 缺失)"
            );
        }
    }
}
