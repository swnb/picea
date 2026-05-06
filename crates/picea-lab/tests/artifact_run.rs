use std::{fs, path::Path};

use picea::events::CcdTargetKind;
use picea::prelude::{CollisionLayerPreset, MaterialPreset};
use picea_lab::{
    instantiate_scene_fixture, run_scenario, ArtifactFile, ArtifactStore, DebugRenderArtifact,
    DebugRenderFrame, DiagnosticMarkerKind, DiagnosticSeverity, DiagnosticSource, FrameRecord,
    MissingEvidenceKind, RunConfig, RunManifest, ScenarioId, SceneRecipeFixture,
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
fn stack_stability_tower_artifacts_capture_multi_body_stack_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::StackStabilityTower,
            frame_count: 240,
            run_id: Some("m35-stack-stability".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("stack stability tower run should write artifacts");

    let manifest: RunManifest = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::Manifest.file_name()))
            .expect("manifest should be readable"),
    )
    .expect("manifest should match schema");
    assert_eq!(manifest.scenario_id, ScenarioId::StackStabilityTower);

    let dynamic_bodies = run.frames[0]
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
        .count();
    assert!(
        dynamic_bodies > 4,
        "M35 tower should exercise a denser stack than stack_4"
    );

    assert!(
        run.frames
            .iter()
            .any(|frame| frame.stats.contact_count >= 5),
        "tower should expose multi-contact stack frames"
    );
    assert!(
        run.frames.iter().any(|frame| frame.stats.island_count > 0),
        "tower should expose island facts for the observatory"
    );
    assert!(
        run.frames.iter().any(|frame| {
            frame.stats.contact_row_count > 0 || frame.stats.solver_body_slot_count > 0
        }),
        "tower should carry solver-facing counts without modifying solver behavior"
    );
}

#[test]
fn matrix_stack_artifacts_capture_nxm_grid_stack_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::MatrixStack,
            frame_count: 180,
            run_id: Some("matrix-stack-acceptance".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("matrix stack run should write artifacts");

    assert_eq!(run.manifest.scenario_id, ScenarioId::MatrixStack);

    let first = run
        .frames
        .first()
        .expect("matrix stack should write frames");
    let dynamic_bodies = first
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
        .count();
    assert_eq!(
        dynamic_bodies, 48,
        "default matrix stack should be an 8x6 dynamic-body grid"
    );

    assert!(
        run.frames
            .iter()
            .any(|frame| frame.stats.contact_count >= 12),
        "matrix stack should exercise dense multi-contact frames"
    );
    assert!(
        run.frames
            .iter()
            .any(|frame| frame.stats.contact_row_count >= 12),
        "matrix stack should expose solver-row pressure"
    );
    assert!(
        run.frames
            .iter()
            .any(|frame| !frame.diagnostics.markers.is_empty()),
        "matrix stack should produce diagnostics markers for acceptance triage"
    );
}

#[test]
fn stack_artifacts_capture_lab_frame_diagnostics_with_marker_sources() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::Stack4,
            frame_count: 180,
            run_id: Some("e1-stack-diagnostics".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("stack diagnostics run should write artifacts");
    assert_eq!(run.frames.len(), 180);

    let first = run.frames.first().expect("first frame should exist");
    assert!(
        first
            .diagnostics
            .missing_evidence
            .iter()
            .any(|missing| missing.kind == MissingEvidenceKind::PreviousFrame),
        "first frame should explain that counter/churn deltas need a previous frame"
    );
    assert_eq!(
        first.diagnostics.performance.counter_delta.source,
        DiagnosticSource::Missing
    );
    assert_eq!(
        first.diagnostics.stability.contact_churn.source,
        DiagnosticSource::Missing,
        "first-frame contact churn needs adjacent-frame evidence"
    );
    assert!(first
        .diagnostics
        .stability
        .contact_churn
        .missing_evidence
        .contains(&MissingEvidenceKind::PreviousFrame));
    assert_eq!(
        run.frames[1].diagnostics.stability.contact_churn.source,
        DiagnosticSource::LabDerived
    );
    assert_eq!(
        first.diagnostics.stability.warm_start.counts_source,
        DiagnosticSource::RustAuthoritative
    );
    assert_eq!(
        first.diagnostics.stability.warm_start.drop_reasons_source,
        DiagnosticSource::LabDerived
    );
    assert_eq!(
        first.diagnostics.stability.sleep.transition_count_source,
        DiagnosticSource::RustAuthoritative
    );
    assert_eq!(
        first.diagnostics.stability.island.source,
        DiagnosticSource::RustAuthoritative
    );

    let contact_frame = run
        .frames
        .iter()
        .find(|frame| {
            frame
                .diagnostics
                .stability
                .penetration
                .penetrating_contact_count
                > 0
        })
        .expect("stack run should include a contact diagnostics frame");
    assert_eq!(
        contact_frame.diagnostics.stability.penetration.source,
        DiagnosticSource::LabDerived
    );
    assert!(contact_frame.diagnostics.stability.penetration.max_depth >= 0.0);

    let solver_marker = run
        .frames
        .iter()
        .flat_map(|frame| frame.diagnostics.markers.iter())
        .find(|marker| marker.kind == DiagnosticMarkerKind::SolverRowSpike)
        .expect("stack diagnostics should expose a solver-row spike marker");
    assert_eq!(solver_marker.source, DiagnosticSource::LabDerived);
    assert!(matches!(
        solver_marker.severity,
        DiagnosticSeverity::Warning | DiagnosticSeverity::Severe
    ));
    assert!(
        !solver_marker.evidence_fields.is_empty(),
        "markers should report the fields used to compute them"
    );
    let solver_marker_frame_index = solver_marker.frame_index;

    let frame_lines = fs::read_to_string(run.path.join(ArtifactFile::Frames.file_name()))
        .expect("frames should be readable");
    assert!(frame_lines.contains("\"diagnostics\""));
    assert!(frame_lines.contains("\"source\""));
    assert!(frame_lines.contains("\"counts_source\""));
    assert!(frame_lines.contains("\"transition_count_source\""));

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    assert_eq!(render.frames.len(), run.frames.len());
    assert!(
        render
            .frames
            .iter()
            .any(|frame| !frame.diagnostics.markers.is_empty()),
        "debug_render frames should preserve diagnostics markers for Web consumers"
    );
    let projected = &render.frames[solver_marker_frame_index].diagnostics;
    let source = &run.frames[solver_marker_frame_index].diagnostics;
    assert_eq!(projected.performance, source.performance);
    assert_eq!(
        projected.stability.contact_churn,
        source.stability.contact_churn
    );
    assert_eq!(projected.stability.warm_start, source.stability.warm_start);
    assert_eq!(projected.stability.sleep, source.stability.sleep);
    assert_eq!(projected.stability.island, source.stability.island);
    assert_eq!(
        projected.stability.impulse.source,
        source.stability.impulse.source
    );
    assert_eq!(projected.markers, source.markers);
    assert_eq!(projected.missing_evidence, source.missing_evidence);
    assert!(
        (projected.stability.penetration.max_depth - source.stability.penetration.max_depth).abs()
            < 1.0e-12
    );
    assert!(
        (projected.stability.penetration.total_depth - source.stability.penetration.total_depth)
            .abs()
            < 1.0e-12
    );
    assert_eq!(
        projected.stability.penetration.penetrating_contact_count,
        source.stability.penetration.penetrating_contact_count
    );
    assert!(
        (projected.stability.impulse.total_normal_impulse
            - source.stability.impulse.total_normal_impulse)
            .abs()
            < 1.0e-12
    );
    assert!(
        (projected.stability.impulse.total_tangent_impulse
            - source.stability.impulse.total_tangent_impulse)
            .abs()
            < 1.0e-12
    );
    assert!(
        (projected.stability.impulse.max_normal_impulse
            - source.stability.impulse.max_normal_impulse)
            .abs()
            < 1.0e-12
    );
    assert!(
        (projected.stability.impulse.max_tangent_impulse
            - source.stability.impulse.max_tangent_impulse)
            .abs()
            < 1.0e-12
    );
}

#[test]
fn compound_provenance_scenario_writes_frame_and_debug_render_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::CompoundProvenance,
            frame_count: 2,
            run_id: Some("m24-compound-provenance".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("compound provenance run should write artifacts");

    let first = run.frames.first().expect("first frame should exist");
    assert_eq!(first.compound_provenance.len(), 1);
    let authored = &first.compound_provenance[0];
    assert_eq!(authored.authored_body_index, 1);
    assert_eq!(authored.validation_path, "scene.bodies[1].shape.pieces");
    assert_eq!(authored.inherited_density, 1.75);
    assert!(!authored.inherited_is_sensor);
    assert_eq!(authored.pieces.len(), 3);
    for (piece_index, piece) in authored.pieces.iter().enumerate() {
        assert_eq!(piece.generated_piece_index, piece_index);
        assert!(
            piece.collider_handle.is_some(),
            "piece {piece_index} should resolve to a public collider handle"
        );
        assert_eq!(
            piece.validation_path,
            format!("scene.bodies[1].shape.pieces[{piece_index}]")
        );
    }
    assert!(
        first.snapshot.broadphase_tree.depth > 0,
        "compound provenance scenario should also expose a broadphase tree"
    );

    let frame_lines = fs::read_to_string(run.path.join(ArtifactFile::Frames.file_name()))
        .expect("frames should be readable");
    assert!(
        frame_lines.contains("\"compound_provenance\""),
        "frames.jsonl should preserve per-frame provenance facts"
    );

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let render_first = render
        .frames
        .first()
        .expect("debug render should include provenance facts");
    assert_eq!(
        render_first.broadphase_tree.depth,
        first.snapshot.broadphase_tree.depth
    );
    assert_eq!(render_first.compound_provenance, first.compound_provenance);
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
fn concave_decomposition_scenario_writes_generated_piece_provenance() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::ConcaveDecomposition,
            frame_count: 2,
            run_id: Some("m27-concave-decomposition".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("concave decomposition run should write artifacts");

    let first = run.frames.first().expect("first frame should exist");
    assert_eq!(first.compound_provenance.len(), 1);
    let generated = &first.compound_provenance[0];
    assert_eq!(generated.authored_body_index, 0);
    assert_eq!(generated.validation_path, "scene.bodies[0].shape.vertices");
    assert_eq!(generated.inherited_material, MaterialPreset::Rough);
    assert_eq!(
        generated.inherited_filter,
        CollisionLayerPreset::StaticGeometry
    );
    assert_eq!(generated.pieces.len(), 4);
    for (piece_index, piece) in generated.pieces.iter().enumerate() {
        assert_eq!(piece.generated_piece_index, piece_index);
        assert!(
            piece.collider_handle.is_some(),
            "generated piece {piece_index} should resolve to a collider handle"
        );
        assert_eq!(
            piece.validation_path,
            format!("scene.bodies[0].shape.generated_pieces[{piece_index}]")
        );
    }

    let frame_lines = fs::read_to_string(run.path.join(ArtifactFile::Frames.file_name()))
        .expect("frames should be readable");
    assert!(
        frame_lines.contains("\"compound_provenance\"")
            && frame_lines.contains("shape.generated_pieces"),
        "frames.jsonl should preserve generated concave-decomposition provenance"
    );

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let render_first = render
        .frames
        .first()
        .expect("debug render should include generated piece facts");
    assert_eq!(render_first.compound_provenance, first.compound_provenance);
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
fn ccd_dynamic_compound_wall_artifacts_capture_piece_trace_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::CcdDynamicCompoundWall,
            frame_count: 2,
            run_id: Some("m26-ccd-dynamic-compound-wall".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("dynamic compound CCD run should write artifacts");

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
        .expect("dynamic compound artifact should expose the selected piece trace");
    let trace = contact.ccd_trace.expect("trace should be present");
    assert_eq!(trace.target_kind, CcdTargetKind::Static);
    assert!(trace.toi > 0.0 && trace.toi < 1.0);
    assert!(trace.advancement >= trace.toi && trace.advancement <= 1.0);
    assert!(trace.clamp > 0.0);
    assert_eq!(trace.target_clamp, 0.0);

    let frame_lines = fs::read_to_string(run.path.join(ArtifactFile::Frames.file_name()))
        .expect("frames should be readable");
    assert!(
        frame_lines.contains("\"ccd_trace\"") && frame_lines.contains("\"target_kind\":\"static\""),
        "frames.jsonl should preserve selected piece CCD trace facts"
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
            .any(|contact| contact.ccd_trace == Some(trace)),
        "debug render should keep dynamic compound CCD trace facts"
    );
}

#[test]
fn lattice_grid_artifacts_capture_joint_lattice_proxy_facts() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::LatticeGrid,
            frame_count: 6,
            run_id: Some("m38-lattice-grid".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("lattice grid run should write artifacts");

    assert_eq!(run.manifest.scenario_id, ScenarioId::LatticeGrid);
    assert_eq!(run.frames.len(), 6);

    let first = run.frames.first().expect("first frame should exist");
    let dynamic_body_count = first
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
        .count();
    assert!(
        dynamic_body_count >= 9,
        "lattice proxy should export a grid of dynamic nodes"
    );
    assert!(
        first.snapshot.joints.len() >= 12,
        "lattice proxy should export enough debug joints for node/edge visualization"
    );
    assert!(
        first
            .snapshot
            .joints
            .iter()
            .any(|joint| joint.kind == picea::debug::DebugJointKind::Distance),
        "lattice proxy should include distance joints"
    );
    assert!(
        first
            .snapshot
            .joints
            .iter()
            .any(|joint| joint.kind == picea::debug::DebugJointKind::WorldAnchor),
        "lattice proxy should include world-anchor joints"
    );
    assert!(
        first
            .snapshot
            .bodies
            .iter()
            .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
            .all(|body| body.island_id.is_some()),
        "dynamic lattice nodes should export island membership"
    );
    assert!(
        run.frames
            .iter()
            .any(|frame| frame.snapshot.stats.joint_row_count > 0),
        "lattice proxy should produce joint solver rows"
    );

    let render: DebugRenderArtifact = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::DebugRender.file_name()))
            .expect("debug render should be readable"),
    )
    .expect("debug render should match schema");
    let render_first = render
        .frames
        .first()
        .expect("debug render should include first frame");
    assert_eq!(
        render_first.joints.len(),
        first.snapshot.joints.len(),
        "debug render should preserve lattice joint carriers"
    );
    assert!(
        !render_first.islands.is_empty(),
        "debug render should preserve island facts for lattice proxy frames"
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
        joints: Vec::new(),
        broadphase_tree: Default::default(),
        islands: Vec::new(),
        compound_provenance: Vec::new(),
        diagnostics: Default::default(),
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
    object.remove("joints");
    object.remove("broadphase_tree");
    object.remove("islands");
    object.remove("compound_provenance");
    object.remove("diagnostics");

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
    assert!(decoded.joints.is_empty());
    assert!(decoded.broadphase_tree.nodes.is_empty());
    assert!(decoded.islands.is_empty());
    assert!(decoded.compound_provenance.is_empty());
    assert!(decoded.diagnostics.markers.is_empty());
    assert_eq!(
        decoded.diagnostics.performance.counter_delta.source,
        DiagnosticSource::Missing,
        "older debug render frames should not decode missing diagnostics as lab-derived zero facts"
    );
    assert_eq!(
        decoded.diagnostics.stability.penetration.source,
        DiagnosticSource::Missing
    );
    assert_eq!(
        decoded.diagnostics.stability.warm_start.counts_source,
        DiagnosticSource::Missing
    );
    assert_eq!(
        decoded.diagnostics.stability.contact_churn.source,
        DiagnosticSource::Missing
    );
}

#[test]
fn diagnostics_marker_source_defaults_to_missing_when_deserializing_older_json() {
    let value = serde_json::json!({
        "performance": {},
        "stability": {},
        "markers": [
            {
                "kind": "solver_row_spike",
                "severity": "warning",
                "frame_index": 7,
                "score": 4.0,
                "threshold_name": "legacy_without_source",
                "evidence_fields": ["stats.contact_row_count"]
            }
        ],
        "missing_evidence": []
    });

    let decoded: picea_lab::FrameDiagnostics =
        serde_json::from_value(value).expect("older diagnostics markers should deserialize");
    let marker = decoded
        .markers
        .first()
        .expect("legacy marker should be preserved");
    assert_eq!(marker.kind, DiagnosticMarkerKind::SolverRowSpike);
    assert_eq!(
        marker.source,
        DiagnosticSource::Missing,
        "nested marker source should default to missing instead of rejecting older JSON"
    );
}

#[test]
fn legacy_frame_record_defaults_optional_evidence_when_missing() {
    let frame = FrameRecord {
        frame_index: 0,
        simulated_time: 0.0,
        state_hash: "legacy".to_owned(),
        report: Default::default(),
        stats: Default::default(),
        events: Vec::new(),
        snapshot: Default::default(),
        compound_provenance: Vec::new(),
        perturbation_provenance: Vec::new(),
        diagnostics: Default::default(),
    };
    let mut value = serde_json::to_value(frame).expect("frame should serialize");
    value
        .as_object_mut()
        .expect("frame should serialize as an object")
        .remove("compound_provenance");
    value
        .as_object_mut()
        .expect("frame should serialize as an object")
        .remove("perturbation_provenance");
    value
        .as_object_mut()
        .expect("frame should serialize as an object")
        .remove("diagnostics");

    let decoded: FrameRecord =
        serde_json::from_value(value).expect("older frame record should deserialize");
    assert!(decoded.compound_provenance.is_empty());
    assert!(decoded.perturbation_provenance.is_empty());
    assert!(decoded.diagnostics.markers.is_empty());
    assert_eq!(
        decoded.diagnostics.performance.counter_delta.source,
        DiagnosticSource::Missing,
        "older frame records should not decode missing diagnostics as lab-derived zero facts"
    );
    assert_eq!(
        decoded.diagnostics.stability.penetration.source,
        DiagnosticSource::Missing
    );
    assert_eq!(
        decoded.diagnostics.stability.warm_start.counts_source,
        DiagnosticSource::Missing
    );
    assert_eq!(
        decoded.diagnostics.stability.contact_churn.source,
        DiagnosticSource::Missing
    );
}

#[test]
fn direct_concave_fixture_rejection_remains_stable_for_m22_boundary() {
    let json = r#"
    {
      "schema_version": 1,
      "bodies": [
        {
          "body_type": "dynamic",
          "shape": {
            "type": "concave_polygon",
            "vertices": [[-1.0, -1.0], [1.0, -1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [-1.0, 1.0]]
          }
        }
      ]
    }
    "#;

    let fixture: SceneRecipeFixture =
        serde_json::from_str(json).expect("fixture json should deserialize");
    let error =
        instantiate_scene_fixture(&fixture).expect_err("direct concave fixture should fail");
    assert_eq!(
        error.to_string(),
        "world setup failed: scene.bodies[0].shape: concave_polygon automatic decomposition is only supported for static bodies"
    );
}
