//! Deterministic, identity-safe episode ordering.
//!
//! Providers scrape listing pages that frequently present episodes newest
//! first (12 → 1). The UI must show Episode 1 → Episode N, so the host sorts
//! once, right after the IPC response is deserialised, before any consumer sees
//! the list.
//!
//! Ordering only affects *display order*. The exact same `Episode` values are
//! reused (moved, not rebuilt), so the structured `number` and — critically —
//! the `url` stay attached to their episode. Playback resolution, resume state
//! and favorites are all keyed by episode number/URL, never by display index.

use crate::models::Episode;

/// Sort `episodes` into ascending natural numeric order (stable).
///
/// * Numbered episodes (positive structured `number`, or a number confidently
///   parsed from the title as a fallback) come first, ascending.
/// * Entries that cannot be confidently ordered (movies, specials, OVAs,
///   unnumbered rows) keep their original relative order and are placed after
///   the numbered episodes. They are never silently assigned a number.
pub fn order_episodes(episodes: &mut [Episode]) {
    episodes.sort_by(|a, b| match (order_key(a), order_key(b)) {
        (Some(left), Some(right)) => left
            .partial_cmp(&right)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        // Stable sort keeps original order for equal / unordered entries.
        (None, None) => std::cmp::Ordering::Equal,
    });
}

/// Ascending order key for an episode, or `None` when the number is unknown.
fn order_key(episode: &Episode) -> Option<f64> {
    if episode.number > 0 {
        // Structured number from the provider is authoritative.
        Some(episode.number as f64)
    } else {
        parse_episode_number(&episode.title)
    }
}

/// Extract an episode number from a display title, conservatively.
///
/// Only recognises explicit episode markers ("Episode 12", "Ep. 12",
/// "EP 12.5") so season numbers, release years ("2023") and quality
/// resolutions ("1080p") are never mistaken for episode numbers.
pub fn parse_episode_number(title: &str) -> Option<f64> {
    let lower = title.to_ascii_lowercase();
    let bytes = lower.as_bytes();

    for marker in ["episode", "ep"] {
        let marker = marker.as_bytes();
        let mut index = 0;

        while index + marker.len() <= bytes.len() {
            if &bytes[index..index + marker.len()] == marker {
                let starts_word = index == 0 || !bytes[index - 1].is_ascii_alphanumeric();
                let ends_word = index + marker.len() >= bytes.len()
                    || !bytes[index + marker.len()].is_ascii_alphabetic();

                if starts_word && ends_word {
                    if let Some(number) = parse_number_after(&bytes[index + marker.len()..]) {
                        return Some(number);
                    }
                }
            }

            index += 1;
        }
    }

    None
}

/// Parse a leading (optionally decimal) number, skipping common separators.
fn parse_number_after(rest: &[u8]) -> Option<f64> {
    let mut index = 0;

    while index < rest.len() && matches!(rest[index], b' ' | b'-' | b'_' | b':' | b'.' | b'#') {
        index += 1;
    }

    let integer_start = index;

    while index < rest.len() && rest[index].is_ascii_digit() {
        index += 1;
    }

    if index == integer_start {
        return None;
    }

    let mut value: f64 = std::str::from_utf8(&rest[integer_start..index])
        .ok()?
        .parse()
        .ok()?;

    // Optional decimal part, e.g. "12.5".
    if index < rest.len() && rest[index] == b'.' {
        let mut fraction_end = index + 1;

        while fraction_end < rest.len() && rest[fraction_end].is_ascii_digit() {
            fraction_end += 1;
        }

        if fraction_end > index + 1 {
            if let Ok(fraction) = std::str::from_utf8(&rest[index..fraction_end])
                .ok()?
                .parse::<f64>()
            {
                value += fraction;
            }
        }
    }

    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn episode(number: i32, url: &str) -> Episode {
        Episode {
            provider_id: "fake".into(),
            number,
            title: format!("Episode {number}"),
            url: url.into(),
        }
    }

    #[test]
    fn descending_episodes_are_sorted_ascending() {
        let mut episodes = vec![
            episode(12, "https://x/12"),
            episode(11, "https://x/11"),
            episode(10, "https://x/10"),
            episode(2, "https://x/2"),
            episode(1, "https://x/1"),
        ];

        order_episodes(&mut episodes);

        let numbers: Vec<i32> = episodes.iter().map(|e| e.number).collect();
        assert_eq!(numbers, vec![1, 2, 10, 11, 12]);
    }

    #[test]
    fn ordering_preserves_each_episode_identity() {
        let mut episodes = vec![
            episode(3, "https://x/ep-three"),
            episode(1, "https://x/ep-one"),
            episode(2, "https://x/ep-two"),
        ];

        order_episodes(&mut episodes);

        // Each number still maps to its original URL: sorting must not swap
        // stream targets or resume mappings.
        assert_eq!(episodes[0].number, 1);
        assert_eq!(episodes[0].url, "https://x/ep-one");
        assert_eq!(episodes[1].number, 2);
        assert_eq!(episodes[1].url, "https://x/ep-two");
        assert_eq!(episodes[2].number, 3);
        assert_eq!(episodes[2].url, "https://x/ep-three");
    }

    #[test]
    fn lexical_order_is_not_used() {
        let mut episodes = vec![
            episode(100, "https://x/100"),
            episode(2, "https://x/2"),
            episode(10, "https://x/10"),
            episode(1, "https://x/1"),
        ];

        order_episodes(&mut episodes);

        let numbers: Vec<i32> = episodes.iter().map(|e| e.number).collect();
        assert_eq!(numbers, vec![1, 2, 10, 100]);
    }

    #[test]
    fn unnumbered_entries_keep_stable_order_at_the_end() {
        let mut episodes = vec![
            Episode {
                provider_id: "fake".into(),
                number: 2,
                title: "Episode 2".into(),
                url: "https://x/2".into(),
            },
            Episode {
                provider_id: "fake".into(),
                number: 0,
                title: "OVA Special".into(),
                url: "https://x/ova".into(),
            },
            Episode {
                provider_id: "fake".into(),
                number: 1,
                title: "Episode 1".into(),
                url: "https://x/1".into(),
            },
            Episode {
                provider_id: "fake".into(),
                number: 0,
                title: "Recap Movie".into(),
                url: "https://x/movie".into(),
            },
        ];

        order_episodes(&mut episodes);

        let urls: Vec<&str> = episodes.iter().map(|e| e.url.as_str()).collect();

        assert_eq!(
            urls,
            vec![
                "https://x/1",
                "https://x/2",
                "https://x/ova",
                "https://x/movie"
            ]
        );
    }

    #[test]
    fn title_fallback_orders_unnumbered_episodes() {
        let mut episodes = vec![
            Episode {
                provider_id: "fake".into(),
                number: 0,
                title: "Episode 12".into(),
                url: "https://x/12".into(),
            },
            Episode {
                provider_id: "fake".into(),
                number: 0,
                title: "Episode 2".into(),
                url: "https://x/2".into(),
            },
            Episode {
                provider_id: "fake".into(),
                number: 0,
                title: "Episode 1".into(),
                url: "https://x/1".into(),
            },
        ];

        order_episodes(&mut episodes);

        let urls: Vec<&str> = episodes.iter().map(|e| e.url.as_str()).collect();
        assert_eq!(urls, vec!["https://x/1", "https://x/2", "https://x/12"]);
    }

    #[test]
    fn parses_common_episode_markers() {
        assert_eq!(parse_episode_number("Episode 1"), Some(1.0));
        assert_eq!(parse_episode_number("Episode 12"), Some(12.0));
        assert_eq!(parse_episode_number("Episode 12.5"), Some(12.5));
        assert_eq!(parse_episode_number("Episode 100"), Some(100.0));
        assert_eq!(parse_episode_number("Ep. 7"), Some(7.0));
        assert_eq!(parse_episode_number("EP 7 - The Fight"), Some(7.0));
        assert_eq!(
            parse_episode_number("Naruto Episode 220 [END]"),
            Some(220.0)
        );
    }

    #[test]
    fn does_not_misread_years_resolutions_or_specials() {
        assert_eq!(parse_episode_number("2023"), None);
        assert_eq!(parse_episode_number("1080p"), None);
        assert_eq!(parse_episode_number("Naruto Shippuden"), None);
        assert_eq!(parse_episode_number("OVA"), None);
        assert_eq!(parse_episode_number("Opening"), None);
    }

    #[test]
    fn already_sorted_lists_are_unchanged() {
        let mut episodes = vec![
            episode(1, "https://x/1"),
            episode(2, "https://x/2"),
            episode(3, "https://x/3"),
        ];

        let original: Vec<String> = episodes.iter().map(|e| e.url.clone()).collect();

        order_episodes(&mut episodes);

        let after: Vec<String> = episodes.iter().map(|e| e.url.clone()).collect();
        assert_eq!(original, after);
    }
}
