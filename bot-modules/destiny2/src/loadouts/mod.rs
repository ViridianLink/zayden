pub(crate) mod domain;
pub(crate) mod mode;
pub(crate) mod record;

use std::sync::{Arc, LazyLock};
use std::time::Duration;

pub use domain::{Archetype, ArmourSlot, Class, Element, StatKind};
pub use mode::Mode;
pub use record::{ArmourRecord, AspectRecord, LoadoutRecord, WeaponRecord};
use serenity::all::{
    AutocompleteChoice,
    AutocompleteOption,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateAutocompleteResponse,
    CreateCommandOption,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    EmojiId,
    GuildId,
    Http,
    MessageFlags,
    Permissions,
    ResolvedOption,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use zayden_core::{
    CoreError,
    EmojiCache,
    EmojiCacheData,
    EmojiResult,
    parse_subcommand,
    sole_option,
};

use crate::db::loadouts as loadout_db;
use crate::{DestinyError, Result};

const DUPLICATE: EmojiId = EmojiId::new(1_395_743_560_388_706_374);

static CACHE: LazyLock<RwLock<Option<Arc<[LoadoutRecord]>>>> =
    LazyLock::new(|| RwLock::new(None));

async fn cached(pool: &PgPool) -> Result<Arc<[LoadoutRecord]>> {
    {
        let guard = CACHE.read().await;
        if let Some(records) = guard.as_ref() {
            return Ok(Arc::clone(records));
        }
    }

    let loaded: Arc<[LoadoutRecord]> = loadout_db::all(pool).await?.into();
    let mut guard = CACHE.write().await;
    Ok(Arc::clone(guard.get_or_insert(loaded)))
}

pub async fn invalidate_cache() {
    *CACHE.write().await = None;
}

pub struct Loadout;

impl Loadout {
    pub fn register<'a>() -> CreateCommandOption<'a> {
        fn class_sub<'b>(
            name: &'static str,
            description: &'static str,
        ) -> CreateCommandOption<'b> {
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                name,
                description,
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "build",
                    "Select the build",
                )
                .required(true)
                .set_autocomplete(true),
            )
        }

        CreateCommandOption::new(
            CommandOptionType::SubCommandGroup,
            "builds",
            "Destiny 2 Builds",
        )
        .add_sub_option(class_sub("warlock", "Warlock Builds"))
        .add_sub_option(class_sub("titan", "Titan Builds"))
        .add_sub_option(class_sub("hunter", "Hunter Builds"))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "refresh",
            "Reload loadouts from the database (Manage Server only)",
        ))
    }

    pub async fn run<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
        parent_token: &str,
        home_guild: GuildId,
    ) -> Result<()> {
        let (name, sub_options) = parse_subcommand(options)?;

        if name == "refresh" {
            return Self::refresh(ctx, interaction, pool, home_guild).await;
        }

        let value: &str = sole_option(&mut sub_options.into_vec())?;

        let id: i32 =
            value.parse().map_err(|_err| CoreError::invalid_option("build"))?;

        let records = cached(pool).await?;
        let Some(record) = records.iter().find(|r| r.id == id) else {
            return Err(CoreError::missing_data("matching build").into());
        };

        let component =
            record.clone().into_component::<Data>(ctx, parent_token).await?;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(MessageFlags::IS_COMPONENTS_V2)
                        .components(vec![component]),
                ),
            )
            .await?;

        Ok(())
    }

    async fn refresh(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
        home_guild: GuildId,
    ) -> Result<()> {
        if interaction.guild_id != Some(home_guild) {
            return Err(DestinyError::NotHomeGuild);
        }

        let permissions =
            interaction.member.as_ref().and_then(|member| member.permissions);
        if !is_privileged(permissions) {
            return Err(DestinyError::NotPrivileged);
        }

        invalidate_cache().await;
        let records = cached(pool).await?;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(MessageFlags::EPHEMERAL)
                        .content(format!(
                            "Reloaded {} loadouts from the database.",
                            records.len()
                        )),
                ),
            )
            .await?;

        Ok(())
    }

    pub async fn autocomplete(
        http: &Http,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        pool: &PgPool,
    ) -> Result<()> {
        let (_, group) = parse_subcommand(interaction.data.options())?;
        let (class_name, _) = parse_subcommand(group)?;
        let Ok(class) = class_name.parse::<Class>() else {
            return Ok(());
        };

        let input = option.value.to_lowercase();
        let records = cached(pool).await?;

        let choices = records
            .iter()
            .filter(|r| {
                r.class == class && r.choice_label().to_lowercase().contains(&input)
            })
            .take(25)
            .map(|r| AutocompleteChoice::new(r.choice_label(), r.choice_value()))
            .collect::<Vec<_>>();

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

async fn resolve_emoji<T>(
    emoji_cache: &mut EmojiCache,
    ctx: &Context,
    parent_token: &str,
    mut f: impl FnMut(&EmojiCache) -> EmojiResult<T>,
) -> Result<T> {
    const MAX_ATTEMPTS: u8 = 10;

    for _ in 0..MAX_ATTEMPTS {
        match f(emoji_cache) {
            Ok(value) => return Ok(value),
            Err(name) => {
                emoji_cache.upload(ctx, parent_token, &name).await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            },
        }
    }

    Err(CoreError::missing_data(format!(
        "emoji unavailable after {MAX_ATTEMPTS} upload attempts"
    ))
    .into())
}

#[must_use]
pub fn is_privileged(permissions: Option<Permissions>) -> bool {
    permissions.is_some_and(Permissions::manage_guild)
}
