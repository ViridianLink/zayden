use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    EditInteractionResponse,
};

use super::{PanelCtx, QUEUE_PAGER_PREFIX};
use crate::embeds;
use crate::error::{MusicError, Result};

pub struct QueuePager;

impl QueuePager {
    pub fn buttons(page: usize, total_pages: usize) -> CreateActionRow<'static> {
        let prev_page = page.saturating_sub(1);
        let next_page = (page + 1).min(total_pages.saturating_sub(1));

        CreateActionRow::buttons(vec![
            CreateButton::new(format!("{QUEUE_PAGER_PREFIX}{prev_page}"))
                .label("Previous")
                .style(ButtonStyle::Secondary)
                .disabled(page == 0),
            CreateButton::new(format!("{QUEUE_PAGER_PREFIX}{next_page}"))
                .label("Next")
                .style(ButtonStyle::Secondary)
                .disabled(page + 1 >= total_pages),
        ])
    }

    pub async fn run(ctx: &PanelCtx<'_>, page_suffix: &str) -> Result<()> {
        ctx.interaction.defer(ctx.http).await?;

        let page: usize = page_suffix.parse().unwrap_or(0);

        let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::QueueEmpty)?;
        let guard = player.lock().await;
        let current = guard.current.as_ref().map(|now| &now.track);
        let embed = embeds::queue_embed(&guard.queue, current, page);
        let total_pages = embeds::queue_page_count(guard.queue.len());
        drop(guard);

        ctx.interaction
            .edit_response(
                ctx.http,
                EditInteractionResponse::new().embed(embed).components(vec![
                    CreateComponent::ActionRow(Self::buttons(page, total_pages)),
                ]),
            )
            .await?;

        Ok(())
    }
}
