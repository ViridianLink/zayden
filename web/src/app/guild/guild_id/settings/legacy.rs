use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::{redirect_permanent, see_other};
use topcoat::router::request::uri;
use topcoat::router::{page, path_param, segment};
use topcoat::view::{View, view};

use crate::app::guild::guild_id::GuildId;
use crate::flash::{self, FlashKind};
use crate::nav;

path_param!(section);

segment!(rename = "settings");

pub const NOT_SAVED: &str = "This page was updated and your change was not \
                             saved. Please try again.";

fn target(cx: &Cx) -> String {
    let guild_id: &str = path_param::<GuildId>(cx);
    let slug: &str = path_param::<Section>(cx);
    nav::legacy_href(guild_id, slug)
}

#[page("./{section}")]
async fn moved(cx: &Cx) -> Result<impl View> {
    let target = target(cx);
    let location = uri(cx)
        .query()
        .map_or_else(|| target.clone(), |query| format!("{target}?{query}"));
    Err::<(), _>(redirect_permanent(location))?;
    Ok(view! { "" })
}

#[page(POST "./{section}")]
async fn moved_post(cx: &Cx) -> Result<impl View> {
    flash::set(cx, FlashKind::Error, NOT_SAVED)?;
    Err::<(), _>(see_other(target(cx)))?;
    Ok(view! { "" })
}
