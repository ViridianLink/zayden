use async_trait::async_trait;
use serenity::all::{
    Colour,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    CreateMessage,
    DiscordJsonError,
    EditInteractionResponse,
    EditMessage,
    ErrorResponse,
    GenericChannelId,
    GenericInteractionChannel,
    GuildId,
    HttpError,
    JsonErrorCode,
    MessageId,
    Permissions,
    ResolvedValue,
};
use sqlx::PgPool;
use zayden_core::error::CoreError;
use zayden_core::{
    HandlerError,
    InvocationCtx,
    ModuleCommand,
    as_i64,
    as_u64,
    optional_option,
    parse_options,
    parse_subcommand,
    required_option,
};

const MAX_FIELDS: usize = 25;

const DEFAULT_TITLE: &str = "Server Rules";
const DEFAULT_COLOUR: i32 = 0x00ff_0000; // red

pub(super) struct RulesCommand;

#[async_trait]
impl ModuleCommand for RulesCommand {
    fn module(&self) -> Option<&'static str> {
        Some("moderation")
    }

    fn definition(&self) -> CreateCommand<'static> {
        let config = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "config",
            "Set the channel the rules are posted in and the embed styling",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "channel",
                "The channel the rules message lives in",
            )
            .required(true),
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "title",
            "The embed title (default \"Server Rules\")",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "description",
            "Text shown above the rules",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "colour",
            "The embed colour as a hex code, e.g. FF0000",
        ));

        let add = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "add",
            "Add a rule to the end of the list",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "title",
                "The rule heading",
            )
            .required(true),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "body",
                "The rule text",
            )
            .required(true),
        );

        let edit = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "edit",
            "Edit an existing rule by its position",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "position",
                "The 1-based position of the rule to edit",
            )
            .min_int_value(1)
            .required(true),
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "title",
            "The new rule heading",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "body",
            "The new rule text",
        ));

        let remove = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "remove",
            "Remove a rule by its position",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "position",
                "The 1-based position of the rule to remove",
            )
            .min_int_value(1)
            .required(true),
        );

        let reorder = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "reorder",
            "Move a rule to a new position",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "position",
                "The current 1-based position of the rule",
            )
            .min_int_value(1)
            .required(true),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "to",
                "The new 1-based position for the rule",
            )
            .min_int_value(1)
            .required(true),
        );

        let list = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "Show the configured rules and where they post",
        );

        let post = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "post",
            "Post or refresh the rules message in the configured channel",
        );

        CreateCommand::new("rules")
            .description("Manage this server's rules")
            .default_member_permissions(Permissions::MODERATE_MEMBERS)
            .add_option(config)
            .add_option(add)
            .add_option(edit)
            .add_option(remove)
            .add_option(reorder)
            .add_option(list)
            .add_option(post)
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

        let guild_id = cx.interaction.guild_id.ok_or(CoreError::MissingGuildId)?;

        let (name, sub_options) = parse_subcommand(cx.interaction.data.options())?;
        let mut options = parse_options(sub_options);

        let content = match name {
            "config" => config(&cx.app.db, guild_id, &mut options).await?,
            "add" => add(&cx.app.db, guild_id, &mut options).await?,
            "edit" => edit(&cx.app.db, guild_id, &mut options).await?,
            "remove" => remove(&cx.app.db, guild_id, &mut options).await?,
            "reorder" => reorder(&cx.app.db, guild_id, &mut options).await?,
            "list" => list(&cx.app.db, guild_id).await?,
            "post" => post(cx.ctx, &cx.app.db, guild_id).await?,
            other => {
                return Err(HandlerError::from_respond(CoreError::Other(format!(
                    "unexpected rules subcommand: {other}"
                ))));
            },
        };

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

struct RulesConfig {
    channel_id: Option<i64>,
    message_id: Option<i64>,
    title: String,
    description: Option<String>,
    colour: i32,
}

struct RuleEntry {
    position: i32,
    title: String,
    body: String,
}

async fn config(
    pool: &PgPool,
    guild_id: GuildId,
    options: &mut std::collections::HashMap<&str, ResolvedValue<'_>>,
) -> Result<String, HandlerError> {
    let channel: &GenericInteractionChannel = required_option(options, "channel")?;
    let title = optional_option::<&str, _>(options, "title");
    let description = optional_option::<&str, _>(options, "description");

    let colour = match optional_option::<&str, _>(options, "colour") {
        Some(raw) => match parse_colour(raw) {
            Some(colour) => Some(colour),
            None => {
                return Ok(format!(
                    "`{raw}` is not a valid hex colour. Use a code like `FF0000`."
                ));
            },
        },
        None => None,
    };

    sqlx::query!(
        "INSERT INTO guild_rules (guild_id, channel_id, title, description, colour)
         VALUES ($1, $2, COALESCE($3, $6), $4, COALESCE($5::integer, $7::integer))
         ON CONFLICT (guild_id) DO UPDATE SET
             channel_id = $2,
             title = COALESCE($3, guild_rules.title),
             description = COALESCE($4, guild_rules.description),
             colour = COALESCE($5, guild_rules.colour)",
        as_i64(guild_id.get()),
        as_i64(channel.id().get()),
        title,
        description,
        colour,
        DEFAULT_TITLE,
        DEFAULT_COLOUR,
    )
    .execute(pool)
    .await?;

    Ok(format!(
        "Rules will post in <#{}>. Use `/rules add` then `/rules post`.",
        channel.id().get()
    ))
}

async fn add(
    pool: &PgPool,
    guild_id: GuildId,
    options: &mut std::collections::HashMap<&str, ResolvedValue<'_>>,
) -> Result<String, HandlerError> {
    let title: &str = required_option(options, "title")?;
    let body: &str = required_option(options, "body")?;

    let guild_id = as_i64(guild_id.get());

    let mut tx = pool.begin().await?;

    sqlx::query!(
        "INSERT INTO guild_rules (guild_id) VALUES ($1) ON CONFLICT DO NOTHING",
        guild_id,
    )
    .execute(&mut *tx)
    .await?;

    let position = sqlx::query_scalar!(
        "INSERT INTO guild_rule (guild_id, position, title, body)
         VALUES (
             $1,
             (SELECT COALESCE(MAX(position), 0) + 1
              FROM guild_rule WHERE guild_id = $1),
             $2,
             $3
         )
         RETURNING position",
        guild_id,
        title,
        body,
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(format!("Added rule #{position}. Run `/rules post` to publish it."))
}

async fn edit(
    pool: &PgPool,
    guild_id: GuildId,
    options: &mut std::collections::HashMap<&str, ResolvedValue<'_>>,
) -> Result<String, HandlerError> {
    let position = i32::try_from(required_option::<i64, _>(options, "position")?)
        .unwrap_or(i32::MAX);
    let title = optional_option::<&str, _>(options, "title");
    let body = optional_option::<&str, _>(options, "body");

    if title.is_none() && body.is_none() {
        return Ok("Provide a new `title` and/or `body` to edit.".to_string());
    }

    let affected = sqlx::query!(
        "UPDATE guild_rule
         SET title = COALESCE($3, title), body = COALESCE($4, body)
         WHERE guild_id = $1 AND position = $2",
        as_i64(guild_id.get()),
        position,
        title,
        body,
    )
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Ok(format!("There is no rule at position #{position}."));
    }

    Ok(format!("Edited rule #{position}. Run `/rules post` to publish it."))
}

async fn remove(
    pool: &PgPool,
    guild_id: GuildId,
    options: &mut std::collections::HashMap<&str, ResolvedValue<'_>>,
) -> Result<String, HandlerError> {
    let position = i32::try_from(required_option::<i64, _>(options, "position")?)
        .unwrap_or(i32::MAX);

    let guild_id = as_i64(guild_id.get());

    let mut tx = pool.begin().await?;

    let deleted = sqlx::query!(
        "DELETE FROM guild_rule WHERE guild_id = $1 AND position = $2",
        guild_id,
        position,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if deleted == 0 {
        return Ok(format!("There is no rule at position #{position}."));
    }

    sqlx::query!(
        "UPDATE guild_rule SET position = position - 1
         WHERE guild_id = $1 AND position > $2",
        guild_id,
        position,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(format!("Removed rule #{position}. Run `/rules post` to publish."))
}

async fn reorder(
    pool: &PgPool,
    guild_id: GuildId,
    options: &mut std::collections::HashMap<&str, ResolvedValue<'_>>,
) -> Result<String, HandlerError> {
    let from = i32::try_from(required_option::<i64, _>(options, "position")?)
        .unwrap_or(i32::MAX);
    let to =
        i32::try_from(required_option::<i64, _>(options, "to")?).unwrap_or(i32::MAX);

    let guild_id = as_i64(guild_id.get());

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM guild_rule WHERE guild_id = $1",
        guild_id,
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    let count = i32::try_from(count).unwrap_or(i32::MAX);
    if count == 0 {
        return Ok("There are no rules to reorder.".to_string());
    }

    let to = to.clamp(1, count);
    if from == to {
        return Ok(format!("Rule #{from} is already at that position."));
    }

    let mut tx = pool.begin().await?;

    let parked = sqlx::query!(
        "UPDATE guild_rule SET position = 0 WHERE guild_id = $1 AND position = $2",
        guild_id,
        from,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if parked == 0 {
        return Ok(format!("There is no rule at position #{from}."));
    }

    if from < to {
        sqlx::query!(
            "UPDATE guild_rule SET position = position - 1
             WHERE guild_id = $1 AND position > $2 AND position <= $3",
            guild_id,
            from,
            to,
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE guild_rule SET position = position + 1
             WHERE guild_id = $1 AND position >= $2 AND position < $3",
            guild_id,
            to,
            from,
        )
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query!(
        "UPDATE guild_rule SET position = $2 WHERE guild_id = $1 AND position = 0",
        guild_id,
        to,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(format!("Moved rule #{from} to #{to}. Run `/rules post` to publish."))
}

async fn list(pool: &PgPool, guild_id: GuildId) -> Result<String, HandlerError> {
    use std::fmt::Write as _;

    let config = fetch_config(pool, guild_id).await?;
    let rules = fetch_rules(pool, guild_id).await?;

    let mut out = String::new();
    match config.as_ref().and_then(|c| c.channel_id) {
        Some(channel_id) => {
            let _ = writeln!(out, "**Channel:** <#{}>", as_u64(channel_id));
        },
        None => out.push_str("**Channel:** not set (use `/rules config`)\n"),
    }

    if rules.is_empty() {
        out.push_str("\nNo rules configured yet. Add one with `/rules add`.");
        return Ok(out);
    }

    for rule in &rules {
        let _ =
            write!(out, "\n**{}. {}**\n{}\n", rule.position, rule.title, rule.body);
    }

    Ok(out)
}

async fn post(
    ctx: &Context,
    pool: &PgPool,
    guild_id: GuildId,
) -> Result<String, HandlerError> {
    let Some(config) = fetch_config(pool, guild_id).await? else {
        return Ok(
            "No rules configured yet. Use `/rules config` and `/rules add` first."
                .to_string(),
        );
    };

    let Some(channel_id) = config.channel_id else {
        return Ok(
            "No channel is configured. Set one with `/rules config`.".to_string()
        );
    };

    let rules = fetch_rules(pool, guild_id).await?;
    if rules.is_empty() {
        return Ok(
            "There are no rules to post. Add some with `/rules add`.".to_string()
        );
    }

    if rules.len() > MAX_FIELDS {
        return Ok(format!(
            "A rules embed supports at most {MAX_FIELDS} rules; you have {}. \
             Please consolidate some before posting.",
            rules.len()
        ));
    }

    let embed = build_embed(&config, &rules);
    let channel = GenericChannelId::new(as_u64(channel_id));

    if let Some(message_id) = config.message_id {
        let message_id = MessageId::new(as_u64(message_id));
        match channel
            .edit_message(
                &ctx.http,
                message_id,
                EditMessage::new().embed(embed.clone()),
            )
            .await
        {
            Ok(_) => return Ok("Rules message updated.".to_string()),
            // Stored message/channel is gone — fall through and post a fresh one.
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                ErrorResponse {
                    error:
                        DiscordJsonError {
                            code:
                                JsonErrorCode::UnknownMessage
                                | JsonErrorCode::UnknownChannel,
                            ..
                        },
                    ..
                },
            ))) => {},
            Err(e) => return Err(e.into()),
        }
    }

    let message =
        channel.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;

    sqlx::query!(
        "UPDATE guild_rules SET message_id = $2 WHERE guild_id = $1",
        as_i64(guild_id.get()),
        as_i64(message.id.get()),
    )
    .execute(pool)
    .await?;

    Ok("Rules message posted.".to_string())
}

async fn fetch_config(
    pool: &PgPool,
    guild_id: GuildId,
) -> Result<Option<RulesConfig>, HandlerError> {
    let config = sqlx::query_as!(
        RulesConfig,
        "SELECT channel_id, message_id, title, description, colour
         FROM guild_rules WHERE guild_id = $1",
        as_i64(guild_id.get()),
    )
    .fetch_optional(pool)
    .await?;

    Ok(config)
}

async fn fetch_rules(
    pool: &PgPool,
    guild_id: GuildId,
) -> Result<Vec<RuleEntry>, HandlerError> {
    let rules = sqlx::query_as!(
        RuleEntry,
        "SELECT position, title, body FROM guild_rule
         WHERE guild_id = $1 ORDER BY position",
        as_i64(guild_id.get()),
    )
    .fetch_all(pool)
    .await?;

    Ok(rules)
}

fn build_embed(config: &RulesConfig, rules: &[RuleEntry]) -> CreateEmbed<'static> {
    let fields =
        rules.iter().map(|rule| (rule.title.clone(), rule.body.clone(), false));

    let mut embed = CreateEmbed::new()
        .colour(Colour::new(u32::try_from(config.colour).unwrap_or(0x00ff_0000)))
        .title(config.title.clone())
        .fields(fields);

    if let Some(description) = &config.description {
        embed = embed.description(description.clone());
    }

    embed
}

fn parse_colour(raw: &str) -> Option<i32> {
    let hex = raw.trim().trim_start_matches('#');
    let value = u32::from_str_radix(hex, 16).ok()?;
    if value > 0x00ff_ffff {
        return None;
    }
    i32::try_from(value).ok()
}
