use crate::models::{Anime, Episode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchConfidence {
    Exact,
    Alias,
    Ambiguous,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeIdentity {
    pub canonical_title: String,
    pub alt_titles: Vec<String>,
    pub year: Option<u32>,
    pub season: Option<u32>,
}

impl AnimeIdentity {
    pub fn from_anime(anime: &Anime) -> Self {
        Self {
            canonical_title: normalize_title(&anime.title),
            alt_titles: anime
                .alt_titles
                .iter()
                .map(|t| normalize_title(t))
                .collect(),
            year: anime.year,
            season: parse_season(&anime.title),
        }
    }

    pub fn matches(&self, other: &Self) -> MatchConfidence {
        if self.season != other.season {
            // If both parsed a season, they must agree. If one is missing,
            // keep the comparison conservative but do not reject solely on that.
            if self.season.is_some() && other.season.is_some() {
                return MatchConfidence::None;
            }
        }

        if self.year.is_some() && other.year.is_some() && self.year != other.year {
            return MatchConfidence::None;
        }

        if self.canonical_title == other.canonical_title && !self.canonical_title.is_empty() {
            return MatchConfidence::Exact;
        }

        if self.alt_titles.iter().any(|title| {
            !title.is_empty()
                && (title == &other.canonical_title || other.alt_titles.contains(title))
        }) || other
            .alt_titles
            .iter()
            .any(|title| !title.is_empty() && title == &self.canonical_title)
        {
            return MatchConfidence::Alias;
        }

        // Conservative: do not fuzzy-match unknown titles into an automatic match.
        MatchConfidence::Ambiguous
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAnimeMapping {
    pub source_provider: String,
    pub source_url: String,
    pub target_provider: String,
    pub target_url: String,
    pub identity: AnimeIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedEpisodeMapping {
    pub source_provider: String,
    pub source_episode_url: String,
    pub target_provider: String,
    pub target_episode_url: String,
    pub episode_number: Option<i32>,
}

pub fn episodes_match(source: &Episode, target: &Episode) -> bool {
    if source.provider_id == target.provider_id {
        // Same provider is handled by Go; Rust cross-provider mapping requires
        // a distinct provider.
    }

    let source_number = if source.number > 0 {
        Some(source.number)
    } else {
        None
    };
    let target_number = if target.number > 0 {
        Some(target.number)
    } else {
        None
    };

    if let (Some(a), Some(b)) = (source_number, target_number) {
        if a != b {
            return false;
        }
    }

    let source_title = normalize_title(&source.title);
    let target_title = normalize_title(&target.title);

    // Reject obvious OVA/special mismatches when one side is numbered and the
    // other is not.
    let source_special = is_special(&source.title);
    let target_special = is_special(&target.title);
    if source_special != target_special {
        return false;
    }

    if source_number.is_none() && target_number.is_none() {
        return !source_title.is_empty() && source_title == target_title;
    }

    true
}

pub fn find_matching_anime(
    selected: &Anime,
    candidates: &[Anime],
) -> (Option<Anime>, MatchConfidence) {
    let selected_identity = AnimeIdentity::from_anime(selected);
    let mut matches: Vec<(Anime, MatchConfidence)> = Vec::new();

    for candidate in candidates {
        let confidence = selected_identity.matches(&AnimeIdentity::from_anime(candidate));
        if confidence == MatchConfidence::Exact || confidence == MatchConfidence::Alias {
            matches.push((candidate.clone(), confidence));
        }
    }

    if matches.is_empty() {
        // Distinguish ambiguous from no match based on whether any candidate
        // had the same normalized title prefix.
        let any_same_prefix = candidates.iter().any(|c| {
            let a = normalize_title(&selected.title);
            let b = normalize_title(&c.title);
            a.split_whitespace().next() == b.split_whitespace().next() && !a.is_empty()
        });
        if any_same_prefix {
            return (None, MatchConfidence::Ambiguous);
        }
        return (None, MatchConfidence::None);
    }

    if matches.len() > 1 {
        return (None, MatchConfidence::Ambiguous);
    }

    let (anime, confidence) = matches.remove(0);
    (Some(anime), confidence)
}

pub fn find_matching_episode(selected: &Episode, candidates: &[Episode]) -> Option<Episode> {
    let mut matches: Vec<Episode> = candidates
        .iter()
        .filter(|candidate| episodes_match(selected, candidate))
        .cloned()
        .collect();

    if matches.len() == 1 {
        matches.pop()
    } else {
        None
    }
}

pub fn normalize_title(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(' ');
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_season(title: &str) -> Option<u32> {
    let lower = title.to_lowercase();
    for marker in ["season ", "saison ", "season. ", " s"] {
        if let Some(pos) = lower.find(marker) {
            let rest = &lower[pos + marker.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return digits.parse().ok();
            }
        }
    }
    // Patterns like "2nd Season" / "3rd Season".
    for token in lower.split_whitespace() {
        if token.ends_with("nd") || token.ends_with("rd") || token.ends_with("th") {
            let digits: String = token.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() && lower.contains("season") {
                return digits.parse().ok();
            }
        }
    }
    None
}

fn is_special(title: &str) -> bool {
    let lower = title.to_lowercase();
    lower.contains("ova") || lower.contains("special") || lower.contains("movie")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anime(title: &str, year: Option<u32>, alt: &[&str]) -> Anime {
        Anime {
            provider_id: "p".into(),
            title: title.into(),
            url: "u".into(),
            thumbnail_url: None,
            alt_titles: alt.iter().map(|s| s.to_string()).collect(),
            year,
        }
    }

    #[test]
    fn same_title_exact() {
        let a = AnimeIdentity::from_anime(&anime("Naruto", Some(2002), &[]));
        let b = AnimeIdentity::from_anime(&anime("Naruto", Some(2002), &[]));
        assert_eq!(a.matches(&b), MatchConfidence::Exact);
    }

    #[test]
    fn punctuation_normalizes() {
        let a = AnimeIdentity::from_anime(&anime("Shingeki no Kyojin", None, &[]));
        let b = AnimeIdentity::from_anime(&anime("Shingeki-no-Kyojin!", None, &[]));
        assert_eq!(a.matches(&b), MatchConfidence::Exact);
    }

    #[test]
    fn alias_matches() {
        let a = AnimeIdentity::from_anime(&anime("Shingeki no Kyojin", None, &["Attack on Titan"]));
        let b = AnimeIdentity::from_anime(&anime("Attack on Titan", None, &[]));
        assert_eq!(a.matches(&b), MatchConfidence::Alias);
    }

    #[test]
    fn wrong_year_rejects() {
        let a = AnimeIdentity::from_anime(&anime("Naruto", Some(2002), &[]));
        let b = AnimeIdentity::from_anime(&anime("Naruto", Some(2007), &[]));
        assert_eq!(a.matches(&b), MatchConfidence::None);
    }

    #[test]
    fn wrong_season_rejects() {
        let a = AnimeIdentity::from_anime(&anime("Naruto Shippuden Season 1", None, &[]));
        let b = AnimeIdentity::from_anime(&anime("Naruto Shippuden Season 2", None, &[]));
        assert_eq!(a.matches(&b), MatchConfidence::None);
    }

    #[test]
    fn unrelated_similar_is_ambiguous() {
        let a = AnimeIdentity::from_anime(&anime("Naruto", None, &[]));
        let b = AnimeIdentity::from_anime(&anime("Naruto SD", None, &[]));
        assert_eq!(a.matches(&b), MatchConfidence::Ambiguous);
    }

    #[test]
    fn episode_number_mismatch_rejects() {
        let a = Episode {
            provider_id: "a".into(),
            number: 1,
            title: "Episode 1".into(),
            url: "u".into(),
        };
        let b = Episode {
            provider_id: "b".into(),
            number: 2,
            title: "Episode 2".into(),
            url: "u".into(),
        };
        assert!(!episodes_match(&a, &b));
    }

    #[test]
    fn ova_vs_numbered_rejects() {
        let a = Episode {
            provider_id: "a".into(),
            number: 1,
            title: "Episode 1".into(),
            url: "u".into(),
        };
        let b = Episode {
            provider_id: "b".into(),
            number: 0,
            title: "OVA 1".into(),
            url: "u".into(),
        };
        assert!(!episodes_match(&a, &b));
    }
}
