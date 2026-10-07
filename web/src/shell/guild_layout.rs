use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::{Slot, layout, path_param};
use topcoat::view::{View, view};

use super::chrome::frame;
use crate::guild::dto::GuildInfo;
use crate::guild::list_manageable_guilds;

path_param!(pub guild_id);

async fn switcher_guilds(cx: &Cx) -> Result<Vec<GuildInfo>> {
    match list_manageable_guilds(cx).await {
        Ok(guilds) => Ok(guilds),
        Err(e) => {
            e.redirect_unauthenticated()?;
            Ok(Vec::new())
        },
    }
}

#[layout("/guild/{guild_id}")]
pub(super) async fn guild_shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let guilds = switcher_guilds(cx).await?;

    Ok(view! { frame(guild_id: Some(guild_id), guilds: &guilds, (slot)) })
}
