use jiff::{SignedDuration, Timestamp};
use serenity::all::{
    Attachment,
    CreateComponent,
    CreateFileUpload,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateLabel,
    CreateModal,
    CreateModalComponent,
    EditInteractionResponse,
    LabelComponent,
    MessageFlags,
    ModalComponent,
    ModalInteraction,
};
use sqlx::PgPool;
use zayden_core::ctx::ModalCtx;
use zayden_core::{InvocationCtx, as_i64};

use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};
use crate::upload::SaveUpload;
use crate::{embeds, save};

const MAX_UPLOAD_BYTES: u64 = 30 * 1024 * 1024;

pub(super) const MODAL_ID: &str = "palworld_save_upload";
const FILE_ID: &str = "save";

pub(super) async fn open_modal(cx: &InvocationCtx<'_>, pool: &PgPool) -> Result<()> {
    let discord_id = as_i64(cx.interaction.user.id.get());

    if let Some(upload) = SaveUpload::select(pool, discord_id).await?
        && let Some(remaining) = upload.cooldown_remaining()
    {
        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(
                            MessageFlags::IS_COMPONENTS_V2 | MessageFlags::EPHEMERAL,
                        )
                        .components(vec![embeds::upload_cooldown_component(
                            &cooldown_label(remaining),
                        )]),
                ),
            )
            .await?;
        return Ok(());
    }

    let file_upload = CreateFileUpload::new(FILE_ID).max_values(1).required(true);
    let modal =
        CreateModal::new(MODAL_ID, "Upload your Level.sav").components(vec![
            CreateModalComponent::Label(CreateLabel::file_upload(
                "Level.sav",
                file_upload,
            )),
        ]);

    cx.interaction
        .create_response(&cx.ctx.http, CreateInteractionResponse::Modal(modal))
        .await?;
    Ok(())
}

pub(super) async fn submit(
    cx: &ModalCtx<'_>,
    client: &PalworldClient,
    pool: &PgPool,
) -> Result<()> {
    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;
    let discord_id = as_i64(cx.interaction.user.id.get());

    if let Some(upload) = SaveUpload::select(pool, discord_id).await?
        && let Some(remaining) = upload.cooldown_remaining()
    {
        return respond(
            cx,
            embeds::upload_cooldown_component(&cooldown_label(remaining)),
        )
        .await;
    }

    let Some(attachment) = find_attachment(cx.interaction) else {
        return respond(
            cx,
            embeds::upload_invalid_component("No file was attached."),
        )
        .await;
    };

    if !attachment.filename.to_lowercase().ends_with(".sav") {
        return respond(
            cx,
            embeds::upload_invalid_component("That isn't a `.sav` file."),
        )
        .await;
    }
    if u64::from(attachment.size) > MAX_UPLOAD_BYTES {
        return respond(
            cx,
            embeds::upload_invalid_component("That save is larger than 30 MB."),
        )
        .await;
    }

    let bytes = download(&cx.app.http, attachment.url.as_str()).await?;

    let dir = client.uploads_dir().join(discord_id.to_string());
    let file_path = dir.join("Level.sav").to_string_lossy().into_owned();

    let result = tokio::task::spawn_blocking(move || -> Result<()> {
        save::validate_level(&bytes).map_err(|e| {
            PalworldError::Upload(format!(
                "that file isn't a readable Palworld save ({e})"
            ))
        })?;
        save::write_level_atomic(&dir, &bytes)
    })
    .await
    .map_err(|e| PalworldError::Upload(format!("store task failed: {e}")))?;

    if let Err(e) = result {
        if let PalworldError::Upload(reason) = &e {
            return respond(cx, embeds::upload_invalid_component(reason)).await;
        }
        return Err(e);
    }

    let stored = SaveUpload::upsert(pool, discord_id, &file_path).await?;
    let expires = format!("<t:{}:R>", stored.expires_at.to_jiff().as_second());
    respond(cx, embeds::upload_confirm_component(&expires)).await
}

fn find_attachment(interaction: &ModalInteraction) -> Option<&Attachment> {
    interaction.data.components.iter().find_map(|component| {
        let ModalComponent::Label(label) = component else {
            return None;
        };
        let LabelComponent::FileUpload(file_upload) = &label.component else {
            return None;
        };
        file_upload
            .values
            .iter()
            .find_map(|id| interaction.data.resolved.attachments.get(id))
    })
}

async fn download(http: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let resp = http.get(url).send().await?.error_for_status()?;

    if let Some(len) = resp.content_length()
        && len > MAX_UPLOAD_BYTES
    {
        return Err(PalworldError::Upload(
            "That save is larger than 30 MB.".to_string(),
        ));
    }

    let bytes = resp.bytes().await?;
    if bytes.len() as u64 > MAX_UPLOAD_BYTES {
        return Err(PalworldError::Upload(
            "That save is larger than 30 MB.".to_string(),
        ));
    }
    Ok(bytes.to_vec())
}

fn cooldown_label(remaining: SignedDuration) -> String {
    let unix = Timestamp::now()
        .checked_add(remaining)
        .map(Timestamp::as_second)
        .unwrap_or_default();
    format!("<t:{unix}:R>")
}

async fn respond(
    cx: &ModalCtx<'_>,
    component: CreateComponent<'static>,
) -> Result<()> {
    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .flags(MessageFlags::IS_COMPONENTS_V2)
                .components(vec![component]),
        )
        .await?;
    Ok(())
}
