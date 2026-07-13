mod direct;
mod members;
mod modals;
mod privacy;
mod region;

use serenity::all::{
    ButtonStyle,
    ChannelId,
    CreateActionRow,
    CreateButton,
    GenericChannelId,
    UserId,
};
use sqlx::{Database, Pool};

use crate::{
    Result,
    TempVoiceError,
    VoiceChannelManager,
    VoiceChannelRow,
    VoiceStateCache,
};

pub const CLAIM: &str = "voice_claim";
pub const TRANSFER: &str = "voice_transfer";
pub const TRUST: &str = "voice_trust";
pub const KICK: &str = "voice_kick";
pub const DELETE: &str = "voice_delete";
pub const RENAME: &str = "voice_rename";
pub const LIMIT: &str = "voice_limit";
pub const BITRATE: &str = "voice_bitrate";
pub const PASSWORD: &str = "voice_password";
pub const PRIVACY: &str = "voice_privacy";
pub const REGION: &str = "voice_region";

pub const TRANSFER_MENU: &str = "voice_transfer_menu";
pub const TRUST_MENU: &str = "voice_trust_menu";
pub const KICK_MENU: &str = "voice_kick_menu";
pub const PRIVACY_MENU: &str = "voice_privacy_menu";
pub const REGION_MENU: &str = "voice_region_menu";

// TODO: Can regions be pulled from Discord API to avoid future drift
pub const REGIONS: &[(&str, &str)] = &[
    ("Automatic", "automatic"),
    ("Brazil", "brazil"),
    ("Hong Kong", "hongkong"),
    ("India", "india"),
    ("Japan", "japan"),
    ("Rotterdam", "rotterdam"),
    ("Russia", "russia"),
    ("Singapore", "singapore"),
    ("South Africa", "southafrica"),
    ("Sydney", "sydney"),
    ("US Central", "us-central"),
    ("US East", "us-east"),
    ("US South", "us-south"),
    ("US West", "us-west"),
];

pub const PRIVACIES: &[(&str, &str)] = &[
    ("Open", "open"),
    ("Spectator", "spectator"),
    ("Lock", "lock"),
    ("Invisible", "invisible"),
];

pub struct Components;

#[must_use]
pub fn build_panel() -> Vec<CreateActionRow<'static>> {
    vec![
        CreateActionRow::buttons(vec![
            CreateButton::new(RENAME).label("Rename").style(ButtonStyle::Secondary),
            CreateButton::new(LIMIT).label("Limit").style(ButtonStyle::Secondary),
            CreateButton::new(BITRATE)
                .label("Bitrate")
                .style(ButtonStyle::Secondary),
            CreateButton::new(PASSWORD)
                .label("Password")
                .style(ButtonStyle::Secondary),
        ]),
        CreateActionRow::buttons(vec![
            CreateButton::new(PRIVACY)
                .label("Privacy")
                .style(ButtonStyle::Secondary),
            CreateButton::new(REGION).label("Region").style(ButtonStyle::Secondary),
        ]),
        CreateActionRow::buttons(vec![
            CreateButton::new(CLAIM).label("Claim").style(ButtonStyle::Success),
            CreateButton::new(TRANSFER)
                .label("Transfer")
                .style(ButtonStyle::Primary),
            CreateButton::new(TRUST).label("Trust").style(ButtonStyle::Primary),
            CreateButton::new(KICK).label("Kick").style(ButtonStyle::Danger),
            CreateButton::new(DELETE).label("Delete").style(ButtonStyle::Danger),
        ]),
    ]
}

pub async fn resolve_target_channel<
    Db: Database,
    Manager: VoiceChannelManager<Db>,
>(
    pool: &Pool<Db>,
    voice_states: &VoiceStateCache,
    channel_id: GenericChannelId,
    user_id: UserId,
) -> Result<(ChannelId, VoiceChannelRow)> {
    let channel_id = channel_id.expect_channel();

    if let Some(row) = Manager::get(pool, channel_id).await? {
        return Ok((channel_id, row));
    }

    if let Some(current) = voice_states.current_channel(user_id)
        && let Some(row) = Manager::get(pool, current).await?
    {
        return Ok((current, row));
    }

    Err(TempVoiceError::MemberNotInVoiceChannel)
}
