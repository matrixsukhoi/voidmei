//! HttpHelper 的 Rust 移植 (src/prog/util/HttpHelper.java)
//! 8111/9222 双端口 HTTP 客户端: 取数 (getReqResult 系, Socket 层手写 HTTP)、
//! FM 编辑器指令 (fmCmdSetAlt/fmCmdSetSpd)、更新检查 (sendGetURL)、
//! live 机型探测 (getLiveAircraftType)。
//!
//! PORT 头注记 (跨文件问题, §6 只标注不越文件修):
//! 1. `Application.httpHeader` (static, C 类未翻译): Java 字段初始化器在构造期
//!    读取全局; Rust 由 `new(http_header)` 注入并存副本 `self.http_header`
//!    供 send_get 调用期读取 (值源自 Lang.httpHeader, 启动后不变,
//!    构造期值 == 调用期值)。
//! 2. `Application.threadPool` (cachedThreadPool, C 类未翻译): get_req_result
//!    的 submit 以 `std::thread::spawn` 等价 —— cachedThreadPool 的
//!    "每任务并发执行" 语义保留, 线程复用优化不承载行为。
//! 3. vm-data `data/http.rs` 的 POC `http_get` 与本模块并存 (Service 接线时
//!    裁决), 本版按 Java HttpHelper 语义保真 (8111/9222 双端口/单次 read
//!    语义/getLiveAircraftType 等)。
//! 4. §2.13: InterruptedException → `AtomicBool` 停机标志 (exception_helper
//!    契约: 本处 ignore 调用点 ↔ Service 轮询循环的 sleep 点必须是同一标志)。
//! 5. 写侧字符集: Java OutputStreamWriter 未指定字符集处用平台默认
//!    (Windows GBK); 请求串域内纯 ASCII, GBK/UTF-8 编码字节一致,
//!    统一按 UTF-8 (`as_bytes`) 输出。

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::exception_helper;
use crate::logger;
use crate::parser::Indicators;

/// 对应 Java `public static final String nstring = ""`
pub const NSTRING: &str = "";
/// 对应 Java `public static final int buf_len = 8192`
pub const BUF_LEN: usize = 8192;

/// CompletableFuture<Boolean> 的最小复刻 (getReqResult 专用)。
///
/// PORT: 单次完成语义逐字保真 —— `complete`/`complete_exceptionally` 只有在
/// Pending 态才生效并返回 true, 已完成后为无操作返回 false; `get` 永远返回
/// **第一次** 完成的结果。Java 原类跨轮询复用同一实例 (getReqResult 每 ~10Hz
/// 调一次), 完成一次后 complete 即空转、get 立即返回旧值。
/// PORT: Java HttpHelper 只调用 complete(true) —— 池任务抛 IOException 时
/// future 停在 Pending (全文件无 completeExceptionally 调用点), 后续轮次的
/// 成功任务仍可将其 complete。
#[derive(Clone)]
pub struct CompletableFuture {
    inner: Arc<(Mutex<CfState>, Condvar)>,
}

enum CfState {
    Pending,
    Done(bool),
    /// 任务异常完成 (Java: future completes exceptionally)
    Failed,
}

/// `CompletableFuture::get` 的结果 (Java 三种出口: 正常值 / ExecutionException /
/// InterruptedException 的 §2.13 映射)
pub enum CfOutcome {
    Value(bool),
    ExecutionException,
    InterruptedException,
}

impl CompletableFuture {
    pub fn new() -> Self {
        CompletableFuture {
            inner: Arc::new((Mutex::new(CfState::Pending), Condvar::new())),
        }
    }

    /// Java `complete(value)`: Pending → Done 返回 true; 已完成返回 false (无操作)
    pub fn complete(&self, value: bool) -> bool {
        let mut st = self.inner.0.lock().unwrap();
        if matches!(*st, CfState::Pending) {
            *st = CfState::Done(value);
            self.inner.1.notify_all();
            true
        } else {
            false
        }
    }

    /// Java `completeExceptionally()`: Pending → Failed 返回 true; 已完成返回 false
    pub fn complete_exceptionally(&self) -> bool {
        let mut st = self.inner.0.lock().unwrap();
        if matches!(*st, CfState::Pending) {
            *st = CfState::Failed;
            self.inner.1.notify_all();
            true
        } else {
            false
        }
    }

    /// Java `get()` 无超时阻塞等待。
    /// PORT §2.13: 中断位 → 停机标志轮询 (Condvar 10ms 分片唤醒查标志)。
    /// JDK8 语义 (get → waitingGet): 已完成 (result != null) 直接返回值、
    /// **不查中断位**; Pending 才进等待循环查中断位 (已置位则立即抛)。
    /// 故此处先查完成态、后查停机标志。
    pub fn get(&self, stop: &AtomicBool) -> CfOutcome {
        let mut st = self.inner.0.lock().unwrap();
        loop {
            match &*st {
                CfState::Done(v) => return CfOutcome::Value(*v),
                CfState::Failed => return CfOutcome::ExecutionException,
                CfState::Pending => {}
            }
            if stop.load(Ordering::SeqCst) {
                return CfOutcome::InterruptedException;
            }
            let (guard, _) = self
                .inner
                .1
                .wait_timeout(st, Duration::from_millis(10))
                .unwrap();
            st = guard;
        }
    }
}

impl Default for CompletableFuture {
    fn default() -> Self {
        Self::new()
    }
}

/// 对应 Java HttpHelper 实例字段区 (声明顺序一致)。
///
/// PORT: `strState` 在 Java 由池线程写 / Service 线程读 (首次轮询后无同步,
/// 靠引用赋值原子性侥幸成立), Rust 以 `Arc<Mutex<String>>` 显式表达跨线程
/// 写; `buf_state` 同理 (池任务的写缓冲)。
/// PORT: Java `strState` 等字段默认 null —— 读取点均在首轮 getReqResult 之后
/// (Service.java:1713 `strState.length()`), null 不可达; 以 nstring 初始化
/// 对齐兜底后的稳定态 (§2.10 按有意处理)。
pub struct HttpHelper {
    pub completable_future_0: CompletableFuture,
    pub completable_future_1: CompletableFuture,
    pub state_request: String,
    pub indic_request: String,
    pub mapobj_request: String,
    pub mapinfo_request: String,
    pub fmcm_request: String,
    pub set_alt_req: String,
    pub set_vel_req: String,
    pub str_state: Arc<Mutex<String>>,
    pub str_indic: String,
    pub str_map_obj: String,
    pub str_map_info: String,
    /// Java `StringBuilder strBState` —— 唯一写者 send_get_fast_buf_b 无调用者
    /// (getReqResult 内调用行已注释), 保真保留字段; StringBuilder → String
    pub str_b_state: String,
    /// Java `StringBuilder strBIndic`, 同上
    pub str_b_indic: String,
    pub buf_indic: Vec<char>,
    pub buf_state: Arc<Mutex<Vec<char>>>,
    pub buf_mapobj: Vec<char>,
    pub buf_mapinfo: Vec<char>,
    /// PORT: `Application.httpHeader` 的构造期快照 (见模块头注记 1),
    /// send_get 调用期读取
    pub http_header: String,
}

impl HttpHelper {
    /// Java 隐式默认构造器 + 字段初始化器。
    /// PORT: `Application.httpHeader` 未翻译, 由参数注入 (Lang.httpHeader 缺省 "\n")。
    pub fn new(http_header: &str) -> Self {
        HttpHelper {
            completable_future_0: CompletableFuture::new(),
            completable_future_1: CompletableFuture::new(),
            state_request: "GET /state HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            indic_request: "GET /indicators HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            mapobj_request: "GET /map_obj.json HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            mapinfo_request: "GET /map_info.json HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            fmcm_request: "GET /editor/fm_commands?cmd=getFmProperties HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            set_alt_req: "GET /editor/fm_commands?cmd=setAlt&value=%d HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            set_vel_req: "GET /editor/fm_commands?cmd=setVelocity&value=%.0f HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            str_state: Arc::new(Mutex::new(NSTRING.to_string())),
            str_indic: NSTRING.to_string(),
            str_map_obj: NSTRING.to_string(),
            str_map_info: NSTRING.to_string(),
            str_b_state: String::new(),
            str_b_indic: String::new(),
            buf_indic: vec!['\0'; BUF_LEN],
            buf_state: Arc::new(Mutex::new(vec!['\0'; BUF_LEN])),
            buf_mapobj: vec!['\0'; BUF_LEN * 4],
            buf_mapinfo: vec!['\0'; BUF_LEN],
            http_header: http_header.to_string(),
        }
    }

    /// 对应 Java `fmCmdSetAlt(int alt, SocketAddress dest)`。
    /// Java `String.format(setAltReq, alt)` 的 `%d` → 十进制整数。
    pub fn fm_cmd_set_alt(&self, alt: i32, dest: SocketAddr) -> io::Result<()> {
        // PORT: String.format 模板含 %d 占位符, 动态模板不能用 format!,
        // 以 replace 复刻 (模板内仅此一处占位符)
        let tmp_req = self.set_alt_req.replace("%d", &alt.to_string());
        let mut socket = TcpStream::connect(dest)?;
        // socket.
        {
            let mut buffered_writer = io::BufWriter::new(&mut socket);
            buffered_writer.write_all(tmp_req.as_bytes())?;
            buffered_writer.flush()?;
        }
        drop(socket);
        Ok(())
    }

    /// 对应 Java `fmCmdSetSpd(double spd, SocketAddress dest)`。
    pub fn fm_cmd_set_spd(&self, spd: f64, dest: SocketAddr) -> io::Result<()> {
        let tmp_req = self.set_vel_req.replace("%.0f", &format_f_half_up_0(spd));
        let mut socket = TcpStream::connect(dest)?;
        // socket.
        {
            let mut buffered_writer = io::BufWriter::new(&mut socket);
            buffered_writer.write_all(tmp_req.as_bytes())?;
            buffered_writer.flush()?;
        }
        drop(socket);
        Ok(())
    }

    /// 对应 Java `sendGetURL(String url)` (HttpURLConnection)。
    ///
    /// PORT: Java HttpURLConnection 委托 JVM 网络/TLS 栈; Rust 侧 std 无 TLS,
    /// `https://` 返回 Err (调用方 checkUpdate 的 catch(Exception) 语义 ≈ 更新
    /// 检查失败静默跳过)。TLS 栈引入待 workspace 裁决 (CLASSIFY 备注: reqwest),
    /// 本文件不可改 Cargo.toml, 只上报。
    /// PORT: 本方法无 this 使用, 按无 self 关联函数翻译 (string_helper 先例)。
    pub fn send_get_url(url: &str) -> Result<String, String> {
        let mut current = url.to_string();
        let mut redirects = 0usize;
        loop {
            let (scheme, host, port, path) = parse_url(&current)?;
            if scheme == "https" {
                // PORT: TLS 依赖未引入 (见方法注释), 上报项
                // TODO(port): https 支持 (reqwest/TLS 栈引入待 workspace 裁决,
                // CLASSIFY C 类; 唯一生产调用方 checkUpdate 固定 https URL)
                return Err(format!("https 协议暂不受支持 (无 TLS 依赖): {}", current));
            }
            let addr: SocketAddr = format!("{}:{}", host, port)
                .parse()
                .map_err(|_| format!("主机地址无法解析: {}", host))?;
            let mut stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;

            // HttpURLConnection 对非默认端口发 "Host: host:port"
            let host_header = if port != 80 {
                format!("{}:{}", host, port)
            } else {
                host.clone()
            };
            // PORT: Java 默认 keep-alive + Content-Length 读取; 此处
            // Connection: close + read_to_end, 服务器语义等价
            let req = format!(
                "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                path, host_header
            );
            stream
                .write_all(req.as_bytes())
                .map_err(|e| e.to_string())?;

            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;

            let sep = find_subslice(&buf, b"\r\n\r\n").ok_or("响应缺少 header 分隔")?;
            let head = String::from_utf8_lossy(&buf[..sep]).to_string();
            let body = &buf[sep + 4..];

            // 状态行 "HTTP/1.x NNN ..."
            let status_line = head.lines().next().unwrap_or("");
            let code: u16 = status_line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .ok_or(format!("状态行无法解析: {}", status_line))?;

            let head_lower = head.to_ascii_lowercase();
            let location = header_value(&head, "location");

            if (300..400).contains(&code) {
                if let Some(loc) = location {
                    if redirects >= 20 {
                        // Java HttpURLConnection 默认 maxRedirects=20, 超限抛异常
                        return Err("重定向次数过多".to_string());
                    }
                    redirects += 1;
                    current = resolve_location(&scheme, &host, port, &path, &loc)?;
                    continue;
                }
                // PORT: 3xx 无 Location → Java 返回该响应本体, 按终态继续
            }

            // Java getInputStream() 对 4xx/5xx 抛 IOException → 调用方 catch
            if code >= 400 {
                return Err(format!("HTTP {}", code));
            }

            let body_bytes: Vec<u8> = if let Some(cl) = content_length(&head_lower) {
                if body.len() >= cl {
                    body[..cl].to_vec()
                } else {
                    body.to_vec()
                }
            } else if head_lower.contains("transfer-encoding: chunked") {
                decode_chunked(body)?
            } else {
                body.to_vec() // EOF 兜底
            };

            // PORT: lines() 只按 \n/\r\n 切行, Java readLine 额外接受孤立 \r ——
            // 域内 JSON 无孤立 \r
            let result: String = String::from_utf8_lossy(&body_bytes)
                .lines()
                .collect();
            if url.contains("api.github.com") {
                logger::info(
                    "Update",
                    &format!("Latest version info fetched successfully (HTTP {})", code),
                );
            }
            return Ok(result);
        }
    }

    /// 对应 Java `sendGet(String host, int port, String path)`。
    /// PORT: port int → u16 (Java 非法端口在 InetSocketAddress 构造即抛,
    /// 类型层面排除该值域)。
    /// Java `String result = nstring` 初值即被覆盖 (保形, 不复刻死存储)。
    pub fn send_get(&self, host: &str, port: u16, path: &str) -> io::Result<String> {
        let dest: SocketAddr = format!("{}:{}", host, port)
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut socket = TcpStream::connect(dest)?;
        {
            let mut buffered_writer = io::BufWriter::new(&mut socket);
            buffered_writer.write_all(format!("GET {} HTTP/1.1\r\n", path).as_bytes())?;
            buffered_writer.write_all(format!("Host: {}\r\n", host).as_bytes())?;
            buffered_writer.write_all(b"Cache-Control:no-cache")?;
            buffered_writer.write_all(self.http_header.as_bytes())?;
            buffered_writer.write_all(b"\r\n")?;
            buffered_writer.flush()?;
        }

        let mut buffered_reader = BufReader::new(socket);

        // bufferedReader.ready();
        // PORT: Java ready() 调用结果被丢弃 (无副作用), 保留为注释

        // 跳过响应头 6 行 (状态行 + 5 个头字段)
        for _ in 0..6 {
            let mut line = String::new();
            if buffered_reader.read_line(&mut line)? == 0 {
                break; // PORT: Java readLine 返回 null 时循环体不执行, 等价 EOF 提前结束
            }
        }

        let mut content_buf = String::new();
        loop {
            let mut raw = Vec::new();
            if buffered_reader.read_until(b'\n', &mut raw)? == 0 {
                break;
            }
            let mut line = String::from_utf8_lossy(&raw).to_string();
            // Java BufferedReader.readLine 剥离 \r\n / \n 行终止符
            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }
            content_buf.push_str(&line);
        }
        let result = content_buf;
        // Application.debugPrint(result);
        Ok(result)
    }

    /// 对应 Java `sendGetFastBufB(char[] buf, String req_string, SocketAddress dest, StringBuilder bd)`。
    /// PORT: 无 this 使用 → 无 self 关联函数; StringBuilder bd → &mut String。
    pub fn send_get_fast_buf_b(
        buf: &mut [char],
        req_string: &str,
        dest: SocketAddr,
        bd: &mut String,
    ) -> io::Result<()> {
        let mut socket = TcpStream::connect(dest)?;
        // socket.
        {
            let mut buffered_writer = io::BufWriter::new(&mut socket);
            // bufferedWriter.write("\r\n");
            buffered_writer.write_all(req_string.as_bytes())?;
            buffered_writer.flush()?;
        }
        let read_chars = read_once_into_chars(&mut socket, buf)?;
        // PORT: Java close 序列 (reader/writer/socket) → Drop
        let _ = read_chars;
        bd.clear();
        bd.extend(buf.iter());
        Ok(())
    }

    /// 对应 Java `sendGetFastBuf(char[] buf, String req_string, SocketAddress dest)`。
    /// PORT: 无 this 使用 → 无 self 关联函数。
    /// 返回 String.valueOf(buf, 0, rlen) —— 只取实际读入的 rlen 个字符。
    pub fn send_get_fast_buf(
        buf: &mut [char],
        req_string: &str,
        dest: SocketAddr,
    ) -> io::Result<String> {
        let mut socket = TcpStream::connect(dest)?;
        // socket.
        {
            let mut buffered_writer = io::BufWriter::new(&mut socket);
            // bufferedWriter.write("\r\n");
            buffered_writer.write_all(req_string.as_bytes())?;
            buffered_writer.flush()?;
        }
        let rlen = read_once_into_chars(&mut socket, buf)?;
        if rlen == 0 {
            // Java rlen == -1 (EOF) → ""
            return Ok(NSTRING.to_string());
        }
        Ok(buf[..rlen].iter().collect())
        // .valueOf(buf);
    }

    /// 对应 Java `sendGetFast(String req_string, SocketAddress dest)`。
    /// PORT: 无 this 使用 → 无 self 关联函数。
    pub fn send_get_fast(req_string: &str, dest: SocketAddr) -> io::Result<String> {
        let mut socket = TcpStream::connect(dest)?;
        // socket.
        {
            let mut buffered_writer = io::BufWriter::new(&mut socket);
            // bufferedWriter.write("\r\n");
            buffered_writer.write_all(req_string.as_bytes())?;
            buffered_writer.flush()?;
        }

        // BufferedInputStream streamReader = new
        // BufferedInputStream(socket.getInputStream());
        //
        // BufferedReader bufferedReader = new BufferedReader(new
        // InputStreamReader(streamReader, "utf-8"));

        let mut buffered_reader = BufReader::new(socket);

        // TODO: 优化过程，一次性读取
        for _ in 0..6 {
            let mut line = String::new();
            if buffered_reader.read_line(&mut line)? == 0 {
                break; // PORT: Java readLine 返回 null 时继续跳行, 等价 EOF 提前结束
            }
        }

        let mut content_buf = String::new();
        loop {
            let mut raw = Vec::new();
            if buffered_reader.read_until(b'\n', &mut raw)? == 0 {
                break;
            }
            let mut line = String::from_utf8_lossy(&raw).to_string();
            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }
            content_buf.push_str(&line);
        }
        let result = content_buf;
        Ok(result)
    }

    // public Future<String> sendGetAsync(CompletableFuture<String>
    // completableFuture, String req_string, SocketAddress dest) throws
    // InterruptedException {
    //// CompletableFuture<String> completableFuture = new
    // CompletableFuture<String>();
    // Executors.newCachedThreadPool().submit(() -> {
    //// System.out.println("submit\n");
    // s = sendGetFast(req_string, dest);
    //// System.out.println("complete\n");
    // completableFuture.complete(s);
    // return null;
    // });
    //
    // return completableFuture;
    // }
    //

    /// 对应 Java `getReqResult(SocketAddress req_addr)`。
    /// PORT §2.13: InterruptedException → stop 停机标志参数 (Service 线程的
    /// 停机标志, 与 exception_helper::ignore 成对)。
    /// PORT: Java 的 HttpHelper 实例被 Service 线程与 AutoMeasure 线程共享
    /// (AutoMeasure 经 xS.httpClient 同时调 fmCmdSetAlt/fmCmdSetSpd);
    /// `&mut self` 使该共享用法不可表达 —— 接线 AutoMeasure (fmTesting) 时
    /// 需主 agent 裁决 (独享实例或 Mutex 包裹), §6 上报。
    pub fn get_req_result(&mut self, req_addr: SocketAddr, stop: &AtomicBool) {
        // Executors.newCachedThreadPool().submit(() -> {
        // strState = sendGetFast(state_request, req_addr);
        // completableFuture0.complete(true);
        // return null;
        // });
        //
        //// strIndic = sendGetFast(indic_request, req_addr);
        // strIndic = sendGetFastBuf(buf_indic, indic_request, req_addr);
        {
            // PORT: Application.threadPool.submit → std::thread::spawn (模块头注记 2)
            let state_request = self.state_request.clone();
            let buf_state = Arc::clone(&self.buf_state);
            let str_state = Arc::clone(&self.str_state);
            let cf0 = self.completable_future_0.clone();
            std::thread::spawn(move || {
                // sendGetFastBufB(buf_state, state_request, req_addr, strBState);
                // PORT: Java lambda 内 sendGetFastBuf 抛 IOException 时异常直接
                // 传出任务, strState 赋值与 completableFuture0.complete(true) 均
                // 不执行 —— future 停在 Pending (全文件无 completeExceptionally
                // 调用点), 后续轮次的成功任务仍可将其 complete; 故 Err 分支
                // 什么都不做。连带怪癖: "/state 任务失败 + /indicators 同轮成功"
                // 时 get() 无限期阻塞, 仅 stop (interrupt) 可解 —— Java 原行为。
                // PORT: IO 在本地缓冲上进行, 完成后短锁发布到 buf_state ——
                // Java 的共享 buf_state 本是无锁数据竞态 (无读者), Rust 以
                // Mutex 表达时不能把无超时网络 IO 关进锁内 (挂死任务会把
                // 后续所有轮次串行化堵死, Java cachedThreadPool 任务本可并发)。
                let mut local = vec!['\0'; BUF_LEN];
                if let Ok(s) = Self::send_get_fast_buf(&mut local, &state_request, req_addr) {
                    *buf_state.lock().unwrap() = local;
                    *str_state.lock().unwrap() = s;
                    cf0.complete(true);
                }
            });
        }
        // sendGetFastBufB(buf_state, state_request, req_addr, strBIndic);
        match Self::send_get_fast_buf(&mut self.buf_indic, &self.indic_request, req_addr) {
            Ok(s) => {
                self.str_indic = s;
                // System.out.println(strState);
                match self.completable_future_0.get(stop) {
                    CfOutcome::Value(_) => {}
                    CfOutcome::ExecutionException => {
                        // 异步任务执行失败，静默处理（常见于连接断开）
                        // PORT: Java 同构死分支 —— completableFuture0 无
                        // completeExceptionally 调用点, catch(ExecutionException)
                        // 在 Java 亦不可达, 保真保留
                        *self.str_state.lock().unwrap() = NSTRING.to_string();
                        self.str_indic = NSTRING.to_string();
                    }
                    CfOutcome::InterruptedException => {
                        // 中断异常，恢复中断状态
                        exception_helper::ignore(stop);
                        *self.str_state.lock().unwrap() = NSTRING.to_string();
                        self.str_indic = NSTRING.to_string();
                    }
                }
            }
            Err(_) => {
                // IO异常，静默处理（常见于网络问题）
                *self.str_state.lock().unwrap() = NSTRING.to_string();
                self.str_indic = NSTRING.to_string();
            }
        }
    }

    /// 对应 Java `getReqMapObjResult(SocketAddress req_addr)`
    pub fn get_req_map_obj_result(&mut self, req_addr: SocketAddr) {
        match Self::send_get_fast_buf(&mut self.buf_mapobj, &self.mapobj_request, req_addr) {
            Ok(s) => self.str_map_obj = s,
            Err(_) => self.str_map_obj = NSTRING.to_string(),
        }
    }

    /// 对应 Java `getReqMapInfoResult(SocketAddress req_addr)`
    pub fn get_req_map_info_result(&mut self, req_addr: SocketAddr) {
        match Self::send_get_fast_buf(&mut self.buf_mapinfo, &self.mapinfo_request, req_addr) {
            Ok(s) => self.str_map_info = s,
            Err(_) => self.str_map_info = NSTRING.to_string(),
        }
    }

    /// 获取当前 8111 端口的实时机型信息
    ///
    /// 返回 Some(机型名称)，如果获取失败或无效则返回 None
    pub fn get_live_aircraft_type(&self) -> Option<String> {
        // Java catch(Exception) 兜底: 任何失败 → None (Rust 侧错误均为 Result,
        // 解析器为全函数无 panic, 无需 catch_unwind)
        let mut buf_indic = vec!['\0'; BUF_LEN];
        // 使用 127.0.0.1:8111 作为目标
        let dest: SocketAddr = "127.0.0.1:8111".parse().unwrap();
        let indicators_json =
            match Self::send_get_fast_buf(&mut buf_indic, &self.indic_request, dest) {
                Ok(s) => s,
                Err(_) => return None, // e.printStackTrace(); 忽略错误，返回 null
            };

        let mut indicators_parser = Indicators::new();
        indicators_parser.init();
        indicators_parser.update(&indicators_json);

        // PORT: type 经 Indicators.update 已 toUpperCase —— 与字面量
        // "No Cockpit" 的 equals 比较在 Java 同样不成立 (NO COCKPIT), 保真保留
        if indicators_parser.valid.as_deref() == Some("true") {
            if let Some(t) = indicators_parser.r#type.as_deref() {
                if !t.is_empty() && t != "No Cockpit" {
                    // PORT: toLowerCase() 默认 Locale (域内机型名 ASCII, 无行为差);
                    // Java trim() 剥 <= U+0020, Rust trim() 剥 Unicode 空白 ——
                    // 域内机型名无首尾空白, 等价
                    return Some(t.to_lowercase().trim().to_string());
                }
            }
        }
        None
    }
}

/// Java `String.format("%.0f", v)` = java.util.Formatter 的 RoundingMode.HALF_UP
/// (0.5 远离零进位: 123.5→124, -123.5→-124)。
/// PORT: Rust `{:.0}` 是半偶舍入 (§2.3 同源陷阱), 手工复刻;
/// 与 format.rs (FastNumberFormatter 语义, 负零抑制) 是两条不同 Java 代码路径。
/// PORT: ±Infinity → Java 输出 "Infinity"/"-Infinity", 域内速度值不可达, 不复刻。
fn format_f_half_up_0(v: f64) -> String {
    let r = if v >= 0.0 {
        (v + 0.5).floor()
    } else {
        (v - 0.5).ceil()
    };
    format!("{:.0}", r)
}

/// 单次 read 语义复刻 (send_get_fast_buf / send_get_fast_buf_b 共用):
/// Java `bufferedReader.read(buf, 0, buf_len)` —— 至多读 buf_len (8192) 个
/// 字符, 一次返回可用数据, 不保证读满。
/// PORT: 字节域单次 `read` (≤8192 字节) + UTF-8 解码近似 —— ASCII 域字节数 ==
/// 字符数等价; InputStreamReader 的 malformed 替换 (U+FFFD) ↔ from_utf8_lossy。
/// 返回实际读入的字符数 (Java rlen; 0 = EOF, 对应 Java -1)。
fn read_once_into_chars(stream: &mut TcpStream, buf: &mut [char]) -> io::Result<usize> {
    let mut byte_buf = vec![0u8; BUF_LEN];
    let rlen = stream.read(&mut byte_buf)?;
    if rlen == 0 {
        return Ok(0);
    }
    let decoded: Vec<char> = String::from_utf8_lossy(&byte_buf[..rlen]).chars().collect();
    // PORT: Java read 长度上限 buf_len (字符); 字节读入 ≤8192 → ASCII 域解码后
    // 必 ≤8192, 此 min 仅防御 buf 参数小于 8192 的调用 (固定调用点均 ≥8192)
    let n = decoded.len().min(BUF_LEN).min(buf.len());
    buf[..n].copy_from_slice(&decoded[..n]);
    Ok(n)
}

/// 解析 "scheme://host[:port]/path?query"。
/// PORT: 最小实现 —— 不支持 userinfo/IPv6 字面量 (调用域: api.github.com 与
/// 本地 mock URL, 均不涉及); 无 path 时按 "/" (Java HttpURLConnection 行为)。
fn parse_url(url: &str) -> Result<(String, String, u16, String), String> {
    let lower = url.to_ascii_lowercase();
    let (scheme, rest) = if let Some(r) = lower.strip_prefix("http://") {
        ("http", &url[url.len() - r.len()..])
    } else if let Some(r) = lower.strip_prefix("https://") {
        ("https", &url[url.len() - r.len()..])
    } else {
        return Err(format!("URL 缺少协议: {}", url));
    };

    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], rest[i..].to_string()),
        None => (rest, "/".to_string()),
    };
    let (host, port) = match authority.rfind(':') {
        Some(i) => {
            let p: u16 = authority[i + 1..]
                .parse()
                .map_err(|_| format!("端口非法: {}", authority))?;
            (authority[..i].to_string(), p)
        }
        None => (authority.to_string(), if scheme == "https" { 443 } else { 80 }),
    };
    if host.is_empty() {
        return Err(format!("URL 缺少主机: {}", url));
    }
    Ok((scheme.to_string(), host, port, path))
}

/// 从 Location 头解析重定向目标为绝对 URL。
/// PORT: 相对路径取 base path 的目录拼接 (Java URL(URL, String) 语义的最小子集;
/// 调用域 github API 返回绝对路径 "/..." 或绝对 URL)。
fn resolve_location(
    base_scheme: &str,
    base_host: &str,
    base_port: u16,
    base_path: &str,
    loc: &str,
) -> Result<String, String> {
    if loc.starts_with("http://") || loc.starts_with("https://") {
        return Ok(loc.to_string());
    }
    // Java new URL(base, loc): loc 以 '/' 开头 = 根相对, 直接替换 path
    // (不与 base path 的目录拼接, 否则 '/first' + '/final' → '//final' 双斜杠)
    if loc.starts_with('/') {
        return Ok(format!("{}://{}:{}{}", base_scheme, base_host, base_port, loc));
    }
    let dir = match base_path.rfind('/') {
        Some(i) => &base_path[..i + 1],
        None => "/",
    };
    Ok(format!(
        "{}://{}:{}{}{}",
        base_scheme, base_host, base_port, dir, loc
    ))
}

fn header_value(head: &str, name: &str) -> Option<String> {
    for line in head.lines().skip(1) {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with(&format!("{}:", name)) {
            return Some(line[name.len() + 1..].trim().to_string());
        }
    }
    None
}

/// PORT: 以下三个私有函数与 vm-data `data/http.rs` 的 POC 实现并存
/// (D6 依赖方向 vm-data → vm-core, 本 crate 不可反向引用; Service 接线时裁决合一)
fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn content_length(head_lower: &str) -> Option<usize> {
    for line in head_lower.split("\r\n") {
        if let Some(v) = line.strip_prefix("content-length:") {
            return v.trim().parse().ok();
        }
    }
    None
}

fn decode_chunked(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    loop {
        let nl = find_subslice(&body[pos..], b"\r\n")
            .ok_or("chunked: 缺行尾")?
            + pos;
        let size_str = std::str::from_utf8(&body[pos..nl]).map_err(|_| "chunked: size 非法")?;
        let size = usize::from_str_radix(size_str.trim().split(';').next().unwrap_or("0"), 16)
            .map_err(|_| "chunked: size 非十六进制")?;
        pos = nl + 2;
        if size == 0 {
            break;
        }
        // PORT §2.2: 构造性超大 hex size 会使 pos+size 回绕 (Java 对应 IOException)
        let end = pos.checked_add(size).ok_or("chunked: size 非法")?;
        if end > body.len() {
            return Err("chunked: body 截断".into());
        }
        out.extend_from_slice(&body[pos..end]);
        pos = end + 2; // 跳过块尾 CRLF (end <= body.len(), 不溢出)
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
