use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct Wrapped<T> {
    pub attributes: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListResponse<T> {
    pub data: Vec<Wrapped<T>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PanelUser {
    pub id: i32,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EggVariable {
    pub env_variable: String,
    #[serde(default)]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EggRelationships {
    #[serde(default)]
    pub variables: Option<ListResponse<EggVariable>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Egg {
    pub id: i32,
    pub name: String,
    pub docker_image: String,
    pub startup: String,
    #[serde(default)]
    pub relationships: Option<EggRelationships>,
}

impl Egg {
    #[must_use]
    pub fn default_env(&self) -> Vec<(String, String)> {
        self.relationships
            .as_ref()
            .and_then(|r| r.variables.as_ref())
            .map(|vars| {
                vars.data
                    .iter()
                    .map(|v| {
                        (
                            v.attributes.env_variable.clone(),
                            v.attributes.default_value.clone().unwrap_or_default(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Allocation {
    pub id: i32,
    pub ip: String,
    pub port: i32,
    #[serde(default)]
    pub alias: Option<String>,
}

impl Allocation {
    #[must_use]
    pub fn address(&self) -> String {
        let host = self.alias.as_deref().unwrap_or(&self.ip);
        format!("{host}:{}", self.port)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerRelationships {
    #[serde(default)]
    pub allocations: Option<ListResponse<Allocation>>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Container {
    #[serde(default)]
    pub installed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub id: i32,
    pub uuid: String,
    pub identifier: String,
    pub name: String,
    #[serde(default)]
    pub suspended: bool,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub allocation: Option<i32>,
    #[serde(default)]
    pub container: Option<Container>,
    #[serde(default)]
    pub relationships: Option<ServerRelationships>,
}

impl Server {
    #[must_use]
    pub fn is_installing(&self) -> bool {
        self.status.as_deref() == Some("installing")
            || self.container.is_some_and(|c| !c.installed)
    }

    #[must_use]
    pub fn install_failed(&self) -> bool {
        self.status.as_deref() == Some("install_failed")
    }

    #[must_use]
    pub fn primary_address(&self) -> Option<String> {
        let allocations = &self.relationships.as_ref()?.allocations.as_ref()?.data;

        allocations
            .iter()
            .find(|a| Some(a.attributes.id) == self.allocation)
            .or_else(|| allocations.first())
            .map(|a| a.attributes.address())
    }
}

#[derive(Debug, Serialize)]
pub struct CreateUser<'a> {
    pub email: &'a str,
    pub username: &'a str,
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub password: &'a str,
}

#[derive(Debug, Serialize)]
pub struct Limits {
    pub memory: i32,
    pub swap: i32,
    pub disk: i32,
    pub io: i32,
    pub cpu: i32,
}

#[derive(Debug, Serialize)]
pub struct FeatureLimits {
    pub databases: i32,
    pub allocations: i32,
    pub backups: i32,
}

#[derive(Debug, Serialize)]
pub struct Deploy {
    pub locations: Vec<i32>,
    pub dedicated_ip: bool,
    pub port_range: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateServer {
    pub name: String,
    pub user: i32,
    pub egg: i32,
    pub docker_image: String,
    pub startup: String,
    pub environment: BTreeMap<String, String>,
    pub limits: Limits,
    pub feature_limits: FeatureLimits,
    pub deploy: Deploy,
    pub start_on_completion: bool,
}
