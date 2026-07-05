use std::collections::HashMap;

use serenity::all::{CreateEmbed, EditInteractionResponse, ResolvedValue, Role};
use zayden_app::entitlement::{EntitlementScope, Tier};
use zayden_core::{as_i64, optional_option};

use super::MusicCtx;
use crate::error::{MusicError, Result};
use crate::settings::MusicSettingsRow;

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer_ephemeral(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    if options.is_empty() {
        let embed = view_embed(&settings);
        ctx.interaction
            .edit_response(ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;
        return Ok(());
    }

    let clear_dj_role = matches!(
        options.remove("clear_dj_role"),
        Some(ResolvedValue::Boolean(true))
    );
    let dj_role: Option<&Role> = optional_option(&mut options, "dj_role");
    let default_volume: Option<i64> = optional_option(&mut options, "default_volume");
    let auto_disconnect_secs: Option<i64> =
        optional_option(&mut options, "auto_disconnect_secs");
    let announce_now_playing: Option<bool> =
        optional_option(&mut options, "announce_now_playing");
    let stay_connected: Option<bool> = optional_option(&mut options, "stay_connected");
    let autoplay: Option<bool> = optional_option(&mut options, "autoplay");

    if let Some(volume) = default_volume
        && !(0..=100).contains(&volume)
    {
        return Err(MusicError::VolumeOutOfRange);
    }

    if stay_connected == Some(true) || autoplay == Some(true) {
        let scope = EntitlementScope::UserInGuild(
            ctx.interaction.user.id.get(),
            ctx.guild_id.get(),
        );
        if !ctx.entitlements.allows(scope, Tier::Pro).await {
            return Err(MusicError::PremiumRequired);
        }
    }

    let guild_id = as_i64(ctx.guild_id.get());
    let updated = ctx
        .settings
        .update(guild_id, |row| {
            if clear_dj_role {
                row.dj_role_id = None;
            } else if let Some(role) = dj_role {
                row.dj_role_id = Some(as_i64(role.id.get()));
            }
            if let Some(volume) = default_volume {
                row.default_volume = i16::try_from(volume).unwrap_or(100);
            }
            if let Some(secs) = auto_disconnect_secs {
                row.auto_disconnect_secs = i32::try_from(secs.max(0)).unwrap_or(120);
            }
            if let Some(announce) = announce_now_playing {
                row.announce_now_playing = announce;
            }
            if let Some(stay) = stay_connected {
                row.stay_connected = stay;
            }
            if let Some(auto) = autoplay {
                row.autoplay = auto;
            }
        })
        .await?;

    let embed = view_embed(&updated);
    ctx.interaction
        .edit_response(ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}

fn view_embed(row: &MusicSettingsRow) -> CreateEmbed<'static> {
    let dj_role = row
        .dj_role_id
        .map_or_else(|| "Everyone".to_string(), |id| format!("<@&{id}>"));

    CreateEmbed::new()
        .title("Music Settings")
        .field("DJ Role", dj_role, true)
        .field("Default Volume", format!("{}%", row.default_volume), true)
        .field(
            "Auto-disconnect",
            format!("{}s", row.auto_disconnect_secs),
            true,
        )
        .field(
            "Announce Now Playing",
            row.announce_now_playing.to_string(),
            true,
        )
        .field("24/7 (Stay Connected)", row.stay_connected.to_string(), true)
        .field("Autoplay", row.autoplay.to_string(), true)
}
