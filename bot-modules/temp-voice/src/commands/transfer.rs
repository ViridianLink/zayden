use std::collections::HashMap;

use serenity::all::{ChannelId, CommandInteraction, EditInteractionResponse, Http, ResolvedValue};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{Error, Result, VoiceChannelManager, VoiceChannelRow, owner_perms};

pub async fn transfer<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await.unwrap();

    if !row.is_owner(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotOwner));
    }

    let user = match options.remove("user") {
        Some(ResolvedValue::User(user, _)) => user,
        _ => unreachable!("User option is required"),
    };

    row.set_owner(user.id);
    row.save::<Db, Manager>(pool).await?;

    channel_id
        .create_permission(http, owner_perms(user.id), Some("Channel transfered"))
        .await
        .unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Transferred channel."),
        )
        .await
        .unwrap();

    Ok(())
}
