use std::collections::HashMap;

use serenity::all::{
    ChannelId, CommandInteraction, EditChannel, EditInteractionResponse, GuildId, Http,
    PermissionOverwrite, PermissionOverwriteType, Permissions, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{Error, Result, VoiceChannelManager, VoiceChannelRow, owner_perms};

pub async fn password<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await.unwrap();

    if !row.is_owner(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotOwner));
    }

    let pass = match options.remove("pass") {
        Some(ResolvedValue::String(pass)) => pass,
        _ => unreachable!("Password option is required"),
    };

    row.password = Some(pass.to_string());
    row.save::<Db, Manager>(pool).await?;

    let perms = vec![
        owner_perms(interaction.user.id),
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::CONNECT,
            kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
        },
    ];

    channel_id
        .edit(http, EditChannel::new().permissions(perms))
        .await
        .unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Password set."),
        )
        .await
        .unwrap();

    Ok(())
}
