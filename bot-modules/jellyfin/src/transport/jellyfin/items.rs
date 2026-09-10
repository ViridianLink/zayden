use super::JellyfinClient;
use super::model::{Item, ItemCounts, ItemsPage};
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

    pub async fn counts(&self) -> ApiResult<ItemCounts> {
        fetch_json(super::SERVICE, "item counts", || self.get("Items/Counts")).await
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
