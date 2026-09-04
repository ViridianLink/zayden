use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Policy {
    pub(crate) concurrency: usize,
    pub(crate) batch_pause: Duration,
    pub(crate) ttl: Duration,
    pub(crate) crawl: bool,
}

const SMALL: usize = 150;
const LARGE: usize = 1000;

impl Policy {
    #[must_use]
    pub(crate) const fn for_size(pages: usize) -> Self {
        if pages <= SMALL {
            return Self {
                concurrency: 4,
                batch_pause: Duration::from_millis(50),
                ttl: Duration::from_hours(1),
                crawl: true,
            };
        }

        if pages <= LARGE {
            return Self {
                concurrency: 2,
                batch_pause: Duration::from_millis(500),
                ttl: Duration::from_hours(6),
                crawl: true,
            };
        }

        Self {
            concurrency: 2,
            batch_pause: Duration::from_millis(500),
            ttl: Duration::from_hours(12),
            crawl: false,
        }
    }
}
