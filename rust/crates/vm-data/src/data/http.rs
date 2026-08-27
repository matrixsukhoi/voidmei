//! 手写 HTTP/1.1 GET (固定 127.0.0.1:8111, 无 TLS/代理; 对齐 Java HttpHelper 的 Connection: close 行为)

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// GET path, 返回响应 body (读到 EOF; 支持 chunked 防御性解码)
pub fn http_get(port: u16, path: &str, timeout: Duration) -> Result<Vec<u8>, String> {
    let addr = format!("127.0.0.1:{}", port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| format!("connect {}: {}", addr, e))?;
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    let req = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        path, port
    );
    stream.write_all(req.as_bytes())
        .map_err(|e| format!("write: {}", e))?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)
        .map_err(|e| format!("read: {}", e))?;

    // 分离 header/body
    let sep = find_subslice(&buf, b"\r\n\r\n")
        .ok_or("响应缺少 header 分隔")?;
    let (head, body) = buf.split_at(sep + 4);
    let head_str = String::from_utf8_lossy(&head[..sep]).to_string();

    if let Some(cl) = content_length(&head_str) {
        if body.len() >= cl {
            return Ok(body[..cl].to_vec());
        }
    }
    if head_str.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        return decode_chunked(body);
    }
    Ok(body.to_vec()) // EOF 兜底 (mock 无 Content-Length 的契约行为)
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn content_length(head: &str) -> Option<usize> {
    for line in head.split("\r\n") {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            return v.trim().parse().ok();
        }
    }
    None
}

/// ~30 行 chunked 解码 (防御: WT 实测为 Content-Length, 正常不走此路径)
fn decode_chunked(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    loop {
        let nl = find_subslice(&body[pos..], b"\r\n")
            .ok_or("chunked: 缺行尾")? + pos;
        let size_str = std::str::from_utf8(&body[pos..nl])
            .map_err(|_| "chunked: size 非法")?;
        let size = usize::from_str_radix(size_str.trim().split(';').next().unwrap_or("0"), 16)
            .map_err(|_| "chunked: size 非十六进制")?;
        pos = nl + 2;
        if size == 0 {
            break;
        }
        if pos + size > body.len() {
            return Err("chunked: body 截断".into());
        }
        out.extend_from_slice(&body[pos..pos + size]);
        pos += size + 2; // 跳过块尾 CRLF
    }
    Ok(out)
}
