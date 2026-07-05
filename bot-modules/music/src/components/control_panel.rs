use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    EditInteractionResponse,
};
use songbird::tracks::PlayMode;

use super::{PanelCtx, CONTROL_PANEL_PREFIX};
use crate::embeds;
use crate::error::{MusicError, Result};
use crate::track::LoopMode;
use crate::voice;

pub struct ControlPanel;

impl ControlPanel {
    pub fn buttons() -> CreateActionRow<'static> {
        CreateActionRow::buttons(vec![
            CreateButton::new(format!("{CONTROL_PANEL_PREFIX}play_pause"))
                .label("Play/Pause")
                .style(ButtonStyle::Primary),
            CreateButton::new(format!("{CONTROL_PANEL_PREFIX}skip"))
                .label("Skip")
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("{CONTROL_PANEL_PREFIX}stop"))
                .label("Stop")
                .style(ButtonStyle::Danger),
            CreateButton::new(format!("{CONTROL_PANEL_PREFIX}loop"))
                .label("Loop")
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("{CONTROL_PANEL_PREFIX}shuffle"))
                .label("Shuffle")
                .style(ButtonStyle::Secondary),
        ])
    }

    pub async fn run(ctx: &PanelCtx<'_>, action: &str) -> Result<()> {
        ctx.interaction.defer(ctx.http).await?;

        match action {
            "play_pause" => Self::play_pause(ctx).await?,
            "skip" => Self::skip(ctx).await?,
            "stop" => Self::stop(ctx).await?,
            "loop" => Self::cycle_loop(ctx).await?,
            "shuffle" => Self::shuffle(ctx).await?,
            other => {
                return Err(MusicError::Internal(format!(
                    "unknown control panel action: {other}"
                )));
            },
        }

        Self::refresh(ctx).await
    }

    async fn play_pause(ctx: &PanelCtx<'_>) -> Result<()> {
        let settings = ctx.settings().await?;
        ctx.require_privileged(&settings)?;

        let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;
        let handle = {
            let guard = player.lock().await;
            let now = guard.current.as_ref().ok_or(MusicError::NothingPlaying)?;
            let handle = now.handle.clone();
            drop(guard);
            handle
        };

        let state = handle
            .get_info()
            .await
            .map_err(|e| MusicError::Songbird(e.to_string()))?;
        let result = if state.playing == PlayMode::Pause {
            handle.play()
        } else {
            handle.pause()
        };
        result.map_err(|e| MusicError::Songbird(e.to_string()))
    }

    async fn skip(ctx: &PanelCtx<'_>) -> Result<()> {
        let settings = ctx.settings().await?;
        ctx.require_privileged(&settings)?;

        let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;
        let (old_handle, next, generation) = {
            let mut guard = player.lock().await;
            guard.current.as_ref().ok_or(MusicError::NothingPlaying)?;
            let old_handle = guard.current.as_ref().map(|now| now.handle.clone());
            let next = guard.advance_queue();
            (old_handle, next, guard.generation)
        };

        voice::stop_current_and_start(
            &ctx.songbird,
            &ctx.music,
            &ctx.resolver,
            ctx.guild_id,
            old_handle,
            next,
            generation,
        )
        .await
    }

    async fn stop(ctx: &PanelCtx<'_>) -> Result<()> {
        let settings = ctx.settings().await?;
        ctx.require_privileged(&settings)?;

        voice::leave(&ctx.songbird, ctx.guild_id).await?;
        let _ = ctx.music.remove(ctx.guild_id);
        Ok(())
    }

    async fn cycle_loop(ctx: &PanelCtx<'_>) -> Result<()> {
        let settings = ctx.settings().await?;
        ctx.require_privileged(&settings)?;

        let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;
        let mut guard = player.lock().await;
        guard.loop_mode = match guard.loop_mode {
            LoopMode::Off => LoopMode::Track,
            LoopMode::Track => LoopMode::Queue,
            LoopMode::Queue => LoopMode::Off,
        };
        drop(guard);
        Ok(())
    }

    async fn shuffle(ctx: &PanelCtx<'_>) -> Result<()> {
        let settings = ctx.settings().await?;
        ctx.require_privileged(&settings)?;

        let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::QueueEmpty)?;
        player.lock().await.queue.shuffle();
        Ok(())
    }

    async fn refresh(ctx: &PanelCtx<'_>) -> Result<()> {
        let now_playing = match ctx.music.get(ctx.guild_id) {
            Some(player) => {
                let guard = player.lock().await;
                guard
                    .current
                    .as_ref()
                    .map(|now| embeds::now_playing_embed(now, guard.loop_mode))
            },
            None => None,
        };

        let response = now_playing.map_or_else(
            || {
                EditInteractionResponse::new()
                    .content("Nothing is playing.")
                    .embeds(vec![])
                    .components(vec![])
            },
            |embed| {
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(vec![CreateComponent::ActionRow(Self::buttons())])
            },
        );

        ctx.interaction.edit_response(ctx.http, response).await?;
        Ok(())
    }
}
