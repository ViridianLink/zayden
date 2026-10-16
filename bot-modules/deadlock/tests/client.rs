use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use deadlock::{DeadlockApi, DeadlockError};
use deadlock_api::{DeadlockApiError, DeadlockClient, Priority};
use sqlx::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const RANKS: &str = include_str!("fixtures/ranks.json");

type Requests = Arc<Mutex<Vec<String>>>;

struct Reply {
    status: u16,
    headers: Vec<(&'static str, &'static str)>,
    body: &'static str,
}

impl Reply {
    const fn ok(body: &'static str) -> Self {
        Self { status: 200, headers: Vec::new(), body }
    }

    fn render(&self) -> String {
        let mut extra = String::new();
        for (name, value) in &self.headers {
            let _ = write!(extra, "{name}: {value}\r\n");
        }

        format!(
            "HTTP/1.1 {} Mock\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n{extra}\r\n{}",
            self.status,
            self.body.len(),
            self.body
        )
    }
}

async fn read_head(socket: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];

    loop {
        let n = match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => return None,
            Ok(n) => n,
        };
        buf.extend_from_slice(chunk.get(..n)?);

        if buf.windows(4).any(|window| window == b"\r\n\r\n") {
            return Some(String::from_utf8_lossy(&buf).into_owned());
        }
    }
}

async fn spawn_mock(reply: Reply) -> Option<(String, Requests)> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let base = format!("http://{}", listener.local_addr().ok()?);
    let requests = Requests::default();
    let recorder = Arc::clone(&requests);
    let response = reply.render();

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };

            if let Some(head) = read_head(&mut socket).await
                && let Ok(mut recorded) = recorder.lock()
            {
                recorded.push(head);
            }

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    Some((base, requests))
}

fn api(pool: &PgPool, base: &str) -> deadlock_api::Result<DeadlockApi> {
    let client = DeadlockClient::builder().base_url(base).build()?;

    Ok(DeadlockApi::from_client(client, pool.clone()))
}

fn count(requests: &Requests) -> usize {
    requests.lock().map_or(0, |recorded| recorded.len())
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_repeated_call_is_served_from_memory_then_from_postgres(pool: PgPool) {
    let (base, requests) = spawn_mock(Reply::ok(RANKS)).await.unwrap();
    let api_one = api(&pool, &base).unwrap();

    let first = api_one.ranks(Priority::Interactive).await.unwrap();
    api_one.ranks(Priority::Interactive).await.unwrap();

    assert!(!first.stale);
    assert_eq!(first.value.len(), 12);
    assert_eq!(count(&requests), 1);

    let api_two = api(&pool, &base).unwrap();
    let again = api_two.ranks(Priority::Interactive).await.unwrap();

    assert!(!again.stale);
    assert_eq!(again.value.len(), 12);
    assert_eq!(count(&requests), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_expired_row_is_served_stale_when_the_api_is_busy(pool: PgPool) {
    let (base, _) = spawn_mock(Reply::ok(RANKS)).await.unwrap();
    api(&pool, &base).unwrap().ranks(Priority::Interactive).await.unwrap();

    sqlx::query!(
        "UPDATE deadlock_api_cache SET expires_at = now() - interval '1 hour'"
    )
    .execute(&pool)
    .await
    .unwrap();

    let (busy_base, busy) = spawn_mock(Reply {
        status: 503,
        headers: vec![("retry-after", "7")],
        body: "",
    })
    .await
    .unwrap();

    let stale =
        api(&pool, &busy_base).unwrap().ranks(Priority::Interactive).await.unwrap();

    assert!(stale.stale);
    assert_eq!(stale.value.len(), 12);
    assert_eq!(count(&busy), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_busy_api_without_a_cached_row_is_an_error(pool: PgPool) {
    let (base, _) = spawn_mock(Reply {
        status: 503,
        headers: vec![("retry-after", "7")],
        body: "",
    })
    .await
    .unwrap();

    let err =
        api(&pool, &base).unwrap().ranks(Priority::Interactive).await.unwrap_err();

    assert!(
        matches!(
            err,
            DeadlockError::Api(DeadlockApiError::Busy { retry_after })
                if retry_after == Duration::from_secs(7)
        ),
        "{err:?}"
    );
}
