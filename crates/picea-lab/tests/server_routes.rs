use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use picea_lab::{
    server::{app, LabServerState},
    ArtifactStore, ScenarioId, SessionStatus,
};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn server_exposes_scenarios_sessions_artifacts_and_sse_events() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let scenarios = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/scenarios")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(scenarios.status(), StatusCode::OK);
    let scenarios_body = json_body(scenarios).await;
    assert_eq!(
        scenarios_body["scenarios"]
            .as_array()
            .expect("scenarios should be an array")
            .iter()
            .map(|scenario| scenario["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "falling_box_contact",
            "stack_4",
            "stack_stability_tower",
            "matrix_stack",
            "matrix_stack_aligned",
            "newton_cradle",
            "joint_anchor",
            "lattice_grid",
            "broadphase_sparse",
            "sat_polygon",
            "compound_provenance",
            "concave_decomposition",
            "ccd_fast_circle_wall",
            "ccd_fast_convex_walls",
            "ccd_dynamic_convex_pair",
            "ccd_dynamic_compound_wall",
        ]
    );
    let newton_descriptor = scenarios_body["scenarios"]
        .as_array()
        .expect("scenarios should be an array")
        .iter()
        .find(|scenario| scenario["id"] == "newton_cradle")
        .expect("newton cradle descriptor should be exposed");
    assert_eq!(
        newton_descriptor["default_runtime_config"]["step"]["contact_position_correction"],
        "conservative"
    );
    assert_eq!(
        newton_descriptor["default_runtime_config"]["step"]["joint_velocity_projection"],
        true
    );
    assert_eq!(
        newton_descriptor["default_runtime_config"]["substeps_per_frame"],
        16
    );
    let parameter_keys = newton_descriptor["parameter_schema"]
        .as_array()
        .expect("parameter schema should be an array")
        .iter()
        .map(|parameter| parameter["key"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        parameter_keys,
        vec![
            "ball_count",
            "radius",
            "string_length",
            "release_offset",
            "restitution",
            "friction",
            "velocity_iterations",
            "position_iterations",
            "contact_position_correction",
            "joint_velocity_projection",
            "substeps_per_frame",
        ]
    );
    let matrix_descriptor = scenarios_body["scenarios"]
        .as_array()
        .expect("scenarios should be an array")
        .iter()
        .find(|scenario| scenario["id"] == "matrix_stack")
        .expect("matrix stack descriptor should be exposed");
    let matrix_parameter_keys = matrix_descriptor["parameter_schema"]
        .as_array()
        .expect("matrix stack parameter schema should be an array")
        .iter()
        .map(|parameter| parameter["key"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        matrix_parameter_keys,
        vec![
            "columns",
            "rows",
            "box_width",
            "box_height",
            "gap_x",
            "gap_y",
            "layout",
            "material",
            "density",
            "velocity_iterations",
            "position_iterations",
            "contact_position_correction",
            "substeps_per_frame",
        ]
    );
    let lattice_descriptor = scenarios_body["scenarios"]
        .as_array()
        .expect("scenarios should be an array")
        .iter()
        .find(|scenario| scenario["id"] == "lattice_grid")
        .expect("lattice grid descriptor should be exposed");
    let lattice_parameter_keys = lattice_descriptor["parameter_schema"]
        .as_array()
        .expect("lattice grid parameter schema should be an array")
        .iter()
        .map(|parameter| parameter["key"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        lattice_parameter_keys,
        vec![
            "columns",
            "rows",
            "spacing_x",
            "spacing_y",
            "node_radius",
            "constraint_profile",
            "joint_velocity_projection",
            "substeps_per_frame",
        ]
    );

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "scenario_id": "falling_box_contact",
                        "frame_count": 3
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = json_body(created).await;
    assert_eq!(
        created_body["session"]["scenario_id"],
        "falling_box_contact"
    );
    assert_eq!(created_body["session"]["mode"], "artifact_replay");
    assert_eq!(created_body["session"]["status"], "completed");
    assert_eq!(created_body["session"]["buffered_frame_count"], 3);
    assert_eq!(
        serde_json::from_value::<SessionStatus>(created_body["session"]["status"].clone()).unwrap(),
        SessionStatus::Completed
    );
    assert_eq!(
        created_body["session"]["effective_runtime_config"]["step"]["velocity_iterations"],
        10
    );
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    let run_id = created_body["session"]["run_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        created_body["session"]["manifest_artifact"],
        "manifest.json"
    );
    assert_eq!(
        created_body["session"]["final_snapshot_artifact"],
        "final_snapshot.json"
    );

    let newton_created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "scenario_id": "newton_cradle",
                        "frame_count": 1,
                        "overrides": {
                            "scene_params": {
                                "ball_count": 3,
                                "velocity_iterations": 4
                            }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(newton_created.status(), StatusCode::CREATED);
    let newton_created_body = json_body(newton_created).await;
    assert_eq!(
        newton_created_body["session"]["effective_runtime_config"]["scene_params"]["ball_count"],
        3
    );
    assert_eq!(
        newton_created_body["session"]["effective_runtime_config"]["step"]["velocity_iterations"],
        4
    );

    let matrix_created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "scenario_id": "matrix_stack",
                        "frame_count": 1,
                        "overrides": {
                            "scene_params": {
                                "columns": 3,
                                "rows": 2,
                                "layout": "aligned",
                                "substeps_per_frame": 2
                            }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(matrix_created.status(), StatusCode::CREATED);
    let matrix_created_body = json_body(matrix_created).await;
    assert_eq!(
        matrix_created_body["session"]["effective_runtime_config"]["scene_params"]["columns"],
        3
    );
    assert_eq!(
        matrix_created_body["session"]["effective_runtime_config"]["scene_params"]["rows"],
        2
    );
    assert_eq!(
        matrix_created_body["session"]["effective_runtime_config"]["substeps_per_frame"],
        2
    );

    let fetched = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fetched.status(), StatusCode::OK);
    assert_eq!(json_body(fetched).await["session"]["id"], session_id);

    let patched = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/sessions/{session_id}/overrides"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "frame_count": 2 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(patched.status(), StatusCode::OK);
    assert_eq!(
        json_body(patched).await["session"]["overrides"]["frame_count"],
        2
    );

    let reset = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "action": "reset" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reset.status(), StatusCode::OK);
    let reset_body = json_body(reset).await;
    assert_eq!(reset_body["session"]["frame_count"], 2);
    assert_eq!(reset_body["session"]["buffered_frame_count"], 2);
    assert_eq!(reset_body["session"]["current_frame_index"], 0);
    assert_eq!(reset_body["session"]["status"], "completed");

    let step = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "action": "step", "detail": "full" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(step.status(), StatusCode::OK);
    let step_body = json_body(step).await;
    assert_eq!(step_body["session"]["current_frame_index"], 1);
    assert_eq!(step_body["session"]["buffered_frame_count"], 2);
    assert_eq!(
        step_body["session"]["run_id"], reset_body["session"]["run_id"],
        "step should advance within the existing run instead of rerunning physics"
    );

    let play = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "action": "play" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(play.status(), StatusCode::OK);
    let play_body = json_body(play).await;
    assert_eq!(play_body["session"]["current_frame_index"], 1);
    assert_eq!(
        play_body["session"]["run_id"], reset_body["session"]["run_id"],
        "play should consume the existing run artifact instead of rerunning physics"
    );

    let events = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}/events"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(events.status(), StatusCode::OK);
    assert_eq!(
        events.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    let events_body = body_text(events).await;
    assert!(
        events_body.contains("event: frame"),
        "initial SSE endpoint should replay frame events, got {events_body:?}"
    );

    let drained_events = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}/events"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let drained_body = body_text(drained_events).await;
    assert!(
        drained_body.contains("event: idle") && !drained_body.contains("event: failed"),
        "drained SSE queue should be idle instead of failed, got {drained_body:?}"
    );

    for action in ["pause"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/sessions/{session_id}/control"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "action": action }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{action} should be accepted"
        );
    }

    let manifest = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/runs/{run_id}/artifacts/manifest.json"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(manifest.status(), StatusCode::OK);
    let manifest_body = json_body(manifest).await;
    assert_eq!(
        manifest_body["scenario_id"],
        serde_json::to_value(ScenarioId::FallingBoxContact).unwrap()
    );

    let final_snapshot = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/runs/{}/artifacts/final_snapshot.json",
                    reset_body["session"]["run_id"]
                        .as_str()
                        .expect("reset should produce a run")
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(final_snapshot.status(), StatusCode::OK);
    assert_eq!(
        json_body(final_snapshot).await["stats"]["step_index"],
        reset_body["session"]["frame_count"]
    );
}

#[tokio::test]
async fn artifact_reset_runs_off_worker_without_blocking_concurrent_reads() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "scenario_id": "falling_box_contact", "frame_count": 3 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let session_id = json_body(created).await["session"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // The artifact reset re-runs the full simulation on a blocking worker with
    // the session mutex released. A concurrent read issued during that window
    // must observe a consistent, completed record (never a torn or failed
    // intermediate), and neither request may deadlock.
    let reset_fut = app.clone().oneshot(
        Request::builder()
            .method(Method::POST)
            .uri(format!("/api/sessions/{session_id}/control"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "action": "reset" }).to_string()))
            .unwrap(),
    );
    let read_fut = app.clone().oneshot(
        Request::builder()
            .method(Method::GET)
            .uri(format!("/api/sessions/{session_id}"))
            .body(Body::empty())
            .unwrap(),
    );
    let (reset_res, read_res) = tokio::join!(reset_fut, read_fut);

    let reset = reset_res.unwrap();
    assert_eq!(reset.status(), StatusCode::OK);
    let reset_body = json_body(reset).await;
    assert_eq!(reset_body["session"]["status"], "completed");
    assert_eq!(reset_body["session"]["frame_count"], 3);
    assert_eq!(reset_body["session"]["current_frame_index"], 0);

    let read = read_res.unwrap();
    assert_eq!(read.status(), StatusCode::OK);
    let read_body = json_body(read).await;
    assert_eq!(
        read_body["session"]["status"], "completed",
        "a concurrent read during an artifact reset must never observe a torn or failed state"
    );
    assert_eq!(read_body["session"]["frame_count"], 3);
}

#[tokio::test]
async fn live_session_step_advances_backend_world_and_reset_clears_buffer() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "scenario_id": "falling_box_contact",
                        "frame_count": 3,
                        "mode": "live_session"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = json_body(created).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    assert_eq!(created_body["session"]["mode"], "live_session");
    assert_eq!(created_body["session"]["status"], "created");
    assert_eq!(created_body["session"]["run_id"], Value::Null);
    assert_eq!(created_body["session"]["buffered_frame_count"], 0);
    assert_eq!(created_body["session"]["latest_frame"], Value::Null);

    let first_step = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "action": "step", "detail": "full" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first_step.status(), StatusCode::OK);
    let first_step_body = json_body(first_step).await;
    assert_eq!(first_step_body["session"]["status"], "paused");
    assert_eq!(first_step_body["session"]["buffered_frame_count"], 1);
    assert_eq!(first_step_body["session"]["current_frame_index"], 0);
    assert_eq!(first_step_body["session"]["latest_frame"]["frame_index"], 0);
    assert_live_frame_uses_current_full_payload(
        &first_step_body["session"]["latest_frame"],
        "live step should keep the existing full-frame payload until LP-E2 changes the transport detail",
    );
    assert_eq!(
        first_step_body["session"]["latest_frame"]["snapshot"]["stats"]["step_index"],
        1
    );
    let first_hash = first_step_body["session"]["latest_frame"]["state_hash"]
        .as_str()
        .unwrap()
        .to_owned();

    let second_step = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "action": "step", "detail": "full" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second_step.status(), StatusCode::OK);
    let second_step_body = json_body(second_step).await;
    assert_eq!(second_step_body["session"]["buffered_frame_count"], 2);
    assert_eq!(second_step_body["session"]["current_frame_index"], 1);
    assert_eq!(
        second_step_body["session"]["latest_frame"]["frame_index"],
        1
    );
    assert_eq!(
        second_step_body["session"]["latest_frame"]["snapshot"]["stats"]["step_index"],
        2
    );
    let second_hash = second_step_body["session"]["latest_frame"]["state_hash"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(second_hash, first_hash);

    let reset = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "action": "reset" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reset.status(), StatusCode::OK);
    let reset_body = json_body(reset).await;
    assert_eq!(reset_body["session"]["status"], "created");
    assert_eq!(reset_body["session"]["buffered_frame_count"], 0);
    assert_eq!(reset_body["session"]["current_frame_index"], 0);
    assert_eq!(reset_body["session"]["latest_frame"], Value::Null);
    assert_eq!(reset_body["session"]["run_id"], Value::Null);

    let step_after_reset = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "action": "step", "detail": "full" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(step_after_reset.status(), StatusCode::OK);
    let step_after_reset_body = json_body(step_after_reset).await;
    assert_eq!(
        step_after_reset_body["session"]["latest_frame"]["frame_index"],
        0
    );
    assert_eq!(
        step_after_reset_body["session"]["latest_frame"]["snapshot"]["stats"]["step_index"],
        1
    );
    assert_eq!(
        step_after_reset_body["session"]["latest_frame"]["state_hash"], first_hash,
        "reset should rebuild the world and restart the live frame buffer deterministically"
    );
}

#[tokio::test]
async fn live_session_step_defaults_to_summary_and_omits_full_only_payload() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "falling_box_contact", 3).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();

    let default_step = control_session_without_detail(&app, &session_id, "step").await;
    assert_eq!(
        default_step["session"]["latest_frame"],
        Value::Null,
        "default live step should stop shipping the full latest_frame payload on the hot path",
    );
    let latest_summary = &default_step["live_frame_summary"];
    assert_eq!(
        latest_summary["kind"], "summary",
        "summary payload should identify itself so the web can avoid treating missing diagnostics as zero issues",
    );
    for field in [
        "frame_index",
        "simulated_time",
        "state_hash",
        "session_id",
        "session_epoch",
        "world_revision",
        "status",
        "buffered_frame_count",
        "frame_count",
        "snapshot",
        "stats",
    ] {
        assert!(
            latest_summary.get(field).is_some(),
            "summary payload should include {field}"
        );
    }
    assert_eq!(latest_summary["session_id"], session_id);
    assert_eq!(latest_summary["frame_index"], 0);
    assert_eq!(latest_summary["session_epoch"], 0);
    assert_eq!(latest_summary["buffered_frame_count"], 1);
    assert_eq!(latest_summary["frame_count"], 3);
    assert_eq!(latest_summary["status"], "paused");
    assert_eq!(
        latest_summary["snapshot"]["contacts"],
        Value::Null,
        "summary payload should omit full contact facts from the hot response",
    );
    assert_eq!(
        latest_summary["snapshot"]["broadphase_tree"],
        Value::Null,
        "summary payload should omit broadphase tree details from the hot response",
    );
    assert_eq!(
        latest_summary["snapshot"]["islands"],
        Value::Null,
        "summary payload should omit island details from the hot response",
    );
    assert_eq!(
        latest_summary["diagnostics"],
        Value::Null,
        "summary payload should make missing diagnostics explicit instead of pretending there are no issues",
    );
    assert_eq!(latest_summary["events"], Value::Null);
    assert_eq!(latest_summary["report"], Value::Null);
    assert_eq!(latest_summary["compound_provenance"], Value::Null);
    assert_eq!(latest_summary["perturbation_provenance"], Value::Null);
}

#[tokio::test]
async fn live_session_step_detail_full_keeps_existing_latest_frame_payload() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "falling_box_contact", 3).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    let detailed_step = control_session(&app, &session_id, "step").await;
    let latest_frame = &detailed_step["session"]["latest_frame"];

    assert_live_frame_uses_current_full_payload(
        latest_frame,
        "detail=full should keep the old full FrameRecord contract for manual/compat callers",
    );
    assert_eq!(
        detailed_step["live_frame_summary"]["kind"], "summary",
        "detail=full should still surface the additive live summary metadata for the web hot path",
    );
}

#[tokio::test]
async fn live_session_frame_lookup_returns_full_frame_without_mutating_session() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "falling_box_contact", 3).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    let stepped = control_session_without_detail(&app, &session_id, "step").await;
    let session_before_lookup = fetch_session(&app, &session_id).await;
    let world_revision = stepped["live_frame_summary"]["world_revision"].clone();

    let fetched_frame = fetch_live_frame(&app, &session_id, 0).await;
    assert_eq!(fetched_frame["frame"]["frame_index"], 0);
    assert_eq!(
        fetched_frame["frame"]["snapshot"]["meta"]["revision"],
        world_revision,
    );
    assert_live_frame_uses_current_full_payload(
        &fetched_frame["frame"],
        "live frame lookup should hydrate the authoritative full FrameRecord",
    );
    let session_after_lookup = fetch_session(&app, &session_id).await;
    for field in [
        "status",
        "session_epoch",
        "latest_frame",
        "buffered_frame_count",
        "current_frame_index",
        "run_id",
    ] {
        assert_eq!(
            session_after_lookup[field], session_before_lookup[field],
            "missing future frame lookup route must not mutate session field {field}"
        );
    }
}

#[tokio::test]
async fn live_unbounded_session_steps_past_frame_count_with_retained_ring_window() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body =
        create_live_session_with_options(&app, "falling_box_contact", 2, true, 3).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    assert_eq!(created_body["session"]["live_unbounded"], true);
    assert_eq!(created_body["session"]["live_buffer_capacity"], 3);
    assert_eq!(created_body["session"]["produced_frame_count"], 0);
    assert_eq!(created_body["session"]["retained_frame_start"], 0);
    assert_eq!(created_body["session"]["retained_frame_end_exclusive"], 0);

    let mut step_body = Value::Null;
    for _ in 0..5 {
        step_body = control_session(&app, &session_id, "step").await;
    }

    let session = &step_body["session"];
    assert_eq!(session["status"], "paused");
    assert_eq!(session["latest_frame"]["frame_index"], 4);
    assert_eq!(session["current_frame_index"], 4);
    assert_eq!(session["produced_frame_count"], 5);
    assert_eq!(session["buffered_frame_count"], 3);
    assert_eq!(session["retained_frame_start"], 2);
    assert_eq!(session["retained_frame_end_exclusive"], 5);
    assert_eq!(session["live_buffer_capacity"], 3);
    assert_eq!(session["live_unbounded"], true);

    let retained_frame = fetch_live_frame(&app, &session_id, 2).await;
    assert_eq!(retained_frame["frame_index"], 2);
    assert_eq!(retained_frame["frame"]["frame_index"], 2);

    let evicted = fetch_live_frame_response(&app, &session_id, 1).await;
    assert_eq!(evicted.status(), StatusCode::GONE);
    let evicted_body = json_body(evicted).await;
    assert_eq!(evicted_body["error_kind"], "live_frame_evicted");
    assert_eq!(evicted_body["frame_index"], 1);
    assert_eq!(evicted_body["retained_frame_start"], 2);
    assert_eq!(evicted_body["retained_frame_end_exclusive"], 5);

    let future = fetch_live_frame_response(&app, &session_id, 5).await;
    assert_eq!(future.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn live_unbounded_ring_window_rejects_preview_commit_from_evicted_frame() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body =
        create_live_session_with_options(&app, "falling_box_contact", 2, true, 1).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    let first_step = control_session(&app, &session_id, "step").await;
    let first_session = &first_step["session"];
    let world_revision = first_session["latest_frame"]["snapshot"]["meta"]["revision"]
        .as_u64()
        .expect("world revision should be numeric");
    let frame_index = first_session["current_frame_index"]
        .as_u64()
        .expect("frame index should be numeric");
    let dynamic_handle = first_body_with_type(first_session, "dynamic")["handle"].clone();

    let preview_body = post_preview(
        &app,
        &session_id,
        json!({
            "action_id": "evicted-preview",
            "world_revision": world_revision,
            "session_epoch": 0,
            "body_handle": dynamic_handle,
            "frame_index": frame_index,
            "requested_delta": [0.25, 0.0],
            "wake_intent": true
        }),
    )
    .await;
    let preview = preview_body["preview"].clone();
    assert_eq!(preview["rejection_reason"], Value::Null);

    let second_step = control_session(&app, &session_id, "step").await;
    assert_eq!(second_step["session"]["current_frame_index"], 1);
    assert_eq!(second_step["session"]["retained_frame_start"], 1);
    assert_eq!(second_step["session"]["retained_frame_end_exclusive"], 2);

    let stale_preview = post_preview(
        &app,
        &session_id,
        json!({
            "action_id": "evicted-preview-again",
            "world_revision": world_revision,
            "session_epoch": 0,
            "body_handle": dynamic_handle,
            "frame_index": frame_index,
            "requested_delta": [0.25, 0.0],
            "wake_intent": true
        }),
    )
    .await;
    assert_eq!(
        stale_preview["preview"]["rejection_reason"],
        "stale_world_revision"
    );

    let stale_commit = post_commit(&app, &session_id, commit_request_from_preview(&preview)).await;
    assert_eq!(stale_commit["commit"]["accepted"], false);
    assert_eq!(
        stale_commit["commit"]["rejection_reason"],
        "stale_world_revision"
    );
    let after_stale = fetch_session(&app, &session_id).await;
    assert_eq!(after_stale["current_frame_index"], 1);
    assert_eq!(after_stale["retained_frame_start"], 1);
    assert_eq!(after_stale["retained_frame_end_exclusive"], 2);
}

#[tokio::test]
async fn live_lattice_grid_session_steps_with_joint_proxy_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "lattice_grid", 2).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    assert_eq!(created_body["session"]["scenario_id"], "lattice_grid");
    assert_eq!(created_body["session"]["mode"], "live_session");
    assert_eq!(created_body["session"]["latest_frame"], Value::Null);

    let step_body = control_session(&app, &session_id, "step").await;
    let session = &step_body["session"];
    assert_eq!(session["status"], "paused");
    assert_eq!(session["scenario_id"], "lattice_grid");
    assert_eq!(session["buffered_frame_count"], 1);

    let snapshot = &session["latest_frame"]["snapshot"];
    let bodies = snapshot["bodies"]
        .as_array()
        .expect("live lattice frame should include bodies");
    let dynamic_body_count = bodies
        .iter()
        .filter(|body| body["body_type"] == "dynamic")
        .count();
    let kinematic_body_count = bodies
        .iter()
        .filter(|body| body["body_type"] == "kinematic")
        .count();
    assert_eq!(
        dynamic_body_count + kinematic_body_count,
        12,
        "live lattice proxy should build the 4x3 rigid-body node grid"
    );
    assert_eq!(
        (dynamic_body_count, kinematic_body_count),
        (8, 4),
        "live lattice proxy should pin the top row and simulate the remaining nodes"
    );

    let joints = snapshot["joints"]
        .as_array()
        .expect("live lattice frame should include joints");
    let distance_joint_count = joints
        .iter()
        .filter(|joint| joint["kind"] == "distance")
        .count();
    let world_anchor_joint_count = joints
        .iter()
        .filter(|joint| joint["kind"] == "world_anchor")
        .count();
    assert_eq!(distance_joint_count, 23);
    assert_eq!(world_anchor_joint_count, 4);
    assert_eq!(joints.len(), 27);
    assert!(
        joints.iter().all(|joint| {
            joint["anchors"]
                .as_array()
                .is_some_and(|anchors| anchors.len() >= 2)
        }),
        "lattice proxy joints should carry screen-drawable anchor pairs"
    );
    assert!(
        snapshot["stats"]["joint_row_count"]
            .as_u64()
            .is_some_and(|rows| rows > 0),
        "live lattice proxy should expose joint solver row facts"
    );
}

#[tokio::test]
async fn live_session_play_is_status_only_until_backend_step() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "scenario_id": "falling_box_contact",
                        "frame_count": 2,
                        "mode": "live_session"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = json_body(created).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();

    let play = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "action": "play" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(play.status(), StatusCode::OK);
    let play_body = json_body(play).await;
    assert_eq!(play_body["session"]["status"], "running");
    assert_eq!(play_body["session"]["buffered_frame_count"], 0);
    assert_eq!(play_body["session"]["current_frame_index"], 0);
    assert_eq!(play_body["session"]["latest_frame"], Value::Null);

    let events = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}/events"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        body_text(events).await.contains("event: idle"),
        "live play should not synthesize frame events before backend step"
    );

    let step = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "action": "step", "detail": "full" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(step.status(), StatusCode::OK);
    let step_body = json_body(step).await;
    assert_eq!(step_body["session"]["status"], "running");
    assert_eq!(step_body["session"]["buffered_frame_count"], 1);
    assert_eq!(step_body["session"]["latest_frame"]["frame_index"], 0);
    assert_eq!(
        step_body["session"]["latest_frame"]["snapshot"]["stats"]["step_index"],
        1
    );

    let pause_after_step = control_session_with_detail(&app, &session_id, "pause", None).await;
    assert_eq!(pause_after_step["session"]["status"], "paused");
    assert_eq!(
        pause_after_step["session"]["latest_frame"],
        Value::Null,
        "default live control responses should not leak full latest_frame after the summary transport split",
    );
    assert_eq!(
        pause_after_step["live_frame_summary"]["kind"], "summary",
        "default live control responses should still expose lightweight latest-frame metadata",
    );
}

#[tokio::test]
async fn live_session_overrides_patch_is_rejected_without_mutating_live_runtime() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "scenario_id": "falling_box_contact",
                        "frame_count": 3,
                        "mode": "live_session",
                        "overrides": {
                            "gravity": [0.0, 9.8]
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = json_body(created).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();

    let patched = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/sessions/{session_id}/overrides"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "frame_count": 9,
                        "gravity": [0.0, 1.25]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(patched.status(), StatusCode::BAD_REQUEST);
    let patched_body = json_body(patched).await;
    assert!(
        patched_body["error"]
            .as_str()
            .expect("error should be a string")
            .contains("M25-B paused transaction"),
        "live override rejection should explain the M25-B transaction boundary"
    );

    let fetched = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fetched.status(), StatusCode::OK);
    let fetched_body = json_body(fetched).await;
    assert_eq!(fetched_body["session"]["frame_count"], 3);
    assert_eq!(
        fetched_body["session"]["overrides"]["gravity"][0],
        json!(0.0)
    );
    let gravity_y = fetched_body["session"]["overrides"]["gravity"][1]
        .as_f64()
        .expect("gravity y should be numeric");
    assert!(
        (gravity_y - 9.8).abs() < 1.0e-3,
        "live override rejection should keep the original gravity"
    );

    let first_step = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "action": "step", "detail": "full" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first_step.status(), StatusCode::OK);
    let first_step_body = json_body(first_step).await;
    assert_eq!(
        first_step_body["session"]["latest_frame"]["snapshot"]["meta"]["gravity"]["x"],
        json!(0.0)
    );
    let stepped_gravity_y = first_step_body["session"]["latest_frame"]["snapshot"]["meta"]
        ["gravity"]["y"]
        .as_f64()
        .expect("stepped gravity y should be numeric");
    assert!(
        (stepped_gravity_y - 9.8).abs() < 1.0e-3,
        "live runtime should keep the original gravity after a rejected patch"
    );
}

#[tokio::test]
async fn live_gravity_patch_updates_runtime_snapshot_and_reset_gravity() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "falling_box_contact", 4).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();

    let first_step = control_session(&app, &session_id, "step").await;
    assert_eq!(first_step["session"]["session_epoch"], 0);
    let first_revision = first_step["session"]["latest_frame"]["snapshot"]["meta"]["revision"]
        .as_u64()
        .expect("first revision should be numeric");

    let patched = patch_live_gravity(&app, &session_id, 0, json!([1.5, -2.0])).await;
    assert_eq!(patched["session"]["session_epoch"], 1);
    assert_eq!(patched["session"]["current_frame_index"], 0);
    assert_eq!(patched["session"]["latest_frame"]["frame_index"], 0);
    assert_eq!(
        patched["session"]["overrides"]["gravity"],
        json!([1.5, -2.0])
    );
    assert_eq!(
        patched["session"]["latest_frame"]["snapshot"]["meta"]["gravity"],
        json!({ "x": 1.5, "y": -2.0 })
    );
    let patched_revision = patched["session"]["latest_frame"]["snapshot"]["meta"]["revision"]
        .as_u64()
        .expect("patched revision should be numeric");
    assert!(
        patched_revision > first_revision,
        "live gravity patch should bump world revision"
    );

    let next_step = control_session(&app, &session_id, "step").await;
    assert_eq!(
        next_step["session"]["latest_frame"]["snapshot"]["meta"]["gravity"],
        json!({ "x": 1.5, "y": -2.0 })
    );

    let reset = control_session(&app, &session_id, "reset").await;
    assert_eq!(reset["session"]["session_epoch"], 2);
    let step_after_reset = control_session(&app, &session_id, "step").await;
    assert_eq!(
        step_after_reset["session"]["latest_frame"]["snapshot"]["meta"]["gravity"],
        json!({ "x": 1.5, "y": -2.0 }),
        "live reset should rebuild the world with the last applied gravity override"
    );
}

#[tokio::test]
async fn live_velocity_perturbation_preview_is_read_only_for_paused_session() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "falling_box_contact", 3).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    assert_eq!(created_body["session"]["session_epoch"], 0);

    let step_body = control_session(&app, &session_id, "step").await;
    assert_eq!(step_body["session"]["status"], "paused");
    let paused_session = step_body["session"].clone();
    let body = first_body_with_type(&paused_session, "dynamic");
    let body_handle = body["handle"].clone();
    let before_velocity = body["linear_velocity"].clone();
    let before_x = vector_x(&before_velocity);
    let before_y = vector_y(&before_velocity);
    let world_revision = paused_session["latest_frame"]["snapshot"]["meta"]["revision"].clone();
    let frame_index = paused_session["current_frame_index"].clone();

    let preview_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/sessions/{session_id}/velocity-perturbations/preview"
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "action_id": "preview-happy",
                        "world_revision": world_revision,
                        "session_epoch": 0,
                        "body_handle": body_handle,
                        "frame_index": frame_index,
                        "requested_delta": [0.75, -1.25],
                        "wake_intent": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(preview_response.status(), StatusCode::OK);
    let preview_body = json_body(preview_response).await;
    let preview = &preview_body["preview"];
    assert_eq!(preview["action_id"], "preview-happy");
    assert_eq!(preview["session_id"], session_id);
    assert_eq!(preview["world_revision"], world_revision);
    assert_eq!(preview["session_epoch"], 0);
    assert_eq!(preview["body_handle"], body_handle);
    assert_eq!(preview["frame_index"], frame_index);
    assert_eq!(preview["before_velocity"], before_velocity);
    assert_eq!(preview["requested_delta"], json!({ "x": 0.75, "y": -1.25 }));
    assert_close(
        vector_x(&preview["computed_target_velocity"]),
        before_x + 0.75,
    );
    assert_close(
        vector_y(&preview["computed_target_velocity"]),
        before_y - 1.25,
    );
    assert_eq!(preview["wake_intent"], true);
    assert_eq!(preview["rejection_reason"], Value::Null);

    let fetched = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fetched.status(), StatusCode::OK);
    let fetched_body = json_body(fetched).await;
    let fetched_session = &fetched_body["session"];
    assert_eq!(fetched_session["status"], paused_session["status"]);
    assert_eq!(
        fetched_session["buffered_frame_count"],
        paused_session["buffered_frame_count"]
    );
    assert_eq!(
        fetched_session["current_frame_index"],
        paused_session["current_frame_index"]
    );
    assert_eq!(
        fetched_session["latest_frame"],
        paused_session["latest_frame"]
    );
    assert_eq!(fetched_session["run_id"], paused_session["run_id"]);
    assert_eq!(
        fetched_session["session_epoch"],
        paused_session["session_epoch"]
    );

    let events = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}/events"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let events_body = body_text(events).await;
    assert!(
        events_body.contains("event: frame") && !events_body.contains("preview"),
        "preview must neither clear existing frame events nor emit preview events, got {events_body:?}"
    );
}

#[tokio::test]
async fn live_velocity_perturbation_preview_rejects_stale_or_mutating_requests() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "falling_box_contact", 3).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    let step_body = control_session(&app, &session_id, "step").await;
    let paused_session = step_body["session"].clone();
    drain_session_events(&app, &session_id).await;
    let world_revision = paused_session["latest_frame"]["snapshot"]["meta"]["revision"]
        .as_u64()
        .expect("world revision should be numeric");
    let frame_index = paused_session["current_frame_index"]
        .as_u64()
        .expect("frame index should be numeric");
    let dynamic_handle = first_body_with_type(&paused_session, "dynamic")["handle"].clone();
    let static_handle = first_body_with_type(&paused_session, "static")["handle"].clone();

    assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "stale-revision",
            "world_revision": world_revision.saturating_sub(1),
            "session_epoch": 0,
            "body_handle": dynamic_handle,
            "frame_index": frame_index,
            "requested_delta": [0.25, 0.0]
        }),
        "stale_world_revision",
    )
    .await;
    assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "stale-epoch",
            "world_revision": world_revision,
            "session_epoch": 1,
            "body_handle": dynamic_handle,
            "frame_index": frame_index,
            "requested_delta": [0.25, 0.0]
        }),
        "stale_session_epoch",
    )
    .await;
    assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "stale-frame",
            "world_revision": world_revision,
            "session_epoch": 0,
            "body_handle": dynamic_handle,
            "frame_index": frame_index + 1,
            "requested_delta": [0.25, 0.0]
        }),
        "stale_frame",
    )
    .await;
    assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "static-handle",
            "world_revision": world_revision,
            "session_epoch": 0,
            "body_handle": static_handle,
            "frame_index": frame_index,
            "requested_delta": [0.25, 0.0]
        }),
        "static_body",
    )
    .await;
    assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "stale-handle",
            "world_revision": world_revision,
            "session_epoch": 0,
            "body_handle": 18446744073709551615u64,
            "frame_index": frame_index,
            "requested_delta": [0.25, 0.0]
        }),
        "invalid_body_handle",
    )
    .await;
    assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "invalid-delta",
            "world_revision": world_revision,
            "session_epoch": 0,
            "body_handle": dynamic_handle,
            "frame_index": frame_index,
            "requested_delta": [0.0, 0.0]
        }),
        "invalid_velocity_delta",
    )
    .await;
    assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "non-finite-delta",
            "world_revision": world_revision,
            "session_epoch": 0,
            "body_handle": dynamic_handle,
            "frame_index": frame_index,
            "requested_delta": ["NaN", 0.0]
        }),
        "invalid_velocity_delta",
    )
    .await;

    let _running = control_session(&app, &session_id, "play").await;
    assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "running",
            "world_revision": world_revision,
            "session_epoch": 0,
            "body_handle": dynamic_handle,
            "frame_index": frame_index,
            "requested_delta": [0.25, 0.0]
        }),
        "session_running",
    )
    .await;

    let next_step = control_session(&app, &session_id, "step").await;
    assert_eq!(next_step["session"]["status"], "running");
    assert_eq!(next_step["session"]["buffered_frame_count"], 2);
    assert_eq!(next_step["session"]["current_frame_index"], frame_index + 1);
    assert_eq!(
        next_step["session"]["latest_frame"]["frame_index"],
        frame_index + 1
    );
    let next_events = drain_session_events(&app, &session_id).await;
    assert!(
        next_events.contains("event: frame") && !next_events.contains("preview"),
        "next live step should emit only frame events after rejected previews, got {next_events:?}"
    );

    let completed_body = create_live_session(&app, "falling_box_contact", 1).await;
    let completed_session_id = completed_body["session"]["id"].as_str().unwrap().to_owned();
    let completed_step = control_session(&app, &completed_session_id, "step").await;
    let completed_session = &completed_step["session"];
    assert_eq!(completed_session["status"], "completed");
    drain_session_events(&app, &completed_session_id).await;
    let completed_revision = completed_session["latest_frame"]["snapshot"]["meta"]["revision"]
        .as_u64()
        .expect("world revision should be numeric");
    let completed_handle = first_body_with_type(completed_session, "dynamic")["handle"].clone();
    assert_preview_rejection(
        &app,
        &completed_session_id,
        json!({
            "action_id": "completed",
            "world_revision": completed_revision,
            "session_epoch": 0,
            "body_handle": completed_handle,
            "frame_index": 0,
            "requested_delta": [0.25, 0.0]
        }),
        "session_completed",
    )
    .await;
}

#[tokio::test]
async fn live_velocity_perturbation_commit_applies_previewed_velocity_and_provenance() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "falling_box_contact", 4).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    let step_body = control_session(&app, &session_id, "step").await;
    let paused_session = step_body["session"].clone();
    let body = first_body_with_type(&paused_session, "dynamic");
    let body_handle = body["handle"].clone();
    let before_velocity = body["linear_velocity"].clone();
    let world_revision = paused_session["latest_frame"]["snapshot"]["meta"]["revision"].clone();
    let frame_index = paused_session["current_frame_index"].clone();
    let old_frame_hash = paused_session["latest_frame"]["state_hash"]
        .as_str()
        .expect("latest frame should carry a state hash")
        .to_owned();

    let preview_body = post_preview(
        &app,
        &session_id,
        json!({
            "action_id": "commit-happy",
            "world_revision": world_revision,
            "session_epoch": 0,
            "body_handle": body_handle,
            "frame_index": frame_index,
            "requested_delta": [2.0, 0.0],
            "wake_intent": true
        }),
    )
    .await;
    let preview = &preview_body["preview"];
    assert_eq!(preview["rejection_reason"], Value::Null);
    let target_velocity = preview["computed_target_velocity"].clone();
    let target_x = vector_x(&target_velocity);
    let target_y = vector_y(&target_velocity);

    let commit_body = post_commit(&app, &session_id, commit_request_from_preview(preview)).await;
    let commit = &commit_body["commit"];
    assert_eq!(commit["accepted"], true);
    assert_eq!(commit["rejection_reason"], Value::Null);
    assert_eq!(commit["action_id"], "commit-happy");
    assert_eq!(commit["before_velocity"], before_velocity);
    assert_eq!(commit["computed_target_velocity"], target_velocity);
    assert_eq!(commit_body["session"]["status"], "paused");
    assert_eq!(commit_body["session"]["session_epoch"], 1);
    assert_eq!(commit_body["session"]["current_frame_index"], frame_index);
    assert_eq!(
        commit_body["session"]["latest_frame"]["frame_index"],
        frame_index
    );
    let refreshed_frame_hash = commit_body["session"]["latest_frame"]["state_hash"]
        .as_str()
        .expect("refreshed frame should carry a state hash");
    assert_ne!(
        refreshed_frame_hash, old_frame_hash,
        "accepted commit should refresh the current frame hash"
    );
    assert_eq!(commit_body["session"]["buffered_frame_count"], 1);
    assert!(
        commit_body["session"]["latest_frame"]["snapshot"]["meta"]["revision"]
            .as_u64()
            .expect("revision should be numeric")
            > world_revision.as_u64().expect("revision should be numeric"),
        "commit should bump world revision through World::apply_body_patch"
    );

    let committed_body = body_by_handle(
        &commit_body["session"]["latest_frame"]["snapshot"],
        &body_handle,
    );
    assert_close(vector_x(&committed_body["linear_velocity"]), target_x);
    assert_close(vector_y(&committed_body["linear_velocity"]), target_y);

    let provenance = commit_body["session"]["latest_frame"]["perturbation_provenance"]
        .as_array()
        .expect("perturbation provenance should be an array");
    assert_eq!(provenance.len(), 1);
    let entry = &provenance[0];
    assert_eq!(entry["action_id"], "commit-happy");
    assert_eq!(entry["session_id"], session_id);
    assert_eq!(entry["body_handle"], body_handle);
    assert_eq!(entry["frame_index"], frame_index);
    assert_eq!(entry["before_velocity"], before_velocity);
    assert_eq!(entry["requested_delta"], preview["requested_delta"]);
    assert_eq!(entry["computed_target_velocity"], target_velocity);
    assert_eq!(entry["wake_intent"], true);
    assert_eq!(entry["commit_outcome"], "accepted");
    assert_eq!(entry["query_sync_status"], "synced");

    let commit_events = drain_session_events(&app, &session_id).await;
    assert!(
        !commit_events.contains(&old_frame_hash),
        "accepted commit must not leave stale queued frame hashes, got {commit_events:?}"
    );
    if commit_events.contains("event: frame") {
        assert!(
            commit_events.contains(refreshed_frame_hash),
            "queued frame events after commit must match latest_frame hash, got {commit_events:?}"
        );
    }

    let after_commit = commit_body["session"].clone();
    let reused = post_commit(&app, &session_id, commit_request_from_preview(preview)).await;
    assert_eq!(reused["commit"]["accepted"], false);
    assert_eq!(reused["commit"]["rejection_reason"], "reused_action");
    let after_reused = fetch_session(&app, &session_id).await;
    assert_commit_rejection_preserved_session(&after_commit, &after_reused);

    let next_step = control_session(&app, &session_id, "step").await;
    assert_eq!(next_step["session"]["current_frame_index"], 1);
    let next_body = body_by_handle(
        &next_step["session"]["latest_frame"]["snapshot"],
        &body_handle,
    );
    assert_close(vector_x(&next_body["linear_velocity"]), target_x);
}

#[tokio::test]
async fn live_velocity_perturbation_commit_rejects_stale_or_unpreviewed_transactions() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "falling_box_contact", 3).await;
    let created_session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    let revision_probe = post_preview(
        &app,
        &created_session_id,
        json!({
            "action_id": "created-revision-probe",
            "world_revision": 0,
            "session_epoch": 0,
            "body_handle": 0,
            "frame_index": 0,
            "requested_delta": [0.25, 0.0]
        }),
    )
    .await;
    assert_commit_rejection(
        &app,
        &created_session_id,
        json!({
            "action_id": "created",
            "world_revision": revision_probe["preview"]["world_revision"],
            "session_epoch": 0,
            "body_handle": 0,
            "frame_index": 0,
            "requested_delta": [0.25, 0.0]
        }),
        "session_created",
    )
    .await;

    let fixture = paused_commit_fixture(&app, "missing-action", 3).await;
    assert_commit_rejection(
        &app,
        &fixture.session_id,
        json!({
            "action_id": "missing-action",
            "world_revision": fixture.world_revision,
            "session_epoch": 0,
            "body_handle": fixture.dynamic_handle,
            "frame_index": fixture.frame_index,
            "requested_delta": [0.25, 0.0]
        }),
        "missing_preview_action",
    )
    .await;

    let fixture = paused_commit_fixture(&app, "stale-revision", 3).await;
    let preview = preview_fixture_action(&app, &fixture, "stale-revision").await;
    let mut request = commit_request_from_preview(&preview);
    request["world_revision"] = json!(fixture.world_revision.saturating_sub(1));
    assert_commit_rejection(&app, &fixture.session_id, request, "stale_world_revision").await;
    assert_eq!(
        post_commit(
            &app,
            &fixture.session_id,
            commit_request_from_preview(&preview)
        )
        .await["commit"]["accepted"],
        true,
        "a rejected stale revision must not consume the cached preview"
    );

    let fixture = paused_commit_fixture(&app, "stale-epoch", 3).await;
    let preview = preview_fixture_action(&app, &fixture, "stale-epoch").await;
    let mut request = commit_request_from_preview(&preview);
    request["session_epoch"] = json!(1);
    assert_commit_rejection(&app, &fixture.session_id, request, "stale_session_epoch").await;

    let fixture = paused_commit_fixture(&app, "stale-frame", 3).await;
    let preview = preview_fixture_action(&app, &fixture, "stale-frame").await;
    let mut request = commit_request_from_preview(&preview);
    request["frame_index"] = json!(fixture.frame_index + 1);
    assert_commit_rejection(&app, &fixture.session_id, request, "stale_frame").await;

    let fixture = paused_commit_fixture(&app, "stale-handle", 3).await;
    let preview = preview_fixture_action(&app, &fixture, "stale-handle").await;
    let mut request = commit_request_from_preview(&preview);
    request["body_handle"] = json!(18446744073709551615u64);
    assert_commit_rejection(&app, &fixture.session_id, request, "invalid_body_handle").await;

    let fixture = paused_commit_fixture(&app, "static-body", 3).await;
    assert_commit_rejection(
        &app,
        &fixture.session_id,
        json!({
            "action_id": "static-body",
            "world_revision": fixture.world_revision,
            "session_epoch": 0,
            "body_handle": fixture.static_handle,
            "frame_index": fixture.frame_index,
            "requested_delta": [0.25, 0.0]
        }),
        "static_body",
    )
    .await;

    let fixture = paused_commit_fixture(&app, "invalid-delta", 3).await;
    let preview = preview_fixture_action(&app, &fixture, "invalid-delta").await;
    let mut request = commit_request_from_preview(&preview);
    request["requested_delta"] = json!([0.0, 0.0]);
    assert_commit_rejection(&app, &fixture.session_id, request, "invalid_velocity_delta").await;

    let fixture = paused_commit_fixture(&app, "tampered-delta", 3).await;
    let preview = preview_fixture_action(&app, &fixture, "tampered-delta").await;
    let mut request = commit_request_from_preview(&preview);
    request["requested_delta"] = json!([0.5, 0.0]);
    assert_commit_rejection(&app, &fixture.session_id, request, "stale_preview_action").await;
    assert_eq!(
        post_commit(
            &app,
            &fixture.session_id,
            commit_request_from_preview(&preview)
        )
        .await["commit"]["accepted"],
        true,
        "a rejected tampered delta must not consume the cached preview"
    );

    let fixture = paused_commit_fixture(&app, "tampered-target", 3).await;
    let preview = preview_fixture_action(&app, &fixture, "tampered-target").await;
    let mut request = commit_request_from_preview(&preview);
    request["computed_target_velocity"] = json!({ "x": 999.0, "y": 999.0 });
    assert_commit_rejection(&app, &fixture.session_id, request, "stale_preview_action").await;
    assert_eq!(
        post_commit(
            &app,
            &fixture.session_id,
            commit_request_from_preview(&preview)
        )
        .await["commit"]["accepted"],
        true,
        "a rejected tampered target must not consume the cached preview"
    );

    let running = paused_commit_fixture(&app, "running", 3).await;
    let preview = preview_fixture_action(&app, &running, "running").await;
    let _ = control_session(&app, &running.session_id, "play").await;
    assert_commit_rejection(
        &app,
        &running.session_id,
        commit_request_from_preview(&preview),
        "session_running",
    )
    .await;

    let completed = paused_commit_fixture(&app, "completed", 1).await;
    assert_eq!(
        fetch_session(&app, &completed.session_id).await["status"],
        "completed"
    );
    assert_commit_rejection(
        &app,
        &completed.session_id,
        json!({
            "action_id": "completed",
            "world_revision": completed.world_revision,
            "session_epoch": 0,
            "body_handle": completed.dynamic_handle,
            "frame_index": completed.frame_index,
            "requested_delta": [0.25, 0.0]
        }),
        "session_completed",
    )
    .await;
}

#[tokio::test]
async fn live_session_epoch_starts_at_zero_and_reset_increments_without_preview_increment() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created_body = create_live_session(&app, "falling_box_contact", 2).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    assert_eq!(created_body["session"]["session_epoch"], 0);

    let first_step = control_session(&app, &session_id, "step").await;
    let first_step_session = &first_step["session"];
    let body_handle = first_body_with_type(first_step_session, "dynamic")["handle"].clone();
    let stale_revision = first_step_session["latest_frame"]["snapshot"]["meta"]["revision"]
        .as_u64()
        .expect("world revision should be numeric");

    let reset_body = control_session(&app, &session_id, "reset").await;
    assert_eq!(reset_body["session"]["session_epoch"], 1);
    assert_eq!(reset_body["session"]["status"], "created");
    assert_eq!(reset_body["session"]["latest_frame"], Value::Null);
    drain_session_events(&app, &session_id).await;

    let current_revision = assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "created-revision-probe",
            "world_revision": stale_revision,
            "session_epoch": 1,
            "body_handle": body_handle,
            "frame_index": 0,
            "requested_delta": [0.25, 0.0]
        }),
        "stale_world_revision",
    )
    .await;
    let current_revision = current_revision["preview"]["world_revision"].clone();

    assert_preview_rejection(
        &app,
        &session_id,
        json!({
            "action_id": "created-preview",
            "world_revision": current_revision,
            "session_epoch": 1,
            "body_handle": body_handle,
            "frame_index": 0,
            "requested_delta": [0.25, 0.0]
        }),
        "missing_frame_snapshot",
    )
    .await;

    let fetched = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let fetched_session = json_body(fetched).await["session"].clone();
    assert_eq!(fetched_session["session_epoch"], 1);
    assert_eq!(fetched_session["status"], "created");
    assert_eq!(fetched_session["buffered_frame_count"], 0);
    assert_eq!(fetched_session["latest_frame"], Value::Null);

    let step_after_rejected_preview = control_session(&app, &session_id, "step").await;
    assert_eq!(step_after_rejected_preview["session"]["status"], "paused");
    assert_eq!(step_after_rejected_preview["session"]["session_epoch"], 1);
    assert_eq!(
        step_after_rejected_preview["session"]["current_frame_index"],
        0
    );
    assert_eq!(
        step_after_rejected_preview["session"]["buffered_frame_count"],
        1
    );
    assert_eq!(
        step_after_rejected_preview["session"]["latest_frame"]["frame_index"],
        0
    );
}

async fn create_live_session(app: &axum::Router, scenario_id: &str, frame_count: usize) -> Value {
    create_live_session_with_options(app, scenario_id, frame_count, false, 600).await
}

async fn create_live_session_with_options(
    app: &axum::Router,
    scenario_id: &str,
    frame_count: usize,
    live_unbounded: bool,
    live_buffer_capacity: usize,
) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "scenario_id": scenario_id,
                        "frame_count": frame_count,
                        "mode": "live_session",
                        "live_unbounded": live_unbounded,
                        "live_buffer_capacity": live_buffer_capacity
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await
}

async fn control_session(app: &axum::Router, session_id: &str, action: &str) -> Value {
    control_session_with_detail(app, session_id, action, Some("full")).await
}

async fn control_session_without_detail(
    app: &axum::Router,
    session_id: &str,
    action: &str,
) -> Value {
    control_session_with_detail(app, session_id, action, None).await
}

async fn control_session_with_detail(
    app: &axum::Router,
    session_id: &str,
    action: &str,
    detail: Option<&str>,
) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    match detail {
                        Some(detail) => json!({ "action": action, "detail": detail }),
                        None => json!({ "action": action }),
                    }
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn patch_live_gravity(
    app: &axum::Router,
    session_id: &str,
    session_epoch: u64,
    gravity: Value,
) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/gravity"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "session_epoch": session_epoch,
                        "gravity": gravity
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn fetch_live_frame(app: &axum::Router, session_id: &str, frame_index: usize) -> Value {
    let response = fetch_live_frame_response(app, session_id, frame_index).await;
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn fetch_live_frame_response(
    app: &axum::Router,
    session_id: &str,
    frame_index: usize,
) -> axum::response::Response {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}/frames/{frame_index}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    response
}

async fn assert_preview_rejection(
    app: &axum::Router,
    session_id: &str,
    request_body: Value,
    expected_reason: &str,
) -> Value {
    let before = fetch_session(app, session_id).await;
    let body = post_preview(app, session_id, request_body).await;
    assert_eq!(
        body["preview"]["rejection_reason"], expected_reason,
        "preview rejection body was {body:#?}"
    );
    let after = fetch_session(app, session_id).await;
    assert_preview_rejection_preserved_session(&before, &after);
    let events = drain_session_events(app, session_id).await;
    assert!(
        events.contains("event: idle") && !events.contains("preview") && !events.contains("frame"),
        "rejected preview should not emit live events, got {events:?}"
    );
    body
}

async fn post_preview(app: &axum::Router, session_id: &str, request_body: Value) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/sessions/{session_id}/velocity-perturbations/preview"
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn post_commit(app: &axum::Router, session_id: &str, request_body: Value) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/sessions/{session_id}/velocity-perturbations/commit"
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn fetch_session(app: &axum::Router, session_id: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await["session"].clone()
}

async fn assert_commit_rejection(
    app: &axum::Router,
    session_id: &str,
    request_body: Value,
    expected_reason: &str,
) -> Value {
    let before = fetch_session(app, session_id).await;
    let body = post_commit(app, session_id, request_body).await;
    assert_eq!(
        body["commit"]["accepted"], false,
        "commit rejection body was {body:#?}"
    );
    assert_eq!(
        body["commit"]["rejection_reason"], expected_reason,
        "commit rejection body was {body:#?}"
    );
    let after = fetch_session(app, session_id).await;
    assert_commit_rejection_preserved_session(&before, &after);
    let events = drain_session_events(app, session_id).await;
    assert!(
        events.contains("event: idle") && !events.contains("frame"),
        "rejected commit should not emit live events, got {events:?}"
    );
    body
}

async fn drain_session_events(app: &axum::Router, session_id: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/sessions/{session_id}/events"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    body_text(response).await
}

struct CommitFixture {
    session_id: String,
    world_revision: u64,
    frame_index: u64,
    dynamic_handle: Value,
    static_handle: Value,
}

async fn paused_commit_fixture(
    app: &axum::Router,
    _action_id: &str,
    frame_count: usize,
) -> CommitFixture {
    let created = create_live_session(app, "falling_box_contact", frame_count).await;
    let session_id = created["session"]["id"].as_str().unwrap().to_owned();
    let stepped = control_session(app, &session_id, "step").await;
    drain_session_events(app, &session_id).await;
    let session = &stepped["session"];
    let world_revision = session["latest_frame"]["snapshot"]["meta"]["revision"]
        .as_u64()
        .expect("world revision should be numeric");
    let frame_index = session["current_frame_index"]
        .as_u64()
        .expect("frame index should be numeric");
    CommitFixture {
        session_id,
        world_revision,
        frame_index,
        dynamic_handle: first_body_with_type(session, "dynamic")["handle"].clone(),
        static_handle: first_body_with_type(session, "static")["handle"].clone(),
    }
}

async fn preview_fixture_action(
    app: &axum::Router,
    fixture: &CommitFixture,
    action_id: &str,
) -> Value {
    let body = post_preview(
        app,
        &fixture.session_id,
        json!({
            "action_id": action_id,
            "world_revision": fixture.world_revision,
            "session_epoch": 0,
            "body_handle": fixture.dynamic_handle,
            "frame_index": fixture.frame_index,
            "requested_delta": [0.25, 0.0],
            "wake_intent": true
        }),
    )
    .await;
    assert_eq!(body["preview"]["rejection_reason"], Value::Null);
    body["preview"].clone()
}

fn commit_request_from_preview(preview: &Value) -> Value {
    json!({
        "action_id": preview["action_id"],
        "world_revision": preview["world_revision"],
        "session_epoch": preview["session_epoch"],
        "body_handle": preview["body_handle"],
        "frame_index": preview["frame_index"],
        "requested_delta": preview["requested_delta"],
        "computed_target_velocity": preview["computed_target_velocity"]
    })
}

fn assert_preview_rejection_preserved_session(before: &Value, after: &Value) {
    for field in [
        "status",
        "session_epoch",
        "latest_frame",
        "buffered_frame_count",
        "current_frame_index",
        "run_id",
    ] {
        assert_eq!(
            after[field], before[field],
            "rejected preview changed session field {field}"
        );
    }
}

fn assert_commit_rejection_preserved_session(before: &Value, after: &Value) {
    assert_preview_rejection_preserved_session(before, after);
}

fn first_body_with_type<'a>(session: &'a Value, body_type: &str) -> &'a Value {
    session["latest_frame"]["snapshot"]["bodies"]
        .as_array()
        .expect("snapshot bodies should be an array")
        .iter()
        .find(|body| body["body_type"] == body_type)
        .unwrap_or_else(|| panic!("snapshot should include a {body_type} body"))
}

fn body_by_handle<'a>(snapshot: &'a Value, handle: &Value) -> &'a Value {
    snapshot["bodies"]
        .as_array()
        .expect("snapshot bodies should be an array")
        .iter()
        .find(|body| body["handle"] == *handle)
        .unwrap_or_else(|| panic!("snapshot should include body handle {handle}"))
}

fn vector_x(value: &Value) -> f64 {
    value["x"].as_f64().expect("vector x should be numeric")
}

fn vector_y(value: &Value) -> f64 {
    value["y"].as_f64().expect("vector y should be numeric")
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-5,
        "expected {actual} to be close to {expected}"
    );
}

fn assert_live_frame_uses_current_full_payload(frame: &Value, context: &str) {
    for field in ["snapshot", "report", "events", "diagnostics"] {
        assert!(
            frame.get(field).is_some(),
            "{context}: expected current live frame payload to include {field}"
        );
    }
}

async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_str(&body_text(response).await).expect("response body should be JSON")
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    String::from_utf8(bytes.to_vec()).expect("body should be UTF-8")
}
