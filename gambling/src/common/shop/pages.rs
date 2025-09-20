use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShopPage {
    Item,
    Boost1,
    Boost2,
    Mine1,
    Mine2,
}

impl ShopPage {
    pub const fn pages() -> [ShopPage; 5] {
        [
            ShopPage::Item,
            ShopPage::Boost1,
            ShopPage::Boost2,
            ShopPage::Mine1,
            ShopPage::Mine2,
        ]
    }
}

impl Display for ShopPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Item => write!(f, "Item"),
            Self::Boost1 => write!(f, "Boost 1"),
            Self::Boost2 => write!(f, "Boost 2"),
            Self::Mine1 => write!(f, "Mine 1"),
            Self::Mine2 => write!(f, "Mine 2"),
        }
    }
}

impl FromStr for ShopPage {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Item" => Ok(Self::Item),
            "Boost 1" => Ok(Self::Boost1),
            "Boost 2" => Ok(Self::Boost2),
            "Mine 1" => Ok(Self::Mine1),
            "Mine 2" => Ok(Self::Mine2),
            _ => Err(()),
        }
    }
}
