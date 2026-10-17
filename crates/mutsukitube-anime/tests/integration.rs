use mutsukitube_anime::{
    Anime, AnimeManager, Episode, FallbackPolicy, ProcessTimeouts, ProviderConfig,
    ProviderRegistry, SearchFallbackOutcome, StreamFallbackOutcome,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn fake_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake_anime_provider"))
}

fn registry_for_mode(mode: &str) -> ProviderRegistry {
    let config = ProviderConfig {
        id: "fake".into(),
        display_name: "Fake".into(),
        executable_path: fake_bin(),
        executable_args: vec![mode.to_string()],
        enabled: true,
        priority: 1,
        expected_protocol: mutsukitube_anime::PROTOCOL_VERSION,
    };
    ProviderRegistry::new(vec![config])
}

#[tokio::test(flavor = "multi_thread")]
async fn info_roundtrip() {
    let registry = registry_for_mode("ok");
    let manager = AnimeManager::new(registry);
    let info = manager.info("fake").await.unwrap();
    assert_eq!(info.id, "fake");
    assert_eq!(info.protocol_version, 1);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_returns_results() {
    let registry = registry_for_mode("ok");
    let manager = AnimeManager::new(registry);
    let results = manager.search("fake", "Naruto").await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Anime Naruto");
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn episodes_heights_stream() {
    let registry = registry_for_mode("ok");
    let manager = AnimeManager::new(registry);
    let episodes = manager
        .episodes("fake", "https://fake.example/anime/1")
        .await
        .unwrap();
    assert_eq!(episodes.len(), 1);
    let heights = manager
        .heights("fake", "https://fake.example/episode/1")
        .await
        .unwrap();
    assert_eq!(heights, vec![360, 720]);
    let stream = manager
        .stream("fake", "https://fake.example/episode/1", "720p")
        .await
        .unwrap();
    assert_eq!(stream.url, "https://example.com/video.mp4");
    assert_eq!(stream.height, Some(720));
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_search_is_success() {
    let registry = registry_for_mode("ok");
    let manager = AnimeManager::new(registry);
    let results = manager.search("fake", "empty").await.unwrap();
    assert!(results.is_empty());
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_executable_fails() {
    let config = ProviderConfig {
        id: "fake".into(),
        display_name: "Fake".into(),
        executable_path: PathBuf::from("Z:\\definitely\\missing.exe"),
        executable_args: Vec::new(),
        enabled: true,
        priority: 1,
        expected_protocol: mutsukitube_anime::PROTOCOL_VERSION,
    };
    let registry = ProviderRegistry::new(vec![config]);
    let manager = AnimeManager::new(registry);
    let err = manager.info("fake").await.unwrap_err();
    assert_eq!(err.category, mutsukitube_anime::CATEGORY_INVALID_EXECUTABLE);
}

#[tokio::test(flavor = "multi_thread")]
async fn crash_fails_request() {
    let registry = registry_for_mode("crash");
    let manager = AnimeManager::new(registry);
    let err = manager.info("fake").await.unwrap_err();
    assert!(
        err.category == mutsukitube_anime::CATEGORY_PROCESS_SPAWN_FAILED
            || err.category == mutsukitube_anime::CATEGORY_PROCESS_EXITED
    );
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn timeout_kills_provider() {
    let timeouts = ProcessTimeouts {
        info_ms: 200,
        search_ms: 200,
        episodes_ms: 200,
        heights_ms: 200,
        stream_ms: 200,
        startup_ms: 200,
        shutdown_ms: 200,
        max_pending: 64,
    };
    let registry = registry_for_mode("timeout");
    let manager = AnimeManager::with_timeouts(registry, timeouts);
    let err = manager.info("fake").await.unwrap_err();
    assert_eq!(err.category, mutsukitube_anime::CATEGORY_TIMEOUT);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_response_does_not_crash_host() {
    let registry = registry_for_mode("malformed");
    let manager = AnimeManager::new(registry);
    let info = manager.info("fake").await.unwrap();
    assert_eq!(info.id, "fake");
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn oversized_response_fails_safely() {
    let registry = registry_for_mode("oversized");
    let manager = AnimeManager::new(registry);
    let err = manager.info("fake").await.unwrap_err();
    assert_eq!(err.category, mutsukitube_anime::CATEGORY_OVERSIZED_RESPONSE);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_protocol_version_fails() {
    let registry = registry_for_mode("bad-protocol");
    let manager = AnimeManager::new(registry);
    let err = manager.info("fake").await.unwrap_err();
    assert_eq!(
        err.category,
        mutsukitube_anime::CATEGORY_PROTOCOL_VERSION_MISMATCH
    );
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_request_id_does_not_crash() {
    let timeouts = ProcessTimeouts {
        info_ms: 300,
        search_ms: 300,
        episodes_ms: 300,
        heights_ms: 300,
        stream_ms: 300,
        startup_ms: 300,
        shutdown_ms: 300,
        max_pending: 64,
    };
    let registry = registry_for_mode("unknown-id");
    let manager = AnimeManager::with_timeouts(registry, timeouts);
    let err = manager.info("fake").await.unwrap_err();
    assert_eq!(err.category, mutsukitube_anime::CATEGORY_TIMEOUT);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_requests_out_of_order() {
    let registry = registry_for_mode("reorder");
    let manager = std::sync::Arc::new(AnimeManager::new(registry));
    let m1 = std::sync::Arc::clone(&manager);
    let m2 = std::sync::Arc::clone(&manager);
    let t1 = tokio::spawn(async move { m1.search("fake", "one").await });
    let t2 = tokio::spawn(async move { m2.search("fake", "two").await });
    let r1 = t1.await.unwrap();
    let r2 = t2.await.unwrap();
    assert!(r1.is_ok());
    assert!(r2.is_ok());
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn repeated_start_stop_cycles() {
    for _ in 0..3 {
        let registry = registry_for_mode("ok");
        let manager = AnimeManager::new(registry);
        let info = manager.info("fake").await.unwrap();
        assert_eq!(info.id, "fake");
        manager.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn disabled_provider_rejected() {
    let config = ProviderConfig {
        id: "fake".into(),
        display_name: "Fake".into(),
        executable_path: fake_bin(),
        executable_args: Vec::new(),
        enabled: false,
        priority: 1,
        expected_protocol: mutsukitube_anime::PROTOCOL_VERSION,
    };
    let registry = ProviderRegistry::new(vec![config]);
    let manager = AnimeManager::new(registry);
    let err = manager.info("fake").await.unwrap_err();
    assert_eq!(err.category, mutsukitube_anime::CATEGORY_DISABLED_PROVIDER);
}

#[tokio::test(flavor = "multi_thread")]
async fn real_stream_smoke() {
    let base = if let Ok(dir) = std::env::var("MUTSUKITUBE_ANIME_PLUGIN_DIR") {
        std::path::PathBuf::from(dir)
    } else {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../mutsukitube-anime-plugins/dist/v0.1.0")
    };
    for provider in ["ylnime", "astronime", "moenime"] {
        let binary_name = if cfg!(windows) {
            format!("{}-windows-amd64.exe", provider)
        } else {
            format!("{}-linux-amd64", provider)
        };
        let binary = base.join(binary_name);
        if !binary.is_file() {
            eprintln!("skip {} missing", binary.display());
            continue;
        }
        let config = mutsukitube_anime::ProviderConfig {
            id: provider.into(),
            display_name: provider.into(),
            executable_path: binary,
            executable_args: Vec::new(),
            enabled: true,
            priority: 1,
            expected_protocol: mutsukitube_anime::PROTOCOL_VERSION,
        };
        let registry = mutsukitube_anime::ProviderRegistry::new(vec![config]);
        let manager = mutsukitube_anime::AnimeManager::new(registry);
        println!("PROVIDER={}", provider);
        let search = manager.search(provider, "naruto").await;
        println!("SEARCH={:?}", search.as_ref().map(|v| v.len()));
        let anime = match search {
            Ok(mut results) if !results.is_empty() => results.remove(0),
            Ok(_) => {
                println!("no search results");
                manager.shutdown_all().await;
                continue;
            }
            Err(e) => {
                println!("search error: {}", e);
                manager.shutdown_all().await;
                continue;
            }
        };
        println!("ANIME={} {}", anime.title, anime.url);
        let episodes = match manager.episodes(provider, &anime.url).await {
            Ok(v) => v,
            Err(e) => {
                println!("episodes error: {}", e);
                manager.shutdown_all().await;
                continue;
            }
        };
        println!("EPISODES={}", episodes.len());
        let Some(episode) = episodes.first().cloned() else {
            manager.shutdown_all().await;
            continue;
        };
        println!("EPISODE={} {}", episode.number, episode.url);
        let heights = manager
            .heights(provider, &episode.url)
            .await
            .unwrap_or_default();
        println!("HEIGHTS={:?}", heights);
        let stream = manager.stream(provider, &episode.url, "auto").await;
        match stream {
            Ok(s) => println!(
                "STREAM_OK height={:?} source={:?} url_host={}",
                s.height,
                s.source,
                s.url.split('/').nth(2).unwrap_or(&s.url)
            ),
            Err(e) => println!("STREAM_ERROR={}", e),
        }
        manager.shutdown_all().await;
    }
}

fn fallback_registry(providers: Vec<(&str, &str)>) -> ProviderRegistry {
    let mut configs = Vec::new();
    for (id, mode) in providers {
        configs.push(ProviderConfig {
            id: id.to_string(),
            display_name: id.to_string(),
            executable_path: fake_bin(),
            executable_args: vec![mode.to_string()],
            enabled: true,
            priority: configs.len() as u32 + 1,
            expected_protocol: mutsukitube_anime::PROTOCOL_VERSION,
        });
    }
    ProviderRegistry::new(configs)
}

#[tokio::test(flavor = "multi_thread")]
async fn search_auto_primary_success() {
    let registry = fallback_registry(vec![("primary", "ok"), ("backup", "network-error")]);
    let manager = AnimeManager::new(registry);
    let outcome = manager.search_auto("Naruto").await.unwrap();
    assert_eq!(outcome.provider_id.as_deref(), Some("primary"));
    assert_eq!(outcome.results.len(), 1);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_auto_empty_falls_back() {
    let registry = fallback_registry(vec![("primary", "ok"), ("backup", "ok")]);
    // Make primary return empty by using query "empty".
    let manager = AnimeManager::new(registry);
    let outcome = manager.search_auto("empty").await.unwrap();
    // Both return empty; no provider selected.
    assert!(outcome.provider_id.is_none());
    assert!(outcome.results.is_empty());
    assert!(outcome.attempts.len() >= 2);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_auto_network_error_falls_back() {
    let registry = fallback_registry(vec![("primary", "network-error"), ("backup", "ok")]);
    let manager = AnimeManager::new(registry);
    let outcome = manager.search_auto("Naruto").await.unwrap();
    assert_eq!(outcome.provider_id.as_deref(), Some("backup"));
    assert_eq!(outcome.results.len(), 1);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_auto_timeout_falls_back() {
    let timeouts = ProcessTimeouts {
        info_ms: 100,
        search_ms: 200,
        episodes_ms: 200,
        heights_ms: 200,
        stream_ms: 200,
        startup_ms: 100,
        shutdown_ms: 100,
        max_pending: 64,
    };
    let registry = fallback_registry(vec![("primary", "timeout"), ("backup", "ok")]);
    let policy = FallbackPolicy::default();
    let manager = AnimeManager::with_timeouts_and_policy(registry, timeouts, policy);
    let outcome = manager.search_auto("Naruto").await.unwrap();
    assert_eq!(outcome.provider_id.as_deref(), Some("backup"));
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_auto_all_unavailable() {
    let registry = fallback_registry(vec![
        ("primary", "network-error"),
        ("backup", "provider-unavailable"),
    ]);
    let manager = AnimeManager::new(registry);
    let outcome = manager.search_auto("Naruto").await.unwrap();
    assert!(outcome.provider_id.is_none());
    assert!(outcome.results.is_empty());
    assert_eq!(outcome.attempts.len(), 2);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_auto_disabled_skipped() {
    let mut registry = fallback_registry(vec![("primary", "ok"), ("backup", "ok")]);
    // Disable primary by rebuilding config.
    let configs = vec![
        ProviderConfig {
            id: "primary".into(),
            display_name: "primary".into(),
            executable_path: fake_bin(),
            executable_args: vec!["ok".into()],
            enabled: false,
            priority: 1,
            expected_protocol: mutsukitube_anime::PROTOCOL_VERSION,
        },
        ProviderConfig {
            id: "backup".into(),
            display_name: "backup".into(),
            executable_path: fake_bin(),
            executable_args: vec!["ok".into()],
            enabled: true,
            priority: 2,
            expected_protocol: mutsukitube_anime::PROTOCOL_VERSION,
        },
    ];
    registry = ProviderRegistry::new(configs);
    let manager = AnimeManager::new(registry);
    let outcome = manager.search_auto("Naruto").await.unwrap();
    assert_eq!(outcome.provider_id.as_deref(), Some("backup"));
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_auto_cancelled() {
    let registry = fallback_registry(vec![("primary", "timeout"), ("backup", "ok")]);
    let manager = AnimeManager::new(registry);
    let cancel = Arc::new(AtomicBool::new(true));
    let outcome = manager
        .search_auto_with_cancel("Naruto", cancel)
        .await
        .unwrap();
    assert!(outcome.cancelled);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_auto_multiple_failures_then_success() {
    let registry = fallback_registry(vec![
        ("p1", "network-error"),
        ("p2", "parser-changed"),
        ("p3", "ok"),
    ]);
    let manager = AnimeManager::new(registry);
    let outcome = manager.search_auto("Naruto").await.unwrap();
    assert_eq!(outcome.provider_id.as_deref(), Some("p3"));
    assert_eq!(outcome.attempts.len(), 3);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn stream_fallback_primary_success() {
    let registry = fallback_registry(vec![("primary", "ok"), ("backup", "echo-title")]);
    let manager = AnimeManager::new(registry);
    let selected_anime = Anime {
        provider_id: "primary".into(),
        title: "Naruto".into(),
        url: "https://primary/anime/1".into(),
        thumbnail_url: None,
        alt_titles: vec![],
        year: Some(2002),
    };
    let selected_episode = Episode {
        provider_id: "primary".into(),
        number: 1,
        title: "Episode 1".into(),
        url: "https://primary/episode/1".into(),
    };
    let outcome = manager
        .stream_with_fallback(
            &selected_anime,
            &selected_episode,
            "720p",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
    assert_eq!(outcome.provider_id, "primary");
    assert_eq!(outcome.stream.url, "https://example.com/video.mp4");
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn stream_fallback_backup_success() {
    let registry = fallback_registry(vec![("primary", "no-mirror"), ("backup", "echo-title")]);
    let manager = AnimeManager::new(registry);
    let selected_anime = Anime {
        provider_id: "primary".into(),
        title: "Naruto".into(),
        url: "https://primary/anime/1".into(),
        thumbnail_url: None,
        alt_titles: vec![],
        year: Some(2002),
    };
    let selected_episode = Episode {
        provider_id: "primary".into(),
        number: 1,
        title: "Episode 1".into(),
        url: "https://primary/episode/1".into(),
    };
    let outcome = manager
        .stream_with_fallback(
            &selected_anime,
            &selected_episode,
            "720p",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
    assert_eq!(outcome.provider_id, "backup");
    assert_eq!(outcome.stream.url, "https://example.com/video.mp4");
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn stream_fallback_identity_mismatch() {
    let registry = fallback_registry(vec![("primary", "no-mirror"), ("backup", "ok")]);
    let manager = AnimeManager::new(registry);
    let selected_anime = Anime {
        provider_id: "primary".into(),
        title: "Naruto".into(),
        url: "https://primary/anime/1".into(),
        thumbnail_url: None,
        alt_titles: vec![],
        year: Some(2002),
    };
    let selected_episode = Episode {
        provider_id: "primary".into(),
        number: 1,
        title: "Episode 1".into(),
        url: "https://primary/episode/1".into(),
    };
    let err = manager
        .stream_with_fallback(
            &selected_anime,
            &selected_episode,
            "720p",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap_err();
    // All providers exhausted due to identity mismatch / no mirror.
    assert!(!err.category.is_empty());
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn stream_fallback_episode_mismatch() {
    let registry = fallback_registry(vec![("primary", "no-mirror"), ("backup", "echo-title")]);
    let manager = AnimeManager::new(registry);
    let selected_anime = Anime {
        provider_id: "primary".into(),
        title: "Naruto".into(),
        url: "https://primary/anime/1".into(),
        thumbnail_url: None,
        alt_titles: vec![],
        year: Some(2002),
    };
    let selected_episode = Episode {
        provider_id: "primary".into(),
        number: 2,
        title: "Episode 2".into(),
        url: "https://primary/episode/2".into(),
    };
    let err = manager
        .stream_with_fallback(
            &selected_anime,
            &selected_episode,
            "720p",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap_err();
    assert!(!err.category.is_empty());
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn stream_fallback_all_exhausted() {
    let registry = fallback_registry(vec![("primary", "no-mirror"), ("backup", "no-mirror")]);
    let manager = AnimeManager::new(registry);
    let selected_anime = Anime {
        provider_id: "primary".into(),
        title: "Naruto".into(),
        url: "https://primary/anime/1".into(),
        thumbnail_url: None,
        alt_titles: vec![],
        year: Some(2002),
    };
    let selected_episode = Episode {
        provider_id: "primary".into(),
        number: 1,
        title: "Episode 1".into(),
        url: "https://primary/episode/1".into(),
    };
    let err = manager
        .stream_with_fallback(
            &selected_anime,
            &selected_episode,
            "720p",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap_err();
    assert_eq!(err.category, mutsukitube_anime::CATEGORY_NO_PLAYABLE_MIRROR);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn stream_fallback_cancelled() {
    let registry = fallback_registry(vec![("primary", "no-mirror"), ("backup", "echo-title")]);
    let manager = AnimeManager::new(registry);
    let selected_anime = Anime {
        provider_id: "primary".into(),
        title: "Naruto".into(),
        url: "https://primary/anime/1".into(),
        thumbnail_url: None,
        alt_titles: vec![],
        year: Some(2002),
    };
    let selected_episode = Episode {
        provider_id: "primary".into(),
        number: 1,
        title: "Episode 1".into(),
        url: "https://primary/episode/1".into(),
    };
    let cancel = Arc::new(AtomicBool::new(true));
    let err = manager
        .stream_with_fallback(&selected_anime, &selected_episode, "720p", cancel)
        .await
        .unwrap_err();
    assert_eq!(err.category, mutsukitube_anime::CATEGORY_CANCELLED);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn installed_version_directory_is_discovered_and_handshakes() {
    let source_base = if let Ok(dir) = std::env::var("MUTSUKITUBE_ANIME_PLUGIN_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../mutsukitube-anime-plugins/dist/v0.1.0")
    };
    let binary_name = if cfg!(windows) {
        "ylnime-windows-amd64.exe"
    } else {
        "ylnime-linux-amd64"
    };
    let source_binary = source_base.join(binary_name);
    if !source_binary.is_file() {
        eprintln!(
            "skipping installed-version discovery test: {} not found",
            source_binary.display()
        );
        return;
    }

    // Simulate the updater's versioned install layout: <base>/<version>/<binary>.
    let temp = tempfile::tempdir().unwrap();
    let version_dir = temp.path().join("v0.1.0");
    std::fs::create_dir_all(&version_dir).unwrap();
    std::fs::copy(&source_binary, version_dir.join(binary_name)).unwrap();

    let registry = ProviderRegistry::with_standard(Some(version_dir));
    let discovered = registry.get("ylnime").unwrap();
    assert!(
        discovered.executable_path.is_file(),
        "installed binary was not discovered: {}",
        discovered.executable_path.display()
    );

    let manager = AnimeManager::new(registry);
    let info = manager.info("ylnime").await.unwrap();
    assert_eq!(info.id, "ylnime");
    assert_eq!(info.protocol_version, 1);

    manager.shutdown_all().await;
}

fn real_plugin_source_base() -> PathBuf {
    if let Ok(dir) = std::env::var("MUTSUKITUBE_ANIME_PLUGIN_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../mutsukitube-anime-plugins/dist/v0.1.0")
    }
}

fn real_plugin_binary_name() -> &'static str {
    if cfg!(windows) {
        "ylnime-windows-amd64.exe"
    } else {
        "ylnime-linux-amd64"
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn updater_install_then_manager_handshake() {
    use mutsukitube_plugin_updater as updater;

    let source_binary = real_plugin_source_base().join(real_plugin_binary_name());
    if !source_binary.is_file() {
        eprintln!(
            "skipping updater->manager chain test: {} not found",
            source_binary.display()
        );
        return;
    }

    let bytes = std::fs::read(&source_binary).unwrap();
    let artifact = updater::Artifact {
        path: real_plugin_binary_name().to_string(),
        provider: "ylnime".into(),
        os: if cfg!(windows) { "windows" } else { "linux" }.into(),
        arch: "amd64".into(),
        sha256: updater::sha256_hex(&bytes),
        size: bytes.len() as i64,
        url: String::new(),
    };

    let temp = tempfile::tempdir().unwrap();
    let installed = updater::install_artifact(temp.path(), "v0.1.0", &artifact, &bytes).unwrap();
    assert!(installed.is_file());
    updater::activate_version(temp.path(), "v0.1.0").unwrap();
    assert_eq!(
        updater::active_version(temp.path()).as_deref(),
        Some("v0.1.0")
    );

    // Tampered bytes must be rejected without disturbing the active version.
    let mut tampered = bytes.clone();
    tampered[0] ^= 1;
    assert!(updater::install_artifact(temp.path(), "v0.2.0", &artifact, &tampered).is_err());
    assert_eq!(
        updater::active_version(temp.path()).as_deref(),
        Some("v0.1.0")
    );

    // AnimeManager must discover the installed versioned executable and the
    // Rust-Go IPC info handshake must succeed.
    let registry = ProviderRegistry::with_standard(Some(temp.path().join("v0.1.0")));
    let manager = AnimeManager::new(registry);
    let info = manager.info("ylnime").await.unwrap();
    assert_eq!(info.id, "ylnime");
    assert_eq!(info.protocol_version, 1);
    manager.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn real_go_provider_smoke() {
    let base = if let Ok(dir) = std::env::var("MUTSUKITUBE_ANIME_PLUGIN_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../mutsukitube-anime-plugins/dist/v0.1.0")
    };
    let binary_name = if cfg!(windows) {
        "ylnime-windows-amd64.exe"
    } else {
        "ylnime-linux-amd64"
    };
    let binary = base.join(binary_name);
    if !binary.is_file() {
        eprintln!(
            "skipping real Go smoke test: {} not found",
            binary.display()
        );
        return;
    }

    let config = ProviderConfig {
        id: "ylnime".into(),
        display_name: "YLnime".into(),
        executable_path: binary,
        executable_args: Vec::new(),
        enabled: true,
        priority: 1,
        expected_protocol: mutsukitube_anime::PROTOCOL_VERSION,
    };
    let registry = ProviderRegistry::new(vec![config]);
    let manager = AnimeManager::new(registry);

    let info = manager.info("ylnime").await.unwrap();
    assert_eq!(info.id, "ylnime");
    assert_eq!(info.protocol_version, 1);

    // External source availability is not guaranteed. Both success and a
    // structured provider error are acceptable; IPC failure is not.
    match manager.search("ylnime", "naruto").await {
        Ok(results) => {
            let _ = results;
        }
        Err(err) => {
            assert!(!err.category.is_empty());
        }
    }

    manager.shutdown_all().await;
}
