use std::fs;
use std::path::Path;

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn locked_version(lock: &str, package: &str) -> Option<String> {
    let header = format!("name = \"{package}\"");
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == header {
            let version = lines.next()?.trim().strip_prefix("version = \"")?;
            return version.strip_suffix('"').map(str::to_owned);
        }
    }
    None
}

fn documented_version(readme: &str) -> Option<String> {
    readme
        .lines()
        .find_map(|line| line.trim().strip_prefix("Verified against: topcoat "))
        .map(|version| version.trim().to_owned())
}

#[test]
fn the_topcoat_reference_matches_the_locked_version() {
    let root = Path::new(CRATE_ROOT).join("..");
    let lock = fs::read_to_string(root.join("Cargo.lock")).unwrap();
    let readme =
        fs::read_to_string(root.join("design-docs/topcoat/README.md")).unwrap();

    let locked = locked_version(&lock, "topcoat").unwrap();
    let documented = documented_version(&readme).unwrap();

    assert_eq!(
        documented, locked,
        "design-docs/topcoat/ documents topcoat {documented} but Cargo.lock has {locked}; follow the update steps in design-docs/topcoat/README.md"
    );
}
