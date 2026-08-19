use std::borrow::Cow;
use std::fmt::Write as _;

use async_trait::async_trait;
use family::commands::{
    Adopt,
    Block,
    Children,
    Divorce,
    Marry,
    Parents,
    Partner,
    Relationship,
    ResetFamily,
    Siblings,
    Tree,
    TreeImage,
    Unblock,
};
use family::{FamilyError, TreeQuota};
use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateAttachment,
    CreateButton,
    CreateCommand,
    CreateComponent,
    CreateEmbed,
    CreateEmbedFooter,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    EditInteractionResponse,
    Mentionable,
};
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;
use zayden_core::server_tier;

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(MarryCmd)
        .add_command(DivorceCmd)
        .add_command(AdoptCmd)
        .add_command(BlockCmd)
        .add_command(UnblockCmd)
        .add_command(ChildrenCmd)
        .add_command(ParentsCmd)
        .add_command(PartnerCmd)
        .add_command(SiblingsCmd)
        .add_command(RelationshipCmd)
        .add_command(ResetFamilyCmd)
        .add_command(TreeCmd)
        .add_component(MarryAccept)?
        .add_component(MarryDecline)?
        .add_component(AdoptAccept)?
        .add_component(AdoptDecline)?;

    Ok(())
}

fn proposal_buttons<'a>(
    accept_id: &'a str,
    decline_id: &'a str,
) -> CreateComponent<'a> {
    CreateComponent::ActionRow(CreateActionRow::buttons(vec![
        CreateButton::new(accept_id).label("Accept").style(ButtonStyle::Success),
        CreateButton::new(decline_id).label("Decline").style(ButtonStyle::Danger),
    ]))
}

pub struct MarryCmd;

#[async_trait]
impl ModuleCommand for MarryCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("marry")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Marry::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let target_id = Marry::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        let content = format!(
            "{}, {} wants to marry you! Do you accept?",
            target_id.mention(),
            cx.interaction.user.mention()
        );

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(content)
                        .components(vec![proposal_buttons(
                            "marry_accept",
                            "marry_decline",
                        )]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct DivorceCmd;

#[async_trait]
impl ModuleCommand for DivorceCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("divorce")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Divorce::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let partner_id = Divorce::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        let content = format!("You have divorced {}.", partner_id.mention());

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(content)
                        .ephemeral(true),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct AdoptCmd;

#[async_trait]
impl ModuleCommand for AdoptCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("adopt")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Adopt::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let target_id = Adopt::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        let content = format!(
            "{}, {} wants to adopt you! Do you accept?",
            target_id.mention(),
            cx.interaction.user.mention()
        );

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(content)
                        .components(vec![proposal_buttons(
                            "adopt_accept",
                            "adopt_decline",
                        )]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct BlockCmd;

#[async_trait]
impl ModuleCommand for BlockCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("block")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Block::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Block::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("User blocked.")
                        .ephemeral(true),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct UnblockCmd;

#[async_trait]
impl ModuleCommand for UnblockCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("unblock")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Unblock::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Unblock::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("User unblocked.")
                        .ephemeral(true),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct ChildrenCmd;

#[async_trait]
impl ModuleCommand for ChildrenCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("children")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Children::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let (user_id, names) =
            Children::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        let content =
            format!("{}'s children: {}", user_id.mention(), names.join(", "));

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct ParentsCmd;

#[async_trait]
impl ModuleCommand for ParentsCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("parents")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Parents::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let (user_id, names) =
            Parents::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        let content =
            format!("{}'s parents: {}", user_id.mention(), names.join(", "));

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct PartnerCmd;

#[async_trait]
impl ModuleCommand for PartnerCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("partner")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Partner::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let (user_id, names) =
            Partner::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        let content =
            format!("{}'s partners: {}", user_id.mention(), names.join(", "));

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct SiblingsCmd;

#[async_trait]
impl ModuleCommand for SiblingsCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("siblings")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Siblings::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let (user_id, names) =
            Siblings::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        let content =
            format!("{}'s siblings: {}", user_id.mention(), names.join(", "));

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct RelationshipCmd;

#[async_trait]
impl ModuleCommand for RelationshipCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("relationship")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Relationship::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let resp =
            Relationship::run(&cx.ctx.http, cx.interaction, &cx.app.db).await?;

        let content = format!(
            "{} and {} are: **{}**",
            resp.user_id.mention(),
            resp.other_id.mention(),
            resp.relationship
        );

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct ResetFamilyCmd;

#[async_trait]
impl ModuleCommand for ResetFamilyCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("resetfamily")
    }

    fn definition(&self) -> CreateCommand<'static> {
        ResetFamily::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        ResetFamily::run(cx.ctx, cx.interaction, &cx.app.db).await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Family trees have been reset.")
                        .ephemeral(true),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct TreeCmd;

#[async_trait]
impl ModuleCommand for TreeCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("tree")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Tree::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let guild_id = cx.interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        let tier = server_tier(&cx.ctx.http, &cx.app.entitlements, guild_id).await;

        let image =
            Tree::run(cx.interaction, &cx.app.db, &cx.app.http, tier).await?;

        let file = CreateAttachment::bytes(image.png.clone(), TREE_FILENAME);

        let mut embed = CreateEmbed::new()
            .title(format!("{}'s family tree", image.target_name))
            .colour(TREE_COLOUR)
            .image(
                format!("attachment://{TREE_FILENAME}"),
                Some(Cow::Owned(format!(
                    "A family tree diagram showing {} members related to {}.",
                    image.shown, image.target_name,
                ))),
            );

        if let Some(footer) = tree_footer(&image, cx.app.upgrade_url.as_deref()) {
            embed = embed.footer(CreateEmbedFooter::new(footer));
        }

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().new_attachment(file).embed(embed),
            )
            .await?;

        Ok(())
    }
}

const TREE_COLOUR: u32 = 0x0058_65f2;
const TREE_FILENAME: &str = "family-tree.png";

fn tree_footer(image: &TreeImage, upgrade_url: Option<&str>) -> Option<String> {
    if !image.is_collapsed() {
        return None;
    }

    let total = if image.truncated {
        format!("{}+", image.total)
    } else {
        image.total.to_string()
    };

    let mut footer = format!("Showing {} of {total} members", image.shown);

    if let Some((next, quota)) = TreeQuota::next_tier(image.tier) {
        let _ = write!(
            footer,
            " \u{00b7} {} renders up to {}",
            next.as_str(),
            quota.node_budget,
        );

        if let Some(url) = upgrade_url {
            let _ = write!(footer, " \u{00b7} {url}");
        }
    }

    Some(footer)
}

pub struct MarryAccept;

#[async_trait]
impl ModuleComponent for MarryAccept {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("marry_accept"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        family::components::marry::accept(cx.interaction, &cx.app.db).await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Congratulations! You are now married!")
                        .components(vec![]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct MarryDecline;

#[async_trait]
impl ModuleComponent for MarryDecline {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("marry_decline"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        family::components::marry::decline(cx.interaction)?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Marriage proposal declined.")
                        .components(vec![]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct AdoptAccept;

#[async_trait]
impl ModuleComponent for AdoptAccept {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("adopt_accept"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        family::components::adopt::accept(cx.interaction, &cx.app.db).await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Adoption accepted! Welcome to the family!")
                        .components(vec![]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct AdoptDecline;

#[async_trait]
impl ModuleComponent for AdoptDecline {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("adopt_decline"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        family::components::adopt::decline(cx.interaction)?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Adoption declined.")
                        .components(vec![]),
                ),
            )
            .await?;
        Ok(())
    }
}
