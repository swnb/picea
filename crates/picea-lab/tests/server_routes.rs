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
            "joint_anchor",
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
                .body(Body::from(json!({ "action": "step" }).to_string()))
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
                .body(Body::from(json!({ "action": "step" }).to_string()))
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
                .body(Body::from(json!({ "action": "step" }).to_string()))
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
                .body(Body::from(json!({ "action": "step" }).to_string()))
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
                .body(Body::from(json!({ "action": "step" }).to_string()))
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
                .body(Body::from(json!({ "action": "step" }).to_string()))
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

async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_str(&body_text(response).await).expect("response body should be JSON")
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    String::from_utf8(bytes.to_vec()).expect("body should be UTF-8")
}
