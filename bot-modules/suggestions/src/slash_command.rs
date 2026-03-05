use std::time;

use futures::{StreamExt, stream};
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption, CreateEmbed,
    CreateMessage, EditInteractionResponse, Http, Mentionable, Permissions, ResolvedOption,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::{Error, Result, SuggestionsGuildManager};

pub struct FetchSuggestions;

impl FetchSuggestions {
    pub async fn run<Db: Database, Manager: SuggestionsGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let start_time = time::Instant::now();

        let guild_id = interaction.guild_id.ok_or(Error::MissingGuildId)?;

        let mut options = parse_options(options);

        let channel_id = match options.remove("channel") {
            Some(ResolvedValue::Channel(channel)) => channel.id().expect_channel(),
            _ => Manager::get(pool, guild_id)
                .await
                .unwrap()
                .ok_or(Error::MissingSuggesionChannel)?
                .channel_id()
                .ok_or(Error::MissingSuggesionChannel)?,
        };

        let active_guild_threads = guild_id.get_active_threads(http).await.unwrap();
        let threads_iter = active_guild_threads
            .threads
            .into_iter()
            .filter(|thread| thread.parent_id == channel_id)
            .chain(
                channel_id
                    .get_archived_public_threads(http, None, None)
                    .await
                    .unwrap()
                    .threads,
            );

        let mut reaction_counts = stream::iter(threads_iter)
            .then(|thread| async {
                let reactions = thread
                    .id
                    .widen()
                    .reaction_users(http, thread.id.get().into(), '👍', Some(100), None)
                    .await
                    .unwrap();

                (thread, reactions.len())
            })
            .collect::<Vec<_>>()
            .await;

        reaction_counts.sort_by(|a, b| b.1.cmp(&a.1));

        let elapsed_time = start_time.elapsed();

        let fields_iter =
            reaction_counts
                .into_iter()
                .take(10)
                .enumerate()
                .map(|(i, (thread, count))| {
                    (
                        format!("{}. 👍: {}", i + 1, count),
                        format!("Link: {}", thread.mention()),
                        false,
                    )
                });

        let embed = CreateEmbed::new()
            .title("Top 10 suggestions")
            .description("Here are the top 10 suggestions, sorted by votes.")
            .fields(fields_iter);

        interaction
            .user
            .id
            .direct_message(&http, CreateMessage::new().embed(embed))
            .await
            .unwrap();

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content(format!(
                    "Suggestions fetched. Took {} seconds",
                    elapsed_time.as_secs()
                )),
            )
            .await
            .unwrap();

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
