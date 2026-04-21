use std::io;

use picea::{
    element::ElementBuilder,
    math::FloatNum,
    meta::MetaBuilder,
    scene::Scene,
    shape::{line::Line, Circle, Rect},
    tools::observability::{capture_scene_artifacts, run_observed_ticks, LabArtifacts},
};

use crate::scene_spec::{ObjectShape, ObjectSpec, SceneTemplate, WorldSpec};

pub const STEP_DT: FloatNum = 1. / 60.;
const DEFAULT_BOUNDARY_THICKNESS: FloatNum = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub enum BenchmarkScenario {
    ContactRefreshTransfer,
    Circles32,
}

impl BenchmarkScenario {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "contact_refresh_transfer" => Some(Self::ContactRefreshTransfer),
            "circles_32" => Some(Self::Circles32),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunRecipe {
    ContactReplay {
        run_id: String,
        second_circle_x: FloatNum,
        steps: usize,
    },
    Benchmark {
        run_id: String,
        scenario: BenchmarkScenario,
        steps: usize,
    },
}

pub fn capture_default_contact_artifacts(run_id: &str) -> LabArtifacts {
    capture_recipe(RunRecipe::ContactReplay {
        run_id: run_id.to_owned(),
        second_circle_x: 1.5,
        steps: 1,
    })
}

pub fn capture_benchmark_artifacts(
    run_id: String,
    scenario: BenchmarkScenario,
    steps: usize,
) -> LabArtifacts {
    capture_recipe(RunRecipe::Benchmark {
        run_id,
        scenario,
        steps,
    })
}

pub fn capture_recipe(recipe: RunRecipe) -> LabArtifacts {
    match recipe {
        RunRecipe::ContactReplay {
            run_id,
            second_circle_x,
            steps,
        } => capture_scene_template_artifacts(
            run_id,
            &contact_replay_template(second_circle_x),
            steps,
        ),
        RunRecipe::Benchmark {
            run_id,
            scenario,
            steps,
        } => capture_scene_template_artifacts(run_id, &benchmark_template(scenario), steps),
    }
}

pub fn capture_benchmark_artifacts_cli(
    run_id: String,
    scenario: String,
    steps: usize,
) -> io::Result<LabArtifacts> {
    let Some(scenario) = BenchmarkScenario::parse(&scenario) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown benchmark scenario: {scenario}"),
        ));
    };

    Ok(capture_benchmark_artifacts(run_id, scenario, steps))
}

pub fn build_scene(template: &SceneTemplate) -> Scene<()> {
    let mut scene =
        Scene::width_capacity(template.objects.len() + runtime_boundary_object_count(template));
    scene.set_gravity(|_| (0.0, template.world.gravity).into());

    for object in &template.objects {
        let element_id = scene.push_element(build_object_element(object));
        *scene
            .get_element_mut(element_id)
            .expect("freshly inserted object exists")
            .meta_mut()
            .velocity_mut() = (object.velocity[0], object.velocity[1]).into();
    }

    if template.world.runtime_boundary {
        push_runtime_boundary_walls(&mut scene, &template.world);
    }

    scene
}

pub fn capture_scene_template_artifacts(
    run_id: impl Into<String>,
    template: &SceneTemplate,
    steps: usize,
) -> LabArtifacts {
    let run_id = run_id.into();
    let mut scene = build_scene(template);
    if steps == 0 {
        capture_scene_artifacts(run_id, &scene)
    } else {
        run_observed_ticks(run_id, &mut scene, std::iter::repeat(STEP_DT).take(steps))
    }
}

fn build_object_element(object: &ObjectSpec) -> ElementBuilder<()> {
    let meta = MetaBuilder::new().mass(1.0).is_fixed(object.is_fixed);
    match object.shape {
        ObjectShape::Circle { radius } => ElementBuilder::new(
            Circle::new((object.position[0], object.position[1]), radius),
            meta,
            (),
        ),
        ObjectShape::Box { width, height } => ElementBuilder::new(
            Rect::new(
                object.position[0] - (width * 0.5),
                object.position[1] - (height * 0.5),
                width,
                height,
            ),
            meta,
            (),
        ),
    }
}

fn runtime_boundary_object_count(template: &SceneTemplate) -> usize {
    if template.world.runtime_boundary {
        4
    } else {
        0
    }
}

fn push_runtime_boundary_walls(scene: &mut Scene<()>, world: &WorldSpec) {
    let inset = boundary_inset(world);
    let bounds = [
        Line::new((-inset, -inset), (world.width + inset, -inset)),
        Line::new(
            (-inset, world.height + inset),
            (world.width + inset, world.height + inset),
        ),
        Line::new((-inset, -inset), (-inset, world.height + inset)),
        Line::new(
            (world.width + inset, -inset),
            (world.width + inset, world.height + inset),
        ),
    ];

    for wall in bounds {
        scene.push_element(ElementBuilder::new(
            wall,
            MetaBuilder::new().mass(1.0).is_fixed(true),
            (),
        ));
    }
}

fn boundary_inset(world: &WorldSpec) -> FloatNum {
    (world.width.min(world.height) * 0.05).max(DEFAULT_BOUNDARY_THICKNESS)
}

fn contact_replay_template(second_circle_x: FloatNum) -> SceneTemplate {
    SceneTemplate {
        world: WorldSpec {
            width: 4.0,
            height: 4.0,
            gravity: 0.0,
            editor_clamp: false,
            runtime_boundary: false,
        },
        objects: vec![
            ObjectSpec {
                id: 1,
                position: [0.0, 0.0],
                velocity: [0.0, 0.0],
                is_fixed: true,
                shape: ObjectShape::Circle { radius: 1.0 },
            },
            ObjectSpec {
                id: 2,
                position: [second_circle_x, 0.0],
                velocity: [0.0, 0.0],
                is_fixed: true,
                shape: ObjectShape::Circle { radius: 1.0 },
            },
        ],
    }
}

fn benchmark_template(scenario: BenchmarkScenario) -> SceneTemplate {
    match scenario {
        BenchmarkScenario::ContactRefreshTransfer => SceneTemplate {
            world: WorldSpec {
                width: 8.0,
                height: 4.0,
                gravity: 0.0,
                editor_clamp: false,
                runtime_boundary: false,
            },
            objects: vec![
                ObjectSpec {
                    id: 1,
                    position: [0.0, 0.0],
                    velocity: [8.0, 0.0],
                    is_fixed: false,
                    shape: ObjectShape::Circle { radius: 1.0 },
                },
                ObjectSpec {
                    id: 2,
                    position: [1.5, 0.0],
                    velocity: [-8.0, 0.0],
                    is_fixed: false,
                    shape: ObjectShape::Circle { radius: 1.0 },
                },
            ],
        },
        BenchmarkScenario::Circles32 => SceneTemplate {
            world: WorldSpec {
                width: 80.0,
                height: 4.0,
                gravity: 0.0,
                editor_clamp: false,
                runtime_boundary: false,
            },
            objects: (0..32)
                .map(|index| ObjectSpec {
                    id: (index + 1) as u64,
                    position: [index as FloatNum * 2.4, 0.0],
                    velocity: [if index % 2 == 0 { 3.0 } else { -3.0 }, 0.0],
                    is_fixed: false,
                    shape: ObjectShape::Circle { radius: 1.0 },
                })
                .collect(),
        },
    }
}
