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
use std::cell::UnsafeCell;
use std::panic;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::common::{
    js_error, parse_web_join_constraint_config, parse_web_meta, parse_web_point, parse_web_vector,
    to_js_value_or_null, to_js_value_result, to_web_point_result, validate_circle_args,
    validate_polygon_vertices, validate_rect_args, validate_regular_polygon_args, JoinConstraint,
    Meta, OptionalWebJoinConstraintConfig, OptionalWebMeta, PointConstraint, Tuple2, WebMeta,
    WebPoint, WebVector,
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

fn ignore_callback_error(context: &str, result: Result<JsValue, JsValue>) {
    if let Err(error) = result {
        log_callback_error(context, &error);
    }
}

#[cfg(target_arch = "wasm32")]
fn log_callback_error(context: &str, _error: &JsValue) {
    log(&format!("picea-web ignored callback error in {context}"));
}

#[cfg(not(target_arch = "wasm32"))]
fn log_callback_error(_context: &str, _error: &JsValue) {}

#[cfg(target_arch = "wasm32")]
fn log_unsupported_edge(context: &str, id: ID) {
    log(&format!(
        "picea-web skipped unsupported edge while running {context} for element {id}"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
fn log_unsupported_edge(_context: &str, _id: ID) {}

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

enum ElementIterationShape {
    Circle(CircleElementShape),
    Polygon(PolygonElementShape),
    UnsupportedEdge,
}

fn build_for_each_element_shape<'a>(
    id: ID,
    center_point: Point,
    edges: impl IntoIterator<Item = Edge<'a>>,
) -> ElementIterationShape {
    let mut vertices = Vec::new();

    for edge in edges {
        match edge {
            Edge::Arc { .. } => return ElementIterationShape::UnsupportedEdge,
            Edge::Circle {
                center_point,
                radius,
            } => {
                return ElementIterationShape::Circle(CircleElementShape {
                    id,
                    center_point: Tuple2::from(&center_point),
                    shape_type: "circle".into(),
                    radius,
                });
            }
            Edge::Line { start_point, .. } => {
                vertices.push(Tuple2::from(start_point));
            }
        }
    }

    ElementIterationShape::Polygon(PolygonElementShape {
        id,
        shape_type: "polygon".into(),
        center_point: Tuple2::from(&center_point),
        vertices,
    })
}

#[wasm_bindgen(typescript_custom_section)]
const _: &str = include_str!("./type.d.ts");

#[wasm_bindgen]
impl WebScene {
    #[wasm_bindgen(js_name = "setGravity")]
    pub fn set_gravity(&self, gravity: WebVector) {
        let _ = self.try_set_gravity(gravity);
    }

    #[wasm_bindgen(js_name = "trySetGravity")]
    pub fn try_set_gravity(&self, gravity: WebVector) -> Result<(), JsValue> {
        let gravity = parse_web_vector(gravity)?;
        self.get_scene_mut().set_gravity(move |_| gravity);
        Ok(())
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
        if validate_rect_args(top_left_x, top_right_y, width, height).is_err() {
            return 0;
        }

        let shape = Rect::new(top_left_x, top_right_y, width, height);
        self.try_create_element(shape, meta_data)
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = "tryCreateRect")]
    pub fn try_create_rect(
        &self,
        top_left_x: FloatNum,
        top_right_y: FloatNum,
        width: FloatNum,
        height: FloatNum,
        meta_data: Option<OptionalWebMeta>,
    ) -> Result<u32, JsValue> {
        validate_rect_args(top_left_x, top_right_y, width, height).map_err(js_error)?;
        let shape = Rect::new(top_left_x, top_right_y, width, height);
        self.try_create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createCircle")]
    pub fn create_circle(
        &mut self,
        center_point_x: FloatNum,
        center_point_y: FloatNum,
        radius: FloatNum,
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        if validate_circle_args(center_point_x, center_point_y, radius).is_err() {
            return 0;
        }

        let shape = Circle::new((center_point_x, center_point_y), radius);
        self.try_create_element(shape, meta_data)
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = "tryCreateCircle")]
    pub fn try_create_circle(
        &self,
        center_point_x: FloatNum,
        center_point_y: FloatNum,
        radius: FloatNum,
        meta_data: Option<OptionalWebMeta>,
    ) -> Result<u32, JsValue> {
        validate_circle_args(center_point_x, center_point_y, radius).map_err(js_error)?;
        let shape = Circle::new((center_point_x, center_point_y), radius);
        self.try_create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createRegularPolygon")]
    pub fn create_regular_polygon(
        &mut self,
        x: FloatNum,
        y: FloatNum,
        edge_count: f64,
        radius: FloatNum,
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        let Ok(edge_count) = validate_regular_polygon_args(x, y, edge_count, radius) else {
            return 0;
        };

        let shape = RegularPolygon::new((x, y), edge_count, radius);
        self.try_create_element(shape, meta_data)
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = "tryCreateRegularPolygon")]
    pub fn try_create_regular_polygon(
        &self,
        x: FloatNum,
        y: FloatNum,
        edge_count: f64,
        radius: FloatNum,
        meta_data: Option<OptionalWebMeta>,
    ) -> Result<u32, JsValue> {
        let edge_count =
            validate_regular_polygon_args(x, y, edge_count, radius).map_err(js_error)?;
        let shape = RegularPolygon::new((x, y), edge_count, radius);
        self.try_create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createPolygon")]
    pub fn create_polygon(
        &self,
        vertices: Vec<WebPoint>,
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        self.try_create_polygon(vertices, meta_data)
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = "tryCreatePolygon")]
    pub fn try_create_polygon(
        &self,
        vertices: Vec<WebPoint>,
        meta_data: Option<OptionalWebMeta>,
    ) -> Result<u32, JsValue> {
        let vertices = vertices
            .into_iter()
            .map(parse_web_point)
            .collect::<Result<Vec<Point>, JsValue>>()?;
        validate_polygon_vertices(&vertices).map_err(js_error)?;

        let shape = ConcavePolygon::new(vertices);
        self.try_create_element(shape, meta_data)
    }

    #[wasm_bindgen(js_name = "createLine")]
    pub fn create_line(
        &mut self,
        start_point: WebPoint,
        end_point: WebPoint,
        meta_data: Option<OptionalWebMeta>,
    ) -> u32 {
        self.try_create_line(start_point, end_point, meta_data)
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = "tryCreateLine")]
    pub fn try_create_line(
        &self,
        start_point: WebPoint,
        end_point: WebPoint,
        meta_data: Option<OptionalWebMeta>,
    ) -> Result<u32, JsValue> {
        let start_point = parse_web_point(start_point)?;
        let end_point = parse_web_point(end_point)?;
        let shape = Line::new(start_point, end_point);

        self.try_create_element(shape, meta_data)
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
                let meta_data = optional_meta_or_default(meta_data);

                let meta_builder: MetaBuilder = (&meta_data).into();

                let element: ElementBuilder = ElementBuilder::new(shape, meta_builder, ());

                self.get_scene_mut().push_element(element)
            })
    }

    #[wasm_bindgen(js_name = "tryCloneElement")]
    pub fn try_clone_element(
        &self,
        element_id: ID,
        meta_data: Option<OptionalWebMeta>,
    ) -> Result<u32, JsValue> {
        let shape = self
            .get_scene_mut()
            .get_element(element_id)
            .map(|element| element.shape().self_clone())
            .ok_or_else(|| js_error("element not found"))?;

        self.try_create_element(shape, meta_data)
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
        let _ = self.try_update_element_meta_data(element_id, meta_data);
    }

    #[wasm_bindgen(js_name = "tryUpdateElementMeta")]
    pub fn try_update_element_meta_data(
        &self,
        element_id: ID,
        meta_data: OptionalWebMeta,
    ) -> Result<(), JsValue> {
        let meta_data = parse_web_meta(meta_data)?;
        let Some(element) = self.get_scene_mut().get_element_mut(element_id) else {
            return Err(js_error("element not found"));
        };

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

        Ok(())
    }

    #[wasm_bindgen(js_name = "getElementMetaData")]
    pub fn get_element_meta_data(&self, element_id: ID) -> Option<WebMeta> {
        self.get_scene_mut()
            .get_element(element_id)
            .map(|element| element.meta())
            .map(|meta_data| {
                let meta_data: Meta = meta_data.into();
                to_js_value_result(&meta_data, "element metadata")
                    .ok()
                    .map(Into::into)
            })
            .flatten()
    }

    #[wasm_bindgen(skip_typescript, js_name = "registerElementPositionUpdateCallback")]
    pub fn register_element_position_update_callback(&self, callback: Function) -> u32 {
        self.get_scene_mut()
            .register_element_position_update_callback(move |id, translate, rotation| {
                let this = JsValue::null();
                ignore_callback_error(
                    "registerElementPositionUpdateCallback",
                    callback.call3(
                        &this,
                        &JsValue::from(id),
                        &JsValue::from(Tuple2::from(&translate)),
                        &JsValue::from_f64(rotation as f64),
                    ),
                );
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
        let Some(element) = self.get_scene_mut().get_element(element_id) else {
            return Vec::new();
        };

        let mut vertices = Vec::new();
        for edge in element.shape().edge_iter() {
            match edge {
                Edge::Line { start_point, .. } => {
                    let Ok(value) = to_web_point_result(start_point) else {
                        return Vec::new();
                    };
                    vertices.push(value);
                }
                Edge::Arc { .. } | Edge::Circle { .. } => {
                    return Vec::new();
                }
            }
        }

        vertices
    }

    #[wasm_bindgen(js_name = "tryGetElementVertices")]
    pub fn try_get_element_vertices(&self, element_id: ID) -> Result<Vec<WebPoint>, JsValue> {
        self.get_scene_mut()
            .get_element(element_id)
            .ok_or_else(|| js_error("element not found"))
            .and_then(|element| {
                let mut vertices = Vec::new();

                for edge in element.shape().edge_iter() {
                    match edge {
                        Edge::Line { start_point, .. } => {
                            vertices.push(to_web_point_result(start_point)?);
                        }
                        Edge::Arc { .. } => {
                            return Err(js_error("arc vertices are not supported"));
                        }
                        Edge::Circle { .. } => {
                            return Err(js_error("circle vertices are not supported"));
                        }
                    }
                }

                Ok(vertices)
            })
    }

    #[wasm_bindgen(js_name = "getElementCenterPoint")]
    pub fn get_element_center_point(&self, element_id: ID) -> Option<WebPoint> {
        let element = self.get_scene_mut().get_element(element_id);
        element
            .map(|element| element.shape().center_point())
            .and_then(|ref point| to_web_point_result(point).ok())
    }

    #[wasm_bindgen(skip_typescript, js_name = "forEachElement")]
    pub fn for_each_element(&self, callback: Function) {
        let this = JsValue::null();

        self.get_scene_mut().elements_iter().for_each(|element| {
            let id = element.id();
            match build_for_each_element_shape(
                id,
                element.center_point(),
                element.shape().edge_iter(),
            ) {
                ElementIterationShape::Circle(shape) => {
                    let value = JsValue::from(shape);
                    ignore_callback_error("forEachElement", callback.call1(&this, &value));
                }
                ElementIterationShape::Polygon(shape) => {
                    let value = JsValue::from(shape);
                    ignore_callback_error("forEachElement", callback.call1(&this, &value));
                }
                ElementIterationShape::UnsupportedEdge => {
                    log_unsupported_edge("forEachElement", id);
                }
            }
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
        self.try_create_point_constraint(element_id, element_point, fixed_point, constraint_config)
            .ok()
    }

    #[wasm_bindgen(js_name = "tryCreatePointConstraint")]
    pub fn try_create_point_constraint(
        &self,
        element_id: ID,
        element_point: WebPoint,
        fixed_point: WebPoint,
        constraint_config: OptionalWebJoinConstraintConfig,
    ) -> Result<PointConstraint, JsValue> {
        if !self.get_scene_mut().has_element(element_id) {
            return Err(js_error("element not found"));
        }

        let element_point = parse_web_point(element_point)?;
        let fixed_point = parse_web_point(fixed_point)?;
        let constraint_config = parse_web_join_constraint_config(constraint_config)?;
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
            .ok_or_else(|| js_error("point constraint was not created"))
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
        self.try_create_join_constraint(
            element_a_id,
            element_a_point,
            element_b_id,
            element_b_point,
            constraint_config,
        )
        .ok()
    }

    #[wasm_bindgen(js_name = "tryCreateJoinConstraint")]
    pub fn try_create_join_constraint(
        &self,
        element_a_id: ID,
        element_a_point: WebPoint,
        element_b_id: ID,
        element_b_point: WebPoint,
        constraint_config: OptionalWebJoinConstraintConfig,
    ) -> Result<JoinConstraint, JsValue> {
        if element_a_id == element_b_id {
            return Err(js_error("join constraint requires two distinct elements"));
        }
        if !self.get_scene_mut().has_element(element_a_id) {
            return Err(js_error("elementA not found"));
        }
        if !self.get_scene_mut().has_element(element_b_id) {
            return Err(js_error("elementB not found"));
        }

        let element_a_point = parse_web_point(element_a_point)?;
        let element_b_point = parse_web_point(element_b_point)?;
        let constraint_config = parse_web_join_constraint_config(constraint_config)?;
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
            .ok_or_else(|| js_error("join constraint was not created"))
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
        self.try_get_element_kinetic(element_id)
            .unwrap_or_else(|_| JsValue::null())
    }

    #[wasm_bindgen(js_name = "tryGetKinetic")]
    pub fn try_get_element_kinetic(&self, element_id: ID) -> Result<JsValue, JsValue> {
        self.get_scene_mut()
            .get_element(element_id)
            .map(|element| {
                serde_wasm_bindgen::to_value(&element.meta().compute_rough_energy())
                    .map_err(|_| js_error("failed to serialize kinetic value"))
            })
            .ok_or_else(|| js_error("element not found"))?
    }

    #[wasm_bindgen(js_name = "getSleepingStatus")]
    pub fn get_element_is_sleeping(&self, element_id: ID) -> Option<bool> {
        self.get_scene_mut()
            .get_element(element_id)
            .map(|element| element.meta().is_sleeping())
    }

    pub fn get_position_fix_map(&self) -> JsValue {
        let result = self.get_scene_mut().get_position_fix_map();

        to_js_value_or_null(&result)
    }

    #[wasm_bindgen(js_name = "enableSleepMode")]
    pub fn enable_sleep_mode(&self) {
        self.get_scene_mut().set_sleep_mode(true)
    }

    #[wasm_bindgen(js_name = "disableSleepMode")]
    pub fn disable_sleep_mode(&self) {
        self.get_scene_mut().set_sleep_mode(false)
    }

    fn try_create_element(
        &self,
        shape: impl Into<Box<dyn ShapeTraitUnion>>,
        meta_data: Option<OptionalWebMeta>,
    ) -> Result<u32, JsValue> {
        let meta_data = match meta_data {
            Some(meta_data) => parse_web_meta(meta_data)?,
            None => Meta::default(),
        };

        let meta_builder: MetaBuilder = (&meta_data).into();

        let element = ElementBuilder::new(shape, meta_builder, ());

        Ok(self.get_scene_mut().push_element(element))
    }

    #[allow(clippy::mut_from_ref)]
    fn get_scene_mut(&self) -> &mut Scene {
        unsafe { &mut *self.scene.get() }
    }
}

fn optional_meta_or_default(meta_data: Option<OptionalWebMeta>) -> Meta {
    match meta_data {
        Some(meta_data) => parse_web_meta(meta_data).unwrap_or_default(),
        None => Meta::default(),
    }
}

/**
 * NOTE must be sure vertices blew to a valid polygon
 */
#[wasm_bindgen(js_name = "isPointValidAddIntoPolygon")]
pub fn is_point_valid_add_into_polygon(point: WebPoint, vertices: Vec<WebPoint>) -> bool {
    try_is_point_valid_add_into_polygon(point, vertices).unwrap_or(false)
}

#[wasm_bindgen(js_name = "tryIsPointValidAddIntoPolygon")]
pub fn try_is_point_valid_add_into_polygon(
    point: WebPoint,
    vertices: Vec<WebPoint>,
) -> Result<bool, JsValue> {
    let point = parse_web_point(point)?;

    if vertices.len() <= 2 {
        return Ok(true);
    }

    let vertices: Vec<Point> = vertices
        .into_iter()
        .map(parse_web_point)
        .collect::<Result<Vec<Point>, JsValue>>()?;

    let segment1: Segment = (vertices[0], point).into();
    let Some(last_vertex) = vertices.last().copied() else {
        return Ok(true);
    };
    let segment2: Segment = (last_vertex, point).into();

    let vertices_len = vertices.len();
    for i in 0..(vertices_len - 1) {
        let start_point = vertices[i];
        let end_point = vertices[(i + 1) % vertices.len()];
        let segment: Segment = (start_point, end_point).into();

        if i == 0 {
            if check_is_segment_cross(&segment, &segment2) {
                return Ok(false);
            }
        } else if i == vertices_len - 2 {
            if check_is_segment_cross(&segment, &segment1) {
                return Ok(false);
            }
        } else if check_is_segment_cross(&segment, &segment1)
            || check_is_segment_cross(&segment, &segment2)
        {
            return Ok(false);
        }
    }

    Ok(true)
}

#[wasm_bindgen(js_name = "createScene")]
pub fn create_scene() -> WebScene {
    WebScene {
        scene: Rc::new(UnsafeCell::new(Scene::new())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen::JsCast;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    fn invalid_web_value<T: JsCast>() -> T {
        JsValue::from_str("invalid picea-web input").unchecked_into::<T>()
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn try_methods_return_errors_for_invalid_js_values() {
        let scene = create_scene();
        let element_id = scene.create_rect(0., 0., 10., 10., None);
        let other_element_id = scene.create_rect(20., 0., 10., 10., None);

        assert!(scene
            .try_set_gravity(invalid_web_value::<WebVector>())
            .is_err());
        assert!(scene
            .try_create_polygon(vec![invalid_web_value::<WebPoint>()], None)
            .is_err());
        assert!(scene
            .try_create_line(
                invalid_web_value::<WebPoint>(),
                invalid_web_value::<WebPoint>(),
                None
            )
            .is_err());
        assert!(scene
            .try_update_element_meta_data(element_id, invalid_web_value::<OptionalWebMeta>())
            .is_err());
        assert!(scene
            .try_create_point_constraint(
                element_id,
                invalid_web_value::<WebPoint>(),
                invalid_web_value::<WebPoint>(),
                invalid_web_value::<OptionalWebJoinConstraintConfig>(),
            )
            .is_err());
        assert!(scene
            .try_create_join_constraint(
                element_id,
                invalid_web_value::<WebPoint>(),
                other_element_id,
                invalid_web_value::<WebPoint>(),
                invalid_web_value::<OptionalWebJoinConstraintConfig>(),
            )
            .is_err());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn legacy_methods_noop_or_zero_for_invalid_js_values() {
        let mut scene = create_scene();
        let element_id = scene.create_rect(0., 0., 10., 10., None);
        let other_element_id = scene.create_rect(20., 0., 10., 10., None);

        assert!(catch_unwind(AssertUnwindSafe(|| {
            scene.set_gravity(invalid_web_value::<WebVector>());
        }))
        .is_ok());
        assert_eq!(
            catch_unwind(AssertUnwindSafe(|| {
                scene.create_polygon(vec![invalid_web_value::<WebPoint>()], None)
            }))
            .expect("legacy createPolygon should not panic"),
            0
        );
        assert_eq!(
            catch_unwind(AssertUnwindSafe(|| {
                scene.create_line(
                    invalid_web_value::<WebPoint>(),
                    invalid_web_value::<WebPoint>(),
                    None,
                )
            }))
            .expect("legacy createLine should not panic"),
            0
        );
        assert!(catch_unwind(AssertUnwindSafe(|| {
            scene.update_element_meta_data(element_id, invalid_web_value::<OptionalWebMeta>());
        }))
        .is_ok());
        assert!(catch_unwind(AssertUnwindSafe(|| {
            scene.create_point_constraint(
                element_id,
                invalid_web_value::<WebPoint>(),
                invalid_web_value::<WebPoint>(),
                invalid_web_value::<OptionalWebJoinConstraintConfig>(),
            );
        }))
        .is_ok());
        assert!(catch_unwind(AssertUnwindSafe(|| {
            scene.create_join_constraint(
                element_id,
                invalid_web_value::<WebPoint>(),
                other_element_id,
                invalid_web_value::<WebPoint>(),
                invalid_web_value::<OptionalWebJoinConstraintConfig>(),
            );
        }))
        .is_ok());
    }

    #[test]
    fn callback_error_isolated_from_rust_scene() {
        assert!(catch_unwind(AssertUnwindSafe(|| {
            ignore_callback_error("test callback", Err(JsValue::UNDEFINED));
        }))
        .is_ok());
    }

    #[test]
    fn remaining_legacy_creation_wrappers_keep_numeric_fallbacks() {
        let mut scene = create_scene();

        assert_eq!(scene.create_rect(0., 0., 0., 10., None), 0);
        assert_eq!(scene.create_circle(0., 0., 0., None), 0);
        assert_eq!(scene.create_regular_polygon(0., 0., 2., 1., None), 0);
        assert_eq!(scene.clone_element(u32::MAX, None), None);
    }

    #[test]
    fn get_element_vertices_returns_empty_for_unsupported_circle_edges_without_panic() {
        let mut scene = create_scene();
        let circle_id = scene.create_circle(0., 0., 1., None);

        assert!(catch_unwind(AssertUnwindSafe(|| {
            assert!(scene.get_element_vertices(circle_id).is_empty());
        }))
        .is_ok());
    }

    #[test]
    fn for_each_element_shape_builder_skips_unsupported_arc_edges_without_panic() {
        let start_point = Point::new(0., 0.);
        let support_point = Point::new(1., 1.);
        let end_point = Point::new(2., 0.);

        assert!(catch_unwind(AssertUnwindSafe(|| {
            assert!(matches!(
                build_for_each_element_shape(
                    7,
                    Point::new(1., 0.),
                    [Edge::Arc {
                        start_point: &start_point,
                        support_point: &support_point,
                        end_point: &end_point,
                    }]
                ),
                ElementIterationShape::UnsupportedEdge
            ));
        }))
        .is_ok());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn remaining_try_creation_methods_return_errors_for_invalid_input() {
        let scene = create_scene();
        let element_id = scene.create_rect(0., 0., 10., 10., None);

        assert!(scene.try_create_rect(0., 0., 0., 10., None).is_err());
        assert!(scene.try_create_circle(0., 0., 0., None).is_err());
        assert!(scene
            .try_create_regular_polygon(0., 0., 3.0_f64, 1., None)
            .is_ok());
        assert!(scene
            .try_create_regular_polygon(0., 0., 1024.0_f64, 1., None)
            .is_ok());
        assert!(scene
            .try_create_regular_polygon(0., 0., f64::NAN, 1., None)
            .is_err());
        assert!(scene
            .try_create_regular_polygon(0., 0., f64::INFINITY, 1., None)
            .is_err());
        assert!(scene
            .try_create_regular_polygon(0., 0., -3.0_f64, 1., None)
            .is_err());
        assert!(scene
            .try_create_regular_polygon(0., 0., 3.00000001_f64, 1., None)
            .is_err());
        assert!(scene
            .try_create_regular_polygon(0., 0., 1024.00001_f64, 1., None)
            .is_err());
        assert!(scene
            .try_create_regular_polygon(0., 0., 3.5_f64, 1., None)
            .is_err());
        assert!(scene
            .try_create_regular_polygon(0., 0., 2.0_f64, 1., None)
            .is_err());
        assert!(scene
            .try_create_regular_polygon(0., 0., 1025.0_f64, 1., None)
            .is_err());
        assert!(scene.try_clone_element(u32::MAX, None).is_err());
        assert!(scene
            .try_clone_element(element_id, Some(invalid_web_value::<OptionalWebMeta>()))
            .is_err());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn point_validity_parses_invalid_point_before_short_vertex_return() {
        assert!(
            try_is_point_valid_add_into_polygon(invalid_web_value::<WebPoint>(), vec![]).is_err()
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn try_get_element_vertices_errors_for_unsupported_circle_edges() {
        let mut scene = create_scene();
        let circle_id = scene.create_circle(0., 0., 1., None);

        assert!(scene.try_get_element_vertices(circle_id).is_err());
        assert!(scene.get_element_vertices(circle_id).is_empty());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn remaining_legacy_creation_methods_keep_fallback_behavior() {
        let mut scene = create_scene();
        let element_id = scene.create_rect(0., 0., 10., 10., None);

        assert_eq!(scene.create_rect(0., 0., 0., 10., None), 0);
        assert_eq!(scene.create_circle(0., 0., 0., None), 0);
        assert_eq!(scene.create_regular_polygon(0., 0., 2., 1., None), 0);
        assert_eq!(scene.clone_element(u32::MAX, None), None);
        assert!(scene
            .clone_element(element_id, Some(invalid_web_value::<OptionalWebMeta>()))
            .is_some());
    }
}
