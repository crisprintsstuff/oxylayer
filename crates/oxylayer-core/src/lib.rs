pub mod blend;
pub mod buffer;
pub mod color;
pub mod document;
pub mod history;
pub mod layer;
pub mod transform;

pub use blend::BlendMode;
pub use buffer::PixelBuffer;
pub use color::{Rgba32F, Rgba8};
pub use document::Document;
pub use history::HistoryManager;
pub use layer::{Layer, LayerId, LayerKind, TextData};
pub use transform::Transform2D;

#[derive(thiserror::Error, Debug)]
pub enum OxyError {
    #[error("Layer not found: {0:?}")]
    LayerNotFound(LayerId),
    #[error("Invalid dimensions: {0}x{1}")]
    InvalidDimensions(u32, u32),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
