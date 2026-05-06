import type { DebugBody, DebugContact, FrameRecord, SelectedEntity } from "../../types"
import type { ResolvedSelection } from "./types"

export const STACK_STABILITY_WINDOW = 24

export type EvidenceMetric = {
  value: number | null
  missingCount: number
}

export type StackMarkerKind =
  | "contactSpike"
  | "wakeTransition"
  | "driftSpike"
  | "angularDrift"
  | "solverSpike"
  | "quietWindow"

export type StackMarker = {
  kind: StackMarkerKind
  frameIndex: number
  score: number
}

export type StackStabilitySummary = {
  windowStart: number
  windowEnd: number
  dynamicBodyCount: number
  awakeBodyCount: number
  sleepingBodyCount: number
  contactCount: number
  contactSpike: number
  normalImpulseTotal: EvidenceMetric
  tangentImpulseTotal: EvidenceMetric
  solverRows: number | null
  activeIslandCount: number | null
  sleepingIslandCount: number | null
  maxBodyDrift: number | null
  maxAngularDrift: number | null
  jitterProxy: number | null
  quietFrames: number
}

export type BodyStabilityContribution = {
  kind: "body"
  drift: number | null
  angularDrift: number | null
  jitterProxy: number | null
  contactFrames: number
  normalImpulseTotal: EvidenceMetric
  tangentImpulseTotal: EvidenceMetric
  islandId: number | null
  sleeping: boolean | null
  awakeTransitions: number
}

export type ContactStabilityContribution = {
  kind: "contact"
  bodyIds: [number, number]
  contactFrames: number
  normalImpulse: EvidenceMetric
  tangentImpulse: EvidenceMetric
  islandIds: Array<number | null>
  sleepingBodies: Array<boolean | null>
}

export type StabilityContribution =
  | BodyStabilityContribution
  | ContactStabilityContribution
  | null

type FrameStats = NonNullable<FrameRecord["stats"]>

type BodyHistory = {
  current: DebugBody | null
  drift: number | null
  angularDrift: number | null
  jitterProxy: number | null
  awakeTransitions: number
}

// M35 keeps these metrics as a read-only web aggregation over exported frames.
// They help explain stability, but they are not authoritative solver facts.
export function buildStackStabilitySummary(
  frames: FrameRecord[],
  frameIndex: number,
): StackStabilitySummary {
  if (frames.length === 0) {
    return {
      windowStart: frameIndex,
      windowEnd: frameIndex,
      dynamicBodyCount: 0,
      awakeBodyCount: 0,
      sleepingBodyCount: 0,
      contactCount: 0,
      contactSpike: 0,
      normalImpulseTotal: { value: null, missingCount: 0 },
      tangentImpulseTotal: { value: null, missingCount: 0 },
      solverRows: null,
      activeIslandCount: null,
      sleepingIslandCount: null,
      maxBodyDrift: null,
      maxAngularDrift: null,
      jitterProxy: null,
      quietFrames: 0,
    }
  }
  const windowFrames = frameWindow(frames, frameIndex)
  const current = windowFrames[windowFrames.length - 1]
  const dynamicBodies = current.snapshot.bodies.filter(
    (body) => body.body_type === "dynamic",
  )
  const awakeBodyCount = dynamicBodies.filter((body) => !body.sleeping).length
  const sleepingBodyCount = dynamicBodies.filter((body) => body.sleeping).length
  const previousStats =
    windowFrames.length > 1 ? statsOf(windowFrames[windowFrames.length - 2]) : null
  const currentStats = statsOf(current)
  const contactSpike = Math.max(
    0,
    currentStats.contact_count - (previousStats?.contact_count ?? currentStats.contact_count),
  )
  const solverRows =
    (currentStats.contact_row_count ?? 0) + (currentStats.joint_row_count ?? 0)
  const islandCount = currentStats.island_count ?? 0
  const activeIslandCount =
    currentStats.active_island_count ?? deriveActiveIslandCount(current)
  const sleepingIslandCount =
    islandCount > 0 && activeIslandCount != null
      ? Math.max(0, islandCount - activeIslandCount)
      : deriveSleepingIslandCount(current)

  return {
    windowStart: windowFrames[0]?.frame_index ?? frameIndex,
    windowEnd: current.frame_index,
    dynamicBodyCount: dynamicBodies.length,
    awakeBodyCount,
    sleepingBodyCount,
    contactCount: current.snapshot.contacts.length,
    contactSpike,
    normalImpulseTotal: sumImpulseMetric(current.snapshot.contacts, "solver_normal_impulse"),
    tangentImpulseTotal: sumImpulseMetric(current.snapshot.contacts, "solver_tangent_impulse"),
    solverRows:
      solverRows > 0 ||
      currentStats.contact_row_count != null ||
      currentStats.joint_row_count != null
        ? solverRows
        : null,
    activeIslandCount:
      activeIslandCount != null && (activeIslandCount > 0 || islandCount > 0)
        ? activeIslandCount
        : null,
    sleepingIslandCount:
      sleepingIslandCount != null && (sleepingIslandCount > 0 || islandCount > 0)
        ? sleepingIslandCount
        : null,
    maxBodyDrift: maxWindowDrift(windowFrames),
    maxAngularDrift: maxWindowAngularDrift(windowFrames),
    jitterProxy: maxWindowJitter(windowFrames),
    quietFrames: consecutiveQuietFrames(windowFrames),
  }
}

export function buildStackMarkers(frames: FrameRecord[]): StackMarker[] {
  if (frames.length === 0) {
    return []
  }

  const contactSpikeMarkers: StackMarker[] = []
  const wakeTransitionMarkers: StackMarker[] = []
  const driftMarkers: StackMarker[] = []
  const angularDriftMarkers: StackMarker[] = []
  const solverMarkers: StackMarker[] = []
  const quietWindowMarkers: StackMarker[] = []

  for (let index = 0; index < frames.length; index += 1) {
    const current = frames[index]
    const previous = index > 0 ? frames[index - 1] : null
    const currentStats = statsOf(current)
    const previousStats = previous ? statsOf(previous) : null

    const contactSpike = Math.max(
      0,
      currentStats.contact_count - (previousStats?.contact_count ?? currentStats.contact_count),
    )
    if (contactSpike > 0) {
      contactSpikeMarkers.push({
        kind: "contactSpike",
        frameIndex: current.frame_index,
        score: contactSpike,
      })
    }

    const wakeScore = wakeTransitionScore(previous, current)
    if (wakeScore > 0) {
      wakeTransitionMarkers.push({
        kind: "wakeTransition",
        frameIndex: current.frame_index,
        score: wakeScore,
      })
    }

    const driftScore = maxWindowDrift(frameWindow(frames, index))
    if (driftScore != null && driftScore > 0.08) {
      driftMarkers.push({
        kind: "driftSpike",
        frameIndex: current.frame_index,
        score: driftScore,
      })
    }

    const angularDriftScore = maxWindowAngularDrift(frameWindow(frames, index))
    if (angularDriftScore != null && angularDriftScore > 0.08) {
      angularDriftMarkers.push({
        kind: "angularDrift",
        frameIndex: current.frame_index,
        score: angularDriftScore,
      })
    }

    const solverRows =
      (currentStats.contact_row_count ?? 0) + (currentStats.joint_row_count ?? 0)
    if (solverRows > 0) {
      solverMarkers.push({
        kind: "solverSpike",
        frameIndex: current.frame_index,
        score: solverRows,
      })
    }

    const quietFrames = consecutiveQuietFrames(frameWindow(frames, index))
    if (quietFrames >= 6) {
      quietWindowMarkers.push({
        kind: "quietWindow",
        frameIndex: current.frame_index,
        score: quietFrames,
      })
    }
  }

  return [
    ...selectTopMarkers(contactSpikeMarkers, 3),
    ...selectTopMarkers(wakeTransitionMarkers, 3),
    ...selectTopMarkers(driftMarkers, 2),
    ...selectTopMarkers(angularDriftMarkers, 2),
    ...selectTopMarkers(solverMarkers, 2),
    ...selectTopMarkers(quietWindowMarkers, 2),
  ].sort((left, right) => left.frameIndex - right.frameIndex)
}

export function buildStabilityContribution(
  frames: FrameRecord[],
  frameIndex: number,
  selected: ResolvedSelection,
): StabilityContribution {
  if (!selected || frames.length === 0) {
    return null
  }
  const windowFrames = frameWindow(frames, frameIndex)
  if (selected.kind === "body") {
    const history = bodyHistory(windowFrames, selected.entity.handle)
    return {
      kind: "body",
      drift: history.drift,
      angularDrift: history.angularDrift,
      jitterProxy: history.jitterProxy,
      contactFrames: windowFrames.filter((frame) =>
        frame.snapshot.contacts.some((contact) => contact.bodies.includes(selected.entity.handle)),
      ).length,
      normalImpulseTotal: sumImpulseMetric(
        flattenContacts(windowFrames).filter((contact) =>
          contact.bodies.includes(selected.entity.handle),
        ),
        "solver_normal_impulse",
      ),
      tangentImpulseTotal: sumImpulseMetric(
        flattenContacts(windowFrames).filter((contact) =>
          contact.bodies.includes(selected.entity.handle),
        ),
        "solver_tangent_impulse",
      ),
      islandId: history.current?.island_id ?? null,
      sleeping: history.current?.sleeping ?? null,
      awakeTransitions: history.awakeTransitions,
    }
  }

  if (selected.kind === "contact") {
    const pair = normalizePair(selected.entity.bodies)
    const matchingContacts = flattenContacts(windowFrames).filter((contact) =>
      matchesSelectedContact(contact, selected.entity),
    )
    const currentBodies = selected.entity.bodies.map((bodyId) =>
      windowFrames[windowFrames.length - 1]?.snapshot.bodies.find(
        (body) => body.handle === bodyId,
      ) ?? null,
    )
    return {
      kind: "contact",
      bodyIds: pair,
      contactFrames: windowFrames.filter((frame) =>
        frame.snapshot.contacts.some((contact) =>
          matchesSelectedContact(contact, selected.entity),
        ),
      ).length,
      normalImpulse: sumImpulseMetric(matchingContacts, "solver_normal_impulse"),
      tangentImpulse: sumImpulseMetric(matchingContacts, "solver_tangent_impulse"),
      islandIds: currentBodies.map((body) => body?.island_id ?? null),
      sleepingBodies: currentBodies.map((body) => body?.sleeping ?? null),
    }
  }

  return null
}

export function selectedBodyTrail(
  frames: FrameRecord[],
  frameIndex: number,
  bodyId: number,
): Array<{ x: number; y: number }> {
  return frameWindow(frames, frameIndex)
    .map(
      (frame) =>
        frame.snapshot.bodies.find((body) => body.handle === bodyId)?.transform.translation ??
        null,
    )
    .filter((point): point is { x: number; y: number } => Boolean(point))
}

function frameWindow(frames: FrameRecord[], frameIndex: number): FrameRecord[] {
  const clampedEnd = Math.min(frameIndex, Math.max(0, frames.length - 1))
  const start = Math.max(0, clampedEnd - STACK_STABILITY_WINDOW + 1)
  return frames.slice(start, clampedEnd + 1)
}

function statsOf(frame: FrameRecord): FrameStats {
  return frame.stats ?? frame.report?.stats ?? frame.snapshot.stats
}

function flattenContacts(frames: FrameRecord[]): DebugContact[] {
  return frames.flatMap((frame) => frame.snapshot.contacts)
}

// Exported contact ids are the strongest identity when present, but older or
// reduced windows may only preserve the same body pair + feature id. Match on
// both so the inspector stays scoped to one contact lineage without summing
// unrelated contacts on the same pair.
function matchesSelectedContact(
  contact: DebugContact,
  selected: DebugContact,
): boolean {
  const selectedPair = normalizePair(selected.bodies)
  const contactPair = normalizePair(contact.bodies)
  if (selected.id === contact.id) {
    return true
  }
  return (
    selected.feature_id === contact.feature_id &&
    selectedPair[0] === contactPair[0] &&
    selectedPair[1] === contactPair[1]
  )
}

function bodyHistory(frames: FrameRecord[], bodyId: number): BodyHistory {
  let current: DebugBody | null = null
  let drift: number | null = null
  let angularDrift: number | null = null
  let jitterProxy: number | null = null
  let awakeTransitions = 0
  let previous: DebugBody | null = null

  for (const frame of frames) {
    const body =
      frame.snapshot.bodies.find((candidate) => candidate.handle === bodyId) ?? null
    if (!body) {
      continue
    }
    current = body
    if (previous) {
      drift = maxNullable(
        drift,
        distance(previous.transform.translation, body.transform.translation),
      )
      angularDrift = maxNullable(
        angularDrift,
        Math.abs(previous.transform.rotation - body.transform.rotation),
      )
      jitterProxy = maxNullable(
        jitterProxy,
        distance(previous.linear_velocity, body.linear_velocity) +
          Math.abs(previous.angular_velocity - body.angular_velocity) * 0.25,
      )
      if (previous.sleeping !== body.sleeping) {
        awakeTransitions += 1
      }
    }
    previous = body
  }

  return {
    current,
    drift,
    angularDrift,
    jitterProxy,
    awakeTransitions,
  }
}

function sumImpulseMetric(
  contacts: DebugContact[],
  key: "solver_normal_impulse" | "solver_tangent_impulse",
): EvidenceMetric {
  let total = 0
  let seen = 0
  let missingCount = 0
  for (const contact of contacts) {
    const value = contact[key]
    if (typeof value === "number") {
      total += value
      seen += 1
    } else {
      missingCount += 1
    }
  }
  return {
    value: seen > 0 ? total : null,
    missingCount,
  }
}

function deriveActiveIslandCount(frame: FrameRecord): number | null {
  const islands = frame.snapshot.islands
  return islands?.length ? islands.filter((island) => !island.sleeping).length : null
}

function deriveSleepingIslandCount(frame: FrameRecord): number | null {
  const islands = frame.snapshot.islands
  return islands?.length ? islands.filter((island) => island.sleeping).length : null
}

function maxWindowDrift(frames: FrameRecord[]): number | null {
  let maxDrift: number | null = null
  for (const bodyId of dynamicBodyIds(frames[frames.length - 1])) {
    maxDrift = maxNullable(maxDrift, bodyHistory(frames, bodyId).drift)
  }
  return maxDrift
}

function maxWindowAngularDrift(frames: FrameRecord[]): number | null {
  let maxDrift: number | null = null
  for (const bodyId of dynamicBodyIds(frames[frames.length - 1])) {
    maxDrift = maxNullable(maxDrift, bodyHistory(frames, bodyId).angularDrift)
  }
  return maxDrift
}

function maxWindowJitter(frames: FrameRecord[]): number | null {
  let maxDrift: number | null = null
  for (const bodyId of dynamicBodyIds(frames[frames.length - 1])) {
    maxDrift = maxNullable(maxDrift, bodyHistory(frames, bodyId).jitterProxy)
  }
  return maxDrift
}

function dynamicBodyIds(frame: FrameRecord | undefined): number[] {
  return (
    frame?.snapshot.bodies
      .filter((body) => body.body_type === "dynamic")
      .map((body) => body.handle) ?? []
  )
}

function consecutiveQuietFrames(frames: FrameRecord[]): number {
  let quiet = 0
  for (let index = frames.length - 1; index >= 0; index -= 1) {
    const current = frames[index]
    const previous = index > 0 ? frames[index - 1] : null
    const currentStats = statsOf(current)
    const previousStats = previous ? statsOf(previous) : null
    const drift = maxWindowDrift(frameWindow(frames, index))
    const jitter = maxWindowJitter(frameWindow(frames, index))
    const contactDelta = Math.abs(
      currentStats.contact_count - (previousStats?.contact_count ?? currentStats.contact_count),
    )
    if (contactDelta <= 1 && (drift ?? 0) <= 0.08 && (jitter ?? 0) <= 0.35) {
      quiet += 1
      continue
    }
    break
  }
  return quiet
}

function wakeTransitionScore(previous: FrameRecord | null, current: FrameRecord): number {
  if (!previous) {
    return 0
  }
  const previousBodies = new Map(
    previous.snapshot.bodies
      .filter((body) => body.body_type === "dynamic")
      .map((body) => [body.handle, body]),
  )
  let score = 0
  for (const body of current.snapshot.bodies) {
    if (body.body_type !== "dynamic") {
      continue
    }
    const old = previousBodies.get(body.handle)
    if (old && old.sleeping !== body.sleeping) {
      score += 1
    }
  }
  return score
}

function selectTopMarkers(markers: StackMarker[], limit: number): StackMarker[] {
  const selected: StackMarker[] = []
  for (const marker of [...markers].sort(compareMarkerStrength)) {
    if (
      selected.some((existing) => Math.abs(existing.frameIndex - marker.frameIndex) <= 2)
    ) {
      continue
    }
    selected.push(marker)
    if (selected.length >= limit) {
      break
    }
  }
  return selected
}

function compareMarkerStrength(left: StackMarker, right: StackMarker): number {
  if (right.score !== left.score) {
    return right.score - left.score
  }
  return left.frameIndex - right.frameIndex
}

function normalizePair(pair: [number, number]): [number, number] {
  return pair[0] <= pair[1] ? pair : [pair[1], pair[0]]
}

function maxNullable(current: number | null, next: number | null): number | null {
  if (next == null) {
    return current
  }
  if (current == null) {
    return next
  }
  return Math.max(current, next)
}

function distance(left: { x: number; y: number }, right: { x: number; y: number }): number {
  return Math.hypot(left.x - right.x, left.y - right.y)
}
