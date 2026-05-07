import { useEffect, useMemo, useRef, useState } from "react"

import {
  commitVelocityPerturbation,
  controlSession,
  createSession,
  fetchFinalSnapshot,
  fetchFrames,
  fetchLiveFrame,
  fetchPerf,
  fetchScenarios,
  openSessionEvents,
  previewVelocityPerturbation,
} from "./api"
import { WorkbenchLayout } from "./components/workbench/WorkbenchLayout"
import { log, messageOf } from "./components/workbench/format"
import { resolveSelection } from "./components/workbench/selection"
import {
  buildStackStabilitySummary,
  type StackStabilitySummary,
} from "./components/workbench/stackStability"
import {
  buildTrajectoryMarkers,
  buildTrajectoryOverlay,
  buildTrajectorySummary,
  type TrajectoryMarker,
} from "./components/workbench/trajectory"
import {
  deriveLatticeProxy,
  defaultLayers,
  defaultTrajectorySettings,
  type CanvasDebugView,
  type ControlAction,
  type LayerState,
  type LiveCadenceStatus,
  type PerfEvidenceStatus,
  type RunMode,
  type SourceKind,
  type TrajectorySettings,
  type VelocityPerturbationPanelState,
  type VelocityPerturbationTarget,
} from "./components/workbench/types"
import { demoScenarios, makeDemoFrames } from "./demo"
import {
  actionLabel,
  detectInitialLocale,
  localizeScenario,
  scenarioGroupForId,
  statusLabel,
  storeLocale,
  t,
  type Locale,
  type MessageKey,
  type OverlayPresetId,
  type StatusKind,
} from "./i18n"
import { profileAsync, profileMeasure, profileStart } from "./profile"
import type {
  DebugSnapshot,
  FrameDiagnostics,
  FrameRecord,
  LiveFrameAuthority,
  LiveFrameSummary,
  PerfArtifact,
  ScenarioDescriptor,
  SessionControlResponse,
  SelectedEntity,
  SessionRecord,
  VelocityPerturbationCommit,
  VelocityPerturbationPreview,
  WorkbenchLog,
} from "./types"

const EMPTY_FRAME: FrameRecord = {
  kind: "full",
  frame_index: 0,
  simulated_time: 0,
  state_hash: "pending-live-frame",
  snapshot: {
    meta: {
      revision: null,
      dt: 0,
      simulated_time: 0,
      gravity: { x: 0, y: 0 },
    },
    bodies: [],
    colliders: [],
    joints: [],
    contacts: [],
    manifolds: [],
    islands: [],
    broadphase_tree: { root: null, depth: 0, nodes: [] },
    primitives: [],
    stats: {
      step_index: 0,
      active_body_count: 0,
      active_collider_count: 0,
      active_joint_count: 0,
      broadphase_candidate_count: 0,
      contact_count: 0,
      manifold_count: 0,
    },
  },
  compound_provenance: [],
  perturbation_provenance: [],
}

const LIVE_TARGET_FPS = 30
const LIVE_TARGET_FRAME_MS = 1000 / LIVE_TARGET_FPS

function idleLiveCadenceStatus(): LiveCadenceStatus {
  return {
    targetFps: LIVE_TARGET_FPS,
    actualFps: null,
    lastStepMs: null,
    nextDelayMs: null,
    degraded: false,
    pending: false,
  }
}

function liveAuthorityFromSummary(summary: LiveFrameSummary): LiveFrameAuthority {
  return {
    session_id: summary.session_id,
    session_epoch: summary.session_epoch,
    world_revision: summary.world_revision,
    status: summary.status,
    buffered_frame_count: summary.buffered_frame_count,
    frame_count: summary.frame_count,
    not_hydrated: true,
  }
}

function annotateFullLiveFrame(
  frame: FrameRecord,
  authority: Omit<LiveFrameAuthority, "not_hydrated">,
): FrameRecord {
  return {
    ...frame,
    kind: "full",
    live_authority: {
      ...authority,
      not_hydrated: false,
    },
  }
}

function frameFromLiveSummary(summary: LiveFrameSummary): FrameRecord {
  return {
    kind: "summary",
    frame_index: summary.frame_index,
    simulated_time: summary.simulated_time,
    state_hash: summary.state_hash,
    stats: summary.stats,
    snapshot: {
      meta: summary.snapshot.meta,
      bodies: summary.snapshot.bodies,
      colliders: summary.snapshot.colliders,
      joints: summary.snapshot.joints,
      contacts: [],
      manifolds: [],
      islands: undefined,
      broadphase_tree: undefined,
      primitives: summary.snapshot.primitives,
      stats: summary.snapshot.stats,
    },
    live_authority: liveAuthorityFromSummary(summary),
  }
}

function liveFrameFromControlResponse(
  response: SessionControlResponse,
): FrameRecord | null {
  const summary = response.live_frame_summary ?? null
  if (response.session.latest_frame) {
    return annotateFullLiveFrame(response.session.latest_frame, {
      session_id: response.session.id,
      session_epoch: response.session.session_epoch,
      world_revision:
        summary?.world_revision ??
        response.session.latest_frame.snapshot.meta.revision ??
        null,
      status: response.session.status,
      buffered_frame_count: response.session.buffered_frame_count,
      frame_count: response.session.frame_count,
    })
  }
  if (summary) {
    return frameFromLiveSummary(summary)
  }
  return null
}

function isSummaryFrame(frame: FrameRecord | null | undefined): boolean {
  return frame?.kind === "summary" || frame?.live_authority?.not_hydrated === true
}

type LiveResponseGuard = {
  generation: number
  token: number
  sessionId: string
}

type PerturbationResponseGuard = {
  token: number
  actionId: string
  sessionId: string
  frameIndex: number
  sessionEpoch: number
  bodyHandle: number
  worldRevision: number
}

type PerturbationAvailability = {
  reasonKey: MessageKey | null
  latestAuthoritativeLiveFrame: FrameRecord | null
  isAuthoritativeLiveFrameSelected: boolean
  worldRevision: number | null
}

type DebugContextPayload = {
  scenario: {
    id: string
    group: ReturnType<typeof scenarioGroupForId>
    name: string
    description: string
  }
  source: {
    kind: SourceKind
    status: StatusKind
    runMode: RunMode
  }
  session: {
    sessionId: string | null
    sessionEpoch: number | null
    runId: string | null
    frameIndex: number
    bufferedFrameCount: number
    stateHash: string
    worldRevision: number | null
    liveAuthority: {
      latestFrameIndex: number | null
      latestStateHash: string | null
      latestWorldRevision: number | null
      viewingLatestAuthoritativeFrame: boolean
    }
  }
  selection: SelectedEntity | null
  layers: {
    enabled: Array<keyof LayerState>
    values: LayerState
  }
  camera: {
    mode: CanvasDebugView["mode"]
    targetDescription: string | null
    targetBounds: CanvasDebugView["targetBounds"]
    dampingEnabled: boolean
    center: CanvasDebugView["center"]
    zoom: number
    grid: boolean
    rulers: boolean
  } | null
  trajectory: {
    settings: TrajectorySettings
    overlay: {
      mode: string
      windowStart: number
      windowEnd: number
      emptyState: string | null
      bodyTrailCount: number
      contactTrailCount: number
      ccdTrailCount: number
    } | null
    summary: ReturnType<typeof buildTrajectorySummary> | null
    markers: Array<{ kind: string; frameIndex: number; score: number }>
    notHydrated: boolean
  }
  stackStability: StackStabilitySummary | null
  diagnostics: {
    frame: FrameDiagnostics | null
    summary: {
      available: boolean
      marker_count: number | null
      marker_kinds: string[]
      missing_evidence: string[]
      counter_delta_source: string | null
      top_marker_threshold: string | null
    }
  }
  latticeProxy: {
    enabled: boolean
    nodeCount: number | null
    edgeCount: number | null
    worldAnchorEdgeCount: number | null
    maxStretchRatio: number | null
    jointRowCount: number | null
    contactCount: number | null
    islandCount: number | null
    activeIslandCount: number | null
    sleepingIslandCount: number | null
    proxyOnly: true
    notHydrated: boolean
  }
  perturbation: {
    selectedBodyHandle: number | null
    sessionEpoch: number | null
    worldRevision: number | null
    delta: { x: string; y: string }
    preview: {
      actionId: string
      rejectionReason: string | null
      beforeVelocity: { x: number; y: number } | null
      requestedDelta: { x: number; y: number } | null
      targetVelocity: { x: number; y: number } | null
      wakeIntent: boolean
      frameIndex: number
      sessionEpoch: number
      worldRevision: number
    } | null
    commit: {
      actionId: string
      accepted: boolean
      rejectionReason: string | null
      querySyncStatus: string | null
      targetVelocity: { x: number; y: number } | null
      frameIndex: number
      sessionEpoch: number
      worldRevision: number
    } | null
    currentFrameProvenance: Array<{
      actionId: string
      bodyHandle: number
      frameIndex: number
      sessionEpoch: number
      worldRevision: number
      requestedDelta: { x: number; y: number }
      targetVelocity: { x: number; y: number }
      commitOutcome: string
    }>
  }
  perf: {
    status: PerfEvidenceStatus
    frameCount: number
    finalStateHash: string
    counterSummary: PerfArtifact["counter_summary"] | null
  } | null
}

function shouldLogLiveFrameBuffer(frameIndex: number): boolean {
  return frameIndex === 0 || (frameIndex + 1) % 30 === 0
}

function buildVelocityPerturbationActionId(
  sessionId: string,
  bodyHandle: number,
  frameIndex: number,
): string {
  return `vp-${sessionId}-${bodyHandle}-${frameIndex}-${Date.now().toString(36)}`
}

function parseVelocityDeltaInput(value: string): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : 0
}

async function copyTextToClipboard(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text)
      return
    } catch {
      // Some embedded browsers deny Clipboard API writes even from a click handler.
    }
  }

  const textArea = document.createElement("textarea")
  textArea.value = text
  textArea.readOnly = true
  textArea.style.position = "fixed"
  textArea.style.left = "-9999px"
  textArea.style.top = "0"
  document.body.appendChild(textArea)
  textArea.focus()
  textArea.select()

  try {
    if (!document.execCommand("copy")) {
      throw new Error("document.execCommand('copy') returned false")
    }
  } finally {
    document.body.removeChild(textArea)
  }
}

function overlayPresetState(preset: OverlayPresetId): {
  layers: LayerState
  trajectory: TrajectorySettings
} {
  const baseLayers: LayerState = {
    ...defaultLayers,
    broadphaseTree: false,
    islands: false,
    provenance: false,
    stackStability: false,
    lattice: false,
    trace: true,
  }

  if (preset === "stackStability") {
    return {
      layers: {
        ...baseLayers,
        contacts: true,
        velocities: true,
        stackStability: true,
        islands: true,
      },
      trajectory: {
        mode: "selectedIsland",
        historyLength: 96,
        samplingStride: 2,
        fade: 0.78,
        colorMode: "island",
      },
    }
  }

  if (preset === "trajectoryFocus") {
    return {
      layers: {
        ...baseLayers,
        contacts: false,
        velocities: false,
      },
      trajectory: {
        mode: "selectedBody",
        historyLength: 128,
        samplingStride: 1,
        fade: 0.84,
        colorMode: "body",
      },
    }
  }

  if (preset === "perturbationReview") {
    return {
      layers: {
        ...baseLayers,
        contacts: true,
        velocities: true,
        provenance: true,
        stackStability: true,
      },
      trajectory: {
        mode: "selectedBody",
        historyLength: 72,
        samplingStride: 1,
        fade: 0.8,
        colorMode: "body",
      },
    }
  }

  return {
    layers: {
      ...baseLayers,
      contacts: false,
      velocities: false,
      islands: true,
      lattice: true,
    },
    trajectory: {
      mode: "selectedIsland",
      historyLength: 80,
      samplingStride: 2,
      fade: 0.7,
      colorMode: "island",
    },
  }
}

function selectTrajectoryContextMarkers(
  markers: TrajectoryMarker[],
  windowStart: number,
  windowEnd: number,
  frameIndex: number,
): TrajectoryMarker[] {
  const byKey = new Map<string, TrajectoryMarker>()
  for (const marker of markers) {
    const inWindow =
      marker.frameIndex >= windowStart && marker.frameIndex <= windowEnd
    const nearViewedFrame = Math.abs(marker.frameIndex - frameIndex) <= 12
    if (inWindow || nearViewedFrame) {
      byKey.set(`${marker.kind}:${marker.frameIndex}`, marker)
    }
  }

  const candidates =
    byKey.size > 0
      ? [...byKey.values()]
      : [...markers]
          .sort(
            (left, right) =>
              Math.abs(left.frameIndex - frameIndex) -
                Math.abs(right.frameIndex - frameIndex) ||
              right.score - left.score,
          )
          .slice(0, 6)

  return candidates
    .sort(
      (left, right) =>
        Math.abs(left.frameIndex - frameIndex) -
          Math.abs(right.frameIndex - frameIndex) ||
        right.score - left.score,
    )
    .slice(0, 6)
    .sort((left, right) => left.frameIndex - right.frameIndex)
}

function summarizeFrameDiagnostics(
  diagnostics: FrameDiagnostics | null | undefined,
): DebugContextPayload["diagnostics"]["summary"] {
  if (!diagnostics) {
    return {
      available: false,
      marker_count: null,
      marker_kinds: [],
      missing_evidence: ["frame_diagnostics"],
      counter_delta_source: "missing",
      top_marker_threshold: null,
    }
  }

  const markers = diagnostics.markers ?? []
  const missingEvidence = diagnostics.missing_evidence ?? []

  return {
    available: true,
    marker_count: markers.length,
    marker_kinds: markers.slice(0, 4).map((marker) => marker.kind),
    missing_evidence: missingEvidence.map((entry) => entry.kind),
    counter_delta_source: diagnostics.performance?.counter_delta?.source ?? null,
    top_marker_threshold: markers[0]?.threshold_name ?? null,
  }
}

export function App() {
  const [locale, setLocale] = useState<Locale>(() => detectInitialLocale())
  const [scenarios, setScenarios] =
    useState<ScenarioDescriptor[]>(demoScenarios)
  const [selectedScenario, setSelectedScenario] = useState(
    "falling_box_contact",
  )
  const [frameCount, setFrameCount] = useState(120)
  const [runMode, setRunMode] = useState<RunMode>("artifact_replay")
  const [frames, setFrames] = useState<FrameRecord[]>(() =>
    makeDemoFrames("falling_box_contact", 120),
  )
  const [frameIndex, setFrameIndex] = useState(0)
  const [selectedEntity, setSelectedEntity] = useState<SelectedEntity | null>({
    kind: "collider",
    id: 2,
  })
  const [sessionId, setSessionId] = useState<string | null>(null)
  const [runId, setRunId] = useState<string | null>(null)
  const [manifestArtifact, setManifestArtifact] = useState<string | null>(null)
  const [finalSnapshotArtifact, setFinalSnapshotArtifact] = useState<
    string | null
  >(null)
  const [finalSnapshot, setFinalSnapshot] = useState<DebugSnapshot | null>(null)
  const [perfArtifact, setPerfArtifact] = useState<PerfArtifact | null>(null)
  const [perfStatus, setPerfStatus] = useState<PerfEvidenceStatus>("none")
  const [source, setSource] = useState<SourceKind>("demo")
  const [status, setStatus] = useState<StatusKind>("idle")
  const [logs, setLogs] = useState<WorkbenchLog[]>([
    log("warn", t(locale, "log.serverNotConfirmed")),
  ])
  const [layers, setLayers] = useState<LayerState>(defaultLayers)
  const [trajectorySettings, setTrajectorySettings] = useState<TrajectorySettings>(
    defaultTrajectorySettings,
  )
  const [canvasView, setCanvasView] = useState<CanvasDebugView | null>(null)
  const [useCustomGravity, setUseCustomGravity] = useState(false)
  const [gravityY, setGravityY] = useState(9.8)
  const [liveControlBusy, setLiveControlBusy] = useState(false)
  const [liveCadence, setLiveCadence] = useState<LiveCadenceStatus>(() =>
    idleLiveCadenceStatus(),
  )
  const [sessionEpoch, setSessionEpoch] = useState(0)
  const [perturbationDeltaX, setPerturbationDeltaX] = useState("0.0")
  const [perturbationDeltaY, setPerturbationDeltaY] = useState("0.0")
  const [perturbationPreview, setPerturbationPreview] =
    useState<VelocityPerturbationPreview | null>(null)
  const [perturbationCommit, setPerturbationCommit] =
    useState<VelocityPerturbationCommit | null>(null)
  const [perturbationBusy, setPerturbationBusy] =
    useState<VelocityPerturbationPanelState["busy"]>("idle")
  const [perturbationRequestError, setPerturbationRequestError] =
    useState<string | null>(null)
  const playTimer = useRef<number | null>(null)
  const liveStepInFlight = useRef(false)
  const liveHydrationInFlight = useRef<string | null>(null)
  const liveGenerationRef = useRef(0)
  const liveRequestTokenRef = useRef(0)
  const liveLoopGenerationRef = useRef(0)
  const perturbationRequestTokenRef = useRef(0)

  const currentFrame =
    frames[Math.min(frameIndex, Math.max(0, frames.length - 1))] ?? EMPTY_FRAME
  // Live velocity edits are only authoritative on the newest server-backed frame.
  const latestAuthoritativeLiveFrame =
    source === "live" ? (frames[frames.length - 1] ?? null) : null
  const isAuthoritativeLiveFrameSelected =
    latestAuthoritativeLiveFrame != null &&
    frameIndex === latestAuthoritativeLiveFrame.frame_index &&
    currentFrame.state_hash === latestAuthoritativeLiveFrame.state_hash
  const scenario = localizeScenario(
    locale,
    scenarios.find((entry) => entry.id === selectedScenario) ?? scenarios[0],
  )

  useEffect(() => {
    document.documentElement.lang = locale
    storeLocale(locale)
  }, [locale])

  useEffect(() => {
    let cancelled = false
    fetchScenarios()
      .then((next) => {
        if (cancelled) {
          return
        }
        setScenarios(next)
        pushLogs(log("info", t(locale, "log.connectedScenarios")))
      })
      .catch((error: Error) => {
        if (cancelled) {
          return
        }
        setSource("demo")
        pushLogs(
          log(
            "warn",
            t(locale, "log.serverUnavailable", { message: error.message }),
          ),
        )
      })
    return () => {
      cancelled = true
    }
  }, [locale])

  useEffect(() => {
    if (source !== "live" || status !== "playing") {
      return
    }

    let cancelled = false
    const loopGeneration = ++liveLoopGenerationRef.current

    async function tick() {
      if (cancelled || loopGeneration !== liveLoopGenerationRef.current) {
        return
      }

      const startedAt = profileStart()
      setLiveCadence((current) => ({
        ...current,
        pending: true,
        nextDelayMs: null,
      }))

      await advanceLiveFrame()

      const stepMs = performance.now() - startedAt
      const nextDelayMs = Math.max(0, LIVE_TARGET_FRAME_MS - stepMs)
      const actualFps = 1000 / Math.max(LIVE_TARGET_FRAME_MS, stepMs)
      const degraded = stepMs > LIVE_TARGET_FRAME_MS * 1.15

      setLiveCadence({
        targetFps: LIVE_TARGET_FPS,
        actualFps,
        lastStepMs: stepMs,
        nextDelayMs,
        degraded,
        pending: false,
      })
      profileMeasure("live.cadence", startedAt, {
        targetFps: LIVE_TARGET_FPS,
        actualFps: Number(actualFps.toFixed(1)),
        stepMs: Number(stepMs.toFixed(1)),
        nextDelayMs: Number(nextDelayMs.toFixed(1)),
        degraded,
      })

      if (!cancelled && loopGeneration === liveLoopGenerationRef.current) {
        playTimer.current = window.setTimeout(tick, nextDelayMs)
      }
    }

    void tick()

    return () => {
      cancelled = true
      liveLoopGenerationRef.current += 1
      if (playTimer.current !== null) {
        window.clearTimeout(playTimer.current)
        playTimer.current = null
      }
      setLiveCadence((current) => ({
        ...current,
        pending: false,
        nextDelayMs: null,
      }))
    }
  }, [source, status]) // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (source === "live" || status !== "playing") {
      return
    }

    playTimer.current = window.setInterval(() => {
      setFrameIndex((next) => {
        if (next >= frames.length - 1) {
          setStatus("paused")
          return next
        }
        return next + 1
      })
    }, LIVE_TARGET_FRAME_MS)

    return () => {
      if (playTimer.current !== null) {
        window.clearInterval(playTimer.current)
        playTimer.current = null
      }
    }
  }, [frames.length, source, status])

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (
        event.target instanceof HTMLInputElement ||
        event.target instanceof HTMLTextAreaElement
      ) {
        return
      }
      switch (event.key) {
        case " ":
          event.preventDefault()
          if (status === "playing") {
            void handleControl("pause")
          } else {
            void handleControl("play")
          }
          break
        case "ArrowRight":
          event.preventDefault()
          if (source === "live" && frameIndex >= frames.length - 1) {
            void handleControl("step")
          } else {
            setStatus("paused")
            setFrameIndex((i) => Math.min(Math.max(0, frames.length - 1), i + 1))
          }
          break
        case "ArrowLeft":
          event.preventDefault()
          setStatus("paused")
          setFrameIndex((i) => Math.max(0, i - 1))
          break
        case "Escape":
          event.preventDefault()
          setSelectedEntity(null)
          break
      }
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [status, frameIndex, frames.length, source]) // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (source !== "live" || status !== "paused" || !sessionId) {
      return
    }
    if (!isSummaryFrame(currentFrame)) {
      return
    }
    void hydrateLiveFrameIfNeeded(frameIndex)
  }, [
    currentFrame.kind,
    currentFrame.state_hash,
    frameIndex,
    sessionEpoch,
    sessionId,
    source,
    status,
  ])

  const selectedDetails = useMemo(
    () => resolveSelection(currentFrame, selectedEntity),
    [currentFrame, selectedEntity],
  )
  const stackSummary = useMemo(
    () => buildStackStabilitySummary(frames, frameIndex),
    [frameIndex, frames],
  )
  const trajectoryOverlay = useMemo(
    () =>
      buildTrajectoryOverlay(
        frames,
        frameIndex,
        selectedEntity,
        trajectorySettings,
      ),
    [frameIndex, frames, selectedEntity, trajectorySettings],
  )
  const trajectorySummary = useMemo(
    () =>
      buildTrajectorySummary(
        frames,
        frameIndex,
        selectedEntity,
        trajectorySettings,
      ),
    [frameIndex, frames, selectedEntity, trajectorySettings],
  )
  const trajectoryMarkers = useMemo(() => buildTrajectoryMarkers(frames), [frames])
  const latticeSummary = useMemo(
    () => deriveLatticeProxy(currentFrame, frames[0] ?? currentFrame),
    [currentFrame, frames],
  )
  const currentFrameNotHydrated = isSummaryFrame(currentFrame)
  const hasUnhydratedLiveFrames = useMemo(
    () => frames.some(isSummaryFrame),
    [frames],
  )

  const selectedPerturbationTarget = useMemo<VelocityPerturbationTarget | null>(
    () => {
      if (selectedDetails?.kind === "body") {
        return {
          bodyHandle: selectedDetails.entity.handle,
          bodyType: selectedDetails.entity.body_type,
          currentVelocity: selectedDetails.entity.linear_velocity,
        }
      }
      if (selectedDetails?.kind === "collider" && selectedDetails.body) {
        return {
          bodyHandle: selectedDetails.body.handle,
          bodyType: selectedDetails.body.body_type,
          currentVelocity: selectedDetails.body.linear_velocity,
        }
      }
      return null
    },
    [selectedDetails],
  )

  useEffect(() => {
    invalidatePerturbationResponses()
    setPerturbationBusy("idle")
    setPerturbationPreview(null)
    setPerturbationCommit(null)
    setPerturbationRequestError(null)
  }, [
    perturbationDeltaX,
    perturbationDeltaY,
    source,
    sessionId,
    frameIndex,
    status,
    sessionEpoch,
    currentFrame.state_hash,
    selectedPerturbationTarget?.bodyHandle,
  ])

  async function runScenario() {
    setStatus("loading")
    setFrameIndex(0)
    setSelectedEntity(null)
    invalidateLiveResponses()
    const gravity = useCustomGravity
      ? ([0, gravityY] as [number, number])
      : null

    try {
      const session = await createSession(
        selectedScenario,
        frameCount,
        runMode,
        gravity,
      )
      setSessionEpoch(session.session_epoch)
      subscribeToEvents(session.id)

      if (session.mode === "live_session") {
        activateLiveSession(session.id)
        clearArtifacts()
        setPerfStatus("live_unavailable")
        setFrames([])
        setFrameIndex(0)
        setSource("live")
        setStatus(session.status)
        pushLogs(
          log(
            "info",
            t(locale, "log.liveSessionReady", { sessionId: session.id }),
          ),
          log(
            "info",
            t(locale, "log.sessionStatus", {
              sessionId: session.id,
              status: statusLabel(locale, session.status),
            }),
          ),
        )
        await startLiveSessionFromRun(session.id)
        return
      }

      setSessionId(session.id)
      if (!session.run_id) {
        throw new Error(t(locale, "error.sessionWithoutRun"))
      }
      const completedRunId = session.run_id
      const nextFrames = await fetchFrames(completedRunId)
      if (nextFrames.length === 0) {
        throw new Error(t(locale, "error.emptyFrames"))
      }
      const nextFinalSnapshot = await fetchFinalSnapshot(completedRunId)
      setRunId(completedRunId)
      setManifestArtifact(session.manifest_artifact ?? "manifest.json")
      setFinalSnapshotArtifact(
        session.final_snapshot_artifact ?? "final_snapshot.json",
      )
      setFinalSnapshot(nextFinalSnapshot)
      await loadPerfArtifact(completedRunId)
      setFrames(nextFrames)
      setSource("artifact")
      setStatus("paused")
      pushLogs(
        log(
          "info",
          t(locale, "log.loadedFrames", {
            count: nextFrames.length,
            runId: completedRunId,
          }),
        ),
        log(
          "info",
          t(locale, "log.finalSnapshotLoaded", {
            step: nextFinalSnapshot.stats.step_index,
          }),
        ),
        log(
          "info",
          t(locale, "log.artifactsAvailable", {
            manifest: session.manifest_artifact ?? "manifest.json",
            finalSnapshot:
              session.final_snapshot_artifact ?? "final_snapshot.json",
          }),
        ),
        log(
          "info",
          t(locale, "log.sessionStatus", {
            sessionId: session.id,
            status: statusLabel(locale, session.status),
          }),
        ),
      )
    } catch (error) {
      const nextFrames = makeDemoFrames(selectedScenario, frameCount)
      setFrames(nextFrames)
      clearRunState()
      setSource("demo")
      setStatus("paused")
      pushLogs(
        log(
          "warn",
          t(locale, "log.serverRunFailed", { message: messageOf(error) }),
        ),
        log(
          "info",
          t(locale, "log.generatedDemoFrames", {
            count: nextFrames.length,
            scenarioId: selectedScenario,
          }),
        ),
      )
    }
  }

  async function advanceLiveFrame({
    logAccepted = false,
    detail = "summary",
  }: { logAccepted?: boolean; detail?: "summary" | "full" } = {}) {
    await profileAsync("live.advanceFrame", async () => {
      if (!sessionId || liveStepInFlight.current) {
        return
      }
      liveStepInFlight.current = true
      const guard = issueLiveGuard(sessionId)
      try {
        const result = await controlSession(sessionId, "step", detail)
        applyLiveSessionFrame(result, guard)
        const nextFrame = liveFrameFromControlResponse(result)
        const nextFrameIndex = nextFrame?.frame_index
        if (
          logAccepted ||
          (nextFrameIndex !== undefined && shouldLogLiveFrameBuffer(nextFrameIndex))
        ) {
          pushLogs(
            log(
              "info",
              t(locale, "log.serverAccepted", {
                action: actionLabel(locale, "step"),
                status: statusLabel(locale, result.session.status),
              }),
            ),
          )
        }
      } catch (error) {
        setStatus("paused")
        pushLogs(
          log(
            "warn",
            t(locale, "log.serverControlFailed", {
              action: actionLabel(locale, "step"),
              message: messageOf(error),
            }),
          ),
        )
      } finally {
        liveStepInFlight.current = false
      }
    }, () => ({
      bufferedFrames: frames.length,
      frameIndex,
      scenarioId: selectedScenario,
    }))
  }

  function applyLiveSessionFrame(
    result: SessionControlResponse | { session: SessionRecord; live_frame_summary?: LiveFrameSummary | null },
    guard: LiveResponseGuard,
  ) {
    const startedAt = profileStart()
    const session = result.session
    const nextFrame = liveFrameFromControlResponse(result)
    const accepted = !shouldIgnoreLiveResponse(guard, session)
    try {
      if (!accepted) {
        return
      }
      setSessionEpoch(session.session_epoch)
      if (nextFrame) {
        setFrames((prev) => {
          if (
            prev[nextFrame.frame_index]?.state_hash === nextFrame.state_hash &&
            prev[nextFrame.frame_index]?.kind === nextFrame.kind
          ) {
            return prev
          }
          const next = prev.slice(0, nextFrame.frame_index)
          next.push(nextFrame)
          return next
        })
        setFrameIndex(nextFrame.frame_index)
        if (shouldLogLiveFrameBuffer(nextFrame.frame_index)) {
          pushLogs(
            log(
              "info",
              t(locale, "log.liveFrameBuffered", { frameIndex: nextFrame.frame_index }),
            ),
          )
        }
      }
      if (session.status === "completed") {
        setStatus("paused")
      } else if (status !== "playing") {
        setStatus(session.status)
      }
    } finally {
      profileMeasure("live.applyFrame", startedAt, {
        accepted,
        nextFrameIndex: nextFrame?.frame_index ?? null,
        bodies: nextFrame?.snapshot.bodies.length ?? 0,
        contacts: nextFrame?.snapshot.contacts.length ?? 0,
        detail: nextFrame?.kind ?? null,
      })
    }
  }

  async function hydrateLiveFrameIfNeeded(targetFrameIndex: number): Promise<void> {
    if (source !== "live" || !sessionId) {
      return
    }
    const frame = frames[targetFrameIndex]
    if (!frame || !isSummaryFrame(frame) || !frame.live_authority) {
      return
    }
    const authority = frame.live_authority
    const hydrationKey = [
      authority.session_id,
      authority.session_epoch,
      frame.frame_index,
      frame.state_hash,
    ].join(":")
    if (liveHydrationInFlight.current === hydrationKey) {
      return
    }
    liveHydrationInFlight.current = hydrationKey
    try {
      const result = await fetchLiveFrame(sessionId, frame.frame_index)
      setFrames((prev) => {
        const current = prev[targetFrameIndex]
        if (
          !current ||
          !isSummaryFrame(current) ||
          !current.live_authority ||
          current.frame_index !== result.frame_index ||
          current.state_hash !== result.frame.state_hash ||
          current.live_authority.session_id !== result.session_id ||
          current.live_authority.session_epoch !== result.session_epoch ||
          current.live_authority.world_revision !== result.world_revision
        ) {
          return prev
        }
        const next = prev.slice()
        next[targetFrameIndex] = annotateFullLiveFrame(result.frame, {
          session_id: result.session_id,
          session_epoch: result.session_epoch,
          world_revision: result.world_revision,
          status: current.live_authority.status,
          buffered_frame_count: current.live_authority.buffered_frame_count,
          frame_count: current.live_authority.frame_count,
        })
        return next
      })
    } catch {
      // Keep the summary frame visible; diagnostics panels already expose this
      // as not hydrated instead of silently claiming zero evidence.
    } finally {
      if (liveHydrationInFlight.current === hydrationKey) {
        liveHydrationInFlight.current = null
      }
    }
  }

  function activateLiveSession(nextSessionId: string) {
    invalidateLiveResponses()
    setSessionId(nextSessionId)
  }

  async function startLiveSessionFromRun(nextSessionId: string) {
    const guard = issueLiveGuard(nextSessionId)
    setLiveControlBusy(true)
    try {
      const result = await controlSession(nextSessionId, "play")
      const session = result.session
      if (shouldIgnoreLiveResponse(guard, session)) {
        return
      }
      setSessionEpoch(session.session_epoch)
      pushLogs(
        log(
          "info",
          t(locale, "log.serverAccepted", {
            action: actionLabel(locale, "play"),
            status: statusLabel(locale, session.status),
          }),
        ),
      )
      setStatus("playing")
    } catch (error) {
      setStatus("paused")
      pushLogs(
        log(
          "warn",
          t(locale, "log.serverControlFailed", {
            action: actionLabel(locale, "play"),
            message: messageOf(error),
          }),
        ),
      )
    } finally {
      setLiveControlBusy(false)
    }
  }

  function invalidateLiveResponses() {
    liveGenerationRef.current += 1
    liveRequestTokenRef.current = 0
    liveLoopGenerationRef.current += 1
    liveStepInFlight.current = false
    liveHydrationInFlight.current = null
    setLiveCadence(idleLiveCadenceStatus())
    invalidatePerturbationResponses()
    setLiveControlBusy(false)
  }

  function invalidatePerturbationResponses() {
    perturbationRequestTokenRef.current += 1
  }

  function issueLiveGuard(
    sessionIdValue: string,
    bumpGeneration = false,
  ): LiveResponseGuard {
    if (bumpGeneration) {
      invalidateLiveResponses()
    }
    return {
      generation: liveGenerationRef.current,
      token: ++liveRequestTokenRef.current,
      sessionId: sessionIdValue,
    }
  }

  function shouldIgnoreLiveResponse(
    guard: LiveResponseGuard,
    session: SessionRecord,
  ) {
    if (
      guard.generation !== liveGenerationRef.current ||
      guard.token !== liveRequestTokenRef.current
    ) {
      return true
    }
    return guard.sessionId !== session.id
  }

  function issuePerturbationGuard(
    actionIdValue: string,
    sessionIdValue: string,
    frameIndexValue: number,
    sessionEpochValue: number,
    bodyHandleValue: number,
    worldRevisionValue: number,
  ): PerturbationResponseGuard {
    // Preview uses its own token lane so a late response cannot revive submit
    // after a step/reset/scrub or a second preview request.
    return {
      token: ++perturbationRequestTokenRef.current,
      actionId: actionIdValue,
      sessionId: sessionIdValue,
      frameIndex: frameIndexValue,
      sessionEpoch: sessionEpochValue,
      bodyHandle: bodyHandleValue,
      worldRevision: worldRevisionValue,
    }
  }

  function isCurrentPerturbationGuard(guard: PerturbationResponseGuard): boolean {
    return (
      guard.token === perturbationRequestTokenRef.current &&
      guard.sessionId === sessionId &&
      guard.frameIndex === frameIndex &&
      guard.sessionEpoch === sessionEpoch &&
      guard.bodyHandle === selectedPerturbationTarget?.bodyHandle &&
      guard.worldRevision === perturbationAvailability.worldRevision
    )
  }

  function shouldIgnorePerturbationResponse(
    guard: PerturbationResponseGuard,
    preview: VelocityPerturbationPreview,
  ) {
    if (!isCurrentPerturbationGuard(guard)) {
      return true
    }
    return (
      preview.action_id !== guard.actionId ||
      preview.session_id !== guard.sessionId ||
      preview.frame_index !== guard.frameIndex ||
      preview.session_epoch !== guard.sessionEpoch ||
      preview.body_handle !== guard.bodyHandle ||
      preview.world_revision !== guard.worldRevision
    )
  }

  function shouldIgnorePerturbationCommitResponse(
    guard: PerturbationResponseGuard,
    result: {
      commit: VelocityPerturbationCommit
      session: SessionRecord
    },
  ) {
    if (!isCurrentPerturbationGuard(guard)) {
      return true
    }
    return (
      result.session.id !== guard.sessionId ||
      result.commit.action_id !== guard.actionId ||
      result.commit.session_id !== guard.sessionId ||
      result.commit.frame_index !== guard.frameIndex ||
      result.commit.body_handle !== guard.bodyHandle
    )
  }

  function subscribeToEvents(nextSessionId: string) {
    try {
      const events = openSessionEvents(nextSessionId)
      events.addEventListener("frame", (event) => {
        pushLogs(log("info", t(locale, "log.sseFrame", { data: event.data })))
      })
      events.addEventListener("failed", (event) => {
        pushLogs(
          log("error", t(locale, "log.sseFailed", { data: event.data })),
        )
      })
      events.addEventListener("idle", (event) => {
        pushLogs(log("info", t(locale, "log.sseIdle", { data: event.data })))
      })
      window.setTimeout(() => events.close(), 2000)
    } catch (error) {
      pushLogs(
        log(
          "warn",
          t(locale, "log.sseUnavailable", { message: messageOf(error) }),
        ),
      )
    }
  }

  function changeScenario(nextScenario: string) {
    invalidateLiveResponses()
    setSelectedScenario(nextScenario)
    setFrames(makeDemoFrames(nextScenario, frameCount))
    setFrameIndex(0)
    setSelectedEntity(null)
    clearRunState()
    setSource("demo")
  }

  async function handleControl(action: ControlAction) {
    if (source === "live" && sessionId) {
      await handleLiveControl(action, sessionId)
      return
    }

    if (source === "artifact" && sessionId) {
      await handleArtifactControl(action, sessionId)
      return
    }

    handleLocalControl(action)
  }

  function handleLocalControl(action: ControlAction) {
    if (action === "pause") {
      setStatus("paused")
    } else if (action === "play") {
      setStatus("playing")
    } else if (action === "step") {
      setStatus("paused")
      setFrameIndex((value) => Math.min(Math.max(0, frames.length - 1), value + 1))
    } else {
      setStatus("paused")
      setFrameIndex(0)
    }
  }

  async function handleLiveControl(
    action: ControlAction,
    activeSessionId: string,
  ) {
    if (liveStepInFlight.current && action !== "reset" && action !== "pause") {
      return
    }
    if (action === "pause") {
      setStatus("paused")
    } else if (action === "step") {
      setStatus("paused")
    } else {
      setStatus("created")
    }

    try {
      if (action === "step") {
        await advanceLiveFrame({ logAccepted: true, detail: "full" })
        return
      }

      const guard = issueLiveGuard(activeSessionId, action === "reset")
      setLiveControlBusy(true)
      const result = await controlSession(activeSessionId, action)
      const session = result.session
      if (shouldIgnoreLiveResponse(guard, session)) {
        return
      }
      setSessionEpoch(session.session_epoch)
      pushLogs(
        log(
          "info",
          t(locale, "log.serverAccepted", {
            action: actionLabel(locale, action),
            status: statusLabel(locale, session.status),
          }),
        ),
      )

      if (action === "reset") {
        setFrames([])
        setFrameIndex(0)
        setSelectedEntity(null)
        clearArtifacts()
        setPerfStatus("live_unavailable")
        setStatus(session.status)
      } else if (action === "play") {
        setStatus("playing")
      } else if (action === "pause") {
        setStatus("paused")
      }
    } catch (error) {
      setStatus("paused")
      pushLogs(
        log(
          "warn",
          t(locale, "log.serverControlFailed", {
            action: actionLabel(locale, action),
            message: messageOf(error),
          }),
        ),
      )
    } finally {
      setLiveControlBusy(false)
    }
  }

  async function handleArtifactControl(
    action: ControlAction,
    activeSessionId: string,
  ) {
    handleLocalControl(action)

    try {
      const result = await controlSession(activeSessionId, action)
      const session = result.session
      setSessionEpoch(session.session_epoch)
      pushLogs(
        log(
          "info",
          t(locale, "log.serverAccepted", {
            action: actionLabel(locale, action),
            status: statusLabel(locale, session.status),
          }),
        ),
      )

      if (action === "reset" && session.run_id) {
        const nextFrames = await fetchFrames(session.run_id)
        if (nextFrames.length === 0) {
          throw new Error(t(locale, "error.emptyFrames"))
        }
        const nextFinalSnapshot = await fetchFinalSnapshot(session.run_id)
        await loadPerfArtifact(session.run_id)
        setFrames(nextFrames)
        setRunId(session.run_id)
        setManifestArtifact(session.manifest_artifact ?? "manifest.json")
        setFinalSnapshotArtifact(
          session.final_snapshot_artifact ?? "final_snapshot.json",
        )
        setFinalSnapshot(nextFinalSnapshot)
        setFrameIndex(session.current_frame_index)
      }
    } catch (error) {
      setStatus("paused")
      pushLogs(
        log(
          "warn",
          t(locale, "log.serverControlFailed", {
            action: actionLabel(locale, action),
            message: messageOf(error),
          }),
        ),
      )
    }
  }

  async function handleVelocityPerturbationPreview() {
    if (
      !sessionId ||
      !selectedPerturbationTarget ||
      perturbationAvailability.reasonKey !== null ||
      perturbationAvailability.worldRevision == null
    ) {
      return
    }
    const actionId = buildVelocityPerturbationActionId(
      sessionId,
      selectedPerturbationTarget.bodyHandle,
      frameIndex,
    )
    const guard = issuePerturbationGuard(
      actionId,
      sessionId,
      frameIndex,
      sessionEpoch,
      selectedPerturbationTarget.bodyHandle,
      perturbationAvailability.worldRevision,
    )
    setPerturbationBusy("preview")
    setPerturbationCommit(null)
    setPerturbationRequestError(null)
    try {
      const preview = await previewVelocityPerturbation(sessionId, {
        action_id: actionId,
        world_revision: perturbationAvailability.worldRevision,
        session_epoch: sessionEpoch,
        body_handle: selectedPerturbationTarget.bodyHandle,
        frame_index: frameIndex,
        requested_delta: {
          x: parseVelocityDeltaInput(perturbationDeltaX),
          y: parseVelocityDeltaInput(perturbationDeltaY),
        },
        wake_intent: true,
      })
      if (shouldIgnorePerturbationResponse(guard, preview)) {
        return
      }
      setPerturbationPreview(preview)
    } catch (error) {
      if (isCurrentPerturbationGuard(guard)) {
        setPerturbationRequestError(messageOf(error))
      }
    } finally {
      if (isCurrentPerturbationGuard(guard)) {
        setPerturbationBusy("idle")
      }
    }
  }

  async function handleVelocityPerturbationCommit() {
    if (
      !sessionId ||
      !perturbationPreview ||
      perturbationAvailability.reasonKey !== null
    ) {
      return
    }

    const liveGuard = issueLiveGuard(sessionId, true)
    // Submit needs both lanes guarded: the live guard invalidates older frame
    // refreshes, while the perturbation guard blocks late commit responses from
    // reviving stale requestError/busy state after reset/new preview/new submit.
    const perturbationGuard = issuePerturbationGuard(
      perturbationPreview.action_id,
      sessionId,
      perturbationPreview.frame_index,
      perturbationPreview.session_epoch,
      perturbationPreview.body_handle,
      perturbationPreview.world_revision,
    )
    setPerturbationBusy("submit")
    setPerturbationRequestError(null)
    try {
      const result = await commitVelocityPerturbation(sessionId, {
        action_id: perturbationPreview.action_id,
        world_revision: perturbationPreview.world_revision,
        session_epoch: perturbationPreview.session_epoch,
        body_handle: perturbationPreview.body_handle,
        frame_index: perturbationPreview.frame_index,
        requested_delta: perturbationPreview.requested_delta ?? {
          x: parseVelocityDeltaInput(perturbationDeltaX),
          y: parseVelocityDeltaInput(perturbationDeltaY),
        },
        computed_target_velocity:
          perturbationPreview.computed_target_velocity ?? undefined,
      })
      if (shouldIgnorePerturbationCommitResponse(perturbationGuard, result)) {
        return
      }
      setPerturbationCommit(result.commit)
      setSessionEpoch(result.session.session_epoch)
      applyLiveSessionFrame({ session: result.session }, liveGuard)
      if (result.session.status !== "completed") {
        setStatus(result.session.status)
      }
    } catch (error) {
      if (isCurrentPerturbationGuard(perturbationGuard)) {
        setPerturbationRequestError(messageOf(error))
      }
    } finally {
      if (isCurrentPerturbationGuard(perturbationGuard)) {
        setPerturbationBusy("idle")
      }
    }
  }

  function updateLayer(key: keyof LayerState, value: boolean) {
    setLayers((prev) => ({ ...prev, [key]: value }))
  }

  function applyOverlayPreset(preset: OverlayPresetId) {
    const next = overlayPresetState(preset)
    setLayers(next.layers)
    setTrajectorySettings(next.trajectory)
  }

  function clearArtifacts() {
    setRunId(null)
    setManifestArtifact(null)
    setFinalSnapshotArtifact(null)
    setFinalSnapshot(null)
    setPerfArtifact(null)
    setPerfStatus("none")
  }

  function clearRunState() {
    invalidateLiveResponses()
    setSessionId(null)
    setSessionEpoch(0)
    clearArtifacts()
  }

  function pushLogs(...entries: WorkbenchLog[]) {
    setLogs((prev) => [...entries, ...prev].slice(0, 80))
  }

  async function loadPerfArtifact(nextRunId: string) {
    try {
      const nextPerf = await fetchPerf(nextRunId)
      setPerfArtifact(nextPerf)
      setPerfStatus("available")
    } catch {
      setPerfArtifact(null)
      setPerfStatus("missing")
    }
  }

  const perturbationAvailability = useMemo<PerturbationAvailability>(() => {
    const worldRevision =
      latestAuthoritativeLiveFrame?.snapshot.meta.revision ?? null
    if (source !== "live") {
      return {
        reasonKey: "perturbation.liveOnly",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    if (!sessionId) {
      return {
        reasonKey: "perturbation.unavailable",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    if (status === "playing" || status === "running") {
      return {
        reasonKey: "perturbation.runningBlocked",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    if (status === "completed") {
      return {
        reasonKey: "perturbation.completedBlocked",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    if (status === "failed") {
      return {
        reasonKey: "perturbation.failedBlocked",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    if (!latestAuthoritativeLiveFrame || worldRevision == null) {
      return {
        reasonKey: "perturbation.missingFrame",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    if (!isAuthoritativeLiveFrameSelected) {
      return {
        reasonKey: "perturbation.currentFrameOnly",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    if (!selectedPerturbationTarget) {
      return {
        reasonKey: "perturbation.selectDynamicBody",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    if (selectedPerturbationTarget.bodyType !== "dynamic") {
      return {
        reasonKey: "perturbation.selectDynamicBody",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    if (status !== "paused") {
      return {
        reasonKey: "perturbation.pausedOnly",
        latestAuthoritativeLiveFrame,
        isAuthoritativeLiveFrameSelected,
        worldRevision,
      }
    }
    return {
      reasonKey: null,
      latestAuthoritativeLiveFrame,
      isAuthoritativeLiveFrameSelected,
      worldRevision,
    }
  }, [
    isAuthoritativeLiveFrameSelected,
    latestAuthoritativeLiveFrame,
    selectedPerturbationTarget,
    sessionId,
    source,
    status,
  ])

  const perturbationUnavailableReason = perturbationAvailability.reasonKey
    ? t(locale, perturbationAvailability.reasonKey)
    : null

  const velocityPerturbation = useMemo<VelocityPerturbationPanelState>(
    () => ({
      enabled: source === "live",
      target: selectedPerturbationTarget,
      deltaX: perturbationDeltaX,
      deltaY: perturbationDeltaY,
      busy: perturbationBusy,
      unavailableReason: perturbationUnavailableReason,
      requestError: perturbationRequestError,
      sessionEpoch: sessionId ? sessionEpoch : null,
      worldRevision: perturbationAvailability.worldRevision,
      frameIndex,
      preview: perturbationPreview,
      commit: perturbationCommit,
    }),
    [
      frameIndex,
      perturbationAvailability.worldRevision,
      perturbationBusy,
      perturbationCommit,
      perturbationDeltaX,
      perturbationDeltaY,
      perturbationPreview,
      perturbationRequestError,
      perturbationUnavailableReason,
      selectedPerturbationTarget,
      sessionEpoch,
      sessionId,
      source,
    ],
  )

  const currentFramePerturbationProvenance = useMemo(() => {
    const selectedBodyHandle = selectedPerturbationTarget?.bodyHandle ?? null
    const provenance = currentFrame.perturbation_provenance ?? []
    const scoped =
      selectedBodyHandle == null
        ? provenance
        : provenance.filter((entry) => entry.body_handle === selectedBodyHandle)
    return scoped.map((entry) => ({
      actionId: entry.action_id,
      bodyHandle: entry.body_handle,
      frameIndex: entry.frame_index,
      sessionEpoch: entry.session_epoch,
      worldRevision: entry.world_revision,
      requestedDelta: entry.requested_delta,
      targetVelocity: entry.computed_target_velocity,
      commitOutcome: entry.commit_outcome,
    }))
  }, [currentFrame.perturbation_provenance, selectedPerturbationTarget?.bodyHandle])

  const debugContextPayload = useMemo<DebugContextPayload>(
    () => ({
      scenario: {
        id: selectedScenario,
        group: scenarioGroupForId(selectedScenario),
        name: scenario.name,
        description: scenario.description,
      },
      source: {
        kind: source,
        status,
        runMode,
      },
      session: {
        sessionId,
        sessionEpoch: sessionId ? sessionEpoch : null,
        runId,
        frameIndex,
        bufferedFrameCount: frames.length,
        stateHash: currentFrame.state_hash,
        worldRevision: currentFrame.snapshot.meta.revision,
        liveAuthority: {
          latestFrameIndex: latestAuthoritativeLiveFrame?.frame_index ?? null,
          latestStateHash: latestAuthoritativeLiveFrame?.state_hash ?? null,
          latestWorldRevision:
            latestAuthoritativeLiveFrame?.snapshot.meta.revision ?? null,
          viewingLatestAuthoritativeFrame: isAuthoritativeLiveFrameSelected,
        },
      },
      selection: selectedEntity,
      layers: {
        enabled: (Object.keys(layers) as Array<keyof LayerState>).filter(
          (key) => layers[key],
        ),
        values: layers,
      },
      camera: canvasView
        ? {
            mode: canvasView.mode,
            targetDescription: canvasView.targetDescription,
            targetBounds: canvasView.targetBounds,
            dampingEnabled: canvasView.dampingEnabled,
            center: canvasView.center,
            zoom: canvasView.zoom,
            grid: canvasView.grid,
            rulers: canvasView.rulers,
          }
        : null,
      trajectory: {
        settings: trajectorySettings,
        overlay: hasUnhydratedLiveFrames
          ? null
          : {
              mode: trajectoryOverlay.mode,
              windowStart: trajectoryOverlay.windowStart,
              windowEnd: trajectoryOverlay.windowEnd,
              emptyState: trajectoryOverlay.emptyState,
              bodyTrailCount: trajectoryOverlay.bodyTrails.length,
              contactTrailCount: trajectoryOverlay.contactTrails.length,
              ccdTrailCount: trajectoryOverlay.ccdTrails.length,
            },
        summary: hasUnhydratedLiveFrames ? null : trajectorySummary,
        markers: hasUnhydratedLiveFrames
          ? []
          : selectTrajectoryContextMarkers(
              trajectoryMarkers,
              trajectoryOverlay.windowStart,
              trajectoryOverlay.windowEnd,
              frameIndex,
            ).map((marker) => ({
              kind: marker.kind,
              frameIndex: marker.frameIndex,
              score: marker.score,
            })),
        notHydrated: hasUnhydratedLiveFrames,
      },
      stackStability: hasUnhydratedLiveFrames ? null : stackSummary,
      diagnostics: {
        frame: currentFrameNotHydrated ? null : currentFrame.diagnostics ?? null,
        summary: currentFrameNotHydrated
          ? {
              available: false,
              marker_count: null,
              marker_kinds: [],
              missing_evidence: ["live_summary_not_hydrated"],
              counter_delta_source: "missing",
              top_marker_threshold: null,
            }
          : summarizeFrameDiagnostics(currentFrame.diagnostics),
      },
      latticeProxy: {
        enabled: !hasUnhydratedLiveFrames && latticeSummary.enabled,
        nodeCount: hasUnhydratedLiveFrames ? null : latticeSummary.nodes.length,
        edgeCount: hasUnhydratedLiveFrames ? null : latticeSummary.edges.length,
        worldAnchorEdgeCount: hasUnhydratedLiveFrames
          ? null
          : latticeSummary.worldAnchorEdgeCount,
        maxStretchRatio: hasUnhydratedLiveFrames ? null : latticeSummary.maxStretchRatio,
        jointRowCount: hasUnhydratedLiveFrames ? null : latticeSummary.jointRowCount,
        contactCount: hasUnhydratedLiveFrames ? null : latticeSummary.contactCount,
        islandCount: hasUnhydratedLiveFrames ? null : latticeSummary.islandCount,
        activeIslandCount: hasUnhydratedLiveFrames
          ? null
          : latticeSummary.activeIslandCount,
        sleepingIslandCount: hasUnhydratedLiveFrames
          ? null
          : latticeSummary.sleepingIslandCount,
        proxyOnly: true,
        notHydrated: hasUnhydratedLiveFrames,
      },
      perturbation: {
        selectedBodyHandle: selectedPerturbationTarget?.bodyHandle ?? null,
        sessionEpoch: velocityPerturbation.sessionEpoch,
        worldRevision: velocityPerturbation.worldRevision,
        delta: {
          x: perturbationDeltaX,
          y: perturbationDeltaY,
        },
        preview: perturbationPreview
          ? {
              actionId: perturbationPreview.action_id,
              rejectionReason: perturbationPreview.rejection_reason,
              beforeVelocity: perturbationPreview.before_velocity,
              requestedDelta: perturbationPreview.requested_delta,
              targetVelocity: perturbationPreview.computed_target_velocity,
              wakeIntent: perturbationPreview.wake_intent,
              frameIndex: perturbationPreview.frame_index,
              sessionEpoch: perturbationPreview.session_epoch,
              worldRevision: perturbationPreview.world_revision,
            }
          : null,
        commit: perturbationCommit
          ? {
              actionId: perturbationCommit.action_id,
              accepted: perturbationCommit.accepted,
              rejectionReason: perturbationCommit.rejection_reason,
              querySyncStatus: perturbationCommit.query_sync_status,
              targetVelocity: perturbationCommit.computed_target_velocity,
              frameIndex: perturbationCommit.frame_index,
              sessionEpoch: perturbationCommit.session_epoch,
              worldRevision: perturbationCommit.world_revision,
            }
          : null,
        currentFrameProvenance: currentFramePerturbationProvenance,
      },
      perf: perfArtifact
        ? {
            status: perfStatus,
            frameCount: perfArtifact.frame_count,
            finalStateHash: perfArtifact.final_state_hash,
            counterSummary: perfArtifact.counter_summary ?? null,
          }
        : null,
    }),
    [
      canvasView,
      currentFrame,
      currentFramePerturbationProvenance,
      currentFrameNotHydrated,
      frameIndex,
      frames.length,
      hasUnhydratedLiveFrames,
      isAuthoritativeLiveFrameSelected,
      latticeSummary,
      latestAuthoritativeLiveFrame?.frame_index,
      latestAuthoritativeLiveFrame?.snapshot.meta.revision,
      latestAuthoritativeLiveFrame?.state_hash,
      layers,
      perfArtifact,
      perfStatus,
      perturbationCommit,
      perturbationDeltaX,
      perturbationDeltaY,
      perturbationPreview,
      runId,
      runMode,
      scenario.description,
      scenario.name,
      selectedEntity,
      selectedPerturbationTarget?.bodyHandle,
      selectedScenario,
      sessionEpoch,
      sessionId,
      source,
      stackSummary,
      status,
      trajectoryMarkers,
      trajectoryOverlay,
      trajectorySettings,
      trajectorySummary,
      velocityPerturbation.sessionEpoch,
      velocityPerturbation.worldRevision,
    ],
  )

  const debugContextText = useMemo(
    () => JSON.stringify(debugContextPayload, null, 2),
    [debugContextPayload],
  )

  async function copyDebugContext() {
    try {
      await copyTextToClipboard(debugContextText)
      pushLogs(log("info", t(locale, "debug.contextCopied")))
    } catch (error) {
      pushLogs(
        log(
          "warn",
          t(locale, "debug.contextCopyFailed", { message: messageOf(error) }),
        ),
        log("info", t(locale, "debug.contextCopyFallback")),
      )
    }
  }

  return (
    <WorkbenchLayout
      locale={locale}
      onLocaleChange={setLocale}
      scenario={scenario}
      scenarios={scenarios}
      selectedScenario={selectedScenario}
      onScenarioChange={changeScenario}
      status={status}
      source={source}
      sessionId={sessionId}
      runId={runId}
      manifestArtifact={manifestArtifact}
      finalSnapshotArtifact={finalSnapshotArtifact}
      finalSnapshotStep={finalSnapshot?.stats.step_index ?? null}
      perfArtifact={perfArtifact}
      perfStatus={perfStatus}
      onRun={() => void runScenario()}
      liveControlBusy={liveControlBusy}
      liveCadence={liveCadence}
      runMode={runMode}
      onRunModeChange={setRunMode}
      layers={layers}
      onLayerChange={updateLayer}
      onApplyOverlayPreset={applyOverlayPreset}
      trajectorySettings={trajectorySettings}
      onTrajectorySettingsChange={setTrajectorySettings}
      currentFrame={currentFrame}
      frames={frames}
      frameIndex={frameIndex}
      onFrameChange={setFrameIndex}
      selectedEntity={selectedEntity}
      selectedDetails={selectedDetails}
      onSelectEntity={setSelectedEntity}
      canvasView={canvasView}
      debugContextText={debugContextText}
      onCanvasViewChange={setCanvasView}
      logs={logs}
      frameCount={frameCount}
      setFrameCount={setFrameCount}
      useCustomGravity={useCustomGravity}
      setUseCustomGravity={setUseCustomGravity}
      gravityY={gravityY}
      setGravityY={setGravityY}
      velocityPerturbation={velocityPerturbation}
      onVelocityPerturbationDeltaChange={(axis, value) => {
        if (axis === "x") {
          setPerturbationDeltaX(value)
        } else {
          setPerturbationDeltaY(value)
        }
      }}
      onVelocityPerturbationPreview={() =>
        void handleVelocityPerturbationPreview()
      }
      onVelocityPerturbationSubmit={() =>
        void handleVelocityPerturbationCommit()
      }
      onControl={(action) => void handleControl(action)}
      onCopyDebugContext={() => void copyDebugContext()}
    />
  )
}
