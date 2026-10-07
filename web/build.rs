use topcoat::tailwind::BuildConfig;

const TAILWIND_CLI: &str = "TAILWIND_CLI";
const TAILWIND_VERSION: &str = "4.3.2";
const TAILWIND_LINUX_X64_SHA256: &str =
    "sha256:5036c4fb4328e0bcdbb6065c70d8ac9452e0d4c947113a788a8f94fd390425c1";

fn main() {
    let config = BuildConfig::new().input("styles.css");
    let config =
        if std::env::var_os(TAILWIND_CLI).is_some_and(|value| !value.is_empty()) {
            config.executable_env(TAILWIND_CLI)
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            config.version_checksum(TAILWIND_VERSION, TAILWIND_LINUX_X64_SHA256)
        } else {
            config.version(TAILWIND_VERSION)
        };

    if let Err(error) = config.render() {
        println!("cargo::error=tailwind failed: {error}");
    }

    println!("cargo::rerun-if-env-changed={TAILWIND_CLI}");
    println!("cargo::rerun-if-changed=styles.css");
    println!("cargo::rerun-if-changed=style");
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=assets");
}
