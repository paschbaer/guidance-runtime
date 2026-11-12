use guidance_protocol::UnknownMethod;

/// Typed failure of a single dispatch, mapped exhaustively onto the stable
/// wire error codes. Replaces the former `message.contains("not found")`
/// heuristic: the code is now a property of the variant, not of the text.
#[derive(Debug)]
pub enum DispatchError {
    /// The `method` field named no known method.
    UnknownMethod(UnknownMethod),
    /// A referenced workload (or other addressed entity) does not exist.
    NotFound(String),
    /// A mandatory request parameter is missing or malformed. Reports
    /// `internal_error` on purpose: that is what clients received before the
    /// typed mapping existed, and the wire contract must not change.
    MissingParam(String),
    /// Anything else.
    Internal(anyhow::Error),
}

impl DispatchError {
    /// The `error.code` value sent to the client. These strings are part of
    /// the wire contract and must not change.
    pub fn code(&self) -> &'static str {
        match self {
            DispatchError::UnknownMethod(_) => "not_found",
            DispatchError::NotFound(_) => "not_found",
            DispatchError::MissingParam(_) => "internal_error",
            DispatchError::Internal(_) => "internal_error",
        }
    }

    pub fn message(&self) -> String {
        match self {
            DispatchError::UnknownMethod(e) => e.to_string(),
            DispatchError::NotFound(m) | DispatchError::MissingParam(m) => m.clone(),
            DispatchError::Internal(e) => e.to_string(),
        }
    }

    /// Explicitly classifies an unhandled failure. Handlers call this instead
    /// of relying on `From<anyhow::Error>`, so the conversion cannot fire by
    /// accident through `?`.
    pub fn internal(e: anyhow::Error) -> Self {
        DispatchError::Internal(e)
    }
}

impl From<UnknownMethod> for DispatchError {
    fn from(e: UnknownMethod) -> Self {
        DispatchError::UnknownMethod(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guidance_protocol::Method;

    #[test]
    fn unknown_method_reports_the_legacy_message() {
        let err = DispatchError::from("workload.explode".parse::<Method>().unwrap_err());
        assert_eq!(err.code(), "not_found");
        assert_eq!(err.message(), "method not found: workload.explode");
    }

    #[test]
    fn not_found_maps_to_not_found_code() {
        let err = DispatchError::NotFound("workload not found: api".into());
        assert_eq!(err.code(), "not_found");
        assert_eq!(err.message(), "workload not found: api");
    }

    #[test]
    fn missing_param_keeps_the_internal_error_code() {
        let err = DispatchError::MissingParam("workload_id required".into());
        assert_eq!(err.code(), "internal_error");
        assert_eq!(err.message(), "workload_id required");
    }

    #[test]
    fn internal_maps_to_internal_error() {
        let err = DispatchError::internal(anyhow::anyhow!("boom"));
        assert_eq!(err.code(), "internal_error");
        assert_eq!(err.message(), "boom");
    }
}
