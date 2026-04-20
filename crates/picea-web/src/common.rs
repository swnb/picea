use std::{cell::UnsafeCell, rc::Rc};

use picea::{prelude::*, scene::Scene};
use picea_macro_tools::wasm_config;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

pub(crate) const MAX_REGULAR_POLYGON_EDGE_COUNT: usize = 1024;

#[wasm_bindgen]
#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct Tuple2 {
    pub x: FloatNum,
    pub y: FloatNum,
}

impl From<&Point> for Tuple2 {
    fn from(value: &Point) -> Self {
        Tuple2 {
            x: value.x(),
            y: value.y(),
        }
    }
}

impl From<Tuple2> for Point {
    fn from(value: Tuple2) -> Self {
        Self::new(value.x, value.y)
    }
}

impl From<&Vector> for Tuple2 {
    fn from(value: &Vector) -> Self {
        Tuple2 {
            x: value.x(),
            y: value.y(),
        }
    }
}

impl From<&Point> for WebPoint {
    fn from(value: &Point) -> Self {
        to_web_point_or_null(value)
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Vector")]
    pub type WebVector;
    #[wasm_bindgen(typescript_type = "Point")]
    pub type WebPoint;
}

impl From<Tuple2> for Vector {
    fn from(value: Tuple2) -> Vector {
        (value.x, value.y).into()
    }
}

impl TryInto<Vector> for WebVector {
    type Error = &'static str;

    fn try_into(self) -> Result<Vector, Self::Error> {
        let value: JsValue = self.into();
        let value: Tuple2 = serde_wasm_bindgen::from_value(value)
            .map_err(|_| "vector should be {x:number,y:number}")?;
        validate_tuple2("vector", &value)?;
        Ok(value.into())
    }
}

impl TryInto<Point> for WebPoint {
    type Error = &'static str;

    fn try_into(self) -> Result<Point, Self::Error> {
        let value: JsValue = self.into();
        let value: Tuple2 = serde_wasm_bindgen::from_value(value)
            .map_err(|_| "point should be {x:number,y:number}")?;
        validate_tuple2("point", &value)?;
        Ok(value.into())
    }
}

pub(crate) fn js_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

pub(crate) fn to_js_value_result<T>(value: &T, label: &str) -> Result<JsValue, JsValue>
where
    T: Serialize + ?Sized,
{
    serde_wasm_bindgen::to_value(value)
        .map_err(|_| js_error(format!("failed to serialize {label}")))
}

pub(crate) fn to_js_value_or_null<T>(value: &T) -> JsValue
where
    T: Serialize + ?Sized,
{
    to_js_value_result(value, "value").unwrap_or_else(|_| JsValue::null())
}

pub(crate) fn to_web_point_result(point: &Point) -> Result<WebPoint, JsValue> {
    let value: Tuple2 = point.into();
    to_js_value_result(&value, "point").map(Into::into)
}

fn to_web_point_or_null(point: &Point) -> WebPoint {
    to_web_point_result(point).unwrap_or_else(|_| JsValue::null().into())
}

pub(crate) fn parse_web_vector(value: WebVector) -> Result<Vector, JsValue> {
    value.try_into().map_err(js_error)
}

pub(crate) fn parse_web_point(value: WebPoint) -> Result<Point, JsValue> {
    value.try_into().map_err(js_error)
}

pub(crate) fn parse_web_meta(value: OptionalWebMeta) -> Result<Meta, JsValue> {
    let meta: Meta = value.try_into().map_err(js_error)?;
    validate_meta(&meta).map_err(js_error)?;
    Ok(meta)
}

pub(crate) fn parse_web_join_constraint_config(
    value: OptionalWebJoinConstraintConfig,
) -> Result<JoinConstraintConfig, JsValue> {
    let config: JoinConstraintConfig = value.try_into().map_err(js_error)?;
    validate_join_constraint_config(&config).map_err(js_error)?;
    Ok(config)
}

pub(crate) fn validate_polygon_vertices(vertices: &[Point]) -> Result<(), &'static str> {
    if vertices.len() < 3 {
        return Err("polygon should contain at least 3 points");
    }

    Ok(())
}

pub(crate) fn validate_rect_args(
    top_left_x: FloatNum,
    top_right_y: FloatNum,
    width: FloatNum,
    height: FloatNum,
) -> Result<(), &'static str> {
    validate_finite_number(top_left_x, "rect.x should be a finite number")?;
    validate_finite_number(top_right_y, "rect.y should be a finite number")?;
    validate_positive_number(width, "rect.width should be greater than 0")?;
    validate_positive_number(height, "rect.height should be greater than 0")?;
    Ok(())
}

pub(crate) fn validate_circle_args(
    center_point_x: FloatNum,
    center_point_y: FloatNum,
    radius: FloatNum,
) -> Result<(), &'static str> {
    validate_finite_number(center_point_x, "circle.x should be a finite number")?;
    validate_finite_number(center_point_y, "circle.y should be a finite number")?;
    validate_positive_number(radius, "circle.radius should be greater than 0")?;
    Ok(())
}

pub(crate) fn validate_regular_polygon_args(
    x: FloatNum,
    y: FloatNum,
    edge_count: f64,
    radius: FloatNum,
) -> Result<usize, &'static str> {
    validate_finite_number(x, "regularPolygon.x should be a finite number")?;
    validate_finite_number(y, "regularPolygon.y should be a finite number")?;

    let edge_count = parse_regular_polygon_edge_count(edge_count)?;
    validate_positive_number(radius, "regularPolygon.radius should be greater than 0")?;
    Ok(edge_count)
}

fn parse_regular_polygon_edge_count(edge_count: f64) -> Result<usize, &'static str> {
    validate_finite_number(
        edge_count,
        "regularPolygon.edgeCount should be a finite number",
    )?;
    if edge_count.fract() != 0. {
        return Err("regularPolygon.edgeCount should be an integer");
    }
    if edge_count < 3. {
        return Err("regularPolygon.edgeCount should be at least 3");
    }
    if edge_count > MAX_REGULAR_POLYGON_EDGE_COUNT as f64 {
        return Err("regularPolygon.edgeCount should be at most 1024");
    }

    Ok(edge_count as usize)
}

fn validate_tuple2(label: &'static str, value: &Tuple2) -> Result<(), &'static str> {
    if value.x.is_finite() && value.y.is_finite() {
        return Ok(());
    }

    match label {
        "vector" => Err("vector x/y should be finite numbers"),
        "point" => Err("point x/y should be finite numbers"),
        _ => Err("tuple x/y should be finite numbers"),
    }
}

fn validate_finite_number<T: Into<f64>>(
    value: T,
    error_message: &'static str,
) -> Result<(), &'static str> {
    let value = value.into();
    if !value.is_finite() {
        return Err(error_message);
    }

    Ok(())
}

fn validate_positive_number(
    value: FloatNum,
    error_message: &'static str,
) -> Result<(), &'static str> {
    validate_finite_number(value, error_message)?;
    if value <= 0. {
        return Err(error_message);
    }

    Ok(())
}

fn validate_optional_float(
    value: Option<&FloatNum>,
    error_message: &'static str,
) -> Result<(), &'static str> {
    if value.is_some_and(|value| !value.is_finite()) {
        return Err(error_message);
    }

    Ok(())
}

fn validate_meta(meta: &Meta) -> Result<(), &'static str> {
    validate_optional_float(meta.mass().as_ref(), "meta.mass should be a finite number")?;
    validate_optional_float(
        meta.factor_friction().as_ref(),
        "meta.factorFriction should be a finite number",
    )?;
    validate_optional_float(
        meta.factor_restitution().as_ref(),
        "meta.factorRestitution should be a finite number",
    )?;

    if let Some(velocity) = meta.velocity() {
        validate_tuple2("vector", velocity)?;
    }

    Ok(())
}

fn validate_join_constraint_config(config: &JoinConstraintConfig) -> Result<(), &'static str> {
    validate_optional_float(
        config.distance().as_ref(),
        "constraint.distance should be a finite number",
    )?;
    validate_optional_float(
        config.damping_ratio().as_ref(),
        "constraint.dampingRatio should be a finite number",
    )?;
    validate_optional_float(
        config.frequency().as_ref(),
        "constraint.frequency should be a finite number",
    )?;

    if let Some(distance) = config.distance() {
        if *distance < 0. {
            return Err("constraint.distance should be greater than or equal to 0");
        }
    }

    Ok(())
}

#[wasm_config(bind = Meta)]
pub(crate) struct Meta {
    #[default = 1.0]
    pub mass: FloatNum,
    #[default = true]
    pub is_fixed: bool,
    pub is_transparent: bool,
    pub velocity: Tuple2,
    #[default = 0.2]
    pub factor_friction: FloatNum,
    #[default = 1.]
    pub factor_restitution: FloatNum,
}

#[derive(Clone)]
#[wasm_config(bind = JoinConstraintConfig)]
pub(crate) struct JoinConstraintConfig {
    #[default = 0.]
    pub distance: FloatNum,
    #[default = 1.]
    pub damping_ratio: FloatNum,
    #[default = 0.5]
    pub frequency: FloatNum,
    #[default = false]
    pub hard: bool,
}

impl JoinConstraintConfig {
    fn assign(&self, config: &mut picea::prelude::JoinConstraintConfig) {
        if let Some(v) = self.hard {
            *config.hard_mut() = v;
        }
        if let Some(v) = self.damping_ratio {
            *config.damping_ratio_mut() = v;
        }
        if let Some(v) = self.distance {
            *config.distance_mut() = v;
        }
        if let Some(v) = self.frequency {
            *config.frequency_mut() = v;
        }
    }
}

#[wasm_bindgen]
pub struct PointConstraint {
    id: u32,
    scene: Rc<UnsafeCell<Scene>>,
    is_dispose: UnsafeCell<bool>,
}

#[wasm_bindgen]
impl PointConstraint {
    pub(crate) fn new(id: ID, scene: Rc<UnsafeCell<Scene>>) -> Self {
        Self {
            id,
            scene,
            is_dispose: UnsafeCell::new(false),
        }
    }

    pub fn config(&self) -> WebJoinConstraintConfig {
        self.scene_mut()
            .get_point_constraint(self.id)
            .map(|constraint| {
                let config: &JoinConstraintConfig = &constraint.config().into();
                config.into()
            })
            .unwrap_or_else(|| JsValue::null().into())
    }

    #[wasm_bindgen(js_name = "updateMovePoint")]
    pub fn update_move_point(&self, point: WebPoint) {
        let _ = self.try_update_move_point(point);
    }

    #[wasm_bindgen(js_name = "tryUpdateMovePoint")]
    pub fn try_update_move_point(&self, point: WebPoint) -> Result<(), JsValue> {
        let point = parse_web_point(point)?;
        let Some(constraint) = self.scene_mut().get_point_constraint_mut(self.id) else {
            return Err(js_error("point constraint not found"));
        };

        *constraint.fixed_point_mut() = point;
        Ok(())
    }

    #[wasm_bindgen(js_name = "getPointPair")]
    pub fn get_point_pair(&self) -> Vec<WebPoint> {
        self.scene_mut()
            .get_point_constraint(self.id)
            .map(|point_constraint| {
                let p1: WebPoint = point_constraint.move_point().into();
                let p2: WebPoint = point_constraint.fixed_point().into();

                vec![p1, p2]
            })
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = "updateConfig")]
    pub fn update_config(&self, config: OptionalWebJoinConstraintConfig) {
        let _ = self.try_update_config(config);
    }

    #[wasm_bindgen(js_name = "tryUpdateConfig")]
    pub fn try_update_config(
        &self,
        config: OptionalWebJoinConstraintConfig,
    ) -> Result<(), JsValue> {
        let config = parse_web_join_constraint_config(config)?;
        let Some(point_constraint) = self.scene_mut().get_point_constraint_mut(self.id) else {
            return Err(js_error("point constraint not found"));
        };

        config.assign(point_constraint.config_mut());
        Ok(())
    }

    pub fn dispose(&self) {
        unsafe {
            if *self.is_dispose.get() {
                return;
            }
            *self.is_dispose.get() = true;
            self.scene_mut().remove_point_constraint(self.id);
        }
    }

    #[allow(clippy::mut_from_ref)]
    fn scene_mut(&self) -> &mut Scene {
        unsafe { &mut *self.scene.get() }
    }
}

#[wasm_bindgen]
pub struct JoinConstraint {
    id: u32,
    scene: Rc<UnsafeCell<Scene>>,
    is_dispose: UnsafeCell<bool>,
}

#[wasm_bindgen]
impl JoinConstraint {
    pub(crate) fn new(id: ID, scene: Rc<UnsafeCell<Scene>>) -> Self {
        Self {
            id,
            scene,
            is_dispose: UnsafeCell::new(false),
        }
    }

    pub fn config(&self) -> WebJoinConstraintConfig {
        self.scene_mut()
            .get_join_constraint(self.id)
            .map(|constraint| {
                let config: &JoinConstraintConfig = &constraint.config().into();
                config.into()
            })
            .unwrap_or_else(|| JsValue::null().into())
    }

    #[wasm_bindgen(js_name = "getPointPair")]
    pub fn get_point_pair(&self) -> Vec<WebPoint> {
        self.scene_mut()
            .get_join_constraint_mut(self.id)
            .map(|join_constraint| {
                let (p1, p2) = join_constraint.move_point_pair();
                let p1: WebPoint = p1.into();
                let p2: WebPoint = p2.into();

                vec![p1, p2]
            })
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = "updateConfig")]
    pub fn update_config(&self, config: OptionalWebJoinConstraintConfig) {
        let _ = self.try_update_config(config);
    }

    #[wasm_bindgen(js_name = "tryUpdateConfig")]
    pub fn try_update_config(
        &self,
        config: OptionalWebJoinConstraintConfig,
    ) -> Result<(), JsValue> {
        let config = parse_web_join_constraint_config(config)?;
        let Some(join_constraint) = self.scene_mut().get_join_constraint_mut(self.id) else {
            return Err(js_error("join constraint not found"));
        };

        config.assign(join_constraint.config_mut());
        Ok(())
    }

    pub fn dispose(&self) {
        unsafe {
            if *self.is_dispose.get() {
                return;
            }
            *self.is_dispose.get() = true;
            self.scene_mut().remove_join_constraint(self.id);
        }
    }

    #[allow(clippy::mut_from_ref)]
    fn scene_mut(&self) -> &mut Scene {
        unsafe { &mut *self.scene.get() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    fn empty_meta() -> Meta {
        Meta {
            mass: None,
            is_fixed: None,
            is_transparent: None,
            velocity: None,
            factor_friction: None,
            factor_restitution: None,
        }
    }

    fn empty_join_config() -> JoinConstraintConfig {
        JoinConstraintConfig {
            distance: None,
            damping_ratio: None,
            frequency: None,
            hard: None,
        }
    }

    #[test]
    fn validation_rejects_invalid_vector_point_and_meta_without_panic() {
        assert!(validate_tuple2("vector", &Tuple2 { x: f32::NAN, y: 0. }).is_err());
        assert!(validate_tuple2(
            "point",
            &Tuple2 {
                x: 0.,
                y: f32::INFINITY
            }
        )
        .is_err());

        let mut meta = empty_meta();
        meta.mass = Some(f32::NAN);
        assert!(validate_meta(&meta).is_err());

        let mut meta = empty_meta();
        meta.velocity = Some(Tuple2 {
            x: 0.,
            y: f32::NEG_INFINITY,
        });
        assert!(validate_meta(&meta).is_err());
    }

    #[test]
    fn validation_rejects_invalid_polygon_and_constraint_config_without_panic() {
        assert!(validate_polygon_vertices(&[(0., 0.).into(), (1., 0.).into()]).is_err());

        let mut config = empty_join_config();
        config.distance = Some(-1.);
        assert!(validate_join_constraint_config(&config).is_err());

        let mut config = empty_join_config();
        config.frequency = Some(f32::NAN);
        assert!(validate_join_constraint_config(&config).is_err());
    }

    #[test]
    fn validation_rejects_invalid_shape_creation_numbers_without_panic() {
        assert!(validate_rect_args(0., 0., 10., 10.).is_ok());
        assert!(validate_rect_args(f32::NAN, 0., 10., 10.).is_err());
        assert!(validate_rect_args(0., 0., 0., 10.).is_err());
        assert!(validate_rect_args(0., 0., 10., -1.).is_err());

        assert!(validate_circle_args(0., 0., 1.).is_ok());
        assert!(validate_circle_args(0., f32::INFINITY, 1.).is_err());
        assert!(validate_circle_args(0., 0., 0.).is_err());

        assert_eq!(validate_regular_polygon_args(0., 0., 3.0_f64, 1.), Ok(3));
        assert_eq!(
            validate_regular_polygon_args(0., 0., 1024.0_f64, 1.),
            Ok(1024)
        );
        assert!(validate_regular_polygon_args(0., 0., f64::NAN, 1.).is_err());
        assert!(validate_regular_polygon_args(0., 0., f64::INFINITY, 1.).is_err());
        assert!(validate_regular_polygon_args(0., 0., -3.0_f64, 1.).is_err());
        assert!(validate_regular_polygon_args(0., 0., 3.00000001_f64, 1.).is_err());
        assert!(validate_regular_polygon_args(0., 0., 1024.00001_f64, 1.).is_err());
        assert!(validate_regular_polygon_args(0., 0., 3.5_f64, 1.).is_err());
        assert!(validate_regular_polygon_args(0., 0., 2.0_f64, 1.).is_err());
        assert!(validate_regular_polygon_args(0., 0., 1025.0_f64, 1.).is_err());
        assert!(validate_regular_polygon_args(0., 0., 3.0_f64, f32::NAN).is_err());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn serialization_helpers_return_error_or_null_without_panic() {
        struct FailingSerialize;

        impl Serialize for FailingSerialize {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(serde::ser::Error::custom("forced serialization failure"))
            }
        }

        assert!(to_js_value_result(&FailingSerialize, "test value").is_err());
        assert!(to_js_value_or_null(&FailingSerialize).is_null());
    }
}
