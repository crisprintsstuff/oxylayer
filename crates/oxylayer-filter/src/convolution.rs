use oxylayer_core::{PixelBuffer, Rgba32F, Rgba8};
use rayon::prelude::*;

/// Fast separable Gaussian Blur
pub fn gaussian_blur(src: &PixelBuffer, radius: f32) -> PixelBuffer {
    if radius <= 0.05 {
        return src.clone();
    }

    let r = radius.ceil() as i32;
    let sigma = radius / 2.0;
    let two_sigma_sq = 2.0 * sigma * sigma;

    // Compute 1D Gaussian kernel
    let mut kernel = Vec::with_capacity((2 * r + 1) as usize);
    let mut sum = 0.0;
    for x in -r..=r {
        let weight = (-((x * x) as f32) / two_sigma_sq).exp();
        kernel.push(weight);
        sum += weight;
    }
    for w in &mut kernel {
        *w /= sum;
    }

    let w = src.width();
    let h = src.height();

    // Horizontal pass
    let mut temp = PixelBuffer::new(w, h);
    let src_ref = src;
    let k_slice = &kernel;

    temp.as_raw_mut()
        .par_chunks_exact_mut((w * 4) as usize)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w {
                let mut acc_r = 0.0;
                let mut acc_g = 0.0;
                let mut acc_b = 0.0;
                let mut acc_a = 0.0;

                for (idx, offset) in (-r..=r).enumerate() {
                    let sample_x = (x as i32 + offset).clamp(0, (w - 1) as i32) as u32;
                    let px = src_ref.get_pixel(sample_x, y as u32).unwrap().to_f32();
                    let weight = k_slice[idx];

                    acc_r += px.r * weight;
                    acc_g += px.g * weight;
                    acc_b += px.b * weight;
                    acc_a += px.a * weight;
                }

                let out_px = Rgba32F::new(acc_r, acc_g, acc_b, acc_a).to_rgba8();
                let out_idx = (x * 4) as usize;
                row[out_idx] = out_px.r;
                row[out_idx + 1] = out_px.g;
                row[out_idx + 2] = out_px.b;
                row[out_idx + 3] = out_px.a;
            }
        });

    // Vertical pass
    let mut output = PixelBuffer::new(w, h);
    let temp_ref = &temp;

    output
        .as_raw_mut()
        .par_chunks_exact_mut((w * 4) as usize)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w {
                let mut acc_r = 0.0;
                let mut acc_g = 0.0;
                let mut acc_b = 0.0;
                let mut acc_a = 0.0;

                for (idx, offset) in (-r..=r).enumerate() {
                    let sample_y = (y as i32 + offset).clamp(0, (h - 1) as i32) as u32;
                    let px = temp_ref.get_pixel(x, sample_y).unwrap().to_f32();
                    let weight = k_slice[idx];

                    acc_r += px.r * weight;
                    acc_g += px.g * weight;
                    acc_b += px.b * weight;
                    acc_a += px.a * weight;
                }

                let out_px = Rgba32F::new(acc_r, acc_g, acc_b, acc_a).to_rgba8();
                let out_idx = (x * 4) as usize;
                row[out_idx] = out_px.r;
                row[out_idx + 1] = out_px.g;
                row[out_idx + 2] = out_px.b;
                row[out_idx + 3] = out_px.a;
            }
        });

    output
}

/// Unsharp mask sharpening
pub fn sharpen(src: &PixelBuffer, amount: f32) -> PixelBuffer {
    if amount <= 0.0 {
        return src.clone();
    }

    let blurred = gaussian_blur(src, 1.5);
    let mut output = src.clone();

    output
        .as_raw_mut()
        .par_chunks_exact_mut(4)
        .zip(blurred.as_raw().par_chunks_exact(4))
        .for_each(|(out_chunk, blur_chunk)| {
            for i in 0..3 {
                let orig = out_chunk[i] as f32;
                let blur = blur_chunk[i] as f32;
                let diff = orig - blur;
                let sharpened = (orig + diff * amount).clamp(0.0, 255.0);
                out_chunk[i] = sharpened as u8;
            }
        });

    output
}

/// Vignette effect
pub fn vignette(buf: &mut PixelBuffer, radius: f32, softness: f32) {
    let w = buf.width();
    let h = buf.height();
    let fw = w as f32;
    let fh = h as f32;
    let center_x = fw * 0.5;
    let center_y = fh * 0.5;
    let max_dist = (center_x.powi(2) + center_y.powi(2)).sqrt();

    buf.as_raw_mut()
        .par_chunks_exact_mut((w * 4) as usize)
        .enumerate()
        .for_each(|(y_idx, row)| {
            let dy = y_idx as f32 - center_y;
            for x_idx in 0..w {
                let dx = x_idx as f32 - center_x;
                let dist = (dx * dx + dy * dy).sqrt() / max_dist;

                let factor = if dist <= radius {
                    1.0
                } else {
                    let d = (dist - radius) / softness.max(0.01);
                    (1.0 - d).clamp(0.0, 1.0)
                };

                let offset = (x_idx * 4) as usize;
                row[offset] = (row[offset] as f32 * factor) as u8;
                row[offset + 1] = (row[offset + 1] as f32 * factor) as u8;
                row[offset + 2] = (row[offset + 2] as f32 * factor) as u8;
            }
        });
}

/// Pixelate / Mosaic effect
pub fn pixelate(src: &PixelBuffer, block_size: u32) -> PixelBuffer {
    if block_size <= 1 {
        return src.clone();
    }

    let mut output = src.clone();
    let w = src.width();
    let h = src.height();

    for by in (0..h).step_by(block_size as usize) {
        for bx in (0..w).step_by(block_size as usize) {
            let block_w = (w - bx).min(block_size);
            let block_h = (h - by).min(block_size);

            let mut sum_r = 0u64;
            let mut sum_g = 0u64;
            let mut sum_b = 0u64;
            let mut sum_a = 0u64;
            let total = (block_w * block_h) as u64;

            for y in by..by + block_h {
                for x in bx..bx + block_w {
                    let px = src.get_pixel(x, y).unwrap();
                    sum_r += px.r as u64;
                    sum_g += px.g as u64;
                    sum_b += px.b as u64;
                    sum_a += px.a as u64;
                }
            }

            let avg = Rgba8 {
                r: (sum_r / total) as u8,
                g: (sum_g / total) as u8,
                b: (sum_b / total) as u8,
                a: (sum_a / total) as u8,
            };

            for y in by..by + block_h {
                for x in bx..bx + block_w {
                    output.set_pixel(x, y, avg);
                }
            }
        }
    }

    output
}
