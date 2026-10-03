use topcoat::context::Cx;
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

use super::error::AdminError;
use crate::auth::{WebRole, discord_client, require_role};
use crate::components::guild_grid::GuildCard;

const GUILD_PAGE_LIMIT: u16 = 200;

/// Every server the bot is in, sorted by lowercased name.
pub async fn list_bot_guilds(cx: &Cx) -> Result<Vec<GuildCard>, AdminError> {
    require_role(cx, WebRole::Operator).await?;

    let http = discord_client(cx)?;

    let mut guilds: Vec<GuildCard> = Vec::new();
    let mut after: Option<Id<GuildMarker>> = None;

    loop {
        let mut request = http.current_user_guilds().limit(GUILD_PAGE_LIMIT);
        if let Some(id) = after {
            request = request.after(id);
        }

        let page = request.await?.model().await?;

        let exhausted = page.len() < usize::from(GUILD_PAGE_LIMIT);
        after = page.last().map(|g| g.id);

        guilds.extend(page.iter().map(GuildCard::from));

        if exhausted {
            break;
        }
    }

    guilds.sort_by_key(|g| g.name.to_lowercase());

    Ok(guilds)
}

/// A typed server id: trimmed ASCII digits that fit a `u64` and are not `0`.
#[must_use]
pub fn parse_guild_id(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();

    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    trimmed.parse::<u64>().ok().filter(|id| *id != 0)
}
