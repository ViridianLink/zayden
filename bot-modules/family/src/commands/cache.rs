use async_trait::async_trait;

use crate::{FamilyRow, Result};

#[async_trait]
trait CacheHandler {
    async fn cache_user(&self, row: FamilyRow) -> Result<()>;
}

struct Cache {}

#[async_trait]
impl CacheHandler for Cache {
    async fn cache_user(&self, row: FamilyRow) -> Result<()> {
        Ok(())
    }
}
