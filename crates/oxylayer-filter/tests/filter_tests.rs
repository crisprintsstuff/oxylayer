use oxylayer_core::{PixelBuffer, Rgba32F, Rgba8};
use oxylayer_filter::{gaussian_blur, pixelate, sharpen, ColorAdjustments};

#[test]
fn test_color_adjustments_invert() {
    let mut adj = ColorAdjustments::default();
    adj.invert = true;

    let col = Rgba32F::new(0.2, 0.4, 0.6, 1.0);
    let res = adj.apply_pixel(col);

    assert!((res.r - 0.8).abs() < 1e-4);
    assert!((res.g - 0.6).abs() < 1e-4);
    assert!((res.b - 0.4).abs() < 1e-4);
}

#[test]
fn test_color_adjustments_black_and_white() {
    let mut adj = ColorAdjustments::default();
    adj.black_and_white = true;

    let col = Rgba32F::new(0.5, 0.2, 0.8, 1.0);
    let res = adj.apply_pixel(col);

    assert_eq!(res.r, res.g);
    assert_eq!(res.g, res.b);
}

#[test]
fn test_gaussian_blur_and_sharpen() {
    let mut buf = PixelBuffer::new(20, 20);
    buf.set_pixel(10, 10, Rgba8::WHITE);

    let blurred = gaussian_blur(&buf, 2.0);
    let center = blurred.get_pixel(10, 10).unwrap();
    let neighbor = blurred.get_pixel(11, 10).unwrap();

    // Center brightness spreads to neighbors
    assert!(neighbor.r > 0);
    assert!(center.r > neighbor.r);

    let sharpened = sharpen(&blurred, 1.0);
    assert_eq!(sharpened.width(), 20);
}

#[test]
fn test_pixelate() {
    let mut buf = PixelBuffer::filled(16, 16, Rgba8::BLACK);
    buf.set_pixel(0, 0, Rgba8::WHITE);

    let pix = pixelate(&buf, 4);
    // Block 0..4 should have uniform average
    let p0 = pix.get_pixel(0, 0).unwrap();
    let p1 = pix.get_pixel(1, 1).unwrap();
    assert_eq!(p0, p1);
}
