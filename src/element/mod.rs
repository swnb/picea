use crate::{
    algo::constraint::update_elements_by_duration,
    math::{
        point::Point,
        vector::{Vector, Vector3},
    },
    meta::{Mass, Meta},
    scene::AxisDirection,
    shape::{circle::CircleShape, rect::RectShape},
};

type ID = u32;

#[derive(Clone, Debug)]
pub enum ElementShape {
    Rect(RectShape),
    Circle(CircleShape),
}

impl ElementShape {
    pub fn projection_on_axis(&self, axis_direction: AxisDirection) -> (f32, f32) {
        use AxisDirection::*;
        use ElementShape::*;
        match self {
            Rect(shape) => match axis_direction {
                X => shape.projection_on_x_axis(),
                Y => shape.projection_on_y_axis(),
            },
            Circle(shape) => {
                let center_point = shape.get_center_point();
                let (center_x, center_y): (f32, f32) = center_point.into();
                let radius = shape.radius();
                match axis_direction {
                    X => (center_x - radius, center_x + radius),
                    Y => (center_y - radius, center_y + radius),
                }
            }
        }
    }

    pub fn project_on_axis(&self, axis: Vector<f32>) -> (f32, f32) {
        use ElementShape::*;
        match self {
            Rect(shape) => shape
                .corner_iter()
                .fold((f32::MAX, f32::MIN), |mut pre, &corner| {
                    let size = corner >> axis;
                    if size < pre.0 {
                        pre.0 = size
                    }
                    if size > pre.1 {
                        pre.1 = size
                    }
                    pre
                }),
            Circle(shape) => {
                // TODO 实现圆的投影逻辑
                unimplemented!()
            }
        }
    }

    pub fn compute_center_point(&self) -> Point<f32> {
        use ElementShape::*;
        match self {
            Rect(shape) => shape.compute_center(),
            Circle(shape) => shape.get_center_point(),
        }
    }

    pub fn projection(&self, vector: Vector<f32>) -> (Point<f32>, Point<f32>) {
        use ElementShape::*;
        match self {
            Rect(shape) => shape.projection(vector),
            // TODO implement this
            Circle(shape) => unimplemented!(),
        }
    }

    pub fn translate(&mut self, vector: &Vector<f32>) {
        use ElementShape::*;

        match self {
            Rect(shape) => shape.translate(vector),
            Circle(shape) => shape.translate(vector),
        }
    }

    pub fn rotate(&mut self, deg: f32) {
        use ElementShape::*;

        match self {
            Rect(shape) => shape.rotate(deg),
            Circle(shape) => {
                // TODO impl circle rotate deg
            }
        }
    }
}

// TODO refactor element builder
pub struct ElementBuilder {
    shape: ElementShape,
    meta: Meta,
}

impl ElementBuilder {
    pub fn new(shape: ElementShape, meta: Meta) -> Self {
        Self { shape, meta }
    }

    pub fn shape(mut self, shape: ElementShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }
}

#[derive(Clone)]
pub struct Element {
    id: ID,
    shape: ElementShape,
    meta: Meta,
}

impl Element {
    pub fn id(&self) -> ID {
        self.id
    }

    pub(crate) fn inject_id(&mut self, id: ID) {
        self.id = id
    }

    pub fn new(shape: impl Into<ElementShape>, meta: impl Into<Meta>) -> Self {
        let mut shape = shape.into();
        let mut meta = meta.into();

        // TODO impl circle rotate

        if let ElementShape::Rect(ref mut shape) = shape {
            shape.rotate(meta.angular());
        }

        let moment_of_inertia = compute_moment_of_inertia(&shape, meta.mass());

        meta.set_moment_of_inertia(|_| moment_of_inertia);

        let id = 0;

        Self { id, shape, meta }
    }

    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut Meta {
        &mut self.meta
    }

    pub fn shape(&self) -> &ElementShape {
        &self.shape
    }

    pub fn shape_mut(&mut self) -> &mut ElementShape {
        &mut self.shape
    }

    pub fn tick(&mut self, secs: f32) {
        if !self.meta.is_fixed() {
            update_elements_by_duration(self, secs)
        }
    }

    /**
     * assume point is inside element
     */
    pub(crate) fn compute_point_velocity(&self, point: Point<f32>) -> Vector<f32> {
        let center_point = self.shape.compute_center_point();
        let w = self.meta().angular_velocity();
        let w: Vector3<_> = (0., 0., w).into();
        let r: Vector<_> = (center_point, point).into();
        let angular_velocity = w ^ r.into();
        let velocity = self.meta().velocity();
        velocity + Vector::from(angular_velocity)
    }
}

// compute moment of inertia;
fn compute_moment_of_inertia(shape: &ElementShape, m: Mass) -> f32 {
    use ElementShape::*;

    match shape {
        Rect(shape) => {
            // m(x^2+y^2)/12;
            let (width, height) = shape.compute_bounding();
            (width.powf(2.) + height.powf(2.)) * m * 12f32.recip()
        }
        Circle(shape) => m * shape.radius().powf(2.) * 0.5,
    }
}

pub struct ElementStore {
    vec: Vec<Element>,
    sort_vec: Vec<ID>,
    should_sort: bool, // should use quick sort;
}

impl ElementStore {
    pub fn push(&mut self, element: Element) {
        let id = element.id;
        self.vec.push(element);
        self.sort_vec.push(id);
    }
}
