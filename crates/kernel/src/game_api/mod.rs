//! 游戏本地 API 域 (8111): ureq HTTP 客户端 + State/Indicators/地图 serde 解析器。
//! (波20 更名: telemetry → game_api — 名字描述职责而非数据性质;
//!  8111 是固定端口术语, 可被 CLI/cfg 覆写为 9222 等)

pub mod client;
pub mod parser;
