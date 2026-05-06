import type {
  DebugBody,
  DebugCollider,
  DebugContact,
  FrameRecord,
  SelectedEntity,
  Vec2,
} from "../../types"
import type {
  TrajectoryColorMode,
  TrajectoryMode,
  TrajectorySettings,
} from "./types"

const HISTORY_CAP = 240
const STRIDE_CAP = 12
const MAX_BODY_TRAILS = 18
const MAX_CONTACT_TRAILS = 24
const MAX_CCD_TRAILS = 12
const PALETTE = [
  "#7fb069",
  "#56b6c2",
  "#f0c36b",
  "#d06464",
  "#8f86ff",
  "#ff8b5e",
  "#65d46e",
  "#6dc8ff",
]

export type TrajectoryEmptyState =
  | "selectedBody"
  | "allDynamic"
  | "selectedIsland"
  | "contacts"
  | "ccd"
  | "unsupportedSelection"

export type TrajectoryTrail = {
  key: string
  color: string
  points: Vec2[]
  latestPoint: Vec2
  emphasis: boolean
}

export type CcdTrajectoryTrail = {
  key: string
  color: string
  frameIndex: number
  sweptStart: Vec2
  sweptEnd: Vec2
  toiPoint: Vec2
  targetKind: "static" | "dynamic"
  targetSweptStart?: Vec2
  targetSweptEnd?: Vec2
  clamp: number
  targetClamp?: number
}

export type TrajectoryOverlay = {
  mode: TrajectoryMode
  windowStart: number
  windowEnd: number
  emptyState: TrajectoryEmptyState | null
  bodyTrails: TrajectoryTrail[]
  contactTrails: TrajectoryTrail[]
  ccdTrails: CcdTrajectoryTrail[]
}

export type TrajectoryMarkerKind =
  | "bodyMotion"
  | "contactAppeared"
  | "contactDisappeared"
  | "contactBurst"
  | "ccdClamp"
  | "ccdHit"

export type TrajectoryMarker = {
  kind: TrajectoryMarkerKind
  frameIndex: number
  score: number
}

export type TrajectorySummary = {
  mode: TrajectoryMode
  windowStart: number
  windowEnd: number
  distance: number | null
  maxSpeed: number | null
  awakeFrames: number | null
  sleepingFrames: number | null
  contactCount: number
  ccdEvidenceCount: number
}

type ContactHistory = {
  key: string
  color: string
  points: Vec2[]
  latestPoint: Vec2
  lastFrameIndex: number
}

type ContactLineage = {
  colliderPairKey: string
  featureId: number
  normalBucket: string
}

// These helpers intentionally stay on the Web side. They aggregate exported
// frames for visualization, but never pretend to be authoritative physics state.
export function buildTrajectoryOverlay(
  frames: FrameRecord[],
  frameIndex: number,
  selected: SelectedEntity | null,
  settings: TrajectorySettings,
): TrajectoryOverlay {
  const windowFrames = trajectoryWindow(frames, frameIndex, settings)
  const current = windowFrames[windowFrames.length - 1]
  if (!current) {
    return {
      mode: settings.mode,
      windowStart: frameIndex,
      windowEnd: frameIndex,
      emptyState: emptyStateForMode(settings.mode),
      bodyTrails: [],
      contactTrails: [],
      ccdTrails: [],
    }
  }

  if (settings.mode === "contacts") {
    const contactTrails = buildContactTrails(windowFrames, settings)
    return {
      mode: settings.mode,
      windowStart: windowFrames[0]?.frame_index ?? current.frame_index,
      windowEnd: current.frame_index,
      emptyState: contactTrails.length === 0 ? "contacts" : null,
      bodyTrails: [],
      contactTrails,
      ccdTrails: [],
    }
  }

  if (settings.mode === "ccd") {
    const ccdTrails = buildCcdTrails(windowFrames, settings)
    return {
      mode: settings.mode,
      windowStart: windowFrames[0]?.frame_index ?? current.frame_index,
      windowEnd: current.frame_index,
      emptyState: ccdTrails.length === 0 ? "ccd" : null,
      bodyTrails: [],
      contactTrails: [],
      ccdTrails,
    }
  }

  const currentBodies = current.snapshot.bodies
  const bodyHandles =
    settings.mode === "selectedBody"
      ? selectedBodyHandles(current, selected)
      : settings.mode === "allDynamic"
        ? currentBodies
            .filter((body) => body.body_type === "dynamic")
            .map((body) => body.handle)
        : selectedIslandBodyHandles(current, selected)

  if (bodyHandles.length === 0) {
    return {
      mode: settings.mode,
      windowStart: windowFrames[0]?.frame_index ?? current.frame_index,
      windowEnd: current.frame_index,
      emptyState:
        settings.mode === "selectedBody"
          ? "selectedBody"
          : settings.mode === "allDynamic"
            ? "allDynamic"
          : settings.mode === "selectedIsland"
            ? "selectedIsland"
            : "unsupportedSelection",
      bodyTrails: [],
      contactTrails: [],
      ccdTrails: [],
    }
  }

  return {
    mode: settings.mode,
    windowStart: windowFrames[0]?.frame_index ?? current.frame_index,
    windowEnd: current.frame_index,
    emptyState: null,
    bodyTrails: buildBodyTrails(windowFrames, current, bodyHandles, selected, settings),
    contactTrails: [],
    ccdTrails: [],
  }
}

export function buildTrajectoryMarkers(frames: FrameRecord[]): TrajectoryMarker[] {
  if (frames.length === 0) {
    return []
  }

  const motionMarkers: TrajectoryMarker[] = []
  const contactAppeared: TrajectoryMarker[] = []
  const contactDisappeared: TrajectoryMarker[] = []
  const contactBurst: TrajectoryMarker[] = []
  const ccdClamp: TrajectoryMarker[] = []
  const ccdHit: TrajectoryMarker[] = []

  for (let index = 1; index < frames.length; index += 1) {
    const previous = frames[index - 1]
    const current = frames[index]

    const maxMotion = bodyMotionScore(previous, current)
    if (maxMotion > 0.18) {
      motionMarkers.push({
        kind: "bodyMotion",
        frameIndex: current.frame_index,
        score: maxMotion,
      })
    }

    const previousContacts = new Set(previous.snapshot.contacts.map(contactLineageKey))
    const currentContacts = new Set(current.snapshot.contacts.map(contactLineageKey))
    const appeared = [...currentContacts].filter((key) => !previousContacts.has(key)).length
    const disappeared = [...previousContacts].filter((key) => !currentContacts.has(key)).length
    const burst = Math.max(
      0,
      current.snapshot.contacts.length - previous.snapshot.contacts.length,
    )

    if (appeared > 0) {
      contactAppeared.push({
        kind: "contactAppeared",
        frameIndex: current.frame_index,
        score: appeared,
      })
    }
    if (disappeared > 0) {
      contactDisappeared.push({
        kind: "contactDisappeared",
        frameIndex: current.frame_index,
        score: disappeared,
      })
    }
    if (burst > 1) {
      contactBurst.push({
        kind: "contactBurst",
        frameIndex: current.frame_index,
        score: burst,
      })
    }

    const ccdEvents = dedupeCcdContacts(current.snapshot.contacts)
    if (ccdEvents.length > 0) {
      ccdHit.push({
        kind: "ccdHit",
        frameIndex: current.frame_index,
        score: ccdEvents.length,
      })
    }
    const clampScore = ccdEvents.reduce(
      (best, contact) =>
        Math.max(best, contact.ccd_trace?.clamp ?? 0, contact.ccd_trace?.target_clamp ?? 0),
      0,
    )
    if (clampScore > 0) {
      ccdClamp.push({
        kind: "ccdClamp",
        frameIndex: current.frame_index,
        score: clampScore,
      })
    }
  }

  return [
    ...selectTopMarkers(motionMarkers, 3),
    ...selectTopMarkers(contactAppeared, 2),
    ...selectTopMarkers(contactDisappeared, 2),
    ...selectTopMarkers(contactBurst, 2),
    ...selectTopMarkers(ccdClamp, 2),
    ...selectTopMarkers(ccdHit, 2),
  ].sort((left, right) => left.frameIndex - right.frameIndex)
}

export function buildTrajectorySummary(
  frames: FrameRecord[],
  frameIndex: number,
  selected: SelectedEntity | null,
  settings: TrajectorySettings,
): TrajectorySummary | null {
  if (!selected || frames.length === 0) {
    return null
  }

  const windowFrames = trajectoryWindow(frames, frameIndex, settings)
  const current = windowFrames[windowFrames.length - 1]
  if (!current) {
    return null
  }

  if (selected.kind === "contact") {
    const currentExactMatch = current.snapshot.contacts.find(
      (contact) => contact.id === selected.id,
    )
    const lineage = findSelectedContactLineage(windowFrames, selected.id)
    if (!currentExactMatch && !lineage) {
      return null
    }
    const entries = windowFrames.flatMap((frame) =>
      frame.snapshot.contacts
        .filter((contact) =>
          currentExactMatch && frame.frame_index === current.frame_index
            ? contact.id === selected.id
            : lineage
              ? matchesContactLineage(contact, lineage)
              : false,
        )
        .map((contact) => ({ frame, contact })),
    )
    if (entries.length === 0) {
      return null
    }
    return {
      mode: settings.mode,
      windowStart: windowFrames[0]?.frame_index ?? current.frame_index,
      windowEnd: current.frame_index,
      distance: pathDistance(entries.map((entry) => entry.contact.point)),
      maxSpeed: null,
      awakeFrames: null,
      sleepingFrames: null,
      contactCount: entries.length,
      ccdEvidenceCount: entries.filter((entry) => entry.contact.ccd_trace).length,
    }
  }

  const bodyHandle =
    selected.kind === "body"
      ? selected.id
      : selected.kind === "collider"
        ? current.snapshot.colliders.find((collider) => collider.handle === selected.id)?.body ?? null
        : null
  if (bodyHandle == null) {
    return null
  }

  const bodySamples = windowFrames
    .map((frame) => frame.snapshot.bodies.find((body) => body.handle === bodyHandle) ?? null)
    .filter((body): body is DebugBody => Boolean(body))
  if (bodySamples.length === 0) {
    return null
  }

  const ownedColliders = new Set(
    current.snapshot.colliders
      .filter((collider) => collider.body === bodyHandle)
      .map((collider) => collider.handle),
  )

  return {
    mode: settings.mode,
    windowStart: windowFrames[0]?.frame_index ?? current.frame_index,
    windowEnd: current.frame_index,
    distance: pathDistance(bodySamples.map((body) => body.transform.translation)),
    maxSpeed: maxLinearSpeed(bodySamples),
    awakeFrames: bodySamples.filter((body) => !body.sleeping).length,
    sleepingFrames: bodySamples.filter((body) => body.sleeping).length,
    contactCount: windowFrames.reduce(
      (count, frame) =>
        count +
        frame.snapshot.contacts.filter((contact) =>
          contact.bodies.includes(bodyHandle) ||
          contact.colliders.some((handle) => ownedColliders.has(handle)),
        ).length,
      0,
    ),
    ccdEvidenceCount: windowFrames.reduce(
      (count, frame) =>
        count +
        frame.snapshot.contacts.filter((contact) =>
          Boolean(contact.ccd_trace) &&
          (contact.bodies.includes(bodyHandle) ||
            contact.colliders.some((handle) => ownedColliders.has(handle))),
        ).length,
      0,
    ),
  }
}

function trajectoryWindow(
  frames: FrameRecord[],
  frameIndex: number,
  settings: TrajectorySettings,
): FrameRecord[] {
  if (frames.length === 0) {
    return []
  }
  const clampedIndex = Math.min(frameIndex, frames.length - 1)
  const historyLength = Math.max(12, Math.min(HISTORY_CAP, settings.historyLength))
  const stride = Math.max(1, Math.min(STRIDE_CAP, settings.samplingStride))
  const start = Math.max(0, clampedIndex - historyLength + 1)
  const sampled: FrameRecord[] = []
  for (let index = start; index <= clampedIndex; index += stride) {
    sampled.push(frames[index])
  }
  const current = frames[clampedIndex]
  if (sampled[sampled.length - 1] !== current) {
    sampled.push(current)
  }
  return sampled
}

function buildBodyTrails(
  windowFrames: FrameRecord[],
  current: FrameRecord,
  bodyHandles: number[],
  selected: SelectedEntity | null,
  settings: TrajectorySettings,
): TrajectoryTrail[] {
  const dynamicOrder = bodyHandles
    .map((handle) => current.snapshot.bodies.find((body) => body.handle === handle))
    .filter((body): body is DebugBody => Boolean(body))
    .sort((left, right) => left.handle - right.handle)
    .slice(0, MAX_BODY_TRAILS)

  return dynamicOrder
    .map((body) => {
      const points = windowFrames
        .map((frame) =>
          frame.snapshot.bodies.find((entry) => entry.handle === body.handle)?.transform.translation,
        )
        .filter((point): point is Vec2 => Boolean(point))
      const latestPoint = points[points.length - 1]
      if (!latestPoint || points.length < 2) {
        return null
      }
      return {
        key: `body-${body.handle}`,
        color: pickTrajectoryColor(body.handle, body.island_id ?? null, settings.colorMode),
        points,
        latestPoint,
        emphasis:
          selected?.kind === "body"
            ? selected.id === body.handle
            : selected?.kind === "collider"
              ? current.snapshot.colliders.find((collider) => collider.handle === selected.id)?.body ===
                body.handle
              : false,
      }
    })
    .filter((trail): trail is TrajectoryTrail => Boolean(trail))
}

function buildContactTrails(
  windowFrames: FrameRecord[],
  settings: TrajectorySettings,
): TrajectoryTrail[] {
  const histories = new Map<string, ContactHistory>()
  for (const frame of windowFrames) {
    for (const contact of frame.snapshot.contacts) {
      const key = contactLineageKey(contact)
      const islandHint = deriveContactIslandId(frame, contact)
      const color = pickTrajectoryColor(
        contact.colliders[0] * 101 + contact.feature_id,
        islandHint,
        settings.colorMode,
      )
      const existing = histories.get(key)
      if (existing) {
        existing.points.push(contact.point)
        existing.latestPoint = contact.point
        existing.lastFrameIndex = frame.frame_index
      } else {
        histories.set(key, {
          key,
          color,
          points: [contact.point],
          latestPoint: contact.point,
          lastFrameIndex: frame.frame_index,
        })
      }
    }
  }

  return [...histories.values()]
    .filter((history) => history.points.length >= 2)
    .sort((left, right) => right.points.length - left.points.length)
    .slice(0, MAX_CONTACT_TRAILS)
    .map((history) => ({
      key: history.key,
      color: history.color,
      points: history.points,
      latestPoint: history.latestPoint,
      emphasis: false,
    }))
}

function buildCcdTrails(
  windowFrames: FrameRecord[],
  settings: TrajectorySettings,
): CcdTrajectoryTrail[] {
  const trails = windowFrames.flatMap((frame) =>
    dedupeCcdContacts(frame.snapshot.contacts).map((contact) => {
      const ccd = contact.ccd_trace!
      return {
        key: [
          frame.frame_index,
          ccd.moving_body,
          ccd.moving_collider,
          ccd.static_body,
          ccd.static_collider,
          contact.feature_id,
        ].join(":"),
        color: pickTrajectoryColor(
          ccd.moving_body,
          deriveContactIslandId(frame, contact),
          settings.colorMode,
        ),
        frameIndex: frame.frame_index,
        sweptStart: ccd.swept_start,
        sweptEnd: ccd.swept_end,
        toiPoint: ccd.toi_point,
        targetKind: ccd.target_kind ?? "static",
        targetSweptStart: ccd.target_swept_start,
        targetSweptEnd: ccd.target_swept_end,
        clamp: ccd.clamp,
        targetClamp: ccd.target_clamp,
      }
    }),
  )

  return trails
    .sort(
      (left, right) =>
        Math.max(right.clamp, right.targetClamp ?? 0) -
        Math.max(left.clamp, left.targetClamp ?? 0),
    )
    .slice(0, MAX_CCD_TRAILS)
}

function selectedBodyHandles(frame: FrameRecord, selected: SelectedEntity | null): number[] {
  if (!selected) {
    return []
  }
  if (selected.kind === "body") {
    return frame.snapshot.bodies.some((body) => body.handle === selected.id)
      ? [selected.id]
      : []
  }
  if (selected.kind === "collider") {
    const owner = frame.snapshot.colliders.find((collider) => collider.handle === selected.id)?.body
    return owner == null ? [] : [owner]
  }
  return []
}

function selectedIslandBodyHandles(frame: FrameRecord, selected: SelectedEntity | null): number[] {
  const islandId = selectedIslandId(frame, selected)
  if (islandId == null) {
    return []
  }
  return frame.snapshot.bodies
    .filter((body) => body.body_type === "dynamic" && body.island_id === islandId)
    .map((body) => body.handle)
}

function selectedIslandId(frame: FrameRecord, selected: SelectedEntity | null): number | null {
  if (!selected) {
    return null
  }
  if (selected.kind === "body") {
    return frame.snapshot.bodies.find((body) => body.handle === selected.id)?.island_id ?? null
  }
  if (selected.kind === "collider") {
    const owner = frame.snapshot.colliders.find((collider) => collider.handle === selected.id)?.body
    return owner == null
      ? null
      : frame.snapshot.bodies.find((body) => body.handle === owner)?.island_id ?? null
  }
  if (selected.kind === "contact") {
    const contact = frame.snapshot.contacts.find((entry) => entry.id === selected.id)
    return contact ? deriveContactIslandId(frame, contact) : null
  }
  const joint = frame.snapshot.joints.find((entry) => entry.handle === selected.id)
  if (!joint) {
    return null
  }
  const islandIds = [
    ...new Set(
      joint.bodies
        .map((bodyHandle) => frame.snapshot.bodies.find((body) => body.handle === bodyHandle)?.island_id)
        .filter((value): value is number => value != null),
    ),
  ]
  return islandIds.length === 1 ? islandIds[0] : null
}

function deriveContactIslandId(frame: FrameRecord, contact: DebugContact): number | null {
  const islandIds = [
    ...new Set(
      contact.bodies
        .map((bodyHandle) => frame.snapshot.bodies.find((body) => body.handle === bodyHandle)?.island_id)
        .filter((value): value is number => value != null),
    ),
  ]
  return islandIds.length === 1 ? islandIds[0] : null
}

function contactLineageKey(contact: DebugContact): string {
  const lineage = exportedContactLineage(contact)
  return `colliders:${lineage.colliderPairKey}:feature:${lineage.featureId}:normal:${lineage.normalBucket}`
}

function findSelectedContactLineage(
  windowFrames: FrameRecord[],
  contactId: number,
): ContactLineage | null {
  // Selection only keeps the transient contact id. We recover a more stable
  // lineage from the nearest exported frame that still carries that id, then
  // reuse that lineage across the trail window so summaries do not blink out
  // just because the solver rotated ids between adjacent frames.
  for (let index = windowFrames.length - 1; index >= 0; index -= 1) {
    const contact = windowFrames[index].snapshot.contacts.find((entry) => entry.id === contactId)
    if (contact) {
      return exportedContactLineage(contact)
    }
  }
  return null
}

function matchesContactLineage(contact: DebugContact, lineage: ContactLineage): boolean {
  const candidate = exportedContactLineage(contact)
  return (
    candidate.colliderPairKey === lineage.colliderPairKey &&
    candidate.featureId === lineage.featureId &&
    candidate.normalBucket === lineage.normalBucket
  )
}

function exportedContactLineage(contact: DebugContact): ContactLineage {
  return {
    colliderPairKey: sortedColliderPairKey(contact.colliders),
    featureId: contact.feature_id,
    normalBucket: approximateNormalBucket(contact.normal),
  }
}

function sortedColliderPairKey(colliders: [number, number]): string {
  return [...colliders].sort((left, right) => left - right).join("-")
}

function approximateNormalBucket(normal: Vec2): string {
  const angle = Math.atan2(normal.y, normal.x)
  const octant = Math.round(angle / (Math.PI / 4))
  return String((octant + 8) % 8)
}

function dedupeCcdContacts(contacts: DebugContact[]): DebugContact[] {
  const seen = new Set<string>()
  const unique: DebugContact[] = []
  for (const contact of contacts) {
    if (!contact.ccd_trace) {
      continue
    }
    const ccd = contact.ccd_trace
    const key = [
      ccd.moving_body,
      ccd.moving_collider,
      ccd.static_body,
      ccd.static_collider,
      contact.feature_id,
      rounded(ccd.toi_point.x),
      rounded(ccd.toi_point.y),
    ].join(":")
    if (seen.has(key)) {
      continue
    }
    seen.add(key)
    unique.push(contact)
  }
  return unique
}

function bodyMotionScore(previous: FrameRecord, current: FrameRecord): number {
  let maxMotion = 0
  for (const body of current.snapshot.bodies) {
    if (body.body_type !== "dynamic") {
      continue
    }
    const older = previous.snapshot.bodies.find((entry) => entry.handle === body.handle)
    if (!older) {
      continue
    }
    maxMotion = Math.max(
      maxMotion,
      distance(older.transform.translation, body.transform.translation),
    )
  }
  return maxMotion
}

function maxLinearSpeed(bodies: DebugBody[]): number {
  return bodies.reduce(
    (best, body) => Math.max(best, vectorLength(body.linear_velocity)),
    0,
  )
}

function pathDistance(points: Vec2[]): number | null {
  if (points.length < 2) {
    return null
  }
  let total = 0
  for (let index = 1; index < points.length; index += 1) {
    total += distance(points[index - 1], points[index])
  }
  return total
}

function pickTrajectoryColor(
  identity: number,
  islandId: number | null,
  colorMode: TrajectoryColorMode,
): string {
  const seed = colorMode === "island" && islandId != null ? islandId : identity
  return PALETTE[Math.abs(seed) % PALETTE.length]
}

function distance(left: Vec2, right: Vec2): number {
  return Math.hypot(right.x - left.x, right.y - left.y)
}

function vectorLength(value: Vec2): number {
  return Math.hypot(value.x, value.y)
}

function rounded(value: number): string {
  return value.toFixed(3)
}

function selectTopMarkers(markers: TrajectoryMarker[], limit: number): TrajectoryMarker[] {
  return markers
    .slice()
    .sort((left, right) => right.score - left.score || left.frameIndex - right.frameIndex)
    .slice(0, limit)
    .sort((left, right) => left.frameIndex - right.frameIndex)
}

function emptyStateForMode(mode: TrajectoryMode): TrajectoryEmptyState {
  switch (mode) {
    case "selectedBody":
      return "selectedBody"
    case "allDynamic":
      return "allDynamic"
    case "selectedIsland":
      return "selectedIsland"
    case "contacts":
      return "contacts"
    case "ccd":
      return "ccd"
  }
}
