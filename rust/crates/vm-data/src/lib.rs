//! vm-data: 8111 轮询与派生量计算 (POC data/ 迁入)
pub mod data;
pub use data::derive::{Deriver, FlightValues};
pub use data::json::{parse_indicators, parse_state};
pub use data::http::http_get;

pub mod service_fields;
pub mod service_loop;
