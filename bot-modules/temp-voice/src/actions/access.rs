use serenity::all::{
    ChannelId,
    EditChannel,
    GuildId,
    Http,
    PermissionOverwrite,
    PermissionOverwriteType,
    Permissions,
    UserId,
};
use sqlx::{Database, Pool};

use super::{require_owner, require_trusted};
use crate::{Result, VoiceChannelManager, VoiceChannelRow, owner_perms};

pub async fn trust<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    pool: &Pool<Db>,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
    user_id: UserId,
    target: UserId,
) -> Result<String> {
    require_owner(&row, user_id)?;

    row.trust(target);
    row.save::<Db, Manager>(pool).await?;

    channel_id
        .create_permission(
            http,
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::MANAGE_CHANNELS
                    | Permissions::CONNECT
                    | Permissions::SET_VOICE_CHANNEL_STATUS,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(target),
            },
            Some("User trusted"),
        )
        .await?;

    Ok("Set user to trusted.".to_string())
}

pub async fn kick(
    http: &Http,
    guild_id: GuildId,
    row: &VoiceChannelRow,
    user_id: UserId,
    target: UserId,
) -> Result<String> {
    require_trusted(row, user_id)?;

    guild_id.disconnect_member(http, target).await?;

    Ok("User kicked from channel.".to_string())
}

pub async fn password<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    pool: &Pool<Db>,
    guild_id: GuildId,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
    user_id: UserId,
    pass: String,
) -> Result<String> {
    require_owner(&row, user_id)?;

    row.password = Some(pass);
    row.save::<Db, Manager>(pool).await?;

    let perms = vec![owner_perms(user_id), PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::CONNECT,
        kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
    }];

    channel_id.edit(http, EditChannel::new().permissions(perms)).await?;

    Ok("Password set.".to_string())
}
