use oxylayer_core::{PixelBuffer, Rgba32F, Rgba8};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Non-destructive color adjustment parameters (Pixelmator Pro parity)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorAdjustments {
    pub exposure: f32,    // -3.0 to +3.0 EV
    pub brightness: f32,  // -1.0 to +1.0
    pub contrast: f32,    // -1.0 to +1.0
    pub saturation: f32,  // -1.0 to +1.0
    pub vibrance: f32,    // -1.0 to +1.0
    pub temperature: f32, // -1.0 (cool/blue) to +1.0 (warm/yellow)
    pub tint: f32,        // -1.0 (green) to +1.0 (magenta)
    pub highlights: f32,  // -1.0 to +1.0
    pub shadows: f32,     // -1.0 to +1.0
    pub black_and_white: bool,
    pub invert: bool,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            brightness: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            vibrance: 0.0,
            temperature: 0.0,
            tint: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            black_and_white: false,
            invert: false,
        }
    }
}

impl ColorAdjustments {
    pub fn is_identity(&self) -> bool {
        self.exposure.abs() < 1e-4
            && self.brightness.abs() < 1e-4
            && self.contrast.abs() < 1e-4
            && self.saturation.abs() < 1e-4
            && self.vibrance.abs() < 1e-4
            && self.temperature.abs() < 1e-4
            && self.tint.abs() < 1e-4
            && self.highlights.abs() < 1e-4
            && self.shadows.abs() < 1e-4
            && !self.black_and_white
            && !self.invert
    }

    /// Evaluates adjustment on a single 32-bit linear/sRGB color
    #[inline]
    pub fn apply_pixel(&self, mut col: Rgba32F) -> Rgba32F {
        if self.invert {
            col.r = 1.0 - col.r;
            col.g = 1.0 - col.g;
            col.b = 1.0 - col.b;
        }

        // Exposure (EV: 2^exposure)
        if self.exposure.abs() > 1e-4 {
            let mult = 2.0f32.powf(self.exposure);
            col.r *= mult;
            col.g *= mult;
            col.b *= mult;
        }

        // Brightness & Contrast
        if self.brightness.abs() > 1e-4 {
            col.r += self.brightness;
            col.g += self.brightness;
            col.b += self.brightness;
        }
        if self.contrast.abs() > 1e-4 {
            let factor = (1.015 * (self.contrast + 1.0)) / (1.015 - self.contrast);
            col.r = factor * (col.r - 0.5) + 0.5;
            col.g = factor * (col.g - 0.5) + 0.5;
            col.b = factor * (col.b - 0.5) + 0.5;
        }

        // Temperature (cool/warm) & Tint (green/magenta)
        if self.temperature.abs() > 1e-4 {
            col.r += self.temperature * 0.15;
            col.b -= self.temperature * 0.15;
        }
        if self.tint.abs() > 1e-4 {
            col.g -= self.tint * 0.15;
            col.r += self.tint * 0.08;
            col.b += self.tint * 0.08;
        }

        // Highlights & Shadows
        let lum = col.luminance();
        if self.shadows.abs() > 1e-4 && lum < 0.5 {
            let factor = (1.0 - lum * 2.0) * self.shadows * 0.5;
            col.r += factor;
            col.g += factor;
            col.b += factor;
        }
        if self.highlights.abs() > 1e-4 && lum > 0.5 {
            let factor = ((lum - 0.5) * 2.0) * self.highlights * 0.5;
            col.r += factor;
            col.g += factor;
            col.b += factor;
        }

        // Black & White conversion
        if self.black_and_white {
            let gray = col.luminance();
            col.r = gray;
            col.g = gray;
            col.b = gray;
        } else {
            // Vibrance & Saturation
            if self.saturation.abs() > 1e-4 || self.vibrance.abs() > 1e-4 {
                let (h, mut s, l, a) = col.to_hsl();

                if self.saturation.abs() > 1e-4 {
                    s = (s * (1.0 + self.saturation)).clamp(0.0, 1.0);
                }

                if self.vibrance.abs() > 1e-4 {
                    // Vibrance selectively boosts less-saturated colors more
                    let vib_factor = (1.0 - s) * self.vibrance;
                    s = (s + vib_factor).clamp(0.0, 1.0);
                }

                col = Rgba32F::from_hsl(h, s, l, a);
            }
        }

        Rgba32F {
            r: col.r.clamp(0.0, 1.0),
            g: col.g.clamp(0.0, 1.0),
            b: col.b.clamp(0.0, 1.0),
            a: col.a,
        }
    }

    /// Parallel application across an entire PixelBuffer
    pub fn apply_to_buffer(&self, buffer: &mut PixelBuffer) {
        if self.is_identity() {
            return;
        }

        buffer.as_raw_mut().par_chunks_exact_mut(4).for_each(|chunk| {
            let px = Rgba8 {
                r: chunk[0],
                g: chunk[1],
                b: chunk[2],
                a: chunk[3],
            };
            let adj = self.apply_pixel(px.to_f32()).to_rgba8();
            chunk[0] = adj.r;
            chunk[1] = adj.g;
            chunk[2] = adj.b;
            chunk[3] = adj.a;
        });
    }
}
