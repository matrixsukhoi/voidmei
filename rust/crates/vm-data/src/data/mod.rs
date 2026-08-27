//! 数据层入口: 轮询线程 + 快照共享
//! 对应 Java Service 轮询模型 (50ms/轮, 失败保留上帧)

pub mod derive;
pub mod http;
pub mod json;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use derive::{Deriver, FlightValues};

/// 轮询间隔 (对应 serviceLoopIntervalMs 默认 50)
const POLL_INTERVAL_MS: u64 = 50;
/// 备用端口 = 主端口 + 1111 (Java appPortBkp 行为)
const PORT_BKP_OFFSET: u16 = 1111;

/// 启动轮询线程, 返回快照句柄 (渲染侧 50ms 拉取, 脏检查)
pub fn start_polling(port: u16) -> Arc<Mutex<Option<FlightValues>>> {
    let snapshot = Arc::new(Mutex::new(None));
    let snap2 = Arc::clone(&snapshot);
    std::thread::spawn(move || {
        let mut deriver = Deriver::new(POLL_INTERVAL_MS);
        let mut port = port; // 闭包 move 入参转可变 (Java: port 变量在循环中被改写)
        let primary = port; // Java: 主端口 8111, 备端口 = 主+1111
        let mut last_warn = Instant::now() - Duration::from_secs(10);
        loop {
            let t0 = Instant::now();
            match fetch_and_step(&mut deriver, port) {
                Some(values) => {
                    if let Ok(mut s) = snap2.lock() {
                        *s = Some(values);
                    }
                }
                None => {
                    // 主端口失败翻备用端口 (Java 8111/9222 行为); 失败保留上帧
                    port = if port == primary { primary + PORT_BKP_OFFSET } else { primary };
                    if last_warn.elapsed() >= Duration::from_secs(1) {
                        eprintln!("警告: 8111 数据获取失败, 重试中 (当前端口 {})", port);
                        last_warn = Instant::now();
                    }
                }
            }
            let elapsed = t0.elapsed();
            if elapsed < Duration::from_millis(POLL_INTERVAL_MS) {
                std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS) - elapsed);
            }
        }
    });
    snapshot
}

fn fetch_and_step(deriver: &mut Deriver, port: u16) -> Option<FlightValues> {
    let timeout = Duration::from_millis(POLL_INTERVAL_MS * 4);
    let state_raw = http::http_get(port, "/state", timeout).ok()?;
    let indic_raw = http::http_get(port, "/indicators", timeout).ok()?;
    let st = json::parse_state(&state_raw)?;
    let ind = json::parse_indicators(&indic_raw)?;
    if !ind.valid {
        return None;
    }
    Some(deriver.step(&st, &ind, POLL_INTERVAL_MS as f64))
}
