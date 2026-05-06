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
    id: "stack_stability_tower",
    name: "Stack stability tower",
    description: "Offline stability observatory preview with a denser tower, derived markers, and overlay facts.",
  },
  {
    id: "matrix_stack",
    name: "Matrix stack 8x6",
    description: "Offline matrix stack preview for dense resting-contact diagnostics.",
  },
  {
    id: "joint_anchor",
    name: "World anchor joint",
    description: "Offline joint anchor preview with a constraint line.",
  },
  {
    id: "lattice_grid",
    name: "Rigid-body lattice grid proxy",
    description:
      "Offline rigid-body joint lattice / grid proxy with node, edge, island and stretch facts, not a true soft-body solver.",
  },
  {
    id: "compound_provenance",
    name: "Compound provenance fixture",
    description: "Offline fixture preview with broadphase tree, island facts, and authored compound provenance.",
  },
  {
    id: "ccd_fast_circle_wall",
    name: "CCD fast circle wall",
    description: "Offline CCD preview with swept path, TOI point, and clamped contact.",
  },
  {
    id: "ccd_fast_convex_walls",
    name: "CCD fast convex walls",
    description: "Offline CCD preview for a fast rectangle selecting the earliest thin-wall hit.",
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
  if (scenarioId === "lattice_grid") {
    return makeLatticeGridFrames(frameCount)
  }
  if (scenarioId === "stack_4") {
    return makeStackFrames(frameCount);
  }
  if (scenarioId === "stack_stability_tower") {
    return makeStackStabilityFrames(frameCount);
  }
  if (scenarioId === "matrix_stack") {
    return makeStackStabilityFrames(frameCount);
  }
  if (scenarioId === "compound_provenance") {
    return makeCompoundProvenanceFrames(frameCount);
  }
  if (scenarioId === "ccd_fast_circle_wall") {
    return makeCcdFrames(frameCount);
  }
  if (scenarioId === "ccd_fast_convex_walls") {
    return makeCcdConvexFrames(frameCount);
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

function makeStackStabilityFrames(frameCount: number): FrameRecord[] {
  const configs = [
    { handle: 3, width: 1.35, height: 0.48, x: 0.0, y: 1.56, phase: 0.2, sleepFrame: 74 },
    { handle: 4, width: 1.1, height: 0.46, x: 0.08, y: 1.02, phase: 0.7, sleepFrame: 78 },
    { handle: 5, width: 0.92, height: 0.44, x: -0.05, y: 0.49, phase: 1.0, sleepFrame: 82 },
    { handle: 6, width: 0.74, height: 0.42, x: 0.09, y: -0.01, phase: 1.35, sleepFrame: 86 },
    { handle: 7, width: 0.58, height: 0.38, x: 0.0, y: -0.48, phase: 1.7, sleepFrame: 90 },
    { handle: 8, width: 0.46, height: 0.34, x: 0.03, y: -0.92, phase: 2.1, sleepFrame: 94 },
  ] as const

  return Array.from({ length: frameCount }, (_, frameIndex) => {
    const progress = frameIndex / Math.max(1, frameCount - 1)
    const settle = 1 - Math.min(1, progress * 1.35)
    const impact = Math.max(0, 1 - Math.abs(frameIndex - 24) / 18)
    const sideLoadX =
      frameIndex < 24
        ? -1.45 + progress * 3.8
        : -0.12 + Math.sin(progress * Math.PI * 3) * 0.05 * settle

    const dynamicBodies = configs.map((config, index) => {
      const sway = Math.sin(progress * Math.PI * 8 + config.phase) * 0.04 * settle
      const bounce = Math.cos(progress * Math.PI * 6 + config.phase) * 0.025 * settle
      const rotation = Math.sin(progress * Math.PI * 5 + config.phase) * 0.08 * settle
      const bodyEntry = body(
        config.handle,
        "dynamic",
        { x: config.x + sway, y: config.y + bounce },
        {
          x: (Math.cos(progress * Math.PI * 8 + config.phase) * 0.08 + impact * 0.03) * settle,
          y: Math.sin(progress * Math.PI * 5 + config.phase) * 0.03 * settle,
        },
      )
      bodyEntry.transform.rotation = rotation
      bodyEntry.angular_velocity =
        Math.cos(progress * Math.PI * 5 + config.phase) * 0.12 * settle
      bodyEntry.sleeping = frameIndex >= config.sleepFrame
      bodyEntry.island_id = frameIndex < 24 ? 1 : frameIndex < 34 ? 2 : 1
      return bodyEntry
    })

    const sideBody = body(
      9,
      "dynamic",
      {
        x: sideLoadX,
        y: 1.18 + Math.cos(progress * Math.PI * 3.5) * 0.06 * settle,
      },
      {
        x: frameIndex < 24 ? 1.2 : -0.1 * settle,
        y: 0.03 * settle,
      },
    )
    sideBody.transform.rotation = -0.02 * settle
    sideBody.angular_velocity = 0.05 * settle
    sideBody.sleeping = frameIndex >= 70
    sideBody.island_id = frameIndex < 28 ? 2 : 1

    const bodies = [
      body(1, "static", { x: 0, y: 2.6 }, { x: 0, y: 0 }),
      body(2, "static", { x: 0.15, y: 2.05 }, { x: 0, y: 0 }),
      ...dynamicBodies,
      sideBody,
    ]

    const snapshot = baseSnapshot(frameIndex, frameIndex / 60, bodies)
    snapshot.colliders = [
      collider(1, 1, { min: { x: -4.5, y: 2.3 }, max: { x: 4.5, y: 2.9 } }),
      collider(2, 2, { min: { x: -0.7, y: 1.925 }, max: { x: 1.0, y: 2.175 } }),
      ...configs.map((config) => {
        const bodyEntry = bodies.find((bodyEntry) => bodyEntry.handle === config.handle)!
        return collider(
          config.handle,
          config.handle,
          boxAabb(bodyEntry.transform.translation, config.width, config.height),
        )
      }),
      collider(9, 9, boxAabb(sideBody.transform.translation, 0.36, 0.36)),
    ]

    const contactDepth = 0.018 + impact * 0.01
    const contacts = [
      makeStackContact(1, 3, 2, 3, 2, { x: 0.02, y: 1.82 }, 1.9 - settle * 0.6, 0.34),
      makeStackContact(2, 4, 3, 4, 3, { x: 0.06, y: 1.28 }, 1.4 - settle * 0.4, 0.22),
      makeStackContact(3, 5, 4, 5, 4, { x: -0.01, y: 0.75 }, 1.1 - settle * 0.35, 0.17),
      makeStackContact(4, 6, 5, 6, 5, { x: 0.04, y: 0.24 }, 0.84 - settle * 0.22, 0.12),
      makeStackContact(5, 7, 6, 7, 6, { x: 0.02, y: -0.23 }, 0.58 - settle * 0.15, 0.09),
      makeStackContact(6, 8, 7, 8, 7, { x: 0.03, y: -0.66 }, 0.36 - settle * 0.08, 0.05),
    ]

    if (frameIndex >= 18 && frameIndex <= 42) {
      contacts.push(
        makeStackContact(
          7,
          9,
          4,
          9,
          4,
          { x: sideBody.transform.translation.x + 0.18, y: 1.14 },
          1.7 * impact + 0.2,
          0.52 * impact,
        ),
      )
    }

    snapshot.contacts = contacts.map((contact, index) => ({
      ...contact,
      depth: contactDepth + index * 0.002,
      solver_tangent_impulse:
        index === contacts.length - 1 && frameIndex % 11 === 0
          ? undefined
          : contact.solver_tangent_impulse,
    }))
    snapshot.manifolds = snapshot.contacts.map((contact) => ({
      id: contact.id,
      bodies: contact.bodies,
      colliders: contact.colliders,
      contact_ids: [contact.id],
      points: [
        {
          contact_id: contact.id,
          feature_id: contact.feature_id,
          point: contact.point,
          depth: contact.depth,
        },
      ],
      normal: contact.normal,
      depth: contact.depth,
      reduction_reason: contact.reduction_reason,
      warm_start_hit_count: frameIndex > 12 ? 1 : 0,
      warm_start_miss_count: frameIndex > 12 ? 0 : 1,
      warm_start_drop_count: 0,
      active: true,
    }))
    snapshot.islands = [
      {
        id: 1,
        bodies: [3, 4, 5, 6, 7, 8, ...(frameIndex >= 28 ? [9] : [])],
        sleeping: frameIndex >= 84,
        reason: frameIndex >= 84 ? "stability_window" : "impact",
      },
      ...(frameIndex < 28
        ? [
            {
              id: 2,
              bodies: [9],
              sleeping: false,
              reason: "impact" as const,
            },
          ]
        : []),
    ]
    snapshot.stats.contact_count = snapshot.contacts.length
    snapshot.stats.manifold_count = snapshot.manifolds.length
    snapshot.stats.island_count = snapshot.islands.length
    snapshot.stats.active_island_count = snapshot.islands.filter(
      (island) => !island.sleeping,
    ).length
    snapshot.stats.sleeping_island_skip_count = snapshot.islands.filter(
      (island) => island.sleeping,
    ).length
    snapshot.stats.solver_body_slot_count = 7
    snapshot.stats.contact_row_count = snapshot.contacts.length
    snapshot.stats.joint_row_count = 0
    snapshot.stats.broadphase_candidate_count = 8 + Math.round(impact * 3)
    snapshot.stats.broadphase_traversal_count = 12 + Math.round(impact * 4)
    snapshot.stats.broadphase_pruned_count = 6
    snapshot.stats.broadphase_tree_depth = 4
    snapshot.stats.warm_start_hit_count = frameIndex > 12 ? snapshot.contacts.length - 1 : 0
    snapshot.stats.warm_start_miss_count = frameIndex > 12 ? 1 : snapshot.contacts.length

    return frame(frameIndex, snapshot)
  })
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

function makeLatticeGridFrames(frameCount: number): FrameRecord[] {
  const cols = 4
  const rows = 3
  const spacingX = 0.62
  const spacingY = 0.58
  const originX = -0.93
  const originY = -1.22
  const anchorPoints = Array.from({ length: cols }, (_, col) => ({
    x: originX + col * spacingX,
    y: originY + (col % 2 === 0 ? -0.18 : -0.22),
  }))
  const diagonalRestLength = Math.hypot(spacingX, spacingY)

  return Array.from({ length: frameCount }, (_, frameIndex) => {
    const progress = frameIndex / Math.max(1, frameCount - 1)
    const settle = 1 - Math.min(1, progress * 1.05)
    const bodies = [] as DebugSnapshot["bodies"]
    const colliders = [] as DebugSnapshot["colliders"]
    const joints = [] as DebugSnapshot["joints"]
    const handleFor = (row: number, col: number) => row * cols + col + 1
    const positionFor = (row: number, col: number): Vec2 => {
      const sway = Math.sin(progress * Math.PI * 3 + row * 0.5 + col * 0.35) * 0.035 * settle
      const bounce = Math.cos(progress * Math.PI * 2.5 + row * 0.65 + col * 0.2) * 0.024 * settle
      const sag = row * row * 0.028 * (1 - settle)
      return {
        x: originX + col * spacingX + sway + (row % 2 === 1 ? 0.012 * (1 - settle) : 0),
        y: originY + row * spacingY + bounce + sag,
      }
    }

    for (let row = 0; row < rows; row += 1) {
      for (let col = 0; col < cols; col += 1) {
        const handle = handleFor(row, col)
        const translation = positionFor(row, col)
        const velocity = {
          x: Math.cos(progress * Math.PI * 3 + row + col * 0.4) * 0.12 * settle,
          y: Math.sin(progress * Math.PI * 2.5 + row * 0.7 + col * 0.3) * 0.08 * settle + row * 0.02,
        }
        const entry = body(handle, "dynamic", translation, velocity)
        entry.sleeping = frameIndex >= Math.floor(frameCount * 0.82) && row > 0
        entry.island_id = 1
        bodies.push(entry)
        colliders.push(circleCollider(handle, handle, translation, 0.12))
      }
    }

    let jointHandle = 1
    for (let col = 0; col < cols; col += 1) {
      joints.push({
        handle: jointHandle++,
        kind: "world_anchor",
        bodies: [handleFor(0, col)],
        anchors: [positionFor(0, col), anchorPoints[col]],
      })
    }
    for (let row = 0; row < rows; row += 1) {
      for (let col = 0; col < cols; col += 1) {
        if (col + 1 < cols) {
          joints.push({
            handle: jointHandle++,
            kind: "distance",
            bodies: [handleFor(row, col), handleFor(row, col + 1)],
            anchors: [positionFor(row, col), positionFor(row, col + 1)],
          })
        }
        if (row + 1 < rows) {
          joints.push({
            handle: jointHandle++,
            kind: "distance",
            bodies: [handleFor(row, col), handleFor(row + 1, col)],
            anchors: [positionFor(row, col), positionFor(row + 1, col)],
          })
        }
        if (row + 1 < rows && col + 1 < cols) {
          const forward = (row + col) % 2 === 0
          const start = forward ? positionFor(row, col) : positionFor(row, col + 1)
          const end = forward ? positionFor(row + 1, col + 1) : positionFor(row + 1, col)
          joints.push({
            handle: jointHandle++,
            kind: "distance",
            bodies: forward
              ? [handleFor(row, col), handleFor(row + 1, col + 1)]
              : [handleFor(row, col + 1), handleFor(row + 1, col)],
            anchors: [start, end],
          })
        }
      }
    }

    const snapshot = baseSnapshot(frameIndex, frameIndex / 60, bodies)
    snapshot.colliders = colliders
    snapshot.joints = joints
    snapshot.islands = [{ id: 1, bodies: bodies.map((bodyEntry) => bodyEntry.handle), sleeping: settle < 0.08 }]
    snapshot.stats.active_collider_count = colliders.length
    snapshot.stats.active_joint_count = joints.length
    snapshot.stats.contact_count = 0
    snapshot.stats.manifold_count = 0
    snapshot.stats.island_count = 1
    snapshot.stats.active_island_count = settle < 0.08 ? 0 : 1
    snapshot.stats.sleeping_island_skip_count = settle < 0.08 ? 1 : 0
    snapshot.stats.solver_body_slot_count = bodies.length
    snapshot.stats.contact_row_count = 0
    snapshot.stats.joint_row_count = joints.length
    snapshot.stats.broadphase_candidate_count = colliders.length + joints.length
    snapshot.primitives = [
      {
        kind: "label",
        position: { x: originX - 0.18, y: originY - 0.45 },
        text: `lattice ${cols}x${rows} proxy`,
        color: { r: 216, g: 173, b: 91, a: 255 },
      },
      {
        kind: "polyline",
        points: anchorPoints,
        closed: false,
        color: { r: 84, g: 98, b: 118, a: 180 },
      },
    ]

    const record = frame(frameIndex, snapshot)
    record.stats = snapshot.stats
    void diagonalRestLength
    return record
  })
}

function makeCompoundProvenanceFrames(frameCount: number): FrameRecord[] {
  return Array.from({ length: frameCount }, (_, frameIndex) => {
    const t = frameIndex / Math.max(1, frameCount - 1);
    const compoundCenter = { x: 0.58, y: 0.46 };
    const snapshot = baseSnapshot(frameIndex, frameIndex / 60, [
      body(1, "static", { x: 0, y: 2.2 }, { x: 0, y: 0 }),
      body(2, "dynamic", compoundCenter, { x: 0, y: 0 }),
    ]);

    snapshot.meta.gravity = { x: 0, y: 0 };
    snapshot.colliders = [
      collider(1, 1, { min: { x: -4, y: 2.0 }, max: { x: 4, y: 2.4 } }),
      {
        ...collider(2, 2, boxAabb({ x: 0.58, y: 0.46 }, 1.4, 0.4)),
        density: 1.75,
      },
      {
        ...circleCollider(3, 2, { x: 1.42, y: 0.25 }, 0.25),
        density: 1.75,
      },
      {
        ...collider(4, 2, { min: { x: -0.69, y: 0.26 }, max: { x: 0.13, y: 0.98 } }),
        density: 1.75,
      },
    ];
    snapshot.islands = [
      {
        id: 1,
        bodies: [2],
        sleeping: t > 0.5,
        reason: t > 0.5 ? "stability_window" : "impact",
      },
    ];
    snapshot.broadphase_tree = {
      root: 1,
      depth: 3,
      nodes: [
        { id: 1, depth: 1, parent: null, left: 2, right: 5, collider: null, aabb: { min: { x: -4, y: 0.0 }, max: { x: 4, y: 2.4 } } },
        { id: 2, depth: 2, parent: 1, left: 3, right: 4, collider: null, aabb: { min: { x: -0.69, y: 0.0 }, max: { x: 1.67, y: 0.98 } } },
        { id: 3, depth: 3, parent: 2, left: null, right: null, collider: 2, aabb: { min: { x: -0.12, y: 0.26 }, max: { x: 1.28, y: 0.66 } } },
        { id: 4, depth: 3, parent: 2, left: null, right: null, collider: 4, aabb: { min: { x: -0.69, y: 0.26 }, max: { x: 0.13, y: 0.98 } } },
        { id: 5, depth: 2, parent: 1, left: 6, right: 7, collider: null, aabb: { min: { x: -4, y: 0.0 }, max: { x: 4, y: 2.4 } } },
        { id: 6, depth: 3, parent: 5, left: null, right: null, collider: 1, aabb: { min: { x: -4, y: 2.0 }, max: { x: 4, y: 2.4 } } },
        { id: 7, depth: 3, parent: 5, left: null, right: null, collider: 3, aabb: { min: { x: 1.17, y: 0.0 }, max: { x: 1.67, y: 0.5 } } },
      ],
    };
    snapshot.stats.broadphase_candidate_count = 1;
    snapshot.stats.broadphase_update_count = frameIndex === 0 ? 4 : 0;
    snapshot.stats.broadphase_traversal_count = 3;
    snapshot.stats.broadphase_pruned_count = 2;
    snapshot.stats.broadphase_rebuild_count = frameIndex === 0 ? 1 : 0;
    snapshot.stats.broadphase_tree_depth = 3;
    snapshot.stats.island_count = 1;
    snapshot.stats.active_island_count = t > 0.5 ? 0 : 1;
    snapshot.stats.sleeping_island_skip_count = t > 0.5 ? 1 : 0;
    snapshot.stats.solver_body_slot_count = 1;
    snapshot.stats.contact_row_count = 0;
    snapshot.stats.joint_row_count = 0;

    return {
      ...frame(frameIndex, snapshot),
      compound_provenance: [
        {
          authored_body_index: 1,
          body_handle: 2,
          validation_path: "scene.bodies[1].shape.pieces",
          inherited_material: "sticky",
          inherited_filter: "dynamic_body",
          inherited_density: 1.75,
          inherited_is_sensor: false,
          pieces: [
            {
              generated_piece_index: 0,
              collider_handle: 2,
              validation_path: "scene.bodies[1].shape.pieces[0]",
              local_pose: [0, 0, 0],
            },
            {
              generated_piece_index: 1,
              collider_handle: 3,
              validation_path: "scene.bodies[1].shape.pieces[1]",
              local_pose: [0.9, -0.2, 0],
            },
            {
              generated_piece_index: 2,
              collider_handle: 4,
              validation_path: "scene.bodies[1].shape.pieces[2]",
              local_pose: [-0.85, 0.3, -0.35],
            },
          ],
        },
      ],
    };
  });
}

function makeCcdFrames(frameCount: number): FrameRecord[] {
  const start = { x: -1, y: 0 };
  const sweptEnd = { x: 2.333, y: 0 };
  const toiPoint = { x: -0.05, y: 0 };
  const clampedCenter = { x: -0.1, y: 0 };
  const toi = (clampedCenter.x - start.x) / (sweptEnd.x - start.x);

  return Array.from({ length: frameCount }, (_, frameIndex) => {
    const t = frameIndex / Math.max(1, frameCount - 1);
    const hit = t >= toi;
    const center = hit
      ? clampedCenter
      : {
          x: start.x + (clampedCenter.x - start.x) * Math.min(1, t / toi),
          y: 0,
        };
    const snapshot = baseSnapshot(frameIndex, frameIndex / 60, [
      body(1, "static", { x: 0, y: 0 }, { x: 0, y: 0 }),
      body(2, "dynamic", center, hit ? { x: 0, y: 0 } : { x: 200, y: 0 }),
    ]);

    snapshot.meta.gravity = { x: 0, y: 0 };
    snapshot.colliders = [
      collider(1, 1, { min: { x: -0.05, y: -5 }, max: { x: 0.05, y: 5 } }),
      circleCollider(2, 2, center, 0.05),
    ];
    snapshot.stats.ccd_candidate_count = 1;
    snapshot.stats.ccd_hit_count = hit ? 1 : 0;
    snapshot.stats.ccd_miss_count = hit ? 0 : 1;
    snapshot.stats.ccd_clamp_count = hit ? 1 : 0;
    snapshot.primitives = [
      {
        kind: "line",
        start,
        end: sweptEnd,
        color: { r: 126, g: 176, b: 105, a: 210 },
      },
      {
        kind: "circle",
        center: toiPoint,
        radius: 0.08,
        color: { r: 240, g: 195, b: 107, a: 255 },
      },
      {
        kind: "label",
        position: toiPoint,
        text: "TOI",
        color: { r: 240, g: 195, b: 107, a: 255 },
      },
    ];

    if (hit) {
      snapshot.contacts = [
        {
          id: 1,
          bodies: [2, 1],
          colliders: [2, 1],
          feature_id: 1,
          point: toiPoint,
          normal: { x: -1, y: 0 },
          depth: 0.002,
          reduction_reason: "single_point",
          warm_start_reason: "miss_no_previous",
          normal_impulse: 0,
          tangent_impulse: 0,
          solver_normal_impulse: 0.6,
          solver_tangent_impulse: 0,
          normal_impulse_clamped: false,
          tangent_impulse_clamped: false,
          restitution_velocity_threshold: 1,
          restitution_applied: false,
          ccd_trace: {
            moving_body: 2,
            static_body: 1,
            moving_collider: 2,
            static_collider: 1,
            swept_start: start,
            swept_end: sweptEnd,
            toi,
            advancement: toi,
            clamp: sweptEnd.x - clampedCenter.x,
            slop: 0.002,
            toi_point: toiPoint,
          },
        },
      ];
      snapshot.manifolds = [
        {
          id: 1,
          bodies: [2, 1],
          colliders: [2, 1],
          contact_ids: [1],
          points: [{ contact_id: 1, feature_id: 1, point: toiPoint, depth: 0.002 }],
          normal: { x: -1, y: 0 },
          depth: 0.002,
          reduction_reason: "single_point",
          warm_start_hit_count: 0,
          warm_start_miss_count: 1,
          warm_start_drop_count: 0,
          active: true,
        },
      ];
      snapshot.stats.contact_count = 1;
      snapshot.stats.manifold_count = 1;
      snapshot.stats.warm_start_miss_count = 1;
    }

    return frame(frameIndex, snapshot);
  });
}

function makeCcdConvexFrames(frameCount: number): FrameRecord[] {
  const start = { x: -1, y: 0 };
  const sweptEnd = { x: 2.333, y: 0 };
  const toiPoint = { x: -0.05, y: 0 };
  const clampedCenter = { x: -0.099, y: 0 };
  const toi = (-0.1 - start.x) / (sweptEnd.x - start.x);

  return Array.from({ length: frameCount }, (_, frameIndex) => {
    const t = frameIndex / Math.max(1, frameCount - 1);
    const hit = t >= toi;
    const center = hit
      ? clampedCenter
      : {
          x: start.x + (clampedCenter.x - start.x) * Math.min(1, t / toi),
          y: 0,
        };
    const snapshot = baseSnapshot(frameIndex, frameIndex / 60, [
      body(1, "static", { x: 0, y: 0 }, { x: 0, y: 0 }),
      body(2, "static", { x: 0.8, y: 0 }, { x: 0, y: 0 }),
      body(3, "dynamic", center, hit ? { x: 0, y: 0 } : { x: 200, y: 0 }),
    ]);

    snapshot.meta.gravity = { x: 0, y: 0 };
    snapshot.colliders = [
      collider(1, 1, { min: { x: -0.05, y: -5 }, max: { x: 0.05, y: 5 } }),
      collider(2, 2, { min: { x: 0.75, y: -5 }, max: { x: 0.85, y: 5 } }),
      collider(3, 3, boxAabb(center, 0.1, 0.1)),
    ];
    snapshot.stats.ccd_candidate_count = 2;
    snapshot.stats.ccd_hit_count = hit ? 2 : 0;
    snapshot.stats.ccd_miss_count = hit ? 0 : 2;
    snapshot.stats.ccd_clamp_count = hit ? 1 : 0;
    snapshot.primitives = [
      {
        kind: "line",
        start,
        end: sweptEnd,
        color: { r: 126, g: 176, b: 105, a: 210 },
      },
      {
        kind: "circle",
        center: toiPoint,
        radius: 0.08,
        color: { r: 240, g: 195, b: 107, a: 255 },
      },
      {
        kind: "label",
        position: { x: 0.42, y: -0.38 },
        text: "budget skips later hit",
        color: { r: 183, g: 198, b: 211, a: 255 },
      },
    ];

    if (hit) {
      snapshot.contacts = [
        {
          id: 1,
          bodies: [3, 1],
          colliders: [3, 1],
          feature_id: 1,
          point: toiPoint,
          normal: { x: -1, y: 0 },
          depth: 0.002,
          reduction_reason: "clipped",
          warm_start_reason: "miss_no_previous",
          normal_impulse: 0,
          tangent_impulse: 0,
          solver_normal_impulse: 0.6,
          solver_tangent_impulse: 0,
          normal_impulse_clamped: false,
          tangent_impulse_clamped: false,
          restitution_velocity_threshold: 1,
          restitution_applied: false,
          ccd_trace: {
            moving_body: 3,
            static_body: 1,
            moving_collider: 3,
            static_collider: 1,
            swept_start: start,
            swept_end: sweptEnd,
            toi,
            advancement: toi,
            clamp: sweptEnd.x - clampedCenter.x,
            slop: 0.002,
            toi_point: toiPoint,
          },
        },
      ];
      snapshot.manifolds = [
        {
          id: 1,
          bodies: [3, 1],
          colliders: [3, 1],
          contact_ids: [1],
          points: [{ contact_id: 1, feature_id: 1, point: toiPoint, depth: 0.002 }],
          normal: { x: -1, y: 0 },
          depth: 0.002,
          reduction_reason: "clipped",
          warm_start_hit_count: 0,
          warm_start_miss_count: 1,
          warm_start_drop_count: 0,
          active: true,
        },
      ];
      snapshot.stats.contact_count = 1;
      snapshot.stats.manifold_count = 1;
      snapshot.stats.warm_start_miss_count = 1;
    }

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
    islands: [],
    broadphase_tree: { root: null, depth: 0, nodes: [] },
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
      ccd_candidate_count: 0,
      ccd_hit_count: 0,
      ccd_miss_count: 0,
      ccd_clamp_count: 0,
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

function circleCollider(handle: number, bodyHandle: number, center: Vec2, radius: number): DebugSnapshot["colliders"][number] {
  return {
    handle,
    body: bodyHandle,
    local_transform: { translation: { x: 0, y: 0 }, rotation: 0 },
    world_transform: { translation: center, rotation: 0 },
    aabb: {
      min: { x: center.x - radius, y: center.y - radius },
      max: { x: center.x + radius, y: center.y + radius },
    },
    shape: { kind: "circle", center, radius },
    density: 1,
    material: { friction: 0.5, restitution: 0.05 },
    filter: { memberships: 1, collides_with: 4294967295 },
    is_sensor: false,
    user_data: 0,
  };
}

function makeStackContact(
  id: number,
  topBody: number,
  bottomBody: number,
  topCollider: number,
  bottomCollider: number,
  point: Vec2,
  normalImpulse: number,
  tangentImpulse: number,
): DebugSnapshot["contacts"][number] {
  return {
    id,
    bodies: [topBody, bottomBody],
    colliders: [topCollider, bottomCollider],
    feature_id: id,
    point,
    normal: { x: 0, y: -1 },
    depth: 0.02,
    reduction_reason: "single_point",
    warm_start_reason: "hit",
    normal_impulse: normalImpulse * 0.32,
    tangent_impulse: tangentImpulse * 0.28,
    solver_normal_impulse: normalImpulse,
    solver_tangent_impulse: tangentImpulse,
    normal_impulse_clamped: false,
    tangent_impulse_clamped: tangentImpulse > 0.45,
    restitution_velocity_threshold: 1,
    restitution_applied: false,
  }
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
