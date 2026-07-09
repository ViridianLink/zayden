use std::sync::Arc;

use serde_json::Value;

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
        match self.marathondb.weapon(slug).await {
            Ok(data) => {
                let mut weapon = parse::marathondb_weapon_to_model(slug, &data);
                self.enrich_weapon(&mut weapon).await;
                Ok(weapon)
            },
            Err(err) => match &self.mobalytics {
                Some(mobalytics) => {
                    let doc = mobalytics
                        .fetch_document(&format!("weapons/{slug}"))
                        .await?;
                    Ok(parse::parse_weapon(slug, &doc))
                },
                None => Err(err),
            },
        }
    }

    async fn enrich_weapon(&self, weapon: &mut Weapon) {
        if weapon.attachment_slots.is_empty()
            && let Some(mobalytics) = &self.mobalytics
            && let Ok(doc) =
                mobalytics.fetch_document(&format!("weapons/{}", weapon.slug)).await
        {
            let parsed = parse::parse_weapon(&weapon.slug, &doc);
            weapon.attachment_slots = parsed.attachment_slots;
            if weapon.thumbnail_url.is_none() {
                weapon.thumbnail_url = parsed.thumbnail_url;
            }
        }

        if weapon.description.is_none() {
            weapon.description = self.fandom.description(&weapon.name).await;
        }
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
