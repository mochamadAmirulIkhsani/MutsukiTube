use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::thread;
use std::time::Duration;

fn main() {
    let mode = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("FAKE_PROVIDER_MODE").ok())
        .unwrap_or_else(|| "ok".to_string());
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    if mode == "reorder" {
        let mut lines = reader.lines();
        let first = lines.next().unwrap().unwrap();
        let second = lines.next().unwrap().unwrap();
        let req1: Value = serde_json::from_str(first.trim()).unwrap();
        let req2: Value = serde_json::from_str(second.trim()).unwrap();
        let id1 = req1.get("requestId").unwrap().as_str().unwrap().to_string();
        let id2 = req2.get("requestId").unwrap().as_str().unwrap().to_string();
        thread::sleep(Duration::from_millis(50));
        let r2 = json!({"protocolVersion":1,"requestId":id2,"success":true,"data":{"results":[]}});
        let _ = writeln!(stdout, "{}", r2);
        let _ = stdout.flush();
        thread::sleep(Duration::from_millis(50));
        let r1 = json!({"protocolVersion":1,"requestId":id1,"success":true,"data":{"results":[]}});
        let _ = writeln!(stdout, "{}", r1);
        let _ = stdout.flush();
        return;
    }

    if mode == "crash" {
        // Exit immediately so the host observes a spawn/start failure or EOF.
        std::process::exit(1);
    }

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                let _ = writeln!(
                    stdout,
                    "{{\"protocolVersion\":1,\"requestId\":\"\",\"success\":false,\"error\":{{\"code\":\"UNSUPPORTED\",\"message\":\"malformed\",\"retryable\":false}}}}"
                );
                let _ = stdout.flush();
                continue;
            }
        };

        let request_id = req
            .get("requestId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let action = req
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        match mode.as_str() {
            "timeout" => {
                thread::sleep(Duration::from_secs(10));
                continue;
            }
            "malformed" => {
                let _ = writeln!(stdout, "not-json-at-all");
                let _ = stdout.flush();
                // Then send a valid response so the host can recover.
                let response = json!({
                    "protocolVersion": 1,
                    "requestId": request_id,
                    "success": true,
                    "data": {"info": info()}
                });
                let _ = writeln!(stdout, "{}", response);
                let _ = stdout.flush();
                continue;
            }
            "oversized" => {
                let big = "x".repeat((1 << 20) + 100);
                let _ = writeln!(stdout, "{}", big);
                let _ = stdout.flush();
                continue;
            }
            "bad-protocol" => {
                let response = json!({
                    "protocolVersion": 2,
                    "requestId": request_id,
                    "success": true,
                    "data": {"info": info()}
                });
                let _ = writeln!(stdout, "{}", response);
                let _ = stdout.flush();
                continue;
            }
            "unknown-id" => {
                let response = json!({
                    "protocolVersion": 1,
                    "requestId": "definitely-not-the-request-id",
                    "success": true,
                    "data": {"info": info()}
                });
                let _ = writeln!(stdout, "{}", response);
                let _ = stdout.flush();
                continue;
            }
            "reorder" => {
                // Collect two requests, then respond out of order.
                // This mode is handled specially below; fall through to default for single.
            }
            _ => {}
        }

        // Reorder mode: read a second request first.
        if mode == "reorder" {
            // We already read first line; read second.
            // Use a nested reader? Simpler: respond after a small delay in reverse by storing.
            // Implementation: sleep, then write second response, then first.
            // But we don't have second request yet. Read it now.
            // Since this loop processes one line at a time, reorder mode needs state.
            // Fall back to immediate response for reorder tests handled by "reorder2".
        }

        let response = match action.as_str() {
            "info" => json!({
                "protocolVersion": 1,
                "requestId": request_id,
                "success": true,
                "data": {"info": info()}
            }),
            "search" => {
                let query = req
                    .pointer("/params/query")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if query == "empty" {
                    json!({
                        "protocolVersion": 1,
                        "requestId": request_id,
                        "success": true,
                        "data": {"results": []}
                    })
                } else {
                    json!({
                        "protocolVersion": 1,
                        "requestId": request_id,
                        "success": true,
                        "data": {"results": [anime(query)]}
                    })
                }
            }
            "episodes" => json!({
                "protocolVersion": 1,
                "requestId": request_id,
                "success": true,
                "data": {"episodes": [episode()]}
            }),
            "heights" => json!({
                "protocolVersion": 1,
                "requestId": request_id,
                "success": true,
                "data": {"heights": [360, 720]}
            }),
            "stream" => json!({
                "protocolVersion": 1,
                "requestId": request_id,
                "success": true,
                "data": {
                    "url": "https://example.com/video.mp4",
                    "referer": "https://example.com/",
                    "headers": {},
                    "height": 720,
                    "source": "fake-720p",
                    "expiresAt": "2026-10-18T03:00:00Z",
                    "expiryNote": "signed"
                }
            }),
            _ => json!({
                "protocolVersion": 1,
                "requestId": request_id,
                "success": false,
                "error": {"code": "UNSUPPORTED", "message": "unknown action", "retryable": false}
            }),
        };

        let _ = writeln!(stdout, "{}", response);
        let _ = stdout.flush();
    }
}

fn info() -> Value {
    json!({
        "id": "fake",
        "name": "Fake Provider",
        "pluginVersion": "1.0.0",
        "protocolVersion": 1,
        "baseUrl": "https://fake.example",
        "language": "English",
        "description": "Fake provider for tests"
    })
}

fn anime(query: &str) -> Value {
    json!({
        "providerId": "fake",
        "title": format!("Anime {}", query),
        "url": "https://fake.example/anime/1",
        "thumbnailUrl": "https://fake.example/thumb.jpg",
        "altTitles": ["Alt"],
        "year": 2026
    })
}

fn episode() -> Value {
    json!({
        "providerId": "fake",
        "number": 1,
        "title": "Episode 1",
        "url": "https://fake.example/episode/1"
    })
}
