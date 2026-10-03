/// One module card on the guild overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleView {
    pub id: String,
    pub label: String,
    pub description: String,
    /// `None` when a command-backed module has no stored state yet.
    pub enabled: Option<bool>,
    /// Why the card's toggle is locked, for modules switched on elsewhere.
    pub locked: Option<String>,
}
