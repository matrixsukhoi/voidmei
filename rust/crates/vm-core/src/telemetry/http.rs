//! HttpHelper 的 Rust 移植 (src/prog/util/HttpHelper.java)
//! 8111/9222 双端口 HTTP 客户端: 取数 (getReqResult 系, Socket 层手写 HTTP)、
//! FM 编辑器指令 (fmCmdSetAlt/fmCmdSetSpd)、更新检查 (sendGetURL)、
//! live 机型探测 (getLiveAircraftType)。
//!
//! 重构波5 重写为单线程阻塞客户端 (Service 轮询线程独占):
//! - /state + /indicators 同线程顺序 GET (原每轮 spawn 线程 + 自制
//!   CompletableFuture 的 Java 直译退役 — 本地回环 RTT 亚毫秒, 顺序化
//!   每 tick 至多多 ~1ms, 换掉全部复杂度与三个 Java 怪癖);
//! - 每请求 connect 250ms / read 500ms 超时 (原无任何超时, 理论可无限挂);
//! - 头解析规范化: 读至 \r\n\r\n 分隔, body 按 Content-Length, 无则读到 EOF
//!   (原 "跳 6 行头" 魔法与单次 read 截断语义退役)。
//!
//! PORT(行为变化备案, 波5 裁决):
//! 1. "/state 失败 + /indicators 成功 → get() 无限阻塞" 的 Java 原怪癖退役 —
//!    /state 失败 → 空串复位, 走既有端口翻转/等待分支;
//! 2. 单次 read 截断怪癖 (响应 >8KB 或 TCP 分段即截) 退役 — 读全;
//! 3. 新增超时上限 (此前 connect/read 均无界)。
//!
//! 保留: 8111/9222 双端口翻转 (调用方)、httpHeader 注入、fmCmdSetAlt/Spd、
//! sendGetURL (更新检查)、getLiveAircraftType。
//! §2.13: stop 停机标志参数保留 (get_req_result 签名), 超时上限使阻塞有界。

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::base::logger;
use crate::telemetry::parser::Indicators;

/// 对应 Java `public static final String nstring = ""`
pub const NSTRING: &str = "";

/// 连接超时 (本地服务器上限; 原无超时)
const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
/// 读超时 (本地服务器上限; 原无超时)
const READ_TIMEOUT: Duration = Duration::from_millis(500);

/// 对应 Java HttpHelper 实例字段区。
/// 波5 重写: CompletableFuture/buf_*/strB_* 死字段族退役; str_state 保持
/// Arc<Mutex> (vm-data Service 读 + tests mock 注入面)。
pub struct HttpHelper {
    pub state_request: String,
    pub indic_request: String,
    pub mapobj_request: String,
    pub mapinfo_request: String,
    pub set_alt_req: String,
    pub set_vel_req: String,
    pub str_state: std::sync::Arc<std::sync::Mutex<String>>,
    pub str_indic: String,
    pub str_map_obj: String,
    pub str_map_info: String,
    /// PORT: `Application.httpHeader` 的构造期快照 (值源自 Lang.httpHeader,
    /// 启动后不变)
    pub http_header: String,
}

/// 波5 规范 GET 核: 超时 + 头解析 (\r\n\r\n 分隔) + Content-Length/EOF 定体。
/// 请求串沿用 Java 模板 (GET 行 + Host + Cache-Control + httpHeader 注入)。
fn http_get(req_string: &str, dest: SocketAddr) -> io::Result<String> {
    let mut socket = TcpStream::connect_timeout(&dest, CONNECT_TIMEOUT)?;
    socket.set_read_timeout(Some(READ_TIMEOUT))?;
    socket.write_all(req_string.as_bytes())?;
    let mut buffered_reader = BufReader::new(socket);
    // 头: 读至空行 (\r\n\r\n)
    let mut head = String::new();
    loop {
        let mut line = String::new();
        if buffered_reader.read_line(&mut line)? == 0 {
            break; // EOF (无分隔的病态响应) — 头空, 体按空处理
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        head.push_str(&line);
    }
    // 体: Content-Length 优先, 无则读到 EOF
    let content_length = head
        .to_ascii_lowercase()
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse::<usize>().ok());
    let mut body = String::new();
    match content_length {
        Some(n) => {
            let mut buf = vec![0u8; n];
            buffered_reader.read_exact(&mut buf)?;
            body = String::from_utf8_lossy(&buf).into_owned();
        }
        None => {
            buffered_reader.read_to_string(&mut body)?;
        }
    }
    Ok(body)
}

impl HttpHelper {
    /// Java 隐式默认构造器 + 字段初始化器。
    /// PORT: `Application.httpHeader` 未翻译, 由参数注入 (Lang.httpHeader 缺省 "\n")。
    pub fn new(http_header: &str) -> Self {
        HttpHelper {
            state_request: "GET /state HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            indic_request: "GET /indicators HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            mapobj_request: "GET /map_obj.json HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            mapinfo_request: "GET /map_info.json HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            set_alt_req: "GET /editor/fm_commands?cmd=setAlt&value=%d HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            set_vel_req: "GET /editor/fm_commands?cmd=setVelocity&value=%.0f HTTP/1.1\nHost: 127.0.0.1\nCache-Control:no-cache\n".to_string()
                + http_header + "\n",
            str_state: std::sync::Arc::new(std::sync::Mutex::new(NSTRING.to_string())),
            str_indic: NSTRING.to_string(),
            str_map_obj: NSTRING.to_string(),
            str_map_info: NSTRING.to_string(),
            http_header: http_header.to_string(),
        }
    }

    /// 对应 Java `fmCmdSetAlt(int alt, SocketAddress dest)`。
    /// Java `String.format(setAltReq, alt)` 的 `%d` → 十进制整数。
    pub fn fm_cmd_set_alt(&self, alt: i32, dest: SocketAddr) -> io::Result<()> {
        // PORT: String.format 模板含 %d 占位符, 动态模板不能用 format!,
        // 以 replace 复刻 (模板内仅此一处占位符)
        let tmp_req = self.set_alt_req.replace("%d", &alt.to_string());
        let mut socket = TcpStream::connect_timeout(&dest, CONNECT_TIMEOUT)?;
        socket.set_write_timeout(Some(READ_TIMEOUT))?;
        socket.write_all(tmp_req.as_bytes())?;
        Ok(())
    }

    /// 对应 Java `fmCmdSetSpd(double spd, SocketAddress dest)`。
    pub fn fm_cmd_set_spd(&self, spd: f64, dest: SocketAddr) -> io::Result<()> {
        let tmp_req = self.set_vel_req.replace("%.0f", &format_f_half_up_0(spd));
        let mut socket = TcpStream::connect_timeout(&dest, CONNECT_TIMEOUT)?;
        socket.set_write_timeout(Some(READ_TIMEOUT))?;
        socket.write_all(tmp_req.as_bytes())?;
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
                // TODO(https): 真实功能缺口 — 唯一生产调用方 checkUpdate 固定
                // https URL, 引入 reqwest/TLS 栈前更新检查持续失败 (静默跳过)
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
            let result: String = String::from_utf8_lossy(&body_bytes).lines().collect();
            if url.contains("api.github.com") {
                logger::info(
                    "Update",
                    &format!("Latest version info fetched successfully (HTTP {})", code),
                );
            }
            return Ok(result);
        }
    }

    /// 对应 Java `getReqResult(SocketAddress req_addr)`。
    /// 波5 重写: 同线程顺序 GET /state → /indicators (原 spawn 线程 +
    /// CompletableFuture 直译退役); 任一失败 → 空串复位 (走调用方既有
    /// 端口翻转/等待分支 — 原无限阻塞怪癖退役, 见模块头备案 1)。
    pub fn get_req_result(&mut self, req_addr: SocketAddr, _stop: &AtomicBool) {
        let state = http_get(&self.state_request, req_addr);
        let indic = http_get(&self.indic_request, req_addr);
        match (state, indic) {
            (Ok(s), Ok(i)) => {
                *self.str_state.lock().unwrap() = s;
                self.str_indic = i;
            }
            _ => {
                // IO 异常静默复位 (常见于网络问题/游戏未开 — Java 同语义)
                *self.str_state.lock().unwrap() = NSTRING.to_string();
                self.str_indic = NSTRING.to_string();
            }
        }
    }

    /// 对应 Java `getReqMapObjResult(SocketAddress req_addr)`
    pub fn get_req_map_obj_result(&mut self, req_addr: SocketAddr) {
        match http_get(&self.mapobj_request, req_addr) {
            Ok(s) => self.str_map_obj = s,
            Err(_) => self.str_map_obj = NSTRING.to_string(),
        }
    }

    /// 对应 Java `getReqMapInfoResult(SocketAddress req_addr)`
    pub fn get_req_map_info_result(&mut self, req_addr: SocketAddr) {
        match http_get(&self.mapinfo_request, req_addr) {
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
        // 使用 127.0.0.1:8111 作为目标
        let dest: SocketAddr = "127.0.0.1:8111".parse().unwrap();
        let indicators_json = match http_get(&self.indic_request, dest) {
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
        None => (
            authority.to_string(),
            if scheme == "https" { 443 } else { 80 },
        ),
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
        return Ok(format!(
            "{}://{}:{}{}",
            base_scheme, base_host, base_port, loc
        ));
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
        let nl = find_subslice(&body[pos..], b"\r\n").ok_or("chunked: 缺行尾")? + pos;
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
