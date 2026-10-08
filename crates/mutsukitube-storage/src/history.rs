use std::time::{SystemTime, UNIX_EPOCH};

use mutsukitube_core::Video;
use rusqlite::{Result, params};

use crate::database::open_connection;

pub fn save_video(video: &Video) -> Result<()> {
    let conn = open_connection()?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let duration = video.duration.map(|v| v as i64);

    conn.execute(
        "
        INSERT INTO watch_history (
            video_id,
            title,
            channel,
            thumbnail,
            duration,
            last_opened_at,
            open_count
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)

        ON CONFLICT(video_id) DO UPDATE SET
            title = excluded.title,
            channel = excluded.channel,
            thumbnail = excluded.thumbnail,
            duration = excluded.duration,
            last_opened_at = excluded.last_opened_at,
            open_count = watch_history.open_count + 1
        ",
        params![
            video.id,
            video.title,
            video.channel,
            video.thumbnail,
            duration,
            timestamp,
        ],
    )?;

    Ok(())
}

pub fn get_history(limit: usize) -> Result<Vec<Video>> {
    let conn = open_connection()?;

    let mut statement = conn.prepare(
        "
        SELECT
            video_id,
            title,
            channel,
            thumbnail,
            duration
        FROM watch_history
        ORDER BY
            last_opened_at DESC,
            rowid DESC
        LIMIT ?1
        ",
    )?;

    let videos = statement.query_map(params![limit as i64], |row| {
        let duration: Option<i64> = row.get(4)?;

        Ok(Video {
            id: row.get(0)?,
            title: row.get(1)?,
            channel: row.get(2)?,
            thumbnail: row.get(3)?,
            duration: duration.map(|v| v as u64),
        })
    })?;

    videos.collect()
}

pub fn clear_history() -> Result<()> {
    let conn = open_connection()?;

    conn.execute("DELETE FROM watch_history", [])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    #[test]
    fn sqlite_upsert_works() {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(
            "
            CREATE TABLE watch_history (
                video_id TEXT PRIMARY KEY,
                open_count INTEGER NOT NULL DEFAULT 1
            );
            ",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO watch_history (video_id) VALUES (?1)",
            params!["test-video"],
        )
        .unwrap();

        conn.execute(
            "
            INSERT INTO watch_history (video_id)
            VALUES (?1)
            ON CONFLICT(video_id) DO UPDATE SET
                open_count = open_count + 1
            ",
            params!["test-video"],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT open_count FROM watch_history WHERE video_id = ?1",
                params!["test-video"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 2);
    }
}
