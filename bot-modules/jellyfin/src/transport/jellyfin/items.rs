use serde::de::IgnoredAny;

use super::JellyfinClient;
use super::model::{Item, ItemsPage, LibraryCounts};
use crate::transport::http::{ApiResult, fetch_json};

const INDEX_FIELDS: &str =
    "Path,ProviderIds,RunTimeTicks,Genres,SortName,DateCreated,ProductionYear";
const PAGE_SIZE: i64 = 500;

impl JellyfinClient {
    pub async fn item(&self, item_id: &str) -> ApiResult<Item> {
        fetch_json(super::SERVICE, "item lookup", || {
            self.get(&format!("Items/{item_id}")).query(&[("fields", INDEX_FIELDS)])
        })
        .await
    }

    pub async fn library_count(
        &self,
        parent_id: &str,
        item_type: &str,
    ) -> ApiResult<i64> {
        let page: ItemsPage<IgnoredAny> =
            fetch_json(super::SERVICE, "library count", || {
                self.get("Items").query(&[
                    ("parentId", parent_id),
                    ("includeItemTypes", item_type),
                    ("recursive", "true"),
                    ("collapseBoxSetItems", "false"),
                    ("isMissing", "false"),
                    ("limit", "0"),
                    ("enableTotalRecordCount", "true"),
                ])
            })
            .await?;

        Ok(page.total_record_count)
    }

    pub async fn counts(&self) -> ApiResult<LibraryCounts> {
        let (movies, series, episodes) = futures::try_join!(
            self.library_count(self.movie_library_id(), "Movie"),
            self.library_count(self.show_library_id(), "Series"),
            self.library_count(self.show_library_id(), "Episode"),
        )?;

        Ok(LibraryCounts { movies, series, episodes })
    }

    pub async fn library_page(
        &self,
        parent_id: &str,
        item_type: &str,
        start_index: i64,
    ) -> ApiResult<ItemsPage<Item>> {
        fetch_json(super::SERVICE, "library page", || {
            self.get("Items").query(&[
                ("parentId", parent_id),
                ("includeItemTypes", item_type),
                ("recursive", "true"),
                ("collapseBoxSetItems", "false"),
                ("fields", INDEX_FIELDS),
                ("startIndex", &start_index.to_string()),
                ("limit", &PAGE_SIZE.to_string()),
                ("sortBy", "SortName"),
                ("enableTotalRecordCount", "true"),
            ])
        })
        .await
    }

    pub async fn library_items(
        &self,
        parent_id: &str,
        item_type: &str,
    ) -> ApiResult<Vec<Item>> {
        let mut all = Vec::new();
        let mut start_index = 0;

        loop {
            let page = self.library_page(parent_id, item_type, start_index).await?;
            let fetched = i64::try_from(page.items.len()).unwrap_or(i64::MAX);
            all.extend(page.items);

            start_index += fetched;
            if fetched == 0 || start_index >= page.total_record_count {
                break;
            }
        }

        Ok(all)
    }

    pub async fn episodes(&self, series_id: &str) -> ApiResult<Vec<Item>> {
        let page: ItemsPage<Item> =
            fetch_json(super::SERVICE, "series episodes", || {
                self.get(&format!("Shows/{series_id}/Episodes"))
                    .query(&[("fields", INDEX_FIELDS)])
            })
            .await?;

        Ok(page.items)
    }
}
