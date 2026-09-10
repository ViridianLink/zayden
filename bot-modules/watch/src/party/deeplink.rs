use jellyfin::transport::JellyfinClient;

use crate::party::row::PartyRow;

#[must_use]
pub fn join_guide(client: &JellyfinClient, party: &PartyRow) -> String {
    format!(
        "**{}**\n\
         1. Open {}\n\
         2. Sign in with the credentials above, then open the title: {}\n\
         3. When the host says go, open the cast menu and join their SyncPlay \
         group so everyone stays in step.\n\n\
         Your access is limited to this one title and disappears a couple of \
         hours after the party.",
        party.item_name,
        client.base_url(),
        client.item_url(&party.item_id),
    )
}

#[must_use]
pub fn watch_link(client: &JellyfinClient, party: &PartyRow) -> String {
    client.item_url(&party.item_id)
}
