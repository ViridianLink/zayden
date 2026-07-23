mod commands;
mod components;
pub use commands::Voice;

pub mod events;

use components::{
    VoiceBitrate,
    VoiceBitrateModal,
    VoiceClaim,
    VoiceDelete,
    VoiceKick,
    VoiceKickMenu,
    VoiceLimit,
    VoiceLimitModal,
    VoicePassword,
    VoicePasswordModal,
    VoicePrivacy,
    VoicePrivacyMenu,
    VoiceRegion,
    VoiceRegionMenu,
    VoiceRename,
    VoiceRenameModal,
    VoiceTransfer,
    VoiceTransferMenu,
    VoiceTrust,
    VoiceTrustMenu,
};

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Voice)
        .add_component(VoiceClaim)?
        .add_component(VoiceDelete)?
        .add_component(VoiceRename)?
        .add_component(VoiceLimit)?
        .add_component(VoiceBitrate)?
        .add_component(VoicePassword)?
        .add_component(VoicePrivacy)?
        .add_component(VoiceRegion)?
        .add_component(VoiceTransfer)?
        .add_component(VoiceTrust)?
        .add_component(VoiceKick)?
        .add_component(VoicePrivacyMenu)?
        .add_component(VoiceRegionMenu)?
        .add_component(VoiceTransferMenu)?
        .add_component(VoiceTrustMenu)?
        .add_component(VoiceKickMenu)?
        .add_modal(VoiceRenameModal)?
        .add_modal(VoiceLimitModal)?
        .add_modal(VoiceBitrateModal)?
        .add_modal(VoicePasswordModal)?;

    Ok(())
}
