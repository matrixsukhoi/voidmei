use super::*;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

/// 本地 mock 服务器: 接受 n 个连接, 每连接收满请求 (读间隙 100ms 超时)
/// 后以 f(req) 的返回值响应并关闭。f 内部可 sleep 制造延迟。
fn serve_n<F>(n: usize, f: F) -> (SocketAddr, std::thread::JoinHandle<()>)
where
    F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定测试端口失败");
    let addr = listener.local_addr().unwrap();
    let f = Arc::new(f);
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

fn stop_off() -> AtomicBool {
    AtomicBool::new(false)
}

// ---- 请求串构造 (Java 字段初始化器的逐字格式) ----
// (波20 清场: set_alt/set_vel 请求模板已随 fm_cmd_* 死方法退役)

#[test]
fn request_strings_exact_format() {
    // Lang.httpHeader 缺省 "\n" (lang/table.rs oracle)
    let h = HttpHelper::new("\n");
    assert_eq!(
        h.state_request,
        "GET /state HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n\n\n"
    );
    assert_eq!(
        h.indic_request,
        "GET /indicators HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n\n\n"
    );
    assert_eq!(
        h.mapobj_request,
        "GET /map_obj.json HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n\n\n"
    );
    assert_eq!(
        h.mapinfo_request,
        "GET /map_info.json HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n\n\n"
    );
    // 自定义 httpHeader 注入 (Lang 配置可改写该值)
    let h2 = HttpHelper::new("User-Agent:AppleWebKit/537.88\r\n");
    assert_eq!(
        h2.state_request,
        "GET /state HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\nUser-Agent:AppleWebKit/537.88\r\n\n"
    );
}

// ---- get_req_result: 双并发取数 + 兜底 ----

#[test]
fn get_req_result_populates_state_and_indic() {
    let state_body = "{\"valid\":true,\"alt\":500}";
    let indic_body = "{\"valid\":true,\"speed\":42.0}";
    let state_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        state_body.len(),
        state_body
    );
    let indic_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
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
    let mut hh = HttpHelper::new("\n");
    let stop = stop_off();
    hh.get_req_result(addr, &stop);
    assert!(hh.str_state.lock().unwrap().contains("\"alt\":500"));
    assert!(hh.str_indic.contains("\"speed\":42.0"));
    h.join().unwrap();
}

#[test]
fn get_req_result_refused_sets_nstring() {
    let mut hh = HttpHelper::new("\n");
    let stop = stop_off();
    hh.get_req_result(refused_addr(), &stop);
    assert_eq!(*hh.str_state.lock().unwrap(), NSTRING);
    assert_eq!(hh.str_indic, NSTRING);
}

#[test]
fn get_req_result_recovers_after_failed_round() {
    // Java 原行为: 任务失败时 future 停在 Pending (无 completeExceptionally),
    // 下一轮成功的 /state 任务照常 complete(true) —— 8111 瞬断后完全恢复,
    // 不存在"首败后永久 ExecutionException"的粘性
    let mut hh = HttpHelper::new("\n");
    let stop = stop_off();
    // 第 1 轮: 双双连接被拒 → IO 兜底清空, future 留 Pending
    hh.get_req_result(refused_addr(), &stop);
    assert_eq!(*hh.str_state.lock().unwrap(), NSTRING);
    assert_eq!(hh.str_indic, NSTRING);

    // 第 2 轮: 服务健康 → future 被 complete (此前 Pending), get() 正常返回
    let state_body = "{\"valid\":true,\"alt\":999}";
    let state_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        state_body.len(),
        state_body
    );
    let indic_body = "{\"valid\":true,\"speed\":7.0}";
    let indic_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
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
    hh.get_req_result(addr, &stop);
    // 波5: 同线程顺序取数, 返回后双字段有值
    assert!(hh.str_state.lock().unwrap().contains("\"alt\":999"));
    assert!(hh.str_indic.contains("\"speed\":7.0"));
    h.join().unwrap();
}

#[test]
fn get_req_map_info_result_refused_sets_nstring() {
    let mut hh = HttpHelper::new("\n");
    hh.get_req_map_info_result(refused_addr());
    assert_eq!(hh.str_map_info, NSTRING);
}

// ---- getLiveAircraftType (8111 固定端口; 被占自动跳过, e2e 先例) ----

/// 8111 占用探测: 有服务应答即视为被占 (真机在跑) → 跳过。
/// bind 探测在 Windows SO_REUSEADDR 语义下不可靠 —— 实测真机监听与我方
/// bind 可共存且真机抢答连接, 必须 connect 探测。
fn port_8111_answered() -> bool {
    TcpStream::connect("127.0.0.1:8111").is_ok()
}

/// 8111 相关测试串行锁 (fm::test_guard 同款)。PORT: get_live_aircraft_type
/// 硬编码 127.0.0.1:8111 (Java 保真, 生产段无端口注入面), 三个用例共用 8111 ——
/// 并行时 no_server 的 connect 会抢走 parses/invalid 的唯一 accept 名额
/// (自己拿到 Some 假失败), 服务方随后 connect 被拒返回 None 也假失败, 双双
/// flaky; 持锁串行且探测→bind/connect 同临界区后, no_server 连的 8111 在本
/// 测试二进制内必然空闲。锁中毒无不变量可破, 复取即可 (一次失败不连锁炸后续用例)。
static PORT8111_LOCK: Mutex<()> = Mutex::new(());

fn lock_8111() -> std::sync::MutexGuard<'static, ()> {
    PORT8111_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn get_live_aircraft_type_parses_indicators() {
    let _g = lock_8111();
    if port_8111_answered() {
        eprintln!("跳过: 8111 有服务应答 (真机在跑) —— 同 e2e_fm 约定");
        return;
    }
    let listener = match TcpListener::bind("127.0.0.1:8111") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("跳过: 8111 被占用 ({}) —— 同 e2e_fm 约定", e);
            return;
        }
    };
    let resp = "HTTP/1.1 200 OK\r\n\r\n{\"valid\": true, \"type\": \"bf-109f-4\"}";
    let h = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            stream
                .set_read_timeout(Some(Duration::from_millis(200)))
                .ok();
            let mut b = [0u8; 4096];
            let _ = stream.read(&mut b); // 收请求 (丢弃)
            stream.write_all(resp.as_bytes()).ok();
        }
    });
    let hh = HttpHelper::new("\n");
    assert_eq!(hh.get_live_aircraft_type().as_deref(), Some("bf-109f-4"));
    h.join().unwrap();
}

#[test]
fn get_live_aircraft_type_invalid_returns_none() {
    let _g = lock_8111();
    if port_8111_answered() {
        eprintln!("跳过: 8111 有服务应答 (真机在跑) —— 同 e2e_fm 约定");
        return;
    }
    let listener = match TcpListener::bind("127.0.0.1:8111") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("跳过: 8111 被占用 ({}) —— 同 e2e_fm 约定", e);
            return;
        }
    };
    let resp = "HTTP/1.1 200 OK\r\n\r\n{\"valid\": false}";
    let h = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            stream
                .set_read_timeout(Some(Duration::from_millis(200)))
                .ok();
            let mut b = [0u8; 4096];
            let _ = stream.read(&mut b);
            stream.write_all(resp.as_bytes()).ok();
        }
    });
    let hh = HttpHelper::new("\n");
    assert_eq!(hh.get_live_aircraft_type(), None);
    h.join().unwrap();
}

#[test]
fn get_live_aircraft_type_no_server_returns_none() {
    // 持锁后 8111 在本二进制内必然空闲 (另两用例的 listener 已 join 并
    // drop 才放锁) → connect 必被拒, 语义保持 "连一个必然空闲的端口"
    let _g = lock_8111();
    if port_8111_answered() {
        eprintln!("跳过: 8111 有服务应答 (真机在跑) —— 同 e2e_fm 约定");
        return;
    }
    // 无服务应答 → 连接被拒 → None
    let hh = HttpHelper::new("\n");
    assert_eq!(hh.get_live_aircraft_type(), None);
}
