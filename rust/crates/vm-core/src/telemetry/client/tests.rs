use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

/// 本地 mock 服务器: 接受 n 个连接, 每连接收满请求 (读间隙 100ms 超时)
/// 后以 f(req) 的返回值响应并关闭。
/// 波20: 请求方是 ureq (标准 HTTP/1.1), 响应带 Connection: close 禁用连接复用,
/// 每请求一个新连接与 serve_n 的 accept 循环对齐。
fn serve_n<F>(n: usize, f: F) -> (SocketAddr, std::thread::JoinHandle<()>)
where
    F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定测试端口失败");
    let addr = listener.local_addr().unwrap();
    let f = std::sync::Arc::new(f);
    let h = std::thread::spawn(move || {
        for _ in 0..n {
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(_) => break,
            };
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .ok();
            let mut req = Vec::new();
            let mut b = [0u8; 4096];
            loop {
                match stream.read(&mut b) {
                    Ok(0) => break,
                    Ok(k) => {
                        req.extend_from_slice(&b[..k]);
                        if req.len() > 65536 {
                            break;
                        }
                    }
                    Err(_) => break, // 读间隙超时 → 请求收满
                }
            }
            let resp = f(&req);
            stream.write_all(&resp).ok();
        }
    });
    (addr, h)
}

fn refused_addr() -> SocketAddr {
    // 绑定即释放 → 端口上无监听 → 连接被拒 (localhost 即时 RST)
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let a = l.local_addr().unwrap();
    drop(l);
    a
}

// ---- get_req_result: 顺序取数 + 失败复位/恢复 ----

#[test]
fn get_req_result_populates_state_and_indic() {
    let state_body = "{\"valid\":true,\"alt\":500}";
    let indic_body = "{\"valid\":true,\"speed\":42.0}";
    let state_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        state_body.len(),
        state_body
    );
    let indic_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        indic_body.len(),
        indic_body
    );
    let (addr, h) = serve_n(2, move |req| {
        if std::str::from_utf8(req).is_ok_and(|s| s.contains("/state")) {
            state_resp.clone().into_bytes()
        } else {
            indic_resp.clone().into_bytes()
        }
    });
    let mut c = GameApiClient::new("\n");
    c.get_req_result(addr);
    assert!(c.str_state.lock().unwrap().contains("\"alt\":500"));
    assert!(c.str_indic.contains("\"speed\":42.0"));
    h.join().unwrap();
}

#[test]
fn get_req_result_injects_extra_header() {
    // httpHeader 行串 → 标准 header 注入 (服务侧验收)
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen2 = std::sync::Arc::clone(&seen);
    let (addr, h) = serve_n(2, move |req| {
        *seen2.lock().unwrap() = req.to_vec();
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_vec()
    });
    let mut c = GameApiClient::new("User-Agent: VoidMei-Test\r\n");
    c.get_req_result(addr);
    h.join().unwrap();
    let got = String::from_utf8(seen.lock().unwrap().clone()).unwrap();
    assert!(got.contains("User-Agent: VoidMei-Test"), "httpHeader 应注入: {got}");
}

#[test]
fn get_req_result_refused_sets_nstring() {
    let mut c = GameApiClient::new("\n");
    c.get_req_result(refused_addr());
    assert_eq!(*c.str_state.lock().unwrap(), NSTRING);
    assert_eq!(c.str_indic, NSTRING);
}

#[test]
fn get_req_result_recovers_after_failed_round() {
    // 8111 瞬断后完全恢复: 第 1 轮被拒复位 → 第 2 轮健康服务恢复取数
    let mut c = GameApiClient::new("\n");
    c.get_req_result(refused_addr());
    assert_eq!(*c.str_state.lock().unwrap(), NSTRING);
    assert_eq!(c.str_indic, NSTRING);

    let state_body = "{\"valid\":true,\"alt\":999}";
    let state_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        state_body.len(),
        state_body
    );
    let indic_body = "{\"valid\":true,\"speed\":7.0}";
    let indic_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        indic_body.len(),
        indic_body
    );
    let (addr, h) = serve_n(2, move |req| {
        if std::str::from_utf8(req).is_ok_and(|s| s.contains("/state")) {
            state_resp.clone().into_bytes()
        } else {
            indic_resp.clone().into_bytes()
        }
    });
    c.get_req_result(addr);
    assert!(c.str_state.lock().unwrap().contains("\"alt\":999"));
    assert!(c.str_indic.contains("\"speed\":7.0"));
    h.join().unwrap();
}

#[test]
fn get_req_map_info_result_refused_sets_nstring() {
    let mut c = GameApiClient::new("\n");
    c.get_req_map_info_result(refused_addr());
    assert_eq!(c.str_map_info, NSTRING);
}

#[test]
fn get_req_map_obj_result_populates() {
    let body = "[{\"icon\": \"Player\", \"x\": 0.5, \"y\": 0.6}]";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let (addr, h) = serve_n(1, move |_| resp.clone().into_bytes());
    let mut c = GameApiClient::new("\n");
    c.get_req_map_obj_result(addr);
    assert!(c.str_map_obj.contains("Player"));
    h.join().unwrap();
}

// ---- get_live_aircraft_type (端口注入 — 随机 mock 端口, 不再占用真实 8111) ----

#[test]
fn get_live_aircraft_type_parses_indicators() {
    let body = "{\"valid\": true, \"type\": \"bf-109f-4\"}";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let (addr, h) = serve_n(1, move |_| resp.clone().into_bytes());
    let c = GameApiClient::new("\n");
    assert_eq!(
        c.get_live_aircraft_type(addr.port()).as_deref(),
        Some("bf-109f-4")
    );
    h.join().unwrap();
}

#[test]
fn get_live_aircraft_type_invalid_returns_none() {
    let body = "{\"valid\": false}";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let (addr, h) = serve_n(1, move |_| resp.clone().into_bytes());
    let c = GameApiClient::new("\n");
    assert_eq!(c.get_live_aircraft_type(addr.port()), None);
    h.join().unwrap();
}

#[test]
fn get_live_aircraft_type_no_server_returns_none() {
    let c = GameApiClient::new("\n");
    // 随机死端口 (绑定即释放) → 连接被拒 → None
    assert_eq!(c.get_live_aircraft_type(refused_addr().port()), None);
}
