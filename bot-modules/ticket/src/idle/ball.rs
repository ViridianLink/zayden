#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ball {
    Op,
    Helper,
}

impl Ball {
    #[must_use]
    pub const fn from_column(waiting_on_helper: bool) -> Self {
        if waiting_on_helper { Self::Helper } else { Self::Op }
    }

    #[must_use]
    pub const fn column(self) -> bool {
        matches!(self, Self::Helper)
    }
}
