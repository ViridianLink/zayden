use serenity::all::{
    ComponentInteraction,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateMessage,
    EditInteractionResponse,
    Http,
    ModalInteraction,
    UserId,
};
use sqlx::PgPool;
use tracing::warn;
use zayden_app::entitlement::EntitlementService;
use zayden_core::parse_modal_components;

use crate::error::{HostingError, Result};
use crate::modal::{self, NAME_FIELD, PLAN_FIELD};
use crate::pelican::generate_claim_code;
use crate::pricing::{self, Plan};
use crate::provision::{self, Provisioned};
use crate::store::{self, NewServer, billing, state};
use crate::{HostingRuntime, commands};

pub async fn spec_submitted(
    http: &Http,
    interaction: &ModalInteraction,
    runtime: &HostingRuntime,
    pool: &PgPool,
    entitlements: &EntitlementService,
) -> Result<()> {
    let game_key = interaction
        .data
        .custom_id
        .strip_prefix(modal::MODAL_PREFIX)
        .ok_or_else(|| HostingError::internal("malformed modal id"))?;

    let game = runtime.catalog.get(game_key)?;

    let fields = parse_modal_components(&interaction.data.components);
    let plan_key = fields
        .get(PLAN_FIELD)
        .and_then(|values| values.first())
        .ok_or_else(|| HostingError::UnknownPlan(String::new()))?;
    let plan = Plan::from_key(plan_key)?;

    let owner = interaction.user.id;
    let tier = entitlements.user_tier(owner.get()).await;
    let price = pricing::price_cents(plan, tier);

    commands::check_capacity(runtime, pool, owner, tier, plan).await?;

    store::ensure_user(pool, owner, &interaction.user.name).await?;
    if let Some(guild_id) = interaction.guild_id {
        store::ensure_guild(pool, guild_id).await?;
    }

    let billing_source =
        if price == 0 { billing::ENTITLEMENT } else { billing::KOFI };

    let row_id = store::insert_provisioning(pool, &NewServer {
        owner_id: owner,
        guild_id: interaction.guild_id,
        game_key: &game.key,
        plan,
        pelican_user_id: store::panel_user_id(pool, owner).await?,
        price_cents: price,
        billing_source,
        claim_code: &generate_claim_code(),
    })
    .await?;

    let requested_name = fields
        .get(NAME_FIELD)
        .and_then(|values| values.first())
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .unwrap_or(&game.name);

    let summary = format!(
        "**{}** — {} ({} GB RAM · {}% CPU · {} GB disk)\n\
         Name: `{}`\nPrice: **{}** per month, after a {} free trial to check \
         it comes up and the specs are what you expect.",
        game.name,
        plan.label(),
        plan.memory_mib() / 1024,
        plan.cpu_percent(),
        plan.disk_mib() / 1024,
        modal::truncate(requested_name, 48),
        pricing::format_price(price),
        format_hours(runtime.config.trial_hours)
    );

    interaction
        .create_response(http, modal::confirm_response(row_id, &summary))
        .await?;

    Ok(())
}

pub async fn confirm(
    http: &Http,
    interaction: &ComponentInteraction,
    runtime: &HostingRuntime,
    pool: &PgPool,
) -> Result<()> {
    let row_id = row_id(&interaction.data.custom_id, modal::CONFIRM_PREFIX)?;
    let row = store::get_owned(pool, row_id, interaction.user.id).await?;

    if row.state != state::PROVISIONING {
        return Err(HostingError::ServerNotFound);
    }

    interaction.defer_ephemeral(http).await?;

    let display_name = interaction.user.display_name().to_owned();
    let Provisioned { address, panel_username, password } =
        provision::provision(runtime, pool, row_id, row.owner(), &display_name)
            .await?;

    let game = runtime.catalog.get(&row.game_key)?;
    let panel = &runtime.config.panel_url;
    let kofi = &runtime.config.kofi_url;
    let price = pricing::format_price(i64::from(row.price_cents));

    let address = address.unwrap_or_else(|| "pending".to_owned());
    let billing = if row.price_cents == 0 {
        "Your subscription covers this plan, so there is nothing to pay.".to_owned()
    } else {
        format!(
            "After the trial it is **{price}/month** — pay at {kofi} and put \
             `{}` in the message.",
            row.claim_code
        )
    };

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content(format!(
                "**{}** is up.\n\nAddress: `{address}`\nPanel: {panel}\n\
                 Trial ends in {}, and the server is suspended at that point \
                 unless it has been paid for. {billing}",
                game.name,
                format_hours(runtime.config.trial_hours)
            )),
        )
        .await?;

    send_credentials(http, interaction.user.id, panel, &panel_username, password)
        .await;

    Ok(())
}

pub async fn cancel(
    http: &Http,
    interaction: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let row_id = row_id(&interaction.data.custom_id, modal::CANCEL_PREFIX)?;
    let row = store::get_owned(pool, row_id, interaction.user.id).await?;

    if row.state == state::PROVISIONING {
        store::mark_failed(pool, row_id, "cancelled by the user").await?;
    }

    interaction
        .create_response(
            http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content("Cancelled — nothing was created.")
                    .components(Vec::new()),
            ),
        )
        .await?;

    Ok(())
}

async fn send_credentials(
    http: &Http,
    user: UserId,
    panel_url: &str,
    username: &str,
    password: Option<String>,
) {
    let content = password.map_or_else(
        || {
            format!(
                "Your server is attached to your existing panel account \
                 (`{username}`) at {panel_url}. Use the panel's password reset \
                 if you have lost the password."
            )
        },
        |password| {
            format!(
                "Panel login for your new server:\n{panel_url}\n\
                 Username: `{username}`\nPassword: ||`{password}`||\n\
                 Change it once you are in — this message is the only copy I \
                 have."
            )
        },
    );

    if let Err(e) =
        user.direct_message(http, CreateMessage::new().content(content)).await
    {
        warn!(
            error = %e,
            %user,
            "hosting: could not DM panel credentials; the user must use the \
             panel's password reset"
        );
    }
}

fn format_hours(hours: i64) -> String {
    match hours {
        1 => "1 hour".to_owned(),
        h => format!("{h} hours"),
    }
}

fn row_id(custom_id: &str, prefix: &str) -> Result<i64> {
    custom_id
        .strip_prefix(prefix)
        .and_then(|id| id.parse().ok())
        .ok_or_else(|| HostingError::internal("malformed component id"))
}
