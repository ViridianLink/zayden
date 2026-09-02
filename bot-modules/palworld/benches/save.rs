use std::hint::black_box;
use std::path::{Path, PathBuf};

use criterion::{Criterion, criterion_group, criterion_main};
use palworld::save::{decompress, dps, extract, gvas, load_world};

const WORLDS: [&str; 2] = ["storage-world", "progressed-world"];

fn world(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// Container decompress -> GVAS decode -> character extraction, timed
/// separately so a regression can be attributed to one stage.
fn stages(c: &mut Criterion) {
    let raw =
        std::fs::read(world("storage-world").join("Level.sav")).unwrap_or_default();
    assert!(!raw.is_empty(), "storage-world/Level.sav fixture is missing");

    let decompressed = decompress::decompress(&raw).unwrap_or_default();
    assert!(!decompressed.is_empty(), "storage-world/Level.sav did not decompress");

    let level = match gvas::read_gvas(&decompressed) {
        Ok(level) => level,
        Err(e) => {
            eprintln!("save bench: storage-world/Level.sav did not decode: {e}");
            return;
        },
    };

    let mut group = c.benchmark_group("save/stages");
    group.bench_function("decompress", |b| {
        b.iter(|| decompress::decompress(black_box(&raw)));
    });
    group.bench_function("read_gvas", |b| {
        b.iter(|| gvas::read_gvas(black_box(&decompressed)));
    });
    group.bench_function("extract", |b| {
        b.iter(|| extract::extract(black_box(&level)));
    });
    group.finish();
}

/// The two public entry points. `dps_load_all` is the fan-out over
/// `Players/*_dps.sav`; `load_world` is that plus the whole `Level.sav` path.
/// Both read from disk inside the timed loop, as they do in production.
fn roster(c: &mut Criterion) {
    let mut group = c.benchmark_group("save/roster");
    for name in WORLDS {
        let dir = world(name);
        group.bench_function(format!("dps_load_all/{name}"), |b| {
            b.iter(|| dps::load_all(black_box(&dir)));
        });
        group.bench_function(format!("load_world/{name}"), |b| {
            b.iter(|| load_world(black_box(&dir)));
        });
    }
    group.finish();
}

criterion_group!(benches, stages, roster);
criterion_main!(benches);
