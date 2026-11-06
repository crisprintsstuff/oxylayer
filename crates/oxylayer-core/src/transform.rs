use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

/// 2D Transform for layers and objects
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform2D {
    pub translation: Vec2,
    pub rotation: f32, // In radians
    pub scale: Vec2,
    pub pivot: Vec2,   // Normalized pivot within layer bounds (0.5, 0.5 = center)
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            pivot: Vec2::new(0.5, 0.5),
        }
    }
}

impl Transform2D {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            translation: Vec2::new(x, y),
            ..Default::default()
        }
    }

    /// Computes the forward 2D affine matrix for a layer of given width and height
    pub fn to_affine2(&self, width: f32, height: f32) -> Affine2 {
        let pivot_px = Vec2::new(self.pivot.x * width, self.pivot.y * height);

        // Translate to layer position -> Translate by pivot -> Rotate & Scale -> Translate back
        Affine2::from_translation(self.translation + pivot_px)
            * Affine2::from_angle(self.rotation)
            * Affine2::from_scale(self.scale)
            * Affine2::from_translation(-pivot_px)
    }

    /// Computes the inverse matrix to map canvas coordinates back to layer pixel coordinates
    pub fn to_inverse_affine2(&self, width: f32, height: f32) -> Option<Affine2> {
        let forward = self.to_affine2(width, height);
        let det = forward.matrix2.determinant();
        if det.abs() < 1e-6 {
            None
        } else {
            Some(forward.inverse())
        }
    }
}
