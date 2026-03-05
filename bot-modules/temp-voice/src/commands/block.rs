use std::collections::HashMap;

use serenity::all::{
    ChannelId, CommandInteraction, EditInteractionResponse, GuildId, Http, PermissionOverwrite,
    PermissionOverwriteType, Permissions, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{Error, VoiceChannelManager, VoiceChannelRow};

pub async fn block<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<(), Error> {
    interaction.defer_ephemeral(http).await.unwrap();

    if !row.is_trusted(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotTrusted));
    }

    let user = match options.remove("user") {
        Some(ResolvedValue::User(user, _)) => user,
        _ => unreachable!("User option is required"),
    };

    row.block(user.id);
    row.save::<Db, Manager>(pool).await?;

    channel_id
        .create_permission(
            http,
            PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::all(),
                kind: PermissionOverwriteType::Member(user.id),
            },
            Some("User blocked from channel"),
        )
        .await
        .unwrap();

    guild_id.disconnect_member(http, user.id).await.unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Set user to blocked."),
        )
        .await
        .unwrap();

    Ok(())
}
