//! The captured frame: an RGB buffer, how to get one, and how to encode it.

use std::error::Error;
use std::process::{Command, Stdio};

use crate::geometry::Rect;

/// How large the effects are at a capture scale of 1, in pixels. Both are
/// deliberately heavy, since small ones leave text guessable.
const PIXEL_BLOCK: f64 = 16.0;
const BLUR_RADIUS: f64 = 10.0;
const BLUR_PASSES: usize = 3;

/// A redaction computed from the pixels underneath it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    Blur,
    Pixelate,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

impl Image {
    /// Captures every monitor at once. Must run before any window of ours
    /// exists: a surface under the pointer is what ends hover states.
    pub fn capture(include_cursor: bool) -> Result<Image, Box<dyn Error>> {
        let mut grim = Command::new("grim");
        grim.args(["-t", "ppm"]);
        if include_cursor {
            grim.arg("-c");
        }
        let output = grim
            .arg("-")
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("could not run grim: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("grim failed: {}", stderr.trim()).into());
        }
        Image::from_ppm(output.stdout)
    }

    /// Parses the binary PPM that grim writes: `P6\n<w> <h>\n255\n<pixels>`.
    fn from_ppm(mut data: Vec<u8>) -> Result<Image, Box<dyn Error>> {
        let mut pos = 0;
        let mut next_token = || -> Result<String, Box<dyn Error>> {
            let rest = data.get(pos..).unwrap_or_default();
            let start = rest
                .iter()
                .position(|b| !b.is_ascii_whitespace())
                .ok_or("truncated PPM header")?;
            let len = rest[start..]
                .iter()
                .position(|b| b.is_ascii_whitespace())
                .ok_or("truncated PPM header")?;
            pos += start + len;
            Ok(String::from_utf8_lossy(&rest[start..start + len]).into_owned())
        };

        if next_token()? != "P6" {
            return Err("grim did not produce a P6 PPM".into());
        }
        let width: u32 = next_token()?.parse()?;
        let height: u32 = next_token()?.parse()?;
        if next_token()? != "255" {
            return Err("unsupported PPM color depth".into());
        }

        // Exactly one whitespace byte separates the header from the pixels.
        data.drain(..pos + 1);
        let expected = width as usize * height as usize * 3;
        if data.len() != expected {
            return Err(format!("expected {expected} pixel bytes, got {}", data.len()).into());
        }
        Ok(Image {
            width,
            height,
            rgb: data,
        })
    }

    /// The color of one pixel, or `None` outside the image.
    pub fn pixel(&self, x: i32, y: i32) -> Option<[u8; 3]> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let i = (y as usize * self.width as usize + x as usize) * 3;
        Some([self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]])
    }

    /// Copies out a rectangle, clipped to the image. `None` if nothing is left.
    pub fn crop(&self, rect: &Rect) -> Option<Image> {
        let full = Rect {
            x: 0,
            y: 0,
            w: self.width as i32,
            h: self.height as i32,
        };
        let r = rect.intersect(&full)?;
        let stride = self.width as usize * 3;
        let row_len = r.w as usize * 3;
        let mut rgb = Vec::with_capacity(row_len * r.h as usize);
        for row in r.y as usize..r.bottom() as usize {
            let start = row * stride + r.x as usize * 3;
            rgb.extend_from_slice(&self.rgb[start..start + row_len]);
        }
        Some(Image {
            width: r.w as u32,
            height: r.h as u32,
            rgb,
        })
    }

    /// Makes a redacted copy of a rectangle of this image. It is computed from
    /// that rectangle alone, so nothing beside it can show through.
    pub fn redacted(&self, rect: &Rect, effect: Effect, scale: f64) -> Option<Image> {
        let mut patch = self.crop(rect)?;
        match effect {
            Effect::Blur => patch.blur((BLUR_RADIUS * scale).round().max(1.0) as usize),
            Effect::Pixelate => patch.pixelate((PIXEL_BLOCK * scale).round().max(1.0) as usize),
        }
        Some(patch)
    }

    fn pixelate(&mut self, block: usize) {
        let (width, height) = (self.width as usize, self.height as usize);
        for top in (0..height).step_by(block) {
            for left in (0..width).step_by(block) {
                let (right, bottom) = ((left + block).min(width), (top + block).min(height));
                let count = ((right - left) * (bottom - top)) as u32;
                let mut sum = [0u32; 3];
                for y in top..bottom {
                    for x in left..right {
                        let i = (y * width + x) * 3;
                        (0..3).for_each(|c| sum[c] += u32::from(self.rgb[i + c]));
                    }
                }
                let average = sum.map(|total| ((total + count / 2) / count) as u8);
                for y in top..bottom {
                    for x in left..right {
                        self.rgb[(y * width + x) * 3..][..3].copy_from_slice(&average);
                    }
                }
            }
        }
    }

    /// Repeated box blurs, which together approximate a gaussian.
    fn blur(&mut self, radius: usize) {
        let (width, height) = (self.width as usize, self.height as usize);
        let mut scratch = vec![0; self.rgb.len()];
        for _ in 0..BLUR_PASSES {
            box_pass(&self.rgb, &mut scratch, height, width, width * 3, 3, radius);
            box_pass(&scratch, &mut self.rgb, width, height, 3, width * 3, radius);
        }
    }

    /// Copies `patch` so its top-left lands at `(x, y)`, clipped to this image.
    pub fn paste(&mut self, patch: &Image, x: i32, y: i32) {
        let bounds = Rect { x: 0, y: 0, w: self.width as i32, h: self.height as i32 };
        let area = Rect { x, y, w: patch.width as i32, h: patch.height as i32 };
        let Some(r) = area.intersect(&bounds) else { return };
        let row_len = r.w as usize * 3;
        for row in 0..r.h as usize {
            let src = (row + (r.y - y) as usize) * patch.width as usize * 3 + (r.x - x) as usize * 3;
            let dst = (row + r.y as usize) * self.width as usize * 3 + r.x as usize * 3;
            self.rgb[dst..dst + row_len].copy_from_slice(&patch.rgb[src..src + row_len]);
        }
    }

    pub fn to_png(&self) -> Result<Vec<u8>, png::EncodingError> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.rgb)?;
        writer.finish()?;
        Ok(out)
    }
}

/// Box-blurs `lines` lines of `len` RGB pixels each, from `src` into `dst`.
/// `line_step` and `pixel_step` are the byte distances between lines and
/// between pixels along a line. Past either end, the last pixel is repeated.
fn box_pass(src: &[u8], dst: &mut [u8], lines: usize, len: usize, line_step: usize, pixel_step: usize, radius: usize) {
    let window = 2 * radius as u32 + 1;
    for line in 0..lines {
        let base = line * line_step;
        for channel in 0..3 {
            let at = |i: isize| u32::from(src[base + i.clamp(0, len as isize - 1) as usize * pixel_step + channel]);
            let r = radius as isize;
            let mut sum: u32 = (-r..=r).map(at).sum();
            for i in 0..len as isize {
                dst[base + i as usize * pixel_step + channel] = ((sum + window / 2) / window) as u8;
                sum = sum + at(i + r + 1) - at(i - r);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3x2 image where each pixel's red channel is its index.
    fn sample() -> Image {
        Image {
            width: 3,
            height: 2,
            rgb: (0u8..6).flat_map(|i| [i, 0, 255]).collect(),
        }
    }

    #[test]
    fn parses_ppm_whose_first_pixel_looks_like_whitespace() {
        let mut ppm = b"P6\n2 1\n255\n".to_vec();
        ppm.extend_from_slice(&[b'\n', b' ', 9, 1, 2, 3]);
        let image = Image::from_ppm(ppm).unwrap();
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.rgb, [b'\n', b' ', 9, 1, 2, 3]);
    }

    #[test]
    fn rejects_truncated_pixels() {
        assert!(Image::from_ppm(b"P6\n2 2\n255\nabc".to_vec()).is_err());
    }

    #[test]
    fn crop_takes_the_requested_pixels() {
        let rect = Rect { x: 1, y: 0, w: 2, h: 2 };
        let red: Vec<u8> = sample().crop(&rect).unwrap().rgb.chunks(3).map(|p| p[0]).collect();
        assert_eq!(red, [1, 2, 4, 5]);
    }

    #[test]
    fn crop_is_clipped_to_the_image() {
        let rect = Rect { x: 2, y: 1, w: 50, h: 50 };
        let cropped = sample().crop(&rect).unwrap();
        assert_eq!((cropped.width, cropped.height), (1, 1));
        assert_eq!(cropped.rgb, [5, 0, 255]);
    }

    #[test]
    fn crop_outside_the_image_is_none() {
        assert_eq!(sample().crop(&Rect { x: 9, y: 9, w: 2, h: 2 }), None);
    }

    /// A vertical black and white stripe every other pixel.
    fn stripes(width: u32, height: u32) -> Image {
        let rgb = (0..height)
            .flat_map(|_| (0..width).flat_map(|x| [if x % 2 == 0 { 0u8 } else { 255 }; 3]))
            .collect();
        Image { width, height, rgb }
    }

    #[test]
    fn pixelate_makes_each_block_one_color() {
        let rect = Rect { x: 0, y: 0, w: 32, h: 32 };
        let patch = stripes(32, 32).redacted(&rect, Effect::Pixelate, 1.0).unwrap();
        assert_eq!((patch.width, patch.height), (32, 32));
        assert!(patch.rgb.iter().all(|&v| v == 128));
    }

    #[test]
    fn pixelate_handles_a_partial_last_block() {
        let rect = Rect { x: 0, y: 0, w: 20, h: 5 };
        assert!(stripes(20, 5).redacted(&rect, Effect::Pixelate, 1.0).is_some());
    }

    #[test]
    fn blur_removes_fine_detail() {
        let rect = Rect { x: 0, y: 0, w: 100, h: 100 };
        let patch = stripes(100, 100).redacted(&rect, Effect::Blur, 1.0).unwrap();
        // Three passes of radius 10 reach 30 pixels; past that only stripes count.
        for x in 30..70 {
            let [r, g, b] = pixel_at(&patch, x, 50);
            assert!((120..=135).contains(&r) && r == g && g == b, "detail left at {x}: {r}");
        }
    }

    #[test]
    fn blur_keeps_a_flat_color_exact() {
        let image = Image { width: 8, height: 8, rgb: vec![77; 8 * 8 * 3] };
        let rect = Rect { x: 0, y: 0, w: 8, h: 8 };
        assert_eq!(image.redacted(&rect, Effect::Blur, 1.0).unwrap(), image);
    }

    #[test]
    fn a_redaction_ignores_pixels_beside_it() {
        let mut image = Image { width: 40, height: 40, rgb: vec![10; 40 * 40 * 3] };
        let rect = Rect { x: 0, y: 0, w: 20, h: 40 };
        let before = image.redacted(&rect, Effect::Blur, 1.0).unwrap();
        let bright = Image { width: 20, height: 40, rgb: vec![250; 20 * 40 * 3] };
        image.paste(&bright, 20, 0);
        assert_eq!(image.redacted(&rect, Effect::Blur, 1.0).unwrap(), before);
    }

    #[test]
    fn paste_is_clipped_to_the_image() {
        let mut image = Image { width: 3, height: 2, rgb: vec![0; 18] };
        let patch = Image { width: 2, height: 2, rgb: (1u8..=12).collect() };
        image.paste(&patch, 2, -1);
        assert_eq!(pixel_at(&image, 2, 0), [7, 8, 9]);
        assert_eq!(pixel_at(&image, 2, 1), [0, 0, 0]);
    }

    fn pixel_at(image: &Image, x: usize, y: usize) -> [u8; 3] {
        let i = (y * image.width as usize + x) * 3;
        [image.rgb[i], image.rgb[i + 1], image.rgb[i + 2]]
    }

    #[test]
    fn pixel_reads_inside_the_image_only() {
        let image = sample();
        assert_eq!(image.pixel(1, 1), Some([4, 0, 255]));
        assert_eq!(image.pixel(3, 0), None);
        assert_eq!(image.pixel(0, -1), None);
    }

    #[test]
    fn png_round_trips() {
        let image = sample();
        let bytes = image.to_png().unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!((info.width, info.height), (3, 2));
        assert_eq!(buf, image.rgb);
    }
}
