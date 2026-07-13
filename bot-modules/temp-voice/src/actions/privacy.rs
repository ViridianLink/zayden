use serenity::all::{
    ChannelId,
    EditChannel,
    GuildId,
    Http,
    PermissionOverwrite,
    PermissionOverwriteType,
    Permissions,
    RoleId,
    UserId,
};
use serenity::small_fixed_array::FixedArray;

use super::require_trusted;
use crate::{Result, TempVoiceError, VoiceChannelRow, VoiceStateCache};

pub async fn privacy(
    http: &Http,
    guild_id: GuildId,
    voice_states: &VoiceStateCache,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
    user_id: UserId,
    privacy: &str,
) -> Result<String> {
    require_trusted(row, user_id)?;

    let everyone_role = guild_id.everyone_role();
    let channel = channel_id.to_guild_channel(http, Some(guild_id)).await?;
    let users = voice_states.occupants(channel_id);
    let perms = channel.permission_overwrites;

    let builder = match privacy {
        "open" => open_builder(perms, everyone_role),
        "spectator" => spectate_builder(perms, everyone_role, users),
        "lock" => lock_builder(perms, everyone_role),
        "invisible" => invisible_builder(perms, everyone_role),
        _ => return Err(TempVoiceError::IneligibleChannel),
    };

    channel_id.edit(http, builder).await?;

    Ok("Channel privacy updated.".to_string())
}

fn open_builder<'a>(
    perms: FixedArray<PermissionOverwrite>,
    everyone: RoleId,
) -> EditChannel<'a> {
    let perms = perms
        .into_iter()
        .map(|perm| {
            if perm.kind == PermissionOverwriteType::Role(everyone) {
                PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(everyone),
                }
            } else {
                perm
            }
        })
        .collect::<Vec<_>>();

    EditChannel::new().permissions(perms)
}

fn spectate_builder<'a>(
    perms: FixedArray<PermissionOverwrite>,
    everyone: RoleId,
    users: Vec<UserId>,
) -> EditChannel<'a> {
    let mut perms = perms.into_vec();

    for user in users {
        let perm = perms.iter_mut().find(
            |perm| matches!(perm.kind, PermissionOverwriteType::Member(user_id) if user_id == user),
        );

        if let Some(perm) = perm {
            perm.allow |= Permissions::SPEAK;
        } else {
            perms.push(PermissionOverwrite {
                allow: Permissions::SPEAK,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(user),
            });
        }
    }

    match perms
        .iter_mut()
        .find(|perm| matches!(perm.kind, PermissionOverwriteType::Role(role) if role == everyone))
    {
        Some(perm) => perm.deny |= Permissions::SPEAK,
        None => perms.push(PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::SPEAK,
            kind: PermissionOverwriteType::Role(everyone),
        }),
    }

    EditChannel::new().permissions(perms)
}

fn lock_builder<'a>(
    perms: FixedArray<PermissionOverwrite>,
    everyone: RoleId,
) -> EditChannel<'a> {
    let perms = perms
        .into_iter()
        .map(|perm| {
            if perm.kind == PermissionOverwriteType::Role(everyone) {
                PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::CONNECT,
                    kind: PermissionOverwriteType::Role(everyone),
                }
            } else {
                perm
            }
        })
        .collect::<Vec<_>>();

    EditChannel::new().permissions(perms)
}

fn invisible_builder<'a>(
    perms: FixedArray<PermissionOverwrite>,
    everyone: RoleId,
) -> EditChannel<'a> {
    let perms = perms
        .into_iter()
        .map(|perm| {
            if perm.kind == PermissionOverwriteType::Role(everyone) {
                PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::VIEW_CHANNEL,
                    kind: PermissionOverwriteType::Role(everyone),
                }
            } else {
                perm
            }
        })
        .collect::<Vec<_>>();

    EditChannel::new().permissions(perms)
}
