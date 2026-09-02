use std::fs;
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use marathon::parse;
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(Value::Null)
}

fn map_parse(c: &mut Criterion) {
    let taxonomy = fixture("mapgenie_manifest");
    let data = fixture("mapgenie_map_outpost");
    assert!(!data.is_null(), "mapgenie_map_outpost.json fixture is missing");

    c.bench_function("parse/mapgenie_map_to_model", |b| {
        b.iter(|| {
            parse::mapgenie_map_to_model(
                black_box("outpost"),
                black_box(&taxonomy),
                black_box(&data),
            )
        });
    });
}

criterion_group!(benches, map_parse);
criterion_main!(benches);
