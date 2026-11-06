use kurbo::{BezPath, Circle, Ellipse, Point, Rect, RoundedRect, Shape};
use oxylayer_core::{BlendMode, PixelBuffer, Rgba32F, Rgba8};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VectorShape {
    Rectangle { width: f64, height: f64, corner_radius: f64 },
    Circle { radius: f64 },
    Ellipse { rx: f64, ry: f64 },
    Star { points: u32, inner_radius: f64, outer_radius: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorElement {
    pub shape: VectorShape,
    pub fill_color: Option<Rgba8>,
    pub stroke_color: Option<Rgba8>,
    pub stroke_width: f64,
    pub pos_x: f64,
    pub pos_y: f64,
}

impl VectorElement {
    pub fn new_rectangle(x: f64, y: f64, width: f64, height: f64, corner_radius: f64) -> Self {
        Self {
            shape: VectorShape::Rectangle { width, height, corner_radius },
            fill_color: Some(Rgba8::new(0, 122, 255, 255)), // System Blue
            stroke_color: Some(Rgba8::WHITE),
            stroke_width: 2.0,
            pos_x: x,
            pos_y: y,
        }
    }

    pub fn new_circle(center_x: f64, center_y: f64, radius: f64) -> Self {
        Self {
            shape: VectorShape::Circle { radius },
            fill_color: Some(Rgba8::new(255, 45, 85, 255)), // System Pink
            stroke_color: None,
            stroke_width: 0.0,
            pos_x: center_x,
            pos_y: center_y,
        }
    }

    pub fn new_star(center_x: f64, center_y: f64, points: u32, inner_radius: f64, outer_radius: f64) -> Self {
        Self {
            shape: VectorShape::Star { points, inner_radius, outer_radius },
            fill_color: Some(Rgba8::new(255, 204, 0, 255)), // System Yellow
            stroke_color: Some(Rgba8::BLACK),
            stroke_width: 1.5,
            pos_x: center_x,
            pos_y: center_y,
        }
    }

    /// Converts to kurbo BezPath
    pub fn to_bez_path(&self) -> BezPath {
        match &self.shape {
            VectorShape::Rectangle { width, height, corner_radius } => {
                let rect = Rect::new(self.pos_x, self.pos_y, self.pos_x + width, self.pos_y + height);
                if *corner_radius > 0.0 {
                    RoundedRect::from_rect(rect, *corner_radius).to_path(0.1)
                } else {
                    rect.to_path(0.1)
                }
            }
            VectorShape::Circle { radius } => {
                Circle::new(Point::new(self.pos_x, self.pos_y), *radius).to_path(0.1)
            }
            VectorShape::Ellipse { rx, ry } => {
                Ellipse::new(Point::new(self.pos_x, self.pos_y), (*rx, *ry), 0.0).to_path(0.1)
            }
            VectorShape::Star { points, inner_radius, outer_radius } => {
                let mut path = BezPath::new();
                let pts = *points.max(&3);
                let step = std::f64::consts::PI / pts as f64;

                for i in 0..(pts * 2) {
                    let r = if i % 2 == 0 { *outer_radius } else { *inner_radius };
                    let angle = i as f64 * step - std::f64::consts::FRAC_PI_2;
                    let x = self.pos_x + r * angle.cos();
                    let y = self.pos_y + r * angle.sin();

                    if i == 0 {
                        path.move_to(Point::new(x, y));
                    } else {
                        path.line_to(Point::new(x, y));
                    }
                }
                path.close_path();
                path
            }
        }
    }

    /// Rasterizes vector shape to a target PixelBuffer with anti-aliasing
    pub fn rasterize(&self, target: &mut PixelBuffer) {
        if self.fill_color.is_none() && self.stroke_color.is_none() {
            return;
        }

        let path = self.to_bez_path();
        let bbox = path.bounding_box();
        let min_x = (bbox.x0 - self.stroke_width).floor().max(0.0) as u32;
        let max_x = (bbox.x1 + self.stroke_width).ceil().min(target.width() as f64 - 1.0) as u32;
        let min_y = (bbox.y0 - self.stroke_width).floor().max(0.0) as u32;
        let max_y = (bbox.y1 + self.stroke_width).ceil().min(target.height() as f64 - 1.0) as u32;

        let fill_color = self.fill_color.map(|c| c.to_f32());

        // 2x2 sub-pixel super-sampling for anti-aliasing
        let sub_offsets = [
            (0.25, 0.25),
            (0.75, 0.25),
            (0.25, 0.75),
            (0.75, 0.75),
        ];

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let mut inside_count = 0;

                for (ox, oy) in sub_offsets {
                    let pt = Point::new(x as f64 + ox, y as f64 + oy);
                    // Point-in-polygon test via winding rule
                    if path.winding(pt) != 0 {
                        inside_count += 1;
                    }
                }

                if inside_count > 0 {
                    let coverage = inside_count as f32 / 4.0;
                    if let Some(fill) = fill_color {
                        let existing = target.get_pixel(x, y).unwrap().to_f32();
                        let blended = BlendMode::Normal.blend(
                            existing,
                            Rgba32F::new(fill.r, fill.g, fill.b, fill.a * coverage),
                            1.0,
                        );
                        target.set_pixel(x, y, blended.to_rgba8());
                    }
                }
            }
        }
    }
}
