use crate::color::{Rgba32F, Rgba8};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BlendMode {
    #[default]
    Normal,
    Dissolve,

    // Darken
    Darken,
    Multiply,
    ColorBurn,
    LinearBurn,

    // Lighten
    Lighten,
    Screen,
    ColorDodge,
    LinearDodge,

    // Contrast
    Overlay,
    SoftLight,
    HardLight,
    VividLight,
    LinearLight,
    PinLight,
    HardMix,

    // Inversion / Cancellation
    Difference,
    Exclusion,
    Subtract,
    Divide,

    // Component / HSL
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    pub const ALL: [BlendMode; 23] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::VividLight,
        BlendMode::LinearLight,
        BlendMode::PinLight,
        BlendMode::HardMix,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Subtract,
        BlendMode::Divide,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
        BlendMode::Dissolve,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Dissolve => "Dissolve",
            BlendMode::Darken => "Darken",
            BlendMode::Multiply => "Multiply",
            BlendMode::ColorBurn => "Color Burn",
            BlendMode::LinearBurn => "Linear Burn",
            BlendMode::Lighten => "Lighten",
            BlendMode::Screen => "Screen",
            BlendMode::ColorDodge => "Color Dodge",
            BlendMode::LinearDodge => "Linear Dodge (Add)",
            BlendMode::Overlay => "Overlay",
            BlendMode::SoftLight => "Soft Light",
            BlendMode::HardLight => "Hard Light",
            BlendMode::VividLight => "Vivid Light",
            BlendMode::LinearLight => "Linear Light",
            BlendMode::PinLight => "Pin Light",
            BlendMode::HardMix => "Hard Mix",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
            BlendMode::Subtract => "Subtract",
            BlendMode::Divide => "Divide",
            BlendMode::Hue => "Hue",
            BlendMode::Saturation => "Saturation",
            BlendMode::Color => "Color",
            BlendMode::Luminosity => "Luminosity",
        }
    }

    /// Blends top color `src` over bottom color `dst` with layer opacity
    #[inline]
    pub fn blend(&self, dst: Rgba32F, src: Rgba32F, opacity: f32) -> Rgba32F {
        let eff_src_a = src.a * opacity.clamp(0.0, 1.0);
        if eff_src_a <= 0.0 {
            return dst;
        }
        if dst.a <= 0.0 {
            return Rgba32F {
                r: src.r,
                g: src.g,
                b: src.b,
                a: eff_src_a,
            };
        }

        // Standard compositing equation:
        // out_a = src_a + dst_a * (1 - src_a)
        // out_rgb = (blend_rgb * src_a + dst_rgb * dst_a * (1 - src_a)) / out_a
        let out_a = eff_src_a + dst.a * (1.0 - eff_src_a);
        if out_a <= 1e-6 {
            return Rgba32F::TRANSPARENT;
        }

        let blended_rgb = self.blend_channels(dst, src);

        let r = (blended_rgb.0 * eff_src_a + dst.r * dst.a * (1.0 - eff_src_a)) / out_a;
        let g = (blended_rgb.1 * eff_src_a + dst.g * dst.a * (1.0 - eff_src_a)) / out_a;
        let b = (blended_rgb.2 * eff_src_a + dst.b * dst.a * (1.0 - eff_src_a)) / out_a;

        Rgba32F {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: out_a.clamp(0.0, 1.0),
        }
    }

    #[inline]
    fn blend_channels(&self, dst: Rgba32F, src: Rgba32F) -> (f32, f32, f32) {
        match self {
            BlendMode::Normal | BlendMode::Dissolve => (src.r, src.g, src.b),
            BlendMode::Multiply => (dst.r * src.r, dst.g * src.g, dst.b * src.b),
            BlendMode::Screen => (
                1.0 - (1.0 - dst.r) * (1.0 - src.r),
                1.0 - (1.0 - dst.g) * (1.0 - src.g),
                1.0 - (1.0 - dst.b) * (1.0 - src.b),
            ),
            BlendMode::Overlay => (
                overlay(dst.r, src.r),
                overlay(dst.g, src.g),
                overlay(dst.b, src.b),
            ),
            BlendMode::Darken => (dst.r.min(src.r), dst.g.min(src.g), dst.b.min(src.b)),
            BlendMode::Lighten => (dst.r.max(src.r), dst.g.max(src.g), dst.b.max(src.b)),
            BlendMode::ColorDodge => (
                color_dodge(dst.r, src.r),
                color_dodge(dst.g, src.g),
                color_dodge(dst.b, src.b),
            ),
            BlendMode::ColorBurn => (
                color_burn(dst.r, src.r),
                color_burn(dst.g, src.g),
                color_burn(dst.b, src.b),
            ),
            BlendMode::HardLight => (
                overlay(src.r, dst.r),
                overlay(src.g, dst.g),
                overlay(src.b, dst.b),
            ),
            BlendMode::SoftLight => (
                soft_light(dst.r, src.r),
                soft_light(dst.g, src.g),
                soft_light(dst.b, src.b),
            ),
            BlendMode::LinearBurn => (
                (dst.r + src.r - 1.0).max(0.0),
                (dst.g + src.g - 1.0).max(0.0),
                (dst.b + src.b - 1.0).max(0.0),
            ),
            BlendMode::LinearDodge => (
                (dst.r + src.r).min(1.0),
                (dst.g + src.g).min(1.0),
                (dst.b + src.b).min(1.0),
            ),
            BlendMode::VividLight => (
                vivid_light(dst.r, src.r),
                vivid_light(dst.g, src.g),
                vivid_light(dst.b, src.b),
            ),
            BlendMode::LinearLight => (
                linear_light(dst.r, src.r),
                linear_light(dst.g, src.g),
                linear_light(dst.b, src.b),
            ),
            BlendMode::PinLight => (
                pin_light(dst.r, src.r),
                pin_light(dst.g, src.g),
                pin_light(dst.b, src.b),
            ),
            BlendMode::HardMix => (
                if dst.r + src.r >= 1.0 { 1.0 } else { 0.0 },
                if dst.g + src.g >= 1.0 { 1.0 } else { 0.0 },
                if dst.b + src.b >= 1.0 { 1.0 } else { 0.0 },
            ),
            BlendMode::Difference => (
                (dst.r - src.r).abs(),
                (dst.g - src.g).abs(),
                (dst.b - src.b).abs(),
            ),
            BlendMode::Exclusion => (
                dst.r + src.r - 2.0 * dst.r * src.r,
                dst.g + src.g - 2.0 * dst.g * src.g,
                dst.b + src.b - 2.0 * dst.b * src.b,
            ),
            BlendMode::Subtract => (
                (dst.r - src.r).max(0.0),
                (dst.g - src.g).max(0.0),
                (dst.b - src.b).max(0.0),
            ),
            BlendMode::Divide => (
                if src.r > 1e-5 { (dst.r / src.r).min(1.0) } else { 1.0 },
                if src.g > 1e-5 { (dst.g / src.g).min(1.0) } else { 1.0 },
                if src.b > 1e-5 { (dst.b / src.b).min(1.0) } else { 1.0 },
            ),
            BlendMode::Hue => {
                let (h_src, _, _, _) = src.to_hsl();
                let (_, s_dst, l_dst, _) = dst.to_hsl();
                let res = Rgba32F::from_hsl(h_src, s_dst, l_dst, 1.0);
                (res.r, res.g, res.b)
            }
            BlendMode::Saturation => {
                let (h_dst, _, l_dst, _) = dst.to_hsl();
                let (_, s_src, _, _) = src.to_hsl();
                let res = Rgba32F::from_hsl(h_dst, s_src, l_dst, 1.0);
                (res.r, res.g, res.b)
            }
            BlendMode::Color => {
                let (h_src, s_src, _, _) = src.to_hsl();
                let (_, _, l_dst, _) = dst.to_hsl();
                let res = Rgba32F::from_hsl(h_src, s_src, l_dst, 1.0);
                (res.r, res.g, res.b)
            }
            BlendMode::Luminosity => {
                let (h_dst, s_dst, _, _) = dst.to_hsl();
                let (_, _, l_src, _) = src.to_hsl();
                let res = Rgba32F::from_hsl(h_dst, s_dst, l_src, 1.0);
                (res.r, res.g, res.b)
            }
        }
    }

    /// Fast 8-bit blend convenience
    #[inline]
    pub fn blend_u8(&self, dst: Rgba8, src: Rgba8, opacity: f32) -> Rgba8 {
        self.blend(dst.to_f32(), src.to_f32(), opacity).to_rgba8()
    }
}

#[inline]
fn overlay(d: f32, s: f32) -> f32 {
    if d < 0.5 {
        2.0 * d * s
    } else {
        1.0 - 2.0 * (1.0 - d) * (1.0 - s)
    }
}

#[inline]
fn soft_light(d: f32, s: f32) -> f32 {
    if s <= 0.5 {
        d - (1.0 - 2.0 * s) * d * (1.0 - d)
    } else {
        let d_sqrt = if d <= 0.25 {
            ((16.0 * d - 12.0) * d + 4.0) * d
        } else {
            d.sqrt()
        };
        d + (2.0 * s - 1.0) * (d_sqrt - d)
    }
}

#[inline]
fn color_dodge(d: f32, s: f32) -> f32 {
    if s >= 1.0 {
        1.0
    } else {
        (d / (1.0 - s)).min(1.0)
    }
}

#[inline]
fn color_burn(d: f32, s: f32) -> f32 {
    if s <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - d) / s).min(1.0)
    }
}

#[inline]
fn vivid_light(d: f32, s: f32) -> f32 {
    if s <= 0.5 {
        color_burn(d, 2.0 * s)
    } else {
        color_dodge(d, 2.0 * (s - 0.5))
    }
}

#[inline]
fn linear_light(d: f32, s: f32) -> f32 {
    if s <= 0.5 {
        (d + 2.0 * s - 1.0).max(0.0)
    } else {
        (d + 2.0 * (s - 0.5)).min(1.0)
    }
}

#[inline]
fn pin_light(d: f32, s: f32) -> f32 {
    if s <= 0.5 {
        d.min(2.0 * s)
    } else {
        d.max(2.0 * (s - 0.5))
    }
}
