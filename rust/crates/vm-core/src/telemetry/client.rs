//! 8111 游戏本地 API HTTP 客户端 (GameApiClient, 原 Java HttpHelper)。
//!
//! 波20 ureq 化: 手写 socket HTTP (裸 TcpStream 拼请求串 + 手工头解析) 退役 →
//! ureq 2.x 阻塞客户端 (无 TLS 特性 — 纯 localhost 场景), 与"单线程轮询线程
//! 独占"的波5 设计契合。HttpHelper → GameApiClient; 请求模板串退役 (端点
//! 路径常量化 + 标准 header API); get_live_aircraft_type 的 8111 硬编码改端口
//! 注入 (测试不再占用真实 8111)。
//!
//! 沿袭语义 (波5 备案):
//! - /state + /indicators 同线程顺序 GET; 任一失败 → 双双空串复位
//!   (走调用方既有端口翻转/等待分支);
//! - connect 250ms / read 500ms 超时上限 (阻塞有界);
//! - httpHeader 注入 (Lang 配置的 "Name: value" 行串 → 标准 header)。

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::telemetry::parser::Indicators;

/// 对应 Java HttpHelper.nstring (失败复位空串)
pub const NSTRING: &str = "";

/// 连接超时 (本地服务器上限)
const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
/// 读超时 (本地服务器上限)
const READ_TIMEOUT: Duration = Duration::from_millis(500);

/// 8111 游戏本地 API 客户端 (单线程阻塞, Service 轮询线程独占)。
pub struct GameApiClient {
    agent: ureq::Agent,
    /// Java httpHeader (Lang 配置, 启动后不变): "Name: value\r\n" 行串,
    /// 构造期解析为名值对, 经标准 header API 注入每个请求
    extra_headers: Vec<(String, String)>,
    /// /state 响应体 — Arc<Mutex> 跨线程读面 + vm-data tests mock 注入面
    pub str_state: Arc<Mutex<String>>,
    pub str_indic: String,
    pub str_map_obj: String,
    pub str_map_info: String,
}

/// "Name: value\r\n" 行串 → 名值对; 缺省 "\n" → 空表
fn parse_header_lines(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.is_empty() {
                return None;
            }
            let (n, v) = l.split_once(':')?;
            Some((n.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

impl GameApiClient {
    pub fn new(http_header: &str) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(CONNECT_TIMEOUT)
            .timeout_read(READ_TIMEOUT)
            .build();
        GameApiClient {
            agent,
            extra_headers: parse_header_lines(http_header),
            str_state: Arc::new(Mutex::new(NSTRING.to_string())),
            str_indic: NSTRING.to_string(),
            str_map_obj: NSTRING.to_string(),
            str_map_info: NSTRING.to_string(),
        }
    }

    /// 单次 GET: 成功返回响应体, 失败 (连接/超时/HTTP 错误码) → None。
    fn get_text(&self, dest: SocketAddr, path: &str) -> Option<String> {
        let url = format!("http://{}{}", dest, path);
        let mut req = self.agent.get(&url).set("Cache-Control", "no-cache");
        for (n, v) in &self.extra_headers {
            req = req.set(n, v);
        }
        // Java catch(Exception) 兜底语义: 任何 IO/状态错误统一按失败处理
        match req.call() {
            Ok(resp) => resp.into_string().ok(),
            Err(_) => None,
        }
    }

    /// 对应 Java `getReqResult`: 顺序 GET /state → /indicators;
    /// 任一失败 → 双双空串复位 (端口翻转/等待信号)。
    /// (波20: stop 停机标志死参数退役 — 超时上限已使阻塞有界)
    pub fn get_req_result(&mut self, req_addr: SocketAddr) {
        let state = self.get_text(req_addr, "/state");
        let indic = self.get_text(req_addr, "/indicators");
        match (state, indic) {
            (Some(s), Some(i)) => {
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

    /// 对应 Java `getReqMapObjResult`
    pub fn get_req_map_obj_result(&mut self, req_addr: SocketAddr) {
        match self.get_text(req_addr, "/map_obj.json") {
            Some(s) => self.str_map_obj = s,
            None => self.str_map_obj = NSTRING.to_string(),
        }
    }

    /// 对应 Java `getReqMapInfoResult`
    pub fn get_req_map_info_result(&mut self, req_addr: SocketAddr) {
        match self.get_text(req_addr, "/map_info.json") {
            Some(s) => self.str_map_info = s,
            None => self.str_map_info = NSTRING.to_string(),
        }
    }

    /// 获取指定端口 (缺省 8111) 的实时机型信息; 失败/无效返回 None。
    /// (波20: 端口从硬编码 127.0.0.1:8111 改注入 — 测试可用任意 mock 端口)
    pub fn get_live_aircraft_type(&self, port: u16) -> Option<String> {
        let dest: SocketAddr = format!("127.0.0.1:{}", port).parse().ok()?;
        let indicators_json = self.get_text(dest, "/indicators")?;

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
