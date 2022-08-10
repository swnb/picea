extern crate console_error_panic_hook;
extern crate wasm_bindgen;
use crate::{
    element::{Element, ElementShape},
    meta::MetaBuilder,
    scene::Scene,
};
use js_sys::Function;
use std::panic;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn set_panic_console_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub struct WebScene {
    scene: Scene,
}

#[wasm_bindgen]
pub struct WebPicea;

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Tuple2 {
    pub x: f32,
    pub y: f32,
}

#[wasm_bindgen]
impl Tuple2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct RectElementBuilder {
    top_left_point: Tuple2,
    width: f32,
    height: f32,
    weight: f32,
    angular: f32,
    angular_velocity: f32,
    velocity: Tuple2,
    fixed: bool,
    force: Tuple2,
}

#[wasm_bindgen]
impl RectElementBuilder {
    pub fn new(top_left_point: Tuple2, width: f32, height: f32) -> Self {
        Self {
            top_left_point,
            width,
            height,
            weight: 1.,
            angular: 0.,
            angular_velocity: 0.,
            fixed: false,
            velocity: Tuple2::new(0., 0.),
            force: Tuple2::new(0., 0.),
        }
    }

    pub fn weight(&mut self, weight: f32) -> Self {
        self.weight = weight;
        self.clone()
    }

    pub fn angular(&mut self, angular: f32) -> Self {
        self.angular = angular;
        self.clone()
    }

    pub fn angular_velocity(&mut self, angular_velocity: f32) -> Self {
        self.angular_velocity = angular_velocity;
        self.clone()
    }

    pub fn velocity(&mut self, velocity: Tuple2) -> Self {
        self.velocity = velocity;
        self.clone()
    }

    pub fn fixed(&mut self, fixed: bool) -> Self {
        self.fixed = fixed;
        self.clone()
    }

    pub fn force(&mut self, force: Tuple2) -> Self {
        self.force = force;
        self.clone()
    }
}

#[wasm_bindgen]
impl WebScene {
    pub fn push_rect_element(&mut self, params: &RectElementBuilder) {
        let &RectElementBuilder {
            top_left_point: Tuple2 { x, y },
            width,
            height,
            weight,
            angular,
            angular_velocity,
            velocity: Tuple2 { x: vx, y: vy },
            fixed: is_fixed,
            force: Tuple2 { x: fx, y: fy },
        } = params;

        self.scene.push_element(Element::new(
            ElementShape::Rect(((x, y), (width, height)).into()),
            MetaBuilder::new(weight)
                .angular(angular)
                .angular_velocity(angular_velocity)
                .velocity((vx, vy))
                .is_fixed(is_fixed)
                .force("f", (fx, fy)),
        ))
    }

    pub fn tick(&mut self, delta_t: f32) {
        self.scene.update_elements_by_duration(delta_t, |_| {});
    }

    pub fn for_each_element(&self, callback: Function) {
        self.scene.elements_iter().for_each(|element| {
            if let ElementShape::Rect(shape) = element.shape() {
                let this = JsValue::null();
                let result = js_sys::Array::new_with_length(8);
                shape
                    .corner_iter()
                    .map(|&v| Tuple2 { x: v.x(), y: v.y() })
                    .enumerate()
                    .for_each(|(i, p)| {
                        let f = JsValue::from;

                        result.set(2 * i as u32, f(p.x));
                        result.set(2 * i as u32 + 1, f(p.y));
                    });

                callback.call1(&this, result.as_ref()).unwrap();
            }
        });
    }
}

#[wasm_bindgen]
impl WebPicea {
    pub fn create_scene() -> WebScene {
        WebScene {
            scene: Scene::new(),
        }
    }
}
