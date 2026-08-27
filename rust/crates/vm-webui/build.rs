//! tauri 资源构建 (窗口 icon 嵌入; bundle 关闭 — 分发走 build.py, 不用 tauri CLI)
fn main() {
    tauri_build::build()
}
