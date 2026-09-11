//! 8111 HTTP 客户端黑盒场景 (kernel::game_api::client)。
//! 本地 std TcpListener 起 mini-8111 mock (data/tests/common 的 serve_conn
//! 简化单端口版), 走真 ureq 链路; 连接失败/404 复位正是被测行为。
//! 真机快照 = script/mock_scenarios (mock_8111 同源, 防双份漂移)。
#![allow(non_snake_case)] // 测试名 = 模块前缀英文 + 中文场景 (风格约定, 豁免蛇形)

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use kernel::game_api::client::GameApiClient;

/// 真机 p51d 快照的指定端点 body (mock_8111 打桩同源)
fn snapshot_body(endpoint: &str) -> String {
    let raw = include_str!("../../../script/mock_scenarios/snapshots/plane_p51d.json");
    let v: serde_json::Value = serde_json::from_str(raw).expect("快照非 JSON");
    v[endpoint].to_string()
}

// ---- mini-8111 mock (单端口: GameApiClient 直接吃 SocketAddr) ----

struct ServerState {
    bodies: HashMap<String, String>,
    /// 一律不回写直接断连 (连接层故障面)
    refuse: bool,
}

struct Mini8111 {
    port: u16,
    state: Arc<Mutex<ServerState>>,
}

impl Mini8111 {
    fn start() -> Mini8111 {
        let state = Arc::new(Mutex::new(ServerState {
            bodies: HashMap::new(),
            refuse: false,
        }));
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("mock bind");
        let port = listener.local_addr().unwrap().port();
        let st = Arc::clone(&state);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                let st = Arc::clone(&st);
                std::thread::spawn(move || serve_conn(stream, st));
            }
        });
        Mini8111 { port, state }
    }

    fn addr(&self) -> SocketAddr {
        format!("127.0.0.1:{}", self.port).parse().unwrap()
    }

    /// 真机快照四端点全量装载
    fn set_snapshot(&self) {
        let mut st = self.state.lock().unwrap();
        st.bodies.insert("/state".into(), snapshot_body("/state"));
        st.bodies.insert("/indicators".into(), snapshot_body("/indicators"));
        st.refuse = false;
    }

    /// 全端点 404
    fn set_not_found(&self) {
        let mut st = self.state.lock().unwrap();
        st.bodies.clear();
        st.refuse = false;
    }

    /// 连接层故障 (accept 后直接断连, 客户端读 EOF)
    fn set_refuse(&self) {
        self.state.lock().unwrap().refuse = true;
    }

    /// 覆盖单端点 body (get_live_aircraft_type 变体用)
    fn set_body(&self, path: &str, body: &str) {
        self.state.lock().unwrap().bodies.insert(path.to_string(), body.to_string());
    }
}

/// 单连接: 读请求头 → 按 path 回 body → 关闭 (Connection: close 语义)
fn serve_conn(mut stream: TcpStream, state: Arc<Mutex<ServerState>>) {
    let mut buf = [0u8; 4096];
    let mut req = Vec::new();
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                req.extend_from_slice(&buf[..n]);
                if req.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                if req.len() > 64 * 1024 {
                    return;
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

    let (body, status) = {
        let st = state.lock().unwrap();
        if st.refuse {
            return; // 直接断连: 客户端读 EOF = 失败
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

// ---- 场景 ----

/// 成功链路: 真机快照双端点 200 → str_state/str_indic 填充
#[test]
fn client_get_req_result_成功双串() {
    let mock = Mini8111::start();
    mock.set_snapshot();

    let mut c = GameApiClient::new("\n"); // httpHeader 缺省 "\n" → 无附加头
    c.get_req_result(mock.addr());
    let state = c.str_state.lock().unwrap().clone();
    assert_eq!(state, snapshot_body("/state"), "str_state = /state 响应体原文");
    assert_eq!(c.str_indic, snapshot_body("/indicators"), "str_indic = /indicators 响应体");
    // 附带头注入面: 带 "Name: value" 行串构造不炸
    let mut c2 = GameApiClient::new("X-Test: 1\r\nEmpty:\r\n");
    c2.get_req_result(mock.addr());
    assert!(!c2.str_state.lock().unwrap().is_empty());
}

/// 双串复位: 连接拒绝 (游戏关闭) 与 HTTP 404 都走双双空串复位
/// (调用方的端口翻转/等待分支信号)
#[test]
fn client_失败复位_连接拒绝与404() {
    let mock = Mini8111::start();
    mock.set_snapshot();

    let mut c = GameApiClient::new("\n");
    c.get_req_result(mock.addr());
    assert!(!c.str_state.lock().unwrap().is_empty(), "前置: 已填充");

    // 404: 游戏在场但端点异常
    mock.set_not_found();
    c.get_req_result(mock.addr());
    assert_eq!(*c.str_state.lock().unwrap(), "");
    assert_eq!(c.str_indic, "");

    // 重新可用后恢复
    mock.set_snapshot();
    c.get_req_result(mock.addr());
    assert!(!c.str_state.lock().unwrap().is_empty());

    // 连接层故障: accept 后断连 → 读 EOF 失败 → 复位
    mock.set_refuse();
    c.get_req_result(mock.addr());
    assert_eq!(*c.str_state.lock().unwrap(), "");
    assert_eq!(c.str_indic, "");
}

/// 无监听端口 (连接拒绝): 250ms 连接超时上限内的复位路径
#[test]
fn client_无监听端口复位() {
    // 端口 9 (discard 协议口): 本机无监听, connect 立即 RST — 无需真等超时
    let dead: SocketAddr = "127.0.0.1:9".parse().unwrap();
    let mut c = GameApiClient::new("\n");
    *c.str_state.lock().unwrap() = "stale".into();
    c.str_indic = "stale".into();
    c.get_req_result(dead);
    assert_eq!(*c.str_state.lock().unwrap(), "", "连接拒绝 → str_state 复位");
    assert_eq!(c.str_indic, "", "连接拒绝 → str_indic 复位");
}

/// get_live_aircraft_type: 有效 indicators → 小写机型名;
/// 观战/无效/404 → None (FM-Detect 探测面的三态)
#[test]
fn client_live机型探测() {
    let mock = Mini8111::start();
    mock.set_snapshot();

    let c = GameApiClient::new("\n");
    // 真机快照 type = "P-51D-20_CHINA" (Indicators.update 大写化后)
    assert_eq!(
        c.get_live_aircraft_type(mock.port).as_deref(),
        Some("p-51d-20_china"),
        "valid 机型 → 小写规范名"
    );

    // valid 缺失 (观战形态) → None
    mock.set_body("/indicators", r#"{"type": "x"}"#);
    assert_eq!(c.get_live_aircraft_type(mock.port), None, "缺 valid → None");

    // valid 但 type 空 → None
    mock.set_body("/indicators", r#"{"valid": true, "type": ""}"#);
    assert_eq!(c.get_live_aircraft_type(mock.port), None, "空 type → None");

    // 404 → None
    mock.set_not_found();
    assert_eq!(c.get_live_aircraft_type(mock.port), None, "HTTP 失败 → None");
}
