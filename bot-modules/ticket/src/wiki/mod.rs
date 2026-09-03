mod config;
mod error;
mod graphql;
mod page;
mod search;

pub use config::WikiConfig;
pub use error::WikiError;
pub use page::{Page, page};
pub use search::{SearchResult, search};
