//! Typed navigation model for context-aware "Back" navigation.
//
//! Every meaningful screen transition pushes a `NavEntry` onto a bounded stack
//! stored in `AppState.nav_stack`. Back pops exactly the current entry and
//! restores the entry beneath it (the origin), so WatchPage returns to the
//! screen that opened the video instead of always jumping to Home.
//!
//! Temporary media sources (signed stream URLs, libmpv state) are never stored
//! here — only stable identities: video id, anime key, search query, playlist
//! name.

use crate::{AppWindow, app_context::AppContext, state::AppState};

/// A navigable destination. Each variant carries just enough stable context to
/// restore the screen without re-fetching when cached results are still valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavRoute {
    YouTubeHome,
    YouTubeSearch { query: String },
    YouTubeHistory,
    YouTubeFavorites,
    YouTubePlaylist { name: String },
    AnimeHome,
    AnimeDetails { key: String },
    WatchPage { video_id: String },
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavEntry {
    pub route: NavRoute,
    pub title: String,
}

impl NavEntry {
    pub fn new(route: NavRoute) -> Self {
        Self {
            title: route.title().to_string(),
            route,
        }
    }
}

impl NavRoute {
    /// The Slint `current-page` value that renders this route.
    pub fn page(&self) -> &'static str {
        match self {
            NavRoute::YouTubeHome => "youtube-home",
            NavRoute::YouTubeSearch { .. } => "search",
            NavRoute::YouTubeHistory => "history",
            NavRoute::YouTubeFavorites | NavRoute::YouTubePlaylist { .. } => "library",
            NavRoute::AnimeHome | NavRoute::AnimeDetails { .. } => "anime",
            NavRoute::WatchPage { .. } => "watch",
            NavRoute::Settings => "settings",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            NavRoute::YouTubeHome => "YouTube Home",
            NavRoute::YouTubeSearch { .. } => "Search Results",
            NavRoute::YouTubeHistory => "Watch History",
            NavRoute::YouTubeFavorites => "Favorites",
            NavRoute::YouTubePlaylist { .. } => "Playlist",
            NavRoute::AnimeHome => "Anime",
            NavRoute::AnimeDetails { .. } => "Anime Details",
            NavRoute::WatchPage { .. } => "Watch",
            NavRoute::Settings => "Settings",
        }
    }
}

const NAV_STACK_LIMIT: usize = 50;

fn same_route(a: &NavRoute, b: &NavRoute) -> bool {
    match (a, b) {
        (NavRoute::YouTubeHome, NavRoute::YouTubeHome) => true,
        (NavRoute::YouTubeSearch { query: q1 }, NavRoute::YouTubeSearch { query: q2 }) => q1 == q2,
        (NavRoute::YouTubeHistory, NavRoute::YouTubeHistory) => true,
        (NavRoute::YouTubeFavorites, NavRoute::YouTubeFavorites) => true,
        (NavRoute::YouTubePlaylist { name: n1 }, NavRoute::YouTubePlaylist { name: n2 }) => {
            n1 == n2
        }
        (NavRoute::AnimeHome, NavRoute::AnimeHome) => true,
        (NavRoute::AnimeDetails { key: k1 }, NavRoute::AnimeDetails { key: k2 }) => k1 == k2,
        (NavRoute::WatchPage { video_id: v1 }, NavRoute::WatchPage { video_id: v2 }) => v1 == v2,
        (NavRoute::Settings, NavRoute::Settings) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_stack(routes: Vec<NavRoute>) -> AppState {
        AppState {
            nav_stack: routes.into_iter().map(NavEntry::new).collect(),
            ..AppState::default()
        }
    }

    #[test]
    fn push_entry_records_the_new_screen() {
        let mut state = state_with_stack(vec![NavRoute::YouTubeHome]);

        push_entry(
            &mut state,
            NavEntry::new(NavRoute::YouTubeSearch {
                query: "naruto".into(),
            }),
        );

        assert_eq!(
            state.nav_stack.last().map(|e| e.route.clone()),
            Some(NavRoute::YouTubeSearch {
                query: "naruto".into()
            })
        );
        assert_eq!(state.nav_stack.len(), 2);
    }

    #[test]
    fn push_entry_skips_consecutive_duplicates() {
        let mut state = state_with_stack(vec![
            NavRoute::YouTubeHome,
            NavRoute::YouTubeSearch {
                query: "naruto".into(),
            },
        ]);

        push_entry(
            &mut state,
            NavEntry::new(NavRoute::YouTubeSearch {
                query: "naruto".into(),
            }),
        );

        assert_eq!(state.nav_stack.len(), 2);
    }

    #[test]
    fn pop_current_returns_the_origin() {
        let mut state = state_with_stack(vec![
            NavRoute::YouTubeHome,
            NavRoute::YouTubeSearch {
                query: "naruto".into(),
            },
            NavRoute::WatchPage {
                video_id: "abc123".into(),
            },
        ]);

        let destination = pop_current(&mut state).unwrap();

        assert_eq!(
            destination.route,
            NavRoute::WatchPage {
                video_id: "abc123".into()
            }
        );
        assert_eq!(
            state.nav_stack.last().map(|e| e.route.clone()),
            Some(NavRoute::YouTubeSearch {
                query: "naruto".into()
            })
        );
    }

    #[test]
    fn pop_current_keeps_the_root() {
        let mut state = state_with_stack(vec![NavRoute::YouTubeHome]);

        assert!(pop_current(&mut state).is_none());
        assert_eq!(state.nav_stack.len(), 1);
    }

    #[test]
    fn stack_is_capped() {
        let mut state = state_with_stack(vec![]);

        for i in 0..(NAV_STACK_LIMIT + 10) {
            push_entry(
                &mut state,
                NavEntry::new(NavRoute::YouTubeSearch {
                    query: format!("q{i}"),
                }),
            );
        }

        assert!(state.nav_stack.len() <= NAV_STACK_LIMIT);
        // The oldest entries are dropped from the bottom.
        assert_eq!(
            state.nav_stack.first().map(|e| e.route.clone()),
            Some(NavRoute::YouTubeSearch {
                query: "q0".into()
            })
        );
    }

    #[test]
    fn same_route_with_different_context_is_not_a_duplicate() {
        let mut state = state_with_stack(vec![NavRoute::YouTubeHome]);

        push_entry(
            &mut state,
            NavEntry::new(NavRoute::YouTubeSearch {
                query: "naruto".into(),
            }),
        );
        push_entry(
            &mut state,
            NavEntry::new(NavRoute::YouTubeSearch {
                query: "bleach".into(),
            }),
        );

        assert_eq!(state.nav_stack.len(), 3);
    }
}

/// Push a navigation entry. Consecutive duplicates (same route + context) are
/// ignored so rapid re-opens do not grow the stack. The stack is capped.
pub fn push_entry(state: &mut AppState, entry: NavEntry) {
    if let Some(last) = state.nav_stack.last() {
        if same_route(&last.route, &entry.route) {
            return;
        }
    }
    state.nav_stack.push(entry);
    while state.nav_stack.len() > NAV_STACK_LIMIT {
        if state.nav_stack.len() > 1 {
            state.nav_stack.remove(1);
        } else {
            break;
        }
    }
}

/// Pop the current screen and return the destination (the new top of the
/// stack). Returns `None` when only the root remains, meaning there is
/// nowhere to go back to.
pub fn pop_current(state: &mut AppState) -> Option<NavEntry> {
    if state.nav_stack.len() <= 1 {
        return None;
    }
    state.nav_stack.pop()
}

/// Restore UI context for a destination route without re-fetching cached data.
pub fn restore_destination(ui: &AppWindow, route: &NavRoute) {
    match route {
        NavRoute::YouTubeSearch { query } => {
            // Echo the preserved query into the search box. The results already
            // live in `AppState.search_results` and are deliberately not
            // re-fetched here.
            let _ = ui.set_search_query(query.clone().into());
        }
        NavRoute::AnimeHome => {
            let _ = ui.set_anime_view("home".into());
            let _ = ui.set_anime_selected_episode_number(-1);
        }
        NavRoute::AnimeDetails { key } => {
            let _ = ui.set_anime_view("details".into());
            let _ = ui.set_anime_selected_key(key.clone().into());
        }
        _ => {}
    }
}

fn set_page(ui: &AppWindow, route: &NavRoute) {
    ui.set_current_page(route.page().into());
}

/// Re-activate a destination route after navigating back to it. Loads durable
/// data (history / library) when the screen has no cached content yet.
fn activate_route(ui: &AppWindow, route: &NavRoute) {
    match route {
        NavRoute::YouTubeHome => {
            let _ = ui.invoke_load_home();
        }
        NavRoute::YouTubeHistory => {
            let _ = ui.invoke_show_history();
        }
        NavRoute::YouTubeFavorites => {
            let _ = ui.set_library_mode("favorites".into());
            let _ = ui.set_selected_playlist("".into());
            let _ = ui.invoke_refresh_library();
        }
        NavRoute::YouTubePlaylist { name } => {
            let _ = ui.set_library_mode("playlist".into());
            let _ = ui.set_selected_playlist(name.clone().into());
            let _ = ui.invoke_refresh_library();
        }
        _ => {}
    }
}

/// Navigate to a new screen: push an entry, show its page, restore context,
/// and activate the destination.
pub fn push_navigation(ui: &AppWindow, context: &AppContext, entry: NavEntry) {
    let route = entry.route.clone();
    {
        let Ok(mut state) = context.state.lock() else {
            return;
        };
        push_entry(&mut state, entry);
        set_page(ui, &route);
        restore_destination(ui, &route);
    }
    activate_route(ui, &route);
}

/// Go back exactly one screen: pop the current entry, show and restore the
/// destination, and activate it. Returns `false` when there is no destination
/// (root).
pub fn navigate_back(ui: &AppWindow, context: &AppContext) -> bool {
    let destination = {
        let Ok(mut state) = context.state.lock() else {
            return false;
        };
        match pop_current(&mut state) {
            Some(_) => state.nav_stack.last().map(|entry| entry.route.clone()),
            None => None,
        }
    };

    let Some(route) = destination else {
        return false;
    };

    set_page(ui, &route);
    restore_destination(ui, &route);
    activate_route(ui, &route);

    true
}
