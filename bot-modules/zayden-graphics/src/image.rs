use std::io::{Cursor, Write};

use resvg::tiny_skia::{Pixmap, PremultipliedColorU8};

use crate::error::GraphicsError;

pub const AVATAR_MAX_BYTES: usize = 128 * 1024;
const MAX_SOURCE_PX: u32 = 512;

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 0.0..=255.0 first, so the u8 cast is exact"
)]
fn quantise(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn mul255(a: u8, b: u8) -> u8 {
    let product = u32::from(a) * u32::from(b) + 127;
    u8::try_from((product + product / 255) / 256).unwrap_or(u8::MAX)
}

fn to_rgba(data: &[u8], color: png::ColorType) -> Result<Vec<u8>, GraphicsError> {
    let rgba = match color {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|[r, g, b]| [*r, *g, *b, u8::MAX])
            .collect(),
        png::ColorType::Grayscale => {
            data.iter().flat_map(|g| [*g, *g, *g, u8::MAX]).collect()
        },
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|[g, a]| [*g, *g, *g, *a])
            .collect(),
        png::ColorType::Indexed => return Err(GraphicsError::AvatarColorType),
    };

    Ok(rgba)
}

fn resize_square(src: &[u8], sw: usize, sh: usize, dest: usize) -> Vec<u8> {
    if sw == dest && sh == dest {
        return src.to_vec();
    }

    let mut out = vec![0u8; dest * dest * 4];

    for (dy, row) in out.chunks_exact_mut(dest * 4).enumerate() {
        let y0 = dy * sh / dest;
        let y1 = ((dy + 1) * sh).div_ceil(dest).clamp(y0 + 1, sh);

        for (dx, px) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let x0 = dx * sw / dest;
            let x1 = ((dx + 1) * sw).div_ceil(dest).clamp(x0 + 1, sw);

            let mut acc = [0u32; 4];
            let mut count = 0u32;

            for sy in y0..y1 {
                for sx in x0..x1 {
                    let offset = (sy * sw + sx) * 4;
                    if let Some(sample) = src.get(offset..offset + 4) {
                        for (slot, value) in acc.iter_mut().zip(sample) {
                            *slot += u32::from(*value);
                        }
                        count += 1;
                    }
                }
            }

            for (slot, total) in px.iter_mut().zip(acc) {
                if let Some(mean) = total.checked_div(count) {
                    *slot = u8::try_from(mean).unwrap_or(u8::MAX);
                }
            }
        }
    }

    out
}

fn circular_pixmap(rgba: &[u8], size: u32) -> Result<Pixmap, GraphicsError> {
    let mut pixmap = Pixmap::new(size, size)
        .ok_or(GraphicsError::PixmapAlloc { width: size, height: size })?;

    let side = u16::try_from(size).unwrap_or(u16::MAX);
    let radius = f32::from(side) / 2.0;

    let stride = usize::try_from(size).unwrap_or(usize::MAX);
    let rows = pixmap.pixels_mut().chunks_exact_mut(stride);

    for (y, row) in rows.enumerate() {
        let dy = f32::from(u16::try_from(y).unwrap_or(u16::MAX)) + 0.5 - radius;

        for (x, slot) in row.iter_mut().enumerate() {
            let dx = f32::from(u16::try_from(x).unwrap_or(u16::MAX)) + 0.5 - radius;

            let coverage = quantise(radius - dx.hypot(dy) + 0.5);
            let offset = (y * stride + x) * 4;

            let Some([r, g, b, a]) = rgba.get(offset..offset + 4) else {
                continue;
            };

            let alpha = mul255(*a, coverage);
            *slot = PremultipliedColorU8::from_rgba(
                mul255(*r, alpha),
                mul255(*g, alpha),
                mul255(*b, alpha),
                alpha,
            )
            .unwrap_or(PremultipliedColorU8::TRANSPARENT);
        }
    }

    Ok(pixmap)
}

pub fn decode_avatar(bytes: &[u8], size: u32) -> Result<Pixmap, GraphicsError> {
    if bytes.len() > AVATAR_MAX_BYTES {
        return Err(GraphicsError::AvatarTooLarge {
            bytes: bytes.len(),
            limit: AVATAR_MAX_BYTES,
        });
    }

    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info()?;
    let (width, height) = (reader.info().width, reader.info().height);

    if width == 0 || height == 0 || width > MAX_SOURCE_PX || height > MAX_SOURCE_PX {
        return Err(GraphicsError::AvatarTooBig { width, height });
    }

    let capacity = reader
        .output_buffer_size()
        .ok_or(GraphicsError::AvatarTooBig { width, height })?;
    let mut buffer = vec![0u8; capacity];
    let frame = reader.next_frame(&mut buffer)?;
    let color = frame.color_type;
    let decoded =
        buffer.get(..frame.buffer_size()).ok_or(GraphicsError::AvatarColorType)?;

    let rgba = to_rgba(decoded, color)?;

    let sw = usize::try_from(width).unwrap_or(usize::MAX);
    let sh = usize::try_from(height).unwrap_or(usize::MAX);
    let dest = usize::try_from(size).unwrap_or(usize::MAX);

    let scaled = resize_square(&rgba, sw, sh, dest);

    circular_pixmap(&scaled, size)
}

pub fn encode_png(pixmap: &Pixmap) -> Result<Vec<u8>, GraphicsError> {
    let width = pixmap.width();
    let height = pixmap.height();

    if width == 0 || height == 0 {
        return Err(GraphicsError::EmptyCanvas);
    }

    let stride = usize::try_from(width).map_err(|_e| GraphicsError::EmptyCanvas)?;
    let row_bytes = stride.checked_mul(4).ok_or(GraphicsError::EmptyCanvas)?;

    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);

        let mut writer = encoder.write_header()?;
        let mut stream = writer.stream_writer()?;
        let mut row = vec![0u8; row_bytes];

        for line in pixmap.pixels().chunks_exact(stride) {
            let (slots, _) = row.as_chunks_mut::<4>();
            for (slot, pixel) in slots.iter_mut().zip(line) {
                let colour = pixel.demultiply();
                *slot =
                    [colour.red(), colour.green(), colour.blue(), colour.alpha()];
            }

            stream.write_all(&row).map_err(png::EncodingError::from)?;
        }

        stream.finish()?;
    }

    Ok(out)
}
