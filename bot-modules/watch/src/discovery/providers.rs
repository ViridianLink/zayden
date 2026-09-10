use jellyfin::transport::jellyseerr::model::RegionProviders;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegionAvailability {
    pub stream: Vec<String>,
    pub buy: Vec<String>,
    pub link: Option<String>,
}

impl RegionAvailability {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.stream.is_empty() && self.buy.is_empty()
    }
}

#[must_use]
pub fn for_region(
    providers: &[RegionProviders],
    region: &str,
) -> RegionAvailability {
    let Some(entry) = providers.iter().find(|p| p.region == region) else {
        return RegionAvailability::default();
    };

    RegionAvailability {
        stream: entry.flatrate.iter().map(|p| p.name.clone()).collect(),
        buy: entry.buy.iter().map(|p| p.name.clone()).collect(),
        link: entry.link.clone(),
    }
}
