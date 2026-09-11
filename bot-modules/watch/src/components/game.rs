use serenity::all::{
    CreateInputText,
    CreateInteractionResponse,
    CreateLabel,
    CreateModal,
    CreateModalComponent,
    InputTextStyle,
};
use zayden_core::{ComponentCtx, ModalCtx, parse_modal_components};

use crate::components::GUESS_MODAL_PREFIX;
use crate::error::{Result, WatchError};
use crate::games::round::RoundRow;
use crate::games::score::ScoreRow;

pub async fn trivia_answer(cx: &ComponentCtx<'_>, suffix: &str) -> Result<()> {
    let (round_id, index) = suffix
        .split_once(':')
        .ok_or_else(|| WatchError::Internal(format!("bad trivia id `{suffix}`")))?;

    let round_id: i64 = round_id
        .parse()
        .map_err(|_e| WatchError::Internal("bad round id".to_owned()))?;
    let index: usize = index
        .parse()
        .map_err(|_e| WatchError::Internal("bad choice index".to_owned()))?;

    let Some(round) = RoundRow::get(&cx.app.db, round_id).await? else {
        return Err(WatchError::RoundExpired);
    };

    let choices: Vec<String> = round
        .choices
        .clone()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let picked = choices
        .get(index)
        .ok_or_else(|| WatchError::Internal("choice out of range".to_owned()))?;

    settle(cx, &round, picked).await
}

async fn settle(
    cx: &ComponentCtx<'_>,
    round: &RoundRow,
    picked: &str,
) -> Result<()> {
    let guild_id = cx.interaction.guild_id.ok_or(WatchError::MissingGuildId)?;
    let user_id = cx.interaction.user.id;
    let username = cx.interaction.user.name.as_str();

    // Atomic: exactly one simultaneous click can claim the round.
    let Some(answer) =
        RoundRow::claim(&cx.app.db, round.id, user_id, username).await?
    else {
        return if round.is_expired() {
            Err(WatchError::RoundExpired)
        } else {
            Err(WatchError::RoundClosed)
        };
    };

    let correct = answer.eq_ignore_ascii_case(picked.trim());
    ScoreRow::record(&cx.app.db, guild_id, user_id, username, &round.game, correct)
        .await?;

    if correct {
        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    serenity::all::CreateInteractionResponseMessage::new()
                        .content(format!("<@{user_id}> got it — **{answer}**.")),
                ),
            )
            .await?;
    } else {
        // Wrong answers must not lock the round, or one fast miss ends the game.
        RoundRow::release(&cx.app.db, round.id).await?;
        cx.ephemeral("Not that one — someone else can still get it.").await?;
    }

    Ok(())
}

pub async fn open_guess_modal(cx: &ComponentCtx<'_>, suffix: &str) -> Result<()> {
    let modal =
        CreateModal::new(format!("{GUESS_MODAL_PREFIX}{suffix}"), "What is it?")
            .components(vec![CreateModalComponent::Label(CreateLabel::input_text(
                "Title",
                CreateInputText::new(InputTextStyle::Short, "answer")
                    .placeholder("Your guess")
                    .required(true),
            ))]);

    cx.interaction
        .create_response(&cx.ctx.http, CreateInteractionResponse::Modal(modal))
        .await?;

    Ok(())
}

pub async fn guess_submit(cx: &ModalCtx<'_>, suffix: &str) -> Result<()> {
    let round_id: i64 = suffix
        .parse()
        .map_err(|_e| WatchError::Internal(format!("bad round id `{suffix}`")))?;

    let mut inputs =
        parse_modal_components(cx.interaction.data.components.as_slice());
    let guess = inputs
        .remove("answer")
        .and_then(|mut v| v.pop())
        .unwrap_or_default()
        .to_string();

    let Some(round) = RoundRow::get(&cx.app.db, round_id).await? else {
        return Err(WatchError::RoundExpired);
    };

    let guild_id = cx.interaction.guild_id.ok_or(WatchError::MissingGuildId)?;
    let user_id = cx.interaction.user.id;
    let username = cx.interaction.user.name.as_str();

    let Some(answer) =
        RoundRow::claim(&cx.app.db, round_id, user_id, username).await?
    else {
        return if round.is_expired() {
            Err(WatchError::RoundExpired)
        } else {
            Err(WatchError::RoundClosed)
        };
    };

    let correct = answer.trim().eq_ignore_ascii_case(guess.trim());
    ScoreRow::record(&cx.app.db, guild_id, user_id, username, &round.game, correct)
        .await?;

    let content = if correct {
        format!("<@{user_id}> got it — **{answer}**.")
    } else {
        RoundRow::release(&cx.app.db, round_id).await?;
        format!("`{guess}` is not it. Still open.")
    };

    cx.interaction
        .create_response(
            &cx.ctx.http,
            CreateInteractionResponse::Message(
                serenity::all::CreateInteractionResponseMessage::new()
                    .content(content),
            ),
        )
        .await?;

    Ok(())
}
