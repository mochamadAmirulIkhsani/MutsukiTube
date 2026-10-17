use crate::cache::BoundedCache;
use crate::errors::*;
use crate::fallback::{
    FallbackPolicy, ProviderAttempt, SearchFallbackOutcome, StreamFallbackOutcome,
};
use crate::health::ProviderHealthTracker;
use crate::identity::{
    AnimeIdentity, MatchConfidence, VerifiedAnimeMapping, find_matching_anime,
    find_matching_episode,
};
use crate::models::{Anime, Episode, PROTOCOL_VERSION, PluginInfo, StreamInfo};
use crate::process::{ProcessTimeouts, ProviderProcess, debug_logging_enabled};
use crate::registry::ProviderRegistry;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

pub struct AnimeManager {
    registry: StdMutex<ProviderRegistry>,
    plugin_dir: std::path::PathBuf,
    processes: StdMutex<HashMap<String, std::sync::Arc<ProviderProcess>>>,
    timeouts: ProcessTimeouts,
    policy: FallbackPolicy,
    health: ProviderHealthTracker,
    identity_cache: StdMutex<BoundedCache<String, VerifiedAnimeMapping>>,
}

impl AnimeManager {
    pub fn new(registry: ProviderRegistry) -> Self {
        Self::with_policy(registry, FallbackPolicy::default())
    }

    pub fn with_timeouts(registry: ProviderRegistry, timeouts: ProcessTimeouts) -> Self {
        Self {
            registry: StdMutex::new(registry),
            plugin_dir: crate::registry::plugin_dir_from_env().unwrap_or_else(|| std::path::PathBuf::from("plugins")),
            processes: StdMutex::new(HashMap::new()),
            timeouts,
            policy: FallbackPolicy::default(),
            health: ProviderHealthTracker::new(),
            identity_cache: StdMutex::new(BoundedCache::new(
                128,
                std::time::Duration::from_secs(600),
            )),
        }
    }

    pub fn with_policy(registry: ProviderRegistry, policy: FallbackPolicy) -> Self {
        Self {
            registry: StdMutex::new(registry),
            plugin_dir: crate::registry::plugin_dir_from_env().unwrap_or_else(|| std::path::PathBuf::from("plugins")),
            processes: StdMutex::new(HashMap::new()),
            timeouts: ProcessTimeouts::default(),
            policy,
            health: ProviderHealthTracker::new(),
            identity_cache: StdMutex::new(BoundedCache::new(
                128,
                std::time::Duration::from_secs(600),
            )),
        }
    }

    pub fn with_timeouts_and_policy(
        registry: ProviderRegistry,
        timeouts: ProcessTimeouts,
        policy: FallbackPolicy,
    ) -> Self {
        Self {
            registry: StdMutex::new(registry),
            plugin_dir: crate::registry::plugin_dir_from_env().unwrap_or_else(|| std::path::PathBuf::from("plugins")),
            processes: StdMutex::new(HashMap::new()),
            timeouts,
            policy,
            health: ProviderHealthTracker::new(),
            identity_cache: StdMutex::new(BoundedCache::new(
                128,
                std::time::Duration::from_secs(600),
            )),
        }
    }

    pub fn registry(&self) -> ProviderRegistry {
        self.registry.lock().unwrap().clone()
    }

    pub fn provider_ids(&self) -> Vec<String> {
        self.registry.lock().unwrap().ids_by_priority()
    }

    pub fn reload_registry(&self, plugin_dir: Option<std::path::PathBuf>) {
        let dir = plugin_dir.unwrap_or_else(|| self.plugin_dir.clone());
        let registry = ProviderRegistry::with_standard(Some(dir.clone()));
        *self.registry.lock().unwrap() = registry;
        *self.processes.lock().unwrap() = HashMap::new();
        // Update plugin_dir? keep original
    }

    fn process_for(
        &self,
        provider_id: &str,
    ) -> Result<std::sync::Arc<ProviderProcess>, AnimeError> {
        let registry = self.registry.lock().unwrap();
        let config = registry.get(provider_id).ok_or_else(|| {
            AnimeError::new(
                CATEGORY_PROVIDER_NOT_FOUND,
                format!("unknown provider {provider_id}"),
                false,
            )
        })?;

        if !config.enabled {
            return Err(AnimeError::new(
                CATEGORY_DISABLED_PROVIDER,
                format!("provider {provider_id} is disabled"),
                false,
            ));
        }

        config.validate()?;

        let mut processes = self.processes.lock().unwrap();
        if let Some(process) = processes.get(provider_id) {
            return Ok(std::sync::Arc::clone(process));
        }

        let process = ProviderProcess::new(
            provider_id,
            config.executable_path.clone(),
            config.executable_args.clone(),
            config.expected_protocol,
            self.timeouts,
        );
        processes.insert(provider_id.to_string(), std::sync::Arc::clone(&process));
        Ok(process)
    }

    pub async fn info(&self, provider_id: &str) -> Result<PluginInfo, AnimeError> {
        let process = self.process_for(provider_id)?;
        let response = process
            .request("info", json!({}), Some(self.timeouts.info_ms))
            .await?;
        let data = response.data.ok_or_else(|| {
            AnimeError::new(
                CATEGORY_MALFORMED_RESPONSE,
                "info response has no data",
                false,
            )
        })?;
        let info_value = data.get("info").cloned().ok_or_else(|| {
            AnimeError::new(
                CATEGORY_MALFORMED_RESPONSE,
                "info response has no info field",
                false,
            )
        })?;
        let mut info: PluginInfo = serde_json::from_value(info_value)
            .map_err(|e| AnimeError::new(CATEGORY_MALFORMED_RESPONSE, e.to_string(), false))?;
        if info.protocol_version == 0 {
            info.protocol_version = PROTOCOL_VERSION;
        }
        Ok(info)
    }

    pub async fn search(&self, provider_id: &str, query: &str) -> Result<Vec<Anime>, AnimeError> {
        if query.trim().is_empty() {
            return Err(AnimeError::new(
                CATEGORY_UNSUPPORTED,
                "search query is empty",
                false,
            ));
        }
        let process = self.process_for(provider_id)?;
        let response = process
            .request(
                "search",
                json!({"query": query}),
                Some(self.timeouts.search_ms),
            )
            .await?;
        let data = response.data.ok_or_else(|| {
            AnimeError::new(
                CATEGORY_MALFORMED_RESPONSE,
                "search response has no data",
                false,
            )
        })?;
        let results_value = data.get("results").cloned().unwrap_or_else(|| json!([]));
        let results: Vec<Anime> = serde_json::from_value(results_value)
            .map_err(|e| AnimeError::new(CATEGORY_MALFORMED_RESPONSE, e.to_string(), false))?;
        Ok(results)
    }

    pub async fn episodes(
        &self,
        provider_id: &str,
        anime_url: &str,
    ) -> Result<Vec<Episode>, AnimeError> {
        if anime_url.trim().is_empty() {
            return Err(AnimeError::new(
                CATEGORY_UNSUPPORTED,
                "anime URL is empty",
                false,
            ));
        }
        let process = self.process_for(provider_id)?;
        let response = process
            .request(
                "episodes",
                json!({"animeUrl": anime_url}),
                Some(self.timeouts.episodes_ms),
            )
            .await?;
        let data = response.data.ok_or_else(|| {
            AnimeError::new(
                CATEGORY_MALFORMED_RESPONSE,
                "episodes response has no data",
                false,
            )
        })?;
        let episodes_value = data.get("episodes").cloned().unwrap_or_else(|| json!([]));
        let episodes: Vec<Episode> = serde_json::from_value(episodes_value)
            .map_err(|e| AnimeError::new(CATEGORY_MALFORMED_RESPONSE, e.to_string(), false))?;
        Ok(episodes)
    }

    pub async fn heights(
        &self,
        provider_id: &str,
        episode_url: &str,
    ) -> Result<Vec<u32>, AnimeError> {
        if episode_url.trim().is_empty() {
            return Err(AnimeError::new(
                CATEGORY_UNSUPPORTED,
                "episode URL is empty",
                false,
            ));
        }
        let process = self.process_for(provider_id)?;
        let response = process
            .request(
                "heights",
                json!({"episodeUrl": episode_url}),
                Some(self.timeouts.heights_ms),
            )
            .await?;
        let data = response.data.ok_or_else(|| {
            AnimeError::new(
                CATEGORY_MALFORMED_RESPONSE,
                "heights response has no data",
                false,
            )
        })?;
        let heights_value = data.get("heights").cloned().unwrap_or_else(|| json!([]));
        let heights: Vec<i64> = serde_json::from_value(heights_value)
            .map_err(|e| AnimeError::new(CATEGORY_MALFORMED_RESPONSE, e.to_string(), false))?;
        Ok(heights.into_iter().map(|h| h as u32).collect())
    }

    pub async fn stream(
        &self,
        provider_id: &str,
        episode_url: &str,
        quality: &str,
    ) -> Result<StreamInfo, AnimeError> {
        if episode_url.trim().is_empty() {
            return Err(AnimeError::new(
                CATEGORY_UNSUPPORTED,
                "episode URL is empty",
                false,
            ));
        }
        if debug_logging_enabled() {
            eprintln!(
                "[mutsukitube-anime] stream provider={} quality={}",
                provider_id, quality
            );
        }
        let process = self.process_for(provider_id)?;
        let quality = if quality.trim().is_empty() {
            "auto"
        } else {
            quality
        };
        let response = process
            .request(
                "stream",
                json!({"episodeUrl": episode_url, "quality": quality}),
                Some(self.timeouts.stream_ms),
            )
            .await?;
        let data = response.data.ok_or_else(|| {
            AnimeError::new(
                CATEGORY_MALFORMED_RESPONSE,
                "stream response has no data",
                false,
            )
        })?;
        let stream: StreamInfo = serde_json::from_value(data)
            .map_err(|e| AnimeError::new(CATEGORY_MALFORMED_RESPONSE, e.to_string(), false))?;
        // The Go response wraps stream fields directly in data, not under a "stream" key.
        Ok(stream)
    }

    pub fn policy(&self) -> &FallbackPolicy {
        &self.policy
    }

    pub fn health(&self) -> &ProviderHealthTracker {
        &self.health
    }

    pub async fn search_auto(&self, query: &str) -> Result<SearchFallbackOutcome, AnimeError> {
        self.search_auto_with_cancel(query, Arc::new(AtomicBool::new(false)))
            .await
    }

    pub async fn search_auto_with_cancel(
        &self,
        query: &str,
        cancel: Arc<AtomicBool>,
    ) -> Result<SearchFallbackOutcome, AnimeError> {
        if query.trim().is_empty() {
            return Err(AnimeError::new(
                CATEGORY_UNSUPPORTED,
                "search query is empty",
                false,
            ));
        }

        let mut attempts = Vec::new();
        let mut cancelled = false;
        let providers = self.registry.lock().unwrap().ids_by_priority();
        let mut attempt_count = 0usize;

        for provider_id in providers {
            if cancel.load(Ordering::Relaxed) {
                cancelled = true;
                break;
            }
            if attempt_count >= self.policy.max_provider_attempts {
                break;
            }
            attempt_count += 1;

            let now = std::time::Instant::now();
            if debug_logging_enabled() {
                eprintln!(
                    "[mutsukitube-anime] search_auto provider={} query_len={} eligible={}",
                    provider_id,
                    query.len(),
                    self.health.is_available(&provider_id, now)
                );
            }
            if !self.health.is_available(&provider_id, now) {
                attempts.push(ProviderAttempt {
                    provider_id: provider_id.clone(),
                    reason: crate::fallback::FallbackReason::CoolingDown,
                    error: None,
                });
                continue;
            }

            match self.search(&provider_id, query).await {
                Ok(results) => {
                    if debug_logging_enabled() {
                        eprintln!(
                            "[mutsukitube-anime] search_auto provider={} results={}",
                            provider_id,
                            results.len()
                        );
                    }
                    if results.is_empty() {
                        self.health.record_empty(&provider_id);
                        attempts.push(ProviderAttempt {
                            provider_id: provider_id.clone(),
                            reason: self.policy.reason_for_empty_search(),
                            error: None,
                        });
                        if self.policy.decision_for_empty_search()
                            == crate::fallback::FallbackAction::Continue
                        {
                            continue;
                        }
                        break;
                    }

                    self.health.record_success(&provider_id);
                    attempts.push(ProviderAttempt {
                        provider_id: provider_id.clone(),
                        reason: crate::fallback::FallbackReason::Success,
                        error: None,
                    });
                    return Ok(SearchFallbackOutcome {
                        results,
                        provider_id: Some(provider_id),
                        attempts,
                        cancelled: false,
                    });
                }
                Err(error) => {
                    let action = self.policy.decision_for_error(&error);
                    let reason = self.policy.reason_for_error(&error);
                    if debug_logging_enabled() {
                        eprintln!(
                            "[mutsukitube-anime] provider={} error={} retryable={} fallback={:?}",
                            provider_id, error.category, error.retryable, action
                        );
                    }
                    self.health.record_failure(
                        &provider_id,
                        &error,
                        now,
                        self.policy.failure_threshold,
                        self.policy.base_cooldown,
                        self.policy.max_cooldown,
                    );
                    attempts.push(ProviderAttempt {
                        provider_id: provider_id.clone(),
                        reason,
                        error: Some(error),
                    });
                    if action == crate::fallback::FallbackAction::Stop {
                        break;
                    }
                }
            }
        }

        Ok(SearchFallbackOutcome {
            results: Vec::new(),
            provider_id: None,
            attempts,
            cancelled,
        })
    }

    pub async fn stream_with_fallback(
        &self,
        selected_anime: &Anime,
        selected_episode: &Episode,
        quality: &str,
        cancel: Arc<AtomicBool>,
    ) -> Result<StreamFallbackOutcome, AnimeError> {
        let mut attempts = Vec::new();
        let mut providers = vec![selected_anime.provider_id.clone()];
        for id in self.registry.lock().unwrap().ids_by_priority() {
            if id != selected_anime.provider_id {
                providers.push(id);
            }
        }

        let mut attempt_count = 0usize;
        for provider_id in providers {
            if cancel.load(Ordering::Relaxed) {
                return Err(AnimeError::new(
                    CATEGORY_CANCELLED,
                    "stream fallback cancelled",
                    true,
                ));
            }
            if attempt_count >= self.policy.max_provider_attempts {
                break;
            }
            attempt_count += 1;

            let now = std::time::Instant::now();
            if debug_logging_enabled() {
                eprintln!(
                    "[mutsukitube-anime] stream_with_fallback provider={} quality={} episode={} eligible={}",
                    provider_id,
                    quality,
                    selected_episode.number,
                    self.health.is_available(&provider_id, now)
                );
            }
            if !self.health.is_available(&provider_id, now) {
                attempts.push(ProviderAttempt {
                    provider_id: provider_id.clone(),
                    reason: crate::fallback::FallbackReason::CoolingDown,
                    error: None,
                });
                continue;
            }

            let (target_anime, target_episode) = if provider_id == selected_anime.provider_id {
                (selected_anime.clone(), selected_episode.clone())
            } else {
                let search_results = match self.search(&provider_id, &selected_anime.title).await {
                    Ok(results) => results,
                    Err(error) => {
                        let reason = self.policy.reason_for_error(&error);
                        self.health.record_failure(
                            &provider_id,
                            &error,
                            now,
                            self.policy.failure_threshold,
                            self.policy.base_cooldown,
                            self.policy.max_cooldown,
                        );
                        attempts.push(ProviderAttempt {
                            provider_id: provider_id.clone(),
                            reason,
                            error: Some(error),
                        });
                        continue;
                    }
                };

                let (matched_anime, confidence) =
                    find_matching_anime(selected_anime, &search_results);
                if confidence != MatchConfidence::Exact && confidence != MatchConfidence::Alias {
                    if debug_logging_enabled() {
                        eprintln!(
                            "[mutsukitube-anime] stream_with_fallback provider={} identity_match={:?}",
                            provider_id, confidence
                        );
                    }
                    attempts.push(ProviderAttempt {
                        provider_id: provider_id.clone(),
                        reason: if confidence == MatchConfidence::Ambiguous {
                            crate::fallback::FallbackReason::AmbiguousMatch
                        } else {
                            crate::fallback::FallbackReason::IdentityMismatch
                        },
                        error: None,
                    });
                    continue;
                }
                let matched_anime = match matched_anime {
                    Some(anime) => anime,
                    None => {
                        attempts.push(ProviderAttempt {
                            provider_id: provider_id.clone(),
                            reason: crate::fallback::FallbackReason::IdentityMismatch,
                            error: None,
                        });
                        continue;
                    }
                };

                let episodes = match self.episodes(&provider_id, &matched_anime.url).await {
                    Ok(episodes) => episodes,
                    Err(error) => {
                        let reason = self.policy.reason_for_error(&error);
                        self.health.record_failure(
                            &provider_id,
                            &error,
                            now,
                            self.policy.failure_threshold,
                            self.policy.base_cooldown,
                            self.policy.max_cooldown,
                        );
                        attempts.push(ProviderAttempt {
                            provider_id: provider_id.clone(),
                            reason,
                            error: Some(error),
                        });
                        continue;
                    }
                };

                match find_matching_episode(selected_episode, &episodes) {
                    Some(episode) => (matched_anime, episode),
                    None => {
                        if debug_logging_enabled() {
                            eprintln!(
                                "[mutsukitube-anime] stream_with_fallback provider={} episode_mismatch selected_episode={}",
                                provider_id, selected_episode.number
                            );
                        }
                        attempts.push(ProviderAttempt {
                            provider_id: provider_id.clone(),
                            reason: crate::fallback::FallbackReason::EpisodeMismatch,
                            error: None,
                        });
                        continue;
                    }
                }
            };

            match self
                .stream(&provider_id, &target_episode.url, quality)
                .await
            {
                Ok(stream) => {
                    self.health.record_success(&provider_id);
                    let cache_key = format!(
                        "{}:{}->{}",
                        selected_anime.provider_id, selected_anime.url, provider_id
                    );
                    self.identity_cache.lock().unwrap().insert(
                        cache_key,
                        VerifiedAnimeMapping {
                            source_provider: selected_anime.provider_id.clone(),
                            source_url: selected_anime.url.clone(),
                            target_provider: provider_id.clone(),
                            target_url: target_anime.url.clone(),
                            identity: AnimeIdentity::from_anime(&target_anime),
                        },
                        std::time::Instant::now(),
                    );
                    if debug_logging_enabled() {
                        eprintln!(
                            "[mutsukitube-anime] stream_with_fallback provider={} success height={:?} source={:?}",
                            provider_id, stream.height, stream.source
                        );
                    }
                    attempts.push(ProviderAttempt {
                        provider_id: provider_id.clone(),
                        reason: crate::fallback::FallbackReason::Success,
                        error: None,
                    });
                    return Ok(StreamFallbackOutcome {
                        stream,
                        provider_id,
                        anime_url: target_anime.url,
                        episode_url: target_episode.url,
                        attempts,
                    });
                }
                Err(error) => {
                    let action = self.policy.decision_for_error(&error);
                    let reason = self.policy.reason_for_error(&error);
                    if debug_logging_enabled() {
                        eprintln!(
                            "[mutsukitube-anime] provider={} error={} retryable={} fallback={:?}",
                            provider_id, error.category, error.retryable, action
                        );
                    }
                    self.health.record_failure(
                        &provider_id,
                        &error,
                        now,
                        self.policy.failure_threshold,
                        self.policy.base_cooldown,
                        self.policy.max_cooldown,
                    );
                    attempts.push(ProviderAttempt {
                        provider_id: provider_id.clone(),
                        reason,
                        error: Some(error),
                    });
                    if action == crate::fallback::FallbackAction::Stop {
                        break;
                    }
                }
            }
        }

        if debug_logging_enabled() {
            eprintln!(
                "[mutsukitube-anime] stream_with_fallback exhausted attempts={}",
                attempts.len()
            );
        }
        Err(AnimeError::new(
            CATEGORY_NO_PLAYABLE_MIRROR,
            "all eligible providers exhausted",
            false,
        ))
    }

    pub async fn shutdown_all(&self) {
        let processes: Vec<std::sync::Arc<ProviderProcess>> = {
            let mut guard = self.processes.lock().unwrap();
            guard.drain().map(|(_, p)| p).collect()
        };
        for process in processes {
            process.shutdown().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ProviderRegistry;

    #[tokio::test]
    async fn manager_rejects_unknown_provider() {
        let registry = ProviderRegistry::new(vec![]);
        let manager = AnimeManager::new(registry);
        let err = manager.search("missing", "naruto").await.unwrap_err();
        assert_eq!(err.category, CATEGORY_PROVIDER_NOT_FOUND);
    }
}
