use std::{cell::UnsafeCell, rc::Rc};

use picea::{prelude::*, scene::Scene};
use picea_macro_tools::wasm_config;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

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
        let value: Tuple2 = value.into();
        serde_wasm_bindgen::to_value(&value).unwrap().into()
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
        if let Some(constraint) = self.scene_mut().get_point_constraint_mut(self.id) {
            let point: Point = point.try_into().unwrap();
            *constraint.fixed_point_mut() = point;
        }
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
        if let Some(point_constraint) = self.scene_mut().get_point_constraint_mut(self.id) {
            let config: &JoinConstraintConfig = &config.try_into().unwrap();
            config.assign(point_constraint.config_mut());
        }
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
        if let Some(join_constraint) = self.scene_mut().get_join_constraint_mut(self.id) {
            let config: &JoinConstraintConfig = &config.try_into().unwrap();
            config.assign(join_constraint.config_mut());
        }
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
