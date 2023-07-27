use crate::math::{vector::Vector, FloatNum};

use super::ID;

pub type ElementPositionUpdateCallback = dyn FnMut(ID, Vector, FloatNum);

#[derive(Default)]
pub(crate) struct CallbackHook {
    element_position_update_callbacks: Vec<Box<ElementPositionUpdateCallback>>,
}

impl CallbackHook {
    pub fn new() -> Self {
        Self {
            element_position_update_callbacks: Vec::new(),
        }
    }

    pub fn register_callback<F>(&mut self, callback: F)
    where
        F: FnMut(ID, Vector, FloatNum) + 'static,
    {
        self.element_position_update_callbacks
            .push(Box::new(callback));
    }

    pub fn emit(&mut self, id: ID, translate: Vector, rotation: FloatNum) {
        for callback in self.element_position_update_callbacks.iter_mut() {
            callback(id, translate, rotation);
        }
    }
}
