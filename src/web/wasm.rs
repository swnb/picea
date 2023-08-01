extern crate console_error_panic_hook;
extern crate wasm_bindgen;
use crate::{
    algo::is_point_inside_shape,
    element::{ElementBuilder, ShapeTraitUnion, ID},
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
    meta::{Meta, MetaBuilder},
    scene::Scene,
    shape::{
        line::Line,
        polygon::{Rect, RegularPolygon},
    },
    tools::snapshot,
};
use js_sys::Function;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use std::panic;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn set_panic_console_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
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

impl From<Vector> for Tuple2 {
    fn from(value: Vector) -> Self {
        Tuple2 {
            x: value.x(),
            y: value.y(),
        }
    }
}

// TODO should keep copy of element for web
// each time engine update element, get update vector and angle should be more fast

#[wasm_bindgen(typescript_custom_section)]
const FOREACH_ELEMENT_CALLBACK: &'static str = r#"
interface WebScene {
    for_each_element(callback: (points:{x:number,y:number}[],id :number) => void): void;
}
"#;

#[wasm_bindgen(typescript_custom_section)]
const ELEMENT_POSITION_UPDATE_CALLBACK: &'static str = r#"
interface WebScene {
    register_element_position_update_callback(callback: (id:number,translate:{x:number,y:number},rotation:number) => void): number;
}
"#;

#[wasm_bindgen]
#[derive(Default, Deserialize, Serialize)]
pub struct MetaData {
    pub mass: Option<FloatNum>,
    pub is_fixed: Option<bool>,
    pub is_transparent: Option<bool>,
    pub angle: Option<FloatNum>,
}

impl From<&Meta> for MetaData {
    fn from(value: &Meta) -> Self {
        Self {
            mass: Some(value.mass()),
            is_fixed: Some(value.is_fixed()),
            is_transparent: Some(value.is_transparent()),
            angle: Some(value.angle()),
        }
    }
}

impl Into<MetaBuilder> for MetaData {
    fn into(self) -> MetaBuilder {
        MetaBuilder::new(self.mass.unwrap_or(1.))
            .is_fixed(self.is_fixed.unwrap_or(false))
            .is_transparent(self.is_transparent.unwrap_or(false))
            .angle(self.angle.unwrap_or(0.))
    }
}

#[wasm_bindgen]
impl WebScene {
    fn create_element(
        &mut self,
        shape: impl Into<Box<dyn ShapeTraitUnion>>,
        meta_data: JsValue,
    ) -> u32 {
        let meta_data: MetaData = from_value(meta_data).unwrap_or(Default::default());

        let meta_builder: MetaBuilder = meta_data.into();

        let meta = meta_builder.force("gravity", (0., 20.));

        let element = ElementBuilder::new(shape, meta);

        self.scene.push_element(element)
    }

    pub fn create_rect(
        &mut self,
        x: FloatNum,
        y: FloatNum,
        width: FloatNum,
        height: FloatNum,
        meta_data: JsValue,
    ) -> u32 {
        let shape = Rect::new(x, y, width, height);

        self.create_element(shape, meta_data)
    }

    pub fn create_regular_polygon(
        &mut self,
        x: FloatNum,
        y: FloatNum,
        edge_count: usize,
        radius: FloatNum,
        meta_data: JsValue,
    ) -> u32 {
        let shape = RegularPolygon::new((x, y), edge_count, radius);

        self.create_element(shape, meta_data)
    }

    pub fn create_line(
        &mut self,
        start_point: Tuple2,
        end_point: Tuple2,
        meta_data: JsValue,
    ) -> u32 {
        let shape = Line::new(start_point, end_point);

        self.create_element(shape, meta_data)
    }

    pub fn tick(&mut self, delta_t: f32) {
        self.scene.update_elements_by_duration(delta_t);
    }

    pub fn clone_element(&mut self, element_id: ID, meta_data: JsValue) -> Option<u32> {
        self.scene
            .get_element(element_id)
            .map(|element| element.shape())
            .map(|shape| shape.self_clone())
            .map(|shape| {
                let meta_data: MetaData = from_value(meta_data).unwrap_or(Default::default());

                let meta_builder: MetaBuilder = meta_data.into();

                let element: ElementBuilder = ElementBuilder::new(shape, meta_builder);

                self.scene.push_element(element)
            })
    }

    pub fn update_element_position(
        &mut self,
        element_id: ID,
        translate: Tuple2,
        rotation: FloatNum,
    ) {
        if let Some(element) = self.scene.get_element_mut(element_id) {
            element.translate(&(translate.x, translate.y).into());
            element.rotate(rotation)
        }
    }

    pub fn update_element_meta_data(&mut self, element_id: ID, meta_data: JsValue) {
        if let Ok(meta_data) = from_value::<MetaData>(meta_data) {
            if let Some(element) = self.scene.get_element_mut(element_id) {
                if let Some(mass) = meta_data.mass {
                    element.meta_mut().set_mass(|_| mass);
                }

                if let Some(is_fixed) = meta_data.is_fixed {
                    element.meta_mut().mark_is_fixed(is_fixed);
                }

                if let Some(is_transparent) = meta_data.is_transparent {
                    element.meta_mut().mark_is_transparent(is_transparent);
                };

                if let Some(angle) = meta_data.angle {
                    element.meta_mut().set_angle(|_| angle);
                }
            }
        }
    }

    pub fn get_element_meta_data(&self, element_id: ID) -> JsValue {
        let meta_data = self
            .scene
            .get_element(element_id)
            .map(|element| element.meta());

        // REVIEW
        if let Some(meta_data) = meta_data {
            let meta_data: MetaData = meta_data.into();
            serde_wasm_bindgen::to_value(&meta_data).unwrap()
        } else {
            JsValue::UNDEFINED
        }
    }

    #[wasm_bindgen(skip_typescript, typescript_type = "ELEMENT_POSITION_UPDATE_CALLBACK")]
    pub fn register_element_position_update_callback(&mut self, callback: Function) -> u32 {
        self.scene
            .register_element_position_update_callback(move |id, translate, rotation| {
                let this = JsValue::null();
                callback
                    .call3(
                        &this,
                        &JsValue::from(id),
                        &JsValue::from(Tuple2::from(translate)),
                        &JsValue::from_f64(rotation as f64),
                    )
                    .unwrap();
            })
    }

    pub fn to_raw_code(&self, element_id: ID) -> String {
        let element = self.scene.get_element(element_id);
        element
            .map(snapshot::create_element_construct_code_snapshot)
            .unwrap_or(String::new())
    }

    pub fn is_point_inside(&self, x: FloatNum, y: FloatNum, element_id: ID) -> bool {
        self.scene
            .get_element(element_id)
            .map(|element| is_point_inside_shape((x, y), &mut element.shape().edge_iter()))
            .unwrap_or(false)
    }

    pub fn unregister_element_position_update_callback(&mut self, callback_id: u32) {
        self.scene
            .unregister_element_position_update_callback(callback_id)
    }

    pub fn element_ids(&self) -> Vec<ID> {
        self.scene.elements_iter().map(|ele| ele.id()).collect()
    }

    pub fn element_vertexes(&self, element_id: ID) -> Vec<JsValue> {
        self.scene
            .get_element(element_id)
            .map(|element| {
                element
                    .shape()
                    .edge_iter()
                    .map(|edge| match edge {
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
                        Edge::Line { start_point, .. } => JsValue::from(Tuple2::from(*start_point)),
                    })
                    .collect::<Vec<JsValue>>()
            })
            .unwrap_or(Default::default())
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
                .call2(&this, &JsValue::from(id), &JsValue::from(result))
                .unwrap();
        });
    }

    pub fn clear(&mut self) {
        self.scene.clear();
    }

    pub fn frame_count(&self) -> u64 {
        self.scene.frame_count() as u64
    }
}

#[wasm_bindgen]
pub fn create_scene() -> WebScene {
    WebScene {
        scene: Scene::new(),
    }
}
