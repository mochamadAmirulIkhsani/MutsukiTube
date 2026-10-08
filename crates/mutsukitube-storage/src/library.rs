use mutsukitube_core::Video;

use rusqlite::{OptionalExtension, Result, params};

use crate::database::open_connection;

pub fn is_favorite(video_id: &str) -> Result<bool> {
    let conn = open_connection()?;

    let result: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM favorites WHERE video_id = ?1",
            params![video_id],
            |row| row.get(0),
        )
        .optional()?;

    Ok(result.is_some())
}

pub fn toggle_favorite(video: &Video) -> Result<bool> {
    let mut conn = open_connection()?;
    let tx = conn.transaction()?;

    let exists: Option<i64> = tx
        .query_row(
            "SELECT 1 FROM favorites WHERE video_id = ?1",
            params![&video.id],
            |row| row.get(0),
        )
        .optional()?;

    let added = exists.is_none();

    if added {
        tx.execute(
            "
            INSERT INTO favorites (
                video_id, title, channel, thumbnail, duration
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                &video.id,
                &video.title,
                &video.channel,
                &video.thumbnail,
                video.duration.map(|v| v as i64),
            ],
        )?;
    } else {
        tx.execute(
            "DELETE FROM favorites WHERE video_id = ?1",
            params![&video.id],
        )?;
    }

    tx.commit()?;

    Ok(added)
}

pub fn get_favorites() -> Result<Vec<Video>> {
    let conn = open_connection()?;

    let mut stmt = conn.prepare(
        "
        SELECT video_id, title, channel, thumbnail, duration
        FROM favorites
        ORDER BY added_at DESC, rowid DESC
        ",
    )?;

    let rows = stmt.query_map([], |row| {
        let duration: Option<i64> = row.get(4)?;

        Ok(Video {
            id: row.get(0)?,
            title: row.get(1)?,
            channel: row.get(2)?,
            thumbnail: row.get(3)?,
            duration: duration.map(|v| v as u64),
        })
    })?;

    rows.collect()
}

pub fn create_playlist(name: &str) -> Result<()> {
    let name = name.trim();

    if name.is_empty() || name.len() > 100 {
        return Err(rusqlite::Error::InvalidParameterName(
            "Playlist name must be 1-100 bytes".into(),
        ));
    }

    let conn = open_connection()?;

    conn.execute("INSERT INTO playlists (name) VALUES (?1)", params![name])?;

    Ok(())
}

pub fn get_playlist_names() -> Result<Vec<String>> {
    let conn = open_connection()?;

    let mut stmt =
        conn.prepare("SELECT name FROM playlists ORDER BY created_at DESC, rowid DESC")?;

    let rows = stmt.query_map([], |row| row.get(0))?;

    rows.collect()
}

pub fn add_to_playlist(name: &str, video: &Video) -> Result<()> {
    let conn = open_connection()?;

    conn.execute(
        "
        INSERT INTO playlist_items (
            playlist_name,
            video_id,
            title,
            channel,
            thumbnail,
            duration
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)

        ON CONFLICT(playlist_name, video_id) DO NOTHING
        ",
        params![
            name,
            &video.id,
            &video.title,
            &video.channel,
            &video.thumbnail,
            video.duration.map(|v| v as i64),
        ],
    )?;

    Ok(())
}

pub fn get_playlist_videos(name: &str) -> Result<Vec<Video>> {
    let conn = open_connection()?;

    let mut stmt = conn.prepare(
        "
        SELECT video_id, title, channel, thumbnail, duration
        FROM playlist_items
        WHERE playlist_name = ?1
        ORDER BY added_at DESC, rowid DESC
        ",
    )?;

    let rows = stmt.query_map(params![name], |row| {
        let duration: Option<i64> = row.get(4)?;

        Ok(Video {
            id: row.get(0)?,
            title: row.get(1)?,
            channel: row.get(2)?,
            thumbnail: row.get(3)?,
            duration: duration.map(|v| v as u64),
        })
    })?;

    rows.collect()
}
