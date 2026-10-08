use rusqlite::{OptionalExtension, Result, params};

use crate::database::open_connection;

const PROVIDER_KEY: &str = "provider_mode";

pub fn validate_provider_mode(value: &str) -> bool {
    matches!(value, "auto" | "native" | "ytdlp")
}

pub fn get_provider_mode() -> Result<String> {
    let conn = open_connection()?;

    let result: Option<String> = conn
        .query_row(
            "
            SELECT value
            FROM app_settings
            WHERE key = ?1
            ",
            params![PROVIDER_KEY],
            |row| row.get(0),
        )
        .optional()?;

    let value = result.unwrap_or_else(|| "auto".to_string());

    if validate_provider_mode(&value) {
        Ok(value)
    } else {
        Ok("auto".to_string())
    }
}

pub fn set_provider_mode(mode: &str) -> Result<()> {
    if !validate_provider_mode(mode) {
        return Err(rusqlite::Error::InvalidParameterName(mode.to_string()));
    }

    let conn = open_connection()?;

    conn.execute(
        "
        INSERT INTO app_settings (key, value)
        VALUES (?1, ?2)

        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value
        ",
        params![PROVIDER_KEY, mode],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_provider_modes() {
        assert!(validate_provider_mode("auto"));
        assert!(validate_provider_mode("native"));
        assert!(validate_provider_mode("ytdlp"));

        assert!(!validate_provider_mode("invalid"));
    }
}
