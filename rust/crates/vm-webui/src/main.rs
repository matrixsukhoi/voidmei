//! vm-webui-selftest bin (D9 阶段① POC 验收):
//! - `--poc-showhide N`: 手动泵长跑, show/hide × N (run_iteration 稳定性)
//! - `--bench-reopen N`: 预热重开延迟 (show() → 前端 WindowEcho 回执), 均值/P95
//! - `--poc-run SECS`: 起窗泵 SECS 秒后干净退出 (共存观察)
//! 全部模式先等前端就绪 (UiReady, 30s 超时), 验证预热链路本身。

use std::thread::sleep;
use std::time::{Duration, Instant};

use vm_webui::ShellForm;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pick = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<u64>().ok());

    let mut form = match ShellForm::new(ShellForm::default_dispatcher()) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("vm-webui-selftest: {e}");
            std::process::exit(2);
        }
    };

    // 预热: 泵至前端就绪 (首启含 WebView2 进程创建, 宽限 30s)
    let t0 = Instant::now();
    while !form.is_web_ready() {
        form.pump_once();
        sleep(Duration::from_millis(5));
        if t0.elapsed() > Duration::from_secs(30) {
            eprintln!("vm-webui-selftest: 前端 30s 未就绪 (WebView2 异常)");
            std::process::exit(3);
        }
    }
    println!(
        "vm-webui-selftest: 前端就绪 (首启预热 {} ms)",
        t0.elapsed().as_millis()
    );

    let code = if let Some(n) = pick("--poc-showhide") {
        poc_showhide(&mut form, n)
    } else if let Some(n) = pick("--bench-reopen") {
        bench_reopen(&mut form, n)
    } else if let Some(secs) = pick("--poc-run") {
        poc_run(&mut form, secs)
    } else {
        eprintln!(
            "用法: vm-webui-selftest --poc-showhide <N> | --bench-reopen <N> | --poc-run <secs>"
        );
        1
    };
    std::process::exit(code);
}

/// POC①: show/hide × N 长跑 (每轮 pump 期间窗口事件被处理, 验证 run_iteration 稳定)
fn poc_showhide(form: &mut ShellForm, n: u64) -> i32 {
    for i in 0..n {
        form.show();
        for _ in 0..20 {
            form.pump_once();
            sleep(Duration::from_millis(5));
        }
        form.hide();
        for _ in 0..10 {
            form.pump_once();
            sleep(Duration::from_millis(5));
        }
        if (i + 1) % 100 == 0 {
            println!("vm-webui-selftest: showhide 进度 {}/{}", i + 1, n);
        }
    }
    println!("vm-webui-selftest: showhide ×{n} 完成, 无 panic/退出请求");
    0
}

/// POC②: 预热重开延迟 (show() → 前端 WindowEcho 回执到达; webview 活性口径)
fn bench_reopen(form: &mut ShellForm, n: u64) -> i32 {
    let mut samples_ms: Vec<u128> = Vec::new();
    for i in 0..n {
        form.reset_echo();
        let t0 = Instant::now();
        form.show();
        // 泵等回执 (5s 超时判失败 — 不做假通过)
        let mut ok = false;
        while t0.elapsed() < Duration::from_secs(5) {
            form.pump_once();
            if form.echo_at().is_some_and(|e| e >= t0) {
                ok = true;
                break;
            }
            sleep(Duration::from_millis(1));
        }
        if !ok {
            // 诊断: 窗口可见性/webview 状态 — 定位回执断在哪一环
            let vis = form.main_window().map(|w| w.is_visible().unwrap_or(false));
            eprintln!(
                "vm-webui-selftest: bench-reopen 第 {}/{} 轮回执 5s 超时 — FAIL (visible={vis:?}, web_ready={})",
                i + 1,
                n,
                form.is_web_ready()
            );
            return 1;
        }
        println!("vm-webui-selftest: run {}: {} ms", i + 1, t0.elapsed().as_millis());
        samples_ms.push(t0.elapsed().as_millis());
        form.hide();
        for _ in 0..20 {
            form.pump_once();
            sleep(Duration::from_millis(5));
        }
    }
    samples_ms.sort_unstable();
    let mean = samples_ms.iter().sum::<u128>() as f64 / samples_ms.len() as f64;
    let p95 = samples_ms[(samples_ms.len() as f64 * 0.95) as usize % samples_ms.len()];
    println!(
        "vm-webui-selftest: bench-reopen ×{n}  均值 {mean:.0} ms  P95 {p95} ms  最大 {} ms",
        samples_ms[samples_ms.len() - 1]
    );
    0
}

/// POC③辅助: 起窗泵 SECS 秒 (与其他子系统共存观察), 到点干净退出
fn poc_run(form: &mut ShellForm, secs: u64) -> i32 {
    form.show();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(secs) {
        form.pump_once();
        sleep(Duration::from_millis(10));
    }
    println!("vm-webui-selftest: poc-run {secs}s 完成");
    0
}
