use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::struct_property::StructPropertyValue;

use super::decompress;
use super::extract::{field, owned_pal, owner_uid, struct_fields};
use crate::error::Result;
use crate::model::OwnedPal;

pub type StoredPals = HashMap<String, Vec<OwnedPal>>;

#[must_use]
pub fn list_files(save_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(save_dir.join("Players")) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("_dps.sav"))
        })
        .collect()
}

pub async fn list_files_with_mtime(save_dir: &Path) -> Vec<(PathBuf, u64)> {
    let save_dir = save_dir.to_path_buf();

    tokio::task::spawn_blocking(move || {
        list_files(&save_dir)
            .into_iter()
            .filter_map(|path| {
                let meta = std::fs::metadata(&path).ok()?;
                Some((path, super::mtime_nanos(&meta)))
            })
            .collect()
    })
    .await
    .unwrap_or_else(|e| {
        tracing::warn!(error = %e, "palworld: Pal storage listing task failed");
        Vec::new()
    })
}

pub fn load_file(path: &Path) -> Result<StoredPals> {
    let raw = std::fs::read(path)?;
    parse(&raw)
}

#[must_use]
pub fn load_all(save_dir: &Path) -> StoredPals {
    let paths = list_files(save_dir);
    let mut out = StoredPals::new();

    std::thread::scope(|scope| {
        let handles: Vec<_> = paths
            .iter()
            .map(|path| scope.spawn(move || (path, load_file(path))))
            .collect();

        for handle in handles {
            let Ok((path, parsed)) = handle.join() else { continue };
            match parsed {
                Ok(pals) => {
                    for (uid, mut owned) in pals {
                        out.entry(uid).or_default().append(&mut owned);
                    }
                },
                Err(e) => tracing::warn!(
                    error = %e,
                    file = %path.display(),
                    "palworld: skipping unreadable Pal storage save",
                ),
            }
        }
    });

    out
}

pub fn parse(raw: &[u8]) -> Result<StoredPals> {
    let file = super::gvas::read_gvas(&decompress::decompress(raw)?)?;

    let Some(Property::ArrayProperty(ArrayProperty::Structs { structs, .. })) =
        file.properties.0.get("SaveParameterArray")
    else {
        return Ok(HashMap::new());
    };

    let mut out = StoredPals::new();
    for slot in structs {
        let StructPropertyValue::CustomStruct(fields) = slot else { continue };
        let Some(save_param) =
            field(fields, "SaveParameter").and_then(struct_fields)
        else {
            continue;
        };
        let (Some(owner), Some(pal)) =
            (owner_uid(save_param), owned_pal(save_param))
        else {
            continue;
        };
        out.entry(owner).or_default().push(pal);
    }

    Ok(out)
}
