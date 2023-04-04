use crate::math::point::Point;

use super::{convex::ConvexPolygon, utils::split_concave_polygon_to_convex_polygons};

#[derive(Default)]
pub struct ConcavePolygon {
    sub_convex_polygons: Vec<ConvexPolygon>,
}

impl ConcavePolygon {
    pub fn new(vertexes: &[Point]) -> Self {
        let sub_convex_polygons = split_concave_polygon_to_convex_polygons(vertexes)
            .into_iter()
            .map(ConvexPolygon::new)
            .collect();
        Self {
            sub_convex_polygons,
        }
    }
}

/**
 * 凹多边形的转动惯量可以通过以下公式计算：
 * I = Σ(Ig + A * d^2)
 * 其中，Ig是每个三角形相对于重心的转动惯量，A是三角形的面积，d是三角形重心到凹多边形重心的距离。Σ表示对所有三角形求和。
 * 具体而言，对于每个三角形，可以通过以下公式计算其相对于重心的转动惯量：
 * Ig = (1/36) * m * (a^2 + b^2 + c^2)
 * 其中，m是三角形的质量（可以看作是面积），a、b、c是三角形的三条边的长度。该公式可以通过将三角形看作是一个平面薄片并绕过重心的轴旋转来推导出来。
 * 需要注意的是，凹多边形的重心需要使用刚才提到的方法来计算。同时，该公式仅适用于二维几何形状，对于三维几何形状的转动惯量计算，公式会有所不同。
 */
fn compute_moment_of_inertia() {}
