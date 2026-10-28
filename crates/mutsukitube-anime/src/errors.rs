use std::fmt;

/// Coarse error category reported by Go providers over IPC.
///
/// The category is kept as a string so future protocol categories are
/// preserved instead of being collapsed into an "unknown" variant.
pub type ErrorCategory = String;

pub const CATEGORY_NETWORK_ERROR: &str = "NETWORK_ERROR";
pub const CATEGORY_TIMEOUT: &str = "TIMEOUT";
pub const CATEGORY_RATE_LIMITED: &str = "RATE_LIMITED";
pub const CATEGORY_PROVIDER_UNAVAILABLE: &str = "PROVIDER_UNAVAILABLE";
pub const CATEGORY_ANIME_NOT_FOUND: &str = "ANIME_NOT_FOUND";
pub const CATEGORY_EPISODE_NOT_FOUND: &str = "EPISODE_NOT_FOUND";
pub const CATEGORY_PARSER_CHANGED: &str = "PARSER_CHANGED";
pub const CATEGORY_NO_PLAYABLE_MIRROR: &str = "NO_PLAYABLE_MIRROR";
pub const CATEGORY_AUTH_REQUIRED: &str = "AUTH_REQUIRED";
pub const CATEGORY_UNSUPPORTED: &str = "UNSUPPORTED";
pub const CATEGORY_PROTOCOL_VERSION_MISMATCH: &str = "PROTOCOL_VERSION_MISMATCH";
pub const CATEGORY_OVERSIZED_RESPONSE: &str = "OVERSIZED_RESPONSE";
pub const CATEGORY_MALFORMED_RESPONSE: &str = "MALFORMED_RESPONSE";
pub const CATEGORY_PROCESS_EXITED: &str = "PROCESS_EXITED";
pub const CATEGORY_PROCESS_SPAWN_FAILED: &str = "PROCESS_SPAWN_FAILED";
pub const CATEGORY_INVALID_PROVIDER: &str = "INVALID_PROVIDER";
pub const CATEGORY_DISABLED_PROVIDER: &str = "DISABLED_PROVIDER";
pub const CATEGORY_INVALID_EXECUTABLE: &str = "INVALID_EXECUTABLE_PATH";
pub const CATEGORY_PENDING_LIMIT: &str = "PENDING_REQUEST_LIMIT";
pub const CATEGORY_PROVIDER_NOT_FOUND: &str = "PROVIDER_NOT_FOUND";
pub const CATEGORY_CANCELLED: &str = "CANCELLED";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeError {
    pub category: ErrorCategory,
    pub message: String,
    pub retryable: bool,
}

impl AnimeError {
    pub fn new(
        category: impl Into<ErrorCategory>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            category: category.into(),
            message: message.into(),
            retryable,
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.retryable
    }
}

impl fmt::Display for AnimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.category, self.message)
    }
}

impl std::error::Error for AnimeError {}

impl From<std::io::Error> for AnimeError {
    fn from(value: std::io::Error) -> Self {
        Self::new(CATEGORY_NETWORK_ERROR, value.to_string(), true)
    }
}

impl From<serde_json::Error> for AnimeError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(CATEGORY_MALFORMED_RESPONSE, value.to_string(), false)
    }
}
