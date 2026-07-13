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
    Http,
    MessageFlags,
    ResolvedOption,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use zayden_core::{
    CoreError,
    EmojiCache,
    EmojiCacheData,
    EmojiResult,
    SubCommandOptions,
    parse_subcommand,
    sole_option,
};

use crate::Result;
use crate::db::loadouts as loadout_db;

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
    }

    pub async fn run<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
        parent_token: &str,
    ) -> Result<()> {
        let value: &str = {
            let options: SubCommandOptions<'_> = sole_option(&mut options)?;
            sole_option(&mut options.into_vec())?
        };

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

    pub async fn autocomplete(
        http: &Http,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        pool: &PgPool,
    ) -> Result<()> {
        // Path is `builds` (group) -> `<class>` (subcommand) -> `build`.
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

/// Retry-with-upload wrapper around an emoji-cache lookup: if the closure
/// reports a missing emoji, upload it and retry (bounded).
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
