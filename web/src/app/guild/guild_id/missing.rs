use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::not_found;
use topcoat::router::{StatusCode, page, path_param, segment};
use topcoat::view::{View, view};

use super::GuildId;
use crate::components::error_panel::{ErrorAction, error_panel};
use crate::guild::list_manageable_guilds;
use crate::nav::path_segment;

segment!(kind = Group);

const MESSAGE: &str = "There is no page at this address in this server. The link may \
                       have a typo, or the page may have moved.";

#[page("./{*rest}")]
pub(super) async fn guild_not_found(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let guilds = match list_manageable_guilds(cx).await {
        Ok(guilds) => guilds,
        Err(error) => {
            error.redirect_unauthenticated()?;
            Vec::new()
        },
    };
    let Some(guild) = guilds.iter().find(|guild| guild.id == guild_id) else {
        return Err(not_found().into());
    };
    let actions = [
        ErrorAction::new(
            &format!("Back to {}", guild.name),
            &format!("/guild/{}", path_segment(guild_id)),
        ),
        ErrorAction::new("Back to servers", "/guilds"),
    ];

    Ok(view! {
        (StatusCode::NOT_FOUND)
        <div class="page">
            error_panel(title: "Page not found", message: MESSAGE, actions: &actions)
        </div>
    })
}
