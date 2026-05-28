/// Single read/write entry point for guild configuration.
///
/// Caches config in-memory and emits `AppEvent::ConfigChanged` on writes.
pub struct ConfigStore;
