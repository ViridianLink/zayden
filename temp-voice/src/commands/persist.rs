use serenity::all::{CommandInteraction, EditInteractionResponse, Http};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{Error, Result, VoiceChannelManager, VoiceChannelRow};

pub async fn persist<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut row: VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await.unwrap();

    let member = interaction.member.as_ref().unwrap();
    let is_moderator = member.permissions.unwrap().manage_channels();

    if row.is_owner(interaction.user.id) && !is_moderator {
        return Err(Error::MissingPermissions(PermissionError::NotOwner));
    }

    let persistent_count = Manager::count_persistent_channels(pool, interaction.user.id)
        .await
        .unwrap();

    if persistent_count == 1 && !is_moderator {
        return Err(Error::MaxChannels);
    }

    row.toggle_persist();
    let state = if row.is_persistent() {
        "enabled"
    } else {
        "disabled"
    };

    row.save::<Db, Manager>(pool).await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content(format!("Channel persistence is now {state}.")),
        )
        .await
        .unwrap();

    Ok(())
}
