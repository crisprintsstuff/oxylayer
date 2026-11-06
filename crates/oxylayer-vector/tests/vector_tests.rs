use oxylayer_core::PixelBuffer;
use oxylayer_vector::VectorElement;

#[test]
fn test_vector_rasterize_rectangle() {
    let mut buf = PixelBuffer::new(100, 100);
    let elem = VectorElement::new_rectangle(20.0, 20.0, 40.0, 40.0, 0.0);
    elem.rasterize(&mut buf);

    // Inside rectangle center (40, 40)
    let inside = buf.get_pixel(40, 40).unwrap();
    assert!(inside.a > 0);

    // Outside rectangle (10, 10)
    let outside = buf.get_pixel(10, 10).unwrap();
    assert_eq!(outside.a, 0);
}

#[test]
fn test_vector_rasterize_circle() {
    let mut buf = PixelBuffer::new(100, 100);
    let elem = VectorElement::new_circle(50.0, 50.0, 20.0);
    elem.rasterize(&mut buf);

    let center = buf.get_pixel(50, 50).unwrap();
    assert!(center.a > 0);

    let outside = buf.get_pixel(10, 10).unwrap();
    assert_eq!(outside.a, 0);
}
