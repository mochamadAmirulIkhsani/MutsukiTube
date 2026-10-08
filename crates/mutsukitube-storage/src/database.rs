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

        CREATE INDEX IF NOT EXISTS
            idx_history_last_opened
        ON watch_history(last_opened_at DESC);
        ",
    )?;

    Ok(connection)
}
