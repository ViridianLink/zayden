pub mod dispatch_map;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use dispatch_map::DispatchMap;
pub use dispatch_map::OverlapError;
use serenity::all::{
    CommandInteraction,
    ComponentInteraction,
    Context,
    CreateCommand,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    GuildId,
    ModalInteraction,
};
use tracing::warn;
use zayden_app::entitlement::{EntitlementScope, Tier};
use zayden_app::state::AppState;
use zayden_core::ctx::{AutocompleteCtx, ComponentCtx, InvocationCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{
    ModuleAutocomplete,
    ModuleCommand,
    ModuleComponent,
    ModuleModal,
};
use zayden_core::scope::CommandScope;

/// Mutable builder used at startup to register all module handlers.
///
/// Consume via [`RegistryBuilder::build`] to produce a frozen
/// [`CommandRegistry`].
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
    #[must_use]
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

    pub fn add_component(
        &mut self,
        comp: impl ModuleComponent + 'static,
    ) -> Result<&mut Self, OverlapError> {
        let id_match = comp.id_match();
        self.components.insert(id_match, Arc::new(comp))?;
        Ok(self)
    }

    pub fn add_modal(
        &mut self,
        modal: impl ModuleModal + 'static,
    ) -> Result<&mut Self, OverlapError> {
        let id_match = modal.id_match();
        self.modals.insert(id_match, Arc::new(modal))?;
        Ok(self)
    }

    pub fn add_autocomplete(
        &mut self,
        auto: impl ModuleAutocomplete + 'static,
    ) -> &mut Self {
        let cmd = auto.command();
        self.autocompletes.insert(cmd, Arc::new(auto));
        self
    }

    #[must_use]
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
    /// Return the slash-command definitions that should be registered for
    /// `guild_id`, honouring each command's [`CommandScope`].
    #[must_use]
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

    #[must_use]
    pub fn commands_by_module(
        &self,
    ) -> BTreeMap<&'static str, Vec<Cow<'static, str>>> {
        let mut map: BTreeMap<&'static str, Vec<Cow<'static, str>>> =
            BTreeMap::new();

        for cmd in self.commands.values() {
            if let Some(module) = cmd.module() {
                map.entry(module).or_default().push(cmd.name());
            }
        }

        map
    }

    pub async fn run_command(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        app: Arc<AppState>,
    ) -> Option<Result<(), HandlerError>> {
        let cmd = Arc::clone(self.commands.get(interaction.data.name.as_str())?);

        // Entitlement gate
        let required = cmd.metadata().required_tier;
        if required != Tier::Free {
            let scope = interaction.guild_id.map_or_else(
                || EntitlementScope::User(interaction.user.id.get()),
                |gid| {
                    EntitlementScope::UserInGuild(
                        interaction.user.id.get(),
                        gid.get(),
                    )
                },
            );
            if !app.entitlements.allows(scope, required).await {
                let upgrade_url = app
                    .upgrade_url
                    .as_deref()
                    .unwrap_or("https://ko-fi.com/zaydenbot");

                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content(format!(
                            "This command requires a premium subscription. Upgrade at {upgrade_url}"
                        )),
                );
                if let Err(e) =
                    interaction.create_response(&ctx.http, response).await
                {
                    warn!(?e, "failed to send upgrade prompt");
                }
                return Some(Ok(()));
            }
        }

        let cx = InvocationCtx { ctx, interaction, app };
        Some(cmd.run(&cx).await)
    }

    pub async fn run_component(
        &self,
        ctx: &Context,
        interaction: &ComponentInteraction,
        app: Arc<AppState>,
    ) -> Option<Result<(), HandlerError>> {
        let comp = Arc::clone(self.components.lookup(&interaction.data.custom_id)?);
        let cx = ComponentCtx { ctx, interaction, app };
        Some(comp.run(&cx).await)
    }

    pub async fn run_modal(
        &self,
        ctx: &Context,
        interaction: &ModalInteraction,
        app: Arc<AppState>,
    ) -> Option<Result<(), HandlerError>> {
        let modal = Arc::clone(self.modals.lookup(&interaction.data.custom_id)?);
        let cx = ModalCtx { ctx, interaction, app };
        Some(modal.run(&cx).await)
    }

    pub async fn run_autocomplete(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        app: Arc<AppState>,
    ) -> Option<Result<(), HandlerError>> {
        let auto =
            Arc::clone(self.autocompletes.get(interaction.data.name.as_str())?);
        let cx = AutocompleteCtx { ctx, interaction, app };
        Some(auto.run(&cx).await)
    }
}
