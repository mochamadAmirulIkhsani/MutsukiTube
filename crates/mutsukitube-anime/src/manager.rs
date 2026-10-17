use crate::errors::*;
use crate::models::{Anime, Episode, PROTOCOL_VERSION, PluginInfo, StreamInfo};
use crate::process::{ProcessTimeouts, ProviderProcess};
use crate::registry::ProviderRegistry;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex as StdMutex;

pub struct AnimeManager {
    registry: ProviderRegistry,
    processes: StdMutex<HashMap<String, std::sync::Arc<ProviderProcess>>>,
    timeouts: ProcessTimeouts,
}

impl AnimeManager {
    pub fn new(registry: ProviderRegistry) -> Self {
        Self {
            registry,
            processes: StdMutex::new(HashMap::new()),
            timeouts: ProcessTimeouts::default(),
        }
    }

    pub fn with_timeouts(registry: ProviderRegistry, timeouts: ProcessTimeouts) -> Self {
        Self {
            registry,
            processes: StdMutex::new(HashMap::new()),
            timeouts,
        }
    }

    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry
    }

    pub fn provider_ids(&self) -> Vec<String> {
        self.registry.ids_by_priority()
    }

    fn process_for(
        &self,
        provider_id: &str,
    ) -> Result<std::sync::Arc<ProviderProcess>, AnimeError> {
        let config = self.registry.get(provider_id).ok_or_else(|| {
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
