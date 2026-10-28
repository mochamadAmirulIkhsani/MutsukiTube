use mutsukitube_youtube::{YtDlpProvider, YoutubeProvider};

#[tokio::test]
#[ignore = "requires live YouTube network access"]
async fn test_trending_live() {
    let provider = YoutubeProvider::new();
    let res = provider.trending(None, Some("ID")).await;
    match res {
        Ok(videos) => {
            println!("Got {} trending videos!", videos.len());
            for v in videos.iter().take(5) {
                println!("- {} [{}] ({})", v.title, v.id, v.channel);
            }
            assert!(!videos.is_empty());
        }
        Err(e) => panic!("Trending failed: {e:?}"),
    }
}

#[tokio::test]
#[ignore = "requires live YouTube network access"]
async fn test_ytdlp_trending_live() {
    let provider = YtDlpProvider::new();
    let res = provider.trending(None, Some("ID")).await;
    match res {
        Ok(videos) => {
            println!("yt-dlp got {} trending videos!", videos.len());
            assert!(!videos.is_empty());
        }
        Err(e) => panic!("yt-dlp trending failed: {e:?}"),
    }
}
