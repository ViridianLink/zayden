//! Rasterisation coverage for the SVG -> PNG pipeline.
//!
//! Every SVG here is deliberately **text-free** and every renderer is built
//! with an empty font database, so these tests are hermetic: they pass on a
//! dev machine with no Noto (or any font) installed. Text shaping is resvg's
//! problem, not ours; what we own is the budget arithmetic, the overlay
//! compositing and the PNG encoding.

use std::io::Cursor;
use std::sync::Arc;

use zayden_graphics::error::GraphicsError;
use zayden_graphics::renderer::{
    Canvas,
    Overlay,
    RENDER_BUDGET_MP,
    RasterLimits,
    Renderer,
};
use zayden_graphics::tiny_skia::{Color, Pixmap};
use zayden_graphics::usvg::fontdb;

/// Generous ceilings, so a test only trips a budget when it means to.
const OPEN: RasterLimits = RasterLimits { max_pixels: 4_000_000, max_dim: 4_000 };

fn renderer() -> Renderer {
    Renderer::with_fonts(Arc::new(fontdb::Database::new()), "sans-serif".to_string())
}

fn svg(width: u32, height: u32, body: &str) -> String {
    let open = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    );
    format!("{open}{body}</svg>")
}

// `decode!` and `pixel!` are macros, not functions, on purpose: `clippy.toml`
// sets `allow-expect-in-tests`, but that only covers code lexically inside a
// `#[test]` item. A free helper fn using `.expect()` or indexing trips
// `expect_used` / `indexing_slicing` under the workspace `-D warnings` gate.

/// Decodes PNG bytes back to `(width, height, straight RGBA8)`.
macro_rules! decode {
    ($bytes:expr) => {{
        let decoder = png::Decoder::new(Cursor::new($bytes));
        let mut reader = decoder.read_info().expect("PNG header should parse");
        let size = reader.output_buffer_size().expect("buffer size should be known");
        let mut buf = vec![0u8; size];
        let info = reader.next_frame(&mut buf).expect("frame should decode");
        buf.truncate(info.buffer_size());
        (info.width, info.height, buf)
    }};
}

/// Reads one RGBA pixel out of a decoded frame.
macro_rules! pixel {
    ($rgba:expr, $width:expr, $x:expr, $y:expr) => {{
        let offset =
            usize::try_from(($y * $width + $x) * 4).expect("offset fits usize");
        // `try_from` rather than indexing: `indexing_slicing` is reported at
        // the macro definition site, where no test exemption applies.
        $rgba
            .get(offset..offset + 4)
            .and_then(|px| <[u8; 4]>::try_from(px).ok())
            .expect("pixel should be in range")
    }};
}

#[tokio::test]
async fn renders_a_text_free_svg_to_a_png_of_the_requested_size() {
    let markup = svg(40, 30, r##"<rect width="40" height="30" fill="#ff0000"/>"##);
    let canvas = Canvas { width: 40, height: 30 };

    let png = renderer()
        .render(markup, canvas, Vec::new(), OPEN)
        .await
        .expect("a text-free SVG should rasterise without any font");

    assert_eq!(
        png.get(..8),
        Some(b"\x89PNG\r\n\x1a\n".as_slice()),
        "output should carry the PNG magic bytes",
    );

    let (width, height, rgba) = decode!(&png);
    assert_eq!((width, height), (40, 30));
    assert_eq!(pixel!(&rgba, width, 20, 15), [255, 0, 0, 255]);
}

#[tokio::test]
async fn overlays_are_composited_over_the_rasterised_svg() {
    let markup = svg(16, 16, r##"<rect width="16" height="16" fill="#000000"/>"##);

    let mut patch = Pixmap::new(4, 4).expect("4x4 pixmap should allocate");
    patch.fill(Color::from_rgba8(0, 255, 0, 255));

    let png = renderer()
        .render(
            markup,
            Canvas { width: 16, height: 16 },
            vec![Overlay { pixmap: patch, x: 4, y: 4 }],
            OPEN,
        )
        .await
        .expect("render should succeed");

    let (width, _, rgba) = decode!(&png);

    assert_eq!(
        pixel!(&rgba, width, 5, 5),
        [0, 255, 0, 255],
        "inside the overlay the patch colour should win",
    );
    assert_eq!(
        pixel!(&rgba, width, 1, 1),
        [0, 0, 0, 255],
        "outside the overlay the SVG should be untouched",
    );
}

#[tokio::test]
async fn a_canvas_over_the_pixel_ceiling_is_refused() {
    let limits = RasterLimits { max_pixels: 100, max_dim: 4_000 };
    let markup = svg(40, 30, "");

    let err = renderer()
        .render(markup, Canvas { width: 40, height: 30 }, Vec::new(), limits)
        .await
        .expect_err("1200 pixels should exceed a 100 pixel ceiling");

    assert!(
        matches!(err, GraphicsError::OverBudget { pixels: 1200, limit: 100 }),
        "expected OverBudget, got {err:?}",
    );
}

#[tokio::test]
async fn a_canvas_over_the_dimension_ceiling_is_refused() {
    let limits = RasterLimits { max_pixels: 4_000_000, max_dim: 20 };
    let markup = svg(40, 30, "");

    let err = renderer()
        .render(markup, Canvas { width: 40, height: 30 }, Vec::new(), limits)
        .await
        .expect_err("a 40px edge should exceed a 20px ceiling");

    assert!(
        matches!(err, GraphicsError::OverBudget { .. }),
        "expected OverBudget, got {err:?}",
    );
}

#[tokio::test]
async fn an_empty_canvas_is_refused() {
    let err = renderer()
        .render(svg(1, 1, ""), Canvas { width: 0, height: 8 }, Vec::new(), OPEN)
        .await
        .expect_err("a zero-width canvas has nothing to draw");

    assert!(
        matches!(err, GraphicsError::EmptyCanvas),
        "expected EmptyCanvas, got {err:?}",
    );
}

/// The permit weighting is what stops concurrent renders from blowing the
/// memory budget, and asking for more permits than the semaphore can ever hold
/// would hang rather than error -- so an oversized request must be rejected
/// before it reaches `acquire_many`.
#[tokio::test]
async fn a_canvas_beyond_the_global_render_budget_is_refused_not_hung() {
    let huge = RENDER_BUDGET_MP * 2_000_000;
    let limits = RasterLimits { max_pixels: huge, max_dim: 100_000 };

    let err = renderer()
        .render(
            svg(1, 1, ""),
            Canvas { width: 8_000, height: 8_000 },
            Vec::new(),
            limits,
        )
        .await
        .expect_err("64 megapixels is far beyond the global render budget");

    assert!(
        matches!(err, GraphicsError::OverBudget { .. }),
        "expected OverBudget, got {err:?}",
    );
}

/// The SVG is generated from the same layout that produces the canvas size, so
/// a disagreement is a bug in the caller rather than something to paper over.
#[tokio::test]
async fn a_canvas_disagreeing_with_the_svg_is_refused() {
    let err = renderer()
        .render(svg(40, 30, ""), Canvas { width: 80, height: 60 }, Vec::new(), OPEN)
        .await
        .expect_err("declared SVG size and canvas size must agree");

    assert!(
        matches!(err, GraphicsError::SizeMismatch),
        "expected SizeMismatch, got {err:?}",
    );
}
