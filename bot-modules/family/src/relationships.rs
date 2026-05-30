#[derive(Debug, PartialEq, Eq)]
pub enum Relationships {
    Partner,
    Parent,
    Child,
    None,
}

impl std::fmt::Display for Relationships {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Partner => write!(f, "Partner"),
            Self::Parent => write!(f, "Parent"),
            Self::Child => write!(f, "Child"),
            Self::None => write!(f, "None"),
        }
    }
}
