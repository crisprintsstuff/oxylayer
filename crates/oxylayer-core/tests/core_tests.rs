use oxylayer_core::{
    BlendMode, Document, HistoryManager, Layer, PixelBuffer, Rgba32F, Rgba8,
};

#[test]
fn test_color_srgb_linear_roundtrip() {
    let original = Rgba32F::new(0.5, 0.25, 0.75, 1.0);
    let linear = original.srgb_to_linear();
    let back = linear.linear_to_srgb();

    assert!((original.r - back.r).abs() < 1e-4);
    assert!((original.g - back.g).abs() < 1e-4);
    assert!((original.b - back.b).abs() < 1e-4);
}

#[test]
fn test_color_hsl_roundtrip() {
    let original = Rgba32F::new(0.2, 0.6, 0.9, 1.0);
    let (h, s, l, a) = original.to_hsl();
    let back = Rgba32F::from_hsl(h, s, l, a);

    assert!((original.r - back.r).abs() < 1e-3);
    assert!((original.g - back.g).abs() < 1e-3);
    assert!((original.b - back.b).abs() < 1e-3);
}

#[test]
fn test_blend_modes() {
    let white = Rgba32F::WHITE;
    let black = Rgba32F::BLACK;
    let red = Rgba32F::new(1.0, 0.0, 0.0, 1.0);

    // Normal blend
    let res = BlendMode::Normal.blend(white, red, 1.0);
    assert_eq!(res.r, 1.0);
    assert_eq!(res.g, 0.0);
    assert_eq!(res.b, 0.0);

    // Multiply: white * red = red
    let res = BlendMode::Multiply.blend(white, red, 1.0);
    assert_eq!(res.r, 1.0);
    assert_eq!(res.g, 0.0);
    assert_eq!(res.b, 0.0);

    // Multiply: black * red = black
    let res = BlendMode::Multiply.blend(black, red, 1.0);
    assert_eq!(res.r, 0.0);
    assert_eq!(res.g, 0.0);
    assert_eq!(res.b, 0.0);

    // Screen: black + red = red
    let res = BlendMode::Screen.blend(black, red, 1.0);
    assert_eq!(res.r, 1.0);
    assert_eq!(res.g, 0.0);
    assert_eq!(res.b, 0.0);
}

#[test]
fn test_buffer_flip_and_resize() {
    let mut buf = PixelBuffer::new(2, 2);
    buf.set_pixel(0, 0, Rgba8::RED);
    buf.set_pixel(1, 0, Rgba8::BLUE);
    buf.set_pixel(0, 1, Rgba8::GREEN);
    buf.set_pixel(1, 1, Rgba8::WHITE);

    buf.flip_horizontal();
    assert_eq!(buf.get_pixel(0, 0).unwrap(), Rgba8::BLUE);
    assert_eq!(buf.get_pixel(1, 0).unwrap(), Rgba8::RED);

    let resized = buf.resize_bilinear(4, 4);
    assert_eq!(resized.width(), 4);
    assert_eq!(resized.height(), 4);
}

#[test]
fn test_document_composite() {
    let mut doc = Document::new("Test Canvas", 100, 100);
    assert_eq!(doc.layer_count(), 1);

    let top_buf = PixelBuffer::filled(100, 100, Rgba8::new(255, 0, 0, 128));
    let mut top_layer = Layer::new_raster("Red Overlay", top_buf);
    top_layer.opacity = 0.5;
    doc.add_layer(top_layer);

    let comp = doc.composite();
    assert_eq!(comp.width(), 100);
    assert_eq!(comp.height(), 100);

    // Top left should be a blend of white background and red overlay
    let px = comp.get_pixel(0, 0).unwrap();
    assert!(px.r > 200);
    assert!(px.g > 100 && px.g < 255);
}

#[test]
fn test_history_undo_redo() {
    let mut doc = Document::new("Doc", 50, 50);
    let mut history = HistoryManager::new(10);

    history.push_snapshot(&doc);
    let layer = Layer::new_raster("Extra", PixelBuffer::new(50, 50));
    doc.add_layer(layer);
    assert_eq!(doc.layer_count(), 2);

    assert!(history.can_undo());
    assert!(history.undo(&mut doc));
    assert_eq!(doc.layer_count(), 1);

    assert!(history.can_redo());
    assert!(history.redo(&mut doc));
    assert_eq!(doc.layer_count(), 2);
}
