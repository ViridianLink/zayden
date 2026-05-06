#[derive(Debug, PartialEq)]
pub enum Relationships {
    Partner,
    Parent,
    Child,
    None,
}

impl std::fmt::Display for Relationships {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Relationships::Partner => write!(f, "Partner"),
            Relationships::Parent => write!(f, "Parent"),
            Relationships::Child => write!(f, "Child"),
            Relationships::None => write!(f, "None"),
        }
    }
}
