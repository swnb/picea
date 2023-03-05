use crate::math::{point::Point, vector::Vector};

#[derive(Clone, Debug)]
pub enum ContactType {
    Point(Point<f32>),
    Edge([Point<f32>; 2]),
}

#[derive(Debug)]
pub struct CollisionInfo {
    pub(crate) collision_element_id_pair: (u32, u32),
    pub(crate) contact_a: ContactType,
    pub(crate) contact_b: ContactType,
    pub(crate) depth: f32,
    pub(crate) normal: Vector<f32>,
}

impl CollisionInfo {
    pub fn element_id_a(&self) -> u32 {
        self.collision_element_id_pair.0
    }

    pub fn element_id_b(&self) -> u32 {
        self.collision_element_id_pair.1
    }

    pub fn contact_type(&self, id: u32) -> &ContactType {
        if self.element_id_a() == id {
            self.contact_a()
        } else if self.element_id_b() == id {
            self.contact_b()
        } else {
            unreachable!()
        }
    }

    pub fn contact_a(&self) -> &ContactType {
        &self.contact_a
    }

    pub fn contact_b(&self) -> &ContactType {
        &self.contact_b
    }

    pub fn contact_points(&self) -> (&ContactType, &ContactType) {
        (&self.contact_a, &self.contact_b)
    }

    pub fn depth(&self) -> f32 {
        self.depth
    }

    pub fn normal(&self) -> Vector<f32> {
        self.normal
    }
}
