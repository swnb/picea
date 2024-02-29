use crate::{
    collision::{Collider, Projector, SubCollider},
    element::{ComputeMomentOfInertia, SelfClone, ShapeTraitUnion},
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
};

use super::{
    convex::ConvexPolygon,
    utils::{
        find_nearest_point, map_vertexes_to_offset_from_center, projection_polygon_on_vector,
        split_concave_polygon_to_convex_polygons, VertexesToEdgeIter,
    },
    CenterPoint, EdgeIterable, GeometryTransformFromOrigin, NearestPoint, Transform,
};

#[derive(Clone)]
pub struct ConcavePolygon {
    origin_vertexes: Vec<Point>,
    origin_vertexes_offset_from_center: Vec<Vector>,
    vertexes: Vec<Point>,
    sub_convex_polygons: Vec<ConvexPolygon>,
    origin_center_point: Point,
    current_center_point: Point,
    area: FloatNum,
}

impl ConcavePolygon {
    pub fn new(vertexes: impl Into<Vec<Point>>) -> Self {
        let origin_vertexes: Vec<Point> = vertexes.into();

        let sub_convex_polygons: Vec<_> =
            split_concave_polygon_to_convex_polygons(&origin_vertexes)
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

        let origin_vertexes_offset_from_center =
            map_vertexes_to_offset_from_center(origin_vertexes.iter(), &center_point).collect();

        Self {
            origin_vertexes,
            origin_vertexes_offset_from_center,
            vertexes: origin_vertexes.clone(),
            sub_convex_polygons,
            current_center_point: center_point,
            origin_center_point: center_point,
            area: total_area,
        }
    }

    pub fn convex_polygons_iter(&self) -> impl Iterator<Item = &ConvexPolygon> {
        self.sub_convex_polygons.iter()
    }

    pub fn to_convex_polygons(self) -> impl Iterator<Item = ConvexPolygon> {
        self.sub_convex_polygons.into_iter()
    }
}

impl GeometryTransformFromOrigin for ConcavePolygon {
    fn transform_from_origin<'a>(&mut self, transform: Transform<'a>) {
        if transform.is_zero() {
            return;
        }

        let mut rotate = || {
            for (i, offset) in self.origin_vertexes_offset_from_center.iter().enumerate() {
                let offset = offset.affine_transformation_rotate(transform.rad);
                let current_point = self.center_point() + offset;
                self.vertexes[i] = current_point;
            }
            self.sub_convex_polygons
                .iter_mut()
                .for_each(|sub_convex_polygon| {
                    sub_convex_polygon.rotate_from_center_point(
                        &self.origin_center_point,
                        &self.current_center_point,
                        transform.rad,
                    )
                });
        };

        if transform.vector.is_zero() {
            rotate()
        } else if transform.rad == 0. {
            self.current_center_point = self.origin_center_point + transform.vector;
            for i in 0..self.vertexes.len() {
                self.vertexes[i] = self.origin_vertexes[i] + transform.vector;
            }
        } else {
            self.current_center_point = self.origin_center_point + transform.vector;
            rotate()
        }
    }
}

// impl GeometryTransform for ConcavePolygon {
//     fn translate(&mut self, vector: &Vector) {
//         self.sub_convex_polygons
//             .iter_mut()
//             .for_each(|convex_polygon| convex_polygon.translate(vector));
//         translate_polygon(self.origin_vertexes.iter_mut(), vector);
//         self.center_point += vector;
//     }

//     fn rotate(&mut self, origin_point: &Point, rad: f32) {
//         self.sub_convex_polygons
//             .iter_mut()
//             .for_each(|convex_polygon| convex_polygon.rotate(origin_point, rad));

//         rotate_polygon(*origin_point, self.origin_vertexes.iter_mut(), rad);

//         if origin_point != &self.center_point {
//             self.center_point = rotate_point(&self.center_point, origin_point, rad);
//         }
//     }

//     fn scale(&mut self, from: &Point, to: &Point) {
//         self.sub_convex_polygons
//             .iter_mut()
//             .for_each(|convex_polygon| {
//                 convex_polygon.scale_with_center_point(&self.center_point, from, to)
//             });
//     }
// }

impl CenterPoint for ConcavePolygon {
    fn center_point(&self) -> Point {
        self.current_center_point
    }
}

impl Projector for ConcavePolygon {
    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
        projection_polygon_on_vector(self.vertexes.iter(), *vector)
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
}

impl EdgeIterable for ConcavePolygon {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(VertexesToEdgeIter::new(&self.vertexes))
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

impl SelfClone for ConcavePolygon {
    fn self_clone(&self) -> Box<dyn ShapeTraitUnion> {
        self.clone().into()
    }
}
