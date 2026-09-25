//! Reading a page the reader picked from autocomplete, which names it by id.
//!
//! Wiki.js only serves page source over GraphQL to keys with `manage:pages`.
//! A key without it must still reach the page through the source view, the
//! same way a lookup by path already does.

use reqwest::Client;
use ticket::wiki::{self, WikiConfig, WikiError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use zayden_app::config::{FaqSettingsRow, SettingsRow};

const FORBIDDEN: &str = r#"{"data":null,"errors":[{"message":"You are not authorized to view this page.","extensions":{"exception":{"code":6013}}}]}"#;
const LISTED: &str = r#"{"data":{"pages":{"list":[{"id":7,"path":"other","title":"Other","description":"","isPublished":true},{"id":120,"path":"guides/setup","title":"Setup","description":"","isPublished":true}]}}}"#;
const SOURCE_VIEW: &str = "<html><head><title>Setup | Wiki</title></head><body><code v-pre># Setup\nInstall it.</code></body></html>";

async fn read_request(socket: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 4096];

    loop {
        let n = socket.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(chunk.get(..n)?);

        let text = String::from_utf8_lossy(&buf);
        let Some((head, body)) = text.split_once("\r\n\r\n") else { continue };

        let length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);

        if body.len() >= length {
            return Some(text.into_owned());
        }
    }

    Some(String::from_utf8_lossy(&buf).into_owned())
}

fn respond(request: &str) -> (&'static str, &'static str, &'static str) {
    if request.starts_with("GET /s/en/guides/setup ") {
        return ("HTTP/1.1 200 OK", "text/html", SOURCE_VIEW);
    }

    if request.contains("singleByPath") || request.contains("single(") {
        return ("HTTP/1.1 200 OK", "application/json", FORBIDDEN);
    }

    if request.contains("list(") {
        return ("HTTP/1.1 200 OK", "application/json", LISTED);
    }

    ("HTTP/1.1 404 Not Found", "text/plain", "")
}

async fn spawn_wiki() -> Option<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let request = read_request(&mut socket).await.unwrap_or_default();
            let (status, content_type, body) = respond(&request);
            let response = format!(
                "{status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    Some(format!("http://{addr}"))
}

fn config(url: &str) -> Option<WikiConfig> {
    let mut row = FaqSettingsRow::empty(1);
    row.enabled = true;
    row.wiki_url = Some(url.to_owned());
    row.wiki_api_key = Some(String::from("read-only-key"));

    WikiConfig::from_settings(&row).ok().flatten()
}

#[tokio::test]
async fn a_page_picked_by_id_falls_back_to_the_source_view() {
    let url = spawn_wiki().await.expect("start mock wiki");
    let config = config(&url).expect("config builds");

    let page = wiki::page_by_id(&Client::new(), &config, 120)
        .await
        .expect("a forbidden GraphQL read must fall back to the source view");

    assert_eq!(page.path, "guides/setup");
    assert_eq!(page.content, "# Setup\nInstall it.");
}

#[tokio::test]
async fn an_unlisted_id_reads_as_not_found() {
    let url = spawn_wiki().await.expect("start mock wiki");
    let config = config(&url).expect("config builds");

    let err = wiki::page_by_id(&Client::new(), &config, 999)
        .await
        .expect_err("an id the wiki does not list cannot be read");

    assert!(
        matches!(&err, WikiError::PageNotFound(id) if id == "999"),
        "expected PageNotFound(999), got {err:?}"
    );
}
