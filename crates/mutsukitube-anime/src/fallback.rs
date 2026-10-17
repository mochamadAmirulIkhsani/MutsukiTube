use crate::errors::{AnimeError, *};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackAction {
    Continue,
    Stop,
    Cooldown,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    Success,
    EmptySearch,
    NetworkError,
    Timeout,
    RateLimited,
    ProviderUnavailable,
    AnimeNotFound,
    EpisodeNotFound,
    ParserChanged,
    NoPlayableMirror,
    AuthRequired,
    Unsupported,
    Unknown,
    Cancelled,
    Disabled,
    CoolingDown,
    IdentityMismatch,
    EpisodeMismatch,
    AmbiguousMatch,
    Exhausted,
}

#[derive(Debug, Clone)]
pub struct ProviderAttempt {
    pub provider_id: String,
    pub reason: FallbackReason,
    pub error: Option<AnimeError>,
}

#[derive(Debug, Clone)]
pub struct FallbackPolicy {
    pub max_provider_attempts: usize,
    pub failure_threshold: u32,
    pub base_cooldown: Duration,
    pub max_cooldown: Duration,
    pub allow_empty_search_fallback: bool,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self {
            max_provider_attempts: 6,
            failure_threshold: 3,
            base_cooldown: Duration::from_secs(30),
            max_cooldown: Duration::from_secs(300),
            allow_empty_search_fallback: true,
        }
    }
}

impl FallbackPolicy {
    pub fn decision_for_error(&self, error: &AnimeError) -> FallbackAction {
        match error.category.as_str() {
            CATEGORY_NETWORK_ERROR
            | CATEGORY_TIMEOUT
            | CATEGORY_PROVIDER_UNAVAILABLE
            | CATEGORY_PARSER_CHANGED
            | CATEGORY_NO_PLAYABLE_MIRROR
            | CATEGORY_ANIME_NOT_FOUND
            | CATEGORY_EPISODE_NOT_FOUND
            | CATEGORY_RATE_LIMITED => FallbackAction::Continue,
            CATEGORY_AUTH_REQUIRED => FallbackAction::Continue,
            CATEGORY_UNSUPPORTED => FallbackAction::Continue,
            _ => FallbackAction::Stop,
        }
    }

    pub fn reason_for_error(&self, error: &AnimeError) -> FallbackReason {
        match error.category.as_str() {
            CATEGORY_NETWORK_ERROR => FallbackReason::NetworkError,
            CATEGORY_TIMEOUT => FallbackReason::Timeout,
            CATEGORY_RATE_LIMITED => FallbackReason::RateLimited,
            CATEGORY_PROVIDER_UNAVAILABLE => FallbackReason::ProviderUnavailable,
            CATEGORY_ANIME_NOT_FOUND => FallbackReason::AnimeNotFound,
            CATEGORY_EPISODE_NOT_FOUND => FallbackReason::EpisodeNotFound,
            CATEGORY_PARSER_CHANGED => FallbackReason::ParserChanged,
            CATEGORY_NO_PLAYABLE_MIRROR => FallbackReason::NoPlayableMirror,
            CATEGORY_AUTH_REQUIRED => FallbackReason::AuthRequired,
            CATEGORY_UNSUPPORTED => FallbackReason::Unsupported,
            _ => FallbackReason::Unknown,
        }
    }

    pub fn decision_for_empty_search(&self) -> FallbackAction {
        if self.allow_empty_search_fallback {
            FallbackAction::Continue
        } else {
            FallbackAction::Stop
        }
    }

    pub fn reason_for_empty_search(&self) -> FallbackReason {
        FallbackReason::EmptySearch
    }

    pub fn cooldown_for_failures(&self, failures: u32) -> Duration {
        if failures < self.failure_threshold {
            return Duration::ZERO;
        }
        let multiplier = failures.saturating_sub(self.failure_threshold) + 1;
        let cooldown = self.base_cooldown.saturating_mul(multiplier);
        cooldown.min(self.max_cooldown)
    }
}

#[derive(Debug, Clone)]
pub struct SearchFallbackOutcome {
    pub results: Vec<crate::models::Anime>,
    pub provider_id: Option<String>,
    pub attempts: Vec<ProviderAttempt>,
    pub cancelled: bool,
}

#[derive(Debug, Clone)]
pub struct StreamFallbackOutcome {
    pub stream: crate::models::StreamInfo,
    pub provider_id: String,
    pub anime_url: String,
    pub episode_url: String,
    pub attempts: Vec<ProviderAttempt>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(category: &str) -> AnimeError {
        AnimeError::new(category, "test", false)
    }

    #[test]
    fn network_error_continues() {
        let policy = FallbackPolicy::default();
        assert_eq!(
            policy.decision_for_error(&err(CATEGORY_NETWORK_ERROR)),
            FallbackAction::Continue
        );
    }

    #[test]
    fn timeout_continues() {
        let policy = FallbackPolicy::default();
        assert_eq!(
            policy.decision_for_error(&err(CATEGORY_TIMEOUT)),
            FallbackAction::Continue
        );
    }

    #[test]
    fn rate_limited_continues() {
        let policy = FallbackPolicy::default();
        assert_eq!(
            policy.decision_for_error(&err(CATEGORY_RATE_LIMITED)),
            FallbackAction::Continue
        );
    }

    #[test]
    fn provider_unavailable_continues() {
        let policy = FallbackPolicy::default();
        assert_eq!(
            policy.decision_for_error(&err(CATEGORY_PROVIDER_UNAVAILABLE)),
            FallbackAction::Continue
        );
    }

    #[test]
    fn parser_changed_continues() {
        let policy = FallbackPolicy::default();
        assert_eq!(
            policy.decision_for_error(&err(CATEGORY_PARSER_CHANGED)),
            FallbackAction::Continue
        );
    }

    #[test]
    fn no_playable_mirror_continues() {
        let policy = FallbackPolicy::default();
        assert_eq!(
            policy.decision_for_error(&err(CATEGORY_NO_PLAYABLE_MIRROR)),
            FallbackAction::Continue
        );
    }

    #[test]
    fn empty_search_continues_by_default() {
        let policy = FallbackPolicy::default();
        assert_eq!(policy.decision_for_empty_search(), FallbackAction::Continue);
    }

    #[test]
    fn unknown_error_stops() {
        let policy = FallbackPolicy::default();
        assert_eq!(
            policy.decision_for_error(&err("SOMETHING_NEW")),
            FallbackAction::Stop
        );
    }

    #[test]
    fn cooldown_grows_with_failures() {
        let policy = FallbackPolicy::default();
        assert_eq!(policy.cooldown_for_failures(2), Duration::ZERO);
        assert_eq!(policy.cooldown_for_failures(3), Duration::from_secs(30));
        assert_eq!(policy.cooldown_for_failures(4), Duration::from_secs(60));
    }

    #[test]
    fn cooldown_is_capped() {
        let policy = FallbackPolicy::default();
        assert_eq!(policy.cooldown_for_failures(100), Duration::from_secs(300));
    }
}
