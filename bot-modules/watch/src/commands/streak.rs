use std::collections::HashMap;
use std::hash::BuildHasher;

use jellyfin::JellyfinError;
use jellyfin::embeds::COLOUR;
use jellyfin::identity::JellyfinLinkRow;
use jellyfin::stats::{heatmap, streak};
use serenity::all::{
    CreateAttachment,
    CreateEmbed,
    EditInteractionResponse,
    ResolvedValue,
    User,
};
use zayden_core::{InvocationCtx, optional_option};

use crate::error::Result;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let target: Option<&User> = optional_option(&mut options, "user");
    let target = target.unwrap_or(&cx.interaction.user);
    let is_self = target.id == cx.interaction.user.id;

    cx.interaction.defer(&cx.ctx.http).await?;

    let link =
        JellyfinLinkRow::get(&cx.app.db, target.id).await?.ok_or_else(|| {
            if is_self {
                JellyfinError::NotLinked
            } else {
                JellyfinError::TargetNotLinked(target.display_name().to_owned())
            }
        })?;

    // Streaks default to private, so viewing someone else's needs their opt-in.
    if !is_self && !link.streak_public {
        return Err(
            JellyfinError::StreakPrivate(target.display_name().to_owned()).into()
        );
    }

    let stats = streak::load(&cx.app.db, &link.jellyfin_user_id).await?;

    let title = format!("{}'s watch streak", target.display_name());
    let embed = CreateEmbed::new()
        .title(title.clone())
        .colour(COLOUR)
        .field("Current streak", format!("{} days", stats.current), true)
        .field("Longest streak", format!("{} days", stats.longest), true)
        .field("Days watched", stats.total_days.to_string(), true)
        .field("Total watched", format_hours(stats.total_seconds), true);

    // The heatmap is a nicety; a font problem must not cost the user the answer.
    let response = match heatmap::render(&stats.days, &title).await {
        Some(png) => EditInteractionResponse::new()
            .embed(embed.attachment("streak.png"))
            .new_attachment(CreateAttachment::bytes(png, "streak.png")),
        None => EditInteractionResponse::new().embed(embed),
    };

    cx.interaction.edit_response(&cx.ctx.http, response).await?;

    Ok(())
}

#[must_use]
pub fn format_hours(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if hours == 0 { format!("{minutes}m") } else { format!("{hours}h {minutes}m") }
}
