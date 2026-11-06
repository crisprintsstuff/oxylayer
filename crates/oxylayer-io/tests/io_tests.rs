use oxylayer_core::{Document, Layer, PixelBuffer, Rgba8};
use oxylayer_io::{ImageCodec, ProjectFile};
use std::env::temp_dir;

#[test]
fn test_image_png_roundtrip() {
    let mut buf = PixelBuffer::new(32, 32);
    buf.set_pixel(10, 10, Rgba8::new(255, 128, 64, 255));

    let path = temp_dir().join("test_roundtrip.png");
    ImageCodec::save_png(&buf, &path).expect("Save PNG");

    let loaded = ImageCodec::load_image(&path).expect("Load PNG");
    assert_eq!(loaded.width(), 32);
    assert_eq!(loaded.height(), 32);
    assert_eq!(loaded.get_pixel(10, 10).unwrap(), Rgba8::new(255, 128, 64, 255));

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_project_file_roundtrip() {
    let mut doc = Document::new("My Project", 200, 150);
    let layer = Layer::new_raster("Extra Layer", PixelBuffer::filled(200, 150, Rgba8::BLUE));
    doc.add_layer(layer);

    let path = temp_dir().join("test_project.oxy");
    ProjectFile::save(&doc, &path).expect("Save Oxy Project");

    let loaded = ProjectFile::load(&path).expect("Load Oxy Project");
    assert_eq!(loaded.title, "My Project");
    assert_eq!(loaded.width, 200);
    assert_eq!(loaded.height, 150);
    assert_eq!(loaded.layer_count(), 2);

    let _ = std::fs::remove_file(path);
}
