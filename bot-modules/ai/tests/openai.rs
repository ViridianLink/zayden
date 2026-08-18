use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use ai::chat::{Message, Role};
use ai::error::AiError;
use ai::openai::AiClient;
use async_openai::error::OpenAIError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn drain_request(socket: &mut TcpStream) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];

    loop {
        let n = match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        let Some(received) = chunk.get(..n) else { return };
        buf.extend_from_slice(received);

        let Some(header_end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };

        let Some(header_bytes) = buf.get(..header_end) else { return };
        let headers = String::from_utf8_lossy(header_bytes);
        let content_length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|v| v.trim().parse().unwrap_or(0))
            })
            .unwrap_or(0);

        if buf.len().saturating_sub(header_end + 4) >= content_length {
            break;
        }
    }
}

async fn spawn_mock_server(
    status_line: &'static str,
    body: &'static str,
) -> Option<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };

            drain_request(&mut socket).await;

            let response = format!(
                "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    Some(format!("http://{addr}"))
}

/// Answers each request with the next response in `responses`, repeating the
/// last one once the list runs out. Returns the base URL and the counter of
/// requests actually served.
async fn spawn_sequenced_mock_server(
    responses: Vec<(&'static str, &'static str)>,
) -> Option<(String, Arc<AtomicUsize>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;
    let served = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&served);

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };

            drain_request(&mut socket).await;

            let index = counter.fetch_add(1, Ordering::SeqCst);
            let Some((status_line, body)) =
                responses.get(index).or_else(|| responses.last())
            else {
                break;
            };

            let response = format!(
                "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    Some((format!("http://{addr}"), served))
}

/// Accepts the connection, reads the request, then never answers and never
/// hangs up — the silent upstream an outbound timeout has to defend against.
async fn spawn_black_hole_server() -> Option<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };

            tokio::spawn(async move {
                drain_request(&mut socket).await;
                // Hold the connection open, silently, forever.
                std::future::pending::<()>().await;
            });
        }
    });

    Some(format!("http://{addr}"))
}

#[tokio::test]
async fn chat_parses_a_well_formed_openrouter_completion() {
    let body = r#"{
        "id": "chatcmpl-test123",
        "object": "chat.completion",
        "created": 1700000000,
        "model": "test-model",
        "choices": [
            {
                "index": 0,
                "message": { "role": "assistant", "content": "Hello there!" },
                "finish_reason": "stop"
            }
        ],
        "usage": null
    }"#;
    let base_url =
        spawn_mock_server("HTTP/1.1 200 OK", body).await.expect("start mock server");

    let client =
        AiClient::new("test-key", &base_url, "test-model").expect("build client");
    let content = client
        .chat(vec![Message::new(Role::User, "hi")], 16)
        .await
        .expect("well-formed response should parse");

    assert_eq!(content, "Hello there!");
}

#[tokio::test]
async fn chat_surfaces_rate_limit_errors_instead_of_panicking() {
    let body = r#"{
        "error": {
            "message": "Rate limit exceeded",
            "type": "rate_limit_error",
            "code": "rate_limit_exceeded"
        }
    }"#;
    let base_url = spawn_mock_server("HTTP/1.1 429 Too Many Requests", body)
        .await
        .expect("start mock server");

    let client =
        AiClient::new("test-key", &base_url, "test-model").expect("build client");
    let err = client
        .chat(vec![Message::new(Role::User, "hi")], 16)
        .await
        .expect_err("a 429 response must surface as an error, not a panic");

    assert!(
        matches!(err, AiError::OpenAI(OpenAIError::ApiError(_))),
        "expected AiError::OpenAI(ApiError), got {err:?}"
    );
    assert!(err.to_string().contains("Rate limit exceeded"));
}

/// Regression test for the missing outbound timeout (audit `ai` #1).
///
/// Runs on virtual time, so the client's real budget costs the suite nothing:
/// the client's own timer has to fire before the backstop below. Without a
/// timeout on the client there is no timer at all, the backstop wins, and this
/// fails — which is exactly the production symptom, a chat future that never
/// resolves and wedges the command task holding it. The backstop allows for
/// every attempt the retry policy makes, each with its own timeout.
#[tokio::test(start_paused = true)]
async fn chat_gives_up_on_a_hung_upstream_instead_of_waiting_forever() {
    let base_url = spawn_black_hole_server().await.expect("start mock server");

    let client =
        AiClient::new("test-key", &base_url, "test-model").expect("build client");

    let result = tokio::time::timeout(
        Duration::from_secs(180),
        client.chat(vec![Message::new(Role::User, "hi")], 16),
    )
    .await
    .expect(
        "the client must give up on its own; it was still waiting on a silent \
         upstream after 60s",
    );

    let err = result.expect_err("a hung upstream must surface as an error");

    assert!(
        matches!(&err, AiError::OpenAI(OpenAIError::Reqwest(e)) if e.is_timeout()),
        "expected a reqwest timeout, got {err:?}"
    );
}

/// The body `OpenRouter` actually returns when the upstream model gives up: a
/// `200 OK`, a long stretch of keep-alive padding, then an error object. The
/// padding means the request never trips our own timeout, and the `200` means
/// the client library never reaches its error path — so before this was
/// handled it surfaced as `JSONDeserialize("missing field `id`")`.
const ABORTED_UPSTREAM_BODY: &str = "\n         \n\n         \n\n         \n\n         \n{\"error\":{\"message\":\"The operation was aborted\",\"code\":504}}";

const COMPLETION_BODY: &str = r#"{
    "id": "chatcmpl-test123",
    "object": "chat.completion",
    "created": 1700000000,
    "model": "test-model",
    "choices": [
        {
            "index": 0,
            "message": { "role": "assistant", "content": "Hello there!" },
            "finish_reason": "stop"
        }
    ],
    "usage": null
}"#;

#[tokio::test]
async fn a_padded_gateway_error_reads_as_a_provider_error() {
    // Every attempt fails, so the caller sees the classified error rather than
    // an opaque deserialization failure.
    let (base_url, _served) = spawn_sequenced_mock_server(vec![(
        "HTTP/1.1 200 OK",
        ABORTED_UPSTREAM_BODY,
    )])
    .await
    .expect("start mock server");

    let client =
        AiClient::new("test-key", &base_url, "test-model").expect("build client");
    let err = client
        .chat(vec![Message::new(Role::User, "hi")], 16)
        .await
        .expect_err("a gateway error body must surface as an error");

    assert!(
        matches!(&err, AiError::Provider { code: Some(504), message } if message == "The operation was aborted"),
        "expected AiError::Provider(504), got {err:?}"
    );
}

#[tokio::test]
async fn an_aborted_upstream_is_retried() {
    let (base_url, served) = spawn_sequenced_mock_server(vec![
        ("HTTP/1.1 200 OK", ABORTED_UPSTREAM_BODY),
        ("HTTP/1.1 200 OK", COMPLETION_BODY),
    ])
    .await
    .expect("start mock server");

    let client =
        AiClient::new("test-key", &base_url, "test-model").expect("build client");
    let content = client
        .chat(vec![Message::new(Role::User, "hi")], 16)
        .await
        .expect("the second attempt should succeed");

    assert_eq!(content, "Hello there!");
    assert_eq!(served.load(Ordering::SeqCst), 2, "expected exactly one retry");
}

#[tokio::test]
async fn a_rejected_request_is_not_retried() {
    // A 400 is the model refusing the request as written; asking again only
    // burns the user's time and the provider's quota.
    let body = r#"{"error":{"message":"Invalid model","code":400}}"#;
    let (base_url, served) =
        spawn_sequenced_mock_server(vec![("HTTP/1.1 400 Bad Request", body)])
            .await
            .expect("start mock server");

    let client =
        AiClient::new("test-key", &base_url, "test-model").expect("build client");
    let err = client
        .chat(vec![Message::new(Role::User, "hi")], 16)
        .await
        .expect_err("a 400 response must surface as an error");

    assert!(!err.is_transient(), "a 400 must not be treated as transient");
    assert_eq!(served.load(Ordering::SeqCst), 1, "a 400 must not be retried");
}
