use std::fs;

use picea_lab::{
    run_scenario, ArtifactFile, ArtifactStore, DebugRenderArtifact, FrameRecord, RunConfig,
    RunManifest, ScenarioId,
};

#[test]
fn run_writes_expected_artifacts_and_keeps_state_hash_deterministic() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let first = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::FallingBoxContact,
            frame_count: 8,
            run_id: Some("determinism-a".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("first run should write artifacts");
    let second = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::FallingBoxContact,
            frame_count: 8,
            run_id: Some("determinism-b".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("second run should write artifacts");

    assert_eq!(
        first.manifest.final_state_hash, second.manifest.final_state_hash,
        "same scenario and fixed step count should produce the same final state hash"
    );
    assert_ne!(first.manifest.run_id, second.manifest.run_id);

    for artifact in [
        ArtifactFile::Manifest,
        ArtifactFile::Frames,
        ArtifactFile::DebugRender,
        ArtifactFile::FinalSnapshot,
        ArtifactFile::Perf,
    ] {
        assert!(
            first.path.join(artifact.file_name()).is_file(),
            "{} should exist",
            artifact.file_name()
        );
    }

    let manifest: RunManifest = serde_json::from_slice(
        &fs::read(first.path.join(ArtifactFile::Manifest.file_name()))
            .expect("manifest should be readable"),
    )
    .expect("manifest should match schema");
    assert_eq!(manifest.scenario_id, ScenarioId::FallingBoxContact);
    assert_eq!(manifest.frame_count, 8);
    assert_eq!(manifest.artifacts.len(), 5);

    let frames = fs::read_to_string(first.path.join(ArtifactFile::Frames.file_name()))
        .expect("frames should be readable");
    assert_eq!(frames.lines().count(), 8);
    assert!(
        frames.lines().all(|line| line.contains("\"state_hash\"")),
        "each frame line should carry a deterministic state hash"
    );
    assert!(
        frames
            .lines()
            .all(|line| line.contains("\"events\"") && line.contains("\"stats\"")),
        "each frame line should preserve step events and counters for timeline consumers"
    );

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(first.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let first_frame = render
        .frames
        .first()
        .expect("debug render should include frame facts");
    assert!(
        first_frame.world_bounds.is_some(),
        "viewer needs world bounds for camera framing"
    );
    assert!(
        !first_frame.bodies.is_empty() && !first_frame.colliders.is_empty(),
        "viewer render facts should include body and collider layers"
    );
    assert!(
        first
            .frames
            .iter()
            .flat_map(|frame| frame.snapshot.bodies.iter())
            .any(|body| body.mass_properties.mass > 0.0),
        "artifacts should carry density-derived body mass properties"
    );
    assert!(
        first_frame
            .unmeasured
            .iter()
            .all(|fact| fact != "broadphase_candidates"),
        "broadphase counters should be measured in M1 artifacts"
    );
}

#[test]
fn broadphase_scenario_artifacts_capture_candidate_and_tree_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::BroadphaseSparse,
            frame_count: 2,
            run_id: Some("broadphase-facts".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("broadphase run should write artifacts");

    let first = run.frames.first().expect("first frame should exist");
    assert_eq!(first.stats.broadphase_candidate_count, 1);
    assert_eq!(first.stats.broadphase_update_count, 5);
    assert_eq!(first.stats.broadphase_stale_proxy_drop_count, 0);
    assert_eq!(first.stats.broadphase_same_body_drop_count, 0);
    assert_eq!(first.stats.broadphase_filter_drop_count, 0);
    assert_eq!(first.stats.broadphase_narrowphase_drop_count, 0);
    assert!(first.stats.broadphase_tree_depth > 0);
    assert_eq!(
        first.snapshot.stats.broadphase_candidate_count,
        first.stats.broadphase_candidate_count
    );
    assert_eq!(
        first.snapshot.stats.broadphase_tree_depth,
        first.stats.broadphase_tree_depth
    );

    let second = run.frames.get(1).expect("second frame should exist");
    assert_eq!(second.stats.broadphase_candidate_count, 1);
    assert_eq!(second.stats.broadphase_update_count, 0);
    assert_eq!(second.stats.broadphase_stale_proxy_drop_count, 0);
    assert_eq!(second.stats.broadphase_same_body_drop_count, 0);
    assert_eq!(second.stats.broadphase_filter_drop_count, 0);
    assert_eq!(second.stats.broadphase_narrowphase_drop_count, 0);

    let frame_lines = fs::read_to_string(run.path.join(ArtifactFile::Frames.file_name()))
        .expect("frames should be readable");
    let decoded_first: FrameRecord = serde_json::from_str(
        frame_lines
            .lines()
            .next()
            .expect("frames should include the first line"),
    )
    .expect("frame line should match schema");
    assert_eq!(decoded_first.stats.broadphase_candidate_count, 1);

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let render_first = render
        .frames
        .first()
        .expect("debug render should include broadphase frame facts");
    assert_eq!(render_first.broadphase_candidate_count, 1);
    assert_eq!(render_first.broadphase_stale_proxy_drop_count, 0);
    assert_eq!(render_first.broadphase_filter_drop_count, 0);
    assert_eq!(
        render_first.broadphase_tree_depth,
        first.stats.broadphase_tree_depth
    );
    assert!(
        render_first
            .unmeasured
            .iter()
            .all(|fact| fact != "broadphase_candidates"),
        "debug render should no longer mark broadphase candidates as unmeasured"
    );
}

#[test]
fn sat_polygon_artifacts_capture_manifold_points_and_normals() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::SatPolygon,
            frame_count: 1,
            run_id: Some("sat-polygon-facts".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("sat polygon run should write artifacts");

    let first = run.frames.first().expect("first frame should exist");
    assert_eq!(first.stats.contact_count, 2);
    assert_eq!(first.stats.manifold_count, 1);
    let manifold = first
        .snapshot
        .manifolds
        .first()
        .expect("snapshot should expose one manifold");
    assert_eq!(manifold.points.len(), 2);
    assert_eq!(manifold.normal.x(), -1.0);
    assert_eq!(manifold.normal.y(), 0.0);

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let render_first = render
        .frames
        .first()
        .expect("debug render should include sat frame facts");
    assert_eq!(render_first.contacts.len(), 2);
    assert_eq!(render_first.manifolds.len(), 1);
    assert_eq!(render_first.manifolds[0].points.len(), 2);
}
