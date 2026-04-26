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
    assert_eq!(created_body["session"]["status"], "completed");
    assert_eq!(
        serde_json::from_value::<SessionStatus>(created_body["session"]["status"].clone()).unwrap(),
        SessionStatus::Completed
    );
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    let run_id = created_body["session"]["run_id"]
        .as_str()
        .unwrap()
        .to_owned();

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
        events_body.contains("event: frame") || events_body.contains("event: failed"),
        "SSE endpoint should emit at least a frame or failed event, got {events_body:?}"
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
