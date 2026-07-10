use picea::prelude::MaterialPreset;
use serde_json::{json, Value};

use crate::{LabError, LabResult};

use super::{ScenarioParameterDescriptor, ScenarioParameterOption, ScenarioParameterValueType};

pub(super) fn parse_usize_param(
    key: &str,
    value: &Value,
    min: usize,
    max: usize,
) -> LabResult<usize> {
    let parsed = value
        .as_u64()
        .ok_or_else(|| LabError::World(format!("scene_params.{key} must be an unsigned integer")))?
        as usize;
    if parsed < min || parsed > max {
        Err(LabError::World(format!(
            "scene_params.{key} must be between {min} and {max}"
        )))
    } else {
        Ok(parsed)
    }
}

pub(super) fn parse_u16_param(key: &str, value: &Value, min: u16, max: u16) -> LabResult<u16> {
    let parsed = value.as_u64().ok_or_else(|| {
        LabError::World(format!("scene_params.{key} must be an unsigned integer"))
    })?;
    if parsed > u64::from(u16::MAX) {
        return Err(LabError::World(format!(
            "scene_params.{key} must fit in u16"
        )));
    }
    let parsed = parsed as u16;
    if parsed < min || parsed > max {
        Err(LabError::World(format!(
            "scene_params.{key} must be between {min} and {max}"
        )))
    } else {
        Ok(parsed)
    }
}

pub(super) fn parse_f32_param(key: &str, value: &Value, min: f32, max: f32) -> LabResult<f32> {
    let parsed = value
        .as_f64()
        .filter(|value| value.is_finite())
        .map(|value| value as f32)
        .ok_or_else(|| LabError::World(format!("scene_params.{key} must be a finite number")))?;
    if parsed < min || parsed > max {
        Err(LabError::World(format!(
            "scene_params.{key} must be between {min} and {max}"
        )))
    } else {
        Ok(parsed)
    }
}

pub(super) fn parse_material_preset_param(key: &str, value: &Value) -> LabResult<MaterialPreset> {
    serde_json::from_value(value.clone()).map_err(|_| {
        LabError::World(format!(
            "scene_params.{key} must be default, ice, rough, bouncy, or sticky"
        ))
    })
}

pub(super) fn number_param(
    key: &'static str,
    label: &'static str,
    default: f32,
    min: f32,
    max: f32,
    step: f32,
) -> ScenarioParameterDescriptor {
    ScenarioParameterDescriptor {
        key,
        label,
        value_type: ScenarioParameterValueType::Number,
        default: json!(default),
        min: Some(min),
        max: Some(max),
        step: Some(step),
        options: Vec::new(),
    }
}

pub(super) fn integer_param(
    key: &'static str,
    label: &'static str,
    default: usize,
    min: f32,
    max: f32,
) -> ScenarioParameterDescriptor {
    ScenarioParameterDescriptor {
        key,
        label,
        value_type: ScenarioParameterValueType::Integer,
        default: json!(default),
        min: Some(min),
        max: Some(max),
        step: Some(1.0),
        options: Vec::new(),
    }
}

pub(super) fn select_param(
    key: &'static str,
    label: &'static str,
    default: Value,
    options: Vec<ScenarioParameterOption>,
) -> ScenarioParameterDescriptor {
    ScenarioParameterDescriptor {
        key,
        label,
        value_type: ScenarioParameterValueType::Select,
        default,
        min: None,
        max: None,
        step: None,
        options,
    }
}
