use crate::math::{point::Point, vector::Vector};

use super::Shape;

struct Polygon {
    vertexes: Vec<Point<f32>>,
}

impl Shape for Polygon {
    fn compute_center_point(&self) -> Point<f32> {
        let mut point_iter = self.vertexes.iter();
        let mut point = point_iter.next().unwrap().to_vector();
        point_iter.for_each(|v| {
            point += v.to_vector();
        });
        let (x, y) = (point * (self.vertexes.len() as f32).recip()).into();
        (x, y).into()
    }

    fn projection_on_vector(&self, vector: Vector<f32>) -> (Point<f32>, Point<f32>) {
        let mut min = f32::MAX;
        let mut min_point = (0., 0.).into();
        let mut max = f32::MIN;
        let mut max_point = (0., 0.).into();
        self.vertexes.iter().for_each(|&cur| {
            let size = cur >> vector;
            if size < min {
                min = size;
                min_point = cur;
            }
            if size > max {
                max = size;
                max_point = cur;
            }
        });
        (min_point, max_point)
    }

    fn rotate(&mut self, deg: f32) {
        self.compute_center_point();
    }

    fn translate(&mut self, vector: &Vector<f32>) {
        self.vertexes.iter_mut().for_each(|p| {
            *p += vector;
        })
    }
}
