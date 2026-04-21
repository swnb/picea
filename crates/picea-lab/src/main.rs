use std::{env, error::Error, path::Path};

mod recipes;
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
            let artifacts = capture_benchmark_artifacts_cli(run_id.clone(), scenario.clone(), steps)?;
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
    use super::{capture_default_contact_artifacts, capture_recipe, RunRecipe};
    use super::{
        capture_benchmark_artifacts_cli,
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
