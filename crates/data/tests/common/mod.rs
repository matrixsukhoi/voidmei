//! data 集成测试公共基建: mini-8111 mock 服务器 + 等待助手。
//!
//! 黑盒原则: Service 经 `ServiceConfig.app_port` 指向本地 mock, 走真 ureq
//! HTTP 链路 (连接失败/超时/复位正是被测行为), 不注入内部状态。
//! mock body 优先读 `script/mock_scenarios/snapshots/*.json` 真机快照
//! (与打桩调试手册的 mock_8111.py 同源, 防双份漂移)。
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 四端点响应体集 (path → body); refuse = 一律不回写直接断连 (连接层故障)
#[derive(Default)]
struct ServerState {
    bodies: HashMap<String, String>,
    refuse: bool,
}

/// 进程内 mini-8111: 双端口 (P + P+1111) 同时监听, 按 path 回放 body。
/// 主备双端口是 Service 端口翻转语义的前提 (bkp = app_port + 1111)。
pub struct MockServer {
    /// 主端口 (ServiceConfig.app_port 指这里)
    pub port: u16,
    state: Arc<Mutex<ServerState>>,
    /// 持有双 listener (drop 即停)
    _listeners: [TcpListener; 2],
}

impl MockServer {
    /// 起服务: 随机循环直到 P 与 P+1111 都 bind 成功 (消备端口撞车)。
    /// 并行测试下 P+1111 会被兄弟测试的主端口抢占 → 失败随机退避重试
    /// (无退避的紧循环在 7 测试并发下会瞬间耗尽尝试次数, 实测全灭)
    pub fn start() -> MockServer {
        let state = Arc::new(Mutex::new(ServerState::default()));
        // 原子计数器混出的伪随机 jitter 源 (禁 rand 依赖; 只需打散争用即可)
        static START_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let mut jit = START_N
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            .wrapping_mul(6364136223846793005)
            | 1;
        for i in 0..200 {
            if i > 0 {
                jit = jit.wrapping_mul(6364136223846793005).wrapping_add(1);
                std::thread::sleep(Duration::from_millis(1 + jit % 15));
            }
            let a = TcpListener::bind(("127.0.0.1", 0)).expect("mock bind");
            let pa = a.local_addr().unwrap().port();
            if pa > 60000 {
                continue; // P+1111 越界, 换一个
            }
            let Ok(b) = TcpListener::bind(("127.0.0.1", pa + 1111)) else {
                continue;
            };
            for l in [&a, &b] {
                spawn_accept(l.try_clone().expect("listener clone"), Arc::clone(&state));
            }
            return MockServer {
                port: pa,
                state,
                _listeners: [a, b],
            };
        }
        panic!("200 次尝试内未凑齐双端口");
    }

    /// 装载真机快照 (script/mock_scenarios/snapshots/<name>.json, 四端点全量)
    pub fn set_snapshot(&self, name: &str) {
        let path = format!(
            "{}/../../script/mock_scenarios/snapshots/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读快照 {path} 失败 (打桩数据缺失?): {e}"));
        let v: serde_json::Value = serde_json::from_str(&raw).expect("快照非 JSON");
        let mut st = self.state.lock().unwrap();
        st.bodies.clear();
        if let serde_json::Value::Object(map) = v {
            for (k, body) in map {
                st.bodies.insert(k, body.to_string());
            }
        }
        st.refuse = false;
    }

    /// 全端点 404 (游戏在场但端点异常)
    pub fn set_not_found(&self) {
        let mut st = self.state.lock().unwrap();
        st.bodies.clear();
        st.refuse = false;
    }

    /// 连接层故障 (游戏关闭: accept 后直接断连, 客户端读 EOF)
    pub fn set_refuse(&self) {
        self.state.lock().unwrap().refuse = true;
    }

    /// 收到的请求数 (按 path 计数, 断言轮询活性用)
    pub fn request_counts(&self) -> HashMap<String, u32> {
        REQUEST_COUNTS
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default()
    }
}

/// 全局请求计数 (跨连接累计; 测试二进制单进程, 无串扰)
static REQUEST_COUNTS: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);

fn count_request(path: &str) {
    let mut m = REQUEST_COUNTS.lock().unwrap();
    m.get_or_insert_with(HashMap::new)
        .entry(path.to_string())
        .and_modify(|c| *c += 1)
        .or_insert(1);
}

fn spawn_accept(listener: TcpListener, state: Arc<Mutex<ServerState>>) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { break };
            let st = Arc::clone(&state);
            std::thread::spawn(move || serve_conn(stream, st));
        }
    });
}

/// 单连接: 读请求头 → 按 path 回 body → 关闭 (Connection: close 语义)
fn serve_conn(mut stream: TcpStream, state: Arc<Mutex<ServerState>>) {
    let mut buf = [0u8; 4096];
    let mut req = Vec::new();
    // 读到请求头终止 (GET 无 body)
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                req.extend_from_slice(&buf[..n]);
                if req.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                if req.len() > 64 * 1024 {
                    return; // 头部异常膨胀, 丢弃
                }
            }
        }
    }
    let head = String::from_utf8_lossy(&req);
    let path = head
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string();
    count_request(&path);

    let (body, status) = {
        let st = state.lock().unwrap();
        if st.refuse {
            return; // 直接断连: 客户端读到 EOF = 失败
        }
        match st.bodies.get(&path) {
            Some(b) => (b.clone(), "200 OK"),
            None => (String::new(), "404 Not Found"),
        }
    };
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

// ---------------------------------------------------------------------
// 等待助手 (统一 5s deadline / 50ms 轮询, 禁固定 sleep — CI 慢机时序防御)
// ---------------------------------------------------------------------

/// 轮询等待谓词为真 (5s 上限; 超时 panic 带上下文)
pub fn poll_until(desc: &str, f: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("等待超时: {desc} (5s)");
}
