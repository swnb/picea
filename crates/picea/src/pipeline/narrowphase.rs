use crate::{
    body::Pose,
    collider::{ShapeAabb, SharedShape},
    math::{point::Point, vector::Vector, FloatNum},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContactGeometry {
    pub(crate) point: Point,
    pub(crate) normal: Vector,
    pub(crate) depth: FloatNum,
}

pub(crate) fn contact_from_shapes(
    shape_a: &SharedShape,
    pose_a: Pose,
    aabb_a: ShapeAabb,
    shape_b: &SharedShape,
    pose_b: Pose,
    aabb_b: ShapeAabb,
) -> Option<ContactGeometry> {
    match (shape_a, shape_b) {
        (SharedShape::Circle { radius: radius_a }, SharedShape::Circle { radius: radius_b }) => {
            contact_from_circles(pose_a.point(), *radius_a, pose_b.point(), *radius_b)
        }
        _ => overlap_from_aabbs(aabb_a, aabb_b),
    }
}

fn contact_from_circles(
    center_a: Point,
    radius_a: FloatNum,
    center_b: Point,
    radius_b: FloatNum,
) -> Option<ContactGeometry> {
    let offset_to_a = center_a - center_b;
    let distance = offset_to_a.length();
    let radius_sum = radius_a + radius_b;
    let depth = radius_sum - distance;
    if depth <= 0.0 {
        return None;
    }

    // The contact normal points from collider B toward collider A, matching
    // the event/debug contract and the existing AABB fallback correction.
    let normal = if distance <= FloatNum::EPSILON {
        Vector::new(-1.0, 0.0)
    } else {
        offset_to_a / distance
    };
    let point_on_a = center_a - normal * radius_a;
    let point_on_b = center_b + normal * radius_b;
    let point = Point::from((Vector::from(point_on_a) + Vector::from(point_on_b)) * 0.5);

    Some(ContactGeometry {
        point,
        normal,
        depth,
    })
}

pub(crate) fn overlap_from_aabbs(a: ShapeAabb, b: ShapeAabb) -> Option<ContactGeometry> {
    let overlap_x = a.max.x().min(b.max.x()) - a.min.x().max(b.min.x());
    let overlap_y = a.max.y().min(b.max.y()) - a.min.y().max(b.min.y());
    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }

    let point = Point::new(
        (a.min.x().max(b.min.x()) + a.max.x().min(b.max.x())) * 0.5,
        (a.min.y().max(b.min.y()) + a.max.y().min(b.max.y())) * 0.5,
    );
    let center_a = Point::new((a.min.x() + a.max.x()) * 0.5, (a.min.y() + a.max.y()) * 0.5);
    let center_b = Point::new((b.min.x() + b.max.x()) * 0.5, (b.min.y() + b.max.y()) * 0.5);
    let delta = center_a - center_b;

    if overlap_x <= overlap_y {
        let normal = if delta.x() <= 0.0 {
            Vector::new(-1.0, 0.0)
        } else {
            Vector::new(1.0, 0.0)
        };
        Some(ContactGeometry {
            point,
            normal,
            depth: overlap_x,
        })
    } else {
        let normal = if delta.y() <= 0.0 {
            Vector::new(0.0, -1.0)
        } else {
            Vector::new(0.0, 1.0)
        };
        Some(ContactGeometry {
            point,
            normal,
            depth: overlap_y,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{contact_from_shapes, overlap_from_aabbs};
    use crate::{
        body::Pose,
        collider::{ShapeAabb, SharedShape},
        math::{point::Point, vector::Vector},
    };

    fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ShapeAabb {
        ShapeAabb {
            min: Point::new(min_x, min_y),
            max: Point::new(max_x, max_y),
        }
    }

    #[test]
    fn narrowphase_rejects_separated_circles_with_overlapping_aabbs() {
        let shape = SharedShape::circle(1.0);
        let contact = contact_from_shapes(
            &shape,
            Pose::from_xy_angle(0.0, 0.0, 0.0),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &shape,
            Pose::from_xy_angle(1.5, 1.5, 0.0),
            aabb(0.5, 0.5, 2.5, 2.5),
        );

        assert_eq!(contact, None);
    }

    #[test]
    fn narrowphase_reports_circle_contact_toward_a() {
        let shape = SharedShape::circle(1.0);
        let contact = contact_from_shapes(
            &shape,
            Pose::from_xy_angle(0.0, 0.0, 0.0),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &shape,
            Pose::from_xy_angle(1.5, 0.0, 0.0),
            aabb(0.5, -1.0, 2.5, 1.0),
        )
        .expect("overlapping circles should contact");

        assert_eq!(contact.normal, Vector::new(-1.0, 0.0));
        assert!((contact.depth - 0.5).abs() < f32::EPSILON);
        assert!((contact.point.x() - 0.75).abs() < f32::EPSILON);
        assert!((contact.point.y() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn coincident_circle_centers_use_deterministic_aabb_tie_normal() {
        let shape = SharedShape::circle(1.0);
        let contact = contact_from_shapes(
            &shape,
            Pose::from_xy_angle(2.0, 3.0, 0.0),
            aabb(1.0, 2.0, 3.0, 4.0),
            &shape,
            Pose::from_xy_angle(2.0, 3.0, 0.0),
            aabb(1.0, 2.0, 3.0, 4.0),
        )
        .expect("coincident circles should contact");

        assert_eq!(contact.normal, Vector::new(-1.0, 0.0));
        assert_eq!(contact.depth, 2.0);
        assert!(contact.point.x().is_finite());
        assert!(contact.point.y().is_finite());
    }

    #[test]
    fn non_circle_pairs_keep_aabb_fallback_behavior() {
        let circle = SharedShape::circle(1.0);
        let rect = SharedShape::rect(2.0, 2.0);
        let circle_aabb = aabb(-1.0, -1.0, 1.0, 1.0);
        let rect_aabb = aabb(0.5, -1.0, 2.5, 1.0);

        let fallback = overlap_from_aabbs(circle_aabb, rect_aabb);
        let contact = contact_from_shapes(
            &circle,
            Pose::default(),
            circle_aabb,
            &rect,
            Pose::from_xy_angle(1.5, 0.0, 0.0),
            rect_aabb,
        );

        assert_eq!(contact, fallback);
    }
}
