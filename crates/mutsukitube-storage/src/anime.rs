use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::database::open_connection;

/// Fraction of an episode that must be watched before it counts as completed.
/// Documented so the UI and storage agree on the watched threshold.
pub const WATCHED_COMPLETION_THRESHOLD: f64 = 0.9;

/// Minimum watched seconds before an episode becomes "continue watching".
pub const MIN_RESUME_SECONDS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeEntry {
    pub provider_id: String,
    pub anime_url: String,
    pub title: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeProgressRecord {
    pub provider_id: String,
    pub anime_url: String,
    pub title: String,
    pub thumbnail_url: String,
    pub episode_number: i32,
    pub episode_title: String,
    pub position_secs: u64,
    pub duration_secs: u64,
    pub completed: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeHistoryRecord {
    pub provider_id: String,
    pub anime_url: String,
    pub title: String,
    pub thumbnail_url: String,
    pub last_watched_at: i64,
}

pub fn anime_key(provider_id: &str, anime_url: &str) -> String {
    format!("{provider_id}|{anime_url}")
}

fn episode_key(anime_key: &str, episode_number: i32) -> String {
    format!("{anime_key}#{episode_number}")
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn ensure_anime_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS anime_history (
            anime_key TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            anime_url TEXT NOT NULL,
            title TEXT NOT NULL,
            thumbnail_url TEXT NOT NULL DEFAULT '',
            last_watched_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS anime_progress (
            episode_key TEXT PRIMARY KEY,
            anime_key TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            anime_url TEXT NOT NULL,
            episode_number INTEGER NOT NULL,
            episode_title TEXT NOT NULL DEFAULT '',
            position_secs INTEGER NOT NULL DEFAULT 0,
            duration_secs INTEGER NOT NULL DEFAULT 0,
            completed INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS anime_favorites (
            anime_key TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            anime_url TEXT NOT NULL,
            title TEXT NOT NULL,
            thumbnail_url TEXT NOT NULL DEFAULT '',
            added_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_anime_history_recent
        ON anime_history(last_watched_at DESC);

        CREATE INDEX IF NOT EXISTS idx_anime_progress_updated
        ON anime_progress(updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_anime_favorites_added
        ON anime_favorites(added_at DESC);
        ",
    )
}

// ---------------------------------------------------------------- public API

/// Record that an anime episode was opened, and persist its position.
///
/// This is the durable Continue Watching + history write path. It never stores
/// streaming URLs or headers, only provider identity, episode number, title and
/// the poster reference.
pub fn save_anime_progress(
    entry: &AnimeEntry,
    episode_number: i32,
    episode_title: &str,
    position_secs: u64,
    duration_secs: u64,
) -> Result<()> {
    let conn = open_connection()?;
    ensure_anime_tables(&conn)?;
    save_anime_progress_with_conn(
        &conn,
        entry,
        episode_number,
        episode_title,
        position_secs,
        duration_secs,
    )
}

/// Episodes that are in progress (not completed) and worth resuming.
pub fn get_continue_watching(limit: usize) -> Result<Vec<AnimeProgressRecord>> {
    let conn = open_connection()?;
    ensure_anime_tables(&conn)?;
    get_continue_watching_with_conn(&conn, limit)
}

pub fn get_anime_history(limit: usize) -> Result<Vec<AnimeHistoryRecord>> {
    let conn = open_connection()?;
    ensure_anime_tables(&conn)?;
    get_anime_history_with_conn(&conn, limit)
}

pub fn get_anime_progress_for(
    provider_id: &str,
    anime_url: &str,
) -> Result<Vec<AnimeProgressRecord>> {
    let conn = open_connection()?;
    ensure_anime_tables(&conn)?;
    get_anime_progress_for_with_conn(&conn, provider_id, anime_url)
}

/// Toggle favorite state; returns the new state (true = favorited).
pub fn toggle_anime_favorite(entry: &AnimeEntry) -> Result<bool> {
    let conn = open_connection()?;
    ensure_anime_tables(&conn)?;
    toggle_anime_favorite_with_conn(&conn, entry)
}

pub fn is_anime_favorite(provider_id: &str, anime_url: &str) -> Result<bool> {
    let conn = open_connection()?;
    ensure_anime_tables(&conn)?;
    is_anime_favorite_with_conn(&conn, provider_id, anime_url)
}

pub fn get_anime_favorites(limit: usize) -> Result<Vec<AnimeEntry>> {
    let conn = open_connection()?;
    ensure_anime_tables(&conn)?;
    get_anime_favorites_with_conn(&conn, limit)
}

pub fn is_completed(position_secs: u64, duration_secs: u64) -> bool {
    if duration_secs == 0 {
        return false;
    }
    position_secs as f64 / duration_secs as f64 >= WATCHED_COMPLETION_THRESHOLD
}

// ------------------------------------------------- connection-scoped helpers

fn save_anime_progress_with_conn(
    conn: &Connection,
    entry: &AnimeEntry,
    episode_number: i32,
    episode_title: &str,
    position_secs: u64,
    duration_secs: u64,
) -> Result<()> {
    let key = anime_key(&entry.provider_id, &entry.anime_url);
    let completed = is_completed(position_secs, duration_secs);
    let timestamp = now();

    conn.execute(
        "
        INSERT INTO anime_history (
            anime_key, provider_id, anime_url, title, thumbnail_url, last_watched_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)

        ON CONFLICT(anime_key) DO UPDATE SET
            title = excluded.title,
            thumbnail_url = excluded.thumbnail_url,
            last_watched_at = excluded.last_watched_at
        ",
        params![
            key,
            entry.provider_id,
            entry.anime_url,
            entry.title,
            entry.thumbnail_url,
            timestamp,
        ],
    )?;

    let ep_key = episode_key(&key, episode_number);

    conn.execute(
        "
        INSERT INTO anime_progress (
            episode_key, anime_key, provider_id, anime_url, episode_number,
            episode_title, position_secs, duration_secs, completed, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)

        ON CONFLICT(episode_key) DO UPDATE SET
            episode_title = excluded.episode_title,
            position_secs = excluded.position_secs,
            duration_secs = excluded.duration_secs,
            completed = excluded.completed,
            updated_at = excluded.updated_at
        ",
        params![
            ep_key,
            key,
            entry.provider_id,
            entry.anime_url,
            episode_number,
            episode_title,
            position_secs as i64,
            duration_secs as i64,
            completed as i64,
            timestamp,
        ],
    )?;

    Ok(())
}

fn get_continue_watching_with_conn(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<AnimeProgressRecord>> {
    let mut statement = conn.prepare(
        "
        SELECT
            p.provider_id,
            p.anime_url,
            COALESCE(h.title, ''),
            COALESCE(h.thumbnail_url, ''),
            p.episode_number,
            p.episode_title,
            p.position_secs,
            p.duration_secs,
            p.completed,
            p.updated_at
        FROM anime_progress p
        LEFT JOIN anime_history h ON h.anime_key = p.anime_key
        WHERE p.completed = 0 AND p.position_secs >= ?2
        ORDER BY p.updated_at DESC
        LIMIT ?1
        ",
    )?;

    let rows = statement.query_map(
        params![limit as i64, MIN_RESUME_SECONDS as i64],
        map_progress,
    )?;

    rows.collect()
}

fn get_anime_progress_for_with_conn(
    conn: &Connection,
    provider_id: &str,
    anime_url: &str,
) -> Result<Vec<AnimeProgressRecord>> {
    let key = anime_key(provider_id, anime_url);

    let mut statement = conn.prepare(
        "
        SELECT
            p.provider_id,
            p.anime_url,
            COALESCE(h.title, ''),
            COALESCE(h.thumbnail_url, ''),
            p.episode_number,
            p.episode_title,
            p.position_secs,
            p.duration_secs,
            p.completed,
            p.updated_at
        FROM anime_progress p
        LEFT JOIN anime_history h ON h.anime_key = p.anime_key
        WHERE p.anime_key = ?1
        ",
    )?;

    let rows = statement.query_map(params![key], map_progress)?;

    rows.collect()
}

fn map_progress(row: &rusqlite::Row<'_>) -> Result<AnimeProgressRecord> {
    Ok(AnimeProgressRecord {
        provider_id: row.get(0)?,
        anime_url: row.get(1)?,
        title: row.get(2)?,
        thumbnail_url: row.get(3)?,
        episode_number: row.get(4)?,
        episode_title: row.get(5)?,
        position_secs: row.get::<_, i64>(6)? as u64,
        duration_secs: row.get::<_, i64>(7)? as u64,
        completed: row.get::<_, i64>(8)? != 0,
        updated_at: row.get(9)?,
    })
}

fn get_anime_history_with_conn(conn: &Connection, limit: usize) -> Result<Vec<AnimeHistoryRecord>> {
    let mut statement = conn.prepare(
        "
        SELECT provider_id, anime_url, title, thumbnail_url, last_watched_at
        FROM anime_history
        ORDER BY last_watched_at DESC
        LIMIT ?1
        ",
    )?;

    let rows = statement.query_map(params![limit as i64], |row| {
        Ok(AnimeHistoryRecord {
            provider_id: row.get(0)?,
            anime_url: row.get(1)?,
            title: row.get(2)?,
            thumbnail_url: row.get(3)?,
            last_watched_at: row.get(4)?,
        })
    })?;

    rows.collect()
}

fn toggle_anime_favorite_with_conn(conn: &Connection, entry: &AnimeEntry) -> Result<bool> {
    let key = anime_key(&entry.provider_id, &entry.anime_url);

    let existing: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM anime_favorites WHERE anime_key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?;

    if existing.is_some() {
        conn.execute(
            "DELETE FROM anime_favorites WHERE anime_key = ?1",
            params![key],
        )?;
        Ok(false)
    } else {
        conn.execute(
            "
            INSERT INTO anime_favorites (
                anime_key, provider_id, anime_url, title, thumbnail_url, added_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                key,
                entry.provider_id,
                entry.anime_url,
                entry.title,
                entry.thumbnail_url,
                now(),
            ],
        )?;
        Ok(true)
    }
}

fn is_anime_favorite_with_conn(
    conn: &Connection,
    provider_id: &str,
    anime_url: &str,
) -> Result<bool> {
    let key = anime_key(provider_id, anime_url);

    let existing: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM anime_favorites WHERE anime_key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?;

    Ok(existing.is_some())
}

fn get_anime_favorites_with_conn(conn: &Connection, limit: usize) -> Result<Vec<AnimeEntry>> {
    let mut statement = conn.prepare(
        "
        SELECT provider_id, anime_url, title, thumbnail_url
        FROM anime_favorites
        ORDER BY added_at DESC
        LIMIT ?1
        ",
    )?;

    let rows = statement.query_map(params![limit as i64], |row| {
        Ok(AnimeEntry {
            provider_id: row.get(0)?,
            anime_url: row.get(1)?,
            title: row.get(2)?,
            thumbnail_url: row.get(3)?,
        })
    })?;

    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(url: &str) -> AnimeEntry {
        AnimeEntry {
            provider_id: "fake".into(),
            anime_url: url.into(),
            title: "Test Anime".into(),
            thumbnail_url: "https://fake.example/poster.jpg".into(),
        }
    }

    fn connection() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        ensure_anime_tables(&conn).expect("schema");
        conn
    }

    #[test]
    fn completion_threshold() {
        assert!(!is_completed(0, 0));
        assert!(!is_completed(10, 100));
        assert!(!is_completed(89, 100));
        assert!(is_completed(90, 100));
        assert!(is_completed(100, 100));
    }

    #[test]
    fn anime_key_is_stable() {
        assert_eq!(
            anime_key("fake", "https://x/1"),
            anime_key("fake", "https://x/1")
        );
        assert_ne!(
            anime_key("fake", "https://x/1"),
            anime_key("fake", "https://x/2")
        );
    }

    #[test]
    fn progress_below_minimum_is_not_continue_watching() {
        let conn = connection();
        let item = entry("https://fake.example/anime/a");

        save_anime_progress_with_conn(&conn, &item, 1, "Pilot", 10, 1200).unwrap();

        assert!(
            get_continue_watching_with_conn(&conn, 10)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn in_progress_episode_appears_in_continue_watching() {
        let conn = connection();
        let item = entry("https://fake.example/anime/b");

        save_anime_progress_with_conn(&conn, &item, 3, "Episode Three", 300, 1200).unwrap();

        let records = get_continue_watching_with_conn(&conn, 10).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].episode_number, 3);
        assert_eq!(records[0].position_secs, 300);
        assert_eq!(records[0].title, "Test Anime");
        assert!(!records[0].completed);
    }

    #[test]
    fn completed_episode_leaves_continue_watching_but_stays_in_history() {
        let conn = connection();
        let item = entry("https://fake.example/anime/c");

        save_anime_progress_with_conn(&conn, &item, 1, "Pilot", 1150, 1200).unwrap();

        assert!(
            get_continue_watching_with_conn(&conn, 10)
                .unwrap()
                .is_empty()
        );

        let history = get_anime_history_with_conn(&conn, 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].anime_url, item.anime_url);

        let progress =
            get_anime_progress_for_with_conn(&conn, &item.provider_id, &item.anime_url).unwrap();
        assert_eq!(progress.len(), 1);
        assert!(progress[0].completed);
    }

    #[test]
    fn progress_upsert_does_not_duplicate_rows() {
        let conn = connection();
        let item = entry("https://fake.example/anime/d");

        save_anime_progress_with_conn(&conn, &item, 2, "Two", 100, 1200).unwrap();
        save_anime_progress_with_conn(&conn, &item, 2, "Two", 500, 1200).unwrap();

        let records = get_continue_watching_with_conn(&conn, 10).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].position_secs, 500);

        let history = get_anime_history_with_conn(&conn, 10).unwrap();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn progress_is_scoped_per_episode() {
        let conn = connection();
        let item = entry("https://fake.example/anime/e");

        save_anime_progress_with_conn(&conn, &item, 1, "One", 100, 1200).unwrap();
        save_anime_progress_with_conn(&conn, &item, 2, "Two", 200, 1200).unwrap();

        let records = get_continue_watching_with_conn(&conn, 10).unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn favorite_toggle_round_trips() {
        let conn = connection();
        let item = entry("https://fake.example/anime/f");

        assert!(!is_anime_favorite_with_conn(&conn, &item.provider_id, &item.anime_url).unwrap());

        assert!(toggle_anime_favorite_with_conn(&conn, &item).unwrap());
        assert!(is_anime_favorite_with_conn(&conn, &item.provider_id, &item.anime_url).unwrap());

        let favorites = get_anime_favorites_with_conn(&conn, 10).unwrap();
        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].anime_url, item.anime_url);

        assert!(!toggle_anime_favorite_with_conn(&conn, &item).unwrap());
        assert!(get_anime_favorites_with_conn(&conn, 10).unwrap().is_empty());
    }
}
