use std::collections::BTreeMap;

use picea::pipeline::ContactPositionCorrectionPolicy;
use picea::prelude::*;
use serde_json::{json, Value};

use crate::{LabError, LabResult};

use super::fixture::{
    default_fixture_density, SceneBodyFixture, SceneFixtureWorld, SceneRecipeFixture,
    SceneShapeFixture,
};
use super::scene_params::{
    integer_param, number_param, parse_f32_param, parse_material_preset_param, parse_u16_param,
    parse_usize_param, select_param,
};
use super::{
    ScenarioId, ScenarioParameterDescriptor, ScenarioParameterOption, ScenarioRuntimeConfig,
    SCENE_RECIPE_SCHEMA_VERSION,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatrixStackLayout {
    /// A deliberately asymmetric matrix that exercises edge ejection and churn.
    StaggeredStress,
    /// A centered matrix with column-aligned support for stable-form behavior locks.
    Aligned,
}

impl MatrixStackLayout {
    const fn as_param_value(self) -> &'static str {
        match self {
            Self::StaggeredStress => "staggered",
            Self::Aligned => "aligned",
        }
    }

    fn from_param_value(key: &str, value: &Value) -> LabResult<Self> {
        match value.as_str() {
            Some("staggered") => Ok(Self::StaggeredStress),
            Some("aligned") => Ok(Self::Aligned),
            _ => Err(LabError::World(format!(
                "scene_params.{key} must be aligned or staggered"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RectStackParams {
    columns: usize,
    rows: usize,
    box_width: f32,
    box_height: f32,
    gap_x: f32,
    gap_y: f32,
    layout: MatrixStackLayout,
    material: MaterialPreset,
    density: f32,
}

pub(super) fn rect_stack_fixture(gravity: [f32; 2], params: RectStackParams) -> SceneRecipeFixture {
    const FLOOR_Y: f32 = 2.62;
    const FLOOR_HEIGHT: f32 = 0.45;

    let columns = params.columns.max(1);
    let rows = params.rows.max(1);
    let spacing_x = params.box_width + params.gap_x;
    let spacing_y = params.box_height + params.gap_y;
    let total_width = params.box_width + (columns.saturating_sub(1) as f32 * spacing_x);
    let left = -0.5 * (columns.saturating_sub(1) as f32 * spacing_x);
    let bottom_center_y = FLOOR_Y - FLOOR_HEIGHT * 0.5 - params.box_height * 0.5 - 0.015;

    let mut bodies = Vec::with_capacity(columns * rows + 1);
    bodies.push(SceneBodyFixture {
        body_type: BodyType::Static,
        pose: [0.0, FLOOR_Y, 0.0],
        linear_velocity: [0.0, 0.0],
        can_sleep: false,
        shape: SceneShapeFixture::Rect {
            width: total_width + 2.0,
            height: FLOOR_HEIGHT,
        },
        material: MaterialPreset::Rough,
        filter: CollisionLayerPreset::StaticGeometry,
        density: default_fixture_density(),
        is_sensor: false,
    });

    for row in 0..rows {
        for column in 0..columns {
            let (centered_row_offset, column_bias, angle) = match params.layout {
                MatrixStackLayout::StaggeredStress => {
                    // Small deterministic offsets keep the stress scene from
                    // being a perfectly symmetric toy case. This makes contact
                    // churn and solver handoff easier to inspect.
                    let row_offset = if row % 2 == 0 { 0.0 } else { spacing_x * 0.5 };
                    let centered_row_offset = if columns > 1 {
                        row_offset - spacing_x * 0.25
                    } else {
                        0.0
                    };
                    let column_bias = match (row + column) % 3 {
                        0 => -0.008,
                        1 => 0.0,
                        _ => 0.008,
                    };
                    let angle = match (row + column) % 4 {
                        0 => -0.012,
                        1 => 0.006,
                        2 => 0.012,
                        _ => -0.006,
                    };
                    (centered_row_offset, column_bias, angle)
                }
                MatrixStackLayout::Aligned => (0.0, 0.0, 0.0),
            };

            bodies.push(SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [
                    left + column as f32 * spacing_x + centered_row_offset + column_bias,
                    bottom_center_y - row as f32 * spacing_y,
                    angle,
                ],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Rect {
                    width: params.box_width,
                    height: params.box_height,
                },
                material: params.material,
                filter: CollisionLayerPreset::DynamicBody,
                density: params.density,
                is_sensor: false,
            });
        }
    }

    SceneRecipeFixture {
        schema_version: SCENE_RECIPE_SCHEMA_VERSION,
        world: SceneFixtureWorld {
            gravity,
            enable_sleep: true,
        },
        bodies,
        joints: Vec::new(),
    }
}

pub(super) fn default_rect_stack_runtime_parts(
    scenario_id: ScenarioId,
) -> (RectStackParams, StepConfig, usize) {
    let params = match scenario_id {
        ScenarioId::Stack4 => RectStackParams {
            columns: 1,
            rows: 4,
            box_width: 0.9,
            box_height: 0.9,
            gap_x: 0.035,
            gap_y: 0.035,
            layout: MatrixStackLayout::Aligned,
            material: MaterialPreset::Default,
            density: default_fixture_density(),
        },
        ScenarioId::MatrixStackAligned => RectStackParams {
            columns: 4,
            rows: 3,
            box_width: 0.42,
            box_height: 0.42,
            gap_x: 0.035,
            gap_y: 0.035,
            layout: MatrixStackLayout::Aligned,
            material: MaterialPreset::Rough,
            density: default_fixture_density(),
        },
        _ => RectStackParams {
            columns: 8,
            rows: 6,
            box_width: 0.42,
            box_height: 0.42,
            gap_x: 0.035,
            gap_y: 0.035,
            layout: MatrixStackLayout::StaggeredStress,
            material: MaterialPreset::Rough,
            density: default_fixture_density(),
        },
    };
    (params, StepConfig::default(), 1)
}

pub(super) fn rect_stack_runtime_config(
    params: RectStackParams,
    step: StepConfig,
    substeps_per_frame: usize,
) -> ScenarioRuntimeConfig {
    let mut scene_params = BTreeMap::new();
    scene_params.insert("columns".to_owned(), json!(params.columns));
    scene_params.insert("rows".to_owned(), json!(params.rows));
    scene_params.insert("box_width".to_owned(), json!(params.box_width));
    scene_params.insert("box_height".to_owned(), json!(params.box_height));
    scene_params.insert("gap_x".to_owned(), json!(params.gap_x));
    scene_params.insert("gap_y".to_owned(), json!(params.gap_y));
    scene_params.insert("layout".to_owned(), json!(params.layout.as_param_value()));
    scene_params.insert("material".to_owned(), json!(params.material));
    scene_params.insert("density".to_owned(), json!(params.density));
    scene_params.insert(
        "velocity_iterations".to_owned(),
        json!(step.velocity_iterations),
    );
    scene_params.insert(
        "position_iterations".to_owned(),
        json!(step.position_iterations),
    );
    scene_params.insert(
        "contact_position_correction".to_owned(),
        json!(step.contact_position_correction),
    );
    scene_params.insert("substeps_per_frame".to_owned(), json!(substeps_per_frame));

    ScenarioRuntimeConfig {
        step,
        substeps_per_frame,
        scene_params,
    }
}

pub(super) fn resolve_rect_stack_runtime_parts(
    scenario_id: ScenarioId,
    overrides: &BTreeMap<String, Value>,
) -> LabResult<(RectStackParams, StepConfig, usize)> {
    let (mut params, mut step, mut substeps_per_frame) =
        default_rect_stack_runtime_parts(scenario_id);
    for (key, value) in overrides {
        match key.as_str() {
            "columns" => params.columns = parse_usize_param(key, value, 1, 16)?,
            "rows" => params.rows = parse_usize_param(key, value, 1, 12)?,
            "box_width" => params.box_width = parse_f32_param(key, value, 0.2, 1.5)?,
            "box_height" => params.box_height = parse_f32_param(key, value, 0.2, 1.5)?,
            "gap_x" => params.gap_x = parse_f32_param(key, value, 0.0, 0.25)?,
            "gap_y" => params.gap_y = parse_f32_param(key, value, 0.0, 0.25)?,
            "layout" => params.layout = MatrixStackLayout::from_param_value(key, value)?,
            "material" => params.material = parse_material_preset_param(key, value)?,
            "density" => params.density = parse_f32_param(key, value, 0.1, 5.0)?,
            "velocity_iterations" => step.velocity_iterations = parse_u16_param(key, value, 0, 80)?,
            "position_iterations" => {
                step.position_iterations = parse_u16_param(key, value, 0, 120)?
            }
            "contact_position_correction" => {
                step.contact_position_correction =
                    serde_json::from_value(value.clone()).map_err(|_| {
                        LabError::World(
                            "scene_params.contact_position_correction must be enabled, conservative, or disabled"
                                .to_owned(),
                        )
                    })?;
            }
            "substeps_per_frame" => substeps_per_frame = parse_usize_param(key, value, 1, 8)?,
            other => {
                return Err(LabError::World(format!(
                    "scene_params.{other}: unknown rectangle stack parameter"
                )));
            }
        }
    }
    validate_rect_stack_params(params)?;
    step.dt = 1.0 / (60.0 * substeps_per_frame as f32);
    Ok((params, step, substeps_per_frame))
}

fn validate_rect_stack_params(params: RectStackParams) -> LabResult<()> {
    if params.box_width + params.gap_x <= 0.0 || params.box_height + params.gap_y <= 0.0 {
        return Err(LabError::World(
            "scene_params box size plus gap must stay positive".to_owned(),
        ));
    }
    if params.columns.saturating_mul(params.rows) > 128 {
        return Err(LabError::World(
            "scene_params.columns * scene_params.rows must be <= 128".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn rect_stack_parameter_schema(
    scenario_id: ScenarioId,
) -> Vec<ScenarioParameterDescriptor> {
    let (params, step, substeps_per_frame) = default_rect_stack_runtime_parts(scenario_id);
    vec![
        integer_param("columns", "Columns", params.columns, 1.0, 16.0),
        integer_param("rows", "Rows", params.rows, 1.0, 12.0),
        number_param("box_width", "Box width", params.box_width, 0.2, 1.5, 0.01),
        number_param(
            "box_height",
            "Box height",
            params.box_height,
            0.2,
            1.5,
            0.01,
        ),
        number_param("gap_x", "Horizontal gap", params.gap_x, 0.0, 0.25, 0.005),
        number_param("gap_y", "Vertical gap", params.gap_y, 0.0, 0.25, 0.005),
        select_param(
            "layout",
            "Layout",
            json!(params.layout.as_param_value()),
            vec![
                ScenarioParameterOption {
                    value: json!("aligned"),
                    label: "Aligned",
                },
                ScenarioParameterOption {
                    value: json!("staggered"),
                    label: "Staggered",
                },
            ],
        ),
        select_param(
            "material",
            "Material",
            json!(params.material),
            vec![
                ScenarioParameterOption {
                    value: json!(MaterialPreset::Default),
                    label: "Default",
                },
                ScenarioParameterOption {
                    value: json!(MaterialPreset::Ice),
                    label: "Ice",
                },
                ScenarioParameterOption {
                    value: json!(MaterialPreset::Rough),
                    label: "Rough",
                },
                ScenarioParameterOption {
                    value: json!(MaterialPreset::Sticky),
                    label: "Sticky",
                },
            ],
        ),
        number_param("density", "Density", params.density, 0.1, 5.0, 0.05),
        integer_param(
            "velocity_iterations",
            "Velocity iterations",
            usize::from(step.velocity_iterations),
            0.0,
            80.0,
        ),
        integer_param(
            "position_iterations",
            "Position iterations",
            usize::from(step.position_iterations),
            0.0,
            120.0,
        ),
        select_param(
            "contact_position_correction",
            "Contact position correction",
            json!(step.contact_position_correction),
            vec![
                ScenarioParameterOption {
                    value: json!(ContactPositionCorrectionPolicy::Enabled),
                    label: "Enabled",
                },
                ScenarioParameterOption {
                    value: json!(ContactPositionCorrectionPolicy::Conservative),
                    label: "Conservative",
                },
                ScenarioParameterOption {
                    value: json!(ContactPositionCorrectionPolicy::Disabled),
                    label: "Disabled",
                },
            ],
        ),
        integer_param(
            "substeps_per_frame",
            "Substeps per frame",
            substeps_per_frame,
            1.0,
            8.0,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::scene_dispatch::build_scenario;
    use crate::scenario::ScenarioOverrides;

    #[test]
    fn matrix_stack_scene_params_affect_grid_shape_and_runtime_config() {
        let mut scene_params = BTreeMap::new();
        scene_params.insert("columns".to_owned(), json!(3));
        scene_params.insert("rows".to_owned(), json!(2));
        scene_params.insert("box_width".to_owned(), json!(0.5));
        scene_params.insert("box_height".to_owned(), json!(0.3));
        scene_params.insert("gap_x".to_owned(), json!(0.02));
        scene_params.insert("gap_y".to_owned(), json!(0.01));
        scene_params.insert("layout".to_owned(), json!("aligned"));
        scene_params.insert("material".to_owned(), json!("sticky"));
        scene_params.insert("density".to_owned(), json!(1.8));
        scene_params.insert("velocity_iterations".to_owned(), json!(6));
        scene_params.insert("position_iterations".to_owned(), json!(9));
        scene_params.insert(
            "contact_position_correction".to_owned(),
            json!("conservative"),
        );
        scene_params.insert("substeps_per_frame".to_owned(), json!(3));

        let builtin = build_scenario(
            ScenarioId::MatrixStack,
            &ScenarioOverrides {
                scene_params,
                ..ScenarioOverrides::default()
            },
        )
        .expect("matrix stack scene params should build a parameterized grid");

        assert_eq!(
            builtin.effective_runtime_config.substeps_per_frame, 3,
            "stack params should own substeps instead of borrowing another scene default"
        );
        assert_eq!(
            builtin
                .effective_runtime_config
                .step
                .contact_position_correction,
            ContactPositionCorrectionPolicy::Conservative
        );
        assert_eq!(builtin.effective_runtime_config.step.velocity_iterations, 6);
        assert_eq!(builtin.effective_runtime_config.step.position_iterations, 9);

        let dynamic_bodies = builtin
            .world
            .bodies()
            .filter(|handle| {
                builtin
                    .world
                    .body(*handle)
                    .expect("body should resolve")
                    .body_type()
                    == BodyType::Dynamic
            })
            .collect::<Vec<_>>();
        assert_eq!(dynamic_bodies.len(), 6);

        let dynamic_colliders = dynamic_bodies
            .iter()
            .flat_map(|body| {
                builtin
                    .world
                    .colliders_for_body(*body)
                    .expect("dynamic body should resolve")
            })
            .filter_map(|collider| builtin.world.collider(collider).ok())
            .collect::<Vec<_>>();
        assert_eq!(dynamic_colliders.len(), 6);
        assert!(
            dynamic_colliders
                .iter()
                .all(|collider| (collider.density() - 1.8).abs() <= f32::EPSILON),
            "density should be authored per dynamic stack box"
        );
        assert!(
            dynamic_colliders
                .iter()
                .all(|collider| collider.material() == Material::preset(MaterialPreset::Sticky)),
            "material preset should be authored per dynamic stack box"
        );
    }

    #[test]
    fn matrix_stack_builtin_exports_default_nxm_dynamic_grid() {
        let builtin = build_scenario(ScenarioId::MatrixStack, &ScenarioOverrides::default())
            .expect("matrix stack scenario should build");

        let dynamic_bodies = builtin
            .world
            .bodies()
            .filter(|handle| {
                builtin
                    .world
                    .body(*handle)
                    .expect("body should resolve")
                    .body_type()
                    == BodyType::Dynamic
            })
            .count();
        assert_eq!(dynamic_bodies, 48);

        let static_bodies = builtin
            .world
            .bodies()
            .filter(|handle| {
                builtin
                    .world
                    .body(*handle)
                    .expect("body should resolve")
                    .body_type()
                    == BodyType::Static
            })
            .count();
        assert_eq!(static_bodies, 1);
    }

    #[test]
    fn aligned_matrix_stack_builtin_exports_small_nxm_behavior_lock_grid() {
        let builtin = build_scenario(
            ScenarioId::MatrixStackAligned,
            &ScenarioOverrides::default(),
        )
        .expect("aligned matrix stack scenario should build");

        let dynamic_bodies = builtin
            .world
            .bodies()
            .filter(|handle| {
                builtin
                    .world
                    .body(*handle)
                    .expect("body should resolve")
                    .body_type()
                    == BodyType::Dynamic
            })
            .count();
        assert_eq!(dynamic_bodies, 12);

        let static_bodies = builtin
            .world
            .bodies()
            .filter(|handle| {
                builtin
                    .world
                    .body(*handle)
                    .expect("body should resolve")
                    .body_type()
                    == BodyType::Static
            })
            .count();
        assert_eq!(static_bodies, 1);
    }
}
