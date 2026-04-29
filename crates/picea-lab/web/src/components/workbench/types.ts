import type { DebugBody, DebugCollider, FrameRecord } from "../../types"

export type LayerState = {
  shapes: boolean
  aabbs: boolean
  contacts: boolean
  velocities: boolean
  trace: boolean
  broadphaseTree: boolean
  islands: boolean
  provenance: boolean
}

export const defaultLayers: LayerState = {
  shapes: true,
  aabbs: true,
  contacts: true,
  velocities: true,
  trace: true,
  broadphaseTree: false,
  islands: false,
  provenance: false,
}

export type SourceKind = "server" | "demo"

export type ControlAction = "play" | "pause" | "step" | "reset"

export type ResolvedSelection =
  | { kind: "body"; entity: DebugBody }
  | { kind: "collider"; entity: DebugCollider; body?: DebugBody }
  | { kind: "contact"; entity: FrameRecord["snapshot"]["contacts"][number] }
  | { kind: "joint"; entity: FrameRecord["snapshot"]["joints"][number] }
  | null
