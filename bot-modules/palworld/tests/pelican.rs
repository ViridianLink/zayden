//! Unit coverage for the Pelican transport's `modified_at` parsing - the field
//! `refresh_shared_if_stale` compares against the local save's mtime. No live
//! panel is involved; this pins the timestamp-format contract.

use palworld::transport::{Pelican, parse_modified_at};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn parses_iso8601_utc_to_unix_seconds() {
    // 2026-07-14T12:00:00Z == 1_784_030_400 seconds since the epoch.
    assert_eq!(parse_modified_at("2026-07-14T12:00:00Z").unwrap(), 1_784_030_400);
}

#[test]
fn parses_offset_timestamp() {
    // A +01:00 offset is one hour earlier in UTC than the same wall-clock Z.
    let z = parse_modified_at("2026-07-14T12:00:00Z").unwrap();
    let offset = parse_modified_at("2026-07-14T13:00:00+01:00").unwrap();
    assert_eq!(z, offset);
}

#[test]
fn rejects_garbage() {
    assert!(parse_modified_at("not a timestamp").is_err());
}

/// A panel fault only ever reaches the operator through the log line, so the
/// body it answered with has to survive into the error - a bare "HTTP 500"
/// cannot distinguish a rejected key from a daemon that is down.
#[tokio::test]
async fn panel_error_body_reaches_the_error() {
    const BODY: &str = r#"{"errors":[{"code":"DaemonConnectionException","detail":"the daemon said no"}]}"#;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        // Drain one read of the request line and headers; the body is empty.
        let mut buf = [0_u8; 1024];
        let _read = socket.read(&mut buf).await.unwrap();
        socket
            .write_all(
                format!(
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: \
                     application/json\r\nContent-Length: {}\r\n\r\n{BODY}",
                    BODY.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        socket.flush().await.unwrap();
    });

    let pelican = Pelican::new(
        reqwest::Client::new(),
        base_url,
        "key".to_string(),
        "server".to_string(),
        "/Pal/Saved".to_string(),
    );

    let err = pelican.level_modified().await.unwrap_err().to_string();
    server.await.unwrap();

    assert!(err.contains("the daemon said no"), "detail dropped from: {err}");
    assert!(err.contains("500"), "status dropped from: {err}");
    assert!(err.contains("/Pal/Saved"), "directory dropped from: {err}");
}
