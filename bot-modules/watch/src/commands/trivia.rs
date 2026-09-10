use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::identity::JellyfinLinkRow;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateEmbed,
    EditInteractionResponse,
    ResolvedValue,
};
use zayden_core::{InvocationCtx, optional_option, required_option};

use crate::components::TRIVIA_ANSWER_PREFIX;
use crate::embeds::COLOUR;
use crate::error::{Result, WatchError};
use crate::games::question::history::{self, Tier};
use crate::games::round::{NewRound, RoundRow};

pub const GAME: &str = "trivia";

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let scope: &str = required_option(&mut options, "scope")?;
    let tier =
        Tier::parse(optional_option(&mut options, "tier").unwrap_or("recent"));

    let guild_id = cx.interaction.guild_id.ok_or(WatchError::MissingGuildId)?;
    let channel_id = cx.interaction.channel_id;

    cx.interaction.defer(&cx.ctx.http).await?;

    // "me" reads the caller's own history; "server" reads an aggregate. There is
    // deliberately no third option that targets a named member.
    let plays = if scope == "me" {
        let link =
            JellyfinLinkRow::require(&cx.app.db, cx.interaction.user.id).await?;
        history::own_history(runtime, &link.jellyfin_user_id).await?
    } else {
        history::server_history(runtime).await?
    };

    let Some(question) = history::build(&cx.app.db, &plays, tier).await? else {
        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(
                    "There is not enough watch history yet to build a question.",
                ),
            )
            .await?;
        return Ok(());
    };

    let choices = serde_json::to_value(&question.choices)
        .map_err(|e| WatchError::Internal(e.to_string()))?;

    let round_id = NewRound {
        guild_id,
        channel_id,
        started_by: cx.interaction.user.id,
        game: GAME,
        kind: scope,
        answer: &question.answer,
        choices: Some(choices),
    }
    .open(&cx.app.db)
    .await?;

    let buttons = question
        .choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            CreateButton::new(format!("{TRIVIA_ANSWER_PREFIX}{round_id}:{index}"))
                .label(truncate_label(choice))
                .style(ButtonStyle::Secondary)
        })
        .collect::<Vec<_>>();

    let message = cx
        .interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .embed(
                    CreateEmbed::new()
                        .title("Trivia")
                        .colour(COLOUR)
                        .description(question.prompt),
                )
                .components(vec![CreateComponent::ActionRow(
                    CreateActionRow::buttons(buttons),
                )]),
        )
        .await?;

    RoundRow::set_message(&cx.app.db, round_id, message.id).await?;

    Ok(())
}

fn truncate_label(label: &str) -> String {
    if label.chars().count() <= 80 {
        return label.to_owned();
    }
    label.chars().take(79).chain(std::iter::once('…')).collect()
}
