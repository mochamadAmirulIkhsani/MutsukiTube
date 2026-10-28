//! Verification tests for mutsukitube-anime prologue ordering.

use mutsukitube_anime::{Episode, order_episodes, parse_episode_number};

#[test]
fn verify_issue_12_prologue_episode_zero_sorted_before_episode_one() {
    let mut episodes = vec![
        Episode {
            provider_id: "fake".into(),
            number: 1,
            title: "Episode 1".into(),
            url: "https://x/1".into(),
        },
        Episode {
            provider_id: "fake".into(),
            number: 0,
            title: "Prologue".into(),
            url: "https://x/0".into(),
        },
    ];

    order_episodes(&mut episodes);

    let urls: Vec<&str> = episodes.iter().map(|e| e.url.as_str()).collect();
    assert_eq!(
        urls,
        vec!["https://x/0", "https://x/1"],
        "FIXED: Prologue is recognized as episode 0.0 and sorted before Episode 1"
    );
}

#[test]
fn verify_issue_13_episode_number_in_title_with_fractional_delimiter() {
    assert_eq!(parse_episode_number("Episode 12.5"), Some(12.5));
    assert_eq!(parse_episode_number("Ep. 01v2"), Some(1.0));
}
