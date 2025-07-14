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
use setup::setup;
use sqlx::{Database, Pool};
use transfer::transfer;
use trust::trust;
use unblock::unblock;
use untrust::untrust;

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    ResolvedValue,
};
use zayden_core::parse_options;

use crate::{
    Error, Result, VoiceChannelManager, VoiceStateCache, get_voice_state,
    guild_manager::TempVoiceGuildManager,
};

pub struct VoiceCommand;

impl VoiceCommand {
    pub async fn run<
        Data: VoiceStateCache,
        Db: Database,
        GuildManager: TempVoiceGuildManager<Db>,
        ChannelManager: VoiceChannelManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(Error::MissingGuildId)?;

        let command = interaction.data.options().pop().unwrap();

        let mut options = match command.value {
            ResolvedValue::SubCommand(options) => parse_options(options),
            _ => unreachable!("Subcommand is required"),
        };

        match command.name {
            "setup" => {
                setup::<Db, GuildManager>(&ctx.http, interaction, pool, guild_id, options).await?;

                return Ok(());
            }
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
            }
            _ => {}
        }

        let channel_id = match options.remove("channel") {
            Some(ResolvedValue::Channel(channel)) => channel.id().expect_channel(),
            _ => get_voice_state(&ctx.http, guild_id, interaction.user.id)
                .await?
                .channel_id
                .ok_or(Error::MemberNotInVoiceChannel)?,
        };

        let row = ChannelManager::get(pool, channel_id).await.unwrap();

        if command.name == "claim" {
            claim::<Data, Db, ChannelManager>(ctx, interaction, pool, channel_id, row).await?;
            return Ok(());
        }

        let row = row.ok_or(Error::ChannelNotFound(channel_id))?;

        match command.name {
            "join" => {
                join(&ctx.http, interaction, options, guild_id, channel_id, &row).await?;
            }
            "persist" => {
                persist::<Db, ChannelManager>(&ctx.http, interaction, pool, row).await?;
            }
            "name" => {
                name(&ctx.http, interaction, options, channel_id, &row).await?;
            }
            "limit" => {
                limit(&ctx.http, interaction, options, channel_id, &row).await?;
            }
            "privacy" => {
                privacy::<Data>(ctx, interaction, options, guild_id, channel_id, row).await?;
            }
            "waiting" => {
                // waiting(ctx, interaction, guild_id, options).await?;
            }
            "trust" => {
                trust::<Db, ChannelManager>(&ctx.http, interaction, pool, options, channel_id, row)
                    .await?;
            }
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
            }
            "invite" => {
                invite(&ctx.http, interaction, options, channel_id, row).await?;
            }
            "kick" => {
                kick(&ctx.http, interaction, options, guild_id, &row).await?;
            }
            "region" => {
                region(&ctx.http, interaction, options, channel_id, &row).await?;
            }
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
            }
            "unblock" => {
                unblock(&ctx.http, interaction, options, channel_id, &row).await?;
            }
            "delete" => {
                delete::<Db, ChannelManager>(&ctx.http, interaction, pool, channel_id, row).await?;
            }
            "bitrate" => {
                bitrate(&ctx.http, interaction, options, channel_id, &row).await?;
            }
            "info" => {
                // info(ctx, interaction, guild_id, options).await?;
            }
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
            }
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
            }
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
            }
            _ => unreachable!("Invalid subcommand name"),
        };

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
