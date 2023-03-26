extern crate console_error_panic_hook;
extern crate wasm_bindgen;
use crate::{
    element::ElementBuilder,
    math::{edge::Edge, point::Point, FloatNum},
    meta::MetaBuilder,
    scene::Scene,
    shape::{line::Line, polygon::Rect},
};
use js_sys::Function;
use serde::{Deserialize, Serialize};
use std::panic;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn set_panic_console_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub struct WebScene {
    scene: Scene,
}

#[wasm_bindgen]
pub struct WebPicea;

#[wasm_bindgen]
#[derive(Serialize, Deserialize, Clone, Copy)]
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

impl From<Point> for Tuple2 {
    fn from(value: Point) -> Self {
        Tuple2 {
            x: value.x,
            y: value.y,
        }
    }
}

impl Into<Point> for Tuple2 {
    fn into(self) -> Point {
        Point {
            x: self.x,
            y: self.y,
        }
    }
}

// TODO should keep copy of element for web
// each time engine update element, get update vector and angular should be more fast

#[wasm_bindgen(typescript_custom_section)]
const FOREACH_ELEMENT_CALLBACK: &'static str = r#"
interface WebScene {
    for_each_element(callback: (points:{x:number,y:number}[],id :number) => void): void;
}
"#;

#[wasm_bindgen]
impl WebScene {
    pub fn create_rect(
        &mut self,
        x: FloatNum,
        y: FloatNum,
        width: FloatNum,
        height: FloatNum,
    ) -> u32 {
        let rect = Rect::new(x, y, width, height);
        let meta = MetaBuilder::new(1.).force("gravity", (0., 20.));
        let element = ElementBuilder::new(rect, meta);
        self.scene.push_element(element)
    }

    pub fn create_line(&mut self, start_point: Tuple2, end_point: Tuple2) -> u32 {
        let line = Line::new(start_point, end_point);
        let meta = MetaBuilder::new(1.)
            .force("gravity", (0., 10.))
            .is_fixed(true);
        let element = ElementBuilder::new(line, meta);
        self.scene.push_element(element)
    }

    pub fn tick(&mut self, delta_t: f32) {
        self.scene.update_elements_by_duration(delta_t);
    }

    #[wasm_bindgen(skip_typescript, typescript_type = "FOREACH_ELEMENT_CALLBACK")]
    pub fn for_each_element(&self, callback: Function) {
        let this = JsValue::null();

        self.scene.elements_iter().for_each(|element| {
            let id = element.id();
            let result = js_sys::Array::new();
            element.shape().edge_iter().for_each(|edge| match edge {
                Edge::Arc {
                    start_point,
                    support_point,
                    end_point,
                } => {
                    todo!()
                }
                Edge::Circle {
                    center_point,
                    radius,
                } => {
                    todo!()
                }
                Edge::Line {
                    start_point,
                    end_point,
                } => {
                    let point: Tuple2 = (*start_point).into();
                    let value = serde_wasm_bindgen::to_value(&point).unwrap();
                    result.push(&value);
                }
            });

            callback
                .call2(&this, &JsValue::from(result), &JsValue::from(id))
                .unwrap();
        });
    }
}

#[wasm_bindgen]
pub fn create_scene() -> WebScene {
    WebScene {
        scene: Scene::new(),
    }
}
