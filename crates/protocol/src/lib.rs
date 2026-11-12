use serde::{Deserialize, Serialize};
use serde_json::Value;
pub const VERSION: &str = "1.0";
pub const PIPE: &str = r"\\.\pipe\guidance-runtime-v1";
pub const SOCKET: &str = "/tmp/guidance-runtime-v1.sock";
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub protocol: String,
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}
impl Request {
    pub fn new(method: &str, params: Value) -> Self {
        Self {
            protocol: VERSION.into(),
            id: uuid::Uuid::new_v4().to_string(),
            method: method.into(),
            params,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub protocol: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}
impl Response {
    pub fn ok(id: String, v: Value) -> Self {
        Self {
            protocol: VERSION.into(),
            id,
            result: Some(v),
            error: None,
        }
    }
    pub fn err(id: String, c: &str, m: String) -> Self {
        Self {
            protocol: VERSION.into(),
            id,
            result: None,
            error: Some(ApiError {
                code: c.into(),
                message: m,
            }),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadStatus {
    pub id: String,
    pub state: String,
    pub pid: Option<u32>,
    pub restart_count: u32,
    pub last_error: Option<String>,
    pub endpoint: Option<String>,
}
