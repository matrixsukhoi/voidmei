use super::*;
use std::net::TcpListener;

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
            stream.set_read_timeout(Some(Duration::from_millis(100))).ok();
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
    assert_eq!(
        h.fmcm_request,
        "GET /editor/fm_commands?cmd=getFmProperties HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n\n\n"
    );
    assert_eq!(
        h.set_alt_req,
        "GET /editor/fm_commands?cmd=setAlt&value=%d HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n\n\n"
    );
    assert_eq!(
        h.set_vel_req,
        "GET /editor/fm_commands?cmd=setVelocity&value=%.0f HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n\n\n"
    );
    // 自定义 httpHeader 注入 (Lang 配置可改写该值)
    let h2 = HttpHelper::new("User-Agent:AppleWebKit/537.88\r\n");
    assert_eq!(
        h2.state_request,
        "GET /state HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\nUser-Agent:AppleWebKit/537.88\r\n\n"
    );
}

// ---- CompletableFuture 单次完成语义 ----

#[test]
fn completable_future_complete_once_sticky_success() {
    let cf = CompletableFuture::new();
    let stop = stop_off();
    assert!(cf.complete(true));
    assert!(!cf.complete(true), "第二次 complete 必须无操作");
    assert!(!cf.complete_exceptionally(), "已完成后再异常完成也无效");
    assert!(matches!(cf.get(&stop), CfOutcome::Value(true)));
}

#[test]
fn completable_future_exceptional_is_sticky() {
    // Java CompletableFuture 单次完成语义: completeExceptionally 置入后
    // complete 无效、get 抛 ExecutionException。
    // 注: HttpHelper 从不调用 completeExceptionally (worker 失败时 future
    // 停在 Pending), 本语义在 getReqResult 链路不可达 —— 见 get_req_result 注释
    let cf = CompletableFuture::new();
    let stop = stop_off();
    assert!(cf.complete_exceptionally());
    assert!(!cf.complete(true));
    assert!(matches!(cf.get(&stop), CfOutcome::ExecutionException));
}

#[test]
fn completable_future_completed_wins_over_interrupt() {
    // JDK8 waitingGet: result != null 时直接返回值, 不查中断位 ——
    // 已完成的 future 不因停机标志改走 InterruptedException
    let cf = CompletableFuture::new();
    cf.complete(true);
    let stop = AtomicBool::new(true);
    assert!(matches!(cf.get(&stop), CfOutcome::Value(true)));
}

#[test]
fn completable_future_get_blocks_until_completed() {
    let cf = CompletableFuture::new();
    let cf2 = cf.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        cf2.complete(true);
    });
    let stop = stop_off();
    let t0 = std::time::Instant::now();
    assert!(matches!(cf.get(&stop), CfOutcome::Value(true)));
    assert!(
        t0.elapsed() >= Duration::from_millis(60),
        "get 应阻塞至 complete"
    );
}

#[test]
fn completable_future_get_interrupted_when_stop_pre_set() {
    let cf = CompletableFuture::new();
    let stop = AtomicBool::new(true);
    let t0 = std::time::Instant::now();
    assert!(matches!(cf.get(&stop), CfOutcome::InterruptedException));
    assert!(t0.elapsed() < Duration::from_millis(500));
    // 标志保持置位 = 恢复中断状态语义
    assert!(stop.load(Ordering::SeqCst));
}

// ---- send_get_fast_buf: 原始单次 read 语义 ----

#[test]
fn send_get_fast_buf_raw_single_read_semantics() {
    let body = "{\"valid\":true,\"speed\":131.0}";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nServer: wt\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let (addr, h) = serve_n(1, move |_| resp.clone().into_bytes());
    let mut h_helper = HttpHelper::new("\n");
    let r = HttpHelper::send_get_fast_buf(
        &mut h_helper.buf_indic,
        &h_helper.indic_request,
        addr,
    )
    .unwrap();
    // 不跳响应头, 原文含 HTTP 头 + body (Java 同此, 交给子串提取解析器)
    assert!(r.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(r.contains("\"valid\":true"));
    // buf 参数被写入 (Java 副作用, send_get_fast_buf_b 依赖)
    assert!(h_helper.buf_indic[..r.chars().count()]
        .iter()
        .collect::<String>()
        .starts_with("HTTP/1.1"));
    h.join().unwrap();
}

#[test]
fn send_get_fast_buf_connection_refused_is_err() {
    let mut h = HttpHelper::new("\n");
    assert!(HttpHelper::send_get_fast_buf(&mut h.buf_indic, &h.indic_request, refused_addr())
        .is_err());
}

// ---- send_get_fast: 跳 6 行 + 换行剥离拼接 ----

#[test]
fn send_get_fast_skips_six_header_lines_and_joins_body() {
    let resp = "HTTP/1.1 200 OK\r\nA: 1\r\nB: 2\r\nC: 3\r\nD: 4\r\nE: 5\r\n\r\nLINE1\nLINE2\nLINE3";
    let (addr, h) = serve_n(1, move |_| resp.as_bytes().to_vec());
    let hh = HttpHelper::new("\n");
    let r = HttpHelper::send_get_fast(&hh.state_request, addr).unwrap();
    assert_eq!(r, "LINE1LINE2LINE3");
    h.join().unwrap();
}

// ---- send_get: 请求组装 + 读体 ----

#[test]
fn send_get_request_composition_and_body() {
    let reqs = Arc::new(Mutex::new(Vec::new()));
    let reqs2 = Arc::clone(&reqs);
    let (addr, h) = serve_n(1, move |req| {
        *reqs2.lock().unwrap() = req.to_vec();
        b"HTTP/1.1 200 OK\r\nA: 1\r\nB: 2\r\nC: 3\r\nD: 4\r\nE: 5\r\n\r\nBODY1\nBODY2".to_vec()
    });
    let hh = HttpHelper::new("\n");
    let host = "127.0.0.1";
    let r = hh.send_get(host, addr.port(), "/state").unwrap();
    assert_eq!(r, "BODY1BODY2");
    h.join().unwrap();
    let got = String::from_utf8(reqs.lock().unwrap().clone()).unwrap();
    // Java 四次 write + flush 的逐字节拼接 (httpHeader 缺省 "\n")
    assert_eq!(
        got,
        format!(
            "GET /state HTTP/1.1\r\nHost: {}\r\nCache-Control:no-cache\n\r\n",
            host
        )
    );
}

// ---- fmCmd 指令格式 ----

#[test]
fn fm_cmd_set_alt_percent_d_format() {
    let reqs = Arc::new(Mutex::new(Vec::new()));
    let reqs2 = Arc::clone(&reqs);
    let (addr, h) = serve_n(1, move |req| {
        *reqs2.lock().unwrap() = req.to_vec();
        Vec::new()
    });
    let hh = HttpHelper::new("\n");
    hh.fm_cmd_set_alt(3000, addr).unwrap();
    h.join().unwrap();
    let got = String::from_utf8(reqs.lock().unwrap().clone()).unwrap();
    assert!(got.starts_with("GET /editor/fm_commands?cmd=setAlt&value=3000 HTTP/1.1\n"));
}

#[test]
fn fm_cmd_set_spd_half_up_format() {
    let reqs = Arc::new(Mutex::new(Vec::new()));
    let reqs2 = Arc::clone(&reqs);
    let (addr, h) = serve_n(1, move |req| {
        *reqs2.lock().unwrap() = req.to_vec();
        Vec::new()
    });
    let hh = HttpHelper::new("\n");
    hh.fm_cmd_set_spd(123.5, addr).unwrap();
    h.join().unwrap();
    let got = String::from_utf8(reqs.lock().unwrap().clone()).unwrap();
    // %.0f HALF_UP: 123.5 → 124
    assert!(got.starts_with("GET /editor/fm_commands?cmd=setVelocity&value=124 HTTP/1.1\n"));
}

#[test]
fn format_f_half_up_boundaries() {
    assert_eq!(format_f_half_up_0(123.5), "124");
    assert_eq!(format_f_half_up_0(123.4), "123");
    assert_eq!(format_f_half_up_0(0.4), "0");
    // HALF_UP 远离零: -123.5 → -124 (区别于 Math.round 的 -123)
    assert_eq!(format_f_half_up_0(-123.5), "-124");
    assert_eq!(format_f_half_up_0(-0.4), "-0");
    assert_eq!(format_f_half_up_0(0.0), "0");
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
    // get() 阻塞至 worker complete, 返回后双字段有值
    assert!(hh.str_state.lock().unwrap().contains("\"alt\":999"));
    assert!(hh.str_indic.contains("\"speed\":7.0"));
    h.join().unwrap();
}

#[test]
fn get_req_result_state_fail_indic_ok_blocks_until_stop() {
    // Java 原怪癖: 同轮 "/state 任务失败 + /indicators 成功" → future 永远
    // Pending (无人再 complete 它), get() 无限期阻塞, 仅 Controller.stop 的
    // interrupt 可解 → InterruptedException 分支清空双字段
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(_) => break,
            };
            stream.set_read_timeout(Some(Duration::from_millis(200))).ok();
            // 只读请求前缀用于分类 (16 字节缓冲, 单次 read 至多 16 字节)
            let mut b = [0u8; 16];
            let mut head = Vec::new();
            while head.len() < 10 {
                match stream.read(&mut b) {
                    Ok(0) => break,
                    Ok(k) => head.extend_from_slice(&b[..k]),
                    Err(_) => break,
                }
            }
            let is_state = head.starts_with(b"GET /state");
            if is_state {
                // 剩余 ~44 字节请求未读即关闭 → 对端收到 RST,
                // 客户端 read 得到 Err (对齐 Java 任务内 IOException)
                continue;
            }
            // /indicators: 排干请求 (避免未读数据把干净 FIN 变 RST) 后响应
            let mut b = [0u8; 4096];
            loop {
                match stream.read(&mut b) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => break, // 读间隙超时 → 请求收满
                }
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                .ok();
        }
    });

    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = Arc::clone(&stop);
    let stopper = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        stop2.store(true, Ordering::SeqCst);
    });

    let mut hh = HttpHelper::new("\n");
    let t0 = std::time::Instant::now();
    hh.get_req_result(addr, &stop);
    let elapsed = t0.elapsed();
    // get() 应阻塞至 stop 置位 (而非立即返回)
    assert!(
        elapsed >= Duration::from_millis(200),
        "get() 应阻塞至 stop, 实际 {:?}",
        elapsed
    );
    // InterruptedException 分支清空双字段
    assert_eq!(*hh.str_state.lock().unwrap(), NSTRING);
    assert_eq!(hh.str_indic, NSTRING);
    stopper.join().unwrap();
    server.join().unwrap();
}

// ---- map_obj / map_info ----

#[test]
fn get_req_map_obj_result_truncates_at_buf_len() {
    // 响应 > 8192 字节 → 单次 read 上限 BUF_LEN
    let mut resp = String::from("HTTP/1.1 200 OK\r\n\r\n");
    resp.push_str(&"A".repeat(20000));
    let (addr, h) = serve_n(1, move |_| resp.clone().into_bytes());
    let mut hh = HttpHelper::new("\n");
    hh.get_req_map_obj_result(addr);
    let s = &hh.str_map_obj;
    assert!(s.starts_with("HTTP/1.1 200 OK"));
    assert!(s.chars().count() <= BUF_LEN, "实际 {} > {}", s.chars().count(), BUF_LEN);
    h.join().unwrap();
}

#[test]
fn get_req_map_info_result_refused_sets_nstring() {
    let mut hh = HttpHelper::new("\n");
    hh.get_req_map_info_result(refused_addr());
    assert_eq!(hh.str_map_info, NSTRING);
}

// ---- send_get_fast_buf_b: 整数组 append 怪癖 ----

#[test]
fn send_get_fast_buf_b_appends_whole_buffer() {
    let resp = "HTTP/1.1 200 OK\r\n\r\nXY";
    let (addr, h) = serve_n(1, move |_| resp.as_bytes().to_vec());
    let mut hh = HttpHelper::new("\n");
    let mut bd = String::from("OLD");
    HttpHelper::send_get_fast_buf_b(&mut hh.buf_mapinfo, &hh.mapinfo_request, addr, &mut bd)
        .unwrap();
    // Java bd.append(char[]) 追加**整个数组** (含未读区段的 '\0')
    assert_eq!(bd.chars().count(), BUF_LEN);
    assert!(bd.starts_with("HTTP/1.1 200 OK\r\n\r\nXY"));
    assert!(bd.ends_with('\0'));
    h.join().unwrap();
}

// ---- getLiveAircraftType (8111 固定端口; 被占自动跳过, e2e 先例) ----

/// 8111 占用探测: 有服务应答即视为被占 (真机在跑) → 跳过。
/// bind 探测在 Windows SO_REUSEADDR 语义下不可靠 —— 实测真机监听与我方
/// bind 可共存且真机抢答连接, 必须 connect 探测。
fn port_8111_answered() -> bool {
    TcpStream::connect("127.0.0.1:8111").is_ok()
}

/// 8111 相关测试串行锁 (other_service::tests PORT8111_LOCK / fm::test_guard
/// 同款)。PORT: get_live_aircraft_type 硬编码 127.0.0.1:8111 (Java 保真,
/// 生产段无端口注入面), 三个用例共用 8111 —— 并行时 no_server 的 connect
/// 会抢走 parses/invalid 的唯一 accept 名额 (自己拿到 Some 假失败), 服务方
/// 随后 connect 被拒返回 None 也假失败, 双双 flaky; 持锁串行且探测→bind/
/// connect 同临界区后, no_server 连的 8111 在本测试二进制内必然空闲。
/// 锁中毒无不变量可破, 复取即可 (一次失败不连锁炸后续用例)。
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
            stream.set_read_timeout(Some(Duration::from_millis(200))).ok();
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
            stream.set_read_timeout(Some(Duration::from_millis(200))).ok();
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

// ---- send_get_url ----

#[test]
fn send_get_url_joins_lines_stripping_newlines() {
    let body = "{\n\"tag_name\": \"1.590\",\n\"x\": 1\n}";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let (addr, h) = serve_n(1, move |_| resp.clone().into_bytes());
    let r = HttpHelper::send_get_url(&format!("http://127.0.0.1:{}/x", addr.port())).unwrap();
    // Java readLine 循环 append: 换行被剥离
    assert_eq!(r, "{\"tag_name\": \"1.590\",\"x\": 1}");
    h.join().unwrap();
}

#[test]
fn send_get_url_error_status_is_err() {
    let (addr, h) = serve_n(1, |_| b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec());
    let r = HttpHelper::send_get_url(&format!("http://127.0.0.1:{}/miss", addr.port()));
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("404"));
    h.join().unwrap();
}

#[test]
fn send_get_url_follows_redirect() {
    let (addr, h) = serve_n(2, |req| {
        let s = std::str::from_utf8(req).unwrap_or("");
        let first = s.lines().next().unwrap_or("");
        // Java new URL(base, "/final") 语义 = 根相对 → 请求行必须恰为
        // "GET /final HTTP/1.1" ('//final' 双斜杠拼接是错译, 会落 404)
        if first.starts_with("GET /first ") {
            b"HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\n\r\n".to_vec()
        } else if first.starts_with("GET /final ") {
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK".to_vec()
        } else {
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec()
        }
    });
    let r = HttpHelper::send_get_url(&format!("http://127.0.0.1:{}/first", addr.port())).unwrap();
    assert_eq!(r, "OK");
    h.join().unwrap();
}

#[test]
fn send_get_url_https_unsupported() {
    // TLS 依赖未引入 (PORT 上报项); 协议检查先于连接, 无网络请求
    assert!(HttpHelper::send_get_url("https://api.github.com/repos/x/y/releases/latest").is_err());
}

#[test]
fn send_get_url_eof_body_without_content_length() {
    let (addr, h) = serve_n(1, |_| b"HTTP/1.1 200 OK\r\n\r\nPLAIN".to_vec());
    let r = HttpHelper::send_get_url(&format!("http://127.0.0.1:{}/x", addr.port())).unwrap();
    assert_eq!(r, "PLAIN");
    h.join().unwrap();
}

#[test]
fn send_get_url_chunked_body() {
    let (addr, h) = serve_n(1, |_| {
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n".to_vec()
    });
    let r = HttpHelper::send_get_url(&format!("http://127.0.0.1:{}/x", addr.port())).unwrap();
    assert_eq!(r, "hello world");
    h.join().unwrap();
}

#[test]
fn send_get_url_no_protocol_is_err() {
    assert!(HttpHelper::send_get_url("127.0.0.1:8111/state").is_err());
}
