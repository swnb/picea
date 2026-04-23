use picea::math::{point::Point, vector::Vector};

#[test]
fn named_algebra_methods_cover_previous_operator_tricks() {
    let vector: Vector = Vector::new(3.0, 4.0);
    let axis: Vector = Vector::new(2.0, 0.0);

    assert_eq!(vector.length_squared(), 25.0);
    assert!((vector.length() - 5.0).abs() <= f32::EPSILON);
    assert_eq!(vector.dot(axis), 6.0);
    assert_eq!(vector.cross(axis), -8.0);
    assert_eq!(vector.perp(), Vector::new(4.0, -3.0));
    assert_eq!(vector.project_onto(axis), 3.0);
    assert_eq!(Vector::from(Point::new(1.0, 2.0)), Vector::new(1.0, 2.0));

    let normalized = vector.normalized();
    assert!((normalized.length() - 1.0).abs() <= f32::EPSILON);
    let zero: Vector = Vector::default();
    assert_eq!(zero.normalized_or_zero(), Vector::default());
    let unit_y: Vector = Vector::new(0.0, 1.0);
    assert_eq!(
        unit_y.rotated(std::f32::consts::FRAC_PI_2),
        Vector::new(1.0, 0.0)
    );
}
