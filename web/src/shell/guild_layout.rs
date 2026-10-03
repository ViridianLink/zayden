use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::{Slot, layout, path_param};
use topcoat::view::{View, view};

use super::chrome::frame;
use super::sidebar::guild_sidebar;
use crate::guild::dto::GuildInfo;
use crate::guild::list_manageable_guilds;

path_param!(pub guild_id);

/// The guilds for the server switcher. A signed-out visitor is sent to the
/// login page; any other failure leaves the switcher's menu empty.
async fn switcher_guilds(cx: &Cx) -> Result<Vec<GuildInfo>> {
    match list_manageable_guilds(cx).await {
        Ok(guilds) => Ok(guilds),
        Err(e) => {
            e.redirect_unauthenticated()?;
            Ok(Vec::new())
        },
    }
}

/// Frames every page under `/guild/{guild_id}` with the guild sidebar.
#[layout("/guild/{guild_id}")]
pub(super) async fn guild_shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let guilds = switcher_guilds(cx).await?;

    Ok(view! {
        frame(
            sidebar: view! { guild_sidebar(guild_id: guild_id, guilds: &guilds) }.into(),
            (slot)
        )
    })
}
