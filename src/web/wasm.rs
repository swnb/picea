extern crate console_error_panic_hook;
extern crate wasm_bindgen;
use crate::{
    algo::is_point_inside_shape,
    element::{ElementBuilder, ShapeTraitUnion, ID},
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
    meta::{Meta, MetaBuilder},
    scene::Scene,
    shape::{
        concave::ConcavePolygon,
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

#[wasm_bindgen(js_name = "setPanicConsoleHook")]
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
struct Tuple2 {
    x: FloatNum,
    y: FloatNum,
}

impl From<Point> for Tuple2 {
    fn from(value: Point) -> Self {
        Tuple2 {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<Tuple2> for Point {
    fn from(value: Tuple2) -> Self {
        Self {
            x: value.x,
            y: value.y,
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

impl From<Tuple2> for Vector {
    fn from(value: Tuple2) -> Vector {
        (value.x, value.y).into()
    }
}

#[derive(Default, Deserialize, Serialize)]
struct MetaDataConfig {
    pub mass: Option<FloatNum>,
    #[serde(rename = "isFixed")]
    pub is_fixed: Option<bool>,
    #[serde(rename = "isTransparent")]
    pub is_transparent: Option<bool>,
    pub angle: Option<FloatNum>,
}

impl From<&Meta> for MetaDataConfig {
    fn from(value: &Meta) -> Self {
        Self {
            mass: Some(value.mass()),
            is_fixed: Some(value.is_fixed()),
            is_transparent: Some(value.is_transparent()),
            angle: Some(value.angle()),
        }
    }
}

impl From<MetaDataConfig> for MetaBuilder {
    fn from(value: MetaDataConfig) -> Self {
        MetaBuilder::new(value.mass.unwrap_or(1.))
            .is_fixed(value.is_fixed.unwrap_or(false))
            .is_transparent(value.is_transparent.unwrap_or(false))
            .angle(value.angle.unwrap_or(0.))
    }
}

#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT_DEFINE: &str = r#"
type Vector = {x:number,y:number};
type Point = {x:number,y:number};
type MetaData = {mass:number,isFixed:boolean,isTransparent:boolean,angle:number};
type MetaDataConfig = Partial<MetaData>;
interface WebScene {
    forEachElement(callback: (points:{x:number,y:number}[],id :number) => void): void;
    registerElementPositionUpdateCallback(callback: (id:number,translate:{x:number,y:number},rotation:number) => void): number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Vector")]
    pub type WebVector;
    #[wasm_bindgen(typescript_type = "Point")]
    pub type WebPoint;

    #[wasm_bindgen(typescript_type = "MetaDataConfig")]
    pub type WebMetaDataConfig;

    #[wasm_bindgen(typescript_type = "MetaData")]
    pub type WebMetaData;
}

impl TryInto<Vector> for WebVector {
    type Error = &'static str;

    fn try_into(self) -> Result<Vector, Self::Error> {
        let value: JsValue = self.into();
        let value: Tuple2 = serde_wasm_bindgen::from_value(value)
            .map_err(|_| "vector should be {x:number,y:number}")?;
        Ok(value.into())
    }
}

impl TryInto<Point> for WebPoint {
    type Error = &'static str;

    fn try_into(self) -> Result<Point, Self::Error> {
        let value: JsValue = self.into();
        let value: Tuple2 = serde_wasm_bindgen::from_value(value)
            .map_err(|_| "point should be {x:number,y:number}")?;
        Ok(value.into())
    }
}

impl TryInto<MetaDataConfig> for WebMetaDataConfig {
    type Error = &'static str;
    fn try_into(self) -> Result<MetaDataConfig, Self::Error> {
        let value: JsValue = self.into();
        let value: MetaDataConfig = from_value(value).map_err(|_| {
            "meta data should be {mass:number,isFixed:boolean,isTransparent:boolean,angle:number}"
        })?;

        Ok(value)
    }
}

#[wasm_bindgen]
impl WebScene {
    #[wasm_bindgen(js_name = "createRect")]
    pub fn create_rect(
        &mut self,
        top_left_x: FloatNum,
        top_right_y: FloatNum,
        width: FloatNum,
        height: FloatNum,
        meta_data: Option<WebMetaDataConfig>,
    ) -> u32 {
        let shape = Rect::new(top_left_x, top_right_y, width, height);

        self.create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createRegularPolygon")]
    pub fn create_regular_polygon(
        &mut self,
        x: FloatNum,
        y: FloatNum,
        edge_count: usize,
        radius: FloatNum,
        meta_data: Option<WebMetaDataConfig>,
    ) -> u32 {
        let shape = RegularPolygon::new((x, y), edge_count, radius);

        self.create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createPolygon")]
    pub fn create_polygon(
        &mut self,
        vertexes: Vec<WebPoint>,
        meta_data: Option<WebMetaDataConfig>,
    ) -> u32 {
        let shape = ConcavePolygon::new(
            vertexes
                .into_iter()
                .map(|v| v.try_into().unwrap())
                .collect::<Vec<Point>>(),
        );

        self.create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createLine")]
    pub fn create_line(
        &mut self,
        start_point: WebPoint,
        end_point: WebPoint,
        meta_data: Option<WebMetaDataConfig>,
    ) -> u32 {
        let start_point: Point = start_point.try_into().unwrap();
        let end_point: Point = end_point.try_into().unwrap();

        let shape = Line::new(start_point, end_point);

        self.create_element(shape, meta_data)
    }

    pub fn tick(&mut self, delta_t: f32) {
        self.scene.update_elements_by_duration(delta_t);
    }

    #[wasm_bindgen(js_name = "cloneElement")]
    pub fn clone_element(
        &mut self,
        element_id: ID,
        meta_data: Option<WebMetaDataConfig>,
    ) -> Option<u32> {
        self.scene
            .get_element(element_id)
            .map(|element| element.shape())
            .map(|shape| shape.self_clone())
            .map(|shape| {
                let meta_data = meta_data.into();
                let meta_data: MetaDataConfig = from_value(meta_data).unwrap_or(Default::default());

                let meta_builder: MetaBuilder = meta_data.into();

                let element: ElementBuilder = ElementBuilder::new(shape, meta_builder);

                self.scene.push_element(element)
            })
    }

    #[wasm_bindgen(js_name = "updateElementPosition")]
    pub fn update_element_position(
        &mut self,
        element_id: ID,
        translate_vector: WebVector,
        rotation: FloatNum,
    ) {
        if let Some(element) = self.scene.get_element_mut(element_id) {
            let translate_vector: Vector = translate_vector.try_into().unwrap();
            element.translate(&translate_vector);
            element.rotate(rotation)
        }
    }

    #[wasm_bindgen(js_name = "scaleElementByMovement")]
    pub fn scale_element_by_movement(&mut self, element_id: ID, from: WebPoint, to: WebPoint) {
        if let Some(element) = self.scene.get_element_mut(element_id) {
            let from: Point = from.try_into().unwrap();
            let to: Point = to.try_into().unwrap();
            element.scale(&from, &to);
        }
    }

    #[wasm_bindgen(js_name = "updateElementMetaData")]
    pub fn update_element_meta_data(&mut self, element_id: ID, meta_data: WebMetaDataConfig) {
        if let Some(element) = self.scene.get_element_mut(element_id) {
            let meta_data: MetaDataConfig = meta_data.try_into().unwrap();

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

    #[wasm_bindgen(js_name = "getElementMetaData")]
    pub fn get_element_meta_data(&self, element_id: ID) -> Option<WebMetaData> {
        self.scene
            .get_element(element_id)
            .map(|element| element.meta())
            .map(|meta_data| {
                let meta_data: MetaDataConfig = meta_data.into();
                serde_wasm_bindgen::to_value(&meta_data).unwrap().into()
            })
    }

    #[wasm_bindgen(skip_typescript, js_name = "registerElementPositionUpdateCallback")]
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

    #[wasm_bindgen(js_name = "unregisterElementPositionUpdateCallback")]
    pub fn unregister_element_position_update_callback(&mut self, callback_id: u32) {
        self.scene
            .unregister_element_position_update_callback(callback_id)
    }

    /**
     * get raw construct rust code of element by element id
     */
    #[wasm_bindgen(js_name = "getElementRawRustCode")]
    pub fn get_element_raw_rust_code(&self, element_id: ID) -> String {
        let element = self.scene.get_element(element_id);
        element
            .map(snapshot::create_element_construct_code_snapshot)
            .unwrap_or(String::new())
    }

    #[wasm_bindgen(js_name = "isPointInsideElement")]
    pub fn is_point_inside_element(&self, x: FloatNum, y: FloatNum, element_id: ID) -> bool {
        self.scene
            .get_element(element_id)
            .map(|element| is_point_inside_shape((x, y), &mut element.shape().edge_iter()))
            .unwrap_or(false)
    }

    #[wasm_bindgen(js_name = "getElementIds")]
    pub fn element_ids(&self) -> Vec<ID> {
        self.scene.elements_iter().map(|ele| ele.id()).collect()
    }

    #[wasm_bindgen(js_name = "getElementVertexes")]
    pub fn get_element_vertexes(&self, element_id: ID) -> Vec<WebPoint> {
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
                        Edge::Line { start_point, .. } => {
                            JsValue::from(Tuple2::from(*start_point)).into()
                        }
                    })
                    .collect::<Vec<WebPoint>>()
            })
            .unwrap_or(Default::default())
    }

    #[wasm_bindgen(js_name = "getElementCenterPoint")]
    pub fn get_element_center_point(&self, element_id: ID) -> Option<WebPoint> {
        let element = self.scene.get_element(element_id);
        element
            .map(|element| element.shape().center_point())
            .map(|point| point.into())
            .map(|point: Tuple2| serde_wasm_bindgen::to_value(&point).unwrap().into())
    }

    #[wasm_bindgen(skip_typescript, js_name = "forEachElement")]
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

    #[wasm_bindgen(getter, js_name = "frameCount")]
    pub fn frame_count(&self) -> u64 {
        self.scene.frame_count() as u64
    }

    fn create_element(
        &mut self,
        shape: impl Into<Box<dyn ShapeTraitUnion>>,
        meta_data: Option<WebMetaDataConfig>,
    ) -> u32 {
        let meta_data: JsValue = meta_data.into();
        let meta_data: MetaDataConfig = from_value(meta_data).unwrap_or(Default::default());

        let meta_builder: MetaBuilder = meta_data.into();

        let element = ElementBuilder::new(shape, meta_builder);

        self.scene.push_element(element)
    }
}

#[wasm_bindgen]
pub fn create_scene() -> WebScene {
    WebScene {
        scene: Scene::new(),
    }
}
