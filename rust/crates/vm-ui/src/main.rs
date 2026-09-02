//! vm-ui bin: headless-only 状态机驱动入口 (D9: 窗口形态归 vm-webui web 壳,
//! 原 iced 窗口启动段已删)。
//!
//! 用法: `vm-ui --headless [--persist <path>]` — 无 `--headless` 时打印用法退出 1。
//! 生产链 (表单窗口 + 主循环) 见 vm-app (组装 bin)。

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--headless") {
        // --persist <path>: 固定序列落盘 (D9 换框架基线/diff 验收, 见 run_headless 文档)
        let persist = args
            .iter()
            .position(|a| a == "--persist")
            .and_then(|i| args.get(i + 1))
            .cloned();
        std::process::exit(vm_ui::main_form::headless::run_headless(persist));
    }
    eprintln!("用法: vm-ui --headless [--persist <path>]");
    eprintln!("D9: 设置窗 (MainForm) 渲染已归 vm-webui web 壳 (vm-app 组装), 本 bin 仅作无窗口状态机验证");
    std::process::exit(1);
}
