#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StrictPreflightValidationError {
    code: &'static str,
    message: String,
}

impl StrictPreflightValidationError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

pub(crate) fn validate_required_strict_preflight_input_v0(
    root: &Path,
) -> Result<(), StrictPreflightValidationError> {
    load_required_strict_preflight_input_v0(root)
        .map(|_| ())
        .map_err(|err| StrictPreflightValidationError::new(err.code(), err.message()))
}

