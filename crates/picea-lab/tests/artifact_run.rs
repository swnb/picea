use std::{
    collections::{BTreeMap, VecDeque},
    env, fs,
    path::Path,
};

use picea::debug::DebugShape;
use picea::events::{
    CcdTargetKind, ContactLifecycleReason, SourceRowContinuityReason, WarmStartCacheReason,
};
use picea::prelude::{
    BodyHandle, ColliderHandle, CollisionLayerPreset, DebugCollider, DebugContact, MaterialPreset,
};
use picea_lab::{
    instantiate_scene_fixture, run_scenario, ArtifactFile, ArtifactStore, DebugRenderArtifact,
    DebugRenderFrame, DiagnosticMarkerKind, DiagnosticSeverity, DiagnosticSource, FrameRecord,
    MissingEvidenceKind, RunConfig, RunManifest, RunResult, ScenarioId, ScenarioOverrides,
    SceneRecipeFixture,
};
use serde_json::json;

const MATRIX_STACK_QUIET_WINDOW_START_FRAME: usize = 120;
const MATRIX_STACK_PRESSURE_WINDOW_START_FRAME: usize = 12;
const MATRIX_STACK_PRESSURE_WINDOW_END_FRAME: usize = 46;
const SUPPORT_GAP_TANGENT_LINEAR_ONSET_THRESHOLD: f32 = 1.0;
const SUPPORT_GAP_TANGENT_ANGULAR_ONSET_THRESHOLD: f32 = 0.5;
const SUPPORT_ELIGIBILITY_COUNTERPART_TANGENT_THRESHOLD: f32 = 0.05;
const SUPPORT_ELIGIBILITY_NORMAL_ANCHOR_DRIFT_THRESHOLD: f32 = 0.01;
const SUPPORT_ELIGIBILITY_TANGENT_ANCHOR_DRIFT_THRESHOLD: f32 = 0.25;
const FEATURE_CHURN_LOCAL_ANCHOR_DRIFT_THRESHOLD: f32 = 0.05;
const SOURCE_ROW_NORMAL_DOT_DRY_RUN_THRESHOLD: f32 = 0.98;
const SOURCE_ROW_PSEUDO_SUPPORT_SKIN: f32 = 0.01;

#[derive(Debug)]
struct MatrixStackStressReport {
    final_state_hash: String,
    first_bad_frame: Option<usize>,
    first_bad_state_hash: Option<String>,
    first_bad_markers: Vec<String>,
    max_penetration_depth: f64,
    max_penetration_sum: f64,
    peak_churn: usize,
    final_warm_start_hit_count: usize,
    final_warm_start_miss_count: usize,
    final_warm_start_drop_count: usize,
    final_awake_dynamic_body_count: usize,
    final_sleeping_dynamic_body_count: usize,
    final_max_linear_speed: f32,
    final_max_linear_body: Option<String>,
    final_max_angular_speed: f32,
    final_max_angular_body: Option<String>,
    quiet_window_start_frame: usize,
    quiet_window_max_linear_speed: f32,
    quiet_window_max_linear_body: Option<String>,
    quiet_window_max_linear_frame: Option<usize>,
    late_linear_spike_trace: LateVelocityTrace,
    quiet_window_max_angular_speed: f32,
    quiet_window_max_angular_body: Option<String>,
    quiet_window_max_angular_frame: Option<usize>,
    late_angular_spike_trace: LateVelocityTrace,
    early_pressure_trace: PressureWindowTrace,
    feature_churn_trace: FeatureChurnTrace,
    late_position_correction_max_translation: f32,
    late_position_correction_total_translation: f32,
    late_position_correction_body_count: usize,
    late_warm_start_drop_count: usize,
    late_contact_churn_peak: usize,
    max_support_friction_impulse: f32,
    max_support_friction_frame: Option<usize>,
    support_friction_contact_count: usize,
    first_floor_exit_frame: Option<usize>,
    first_floor_exit_body: Option<String>,
    first_floor_exit_x: Option<f32>,
    final_outside_floor_body_count: usize,
    ejection_pre_exit_frame: Option<usize>,
    ejection_pre_exit_x: Option<f32>,
    ejection_pre_exit_linear_speed: f32,
    ejection_pre_exit_angular_speed: f32,
    ejection_last_contact_frame: Option<usize>,
    ejection_last_contact_count: usize,
    ejection_last_contact_counterparts: Vec<String>,
    ejection_last_contact_max_depth: f32,
    ejection_last_contact_normal_impulse_sum: f32,
    ejection_last_contact_tangent_impulse_sum: f32,
    ejection_last_dynamic_contact_frame: Option<usize>,
    ejection_last_dynamic_contact_count: usize,
    ejection_last_dynamic_contact_counterparts: Vec<String>,
    ejection_last_dynamic_contact_max_depth: f32,
    ejection_last_dynamic_contact_normal_impulse_sum: f32,
    ejection_last_dynamic_contact_tangent_impulse_sum: f32,
    ejection_last_dynamic_contact_warm_start_normal_sum: f32,
    ejection_last_dynamic_contact_warm_start_tangent_sum: f32,
    ejection_last_dynamic_contact_warm_start_reasons: Vec<String>,
    ejection_last_dynamic_contact_normal_speed_min: f32,
    ejection_last_dynamic_contact_normal_speed_max: f32,
    ejection_last_dynamic_contact_tangent_speed_abs_max: f32,
    ejection_last_dynamic_impulse_frame: Option<usize>,
    ejection_last_dynamic_impulse_counterparts: Vec<String>,
    ejection_last_dynamic_impulse_normal_sum: f32,
    ejection_last_dynamic_impulse_tangent_sum: f32,
    ejection_last_dynamic_impulse_normal_speed_min: f32,
    ejection_last_dynamic_impulse_normal_speed_max: f32,
    ejection_last_dynamic_impulse_tangent_speed_abs_max: f32,
    ejection_row_max_frame: Option<usize>,
    ejection_row_max_linear_speed: f32,
    ejection_row_max_angular_speed: f32,
    ejection_last_dynamic_impulse_body_linear_speed: f32,
    ejection_last_dynamic_impulse_body_angular_speed: f32,
    ejection_last_dynamic_contact_body_linear_speed: f32,
    ejection_last_dynamic_contact_body_angular_speed: f32,
    ejection_support_gap_start_frame: Option<usize>,
    ejection_support_gap_start_linear_speed: f32,
    ejection_support_gap_start_angular_speed: f32,
    ejection_support_gap_previous_frame: Option<usize>,
    ejection_support_gap_previous_counterparts: Vec<String>,
    ejection_support_gap_previous_max_depth: f32,
    ejection_support_gap_previous_warm_start_normal_sum: f32,
    ejection_support_gap_previous_warm_start_tangent_sum: f32,
    ejection_support_gap_previous_normal_impulse_sum: f32,
    ejection_support_gap_previous_tangent_impulse_sum: f32,
    ejection_support_gap_previous_initial_normal_speed_min: f32,
    ejection_support_gap_previous_initial_normal_speed_max: f32,
    ejection_support_gap_previous_normal_speed_min: f32,
    ejection_support_gap_previous_normal_speed_max: f32,
    ejection_support_gap_previous_tangent_speed_abs_max: f32,
    ejection_support_gap_previous_body_linear_speed: f32,
    ejection_support_gap_previous_body_angular_speed: f32,
    ejection_support_gap_previous_counterpart_linear_speed_max: f32,
    ejection_support_gap_previous_counterpart_angular_speed_max: f32,
    ejection_support_gap_previous_body_tangent_linear_component_max: f32,
    ejection_support_gap_previous_body_tangent_angular_component_max: f32,
    ejection_support_gap_previous_counterpart_tangent_linear_component_max: f32,
    ejection_support_gap_previous_counterpart_tangent_angular_component_max: f32,
    ejection_support_gap_previous_normal_anchor_drift_max: f32,
    ejection_support_gap_previous_tangent_anchor_drift_max: f32,
    ejection_support_gap_previous_position_bias_max: f32,
    ejection_support_gap_previous_restitution_bias_max: f32,
    ejection_support_gap_previous_position_correction_depth_sum: f32,
    ejection_support_gap_previous_position_correction_translation_sum: f32,
    ejection_support_gap_next_frame: Option<usize>,
    ejection_support_gap_next_counterparts: Vec<String>,
    ejection_support_gap_next_max_depth: f32,
    ejection_support_gap_next_warm_start_normal_sum: f32,
    ejection_support_gap_next_warm_start_tangent_sum: f32,
    ejection_support_gap_next_normal_impulse_sum: f32,
    ejection_support_gap_next_tangent_impulse_sum: f32,
    ejection_support_gap_next_initial_normal_speed_min: f32,
    ejection_support_gap_next_initial_normal_speed_max: f32,
    ejection_support_gap_next_normal_speed_min: f32,
    ejection_support_gap_next_normal_speed_max: f32,
    ejection_support_gap_next_tangent_speed_abs_max: f32,
    ejection_support_gap_next_body_linear_speed: f32,
    ejection_support_gap_next_body_angular_speed: f32,
    ejection_support_gap_next_counterpart_linear_speed_max: f32,
    ejection_support_gap_next_counterpart_angular_speed_max: f32,
    ejection_support_gap_next_body_tangent_linear_component_max: f32,
    ejection_support_gap_next_body_tangent_angular_component_max: f32,
    ejection_support_gap_next_counterpart_tangent_linear_component_max: f32,
    ejection_support_gap_next_counterpart_tangent_angular_component_max: f32,
    ejection_support_gap_next_normal_anchor_drift_max: f32,
    ejection_support_gap_next_tangent_anchor_drift_max: f32,
    ejection_support_gap_next_position_bias_max: f32,
    ejection_support_gap_next_restitution_bias_max: f32,
    ejection_support_gap_next_position_correction_depth_sum: f32,
    ejection_support_gap_next_position_correction_translation_sum: f32,
    ejection_support_gap_upstream_frame: Option<usize>,
    ejection_support_gap_upstream_counterpart: Option<String>,
    ejection_support_gap_upstream_source_bodies: Vec<String>,
    ejection_support_gap_upstream_counterpart_linear_speed: f32,
    ejection_support_gap_upstream_counterpart_angular_speed: f32,
    ejection_support_gap_upstream_source_contact_count: usize,
    ejection_support_gap_upstream_source_candidate_count: usize,
    ejection_support_gap_upstream_source_warm_start_reasons: Vec<String>,
    ejection_support_gap_upstream_source_continuity_reasons: Vec<String>,
    ejection_support_gap_upstream_source_same_pair_previous_count: usize,
    ejection_support_gap_upstream_source_edge_swap_candidate_count: usize,
    ejection_support_gap_upstream_source_point_drift_min: Option<f32>,
    ejection_support_gap_upstream_source_normal_dot_max: Option<f32>,
    ejection_support_gap_upstream_source_local_anchor_drift_min: Option<f32>,
    ejection_support_gap_upstream_source_max_depth: f32,
    ejection_support_gap_upstream_source_initial_normal_speed_min: f32,
    ejection_support_gap_upstream_source_initial_normal_speed_max: f32,
    ejection_support_gap_upstream_source_normal_speed_min: f32,
    ejection_support_gap_upstream_source_normal_speed_max: f32,
    ejection_support_gap_upstream_source_tangent_speed_abs_max: f32,
    ejection_support_gap_upstream_source_position_bias_max: f32,
    ejection_support_gap_upstream_source_support_friction_sum: f32,
    ejection_support_gap_upstream_source_normal_impulse_sum: f32,
    ejection_support_gap_upstream_source_tangent_impulse_sum: f32,
    ejection_support_gap_upstream_source_correction_depth_sum: f32,
    ejection_support_gap_upstream_source_correction_translation_sum: f32,
    ejection_support_gap_upstream_source_dry_run_geometry_count: usize,
    ejection_support_gap_upstream_source_dry_run_pressure_count: usize,
    ejection_support_gap_upstream_source_dry_run_counterpart_reject_count: usize,
    ejection_support_gap_upstream_source_dry_run_tangent_reject_count: usize,
    ejection_support_gap_upstream_source_dry_run_eligible_count: usize,
    ejection_support_gap_upstream_source_dry_run_decision: String,
    ejection_support_gap_upstream_source_dry_run_pseudo_depth_min: Option<f32>,
    ejection_support_gap_upstream_source_dry_run_pseudo_depth_max: Option<f32>,
    ejection_support_gap_upstream_source_dry_run_pseudo_support_skin_count: usize,
    ejection_support_gap_upstream_handoff_frame_delta: Option<usize>,
    ejection_support_gap_upstream_handoff_linear_speed_delta: f32,
    ejection_support_gap_upstream_handoff_angular_speed_delta: f32,
    max_contact_row_count: usize,
    max_contact_row_frame: Option<usize>,
    missing_evidence: Vec<MissingEvidenceKind>,
}

impl MatrixStackStressReport {
    // E2 is a read-only triage layer: it turns existing lifecycle and pressure facts
    // into a milestone-routing hint without changing solver truth or artifact hashes.
    fn e2_shadow_direction(&self) -> &'static str {
        let identity_lifecycle_signal = self.feature_churn_trace.miss_feature_id_count > 0
            && self.feature_churn_trace.same_pair_previous_count > 0
            && self.feature_churn_trace.close_local_anchor_count > 0;
        let stale_position_row_signal = self
            .late_linear_spike_trace
            .support_gap_lifecycle
            .iter()
            .any(|frame| {
                matches!(
                    frame.eligibility_decision.as_str(),
                    "retain_candidate_correction_active"
                        | "retain_candidate_needs_position_re_evaluation"
                        | "reject_counterpart_motion"
                )
            })
            && self.late_position_correction_total_translation > 0.0;

        match (identity_lifecycle_signal, stale_position_row_signal) {
            (true, true) => "intertwined",
            (true, false) => "identity_lifecycle_primary",
            (false, true) => "stale_position_row_primary",
            (false, false) => "insufficient_evidence",
        }
    }

    fn e2_shadow_gate(&self) -> &'static str {
        if self
            .late_linear_spike_trace
            .support_gap_lifecycle
            .iter()
            .any(|frame| frame.eligibility_decision == "reject_counterpart_motion")
        {
            "reject_counterpart_motion"
        } else if self
            .late_linear_spike_trace
            .support_gap_lifecycle
            .iter()
            .any(|frame| {
                frame.eligibility_decision == "retain_candidate_needs_position_re_evaluation"
            })
        {
            "candidate_needs_position_re_evaluation"
        } else if self
            .late_linear_spike_trace
            .support_gap_lifecycle
            .iter()
            .any(|frame| frame.eligibility_decision == "retain_candidate_correction_active")
        {
            "correction_active"
        } else {
            "no_shadow_gate"
        }
    }

    fn e2_next_milestone_hint(&self) -> &'static str {
        match self.e2_shadow_direction() {
            "identity_lifecycle_primary" => "D4/E6",
            "stale_position_row_primary" => "E3",
            "intertwined" => "E3 with D4/E6 watch",
            _ => "collect more evidence",
        }
    }

    fn to_markdown(&self, run: &RunResult) -> String {
        let first_bad_frame = self
            .first_bad_frame
            .map(|frame| frame.to_string())
            .unwrap_or_else(|| "none".to_owned());
        let first_bad_state_hash = self
            .first_bad_state_hash
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let markers = if self.first_bad_markers.is_empty() {
            "none".to_owned()
        } else {
            self.first_bad_markers.join(", ")
        };
        let missing = if self.missing_evidence.is_empty() {
            "none".to_owned()
        } else {
            self.missing_evidence
                .iter()
                .map(|kind| format!("{kind:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let final_max_linear_body = self
            .final_max_linear_body
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let final_max_angular_body = self
            .final_max_angular_body
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let quiet_max_linear_body = self
            .quiet_window_max_linear_body
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let quiet_max_angular_body = self
            .quiet_window_max_angular_body
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let floor_exit = match (
            self.first_floor_exit_frame,
            self.first_floor_exit_body.as_deref(),
            self.first_floor_exit_x,
        ) {
            (Some(frame), Some(body), Some(x)) => {
                format!("first exit frame {frame} body {body} center_x {x:.6}")
            }
            _ => "none".to_owned(),
        };
        let ejection_trace = match (
            self.ejection_pre_exit_frame,
            self.ejection_pre_exit_x,
            self.ejection_last_contact_frame,
        ) {
            (Some(pre_frame), Some(x), Some(last_contact_frame)) => format!(
                "pre-exit frame {pre_frame} center_x {x:.6} speed {:.6} angular {:.6}; last contact frame {last_contact_frame} count {} counterparts [{}] max_depth {:.6} normal_impulse_sum {:.6} tangent_impulse_sum {:.6}",
                self.ejection_pre_exit_linear_speed,
                self.ejection_pre_exit_angular_speed,
                self.ejection_last_contact_count,
                self.ejection_last_contact_counterparts.join(", "),
                self.ejection_last_contact_max_depth,
                self.ejection_last_contact_normal_impulse_sum,
                self.ejection_last_contact_tangent_impulse_sum,
            ),
            (Some(pre_frame), Some(x), None) => format!(
                "pre-exit frame {pre_frame} center_x {x:.6} speed {:.6} angular {:.6}; last contact none",
                self.ejection_pre_exit_linear_speed,
                self.ejection_pre_exit_angular_speed,
            ),
            _ => "none".to_owned(),
        };
        let ejection_dynamic_trace = match self.ejection_last_dynamic_contact_frame {
            Some(frame) => format!(
                "last dynamic contact frame {frame} count {} counterparts [{}] max_depth {:.6} normal_impulse_sum {:.6} tangent_impulse_sum {:.6} warm_start_impulse {:.6}/{:.6} warm_start [{}] normal_speed {:.6}..{:.6} tangent_speed_abs_max {:.6}",
                self.ejection_last_dynamic_contact_count,
                self.ejection_last_dynamic_contact_counterparts.join(", "),
                self.ejection_last_dynamic_contact_max_depth,
                self.ejection_last_dynamic_contact_normal_impulse_sum,
                self.ejection_last_dynamic_contact_tangent_impulse_sum,
                self.ejection_last_dynamic_contact_warm_start_normal_sum,
                self.ejection_last_dynamic_contact_warm_start_tangent_sum,
                self.ejection_last_dynamic_contact_warm_start_reasons.join(", "),
                self.ejection_last_dynamic_contact_normal_speed_min,
                self.ejection_last_dynamic_contact_normal_speed_max,
                self.ejection_last_dynamic_contact_tangent_speed_abs_max,
            ),
            None => "none".to_owned(),
        };
        let ejection_dynamic_impulse_trace = match self.ejection_last_dynamic_impulse_frame {
            Some(frame) => format!(
                "last dynamic impulse frame {frame} counterparts [{}] normal_impulse_sum {:.6} tangent_impulse_sum {:.6} normal_speed {:.6}..{:.6} tangent_speed_abs_max {:.6}",
                self.ejection_last_dynamic_impulse_counterparts.join(", "),
                self.ejection_last_dynamic_impulse_normal_sum,
                self.ejection_last_dynamic_impulse_tangent_sum,
                self.ejection_last_dynamic_impulse_normal_speed_min,
                self.ejection_last_dynamic_impulse_normal_speed_max,
                self.ejection_last_dynamic_impulse_tangent_speed_abs_max,
            ),
            None => "none".to_owned(),
        };
        let ejection_velocity_trace = match self.ejection_pre_exit_frame {
            Some(pre_frame) => format!(
                "row-max frame {} speed {:.6}/{:.6}; last impulse speed {:.6}/{:.6}; last dynamic contact speed {:.6}/{:.6}; pre-exit frame {pre_frame} speed {:.6}/{:.6}",
                self.ejection_row_max_frame
                    .map(|frame| frame.to_string())
                    .unwrap_or_else(|| "none".to_owned()),
                self.ejection_row_max_linear_speed,
                self.ejection_row_max_angular_speed,
                self.ejection_last_dynamic_impulse_body_linear_speed,
                self.ejection_last_dynamic_impulse_body_angular_speed,
                self.ejection_last_dynamic_contact_body_linear_speed,
                self.ejection_last_dynamic_contact_body_angular_speed,
                self.ejection_pre_exit_linear_speed,
                self.ejection_pre_exit_angular_speed,
            ),
            None => "none".to_owned(),
        };
        let ejection_support_gap_trace = match self.ejection_support_gap_start_frame {
            Some(gap_frame) => format!(
                "gap start frame {gap_frame} speed {:.6}/{:.6}; previous frame {} counterparts [{}] max_depth {:.6} warm {:.6}/{:.6} impulse {:.6}/{:.6} initial_speed {:.6}..{:.6} post_speed {:.6}..{:.6}/{:.6} body_speed {:.6}/{:.6} counterpart_speed {:.6}/{:.6} tangent_components body {:.6}/{:.6} counterpart {:.6}/{:.6} drift {:.6}/{:.6} bias {:.6}/{:.6} correction {:.6}/{:.6}; next frame {} counterparts [{}] max_depth {:.6} warm {:.6}/{:.6} impulse {:.6}/{:.6} initial_speed {:.6}..{:.6} post_speed {:.6}..{:.6}/{:.6} body_speed {:.6}/{:.6} counterpart_speed {:.6}/{:.6} tangent_components body {:.6}/{:.6} counterpart {:.6}/{:.6} drift {:.6}/{:.6} bias {:.6}/{:.6} correction {:.6}/{:.6}",
                self.ejection_support_gap_start_linear_speed,
                self.ejection_support_gap_start_angular_speed,
                self.ejection_support_gap_previous_frame
                    .map(|frame| frame.to_string())
                    .unwrap_or_else(|| "none".to_owned()),
                self.ejection_support_gap_previous_counterparts.join(", "),
                self.ejection_support_gap_previous_max_depth,
                self.ejection_support_gap_previous_warm_start_normal_sum,
                self.ejection_support_gap_previous_warm_start_tangent_sum,
                self.ejection_support_gap_previous_normal_impulse_sum,
                self.ejection_support_gap_previous_tangent_impulse_sum,
                self.ejection_support_gap_previous_initial_normal_speed_min,
                self.ejection_support_gap_previous_initial_normal_speed_max,
                self.ejection_support_gap_previous_normal_speed_min,
                self.ejection_support_gap_previous_normal_speed_max,
                self.ejection_support_gap_previous_tangent_speed_abs_max,
                self.ejection_support_gap_previous_body_linear_speed,
                self.ejection_support_gap_previous_body_angular_speed,
                self.ejection_support_gap_previous_counterpart_linear_speed_max,
                self.ejection_support_gap_previous_counterpart_angular_speed_max,
                self.ejection_support_gap_previous_body_tangent_linear_component_max,
                self.ejection_support_gap_previous_body_tangent_angular_component_max,
                self.ejection_support_gap_previous_counterpart_tangent_linear_component_max,
                self.ejection_support_gap_previous_counterpart_tangent_angular_component_max,
                self.ejection_support_gap_previous_normal_anchor_drift_max,
                self.ejection_support_gap_previous_tangent_anchor_drift_max,
                self.ejection_support_gap_previous_position_bias_max,
                self.ejection_support_gap_previous_restitution_bias_max,
                self.ejection_support_gap_previous_position_correction_depth_sum,
                self.ejection_support_gap_previous_position_correction_translation_sum,
                self.ejection_support_gap_next_frame
                    .map(|frame| frame.to_string())
                    .unwrap_or_else(|| "none".to_owned()),
                self.ejection_support_gap_next_counterparts.join(", "),
                self.ejection_support_gap_next_max_depth,
                self.ejection_support_gap_next_warm_start_normal_sum,
                self.ejection_support_gap_next_warm_start_tangent_sum,
                self.ejection_support_gap_next_normal_impulse_sum,
                self.ejection_support_gap_next_tangent_impulse_sum,
                self.ejection_support_gap_next_initial_normal_speed_min,
                self.ejection_support_gap_next_initial_normal_speed_max,
                self.ejection_support_gap_next_normal_speed_min,
                self.ejection_support_gap_next_normal_speed_max,
                self.ejection_support_gap_next_tangent_speed_abs_max,
                self.ejection_support_gap_next_body_linear_speed,
                self.ejection_support_gap_next_body_angular_speed,
                self.ejection_support_gap_next_counterpart_linear_speed_max,
                self.ejection_support_gap_next_counterpart_angular_speed_max,
                self.ejection_support_gap_next_body_tangent_linear_component_max,
                self.ejection_support_gap_next_body_tangent_angular_component_max,
                self.ejection_support_gap_next_counterpart_tangent_linear_component_max,
                self.ejection_support_gap_next_counterpart_tangent_angular_component_max,
                self.ejection_support_gap_next_normal_anchor_drift_max,
                self.ejection_support_gap_next_tangent_anchor_drift_max,
                self.ejection_support_gap_next_position_bias_max,
                self.ejection_support_gap_next_restitution_bias_max,
                self.ejection_support_gap_next_position_correction_depth_sum,
                self.ejection_support_gap_next_position_correction_translation_sum,
            ),
            None => "none".to_owned(),
        };
        let ejection_support_gap_upstream_trace = match (
            self.ejection_support_gap_upstream_frame,
            self.ejection_support_gap_upstream_counterpart.as_deref(),
        ) {
            (Some(frame), Some(counterpart)) => format!(
                "frame {frame} counterpart {counterpart} speed {:.6}/{:.6}; source_bodies [{}] source_contacts {} source_candidates {} warm_start [{}] continuity_reasons [{}] continuity same_pair_previous {} edge_swap_candidate {} point_drift {} normal_dot {} local_anchor_drift {}; max_depth {:.6} initial_speed {:.6}..{:.6} post_speed {:.6}..{:.6}/{:.6} bias {:.6} support_friction {:.6} impulse {:.6}/{:.6} correction {:.6}/{:.6}; dry_run geometry {} pressure {} reject_counterpart {} reject_tangent {} eligible {} decision {} pseudo_depth {}..{} pseudo_skin {}; handoff frame_delta {} speed_delta {:.6}/{:.6}",
                self.ejection_support_gap_upstream_counterpart_linear_speed,
                self.ejection_support_gap_upstream_counterpart_angular_speed,
                self.ejection_support_gap_upstream_source_bodies.join(", "),
                self.ejection_support_gap_upstream_source_contact_count,
                self.ejection_support_gap_upstream_source_candidate_count,
                self.ejection_support_gap_upstream_source_warm_start_reasons
                    .join(", "),
                self.ejection_support_gap_upstream_source_continuity_reasons
                    .join(", "),
                self.ejection_support_gap_upstream_source_same_pair_previous_count,
                self.ejection_support_gap_upstream_source_edge_swap_candidate_count,
                option_f32(self.ejection_support_gap_upstream_source_point_drift_min),
                option_f32(self.ejection_support_gap_upstream_source_normal_dot_max),
                option_f32(self.ejection_support_gap_upstream_source_local_anchor_drift_min),
                self.ejection_support_gap_upstream_source_max_depth,
                self.ejection_support_gap_upstream_source_initial_normal_speed_min,
                self.ejection_support_gap_upstream_source_initial_normal_speed_max,
                self.ejection_support_gap_upstream_source_normal_speed_min,
                self.ejection_support_gap_upstream_source_normal_speed_max,
                self.ejection_support_gap_upstream_source_tangent_speed_abs_max,
                self.ejection_support_gap_upstream_source_position_bias_max,
                self.ejection_support_gap_upstream_source_support_friction_sum,
                self.ejection_support_gap_upstream_source_normal_impulse_sum,
                self.ejection_support_gap_upstream_source_tangent_impulse_sum,
                self.ejection_support_gap_upstream_source_correction_depth_sum,
                self.ejection_support_gap_upstream_source_correction_translation_sum,
                self.ejection_support_gap_upstream_source_dry_run_geometry_count,
                self.ejection_support_gap_upstream_source_dry_run_pressure_count,
                self.ejection_support_gap_upstream_source_dry_run_counterpart_reject_count,
                self.ejection_support_gap_upstream_source_dry_run_tangent_reject_count,
                self.ejection_support_gap_upstream_source_dry_run_eligible_count,
                self.ejection_support_gap_upstream_source_dry_run_decision,
                option_f32(self.ejection_support_gap_upstream_source_dry_run_pseudo_depth_min),
                option_f32(self.ejection_support_gap_upstream_source_dry_run_pseudo_depth_max),
                self.ejection_support_gap_upstream_source_dry_run_pseudo_support_skin_count,
                option_frame(self.ejection_support_gap_upstream_handoff_frame_delta),
                self.ejection_support_gap_upstream_handoff_linear_speed_delta,
                self.ejection_support_gap_upstream_handoff_angular_speed_delta,
            ),
            _ => "none".to_owned(),
        };
        let late_linear_spike_trace = self.late_linear_spike_trace.to_report_line();
        let late_angular_spike_trace = self.late_angular_spike_trace.to_report_line();
        let early_pressure_trace = self.early_pressure_trace.to_report_line();
        let feature_churn_trace = self.feature_churn_trace.to_report_line();
        let e2_shadow_direction = self.e2_shadow_direction();
        let e2_shadow_gate = self.e2_shadow_gate();
        let e2_next_milestone_hint = self.e2_next_milestone_hint();

        format!(
            "## Matrix Stack Stability Report\n\n\
- Scenario: {}\n\
- Frame count: {}\n\
- Run path: {}\n\
- Final state hash: {}\n\
- First bad frame: {} ({})\n\
- Markers: {}\n\
- Penetration max / sum: {:.6} / {:.6}\n\
- Contact churn: peak entered+exited={}\n\
- Warm-start hit / miss / drop: {} / {} / {}\n\
- Sleep awake / sleeping: {} / {}\n\
- Sleep blocker speeds: linear {:.6} at {}, angular {:.6} at {}\n\
- Quiet window f>={}: max linear {:.6} at {} frame {}, max angular {:.6} at {} frame {}\n\
- Late linear spike trace: {}\n\
- Late angular spike trace: {}\n\
- Early pressure trace f={}..{}: {}\n\
- Feature churn trace: {}\n\
- E2 shadow direction: {} (miss_feature_id {} same_pair_previous {} close_local_anchor {} edge_swap_candidate {}; dry_run {} late_correction {:.6})\n\
- Late correction/churn f>={}: max correction translation {:.6}, max correction total {:.6}, max corrected bodies {}, warm-start drop peak {}, churn peak {}\n\
- Dense support friction: max {:.6} at frame {}, contacts used {}\n\
- Floor ejection: {}; final outside floor bodies={}\n\
- Ejection trace: {}\n\
- Ejection dynamic-support trace: {}\n\
- Ejection dynamic-impulse trace: {}\n\
- Ejection velocity trace: {}\n\
- Ejection support-gap trace: {}\n\
- Ejection support-gap upstream trace: {}\n\
- Solver row max: {} at frame {}\n\
- Missing evidence: {}\n\
- Suspected next milestone: {}\n\
- Verification commands: rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack\n",
            run.manifest.scenario_id,
            run.frames.len(),
            run.path.display(),
            self.final_state_hash,
            first_bad_frame,
            first_bad_state_hash,
            markers,
            self.max_penetration_depth,
            self.max_penetration_sum,
            self.peak_churn,
            self.final_warm_start_hit_count,
            self.final_warm_start_miss_count,
            self.final_warm_start_drop_count,
            self.final_awake_dynamic_body_count,
            self.final_sleeping_dynamic_body_count,
            self.final_max_linear_speed,
            final_max_linear_body,
            self.final_max_angular_speed,
            final_max_angular_body,
            self.quiet_window_start_frame,
            self.quiet_window_max_linear_speed,
            quiet_max_linear_body,
            self.quiet_window_max_linear_frame
                .map(|frame| frame.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.quiet_window_max_angular_speed,
            quiet_max_angular_body,
            self.quiet_window_max_angular_frame
                .map(|frame| frame.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            late_linear_spike_trace,
            late_angular_spike_trace,
            self.early_pressure_trace.start_frame,
            self.early_pressure_trace.end_frame,
            early_pressure_trace,
            feature_churn_trace,
            e2_shadow_direction,
            self.feature_churn_trace.miss_feature_id_count,
            self.feature_churn_trace.same_pair_previous_count,
            self.feature_churn_trace.close_local_anchor_count,
            self.feature_churn_trace.edge_swap_candidate_count,
            e2_shadow_gate,
            self.late_position_correction_total_translation,
            self.quiet_window_start_frame,
            self.late_position_correction_max_translation,
            self.late_position_correction_total_translation,
            self.late_position_correction_body_count,
            self.late_warm_start_drop_count,
            self.late_contact_churn_peak,
            self.max_support_friction_impulse,
            self.max_support_friction_frame
                .map(|frame| frame.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.support_friction_contact_count,
            floor_exit,
            self.final_outside_floor_body_count,
            ejection_trace,
            ejection_dynamic_trace,
            ejection_dynamic_impulse_trace,
            ejection_velocity_trace,
            ejection_support_gap_trace,
            ejection_support_gap_upstream_trace,
            self.max_contact_row_count,
            self.max_contact_row_frame
                .map(|frame| frame.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            missing,
            e2_next_milestone_hint,
        )
    }
}

fn matrix_stack_stress_report(run: &RunResult) -> MatrixStackStressReport {
    let first_bad_frame = run.frames.iter().find(|frame| {
        frame.diagnostics.markers.iter().any(|marker| {
            matches!(
                marker.severity,
                DiagnosticSeverity::Warning | DiagnosticSeverity::Severe
            )
        })
    });
    let max_penetration_depth = run
        .frames
        .iter()
        .map(|frame| frame.diagnostics.stability.penetration.max_depth)
        .fold(0.0, f64::max);
    let max_penetration_sum = run
        .frames
        .iter()
        .map(|frame| frame.diagnostics.stability.penetration.total_depth)
        .fold(0.0, f64::max);
    // "churn" means contact pairs entering/exiting instead of persisting.
    // Tracking the peak frame-level churn keeps the stress report comparable
    // without inventing a second diagnostics pipeline.
    let peak_churn = run
        .frames
        .iter()
        .map(|frame| {
            frame.diagnostics.stability.contact_churn.entered
                + frame.diagnostics.stability.contact_churn.exited
        })
        .max()
        .unwrap_or(0);
    let final_frame = run
        .frames
        .last()
        .expect("stress run should have a final frame");
    let final_dynamic_bodies = final_frame
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
        .collect::<Vec<_>>();
    let final_max_linear = final_dynamic_bodies
        .iter()
        .map(|body| (body.linear_velocity.length(), format!("{:?}", body.handle)))
        .max_by(|(lhs, _), (rhs, _)| lhs.partial_cmp(rhs).unwrap());
    let final_max_angular = final_dynamic_bodies
        .iter()
        .map(|body| (body.angular_velocity.abs(), format!("{:?}", body.handle)))
        .max_by(|(lhs, _), (rhs, _)| lhs.partial_cmp(rhs).unwrap());
    let quiet_window_frames = run
        .frames
        .iter()
        .filter(|frame| frame.frame_index >= MATRIX_STACK_QUIET_WINDOW_START_FRAME)
        .collect::<Vec<_>>();
    let quiet_window_max_linear = quiet_window_frames
        .iter()
        .flat_map(|frame| {
            frame
                .snapshot
                .bodies
                .iter()
                .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
                .map(|body| {
                    (
                        body.linear_velocity.length(),
                        format!("{:?}", body.handle),
                        frame.frame_index,
                        body.handle,
                    )
                })
        })
        .max_by(|(lhs, _, _, _), (rhs, _, _, _)| lhs.partial_cmp(rhs).unwrap());
    let quiet_window_max_angular = quiet_window_frames
        .iter()
        .flat_map(|frame| {
            frame
                .snapshot
                .bodies
                .iter()
                .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
                .map(|body| {
                    (
                        body.angular_velocity.abs(),
                        format!("{:?}", body.handle),
                        frame.frame_index,
                        body.handle,
                    )
                })
        })
        .max_by(|(lhs, _, _, _), (rhs, _, _, _)| lhs.partial_cmp(rhs).unwrap());
    let late_linear_spike_trace = quiet_window_max_linear
        .as_ref()
        .map(|(_, _, frame, body)| late_velocity_trace(run, *frame, *body))
        .unwrap_or_default();
    let late_angular_spike_trace = quiet_window_max_angular
        .as_ref()
        .map(|(_, _, frame, body)| late_velocity_trace(run, *frame, *body))
        .unwrap_or_default();
    let early_pressure_trace = pressure_window_trace(
        run,
        MATRIX_STACK_PRESSURE_WINDOW_START_FRAME,
        MATRIX_STACK_PRESSURE_WINDOW_END_FRAME,
    );
    let feature_churn_trace = feature_churn_trace(run);
    let late_position_correction_max_translation = quiet_window_frames
        .iter()
        .map(|frame| frame.stats.position_correction_max_translation)
        .fold(0.0, f32::max);
    let late_position_correction_total_translation = quiet_window_frames
        .iter()
        .map(|frame| frame.stats.position_correction_total_translation)
        .fold(0.0, f32::max);
    let late_position_correction_body_count = quiet_window_frames
        .iter()
        .map(|frame| frame.stats.position_correction_body_count)
        .max()
        .unwrap_or(0);
    let late_warm_start_drop_count = quiet_window_frames
        .iter()
        .map(|frame| frame.stats.warm_start_drop_count)
        .max()
        .unwrap_or(0);
    let late_contact_churn_peak = quiet_window_frames
        .iter()
        .map(|frame| {
            frame.diagnostics.stability.contact_churn.entered
                + frame.diagnostics.stability.contact_churn.exited
        })
        .max()
        .unwrap_or(0);
    let max_support_friction = run
        .frames
        .iter()
        .flat_map(|frame| {
            frame
                .snapshot
                .contacts
                .iter()
                .map(|contact| (contact.solver_support_friction_impulse, frame.frame_index))
        })
        .max_by(|(lhs, _), (rhs, _)| lhs.partial_cmp(rhs).unwrap());
    let support_friction_contact_count = run
        .frames
        .iter()
        .flat_map(|frame| frame.snapshot.contacts.iter())
        .filter(|contact| contact.solver_support_friction_impulse > f32::EPSILON)
        .count();
    let floor_bounds = matrix_stack_floor_x_bounds(run);
    let first_floor_exit = floor_bounds.and_then(|bounds| {
        run.frames.iter().find_map(|frame| {
            frame
                .snapshot
                .bodies
                .iter()
                .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
                .find(|body| {
                    let x = body.transform.translation.x();
                    x < bounds.0 || x > bounds.1
                })
                .map(|body| {
                    (
                        frame.frame_index,
                        body.handle,
                        body.transform.translation.x(),
                    )
                })
        })
    });
    let final_outside_floor_body_count = floor_bounds
        .map(|bounds| {
            final_dynamic_bodies
                .iter()
                .filter(|body| {
                    let x = body.transform.translation.x();
                    x < bounds.0 || x > bounds.1
                })
                .count()
        })
        .unwrap_or(0);
    let ejection_trace = matrix_stack_ejection_trace(run, first_floor_exit);
    let max_contact_row_frame = run
        .frames
        .iter()
        .max_by_key(|frame| frame.diagnostics.stability.island.contact_row_count);
    let mut missing_evidence = Vec::new();
    for frame in &run.frames {
        for missing in &frame.diagnostics.missing_evidence {
            if !missing_evidence.contains(&missing.kind) {
                missing_evidence.push(missing.kind);
            }
        }
    }

    MatrixStackStressReport {
        final_state_hash: run.manifest.final_state_hash.clone(),
        first_bad_frame: first_bad_frame.map(|frame| frame.frame_index),
        first_bad_state_hash: first_bad_frame.map(|frame| frame.state_hash.clone()),
        first_bad_markers: first_bad_frame
            .into_iter()
            .flat_map(|frame| frame.diagnostics.markers.iter())
            .map(|marker| format!("{:?}:{:?}", marker.kind, marker.severity))
            .collect(),
        max_penetration_depth,
        max_penetration_sum,
        peak_churn,
        final_warm_start_hit_count: final_frame.diagnostics.stability.warm_start.hit_count,
        final_warm_start_miss_count: final_frame.diagnostics.stability.warm_start.miss_count,
        final_warm_start_drop_count: final_frame.diagnostics.stability.warm_start.drop_count,
        final_awake_dynamic_body_count: final_frame
            .diagnostics
            .stability
            .sleep
            .awake_dynamic_body_count,
        final_sleeping_dynamic_body_count: final_frame
            .diagnostics
            .stability
            .sleep
            .sleeping_dynamic_body_count,
        final_max_linear_speed: final_max_linear
            .as_ref()
            .map(|(speed, _)| *speed)
            .unwrap_or_default(),
        final_max_linear_body: final_max_linear.map(|(_, body)| body),
        final_max_angular_speed: final_max_angular
            .as_ref()
            .map(|(speed, _)| *speed)
            .unwrap_or_default(),
        final_max_angular_body: final_max_angular.map(|(_, body)| body),
        quiet_window_start_frame: MATRIX_STACK_QUIET_WINDOW_START_FRAME,
        quiet_window_max_linear_speed: quiet_window_max_linear
            .as_ref()
            .map(|(speed, _, _, _)| *speed)
            .unwrap_or_default(),
        quiet_window_max_linear_body: quiet_window_max_linear
            .as_ref()
            .map(|(_, body, _, _)| body.clone()),
        quiet_window_max_linear_frame: quiet_window_max_linear.map(|(_, _, frame, _)| frame),
        late_linear_spike_trace,
        quiet_window_max_angular_speed: quiet_window_max_angular
            .as_ref()
            .map(|(speed, _, _, _)| *speed)
            .unwrap_or_default(),
        quiet_window_max_angular_body: quiet_window_max_angular
            .as_ref()
            .map(|(_, body, _, _)| body.clone()),
        quiet_window_max_angular_frame: quiet_window_max_angular.map(|(_, _, frame, _)| frame),
        late_angular_spike_trace,
        early_pressure_trace,
        feature_churn_trace,
        late_position_correction_max_translation,
        late_position_correction_total_translation,
        late_position_correction_body_count,
        late_warm_start_drop_count,
        late_contact_churn_peak,
        max_support_friction_impulse: max_support_friction
            .map(|(impulse, _)| impulse)
            .unwrap_or(0.0),
        max_support_friction_frame: max_support_friction.map(|(_, frame)| frame),
        support_friction_contact_count,
        first_floor_exit_frame: first_floor_exit.as_ref().map(|(frame, _, _)| *frame),
        first_floor_exit_body: first_floor_exit
            .as_ref()
            .map(|(_, body, _)| format!("{body:?}")),
        first_floor_exit_x: first_floor_exit.map(|(_, _, x)| x),
        final_outside_floor_body_count,
        ejection_pre_exit_frame: ejection_trace.pre_exit_frame,
        ejection_pre_exit_x: ejection_trace.pre_exit_x,
        ejection_pre_exit_linear_speed: ejection_trace.pre_exit_linear_speed,
        ejection_pre_exit_angular_speed: ejection_trace.pre_exit_angular_speed,
        ejection_last_contact_frame: ejection_trace.last_contact_frame,
        ejection_last_contact_count: ejection_trace.last_contact_count,
        ejection_last_contact_counterparts: ejection_trace.last_contact_counterparts,
        ejection_last_contact_max_depth: ejection_trace.last_contact_max_depth,
        ejection_last_contact_normal_impulse_sum: ejection_trace.last_contact_normal_impulse_sum,
        ejection_last_contact_tangent_impulse_sum: ejection_trace.last_contact_tangent_impulse_sum,
        ejection_last_dynamic_contact_frame: ejection_trace.last_dynamic_contact_frame,
        ejection_last_dynamic_contact_count: ejection_trace.last_dynamic_contact_count,
        ejection_last_dynamic_contact_counterparts: ejection_trace
            .last_dynamic_contact_counterparts,
        ejection_last_dynamic_contact_max_depth: ejection_trace.last_dynamic_contact_max_depth,
        ejection_last_dynamic_contact_normal_impulse_sum: ejection_trace
            .last_dynamic_contact_normal_impulse_sum,
        ejection_last_dynamic_contact_tangent_impulse_sum: ejection_trace
            .last_dynamic_contact_tangent_impulse_sum,
        ejection_last_dynamic_contact_warm_start_normal_sum: ejection_trace
            .last_dynamic_contact_warm_start_normal_sum,
        ejection_last_dynamic_contact_warm_start_tangent_sum: ejection_trace
            .last_dynamic_contact_warm_start_tangent_sum,
        ejection_last_dynamic_contact_warm_start_reasons: ejection_trace
            .last_dynamic_contact_warm_start_reasons,
        ejection_last_dynamic_contact_normal_speed_min: ejection_trace
            .last_dynamic_contact_normal_speed_min,
        ejection_last_dynamic_contact_normal_speed_max: ejection_trace
            .last_dynamic_contact_normal_speed_max,
        ejection_last_dynamic_contact_tangent_speed_abs_max: ejection_trace
            .last_dynamic_contact_tangent_speed_abs_max,
        ejection_last_dynamic_impulse_frame: ejection_trace.last_dynamic_impulse_frame,
        ejection_last_dynamic_impulse_counterparts: ejection_trace
            .last_dynamic_impulse_counterparts,
        ejection_last_dynamic_impulse_normal_sum: ejection_trace.last_dynamic_impulse_normal_sum,
        ejection_last_dynamic_impulse_tangent_sum: ejection_trace.last_dynamic_impulse_tangent_sum,
        ejection_last_dynamic_impulse_normal_speed_min: ejection_trace
            .last_dynamic_impulse_normal_speed_min,
        ejection_last_dynamic_impulse_normal_speed_max: ejection_trace
            .last_dynamic_impulse_normal_speed_max,
        ejection_last_dynamic_impulse_tangent_speed_abs_max: ejection_trace
            .last_dynamic_impulse_tangent_speed_abs_max,
        ejection_row_max_frame: ejection_trace.row_max_frame,
        ejection_row_max_linear_speed: ejection_trace.row_max_linear_speed,
        ejection_row_max_angular_speed: ejection_trace.row_max_angular_speed,
        ejection_last_dynamic_impulse_body_linear_speed: ejection_trace
            .last_dynamic_impulse_body_linear_speed,
        ejection_last_dynamic_impulse_body_angular_speed: ejection_trace
            .last_dynamic_impulse_body_angular_speed,
        ejection_last_dynamic_contact_body_linear_speed: ejection_trace
            .last_dynamic_contact_body_linear_speed,
        ejection_last_dynamic_contact_body_angular_speed: ejection_trace
            .last_dynamic_contact_body_angular_speed,
        ejection_support_gap_start_frame: ejection_trace.support_gap_start_frame,
        ejection_support_gap_start_linear_speed: ejection_trace.support_gap_start_linear_speed,
        ejection_support_gap_start_angular_speed: ejection_trace.support_gap_start_angular_speed,
        ejection_support_gap_previous_frame: ejection_trace.support_gap_previous_frame,
        ejection_support_gap_previous_counterparts: ejection_trace
            .support_gap_previous_counterparts,
        ejection_support_gap_previous_max_depth: ejection_trace.support_gap_previous_max_depth,
        ejection_support_gap_previous_warm_start_normal_sum: ejection_trace
            .support_gap_previous_warm_start_normal_sum,
        ejection_support_gap_previous_warm_start_tangent_sum: ejection_trace
            .support_gap_previous_warm_start_tangent_sum,
        ejection_support_gap_previous_normal_impulse_sum: ejection_trace
            .support_gap_previous_normal_impulse_sum,
        ejection_support_gap_previous_tangent_impulse_sum: ejection_trace
            .support_gap_previous_tangent_impulse_sum,
        ejection_support_gap_previous_initial_normal_speed_min: ejection_trace
            .support_gap_previous_initial_normal_speed_min,
        ejection_support_gap_previous_initial_normal_speed_max: ejection_trace
            .support_gap_previous_initial_normal_speed_max,
        ejection_support_gap_previous_normal_speed_min: ejection_trace
            .support_gap_previous_normal_speed_min,
        ejection_support_gap_previous_normal_speed_max: ejection_trace
            .support_gap_previous_normal_speed_max,
        ejection_support_gap_previous_tangent_speed_abs_max: ejection_trace
            .support_gap_previous_tangent_speed_abs_max,
        ejection_support_gap_previous_body_linear_speed: ejection_trace
            .support_gap_previous_body_linear_speed,
        ejection_support_gap_previous_body_angular_speed: ejection_trace
            .support_gap_previous_body_angular_speed,
        ejection_support_gap_previous_counterpart_linear_speed_max: ejection_trace
            .support_gap_previous_counterpart_linear_speed_max,
        ejection_support_gap_previous_counterpart_angular_speed_max: ejection_trace
            .support_gap_previous_counterpart_angular_speed_max,
        ejection_support_gap_previous_body_tangent_linear_component_max: ejection_trace
            .support_gap_previous_body_tangent_linear_component_max,
        ejection_support_gap_previous_body_tangent_angular_component_max: ejection_trace
            .support_gap_previous_body_tangent_angular_component_max,
        ejection_support_gap_previous_counterpart_tangent_linear_component_max: ejection_trace
            .support_gap_previous_counterpart_tangent_linear_component_max,
        ejection_support_gap_previous_counterpart_tangent_angular_component_max: ejection_trace
            .support_gap_previous_counterpart_tangent_angular_component_max,
        ejection_support_gap_previous_normal_anchor_drift_max: ejection_trace
            .support_gap_previous_normal_anchor_drift_max,
        ejection_support_gap_previous_tangent_anchor_drift_max: ejection_trace
            .support_gap_previous_tangent_anchor_drift_max,
        ejection_support_gap_previous_position_bias_max: ejection_trace
            .support_gap_previous_position_bias_max,
        ejection_support_gap_previous_restitution_bias_max: ejection_trace
            .support_gap_previous_restitution_bias_max,
        ejection_support_gap_previous_position_correction_depth_sum: ejection_trace
            .support_gap_previous_position_correction_depth_sum,
        ejection_support_gap_previous_position_correction_translation_sum: ejection_trace
            .support_gap_previous_position_correction_translation_sum,
        ejection_support_gap_next_frame: ejection_trace.support_gap_next_frame,
        ejection_support_gap_next_counterparts: ejection_trace.support_gap_next_counterparts,
        ejection_support_gap_next_max_depth: ejection_trace.support_gap_next_max_depth,
        ejection_support_gap_next_warm_start_normal_sum: ejection_trace
            .support_gap_next_warm_start_normal_sum,
        ejection_support_gap_next_warm_start_tangent_sum: ejection_trace
            .support_gap_next_warm_start_tangent_sum,
        ejection_support_gap_next_normal_impulse_sum: ejection_trace
            .support_gap_next_normal_impulse_sum,
        ejection_support_gap_next_tangent_impulse_sum: ejection_trace
            .support_gap_next_tangent_impulse_sum,
        ejection_support_gap_next_initial_normal_speed_min: ejection_trace
            .support_gap_next_initial_normal_speed_min,
        ejection_support_gap_next_initial_normal_speed_max: ejection_trace
            .support_gap_next_initial_normal_speed_max,
        ejection_support_gap_next_normal_speed_min: ejection_trace
            .support_gap_next_normal_speed_min,
        ejection_support_gap_next_normal_speed_max: ejection_trace
            .support_gap_next_normal_speed_max,
        ejection_support_gap_next_tangent_speed_abs_max: ejection_trace
            .support_gap_next_tangent_speed_abs_max,
        ejection_support_gap_next_body_linear_speed: ejection_trace
            .support_gap_next_body_linear_speed,
        ejection_support_gap_next_body_angular_speed: ejection_trace
            .support_gap_next_body_angular_speed,
        ejection_support_gap_next_counterpart_linear_speed_max: ejection_trace
            .support_gap_next_counterpart_linear_speed_max,
        ejection_support_gap_next_counterpart_angular_speed_max: ejection_trace
            .support_gap_next_counterpart_angular_speed_max,
        ejection_support_gap_next_body_tangent_linear_component_max: ejection_trace
            .support_gap_next_body_tangent_linear_component_max,
        ejection_support_gap_next_body_tangent_angular_component_max: ejection_trace
            .support_gap_next_body_tangent_angular_component_max,
        ejection_support_gap_next_counterpart_tangent_linear_component_max: ejection_trace
            .support_gap_next_counterpart_tangent_linear_component_max,
        ejection_support_gap_next_counterpart_tangent_angular_component_max: ejection_trace
            .support_gap_next_counterpart_tangent_angular_component_max,
        ejection_support_gap_next_normal_anchor_drift_max: ejection_trace
            .support_gap_next_normal_anchor_drift_max,
        ejection_support_gap_next_tangent_anchor_drift_max: ejection_trace
            .support_gap_next_tangent_anchor_drift_max,
        ejection_support_gap_next_position_bias_max: ejection_trace
            .support_gap_next_position_bias_max,
        ejection_support_gap_next_restitution_bias_max: ejection_trace
            .support_gap_next_restitution_bias_max,
        ejection_support_gap_next_position_correction_depth_sum: ejection_trace
            .support_gap_next_position_correction_depth_sum,
        ejection_support_gap_next_position_correction_translation_sum: ejection_trace
            .support_gap_next_position_correction_translation_sum,
        ejection_support_gap_upstream_frame: ejection_trace.support_gap_upstream_frame,
        ejection_support_gap_upstream_counterpart: ejection_trace.support_gap_upstream_counterpart,
        ejection_support_gap_upstream_source_bodies: ejection_trace
            .support_gap_upstream_source_bodies,
        ejection_support_gap_upstream_counterpart_linear_speed: ejection_trace
            .support_gap_upstream_counterpart_linear_speed,
        ejection_support_gap_upstream_counterpart_angular_speed: ejection_trace
            .support_gap_upstream_counterpart_angular_speed,
        ejection_support_gap_upstream_source_contact_count: ejection_trace
            .support_gap_upstream_source_contact_count,
        ejection_support_gap_upstream_source_candidate_count: ejection_trace
            .support_gap_upstream_source_candidate_count,
        ejection_support_gap_upstream_source_warm_start_reasons: ejection_trace
            .support_gap_upstream_source_warm_start_reasons,
        ejection_support_gap_upstream_source_continuity_reasons: ejection_trace
            .support_gap_upstream_source_continuity_reasons,
        ejection_support_gap_upstream_source_same_pair_previous_count: ejection_trace
            .support_gap_upstream_source_same_pair_previous_count,
        ejection_support_gap_upstream_source_edge_swap_candidate_count: ejection_trace
            .support_gap_upstream_source_edge_swap_candidate_count,
        ejection_support_gap_upstream_source_point_drift_min: ejection_trace
            .support_gap_upstream_source_point_drift_min,
        ejection_support_gap_upstream_source_normal_dot_max: ejection_trace
            .support_gap_upstream_source_normal_dot_max,
        ejection_support_gap_upstream_source_local_anchor_drift_min: ejection_trace
            .support_gap_upstream_source_local_anchor_drift_min,
        ejection_support_gap_upstream_source_max_depth: ejection_trace
            .support_gap_upstream_source_max_depth,
        ejection_support_gap_upstream_source_initial_normal_speed_min: ejection_trace
            .support_gap_upstream_source_initial_normal_speed_min,
        ejection_support_gap_upstream_source_initial_normal_speed_max: ejection_trace
            .support_gap_upstream_source_initial_normal_speed_max,
        ejection_support_gap_upstream_source_normal_speed_min: ejection_trace
            .support_gap_upstream_source_normal_speed_min,
        ejection_support_gap_upstream_source_normal_speed_max: ejection_trace
            .support_gap_upstream_source_normal_speed_max,
        ejection_support_gap_upstream_source_tangent_speed_abs_max: ejection_trace
            .support_gap_upstream_source_tangent_speed_abs_max,
        ejection_support_gap_upstream_source_position_bias_max: ejection_trace
            .support_gap_upstream_source_position_bias_max,
        ejection_support_gap_upstream_source_support_friction_sum: ejection_trace
            .support_gap_upstream_source_support_friction_sum,
        ejection_support_gap_upstream_source_normal_impulse_sum: ejection_trace
            .support_gap_upstream_source_normal_impulse_sum,
        ejection_support_gap_upstream_source_tangent_impulse_sum: ejection_trace
            .support_gap_upstream_source_tangent_impulse_sum,
        ejection_support_gap_upstream_source_correction_depth_sum: ejection_trace
            .support_gap_upstream_source_correction_depth_sum,
        ejection_support_gap_upstream_source_correction_translation_sum: ejection_trace
            .support_gap_upstream_source_correction_translation_sum,
        ejection_support_gap_upstream_source_dry_run_geometry_count: ejection_trace
            .support_gap_upstream_source_dry_run_geometry_count,
        ejection_support_gap_upstream_source_dry_run_pressure_count: ejection_trace
            .support_gap_upstream_source_dry_run_pressure_count,
        ejection_support_gap_upstream_source_dry_run_counterpart_reject_count: ejection_trace
            .support_gap_upstream_source_dry_run_counterpart_reject_count,
        ejection_support_gap_upstream_source_dry_run_tangent_reject_count: ejection_trace
            .support_gap_upstream_source_dry_run_tangent_reject_count,
        ejection_support_gap_upstream_source_dry_run_eligible_count: ejection_trace
            .support_gap_upstream_source_dry_run_eligible_count,
        ejection_support_gap_upstream_source_dry_run_decision: ejection_trace
            .support_gap_upstream_source_dry_run_decision,
        ejection_support_gap_upstream_source_dry_run_pseudo_depth_min: ejection_trace
            .support_gap_upstream_source_dry_run_pseudo_depth_min,
        ejection_support_gap_upstream_source_dry_run_pseudo_depth_max: ejection_trace
            .support_gap_upstream_source_dry_run_pseudo_depth_max,
        ejection_support_gap_upstream_source_dry_run_pseudo_support_skin_count: ejection_trace
            .support_gap_upstream_source_dry_run_pseudo_support_skin_count,
        ejection_support_gap_upstream_handoff_frame_delta: ejection_trace
            .support_gap_upstream_handoff_frame_delta,
        ejection_support_gap_upstream_handoff_linear_speed_delta: ejection_trace
            .support_gap_upstream_handoff_linear_speed_delta,
        ejection_support_gap_upstream_handoff_angular_speed_delta: ejection_trace
            .support_gap_upstream_handoff_angular_speed_delta,
        max_contact_row_count: run
            .frames
            .iter()
            .map(|frame| frame.diagnostics.stability.island.contact_row_count)
            .max()
            .unwrap_or(0),
        max_contact_row_frame: max_contact_row_frame.map(|frame| frame.frame_index),
        missing_evidence,
    }
}

#[derive(Default)]
struct EjectionTrace {
    pre_exit_frame: Option<usize>,
    pre_exit_x: Option<f32>,
    pre_exit_linear_speed: f32,
    pre_exit_angular_speed: f32,
    last_contact_frame: Option<usize>,
    last_contact_count: usize,
    last_contact_counterparts: Vec<String>,
    last_contact_max_depth: f32,
    last_contact_normal_impulse_sum: f32,
    last_contact_tangent_impulse_sum: f32,
    last_dynamic_contact_frame: Option<usize>,
    last_dynamic_contact_count: usize,
    last_dynamic_contact_counterparts: Vec<String>,
    last_dynamic_contact_max_depth: f32,
    last_dynamic_contact_normal_impulse_sum: f32,
    last_dynamic_contact_tangent_impulse_sum: f32,
    last_dynamic_contact_warm_start_normal_sum: f32,
    last_dynamic_contact_warm_start_tangent_sum: f32,
    last_dynamic_contact_warm_start_reasons: Vec<String>,
    last_dynamic_contact_normal_speed_min: f32,
    last_dynamic_contact_normal_speed_max: f32,
    last_dynamic_contact_tangent_speed_abs_max: f32,
    last_dynamic_impulse_frame: Option<usize>,
    last_dynamic_impulse_counterparts: Vec<String>,
    last_dynamic_impulse_normal_sum: f32,
    last_dynamic_impulse_tangent_sum: f32,
    last_dynamic_impulse_normal_speed_min: f32,
    last_dynamic_impulse_normal_speed_max: f32,
    last_dynamic_impulse_tangent_speed_abs_max: f32,
    row_max_frame: Option<usize>,
    row_max_linear_speed: f32,
    row_max_angular_speed: f32,
    last_dynamic_impulse_body_linear_speed: f32,
    last_dynamic_impulse_body_angular_speed: f32,
    last_dynamic_contact_body_linear_speed: f32,
    last_dynamic_contact_body_angular_speed: f32,
    support_gap_start_frame: Option<usize>,
    support_gap_start_linear_speed: f32,
    support_gap_start_angular_speed: f32,
    support_gap_previous_frame: Option<usize>,
    support_gap_previous_counterparts: Vec<String>,
    support_gap_previous_max_depth: f32,
    support_gap_previous_warm_start_normal_sum: f32,
    support_gap_previous_warm_start_tangent_sum: f32,
    support_gap_previous_normal_impulse_sum: f32,
    support_gap_previous_tangent_impulse_sum: f32,
    support_gap_previous_initial_normal_speed_min: f32,
    support_gap_previous_initial_normal_speed_max: f32,
    support_gap_previous_normal_speed_min: f32,
    support_gap_previous_normal_speed_max: f32,
    support_gap_previous_tangent_speed_abs_max: f32,
    support_gap_previous_body_linear_speed: f32,
    support_gap_previous_body_angular_speed: f32,
    support_gap_previous_counterpart_linear_speed_max: f32,
    support_gap_previous_counterpart_angular_speed_max: f32,
    support_gap_previous_body_tangent_linear_component_max: f32,
    support_gap_previous_body_tangent_angular_component_max: f32,
    support_gap_previous_counterpart_tangent_linear_component_max: f32,
    support_gap_previous_counterpart_tangent_angular_component_max: f32,
    support_gap_previous_normal_anchor_drift_max: f32,
    support_gap_previous_tangent_anchor_drift_max: f32,
    support_gap_previous_position_bias_max: f32,
    support_gap_previous_restitution_bias_max: f32,
    support_gap_previous_position_correction_depth_sum: f32,
    support_gap_previous_position_correction_translation_sum: f32,
    support_gap_next_frame: Option<usize>,
    support_gap_next_counterparts: Vec<String>,
    support_gap_next_max_depth: f32,
    support_gap_next_warm_start_normal_sum: f32,
    support_gap_next_warm_start_tangent_sum: f32,
    support_gap_next_normal_impulse_sum: f32,
    support_gap_next_tangent_impulse_sum: f32,
    support_gap_next_initial_normal_speed_min: f32,
    support_gap_next_initial_normal_speed_max: f32,
    support_gap_next_normal_speed_min: f32,
    support_gap_next_normal_speed_max: f32,
    support_gap_next_tangent_speed_abs_max: f32,
    support_gap_next_body_linear_speed: f32,
    support_gap_next_body_angular_speed: f32,
    support_gap_next_counterpart_linear_speed_max: f32,
    support_gap_next_counterpart_angular_speed_max: f32,
    support_gap_next_body_tangent_linear_component_max: f32,
    support_gap_next_body_tangent_angular_component_max: f32,
    support_gap_next_counterpart_tangent_linear_component_max: f32,
    support_gap_next_counterpart_tangent_angular_component_max: f32,
    support_gap_next_normal_anchor_drift_max: f32,
    support_gap_next_tangent_anchor_drift_max: f32,
    support_gap_next_position_bias_max: f32,
    support_gap_next_restitution_bias_max: f32,
    support_gap_next_position_correction_depth_sum: f32,
    support_gap_next_position_correction_translation_sum: f32,
    support_gap_upstream_frame: Option<usize>,
    support_gap_upstream_counterpart: Option<String>,
    support_gap_upstream_source_bodies: Vec<String>,
    support_gap_upstream_counterpart_linear_speed: f32,
    support_gap_upstream_counterpart_angular_speed: f32,
    support_gap_upstream_source_contact_count: usize,
    support_gap_upstream_source_candidate_count: usize,
    support_gap_upstream_source_warm_start_reasons: Vec<String>,
    support_gap_upstream_source_continuity_reasons: Vec<String>,
    support_gap_upstream_source_same_pair_previous_count: usize,
    support_gap_upstream_source_edge_swap_candidate_count: usize,
    support_gap_upstream_source_point_drift_min: Option<f32>,
    support_gap_upstream_source_normal_dot_max: Option<f32>,
    support_gap_upstream_source_local_anchor_drift_min: Option<f32>,
    support_gap_upstream_source_max_depth: f32,
    support_gap_upstream_source_initial_normal_speed_min: f32,
    support_gap_upstream_source_initial_normal_speed_max: f32,
    support_gap_upstream_source_normal_speed_min: f32,
    support_gap_upstream_source_normal_speed_max: f32,
    support_gap_upstream_source_tangent_speed_abs_max: f32,
    support_gap_upstream_source_position_bias_max: f32,
    support_gap_upstream_source_support_friction_sum: f32,
    support_gap_upstream_source_normal_impulse_sum: f32,
    support_gap_upstream_source_tangent_impulse_sum: f32,
    support_gap_upstream_source_correction_depth_sum: f32,
    support_gap_upstream_source_correction_translation_sum: f32,
    support_gap_upstream_source_dry_run_geometry_count: usize,
    support_gap_upstream_source_dry_run_pressure_count: usize,
    support_gap_upstream_source_dry_run_counterpart_reject_count: usize,
    support_gap_upstream_source_dry_run_tangent_reject_count: usize,
    support_gap_upstream_source_dry_run_eligible_count: usize,
    support_gap_upstream_source_dry_run_decision: String,
    support_gap_upstream_source_dry_run_pseudo_depth_min: Option<f32>,
    support_gap_upstream_source_dry_run_pseudo_depth_max: Option<f32>,
    support_gap_upstream_source_dry_run_pseudo_support_skin_count: usize,
    support_gap_upstream_handoff_frame_delta: Option<usize>,
    support_gap_upstream_handoff_linear_speed_delta: f32,
    support_gap_upstream_handoff_angular_speed_delta: f32,
}

#[derive(Default, Debug)]
struct PressureWindowTrace {
    start_frame: usize,
    end_frame: usize,
    max_tangent_frame: Option<usize>,
    max_tangent_body_pair: Option<[BodyHandle; 2]>,
    max_tangent_bodies: Vec<String>,
    max_tangent_speed_abs: f32,
    max_tangent_depth: f32,
    max_tangent_initial_normal_speed: f32,
    max_tangent_final_normal_speed: f32,
    max_tangent_normal_impulse: f32,
    max_tangent_tangent_impulse: f32,
    max_tangent_support_friction: f32,
    max_tangent_position_bias: f32,
    max_tangent_warm_start_reason: String,
    max_tangent_feature_id: String,
    max_angular_frame: Option<usize>,
    max_angular_body: Option<String>,
    max_angular_linear_speed: f32,
    max_angular_speed: f32,
    max_correction_frame: Option<usize>,
    max_correction_translation: f32,
    max_correction_total: f32,
    max_correction_body_count: usize,
    max_correction_row_frame: Option<usize>,
    max_correction_row_body_pair: Option<[BodyHandle; 2]>,
    max_correction_row_bodies: Vec<String>,
    max_correction_row_depth: f32,
    max_correction_row_consumed_depth: f32,
    max_correction_row_body_a_translation: f32,
    max_correction_row_body_b_translation: f32,
    max_correction_row_final_normal_speed: f32,
    max_correction_row_final_tangent_speed: f32,
    max_churn_frame: Option<usize>,
    max_churn: usize,
    max_warm_start_drop_frame: Option<usize>,
    max_warm_start_drop: usize,
    max_contact_row_frame: Option<usize>,
    max_contact_row_count: usize,
    max_support_friction_frame: Option<usize>,
    max_support_friction: f32,
    max_anchor_drift_frame: Option<usize>,
    max_anchor_drift_body_pair: Option<[BodyHandle; 2]>,
    max_anchor_drift_bodies: Vec<String>,
    max_anchor_drift: f32,
    max_normal_anchor_drift: f32,
    max_tangent_anchor_drift: f32,
    row_coupling_summary: String,
    warm_start_reason_counts: Vec<String>,
}

impl PressureWindowTrace {
    fn to_report_line(&self) -> String {
        format!(
            "window {}..{}; tangent frame {} bodies [{}] speed {:.6} depth {:.6} normal_speed {:.6}->{:.6} impulse {:.6}/{:.6} support_friction {:.6} bias {:.6} warm_start {} feature {}; angular frame {} body {} speed {:.6}/{:.6}; correction frame {} max/total/bodies {:.6}/{:.6}/{}; correction_row frame {} bodies [{}] depth {:.6} consumed {:.6} translation {:.6}/{:.6} final_speed {:.6}/{:.6}; churn peak {} at frame {}; drops peak {} at frame {}; rows peak {} at frame {}; support_friction max {:.6} at frame {}; anchor_drift max {:.6} bodies [{}] normal {:.6} tangent {:.6} at frame {}; row_coupling {}; warm_start reasons [{}]",
            self.start_frame,
            self.end_frame,
            option_frame(self.max_tangent_frame),
            self.max_tangent_bodies.join(", "),
            self.max_tangent_speed_abs,
            self.max_tangent_depth,
            self.max_tangent_initial_normal_speed,
            self.max_tangent_final_normal_speed,
            self.max_tangent_normal_impulse,
            self.max_tangent_tangent_impulse,
            self.max_tangent_support_friction,
            self.max_tangent_position_bias,
            if self.max_tangent_warm_start_reason.is_empty() {
                "none"
            } else {
                self.max_tangent_warm_start_reason.as_str()
            },
            if self.max_tangent_feature_id.is_empty() {
                "none"
            } else {
                self.max_tangent_feature_id.as_str()
            },
            option_frame(self.max_angular_frame),
            self.max_angular_body.as_deref().unwrap_or("none"),
            self.max_angular_linear_speed,
            self.max_angular_speed,
            option_frame(self.max_correction_frame),
            self.max_correction_translation,
            self.max_correction_total,
            self.max_correction_body_count,
            option_frame(self.max_correction_row_frame),
            self.max_correction_row_bodies.join(", "),
            self.max_correction_row_depth,
            self.max_correction_row_consumed_depth,
            self.max_correction_row_body_a_translation,
            self.max_correction_row_body_b_translation,
            self.max_correction_row_final_normal_speed,
            self.max_correction_row_final_tangent_speed,
            self.max_churn,
            option_frame(self.max_churn_frame),
            self.max_warm_start_drop,
            option_frame(self.max_warm_start_drop_frame),
            self.max_contact_row_count,
            option_frame(self.max_contact_row_frame),
            self.max_support_friction,
            option_frame(self.max_support_friction_frame),
            self.max_anchor_drift,
            self.max_anchor_drift_bodies.join(", "),
            self.max_normal_anchor_drift,
            self.max_tangent_anchor_drift,
            option_frame(self.max_anchor_drift_frame),
            if self.row_coupling_summary.is_empty() {
                "none"
            } else {
                self.row_coupling_summary.as_str()
            },
            self.warm_start_reason_counts.join(", "),
        )
    }
}

#[derive(Default, Debug)]
struct FeatureChurnTrace {
    miss_feature_id_count: usize,
    same_pair_previous_count: usize,
    same_reduction_reason_count: usize,
    same_feature_index_count: usize,
    close_world_point_count: usize,
    same_shape_signature_count: usize,
    close_local_anchor_count: usize,
    edge_swap_transition_count: usize,
    edge_swap_candidate_count: usize,
    point_slot_fallback_eligible_count: usize,
    best_frame: Option<usize>,
    best_pair: Vec<String>,
    best_current_feature_id: String,
    best_previous_feature_id: String,
    best_point_drift: f32,
    best_normal_dot: f32,
    best_local_anchor_drift: f32,
    edge_swap_best_frame: Option<usize>,
    edge_swap_best_pair: Vec<String>,
    edge_swap_best_current_feature_id: String,
    edge_swap_best_previous_feature_id: String,
    edge_swap_best_point_drift: f32,
    edge_swap_best_normal_dot: f32,
    edge_swap_best_local_anchor_drift: f32,
    top_feature_index_transition: String,
    top_feature_index_transition_count: usize,
    top_transition_best_frame: Option<usize>,
    top_transition_best_pair: Vec<String>,
    top_transition_best_current_feature_id: String,
    top_transition_best_previous_feature_id: String,
    top_transition_best_point_drift: f32,
    top_transition_best_normal_dot: f32,
    top_transition_best_local_anchor_drift: f32,
    late_same_pair_previous_count: usize,
}

impl FeatureChurnTrace {
    fn to_report_line(&self) -> String {
        format!(
            "miss_feature_id {} same_pair_previous {} same_reduction_reason {} same_feature_index {} close_world_point {} same_shape_signature {} close_local_anchor {} edge_swap_transition {} edge_swap_candidate {} point_slot_fallback_eligible {} late_same_pair_previous {}; edge_swap_best frame {} pair [{}] feature {}<-{} point_drift {:.6} normal_dot {:.6} local_anchor_drift {:.6}; top_feature_index_transition {} x{}; top_transition_best frame {} pair [{}] feature {}<-{} point_drift {:.6} normal_dot {:.6} local_anchor_drift {:.6}; best frame {} pair [{}] feature {}<-{} point_drift {:.6} normal_dot {:.6} local_anchor_drift {:.6}",
            self.miss_feature_id_count,
            self.same_pair_previous_count,
            self.same_reduction_reason_count,
            self.same_feature_index_count,
            self.close_world_point_count,
            self.same_shape_signature_count,
            self.close_local_anchor_count,
            self.edge_swap_transition_count,
            self.edge_swap_candidate_count,
            self.point_slot_fallback_eligible_count,
            self.late_same_pair_previous_count,
            option_frame(self.edge_swap_best_frame),
            self.edge_swap_best_pair.join(", "),
            if self.edge_swap_best_current_feature_id.is_empty() {
                "none"
            } else {
                self.edge_swap_best_current_feature_id.as_str()
            },
            if self.edge_swap_best_previous_feature_id.is_empty() {
                "none"
            } else {
                self.edge_swap_best_previous_feature_id.as_str()
            },
            self.edge_swap_best_point_drift,
            self.edge_swap_best_normal_dot,
            self.edge_swap_best_local_anchor_drift,
            if self.top_feature_index_transition.is_empty() {
                "none"
            } else {
                self.top_feature_index_transition.as_str()
            },
            self.top_feature_index_transition_count,
            option_frame(self.top_transition_best_frame),
            self.top_transition_best_pair.join(", "),
            if self.top_transition_best_current_feature_id.is_empty() {
                "none"
            } else {
                self.top_transition_best_current_feature_id.as_str()
            },
            if self.top_transition_best_previous_feature_id.is_empty() {
                "none"
            } else {
                self.top_transition_best_previous_feature_id.as_str()
            },
            self.top_transition_best_point_drift,
            self.top_transition_best_normal_dot,
            self.top_transition_best_local_anchor_drift,
            option_frame(self.best_frame),
            self.best_pair.join(", "),
            if self.best_current_feature_id.is_empty() {
                "none"
            } else {
                self.best_current_feature_id.as_str()
            },
            if self.best_previous_feature_id.is_empty() {
                "none"
            } else {
                self.best_previous_feature_id.as_str()
            },
            self.best_point_drift,
            self.best_normal_dot,
            self.best_local_anchor_drift,
        )
    }
}

#[derive(Default)]
struct FeatureTransitionTrace {
    count: usize,
    best_frame: Option<usize>,
    best_pair: Vec<String>,
    best_current_feature_id: String,
    best_previous_feature_id: String,
    best_point_drift: f32,
    best_normal_dot: f32,
    best_local_anchor_drift: f32,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
struct LateVelocityTrace {
    frame: Option<usize>,
    body: Option<String>,
    linear_speed: f32,
    angular_speed: f32,
    contact_count: usize,
    counterparts: Vec<String>,
    max_depth: f32,
    warm_start_normal_sum: f32,
    warm_start_tangent_sum: f32,
    normal_impulse_sum: f32,
    tangent_impulse_sum: f32,
    initial_normal_speed_min: f32,
    initial_normal_speed_max: f32,
    normal_speed_min: f32,
    normal_speed_max: f32,
    tangent_speed_abs_max: f32,
    support_friction_impulse_max: f32,
    support_friction_impulse_sum: f32,
    position_bias_max: f32,
    restitution_bias_max: f32,
    position_correction_max_translation: f32,
    position_correction_total_translation: f32,
    position_correction_body_count: usize,
    warm_start_drop_count: usize,
    contact_churn: usize,
    last_contact_frame: Option<usize>,
    last_contact_count: usize,
    last_contact_counterparts: Vec<String>,
    last_contact_max_depth: f32,
    last_contact_warm_start_normal_sum: f32,
    last_contact_warm_start_tangent_sum: f32,
    last_contact_normal_impulse_sum: f32,
    last_contact_tangent_impulse_sum: f32,
    last_contact_initial_normal_speed_min: f32,
    last_contact_initial_normal_speed_max: f32,
    last_contact_normal_speed_min: f32,
    last_contact_normal_speed_max: f32,
    last_contact_tangent_speed_abs_max: f32,
    last_contact_support_friction_impulse_max: f32,
    last_contact_support_friction_impulse_sum: f32,
    support_gap_start_frame: Option<usize>,
    support_gap_duration: usize,
    support_gap_previous_normal_impulse_sum: f32,
    support_gap_previous_normal_speed_min: f32,
    support_gap_previous_body_linear_speed: f32,
    support_gap_previous_body_angular_speed: f32,
    support_gap_previous_counterpart_linear_speed_max: f32,
    support_gap_previous_counterpart_angular_speed_max: f32,
    support_gap_previous_body_tangent_linear_component_max: f32,
    support_gap_previous_body_tangent_angular_component_max: f32,
    support_gap_previous_counterpart_tangent_linear_component_max: f32,
    support_gap_previous_counterpart_tangent_angular_component_max: f32,
    support_gap_energy_onset_frame: Option<usize>,
    support_gap_energy_onset_body_tangent_linear_component_max: f32,
    support_gap_energy_onset_body_tangent_angular_component_max: f32,
    support_gap_energy_onset_counterpart_tangent_linear_component_max: f32,
    support_gap_energy_onset_counterpart_tangent_angular_component_max: f32,
    support_gap_energy_onset_normal_impulse_sum: f32,
    support_gap_energy_onset_normal_speed_min: f32,
    support_gap_energy_onset_max_depth: f32,
    support_gap_lifecycle: Vec<SupportGapLifecycleFrame>,
    support_gap_previous_position_correction_depth_sum: f32,
    support_gap_previous_position_correction_translation_sum: f32,
    support_gap_summary: String,
}

impl LateVelocityTrace {
    fn to_report_line(&self) -> String {
        let Some(frame) = self.frame else {
            return "none".to_owned();
        };
        let body = self.body.as_deref().unwrap_or("none");
        format!(
            "frame {frame} body {body} speed {:.6}/{:.6}; contacts {} counterparts [{}] max_depth {:.6} warm {:.6}/{:.6} impulse {:.6}/{:.6} initial_speed {:.6}..{:.6} post_speed {:.6}..{:.6}/{:.6} support_friction {:.6}/{:.6} bias {:.6}/{:.6} correction {:.6}/{:.6}/{} churn {} drops {}; last_contact frame {} count {} counterparts [{}] max_depth {:.6} warm {:.6}/{:.6} impulse {:.6}/{:.6} initial_speed {:.6}..{:.6} post_speed {:.6}..{:.6}/{:.6} support_friction {:.6}/{:.6}; late_support_gap {}",
            self.linear_speed,
            self.angular_speed,
            self.contact_count,
            self.counterparts.join(", "),
            self.max_depth,
            self.warm_start_normal_sum,
            self.warm_start_tangent_sum,
            self.normal_impulse_sum,
            self.tangent_impulse_sum,
            self.initial_normal_speed_min,
            self.initial_normal_speed_max,
            self.normal_speed_min,
            self.normal_speed_max,
            self.tangent_speed_abs_max,
            self.support_friction_impulse_max,
            self.support_friction_impulse_sum,
            self.position_bias_max,
            self.restitution_bias_max,
            self.position_correction_max_translation,
            self.position_correction_total_translation,
            self.position_correction_body_count,
            self.contact_churn,
            self.warm_start_drop_count,
            self.last_contact_frame
                .map(|frame| frame.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.last_contact_count,
            self.last_contact_counterparts.join(", "),
            self.last_contact_max_depth,
            self.last_contact_warm_start_normal_sum,
            self.last_contact_warm_start_tangent_sum,
            self.last_contact_normal_impulse_sum,
            self.last_contact_tangent_impulse_sum,
            self.last_contact_initial_normal_speed_min,
            self.last_contact_initial_normal_speed_max,
            self.last_contact_normal_speed_min,
            self.last_contact_normal_speed_max,
            self.last_contact_tangent_speed_abs_max,
            self.last_contact_support_friction_impulse_max,
            self.last_contact_support_friction_impulse_sum,
            if self.support_gap_summary.is_empty() {
                "none"
            } else {
                self.support_gap_summary.as_str()
            },
        )
    }
}

fn pressure_window_trace(
    run: &RunResult,
    start_frame: usize,
    end_frame: usize,
) -> PressureWindowTrace {
    let mut trace = PressureWindowTrace {
        start_frame,
        end_frame,
        ..PressureWindowTrace::default()
    };
    let mut warm_start_reason_counts = BTreeMap::<String, usize>::new();

    for frame in run
        .frames
        .iter()
        .filter(|frame| frame.frame_index >= start_frame && frame.frame_index <= end_frame)
    {
        for contact in &frame.snapshot.contacts {
            *warm_start_reason_counts
                .entry(format!("{:?}", contact.warm_start_reason))
                .or_default() += 1;
            let tangent_speed_abs = contact.solver_final_tangent_speed.abs();
            if tangent_speed_abs > trace.max_tangent_speed_abs {
                trace.max_tangent_frame = Some(frame.frame_index);
                trace.max_tangent_body_pair = Some(contact.bodies);
                trace.max_tangent_bodies = contact_body_labels(contact.bodies);
                trace.max_tangent_speed_abs = tangent_speed_abs;
                trace.max_tangent_depth = contact.depth;
                trace.max_tangent_initial_normal_speed = contact.solver_initial_normal_speed;
                trace.max_tangent_final_normal_speed = contact.solver_final_normal_speed;
                trace.max_tangent_normal_impulse = contact.solver_normal_impulse;
                trace.max_tangent_tangent_impulse = contact.solver_tangent_impulse.abs();
                trace.max_tangent_support_friction = contact.solver_support_friction_impulse;
                trace.max_tangent_position_bias = contact.solver_position_bias;
                trace.max_tangent_warm_start_reason = format!("{:?}", contact.warm_start_reason);
                trace.max_tangent_feature_id = format!("{:?}", contact.feature_id);
            }
            if contact.solver_support_friction_impulse > trace.max_support_friction {
                trace.max_support_friction = contact.solver_support_friction_impulse;
                trace.max_support_friction_frame = Some(frame.frame_index);
            }
            if contact.warm_start_anchor_drift > trace.max_anchor_drift {
                trace.max_anchor_drift = contact.warm_start_anchor_drift;
                trace.max_anchor_drift_body_pair = Some(contact.bodies);
                trace.max_anchor_drift_bodies = contact_body_labels(contact.bodies);
                trace.max_normal_anchor_drift = contact.warm_start_normal_anchor_drift;
                trace.max_tangent_anchor_drift = contact.warm_start_tangent_anchor_drift;
                trace.max_anchor_drift_frame = Some(frame.frame_index);
            }
            if contact.solver_position_correction_depth > trace.max_correction_row_consumed_depth {
                trace.max_correction_row_frame = Some(frame.frame_index);
                trace.max_correction_row_body_pair = Some(contact.bodies);
                trace.max_correction_row_bodies = contact_body_labels(contact.bodies);
                trace.max_correction_row_depth = contact.depth;
                trace.max_correction_row_consumed_depth = contact.solver_position_correction_depth;
                trace.max_correction_row_body_a_translation =
                    contact.solver_position_correction_body_a_translation;
                trace.max_correction_row_body_b_translation =
                    contact.solver_position_correction_body_b_translation;
                trace.max_correction_row_final_normal_speed = contact.solver_final_normal_speed;
                trace.max_correction_row_final_tangent_speed = contact.solver_final_tangent_speed;
            }
        }

        for body in frame
            .snapshot
            .bodies
            .iter()
            .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
        {
            let angular_speed = body.angular_velocity.abs();
            if angular_speed > trace.max_angular_speed {
                trace.max_angular_frame = Some(frame.frame_index);
                trace.max_angular_body = Some(format!("{:?}", body.handle));
                trace.max_angular_linear_speed = body.linear_velocity.length();
                trace.max_angular_speed = angular_speed;
            }
        }

        if frame.stats.position_correction_total_translation > trace.max_correction_total {
            trace.max_correction_frame = Some(frame.frame_index);
            trace.max_correction_translation = frame.stats.position_correction_max_translation;
            trace.max_correction_total = frame.stats.position_correction_total_translation;
            trace.max_correction_body_count = frame.stats.position_correction_body_count;
        }

        let churn = frame.diagnostics.stability.contact_churn.entered
            + frame.diagnostics.stability.contact_churn.exited;
        if churn > trace.max_churn {
            trace.max_churn = churn;
            trace.max_churn_frame = Some(frame.frame_index);
        }

        if frame.stats.warm_start_drop_count > trace.max_warm_start_drop {
            trace.max_warm_start_drop = frame.stats.warm_start_drop_count;
            trace.max_warm_start_drop_frame = Some(frame.frame_index);
        }

        let contact_row_count = frame.diagnostics.stability.island.contact_row_count;
        if contact_row_count > trace.max_contact_row_count {
            trace.max_contact_row_count = contact_row_count;
            trace.max_contact_row_frame = Some(frame.frame_index);
        }
    }

    trace.warm_start_reason_counts = warm_start_reason_counts
        .into_iter()
        .map(|(reason, count)| format!("{reason}={count}"))
        .collect();
    trace.row_coupling_summary = pressure_row_coupling_summary(run, &trace);
    trace
}

fn feature_churn_trace(run: &RunResult) -> FeatureChurnTrace {
    let mut trace = FeatureChurnTrace::default();
    let mut feature_index_transitions = BTreeMap::<String, FeatureTransitionTrace>::new();

    for frame_index in 1..run.frames.len() {
        let previous = &run.frames[frame_index - 1];
        let current = &run.frames[frame_index];
        for contact in current
            .snapshot
            .contacts
            .iter()
            .filter(|contact| contact.warm_start_reason == WarmStartCacheReason::MissFeatureId)
        {
            trace.miss_feature_id_count += 1;
            let previous_pair_contacts = previous
                .snapshot
                .contacts
                .iter()
                .filter(|candidate| candidate.colliders == contact.colliders)
                .collect::<Vec<_>>();
            if previous_pair_contacts.is_empty() {
                continue;
            }

            trace.same_pair_previous_count += 1;
            if current.frame_index >= MATRIX_STACK_QUIET_WINDOW_START_FRAME {
                trace.late_same_pair_previous_count += 1;
            }

            if previous_pair_contacts.iter().any(|previous_contact| {
                previous_contact.reduction_reason == contact.reduction_reason
            }) {
                trace.same_reduction_reason_count += 1;
            }
            if previous_pair_contacts
                .iter()
                .any(|previous_contact| same_feature_index(previous_contact, contact))
            {
                trace.same_feature_index_count += 1;
            }

            let Some((previous_contact, point_drift, normal_dot)) = previous_pair_contacts
                .iter()
                .filter_map(|previous_contact| {
                    let point_drift = (contact.point - previous_contact.point).length();
                    let normal_dot = contact
                        .normal
                        .normalized_or_zero()
                        .dot(previous_contact.normal.normalized_or_zero());
                    point_drift
                        .is_finite()
                        .then_some((*previous_contact, point_drift, normal_dot))
                })
                .min_by(|(_, lhs, _), (_, rhs, _)| lhs.partial_cmp(rhs).unwrap())
            else {
                continue;
            };

            if point_drift <= 0.05 && normal_dot >= 0.98 {
                trace.close_world_point_count += 1;
            }
            let has_same_shape_signature =
                same_shape_signature(previous, current, previous_contact, contact);
            if has_same_shape_signature {
                trace.same_shape_signature_count += 1;
            }
            let local_anchor_drift =
                max_local_anchor_drift(previous, current, previous_contact, contact);
            let has_close_local_anchor = local_anchor_drift
                .is_some_and(|drift| drift <= FEATURE_CHURN_LOCAL_ANCHOR_DRIFT_THRESHOLD);
            if local_anchor_drift
                .is_some_and(|drift| drift <= FEATURE_CHURN_LOCAL_ANCHOR_DRIFT_THRESHOLD)
            {
                trace.close_local_anchor_count += 1;
            }
            let is_edge_swap = feature_index_edge_swap(previous_contact, contact);
            if is_edge_swap {
                trace.edge_swap_transition_count += 1;
                if trace.edge_swap_best_frame.is_none()
                    || point_drift < trace.edge_swap_best_point_drift
                {
                    trace.edge_swap_best_frame = Some(current.frame_index);
                    trace.edge_swap_best_pair = contact_collider_labels(contact.colliders);
                    trace.edge_swap_best_current_feature_id = format!("{:?}", contact.feature_id);
                    trace.edge_swap_best_previous_feature_id =
                        format!("{:?}", previous_contact.feature_id);
                    trace.edge_swap_best_point_drift = point_drift;
                    trace.edge_swap_best_normal_dot = normal_dot;
                    trace.edge_swap_best_local_anchor_drift = local_anchor_drift.unwrap_or(0.0);
                }
            }
            if is_edge_swap
                && previous_contact.reduction_reason == contact.reduction_reason
                && point_drift <= 0.05
                && normal_dot >= 0.98
                && has_same_shape_signature
                && has_close_local_anchor
            {
                trace.edge_swap_candidate_count += 1;
            }
            if let Some(transition) = feature_index_transition(previous_contact, contact) {
                let transition_trace = feature_index_transitions.entry(transition).or_default();
                transition_trace.count += 1;
                if transition_trace.best_frame.is_none()
                    || point_drift < transition_trace.best_point_drift
                {
                    transition_trace.best_frame = Some(current.frame_index);
                    transition_trace.best_pair = contact_collider_labels(contact.colliders);
                    transition_trace.best_current_feature_id = format!("{:?}", contact.feature_id);
                    transition_trace.best_previous_feature_id =
                        format!("{:?}", previous_contact.feature_id);
                    transition_trace.best_point_drift = point_drift;
                    transition_trace.best_normal_dot = normal_dot;
                    transition_trace.best_local_anchor_drift = local_anchor_drift.unwrap_or(0.0);
                }
            }
            if previous_pair_contacts.iter().any(|previous_contact| {
                let candidate_point_drift = (contact.point - previous_contact.point).length();
                let candidate_normal_dot = contact
                    .normal
                    .normalized_or_zero()
                    .dot(previous_contact.normal.normalized_or_zero());
                previous_contact.reduction_reason == contact.reduction_reason
                    && same_feature_index(previous_contact, contact)
                    && candidate_point_drift <= 0.05
                    && candidate_normal_dot >= 0.98
                    && max_local_anchor_drift(previous, current, previous_contact, contact)
                        .is_some_and(|drift| drift <= FEATURE_CHURN_LOCAL_ANCHOR_DRIFT_THRESHOLD)
            }) {
                trace.point_slot_fallback_eligible_count += 1;
            }
            if trace.best_frame.is_none() || point_drift < trace.best_point_drift {
                trace.best_frame = Some(current.frame_index);
                trace.best_pair = contact_collider_labels(contact.colliders);
                trace.best_current_feature_id = format!("{:?}", contact.feature_id);
                trace.best_previous_feature_id = format!("{:?}", previous_contact.feature_id);
                trace.best_point_drift = point_drift;
                trace.best_normal_dot = normal_dot;
                trace.best_local_anchor_drift = local_anchor_drift.unwrap_or(0.0);
            }
        }
    }

    if let Some((transition, transition_trace)) = feature_index_transitions.into_iter().max_by(
        |(lhs_transition, lhs_trace), (rhs_transition, rhs_trace)| {
            lhs_trace
                .count
                .cmp(&rhs_trace.count)
                .then_with(|| rhs_transition.cmp(lhs_transition))
        },
    ) {
        trace.top_feature_index_transition = transition;
        trace.top_feature_index_transition_count = transition_trace.count;
        trace.top_transition_best_frame = transition_trace.best_frame;
        trace.top_transition_best_pair = transition_trace.best_pair;
        trace.top_transition_best_current_feature_id = transition_trace.best_current_feature_id;
        trace.top_transition_best_previous_feature_id = transition_trace.best_previous_feature_id;
        trace.top_transition_best_point_drift = transition_trace.best_point_drift;
        trace.top_transition_best_normal_dot = transition_trace.best_normal_dot;
        trace.top_transition_best_local_anchor_drift = transition_trace.best_local_anchor_drift;
    }

    trace
}

fn same_feature_index(previous_contact: &DebugContact, current_contact: &DebugContact) -> bool {
    contact_feature_index(previous_contact) == contact_feature_index(current_contact)
}

fn contact_feature_index(contact: &DebugContact) -> Option<u64> {
    contact_feature_raw(contact).map(|raw| raw & u32::MAX as u64)
}

fn contact_feature_raw(contact: &DebugContact) -> Option<u64> {
    let debug = format!("{:?}", contact.feature_id);
    debug
        .strip_prefix("ContactFeatureId(")?
        .strip_suffix(')')?
        .parse::<u64>()
        .ok()
}

fn feature_index_transition(
    previous_contact: &DebugContact,
    current_contact: &DebugContact,
) -> Option<String> {
    let previous = contact_feature_index(previous_contact)?;
    let current = contact_feature_index(current_contact)?;
    Some(format!(
        "{}<-{}",
        decoded_feature_index(current),
        decoded_feature_index(previous)
    ))
}

fn feature_index_edge_swap(
    previous_contact: &DebugContact,
    current_contact: &DebugContact,
) -> bool {
    let Some(previous) = decoded_contact_feature(previous_contact) else {
        return false;
    };
    let Some(current) = decoded_contact_feature(current_contact) else {
        return false;
    };
    previous.kind == current.kind
        && previous.reference_edge == current.incident_edge
        && previous.incident_edge == current.reference_edge
        && previous.reference_edge != previous.incident_edge
}

fn decoded_feature_index(index: u64) -> String {
    let decoded = decode_feature_index(index);
    format!(
        "k{}:r{}:i{}",
        decoded.kind, decoded.reference_edge, decoded.incident_edge
    )
}

#[derive(Clone, Copy)]
struct DecodedFeatureIndex {
    kind: u64,
    reference_edge: u64,
    incident_edge: u64,
}

fn decoded_contact_feature(contact: &DebugContact) -> Option<DecodedFeatureIndex> {
    contact_feature_index(contact).map(decode_feature_index)
}

fn decode_feature_index(index: u64) -> DecodedFeatureIndex {
    DecodedFeatureIndex {
        kind: (index >> 24) & 0xff,
        reference_edge: (index >> 12) & 0xfff,
        incident_edge: index & 0xfff,
    }
}

fn same_shape_signature(
    previous_frame: &FrameRecord,
    current_frame: &FrameRecord,
    previous_contact: &DebugContact,
    current_contact: &DebugContact,
) -> bool {
    previous_contact
        .colliders
        .iter()
        .zip(current_contact.colliders)
        .all(|(previous_handle, current_handle)| {
            let Some(previous_collider) = collider_by_handle(previous_frame, *previous_handle)
            else {
                return false;
            };
            let Some(current_collider) = collider_by_handle(current_frame, current_handle) else {
                return false;
            };
            shape_signature(previous_collider) == shape_signature(current_collider)
        })
}

fn max_local_anchor_drift(
    previous_frame: &FrameRecord,
    current_frame: &FrameRecord,
    previous_contact: &DebugContact,
    current_contact: &DebugContact,
) -> Option<f32> {
    previous_contact
        .colliders
        .iter()
        .zip(current_contact.colliders)
        .map(|(previous_handle, current_handle)| {
            let previous_collider = collider_by_handle(previous_frame, *previous_handle)?;
            let current_collider = collider_by_handle(current_frame, current_handle)?;
            let previous_anchor = local_contact_anchor(previous_collider, previous_contact);
            let current_anchor = local_contact_anchor(current_collider, current_contact);
            Some((current_anchor - previous_anchor).length())
        })
        .try_fold(0.0_f32, |max_drift, drift| {
            drift.map(|drift| max_drift.max(drift))
        })
}

fn local_contact_anchor(
    collider: &DebugCollider,
    contact: &DebugContact,
) -> picea::math::vector::Vector {
    let world_offset = contact.point - collider.world_transform.translation;
    picea::math::vector::Vector::from(world_offset).rotated(-collider.world_transform.rotation)
}

fn collider_by_handle(frame: &FrameRecord, handle: ColliderHandle) -> Option<&DebugCollider> {
    frame
        .snapshot
        .colliders
        .iter()
        .find(|collider| collider.handle == handle)
}

fn shape_signature(collider: &DebugCollider) -> String {
    match &collider.shape {
        DebugShape::Circle { radius, .. } => {
            format!("circle:r{}", quantized_signature_part(*radius))
        }
        DebugShape::Polygon { vertices } => {
            let mut edge_lengths = vertices
                .iter()
                .zip(vertices.iter().cycle().skip(1))
                .take(vertices.len())
                .map(|(start, end)| quantized_signature_part((*end - *start).length()))
                .collect::<Vec<_>>();
            edge_lengths.sort_unstable();
            format!("polygon:{}:{edge_lengths:?}", vertices.len())
        }
        DebugShape::Segment { start, end, radius } => {
            let length = (*end - *start).length();
            format!(
                "segment:l{}:r{}",
                quantized_signature_part(length),
                quantized_signature_part(*radius)
            )
        }
    }
}

fn quantized_signature_part(value: f32) -> i32 {
    (value * 1000.0).round() as i32
}

fn pressure_row_coupling_summary(run: &RunResult, trace: &PressureWindowTrace) -> String {
    [
        row_coupling_part(
            "correction_tangent",
            run,
            trace.max_correction_row_frame,
            trace.max_correction_row_body_pair,
            trace.max_tangent_frame,
            trace.max_tangent_body_pair,
        ),
        row_coupling_part(
            "correction_anchor",
            run,
            trace.max_correction_row_frame,
            trace.max_correction_row_body_pair,
            trace.max_anchor_drift_frame,
            trace.max_anchor_drift_body_pair,
        ),
        row_coupling_part(
            "tangent_anchor",
            run,
            trace.max_tangent_frame,
            trace.max_tangent_body_pair,
            trace.max_anchor_drift_frame,
            trace.max_anchor_drift_body_pair,
        ),
    ]
    .join("; ")
}

fn row_coupling_part(
    label: &str,
    run: &RunResult,
    frame_a: Option<usize>,
    pair_a: Option<[BodyHandle; 2]>,
    frame_b: Option<usize>,
    pair_b: Option<[BodyHandle; 2]>,
) -> String {
    let frame_delta = frame_a.zip(frame_b).map(|(a, b)| a.abs_diff(b));
    let comparison_frame = frame_a.zip(frame_b).map(|(a, b)| a.max(b));
    let graph_distance = contact_graph_distance_at_frame(run, comparison_frame, pair_a, pair_b);
    let shared_bodies = pair_a
        .zip(pair_b)
        .map(|(a, b)| shared_body_count(a, b))
        .unwrap_or_default();
    format!(
        "{label} frame_delta {} graph_distance {}@f{} shared_bodies {}",
        option_usize(frame_delta),
        option_usize(graph_distance),
        option_frame(comparison_frame),
        shared_bodies
    )
}

fn contact_graph_distance_at_frame(
    run: &RunResult,
    frame_index: Option<usize>,
    from: Option<[BodyHandle; 2]>,
    to: Option<[BodyHandle; 2]>,
) -> Option<usize> {
    let frame = run
        .frames
        .iter()
        .find(|frame| Some(frame.frame_index) == frame_index)?;
    let from = from?;
    let to = to?;
    if shared_body_count(from, to) > 0 {
        return Some(0);
    }
    contact_pair_graph_distance(frame, from, to)
}

fn contact_pair_graph_distance(
    frame: &FrameRecord,
    from: [BodyHandle; 2],
    to: [BodyHandle; 2],
) -> Option<usize> {
    let mut graph = BTreeMap::<BodyHandle, Vec<BodyHandle>>::new();
    for contact in &frame.snapshot.contacts {
        let [a, b] = contact.bodies;
        graph.entry(a).or_default().push(b);
        graph.entry(b).or_default().push(a);
    }

    let mut queue = VecDeque::<(BodyHandle, usize)>::new();
    let mut visited = Vec::<BodyHandle>::new();
    for body in from {
        queue.push_back((body, 0));
        visited.push(body);
    }

    while let Some((body, distance)) = queue.pop_front() {
        if to.contains(&body) {
            return Some(distance);
        }
        for next in graph.get(&body).into_iter().flatten().copied() {
            if visited.contains(&next) {
                continue;
            }
            visited.push(next);
            queue.push_back((next, distance + 1));
        }
    }
    None
}

fn shared_body_count(a: [BodyHandle; 2], b: [BodyHandle; 2]) -> usize {
    a.into_iter().filter(|body| b.contains(body)).count()
}

fn late_velocity_trace(run: &RunResult, frame_index: usize, body: BodyHandle) -> LateVelocityTrace {
    let Some(frame) = run
        .frames
        .iter()
        .find(|candidate| candidate.frame_index == frame_index)
    else {
        return LateVelocityTrace::default();
    };
    let facts = contact_summary(frame, body);
    let (linear_speed, angular_speed) = body_speeds(frame, body).unwrap_or_default();
    let support_gap = late_support_gap(run, body, frame_index);
    let last_contact = run
        .frames
        .iter()
        .rev()
        .find(|candidate| {
            candidate.frame_index <= frame_index
                && frame_contacts_body(candidate, body).next().is_some()
        })
        .map(|contact_frame| {
            (
                contact_frame.frame_index,
                contact_summary(contact_frame, body),
            )
        });
    LateVelocityTrace {
        frame: Some(frame.frame_index),
        body: Some(format!("{body:?}")),
        linear_speed,
        angular_speed,
        contact_count: facts.contact_count,
        counterparts: facts.counterparts,
        max_depth: facts.max_depth,
        warm_start_normal_sum: facts.warm_start_normal_sum,
        warm_start_tangent_sum: facts.warm_start_tangent_sum,
        normal_impulse_sum: facts.normal_impulse_sum,
        tangent_impulse_sum: facts.tangent_impulse_sum,
        initial_normal_speed_min: facts.initial_normal_speed_min,
        initial_normal_speed_max: facts.initial_normal_speed_max,
        normal_speed_min: facts.normal_speed_min,
        normal_speed_max: facts.normal_speed_max,
        tangent_speed_abs_max: facts.tangent_speed_abs_max,
        support_friction_impulse_max: facts.support_friction_impulse_max,
        support_friction_impulse_sum: facts.support_friction_impulse_sum,
        position_bias_max: facts.position_bias_max,
        restitution_bias_max: facts.restitution_bias_max,
        position_correction_max_translation: frame.stats.position_correction_max_translation,
        position_correction_total_translation: frame.stats.position_correction_total_translation,
        position_correction_body_count: frame.stats.position_correction_body_count,
        warm_start_drop_count: frame.stats.warm_start_drop_count,
        contact_churn: frame.diagnostics.stability.contact_churn.entered
            + frame.diagnostics.stability.contact_churn.exited,
        last_contact_frame: last_contact.as_ref().map(|(frame, _)| *frame),
        last_contact_count: last_contact
            .as_ref()
            .map(|(_, facts)| facts.contact_count)
            .unwrap_or(0),
        last_contact_counterparts: last_contact
            .as_ref()
            .map(|(_, facts)| facts.counterparts.clone())
            .unwrap_or_default(),
        last_contact_max_depth: last_contact
            .as_ref()
            .map(|(_, facts)| facts.max_depth)
            .unwrap_or(0.0),
        last_contact_warm_start_normal_sum: last_contact
            .as_ref()
            .map(|(_, facts)| facts.warm_start_normal_sum)
            .unwrap_or(0.0),
        last_contact_warm_start_tangent_sum: last_contact
            .as_ref()
            .map(|(_, facts)| facts.warm_start_tangent_sum)
            .unwrap_or(0.0),
        last_contact_normal_impulse_sum: last_contact
            .as_ref()
            .map(|(_, facts)| facts.normal_impulse_sum)
            .unwrap_or(0.0),
        last_contact_tangent_impulse_sum: last_contact
            .as_ref()
            .map(|(_, facts)| facts.tangent_impulse_sum)
            .unwrap_or(0.0),
        last_contact_initial_normal_speed_min: last_contact
            .as_ref()
            .map(|(_, facts)| facts.initial_normal_speed_min)
            .unwrap_or(0.0),
        last_contact_initial_normal_speed_max: last_contact
            .as_ref()
            .map(|(_, facts)| facts.initial_normal_speed_max)
            .unwrap_or(0.0),
        last_contact_normal_speed_min: last_contact
            .as_ref()
            .map(|(_, facts)| facts.normal_speed_min)
            .unwrap_or(0.0),
        last_contact_normal_speed_max: last_contact
            .as_ref()
            .map(|(_, facts)| facts.normal_speed_max)
            .unwrap_or(0.0),
        last_contact_tangent_speed_abs_max: last_contact
            .as_ref()
            .map(|(_, facts)| facts.tangent_speed_abs_max)
            .unwrap_or(0.0),
        last_contact_support_friction_impulse_max: last_contact
            .as_ref()
            .map(|(_, facts)| facts.support_friction_impulse_max)
            .unwrap_or(0.0),
        last_contact_support_friction_impulse_sum: last_contact
            .as_ref()
            .map(|(_, facts)| facts.support_friction_impulse_sum)
            .unwrap_or(0.0),
        support_gap_start_frame: support_gap.start_frame,
        support_gap_duration: support_gap.duration,
        support_gap_previous_normal_impulse_sum: support_gap.previous_normal_impulse_sum,
        support_gap_previous_normal_speed_min: support_gap.previous_normal_speed_min,
        support_gap_previous_body_linear_speed: support_gap.previous_body_linear_speed,
        support_gap_previous_body_angular_speed: support_gap.previous_body_angular_speed,
        support_gap_previous_counterpart_linear_speed_max: support_gap
            .previous_counterpart_linear_speed_max,
        support_gap_previous_counterpart_angular_speed_max: support_gap
            .previous_counterpart_angular_speed_max,
        support_gap_previous_body_tangent_linear_component_max: support_gap
            .previous_body_tangent_linear_component_max,
        support_gap_previous_body_tangent_angular_component_max: support_gap
            .previous_body_tangent_angular_component_max,
        support_gap_previous_counterpart_tangent_linear_component_max: support_gap
            .previous_counterpart_tangent_linear_component_max,
        support_gap_previous_counterpart_tangent_angular_component_max: support_gap
            .previous_counterpart_tangent_angular_component_max,
        support_gap_energy_onset_frame: support_gap.energy_onset_frame,
        support_gap_energy_onset_body_tangent_linear_component_max: support_gap
            .energy_onset_body_tangent_linear_component_max,
        support_gap_energy_onset_body_tangent_angular_component_max: support_gap
            .energy_onset_body_tangent_angular_component_max,
        support_gap_energy_onset_counterpart_tangent_linear_component_max: support_gap
            .energy_onset_counterpart_tangent_linear_component_max,
        support_gap_energy_onset_counterpart_tangent_angular_component_max: support_gap
            .energy_onset_counterpart_tangent_angular_component_max,
        support_gap_energy_onset_normal_impulse_sum: support_gap.energy_onset_normal_impulse_sum,
        support_gap_energy_onset_normal_speed_min: support_gap.energy_onset_normal_speed_min,
        support_gap_energy_onset_max_depth: support_gap.energy_onset_max_depth,
        support_gap_lifecycle: support_gap.lifecycle,
        support_gap_previous_position_correction_depth_sum: support_gap
            .previous_position_correction_depth_sum,
        support_gap_previous_position_correction_translation_sum: support_gap
            .previous_position_correction_translation_sum,
        support_gap_summary: support_gap.summary,
    }
}

#[derive(Default)]
struct LateSupportGapTrace {
    start_frame: Option<usize>,
    duration: usize,
    previous_normal_impulse_sum: f32,
    previous_normal_speed_min: f32,
    previous_body_linear_speed: f32,
    previous_body_angular_speed: f32,
    previous_counterpart_linear_speed_max: f32,
    previous_counterpart_angular_speed_max: f32,
    previous_body_tangent_linear_component_max: f32,
    previous_body_tangent_angular_component_max: f32,
    previous_counterpart_tangent_linear_component_max: f32,
    previous_counterpart_tangent_angular_component_max: f32,
    energy_onset_frame: Option<usize>,
    energy_onset_body_tangent_linear_component_max: f32,
    energy_onset_body_tangent_angular_component_max: f32,
    energy_onset_counterpart_tangent_linear_component_max: f32,
    energy_onset_counterpart_tangent_angular_component_max: f32,
    energy_onset_normal_impulse_sum: f32,
    energy_onset_normal_speed_min: f32,
    energy_onset_max_depth: f32,
    lifecycle: Vec<SupportGapLifecycleFrame>,
    previous_position_correction_depth_sum: f32,
    previous_position_correction_translation_sum: f32,
    summary: String,
}

fn late_support_gap(run: &RunResult, body: BodyHandle, frame_index: usize) -> LateSupportGapTrace {
    let Some(spike_frame) = run
        .frames
        .iter()
        .find(|frame| frame.frame_index == frame_index)
    else {
        return LateSupportGapTrace::default();
    };
    if frame_contacts_body(spike_frame, body).next().is_some() {
        return LateSupportGapTrace::default();
    }

    let Some(previous_contact_frame) = run.frames.iter().rev().find(|frame| {
        frame.frame_index < frame_index && frame_contacts_body(frame, body).next().is_some()
    }) else {
        return LateSupportGapTrace::default();
    };
    let gap_start_frame = previous_contact_frame.frame_index + 1;
    let previous_facts = contact_summary(previous_contact_frame, body);
    let energy_onset =
        support_gap_energy_onset(run, body, previous_contact_frame.frame_index).unwrap_or_default();
    let (previous_body_linear, previous_body_angular) =
        body_speeds(previous_contact_frame, body).unwrap_or_default();
    let (gap_start_linear, gap_start_angular) = run
        .frames
        .iter()
        .find(|frame| frame.frame_index == gap_start_frame)
        .and_then(|frame| body_speeds(frame, body))
        .unwrap_or_default();
    let (spike_linear, spike_angular) = body_speeds(spike_frame, body).unwrap_or_default();
    let duration = frame_index.saturating_sub(gap_start_frame) + 1;
    let lifecycle = energy_onset
        .frame
        .map(|start_frame| {
            support_gap_lifecycle(run, body, start_frame, previous_contact_frame.frame_index)
        })
        .unwrap_or_default();
    let lifecycle_summary = support_gap_lifecycle_summary(&lifecycle);
    let summary = format!(
        "start frame {} duration {} speed {:.6}/{:.6}->{:.6}/{:.6}; previous frame {} previous_speed {:.6}/{:.6} counterpart_speed {:.6}/{:.6} tangent_components body {:.6}/{:.6} counterpart {:.6}/{:.6}; energy_onset frame {} tangent_components body {:.6}/{:.6} counterpart {:.6}/{:.6} impulse {:.6} normal_speed_min {:.6} max_depth {:.6}; lifecycle [{}]; counterparts [{}] max_depth {:.6} warm {:.6}/{:.6} impulse {:.6}/{:.6} initial_speed {:.6}..{:.6} post_speed {:.6}..{:.6}/{:.6} bias {:.6}/{:.6} correction_depth {:.6} correction_translation {:.6}",
        gap_start_frame,
        duration,
        gap_start_linear,
        gap_start_angular,
        spike_linear,
        spike_angular,
        previous_contact_frame.frame_index,
        previous_body_linear,
        previous_body_angular,
        previous_facts.counterpart_linear_speed_max,
        previous_facts.counterpart_angular_speed_max,
        previous_facts.body_tangent_linear_component_max,
        previous_facts.body_tangent_angular_component_max,
        previous_facts.counterpart_tangent_linear_component_max,
        previous_facts.counterpart_tangent_angular_component_max,
        option_frame(energy_onset.frame),
        energy_onset.body_tangent_linear_component_max,
        energy_onset.body_tangent_angular_component_max,
        energy_onset.counterpart_tangent_linear_component_max,
        energy_onset.counterpart_tangent_angular_component_max,
        energy_onset.normal_impulse_sum,
        energy_onset.normal_speed_min,
        energy_onset.max_depth,
        lifecycle_summary,
        previous_facts.counterparts.join(", "),
        previous_facts.max_depth,
        previous_facts.warm_start_normal_sum,
        previous_facts.warm_start_tangent_sum,
        previous_facts.normal_impulse_sum,
        previous_facts.tangent_impulse_sum,
        previous_facts.initial_normal_speed_min,
        previous_facts.initial_normal_speed_max,
        previous_facts.normal_speed_min,
        previous_facts.normal_speed_max,
        previous_facts.tangent_speed_abs_max,
        previous_facts.position_bias_max,
        previous_facts.restitution_bias_max,
        previous_facts.position_correction_depth_sum,
        previous_facts.position_correction_translation_sum,
    );
    LateSupportGapTrace {
        start_frame: Some(gap_start_frame),
        duration,
        previous_normal_impulse_sum: previous_facts.normal_impulse_sum,
        previous_normal_speed_min: previous_facts.normal_speed_min,
        previous_body_linear_speed: previous_body_linear,
        previous_body_angular_speed: previous_body_angular,
        previous_counterpart_linear_speed_max: previous_facts.counterpart_linear_speed_max,
        previous_counterpart_angular_speed_max: previous_facts.counterpart_angular_speed_max,
        previous_body_tangent_linear_component_max: previous_facts
            .body_tangent_linear_component_max,
        previous_body_tangent_angular_component_max: previous_facts
            .body_tangent_angular_component_max,
        previous_counterpart_tangent_linear_component_max: previous_facts
            .counterpart_tangent_linear_component_max,
        previous_counterpart_tangent_angular_component_max: previous_facts
            .counterpart_tangent_angular_component_max,
        energy_onset_frame: energy_onset.frame,
        energy_onset_body_tangent_linear_component_max: energy_onset
            .body_tangent_linear_component_max,
        energy_onset_body_tangent_angular_component_max: energy_onset
            .body_tangent_angular_component_max,
        energy_onset_counterpart_tangent_linear_component_max: energy_onset
            .counterpart_tangent_linear_component_max,
        energy_onset_counterpart_tangent_angular_component_max: energy_onset
            .counterpart_tangent_angular_component_max,
        energy_onset_normal_impulse_sum: energy_onset.normal_impulse_sum,
        energy_onset_normal_speed_min: energy_onset.normal_speed_min,
        energy_onset_max_depth: energy_onset.max_depth,
        lifecycle,
        previous_position_correction_depth_sum: previous_facts.position_correction_depth_sum,
        previous_position_correction_translation_sum: previous_facts
            .position_correction_translation_sum,
        summary,
    }
}

#[derive(Default)]
struct SupportGapEnergyOnset {
    frame: Option<usize>,
    body_tangent_linear_component_max: f32,
    body_tangent_angular_component_max: f32,
    counterpart_tangent_linear_component_max: f32,
    counterpart_tangent_angular_component_max: f32,
    normal_impulse_sum: f32,
    normal_speed_min: f32,
    max_depth: f32,
}

#[derive(Clone, Debug)]
struct SupportGapLifecycleFrame {
    frame: usize,
    contact_count: usize,
    max_depth: f32,
    normal_impulse_sum: f32,
    normal_speed_min: f32,
    normal_speed_max: f32,
    tangent_speed_abs_max: f32,
    body_tangent_linear_component_max: f32,
    body_tangent_angular_component_max: f32,
    normal_anchor_drift_max: f32,
    tangent_anchor_drift_max: f32,
    position_correction_depth_sum: f32,
    position_correction_translation_sum: f32,
    eligibility_decision: String,
}

fn support_gap_energy_onset(
    run: &RunResult,
    body: BodyHandle,
    last_contact_frame_index: usize,
) -> Option<SupportGapEnergyOnset> {
    run.frames
        .iter()
        .filter(|frame| frame.frame_index <= last_contact_frame_index)
        .filter_map(|frame| {
            let facts = contact_summary(frame, body);
            let has_linear_onset = facts.body_tangent_linear_component_max
                > SUPPORT_GAP_TANGENT_LINEAR_ONSET_THRESHOLD;
            let has_angular_onset = facts.body_tangent_angular_component_max
                > SUPPORT_GAP_TANGENT_ANGULAR_ONSET_THRESHOLD;
            if !has_linear_onset || !has_angular_onset {
                return None;
            }
            Some(SupportGapEnergyOnset {
                frame: Some(frame.frame_index),
                body_tangent_linear_component_max: facts.body_tangent_linear_component_max,
                body_tangent_angular_component_max: facts.body_tangent_angular_component_max,
                counterpart_tangent_linear_component_max: facts
                    .counterpart_tangent_linear_component_max,
                counterpart_tangent_angular_component_max: facts
                    .counterpart_tangent_angular_component_max,
                normal_impulse_sum: facts.normal_impulse_sum,
                normal_speed_min: facts.normal_speed_min,
                max_depth: facts.max_depth,
            })
        })
        .next()
}

fn support_gap_lifecycle(
    run: &RunResult,
    body: BodyHandle,
    start_frame_index: usize,
    end_frame_index: usize,
) -> Vec<SupportGapLifecycleFrame> {
    run.frames
        .iter()
        .filter(|frame| {
            frame.frame_index >= start_frame_index && frame.frame_index <= end_frame_index
        })
        .filter_map(|frame| {
            let facts = contact_summary(frame, body);
            if facts.contact_count == 0 {
                return None;
            }
            Some(SupportGapLifecycleFrame {
                frame: frame.frame_index,
                contact_count: facts.contact_count,
                max_depth: facts.max_depth,
                normal_impulse_sum: facts.normal_impulse_sum,
                normal_speed_min: facts.normal_speed_min,
                normal_speed_max: facts.normal_speed_max,
                tangent_speed_abs_max: facts.tangent_speed_abs_max,
                body_tangent_linear_component_max: facts.body_tangent_linear_component_max,
                body_tangent_angular_component_max: facts.body_tangent_angular_component_max,
                normal_anchor_drift_max: facts.normal_anchor_drift_max,
                tangent_anchor_drift_max: facts.tangent_anchor_drift_max,
                position_correction_depth_sum: facts.position_correction_depth_sum,
                position_correction_translation_sum: facts.position_correction_translation_sum,
                eligibility_decision: support_eligibility_decision(&facts),
            })
        })
        .collect()
}

fn support_gap_lifecycle_summary(frames: &[SupportGapLifecycleFrame]) -> String {
    if frames.is_empty() {
        return "none".to_owned();
    }
    frames
        .iter()
        .map(|frame| {
            format!(
                "f{} c{} depth {:.6} impulse {:.6} normal {:.6}..{:.6} tangent {:.6} body_tangent {:.6}/{:.6} drift {:.6}/{:.6} correction {:.6}/{:.6} eligibility {}",
                frame.frame,
                frame.contact_count,
                frame.max_depth,
                frame.normal_impulse_sum,
                frame.normal_speed_min,
                frame.normal_speed_max,
                frame.tangent_speed_abs_max,
                frame.body_tangent_linear_component_max,
                frame.body_tangent_angular_component_max,
                frame.normal_anchor_drift_max,
                frame.tangent_anchor_drift_max,
                frame.position_correction_depth_sum,
                frame.position_correction_translation_sum,
                frame.eligibility_decision,
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn support_eligibility_decision(facts: &ContactSummary) -> String {
    if facts.normal_impulse_sum > f32::EPSILON {
        return "observe_normal_impulse_present".to_owned();
    }
    if facts.normal_speed_min <= 0.0 {
        return "observe_not_separating".to_owned();
    }
    if facts.counterpart_tangent_linear_component_max
        > SUPPORT_ELIGIBILITY_COUNTERPART_TANGENT_THRESHOLD
        || facts.counterpart_tangent_angular_component_max
            > SUPPORT_ELIGIBILITY_COUNTERPART_TANGENT_THRESHOLD
    {
        return "reject_counterpart_motion".to_owned();
    }
    if facts.body_tangent_linear_component_max <= SUPPORT_GAP_TANGENT_LINEAR_ONSET_THRESHOLD
        || facts.body_tangent_angular_component_max <= SUPPORT_GAP_TANGENT_ANGULAR_ONSET_THRESHOLD
    {
        return "observe_tangent_risk_low".to_owned();
    }
    if facts.normal_anchor_drift_max > SUPPORT_ELIGIBILITY_NORMAL_ANCHOR_DRIFT_THRESHOLD {
        return "reject_normal_anchor_drift".to_owned();
    }
    if facts.tangent_anchor_drift_max > SUPPORT_ELIGIBILITY_TANGENT_ANCHOR_DRIFT_THRESHOLD {
        return "reject_tangent_anchor_drift".to_owned();
    }
    if facts.position_correction_depth_sum > 0.0 {
        "retain_candidate_correction_active".to_owned()
    } else {
        "retain_candidate_needs_position_re_evaluation".to_owned()
    }
}

fn matrix_stack_ejection_trace(
    run: &RunResult,
    first_floor_exit: Option<(usize, BodyHandle, f32)>,
) -> EjectionTrace {
    let Some((exit_frame, exit_body, _)) = first_floor_exit else {
        return EjectionTrace::default();
    };
    let pre_exit_frame = exit_frame.saturating_sub(1);
    let mut trace = EjectionTrace {
        pre_exit_frame: Some(pre_exit_frame),
        ..EjectionTrace::default()
    };
    if let Some(frame) = run
        .frames
        .iter()
        .find(|frame| frame.frame_index == pre_exit_frame)
    {
        if let Some(body) = frame
            .snapshot
            .bodies
            .iter()
            .find(|body| body.handle == exit_body)
        {
            trace.pre_exit_x = Some(body.transform.translation.x());
            trace.pre_exit_linear_speed = body.linear_velocity.length();
            trace.pre_exit_angular_speed = body.angular_velocity.abs();
        }
    }
    if let Some(frame) = run
        .frames
        .iter()
        .filter(|frame| frame.frame_index <= pre_exit_frame)
        .max_by_key(|frame| frame.diagnostics.stability.island.contact_row_count)
    {
        trace.row_max_frame = Some(frame.frame_index);
        if let Some((linear, angular)) = body_speeds(frame, exit_body) {
            trace.row_max_linear_speed = linear;
            trace.row_max_angular_speed = angular;
        }
    }

    if let Some(frame) = run.frames.iter().rev().find(|frame| {
        frame.frame_index <= pre_exit_frame
            && frame
                .snapshot
                .contacts
                .iter()
                .any(|contact| contact.bodies.contains(&exit_body))
    }) {
        trace.last_contact_frame = Some(frame.frame_index);
        let contacts = frame
            .snapshot
            .contacts
            .iter()
            .filter(|contact| contact.bodies.contains(&exit_body))
            .collect::<Vec<_>>();
        trace.last_contact_count = contacts.len();
        trace.last_contact_counterparts = contacts
            .iter()
            .filter_map(|contact| {
                contact
                    .bodies
                    .iter()
                    .copied()
                    .find(|body| *body != exit_body)
            })
            .map(|body| format!("{body:?}"))
            .collect();
        trace.last_contact_counterparts.sort();
        trace.last_contact_counterparts.dedup();
        trace.last_contact_max_depth = contacts
            .iter()
            .map(|contact| contact.depth)
            .fold(0.0, f32::max);
        trace.last_contact_normal_impulse_sum = contacts
            .iter()
            .map(|contact| contact.solver_normal_impulse)
            .sum();
        trace.last_contact_tangent_impulse_sum = contacts
            .iter()
            .map(|contact| contact.solver_tangent_impulse.abs())
            .sum();
    }
    if let Some(frame) =
        run.frames.iter().rev().find(|frame| {
            frame.frame_index <= pre_exit_frame
                && frame.snapshot.contacts.iter().any(|contact| {
                    contact_has_dynamic_counterpart(frame, contact.bodies, exit_body)
                })
        })
    {
        trace.last_dynamic_contact_frame = Some(frame.frame_index);
        let contacts = frame
            .snapshot
            .contacts
            .iter()
            .filter(|contact| contact_has_dynamic_counterpart(frame, contact.bodies, exit_body))
            .collect::<Vec<_>>();
        trace.last_dynamic_contact_count = contacts.len();
        trace.last_dynamic_contact_counterparts = contacts
            .iter()
            .filter_map(|contact| {
                contact
                    .bodies
                    .iter()
                    .copied()
                    .find(|body| *body != exit_body)
            })
            .map(|body| format!("{body:?}"))
            .collect();
        trace.last_dynamic_contact_counterparts.sort();
        trace.last_dynamic_contact_counterparts.dedup();
        trace.last_dynamic_contact_max_depth = contacts
            .iter()
            .map(|contact| contact.depth)
            .fold(0.0, f32::max);
        trace.last_dynamic_contact_normal_impulse_sum = contacts
            .iter()
            .map(|contact| contact.solver_normal_impulse)
            .sum();
        trace.last_dynamic_contact_tangent_impulse_sum = contacts
            .iter()
            .map(|contact| contact.solver_tangent_impulse.abs())
            .sum();
        trace.last_dynamic_contact_warm_start_normal_sum =
            contacts.iter().map(|contact| contact.normal_impulse).sum();
        trace.last_dynamic_contact_warm_start_tangent_sum = contacts
            .iter()
            .map(|contact| contact.tangent_impulse.abs())
            .sum();
        trace.last_dynamic_contact_warm_start_reasons = contacts
            .iter()
            .map(|contact| format!("{:?}", contact.warm_start_reason))
            .collect();
        trace.last_dynamic_contact_warm_start_reasons.sort();
        trace.last_dynamic_contact_warm_start_reasons.dedup();
        let relative_speeds = contacts
            .iter()
            .map(|contact| contact_solver_final_speeds(contact))
            .collect::<Vec<_>>();
        trace.last_dynamic_contact_normal_speed_min = relative_speeds
            .iter()
            .map(|(normal_speed, _)| *normal_speed)
            .reduce(f32::min)
            .unwrap_or(0.0);
        trace.last_dynamic_contact_normal_speed_max = relative_speeds
            .iter()
            .map(|(normal_speed, _)| *normal_speed)
            .reduce(f32::max)
            .unwrap_or(0.0);
        trace.last_dynamic_contact_tangent_speed_abs_max = relative_speeds
            .iter()
            .map(|(_, tangent_speed)| tangent_speed.abs())
            .reduce(f32::max)
            .unwrap_or(0.0);
        if let Some((linear, angular)) = body_speeds(frame, exit_body) {
            trace.last_dynamic_contact_body_linear_speed = linear;
            trace.last_dynamic_contact_body_angular_speed = angular;
        }
    }
    if let Some(frame) = run.frames.iter().rev().find(|frame| {
        frame.frame_index <= pre_exit_frame
            && frame.snapshot.contacts.iter().any(|contact| {
                contact_has_dynamic_counterpart(frame, contact.bodies, exit_body)
                    && contact.solver_normal_impulse > f32::EPSILON
            })
    }) {
        trace.last_dynamic_impulse_frame = Some(frame.frame_index);
        let contacts = frame
            .snapshot
            .contacts
            .iter()
            .filter(|contact| {
                contact_has_dynamic_counterpart(frame, contact.bodies, exit_body)
                    && contact.solver_normal_impulse > f32::EPSILON
            })
            .collect::<Vec<_>>();
        trace.last_dynamic_impulse_counterparts = contacts
            .iter()
            .filter_map(|contact| {
                contact
                    .bodies
                    .iter()
                    .copied()
                    .find(|body| *body != exit_body)
            })
            .map(|body| format!("{body:?}"))
            .collect();
        trace.last_dynamic_impulse_counterparts.sort();
        trace.last_dynamic_impulse_counterparts.dedup();
        trace.last_dynamic_impulse_normal_sum = contacts
            .iter()
            .map(|contact| contact.solver_normal_impulse)
            .sum();
        trace.last_dynamic_impulse_tangent_sum = contacts
            .iter()
            .map(|contact| contact.solver_tangent_impulse.abs())
            .sum();
        let relative_speeds = contacts
            .iter()
            .map(|contact| contact_solver_final_speeds(contact))
            .collect::<Vec<_>>();
        trace.last_dynamic_impulse_normal_speed_min = relative_speeds
            .iter()
            .map(|(normal_speed, _)| *normal_speed)
            .reduce(f32::min)
            .unwrap_or(0.0);
        trace.last_dynamic_impulse_normal_speed_max = relative_speeds
            .iter()
            .map(|(normal_speed, _)| *normal_speed)
            .reduce(f32::max)
            .unwrap_or(0.0);
        trace.last_dynamic_impulse_tangent_speed_abs_max = relative_speeds
            .iter()
            .map(|(_, tangent_speed)| tangent_speed.abs())
            .reduce(f32::max)
            .unwrap_or(0.0);
        if let Some((linear, angular)) = body_speeds(frame, exit_body) {
            trace.last_dynamic_impulse_body_linear_speed = linear;
            trace.last_dynamic_impulse_body_angular_speed = angular;
        }
    }
    populate_support_gap_trace(run, exit_body, pre_exit_frame, &mut trace);
    populate_support_gap_upstream_trace(run, exit_body, &mut trace);

    trace
}

fn populate_support_gap_trace(
    run: &RunResult,
    exit_body: BodyHandle,
    pre_exit_frame: usize,
    trace: &mut EjectionTrace,
) {
    let Some(first_contact_frame) = run.frames.iter().find(|frame| {
        frame.frame_index <= pre_exit_frame && frame_contacts_body(frame, exit_body).count() > 0
    }) else {
        return;
    };
    let mut previous_contact_frame = Some(first_contact_frame.frame_index);
    for frame in run
        .frames
        .iter()
        .filter(|frame| frame.frame_index > first_contact_frame.frame_index)
        .filter(|frame| frame.frame_index <= pre_exit_frame)
    {
        let has_contact = frame_contacts_body(frame, exit_body).next().is_some();
        if has_contact {
            previous_contact_frame = Some(frame.frame_index);
            continue;
        }
        trace.support_gap_start_frame = Some(frame.frame_index);
        if let Some((linear, angular)) = body_speeds(frame, exit_body) {
            trace.support_gap_start_linear_speed = linear;
            trace.support_gap_start_angular_speed = angular;
        }
        if let Some(previous) = previous_contact_frame.and_then(|frame_index| {
            run.frames
                .iter()
                .find(|frame| frame.frame_index == frame_index)
        }) {
            trace.support_gap_previous_frame = Some(previous.frame_index);
            let facts = contact_summary(previous, exit_body);
            trace.support_gap_previous_counterparts = facts.counterparts;
            trace.support_gap_previous_max_depth = facts.max_depth;
            trace.support_gap_previous_warm_start_normal_sum = facts.warm_start_normal_sum;
            trace.support_gap_previous_warm_start_tangent_sum = facts.warm_start_tangent_sum;
            trace.support_gap_previous_normal_impulse_sum = facts.normal_impulse_sum;
            trace.support_gap_previous_tangent_impulse_sum = facts.tangent_impulse_sum;
            trace.support_gap_previous_initial_normal_speed_min = facts.initial_normal_speed_min;
            trace.support_gap_previous_initial_normal_speed_max = facts.initial_normal_speed_max;
            trace.support_gap_previous_normal_speed_min = facts.normal_speed_min;
            trace.support_gap_previous_normal_speed_max = facts.normal_speed_max;
            trace.support_gap_previous_tangent_speed_abs_max = facts.tangent_speed_abs_max;
            if let Some((linear, angular)) = body_speeds(previous, exit_body) {
                trace.support_gap_previous_body_linear_speed = linear;
                trace.support_gap_previous_body_angular_speed = angular;
            }
            trace.support_gap_previous_counterpart_linear_speed_max =
                facts.counterpart_linear_speed_max;
            trace.support_gap_previous_counterpart_angular_speed_max =
                facts.counterpart_angular_speed_max;
            trace.support_gap_previous_body_tangent_linear_component_max =
                facts.body_tangent_linear_component_max;
            trace.support_gap_previous_body_tangent_angular_component_max =
                facts.body_tangent_angular_component_max;
            trace.support_gap_previous_counterpart_tangent_linear_component_max =
                facts.counterpart_tangent_linear_component_max;
            trace.support_gap_previous_counterpart_tangent_angular_component_max =
                facts.counterpart_tangent_angular_component_max;
            trace.support_gap_previous_normal_anchor_drift_max = facts.normal_anchor_drift_max;
            trace.support_gap_previous_tangent_anchor_drift_max = facts.tangent_anchor_drift_max;
            trace.support_gap_previous_position_bias_max = facts.position_bias_max;
            trace.support_gap_previous_restitution_bias_max = facts.restitution_bias_max;
            trace.support_gap_previous_position_correction_depth_sum =
                facts.position_correction_depth_sum;
            trace.support_gap_previous_position_correction_translation_sum =
                facts.position_correction_translation_sum;
        }
        if let Some(next) = run
            .frames
            .iter()
            .filter(|candidate| candidate.frame_index > frame.frame_index)
            .filter(|candidate| candidate.frame_index <= pre_exit_frame)
            .find(|candidate| frame_contacts_body(candidate, exit_body).next().is_some())
        {
            trace.support_gap_next_frame = Some(next.frame_index);
            let facts = contact_summary(next, exit_body);
            trace.support_gap_next_counterparts = facts.counterparts;
            trace.support_gap_next_max_depth = facts.max_depth;
            trace.support_gap_next_warm_start_normal_sum = facts.warm_start_normal_sum;
            trace.support_gap_next_warm_start_tangent_sum = facts.warm_start_tangent_sum;
            trace.support_gap_next_normal_impulse_sum = facts.normal_impulse_sum;
            trace.support_gap_next_tangent_impulse_sum = facts.tangent_impulse_sum;
            trace.support_gap_next_initial_normal_speed_min = facts.initial_normal_speed_min;
            trace.support_gap_next_initial_normal_speed_max = facts.initial_normal_speed_max;
            trace.support_gap_next_normal_speed_min = facts.normal_speed_min;
            trace.support_gap_next_normal_speed_max = facts.normal_speed_max;
            trace.support_gap_next_tangent_speed_abs_max = facts.tangent_speed_abs_max;
            if let Some((linear, angular)) = body_speeds(next, exit_body) {
                trace.support_gap_next_body_linear_speed = linear;
                trace.support_gap_next_body_angular_speed = angular;
            }
            trace.support_gap_next_counterpart_linear_speed_max =
                facts.counterpart_linear_speed_max;
            trace.support_gap_next_counterpart_angular_speed_max =
                facts.counterpart_angular_speed_max;
            trace.support_gap_next_body_tangent_linear_component_max =
                facts.body_tangent_linear_component_max;
            trace.support_gap_next_body_tangent_angular_component_max =
                facts.body_tangent_angular_component_max;
            trace.support_gap_next_counterpart_tangent_linear_component_max =
                facts.counterpart_tangent_linear_component_max;
            trace.support_gap_next_counterpart_tangent_angular_component_max =
                facts.counterpart_tangent_angular_component_max;
            trace.support_gap_next_normal_anchor_drift_max = facts.normal_anchor_drift_max;
            trace.support_gap_next_tangent_anchor_drift_max = facts.tangent_anchor_drift_max;
            trace.support_gap_next_position_bias_max = facts.position_bias_max;
            trace.support_gap_next_restitution_bias_max = facts.restitution_bias_max;
            trace.support_gap_next_position_correction_depth_sum =
                facts.position_correction_depth_sum;
            trace.support_gap_next_position_correction_translation_sum =
                facts.position_correction_translation_sum;
        }
        return;
    }
}

fn populate_support_gap_upstream_trace(
    run: &RunResult,
    exit_body: BodyHandle,
    trace: &mut EjectionTrace,
) {
    let Some(previous_frame_index) = trace.support_gap_previous_frame else {
        return;
    };
    let Some(previous_frame) = run
        .frames
        .iter()
        .find(|frame| frame.frame_index == previous_frame_index)
    else {
        return;
    };
    let mut counterparts = frame_contacts_body(previous_frame, exit_body)
        .filter_map(|contact| {
            contact
                .bodies
                .iter()
                .copied()
                .find(|body| *body != exit_body)
        })
        .collect::<Vec<_>>();
    counterparts.sort_by_key(|body| format!("{body:?}"));
    counterparts.dedup();
    if counterparts.is_empty() {
        return;
    }

    let window_start = previous_frame_index
        .saturating_sub(60)
        .max(MATRIX_STACK_QUIET_WINDOW_START_FRAME);
    let mut best = None::<SupportGapUpstreamCandidate>;
    for counterpart in counterparts {
        for frame in run.frames.iter().filter(|frame| {
            frame.frame_index >= window_start && frame.frame_index <= previous_frame_index
        }) {
            let Some((linear_speed, angular_speed)) = body_speeds(frame, counterpart) else {
                continue;
            };
            let source_contacts = frame
                .snapshot
                .contacts
                .iter()
                .filter(|contact| {
                    contact.bodies.contains(&counterpart) && !contact.bodies.contains(&exit_body)
                })
                .collect::<Vec<_>>();
            let source_contact_count = source_contacts.len();
            let source_candidate_count = source_contacts
                .iter()
                .filter(|contact| contact.source_row_continuity_candidate)
                .count();
            let mut source_warm_start_reasons = source_contacts
                .iter()
                .map(|contact| format!("{:?}", contact.warm_start_reason))
                .collect::<Vec<_>>();
            source_warm_start_reasons.sort();
            source_warm_start_reasons.dedup();
            let mut source_continuity_reasons = source_contacts
                .iter()
                .map(|contact| format!("{:?}", contact.source_row_continuity_reason))
                .collect::<Vec<_>>();
            source_continuity_reasons.sort();
            source_continuity_reasons.dedup();
            let source_continuity = source_continuity_facts(run, frame, &source_contacts);
            let source_max_depth = source_contacts
                .iter()
                .map(|contact| contact.depth)
                .fold(0.0, f32::max);
            let source_initial_normal_speeds = source_contacts
                .iter()
                .map(|contact| contact.solver_initial_normal_speed)
                .collect::<Vec<_>>();
            let source_final_speeds = source_contacts
                .iter()
                .map(|contact| contact_solver_final_speeds(contact))
                .collect::<Vec<_>>();
            let source_position_bias_max = source_contacts
                .iter()
                .map(|contact| contact.solver_position_bias)
                .reduce(f32::max)
                .unwrap_or(0.0);
            let source_support_friction_sum = source_contacts
                .iter()
                .map(|contact| contact.solver_support_friction_impulse)
                .sum();
            let source_normal_impulse_sum = source_contacts
                .iter()
                .map(|contact| contact.solver_normal_impulse)
                .sum();
            let source_tangent_impulse_sum = source_contacts
                .iter()
                .map(|contact| contact.solver_tangent_impulse.abs())
                .sum();
            let source_correction_depth_sum = source_contacts
                .iter()
                .map(|contact| contact.solver_position_correction_depth)
                .sum();
            let source_correction_translation_sum = source_contacts
                .iter()
                .map(|contact| {
                    contact.solver_position_correction_body_a_translation
                        + contact.solver_position_correction_body_b_translation
                })
                .sum();
            let source_dry_run = source_row_dry_run_facts(
                frame,
                &source_contacts,
                &source_continuity,
                linear_speed,
                angular_speed,
            );
            let (previous_linear_speed, previous_angular_speed) =
                body_speeds(previous_frame, counterpart).unwrap_or((linear_speed, angular_speed));
            let mut source_bodies = source_contacts
                .iter()
                .flat_map(|contact| contact.bodies)
                .filter(|body| *body != counterpart && *body != exit_body)
                .map(|body| format!("{body:?}"))
                .collect::<Vec<_>>();
            source_bodies.sort();
            source_bodies.dedup();

            let candidate = SupportGapUpstreamCandidate {
                frame: frame.frame_index,
                counterpart,
                source_bodies,
                counterpart_linear_speed: linear_speed,
                counterpart_angular_speed: angular_speed,
                source_contact_count,
                source_candidate_count,
                source_warm_start_reasons,
                source_continuity_reasons,
                source_same_pair_previous_count: source_continuity.same_pair_previous_count,
                source_edge_swap_candidate_count: source_continuity.edge_swap_candidate_count,
                source_point_drift_min: source_continuity.point_drift_min,
                source_normal_dot_max: source_continuity.normal_dot_max,
                source_local_anchor_drift_min: source_continuity.local_anchor_drift_min,
                source_max_depth,
                source_initial_normal_speed_min: source_initial_normal_speeds
                    .iter()
                    .copied()
                    .reduce(f32::min)
                    .unwrap_or(0.0),
                source_initial_normal_speed_max: source_initial_normal_speeds
                    .iter()
                    .copied()
                    .reduce(f32::max)
                    .unwrap_or(0.0),
                source_normal_speed_min: source_final_speeds
                    .iter()
                    .map(|(normal_speed, _)| *normal_speed)
                    .reduce(f32::min)
                    .unwrap_or(0.0),
                source_normal_speed_max: source_final_speeds
                    .iter()
                    .map(|(normal_speed, _)| *normal_speed)
                    .reduce(f32::max)
                    .unwrap_or(0.0),
                source_tangent_speed_abs_max: source_final_speeds
                    .iter()
                    .map(|(_, tangent_speed)| tangent_speed.abs())
                    .reduce(f32::max)
                    .unwrap_or(0.0),
                source_position_bias_max,
                source_support_friction_sum,
                source_normal_impulse_sum,
                source_tangent_impulse_sum,
                source_correction_depth_sum,
                source_correction_translation_sum,
                source_dry_run_geometry_count: source_dry_run.geometry_count,
                source_dry_run_pressure_count: source_dry_run.pressure_count,
                source_dry_run_counterpart_reject_count: source_dry_run.counterpart_reject_count,
                source_dry_run_tangent_reject_count: source_dry_run.tangent_reject_count,
                source_dry_run_eligible_count: source_dry_run.eligible_count,
                source_dry_run_decision: source_dry_run.decision,
                source_dry_run_pseudo_depth_min: source_dry_run.pseudo_depth_min,
                source_dry_run_pseudo_depth_max: source_dry_run.pseudo_depth_max,
                source_dry_run_pseudo_support_skin_count: source_dry_run.pseudo_support_skin_count,
                handoff_frame_delta: previous_frame_index.checked_sub(frame.frame_index),
                handoff_linear_speed_delta: previous_linear_speed - linear_speed,
                handoff_angular_speed_delta: previous_angular_speed - angular_speed,
            };
            if best.as_ref().is_none_or(|best| {
                candidate.counterpart_angular_speed > best.counterpart_angular_speed
            }) {
                best = Some(candidate);
            }
        }
    }

    let Some(best) = best else {
        return;
    };
    trace.support_gap_upstream_frame = Some(best.frame);
    trace.support_gap_upstream_counterpart = Some(format!("{:?}", best.counterpart));
    trace.support_gap_upstream_source_bodies = best.source_bodies;
    trace.support_gap_upstream_counterpart_linear_speed = best.counterpart_linear_speed;
    trace.support_gap_upstream_counterpart_angular_speed = best.counterpart_angular_speed;
    trace.support_gap_upstream_source_contact_count = best.source_contact_count;
    trace.support_gap_upstream_source_candidate_count = best.source_candidate_count;
    trace.support_gap_upstream_source_warm_start_reasons = best.source_warm_start_reasons;
    trace.support_gap_upstream_source_continuity_reasons = best.source_continuity_reasons;
    trace.support_gap_upstream_source_same_pair_previous_count =
        best.source_same_pair_previous_count;
    trace.support_gap_upstream_source_edge_swap_candidate_count =
        best.source_edge_swap_candidate_count;
    trace.support_gap_upstream_source_point_drift_min = best.source_point_drift_min;
    trace.support_gap_upstream_source_normal_dot_max = best.source_normal_dot_max;
    trace.support_gap_upstream_source_local_anchor_drift_min = best.source_local_anchor_drift_min;
    trace.support_gap_upstream_source_max_depth = best.source_max_depth;
    trace.support_gap_upstream_source_initial_normal_speed_min =
        best.source_initial_normal_speed_min;
    trace.support_gap_upstream_source_initial_normal_speed_max =
        best.source_initial_normal_speed_max;
    trace.support_gap_upstream_source_normal_speed_min = best.source_normal_speed_min;
    trace.support_gap_upstream_source_normal_speed_max = best.source_normal_speed_max;
    trace.support_gap_upstream_source_tangent_speed_abs_max = best.source_tangent_speed_abs_max;
    trace.support_gap_upstream_source_position_bias_max = best.source_position_bias_max;
    trace.support_gap_upstream_source_support_friction_sum = best.source_support_friction_sum;
    trace.support_gap_upstream_source_normal_impulse_sum = best.source_normal_impulse_sum;
    trace.support_gap_upstream_source_tangent_impulse_sum = best.source_tangent_impulse_sum;
    trace.support_gap_upstream_source_correction_depth_sum = best.source_correction_depth_sum;
    trace.support_gap_upstream_source_correction_translation_sum =
        best.source_correction_translation_sum;
    trace.support_gap_upstream_source_dry_run_geometry_count = best.source_dry_run_geometry_count;
    trace.support_gap_upstream_source_dry_run_pressure_count = best.source_dry_run_pressure_count;
    trace.support_gap_upstream_source_dry_run_counterpart_reject_count =
        best.source_dry_run_counterpart_reject_count;
    trace.support_gap_upstream_source_dry_run_tangent_reject_count =
        best.source_dry_run_tangent_reject_count;
    trace.support_gap_upstream_source_dry_run_eligible_count = best.source_dry_run_eligible_count;
    trace.support_gap_upstream_source_dry_run_decision = best.source_dry_run_decision;
    trace.support_gap_upstream_source_dry_run_pseudo_depth_min =
        best.source_dry_run_pseudo_depth_min;
    trace.support_gap_upstream_source_dry_run_pseudo_depth_max =
        best.source_dry_run_pseudo_depth_max;
    trace.support_gap_upstream_source_dry_run_pseudo_support_skin_count =
        best.source_dry_run_pseudo_support_skin_count;
    trace.support_gap_upstream_handoff_frame_delta = best.handoff_frame_delta;
    trace.support_gap_upstream_handoff_linear_speed_delta = best.handoff_linear_speed_delta;
    trace.support_gap_upstream_handoff_angular_speed_delta = best.handoff_angular_speed_delta;
}

struct SupportGapUpstreamCandidate {
    frame: usize,
    counterpart: BodyHandle,
    source_bodies: Vec<String>,
    counterpart_linear_speed: f32,
    counterpart_angular_speed: f32,
    source_contact_count: usize,
    source_candidate_count: usize,
    source_warm_start_reasons: Vec<String>,
    source_continuity_reasons: Vec<String>,
    source_same_pair_previous_count: usize,
    source_edge_swap_candidate_count: usize,
    source_point_drift_min: Option<f32>,
    source_normal_dot_max: Option<f32>,
    source_local_anchor_drift_min: Option<f32>,
    source_max_depth: f32,
    source_initial_normal_speed_min: f32,
    source_initial_normal_speed_max: f32,
    source_normal_speed_min: f32,
    source_normal_speed_max: f32,
    source_tangent_speed_abs_max: f32,
    source_position_bias_max: f32,
    source_support_friction_sum: f32,
    source_normal_impulse_sum: f32,
    source_tangent_impulse_sum: f32,
    source_correction_depth_sum: f32,
    source_correction_translation_sum: f32,
    source_dry_run_geometry_count: usize,
    source_dry_run_pressure_count: usize,
    source_dry_run_counterpart_reject_count: usize,
    source_dry_run_tangent_reject_count: usize,
    source_dry_run_eligible_count: usize,
    source_dry_run_decision: String,
    source_dry_run_pseudo_depth_min: Option<f32>,
    source_dry_run_pseudo_depth_max: Option<f32>,
    source_dry_run_pseudo_support_skin_count: usize,
    handoff_frame_delta: Option<usize>,
    handoff_linear_speed_delta: f32,
    handoff_angular_speed_delta: f32,
}

#[derive(Default)]
struct SourceRowDryRunFacts {
    geometry_count: usize,
    pressure_count: usize,
    counterpart_reject_count: usize,
    tangent_reject_count: usize,
    eligible_count: usize,
    decision: String,
    pseudo_depth_min: Option<f32>,
    pseudo_depth_max: Option<f32>,
    pseudo_support_skin_count: usize,
}

#[derive(Default)]
struct SourceContinuityFacts {
    same_pair_previous_count: usize,
    edge_swap_candidate_count: usize,
    point_drift_min: Option<f32>,
    normal_dot_max: Option<f32>,
    local_anchor_drift_min: Option<f32>,
}

fn source_continuity_facts(
    run: &RunResult,
    current_frame: &FrameRecord,
    source_contacts: &[&DebugContact],
) -> SourceContinuityFacts {
    let Some(previous_frame_index) = current_frame.frame_index.checked_sub(1) else {
        return SourceContinuityFacts::default();
    };
    let Some(previous_frame) = run
        .frames
        .iter()
        .find(|frame| frame.frame_index == previous_frame_index)
    else {
        return SourceContinuityFacts::default();
    };

    let mut facts = SourceContinuityFacts::default();
    for contact in source_contacts {
        let previous_pair_contacts = previous_frame
            .snapshot
            .contacts
            .iter()
            .filter(|candidate| candidate.colliders == contact.colliders)
            .collect::<Vec<_>>();
        if previous_pair_contacts.is_empty() {
            continue;
        }
        facts.same_pair_previous_count += 1;

        let Some((previous_contact, point_drift, normal_dot)) = previous_pair_contacts
            .iter()
            .filter_map(|previous_contact| {
                let point_drift = (contact.point - previous_contact.point).length();
                let normal_dot = contact
                    .normal
                    .normalized_or_zero()
                    .dot(previous_contact.normal.normalized_or_zero());
                point_drift
                    .is_finite()
                    .then_some((*previous_contact, point_drift, normal_dot))
            })
            .min_by(|(_, lhs, _), (_, rhs, _)| lhs.partial_cmp(rhs).unwrap())
        else {
            continue;
        };

        facts.point_drift_min = min_option_f32(facts.point_drift_min, point_drift);
        facts.normal_dot_max = max_option_f32(facts.normal_dot_max, normal_dot);
        let local_anchor_drift =
            max_local_anchor_drift(previous_frame, current_frame, previous_contact, contact);
        if let Some(local_anchor_drift) = local_anchor_drift {
            facts.local_anchor_drift_min =
                min_option_f32(facts.local_anchor_drift_min, local_anchor_drift);
        }

        let edge_swap_candidate = feature_index_edge_swap(previous_contact, contact)
            && previous_contact.reduction_reason == contact.reduction_reason
            && point_drift <= 0.05
            && normal_dot >= 0.98
            && same_shape_signature(previous_frame, current_frame, previous_contact, contact)
            && local_anchor_drift
                .is_some_and(|drift| drift <= FEATURE_CHURN_LOCAL_ANCHOR_DRIFT_THRESHOLD);
        if edge_swap_candidate {
            facts.edge_swap_candidate_count += 1;
        }
    }

    facts
}

fn source_row_dry_run_facts(
    frame: &FrameRecord,
    source_contacts: &[&DebugContact],
    source_continuity: &SourceContinuityFacts,
    counterpart_linear_speed: f32,
    counterpart_angular_speed: f32,
) -> SourceRowDryRunFacts {
    let mut facts = SourceRowDryRunFacts::default();
    let pseudo_facts = source_row_pseudo_position_facts(frame, source_contacts);
    facts.pseudo_depth_min = pseudo_facts.depth_min;
    facts.pseudo_depth_max = pseudo_facts.depth_max;
    facts.pseudo_support_skin_count = pseudo_facts.support_skin_count;

    for contact in source_contacts
        .iter()
        .copied()
        .filter(|contact| contact.source_row_continuity_candidate)
    {
        let geometry_valid = contact.depth > 0.0
            && source_continuity
                .normal_dot_max
                .is_some_and(|normal_dot| normal_dot >= SOURCE_ROW_NORMAL_DOT_DRY_RUN_THRESHOLD)
            && source_continuity
                .local_anchor_drift_min
                .is_some_and(|drift| drift <= FEATURE_CHURN_LOCAL_ANCHOR_DRIFT_THRESHOLD);
        if !geometry_valid {
            continue;
        }
        facts.geometry_count += 1;

        let correction_pressure_source = contact.solver_position_bias <= f32::EPSILON
            && contact.solver_support_friction_impulse.abs() <= f32::EPSILON
            && contact.solver_position_correction_depth > contact.solver_normal_impulse;
        if !correction_pressure_source {
            continue;
        }
        facts.pressure_count += 1;

        let reject_counterpart_motion = counterpart_linear_speed
            > SUPPORT_ELIGIBILITY_COUNTERPART_TANGENT_THRESHOLD
            || counterpart_angular_speed > SUPPORT_ELIGIBILITY_COUNTERPART_TANGENT_THRESHOLD;
        if reject_counterpart_motion {
            facts.counterpart_reject_count += 1;
        }

        let (_, tangent_speed) = contact_solver_final_speeds(contact);
        let reject_tangent_energy =
            tangent_speed.abs() > SUPPORT_ELIGIBILITY_COUNTERPART_TANGENT_THRESHOLD;
        if reject_tangent_energy {
            facts.tangent_reject_count += 1;
        }

        if !reject_counterpart_motion && !reject_tangent_energy {
            facts.eligible_count += 1;
        }
    }

    facts.decision = if facts.geometry_count == 0 {
        "reject_geometry".to_owned()
    } else if facts.pressure_count == 0 {
        "observe_not_pressure_source".to_owned()
    } else if facts.counterpart_reject_count > 0 {
        "reject_counterpart_motion".to_owned()
    } else if facts.tangent_reject_count > 0 {
        "reject_tangent_energy".to_owned()
    } else if facts.eligible_count > 0 {
        "candidate_needs_position_re_evaluation".to_owned()
    } else {
        "observe_no_source_candidate".to_owned()
    };
    facts
}

#[derive(Default)]
struct SourceRowPseudoPositionFacts {
    depth_min: Option<f32>,
    depth_max: Option<f32>,
    support_skin_count: usize,
}

fn source_row_pseudo_position_facts(
    frame: &FrameRecord,
    source_contacts: &[&DebugContact],
) -> SourceRowPseudoPositionFacts {
    let source_contact_ids = source_contacts
        .iter()
        .filter(|contact| contact.source_row_continuity_candidate)
        .map(|contact| contact.id)
        .collect::<Vec<_>>();
    if source_contact_ids.is_empty() || frame.stats.position_iterations == 0 {
        return SourceRowPseudoPositionFacts::default();
    }

    let eligible_contact_count = frame
        .snapshot
        .contacts
        .iter()
        .filter(|contact| residual_position_depth(contact.depth) > 0.0)
        .count();
    let dense_correction_mode = eligible_contact_count >= 16;
    let mut body_translation_vectors = BTreeMap::<BodyHandle, picea::prelude::Vector>::new();
    let mut queued_translations = BTreeMap::<BodyHandle, picea::prelude::Vector>::new();
    let mut facts = SourceRowPseudoPositionFacts::default();

    for _ in 0..frame.stats.position_iterations {
        for contact in &frame.snapshot.contacts {
            let normal = contact.normal.normalized_or_zero();
            let frame_start_depth = residual_position_depth(contact.depth);
            if normal.length() <= f32::EPSILON || frame_start_depth <= f32::EPSILON {
                continue;
            }
            let inverse_mass_a = debug_body_inverse_mass(frame, contact.bodies[0]);
            let inverse_mass_b = debug_body_inverse_mass(frame, contact.bodies[1]);
            let inverse_mass_sum = inverse_mass_a + inverse_mass_b;
            if inverse_mass_sum <= f32::EPSILON {
                continue;
            }

            let depth = if dense_correction_mode {
                let translation_a = body_translation_vectors
                    .get(&contact.bodies[0])
                    .copied()
                    .unwrap_or_default();
                let translation_b = body_translation_vectors
                    .get(&contact.bodies[1])
                    .copied()
                    .unwrap_or_default();
                residual_position_depth(
                    (contact.depth - (translation_a - translation_b).dot(normal)).max(0.0),
                )
            } else {
                frame_start_depth
            };
            if depth <= f32::EPSILON {
                continue;
            }

            if source_contact_ids.contains(&contact.id) {
                facts.depth_min = min_option_f32(facts.depth_min, depth);
                facts.depth_max = max_option_f32(facts.depth_max, depth);
                if depth <= SOURCE_ROW_PSEUDO_SUPPORT_SKIN {
                    facts.support_skin_count += 1;
                }
            }

            let correction = depth * 0.8 / f32::from(frame.stats.position_iterations);
            let correction_a = normal * (correction * inverse_mass_a / inverse_mass_sum);
            let correction_b = -normal * (correction * inverse_mass_b / inverse_mass_sum);
            if queue_pseudo_position_translation(
                frame,
                &mut queued_translations,
                contact.bodies[0],
                correction_a,
                contact.bodies[1],
            ) {
                *body_translation_vectors
                    .entry(contact.bodies[0])
                    .or_default() += correction_a;
            }
            if queue_pseudo_position_translation(
                frame,
                &mut queued_translations,
                contact.bodies[1],
                correction_b,
                contact.bodies[0],
            ) {
                *body_translation_vectors
                    .entry(contact.bodies[1])
                    .or_default() += correction_b;
            }
        }
    }

    facts
}

fn residual_position_depth(depth: f32) -> f32 {
    (depth - 0.005).max(0.0)
}

fn debug_body_inverse_mass(frame: &FrameRecord, body: BodyHandle) -> f32 {
    frame
        .snapshot
        .bodies
        .iter()
        .find(|debug_body| debug_body.handle == body)
        .map(|debug_body| debug_body.mass_properties.inverse_mass)
        .unwrap_or(0.0)
}

fn queue_pseudo_position_translation(
    frame: &FrameRecord,
    queued_translations: &mut BTreeMap<BodyHandle, picea::prelude::Vector>,
    body: BodyHandle,
    translation: picea::prelude::Vector,
    counterpart: BodyHandle,
) -> bool {
    if translation.length() <= f32::EPSILON {
        return false;
    }
    let Some(record) = frame
        .snapshot
        .bodies
        .iter()
        .find(|debug_body| debug_body.handle == body)
    else {
        return false;
    };
    if record.body_type != picea::prelude::BodyType::Dynamic {
        return false;
    }
    let already_queued = queued_translations.contains_key(&body);
    let wake_sleeping_body = queued_translations.contains_key(&counterpart)
        || frame
            .snapshot
            .bodies
            .iter()
            .find(|debug_body| debug_body.handle == counterpart)
            .is_some_and(|debug_body| {
                debug_body.body_type != picea::prelude::BodyType::Static && !debug_body.sleeping
            });
    if record.sleeping && !wake_sleeping_body && !already_queued {
        return false;
    }
    *queued_translations.entry(body).or_default() += translation;
    true
}

fn option_frame(frame: Option<usize>) -> String {
    frame
        .map(|frame| frame.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn option_f32(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "none".to_owned())
}

fn min_option_f32(current: Option<f32>, candidate: f32) -> Option<f32> {
    current
        .map(|current| current.min(candidate))
        .or(Some(candidate))
}

fn max_option_f32(current: Option<f32>, candidate: f32) -> Option<f32> {
    current
        .map(|current| current.max(candidate))
        .or(Some(candidate))
}

fn option_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn contact_body_labels(bodies: [BodyHandle; 2]) -> Vec<String> {
    let mut labels = bodies
        .iter()
        .map(|body| format!("{body:?}"))
        .collect::<Vec<_>>();
    labels.sort();
    labels
}

fn contact_collider_labels(colliders: [ColliderHandle; 2]) -> Vec<String> {
    let mut labels = colliders
        .iter()
        .map(|collider| format!("{collider:?}"))
        .collect::<Vec<_>>();
    labels.sort();
    labels
}

#[derive(Default)]
struct ContactSummary {
    contact_count: usize,
    counterparts: Vec<String>,
    counterpart_linear_speed_max: f32,
    counterpart_angular_speed_max: f32,
    body_tangent_linear_component_max: f32,
    body_tangent_angular_component_max: f32,
    counterpart_tangent_linear_component_max: f32,
    counterpart_tangent_angular_component_max: f32,
    max_depth: f32,
    warm_start_normal_sum: f32,
    warm_start_tangent_sum: f32,
    normal_impulse_sum: f32,
    tangent_impulse_sum: f32,
    initial_normal_speed_min: f32,
    initial_normal_speed_max: f32,
    normal_speed_min: f32,
    normal_speed_max: f32,
    tangent_speed_abs_max: f32,
    support_friction_impulse_max: f32,
    support_friction_impulse_sum: f32,
    normal_anchor_drift_max: f32,
    tangent_anchor_drift_max: f32,
    position_bias_max: f32,
    restitution_bias_max: f32,
    position_correction_depth_sum: f32,
    position_correction_translation_sum: f32,
}

fn contact_summary(frame: &FrameRecord, body: BodyHandle) -> ContactSummary {
    let contacts = frame_contacts_body(frame, body).collect::<Vec<_>>();
    let mut counterpart_handles = contacts
        .iter()
        .filter_map(|contact| {
            contact
                .bodies
                .iter()
                .copied()
                .find(|candidate| *candidate != body)
        })
        .collect::<Vec<_>>();
    counterpart_handles.sort_by_key(|body| format!("{body:?}"));
    counterpart_handles.dedup();
    let counterparts = counterpart_handles
        .iter()
        .map(|body| format!("{body:?}"))
        .collect::<Vec<_>>();
    let counterpart_speeds = counterpart_handles
        .iter()
        .filter_map(|body| body_speeds(frame, *body))
        .collect::<Vec<_>>();
    let relative_speeds = contacts
        .iter()
        .map(|contact| contact_solver_final_speeds(contact))
        .collect::<Vec<_>>();
    let initial_normal_speeds = contacts
        .iter()
        .map(|contact| contact.solver_initial_normal_speed)
        .collect::<Vec<_>>();
    ContactSummary {
        contact_count: contacts.len(),
        counterparts,
        counterpart_linear_speed_max: counterpart_speeds
            .iter()
            .map(|(linear, _)| *linear)
            .reduce(f32::max)
            .unwrap_or(0.0),
        counterpart_angular_speed_max: counterpart_speeds
            .iter()
            .map(|(_, angular)| *angular)
            .reduce(f32::max)
            .unwrap_or(0.0),
        body_tangent_linear_component_max: contacts
            .iter()
            .filter_map(|contact| contact_tangent_components(frame, contact, body))
            .map(|components| components.body_linear)
            .reduce(f32::max)
            .unwrap_or(0.0),
        body_tangent_angular_component_max: contacts
            .iter()
            .filter_map(|contact| contact_tangent_components(frame, contact, body))
            .map(|components| components.body_angular)
            .reduce(f32::max)
            .unwrap_or(0.0),
        counterpart_tangent_linear_component_max: contacts
            .iter()
            .filter_map(|contact| contact_tangent_components(frame, contact, body))
            .map(|components| components.counterpart_linear)
            .reduce(f32::max)
            .unwrap_or(0.0),
        counterpart_tangent_angular_component_max: contacts
            .iter()
            .filter_map(|contact| contact_tangent_components(frame, contact, body))
            .map(|components| components.counterpart_angular)
            .reduce(f32::max)
            .unwrap_or(0.0),
        max_depth: contacts
            .iter()
            .map(|contact| contact.depth)
            .fold(0.0, f32::max),
        warm_start_normal_sum: contacts.iter().map(|contact| contact.normal_impulse).sum(),
        warm_start_tangent_sum: contacts
            .iter()
            .map(|contact| contact.tangent_impulse.abs())
            .sum(),
        normal_impulse_sum: contacts
            .iter()
            .map(|contact| contact.solver_normal_impulse)
            .sum(),
        tangent_impulse_sum: contacts
            .iter()
            .map(|contact| contact.solver_tangent_impulse.abs())
            .sum(),
        initial_normal_speed_min: initial_normal_speeds
            .iter()
            .copied()
            .reduce(f32::min)
            .unwrap_or(0.0),
        initial_normal_speed_max: initial_normal_speeds
            .iter()
            .copied()
            .reduce(f32::max)
            .unwrap_or(0.0),
        normal_speed_min: relative_speeds
            .iter()
            .map(|(normal_speed, _)| *normal_speed)
            .reduce(f32::min)
            .unwrap_or(0.0),
        normal_speed_max: relative_speeds
            .iter()
            .map(|(normal_speed, _)| *normal_speed)
            .reduce(f32::max)
            .unwrap_or(0.0),
        tangent_speed_abs_max: relative_speeds
            .iter()
            .map(|(_, tangent_speed)| tangent_speed.abs())
            .reduce(f32::max)
            .unwrap_or(0.0),
        support_friction_impulse_max: contacts
            .iter()
            .map(|contact| contact.solver_support_friction_impulse)
            .reduce(f32::max)
            .unwrap_or(0.0),
        support_friction_impulse_sum: contacts
            .iter()
            .map(|contact| contact.solver_support_friction_impulse)
            .sum(),
        normal_anchor_drift_max: contacts
            .iter()
            .map(|contact| contact.warm_start_normal_anchor_drift)
            .reduce(f32::max)
            .unwrap_or(0.0),
        tangent_anchor_drift_max: contacts
            .iter()
            .map(|contact| contact.warm_start_tangent_anchor_drift)
            .reduce(f32::max)
            .unwrap_or(0.0),
        position_bias_max: contacts
            .iter()
            .map(|contact| contact.solver_position_bias)
            .reduce(f32::max)
            .unwrap_or(0.0),
        restitution_bias_max: contacts
            .iter()
            .map(|contact| contact.solver_restitution_bias)
            .reduce(f32::max)
            .unwrap_or(0.0),
        position_correction_depth_sum: contacts
            .iter()
            .map(|contact| contact.solver_position_correction_depth)
            .sum(),
        position_correction_translation_sum: contacts
            .iter()
            .map(|contact| {
                contact.solver_position_correction_body_a_translation
                    + contact.solver_position_correction_body_b_translation
            })
            .sum(),
    }
}

fn frame_contacts_body(
    frame: &FrameRecord,
    body: BodyHandle,
) -> impl Iterator<Item = &DebugContact> {
    frame
        .snapshot
        .contacts
        .iter()
        .filter(move |contact| contact.bodies.contains(&body))
}

fn body_speeds(frame: &FrameRecord, body: BodyHandle) -> Option<(f32, f32)> {
    frame
        .snapshot
        .bodies
        .iter()
        .find(|debug_body| debug_body.handle == body)
        .map(|debug_body| {
            (
                debug_body.linear_velocity.length(),
                debug_body.angular_velocity.abs(),
            )
        })
}

#[derive(Clone, Copy, Default)]
struct TangentComponents {
    body_linear: f32,
    body_angular: f32,
    counterpart_linear: f32,
    counterpart_angular: f32,
}

fn contact_tangent_components(
    frame: &FrameRecord,
    contact: &DebugContact,
    body: BodyHandle,
) -> Option<TangentComponents> {
    let body_debug = frame
        .snapshot
        .bodies
        .iter()
        .find(|debug_body| debug_body.handle == body)?;
    let counterpart = contact
        .bodies
        .iter()
        .copied()
        .find(|candidate| *candidate != body)?;
    let counterpart_debug = frame
        .snapshot
        .bodies
        .iter()
        .find(|debug_body| debug_body.handle == counterpart)?;
    let tangent = contact.normal.perp().normalized_or_zero();
    if tangent.length() <= f32::EPSILON {
        return None;
    }
    let body_anchor = contact.point - body_debug.transform.translation;
    let counterpart_anchor = contact.point - counterpart_debug.transform.translation;
    let body_angular_velocity =
        angular_point_velocity(body_debug.angular_velocity, body_anchor.into());
    let counterpart_angular_velocity = angular_point_velocity(
        counterpart_debug.angular_velocity,
        counterpart_anchor.into(),
    );
    Some(TangentComponents {
        body_linear: body_debug.linear_velocity.dot(tangent).abs(),
        body_angular: body_angular_velocity.dot(tangent).abs(),
        counterpart_linear: counterpart_debug.linear_velocity.dot(tangent).abs(),
        counterpart_angular: counterpart_angular_velocity.dot(tangent).abs(),
    })
}

fn angular_point_velocity(
    angular_velocity: f32,
    anchor: picea::prelude::Vector,
) -> picea::prelude::Vector {
    picea::prelude::Vector::new(
        angular_velocity * anchor.y(),
        -angular_velocity * anchor.x(),
    )
}

fn contact_solver_final_speeds(contact: &DebugContact) -> (f32, f32) {
    (
        contact.solver_final_normal_speed,
        contact.solver_final_tangent_speed,
    )
}

fn contact_has_dynamic_counterpart(
    frame: &FrameRecord,
    bodies: [BodyHandle; 2],
    target: BodyHandle,
) -> bool {
    bodies.contains(&target)
        && bodies
            .iter()
            .copied()
            .filter(|body| *body != target)
            .any(|body| {
                frame
                    .snapshot
                    .bodies
                    .iter()
                    .find(|debug_body| debug_body.handle == body)
                    .map(|debug_body| debug_body.body_type == picea::prelude::BodyType::Dynamic)
                    .unwrap_or(false)
            })
}

fn matrix_stack_floor_x_bounds(run: &RunResult) -> Option<(f32, f32)> {
    let first = run.frames.first()?;
    first
        .snapshot
        .colliders
        .iter()
        .filter(|collider| {
            first.snapshot.bodies.iter().any(|body| {
                body.handle == collider.body && body.body_type == picea::prelude::BodyType::Static
            })
        })
        .filter_map(|collider| collider.aabb)
        .fold(None, |bounds, aabb| {
            let next = (aabb.min.x(), aabb.max.x());
            Some(match bounds {
                Some((min_x, max_x)) => (min_x.min(next.0), max_x.max(next.1)),
                None => next,
            })
        })
}

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
    let source_row_candidate_count = run
        .frames
        .iter()
        .flat_map(|frame| &frame.snapshot.contacts)
        .filter(|contact| contact.source_row_continuity_candidate)
        .count();
    assert!(
        source_row_candidate_count > 0,
        "matrix stack should export source-row continuity candidates for D4/E6 triage"
    );
    assert!(
        run.frames
            .iter()
            .flat_map(|frame| &frame.snapshot.contacts)
            .filter(|contact| contact.source_row_continuity_candidate)
            .all(|contact| contact.source_row_continuity_reason
                == SourceRowContinuityReason::Candidate),
        "source-row candidates should carry core-owned Candidate provenance"
    );
    let persistent_edge_swap_count = run
        .frames
        .iter()
        .flat_map(|frame| &frame.snapshot.contacts)
        .filter(|contact| contact.lifecycle_reason == ContactLifecycleReason::PersistentEdgeSwap)
        .count();
    assert!(
        persistent_edge_swap_count > 0,
        "matrix stack should export persistent-manifold edge-swap lifecycle provenance"
    );

    let static_bodies = first
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Static)
        .count();
    assert_eq!(
        static_bodies, 1,
        "default matrix stack should keep the single static floor visible"
    );

    for frame in run.frames.iter().skip(1) {
        assert_eq!(
            frame.diagnostics.stability.penetration.source,
            DiagnosticSource::LabDerived,
            "stress report needs lab-derived penetration summaries on every comparable frame"
        );
        assert_eq!(
            frame.diagnostics.stability.contact_churn.source,
            DiagnosticSource::LabDerived,
            "stress report needs churn summaries after the first frame"
        );
        assert_eq!(
            frame.diagnostics.stability.warm_start.counts_source,
            DiagnosticSource::RustAuthoritative,
            "warm-start counts should keep using core-owned counters"
        );
        assert_eq!(
            frame.diagnostics.stability.sleep.body_counts_source,
            DiagnosticSource::LabDerived,
            "sleep body counts are a lab summary over exported debug bodies"
        );
        assert_eq!(
            frame.diagnostics.stability.island.source,
            DiagnosticSource::RustAuthoritative,
            "island/solver row facts should stay on the existing FrameDiagnostics contract"
        );
    }

    let report = matrix_stack_stress_report(&run);
    let markdown = report.to_markdown(&run);
    println!("{markdown}");
    assert!(markdown.contains("## Matrix Stack Stability Report"));
    assert!(markdown.contains("- First bad frame:"));
    assert!(markdown.contains("- Penetration max / sum:"));
    assert!(markdown.contains("- Warm-start hit / miss / drop:"));
    assert!(markdown.contains("- Sleep awake / sleeping:"));
    assert!(markdown.contains("- Sleep blocker speeds:"));
    assert!(markdown.contains("- Quiet window f>=120:"));
    assert!(markdown.contains("- Late linear spike trace:"));
    assert!(markdown.contains("- Late angular spike trace:"));
    assert!(markdown.contains("late_support_gap none"));
    assert_eq!(
        report.late_linear_spike_trace.support_gap_start_frame,
        None,
        "persistent manifold warm-start should remove the retained 180-frame late support gap; report={report:?}"
    );
    assert_eq!(
        report.late_linear_spike_trace.support_gap_duration, 0,
        "persistent manifold warm-start should remove the retained 180-frame late support gap duration; report={report:?}"
    );
    assert!(
        report.max_penetration_depth <= 0.041646,
        "E4 position-row work should not regress the retained 8x6 penetration baseline; report={report:?}"
    );
    assert!(
        report.quiet_window_max_linear_speed <= 3.430_82,
        "E4 position-row work should not regress the retained 8x6 late linear spike baseline; report={report:?}"
    );
    assert!(
        report.quiet_window_max_angular_speed <= 5.230875,
        "E4 position-row work should not regress the retained 8x6 late angular spike baseline; report={report:?}"
    );
    assert!(
        report.late_position_correction_total_translation <= 0.739580,
        "E4 position-row work should not increase late correction pressure; report={report:?}"
    );
    assert_eq!(
        report.first_floor_exit_frame, None,
        "persistent manifold warm-start must keep the 180-frame 8x6 stress run free of floor ejection; report={report:?}"
    );
    assert_eq!(
        report.final_outside_floor_body_count, 0,
        "E4 position-row work must keep all 8x6 bodies inside the floor support at the final frame; report={report:?}"
    );
    assert_eq!(
        report.final_awake_dynamic_body_count, 0,
        "E5 resting-island sleep should let the 180-frame 8x6 stress run settle instead of requiring late solver impulses; report={report:?}"
    );
    assert_eq!(
        report.final_sleeping_dynamic_body_count, 48,
        "E5 resting-island sleep should keep every 8x6 dynamic body asleep at the 180-frame endpoint; report={report:?}"
    );
    assert!(
        report.late_linear_spike_trace.contact_count > 0,
        "sleeping islands should still retain late contact facts for inspection; report={report:?}"
    );
    assert!(
        report.late_linear_spike_trace.normal_speed_min <= 0.0,
        "sleeping late support facts should not report separating motion at the 180-frame endpoint; report={report:?}"
    );
    assert!(markdown.contains("- Early pressure trace f=12..46:"));
    assert!(markdown.contains("anchor_drift max"));
    assert!(markdown.contains("correction_row frame"));
    assert!(markdown.contains("row_coupling correction_tangent"));
    assert!(markdown.contains("warm_start reasons ["));
    assert!(markdown.contains("- Feature churn trace:"));
    assert!(
        markdown.contains("- E2 shadow direction:"),
        "E2 report should expose a shadow-only routing hint without hard-locking a specific milestone decision; markdown={markdown}"
    );
    assert!(
        markdown.contains("dry_run "),
        "E2 report should surface the source-row dry-run decision without treating the current decision as a permanent behavior contract; markdown={markdown}"
    );
    assert!(
        markdown.contains("same_shape_signature"),
        "E4b lifecycle diagnostics should report whether feature-id churn stays within the same shape signature; markdown={markdown}"
    );
    assert!(
        markdown.contains("close_local_anchor"),
        "E4b lifecycle diagnostics should report collider-local anchor continuity for feature-id churn; markdown={markdown}"
    );
    assert!(
        markdown.contains("edge_swap_transition"),
        "E4b lifecycle diagnostics should report reference/incident edge-swap churn separately; markdown={markdown}"
    );
    assert!(
        markdown.contains("edge_swap_candidate"),
        "E4b lifecycle diagnostics should report how many edge-swap churn cases satisfy the conservative persistence candidate inputs; markdown={markdown}"
    );
    assert!(
        markdown.contains("point_slot_fallback_eligible"),
        "E4b lifecycle diagnostics should report how often the retained point-slot fallback is applicable; markdown={markdown}"
    );
    assert!(
        markdown.contains("top_feature_index_transition"),
        "E4b lifecycle diagnostics should decode the dominant feature-index transition; markdown={markdown}"
    );
    assert!(
        markdown.contains("top_transition_best frame"),
        "E4b lifecycle diagnostics should report a concrete best sample for the dominant feature-index transition; markdown={markdown}"
    );
    assert!(
        report.feature_churn_trace.miss_feature_id_count > 0,
        "E4b lifecycle diagnostics should preserve feature-id churn in the current 8x6 stress case; report={report:?}"
    );
    assert!(
        report.feature_churn_trace.same_pair_previous_count > 0,
        "E4b lifecycle diagnostics should identify feature churn on collider pairs that existed in the previous frame; report={report:?}"
    );
    assert!(
        report.feature_churn_trace.same_shape_signature_count > 0,
        "E4b lifecycle diagnostics should identify churn where collider shape signatures stayed compatible; report={report:?}"
    );
    assert!(
        report.feature_churn_trace.close_local_anchor_count > 0,
        "E4b lifecycle diagnostics should identify churn where collider-local anchors stayed close; report={report:?}"
    );
    assert!(
        report.feature_churn_trace.edge_swap_transition_count > 0,
        "E4b lifecycle diagnostics should identify feature churn that swaps reference/incident edges; report={report:?}"
    );
    assert!(
        report.feature_churn_trace.edge_swap_candidate_count > 0,
        "E4b lifecycle diagnostics should identify edge-swap churn with compatible reduction, shape, normal, point, and local anchors; report={report:?}"
    );
    assert!(
        report
            .feature_churn_trace
            .top_transition_best_frame
            .is_some(),
        "E4b lifecycle diagnostics should locate the dominant transition on a concrete frame; report={report:?}"
    );
    assert!(markdown.contains("- Late correction/churn f>=120:"));
    assert!(markdown.contains("- Floor ejection:"));
    assert!(markdown.contains("- Ejection trace:"));
    assert!(markdown.contains("- Ejection dynamic-support trace:"));
    assert!(markdown.contains("- Ejection dynamic-impulse trace:"));
    assert!(markdown.contains("- Ejection velocity trace:"));
    assert!(markdown.contains("- Ejection support-gap trace:"));
    assert!(markdown.contains("- Ejection support-gap upstream trace:"));
    assert!(markdown.contains("- Solver row max:"));
    assert_eq!(report.final_state_hash, run.manifest.final_state_hash);
    assert!(
        report.max_contact_row_count >= 12,
        "stress report should capture the dense contact-row pressure visible in StepStats"
    );
    assert!(
        report.early_pressure_trace.max_contact_row_count >= 12,
        "early pressure trace should capture row pressure before the late velocity spike; report={report:?}"
    );
    assert!(
        report.max_penetration_depth <= 0.1,
        "E3 dense matrix correction should keep stress penetration below the interim target; report={report:?}"
    );
    assert!(
        report.first_bad_frame.is_none() || !report.first_bad_markers.is_empty(),
        "when a first bad frame exists, the report should preserve its marker story"
    );
}

#[test]
fn aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::MatrixStackAligned,
            frame_count: 1200,
            run_id: Some("matrix-stack-aligned-behavior-lock".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("aligned matrix stack run should write artifacts");

    assert_eq!(run.manifest.scenario_id, ScenarioId::MatrixStackAligned);

    let first = run
        .frames
        .first()
        .expect("aligned matrix stack should write frames");
    let dynamic_bodies = first
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
        .count();
    assert_eq!(
        dynamic_bodies, 12,
        "aligned matrix lock should be a 4x3 dynamic-body grid"
    );

    let report = matrix_stack_stress_report(&run);
    println!("{}", report.to_markdown(&run));
    assert_eq!(
        report.final_awake_dynamic_body_count, 0,
        "aligned matrix lock should let the whole 4x3 stack converge to sleep; report={report:?}"
    );
    assert_eq!(
        report.final_sleeping_dynamic_body_count, 12,
        "aligned matrix lock should preserve every dynamic body through sleep convergence; report={report:?}"
    );
    assert!(
        report.max_penetration_depth <= 0.04,
        "aligned matrix lock should keep small NxM penetration under the D2 behavior target; report={report:?}"
    );
    assert_eq!(
        report.first_floor_exit_frame, None,
        "aligned matrix lock should not eject any body from the floor support; report={report:?}"
    );
    assert_eq!(
        report.final_outside_floor_body_count, 0,
        "aligned matrix lock should finish with every body inside floor support; report={report:?}"
    );
    assert!(
        report.final_max_linear_speed <= 0.2,
        "aligned matrix lock should keep final linear speed bounded through sleep convergence; report={report:?}"
    );
    assert!(
        report.final_max_angular_speed <= 0.6,
        "aligned matrix lock should keep final angular speed bounded through sleep convergence; report={report:?}"
    );

    let final_max_abs_rotation = run
        .frames
        .last()
        .expect("aligned matrix stack should write a final frame")
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
        .map(|body| body.transform.rotation.abs())
        .fold(0.0, f32::max);
    assert!(
        final_max_abs_rotation <= 0.01,
        "aligned matrix lock should not settle into a one-sided tilted stack; max_abs_rotation={final_max_abs_rotation}; report={report:?}"
    );
}

#[test]
#[ignore = "diagnostic baseline for the current matrix-stack instability story"]
fn matrix_stack_stress_report_repeats_the_current_first_bad_frame_story() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::MatrixStack,
            frame_count: 180,
            run_id: Some("matrix-stack-diagnostic-baseline".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("matrix stack diagnostic run should write artifacts");

    let report = matrix_stack_stress_report(&run);
    println!("{}", report.to_markdown(&run));
    assert!(
        report.first_bad_frame.is_some(),
        "the current dense stack issue should surface a comparable first-bad-frame marker"
    );
    assert!(
        report.max_penetration_depth > 0.04,
        "the current baseline should still reproduce a penetration depth above the clean-gate target"
    );
    assert!(
        report.final_awake_dynamic_body_count > 0,
        "the current baseline should still expose the unresolved sleep convergence story"
    );
}

#[test]
#[ignore = "long-settle observation for the deferred writeback matrix-stack stability risk"]
fn matrix_stack_long_settle_observation_reports_residual_e4_risk() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::MatrixStack,
            frame_count: 600,
            run_id: Some("matrix-stack-long-settle-observation".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("matrix stack long-settle diagnostic run should write artifacts");

    let report = matrix_stack_stress_report(&run);
    println!("{}", report.to_markdown(&run));
    assert_eq!(run.frames.len(), 600);
    assert!(
        report.first_bad_frame.is_some(),
        "long-settle observation should preserve that current 8x6 is not yet a stable hard gate; report={report:?}"
    );
    assert!(
        report.final_awake_dynamic_body_count > 0 || report.final_outside_floor_body_count > 0,
        "long-settle observation should expose either sleep non-convergence or floor-support failure until E4/E5 is complete; report={report:?}"
    );
    assert_eq!(
        report.first_floor_exit_frame,
        Some(245),
        "long-settle observation should keep the current first ejection frame explicit until the next stability slice removes it; report={report:?}"
    );
    assert_eq!(
        report.first_floor_exit_body.as_deref(),
        Some("BodyHandle(41)"),
        "long-settle observation should identify the current ejected body until E4 removes the failure; report={report:?}"
    );
    assert_eq!(
        report.ejection_support_gap_start_frame,
        Some(239),
        "long-settle observation should preserve the pre-ejection dynamic support gap; report={report:?}"
    );
    assert_eq!(
        report
            .late_linear_spike_trace
            .support_gap_start_frame,
        Some(239),
        "long-settle observation should preserve the post-ejection long support gap story; report={report:?}"
    );
    assert!(
        report.quiet_window_max_linear_speed > 60.0,
        "long-settle observation should keep the current runaway-speed risk visible; report={report:?}"
    );
}

#[test]
#[ignore = "future support-retention behavior lock; position re-evaluation must remove the current frame-245 ejection"]
fn matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::MatrixStack,
            frame_count: 240,
            run_id: Some("matrix-stack-e4c-support-gap-lock".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("matrix stack E4c support-gap run should write artifacts");

    let report = matrix_stack_stress_report(&run);
    println!("{}", report.to_markdown(&run));
    assert_eq!(run.frames.len(), 240);
    if report.ejection_support_gap_start_frame.is_some() {
        assert!(
            report.ejection_support_gap_previous_max_depth > 0.0,
            "E4c blocker should preserve that the last pre-gap dynamic support was still geometrically overlapping; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_previous_normal_speed_min > 0.0,
            "E4c blocker should preserve that the last pre-gap support was separating before contact loss; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_previous_counterpart_angular_speed_max > 1.0,
            "E4c blocker should expose when the previous support counterpart is not a static/quiet support candidate; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_previous_counterpart_tangent_angular_component_max
                > SUPPORT_ELIGIBILITY_COUNTERPART_TANGENT_THRESHOLD,
            "E4c blocker should expose that naive support retention would violate the counterpart-motion rejection gate; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_previous_position_correction_depth_sum <= f32::EPSILON,
            "E4c blocker should preserve that the pre-gap shallow support no longer had position correction; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_previous_normal_anchor_drift_max <= f32::EPSILON,
            "E4c blocker should preserve local-anchor continuity separately from the counterpart-motion rejection; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_frame.is_some(),
            "E4c blocker should expose where the high-motion counterpart came from before retaining or rejecting support; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_counterpart_angular_speed > 1.0,
            "E4c upstream trace should preserve the high-angular counterpart source window; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_source_contact_count > 0,
            "E4c upstream trace should show whether the high-motion counterpart is still coupled to other stack contacts; report={report:?}"
        );
        assert!(
            !report
                .ejection_support_gap_upstream_source_warm_start_reasons
                .is_empty(),
            "E4c upstream trace should expose whether the source contact was warm-started or re-solved cold; report={report:?}"
        );
        assert!(
            report
                .ejection_support_gap_upstream_source_continuity_reasons
                .contains(&format!("{:?}", SourceRowContinuityReason::Candidate)),
            "E4c upstream trace should consume core-owned source-row continuity reasons instead of re-inferring every candidate; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_source_position_bias_max <= f32::EPSILON,
            "E4c upstream trace should preserve whether velocity-level position bias participated in the source contact; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_source_correction_depth_sum
                > report.ejection_support_gap_upstream_source_normal_impulse_sum,
            "E4c upstream trace should expose source contacts dominated by residual position correction rather than normal impulse; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_source_same_pair_previous_count > 0,
            "E4c upstream trace should expose whether the source row had a previous same-pair contact; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_source_candidate_count > 0,
            "E4c upstream trace should expose source-row continuity candidates before wiring position-row gating; report={report:?}"
        );
        assert!(
            report
                .ejection_support_gap_upstream_source_local_anchor_drift_min
                .is_some_and(|drift| drift <= FEATURE_CHURN_LOCAL_ANCHOR_DRIFT_THRESHOLD),
            "E4c upstream trace should expose local-anchor continuity before changing residual correction eligibility; report={report:?}"
        );
        assert_eq!(
            report.ejection_support_gap_upstream_source_edge_swap_candidate_count, 0,
            "E4c upstream trace should distinguish this source row from the E4b edge-swap persistence path; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_source_dry_run_geometry_count > 0,
            "E4c source-row dry-run should prove the candidate still has local geometry continuity before any correction gating; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_source_dry_run_pressure_count > 0,
            "E4c source-row dry-run should prove the candidate is a residual position-correction pressure source; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_source_dry_run_counterpart_reject_count > 0,
            "E4c source-row dry-run should reject the current candidate on counterpart motion before retaining or suppressing its correction; report={report:?}"
        );
        assert_eq!(
            report
                .ejection_support_gap_upstream_source_dry_run_decision
                .as_str(),
            "reject_counterpart_motion",
            "E4c source-row dry-run should keep the next implementation from treating this row as an eligible quiet support; report={report:?}"
        );
        assert!(
            report
                .ejection_support_gap_upstream_source_dry_run_pseudo_depth_min
                .is_some_and(|depth| depth > 0.0),
            "E4c source-row pseudo-state dry-run should preserve positive residual depth for the upstream pressure source; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_source_dry_run_pseudo_support_skin_count > 0,
            "E4c source-row pseudo-state dry-run should expose whether repeated position iterations keep the row inside the support skin; report={report:?}"
        );
        assert!(
            report
                .ejection_support_gap_upstream_handoff_frame_delta
                .is_some_and(|delta| delta <= 3),
            "E4c handoff trace should prove the upstream pressure source is temporally adjacent to the support-gap counterpart; report={report:?}"
        );
        assert!(
            report.ejection_support_gap_upstream_handoff_linear_speed_delta > 0.0,
            "E4c handoff trace should expose that the same counterpart gains speed before the support-gap frame; report={report:?}"
        );
    }
    assert_eq!(
        report.ejection_support_gap_start_frame,
        None,
        "E4c must remove the current pre-ejection support-gap onset before it can eject a body; report={report:?}"
    );
    assert_eq!(
        report.late_linear_spike_trace.support_gap_start_frame,
        None,
        "E4c must remove the current post-ejection long support gap in the first support-gap window; report={report:?}"
    );
    assert_eq!(
        report.first_floor_exit_frame, None,
        "E4c must keep the current support-gap failure from ejecting a body in the first 240 frames; report={report:?}"
    );
    assert_eq!(
        report.final_outside_floor_body_count, 0,
        "E4c must keep every body inside floor support after the support-gap window; report={report:?}"
    );
    assert!(
        report.quiet_window_max_linear_speed <= 4.0,
        "E4c must bound runaway linear speed in the first support-gap window before E5 sleep tuning; report={report:?}"
    );
}

#[test]
#[ignore = "future E4/E5 acceptance gate; set PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 to enforce it"]
fn matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed() {
    if env::var_os("PICEA_MATRIX_STACK_E4_ACCEPTANCE").is_none() {
        eprintln!(
            "skipping future matrix-stack acceptance gate; set PICEA_MATRIX_STACK_E4_ACCEPTANCE=1"
        );
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::MatrixStack,
            frame_count: 600,
            run_id: Some("matrix-stack-long-settle-acceptance".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("matrix stack long-settle acceptance run should write artifacts");

    let report = matrix_stack_stress_report(&run);
    println!("{}", report.to_markdown(&run));
    assert_eq!(run.frames.len(), 600);
    assert_eq!(
        report.first_floor_exit_frame, None,
        "stable 8x6 matrix stack must not eject bodies over the long settle window; report={report:?}"
    );
    assert_eq!(
        report.final_outside_floor_body_count, 0,
        "stable 8x6 matrix stack must finish with every body inside floor support; report={report:?}"
    );
    assert!(
        report.max_penetration_depth <= 0.05,
        "stable 8x6 matrix stack should keep long-window penetration bounded before E5 sleep tuning; report={report:?}"
    );
    assert!(
        report.quiet_window_max_linear_speed <= 4.0,
        "stable 8x6 matrix stack must remove the long-window runaway linear speed before E5 sleep tuning; report={report:?}"
    );
    assert!(
        report.quiet_window_max_angular_speed <= 6.0,
        "stable 8x6 matrix stack must remove the long-window runaway angular speed before E5 sleep tuning; report={report:?}"
    );
}

#[test]
#[ignore = "future E5 sleep convergence gate; set PICEA_MATRIX_STACK_E5_SLEEP_ACCEPTANCE=1 to enforce it"]
fn matrix_stack_long_run_acceptance_requires_resting_sleep_convergence() {
    if env::var_os("PICEA_MATRIX_STACK_E5_SLEEP_ACCEPTANCE").is_none() {
        eprintln!(
            "skipping future matrix-stack E5 sleep gate; set PICEA_MATRIX_STACK_E5_SLEEP_ACCEPTANCE=1"
        );
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::MatrixStack,
            frame_count: 1200,
            run_id: Some("matrix-stack-e5-sleep-convergence".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("matrix stack E5 sleep-convergence run should write artifacts");

    let report = matrix_stack_stress_report(&run);
    println!("{}", report.to_markdown(&run));
    assert_eq!(run.frames.len(), 1200);
    assert_eq!(
        report.first_floor_exit_frame, None,
        "stable 8x6 matrix stack must not drift out of floor support before sleeping; report={report:?}"
    );
    assert_eq!(
        report.final_outside_floor_body_count, 0,
        "stable 8x6 matrix stack must finish with every body inside floor support; report={report:?}"
    );
    assert!(
        report.quiet_window_max_linear_speed <= 4.0,
        "stable 8x6 matrix stack must avoid late-window runaway before sleep convergence; report={report:?}"
    );
    assert!(
        report.quiet_window_max_angular_speed <= 6.0,
        "stable 8x6 matrix stack must avoid late-window spin before sleep convergence; report={report:?}"
    );
    assert_eq!(
        report.final_awake_dynamic_body_count, 0,
        "stable 8x6 matrix stack should end with no awake dynamic bodies after the long settle window; report={report:?}"
    );
    assert_eq!(
        report.final_sleeping_dynamic_body_count, 48,
        "stable 8x6 matrix stack should end with every dynamic body sleeping after the long settle window; report={report:?}"
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
        "solver_initial_normal_speed",
        "solver_initial_tangent_speed",
        "solver_final_normal_speed",
        "solver_final_tangent_speed",
        "solver_position_bias",
        "solver_restitution_bias",
        "solver_support_friction_impulse",
        "solver_normal_impulse_delta",
        "solver_tangent_impulse_delta",
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
    let kinematic_body_count = first
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Kinematic)
        .count();
    assert!(
        dynamic_body_count + kinematic_body_count >= 9,
        "lattice proxy should export a grid of node bodies"
    );
    assert!(
        dynamic_body_count >= 6 && kinematic_body_count >= 2,
        "lattice proxy should export a fixed kinematic edge plus dynamic nodes"
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
fn newton_cradle_artifact_retains_first_cycle_kinetic_motion() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::NewtonCradle,
            frame_count: 720,
            run_id: Some("newton-cradle-first-cycle-kinetic".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("newton cradle run should write artifacts");

    assert_eq!(run.manifest.scenario_id, ScenarioId::NewtonCradle);
    assert_eq!(run.frames.len(), 720);
    let first = run.frames.first().expect("first frame should exist");
    assert_eq!(
        first
            .snapshot
            .bodies
            .iter()
            .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
            .count(),
        5,
        "newton cradle should export five dynamic bobs"
    );
    assert_eq!(
        first
            .snapshot
            .joints
            .iter()
            .filter(|joint| joint.kind == picea::debug::DebugJointKind::Distance)
            .count(),
        5,
        "newton cradle should export one suspension joint per bob"
    );

    let first_transfer_peak = peak_dynamic_kinetic_energy(&run.frames[30..90]);
    let first_return_peak = peak_dynamic_kinetic_energy(&run.frames[90..240]);
    let later_cycle_peak = peak_dynamic_kinetic_energy(&run.frames[540..720]);
    assert!(
        first_transfer_peak > 0.2,
        "newton cradle should convert the release into visible first-transfer kinetic motion, got {first_transfer_peak}"
    );
    assert!(
        first_return_peak > 0.18 && later_cycle_peak > 0.05,
        "newton cradle should keep visible kinetic motion through the first return: first_return_peak={first_return_peak}, later_cycle_peak={later_cycle_peak}"
    );
}

#[test]
fn newton_cradle_artifact_keeps_suspension_length_bound() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::NewtonCradle,
            frame_count: 240,
            run_id: Some("newton-cradle-suspension-bound".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("newton cradle run should write artifacts");

    let max_length = max_debug_joint_length(&run.frames);
    assert!(
        max_length <= 1.36,
        "newton cradle suspension should keep each bob near its 1.28u string length, got max_length={max_length}"
    );
}

#[test]
fn newton_cradle_artifact_keeps_suspension_velocity_tangent() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::NewtonCradle,
            frame_count: 6000,
            run_id: Some("newton-cradle-tangent-velocity".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("newton cradle run should write artifacts");

    let radial_speed = max_distance_joint_radial_speed(&run.frames[360..6000]);
    assert!(
        radial_speed <= 0.08,
        "distance-joint suspension should remove radial velocity so the bobs swing instead of pushing through the rope, got {radial_speed}"
    );
    let max_length = max_debug_joint_length(&run.frames);
    let max_escape = max_dynamic_center_escape(&run.frames);
    let late_window_peak = peak_dynamic_kinetic_energy(&run.frames[3000..6000]);
    assert!(
        max_length <= 1.42 && max_escape <= 3.5,
        "newton cradle should keep all bobs visible and attached over the long window: max_length={max_length}, max_escape={max_escape}"
    );
    assert!(
        late_window_peak >= 0.08,
        "newton cradle should retain a visible kinetic envelope over the long window, got {late_window_peak}"
    );
}

#[test]
fn newton_cradle_artifact_releases_left_bob_and_transfers_to_right_bob() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::NewtonCradle,
            frame_count: 360,
            run_id: Some("newton-cradle-transfer-pattern".to_owned()),
            ..RunConfig::default()
        },
    )
    .expect("newton cradle run should write artifacts");

    let first_frame_speeds = dynamic_body_speeds(run.frames.first().expect("first frame"));
    assert!(
        first_frame_speeds.iter().all(|speed| *speed <= 0.2),
        "newton cradle should start from a raised release, not an injected horizontal velocity: {first_frame_speeds:?}"
    );

    let first_frame_y = dynamic_body_centers(run.frames.first().expect("first frame"))
        .into_iter()
        .map(|center| center.y())
        .collect::<Vec<_>>();
    assert!(
        first_frame_y[0] < first_frame_y[1] - 0.12,
        "the released left bob should start visibly raised above the middle bobs: {first_frame_y:?}"
    );

    let peak_speeds = peak_dynamic_body_speeds(&run.frames[38..85]);
    let middle_peak = peak_speeds[1..4].iter().copied().fold(0.0, f32::max);
    assert!(
        peak_speeds[4] >= 0.8 && peak_speeds[4] >= middle_peak * 5.0,
        "rightmost bob should carry the first post-impact swing while middle bobs mostly transmit impulse: peak_speeds={peak_speeds:?}"
    );
}

#[test]
fn newton_cradle_scene_params_affect_manifest_world_and_solver_iterations() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let store = ArtifactStore::new(temp.path().join("runs"));
    let mut scene_params = BTreeMap::new();
    scene_params.insert("ball_count".to_owned(), json!(3));
    scene_params.insert("radius".to_owned(), json!(0.16));
    scene_params.insert("string_length".to_owned(), json!(1.1));
    scene_params.insert("release_offset".to_owned(), json!(0.45));
    scene_params.insert("restitution".to_owned(), json!(0.9));
    scene_params.insert("friction".to_owned(), json!(0.05));
    scene_params.insert("velocity_iterations".to_owned(), json!(4));
    scene_params.insert("position_iterations".to_owned(), json!(7));
    scene_params.insert("contact_position_correction".to_owned(), json!("disabled"));
    scene_params.insert("joint_velocity_projection".to_owned(), json!(false));

    let run = run_scenario(
        &store,
        RunConfig {
            scenario_id: ScenarioId::NewtonCradle,
            frame_count: 8,
            run_id: Some("newton-cradle-scene-params".to_owned()),
            overrides: ScenarioOverrides {
                scene_params,
                ..ScenarioOverrides::default()
            },
        },
    )
    .expect("newton cradle run with scene params should write artifacts");

    assert_eq!(run.manifest.scenario_id, ScenarioId::NewtonCradle);
    assert_eq!(
        run.manifest
            .effective_runtime_config
            .step
            .velocity_iterations,
        4
    );
    assert_eq!(
        run.manifest
            .effective_runtime_config
            .step
            .position_iterations,
        7
    );
    assert!(
        !run.manifest
            .effective_runtime_config
            .step
            .joint_velocity_projection
    );
    assert_eq!(
        run.manifest.effective_runtime_config.scene_params["ball_count"],
        json!(3)
    );

    let first = run.frames.first().expect("first frame should exist");
    assert_eq!(
        first
            .snapshot
            .bodies
            .iter()
            .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
            .count(),
        3,
        "scene_params.ball_count should change the authored bob count"
    );
    assert_eq!(
        first
            .snapshot
            .joints
            .iter()
            .filter(|joint| joint.kind == picea::debug::DebugJointKind::Distance)
            .count(),
        3,
        "scene_params.ball_count should change the suspension joint count"
    );
    assert_eq!(first.stats.velocity_iterations, 4);
    assert_eq!(first.stats.position_iterations, 7);

    let manifest: RunManifest = serde_json::from_slice(
        &fs::read(run.path.join(ArtifactFile::Manifest.file_name()))
            .expect("manifest should be readable"),
    )
    .expect("manifest should match schema");
    assert_eq!(
        manifest.effective_runtime_config,
        run.manifest.effective_runtime_config
    );
}

fn peak_dynamic_kinetic_energy(frames: &[FrameRecord]) -> f32 {
    frames
        .iter()
        .map(|frame| {
            frame
                .snapshot
                .bodies
                .iter()
                .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
                .map(|body| {
                    let speed_squared = body.linear_velocity.x() * body.linear_velocity.x()
                        + body.linear_velocity.y() * body.linear_velocity.y();
                    0.5 * body.mass_properties.mass * speed_squared
                        + 0.5
                            * body.mass_properties.inertia
                            * body.angular_velocity
                            * body.angular_velocity
                })
                .sum::<f32>()
        })
        .fold(0.0, f32::max)
}

fn dynamic_body_centers(frame: &FrameRecord) -> Vec<picea::prelude::Vector> {
    frame
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
        .map(|body| body.transform.translation)
        .collect()
}

fn dynamic_body_speeds(frame: &FrameRecord) -> Vec<f32> {
    frame
        .snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
        .map(|body| body.linear_velocity.length())
        .collect()
}

fn peak_dynamic_body_speeds(frames: &[FrameRecord]) -> Vec<f32> {
    let body_count = frames
        .first()
        .map(dynamic_body_speeds)
        .map(|speeds| speeds.len())
        .unwrap_or(0);
    let mut peaks = vec![0.0; body_count];
    for frame in frames {
        for (index, speed) in dynamic_body_speeds(frame).into_iter().enumerate() {
            peaks[index] = f32::max(peaks[index], speed);
        }
    }
    peaks
}

fn max_debug_joint_length(frames: &[FrameRecord]) -> f32 {
    frames
        .iter()
        .flat_map(|frame| &frame.snapshot.joints)
        .filter_map(|joint| {
            let [start, end] = joint.anchors.as_slice() else {
                return None;
            };
            let dx = end.x() - start.x();
            let dy = end.y() - start.y();
            Some((dx * dx + dy * dy).sqrt())
        })
        .fold(0.0, f32::max)
}

fn max_distance_joint_radial_speed(frames: &[FrameRecord]) -> f32 {
    frames
        .iter()
        .flat_map(|frame| {
            frame.snapshot.joints.iter().filter_map(move |joint| {
                let [anchor, bob] = joint.anchors.as_slice() else {
                    return None;
                };
                let [_, bob_handle] = joint.bodies.as_slice() else {
                    return None;
                };
                let bob_body = frame
                    .snapshot
                    .bodies
                    .iter()
                    .find(|body| body.handle == *bob_handle)?;
                let delta = *bob - *anchor;
                let length = delta.length();
                if length <= f32::EPSILON {
                    return None;
                }
                let direction = delta / length;
                Some(bob_body.linear_velocity.dot(direction).abs())
            })
        })
        .fold(0.0, f32::max)
}

fn max_dynamic_center_escape(frames: &[FrameRecord]) -> f32 {
    frames
        .iter()
        .flat_map(|frame| {
            frame
                .snapshot
                .bodies
                .iter()
                .filter(|body| body.body_type == picea::prelude::BodyType::Dynamic)
                .map(|body| {
                    body.transform
                        .translation
                        .x()
                        .abs()
                        .max(body.transform.translation.y().abs())
                })
        })
        .fold(0.0, f32::max)
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
        "position_correction_input_contact_count",
        "position_correction_input_max_depth",
        "position_correction_input_total_depth",
        "position_correction_body_count",
        "position_correction_max_translation",
        "position_correction_total_translation",
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
        "position_correction_input_contact_count",
        "position_correction_input_max_depth",
        "position_correction_input_total_depth",
        "position_correction_body_count",
        "position_correction_max_translation",
        "position_correction_total_translation",
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
        position_correction_input_contact_count: 1,
        position_correction_input_max_depth: 0.2,
        position_correction_input_total_depth: 0.3,
        position_correction_body_count: 1,
        position_correction_max_translation: 0.1,
        position_correction_total_translation: 0.1,
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
    object.remove("position_correction_input_contact_count");
    object.remove("position_correction_input_max_depth");
    object.remove("position_correction_input_total_depth");
    object.remove("position_correction_body_count");
    object.remove("position_correction_max_translation");
    object.remove("position_correction_total_translation");
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
    assert_eq!(decoded.position_correction_input_contact_count, 0);
    assert_eq!(decoded.position_correction_input_max_depth, 0.0);
    assert_eq!(decoded.position_correction_input_total_depth, 0.0);
    assert_eq!(decoded.position_correction_body_count, 0);
    assert_eq!(decoded.position_correction_max_translation, 0.0);
    assert_eq!(decoded.position_correction_total_translation, 0.0);
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
