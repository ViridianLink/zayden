#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleView {
    pub id: String,
    pub label: String,
    pub description: String,
    pub enabled: Option<bool>,
    pub locked: Option<String>,
}
