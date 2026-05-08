import type {
  DebugBody,
  DebugCollider,
  FrameRecord,
  Vec2,
  VelocityPerturbationCommit,
  VelocityPerturbationPreview,
} from "../../types"

export type LayerState = {
  grid: boolean
  rulers: boolean
  shapes: boolean
  aabbs: boolean
  contacts: boolean
  velocities: boolean
  trace: boolean
  broadphaseTree: boolean
  islands: boolean
  sleep: boolean
  provenance: boolean
  stackStability: boolean
  lattice: boolean
}

export type TrajectoryMode =
  | "selectedBody"
  | "allDynamic"
  | "selectedIsland"
  | "contacts"
  | "ccd"

export type TrajectoryColorMode = "body" | "island"

export type TrajectorySettings = {
  mode: TrajectoryMode
  historyLength: number
  samplingStride: number
  fade: number
  colorMode: TrajectoryColorMode
}

export const defaultLayers: LayerState = {
  grid: true,
  rulers: true,
  shapes: true,
  aabbs: true,
  contacts: true,
  velocities: true,
  trace: true,
  broadphaseTree: false,
  islands: false,
  sleep: true,
  provenance: false,
  stackStability: false,
  lattice: true,
}

export const defaultTrajectorySettings: TrajectorySettings = {
  mode: "selectedBody",
  historyLength: 96,
  samplingStride: 2,
  fade: 0.72,
  colorMode: "body",
}

export type SourceKind = "demo" | "artifact" | "live"

export type RunMode = "artifact_replay" | "live_session"

export type ControlAction = "play" | "pause" | "step" | "reset"

export type LiveCadenceStatus = {
  targetFps: number
  actualFps: number | null
  lastStepMs: number | null
  nextDelayMs: number | null
  degraded: boolean
  pending: boolean
}

export type CanvasDebugView = {
  mode: "free" | "locked_core"
  zoom: number
  center: {
    x: number
    y: number
  }
  targetDescription: string | null
  targetBounds: {
    min: {
      x: number
      y: number
    }
    max: {
      x: number
      y: number
    }
  } | null
  dampingEnabled: boolean
  grid: boolean
  rulers: boolean
}

export type PerfEvidenceStatus =
  | "none"
  | "available"
  | "missing"
  | "live_unavailable"

export type ResolvedSelection =
  | { kind: "body"; entity: DebugBody }
  | { kind: "collider"; entity: DebugCollider; body?: DebugBody }
  | { kind: "contact"; entity: FrameRecord["snapshot"]["contacts"][number] }
  | { kind: "joint"; entity: FrameRecord["snapshot"]["joints"][number] }
  | null

export type VelocityPerturbationTarget = {
  bodyHandle: number
  bodyType: DebugBody["body_type"]
  currentVelocity: Vec2
}

export type VelocityPerturbationPanelState = {
  enabled: boolean
  target: VelocityPerturbationTarget | null
  deltaX: string
  deltaY: string
  busy: "idle" | "preview" | "submit"
  unavailableReason: string | null
  requestError: string | null
  sessionEpoch: number | null
  worldRevision: number | null
  frameIndex: number
  preview: VelocityPerturbationPreview | null
  commit: VelocityPerturbationCommit | null
}

export type LatticeProxyEdge = {
  joint: FrameRecord["snapshot"]["joints"][number]
  currentLength: number
  referenceLength: number | null
  stretchRatio: number | null
  anchorCount: number
  isWorldAnchor: boolean
}

export type LatticeProxyNode = {
  body: DebugBody
  connectedEdges: LatticeProxyEdge[]
  anchorEdgeCount: number
  connectedBodyIds: number[]
  islandId: number | null
  speed: number
}

export type LatticeProxySummary = {
  enabled: boolean
  nodes: LatticeProxyNode[]
  edges: LatticeProxyEdge[]
  worldAnchorEdgeCount: number
  maxStretchRatio: number | null
  jointRowCount: number | null
  contactCount: number
  islandCount: number | null
  activeIslandCount: number | null
  sleepingIslandCount: number | null
}

export function deriveLatticeProxy(
  frame: FrameRecord,
  referenceFrame: FrameRecord = frame,
): LatticeProxySummary {
  const dynamicBodies = frame.snapshot.bodies.filter((body) => body.body_type === "dynamic")
  const referenceJointsByHandle = new Map(
    referenceFrame.snapshot.joints.map((joint) => [joint.handle, joint]),
  )
  const edges = frame.snapshot.joints
    .filter((joint) => joint.anchors.length >= 2)
    .map((joint) => {
      const referenceJoint = referenceJointsByHandle.get(joint.handle) ?? null
      const currentLength = distance(joint.anchors[0], joint.anchors[1])
      const referenceLength =
        referenceJoint && referenceJoint.anchors.length >= 2
          ? distance(referenceJoint.anchors[0], referenceJoint.anchors[1])
          : null
      const stretchRatio =
        referenceLength != null && referenceLength > 1e-6
          ? currentLength / referenceLength
          : null
      return {
        joint,
        currentLength,
        referenceLength,
        stretchRatio,
        anchorCount: joint.anchors.length,
        isWorldAnchor: joint.kind === "world_anchor",
      }
    })
  const worldAnchorEdgeCount = edges.filter((edge) => edge.isWorldAnchor).length
  const distanceEdgeCount = edges.length - worldAnchorEdgeCount
  const enabled =
    dynamicBodies.length >= 6 && distanceEdgeCount >= 6 && worldAnchorEdgeCount >= 2
  if (!enabled) {
    return {
      enabled: false,
      nodes: [],
      edges: [],
      worldAnchorEdgeCount: 0,
      maxStretchRatio: null,
      jointRowCount: null,
      contactCount: frame.snapshot.contacts.length,
      islandCount: frame.snapshot.stats.island_count ?? null,
      activeIslandCount: frame.snapshot.stats.active_island_count ?? null,
      sleepingIslandCount:
        frame.snapshot.stats.sleeping_island_skip_count != null &&
        frame.snapshot.stats.island_count != null &&
        frame.snapshot.stats.active_island_count != null
          ? Math.max(
              0,
              frame.snapshot.stats.island_count -
                frame.snapshot.stats.active_island_count -
                frame.snapshot.stats.sleeping_island_skip_count,
            )
          : null,
    }
  }
  const edgesByBody = new Map<number, LatticeProxyEdge[]>()
  for (const edge of edges) {
    for (const bodyHandle of edge.joint.bodies) {
      const list = edgesByBody.get(bodyHandle)
      if (list) {
        list.push(edge)
      } else {
        edgesByBody.set(bodyHandle, [edge])
      }
    }
  }
  const nodes = dynamicBodies.map((body) => {
    const connectedEdges = edgesByBody.get(body.handle) ?? []
    const connectedBodyIds = connectedEdges
      .flatMap((edge) => edge.joint.bodies.filter((handle) => handle !== body.handle))
      .filter((value, index, values) => values.indexOf(value) === index)
    return {
      body,
      connectedEdges,
      anchorEdgeCount: connectedEdges.filter((edge) => edge.isWorldAnchor).length,
      connectedBodyIds,
      islandId: body.island_id ?? null,
      speed: vectorMagnitude(body.linear_velocity),
    }
  })
  const maxStretchRatio = edges.reduce<number | null>((currentMax, edge) => {
    if (edge.stretchRatio == null) {
      return currentMax
    }
    return currentMax == null ? edge.stretchRatio : Math.max(currentMax, edge.stretchRatio)
  }, null)

  return {
    enabled,
    nodes,
    edges,
    worldAnchorEdgeCount,
    maxStretchRatio,
    jointRowCount: frame.snapshot.stats.joint_row_count ?? null,
    contactCount: frame.snapshot.contacts.length,
    islandCount: frame.snapshot.stats.island_count ?? frame.snapshot.islands?.length ?? null,
    activeIslandCount: frame.snapshot.stats.active_island_count ?? null,
    sleepingIslandCount:
      frame.snapshot.stats.island_count != null &&
      frame.snapshot.stats.active_island_count != null
        ? Math.max(
            0,
            frame.snapshot.stats.island_count - frame.snapshot.stats.active_island_count,
          )
        : null,
  }
}

function distance(a: Vec2, b: Vec2): number {
  return Math.hypot(b.x - a.x, b.y - a.y)
}

function vectorMagnitude(value: Vec2): number {
  return Math.hypot(value.x, value.y)
}
