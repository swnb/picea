use std::io;

use picea::{
    element::ElementBuilder,
    math::FloatNum,
    meta::MetaBuilder,
    scene::Scene,
    shape::Circle,
    tools::observability::{run_observed_ticks, LabArtifacts},
};

pub const STEP_DT: FloatNum = 1. / 60.;

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
        } => {
            let mut scene = Scene::width_capacity(2);
            scene.set_gravity(|_| (0., 0.).into());
            scene.push_element(ElementBuilder::new(
                Circle::new((0., 0.), 1.),
                MetaBuilder::new().mass(1.).is_fixed(true),
                (),
            ));
            scene.push_element(ElementBuilder::new(
                Circle::new((second_circle_x, 0.), 1.),
                MetaBuilder::new().mass(1.).is_fixed(true),
                (),
            ));

            run_observed_ticks(run_id, &mut scene, std::iter::repeat(STEP_DT).take(steps))
        }
        RunRecipe::Benchmark {
            run_id,
            scenario,
            steps,
        } => {
            let mut scene = match scenario {
                BenchmarkScenario::ContactRefreshTransfer => continuing_contact_scene(),
                BenchmarkScenario::Circles32 => circle_sweep_scene(32),
            };
            run_observed_ticks(run_id, &mut scene, std::iter::repeat(STEP_DT).take(steps))
        }
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

fn continuing_contact_scene() -> Scene<()> {
    let mut scene = Scene::width_capacity(2);
    scene.set_gravity(|_| (0., 0.).into());

    let element_a_id = scene.push_element(ElementBuilder::new(
        Circle::new((0., 0.), 1.),
        MetaBuilder::new().mass(1.),
        (),
    ));
    let element_b_id = scene.push_element(ElementBuilder::new(
        Circle::new((1.5, 0.), 1.),
        MetaBuilder::new().mass(1.),
        (),
    ));

    *scene
        .get_element_mut(element_a_id)
        .expect("element a exists")
        .meta_mut()
        .velocity_mut() = (8., 0.).into();
    *scene
        .get_element_mut(element_b_id)
        .expect("element b exists")
        .meta_mut()
        .velocity_mut() = (-8., 0.).into();

    scene
}

fn circle_sweep_scene(count: usize) -> Scene<()> {
    let mut scene = Scene::width_capacity(count);
    scene.set_gravity(|_| (0., 0.).into());
    for index in 0..count {
        let x = index as FloatNum * 2.4;
        let element_id = scene.push_element(ElementBuilder::new(
            Circle::new((x, 0.), 1.),
            MetaBuilder::new().mass(1.),
            (),
        ));
        let direction = if index % 2 == 0 { 1. } else { -1. };
        *scene
            .get_element_mut(element_id)
            .expect("element exists")
            .meta_mut()
            .velocity_mut() = (direction * 3., 0.).into();
    }
    scene
}
