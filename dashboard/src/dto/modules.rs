use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ModuleView {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) commands: Vec<String>,
    pub(crate) enabled: bool,
    pub(crate) locked: bool,
}
