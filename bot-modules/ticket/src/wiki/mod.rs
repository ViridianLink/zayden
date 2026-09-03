mod config;
mod error;
mod graphql;
mod page;
mod search;

pub use config::WikiConfig;
pub use error::WikiError;
pub use page::{Page, page};
pub use search::{SearchResult, search};

#[cynic::schema("wikijs")]
#[expect(
    unreachable_pub,
    reason = "generated markers cover the whole Wiki.js schema, not just the \
              types these queries select"
)]
mod schema {}
