use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

fn cache_dir() -> Option<PathBuf> {
    let mut path = dirs::cache_dir()?;

    path.push("MutsukiTube");
    path.push("thumbnails");

    Some(path)
}

fn cache_file_name(url: &str) -> String {
    let mut hasher = Sha256::new();

    hasher.update(url.as_bytes());

    let hash = hasher.finalize();

    format!("{}.img", hex::encode(hash))
}

fn cache_path(url: &str) -> Option<PathBuf> {
    let mut dir = cache_dir()?;

    dir.push(cache_file_name(url));

    Some(dir)
}

async fn read_cache(path: &Path) -> Option<Vec<u8>> {
    tokio::fs::read(path).await.ok()
}

async fn save_cache(path: &Path, data: &[u8]) {
    let Some(parent) = path.parent() else {
        return;
    };

    if tokio::fs::create_dir_all(parent).await.is_err() {
        return;
    }

    let _ = tokio::fs::write(path, data).await;
}

pub async fn download_thumbnail(url: &str) -> Option<Vec<u8>> {
    if url.is_empty() {
        return None;
    }

    let cache_path = cache_path(url);

    if let Some(path) = cache_path.as_ref() {
        if let Some(data) = read_cache(path).await {
            return Some(data);
        }
    }

    let response = reqwest::get(url).await.ok()?;

    if !response.status().is_success() {
        return None;
    }

    let bytes = response.bytes().await.ok()?;

    let data = bytes.to_vec();

    if let Some(path) = cache_path {
        save_cache(&path, &data).await;
    }

    Some(data)
}
