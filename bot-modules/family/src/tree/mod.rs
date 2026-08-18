pub mod fetch;
pub mod layout;
pub mod model;
pub mod prune;
pub mod quota;
pub mod svg;

pub use fetch::{RawGraph, RawPerson};
pub use layout::{Layout, layout};
pub use model::{Block, FamilyGraph, NodeIdx, Person, Union};
pub use prune::{Pruned, prune};
pub use quota::TreeQuota;
pub use svg::{AvatarSlot, TreeSvg};

pub const NODE_W: f32 = 168.0;
pub const NODE_H: f32 = 46.0;
pub const NODE_GAP: f32 = 24.0;
pub const PARTNER_GAP: f32 = 18.0;
pub const ROW_PITCH: f32 = 130.0;
pub const MARGIN: f32 = 32.0;
pub const MAX_NAME_CHARS: usize = 18;
pub const FONT_SIZE: f32 = 16.0;
pub const AVATAR_BOX: f32 = 30.0;
pub const NODE_PAD: f32 = 8.0;

pub const REFINE_PASSES: usize = 8;
pub const ORDER_SWEEPS: usize = 4;
pub const GEN_RELAX_PASSES: usize = 8;

pub const AVATAR_FETCH_CONCURRENCY: usize = 6;
