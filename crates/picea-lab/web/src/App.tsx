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
  WorkbenchLog,
} from "./types"

export function App() {
  const [locale, setLocale] = useState<Locale>(() => detectInitialLocale())
  const [scenarios, setScenarios] =
    useState<ScenarioDescriptor[]>(demoScenarios)
  const [selectedScenario, setSelectedScenario] = useState(
    "falling_box_contact",
  )
  const [frameCount, setFrameCount] = useState(120)
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

  const currentFrame =
    frames[Math.min(frameIndex, Math.max(0, frames.length - 1))]
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

    playTimer.current = window.setInterval(() => {
      setFrameIndex((next) => {
        if (next >= frames.length - 1) {
          setStatus("paused")
          return next
        }
        return next + 1
      })
    }, 1000 / 30)

    return () => {
      if (playTimer.current !== null) {
        window.clearInterval(playTimer.current)
        playTimer.current = null
      }
    }
  }, [frames.length, status])

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
          if (frameIndex >= frames.length - 1 && source === "server") {
            void handleControl("step")
          } else {
            setStatus("paused")
            setFrameIndex((i) => Math.min(frames.length - 1, i + 1))
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
  }, [status, frameIndex, frames.length, source, sessionId]) // eslint-disable-line react-hooks/exhaustive-deps

  const selectedDetails = useMemo(
    () => resolveSelection(currentFrame, selectedEntity),
    [currentFrame, selectedEntity],
  )

  async function runScenario() {
    setStatus("loading")
    setFrameIndex(0)
    setSelectedEntity(null)
    const gravity = useCustomGravity
      ? ([0, gravityY] as [number, number])
      : null

    try {
      const session = await createSession(selectedScenario, frameCount, gravity)
      if (!session.run_id) {
        throw new Error(t(locale, "error.sessionWithoutRun"))
      }
      const completedRunId = session.run_id
      const nextFrames = await fetchFrames(completedRunId)
      if (nextFrames.length === 0) {
        throw new Error(t(locale, "error.emptyFrames"))
      }
      const nextFinalSnapshot = await fetchFinalSnapshot(completedRunId)
      setSessionId(session.id)
      setRunId(completedRunId)
      setManifestArtifact(session.manifest_artifact ?? "manifest.json")
      setFinalSnapshotArtifact(
        session.final_snapshot_artifact ?? "final_snapshot.json",
      )
      setFinalSnapshot(nextFinalSnapshot)
      setFrames(nextFrames)
      setSource("server")
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
      subscribeToEvents(session.id)
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
    setSelectedScenario(nextScenario)
    setFrames(makeDemoFrames(nextScenario, frameCount))
    setFrameIndex(0)
    setSelectedEntity(null)
    clearRunState()
    setSource("demo")
  }

  async function handleControl(action: ControlAction) {
    if (action === "pause") {
      setStatus("paused")
    } else if (action === "play") {
      setStatus("playing")
    } else if (action === "step") {
      setStatus("paused")
      setFrameIndex((value) => Math.min(frames.length - 1, value + 1))
    } else {
      setStatus("paused")
      setFrameIndex(0)
    }

    if (source !== "server" || !sessionId) {
      return
    }

    try {
      const session = await controlSession(sessionId, action)
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
      if (session.run_id && action === "reset") {
        const nextFrames = await fetchFrames(session.run_id)
        if (nextFrames.length > 0) {
          setFrames(nextFrames)
          setRunId(session.run_id)
          setFrameIndex(session.current_frame_index)
        }
      }
    } catch (error) {
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

  function clearRunState() {
    setSessionId(null)
    setRunId(null)
    setManifestArtifact(null)
    setFinalSnapshotArtifact(null)
    setFinalSnapshot(null)
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
