pub mod dispatch_map;
use dispatch_map::DispatchMap;

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, GuildId, ModalInteraction,
};
use tracing::warn;
use zayden_app::config::GuildConfig;
use zayden_app::entitlement::{EntitlementScope, Tier};
use zayden_app::state::AppState;
use zayden_core::ctx::{AutocompleteCtx, ComponentCtx, InvocationCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleAutocomplete, ModuleCommand, ModuleComponent, ModuleModal};
use zayden_core::scope::CommandScope;

/// Mutable builder used at startup to register all module handlers.
///
/// Consume via [`RegistryBuilder::build`] to produce a frozen [`CommandRegistry`].
pub struct RegistryBuilder {
    commands: HashMap<Cow<'static, str>, Arc<dyn ModuleCommand>>,
    components: DispatchMap<dyn ModuleComponent>,
    modals: DispatchMap<dyn ModuleModal>,
    autocompletes: HashMap<Cow<'static, str>, Arc<dyn ModuleAutocomplete>>,
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            components: DispatchMap::new(),
            modals: DispatchMap::new(),
            autocompletes: HashMap::new(),
        }
    }

    pub fn add_command(&mut self, cmd: impl ModuleCommand + 'static) -> &mut Self {
        let name = cmd.name();
        self.commands.insert(name, Arc::new(cmd));
        self
    }

    pub fn add_component(&mut self, comp: impl ModuleComponent + 'static) -> &mut Self {
        let id_match = comp.id_match();
        self.components.insert(id_match, Arc::new(comp));
        self
    }

    pub fn add_modal(&mut self, modal: impl ModuleModal + 'static) -> &mut Self {
        let id_match = modal.id_match();
        self.modals.insert(id_match, Arc::new(modal));
        self
    }

    pub fn add_autocomplete(&mut self, auto: impl ModuleAutocomplete + 'static) -> &mut Self {
        let cmd = auto.command();
        self.autocompletes.insert(cmd, Arc::new(auto));
        self
    }

    pub fn build(self) -> Arc<CommandRegistry> {
        Arc::new(CommandRegistry {
            commands: self.commands,
            components: self.components,
            modals: self.modals,
            autocompletes: self.autocompletes,
        })
    }
}

/// Immutable, frozen command registry shared by all handler tasks.
///
/// Constructed once at startup via [`RegistryBuilder::build`] and stored in
/// `Arc<CommandRegistry>` on the [`Handler`].
pub struct CommandRegistry {
    commands: HashMap<Cow<'static, str>, Arc<dyn ModuleCommand>>,
    components: DispatchMap<dyn ModuleComponent>,
    modals: DispatchMap<dyn ModuleModal>,
    autocompletes: HashMap<Cow<'static, str>, Arc<dyn ModuleAutocomplete>>,
}

impl CommandRegistry {
    /// Return the slash-command definitions that should be registered for `guild_id`,
    /// honouring each command's [`CommandScope`].
    pub fn definitions_for(&self, guild_id: GuildId) -> Vec<CreateCommand<'static>> {
        self.commands
            .values()
            .filter(|cmd| match cmd.scope() {
                CommandScope::Global => true,
                CommandScope::Guilds(ids) => ids.contains(&guild_id),
                CommandScope::ExcludeGuilds(ids) => !ids.contains(&guild_id),
            })
            .map(|cmd| cmd.definition())
            .collect()
    }

    /// Dispatch a slash-command interaction.
    ///
    /// Returns `None` if no handler is registered for the command name, allowing
    /// the caller to fall back to a legacy dispatch path during the M3.4–M3.5
    /// migration.  Returns `Some(result)` once a handler is found and invoked.
    pub async fn run_command(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        app: Arc<AppState>,
    ) -> Option<Result<(), HandlerError>> {
        let cmd = Arc::clone(self.commands.get(interaction.data.name.as_str())?);
        let guild_config = resolve_guild_config(&app, interaction.guild_id).await;

        // Entitlement gate — runs before the handler so modules cannot forget to check.
        let required = cmd.metadata().required_tier;
        if required != Tier::Free {
            let scope = match interaction.guild_id {
                Some(gid) => EntitlementScope::UserInGuild(interaction.user.id.get(), gid.get()),
                None => EntitlementScope::User(interaction.user.id.get()),
            };
            if !app.entitlements.allows(scope, required).await {
                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("This command requires a premium subscription. Upgrade at https://ko-fi.com/zaydenbot"),
                );
                if let Err(e) = interaction.create_response(&ctx.http, response).await {
                    warn!(?e, "failed to send upgrade prompt");
                }
                return Some(Ok(()));
            }
        }

        let cx = InvocationCtx {
            ctx,
            interaction,
            app,
            guild_config,
        };
        Some(cmd.run(&cx).await)
    }

    /// Dispatch a message-component interaction.
    ///
    /// Returns `None` if no handler matches the `custom_id`.
    pub async fn run_component(
        &self,
        ctx: &Context,
        interaction: &ComponentInteraction,
        app: Arc<AppState>,
    ) -> Option<Result<(), HandlerError>> {
        let comp = Arc::clone(self.components.lookup(&interaction.data.custom_id)?);
        let guild_config = resolve_guild_config(&app, interaction.guild_id).await;
        let cx = ComponentCtx {
            ctx,
            interaction,
            app,
            guild_config,
        };
        Some(comp.run(&cx).await)
    }

    /// Dispatch a modal-submit interaction.
    ///
    /// Returns `None` if no handler matches the `custom_id`.
    pub async fn run_modal(
        &self,
        ctx: &Context,
        interaction: &ModalInteraction,
        app: Arc<AppState>,
    ) -> Option<Result<(), HandlerError>> {
        let modal = Arc::clone(self.modals.lookup(&interaction.data.custom_id)?);
        let guild_config = resolve_guild_config(&app, interaction.guild_id).await;
        let cx = ModalCtx {
            ctx,
            interaction,
            app,
            guild_config,
        };
        Some(modal.run(&cx).await)
    }

    /// Dispatch an autocomplete interaction.
    ///
    /// Returns `None` if no handler is registered for the command name.
    pub async fn run_autocomplete(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        app: Arc<AppState>,
    ) -> Option<Result<(), HandlerError>> {
        let auto = Arc::clone(self.autocompletes.get(interaction.data.name.as_str())?);
        let guild_config = resolve_guild_config(&app, interaction.guild_id).await;
        let cx = AutocompleteCtx {
            ctx,
            interaction,
            app,
            guild_config,
        };
        Some(auto.run(&cx).await)
    }
}

/// Fetch or synthesise a [`GuildConfig`] for the given guild.
///
/// Falls back to an empty config (all `None` / zero) when the guild has no row
/// in the database yet, or when the interaction originates from a DM.
async fn resolve_guild_config(app: &AppState, guild_id: Option<GuildId>) -> Arc<GuildConfig> {
    let Some(gid) = guild_id else {
        return Arc::new(GuildConfig::empty(0));
    };

    match app.config_store.try_get(gid.get() as i64).await {
        Ok(Some(config)) => config,
        Ok(None) => Arc::new(GuildConfig::empty(gid.get() as i64)),
        Err(err) => {
            warn!(
                guild_id = %gid,
                error = ?err,
                "failed to fetch guild config; using empty config",
            );
            Arc::new(GuildConfig::empty(gid.get() as i64))
        }
    }
}
