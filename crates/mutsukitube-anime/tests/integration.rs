use mutsukitube_anime::{AnimeManager, ProcessTimeouts, ProviderConfig, ProviderRegistry};
use std::path::PathBuf;

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
