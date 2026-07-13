//! Live smoke test against the real Paldex source and (best-effort) scrape
//! sources. Skipped unless `PALWORLD_LIVE` is set, so CI stays offline.

use palworld::client::PalworldClient;

#[tokio::test]
async fn live_pipeline_fetches_and_breeds() {
    if std::env::var("PALWORLD_LIVE").is_err() {
        return;
    }

    let client = PalworldClient::new(reqwest::Client::new(), None, None, None);

    let pals = client.pals().await.expect("live pals fetch");
    assert!(pals.iter().any(|p| p.name == "Lamball"), "Lamball present");

    let index = client.breeding_index().await.expect("live breeding fetch");
    // PalCalc keys pals by internal name; Lamball × Lamball → Lamball.
    assert_eq!(index.breed("SheepBall", "SheepBall"), Some("SheepBall"));

    // Full detail path, including graceful-degradation enrichment.
    let lamball = client.pal("SheepBall").await.expect("live pal detail");
    assert_eq!(lamball.name, "Lamball");
    assert!(lamball.description.is_some());

    let items = client.items().await.expect("live items");
    assert!(items.iter().any(|i| i.name == "Gold Coin"));

    let passives = client.passives().await.expect("live passives");
    assert!(passives.iter().any(|p| p.key == "artisan"));
}
