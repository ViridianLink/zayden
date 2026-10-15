use std::sync::{Arc, Mutex};

use accounts::AccountsError;
use accounts::steam::SteamId64;
use accounts::steam::openid::{
    OPENID_NS,
    STEAM_OPENID_ENDPOINT,
    authorize_url,
    validate_assertion,
    verify,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const RETURN_TO: &str = "https://dash.test/account/link/steam/callback?n=abc";
const CLAIMED_ID: &str = "https://steamcommunity.com/openid/id/76561197960287930";

type Params = Vec<(String, String)>;

fn assertion() -> Params {
    [
        ("openid.ns", OPENID_NS),
        ("openid.mode", "id_res"),
        ("openid.op_endpoint", STEAM_OPENID_ENDPOINT),
        ("openid.claimed_id", CLAIMED_ID),
        ("openid.identity", CLAIMED_ID),
        ("openid.return_to", RETURN_TO),
        ("openid.response_nonce", "2026-01-01T00:00:00Zabc"),
        ("openid.assoc_handle", "1234567890"),
        (
            "openid.signed",
            "signed,op_endpoint,claimed_id,identity,return_to,response_nonce,assoc_handle",
        ),
        ("openid.sig", "c2lnbmF0dXJl"),
        ("n", "abc"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect()
}

fn with(mut params: Params, key: &str, value: &str) -> Params {
    for (name, current) in &mut params {
        if name == key {
            value.clone_into(current);
        }
    }
    params
}

fn assert_invalid(params: &Params, field: &'static str) {
    let result = validate_assertion(params, RETURN_TO);
    assert!(
        matches!(&result, Err(AccountsError::InvalidAssertion(f)) if *f == field),
        "{field}: {result:?}"
    );
}

#[test]
fn a_valid_assertion_yields_the_steam_id() {
    let id = validate_assertion(&assertion(), RETURN_TO).unwrap();

    assert_eq!(id, SteamId64::new(76_561_197_960_287_930).unwrap());
}

#[test]
fn each_tampered_field_is_rejected() {
    assert_invalid(&with(assertion(), "openid.ns", "http://example.com"), "ns");
    assert_invalid(&with(assertion(), "openid.mode", "cancel"), "mode");
    assert_invalid(
        &with(assertion(), "openid.op_endpoint", "https://evil.test/openid/login"),
        "op_endpoint",
    );
    assert_invalid(
        &with(assertion(), "openid.return_to", "https://dash.test/other?n=abc"),
        "return_to",
    );
    assert_invalid(
        &with(
            assertion(),
            "openid.identity",
            "https://steamcommunity.com/openid/id/76561197960287931",
        ),
        "identity",
    );
    assert_invalid(
        &with(
            with(assertion(), "openid.claimed_id", "https://evil.test/id/1"),
            "openid.identity",
            "https://evil.test/id/1",
        ),
        "claimed_id",
    );
    assert_invalid(
        &with(
            assertion(),
            "openid.signed",
            "signed,op_endpoint,claimed_id,return_to,response_nonce,assoc_handle",
        ),
        "signed",
    );
}

#[test]
fn a_missing_field_is_rejected() {
    let mut params = assertion();
    params.retain(|(name, _)| name != "openid.signed");

    assert_invalid(&params, "signed");
}

#[test]
fn the_authorize_url_asks_steam_to_pick_the_identity() {
    let url = authorize_url("https://dash.test", RETURN_TO).unwrap();
    let url = url.as_str();

    assert!(url.starts_with("https://steamcommunity.com/openid/login?"), "{url}");
    assert!(url.contains("openid.mode=checkid_setup"), "{url}");
    assert!(
        url.contains(
            "openid.return_to=https%3A%2F%2Fdash.test%2Faccount%2Flink%2Fsteam%2Fcallback%3Fn%3Dabc"
        ),
        "{url}"
    );
    assert!(url.contains("openid.realm=https%3A%2F%2Fdash.test&"), "{url}");
}

/// Reads one whole request off the socket and hands back its body.
async fn drain_request(socket: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];

    loop {
        let n = match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => return None,
            Ok(n) => n,
        };
        let received = chunk.get(..n)?;
        buf.extend_from_slice(received);

        let Some(header_end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };

        let header_bytes = buf.get(..header_end)?;
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
            let body = buf.get(header_end + 4..)?;
            return Some(String::from_utf8_lossy(body).into_owned());
        }
    }
}

/// Answers with `body` and keeps the request bodies it was asked with.
async fn spawn_recording_mock_server(
    body: &'static str,
) -> Option<(String, Arc<Mutex<Vec<String>>>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&requests);

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };

            if let Some(request) = drain_request(&mut socket).await
                && let Ok(mut recorded) = recorder.lock()
            {
                recorded.push(request);
            }

            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    Some((format!("http://{addr}"), requests))
}

#[tokio::test]
async fn steam_confirming_the_assertion_returns_the_id() {
    let (endpoint, requests) = spawn_recording_mock_server(
        "ns:http://specs.openid.net/auth/2.0\nis_valid:true\n",
    )
    .await
    .unwrap();

    let id = verify(&reqwest::Client::new(), &endpoint, &assertion(), RETURN_TO)
        .await
        .unwrap();

    assert_eq!(id.get(), 76_561_197_960_287_930);
    let sent = requests.lock().unwrap().clone();
    let [body] = sent.as_slice() else {
        panic!("expected one request, got {}", sent.len());
    };
    assert!(body.contains("openid.mode=check_authentication"), "{body}");
    assert!(!body.contains("id_res"), "{body}");
    assert!(body.contains("openid.sig=c2lnbmF0dXJl"), "{body}");
    assert!(!body.contains("&n=") && !body.starts_with("n="), "{body}");
}

#[tokio::test]
async fn steam_denying_the_assertion_is_a_rejection() {
    let (endpoint, _) = spawn_recording_mock_server(
        "ns:http://specs.openid.net/auth/2.0\nis_valid:false\n",
    )
    .await
    .unwrap();

    let err = verify(&reqwest::Client::new(), &endpoint, &assertion(), RETURN_TO)
        .await
        .unwrap_err();

    assert!(matches!(err, AccountsError::VerificationRejected), "{err:?}");
}

#[tokio::test]
async fn an_invalid_assertion_never_reaches_steam() {
    let (endpoint, requests) =
        spawn_recording_mock_server("is_valid:true\n").await.unwrap();
    let params = with(assertion(), "openid.mode", "cancel");

    let err = verify(&reqwest::Client::new(), &endpoint, &params, RETURN_TO)
        .await
        .unwrap_err();

    assert!(matches!(err, AccountsError::InvalidAssertion("mode")), "{err:?}");
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn a_repeated_openid_key_is_rejected() {
    let mut params = vec![
        (
            "openid.claimed_id".to_owned(),
            "https://steamcommunity.com/openid/id/76561197960287931".to_owned(),
        ),
        (
            "openid.identity".to_owned(),
            "https://steamcommunity.com/openid/id/76561197960287931".to_owned(),
        ),
    ];
    params.extend(assertion());

    assert_invalid(&params, "duplicate");
}
