mod access;
mod ownership;
mod privacy;
mod settings;

pub use access::{kick, password, trust};
pub use ownership::{claim, delete, transfer};
pub use privacy::privacy;
use serenity::all::UserId;
pub use settings::{bitrate, limit, region, rename};

use crate::error::PermissionError;
use crate::{Result, TempVoiceError, VoiceChannelRow};

fn require_trusted(row: &VoiceChannelRow, user_id: UserId) -> Result<()> {
    if row.is_trusted(user_id) {
        Ok(())
    } else {
        Err(TempVoiceError::MissingPermissions(PermissionError::NotTrusted))
    }
}

fn require_owner(row: &VoiceChannelRow, user_id: UserId) -> Result<()> {
    if row.is_owner(user_id) {
        Ok(())
    } else {
        Err(TempVoiceError::MissingPermissions(PermissionError::NotOwner))
    }
}
