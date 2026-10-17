use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use deadlock::DeadlockApi;
use deadlock_api::{DeadlockClient, Limiter};
use sqlx::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub(crate) type Requests = Arc<Mutex<Vec<String>>>;

type Routes = Arc<Mutex<HashMap<String, Reply>>>;

pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) headers: Vec<(&'static str, &'static str)>,
    pub(crate) body: String,
}

impl Reply {
    pub(crate) fn ok(body: &str) -> Self {
        Self { status: 200, headers: Vec::new(), body: body.to_owned() }
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

pub(crate) struct Mock {
    pub(crate) base: String,
    pub(crate) requests: Requests,
    routes: Routes,
}

impl Mock {
    pub(crate) fn serve(&self, path: &str, reply: Reply) {
        if let Ok(mut routes) = self.routes.lock() {
            routes.insert(path.to_owned(), reply);
        }
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

fn path_of(head: &str) -> &str {
    head.split_whitespace()
        .nth(1)
        .and_then(|target| target.split('?').next())
        .unwrap_or_default()
}

pub(crate) async fn spawn_mock(fallback: Reply) -> Option<Mock> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let base = format!("http://{}", listener.local_addr().ok()?);
    let requests = Requests::default();
    let routes = Routes::default();
    let recorder = Arc::clone(&requests);
    let table = Arc::clone(&routes);
    let fallback = fallback.render();

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };

            let head = read_head(&mut socket).await;
            let path = head.as_deref().map_or("", path_of).to_owned();

            if let Some(head) = head
                && let Ok(mut recorded) = recorder.lock()
            {
                recorded.push(head);
            }

            let response = table
                .lock()
                .ok()
                .and_then(|table| table.get(&path).map(Reply::render))
                .unwrap_or_else(|| fallback.clone());

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    Some(Mock { base, requests, routes })
}

/// No bucket configs, so the limiter grants every request at once.
pub(crate) fn api(pool: &PgPool, base: &str) -> deadlock_api::Result<DeadlockApi> {
    let client = DeadlockClient::builder()
        .base_url(base)
        .limiter(Limiter::with_configs([], Duration::ZERO))
        .build()?;

    Ok(DeadlockApi::from_client(client, pool.clone()))
}
