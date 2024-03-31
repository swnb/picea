use crate::math::{vector::Vector, FloatNum};

use super::ID;

struct ElementPositionUpdateCallback {
    id: u32,
    callback: Box<dyn FnMut(ID, Vector, FloatNum)>,
}

impl ElementPositionUpdateCallback {
    pub fn call(&mut self, id: ID, vector: Vector, rotation: FloatNum) {
        (self.callback)(id, vector, rotation)
    }
}

#[derive(Default)]
pub(crate) struct CallbackHook {
    callback_id_count: u32,
    element_position_update_callbacks: Vec<ElementPositionUpdateCallback>,
}

impl CallbackHook {
    pub fn register_callback<F>(&mut self, callback: F) -> u32
    where
        F: FnMut(ID, Vector, FloatNum) + 'static,
    {
        self.callback_id_count += 1;
        let id = self.callback_id_count;
        let callback = ElementPositionUpdateCallback {
            callback: Box::new(callback),
            id,
        };
        self.element_position_update_callbacks.push(callback);
        id
    }

    pub fn unregister_callback(&mut self, callback_id: u32) {
        self.element_position_update_callbacks
            .retain(|callback| callback.id != callback_id)
    }

    pub fn emit(&mut self, id: ID, translate: Vector, rotation: FloatNum) {
        for callback in self.element_position_update_callbacks.iter_mut() {
            callback.call(id, translate, rotation);
        }
    }
}
