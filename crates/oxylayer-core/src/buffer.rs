use crate::color::{Rgba32F, Rgba8};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// High-performance 2D RGBA8 pixel storage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelBuffer {
    width: u32,
    height: u32,
    data: Vec<u8>, // RGBA 4 bytes per pixel
}

impl PixelBuffer {
    /// Creates a new buffer initialized to transparent black
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            data: vec![0; size],
        }
    }

    /// Creates a buffer filled with a specific color
    pub fn filled(width: u32, height: u32, color: Rgba8) -> Self {
        let mut buf = Self::new(width, height);
        buf.fill(color);
        buf
    }

    /// Creates a buffer from existing RGBA bytes
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        if data.len() == (width as usize) * (height as usize) * 4 {
            Some(Self { width, height, data })
        } else {
            None
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn as_raw(&self) -> &[u8] {
        &self.data
    }

    #[inline]
    pub fn as_raw_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    #[inline]
    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Rgba8> {
        if x < self.width && y < self.height {
            let idx = ((y * self.width + x) * 4) as usize;
            Some(Rgba8 {
                r: self.data[idx],
                g: self.data[idx + 1],
                b: self.data[idx + 2],
                a: self.data[idx + 3],
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Rgba8) {
        if x < self.width && y < self.height {
            let idx = ((y * self.width + x) * 4) as usize;
            self.data[idx] = color.r;
            self.data[idx + 1] = color.g;
            self.data[idx + 2] = color.b;
            self.data[idx + 3] = color.a;
        }
    }

    /// Bilinear sampled pixel interpolation with normalized coordinates [0.0, 1.0]
    pub fn sample_bilinear(&self, u: f32, v: f32) -> Rgba32F {
        if self.width == 0 || self.height == 0 {
            return Rgba32F::TRANSPARENT;
        }

        let x = u * (self.width as f32 - 1.0);
        let y = v * (self.height as f32 - 1.0);

        let x0 = x.floor().clamp(0.0, (self.width - 1) as f32) as u32;
        let y0 = y.floor().clamp(0.0, (self.height - 1) as f32) as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let fx = x - x0 as f32;
        let fy = y - y0 as f32;

        let p00 = self.get_pixel(x0, y0).unwrap().to_f32();
        let p10 = self.get_pixel(x1, y0).unwrap().to_f32();
        let p01 = self.get_pixel(x0, y1).unwrap().to_f32();
        let p11 = self.get_pixel(x1, y1).unwrap().to_f32();

        let top_r = p00.r * (1.0 - fx) + p10.r * fx;
        let top_g = p00.g * (1.0 - fx) + p10.g * fx;
        let top_b = p00.b * (1.0 - fx) + p10.b * fx;
        let top_a = p00.a * (1.0 - fx) + p10.a * fx;

        let bot_r = p01.r * (1.0 - fx) + p11.r * fx;
        let bot_g = p01.g * (1.0 - fx) + p11.g * fx;
        let bot_b = p01.b * (1.0 - fx) + p11.b * fx;
        let bot_a = p01.a * (1.0 - fx) + p11.a * fx;

        Rgba32F {
            r: top_r * (1.0 - fy) + bot_r * fy,
            g: top_g * (1.0 - fy) + bot_g * fy,
            b: top_b * (1.0 - fy) + bot_b * fy,
            a: top_a * (1.0 - fy) + bot_a * fy,
        }
    }

    /// Fills entire buffer with color
    pub fn fill(&mut self, color: Rgba8) {
        let arr = color.to_array();
        self.data.par_chunks_exact_mut(4).for_each(|chunk| {
            chunk.copy_from_slice(&arr);
        });
    }

    /// Clears to transparent black
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// Flips image horizontally in-place
    pub fn flip_horizontal(&mut self) {
        let w = self.width as usize;
        self.data.par_chunks_exact_mut(w * 4).for_each(|row| {
            for x in 0..(w / 2) {
                let left = x * 4;
                let right = (w - 1 - x) * 4;
                for i in 0..4 {
                    row.swap(left + i, right + i);
                }
            }
        });
    }

    /// Flips image vertically in-place
    pub fn flip_vertical(&mut self) {
        let w = self.width as usize;
        let h = self.height as usize;
        let row_bytes = w * 4;
        let mut temp_row = vec![0u8; row_bytes];

        for y in 0..(h / 2) {
            let top_offset = y * row_bytes;
            let bot_offset = (h - 1 - y) * row_bytes;
            temp_row.copy_from_slice(&self.data[top_offset..top_offset + row_bytes]);
            self.data.copy_within(bot_offset..bot_offset + row_bytes, top_offset);
            self.data[bot_offset..bot_offset + row_bytes].copy_from_slice(&temp_row);
        }
    }

    /// Resizes buffer using bilinear interpolation
    pub fn resize_bilinear(&self, new_width: u32, new_height: u32) -> Self {
        let mut target = Self::new(new_width, new_height);
        if new_width == 0 || new_height == 0 || self.width == 0 || self.height == 0 {
            return target;
        }

        let nw = new_width;
        let nh = new_height;
        let self_ref = self;

        target
            .data
            .par_chunks_exact_mut((nw * 4) as usize)
            .enumerate()
            .for_each(|(y, row_chunk)| {
                let v = if nh > 1 { y as f32 / (nh - 1) as f32 } else { 0.0 };
                for x in 0..nw {
                    let u = if nw > 1 { x as f32 / (nw - 1) as f32 } else { 0.0 };
                    let sample = self_ref.sample_bilinear(u, v).to_rgba8();
                    let idx = (x * 4) as usize;
                    row_chunk[idx] = sample.r;
                    row_chunk[idx + 1] = sample.g;
                    row_chunk[idx + 2] = sample.b;
                    row_chunk[idx + 3] = sample.a;
                }
            });

        target
    }

    /// Blits another buffer onto this buffer at `(dst_x, dst_y)`
    pub fn blit(&mut self, src: &PixelBuffer, dst_x: i32, dst_y: i32) {
        let sw = src.width as i32;
        let sh = src.height as i32;
        let dw = self.width as i32;
        let dh = self.height as i32;

        let start_x = dst_x.max(0);
        let end_x = (dst_x + sw).min(dw);
        let start_y = dst_y.max(0);
        let end_y = (dst_y + sh).min(dh);

        if start_x >= end_x || start_y >= end_y {
            return;
        }

        for y in start_y..end_y {
            let sy = (y - dst_y) as u32;
            let src_row_start = ((sy * src.width + (start_x - dst_x) as u32) * 4) as usize;
            let count = ((end_x - start_x) * 4) as usize;
            let dst_row_start = ((y as u32 * self.width + start_x as u32) * 4) as usize;

            self.data[dst_row_start..dst_row_start + count]
                .copy_from_slice(&src.data[src_row_start..src_row_start + count]);
        }
    }
}
