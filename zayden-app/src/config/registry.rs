use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::broadcast;

use super::SettingsStore;
use super::tables::{
    ChannelsSettingsRow,
    FamilySettingsRow,
    LfgSettingsRow,
    MusicSettingsRow,
    RolesSettingsRow,
    SuggestionsSettingsRow,
    SupportSettingsRow,
    TempVoiceSettingsRow,
    TicketSettingsRow,
};
use crate::events::AppEvent;

pub struct SettingsRegistry {
    pub support: Arc<SettingsStore<SupportSettingsRow>>,
    pub suggestions: Arc<SettingsStore<SuggestionsSettingsRow>>,
    pub channels: Arc<SettingsStore<ChannelsSettingsRow>>,
    pub roles: Arc<SettingsStore<RolesSettingsRow>>,
    pub temp_voice: Arc<SettingsStore<TempVoiceSettingsRow>>,
    pub lfg: Arc<SettingsStore<LfgSettingsRow>>,
    pub music: Arc<SettingsStore<MusicSettingsRow>>,
    pub ticket: Arc<SettingsStore<TicketSettingsRow>>,
    pub family: Arc<SettingsStore<FamilySettingsRow>>,
}

impl SettingsRegistry {
    #[must_use]
    pub fn new(db: PgPool, events: &broadcast::Sender<AppEvent>) -> Self {
        let support = Arc::new(SettingsStore::new(db.clone(), events.clone()));
        let suggestions = Arc::new(SettingsStore::new(db.clone(), events.clone()));
        let channels = Arc::new(SettingsStore::new(db.clone(), events.clone()));
        let roles = Arc::new(SettingsStore::new(db.clone(), events.clone()));
        let temp_voice = Arc::new(SettingsStore::new(db.clone(), events.clone()));
        let lfg = Arc::new(SettingsStore::new(db.clone(), events.clone()));
        let music = Arc::new(SettingsStore::new(db.clone(), events.clone()));
        let ticket = Arc::new(SettingsStore::new(db.clone(), events.clone()));
        let family = Arc::new(SettingsStore::new(db, events.clone()));

        SettingsStore::spawn_invalidator(Arc::clone(&support), events.subscribe());
        SettingsStore::spawn_invalidator(
            Arc::clone(&suggestions),
            events.subscribe(),
        );
        SettingsStore::spawn_invalidator(Arc::clone(&channels), events.subscribe());
        SettingsStore::spawn_invalidator(Arc::clone(&roles), events.subscribe());
        SettingsStore::spawn_invalidator(
            Arc::clone(&temp_voice),
            events.subscribe(),
        );
        SettingsStore::spawn_invalidator(Arc::clone(&lfg), events.subscribe());
        SettingsStore::spawn_invalidator(Arc::clone(&music), events.subscribe());
        SettingsStore::spawn_invalidator(Arc::clone(&ticket), events.subscribe());
        SettingsStore::spawn_invalidator(Arc::clone(&family), events.subscribe());

        Self {
            support,
            suggestions,
            channels,
            roles,
            temp_voice,
            lfg,
            music,
            ticket,
            family,
        }
    }
}
