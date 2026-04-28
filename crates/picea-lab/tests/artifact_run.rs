use std::{fs, path::Path};

use picea::events::CcdTargetKind;
use picea_lab::{
    run_scenario, ArtifactFile, ArtifactStore, DebugRenderArtifact, DebugRenderFrame, FrameRecord,
    RunConfig, RunManifest, ScenarioId,
};

#[test]
fn default_artifact_store_uses_workspace_target() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("picea-lab should live under workspace/crates/picea-lab");

    assert_eq!(
        ArtifactStore::default_in_workspace().root(),
        workspace_root.join("target/picea-lab/runs").as_path(),
        "the default store should not depend on whether picea-lab is launched from the workspace root or the crate directory"
    );
}

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

#[test]
fn warm_start_artifacts_capture_per_step_manifold_cache_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::SatPolygon,
            frame_count: 2,
            run_id: Some("warm-start-facts".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("warm-start run should write artifacts");

    let first = run.frames.first().expect("first frame should exist");
    let second = run.frames.get(1).expect("second frame should exist");
    assert_eq!(first.stats.warm_start_miss_count, first.stats.contact_count);
    assert_eq!(first.stats.warm_start_hit_count, 0);
    assert_eq!(
        second.stats.warm_start_hit_count,
        second.stats.contact_count
    );
    assert_eq!(second.stats.warm_start_miss_count, 0);
    assert_eq!(second.stats.warm_start_drop_count, 0);

    let first_manifold = first
        .snapshot
        .manifolds
        .first()
        .expect("first frame should expose a manifold");
    let second_manifold = second
        .snapshot
        .manifolds
        .first()
        .expect("second frame should expose the persisted manifold");
    assert_eq!(first_manifold.id, second_manifold.id);
    assert_eq!(first_manifold.points.len(), second_manifold.points.len());
    assert_eq!(
        second
            .snapshot
            .contacts
            .iter()
            .filter(|contact| contact.warm_start_reason == picea::events::WarmStartCacheReason::Hit)
            .count(),
        second.stats.contact_count
    );

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let render_second = render
        .frames
        .get(1)
        .expect("debug render should include warm-start frame facts");
    assert_eq!(
        render_second.warm_start_hit_count,
        second.stats.warm_start_hit_count
    );
    assert_eq!(
        render_second.manifolds[0].warm_start_hit_count,
        second.stats.contact_count
    );
    assert!(
        render_second
            .unmeasured
            .iter()
            .all(|fact| fact != "contact_impulses"),
        "M5 artifacts should stop marking contact impulses as unmeasured"
    );
}

#[test]
fn stack_artifacts_capture_solver_impulse_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::Stack4,
            frame_count: 20,
            run_id: Some("m5-stack-impulses".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("stack run should write artifacts");
    let debug_render_json =
        fs::read_to_string(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable");
    for field in [
        "solver_normal_impulse",
        "solver_tangent_impulse",
        "normal_impulse_clamped",
        "tangent_impulse_clamped",
        "restitution_velocity_threshold",
        "restitution_applied",
        "islands",
        "reason",
    ] {
        assert!(
            debug_render_json.contains(field),
            "debug render JSON should export M5 solver field `{field}`"
        );
    }
    let render: DebugRenderArtifact =
        serde_json::from_str(&debug_render_json).expect("debug render should match schema");

    let solver_contact = render
        .frames
        .iter()
        .flat_map(|frame| frame.contacts.iter())
        .find(|contact| contact.solver_normal_impulse > 0.0)
        .expect("stack artifacts should include non-zero contact solver impulses");
    assert!(
        solver_contact.solver_tangent_impulse.abs()
            <= solver_contact.solver_normal_impulse + 1.0e-4,
        "solver tangent impulse should stay bounded by the normal impulse in artifacts"
    );
    assert_eq!(solver_contact.restitution_velocity_threshold, 1.0);
    assert!(
        !solver_contact.restitution_applied,
        "default stack contacts should not bounce without restitution material"
    );
    assert!(
        render
            .frames
            .iter()
            .flat_map(|frame| frame.bodies.iter())
            .any(|body| !body.sleeping),
        "stack artifacts should expose body sleep state for M5/M6 inspection"
    );
    assert!(
        render.frames.iter().any(|frame| !frame.islands.is_empty()),
        "M6 stack artifacts should label sleep islands for inspection"
    );
    let island_frame = render
        .frames
        .iter()
        .find(|frame| !frame.islands.is_empty())
        .expect("at least one frame should expose islands");
    for island in &island_frame.islands {
        for body in &island.bodies {
            let body_fact = island_frame
                .bodies
                .iter()
                .find(|candidate| candidate.handle == *body)
                .expect("island member should have a body fact");
            assert_eq!(
                body_fact.island_id,
                Some(island.id),
                "body island_id should match its island label"
            );
        }
    }
    assert!(
        render.frames.iter().all(|frame| frame
            .unmeasured
            .iter()
            .all(|fact| fact != "contact_impulses")),
        "M5 stack artifacts should stop marking contact impulses as unmeasured"
    );
}

#[test]
fn ccd_fast_circle_wall_artifacts_capture_toi_trace_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::CcdFastCircleWall,
            frame_count: 2,
            run_id: Some("m8-ccd-fast-circle-wall".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("ccd run should write artifacts");

    let first = run.frames.first().expect("first frame should exist");
    assert_eq!(first.stats.ccd_candidate_count, 1);
    assert_eq!(first.stats.ccd_hit_count, 1);
    assert_eq!(first.stats.ccd_miss_count, 0);
    assert_eq!(first.stats.ccd_clamp_count, 1);
    let contact = first
        .snapshot
        .contacts
        .iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("ccd artifact should expose a contact trace");
    let trace = contact.ccd_trace.expect("trace should be present");
    assert!(trace.toi > 0.0 && trace.toi < 1.0);
    assert!(trace.advancement >= trace.toi && trace.advancement <= 1.0);
    assert!(trace.clamp > 0.0);
    assert!(trace.slop > 0.0);

    let frame_lines = fs::read_to_string(run.path.join(ArtifactFile::Frames.file_name()))
        .expect("frames should be readable");
    assert!(
        frame_lines.contains("\"ccd_trace\""),
        "frames.jsonl should preserve CCD trace facts"
    );

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let render_first = render
        .frames
        .first()
        .expect("debug render should include ccd frame facts");
    assert_eq!(render_first.ccd_candidate_count, 1);
    assert_eq!(render_first.ccd_hit_count, 1);
    assert_eq!(render_first.ccd_miss_count, 0);
    assert_eq!(render_first.ccd_clamp_count, 1);
    assert!(
        render_first
            .contacts
            .iter()
            .any(|contact| contact.ccd_trace.is_some()),
        "debug render should keep CCD trace facts for the selected-contact inspector"
    );
}

#[test]
fn ccd_fast_convex_walls_artifacts_capture_ordered_budget_trace_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::CcdFastConvexWalls,
            frame_count: 2,
            run_id: Some("m13-ccd-fast-convex-walls".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("ccd convex run should write artifacts");

    let first = run.frames.first().expect("first frame should exist");
    assert_eq!(first.stats.ccd_candidate_count, 2);
    assert_eq!(first.stats.ccd_hit_count, 2);
    assert_eq!(first.stats.ccd_miss_count, 0);
    assert_eq!(first.stats.ccd_clamp_count, 1);
    let contact = first
        .snapshot
        .contacts
        .iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("ccd artifact should expose the selected convex contact trace");
    let trace = contact.ccd_trace.expect("trace should be present");
    assert!(trace.toi > 0.0 && trace.toi < 1.0);
    assert!(trace.advancement >= trace.toi && trace.advancement <= 1.0);
    assert!(trace.clamp > 0.0);
    assert!((trace.toi_point.x() + 0.05).abs() < 1.0e-3);
    assert!(trace.toi_point.y().abs() < 1.0e-3);

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let render_first = render
        .frames
        .first()
        .expect("debug render should include ccd frame facts");
    assert_eq!(render_first.ccd_candidate_count, 2);
    assert_eq!(render_first.ccd_hit_count, 2);
    assert_eq!(render_first.ccd_miss_count, 0);
    assert_eq!(render_first.ccd_clamp_count, 1);
    assert!(
        render_first
            .contacts
            .iter()
            .any(|contact| contact.ccd_trace == Some(trace)),
        "debug render should keep the selected CCD trace while counters expose the ignored budget hit"
    );
}

#[test]
fn ccd_dynamic_convex_pair_artifacts_capture_dynamic_target_trace_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::CcdDynamicConvexPair,
            frame_count: 2,
            run_id: Some("m19-ccd-dynamic-convex-pair".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("dynamic CCD run should write artifacts");

    let first = run.frames.first().expect("first frame should exist");
    assert_eq!(first.stats.ccd_candidate_count, 1);
    assert_eq!(first.stats.ccd_hit_count, 1);
    assert_eq!(first.stats.ccd_miss_count, 0);
    assert_eq!(first.stats.ccd_clamp_count, 2);
    let contact = first
        .snapshot
        .contacts
        .iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("dynamic CCD artifact should expose the selected contact trace");
    let trace = contact.ccd_trace.expect("trace should be present");
    assert_eq!(trace.target_kind, CcdTargetKind::Dynamic);
    assert!(trace.target_clamp > 0.0);
    assert_ne!(trace.target_swept_start, trace.target_swept_end);
    assert!(trace.toi > 0.0 && trace.toi < 1.0);
    assert!(trace.advancement >= trace.toi && trace.advancement <= 1.0);

    let frame_lines = fs::read_to_string(run.path.join(ArtifactFile::Frames.file_name()))
        .expect("frames should be readable");
    assert!(
        frame_lines.contains("\"target_kind\":\"dynamic\"")
            && frame_lines.contains("\"target_clamp\""),
        "frames.jsonl should preserve dynamic-target CCD trace facts"
    );

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let render_first = render
        .frames
        .first()
        .expect("debug render should include ccd frame facts");
    assert_eq!(render_first.ccd_candidate_count, 1);
    assert_eq!(render_first.ccd_hit_count, 1);
    assert_eq!(render_first.ccd_miss_count, 0);
    assert_eq!(render_first.ccd_clamp_count, 2);
    assert!(
        render_first
            .contacts
            .iter()
            .any(|contact| contact.ccd_trace == Some(trace)),
        "debug render should keep dynamic-target CCD trace facts"
    );
}

#[test]
fn artifact_schema_keeps_final_observability_fact_set() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::CcdFastCircleWall,
            frame_count: 1,
            run_id: Some("m9-observability-schema".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("schema run should write artifacts");

    let final_snapshot_bytes = fs::read(run.path.join(ArtifactFile::FinalSnapshot.file_name()))
        .expect("final snapshot should be readable");
    let final_snapshot_json: serde_json::Value = serde_json::from_slice(&final_snapshot_bytes)
        .expect("final snapshot should match JSON schema");
    let final_stats = final_snapshot_json
        .get("stats")
        .and_then(serde_json::Value::as_object)
        .expect("final snapshot should carry a stats object");
    for field in [
        "broadphase_candidate_count",
        "broadphase_traversal_count",
        "broadphase_pruned_count",
        "broadphase_tree_depth",
        "contact_count",
        "manifold_count",
        "island_count",
        "active_island_count",
        "sleeping_island_skip_count",
        "solver_body_slot_count",
        "contact_row_count",
        "joint_row_count",
        "warm_start_hit_count",
        "warm_start_miss_count",
        "warm_start_drop_count",
        "ccd_candidate_count",
        "ccd_hit_count",
        "ccd_miss_count",
        "ccd_clamp_count",
    ] {
        assert!(
            final_stats.contains_key(field),
            "final snapshot stats should preserve observability field `{field}`"
        );
    }
    assert!(
        final_snapshot_json
            .get("contacts")
            .and_then(serde_json::Value::as_array)
            .is_some(),
        "final snapshot should preserve contact carriers"
    );
    assert!(
        final_snapshot_json
            .get("manifolds")
            .and_then(serde_json::Value::as_array)
            .is_some(),
        "final snapshot should preserve manifold carriers"
    );

    let debug_render_bytes = fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
        .expect("debug render should be readable");
    let debug_render_json: serde_json::Value =
        serde_json::from_slice(&debug_render_bytes).expect("debug render should match JSON schema");
    let frames = debug_render_json
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .expect("debug render should carry frame objects");
    let first_frame = frames
        .first()
        .and_then(serde_json::Value::as_object)
        .expect("debug render should include the first frame object");
    for field in [
        "broadphase_candidate_count",
        "broadphase_traversal_count",
        "broadphase_pruned_count",
        "broadphase_tree_depth",
        "contact_count",
        "contacts",
        "manifolds",
        "island_count",
        "active_island_count",
        "sleeping_island_skip_count",
        "solver_body_slot_count",
        "contact_row_count",
        "joint_row_count",
        "warm_start_hit_count",
        "warm_start_miss_count",
        "warm_start_drop_count",
        "ccd_candidate_count",
        "ccd_hit_count",
        "ccd_miss_count",
        "ccd_clamp_count",
        "islands",
    ] {
        assert!(
            first_frame.contains_key(field),
            "debug render frame should preserve observability field `{field}`"
        );
    }

    let bodies = first_frame
        .get("bodies")
        .and_then(serde_json::Value::as_array)
        .expect("debug render frame should carry body facts");
    let first_body = bodies
        .first()
        .and_then(serde_json::Value::as_object)
        .expect("debug render frame should include at least one body fact");
    assert!(
        first_body.contains_key("sleeping") && first_body.contains_key("island_id"),
        "body facts should preserve sleep/island carriers"
    );

    let contacts = first_frame
        .get("contacts")
        .and_then(serde_json::Value::as_array)
        .expect("debug render frame should carry contact facts");
    assert!(
        contacts.iter().any(|contact| contact
            .get("ccd_trace")
            .is_some_and(|trace| !trace.is_null())),
        "contact facts should preserve CCD trace carriers"
    );

    let render: DebugRenderArtifact = serde_json::from_slice(&debug_render_bytes)
        .expect("debug render should deserialize through the typed artifact schema");
    let render_first = render
        .frames
        .first()
        .expect("typed debug render should include first frame");
    assert_eq!(render_first.ccd_hit_count, 1);
    assert!(
        render_first
            .contacts
            .iter()
            .any(|contact| contact.ccd_trace.is_some()),
        "typed debug render contacts should preserve CCD trace facts"
    );

    let perf_bytes = fs::read(run.path.join(ArtifactFile::Perf.file_name()))
        .expect("perf artifact should be readable");
    let perf_json: serde_json::Value =
        serde_json::from_slice(&perf_bytes).expect("perf artifact should match JSON schema");
    let counters = perf_json
        .get("counter_summary")
        .and_then(serde_json::Value::as_object)
        .expect("perf artifact should carry deterministic counter summary");
    for field in [
        "total_broadphase_traversal_count",
        "total_broadphase_pruned_count",
        "total_contact_row_count",
        "total_joint_row_count",
        "total_solver_body_slot_count",
        "total_island_count",
        "total_active_island_count",
        "total_sleeping_island_skip_count",
    ] {
        assert!(
            counters.contains_key(field),
            "perf counter summary should preserve `{field}`"
        );
    }
}

#[test]
fn warm_start_debug_render_frame_fields_default_when_deserializing_older_json() {
    let frame = DebugRenderFrame {
        frame_index: 0,
        body_count: 0,
        collider_count: 0,
        broadphase_candidate_count: 0,
        broadphase_update_count: 0,
        broadphase_stale_proxy_drop_count: 0,
        broadphase_same_body_drop_count: 0,
        broadphase_filter_drop_count: 0,
        broadphase_narrowphase_drop_count: 0,
        broadphase_traversal_count: 0,
        broadphase_pruned_count: 0,
        broadphase_rebuild_count: 0,
        broadphase_tree_depth: 0,
        contact_count: 0,
        island_count: 0,
        active_island_count: 0,
        sleeping_island_skip_count: 0,
        solver_body_slot_count: 0,
        contact_row_count: 0,
        joint_row_count: 0,
        warm_start_hit_count: 1,
        warm_start_miss_count: 2,
        warm_start_drop_count: 3,
        ccd_candidate_count: 4,
        ccd_hit_count: 2,
        ccd_miss_count: 1,
        ccd_clamp_count: 2,
        world_bounds: None,
        bodies: Vec::new(),
        colliders: Vec::new(),
        contacts: Vec::new(),
        manifolds: Vec::new(),
        islands: Vec::new(),
        unmeasured: Vec::new(),
    };
    let mut value = serde_json::to_value(frame).expect("debug render frame should serialize");
    let object = value
        .as_object_mut()
        .expect("debug render frame should serialize as an object");
    object.remove("warm_start_hit_count");
    object.remove("warm_start_miss_count");
    object.remove("warm_start_drop_count");
    object.remove("broadphase_traversal_count");
    object.remove("broadphase_pruned_count");
    object.remove("island_count");
    object.remove("active_island_count");
    object.remove("sleeping_island_skip_count");
    object.remove("solver_body_slot_count");
    object.remove("contact_row_count");
    object.remove("joint_row_count");
    object.remove("ccd_candidate_count");
    object.remove("ccd_hit_count");
    object.remove("ccd_miss_count");
    object.remove("ccd_clamp_count");
    object.remove("islands");

    let decoded: DebugRenderFrame =
        serde_json::from_value(value).expect("older debug render frame should deserialize");

    assert_eq!(decoded.warm_start_hit_count, 0);
    assert_eq!(decoded.warm_start_miss_count, 0);
    assert_eq!(decoded.warm_start_drop_count, 0);
    assert_eq!(decoded.broadphase_traversal_count, 0);
    assert_eq!(decoded.broadphase_pruned_count, 0);
    assert_eq!(decoded.island_count, 0);
    assert_eq!(decoded.active_island_count, 0);
    assert_eq!(decoded.sleeping_island_skip_count, 0);
    assert_eq!(decoded.solver_body_slot_count, 0);
    assert_eq!(decoded.contact_row_count, 0);
    assert_eq!(decoded.joint_row_count, 0);
    assert_eq!(decoded.ccd_candidate_count, 0);
    assert_eq!(decoded.ccd_hit_count, 0);
    assert_eq!(decoded.ccd_miss_count, 0);
    assert_eq!(decoded.ccd_clamp_count, 0);
    assert!(decoded.islands.is_empty());
}
