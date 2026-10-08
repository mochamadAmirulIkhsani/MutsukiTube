use std::{fs, path::PathBuf, time::Duration};

use rusqlite::{Connection, Result};

pub fn database_path() -> std::io::Result<PathBuf> {
    let mut path = dirs::data_local_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Local app data directory unavailable",
        )
    })?;

    path.push("MutsukiTube");

    fs::create_dir_all(&path)?;

    path.push("mutsukitube.db");

    Ok(path)
}

pub fn open_connection() -> Result<Connection> {
    let path = database_path()
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;

    let connection = Connection::open(path)?;

    connection.pragma_update(None, "foreign_keys", "ON")?;

    connection.busy_timeout(Duration::from_secs(5))?;

    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS watch_history (
            video_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            channel TEXT NOT NULL,
            thumbnail TEXT NOT NULL,
            duration INTEGER,
            last_opened_at INTEGER NOT NULL,
            open_count INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL);
        
        CREATE TABLE IF NOT EXISTS favorites (
            video_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            channel TEXT NOT NULL,
            thumbnail TEXT NOT NULL,
            duration INTEGER,
            added_at INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE TABLE IF NOT EXISTS playlists (
            name TEXT PRIMARY KEY,
            created_at INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE TABLE IF NOT EXISTS playlist_items (
            playlist_name TEXT NOT NULL,
            video_id TEXT NOT NULL,
            title TEXT NOT NULL,
            channel TEXT NOT NULL,
            thumbnail TEXT NOT NULL,
            duration INTEGER,
            added_at INTEGER NOT NULL DEFAULT (unixepoch()),

            PRIMARY KEY (playlist_name, video_id),

            FOREIGN KEY (playlist_name)
                REFERENCES playlists(name)
                ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_favorites_added
        ON favorites(added_at DESC);

        CREATE INDEX IF NOT EXISTS idx_playlist_items_added
        ON playlist_items(playlist_name, added_at DESC);

        CREATE INDEX IF NOT EXISTS
            idx_history_last_opened
        ON watch_history(last_opened_at DESC);
        ",
    )?;

    Ok(connection)
}
