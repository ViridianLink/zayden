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
use zayden_app::entitlement::Tier;
use zayden_app::state::AppState;
use zayden_core::ctx::ModalCtx;
use zayden_core::{InvocationCtx, as_i64};

use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};
use crate::upload::{SaveUpload, UploadQuota};
use crate::{embeds, save};

pub(super) const MODAL_ID: &str = "palworld_save_upload";
const FILE_ID: &str = "save";

const MAX_FILES: u8 = 8;

pub(super) async fn open_modal(cx: &InvocationCtx<'_>, pool: &PgPool) -> Result<()> {
    let discord_id = as_i64(cx.interaction.user.id.get());
    let tier = cx.app.entitlements.user_tier(cx.interaction.user.id.get()).await;
    let quota = UploadQuota::for_tier(tier);

    if let Some(upload) = SaveUpload::select(pool, discord_id).await?
        && let Some(remaining) = upload.cooldown_remaining(quota.cooldown)
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
                            upsell_url(cx.app.as_ref(), tier),
                        )]),
                ),
            )
            .await?;
        return Ok(());
    }

    let file_upload =
        CreateFileUpload::new(FILE_ID).max_values(MAX_FILES).required(true);
    let modal =
        CreateModal::new(MODAL_ID, "Upload your world save").components(vec![
            CreateModalComponent::Label(CreateLabel::file_upload(
                "Level.sav — add your Players/<id>.sav for /palworld progress",
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
    let tier = cx.app.entitlements.user_tier(cx.interaction.user.id.get()).await;
    let quota = UploadQuota::for_tier(tier);

    if let Some(upload) = SaveUpload::select(pool, discord_id).await?
        && let Some(remaining) = upload.cooldown_remaining(quota.cooldown)
    {
        return respond(
            cx,
            embeds::upload_cooldown_component(
                &cooldown_label(remaining),
                upsell_url(cx.app.as_ref(), tier),
            ),
        )
        .await;
    }

    let attachments = find_attachments(cx.interaction);
    if attachments.is_empty() {
        return respond(
            cx,
            embeds::upload_invalid_component("No file was attached."),
        )
        .await;
    }

    let dir = client.uploads_dir().join(discord_id.to_string());
    let mut level_stored = false;
    let mut players_stored = 0usize;

    for attachment in attachments {
        if !attachment.filename.to_lowercase().ends_with(".sav") {
            return reject(
                cx,
                &format!("`{}` isn't a `.sav` file.", attachment.filename),
            )
            .await;
        }
        if u64::from(attachment.size) > quota.max_bytes {
            return reject(
                cx,
                &format!(
                    "`{}` is larger than {} MB.",
                    attachment.filename,
                    quota.max_megabytes()
                ),
            )
            .await;
        }

        let bytes =
            download(&cx.app.http, attachment.url.as_str(), quota.max_bytes).await?;

        let dir = dir.clone();
        let filename = attachment.filename.to_string();
        let name = filename.clone();
        let stored = tokio::task::spawn_blocking(move || store(&dir, &name, &bytes))
            .await
            .map_err(|e| PalworldError::Upload(format!("store task failed: {e}")))?;

        match stored {
            Ok(Stored::Level) => level_stored = true,
            Ok(Stored::Player | Stored::Storage) => players_stored += 1,
            Err(PalworldError::Upload(reason)) => {
                return reject(cx, &format!("`{filename}`: {reason}")).await;
            },
            Err(e) => return Err(e),
        }
    }

    if !level_stored {
        let existing = SaveUpload::select(pool, discord_id).await?;
        if existing.is_none_or(|u| u.is_expired()) {
            return reject(
                cx,
                "None of those files was a `Level.sav`. Upload your world's \
                 `Level.sav` - a player save alone can't be read.",
            )
            .await;
        }
    }

    let file_path = dir.join("Level.sav").to_string_lossy().into_owned();
    let stored = SaveUpload::upsert(pool, discord_id, &file_path).await?;
    let expires = format!("<t:{}:R>", stored.expires_at.to_jiff().as_second());
    respond(cx, embeds::upload_confirm_component(&expires, players_stored)).await
}

enum Stored {
    Level,
    Player,
    Storage,
}

fn store(dir: &std::path::Path, filename: &str, bytes: &[u8]) -> Result<Stored> {
    if save::validate_level(bytes).is_ok() {
        save::write_level_atomic(dir, bytes)?;
        return Ok(Stored::Level);
    }

    if let Ok(uid) = save::player::parse_player_uid(bytes) {
        save::write_player_atomic(dir, &uid, bytes)?;
        return Ok(Stored::Player);
    }

    match save::dps::parse(bytes) {
        Ok(stored) => {
            save::write_raw_player(dir, &storage_stem(filename, &stored), bytes)?;
            Ok(Stored::Storage)
        },
        Err(e) => Err(PalworldError::Upload(format!(
            "that isn't a readable Palworld world, player or Pal storage save \
             ({e})"
        ))),
    }
}

fn storage_stem(
    filename: &str,
    stored: &std::collections::HashMap<String, Vec<crate::model::OwnedPal>>,
) -> String {
    let stem = filename.trim_end_matches(".sav").trim_end_matches(".SAV");
    if let Some(uid) = stem.strip_suffix("_dps")
        && uid.len() == 32
        && uid.chars().all(|c| c.is_ascii_hexdigit())
    {
        return format!("{}_dps", uid.to_ascii_uppercase());
    }

    let owner = stored
        .keys()
        .min()
        .and_then(|uid| save::uid_to_filename(uid))
        .unwrap_or_else(|| "00000000000000000000000000000000".to_string());
    format!("{owner}_dps")
}

fn find_attachments(interaction: &ModalInteraction) -> Vec<&Attachment> {
    interaction
        .data
        .components
        .iter()
        .filter_map(|component| {
            let ModalComponent::Label(label) = component else {
                return None;
            };
            let LabelComponent::FileUpload(file_upload) = &label.component else {
                return None;
            };
            Some(&file_upload.values)
        })
        .flatten()
        .filter_map(|id| interaction.data.resolved.attachments.get(id))
        .collect()
}

async fn reject(cx: &ModalCtx<'_>, reason: &str) -> Result<()> {
    respond(cx, embeds::upload_invalid_component(reason)).await
}

async fn download(
    http: &reqwest::Client,
    url: &str,
    max_bytes: u64,
) -> Result<Vec<u8>> {
    let resp = http.get(url).send().await?.error_for_status()?;
    let too_large = || {
        PalworldError::Upload(format!(
            "That save is larger than {} MB.",
            max_bytes / (1024 * 1024)
        ))
    };

    if let Some(len) = resp.content_length()
        && len > max_bytes
    {
        return Err(too_large());
    }

    let bytes = resp.bytes().await?;
    if bytes.len() as u64 > max_bytes {
        return Err(too_large());
    }
    Ok(bytes.to_vec())
}

fn upsell_url(app: &AppState, tier: Tier) -> Option<&str> {
    (tier < Tier::Pro).then_some(app.upgrade_url.as_deref()).flatten()
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
