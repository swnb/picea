use std::{env, error::Error, path::Path};

mod recipes;
mod scene_spec;
mod viewer;

use picea::{math::FloatNum, tools::observability::LabArtifacts};
use recipes::{
    capture_benchmark_artifacts_cli, capture_default_contact_artifacts, capture_recipe, RunRecipe,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, output_dir] if command == "capture-contact" => {
            let artifacts = capture_default_contact_artifacts("contact-smoke");
            artifacts.write_to_dir(output_dir)?;
            println!("wrote Picea Lab artifacts to {}", output_dir);
        }
        [command, output_dir, run_id, second_circle_x, steps] if command == "replay-contact" => {
            let second_circle_x = second_circle_x.parse::<FloatNum>()?;
            let steps = steps.parse::<usize>()?;
            let artifacts = capture_recipe(RunRecipe::ContactReplay {
                run_id: run_id.clone(),
                second_circle_x,
                steps,
            });
            artifacts.write_to_dir(output_dir)?;
            println!("wrote Picea Lab replay artifacts to {}", output_dir);
        }
        [command, output_dir, run_id, scenario, steps] if command == "capture-benchmark" => {
            let steps = steps.parse::<usize>()?;
            let artifacts =
                capture_benchmark_artifacts_cli(run_id.clone(), scenario.clone(), steps)?;
            artifacts.write_to_dir(output_dir)?;
            println!("wrote Picea Lab benchmark artifacts to {}", output_dir);
        }
        [command, left_dir, right_dir] if command == "diff" => {
            let left = LabArtifacts::read_from_dir(Path::new(left_dir))?;
            let right = LabArtifacts::read_from_dir(Path::new(right_dir))?;
            println!("{}", serde_json::to_string_pretty(&left.diff(&right))?);
        }
        [command, artifact_dir] if command == "view" => {
            viewer::run_viewer(artifact_dir)?;
        }
        [command, artifact_dir, output_path] if command == "export-verification" => {
            viewer::export_verification(artifact_dir, output_path)?;
            println!("wrote verification summary to {}", output_path);
        }
        _ => {
            eprintln!(
                "usage:\n  picea-lab capture-contact <output-dir>\n  picea-lab replay-contact <output-dir> <run-id> <second-circle-x> <steps>\n  picea-lab capture-benchmark <output-dir> <run-id> <scenario> <steps>\n  picea-lab diff <left-dir> <right-dir>\n  picea-lab view <artifact-dir>\n  picea-lab export-verification <artifact-dir> <output-md>"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::scene_spec::{ObjectShape, ObjectSpec, SceneTemplate, WorldSpec};

    use picea::tools::observability::{capture_scene_artifacts, DebugEdge};

    use super::capture_benchmark_artifacts_cli;
    use super::{capture_default_contact_artifacts, capture_recipe, RunRecipe};

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
