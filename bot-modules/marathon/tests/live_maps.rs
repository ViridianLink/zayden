//! Live, network-dependent smoke test for the map cross-referencing pipeline.
//! Ignored by default (it hits `MarathonDB` + `MapGenie`); run explicitly with:
//! `cargo test -p marathon --test live_maps -- --ignored --nocapture`

use marathon::client::MarathonClient;

#[tokio::test]
#[ignore = "hits live MarathonDB + MapGenie APIs"]
async fn maps_cross_reference_covers_all_four() {
    let client = MarathonClient::new(reqwest::Client::new(), None);

    let maps = client.maps().await.expect("maps() should succeed");
    let slugs: Vec<&str> = maps.iter().map(|m| m.slug.as_str()).collect();
    println!("resolved {} maps: {slugs:?}", maps.len());

    for map in maps.iter() {
        println!(
            "  {:<14} name={:<14} pois={:>2} exfil={:>2} keycard={:>2} event={:>2}",
            map.slug,
            map.name,
            map.pois.len(),
            map.extractions.len(),
            map.keycard_rooms.len(),
            map.event_spawns.len(),
        );
    }

    for want in ["perimeter", "dire-marsh", "outpost", "cryo-archive"] {
        assert!(
            maps.iter().any(|m| m.slug == want),
            "cross-reference is missing map `{want}`"
        );
    }

    // Outpost was the concrete gap: MarathonDB has it only as an empty stub, so
    // its POIs must now come from MapGenie via the merge.
    let outpost = maps.iter().find(|m| m.slug == "outpost").unwrap();
    assert!(
        !outpost.pois.is_empty()
            || !outpost.extractions.is_empty()
            || !outpost.keycard_rooms.is_empty(),
        "outpost should carry location data after cross-referencing"
    );
}
