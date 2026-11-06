use glam::Vec2;
use oxylayer_core::{BlendMode, PixelBuffer, Rgba32F, Rgba8};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrushSettings {
    pub size: f32,       // Diameter in pixels
    pub hardness: f32,   // 0.0 (fuzzy airbrush) to 1.0 (crisp edge)
    pub opacity: f32,    // 0.0 to 1.0
    pub flow: f32,       // 0.0 to 1.0
    pub spacing: f32,    // Fraction of brush radius (default ~0.15)
    pub color: Rgba8,
    pub is_eraser: bool,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            size: 30.0,
            hardness: 0.8,
            opacity: 1.0,
            flow: 1.0,
            spacing: 0.15,
            color: Rgba8::BLACK,
            is_eraser: false,
        }
    }
}

pub struct BrushEngine;

impl BrushEngine {
    /// Renders a single radial brush stamp dab onto the target pixel buffer
    pub fn stamp(target: &mut PixelBuffer, center: Vec2, settings: &BrushSettings) {
        let radius = settings.size * 0.5;
        if radius <= 0.0 {
            return;
        }

        let min_x = (center.x - radius).floor().max(0.0) as u32;
        let max_x = (center.x + radius).ceil().min(target.width() as f32 - 1.0) as u32;
        let min_y = (center.y - radius).floor().max(0.0) as u32;
        let max_y = (center.y + radius).ceil().min(target.height() as f32 - 1.0) as u32;

        let hardness = settings.hardness.clamp(0.0, 0.999);
        let inner_radius = radius * hardness;
        let edge_width = radius - inner_radius;

        let brush_rgb = settings.color.to_f32();
        let dab_alpha = settings.opacity * settings.flow;

        for y in min_y..=max_y {
            let dy = y as f32 + 0.5 - center.y;
            for x in min_x..=max_x {
                let dx = x as f32 + 0.5 - center.x;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= radius {
                    let falloff = if dist <= inner_radius {
                        1.0
                    } else {
                        let t = (dist - inner_radius) / edge_width;
                        // Smoothstep curve for natural brush falloff
                        (1.0 - t * t * (3.0 - 2.0 * t)).clamp(0.0, 1.0)
                    };

                    let effective_alpha = dab_alpha * falloff;
                    if effective_alpha > 0.0 {
                        let existing = target.get_pixel(x, y).unwrap();

                        if settings.is_eraser {
                            let new_a = (existing.a as f32 * (1.0 - effective_alpha)).round() as u8;
                            target.set_pixel(
                                x,
                                y,
                                Rgba8 {
                                    r: existing.r,
                                    g: existing.g,
                                    b: existing.b,
                                    a: new_a,
                                },
                            );
                        } else {
                            let blended = BlendMode::Normal.blend(
                                existing.to_f32(),
                                Rgba32F::new(brush_rgb.r, brush_rgb.g, brush_rgb.b, 1.0),
                                effective_alpha,
                            );
                            target.set_pixel(x, y, blended.to_rgba8());
                        }
                    }
                }
            }
        }
    }

    /// Interpolates continuous stamps along a line segment from `start` to `end`
    pub fn stroke_segment(
        target: &mut PixelBuffer,
        start: Vec2,
        end: Vec2,
        settings: &BrushSettings,
        residual_dist: &mut f32,
    ) {
        let delta = end - start;
        let dist = delta.length();
        let step = (settings.size * settings.spacing).max(1.0);

        if dist < 1e-4 {
            Self::stamp(target, start, settings);
            return;
        }

        let dir = delta / dist;
        let mut travelled = step - *residual_dist;

        while travelled <= dist {
            let pos = start + dir * travelled;
            Self::stamp(target, pos, settings);
            travelled += step;
        }

        *residual_dist = dist - (travelled - step);
    }
}
