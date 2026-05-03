import { useEffect, useMemo, useRef, useState } from "react"

import {
  controlSession,
  createSession,
  fetchFinalSnapshot,
  fetchFrames,
  fetchScenarios,
  openSessionEvents,
} from "./api"
import { WorkbenchLayout } from "./components/workbench/WorkbenchLayout"
import { log, messageOf } from "./components/workbench/format"
import { resolveSelection } from "./components/workbench/selection"
import {
  defaultLayers,
  type ControlAction,
  type LayerState,
  type RunMode,
  type SourceKind,
} from "./components/workbench/types"
import { demoScenarios, makeDemoFrames } from "./demo"
import {
  actionLabel,
  detectInitialLocale,
  localizeScenario,
  statusLabel,
  storeLocale,
  t,
  type Locale,
  type StatusKind,
} from "./i18n"
import type {
  DebugSnapshot,
  FrameRecord,
  ScenarioDescriptor,
  SelectedEntity,
  SessionRecord,
  WorkbenchLog,
} from "./types"

const EMPTY_FRAME: FrameRecord = {
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
}

type LiveResponseGuard = {
  generation: number
  token: number
  sessionId: string
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
  const [source, setSource] = useState<SourceKind>("demo")
  const [status, setStatus] = useState<StatusKind>("idle")
  const [logs, setLogs] = useState<WorkbenchLog[]>([
    log("warn", t(locale, "log.serverNotConfirmed")),
  ])
  const [layers, setLayers] = useState<LayerState>(defaultLayers)
  const [useCustomGravity, setUseCustomGravity] = useState(false)
  const [gravityY, setGravityY] = useState(9.8)
  const playTimer = useRef<number | null>(null)
  const liveStepInFlight = useRef(false)
  const liveGenerationRef = useRef(0)
  const liveRequestTokenRef = useRef(0)

  const currentFrame =
    frames[Math.min(frameIndex, Math.max(0, frames.length - 1))] ?? EMPTY_FRAME
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
    if (status !== "playing") {
      if (playTimer.current !== null) {
        window.clearInterval(playTimer.current)
        playTimer.current = null
      }
      return
    }

    if (source === "live") {
      playTimer.current = window.setInterval(() => {
        if (liveStepInFlight.current) {
          return
        }
        liveStepInFlight.current = true
        void advanceLiveFrame().finally(() => {
          liveStepInFlight.current = false
        })
      }, 1000 / 30)
    } else {
      playTimer.current = window.setInterval(() => {
        setFrameIndex((next) => {
          if (next >= frames.length - 1) {
            setStatus("paused")
            return next
          }
          return next + 1
        })
      }, 1000 / 30)
    }

    return () => {
      if (playTimer.current !== null) {
        window.clearInterval(playTimer.current)
        playTimer.current = null
      }
    }
  }, [frames.length, source, status]) // eslint-disable-line react-hooks/exhaustive-deps

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

  const selectedDetails = useMemo(
    () => resolveSelection(currentFrame, selectedEntity),
    [currentFrame, selectedEntity],
  )

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
      subscribeToEvents(session.id)

      if (session.mode === "live_session") {
        activateLiveSession(session.id)
        clearArtifacts()
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

  async function advanceLiveFrame() {
    if (!sessionId) {
      return
    }
    const guard = issueLiveGuard(sessionId)
    try {
      const session = await controlSession(sessionId, "step")
      applyLiveSessionFrame(session, guard)
      pushLogs(
        log(
          "info",
          t(locale, "log.serverAccepted", {
            action: actionLabel(locale, "step"),
            status: statusLabel(locale, session.status),
          }),
        ),
      )
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
    }
  }

  function applyLiveSessionFrame(
    session: SessionRecord,
    guard: LiveResponseGuard,
  ) {
    if (shouldIgnoreLiveResponse(guard, session)) {
      return
    }
    const nextFrame = session.latest_frame
    if (nextFrame) {
      setFrames((prev) => {
        if (prev[nextFrame.frame_index]?.state_hash === nextFrame.state_hash) {
          return prev
        }
        const next = prev.slice(0, nextFrame.frame_index)
        next.push(nextFrame)
        return next
      })
      setFrameIndex(nextFrame.frame_index)
      pushLogs(
        log(
          "info",
          t(locale, "log.liveFrameBuffered", { frameIndex: nextFrame.frame_index }),
        ),
      )
    }
    if (session.status === "completed") {
      setStatus("paused")
    } else if (status !== "playing") {
      setStatus(session.status)
    }
  }

  function activateLiveSession(nextSessionId: string) {
    invalidateLiveResponses()
    setSessionId(nextSessionId)
  }

  function invalidateLiveResponses() {
    liveGenerationRef.current += 1
    liveRequestTokenRef.current = 0
    liveStepInFlight.current = false
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
    if (action === "play") {
      setStatus("playing")
    } else if (action === "pause") {
      setStatus("paused")
    } else if (action === "step") {
      setStatus("paused")
    } else {
      setStatus("created")
    }

    try {
      if (action === "step") {
        await advanceLiveFrame()
        return
      }

      const guard = issueLiveGuard(activeSessionId, action === "reset")
      const session = await controlSession(activeSessionId, action)
      if (shouldIgnoreLiveResponse(guard, session)) {
        return
      }
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
        clearArtifacts()
        setStatus(session.status)
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
    }
  }

  async function handleArtifactControl(
    action: ControlAction,
    activeSessionId: string,
  ) {
    handleLocalControl(action)

    try {
      const session = await controlSession(activeSessionId, action)
      pushLogs(
        log(
          "info",
          t(locale, "log.serverAccepted", {
            action: actionLabel(locale, action),
            status: statusLabel(locale, session.status),
          }),
        ),
      )

      setFrameIndex(
        Math.min(session.current_frame_index, Math.max(0, frames.length - 1)),
      )

      if (action === "reset" && session.run_id) {
        const nextFrames = await fetchFrames(session.run_id)
        if (nextFrames.length === 0) {
          throw new Error(t(locale, "error.emptyFrames"))
        }
        const nextFinalSnapshot = await fetchFinalSnapshot(session.run_id)
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

  function updateLayer(key: keyof LayerState, value: boolean) {
    setLayers((prev) => ({ ...prev, [key]: value }))
  }

  function clearArtifacts() {
    setRunId(null)
    setManifestArtifact(null)
    setFinalSnapshotArtifact(null)
    setFinalSnapshot(null)
  }

  function clearRunState() {
    invalidateLiveResponses()
    setSessionId(null)
    clearArtifacts()
  }

  function pushLogs(...entries: WorkbenchLog[]) {
    setLogs((prev) => [...entries, ...prev].slice(0, 80))
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
      onRun={() => void runScenario()}
      runMode={runMode}
      onRunModeChange={setRunMode}
      layers={layers}
      onLayerChange={updateLayer}
      currentFrame={currentFrame}
      frames={frames}
      frameIndex={frameIndex}
      onFrameChange={setFrameIndex}
      selectedEntity={selectedEntity}
      selectedDetails={selectedDetails}
      onSelectEntity={setSelectedEntity}
      logs={logs}
      frameCount={frameCount}
      setFrameCount={setFrameCount}
      useCustomGravity={useCustomGravity}
      setUseCustomGravity={setUseCustomGravity}
      gravityY={gravityY}
      setGravityY={setGravityY}
      onControl={(action) => void handleControl(action)}
    />
  )
}
