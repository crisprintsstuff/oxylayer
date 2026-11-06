use glam::Vec2;
use oxylayer_brush::{BrushEngine, BrushSettings};
use oxylayer_core::{PixelBuffer, Rgba8};

#[test]
fn test_brush_stamp_and_stroke() {
    let mut buf = PixelBuffer::filled(50, 50, Rgba8::WHITE);
    let settings = BrushSettings {
        size: 10.0,
        hardness: 1.0,
        opacity: 1.0,
        flow: 1.0,
        spacing: 0.2,
        color: Rgba8::BLACK,
        is_eraser: false,
    };

    BrushEngine::stamp(&mut buf, Vec2::new(25.0, 25.0), &settings);
    let center = buf.get_pixel(25, 25).unwrap();
    assert_eq!(center, Rgba8::BLACK);

    let corner = buf.get_pixel(0, 0).unwrap();
    assert_eq!(corner, Rgba8::WHITE);

    let mut residual = 0.0;
    BrushEngine::stroke_segment(
        &mut buf,
        Vec2::new(10.0, 10.0),
        Vec2::new(20.0, 10.0),
        &settings,
        &mut residual,
    );
    assert_eq!(buf.get_pixel(15, 10).unwrap(), Rgba8::BLACK);
}

#[test]
fn test_eraser() {
    let mut buf = PixelBuffer::filled(30, 30, Rgba8::RED);
    let settings = BrushSettings {
        size: 12.0,
        hardness: 1.0,
        opacity: 1.0,
        flow: 1.0,
        spacing: 0.2,
        color: Rgba8::BLACK,
        is_eraser: true,
    };

    BrushEngine::stamp(&mut buf, Vec2::new(15.0, 15.0), &settings);
    let center = buf.get_pixel(15, 15).unwrap();
    assert_eq!(center.a, 0); // Transparent after erase
}
