use picea::math::{point::Point, vector::Vector};

#[test]
fn point_and_vector_use_standard_float_equality() {
    let sub_epsilon = f32::EPSILON * 0.75;
    let beyond_epsilon = f32::EPSILON * 1.5;

    assert_ne!(Point::new(0.0, 0.0), Point::new(sub_epsilon, 0.0));
    assert_ne!(Vector::new(0.0, 0.0), Vector::new(sub_epsilon, 0.0));

    // The former epsilon-based operator admitted a non-transitive chain:
    // 0 ~= 0.75e, 0.75e ~= 1.5e, but 0 !~= 1.5e. Value equality must
    // distinguish all three values so comparisons and control flow stay sound.
    assert_ne!(Point::new(sub_epsilon, 0.0), Point::new(0.0, 0.0));
    assert_ne!(
        Point::new(sub_epsilon, 0.0),
        Point::new(beyond_epsilon, 0.0)
    );
    assert_ne!(Point::new(0.0, 0.0), Point::new(beyond_epsilon, 0.0));
    assert_ne!(Vector::new(sub_epsilon, 0.0), Vector::new(0.0, 0.0));
    assert_ne!(
        Vector::new(sub_epsilon, 0.0),
        Vector::new(beyond_epsilon, 0.0)
    );
    assert_ne!(Vector::new(0.0, 0.0), Vector::new(beyond_epsilon, 0.0));

    assert_eq!(Point::new(-0.0, 1.0), Point::new(0.0, 1.0));
    assert_eq!(
        Vector::new(f32::INFINITY, f32::NEG_INFINITY),
        Vector::new(f32::INFINITY, f32::NEG_INFINITY)
    );
    assert_ne!(Point::new(f32::NAN, 0.0), Point::new(f32::NAN, 0.0));
    assert_ne!(Vector::new(0.0, f32::NAN), Vector::new(0.0, f32::NAN));
}

#[test]
fn point_and_vector_support_explicit_absolute_tolerance() {
    let tolerance = 0.25;

    assert!(Point::new(1.0, -2.0).abs_diff_eq(Point::new(1.2, -2.25), tolerance));
    assert!(!Point::new(1.0, -2.0).abs_diff_eq(Point::new(1.2, -2.251), tolerance));
    assert!(Vector::new(1.0, -2.0).abs_diff_eq(Vector::new(1.25, -1.8), tolerance));
    assert!(!Vector::new(1.0, -2.0).abs_diff_eq(Vector::new(1.251, -1.8), tolerance));

    assert!(Point::new(3.0, -4.0).abs_diff_eq(Point::new(3.0, -4.0), 0.0));
    assert!(!Point::new(3.0, -4.0).abs_diff_eq(Point::new(3.0, -4.0), -0.1));
    assert!(!Point::new(3.0, -4.0).abs_diff_eq(Point::new(3.0, -4.0), f32::NAN));
    assert!(!Point::new(3.0, -4.0).abs_diff_eq(Point::new(3.0, -4.0), f32::INFINITY));
    assert!(Vector::new(3.0, -4.0).abs_diff_eq(Vector::new(3.0, -4.0), 0.0));
    assert!(!Vector::new(3.0, -4.0).abs_diff_eq(Vector::new(3.0, -4.0), -0.1));
    assert!(!Vector::new(3.0, -4.0).abs_diff_eq(Vector::new(3.0, -4.0), f32::NAN));
    assert!(!Vector::new(3.0, -4.0).abs_diff_eq(Vector::new(3.0, -4.0), f32::INFINITY));

    assert!(Point::new(-0.0, f32::INFINITY).abs_diff_eq(Point::new(0.0, f32::INFINITY), 0.0));
    assert!(
        Vector::new(f32::NEG_INFINITY, -0.0).abs_diff_eq(Vector::new(f32::NEG_INFINITY, 0.0), 0.0)
    );
    assert!(Point::new(f32::INFINITY, 1.0).abs_diff_eq(Point::new(f32::INFINITY, 1.2), tolerance));
    assert!(Vector::new(f32::NEG_INFINITY, -2.0)
        .abs_diff_eq(Vector::new(f32::NEG_INFINITY, -2.2), tolerance));
    assert!(!Point::new(f32::NAN, 0.0).abs_diff_eq(Point::new(f32::NAN, 0.0), tolerance));
    assert!(!Vector::new(0.0, f32::NAN).abs_diff_eq(Vector::new(0.0, f32::NAN), tolerance));
}

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
    assert!(unit_y
        .rotated(std::f32::consts::FRAC_PI_2)
        .abs_diff_eq(Vector::new(1.0, 0.0), f32::EPSILON));
}
