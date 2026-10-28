//! Verification tests for mutsukitube-youtube duration parsing.

use mutsukitube_youtube::models::parse_duration;

#[test]
fn verify_issue_11_duration_parser_whitespace_trimmed() {
    assert_eq!(parse_duration("3:45"), Some(225));
    assert_eq!(parse_duration("1:02:03"), Some(3723));

    // Timestamps with leading/trailing whitespace now parse correctly
    let space_duration = " 3:45 ";
    assert_eq!(
        parse_duration(space_duration),
        Some(225),
        "FIXED: parse_duration successfully handles whitespace-padded timestamps"
    );

    // Live streams or non-numeric strings return None
    assert_eq!(parse_duration("LIVE"), None);
    assert_eq!(parse_duration("SHORTS"), None);
}
