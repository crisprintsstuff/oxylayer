use crate::blend::BlendMode;
use crate::buffer::PixelBuffer;
use crate::color::Rgba8;
use crate::transform::Transform2D;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub Uuid);

impl LayerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for LayerId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextData {
    pub text: String,
    pub font_size: f32,
    pub color: Rgba8,
    pub letter_spacing: f32,
    pub line_height: f32,
}

impl Default for TextData {
    fn default() -> Self {
        Self {
            text: "Double click to edit".to_string(),
            font_size: 24.0,
            color: Rgba8::WHITE,
            letter_spacing: 0.0,
            line_height: 1.2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerKind {
    Raster(PixelBuffer),
    Text(TextData),
    Group(Vec<LayerId>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32, // 0.0 .. 1.0
    pub blend_mode: BlendMode,
    pub transform: Transform2D,
    pub is_clipping_mask: bool,
    pub mask: Option<PixelBuffer>,
    pub kind: LayerKind,
}

impl Layer {
    pub fn new_raster(name: impl Into<String>, buffer: PixelBuffer) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            is_clipping_mask: false,
            mask: None,
            kind: LayerKind::Raster(buffer),
        }
    }

    pub fn new_text(name: impl Into<String>, text: impl Into<String>, font_size: f32) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            is_clipping_mask: false,
            mask: None,
            kind: LayerKind::Text(TextData {
                text: text.into(),
                font_size,
                ..Default::default()
            }),
        }
    }

    pub fn new_group(name: impl Into<String>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            is_clipping_mask: false,
            mask: None,
            kind: LayerKind::Group(Vec::new()),
        }
    }

    pub fn as_raster(&self) -> Option<&PixelBuffer> {
        match &self.kind {
            LayerKind::Raster(buf) => Some(buf),
            _ => None,
        }
    }

    pub fn as_raster_mut(&mut self) -> Option<&mut PixelBuffer> {
        match &mut self.kind {
            LayerKind::Raster(buf) => Some(buf),
            _ => None,
        }
    }
}
