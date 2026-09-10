use std::collections::HashMap;

use serenity::all::{
    AutocompleteChoice,
    CommandInteraction,
    CommandOptionType,
    CreateAutocompleteResponse,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    CreateInteractionResponse,
    EditInteractionResponse,
    Http,
    ResolvedOption,
    ResolvedValue,
    UserId,
};
use sqlx::PgPool;
use zayden_app::entitlement::{EntitlementService, Tier};
use zayden_core::{parse_options, parse_subcommand, required_option};

use crate::error::{HostingError, Result};
use crate::pricing::{self, Plan};
use crate::store::{self, HostedServer, state};
use crate::{HostingRuntime, modal};

const AUTOCOMPLETE_LIMIT: usize = 25;

pub struct Hosting;

impl Hosting {
    pub fn register<'a>() -> CreateCommand<'a> {
        let host = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "host",
            "Rent a game server",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "game",
                "Which game to host",
            )
            .required(true)
            .set_autocomplete(true),
        );

        let list = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "List the servers you are renting",
        );

        let info = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "info",
            "Show one of your servers in detail",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "id",
                "Server id, from /server list",
            )
            .required(true),
        );

        let cancel = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "cancel",
            "Cancel a server; it is deleted after the grace period",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "id",
                "Server id, from /server list",
            )
            .required(true),
        );

        CreateCommand::new("server")
            .description("Rent and manage game servers")
            .add_option(host)
            .add_option(list)
            .add_option(info)
            .add_option(cancel)
    }

    pub async fn run(
        http: &Http,
        interaction: &CommandInteraction,
        runtime: &HostingRuntime,
        pool: &PgPool,
        entitlements: &EntitlementService,
        options: Vec<ResolvedOption<'_>>,
    ) -> Result<()> {
        if runtime.catalog.is_empty() {
            return Err(HostingError::Disabled);
        }

        let (name, options) = parse_subcommand(options)
            .map_err(|_e| HostingError::internal("missing subcommand"))?;
        let mut options = parse_options(options);

        match name {
            "host" => {
                let game: &str = required_option(&mut options, "game")
                    .map_err(|_e| HostingError::UnknownGame(String::new()))?;
                host(http, interaction, runtime, pool, entitlements, game).await
            },
            "list" => list(http, interaction, runtime, pool).await,
            "info" => {
                let id = required_id(&mut options)?;
                info(http, interaction, runtime, pool, id).await
            },
            "cancel" => {
                let id = required_id(&mut options)?;
                cancel(http, interaction, runtime, pool, id).await
            },
            other => Err(HostingError::internal(format!(
                "unrecognized server subcommand: {other}"
            ))),
        }
    }

    pub async fn autocomplete(
        http: &Http,
        interaction: &CommandInteraction,
        runtime: &HostingRuntime,
    ) -> Result<()> {
        let query =
            interaction.data.autocomplete().map_or("", |option| option.value);

        let choices: Vec<AutocompleteChoice<'_>> = runtime
            .catalog
            .search(query, AUTOCOMPLETE_LIMIT)
            .into_iter()
            .map(|game| AutocompleteChoice::new(game.name.clone(), game.key.clone()))
            .collect();

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(choices),
                ),
            )
            .await?;

        Ok(())
    }
}

async fn host(
    http: &Http,
    interaction: &CommandInteraction,
    runtime: &HostingRuntime,
    pool: &PgPool,
    entitlements: &EntitlementService,
    game_key: &str,
) -> Result<()> {
    let game = runtime.catalog.get(game_key)?;
    let owner = interaction.user.id;
    let tier = entitlements.user_tier(owner.get()).await;

    check_capacity(runtime, pool, owner, tier, Plan::Small).await?;

    interaction
        .create_response(
            http,
            CreateInteractionResponse::Modal(modal::spec_modal(game, tier)),
        )
        .await?;

    Ok(())
}

async fn list(
    http: &Http,
    interaction: &CommandInteraction,
    runtime: &HostingRuntime,
    pool: &PgPool,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let rows = store::list_for_owner(pool, interaction.user.id).await?;

    let content = if rows.is_empty() {
        "You are not renting any servers. `/server host` starts one.".to_owned()
    } else {
        rows.iter().map(|row| summarise(runtime, row)).collect::<Vec<_>>().join("\n")
    };

    interaction
        .edit_response(http, EditInteractionResponse::new().content(content))
        .await?;

    Ok(())
}

async fn info(
    http: &Http,
    interaction: &CommandInteraction,
    runtime: &HostingRuntime,
    pool: &PgPool,
    id: i64,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let row = store::get_owned(pool, id, interaction.user.id).await?;
    let game = runtime.catalog.get(&row.game_key)?;
    let plan = row.plan()?;

    let expiry = row
        .expires_at()
        .map_or_else(|| "—".to_owned(), |t| format!("<t:{}:R>", t.as_second()));

    let embed = CreateEmbed::new()
        .title(format!("{} — {}", game.name, plan.label()))
        .field("State", &row.state, true)
        .field("Address", row.address.as_deref().unwrap_or("pending"), true)
        .field(
            "Specs",
            format!(
                "{} GB RAM · {}% CPU · {} GB disk",
                plan.memory_mib() / 1024,
                plan.cpu_percent(),
                plan.disk_mib() / 1024
            ),
            false,
        )
        .field(
            "Billing",
            format!(
                "{} ({})",
                pricing::format_price(i64::from(row.price_cents)),
                row.billing_source
            ),
            true,
        )
        .field("Renews", expiry, true)
        .field("Claim code", format!("`{}`", row.claim_code), true)
        .field("Panel", &runtime.config.panel_url, false);

    interaction
        .edit_response(http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}

async fn cancel(
    http: &Http,
    interaction: &CommandInteraction,
    runtime: &HostingRuntime,
    pool: &PgPool,
    id: i64,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let row = store::get_owned(pool, id, interaction.user.id).await?;

    if !row.is_live() {
        return Err(HostingError::ServerNotFound);
    }

    let grace = i32::try_from(runtime.config.grace_days.clamp(0, 365)).unwrap_or(0);
    let has_paid = row.paid_until.is_some();

    if let Some(server_id) = row.pelican_server_id {
        runtime.pelican.suspend(server_id).await?;
    }
    crate::sweep::suspend(pool, id, grace).await?;

    // Grace covers a lapsed subscription, not a cancelled trial, so the two
    // cancellations are not equally recoverable.
    let content = if has_paid {
        format!(
            "Server `{id}` is suspended and will be deleted in {grace} days. \
             Pay before then and it comes straight back."
        )
    } else {
        format!(
            "Trial server `{id}` is suspended and will be deleted shortly. \
             Subscribe before that happens to keep it."
        )
    };

    interaction
        .edit_response(http, EditInteractionResponse::new().content(content))
        .await?;

    Ok(())
}

pub async fn check_capacity(
    runtime: &HostingRuntime,
    pool: &PgPool,
    owner: UserId,
    tier: Tier,
    plan: Plan,
) -> Result<()> {
    let allowed = pricing::max_servers(tier);
    let current = store::live_count(pool, owner).await?;

    if current >= allowed {
        return Err(HostingError::ServerCap { current, allowed });
    }

    let (servers, memory) = store::global_usage(pool).await?;

    if servers >= runtime.config.max_servers {
        return Err(HostingError::NoCapacity(servers));
    }

    let requested = i64::from(plan.memory_mib());
    let available = runtime.config.max_ram_mib - memory;

    if requested > available {
        return Err(HostingError::NoMemory { requested, available });
    }

    Ok(())
}

fn summarise(runtime: &HostingRuntime, row: &HostedServer) -> String {
    let name = runtime
        .catalog
        .get(&row.game_key)
        .map_or_else(|_e| row.game_key.clone(), |g| g.name.clone());

    let address = row.address.as_deref().unwrap_or("pending");
    let marker = if row.state == state::SUSPENDED { "⏸" } else { "▶" };

    format!(
        "{marker} `{}` **{name}** — {} · `{address}` · {}",
        row.id,
        row.plan,
        pricing::format_price(i64::from(row.price_cents))
    )
}

fn required_id(options: &mut HashMap<&str, ResolvedValue<'_>>) -> Result<i64> {
    required_option(options, "id")
        .map_err(|_e| HostingError::internal("missing server id"))
}
