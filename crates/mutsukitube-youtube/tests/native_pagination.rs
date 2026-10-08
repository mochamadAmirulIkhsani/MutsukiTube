use mutsukitube_youtube::NativeYoutubeProvider;

#[tokio::test]
#[ignore = "requires live YouTube network access"]
async fn test_native_search_pagination() {
    let provider = NativeYoutubeProvider::new();

    let first_page = provider
        .search_page("rust programming", None)
        .await
        .expect("first page failed");

    println!("First page: {} videos", first_page.videos.len());

    assert!(!first_page.videos.is_empty());

    if let Some(token) = first_page.next_page_token {
        let second_page = provider
            .search_page("rust programming", Some(&token))
            .await
            .expect("second page failed");

        println!("Second page: {} videos", second_page.videos.len());

        assert!(!second_page.videos.is_empty());
    } else {
        println!("No continuation token returned");
    }
}
