use std::{env, error::Error, path::Path};

mod artifact_store;
mod recipes;
mod scene_spec;
mod viewer;

use artifact_store::{make_run_id, ArtifactStore};
use picea::{math::FloatNum, tools::observability::LabArtifacts};
use recipes::{
    capture_benchmark_artifacts_cli, capture_default_contact_artifacts, capture_example_artifacts,
    capture_recipe, ExamplePreset, RunRecipe,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum LabCommand {
    Examples,
    RunExample {
        preset: ExamplePreset,
        steps: usize,
    },
    OpenExample {
        preset: ExamplePreset,
        steps: usize,
    },
    ViewLatest,
    ViewPath(String),
    CaptureContact {
        output_dir: String,
    },
    ReplayContact {
        output_dir: String,
        run_id: String,
        second_circle_x: String,
        steps: String,
    },
    CaptureBenchmark {
        output_dir: String,
        run_id: String,
        scenario: String,
        steps: String,
    },
    CaptureExample {
        output_dir: String,
        run_id: String,
        preset: String,
        steps: String,
    },
    Diff {
        left_dir: String,
        right_dir: String,
    },
    ExportVerification {
        artifact_dir: String,
        output_path: String,
    },
}

fn parse_steps_arg(raw: &str) -> Result<usize, String> {
    let steps = raw
        .parse::<usize>()
        .map_err(|_| "steps must be a positive integer".to_owned())?;
    if steps == 0 {
        return Err("steps must be greater than 0".to_owned());
    }
    Ok(steps)
}

fn usage_text() -> &'static str {
    "usage:\n  picea-lab examples\n  picea-lab run <preset> [steps]\n  picea-lab open <preset> [steps]\n  picea-lab view [latest|artifact-dir]\n  picea-lab capture-contact <output-dir>\n  picea-lab replay-contact <output-dir> <run-id> <second-circle-x> <steps>\n  picea-lab capture-benchmark <output-dir> <run-id> <scenario> <steps>\n  picea-lab capture-example <output-dir> <run-id> <preset> <steps>\n  picea-lab diff <left-dir> <right-dir>\n  picea-lab export-verification <artifact-dir> <output-md>"
}

fn parse_command(args: &[String]) -> Result<LabCommand, String> {
    match args {
        [command] if command == "examples" => Ok(LabCommand::Examples),
        [command] if command == "view" => Ok(LabCommand::ViewLatest),
        [command, preset] if command == "run" => Ok(LabCommand::RunExample {
            preset: ExamplePreset::parse(preset)
                .ok_or_else(|| format!("unknown example preset: {preset}"))?,
            steps: 120,
        }),
        [command, preset, steps] if command == "run" => Ok(LabCommand::RunExample {
            preset: ExamplePreset::parse(preset)
                .ok_or_else(|| format!("unknown example preset: {preset}"))?,
            steps: parse_steps_arg(steps)?,
        }),
        [command, preset] if command == "open" => Ok(LabCommand::OpenExample {
            preset: ExamplePreset::parse(preset)
                .ok_or_else(|| format!("unknown example preset: {preset}"))?,
            steps: 120,
        }),
        [command, preset, steps] if command == "open" => Ok(LabCommand::OpenExample {
            preset: ExamplePreset::parse(preset)
                .ok_or_else(|| format!("unknown example preset: {preset}"))?,
            steps: parse_steps_arg(steps)?,
        }),
        [command, target] if command == "view" && target == "latest" => Ok(LabCommand::ViewLatest),
        [command, artifact_dir] if command == "view" => {
            Ok(LabCommand::ViewPath(artifact_dir.clone()))
        }
        [command, output_dir] if command == "capture-contact" => Ok(LabCommand::CaptureContact {
            output_dir: output_dir.clone(),
        }),
        [command, output_dir, run_id, second_circle_x, steps] if command == "replay-contact" => {
            Ok(LabCommand::ReplayContact {
                output_dir: output_dir.clone(),
                run_id: run_id.clone(),
                second_circle_x: second_circle_x.clone(),
                steps: steps.clone(),
            })
        }
        [command, output_dir, run_id, scenario, steps] if command == "capture-benchmark" => {
            Ok(LabCommand::CaptureBenchmark {
                output_dir: output_dir.clone(),
                run_id: run_id.clone(),
                scenario: scenario.clone(),
                steps: steps.clone(),
            })
        }
        [command, output_dir, run_id, preset, steps] if command == "capture-example" => {
            Ok(LabCommand::CaptureExample {
                output_dir: output_dir.clone(),
                run_id: run_id.clone(),
                preset: preset.clone(),
                steps: steps.clone(),
            })
        }
        [command, left_dir, right_dir] if command == "diff" => Ok(LabCommand::Diff {
            left_dir: left_dir.clone(),
            right_dir: right_dir.clone(),
        }),
        [command, artifact_dir, output_path] if command == "export-verification" => {
            Ok(LabCommand::ExportVerification {
                artifact_dir: artifact_dir.clone(),
                output_path: output_path.clone(),
            })
        }
        _ => Err(usage_text().to_owned()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match parse_command(&args)? {
        LabCommand::Examples => {
            for preset in ExamplePreset::ALL {
                println!("{} ({})", preset.label(), preset.slug());
            }
        }
        LabCommand::RunExample { preset, steps } => {
            let store = ArtifactStore::default();
            let run_id = make_run_id(preset.label());
            let artifacts = capture_example_artifacts(run_id.clone(), preset, steps);
            let dir = store.write_run(&run_id, &artifacts)?;
            println!("wrote Picea Lab example artifacts to {}", dir.display());
        }
        LabCommand::OpenExample { preset, steps } => {
            let store = ArtifactStore::default();
            let run_id = make_run_id(preset.label());
            let artifacts = capture_example_artifacts(run_id.clone(), preset, steps);
            let dir = store.write_run(&run_id, &artifacts)?;
            viewer::run_viewer(&dir)?;
        }
        LabCommand::ViewLatest => {
            let store = ArtifactStore::default();
            let latest = store
                .latest_run_dir()?
                .ok_or_else(|| "no Picea Lab runs found".to_owned())?;
            viewer::run_viewer(&latest)?;
        }
        LabCommand::ViewPath(artifact_dir) => {
            viewer::run_viewer(artifact_dir)?;
        }
        LabCommand::CaptureContact { output_dir } => {
            let artifacts = capture_default_contact_artifacts("contact-smoke");
            artifacts.write_to_dir(&output_dir)?;
            println!("wrote Picea Lab artifacts to {}", output_dir);
        }
        LabCommand::ReplayContact {
            output_dir,
            run_id,
            second_circle_x,
            steps,
        } => {
            let second_circle_x = second_circle_x.parse::<FloatNum>()?;
            let steps = steps.parse::<usize>()?;
            let artifacts = capture_recipe(RunRecipe::ContactReplay {
                run_id,
                second_circle_x,
                steps,
            });
            artifacts.write_to_dir(&output_dir)?;
            println!("wrote Picea Lab replay artifacts to {}", output_dir);
        }
        LabCommand::CaptureBenchmark {
            output_dir,
            run_id,
            scenario,
            steps,
        } => {
            let steps = steps.parse::<usize>()?;
            let artifacts = capture_benchmark_artifacts_cli(run_id, scenario, steps)?;
            artifacts.write_to_dir(&output_dir)?;
            println!("wrote Picea Lab benchmark artifacts to {}", output_dir);
        }
        LabCommand::CaptureExample {
            output_dir,
            run_id,
            preset,
            steps,
        } => {
            let steps = steps.parse::<usize>()?;
            let preset = ExamplePreset::parse(&preset)
                .ok_or_else(|| format!("unknown example preset: {preset}"))?;
            let artifacts = capture_example_artifacts(run_id, preset, steps);
            artifacts.write_to_dir(&output_dir)?;
            println!("wrote Picea Lab example artifacts to {}", output_dir);
        }
        LabCommand::Diff {
            left_dir,
            right_dir,
        } => {
            let left = LabArtifacts::read_from_dir(Path::new(&left_dir))?;
            let right = LabArtifacts::read_from_dir(Path::new(&right_dir))?;
            println!("{}", serde_json::to_string_pretty(&left.diff(&right))?);
        }
        LabCommand::ExportVerification {
            artifact_dir,
            output_path,
        } => {
            viewer::export_verification(artifact_dir, &output_path)?;
            println!("wrote verification summary to {}", output_path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::scene_spec::{ObjectShape, ObjectSpec, SceneTemplate, WorldSpec};

    use picea::tools::observability::{capture_scene_artifacts, DebugEdge};

    use super::capture_benchmark_artifacts_cli;
    use super::ExamplePreset;
    use super::{
        capture_default_contact_artifacts, capture_recipe, parse_command, LabCommand, RunRecipe,
    };

    #[test]
    fn default_contact_capture_has_debug_and_perf_facts() {
        let artifacts = capture_default_contact_artifacts("test-contact");

        assert!(!artifacts.trace_events.is_empty());
        assert_eq!(artifacts.final_snapshot.elements.len(), 2);
        assert!(!artifacts.final_snapshot.contacts.is_empty());
        assert!(!artifacts.debug_render.contacts.is_empty());
        assert!(artifacts
            .perf
            .counters
            .iter()
            .any(|counter| counter.name == "active_manifold_count"));
    }

    #[test]
    fn contact_replay_is_deterministic_for_same_input() {
        let left = capture_recipe(RunRecipe::ContactReplay {
            run_id: "left".to_owned(),
            second_circle_x: 1.5,
            steps: 3,
        });
        let right = capture_recipe(RunRecipe::ContactReplay {
            run_id: "right".to_owned(),
            second_circle_x: 1.5,
            steps: 3,
        });

        assert_eq!(left.final_snapshot.tick, 3);
        assert_eq!(left.state_hash(), right.state_hash());
        assert!(left.diff(&right).same_state);
    }

    #[test]
    fn contact_replay_diff_reports_first_divergent_tick_and_substep() {
        let left = capture_recipe(RunRecipe::ContactReplay {
            run_id: "left".to_owned(),
            second_circle_x: 1.5,
            steps: 3,
        });
        let changed = capture_recipe(RunRecipe::ContactReplay {
            run_id: "changed".to_owned(),
            second_circle_x: 4.0,
            steps: 3,
        });

        let diff = left.diff(&changed);

        assert!(!diff.same_state);
        assert_eq!(diff.first_divergent_tick, Some(1));
        assert_eq!(diff.first_divergent_substep, Some(0));
    }

    #[test]
    fn benchmark_capture_uses_named_scenario_and_exports_perfetto_ready_artifacts() {
        let artifacts = capture_benchmark_artifacts_cli(
            "bench-contact".to_owned(),
            "contact_refresh_transfer".to_owned(),
            2,
        )
        .expect("known benchmark scenario captures");

        assert_eq!(artifacts.final_snapshot.run_id, "bench-contact");
        assert_eq!(artifacts.final_snapshot.tick, 2);
        assert!(artifacts
            .perf
            .counters
            .iter()
            .any(|counter| counter.name == "contact_count"));
        assert!(artifacts
            .to_perfetto_json()
            .expect("perfetto json serializes")
            .contains("traceEvents"));
    }

    #[test]
    fn example_preset_parse_supports_expected_showcase_names() {
        assert_eq!(ExamplePreset::parse("stack"), Some(ExamplePreset::Stack));
        assert_eq!(
            ExamplePreset::parse("newton_cradle"),
            Some(ExamplePreset::NewtonCradle)
        );
        assert_eq!(ExamplePreset::parse("bridge"), Some(ExamplePreset::Bridge));
        assert_eq!(ExamplePreset::parse("cloth"), Some(ExamplePreset::Cloth));
        assert_eq!(ExamplePreset::parse("pit"), Some(ExamplePreset::Pit));
        assert_eq!(ExamplePreset::parse("unknown"), None);
    }

    #[test]
    fn parse_command_supports_scene_first_entrypoints_and_legacy_compatibility() {
        assert_eq!(
            parse_command(&["examples".to_owned()]).expect("examples parses"),
            LabCommand::Examples
        );
        assert_eq!(
            parse_command(&["run".to_owned(), "stack".to_owned()]).expect("run preset parses"),
            LabCommand::RunExample {
                preset: ExamplePreset::Stack,
                steps: 120,
            }
        );
        assert_eq!(
            parse_command(&["open".to_owned(), "cloth".to_owned(), "12".to_owned()])
                .expect("open preset parses"),
            LabCommand::OpenExample {
                preset: ExamplePreset::Cloth,
                steps: 12,
            }
        );
        assert_eq!(
            parse_command(&["view".to_owned()]).expect("view latest parses"),
            LabCommand::ViewLatest
        );
        assert_eq!(
            parse_command(&["view".to_owned(), "latest".to_owned()])
                .expect("view latest explicit parses"),
            LabCommand::ViewLatest
        );
        assert_eq!(
            parse_command(&["view".to_owned(), "/tmp/picea-run".to_owned()])
                .expect("view path parses"),
            LabCommand::ViewPath("/tmp/picea-run".to_owned())
        );
        assert_eq!(
            parse_command(&["capture-contact".to_owned(), "target/run".to_owned()])
                .expect("legacy capture-contact parses"),
            LabCommand::CaptureContact {
                output_dir: "target/run".to_owned(),
            }
        );
    }

    #[test]
    fn example_presets_capture_showcase_scenes() {
        for preset in ExamplePreset::ALL {
            let artifacts = crate::recipes::capture_example_artifacts(
                format!("example-{}", preset.slug()),
                preset,
                2,
            );

            assert!(
                artifacts.final_snapshot.elements.len() >= 2,
                "preset {:?} should create multiple elements",
                preset
            );
            assert!(
                artifacts.debug_render.world_bounds.is_some(),
                "preset {:?} should define world bounds",
                preset
            );
            assert!(
                !artifacts.trace_events.is_empty(),
                "preset {:?} should capture trace events",
                preset
            );
        }
    }

    #[test]
    fn scene_template_builds_circle_and_box_scene() {
        let template = SceneTemplate {
            world: WorldSpec {
                width: 120.0,
                height: 80.0,
                gravity: 0.0,
                editor_clamp: false,
                runtime_boundary: false,
            },
            objects: vec![
                ObjectSpec {
                    id: 1,
                    position: [20.0, 20.0],
                    velocity: [0.0, 0.0],
                    is_fixed: false,
                    shape: ObjectShape::Circle { radius: 8.0 },
                },
                ObjectSpec {
                    id: 2,
                    position: [75.0, 35.0],
                    velocity: [0.0, 0.0],
                    is_fixed: false,
                    shape: ObjectShape::Box {
                        width: 18.0,
                        height: 12.0,
                    },
                },
            ],
        };

        let scene = crate::recipes::build_scene(&template);
        let artifacts = capture_scene_artifacts("scene-template", &scene);

        assert_eq!(artifacts.final_snapshot.elements.len(), 2);
        assert_eq!(artifacts.debug_render.shapes.len(), 2);
        assert!(artifacts.debug_render.shapes.iter().any(|shape| {
            shape.edges.iter().any(|edge| {
                matches!(
                    edge,
                    DebugEdge::Circle { radius, .. } if (*radius - 8.0).abs() < f32::EPSILON
                )
            })
        }));
        assert!(artifacts.debug_render.shapes.iter().any(|shape| {
            shape
                .edges
                .iter()
                .filter(|edge| matches!(edge, DebugEdge::Line { .. }))
                .count()
                >= 4
        }));
    }

    #[test]
    fn runtime_boundary_prevents_object_from_falling_out_of_world() {
        let template = SceneTemplate {
            world: WorldSpec {
                width: 12.0,
                height: 12.0,
                gravity: 24.0,
                editor_clamp: false,
                runtime_boundary: true,
            },
            objects: vec![ObjectSpec {
                id: 1,
                position: [6.0, 2.0],
                velocity: [0.0, 0.0],
                is_fixed: false,
                shape: ObjectShape::Box {
                    width: 2.0,
                    height: 2.0,
                },
            }],
        };

        let mut scene = crate::recipes::build_scene(&template);
        for _ in 0..240 {
            scene.tick(crate::recipes::STEP_DT);
        }
        let artifacts = capture_scene_artifacts("bounded-world", &scene);
        let dynamic_body = artifacts
            .final_snapshot
            .elements
            .iter()
            .find(|element| !element.is_fixed)
            .expect("dynamic body exists");

        assert!(
            dynamic_body.center.y <= 11.8,
            "body escaped world: {:?}",
            dynamic_body
        );
    }

    #[test]
    fn web_viewer_static_assets_and_fixture_are_discoverable() {
        let web_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("web");

        assert!(web_dir.join("index.html").is_file());
        assert!(web_dir.join("app.js").is_file());
        assert!(web_dir.join("styles.css").is_file());
        assert!(web_dir
            .join("fixtures/contact-smoke/final_snapshot.json")
            .is_file());
        assert!(web_dir
            .join("fixtures/contact-smoke/debug_render.json")
            .is_file());
        assert!(web_dir.join("fixtures/contact-smoke/trace.jsonl").is_file());
        assert!(web_dir.join("fixtures/contact-smoke/perf.json").is_file());

        let app_js = std::fs::read_to_string(web_dir.join("app.js")).expect("app.js readable");
        assert!(app_js.contains("visibleShapes()"));
        assert!(app_js.contains("filteredContacts()"));
        assert!(app_js.contains("manifold_labels"));
        assert!(app_js.contains("overlay_text"));
    }
}
