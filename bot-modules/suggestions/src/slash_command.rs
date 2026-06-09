use std::time;

use futures::{StreamExt, stream};
use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    CreateMessage,
    EditInteractionResponse,
    Http,
    Mentionable,
    Permissions,
    ReactionTypes,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::{CoreError as ZaydenError, parse_options};

use crate::{Result, SuggestionsError, SuggestionsGuildManager};

pub struct FetchSuggestions;

impl FetchSuggestions {
    pub async fn run<Db: Database, Manager: SuggestionsGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let start_time = time::Instant::now();

        let guild_id = interaction.guild_id.ok_or(ZaydenError::MissingGuildId)?;

        let mut options = parse_options(options);

        let channel_id = match options.remove("channel") {
            Some(ResolvedValue::Channel(channel)) => channel.id().expect_channel(),
            _ => Manager::get(pool, guild_id)
                .await?
                .ok_or(SuggestionsError::MissingSuggesionChannel)?
                .channel_id()
                .ok_or(SuggestionsError::MissingSuggesionChannel)?,
        };

        let active_guild_threads = guild_id.get_active_threads(http).await?;
        let threads_iter = active_guild_threads
            .threads
            .into_iter()
            .filter(|thread| thread.parent_id == channel_id)
            .chain(
                channel_id
                    .get_archived_public_threads(http, None, None)
                    .await?
                    .threads,
            );

        let mut reaction_counts = stream::iter(threads_iter)
            .then(|thread| async {
                let count = thread
                    .id
                    .widen()
                    .reaction_users(
                        http,
                        thread.id.get().into(),
                        '👍',
                        Some(ReactionTypes::Normal),
                        None,
                        None,
                    )
                    .await
                    .map_or(0, |r| r.len());

                (thread, count)
            })
            .collect::<Vec<_>>()
            .await;

        reaction_counts.sort_by_key(|b| std::cmp::Reverse(b.1));

        let elapsed_time = start_time.elapsed();

        let fields_iter = reaction_counts.into_iter().take(10).enumerate().map(
            |(i, (thread, count))| {
                (
                    format!("{}. 👍: {}", i + 1, count),
                    format!("Link: {}", thread.mention()),
                    false,
                )
            },
        );

        let embed = CreateEmbed::new()
            .title("Top 10 suggestions")
            .description("Here are the top 10 suggestions, sorted by votes.")
            .fields(fields_iter);

        interaction
            .user
            .id
            .direct_message(&http, CreateMessage::new().embed(embed))
            .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content(format!(
                    "Suggestions fetched. Took {} seconds",
                    elapsed_time.as_secs()
                )),
            )
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("fetch_suggestions")
            .description("Fetch suggestions from the suggestion channel")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(CreateCommandOption::new(
                CommandOptionType::Channel,
                "channel",
                "The channel to fetch suggestions from",
            ))
    }
}
