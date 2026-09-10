use std::collections::BTreeMap;
use std::time::Duration;

use rand::seq::IndexedRandom;
use reqwest::{Client, Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{HostingError, Result};
use crate::pelican::model::{
    CreateServer,
    CreateUser,
    Deploy,
    Egg,
    FeatureLimits,
    Limits,
    ListResponse,
    PanelUser,
    Server,
    Wrapped,
};

const MAX_ERROR_BODY_CHARS: usize = 400;
const PASSWORD_LEN: usize = 24;
const CLAIM_CODE_LEN: usize = 6;

#[derive(Debug, Clone)]
pub struct PelicanApp {
    client: Client,
    base_url: String,
    api_key: String,
}

impl PelicanApp {
    #[must_use]
    pub fn new(client: Client, base_url: &str, api_key: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key: api_key.to_owned(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/application/{path}", self.base_url)
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&(impl Serialize + Sync)>,
    ) -> Result<reqwest::Response> {
        let mut req = self
            .client
            .request(method, self.url(path))
            .bearer_auth(&self.api_key)
            .header(reqwest::header::ACCEPT, "application/json");

        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(body) = body {
            req = req.json(body);
        }

        req.send().await.map_err(|e| {
            if e.is_timeout() || e.is_connect() {
                HostingError::PanelUnavailable
            } else {
                HostingError::Http(e)
            }
        })
    }

    async fn ok(resp: reqwest::Response, what: &str) -> Result<reqwest::Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }

        if status.is_server_error() {
            return Err(HostingError::PanelUnavailable);
        }

        let body = resp.text().await.unwrap_or_default();
        let detail: String =
            body.trim().chars().take(MAX_ERROR_BODY_CHARS).collect();

        Err(HostingError::panel(if detail.is_empty() {
            format!("{what}: HTTP {status}")
        } else {
            format!("{what}: HTTP {status}: {detail}")
        }))
    }

    async fn json<T: DeserializeOwned>(
        resp: reqwest::Response,
        what: &str,
    ) -> Result<T> {
        Self::ok(resp, what).await?.json().await.map_err(|e| {
            HostingError::panel(format!("{what}: unreadable response: {e}"))
        })
    }

    pub async fn egg(&self, egg_id: i32) -> Result<Egg> {
        let resp = self
            .send(
                Method::GET,
                &format!("eggs/{egg_id}"),
                &[("include", "variables")],
                None::<&()>,
            )
            .await?;

        let wrapped: Wrapped<Egg> = Self::json(resp, "fetch egg").await?;
        Ok(wrapped.attributes)
    }

    pub async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<PanelUser>> {
        let resp = self
            .send(Method::GET, "users", &[("filter[email]", email)], None::<&()>)
            .await?;

        let list: ListResponse<PanelUser> = Self::json(resp, "search users").await?;

        Ok(list
            .data
            .into_iter()
            .map(|w| w.attributes)
            .find(|u| u.email.eq_ignore_ascii_case(email)))
    }

    pub async fn create_user(
        &self,
        email: &str,
        username: &str,
        display_name: &str,
    ) -> Result<(PanelUser, String)> {
        let password = generate_password();

        let body = CreateUser {
            email,
            username,
            first_name: display_name,
            last_name: "Discord",
            password: &password,
        };

        let resp = self.send(Method::POST, "users", &[], Some(&body)).await?;
        let wrapped: Wrapped<PanelUser> = Self::json(resp, "create user").await?;

        Ok((wrapped.attributes, password))
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the panel's own server-creation payload; grouping \
                  these into a struct would only move the arity one call out"
    )]
    pub async fn create_server(
        &self,
        name: &str,
        user_id: i32,
        egg: &Egg,
        env: BTreeMap<String, String>,
        memory_mib: i32,
        cpu_percent: i32,
        disk_mib: i32,
        location_ids: &[i32],
    ) -> Result<Server> {
        let body = CreateServer {
            name: name.to_owned(),
            user: user_id,
            egg: egg.id,
            docker_image: egg.docker_image.clone(),
            startup: egg.startup.clone(),
            environment: env,
            limits: Limits {
                memory: memory_mib,
                swap: 0,
                disk: disk_mib,
                io: 500,
                cpu: cpu_percent,
            },
            feature_limits: FeatureLimits {
                databases: 0,
                allocations: 1,
                backups: 1,
            },
            deploy: Deploy {
                locations: location_ids.to_vec(),
                dedicated_ip: false,
                port_range: Vec::new(),
            },
            start_on_completion: true,
        };

        let resp = self.send(Method::POST, "servers", &[], Some(&body)).await?;
        let wrapped: Wrapped<Server> = Self::json(resp, "create server").await?;

        Ok(wrapped.attributes)
    }

    pub async fn server(&self, server_id: i32) -> Result<Server> {
        let resp = self
            .send(
                Method::GET,
                &format!("servers/{server_id}"),
                &[("include", "allocations")],
                None::<&()>,
            )
            .await?;

        let wrapped: Wrapped<Server> = Self::json(resp, "fetch server").await?;
        Ok(wrapped.attributes)
    }

    pub async fn await_install(
        &self,
        server_id: i32,
        interval: Duration,
        timeout: Duration,
    ) -> Result<Server> {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let server = self.server(server_id).await?;

            if server.install_failed() {
                return Err(HostingError::panel("the egg's install script failed"));
            }
            if !server.is_installing() {
                return Ok(server);
            }
            if tokio::time::Instant::now() + interval >= deadline {
                return Err(HostingError::InstallTimeout);
            }

            tokio::time::sleep(interval).await;
        }
    }

    pub async fn suspend(&self, server_id: i32) -> Result<()> {
        self.power(server_id, "suspend").await
    }

    pub async fn unsuspend(&self, server_id: i32) -> Result<()> {
        self.power(server_id, "unsuspend").await
    }

    async fn power(&self, server_id: i32, action: &str) -> Result<()> {
        let resp = self
            .send(
                Method::POST,
                &format!("servers/{server_id}/{action}"),
                &[],
                None::<&()>,
            )
            .await?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }

        Self::ok(resp, action).await?;
        Ok(())
    }

    pub async fn delete_server(&self, server_id: i32) -> Result<()> {
        let resp = self
            .send(Method::DELETE, &format!("servers/{server_id}"), &[], None::<&()>)
            .await?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }

        Self::ok(resp, "delete server").await?;
        Ok(())
    }

    pub async fn free_allocations(&self, node_id: i32) -> Result<i64> {
        #[derive(serde::Deserialize)]
        struct NodeAllocation {
            #[serde(default)]
            assigned: bool,
        }

        let resp = self
            .send(
                Method::GET,
                &format!("nodes/{node_id}/allocations"),
                &[("per_page", "500")],
                None::<&()>,
            )
            .await?;

        let list: ListResponse<NodeAllocation> =
            Self::json(resp, "list allocations").await?;

        let free = list.data.iter().filter(|a| !a.attributes.assigned).count();

        Ok(i64::try_from(free).unwrap_or(i64::MAX))
    }
}

const PASSWORD_ALPHABET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

const CLAIM_ALPHABET: &[u8] = b"BCDFGHJKMNPQRSTVWXYZ23456789";

fn sample(alphabet: &[u8], len: usize) -> String {
    let mut rng = rand::rng();

    (0..len)
        .filter_map(|_| alphabet.choose(&mut rng).copied().map(char::from))
        .collect()
}

fn generate_password() -> String {
    sample(PASSWORD_ALPHABET, PASSWORD_LEN)
}

#[must_use]
pub fn generate_claim_code() -> String {
    sample(CLAIM_ALPHABET, CLAIM_CODE_LEN)
}
