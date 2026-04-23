use picea::math::vector::Vector;

fn main() {
    let lhs = Vector::new(1.0, 2.0);
    let rhs = Vector::new(3.0, 4.0);

    let _ = !lhs;
    let _ = lhs ^ rhs;
    let _ = lhs >> rhs;
}
