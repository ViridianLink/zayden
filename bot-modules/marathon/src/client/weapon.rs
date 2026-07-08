use std::sync::Arc;

use super::MarathonClient;
use crate::error::{MarathonError, Result};
use crate::model::{Attachment, Weapon};
use crate::parse;

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
        if let Some(mobalytics) = &self.mobalytics
            && let Ok(doc) =
                mobalytics.fetch_document(&format!("weapons/{slug}")).await
        {
            return Ok(parse::parse_weapon(slug, &doc));
        }
        let data = self.marathondb.weapon(slug).await?;
        Ok(parse::marathondb_weapon_to_model(slug, &data))
    }

    pub async fn weapons(&self) -> Result<Arc<[Weapon]>> {
        if let Some(cached) = self.weapon_list_cache.get(&()).await {
            return Ok(cached);
        }
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let slugs = mobalytics.fetch_listing_slugs("weapons", "weapons").await?;

        let mut weapons = Vec::with_capacity(slugs.len());
        for slug in &slugs {
            weapons.push((*self.weapon(slug).await?).clone());
        }

        let entry: Arc<[Weapon]> = weapons.into();
        self.weapon_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
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
