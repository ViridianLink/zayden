use std::sync::{Arc, OnceLock};

use resvg::tiny_skia::{Pixmap, PixmapPaint, Transform};
use resvg::usvg;
use tokio::sync::Semaphore;

use crate::error::GraphicsError;
use crate::{fonts, image};

pub const RENDER_BUDGET_MP: u32 = 12;
const PIXELS_PER_PERMIT: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterLimits {
    pub max_pixels: u32,
    pub max_dim: u32,
}

#[derive(Debug)]
pub struct Overlay {
    pub pixmap: Pixmap,
    pub x: i32,
    pub y: i32,
}

pub struct Renderer {
    fonts: Arc<usvg::fontdb::Database>,
    family: String,
    permits: Semaphore,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("family", &self.family)
            .field("faces", &self.fonts.len())
            .finish_non_exhaustive()
    }
}

fn init() -> Option<Renderer> {
    let found = fonts::discover()?;

    Some(Renderer::with_fonts(found.db, found.family))
}

impl Renderer {
    #[must_use]
    pub fn with_fonts(fonts: Arc<usvg::fontdb::Database>, family: String) -> Self {
        Self {
            fonts,
            family,
            permits: Semaphore::new(
                usize::try_from(RENDER_BUDGET_MP).unwrap_or(usize::MAX),
            ),
        }
    }

    pub fn shared() -> Result<&'static Self, GraphicsError> {
        static SHARED: OnceLock<Option<Renderer>> = OnceLock::new();

        SHARED.get_or_init(init).as_ref().ok_or(GraphicsError::NoFont)
    }

    #[must_use]
    pub fn font_family(&self) -> &str {
        &self.family
    }

    pub async fn render(
        &self,
        svg: String,
        canvas: Canvas,
        overlays: Vec<Overlay>,
        limits: RasterLimits,
    ) -> Result<Vec<u8>, GraphicsError> {
        let pixels = check_budget(canvas, limits)?;

        let weight = pixels.div_ceil(PIXELS_PER_PERMIT).max(1);
        if weight > RENDER_BUDGET_MP {
            return Err(GraphicsError::OverBudget {
                pixels,
                limit: RENDER_BUDGET_MP.saturating_mul(PIXELS_PER_PERMIT),
            });
        }

        let _permit = self
            .permits
            .acquire_many(weight)
            .await
            .map_err(|_e| GraphicsError::SemaphoreClosed)?;

        let fonts = Arc::clone(&self.fonts);
        let family = self.family.clone();

        tokio::task::spawn_blocking(move || {
            rasterise(&svg, canvas, overlays, fonts, family)
        })
        .await
        .map_err(|_e| GraphicsError::RenderTaskFailed)?
    }
}

fn check_budget(canvas: Canvas, limits: RasterLimits) -> Result<u32, GraphicsError> {
    if canvas.width == 0 || canvas.height == 0 {
        return Err(GraphicsError::EmptyCanvas);
    }

    if canvas.width > limits.max_dim || canvas.height > limits.max_dim {
        return Err(GraphicsError::OverBudget {
            pixels: canvas.width.max(canvas.height),
            limit: limits.max_dim,
        });
    }

    let pixels = canvas.width.checked_mul(canvas.height).ok_or(
        GraphicsError::OverBudget { pixels: u32::MAX, limit: limits.max_pixels },
    )?;

    if pixels > limits.max_pixels {
        return Err(GraphicsError::OverBudget { pixels, limit: limits.max_pixels });
    }

    Ok(pixels)
}

fn rasterise(
    svg: &str,
    canvas: Canvas,
    overlays: Vec<Overlay>,
    fonts: Arc<usvg::fontdb::Database>,
    family: String,
) -> Result<Vec<u8>, GraphicsError> {
    let mut options =
        usvg::Options { font_family: family, ..usvg::Options::default() };
    options.fontdb = fonts;

    let tree = usvg::Tree::from_str(svg, &options)?;

    let size = tree.size().to_int_size();
    if size.width() != canvas.width || size.height() != canvas.height {
        return Err(GraphicsError::SizeMismatch);
    }

    let mut pixmap = Pixmap::new(canvas.width, canvas.height).ok_or(
        GraphicsError::PixmapAlloc { width: canvas.width, height: canvas.height },
    )?;

    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());

    let paint = PixmapPaint::default();
    for overlay in overlays {
        pixmap.draw_pixmap(
            overlay.x,
            overlay.y,
            overlay.pixmap.as_ref(),
            &paint,
            Transform::identity(),
            None,
        );
    }

    image::encode_png(&pixmap)
}
