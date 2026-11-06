use serde::{Deserialize, Serialize};

/// 8-bit per channel RGBA color (sRGB space)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };
    pub const RED: Self = Self { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Self = Self { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255, a: 255 };

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    #[inline]
    pub fn to_f32(self) -> Rgba32F {
        Rgba32F {
            r: self.r as f32 / 255.0,
            g: self.g as f32 / 255.0,
            b: self.b as f32 / 255.0,
            a: self.a as f32 / 255.0,
        }
    }

    #[inline]
    pub fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    #[inline]
    pub fn from_array(arr: [u8; 4]) -> Self {
        Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        }
    }
}

/// 32-bit floating point RGBA color (Linear or sRGB float, normalized 0.0 to 1.0 or HDR)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rgba32F {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba32F {
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub fn to_rgba8(self) -> Rgba8 {
        Rgba8 {
            r: (self.r.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            g: (self.g.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            b: (self.b.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            a: (self.a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        }
    }

    /// Converts sRGB float to linear float (for physically accurate blending)
    #[inline]
    pub fn srgb_to_linear(self) -> Self {
        fn to_lin(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        Self {
            r: to_lin(self.r),
            g: to_lin(self.g),
            b: to_lin(self.b),
            a: self.a,
        }
    }

    /// Converts linear float to sRGB float
    #[inline]
    pub fn linear_to_srgb(self) -> Self {
        fn to_srgb(c: f32) -> f32 {
            if c <= 0.0031308 {
                c * 12.92
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        }
        Self {
            r: to_srgb(self.r),
            g: to_srgb(self.g),
            b: to_srgb(self.b),
            a: self.a,
        }
    }

    /// Converts to HSL (Hue in [0, 360], Saturation in [0, 1], Lightness in [0, 1], Alpha)
    pub fn to_hsl(self) -> (f32, f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;
        let l = (max + min) / 2.0;

        let (h, s) = if delta.abs() < 1e-5 {
            (0.0, 0.0)
        } else {
            let s = if l < 0.5 {
                delta / (max + min)
            } else {
                delta / (2.0 - max - min)
            };

            let h = if (max - self.r).abs() < 1e-5 {
                ((self.g - self.b) / delta) % 6.0
            } else if (max - self.g).abs() < 1e-5 {
                (self.b - self.r) / delta + 2.0
            } else {
                (self.r - self.g) / delta + 4.0
            };

            let h = (h * 60.0 + 360.0) % 360.0;
            (h, s)
        };

        (h, s, l, self.a)
    }

    /// Converts from HSL back to RGB
    pub fn from_hsl(h: f32, s: f32, l: f32, a: f32) -> Self {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let h_prime = (h % 360.0) / 60.0;
        let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r1, g1, b1) = match h_prime as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self {
            r: r1 + m,
            g: g1 + m,
            b: b1 + m,
            a,
        }
    }

    /// Calculates perceptual luminance (Rec. 709)
    #[inline]
    pub fn luminance(&self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }
}
