export type Vec2 = {
  x: number;
  y: number;
};

export type DebugAabb = {
  min: Vec2;
  max: Vec2;
};

export type DebugBroadphaseTreeNode = {
  id: number;
  parent?: number | null;
  left?: number | null;
  right?: number | null;
  collider?: number | null;
  depth: number;
  aabb: DebugAabb;
};

export type DebugBroadphaseTree = {
  root?: number | null;
  depth: number;
  nodes: DebugBroadphaseTreeNode[];
};

export type DebugShape =
  | { kind: "circle"; center: Vec2; radius: number }
  | { kind: "polygon"; vertices: Vec2[] }
  | { kind: "segment"; start: Vec2; end: Vec2; radius: number };

export type DebugColor = {
  r: number;
  g: number;
  b: number;
  a: number;
};

export type DebugPrimitive =
  | { kind: "line"; start: Vec2; end: Vec2; color: DebugColor }
  | { kind: "polyline"; points: Vec2[]; closed: boolean; color: DebugColor }
  | { kind: "polygon"; points: Vec2[]; stroke: DebugColor; fill: DebugColor | null }
  | { kind: "circle"; center: Vec2; radius: number; color: DebugColor }
  | { kind: "arrow"; origin: Vec2; direction: Vec2; color: DebugColor }
  | { kind: "label"; position: Vec2; text: string; color: DebugColor };

export type DebugBody = {
  handle: number;
  body_type: "static" | "dynamic" | "kinematic";
  transform: {
    translation: Vec2;
    rotation: number;
  };
  mass_properties: {
    mass: number;
    inverse_mass: number;
    local_center_of_mass: Vec2;
    inertia: number;
    inverse_inertia: number;
  };
  linear_velocity: Vec2;
  angular_velocity: number;
  sleeping: boolean;
  island_id?: number | null;
  user_data: number;
};

export type DebugCollider = {
  handle: number;
  body: number;
  local_transform: {
    translation: Vec2;
    rotation: number;
  };
  world_transform: {
    translation: Vec2;
    rotation: number;
  };
  aabb: DebugAabb | null;
  shape: DebugShape;
  density: number;
  material: {
    friction: number;
    restitution: number;
  };
  filter: {
    memberships: number;
    collides_with: number;
  };
  is_sensor: boolean;
  user_data: number;
};

export type DebugJoint = {
  handle: number;
  kind: "distance" | "world_anchor";
  bodies: number[];
  anchors: Vec2[];
};

export type ContactReductionReason =
  | "single_point"
  | "clipped"
  | "duplicate_reduced"
  | "non_m2_fallback"
  | "generic_convex_fallback";

export type GenericConvexTrace = {
  fallback_reason: "none" | "generic_convex_fallback" | "epa_failure_contained";
  gjk_termination:
    | "unknown"
    | "separated"
    | "touching"
    | "intersect"
    | "degenerate_direction"
    | "max_iterations"
    | "invalid_support";
  epa_termination:
    | "unknown"
    | "converged"
    | "gjk_did_not_intersect"
    | "degenerate_edge"
    | "max_iterations"
    | "invalid_support";
  gjk_iterations: number;
  epa_iterations: number;
  simplex_len: number;
};

export type CcdTrace = {
  moving_body: number;
  static_body: number;
  moving_collider: number;
  static_collider: number;
  target_kind?: "static" | "dynamic";
  swept_start: Vec2;
  swept_end: Vec2;
  target_swept_start?: Vec2;
  target_swept_end?: Vec2;
  toi: number;
  advancement: number;
  clamp: number;
  target_clamp?: number;
  slop: number;
  toi_point: Vec2;
};

export type DebugContact = {
  id: number;
  bodies: [number, number];
  colliders: [number, number];
  feature_id: number;
  point: Vec2;
  normal: Vec2;
  depth: number;
  reduction_reason: ContactReductionReason;
  warm_start_reason?:
    | "hit"
    | "miss_no_previous"
    | "miss_feature_id"
    | "miss_previous_sensor"
    | "skipped_sensor"
    | "dropped_normal_mismatch"
    | "dropped_point_drift"
    | "dropped_invalid_impulse";
  normal_impulse: number;
  tangent_impulse: number;
  solver_normal_impulse?: number;
  solver_tangent_impulse?: number;
  normal_impulse_clamped?: boolean;
  tangent_impulse_clamped?: boolean;
  restitution_velocity_threshold?: number;
  restitution_applied?: boolean;
  generic_convex_trace?: GenericConvexTrace | null;
  ccd_trace?: CcdTrace | null;
};

export type DebugManifold = {
  id: number;
  bodies: [number, number];
  colliders: [number, number];
  contact_ids: number[];
  points: Array<{
    contact_id: number;
    feature_id: number;
    point: Vec2;
    depth: number;
  }>;
  normal: Vec2;
  depth: number;
  reduction_reason: ContactReductionReason;
  generic_convex_trace?: GenericConvexTrace | null;
  warm_start_hit_count?: number;
  warm_start_miss_count?: number;
  warm_start_drop_count?: number;
  active: boolean;
};

export type SleepTransitionReason =
  | "unknown"
  | "stability_window"
  | "impact"
  | "contact_impulse"
  | "joint_correction"
  | "transform_edit"
  | "velocity_edit"
  | "user_patch"
  | "sleep_disabled";

export type DebugIsland = {
  id: number;
  bodies: number[];
  sleeping: boolean;
  reason?: SleepTransitionReason;
};

export type CompoundProvenancePiece = {
  generated_piece_index: number;
  collider_handle?: number | null;
  validation_path: string;
  local_pose: [number, number, number];
};

export type CompoundProvenance = {
  authored_body_index: number;
  body_handle?: number | null;
  validation_path: string;
  inherited_material:
    | "default"
    | "ice"
    | "rough"
    | "bouncy"
    | "sticky";
  inherited_filter:
    | "default"
    | "static_geometry"
    | "dynamic_body"
    | "sensor"
    | "query_only";
  inherited_density: number;
  inherited_is_sensor: boolean;
  pieces: CompoundProvenancePiece[];
};

export type DebugSnapshot = {
  meta: {
    revision: number | null;
    dt: number;
    simulated_time: number;
    gravity: Vec2;
  };
  bodies: DebugBody[];
  colliders: DebugCollider[];
  joints: DebugJoint[];
  contacts: DebugContact[];
  manifolds: DebugManifold[];
  islands?: DebugIsland[];
  broadphase_tree?: DebugBroadphaseTree;
  primitives: DebugPrimitive[];
  stats: {
    step_index: number;
    active_body_count: number;
    active_collider_count: number;
    active_joint_count: number;
    broadphase_candidate_count: number;
    broadphase_update_count?: number;
    broadphase_traversal_count?: number;
    broadphase_pruned_count?: number;
    broadphase_rebuild_count?: number;
    broadphase_tree_depth?: number;
    contact_count: number;
    manifold_count: number;
    island_count?: number;
    active_island_count?: number;
    sleeping_island_skip_count?: number;
    solver_body_slot_count?: number;
    contact_row_count?: number;
    joint_row_count?: number;
    warm_start_hit_count?: number;
    warm_start_miss_count?: number;
    warm_start_drop_count?: number;
    ccd_candidate_count?: number;
    ccd_hit_count?: number;
    ccd_miss_count?: number;
    ccd_clamp_count?: number;
  };
};

// Diagnostics are lab-owned summaries built from exported Rust frame facts.
// They preserve snake_case so Web mirrors the artifact contract exactly.
export type DiagnosticSource =
  | "rust_authoritative"
  | "lab_derived"
  | "web_derived"
  | "missing";

export type DiagnosticSeverity = "info" | "warning" | "severe";

export type DiagnosticMarkerKind =
  | "performance_counter_spike"
  | "solver_row_spike"
  | "ccd_spike"
  | "numeric_warning"
  | "penetration_spike"
  | "contact_churn_spike"
  | "warm_start_drop_spike"
  | "sleep_transition"
  | "sleep_never_converged"
  | "angular_drift_spike"
  | "body_drift_spike";

export type MissingEvidenceKind =
  | "previous_frame"
  | "contact_facts"
  | "contact_ids"
  | "sleep_events"
  | "island_facts";

export type MissingEvidence = {
  kind: MissingEvidenceKind;
  detail?: string;
};

export type DiagnosticMarker = {
  kind: DiagnosticMarkerKind;
  severity?: DiagnosticSeverity;
  frame_index?: number;
  score?: number;
  threshold_name?: string;
  source?: DiagnosticSource;
  body_handles?: number[];
  contact_ids?: number[];
  island_ids?: number[];
  evidence_fields?: string[];
  missing_evidence?: MissingEvidenceKind[];
};

export type PerformanceCounterDelta = {
  source?: DiagnosticSource;
  broadphase_candidate_count?: number | null;
  broadphase_traversal_count?: number | null;
  broadphase_pruned_count?: number | null;
  contact_count?: number | null;
  island_count?: number | null;
  solver_row_count?: number | null;
  ccd_candidate_count?: number | null;
};

export type FramePerformanceDiagnostics = {
  counter_delta?: PerformanceCounterDelta;
};

export type PenetrationDiagnostics = {
  source?: DiagnosticSource;
  max_depth?: number;
  total_depth?: number;
  penetrating_contact_count?: number;
};

export type ContactChurnDiagnostics = {
  source?: DiagnosticSource;
  missing_evidence?: MissingEvidenceKind[];
  entered?: number;
  persisted?: number;
  exited?: number;
};

export type WarmStartDiagnostics = {
  source?: DiagnosticSource;
  counts_source?: DiagnosticSource;
  drop_reasons_source?: DiagnosticSource;
  hit_count?: number;
  miss_count?: number;
  drop_count?: number;
  drop_reasons?: string[];
};

export type ImpulseDiagnostics = {
  source?: DiagnosticSource;
  total_normal_impulse?: number;
  total_tangent_impulse?: number;
  max_normal_impulse?: number;
  max_tangent_impulse?: number;
};

export type SleepDiagnostics = {
  source?: DiagnosticSource;
  body_counts_source?: DiagnosticSource;
  transition_count_source?: DiagnosticSource;
  reasons_source?: DiagnosticSource;
  awake_dynamic_body_count?: number;
  sleeping_dynamic_body_count?: number;
  transition_count?: number;
  reasons?: string[];
};

export type IslandDiagnostics = {
  source?: DiagnosticSource;
  island_count?: number;
  active_island_count?: number;
  sleeping_island_skip_count?: number;
  solver_body_slot_count?: number;
  contact_row_count?: number;
  joint_row_count?: number;
};

export type FrameStabilityDiagnostics = {
  penetration?: PenetrationDiagnostics;
  contact_churn?: ContactChurnDiagnostics;
  warm_start?: WarmStartDiagnostics;
  impulse?: ImpulseDiagnostics;
  sleep?: SleepDiagnostics;
  island?: IslandDiagnostics;
};

export type FrameDiagnostics = {
  performance?: FramePerformanceDiagnostics;
  stability?: FrameStabilityDiagnostics;
  markers?: DiagnosticMarker[];
  missing_evidence?: MissingEvidence[];
};

export type FrameRecord = {
  frame_index: number;
  simulated_time: number;
  state_hash: string;
  report?: StepReportRecord;
  stats?: DebugSnapshot["stats"];
  events?: WorldEventRecord[];
  snapshot: DebugSnapshot;
  compound_provenance?: CompoundProvenance[];
  perturbation_provenance?: LivePerturbationProvenance[];
  diagnostics?: FrameDiagnostics;
};

export type StepReportRecord = {
  step_index?: number;
  simulated_time?: number;
  dt?: number;
  revision?: number | null;
  stats?: DebugSnapshot["stats"];
  events?: WorldEventRecord[];
};

export type WorldEventRecord = Record<string, unknown>;

export type PerfArtifact = {
  frame_count: number;
  elapsed_micros: number | string;
  final_state_hash: string;
  counter_summary?: PerfCounterSummary;
};

export type PerfCounterSummary = {
  total_broadphase_candidate_count?: number;
  total_broadphase_traversal_count?: number;
  total_broadphase_pruned_count?: number;
  max_broadphase_tree_depth?: number;
  total_contact_count?: number;
  total_contact_row_count?: number;
  total_joint_row_count?: number;
  total_solver_body_slot_count?: number;
  total_island_count?: number;
  total_active_island_count?: number;
  total_sleeping_island_skip_count?: number;
  total_ccd_candidate_count?: number;
  total_ccd_hit_count?: number;
};

export type SessionMode = "artifact_replay" | "live_session";

export type ScenarioDescriptor = {
  id: string;
  name: string;
  description: string;
};

export type SessionRecord = {
  id: string;
  scenario_id: string;
  mode: SessionMode;
  status: "created" | "running" | "paused" | "completed" | "failed";
  session_epoch: number;
  run_id: string | null;
  frame_count: number;
  buffered_frame_count: number;
  current_frame_index: number;
  overrides: {
    frame_count?: number | null;
    gravity?: [number, number] | null;
  };
  final_state_hash: string | null;
  manifest_artifact?: string | null;
  final_snapshot_artifact?: string | null;
  latest_frame?: FrameRecord | null;
  last_error: string | null;
};

// Server-side paused edit gate rejection reasons. These are domain responses
// from the live session contract, not transport-level fetch errors.
export type VelocityPerturbationRejectionReason =
  | "not_live_session"
  | "session_created"
  | "session_running"
  | "session_completed"
  | "session_failed"
  | "missing_frame_snapshot"
  | "stale_world_revision"
  | "stale_session_epoch"
  | "stale_frame"
  | "invalid_body_handle"
  | "static_body"
  | "kinematic_body"
  | "invalid_velocity_delta"
  | "missing_preview_action"
  | "stale_preview_action"
  | "body_patch_failed"
  | "reused_action";

export type LiveQuerySyncStatus = "synced" | "stale";

export type LivePerturbationCommitOutcome = "accepted";

export type VelocityPerturbationPreview = {
  action_id: string;
  session_id: string;
  world_revision: number;
  session_epoch: number;
  body_handle: number;
  frame_index: number;
  before_velocity: Vec2 | null;
  requested_delta: Vec2 | null;
  computed_target_velocity: Vec2 | null;
  wake_intent: boolean;
  rejection_reason: VelocityPerturbationRejectionReason | null;
};

export type VelocityPerturbationCommit = {
  action_id: string;
  session_id: string;
  accepted: boolean;
  world_revision: number;
  session_epoch: number;
  body_handle: number;
  frame_index: number;
  before_velocity: Vec2 | null;
  requested_delta: Vec2 | null;
  computed_target_velocity: Vec2 | null;
  wake_intent: boolean;
  query_sync_status: LiveQuerySyncStatus | null;
  rejection_reason: VelocityPerturbationRejectionReason | null;
};

export type LivePerturbationProvenance = {
  action_id: string;
  session_id: string;
  world_revision: number;
  session_epoch: number;
  body_handle: number;
  frame_index: number;
  before_velocity: Vec2;
  requested_delta: Vec2;
  computed_target_velocity: Vec2;
  wake_intent: boolean;
  commit_outcome: LivePerturbationCommitOutcome;
  query_sync_status: LiveQuerySyncStatus;
};

export type WorkbenchLog = {
  time: string;
  level: "info" | "warn" | "error";
  message: string;
};

export type SelectedEntity =
  | { kind: "body"; id: number }
  | { kind: "collider"; id: number }
  | { kind: "contact"; id: number }
  | { kind: "joint"; id: number };
