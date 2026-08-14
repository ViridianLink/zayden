use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct GreetingImageInfo {
    pub(crate) id: String,
    pub(crate) url: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct GreetingsView {
    pub(crate) morning_message: String,
    pub(crate) night_message: String,
    pub(crate) morning: Vec<GreetingImageInfo>,
    pub(crate) night: Vec<GreetingImageInfo>,
}
