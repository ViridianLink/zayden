use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::{fs, io};

fn main() {
    println!("cargo::rerun-if-changed=style");

    let mut files = Vec::new();
    // cargo-chef cooks dependencies against a skeleton source tree that has no
    // `style/`
    collect(Path::new("style"), &mut files).ok();
    files.sort();

    let mut hasher = DefaultHasher::new();
    for path in files {
        path.hash(&mut hasher);
        if let Ok(bytes) = fs::read(&path) {
            bytes.hash(&mut hasher);
        }
    }

    println!("cargo::rustc-env=STYLE_HASH={:016x}", hasher.finish());
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}
