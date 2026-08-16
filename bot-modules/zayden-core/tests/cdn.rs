//! Which links get sent for re-signing before they are downloaded.
//!
//! Discord attachment links expire about a day after upload and 404 on a
//! server-side fetch once they do, so missing one here means an image that
//! silently stops working. Sending a non-Discord link would leak the stored
//! URL to Discord's API for nothing, so the gate has to be exact in both
//! directions.

use zayden_core::is_discord_cdn;

#[test]
fn both_attachment_hosts_are_recognised() {
    for url in [
        "https://cdn.discordapp.com/attachments/1/2/a.png",
        "https://media.discordapp.net/attachments/1/2/a.png",
        "https://media.discordapp.net/attachments/1/2/a.png?ex=6a7f97aa&is=6a7e462a",
        "https://CDN.DiscordApp.com/attachments/1/2/a.png",
        "https://cdn.discordapp.com:443/attachments/1/2/a.png",
        "https://cdn.discordapp.com",
    ] {
        assert!(is_discord_cdn(url), "{url} is a Discord CDN link");
    }
}

#[test]
fn other_hosts_are_left_alone() {
    for url in [
        "https://example.com/a.png",
        "https://i.imgur.com/a.png",
        "https://media.tenor.com/a.gif",
        "http://cdn.discordapp.com/attachments/1/2/a.png",
    ] {
        assert!(!is_discord_cdn(url), "{url} must not be sent to Discord");
    }
}

/// The host is read up to the first delimiter rather than by substring, so a
/// link that merely mentions the CDN elsewhere is not mistaken for one.
#[test]
fn a_lookalike_host_is_not_a_discord_link() {
    for url in [
        "https://cdn.discordapp.com.evil.example/attachments/1/2/a.png",
        "https://evil.example/cdn.discordapp.com/a.png",
        "https://evil.example/?u=https://cdn.discordapp.com/a.png",
        "https://notcdn.discordapp.com/a.png",
    ] {
        assert!(!is_discord_cdn(url), "{url} is not a Discord CDN link");
    }
}
