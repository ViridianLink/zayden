use std::sync::Arc;

use serde_json::Value;

use super::{MarathonClient, collect_candidate};
use crate::error::{MarathonError, Result};
use crate::model::{Attachment, Weapon};
use crate::source::SourceId;
use crate::{merge, parse};

impl MarathonClient {
    pub async fn weapon(&self, slug: &str) -> Result<Arc<Weapon>> {
        if let Some(cached) = self.weapon_cache.get(slug).await {
            return Ok(cached);
        }
        let weapon = self.fetch_weapon(slug).await?;
        let entry = Arc::new(weapon);
        self.weapon_cache.insert(slug.to_string(), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn fetch_weapon(&self, slug: &str) -> Result<Weapon> {
        let candidates = self.gather_weapon(slug).await;
        let mut weapon = merge::weapon(&candidates).ok_or_else(|| {
            MarathonError::NotFound { entity: "weapon", query: slug.to_string() }
        })?;

        if weapon.description.is_none() {
            weapon.description = self.fandom.description(&weapon.name).await;
        }
        Ok(weapon)
    }

    async fn gather_weapon(&self, slug: &str) -> Vec<(SourceId, Weapon)> {
        let (marathondb, mobalytics, cyberacme, tauceti, marathonmeta) = tokio::join!(
            self.marathondb_weapon(slug),
            self.mobalytics_weapon(slug),
            self.cyberacme_weapon(slug),
            self.tauceti_weapon(slug),
            self.marathonmeta_weapon(slug),
        );

        let mut out = Vec::new();
        collect_candidate(
            &mut out,
            SourceId::MarathonDb,
            marathondb,
            slug,
            "weapon",
        );
        collect_candidate(
            &mut out,
            SourceId::Mobalytics,
            mobalytics,
            slug,
            "weapon",
        );
        collect_candidate(&mut out, SourceId::CyberAcme, cyberacme, slug, "weapon");
        collect_candidate(&mut out, SourceId::TauCeti, tauceti, slug, "weapon");
        collect_candidate(
            &mut out,
            SourceId::MarathonMeta,
            marathonmeta,
            slug,
            "weapon",
        );
        out
    }

    async fn marathondb_weapon(&self, slug: &str) -> Result<Weapon> {
        let data = self.marathondb.weapon(slug).await?;
        Ok(parse::marathondb_weapon_to_model(slug, &data))
    }

    async fn mobalytics_weapon(&self, slug: &str) -> Result<Weapon> {
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let doc = mobalytics.fetch_document(&format!("weapons/{slug}")).await?;
        Ok(parse::parse_weapon(slug, &doc))
    }

    async fn cyberacme_weapon(&self, slug: &str) -> Result<Weapon> {
        let item = self.cyberacme.item(slug).await?;
        Ok(parse::cyberacme_item_to_weapon(slug, &item))
    }

    async fn tauceti_weapon(&self, slug: &str) -> Result<Weapon> {
        let Some(tauceti) = &self.tauceti else {
            return Err(MarathonError::SourceUnavailable);
        };
        let item = tauceti.weapon(slug).await?;
        Ok(parse::tauceti_item_to_weapon(slug, &item))
    }

    async fn marathonmeta_weapon(&self, slug: &str) -> Result<Weapon> {
        let Some(marathonmeta) = &self.marathonmeta else {
            return Err(MarathonError::SourceUnavailable);
        };
        let rendered = marathonmeta.weapon(slug).await?;
        Ok(parse::marathonmeta_html_to_weapon(slug, &rendered))
    }

    pub async fn weapons(&self) -> Result<Arc<[Weapon]>> {
        if let Some(cached) = self.weapon_list_cache.get(&()).await {
            return Ok(cached);
        }

        let slugs = self.weapon_slugs().await?;
        let mut weapons = Vec::with_capacity(slugs.len());
        for slug in &slugs {
            weapons.push((*self.weapon(slug).await?).clone());
        }

        let entry: Arc<[Weapon]> = weapons.into();
        self.weapon_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn weapon_slugs(&self) -> Result<Vec<String>> {
        match self.marathondb.weapons().await {
            Ok(items) => Ok(items
                .iter()
                .filter_map(|w| {
                    w.get("slug").and_then(Value::as_str).map(str::to_string)
                })
                .collect()),
            Err(err) => match &self.mobalytics {
                Some(mobalytics) => {
                    mobalytics.fetch_listing_slugs("weapons", "weapons").await
                },
                None => Err(err),
            },
        }
    }

    pub async fn attachments(&self) -> Result<Vec<Attachment>> {
        Ok(flatten_attachments(&self.weapons().await?))
    }

    pub async fn attachment(&self, slug: &str) -> Result<Attachment> {
        self.attachments().await?.into_iter().find(|a| a.slug == slug).ok_or_else(
            || MarathonError::NotFound {
                entity: "attachment",
                query: slug.to_string(),
            },
        )
    }
}

fn flatten_attachments(weapons: &[Weapon]) -> Vec<Attachment> {
    let mut out: Vec<Attachment> = Vec::new();
    for weapon in weapons {
        for slot in &weapon.attachment_slots {
            let Some(attachment) = &slot.attachment else { continue };
            if let Some(existing) =
                out.iter_mut().find(|a| a.slug == attachment.slug)
            {
                if !existing.compatible_weapons.contains(&weapon.name) {
                    existing.compatible_weapons.push(weapon.name.clone());
                }
            } else {
                out.push(attachment.clone());
            }
        }
    }
    out
}
