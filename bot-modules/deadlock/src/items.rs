use std::borrow::Cow;
use std::collections::HashMap;

use deadlock_api::model::Item;

#[derive(Debug, Clone)]
pub struct ItemIndex {
    by_id: HashMap<u32, Item>,
}

impl ItemIndex {
    #[must_use]
    pub fn new(items: &[Item]) -> Self {
        Self { by_id: items.iter().map(|item| (item.id, item.clone())).collect() }
    }

    #[must_use]
    pub fn get(&self, id: u32) -> Option<&Item> {
        self.by_id.get(&id)
    }

    #[must_use]
    pub fn name(&self, id: u32) -> Cow<'_, str> {
        match self.get(id) {
            Some(item) if !item.name.is_empty() => Cow::Borrowed(&item.name),
            Some(item) if !item.class_name.is_empty() => {
                Cow::Borrowed(&item.class_name)
            },
            Some(_) | None => Cow::Owned(format!("#{id}")),
        }
    }
}
