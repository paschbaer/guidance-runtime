use crate::error::DispatchError;
use serde_json::Value;

/// Upper bound for a `workload.logs` tail request.
pub const MAX_TAIL: usize = 5000;
/// Default number of log lines when the request omits `tail`.
pub const DEFAULT_TAIL: usize = 100;

/// Extracts the mandatory `workload_id` parameter.
pub fn workload_id(params: &Value) -> Result<&str, DispatchError> {
    params
        .get("workload_id")
        .and_then(Value::as_str)
        .ok_or_else(|| DispatchError::MissingParam("workload_id required".into()))
}

/// Extracts the optional `tail` parameter, defaulting to [`DEFAULT_TAIL`] and
/// clamped to [`MAX_TAIL`].
pub fn tail(params: &Value) -> usize {
    params
        .get("tail")
        .and_then(Value::as_u64)
        .map(|n| (n as usize).min(MAX_TAIL))
        .unwrap_or(DEFAULT_TAIL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn workload_id_is_required() {
        let err = workload_id(&json!({})).unwrap_err();
        assert_eq!(err.code(), "internal_error");
        assert_eq!(err.message(), "workload_id required");
    }

    #[test]
    fn workload_id_must_be_a_string() {
        assert!(workload_id(&json!({ "workload_id": 42 })).is_err());
    }

    #[test]
    fn workload_id_is_returned_verbatim() {
        assert_eq!(
            workload_id(&json!({ "workload_id": "api" })).unwrap(),
            "api"
        );
    }

    #[test]
    fn tail_defaults_to_one_hundred() {
        assert_eq!(tail(&json!({})), DEFAULT_TAIL);
    }

    #[test]
    fn tail_is_clamped_to_the_maximum() {
        assert_eq!(tail(&json!({ "tail": 99_999 })), MAX_TAIL);
    }

    #[test]
    fn tail_passes_through_normal_values() {
        assert_eq!(tail(&json!({ "tail": 25 })), 25);
    }
}
