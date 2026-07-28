use std::time::Duration;

mod cloudflare;
mod fandom;
mod palcalc;
mod paldb;
mod paldex;
mod palworldgg;
mod pelican;

pub(crate) const SOURCE_TIMEOUT: Duration = Duration::from_secs(10);

pub use fandom::Fandom;
pub use palcalc::{
    DEFAULT_PALCALC_BASE,
    PalCalc,
    RawPalCalcPal,
    parse_breeding,
    parse_pals,
};
pub use paldb::{PalDb, PalDetails};
pub use paldex::{BreedingMap, DEFAULT_BASE, Paldex, RawItem, RawPal, RawPassive};
pub use palworldgg::PalworldGg;
pub use pelican::{Pelican, parse_modified_at};
