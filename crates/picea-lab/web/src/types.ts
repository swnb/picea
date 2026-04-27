export type Vec2 = {
  x: number;
  y: number;
};

export type DebugAabb = {
  min: Vec2;
  max: Vec2;
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
  swept_start: Vec2;
  swept_end: Vec2;
  toi: number;
  advancement: number;
  clamp: number;
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
  primitives: DebugPrimitive[];
  stats: {
    step_index: number;
    active_body_count: number;
    active_collider_count: number;
    active_joint_count: number;
    broadphase_candidate_count: number;
    contact_count: number;
    manifold_count: number;
    warm_start_hit_count?: number;
    warm_start_miss_count?: number;
    warm_start_drop_count?: number;
    ccd_candidate_count?: number;
    ccd_hit_count?: number;
    ccd_miss_count?: number;
    ccd_clamp_count?: number;
  };
};

export type FrameRecord = {
  frame_index: number;
  simulated_time: number;
  state_hash: string;
  snapshot: DebugSnapshot;
};

export type ScenarioDescriptor = {
  id: string;
  name: string;
  description: string;
};

export type SessionRecord = {
  id: string;
  scenario_id: string;
  status: "created" | "running" | "paused" | "completed" | "failed";
  run_id: string | null;
  frame_count: number;
  current_frame_index: number;
  overrides: {
    frame_count?: number | null;
    gravity?: [number, number] | null;
  };
  final_state_hash: string | null;
  manifest_artifact?: string | null;
  final_snapshot_artifact?: string | null;
  last_error: string | null;
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
