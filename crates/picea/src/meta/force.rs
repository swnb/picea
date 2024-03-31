use std::{collections::BTreeMap, rc::Rc};

use crate::math::vector::Vector;

// TODO exclude mass
pub struct Force {
    id: Rc<str>,
    force: Vector,
    is_temporary: bool,
}

impl Clone for Force {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            force: self.force,
            is_temporary: self.is_temporary,
        }
    }
}

impl Force {
    pub fn new<'a>(id: impl Into<&'a str>, vector: impl Into<Vector>) -> Self {
        let id = id.into();
        let id = Rc::from(id);
        Self {
            id,
            force: vector.into(),
            is_temporary: false,
        }
    }

    pub fn get_vector(&self) -> Vector {
        self.force
    }

    pub fn set_vector(&mut self, mut reducer: impl FnMut(Vector) -> Vector) {
        self.force = reducer(self.force)
    }

    pub fn is_temporary(&self) -> bool {
        self.is_temporary
    }

    pub fn set_temporary(&mut self, is_temporary: bool) {
        self.is_temporary = is_temporary
    }
}

#[derive(Default, Clone)]
pub struct ForceGroup {
    force_set: BTreeMap<String, Force>,
}

impl ForceGroup {
    pub fn new() -> ForceGroup {
        Default::default()
    }

    pub fn is_empty(&self) -> bool {
        self.force_set.is_empty()
    }

    pub fn add_force(&mut self, force: Force) {
        self.force_set.insert(force.id.to_string(), force);
    }

    pub fn get_force(&self, id: &str) -> Option<&Force> {
        self.force_set.get(id)
    }

    pub fn has_force(&self, id: &str) -> bool {
        self.force_set.contains_key(id)
    }

    pub fn get_force_mut(&mut self, id: &str) -> Option<&mut Force> {
        self.force_set.get_mut(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<Force> {
        self.force_set.remove(id)
    }

    pub fn sum_force(&self) -> Vector {
        self.force_set
            .values()
            .fold((0., 0.).into(), |mut acc, cur| {
                acc += cur.get_vector();
                acc
            })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Force)> {
        self.force_set
            .iter()
            .map(|(id, force)| (id.as_str(), force))
    }
}
