//! vm-app bin 的 Windows manifest (D9): tauri (vm-webui) 的 comctl32 subclass 系
//! import 要求 common-controls v6 — 无 manifest 时 loader 按 v5 解析, 加载期即
//! 0xC0000139 (vm-app-probe 实证)。mingw 工具链无 /MANIFEST:EMBED, 以外部同名
//! manifest 落地: build 脚本把 app.manifest 拷到 target 下各 bin 旁 (Windows
//! loader 对 exe 同名 .manifest 的外部清单原生支持, 与嵌入等价)。
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
    for bin in ["voidmei.exe.manifest"] {
        let dst = profile_dir.join(bin);
        if let Err(e) = std::fs::copy(&src, &dst) {
            println!("cargo:warning=manifest 拷贝失败 {}: {e}", dst.display());
        }
    }
}
