use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum GraphicsError {
    NoFont,
    Svg(resvg::usvg::Error),
    EmptyCanvas,
    OverBudget { pixels: u32, limit: u32 },
    PixmapAlloc { width: u32, height: u32 },
    SizeMismatch,
    AvatarTooLarge { bytes: usize, limit: usize },
    AvatarTooBig { width: u32, height: u32 },
    AvatarColorType,
    PngDecode(Box<png::DecodingError>),
    PngEncode(Box<png::EncodingError>),
    RenderTaskFailed,
    SemaphoreClosed,
}

impl Display for GraphicsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFont => write!(
                f,
                "no usable font was found; install fonts-noto-core or set \
                 ZAYDEN_FONT_PATHS"
            ),
            Self::Svg(e) => write!(f, "failed to parse the generated SVG: {e}"),
            Self::EmptyCanvas => write!(f, "the requested canvas is empty"),
            Self::OverBudget { pixels, limit } => {
                write!(f, "canvas of {pixels} pixels exceeds the limit of {limit}")
            },
            Self::PixmapAlloc { width, height } => {
                write!(f, "could not allocate a {width}x{height} pixmap")
            },
            Self::SizeMismatch => {
                write!(f, "the SVG does not declare the requested canvas size")
            },
            Self::AvatarTooLarge { bytes, limit } => {
                write!(f, "avatar of {bytes} bytes exceeds the limit of {limit}")
            },
            Self::AvatarTooBig { width, height } => {
                write!(f, "avatar of {width}x{height} is too large to decode")
            },
            Self::AvatarColorType => {
                write!(f, "unsupported PNG colour type in avatar")
            },
            Self::PngDecode(e) => write!(f, "failed to decode PNG: {e}"),
            Self::PngEncode(e) => write!(f, "failed to encode PNG: {e}"),
            Self::RenderTaskFailed => write!(f, "the render task failed"),
            Self::SemaphoreClosed => write!(f, "the render semaphore is closed"),
        }
    }
}

impl Error for GraphicsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Svg(e) => Some(e),
            Self::PngDecode(e) => Some(e),
            Self::PngEncode(e) => Some(e),
            Self::NoFont
            | Self::EmptyCanvas
            | Self::OverBudget { .. }
            | Self::PixmapAlloc { .. }
            | Self::SizeMismatch
            | Self::AvatarTooLarge { .. }
            | Self::AvatarTooBig { .. }
            | Self::AvatarColorType
            | Self::RenderTaskFailed
            | Self::SemaphoreClosed => None,
        }
    }
}

impl From<resvg::usvg::Error> for GraphicsError {
    fn from(e: resvg::usvg::Error) -> Self {
        Self::Svg(e)
    }
}

impl From<png::DecodingError> for GraphicsError {
    fn from(e: png::DecodingError) -> Self {
        Self::PngDecode(Box::new(e))
    }
}

impl From<png::EncodingError> for GraphicsError {
    fn from(e: png::EncodingError) -> Self {
        Self::PngEncode(Box::new(e))
    }
}
