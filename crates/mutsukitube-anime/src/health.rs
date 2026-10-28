use crate::errors::AnimeError;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderHealthState {
    Healthy,
    CoolingDown,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct ProviderHealthRecord {
    pub failures: u32,
    pub cooldown_until: Option<Instant>,
    pub disabled: bool,
}

impl Default for ProviderHealthRecord {
    fn default() -> Self {
        Self {
            failures: 0,
            cooldown_until: None,
            disabled: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct ProviderHealthTracker {
    records: Mutex<HashMap<String, ProviderHealthRecord>>,
}

impl ProviderHealthTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self, provider_id: &str, now: Instant) -> ProviderHealthState {
        let records = self.records.lock().unwrap();
        let record = records.get(provider_id).cloned().unwrap_or_default();
        if record.disabled {
            return ProviderHealthState::Disabled;
        }
        if let Some(until) = record.cooldown_until {
            if now < until {
                return ProviderHealthState::CoolingDown;
            }
        }
        ProviderHealthState::Healthy
    }

    pub fn is_available(&self, provider_id: &str, now: Instant) -> bool {
        self.state(provider_id, now) == ProviderHealthState::Healthy
    }

    pub fn record_success(&self, provider_id: &str) {
        let mut records = self.records.lock().unwrap();
        records.remove(provider_id);
    }

    pub fn record_empty(&self, _provider_id: &str) {
        // Legitimate empty results do not affect provider health.
    }

    pub fn record_failure(
        &self,
        provider_id: &str,
        error: &AnimeError,
        now: Instant,
        threshold: u32,
        base_cooldown: Duration,
        max_cooldown: Duration,
    ) {
        let mut records = self.records.lock().unwrap();
        let record = records.entry(provider_id.to_string()).or_default();
        if !error.retryable && error.category != crate::errors::CATEGORY_RATE_LIMITED {
            // Non-retryable failures still count as one availability failure
            // for provider-unavailable style errors.
        }
        record.failures = record.failures.saturating_add(1);
        if record.failures >= threshold {
            let multiplier = record.failures.saturating_sub(threshold) + 1;
            let cooldown = base_cooldown.saturating_mul(multiplier).min(max_cooldown);
            record.cooldown_until = Some(now + cooldown);
        }
        if error.category == crate::errors::CATEGORY_RATE_LIMITED {
            record.cooldown_until = Some(now + base_cooldown);
        }
    }

    pub fn disable(&self, provider_id: &str) {
        let mut records = self.records.lock().unwrap();
        records.entry(provider_id.to_string()).or_default().disabled = true;
    }

    pub fn enable(&self, provider_id: &str) {
        let mut records = self.records.lock().unwrap();
        if let Some(record) = records.get_mut(provider_id) {
            record.disabled = false;
        }
    }

    pub fn cooldown_until(&self, provider_id: &str) -> Option<Instant> {
        let records = self.records.lock().unwrap();
        records.get(provider_id).and_then(|r| r.cooldown_until)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AnimeError;
    use std::time::Duration;

    #[test]
    fn healthy_initially() {
        let tracker = ProviderHealthTracker::new();
        assert_eq!(
            tracker.state("p", Instant::now()),
            ProviderHealthState::Healthy
        );
    }

    #[test]
    fn failures_trigger_cooldown() {
        let tracker = ProviderHealthTracker::new();
        let now = Instant::now();
        let err = AnimeError::new(crate::errors::CATEGORY_NETWORK_ERROR, "x", true);
        for _ in 0..3 {
            tracker.record_failure(
                "p",
                &err,
                now,
                3,
                Duration::from_secs(10),
                Duration::from_secs(60),
            );
        }
        assert_eq!(tracker.state("p", now), ProviderHealthState::CoolingDown);
    }

    #[test]
    fn cooldown_expires() {
        let tracker = ProviderHealthTracker::new();
        let now = Instant::now();
        let err = AnimeError::new(crate::errors::CATEGORY_NETWORK_ERROR, "x", true);
        for _ in 0..3 {
            tracker.record_failure(
                "p",
                &err,
                now,
                3,
                Duration::from_secs(10),
                Duration::from_secs(60),
            );
        }
        assert!(tracker.is_available("p", now + Duration::from_secs(11)));
    }

    #[test]
    fn success_clears_state() {
        let tracker = ProviderHealthTracker::new();
        let now = Instant::now();
        let err = AnimeError::new(crate::errors::CATEGORY_NETWORK_ERROR, "x", true);
        tracker.record_failure(
            "p",
            &err,
            now,
            1,
            Duration::from_secs(10),
            Duration::from_secs(60),
        );
        tracker.record_success("p");
        assert!(tracker.is_available("p", now));
    }

    #[test]
    fn empty_result_does_not_trigger_cooldown() {
        let tracker = ProviderHealthTracker::new();
        tracker.record_empty("p");
        assert!(tracker.is_available("p", Instant::now()));
    }

    #[test]
    fn disabled_provider_stays_disabled() {
        let tracker = ProviderHealthTracker::new();
        tracker.disable("p");
        assert_eq!(
            tracker.state("p", Instant::now()),
            ProviderHealthState::Disabled
        );
    }
}
