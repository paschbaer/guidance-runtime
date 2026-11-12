use crate::error::DispatchError;
use crate::params::{tail, workload_id};
use guidance_protocol::{Method, Request, Response, VERSION};
use guidance_runtime::Runtime;
use serde_json::{json, Value};

/// Handles one `runtime.status` request.
async fn runtime_status(rt: &Runtime) -> Value {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "state": "Running",
        "workloads": rt.list(),
    })
}

/// Handles one `workload.list` request.
async fn workload_list(rt: &Runtime) -> Value {
    json!(rt.list())
}

/// Handles one `workload.start` request.
async fn workload_start(rt: &Runtime, params: &Value) -> Result<Value, DispatchError> {
    match rt.start(workload_id(params)?).await {
        Ok(()) => Ok(json!({ "accepted": true })),
        Err(e) => Err(not_found_or_internal(e)),
    }
}

/// Handles one `workload.stop` request.
async fn workload_stop(rt: &Runtime, params: &Value) -> Result<Value, DispatchError> {
    match rt.stop(workload_id(params)?).await {
        Ok(()) => Ok(json!({ "accepted": true })),
        Err(e) => Err(not_found_or_internal(e)),
    }
}

/// Handles one `workload.restart` request.
async fn workload_restart(rt: &Runtime, params: &Value) -> Result<Value, DispatchError> {
    match rt.restart(workload_id(params)?).await {
        Ok(()) => Ok(json!({ "accepted": true })),
        Err(e) => Err(not_found_or_internal(e)),
    }
}

/// Handles one `workload.stop_all` request.
async fn workload_stop_all(rt: &Runtime) -> Value {
    rt.stop_all().await;
    json!({ "accepted": true })
}

/// Handles one `workload.start_all` request.
async fn workload_start_all(rt: &Runtime) -> Value {
    rt.start_enabled().await;
    json!({ "accepted": true })
}

/// Handles one `workload.logs` request.
async fn workload_logs(rt: &Runtime, params: &Value) -> Result<Value, DispatchError> {
    let id = workload_id(params)?;
    match rt.logs(id, tail(params)).await {
        Ok(lines) => Ok(json!({ "lines": lines })),
        Err(e) => Err(not_found_or_internal(e)),
    }
}

/// Handles one `diagnostics.summary` request.
async fn diagnostics_summary(rt: &Runtime) -> Value {
    json!({ "workloads": rt.list() })
}

/// `Runtime` reports a missing workload as `workload not found: <id>`; every
/// other failure is internal. Splitting on the typed prefix keeps the mapping
/// explicit instead of relying on a substring search of the whole message.
fn not_found_or_internal(e: anyhow::Error) -> DispatchError {
    const MARKER: &str = "workload not found: ";
    let message = e.to_string();
    if message.starts_with(MARKER) {
        DispatchError::NotFound(message)
    } else {
        DispatchError::internal(e)
    }
}

/// Dispatches a decoded method onto the runtime.
async fn dispatch(rt: &Runtime, method: Method, params: &Value) -> Result<Value, DispatchError> {
    match method {
        Method::RuntimeStatus => Ok(runtime_status(rt).await),
        Method::WorkloadList => Ok(workload_list(rt).await),
        Method::WorkloadStart => workload_start(rt, params).await,
        Method::WorkloadStop => workload_stop(rt, params).await,
        Method::WorkloadRestart => workload_restart(rt, params).await,
        Method::WorkloadStopAll => Ok(workload_stop_all(rt).await),
        Method::WorkloadStartAll => Ok(workload_start_all(rt).await),
        Method::WorkloadLogs => workload_logs(rt, params).await,
        Method::DiagnosticsSummary => Ok(diagnostics_summary(rt).await),
    }
}

/// Executes one request and renders the response, including the error mapping.
pub(crate) async fn run(rt: &Runtime, q: Request) -> Response {
    let id = q.id.clone();
    if q.protocol != VERSION {
        return Response::err(id, "unsupported_protocol", "expected 1.0".into());
    }
    let method = match q.method.parse::<Method>() {
        Ok(m) => m,
        Err(e) => {
            let err = DispatchError::from(e);
            return Response::err(id, err.code(), err.message());
        }
    };
    match dispatch(rt, method, &q.params).await {
        Ok(value) => Response::ok(id, value),
        Err(err) => Response::err(id, err.code(), err.message()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn workload_not_found_is_classified_as_not_found() {
        let err = not_found_or_internal(anyhow!("workload not found: api"));
        assert_eq!(err.code(), "not_found");
    }

    #[test]
    fn other_failures_stay_internal() {
        let err = not_found_or_internal(anyhow!("open log file: access denied"));
        assert_eq!(err.code(), "internal_error");
    }

    #[test]
    fn a_marker_inside_the_message_is_not_mistaken_for_not_found() {
        let err = not_found_or_internal(anyhow!("io error: workload not found: x"));
        assert_eq!(err.code(), "internal_error");
    }
}
