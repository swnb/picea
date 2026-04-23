use picea::math::vector::Vector;

fn main() {
    let mut vector = Vector::new(3.0, 4.0);

    let _ = vector.normalize();
    let _ = vector.abs();
    let _ = vector.affine_transformation_rotate(1.0);
    vector.affine_transformation_rotate_self(1.0);
}
