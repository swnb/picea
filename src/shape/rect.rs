use crate::math::{point::Point, segment::Segment, vector::Vector};

#[derive(Clone, Debug)]
pub struct RectShape {
    top_left_point: Point<f32>,
    top_right_point: Point<f32>,
    bottom_right_point: Point<f32>,
    bottom_left_point: Point<f32>,
}

impl<P: Into<Point<f32>>> From<(P, (f32, f32))> for RectShape {
    fn from((point, (width, height)): (P, (f32, f32))) -> Self {
        let width = width.abs();
        let height = height.abs();

        let vf = Vector::<_>::from;

        let point = point.into();

        Self {
            top_left_point: point,
            top_right_point: point + vf((width, 0.)),
            bottom_right_point: point + vf((width, height)),
            bottom_left_point: point + vf((0., height)),
        }
    }
}

#[derive(Clone, Copy)]
enum RectCornerType {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

pub struct RectCornerIter<'a> {
    rect: &'a RectShape,
    next_corner: Option<RectCornerType>,
}

impl<'a> Iterator for RectCornerIter<'a> {
    type Item = &'a Point<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        use RectCornerType::*;
        match self.next_corner {
            Some(corner) => match corner {
                TopLeft => {
                    self.next_corner = Some(TopRight);
                    Some(&self.rect.top_left_point)
                }
                TopRight => {
                    self.next_corner = Some(BottomRight);
                    Some(&self.rect.top_right_point)
                }
                BottomRight => {
                    self.next_corner = Some(BottomLeft);
                    Some(&self.rect.bottom_right_point)
                }
                BottomLeft => {
                    self.next_corner = None;
                    Some(&self.rect.bottom_left_point)
                }
            },
            None => None,
        }
    }
}

pub struct RectCornerIterMut<'a> {
    rect: &'a mut RectShape,
    next_corner: Option<RectCornerType>,
}

impl<'a> Iterator for RectCornerIterMut<'a> {
    type Item = &'a mut Point<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        use RectCornerType::*;
        match self.next_corner {
            Some(corner) => match corner {
                TopLeft => {
                    self.next_corner = Some(TopRight);
                    let ptr: *mut _ = &mut self.rect.top_left_point;
                    unsafe { Some(&mut *ptr) }
                }
                TopRight => {
                    self.next_corner = Some(BottomRight);
                    let ptr: *mut _ = &mut self.rect.top_right_point;
                    unsafe { Some(&mut *ptr) }
                }
                BottomRight => {
                    self.next_corner = Some(BottomLeft);
                    let ptr: *mut _ = &mut self.rect.bottom_right_point;
                    unsafe { Some(&mut *ptr) }
                }
                BottomLeft => {
                    self.next_corner = None;
                    unsafe { Some(&mut *(&mut self.rect.bottom_left_point as *mut _)) }
                }
            },
            None => None,
        }
    }
}

pub struct SegmentIter<'a> {
    rect: &'a RectShape,
    next_corner: Option<RectCornerType>,
}

impl<'a> Iterator for SegmentIter<'a> {
    type Item = Segment<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        use RectCornerType::*;
        let rect = self.rect;
        match self.next_corner {
            Some(corner) => match corner {
                TopLeft => {
                    self.next_corner = Some(TopRight);
                    let item = (rect.top_left_point, rect.top_right_point).into();
                    Some(item)
                }
                TopRight => {
                    self.next_corner = Some(BottomRight);
                    let item = (rect.top_right_point, rect.bottom_right_point).into();
                    Some(item)
                }
                BottomRight => {
                    self.next_corner = Some(BottomLeft);
                    let item = (rect.bottom_right_point, rect.bottom_left_point).into();
                    Some(item)
                }
                BottomLeft => {
                    self.next_corner = None;
                    let item = (rect.bottom_left_point, rect.top_left_point).into();
                    Some(item)
                }
            },
            None => None,
        }
    }
}

pub struct EdgeIter<'a> {
    rect: &'a RectShape,
    next_corner: Option<RectCornerType>,
}

impl<'a> Iterator for EdgeIter<'a> {
    type Item = Vector<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        use RectCornerType::*;
        let rect = self.rect;
        match self.next_corner {
            Some(corner) => match corner {
                TopLeft => {
                    self.next_corner = Some(TopRight);
                    let item = (rect.top_left_point, rect.top_right_point).into();
                    Some(item)
                }
                TopRight => {
                    self.next_corner = Some(BottomRight);
                    let item = (rect.top_right_point, rect.bottom_right_point).into();
                    Some(item)
                }
                BottomRight => {
                    self.next_corner = Some(BottomLeft);
                    let item = (rect.bottom_right_point, rect.bottom_left_point).into();
                    Some(item)
                }
                BottomLeft => {
                    self.next_corner = None;
                    let item = (rect.bottom_left_point, rect.top_left_point).into();
                    Some(item)
                }
            },
            None => None,
        }
    }
}

impl RectShape {
    #[inline]
    pub fn new<P: Into<Point<f32>>>(corners: [P; 4]) -> Self {
        let corners: Vec<Point<_>> = corners.into_iter().map(|v| v.into()).collect();
        Self {
            top_left_point: corners[0],
            top_right_point: corners[1],
            bottom_right_point: corners[2],
            bottom_left_point: corners[3],
        }
    }

    #[inline]
    pub fn corner_points(&mut self) -> [&Point<f32>; 4] {
        [
            &self.top_left_point,
            &self.top_right_point,
            &self.bottom_right_point,
            &self.bottom_left_point,
        ]
    }

    #[inline]
    pub fn corner_points_mut(&mut self) -> [&mut Point<f32>; 4] {
        [
            &mut self.top_left_point,
            &mut self.top_right_point,
            &mut self.bottom_right_point,
            &mut self.bottom_left_point,
        ]
    }

    #[inline]
    pub fn corner_iter(&self) -> impl Iterator<Item = &Point<f32>> {
        RectCornerIter {
            rect: self,
            next_corner: Some(RectCornerType::TopLeft),
        }
    }

    pub fn edge_iter(&self) -> impl Iterator<Item = Vector<f32>> + '_ {
        EdgeIter {
            rect: self,
            next_corner: Some(RectCornerType::TopLeft),
        }
    }

    #[inline]
    pub fn segment_iter(&self) -> impl Iterator<Item = Segment<f32>> + '_ {
        SegmentIter {
            rect: self,
            next_corner: Some(RectCornerType::TopLeft),
        }
    }

    #[inline]
    pub fn corner_iter_mut(&mut self) -> impl Iterator<Item = &mut Point<f32>> {
        RectCornerIterMut {
            rect: self,
            next_corner: Some(RectCornerType::TopLeft),
        }
    }

    pub fn translate(&mut self, vector: &Vector<f32>) {
        self.corner_iter_mut().for_each(|v| {
            *v += vector;
        })
    }

    pub fn compute_aspect(&self) -> f32 {
        let (width, height) = self.compute_bounding();
        width * height.recip()
    }

    pub fn compute_bounding(&self) -> (f32, f32) {
        let width = self.top_left_point.x() - self.bottom_right_point.x();
        let height = self.bottom_right_point.y() - self.top_left_point.y();
        (width, height)
    }

    pub fn compute_surface_size(&self) -> f32 {
        let (width, height) = self.compute_bounding();
        width * height
    }

    pub fn compute_center(&self) -> Point<f32> {
        let mut iter = self.corner_iter();
        let mut point = iter.next().unwrap().to_vector();
        iter.for_each(|v| {
            point += v.to_vector();
        });
        let center_point = point * 0.25;
        let (x, y) = (center_point).into();
        (x, y).into()
    }

    pub fn projection_on_x_axis(&self) -> (f32, f32) {
        let iter = self.corner_iter();
        iter.fold((f32::MAX, f32::MIN), |mut pre, v| {
            pre.0 = v.x().min(pre.0);
            pre.1 = v.x().max(pre.1);
            pre
        })
    }

    pub fn projection_on_y_axis(&self) -> (f32, f32) {
        let iter = self.corner_iter();
        iter.fold((f32::MAX, f32::MIN), |mut pre, v| {
            pre.0 = v.y().min(pre.0);
            pre.1 = v.y().max(pre.1);
            pre
        })
    }

    pub fn projection(&self, vector: Vector<f32>) -> (Point<f32>, Point<f32>) {
        let mut min = f32::MAX;
        let mut min_point = (0., 0.).into();
        let mut max = f32::MIN;
        let mut max_point = (0., 0.).into();
        self.corner_iter().for_each(|&cur| {
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

    pub fn rotate(&mut self, deg: f32) {
        let center_point: Point<f32> = self.compute_center();
        self.corner_iter_mut().for_each(|corner| {
            let mut corner_vector: Vector<f32> = (center_point, *corner).into();
            corner_vector.affine_transformation_rotate_self(deg);
            *corner = center_point + corner_vector;
        })
    }

    fn indicate_increase_by_endpoint(
        &mut self,
        end_point: impl Into<Point<f32>>,
        start_point: impl Into<Point<f32>>,
        center_point: Option<impl Into<Point<f32>>>,
    ) -> bool {
        let end_point = end_point.into();
        let start_point = start_point.into();
        let center_point: Point<f32> = center_point
            .map(|v| v.into())
            .unwrap_or_else(|| self.compute_center());

        let start_vector: Vector<f32> = (center_point, start_point).into();
        let end_vector: Vector<f32> = (center_point, end_point).into();

        let start_vector_size = start_vector.abs();
        let end_vector_size = end_vector.abs();

        start_vector_size < end_vector_size
    }

    pub fn resize_by_vector(&mut self, vector: impl Into<Vector<f32>>, is_increase: bool) {
        let vector: Vector<f32> = vector.into();
        let (x, y) = vector.into();

        let mut half_x = (x * 0.5).abs();
        let mut half_y = (y * 0.5).abs();

        if !is_increase {
            half_x = -half_x;
            half_y = -half_y;
        }

        self.top_left_point.set_x(|pre_x| pre_x - half_x);
        self.top_left_point.set_y(|pre_y| pre_y - half_y);

        self.top_right_point.set_x(|pre_x| pre_x + half_x);
        self.top_right_point.set_y(|pre_y| pre_y - half_y);

        self.bottom_right_point.set_x(|pre_x| pre_x + half_x);
        self.bottom_right_point.set_y(|pre_y| pre_y + half_y);

        self.bottom_left_point.set_x(|pre_x| pre_x - half_x);
        self.bottom_left_point.set_y(|pre_y| pre_y + half_y);
    }

    /// It resizes the rectangle by a vector.
    ///
    /// Arguments:
    ///
    /// * `size`: the size of the vector to resize by
    /// * `is_increase`: true if the rectangle is to be increased, false if it is to be decreased
    pub fn resize_by_vector_size(&mut self, size: f32, is_increase: bool) {
        let size = size.abs();
        self.compute_aspect();
        let aspect: f32 = self.compute_aspect();
        let y = size * aspect.hypot(1.).recip();
        let x = aspect * y;
        self.resize_by_vector((x, y), is_increase)
    }
}
