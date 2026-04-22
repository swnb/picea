use crate::{
    collision::{Collider, Projector, SubCollider},
    element::{ComputeMomentOfInertia, SelfClone, ShapeTraitUnion},
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
};

use super::{
    convex::ConvexPolygon,
    utils::{
        find_nearest_point, projection_polygon_on_vector, rotate_polygon,
        split_concave_polygon_to_convex_polygons, VerticesToEdgeIter,
    },
    CenterPoint, EdgeIterable, GeometryTransformer, NearestPoint, Transform,
};

#[derive(Clone)]
pub struct ConcavePolygon {
    origin_vertices: Vec<Point>,
    vertices: Vec<Point>,
    sub_convex_polygons: Vec<ConvexPolygon>,
    origin_center_point: Point,
    center_point: Point,
    area: FloatNum,
}

impl ConcavePolygon {
    pub fn new(vertices: impl Into<Vec<Point>>) -> Self {
        let vertices: Vec<Point> = vertices.into();
        let sub_convex_polygons: Vec<_> = split_concave_polygon_to_convex_polygons(&vertices)
            .into_iter()
            .map(ConvexPolygon::new)
            .collect();

        let total_area = sub_convex_polygons
            .iter()
            .fold(0., |acc, convex_polygon| acc + convex_polygon.area());
        let total_area_inv = total_area.recip();

        let center_point = sub_convex_polygons
            .iter()
            .fold(Default::default(), |acc: Vector, convex_polygon| {
                let rate = convex_polygon.area() * total_area_inv;
                acc + convex_polygon.center_point().to_vector() * rate
            })
            .to_point();

        Self {
            origin_vertices: vertices.clone(),
            vertices,
            sub_convex_polygons,
            origin_center_point: center_point,
            center_point,
            area: total_area,
        }
    }

    pub fn to_convex_polygons(self) -> impl Iterator<Item = ConvexPolygon> {
        self.sub_convex_polygons
            .into_iter()
            .map(ConvexPolygon::into_current_pose)
    }
}

impl GeometryTransformer for ConcavePolygon {
    fn sync_transform(&mut self, transform: &Transform) {
        for (i, p) in self.origin_vertices.iter().enumerate() {
            self.vertices[i] = p + transform.translation();
        }

        self.center_point = self.origin_center_point + transform.translation();

        rotate_polygon(
            self.center_point,
            self.vertices.iter_mut(),
            transform.rotation(),
        );

        for sub_convex_polygon in &mut self.sub_convex_polygons {
            sub_convex_polygon.sync_transform_around_point(transform, self.origin_center_point);
        }
    }
}

impl CenterPoint for ConcavePolygon {
    fn center_point(&self) -> Point {
        self.center_point
    }
}

impl Projector for ConcavePolygon {
    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
        projection_polygon_on_vector(self.vertices.iter(), *vector)
    }
}

impl Collider for ConcavePolygon {
    fn sub_colliders(&'_ self) -> Option<Box<dyn Iterator<Item = &'_ dyn SubCollider> + '_>> {
        Some(Box::new(
            self.sub_convex_polygons
                .iter()
                .map(|p| p as &dyn SubCollider),
        ))
    }

    fn measure_sub_collider_concat_point(&self, contact_point: &Point) -> bool {
        self.vertices.iter().any(|p| p == contact_point)
    }
}

impl SelfClone for ConcavePolygon {
    fn self_clone(&self) -> Box<dyn ShapeTraitUnion> {
        self.clone().into()
    }
}

impl EdgeIterable for ConcavePolygon {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(VerticesToEdgeIter::new(&self.vertices))
    }
}

impl NearestPoint for ConcavePolygon {
    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
        find_nearest_point(self, reference_point, direction)
    }
}

impl ComputeMomentOfInertia for ConcavePolygon {
    fn compute_moment_of_inertia(&self, m: crate::meta::Mass) -> f32 {
        let area_inv = self.area.recip();
        self.sub_convex_polygons
            .iter()
            .fold(0., |acc, convex_polygon| {
                let convex_area = convex_polygon.area();
                let rate = convex_area * area_inv;
                acc + convex_polygon.compute_moment_of_inertia(m * rate) * rate
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collision::{Collider, Projector},
        element::ComputeMomentOfInertia,
        math::{point::Point, vector::Vector, FloatNum},
        meta::Transform,
        shape::{
            utils::{
                concave_split_call_count, projection_polygon_on_vector,
                reset_concave_split_call_count, rotate_point,
            },
            CenterPoint,
        },
    };

    const EPSILON: FloatNum = 0.0001;

    fn concave_vertices() -> Vec<Point> {
        points(&[(-1., 1.), (0., 0.), (1., 1.), (1., -1.), (-1., -1.)])
    }

    fn points(raw: &[(FloatNum, FloatNum)]) -> Vec<Point> {
        raw.iter().copied().map(Point::from).collect()
    }

    fn transform_point(point: Point, origin_center_point: Point, transform: &Transform) -> Point {
        let (translation, rotation) = transform.split();
        let translated_point = point + translation;
        let translated_origin = origin_center_point + translation;
        rotate_point(&translated_point, &translated_origin, rotation)
    }

    fn transform_points(
        points: &[Point],
        origin_center_point: Point,
        transform: &Transform,
    ) -> Vec<Point> {
        points
            .iter()
            .copied()
            .map(|point| transform_point(point, origin_center_point, transform))
            .collect()
    }

    fn assert_float_close(actual: FloatNum, expected: FloatNum) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {actual} to be within {EPSILON} of {expected}"
        );
    }

    fn assert_point_close(actual: Point, expected: Point) {
        assert_float_close(actual.x(), expected.x());
        assert_float_close(actual.y(), expected.y());
    }

    fn assert_points_close(actual: &[Point], expected: &[Point]) {
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert_point_close(*actual, *expected);
        }
    }

    fn assert_projection_matches_points(actual: (Point, Point), points: &[Point], vector: Vector) {
        let expected = projection_polygon_on_vector(points.iter(), vector);
        assert_point_close(actual.0, expected.0);
        assert_point_close(actual.1, expected.1);
    }

    #[test]
    fn repeated_transform_uses_cached_local_decomposition() {
        reset_concave_split_call_count();
        let mut polygon = ConcavePolygon::new(concave_vertices());

        let split_count_after_new = concave_split_call_count();
        let expected_sub_collider_count = polygon.sub_convex_polygons.len();
        let expected_local_pieces: Vec<Vec<Point>> = polygon
            .sub_convex_polygons
            .iter()
            .map(|polygon| polygon.origin_vertices().to_vec())
            .collect();

        assert_eq!(split_count_after_new, 1);
        assert_eq!(
            polygon.sub_colliders().unwrap().count(),
            expected_sub_collider_count
        );

        let transforms = [
            Transform::from((Vector::new(4., -2.), 0.)),
            Transform::from((Vector::new(-3., 2.5), 0.7)),
            Transform::from((Vector::new(1.25, -0.75), -0.45)),
        ];

        for transform in transforms {
            polygon.sync_transform(&transform);

            assert_eq!(concave_split_call_count(), split_count_after_new);
            assert_eq!(
                polygon.sub_convex_polygons.len(),
                expected_sub_collider_count
            );
            assert_eq!(
                polygon.sub_colliders().unwrap().count(),
                expected_sub_collider_count
            );

            let actual_local_pieces: Vec<Vec<Point>> = polygon
                .sub_convex_polygons
                .iter()
                .map(|polygon| polygon.origin_vertices().to_vec())
                .collect();
            assert_eq!(actual_local_pieces, expected_local_pieces);
        }
    }

    #[test]
    fn transform_syncs_parent_vertices_and_sub_collider_world_geometry() {
        let mut polygon = ConcavePolygon::new(concave_vertices());
        let origin_center_point = polygon.origin_center_point;
        let local_sub_geometry: Vec<(Vec<Point>, Point)> = polygon
            .sub_convex_polygons
            .iter()
            .map(|polygon| {
                (
                    polygon.origin_vertices().to_vec(),
                    polygon.origin_center_point(),
                )
            })
            .collect();
        let transform = Transform::from((Vector::new(2.5, -3.25), 0.65));
        let projection_vector = Vector::new(0.8, -0.25);

        let expected_parent_vertices =
            transform_points(&polygon.origin_vertices, origin_center_point, &transform);
        let expected_parent_center =
            transform_point(polygon.origin_center_point, origin_center_point, &transform);
        let expected_sub_geometries: Vec<(Vec<Point>, Point)> = local_sub_geometry
            .iter()
            .map(|(vertices, center)| {
                (
                    transform_points(vertices, origin_center_point, &transform),
                    transform_point(*center, origin_center_point, &transform),
                )
            })
            .collect();

        polygon.sync_transform(&transform);

        assert_points_close(&polygon.vertices, &expected_parent_vertices);
        assert_point_close(polygon.center_point(), expected_parent_center);
        assert_projection_matches_points(
            polygon.projection_on_vector(&projection_vector),
            &expected_parent_vertices,
            projection_vector,
        );

        for (sub_polygon, (expected_vertices, expected_center)) in polygon
            .sub_convex_polygons
            .iter()
            .zip(expected_sub_geometries.iter())
        {
            assert_points_close(sub_polygon.vertices(), expected_vertices);
            assert_point_close(sub_polygon.center_point(), *expected_center);
            assert_projection_matches_points(
                sub_polygon.projection_on_vector(&projection_vector),
                expected_vertices,
                projection_vector,
            );
        }

        let sub_colliders: Vec<_> = polygon.sub_colliders().unwrap().collect();
        assert_eq!(sub_colliders.len(), expected_sub_geometries.len());
        for (sub_collider, (expected_vertices, expected_center)) in
            sub_colliders.iter().zip(expected_sub_geometries.iter())
        {
            assert_point_close(sub_collider.center_point(), *expected_center);
            assert_projection_matches_points(
                sub_collider.projection_on_vector(&projection_vector),
                expected_vertices,
                projection_vector,
            );
        }
    }

    #[test]
    fn repeated_transform_keeps_area_and_moment_of_inertia_stable() {
        let mut polygon = ConcavePolygon::new(concave_vertices());
        let mass = 7.5;
        let initial_area = polygon.area;
        let initial_sub_area: FloatNum = polygon
            .sub_convex_polygons
            .iter()
            .map(ConvexPolygon::area)
            .sum();
        let initial_inertia = polygon.compute_moment_of_inertia(mass);

        let transforms = [
            Transform::from((Vector::new(0., 0.), 0.)),
            Transform::from((Vector::new(3.5, 8.), 0.2)),
            Transform::from((Vector::new(-5., 1.), -1.1)),
            Transform::from((Vector::new(6., -4.), 1.4)),
        ];

        for transform in transforms {
            polygon.sync_transform(&transform);

            assert_float_close(polygon.area, initial_area);
            assert_float_close(
                polygon
                    .sub_convex_polygons
                    .iter()
                    .map(ConvexPolygon::area)
                    .sum(),
                initial_sub_area,
            );
            assert_float_close(polygon.compute_moment_of_inertia(mass), initial_inertia);
        }
    }

    #[test]
    fn exposed_convex_polygons_continue_transforming_from_current_world_pose() {
        let mut polygon = ConcavePolygon::new(concave_vertices());
        let parent_transform = Transform::from((Vector::new(5.5, -2.25), 0.8));
        polygon.sync_transform(&parent_transform);

        let mut convex_polygons: Vec<_> = polygon.to_convex_polygons().collect();
        let mut convex_polygon = convex_polygons.remove(0);

        let exposed_vertices = convex_polygon.vertices().to_vec();
        let exposed_center = convex_polygon.center_point();
        let next_transform = Transform::from((Vector::new(-1.25, 3.75), -0.4));
        let expected_vertices =
            transform_points(&exposed_vertices, exposed_center, &next_transform);
        let expected_center = transform_point(exposed_center, exposed_center, &next_transform);
        let projection_vector = Vector::new(0.45, 1.1);

        convex_polygon.sync_transform(&next_transform);

        assert_points_close(convex_polygon.vertices(), &expected_vertices);
        assert_point_close(convex_polygon.center_point(), expected_center);
        assert_projection_matches_points(
            convex_polygon.projection_on_vector(&projection_vector),
            &expected_vertices,
            projection_vector,
        );
    }
}
