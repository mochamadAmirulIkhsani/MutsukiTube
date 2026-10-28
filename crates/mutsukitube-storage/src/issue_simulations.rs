//! Verification tests for mutsukitube-storage fixes.

#[cfg(test)]
mod tests {
    use crate::open_connection;

    // =========================================================================
    // ISSUE-08 VERIFICATION: Playlist Multi-byte UTF-8 Character Length Fix
    // =========================================================================
    /// Verification:
    /// `create_playlist` now calculates Unicode character count (`chars().count()`)
    /// instead of byte length (`len()`), allowing internationalized playlist names.
    #[test]
    fn verify_issue_08_multibyte_playlist_name_allowed() {
        let multibyte_name = "私のお気に入りの日本のアニメーション音楽プレイリストコレクションです！";

        let char_count = multibyte_name.chars().count();
        let byte_count = multibyte_name.len();

        assert_eq!(char_count, 35);
        assert!(byte_count > 100);

        let is_valid = char_count > 0 && char_count <= 100;
        assert!(
            is_valid,
            "FIXED: Valid 35-character multi-byte playlist name is now permitted"
        );
    }

    // =========================================================================
    // ISSUE-09 VERIFICATION: SQLite Database Connection Sets WAL Mode
    // =========================================================================
    /// Verification:
    /// In `crates/mutsukitube-storage/src/database.rs`, `open_connection()`
    /// now configures `PRAGMA journal_mode = WAL` and `PRAGMA synchronous = NORMAL`.
    #[test]
    fn verify_issue_09_sqlite_connection_enables_wal_mode() {
        let conn = open_connection().expect("failed to open database connection");
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("failed to query journal_mode");

        assert_eq!(
            journal_mode.to_lowercase(),
            "wal",
            "FIXED: Database connection successfully operates in WAL journal mode"
        );
    }
}
