use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use oxylayer_core::{Document, PixelBuffer};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum IoError {
    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image codec error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid document file format")]
    InvalidFormat,
}

pub struct ImageCodec;

impl ImageCodec {
    /// Loads an image file into a PixelBuffer
    pub fn load_image<P: AsRef<Path>>(path: P) -> Result<PixelBuffer, IoError> {
        let img = image::open(path)?.to_rgba8();
        let (width, height) = img.dimensions();
        let raw = img.into_raw();
        Ok(PixelBuffer::from_raw(width, height, raw).unwrap())
    }

    /// Saves PixelBuffer as PNG
    pub fn save_png<P: AsRef<Path>>(buffer: &PixelBuffer, path: P) -> Result<(), IoError> {
        let w = buffer.width();
        let h = buffer.height();
        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(w, h, buffer.as_raw().to_vec())
            .expect("Valid buffer dimensions");
        img.save_with_format(path, ImageFormat::Png)?;
        Ok(())
    }

    /// Saves PixelBuffer as JPEG
    pub fn save_jpeg<P: AsRef<Path>>(buffer: &PixelBuffer, path: P, _quality: u8) -> Result<(), IoError> {
        let w = buffer.width();
        let h = buffer.height();
        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(w, h, buffer.as_raw().to_vec())
            .expect("Valid buffer dimensions");
        // Convert to RGB for standard JPEG
        let rgb_img = DynamicImage::ImageRgba8(img).to_rgb8();
        rgb_img.save_with_format(path, ImageFormat::Jpeg)?;
        Ok(())
    }

    /// Saves PixelBuffer as WebP
    pub fn save_webp<P: AsRef<Path>>(buffer: &PixelBuffer, path: P) -> Result<(), IoError> {
        let w = buffer.width();
        let h = buffer.height();
        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(w, h, buffer.as_raw().to_vec())
            .expect("Valid buffer dimensions");
        img.save_with_format(path, ImageFormat::WebP)?;
        Ok(())
    }
}

pub struct ProjectFile;

impl ProjectFile {
    const MAGIC: &'static [u8; 4] = b"OXY1";

    /// Saves full non-destructive document to compressed `.oxy` archive
    pub fn save<P: AsRef<Path>>(doc: &Document, path: P) -> Result<(), IoError> {
        let json_bytes = serde_json::to_vec(doc)?;
        let compressed = zstd::encode_all(&json_bytes[..], 3)?;

        let mut file = File::create(path)?;
        file.write_all(Self::MAGIC)?;
        file.write_all(&compressed)?;
        file.flush()?;

        Ok(())
    }

    /// Loads document from `.oxy` archive
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Document, IoError> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;

        if &magic != Self::MAGIC {
            return Err(IoError::InvalidFormat);
        }

        let mut compressed = Vec::new();
        file.read_to_end(&mut compressed)?;

        let json_bytes = zstd::decode_all(&compressed[..])?;
        let doc: Document = serde_json::from_slice(&json_bytes)?;

        Ok(doc)
    }
}
