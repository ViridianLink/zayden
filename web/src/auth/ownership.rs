use std::collections::HashSet;

use topcoat::context::Cx;
use twilight_http::Client;
use twilight_model::id::Id;

use super::context::discord_client;
use super::discord::{fetch_guild_channels, fetch_guild_roles};
use super::error::{AuthError, ForeignIdError};

pub fn first_foreign<'a>(
    known: impl IntoIterator<Item = &'a str>,
    submitted: &[i64],
) -> Option<i64> {
    let known: HashSet<u64> =
        known.into_iter().filter_map(|id| id.parse().ok()).collect();

    submitted
        .iter()
        .copied()
        .find(|&id| u64::try_from(id).map_or(true, |id| !known.contains(&id)))
}

#[derive(Debug, Default)]
pub struct GuildIds {
    channels: Vec<i64>,
    roles: Vec<i64>,
    threads: Vec<i64>,
}

impl GuildIds {
    #[must_use]
    pub fn channel(mut self, id: Option<i64>) -> Self {
        self.channels.extend(id);
        self
    }

    #[must_use]
    pub fn role(mut self, id: Option<i64>) -> Self {
        self.roles.extend(id);
        self
    }

    #[must_use]
    pub fn thread(mut self, id: Option<i64>) -> Self {
        self.threads.extend(id);
        self
    }

    pub async fn ensure_in(self, cx: &Cx, guild_id: i64) -> Result<(), AuthError> {
        if self.channels.is_empty()
            && self.roles.is_empty()
            && self.threads.is_empty()
        {
            return Ok(());
        }

        let http = discord_client(cx)?;
        let guild_id = guild_id.cast_unsigned();

        let (channels, roles, threads) = tokio::join!(
            ensure_channels(http, guild_id, &self.channels),
            ensure_roles(http, guild_id, &self.roles),
            ensure_threads(http, guild_id, &self.threads),
        );

        channels.and(roles).and(threads)
    }
}

async fn ensure_channels(
    http: &Client,
    guild_id: u64,
    submitted: &[i64],
) -> Result<(), AuthError> {
    if submitted.is_empty() {
        return Ok(());
    }

    let channels = fetch_guild_channels(http, guild_id).await?;

    match first_foreign(channels.iter().map(|c| c.id.as_str()), submitted) {
        Some(_) => Err(ForeignIdError::Channel.into()),
        None => Ok(()),
    }
}

async fn ensure_roles(
    http: &Client,
    guild_id: u64,
    submitted: &[i64],
) -> Result<(), AuthError> {
    if submitted.is_empty() {
        return Ok(());
    }

    let roles = fetch_guild_roles(http, guild_id).await?;

    match first_foreign(roles.iter().map(|r| r.id.as_str()), submitted) {
        Some(_) => Err(ForeignIdError::Role.into()),
        None => Ok(()),
    }
}

async fn ensure_threads(
    http: &Client,
    guild_id: u64,
    submitted: &[i64],
) -> Result<(), AuthError> {
    for &id in submitted {
        let Some(id) = Id::new_checked(id.cast_unsigned()) else {
            return Err(ForeignIdError::Channel.into());
        };

        let thread = http.channel(id).await?.model().await?;

        if thread.guild_id.map(Id::get) != Some(guild_id) {
            return Err(ForeignIdError::Channel.into());
        }
    }

    Ok(())
}
