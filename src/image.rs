//! The captured frame: an RGB buffer, how to get one, and how to encode it.

use std::error::Error;
use std::process::{Command, Stdio};

use crate::geometry::Rect;

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
