use std::collections::HashMap;

use serenity::all::ResolvedValue;
use zayden_core::{InvocationCtx, required_option};

use super::respond;
use crate::client::PalworldClient;
use crate::embeds;
use crate::error::{PalworldError, Result};
use crate::model::PassiveSkill;

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    client: &PalworldClient,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let query: &str =
        required_option(&mut options, "name").map_err(PalworldError::from)?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let passives = client.passives().await?;
    let key = find_passive_key(&passives, query).ok_or_else(|| {
        PalworldError::NotFound {
            entity: "passive skill",
            query: query.to_string(),
        }
    })?;

    let passive = client.passive(&key).await?;
    respond(cx, embeds::passive_component(&passive)).await
}

fn find_passive_key(passives: &[PassiveSkill], query: &str) -> Option<String> {
    if let Some(skill) = passives.iter().find(|p| p.key == query) {
        return Some(skill.key.clone());
    }
    if let Some(skill) = passives.iter().find(|p| p.name.eq_ignore_ascii_case(query))
    {
        return Some(skill.key.clone());
    }
    let query_lower = query.to_lowercase();
    passives
        .iter()
        .find(|p| p.name.to_lowercase().contains(&query_lower))
        .map(|p| p.key.clone())
}
