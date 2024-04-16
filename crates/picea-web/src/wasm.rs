use js_sys::Function;

use picea::math::edge::Edge;
use picea::prelude::*;
use picea::{
    scene::Scene,
    shape::{
        circle::Circle,
        concave::ConcavePolygon,
        line::Line,
        polygon::RegularPolygon,
        rect::Rect,
        utils::{check_is_segment_cross, is_point_inside_shape},
    },
    tools::snapshot,
};
use serde_wasm_bindgen::from_value;
use std::cell::UnsafeCell;
use std::panic;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::common::{
    JoinConstraint, JoinConstraintConfig, Meta, OptionalWebJoinConstraintConfig, OptionalWebMeta,
    PointConstraint, Tuple2, WebMeta, WebPoint, WebVector,
};

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
    scene: Rc<UnsafeCell<Scene>>,
}

#[wasm_bindgen]
pub enum ElementShapeEnum {
    Circle,
    Polygon,
}

#[wasm_bindgen]
#[derive(Default)]
pub struct PolygonElementShape {
    id: ID,
    shape_type: String, // always polygon
    center_point: Tuple2,
    vertices: Vec<Tuple2>,
}

#[wasm_bindgen]
impl PolygonElementShape {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> ID {
        self.id
    }

    #[wasm_bindgen(js_name = "shapeType", getter)]
    pub fn shape_type(&self) -> String {
        self.shape_type.to_owned()
    }

    #[wasm_bindgen(js_name = "centerPoint", getter)]
    pub fn center_point(&self) -> JsValue {
        self.center_point.into()
    }

    pub fn vertices(&self) -> Vec<WebPoint> {
        self.vertices
            .iter()
            .map(|v| JsValue::from(*v).into())
            .collect()
    }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct CircleElementShape {
    id: ID,
    shape_type: String, // always circle
    radius: FloatNum,
    center_point: Tuple2,
}

#[wasm_bindgen]
impl CircleElementShape {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> ID {
        self.id
    }

    #[wasm_bindgen(js_name = "shapeType", getter)]
    pub fn shape_type(&self) -> String {
        self.shape_type.to_owned()
    }

    #[wasm_bindgen(getter)]
    pub fn radius(&self) -> FloatNum {
        self.radius
    }

    #[wasm_bindgen(js_name = "centerPoint", getter)]
    pub fn center_point(&self) -> JsValue {
        JsValue::from(self.center_point)
    }
}

#[wasm_bindgen(typescript_custom_section)]
const _: &str = include_str!("./type.d.ts");

#[wasm_bindgen]
impl WebScene {
    #[wasm_bindgen(js_name = "setGravity")]
    pub fn set_gravity(&self, gravity: WebVector) {
        let gravity = gravity.try_into().unwrap();
        self.get_scene_mut().set_gravity(move |_| gravity)
    }

    #[wasm_bindgen(js_name = "createRect")]
    pub fn create_rect(
        &self,
        top_left_x: FloatNum,
        top_right_y: FloatNum,
        width: FloatNum,
        height: FloatNum,
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        let shape = Rect::new(top_left_x, top_right_y, width, height);
        self.create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createCircle")]
    pub fn create_circle(
        &mut self,
        center_point_x: FloatNum,
        center_point_y: FloatNum,
        radius: FloatNum,
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        let shape = Circle::new((center_point_x, center_point_y), radius);

        self.create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createRegularPolygon")]
    pub fn create_regular_polygon(
        &mut self,
        x: FloatNum,
        y: FloatNum,
        edge_count: usize,
        radius: FloatNum,
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        let shape = RegularPolygon::new((x, y), edge_count, radius);

        self.create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createPolygon")]
    pub fn create_polygon(
        &self,
        vertices: Vec<WebPoint>,
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        let shape = ConcavePolygon::new(
            vertices
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
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        let start_point: Point = start_point.try_into().unwrap();
        let end_point: Point = end_point.try_into().unwrap();

        let shape = Line::new(start_point, end_point);

        self.create_element(shape, meta_data)
    }

    pub fn tick(&self, delta_t: f32) {
        self.get_scene_mut().tick(delta_t);
    }

    #[wasm_bindgen(js_name = "cloneElement")]
    pub fn clone_element(&self, element_id: ID, meta_data: Option<OptionalWebMeta>) -> Option<u32> {
        self.get_scene_mut()
            .get_element(element_id)
            .map(|element| element.shape())
            .map(|shape| shape.self_clone())
            .map(|shape| {
                let meta_data = meta_data.into();
                let meta_data: &Meta = &from_value(meta_data).unwrap_or_default();

                let meta_builder: MetaBuilder = meta_data.into();

                let element: ElementBuilder = ElementBuilder::new(shape, meta_builder, ());

                self.get_scene_mut().push_element(element)
            })
    }

    #[wasm_bindgen(js_name = "hasElement")]
    pub fn has_element(&self, element_id: ID) -> bool {
        self.get_scene_mut().has_element(element_id)
    }

    #[wasm_bindgen(js_name = "removeElement")]
    pub fn remove_element(&self, element_id: ID) {
        self.get_scene_mut().remove_element(element_id);
    }

    #[wasm_bindgen(js_name = "updateElementMeta")]
    pub fn update_element_meta_data(&self, element_id: ID, meta_data: OptionalWebMeta) {
        if let Some(element) = self.get_scene_mut().get_element_mut(element_id) {
            let meta_data: Meta = meta_data.try_into().unwrap();

            if let Some(mass) = meta_data.mass() {
                element.meta_mut().set_mass(*mass);
            }

            if let Some(is_fixed) = meta_data.is_fixed() {
                *element.meta_mut().is_fixed_mut() = *is_fixed;
            }

            if let Some(is_transparent) = meta_data.is_transparent() {
                *element.meta_mut().is_transparent_mut() = *is_transparent;
            };

            if let Some(factor_friction) = meta_data.factor_friction() {
                *element.meta_mut().factor_friction_mut() = *factor_friction;
            };

            if let Some(factor_restitution) = meta_data.factor_restitution() {
                *element.meta_mut().factor_restitution_mut() = *factor_restitution;
            };

            // if let Some(angle) = meta_data.angle() {
            //     element.meta_mut().set_angle(|_| *angle);
            // }

            if let Some(velocity) = meta_data.velocity() {
                *element.meta_mut().velocity_mut() = (*velocity).into();
            }
        }
    }

    #[wasm_bindgen(js_name = "getElementMetaData")]
    pub fn get_element_meta_data(&self, element_id: ID) -> Option<WebMeta> {
        self.get_scene_mut()
            .get_element(element_id)
            .map(|element| element.meta())
            .map(|meta_data| {
                let meta_data: Meta = meta_data.into();
                serde_wasm_bindgen::to_value(&meta_data).unwrap().into()
            })
    }

    #[wasm_bindgen(skip_typescript, js_name = "registerElementPositionUpdateCallback")]
    pub fn register_element_position_update_callback(&self, callback: Function) -> u32 {
        self.get_scene_mut()
            .register_element_position_update_callback(move |id, translate, rotation| {
                let this = JsValue::null();
                callback
                    .call3(
                        &this,
                        &JsValue::from(id),
                        &JsValue::from(Tuple2::from(&translate)),
                        &JsValue::from_f64(rotation as f64),
                    )
                    .unwrap();
            })
    }

    #[wasm_bindgen(js_name = "unregisterElementPositionUpdateCallback")]
    pub fn unregister_element_position_update_callback(&self, callback_id: u32) {
        self.get_scene_mut()
            .unregister_element_position_update_callback(callback_id)
    }

    /**
     * get raw construct rust code of element by element id
     */
    #[wasm_bindgen(js_name = "getElementRawRustCode")]
    pub fn get_element_raw_rust_code(&self, element_id: ID) -> String {
        let element = self.get_scene_mut().get_element(element_id);
        element
            .map(snapshot::create_element_construct_code_snapshot)
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = "isPointInsideElement")]
    pub fn is_point_inside_element(&self, x: FloatNum, y: FloatNum, element_id: ID) -> bool {
        self.get_scene_mut()
            .get_element(element_id)
            .map(|element| is_point_inside_shape((x, y), &mut element.shape().edge_iter()))
            .unwrap_or(false)
    }

    #[wasm_bindgen(js_name = "getElementIds")]
    pub fn element_ids(&self) -> Vec<ID> {
        self.get_scene_mut()
            .elements_iter()
            .map(|ele| ele.id())
            .collect()
    }

    #[wasm_bindgen(js_name = "getElementVertices")]
    pub fn get_element_vertices(&self, element_id: ID) -> Vec<WebPoint> {
        self.get_scene_mut()
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
                            JsValue::from(Tuple2::from(start_point)).into()
                        }
                    })
                    .collect::<Vec<WebPoint>>()
            })
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = "getElementCenterPoint")]
    pub fn get_element_center_point(&self, element_id: ID) -> Option<WebPoint> {
        let element = self.get_scene_mut().get_element(element_id);
        element
            .map(|element| element.shape().center_point())
            .map(|ref point| point.into())
            .map(|point: Tuple2| serde_wasm_bindgen::to_value(&point).unwrap().into())
    }

    #[wasm_bindgen(skip_typescript, js_name = "forEachElement")]
    pub fn for_each_element(&self, callback: Function) {
        let this = JsValue::null();

        self.get_scene_mut().elements_iter().for_each(|element| {
            let id = element.id();
            let mut result = Vec::new();

            for edge in element.shape().edge_iter() {
                match edge {
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
                        let circle_element_shape = CircleElementShape {
                            id,
                            center_point: Tuple2 {
                                x: center_point.x(),
                                y: center_point.y(),
                            },
                            shape_type: "circle".into(),
                            radius,
                        };

                        let value = &JsValue::from(circle_element_shape);

                        callback.call1(&this, value).unwrap();
                        return;
                    }
                    Edge::Line { start_point, .. } => {
                        let point: Tuple2 = (start_point).into();
                        result.push(point);
                    }
                }
            }

            let element_shape = JsValue::from(PolygonElementShape {
                id,
                shape_type: "polygon".into(),
                center_point: (&element.center_point()).into(),
                vertices: result,
            });

            callback.call1(&this, &element_shape).unwrap();
        });
    }

    pub fn clear(&self) {
        self.get_scene_mut().clear();
    }

    #[wasm_bindgen(getter, js_name = "frameCount")]
    pub fn frame_count(&self) -> u64 {
        self.get_scene_mut().frame_count() as u64
    }

    #[wasm_bindgen(js_name = "isElementCollide")]
    pub fn is_element_collide(
        &self,
        element_a_id: ID,
        element_b_id: ID,
        query_from_manifold: Option<bool>,
    ) -> bool {
        self.get_scene_mut().is_element_collide(
            element_a_id,
            element_b_id,
            query_from_manifold.unwrap_or(true),
        )
    }

    #[wasm_bindgen(js_name = "createPointConstraint")]
    pub fn create_point_constraint(
        &self,
        element_id: ID,
        element_point: WebPoint,
        fixed_point: WebPoint,
        constraint_config: OptionalWebJoinConstraintConfig,
    ) -> Option<PointConstraint> {
        let element_point: Point = element_point.try_into().unwrap();
        let fixed_point: Point = fixed_point.try_into().unwrap();

        let constraint_config: JoinConstraintConfig = constraint_config.try_into().unwrap();
        let constraint_config_builder: JoinConstraintConfigBuilder = (&constraint_config).into();

        let constraint_config: picea::prelude::JoinConstraintConfig =
            constraint_config_builder.into();

        self.get_scene_mut()
            .create_point_constraint(
                element_id,
                element_point,
                fixed_point,
                constraint_config.clone(),
            )
            .map(move |id| PointConstraint::new(id, self.scene.clone()))
    }

    #[wasm_bindgen(js_name = "pointConstraints")]
    pub fn point_constraints(&self) -> Vec<PointConstraint> {
        self.get_scene_mut()
            .point_constraints()
            .map(|constraint| PointConstraint::new(constraint.id(), self.scene.clone()))
            .collect()
    }

    #[wasm_bindgen(js_name = "createJoinConstraint")]
    pub fn create_join_constraint(
        &self,
        element_a_id: ID,
        element_a_point: WebPoint,
        element_b_id: ID,
        element_b_point: WebPoint,
        constraint_config: OptionalWebJoinConstraintConfig,
    ) -> Option<JoinConstraint> {
        let element_a_point: Point = element_a_point.try_into().unwrap();
        let element_b_point: Point = element_b_point.try_into().unwrap();

        let constraint_config: JoinConstraintConfig = constraint_config.try_into().unwrap();
        let constraint_config_builder: JoinConstraintConfigBuilder = (&constraint_config).into();

        let constraint_config: picea::prelude::JoinConstraintConfig =
            constraint_config_builder.into();

        self.get_scene_mut()
            .create_join_constraint(
                element_a_id,
                element_a_point,
                element_b_id,
                element_b_point,
                constraint_config.clone(),
            )
            .map(move |id| JoinConstraint::new(id, self.scene.clone()))
    }

    #[wasm_bindgen(js_name = "joinConstraints")]
    pub fn join_constraints(&self) -> Vec<JoinConstraint> {
        self.get_scene_mut()
            .join_constraints()
            .map(|constraint| JoinConstraint::new(constraint.id(), self.scene.clone()))
            .collect()
    }

    #[wasm_bindgen(js_name = "getKinetic")]
    pub fn get_element_kinetic(&self, element_id: ID) -> JsValue {
        self.get_scene_mut()
            .get_element(element_id)
            .map(|element| {
                serde_wasm_bindgen::to_value(&element.meta().compute_rough_energy()).unwrap()
            })
            .unwrap()
    }

    #[wasm_bindgen(js_name = "getSleepingStatus")]
    pub fn get_element_is_sleeping(&self, element_id: ID) -> Option<bool> {
        self.get_scene_mut()
            .get_element(element_id)
            .map(|element| element.meta().is_sleeping())
    }

    pub fn get_position_fix_map(&self) -> JsValue {
        let result = self.get_scene_mut().get_position_fix_map();

        serde_wasm_bindgen::to_value(&result).unwrap()
    }

    #[wasm_bindgen(js_name = "enableSleepMode")]
    pub fn enable_sleep_mode(&self) {
        self.get_scene_mut().set_sleep_mode(true)
    }

    #[wasm_bindgen(js_name = "disableSleepMode")]
    pub fn disable_sleep_mode(&self) {
        self.get_scene_mut().set_sleep_mode(false)
    }

    fn create_element(
        &self,
        shape: impl Into<Box<dyn ShapeTraitUnion>>,
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        let meta_data: JsValue = meta_data.into();
        let meta_data: &Meta = &from_value(meta_data).unwrap_or_default();

        let meta_builder: MetaBuilder = meta_data.into();

        let element = ElementBuilder::new(shape, meta_builder, ());

        self.get_scene_mut().push_element(element)
    }

    #[allow(clippy::mut_from_ref)]
    fn get_scene_mut(&self) -> &mut Scene {
        unsafe { &mut *self.scene.get() }
    }
}

/**
 * NOTE must be sure vertices blew to a valid polygon
 */
#[wasm_bindgen(js_name = "isPointValidAddIntoPolygon")]
pub fn is_point_valid_add_into_polygon(point: WebPoint, vertices: Vec<WebPoint>) -> bool {
    if vertices.len() <= 2 {
        return true;
    }

    let point: Tuple2 = serde_wasm_bindgen::from_value(point.into()).unwrap();
    let point: Point = point.into();

    let vertices: Vec<Point> = vertices
        .into_iter()
        .map(|v| serde_wasm_bindgen::from_value::<Tuple2>(v.into()).unwrap())
        .map(|v| v.into())
        .collect();

    let segment1: Segment = (vertices[0], point).into();
    let segment2: Segment = (*(vertices.last().unwrap()), point).into();

    let vertices_len = vertices.len();
    for i in 0..(vertices_len - 1) {
        let start_point = vertices[i];
        let end_point = vertices[(i + 1) % vertices.len()];
        let segment: Segment = (start_point, end_point).into();

        if i == 0 {
            if check_is_segment_cross(&segment, &segment2) {
                return false;
            }
        } else if i == vertices_len - 2 {
            if check_is_segment_cross(&segment, &segment1) {
                return false;
            }
        } else if check_is_segment_cross(&segment, &segment1)
            || check_is_segment_cross(&segment, &segment2)
        {
            return false;
        }
    }

    true
}

#[wasm_bindgen(js_name = "createScene")]
pub fn create_scene() -> WebScene {
    WebScene {
        scene: Rc::new(UnsafeCell::new(Scene::new())),
    }
}
