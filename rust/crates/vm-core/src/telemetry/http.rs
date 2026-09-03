//! HttpHelper 的 Rust 移植 (src/prog/util/HttpHelper.java)
//! 8111/9222 双端口 HTTP 客户端: 取数 (getReqResult 系, Socket 层手写 HTTP)、
//! live 机型探测 (getLiveAircraftType)。
//! (波20 清场: fmCmdSetAlt/fmCmdSetSpd (FM 编辑器指令, UI 未移植) 与
//! sendGetURL (更新检查已迁前端 web fetch) 全库零生产调用, 已删)
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
//! 保留: 8111/9222 双端口翻转 (调用方)、httpHeader 注入、getLiveAircraftType。
//! §2.13: stop 停机标志参数保留 (get_req_result 签名), 超时上限使阻塞有界。

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

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

        // PORT: type 经 Indicators.update 已 toUpperCase; Java 的
        // `t != "No Cockpit"` 恒真 (NO COCKPIT), 波20 清场删除该死比较
        if indicators_parser.valid == Some(true) {
            if let Some(t) = indicators_parser.r#type.as_deref() {
                if !t.is_empty() {
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

#[cfg(test)]
mod tests;
