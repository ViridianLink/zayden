use bungie_api::BungieClient;

pub trait BungieClientData: Send + Sync + 'static {
    fn bungie_client(&self) -> &BungieClient;
}
