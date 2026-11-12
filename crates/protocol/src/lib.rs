use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
pub const VERSION: &str = "1.0";
pub const PIPE: &str = r"\\.\pipe\guidance-runtime-v1";
pub const SOCKET: &str = "/tmp/guidance-runtime-v1.sock";

/// Wire method names shared by every client and by the runtime dispatcher.
///
/// This enum is the single source of truth for the request `method` field: a
/// renamed variant string breaks the protocol for existing clients, which is
/// why the wire values are pinned by tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    RuntimeStatus,
    WorkloadList,
    WorkloadStart,
    WorkloadStop,
    WorkloadRestart,
    WorkloadStopAll,
    WorkloadStartAll,
    WorkloadLogs,
    DiagnosticsSummary,
}

impl Method {
    pub const ALL: [Method; 9] = [
        Method::RuntimeStatus,
        Method::WorkloadList,
        Method::WorkloadStart,
        Method::WorkloadStop,
        Method::WorkloadRestart,
        Method::WorkloadStopAll,
        Method::WorkloadStartAll,
        Method::WorkloadLogs,
        Method::DiagnosticsSummary,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Method::RuntimeStatus => "runtime.status",
            Method::WorkloadList => "workload.list",
            Method::WorkloadStart => "workload.start",
            Method::WorkloadStop => "workload.stop",
            Method::WorkloadRestart => "workload.restart",
            Method::WorkloadStopAll => "workload.stop_all",
            Method::WorkloadStartAll => "workload.start_all",
            Method::WorkloadLogs => "workload.logs",
            Method::DiagnosticsSummary => "diagnostics.summary",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Method {
    type Err = UnknownMethod;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "runtime.status" => Ok(Method::RuntimeStatus),
            "workload.list" => Ok(Method::WorkloadList),
            "workload.start" => Ok(Method::WorkloadStart),
            "workload.stop" => Ok(Method::WorkloadStop),
            "workload.restart" => Ok(Method::WorkloadRestart),
            "workload.stop_all" => Ok(Method::WorkloadStopAll),
            "workload.start_all" => Ok(Method::WorkloadStartAll),
            "workload.logs" => Ok(Method::WorkloadLogs),
            "diagnostics.summary" => Ok(Method::DiagnosticsSummary),
            other => Err(UnknownMethod(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownMethod(pub String);

impl fmt::Display for UnknownMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "method not found: {}", self.0)
    }
}

impl std::error::Error for UnknownMethod {}
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_are_pinned() {
        let expected = [
            "runtime.status",
            "workload.list",
            "workload.start",
            "workload.stop",
            "workload.restart",
            "workload.stop_all",
            "workload.start_all",
            "workload.logs",
            "diagnostics.summary",
        ];
        let actual: Vec<&str> = Method::ALL.iter().map(|m| m.as_str()).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn every_method_round_trips() {
        for method in Method::ALL {
            assert_eq!(method.as_str().parse::<Method>(), Ok(method));
        }
    }

    #[test]
    fn unknown_method_is_rejected_with_its_name() {
        let err = "workload.explode".parse::<Method>().unwrap_err();
        assert_eq!(err, UnknownMethod("workload.explode".into()));
        assert_eq!(err.to_string(), "method not found: workload.explode");
    }
}
