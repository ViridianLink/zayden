use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};

use jiff::{SignedDuration, Timestamp};
use sqlx::PgPool;
use zayden_app::AppError;
use zayden_app::modules::{
    Backing,
    ModuleDef,
    ModuleStore,
    command_modules,
    validate,
};

const GUILD: i64 = 1_234_567_890;

const fn joined() -> Timestamp {
    Timestamp::constant(1_750_000_000, 0)
}

const fn module(introduced: Option<Timestamp>) -> ModuleDef {
    ModuleDef {
        id: "test",
        label: "Test",
        description: "",
        backing: Backing::Commands,
        introduced,
    }
}

fn tagged_catalogue() -> BTreeMap<&'static str, Vec<Cow<'static, str>>> {
    command_modules().map(|m| (m.id, vec![Cow::Borrowed(m.id)])).collect()
}

#[test]
fn baseline_module_is_enabled_for_every_guild() {
    assert!(module(None).default_enabled(joined()));
}

#[test]
fn module_introduced_after_the_bot_joined_is_disabled() {
    let introduced = joined() + SignedDuration::from_hours(1);
    assert!(!module(Some(introduced)).default_enabled(joined()));
}

#[test]
fn module_introduced_before_the_bot_joined_is_enabled() {
    let introduced = joined() - SignedDuration::from_hours(1);
    assert!(module(Some(introduced)).default_enabled(joined()));
}

#[test]
fn module_introduced_as_the_bot_joined_is_enabled() {
    assert!(module(Some(joined())).default_enabled(joined()));
}

#[test]
fn validate_accepts_tags_matching_the_catalogue() {
    assert!(validate(&tagged_catalogue()).is_ok());
}

#[test]
fn validate_rejects_an_unknown_tag() {
    let mut tags = tagged_catalogue();
    tags.insert("nonexistent", vec![Cow::Borrowed("cmd")]);

    assert!(matches!(validate(&tags), Err(AppError::UnknownModule { .. })));
}

#[test]
fn validate_rejects_a_tag_on_a_settings_module() {
    let mut tags = tagged_catalogue();
    tags.insert("ai", vec![Cow::Borrowed("cmd")]);

    assert!(matches!(validate(&tags), Err(AppError::UnknownModule { .. })));
}

#[test]
fn validate_rejects_a_command_module_without_commands() {
    let mut tags = tagged_catalogue();
    tags.remove("music");

    assert!(matches!(
        validate(&tags),
        Err(AppError::ModuleWithoutCommands("music"))
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn first_seed_enables_every_module(pool: PgPool) -> sqlx::Result<()> {
    let states =
        ModuleStore::new(pool).seed(GUILD, joined(), &HashSet::new()).await?;

    assert_eq!(states.len(), command_modules().count());
    assert!(states.values().all(|enabled| *enabled));

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn first_seed_applies_imported_overrides(pool: PgPool) -> sqlx::Result<()> {
    let states = ModuleStore::new(pool)
        .seed(GUILD, joined(), &HashSet::from(["music"]))
        .await?;

    assert_eq!(states.get("music"), Some(&false));
    assert_eq!(states.get("gambling"), Some(&true));

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn guild_needs_import_until_seeded(pool: PgPool) -> sqlx::Result<()> {
    let store = ModuleStore::new(pool);

    assert!(store.needs_import(GUILD).await?);
    store.seed(GUILD, joined(), &HashSet::new()).await?;
    assert!(!store.needs_import(GUILD).await?);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn reseed_keeps_toggles(pool: PgPool) -> sqlx::Result<()> {
    let store = ModuleStore::new(pool);

    store.seed(GUILD, joined(), &HashSet::new()).await?;
    store.set(GUILD, "music", false).await?;
    let states = store.seed(GUILD, joined(), &HashSet::new()).await?;

    assert_eq!(states.get("music"), Some(&false));

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn reseed_ignores_imported_overrides(pool: PgPool) -> sqlx::Result<()> {
    let store = ModuleStore::new(pool);

    store.seed(GUILD, joined(), &HashSet::new()).await?;
    let states = store.seed(GUILD, joined(), &HashSet::from(["music"])).await?;

    assert_eq!(states.get("music"), Some(&true));

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn reinvite_resets_toggles(pool: PgPool) -> sqlx::Result<()> {
    let store = ModuleStore::new(pool);

    store.seed(GUILD, joined(), &HashSet::new()).await?;
    store.set(GUILD, "music", false).await?;
    let rejoined = joined() + SignedDuration::from_hours(24);
    let states = store.seed(GUILD, rejoined, &HashSet::from(["gambling"])).await?;

    assert_eq!(states.get("music"), Some(&true));
    assert_eq!(states.get("gambling"), Some(&true));

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn set_is_visible_through_the_cache(pool: PgPool) -> sqlx::Result<()> {
    let store = ModuleStore::new(pool);

    store.seed(GUILD, joined(), &HashSet::new()).await?;
    store.set(GUILD, "music", false).await?;
    assert_eq!(store.states(GUILD).await?.get("music"), Some(&false));

    store.set(GUILD, "music", true).await?;
    assert_eq!(store.states(GUILD).await?.get("music"), Some(&true));

    Ok(())
}
