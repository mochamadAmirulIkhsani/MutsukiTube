use crate::database::open_connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscribedChannel {
    pub id: String,
    pub url: String,
    pub title: String,
    pub avatar_url: Option<String>,
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    fields.push(current.trim().to_string());
    fields
}

/// Language-agnostic parser for Google Takeout subscription CSVs.
///
/// Matches regardless of whether the file is named `subscriptions.csv`, `langganan.csv`,
/// `suscripciones.csv`, or whether the headers are in English, Indonesian, Spanish, etc.
/// Validation relies on YouTube Channel ID format (24 characters, starting with "UC").
pub fn parse_subscriptions_csv(content: &str) -> Vec<SubscribedChannel> {
    let mut channels = Vec::new();

    for line in content.lines() {
        let line = line.trim_start_matches('\u{feff}').trim();
        if line.is_empty() {
            continue;
        }

        let parts = split_csv_line(line);
        if parts.len() < 3 {
            continue;
        }

        let channel_id = &parts[0];
        let channel_url = &parts[1];
        let channel_title = &parts[2];

        // YouTube Channel IDs start with "UC" and are 24 characters long.
        // This pattern ignores header lines across ALL languages.
        if channel_id.starts_with("UC") && channel_id.len() == 24 {
            channels.push(SubscribedChannel {
                id: channel_id.clone(),
                url: channel_url.clone(),
                title: channel_title.clone(),
                avatar_url: None,
            });
        }
    }

    channels
}

/// Batch insert or update subscribed channels in local SQLite database.
pub fn import_subscriptions(channels: &[SubscribedChannel]) -> rusqlite::Result<usize> {
    if channels.is_empty() {
        return Ok(0);
    }

    let mut connection = open_connection()?;
    let tx = connection.transaction()?;

    let mut count = 0;
    for channel in channels {
        let changed = tx.execute(
            "INSERT OR REPLACE INTO subscriptions (channel_id, channel_url, channel_title)
             VALUES (?1, ?2, ?3)",
            (&channel.id, &channel.url, &channel.title),
        )?;
        count += changed;
    }

    tx.commit()?;
    Ok(count)
}

/// Retrieve all imported subscriptions sorted by channel title.
pub fn get_subscriptions() -> rusqlite::Result<Vec<SubscribedChannel>> {
    let connection = open_connection()?;
    let mut stmt = connection.prepare(
        "SELECT channel_id, channel_url, channel_title, avatar_url 
         FROM subscriptions 
         ORDER BY channel_title COLLATE NOCASE ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(SubscribedChannel {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            avatar_url: row.get(3).ok(),
        })
    })?;

    let mut result = Vec::new();
    for channel in rows {
        result.push(channel?);
    }

    Ok(result)
}

/// Update the cached avatar URL for a channel.
pub fn update_channel_avatar(channel_id: &str, avatar_url: &str) -> rusqlite::Result<()> {
    let connection = open_connection()?;
    connection.execute(
        "UPDATE subscriptions SET avatar_url = ?1 WHERE channel_id = ?2",
        (avatar_url, channel_id),
    )?;
    Ok(())
}

/// Retrieve the total number of imported subscriptions.
pub fn get_subscriptions_count() -> rusqlite::Result<usize> {
    let connection = open_connection()?;
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM subscriptions",
        [],
        |row| row.get(0),
    )?;
    Ok(count.max(0) as usize)
}

/// Remove all imported subscriptions from the local database.
pub fn clear_subscriptions() -> rusqlite::Result<usize> {
    let connection = open_connection()?;
    let deleted = connection.execute("DELETE FROM subscriptions", [])?;
    Ok(deleted)
}

/// Remove a single subscribed channel by ID.
pub fn remove_subscription(channel_id: &str) -> rusqlite::Result<usize> {
    let connection = open_connection()?;
    let deleted = connection.execute(
        "DELETE FROM subscriptions WHERE channel_id = ?1",
        [channel_id],
    )?;
    Ok(deleted)
}

/// Check if a channel ID is subscribed.
pub fn is_subscribed(channel_id: &str) -> rusqlite::Result<bool> {
    let connection = open_connection()?;
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM subscriptions WHERE channel_id = ?1",
        [channel_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Add or update a single subscribed channel.
pub fn add_subscription(
    id: &str,
    url: &str,
    title: &str,
    avatar_url: Option<&str>,
) -> rusqlite::Result<()> {
    let connection = open_connection()?;
    connection.execute(
        "INSERT OR REPLACE INTO subscriptions (channel_id, channel_url, channel_title, avatar_url)
         VALUES (?1, ?2, ?3, ?4)",
        (id, url, title, avatar_url),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_english_takeout_csv() {
        let csv = "Channel Id,Channel Url,Channel Title\n\
UC_x5XG1OV2P6uZZ5FSM9Ttw,http://www.youtube.com/channel/UC_x5XG1OV2P6uZZ5FSM9Ttw,Google Developers\n\
UCsBjURrPoezykLs9EqgamOA,http://www.youtube.com/channel/UCsBjURrPoezykLs9EqgamOA,Fireship";

        let channels = parse_subscriptions_csv(csv);
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].id, "UC_x5XG1OV2P6uZZ5FSM9Ttw");
        assert_eq!(channels[0].title, "Google Developers");
        assert_eq!(channels[1].id, "UCsBjURrPoezykLs9EqgamOA");
        assert_eq!(channels[1].title, "Fireship");
    }

    #[test]
    fn parse_indonesian_takeout_csv() {
        let csv = "ID Channel,URL Channel,Judul Channel\n\
UC_x5XG1OV2P6uZZ5FSM9Ttw,http://www.youtube.com/channel/UC_x5XG1OV2P6uZZ5FSM9Ttw,Google Developers\n\
UCBJycsmduvYEL83R_U4JriQ,http://www.youtube.com/channel/UCBJycsmduvYEL83R_U4JriQ,Marques Brownlee";

        let channels = parse_subscriptions_csv(csv);
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].id, "UC_x5XG1OV2P6uZZ5FSM9Ttw");
        assert_eq!(channels[0].title, "Google Developers");
        assert_eq!(channels[1].id, "UCBJycsmduvYEL83R_U4JriQ");
        assert_eq!(channels[1].title, "Marques Brownlee");
    }

    #[test]
    fn parse_csv_handles_quoted_titles_with_commas() {
        let csv = "ID Channel,URL Channel,Judul Channel\n\
UC1234567890123456789012,http://www.youtube.com/channel/UC1234567890123456789012,\"Tech, News & Reviews\"";

        let channels = parse_subscriptions_csv(csv);
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].title, "Tech, News & Reviews");
    }
}

