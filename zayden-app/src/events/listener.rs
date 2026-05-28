/// Postgres LISTEN/NOTIFY listener that forwards config-change payloads to
/// the in-process `AppEvent` broadcast channel.
pub struct ConfigListener;
