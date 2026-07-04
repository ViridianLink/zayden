mod bitrate;
mod block;
mod claim;
mod create;
mod delete;
mod invite;
mod join;
mod kick;
mod limit;
mod name;
mod password;
mod persist;
mod privacy;
mod region;
mod reset;
mod setup;
mod transfer;
mod trust;
mod unblock;
mod untrust;

use bitrate::bitrate;
use block::block;
use claim::claim;
use create::create;
use delete::delete;
use invite::invite;
use join::join;
use kick::kick;
use limit::limit;
use name::name;
use password::password;
use persist::persist;
use privacy::privacy;
use region::region;
use reset::reset;
use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    GenericInteractionChannel,
    Permissions,
};
use setup::setup;
use sqlx::{Database, Pool};
use transfer::transfer;
use trust::trust;
use unblock::unblock;
use untrust::untrust;
use zayden_core::{optional_option, parse_options, parse_subcommand};

use crate::guild_manager::TempVoiceGuildManager;
use crate::{
    Result,
    TempVoiceError,
    VoiceChannelManager,
    VoiceChannelRow,
    VoiceStateCache,
};

fn has_manage_channels(permissions: Option<Permissions>) -> bool {
    permissions.is_some_and(Permissions::manage_channels)
}

pub struct VoiceCommand;

impl VoiceCommand {
    pub async fn run<
        Db: Database,
        GuildManager: TempVoiceGuildManager<Db>,
        ChannelManager: VoiceChannelManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(TempVoiceError::MissingGuildId)?;

        let (sub_name, sub_options) = parse_subcommand(interaction.data.options())?;

        let mut options = parse_options(sub_options);

        match sub_name {
            "setup" => {
                setup::<Db, GuildManager>(
                    &ctx.http,
                    interaction,
                    pool,
                    guild_id,
                    options,
                )
                .await?;

                return Ok(());
            },
            "create" => {
                create::<Db, GuildManager, ChannelManager>(
                    &ctx.http,
                    interaction,
                    pool,
                    guild_id,
                    options,
                )
                .await?;

                return Ok(());
            },
            _ => {},
        }

        let channel_id = match optional_option::<&GenericInteractionChannel, _>(
            &mut options,
            "channel",
        ) {
            Some(channel) => channel.id().expect_channel(),
            None => guild_id
                .get_user_voice_state(&ctx.http, interaction.user.id)
                .await?
                .channel_id
                .ok_or(TempVoiceError::MemberNotInVoiceChannel)?,
        };

        let row = match ChannelManager::get(pool, channel_id).await {
            Ok(Some(row)) => row,
            Ok(None) => {
                let permissions =
                    interaction.member.as_ref().and_then(|m| m.permissions);

                if !has_manage_channels(permissions) {
                    return Err(TempVoiceError::IneligibleChannel);
                }

                let row =
                    VoiceChannelRow::new_persistent(channel_id, interaction.user.id);

                ChannelManager::save(pool, row.clone()).await?;

                row
            },
            Err(e) => return Err(TempVoiceError::Sqlx(e)),
        };

        match sub_name {
            "claim" => {
                claim::<Db, ChannelManager>(
                    ctx,
                    interaction,
                    pool,
                    voice_states,
                    channel_id,
                    row,
                )
                .await?;
            },
            "join" => {
                join(&ctx.http, interaction, options, guild_id, channel_id, &row)
                    .await?;
            },
            "persist" => {
                persist::<Db, ChannelManager>(&ctx.http, interaction, pool, row)
                    .await?;
            },
            "name" => {
                name(&ctx.http, interaction, options, channel_id, &row).await?;
            },
            "limit" => {
                limit(&ctx.http, interaction, options, channel_id, &row).await?;
            },
            "privacy" => {
                privacy(
                    ctx,
                    interaction,
                    voice_states,
                    options,
                    guild_id,
                    channel_id,
                    row,
                )
                .await?;
            },
            "waiting" | "info" => {
                // TODO: Not yet implemented
            },
            "trust" => {
                trust::<Db, ChannelManager>(
                    &ctx.http,
                    interaction,
                    pool,
                    options,
                    channel_id,
                    row,
                )
                .await?;
            },
            "untrust" => {
                untrust::<Db, ChannelManager>(
                    &ctx.http,
                    interaction,
                    pool,
                    options,
                    channel_id,
                    row,
                )
                .await?;
            },
            "invite" => {
                invite(&ctx.http, interaction, options, channel_id, row).await?;
            },
            "kick" => {
                kick(&ctx.http, interaction, options, guild_id, &row).await?;
            },
            "region" => {
                region(&ctx.http, interaction, options, channel_id, &row).await?;
            },
            "block" => {
                block::<Db, ChannelManager>(
                    &ctx.http,
                    interaction,
                    pool,
                    options,
                    guild_id,
                    channel_id,
                    row,
                )
                .await?;
            },
            "unblock" => {
                unblock(&ctx.http, interaction, options, channel_id, &row).await?;
            },
            "delete" => {
                delete::<Db, ChannelManager>(
                    &ctx.http,
                    interaction,
                    pool,
                    channel_id,
                    row,
                )
                .await?;
            },
            "bitrate" => {
                bitrate(&ctx.http, interaction, options, channel_id, &row).await?;
            },
            "password" => {
                password::<Db, ChannelManager>(
                    &ctx.http,
                    interaction,
                    pool,
                    options,
                    guild_id,
                    channel_id,
                    row,
                )
                .await?;
            },
            "reset" => {
                reset::<Db, ChannelManager>(
                    &ctx.http,
                    interaction,
                    pool,
                    guild_id,
                    channel_id,
                    row,
                )
                .await?;
            },
            "transfer" => {
                transfer::<Db, ChannelManager>(
                    &ctx.http,
                    interaction,
                    pool,
                    options,
                    channel_id,
                    row,
                )
                .await?;
            },
            _ => {
                return Err(TempVoiceError::Internal(format!(
                    "unexpected subcommand: {sub_name}"
                )));
            },
        }

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        let setup = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "setup",
            "Setup the temporary voice channel module for the guild.",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "category",
                "The category to create temporary voice channels in.",
            )
            .required(true),
        );

        let create = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "create",
            "Create a temporary voice channel.",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "name",
            "The name of the voice channel.",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Integer,
            "limit",
            "The user limit of the voice channel (0-99).",
        ))
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "privacy",
                "Lock or hide the voice channel.",
            )
            .add_string_choice("Spectator", "spectator")
            .add_string_choice("Open Mic", "open-mic")
            .add_string_choice("Lock", "lock")
            .add_string_choice("Unlock", "unlock")
            .add_string_choice("Invisible", "invisible")
            .add_string_choice("Visible", "visible"),
        );

        CreateCommand::new("voice")
            .description("Commands for creating and managing temporary voice channels.")
            .add_option(setup)
            .add_option(create)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "name",
                    "Change the name of the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "name",
                        "The new name of the voice channel.",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "limit",
                    "Change the user limit of the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "user_limit",
                        "The new user limit of the voice channel (0-99).",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "privacy",
                    "Change the privacy of the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "privacy",
                        "The new privacy of the voice channel.",
                    )
                    .add_string_choice("Open", "open")
                    .add_string_choice("Spectator", "spectator")
                    .add_string_choice("Lock", "lock")
                    .add_string_choice("Invisible", "invisible")
                    .required(true),
                ),
            )
            // .add_option(CreateCommandOption::new(
            //     CommandOptionType::SubCommand,
            //     "waiting",
            //     "Create a waiting room for the voice channel.",
            // ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "trust",
                    "Trusted users have access to the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::User, "user", "The user to trust.")
                        .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "untrust",
                    "Remove trusted users access from the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "The user to untrust.",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "invite",
                    "Invite a user to the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "The user to invite.",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "kick",
                    "Kick a user from the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::User, "user", "The user to kick.")
                        .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "region",
                    "Change the region of the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "region",
                        "The new region of the voice channel.",
                    )
                    .add_string_choice("Brazil", "brazil")
                    .add_string_choice("Hong Kong", "hongkong")
                    .add_string_choice("India", "india")
                    .add_string_choice("Japan", "japan")
                    .add_string_choice("Rotterdam", "rotterdam")
                    .add_string_choice("Russia", "russia")
                    .add_string_choice("Singapore", "singapore")
                    .add_string_choice("South Africa", "southafrica")
                    .add_string_choice("Sydney", "sydney")
                    .add_string_choice("US Central", "us-central")
                    .add_string_choice("US East", "us-east")
                    .add_string_choice("US South", "us-south")
                    .add_string_choice("US West", "us-west")
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "block",
                    "Block a user from the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::User, "user", "The user to block.")
                        .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "unblock",
                    "Unblock a user from the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "The user to unblock.",
                    )
                    .required(true),
                ),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "claim",
                "Claim the voice channel as your own.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "transfer",
                    "Transfer ownership of the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "The user to transfer ownership to.",
                    )
                    .required(true),
                ),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "delete",
                "Delete the voice channel.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "bitrate",
                    "Change the bitrate of the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "kbps",
                        "The new bitrate of the voice channel.",
                    )
                    .required(true),
                ),
            )
            // .add_option(CreateCommandOption::new(
            //     CommandOptionType::SubCommand,
            //     "info",
            //     "Get information about the voice channel.",
            // ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "password",
                    "Set a password for the voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "password",
                        "The password for the voice channel.",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "join",
                    "Join a password protected voice channel.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "channel",
                        "The voice channel to join.",
                    )
                    .required(true)
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "password",
                            "The password for the voice channel.",
                        )
                        .required(true),
                    ),
                ),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "reset",
                "Reset the voice channel to default settings.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "persist",
                    "Convert a temporary voice channel to a persistent voice channel.",
                )
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::Channel,
                    "channel",
                    "The voice channel to persist.",
                )),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_manage_channels_true_when_permission_granted() {
        assert!(has_manage_channels(Some(Permissions::MANAGE_CHANNELS)));
    }

    #[test]
    fn has_manage_channels_true_with_other_permissions_present() {
        assert!(has_manage_channels(Some(
            Permissions::MANAGE_CHANNELS | Permissions::SEND_MESSAGES
        )));
    }

    #[test]
    fn has_manage_channels_false_when_permission_absent() {
        assert!(!has_manage_channels(Some(Permissions::SEND_MESSAGES)));
    }

    #[test]
    fn has_manage_channels_false_when_permissions_missing() {
        assert!(!has_manage_channels(None));
    }
}
