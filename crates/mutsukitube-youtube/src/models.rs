use serde_json::Value;

use mutsukitube_core::Video;

pub fn extract_text(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }

    if let Some(text) = value.get("simpleText").and_then(Value::as_str) {
        return text.to_string();
    }

    value
        .get("runs")
        .and_then(Value::as_array)
        .map(|runs| {
            runs.iter()
                .filter_map(|run| run.get("text").and_then(Value::as_str))
                .collect::<String>()
        })
        .unwrap_or_default()
}

pub fn parse_video_renderer(renderer: &Value) -> Option<Video> {
    let id = renderer.get("videoId")?.as_str()?.to_string();

    let title = extract_text(renderer.get("title")?);

    if title.is_empty() {
        return None;
    }

    let channel = renderer
        .get("ownerText")
        .or_else(|| renderer.get("longBylineText"))
        .or_else(|| renderer.get("shortBylineText"))
        .map(extract_text)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "Unknown channel".into());

    let thumbnail = renderer
        .pointer("/thumbnail/thumbnails")
        .and_then(Value::as_array)
        .and_then(|items| items.last())
        .and_then(|item| item.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let duration_text = renderer
        .get("lengthText")
        .map(extract_text)
        .unwrap_or_default();

    Some(Video {
        id,
        title,
        channel,
        thumbnail,
        duration: parse_duration(&duration_text),
    })
}

pub fn parse_duration(text: &str) -> Option<u64> {
    if text.is_empty() {
        return None;
    }

    let mut total = 0u64;

    for part in text.split(':') {
        let value: u64 = part.parse().ok()?;

        total = total.checked_mul(60)?.checked_add(value)?;
    }

    Some(total)
}

pub fn collect_videos(value: &Value, output: &mut Vec<Video>) {
    match value {
        Value::Object(map) => {
            if let Some(renderer) = map.get("videoRenderer") {
                if let Some(video) = parse_video_renderer(renderer) {
                    if !output.iter().any(|existing| existing.id == video.id) {
                        output.push(video);
                    }
                }

                return;
            }

            for child in map.values() {
                collect_videos(child, output);
            }
        }

        Value::Array(items) => {
            for item in items {
                collect_videos(item, output);
            }
        }

        _ => {}
    }
}

pub fn extract_continuation_token(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(token) = map
                .get("continuationItemRenderer")
                .and_then(|item| {
                    item.pointer("/continuationEndpoint/continuationCommand/token")
                        .or_else(|| {
                            item.pointer("/button/buttonRenderer/command/continuationCommand/token")
                        })
                })
                .and_then(Value::as_str)
            {
                if !token.is_empty() {
                    return Some(token.to_owned());
                }
            }

            if let Some(token) = map
                .get("nextContinuationData")
                .and_then(|item| item.get("continuation"))
                .and_then(Value::as_str)
            {
                if !token.is_empty() {
                    return Some(token.to_owned());
                }
            }

            for child in map.values() {
                if let Some(token) = extract_continuation_token(child) {
                    return Some(token);
                }
            }

            None
        }

        Value::Array(items) => items.iter().find_map(extract_continuation_token),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn duration_parser_works() {
        assert_eq!(parse_duration("3:45"), Some(225));
        assert_eq!(parse_duration("1:02:30"), Some(3750));
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn video_renderer_parser_works() {
        let response = json!({
            "contents": [{
                "videoRenderer": {
                    "videoId": "abc123",
                    "title": {
                        "runs": [{"text": "Learn Rust"}]
                    },
                    "ownerText": {
                        "runs": [{"text": "Rust Channel"}]
                    },
                    "lengthText": {
                        "simpleText": "3:45"
                    },
                    "thumbnail": {
                        "thumbnails": [{
                            "url": "https://example.com/thumb.jpg"
                        }]
                    }
                }
            }]
        });

        let mut videos = Vec::new();

        collect_videos(&response, &mut videos);

        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].id, "abc123");
        assert_eq!(videos[0].title, "Learn Rust");
        assert_eq!(videos[0].duration, Some(225));
    }

    #[test]
    fn continuation_parser_works() {
        let response = json!({
            "contents": [
                {
                    "videoRenderer": {
                        "videoId": "abc123"
                    }
                },
                {
                    "continuationItemRenderer": {
                        "continuationEndpoint": {
                            "continuationCommand": {
                                "token": "NEXT_PAGE_TOKEN"
                            }
                        }
                    }
                }
            ]
        });

        assert_eq!(
            extract_continuation_token(&response),
            Some("NEXT_PAGE_TOKEN".to_string())
        );
    }

    #[test]
    fn no_continuation_returns_none() {
        let response = json!({
            "contents": []
        });

        assert_eq!(extract_continuation_token(&response), None);
    }
}
