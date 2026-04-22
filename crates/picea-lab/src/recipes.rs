use std::io;

use picea::{
    constraints::JoinConstraintConfigBuilder,
    element::ElementBuilder,
    math::{pi, vector::Vector, FloatNum},
    meta::MetaBuilder,
    scene::Scene,
    shape::{concave::ConcavePolygon, line::Line, Circle, Rect},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExamplePreset {
    Stack,
    NewtonCradle,
    Bridge,
    Cloth,
    Pit,
}

impl ExamplePreset {
    pub const ALL: [Self; 5] = [
        Self::Stack,
        Self::NewtonCradle,
        Self::Bridge,
        Self::Cloth,
        Self::Pit,
    ];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "stack" => Some(Self::Stack),
            "newton_cradle" => Some(Self::NewtonCradle),
            "bridge" => Some(Self::Bridge),
            "cloth" => Some(Self::Cloth),
            "pit" => Some(Self::Pit),
            _ => None,
        }
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Self::Stack => "stack",
            Self::NewtonCradle => "newton_cradle",
            Self::Bridge => "bridge",
            Self::Cloth => "cloth",
            Self::Pit => "pit",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Stack => "Stack",
            Self::NewtonCradle => "Newton Cradle",
            Self::Bridge => "Bridge",
            Self::Cloth => "Cloth",
            Self::Pit => "Pit",
        }
    }
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
    Example {
        run_id: String,
        preset: ExamplePreset,
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
        RunRecipe::Example {
            run_id,
            preset,
            steps,
        } => capture_example_artifacts(run_id, preset, steps),
    }
}

pub fn capture_example_artifacts(
    run_id: impl Into<String>,
    preset: ExamplePreset,
    steps: usize,
) -> LabArtifacts {
    let run_id = run_id.into();
    let mut scene = build_example_scene(preset);
    if steps == 0 {
        capture_scene_artifacts(run_id, &scene)
    } else {
        run_observed_ticks(run_id, &mut scene, std::iter::repeat(STEP_DT).take(steps))
    }
}

pub fn example_template(preset: ExamplePreset) -> Option<SceneTemplate> {
    match preset {
        ExamplePreset::Stack => Some(stack_template()),
        _ => None,
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
    let mut scene = build_scene(template);
    capture_scene_run_artifacts(run_id, &mut scene, steps)
}

pub fn capture_scene_run_artifacts(
    run_id: impl Into<String>,
    scene: &mut Scene<()>,
    steps: usize,
) -> LabArtifacts {
    let run_id = run_id.into();
    if steps == 0 {
        capture_scene_artifacts(run_id, scene)
    } else {
        run_observed_ticks(run_id, scene, std::iter::repeat(STEP_DT).take(steps))
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

pub fn build_example_scene(preset: ExamplePreset) -> Scene<()> {
    match preset {
        ExamplePreset::Stack => build_scene(&stack_template()),
        ExamplePreset::NewtonCradle => newton_cradle_scene(),
        ExamplePreset::Bridge => bridge_scene(),
        ExamplePreset::Cloth => cloth_scene(),
        ExamplePreset::Pit => pit_scene(),
    }
}

fn stack_template() -> SceneTemplate {
    let mut objects = Vec::new();
    let mut id = 1_u64;
    for row in 0..5 {
        for col in 0..(5 - row) {
            objects.push(ObjectSpec {
                id,
                position: [
                    52.0 + col as FloatNum * 8.5 + row as FloatNum * 4.0,
                    16.0 + row as FloatNum * 8.5,
                ],
                velocity: [0.0, 0.0],
                is_fixed: false,
                shape: ObjectShape::Box {
                    width: 6.0,
                    height: 6.0,
                },
            });
            id += 1;
        }
    }

    SceneTemplate {
        world: WorldSpec {
            width: 120.0,
            height: 100.0,
            gravity: 18.0,
            editor_clamp: true,
            runtime_boundary: true,
        },
        objects,
    }
}

fn newton_cradle_scene() -> Scene<()> {
    let mut scene = Scene::width_capacity(6);
    let constraint_parameters = scene.context_mut().constraint_parameters_mut();
    *constraint_parameters.max_allow_permeate_mut() = 0.01;
    *constraint_parameters.factor_restitution_mut() = 1.0;
    scene.set_gravity(|_| (0.0, 30.0).into());

    let start_x = 35.0;
    let start_y = 45.0;
    const SIZE: FloatNum = 6.0;
    const BOX_COUNT: usize = 5;

    let mut ids = Vec::new();
    for index in 0..BOX_COUNT {
        let mut meta = MetaBuilder::new();
        if index == BOX_COUNT - 1 {
            meta = meta.velocity((14.0, 0.0));
        }
        let id = scene.push_element(ElementBuilder::new(
            Circle::new((start_x + index as FloatNum * (SIZE * 2.2), start_y), SIZE),
            meta,
            (),
        ));
        ids.push(id);
    }

    for id in ids {
        let center = scene
            .get_element(id)
            .expect("newton cradle element exists")
            .center_point();
        let anchor = center + Vector::from((0.0, -28.0));
        scene.create_point_constraint(
            id,
            center,
            anchor,
            JoinConstraintConfigBuilder::default()
                .distance(28.0)
                .hard(true),
        );
    }

    scene
}

fn bridge_scene() -> Scene<()> {
    let mut scene = Scene::width_capacity(16);
    scene.set_gravity(|_| (0.0, 20.0).into());

    let bridge_count = 10;
    let start_x = 20.0;
    let start_y = 55.0;
    let bridge_width = 10.0;
    let bridge_height = 4.0;

    let mut elements = Vec::new();
    for index in 0..bridge_count {
        let id = scene.push_element(ElementBuilder::new(
            Rect::new(
                start_x + index as FloatNum * bridge_width,
                start_y,
                bridge_width,
                bridge_height,
            ),
            MetaBuilder::new(),
            (),
        ));
        elements.push(id);
    }

    for index in 0..(elements.len() - 1) {
        scene.create_join_constraint(
            elements[index],
            (start_x + (index + 1) as FloatNum * bridge_width, start_y),
            elements[index + 1],
            (start_x + (index + 1) as FloatNum * bridge_width, start_y),
            JoinConstraintConfigBuilder::new()
                .hard(false)
                .damping_ratio(2.0)
                .frequency(0.9),
        );
    }

    scene.create_point_constraint(
        elements[0],
        (start_x, start_y),
        (start_x - 2.0, start_y),
        JoinConstraintConfigBuilder::new().hard(true),
    );
    scene.create_point_constraint(
        *elements.last().expect("bridge has last element"),
        (start_x + elements.len() as FloatNum * bridge_width, start_y),
        (
            start_x + 2.0 + elements.len() as FloatNum * bridge_width,
            start_y,
        ),
        JoinConstraintConfigBuilder::new().hard(true),
    );

    scene.push_element(ElementBuilder::new(
        Circle::new((70.0, 15.0), 6.0),
        MetaBuilder::new().mass(20.0),
        (),
    ));

    scene
}

fn cloth_scene() -> Scene<()> {
    let mut scene = Scene::width_capacity(220);
    scene.set_gravity(|_| (0.0, 18.0).into());

    let start_x = 18.0;
    let start_y = 12.0;
    const GAP: FloatNum = 4.0;
    const ROWS: usize = 18;
    const COLS: usize = 8;
    let mut ids = [[0_u32; COLS]; ROWS];

    for (row_index, row) in ids.iter_mut().enumerate() {
        for (col_index, slot) in row.iter_mut().enumerate() {
            *slot = scene.push_element(ElementBuilder::new(
                Circle::new(
                    (
                        start_x + row_index as FloatNum * GAP,
                        start_y + col_index as FloatNum * GAP,
                    ),
                    0.45,
                ),
                MetaBuilder::default(),
                (),
            ));
        }
    }

    for row in 0..ROWS {
        let top_id = ids[row][0];
        let center = scene
            .get_element(top_id)
            .expect("cloth top node exists")
            .center_point();
        scene.create_point_constraint(
            top_id,
            center,
            center,
            JoinConstraintConfigBuilder::default()
                .damping_ratio(0.5)
                .frequency(pi())
                .distance(GAP),
        );
    }

    for row in 0..(ROWS - 1) {
        for col in 0..COLS {
            create_join_from_centers(&mut scene, ids[row][col], ids[row + 1][col], GAP);
        }
    }

    for row in 0..ROWS {
        for col in 0..(COLS - 1) {
            create_join_from_centers(&mut scene, ids[row][col], ids[row][col + 1], GAP);
        }
    }

    scene.push_element(ElementBuilder::new(
        Circle::new((65.0, 48.0), 7.0),
        MetaBuilder::new()
            .mass(100.0)
            .velocity((0.0, -20.0))
            .is_ignore_gravity(true),
        (),
    ));
    scene.push_element(ElementBuilder::new(
        Line::new((12.0, 96.0), (108.0, 96.0)),
        MetaBuilder::new().mass(100.0).is_fixed(true),
        (),
    ));

    scene
}

fn pit_scene() -> Scene<()> {
    let mut scene = Scene::width_capacity(40);
    *scene
        .context_mut()
        .constraint_parameters_mut()
        .max_allow_permeate_mut() = 0.01;
    scene.set_gravity(|_| (0.0, 16.0).into());

    scene.push_element(ElementBuilder::new(
        Line::new((10.0, 90.0), (210.0, 90.0)),
        MetaBuilder::new().mass(10.0).is_fixed(true),
        (),
    ));

    let concave_vertices = vec![
        (30.0, 70.0).into(),
        (80.0, 70.0).into(),
        (100.0, 50.0).into(),
        (90.0, 30.0).into(),
        (110.0, 30.0).into(),
        (110.0, 80.0).into(),
        (20.0, 80.0).into(),
        (20.0, 30.0).into(),
        (40.0, 30.0).into(),
    ];
    scene.push_element(ElementBuilder::new(
        ConcavePolygon::new(concave_vertices),
        MetaBuilder::new().mass(10.0),
        (),
    ));

    for row in 0..6 {
        for col in 0..6 {
            scene.push_element(ElementBuilder::new(
                Circle::new(
                    (
                        50.0 + col as FloatNum * 5.0 + 2.0,
                        30.0 + row as FloatNum * 5.0 + 2.0,
                    ),
                    2.0,
                ),
                MetaBuilder::new().mass(10.0),
                (),
            ));
        }
    }

    scene
}

fn create_join_from_centers(scene: &mut Scene<()>, a: u32, b: u32, distance: FloatNum) {
    let center_pair = scene
        .get_element(a)
        .map(|element| element.center_point())
        .zip(scene.get_element(b).map(|element| element.center_point()));
    if let Some((center_a, center_b)) = center_pair {
        scene.create_join_constraint(
            a,
            center_a,
            b,
            center_b,
            JoinConstraintConfigBuilder::default()
                .damping_ratio(0.5)
                .frequency(pi())
                .distance(distance),
        );
    }
}
