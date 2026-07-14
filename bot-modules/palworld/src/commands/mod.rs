mod breed_for;
mod breed_plan;
mod breeding;
mod item;
mod link;
mod pal;
mod passive;
mod roster;
mod type_chart;
mod upload;

use serenity::all::{
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    CreateComponent,
    EditInteractionResponse,
    MessageFlags,
};
use sqlx::PgPool;
use zayden_core::ctx::ModalCtx;
use zayden_core::{InvocationCtx, as_i64, parse_options, parse_subcommand};

use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};
use crate::link::PlayerLink;
use crate::model::{Element, Pal, PlayerRoster, WorldRoster};
use crate::upload::SaveUpload;

pub struct Command;

impl Command {
    pub fn register() -> CreateCommand<'static> {
        let pal = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "pal",
            "Paldex entry: stats, elements, work, drops, skills, and breeding",
        )
        .add_sub_option(name_option("Pal name"));

        let breeding = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "breeding",
            "Work out the child of two parent Pals",
        )
        .add_sub_option(name_option_named("parent_a", "First parent"))
        .add_sub_option(name_option_named("parent_b", "Second parent"));

        let breed_for = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "breed-for",
            "List parent pairs that produce a target Pal",
        )
        .add_sub_option(name_option_named("target", "Target Pal"));

        let item = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "item",
            "Item lookup: type, weight, and sell price",
        )
        .add_sub_option(name_option("Item name"));

        let passive = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "passive",
            "Passive skill: effect, drawback, and tier",
        )
        .add_sub_option(name_option("Passive skill name"));

        let mut element_option = CreateCommandOption::new(
            CommandOptionType::String,
            "element",
            "Element",
        )
        .required(true);
        for element in Element::all() {
            element_option =
                element_option.add_string_choice(element.label(), element.key());
        }
        let type_chart = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "type",
            "Elemental effectiveness chart and Pals of that element",
        )
        .add_sub_option(element_option);

        let link = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "link",
            "Link your Discord account to your in-game player",
        )
        .add_sub_option(name_option_named("name", "Your in-game player name"));

        let unlink = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "unlink",
            "Remove your in-game player link",
        );

        let roster = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "roster",
            "Show a player's parsed Pals and genders",
        )
        .add_sub_option(player_option());

        let breed_plan = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "breed-plan",
            "Cheapest breeding path from a player's roster to a target Pal",
        )
        .add_sub_option(name_option_named("target", "Target Pal"))
        .add_sub_option(player_option());

        let upload = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "upload",
            "Upload your own Level.sav to use as your private world",
        );

        CreateCommand::new("palworld")
            .description(
                "Palworld guide: Pals, breeding, items, passives, and types",
            )
            .add_option(pal)
            .add_option(breeding)
            .add_option(breed_for)
            .add_option(item)
            .add_option(passive)
            .add_option(type_chart)
            .add_option(link)
            .add_option(unlink)
            .add_option(roster)
            .add_option(breed_plan)
            .add_option(upload)
    }

    pub async fn run(
        cx: &InvocationCtx<'_>,
        client: &PalworldClient,
        pool: &PgPool,
    ) -> Result<()> {
        let (name, sub_options) = parse_subcommand(cx.interaction.data.options())
            .map_err(PalworldError::from)?;

        match name {
            "pal" => pal::run(cx, client, parse_options(sub_options)).await,
            "breeding" => {
                breeding::run(cx, client, parse_options(sub_options)).await
            },
            "breed-for" => {
                breed_for::run(cx, client, parse_options(sub_options)).await
            },
            "item" => item::run(cx, client, parse_options(sub_options)).await,
            "passive" => passive::run(cx, client, parse_options(sub_options)).await,
            "type" => type_chart::run(cx, client, parse_options(sub_options)).await,
            "link" => link::link(cx, client, pool, parse_options(sub_options)).await,
            "unlink" => link::unlink(cx, pool).await,
            "roster" => {
                roster::run(cx, client, pool, parse_options(sub_options)).await
            },
            "breed-plan" => {
                breed_plan::run(cx, client, pool, parse_options(sub_options)).await
            },
            "upload" => upload::open_modal(cx, pool).await,
            _ => Err(PalworldError::NotFound {
                entity: "subcommand",
                query: name.to_string(),
            }),
        }
    }

    pub async fn upload_submit(
        cx: &ModalCtx<'_>,
        client: &PalworldClient,
        pool: &PgPool,
    ) -> Result<()> {
        upload::submit(cx, client, pool).await
    }
}

fn player_option() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::String,
        "player",
        "In-game player (defaults to your linked player)",
    )
    .set_autocomplete(true)
}

pub(crate) async fn resolve_player(
    cx: &InvocationCtx<'_>,
    client: &PalworldClient,
    pool: &PgPool,
    player: Option<&str>,
) -> Result<PlayerRoster> {
    let discord_id = as_i64(cx.interaction.user.id.get());

    if let Some(upload) = SaveUpload::select(pool, discord_id).await?
        && !upload.is_expired()
    {
        let roster = client.user_roster(discord_id).await?;

        return player.map_or_else(
            || most_populated_player(&roster),
            |name| {
                roster.by_name(name).cloned().ok_or_else(|| {
                    PalworldError::NotFound {
                        entity: "player",
                        query: name.to_string(),
                    }
                })
            },
        );
    }

    let roster = client.roster().await?;

    if let Some(name) = player {
        return roster.by_name(name).cloned().ok_or_else(|| {
            PalworldError::NotFound { entity: "player", query: name.to_string() }
        });
    }

    match PlayerLink::select(pool, discord_id).await? {
        Some(link) => roster.by_uid(&link.player_uid).cloned().ok_or_else(|| {
            PalworldError::NotFound { entity: "player", query: link.in_game_name }
        }),
        None => Err(PalworldError::NotLinked),
    }
}

fn most_populated_player(roster: &WorldRoster) -> Result<PlayerRoster> {
    roster.players.iter().max_by_key(|p| p.pals.len()).cloned().ok_or_else(|| {
        PalworldError::NotFound {
            entity: "player",
            query: "uploaded world".to_string(),
        }
    })
}

fn name_option(description: &'static str) -> CreateCommandOption<'static> {
    name_option_named("name", description)
}

fn name_option_named(
    name: &'static str,
    description: &'static str,
) -> CreateCommandOption<'static> {
    CreateCommandOption::new(CommandOptionType::String, name, description)
        .required(true)
        .set_autocomplete(true)
}

pub(crate) fn find_pal<'a>(pals: &'a [Pal], query: &str) -> Option<&'a Pal> {
    if let Some(pal) = pals.iter().find(|p| p.key == query) {
        return Some(pal);
    }
    if let Some(pal) = pals.iter().find(|p| p.name.eq_ignore_ascii_case(query)) {
        return Some(pal);
    }
    let query_lower = query.to_lowercase();
    pals.iter().find(|p| p.name.to_lowercase().contains(&query_lower))
}

pub(crate) async fn respond(
    cx: &InvocationCtx<'_>,
    component: CreateComponent<'static>,
) -> Result<()> {
    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .flags(MessageFlags::IS_COMPONENTS_V2)
                .components(vec![component]),
        )
        .await?;
    Ok(())
}
