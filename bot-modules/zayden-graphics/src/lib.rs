pub mod error;
pub mod fonts;
pub mod image;
pub mod renderer;

pub use error::GraphicsError;
pub use image::{AVATAR_MAX_BYTES, decode_avatar};
pub use renderer::{Canvas, Overlay, RENDER_BUDGET_MP, RasterLimits, Renderer};
pub use resvg::{tiny_skia, usvg};

pub const AVATAR_PX: u32 = 64;
