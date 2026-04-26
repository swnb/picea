import type { DebugAabb, DebugSnapshot, FrameRecord, ScenarioDescriptor, Vec2 } from "./types";

export const demoScenarios: ScenarioDescriptor[] = [
  {
    id: "falling_box_contact",
    name: "Falling box contact",
    description: "Demo fallback with a dynamic box, floor contact, AABB and trajectory facts.",
  },
  {
    id: "stack_4",
    name: "Four box stack",
    description: "Offline stack preview for smoke builds without the Rust server.",
  },
  {
    id: "joint_anchor",
    name: "World anchor joint",
    description: "Offline joint anchor preview with a constraint line.",
  },
];

const floorAabb: DebugAabb = {
  min: { x: -4.5, y: 1.75 },
  max: { x: 4.5, y: 2.25 },
};

export function makeDemoFrames(scenarioId = "falling_box_contact", frameCount = 96): FrameRecord[] {
  if (scenarioId === "joint_anchor") {
    return makeJointFrames(frameCount);
  }
  if (scenarioId === "stack_4") {
    return makeStackFrames(frameCount);
  }
  return makeFallingBoxFrames(frameCount);
}

function makeFallingBoxFrames(frameCount: number): FrameRecord[] {
  const trace: Vec2[] = [];
  let hasTouched = false;

  return Array.from({ length: frameCount }, (_, frameIndex) => {
    const t = frameIndex / Math.max(1, frameCount - 1);
    const y = Math.min(1.18, -2.25 + 3.8 * easeOutCubic(t));
    const velocityY = y < 1.17 ? 3.2 * (1 - t) : 0;
    const center = { x: 0.35 * Math.sin(t * Math.PI * 2), y };
    trace.push(center);
    const touching = y >= 1.12;
    const snapshot = baseSnapshot(frameIndex, frameIndex / 60, [
      body(1, "static", { x: 0, y: 2 }, { x: 0, y: 0 }),
      body(2, "dynamic", center, { x: 0.25 * Math.cos(t * Math.PI * 2), y: velocityY }),
    ]);

    snapshot.colliders = [
      collider(1, 1, floorAabb),
      collider(2, 2, boxAabb(center, 1, 1)),
    ];
    snapshot.contacts = touching
      ? [
          {
            id: 1,
            bodies: [2, 1],
            colliders: [2, 1],
            feature_id: 1,
            point: { x: center.x, y: 1.5 },
            normal: { x: 0, y: -1 },
            depth: 0.08,
            reduction_reason: "single_point",
            warm_start_reason: hasTouched ? "hit" : "miss_no_previous",
            normal_impulse: 0,
            tangent_impulse: 0,
            solver_normal_impulse: touching ? 1.2 : 0,
            solver_tangent_impulse: 0.15,
            normal_impulse_clamped: false,
            tangent_impulse_clamped: true,
            restitution_velocity_threshold: 1,
            restitution_applied: false,
          },
        ]
      : [];
    snapshot.manifolds = touching
      ? [
          {
            id: 1,
            bodies: [2, 1],
            colliders: [2, 1],
            contact_ids: [1],
            points: [{ contact_id: 1, feature_id: 1, point: { x: center.x, y: 1.5 }, depth: 0.08 }],
            normal: { x: 0, y: -1 },
            depth: 0.08,
            reduction_reason: "single_point",
            warm_start_hit_count: hasTouched ? 1 : 0,
            warm_start_miss_count: hasTouched ? 0 : 1,
            warm_start_drop_count: 0,
            active: true,
          },
        ]
      : [];
    snapshot.stats.contact_count = snapshot.contacts.length;
    snapshot.stats.manifold_count = snapshot.manifolds.length;
    snapshot.stats.warm_start_hit_count = touching && hasTouched ? snapshot.contacts.length : 0;
    snapshot.stats.warm_start_miss_count = touching && !hasTouched ? snapshot.contacts.length : 0;
    snapshot.primitives = [
      {
        kind: "polyline",
        points: trace.slice(Math.max(0, trace.length - 48)),
        closed: false,
        color: { r: 126, g: 176, b: 105, a: 255 },
      },
    ];

    const record = frame(frameIndex, snapshot);
    hasTouched ||= touching;
    return record;
  });
}

function makeStackFrames(frameCount: number): FrameRecord[] {
  return Array.from({ length: frameCount }, (_, frameIndex) => {
    const t = frameIndex / Math.max(1, frameCount - 1);
    const snapshot = baseSnapshot(frameIndex, frameIndex / 60, [
      body(1, "static", { x: 0, y: 2.5 }, { x: 0, y: 0 }),
      ...[0, 1, 2, 3].map((index) =>
        body(2 + index, "dynamic", { x: 0.04 * Math.sin(t * 4 + index), y: 1.72 - index * 0.92 }, {
          x: 0.02 * Math.cos(t * 4 + index),
          y: 0,
        }),
      ),
    ]);
    snapshot.colliders = [
      collider(1, 1, { min: { x: -5, y: 2.25 }, max: { x: 5, y: 2.75 } }),
      ...[0, 1, 2, 3].map((index) =>
        collider(2 + index, 2 + index, boxAabb({ x: 0.04 * Math.sin(t * 4 + index), y: 1.72 - index * 0.92 }, 0.9, 0.9)),
      ),
    ];
    snapshot.stats.contact_count = 3;
    snapshot.stats.manifold_count = 3;
    return frame(frameIndex, snapshot);
  });
}

function makeJointFrames(frameCount: number): FrameRecord[] {
  return Array.from({ length: frameCount }, (_, frameIndex) => {
    const t = frameIndex / Math.max(1, frameCount - 1);
    const center = {
      x: 1.7 * Math.cos(t * Math.PI * 2.2) * (1 - t * 0.25),
      y: 0.95 * Math.sin(t * Math.PI * 2.2),
    };
    const snapshot = baseSnapshot(frameIndex, frameIndex / 60, [
      body(1, "dynamic", center, { x: -center.y * 0.4, y: center.x * 0.4 }),
    ]);
    snapshot.colliders = [collider(1, 1, boxAabb(center, 0.8, 0.8))];
    snapshot.joints = [
      {
        handle: 1,
        kind: "world_anchor",
        bodies: [1],
        anchors: [center, { x: 0, y: 0 }],
      },
    ];
    snapshot.primitives = [
      {
        kind: "line",
        start: center,
        end: { x: 0, y: 0 },
        color: { r: 216, g: 173, b: 91, a: 255 },
      },
    ];
    return frame(frameIndex, snapshot);
  });
}

function baseSnapshot(frameIndex: number, simulatedTime: number, bodies: DebugSnapshot["bodies"]): DebugSnapshot {
  return {
    meta: {
      revision: frameIndex + 1,
      dt: 1 / 60,
      simulated_time: simulatedTime,
      gravity: { x: 0, y: 9.8 },
    },
    bodies,
    colliders: [],
    joints: [],
    contacts: [],
    manifolds: [],
    primitives: [],
    stats: {
      step_index: frameIndex,
      active_body_count: bodies.length,
      active_collider_count: bodies.length,
      active_joint_count: 0,
      broadphase_candidate_count: 0,
      contact_count: 0,
      manifold_count: 0,
      warm_start_hit_count: 0,
      warm_start_miss_count: 0,
      warm_start_drop_count: 0,
    },
  };
}

function body(
  handle: number,
  bodyType: DebugSnapshot["bodies"][number]["body_type"],
  translation: Vec2,
  linearVelocity: Vec2,
): DebugSnapshot["bodies"][number] {
  return {
    handle,
    body_type: bodyType,
    transform: { translation, rotation: 0 },
    mass_properties: {
      mass: bodyType === "dynamic" ? 1 : 0,
      inverse_mass: bodyType === "dynamic" ? 1 : 0,
      local_center_of_mass: { x: 0, y: 0 },
      inertia: bodyType === "dynamic" ? 1 : 0,
      inverse_inertia: bodyType === "dynamic" ? 1 : 0,
    },
    linear_velocity: linearVelocity,
    angular_velocity: 0,
    sleeping: false,
    user_data: 0,
  };
}

function collider(handle: number, bodyHandle: number, aabb: DebugAabb): DebugSnapshot["colliders"][number] {
  const vertices = [
    { x: aabb.min.x, y: aabb.min.y },
    { x: aabb.max.x, y: aabb.min.y },
    { x: aabb.max.x, y: aabb.max.y },
    { x: aabb.min.x, y: aabb.max.y },
  ];
  return {
    handle,
    body: bodyHandle,
    local_transform: { translation: { x: 0, y: 0 }, rotation: 0 },
    world_transform: {
      translation: { x: (aabb.min.x + aabb.max.x) / 2, y: (aabb.min.y + aabb.max.y) / 2 },
      rotation: 0,
    },
    aabb,
    shape: { kind: "polygon", vertices },
    density: bodyHandle === 1 ? 0 : 1,
    material: { friction: 0.5, restitution: 0.05 },
    filter: { memberships: 1, collides_with: 4294967295 },
    is_sensor: false,
    user_data: 0,
  };
}

function boxAabb(center: Vec2, width: number, height: number): DebugAabb {
  return {
    min: { x: center.x - width / 2, y: center.y - height / 2 },
    max: { x: center.x + width / 2, y: center.y + height / 2 },
  };
}

function frame(frameIndex: number, snapshot: DebugSnapshot): FrameRecord {
  return {
    frame_index: frameIndex,
    simulated_time: snapshot.meta.simulated_time,
    state_hash: `demo-${frameIndex.toString(16).padStart(4, "0")}`,
    snapshot,
  };
}

function easeOutCubic(value: number): number {
  return 1 - Math.pow(1 - value, 3);
}
