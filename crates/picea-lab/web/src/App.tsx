import { useEffect, useMemo, useRef, useState, type ReactNode } from "react"
import * as DropdownMenu from "@radix-ui/react-dropdown-menu"
import * as Tabs from "@radix-ui/react-tabs"
import {
  Activity,
  Box,
  Braces,
  Check,
  ChevronRight,
  CircleDot,
  Copy,
  Gauge,
  Languages,
  Layers,
  MousePointer2,
  Pause,
  Play,
  RotateCcw,
  Settings2,
  SkipForward,
  Square,
  Waypoints,
} from "lucide-react"
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels"
import {
  controlSession,
  createSession,
  fetchFinalSnapshot,
  fetchFrames,
  fetchScenarios,
  openSessionEvents,
} from "./api"
import { WorldCanvas } from "./components/workbench/WorldCanvas"
import { Badge } from "./components/ui/badge"
import { Button } from "./components/ui/button"
import { Input } from "./components/ui/input"
import { PanelHeader, PanelTitle } from "./components/ui/panel"
import { Checkbox, Select, Slider, Tooltip } from "./components/ui/radix"
import { demoScenarios, makeDemoFrames } from "./demo"
import {
  actionLabel,
  bodyTypeLabel,
  booleanLabel,
  detectInitialLocale,
  dynamicValueLabel,
  entityKindLabel,
  entityLabel,
  layerLabel,
  localeLabels,
  localizeScenario,
  sourceLabel,
  statusLabel,
  storeLocale,
  t,
  type EntityKind,
  type LayerKey,
  type Locale,
  type StatusKind,
} from "./i18n"
import type {
  DebugBody,
  DebugCollider,
  DebugSnapshot,
  FrameRecord,
  ScenarioDescriptor,
  SelectedEntity,
  WorkbenchLog,
} from "./types"
import { cn } from "./lib/utils"

function warmStartTriplet(stats: FrameRecord["snapshot"]["stats"]) {
  return `${stats.warm_start_hit_count ?? 0}/${stats.warm_start_miss_count ?? 0}/${stats.warm_start_drop_count ?? 0}`
}

function ccdQuad(stats: FrameRecord["snapshot"]["stats"]) {
  return `${stats.ccd_candidate_count ?? 0}/${stats.ccd_hit_count ?? 0}/${stats.ccd_miss_count ?? 0}/${stats.ccd_clamp_count ?? 0}`
}

function traceStage(
  locale: Locale,
  termination: string,
  iterations: number,
): string {
  return `${dynamicValueLabel(locale, termination)} / ${iterations}`
}

type LayerState = {
  shapes: boolean
  aabbs: boolean
  contacts: boolean
  velocities: boolean
  trace: boolean
}

const defaultLayers: LayerState = {
  shapes: true,
  aabbs: true,
  contacts: true,
  velocities: true,
  trace: true,
}

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
  const [source, setSource] = useState<"server" | "demo">("demo")
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
        setLogs((prev) =>
          [log("info", t(locale, "log.connectedScenarios")), ...prev].slice(
            0,
            80,
          ),
        )
      })
      .catch((error: Error) => {
        if (cancelled) {
          return
        }
        setSource("demo")
        setLogs((prev) =>
          [
            log(
              "warn",
              t(locale, "log.serverUnavailable", { message: error.message }),
            ),
            ...prev,
          ].slice(0, 80),
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
      setLogs((prev) =>
        [
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
          ...prev,
        ].slice(0, 80),
      )
      subscribeToEvents(session.id)
    } catch (error) {
      const nextFrames = makeDemoFrames(selectedScenario, frameCount)
      setFrames(nextFrames)
      setSessionId(null)
      setRunId(null)
      setManifestArtifact(null)
      setFinalSnapshotArtifact(null)
      setFinalSnapshot(null)
      setSource("demo")
      setStatus("paused")
      setLogs((prev) =>
        [
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
          ...prev,
        ].slice(0, 80),
      )
    }
  }

  function subscribeToEvents(nextSessionId: string) {
    try {
      const events = openSessionEvents(nextSessionId)
      events.addEventListener("frame", (event) => {
        setLogs((prev) =>
          [
            log("info", t(locale, "log.sseFrame", { data: event.data })),
            ...prev,
          ].slice(0, 80),
        )
      })
      events.addEventListener("failed", (event) => {
        setLogs((prev) =>
          [
            log("error", t(locale, "log.sseFailed", { data: event.data })),
            ...prev,
          ].slice(0, 80),
        )
      })
      events.addEventListener("idle", (event) => {
        setLogs((prev) =>
          [
            log("info", t(locale, "log.sseIdle", { data: event.data })),
            ...prev,
          ].slice(0, 80),
        )
      })
      window.setTimeout(() => events.close(), 2000)
    } catch (error) {
      setLogs((prev) =>
        [
          log(
            "warn",
            t(locale, "log.sseUnavailable", { message: messageOf(error) }),
          ),
          ...prev,
        ].slice(0, 80),
      )
    }
  }

  function changeScenario(nextScenario: string) {
    setSelectedScenario(nextScenario)
    setFrames(makeDemoFrames(nextScenario, frameCount))
    setFrameIndex(0)
    setSelectedEntity(null)
    setSessionId(null)
    setRunId(null)
    setManifestArtifact(null)
    setFinalSnapshotArtifact(null)
    setFinalSnapshot(null)
    setSource("demo")
  }

  async function handleControl(action: "play" | "pause" | "step" | "reset") {
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
      setLogs((prev) =>
        [
          log(
            "info",
            t(locale, "log.serverAccepted", {
              action: actionLabel(locale, action),
              status: statusLabel(locale, session.status),
            }),
          ),
          ...prev,
        ].slice(0, 80),
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
      setLogs((prev) =>
        [
          log(
            "warn",
            t(locale, "log.serverControlFailed", {
              action: actionLabel(locale, action),
              message: messageOf(error),
            }),
          ),
          ...prev,
        ].slice(0, 80),
      )
    }
  }

  function updateLayer(key: keyof LayerState, value: boolean) {
    setLayers((prev) => ({ ...prev, [key]: value }))
  }

  return (
    <div className="flex h-screen min-h-[720px] flex-col overflow-hidden bg-lab-canvas text-lab-text">
      <Toolbar
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
        onRun={runScenario}
        layers={layers}
        onLayerChange={updateLayer}
      />

      <PanelGroup direction="horizontal" className="min-h-0 flex-1">
        <Panel
          defaultSize={20}
          minSize={16}
          maxSize={30}
          className="min-w-[240px] border-r border-lab-line bg-lab-panel"
        >
          <SceneHierarchy
            frame={currentFrame}
            selected={selectedEntity}
            onSelect={setSelectedEntity}
            locale={locale}
          />
        </Panel>
        <ResizeHandle />
        <Panel defaultSize={56} minSize={35} className="min-w-[420px]">
          <PanelGroup direction="vertical">
            <Panel defaultSize={72} minSize={45} className="min-h-[320px]">
              <WorldCanvas
                frames={frames}
                frameIndex={frameIndex}
                selected={selectedEntity}
                layers={layers}
                labels={{
                  frame: t(locale, "canvas.frame"),
                  colliders: t(locale, "canvas.colliders"),
                  contacts: t(locale, "canvas.contacts"),
                }}
                onSelect={setSelectedEntity}
              />
            </Panel>
            <ResizeHandle vertical />
            <Panel
              defaultSize={28}
              minSize={18}
              className="min-h-[180px] border-t border-lab-line bg-lab-panel"
            >
              <BottomTimeline
                frames={frames}
                frameIndex={frameIndex}
                onFrameChange={setFrameIndex}
                logs={logs}
                frameCount={frameCount}
                setFrameCount={setFrameCount}
                useCustomGravity={useCustomGravity}
                setUseCustomGravity={setUseCustomGravity}
                gravityY={gravityY}
                setGravityY={setGravityY}
                locale={locale}
                onPlay={() => void handleControl("play")}
                onPause={() => void handleControl("pause")}
                onStep={() => void handleControl("step")}
                onReset={() => void handleControl("reset")}
              />
            </Panel>
          </PanelGroup>
        </Panel>
        <ResizeHandle />
        <Panel
          defaultSize={24}
          minSize={18}
          maxSize={34}
          className="min-w-[280px] border-l border-lab-line bg-lab-panel"
        >
          <Inspector
            frame={currentFrame}
            selected={selectedDetails}
            locale={locale}
          />
        </Panel>
      </PanelGroup>
    </div>
  )
}

function Toolbar({
  locale,
  onLocaleChange,
  scenario,
  scenarios,
  selectedScenario,
  onScenarioChange,
  status,
  source,
  sessionId,
  runId,
  manifestArtifact,
  finalSnapshotArtifact,
  finalSnapshotStep,
  onRun,
  layers,
  onLayerChange,
}: {
  locale: Locale
  onLocaleChange: (locale: Locale) => void
  scenario: ScenarioDescriptor
  scenarios: ScenarioDescriptor[]
  selectedScenario: string
  onScenarioChange: (value: string) => void
  status: StatusKind
  source: "server" | "demo"
  sessionId: string | null
  runId: string | null
  manifestArtifact: string | null
  finalSnapshotArtifact: string | null
  finalSnapshotStep: number | null
  onRun: () => void
  layers: LayerState
  onLayerChange: (key: keyof LayerState, value: boolean) => void
}) {
  return (
    <header className="flex h-12 shrink-0 items-center gap-2 border-b border-lab-line bg-lab-panel px-3">
      <div className="flex min-w-0 flex-1 items-center gap-3">
        <div className="flex items-center gap-2">
          <Activity className="h-5 w-5 text-lab-accent" />
          <div className="leading-tight">
            <div className="text-sm font-semibold text-lab-text">
              {t(locale, "app.name")}
            </div>
            <div className="truncate text-[11px] text-lab-muted">
              {scenario.description}
            </div>
          </div>
        </div>
        <Select
          value={selectedScenario}
          onValueChange={onScenarioChange}
          items={scenarios.map((entry) => ({
            value: entry.id,
            label: localizeScenario(locale, entry).name,
          }))}
          className="ml-2 w-52"
        />
        <Badge tone={source === "server" ? "green" : "warn"}>
          {sourceLabel(locale, source)}
        </Badge>
        <Badge
          tone={
            status === "failed"
              ? "danger"
              : status === "playing"
                ? "accent"
                : "neutral"
          }
        >
          {statusLabel(locale, status)}
        </Badge>
      </div>

      <div className="flex items-center gap-1">
        <Tooltip label={t(locale, "tooltip.runScenario")}>
          <Button size="icon" onClick={onRun} disabled={status === "loading"}>
            <Play className="h-4 w-4" />
          </Button>
        </Tooltip>
        <LayerMenu
          locale={locale}
          layers={layers}
          onLayerChange={onLayerChange}
        />
        <LanguageMenu locale={locale} onLocaleChange={onLocaleChange} />
      </div>

      <div className="hidden min-w-[260px] flex-col text-right text-[11px] text-lab-muted xl:flex">
        <span>
          {t(locale, "app.session")}: {sessionId ?? t(locale, "app.noSession")}
        </span>
        <span>
          {t(locale, "app.runArtifact")}:{" "}
          {runId ?? t(locale, "app.noRunArtifact")}
        </span>
        <span>
          {t(locale, "app.manifestArtifact")}:{" "}
          {manifestArtifact ?? t(locale, "app.noRunArtifact")}
        </span>
        <span>
          {t(locale, "app.finalSnapshot")}:{" "}
          {finalSnapshotArtifact ?? t(locale, "app.noRunArtifact")} /{" "}
          {finalSnapshotStep ?? t(locale, "common.unknown")}
        </span>
      </div>
    </header>
  )
}

function LanguageMenu({
  locale,
  onLocaleChange,
}: {
  locale: Locale
  onLocaleChange: (locale: Locale) => void
}) {
  const localeOptions: Locale[] = ["zh-CN", "en-US"]
  const triggerRef = useRef<HTMLButtonElement | null>(null)

  function handleCloseAutoFocus(event: Event) {
    event.preventDefault()
    triggerRef.current?.blur()
  }

  return (
    <DropdownMenu.Root>
      <Tooltip label={t(locale, "app.language")}>
        <DropdownMenu.Trigger asChild>
          <Button
            ref={triggerRef}
            size="icon"
            variant="outline"
            aria-label={t(locale, "app.language")}
            className="ml-1"
          >
            <Languages className="h-4 w-4" />
          </Button>
        </DropdownMenu.Trigger>
      </Tooltip>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          onCloseAutoFocus={handleCloseAutoFocus}
          className="z-50 w-36 rounded-md border border-lab-line bg-lab-panel2 p-2 text-sm text-lab-text shadow-xl"
        >
          {localeOptions.map((option) => (
            <DropdownMenu.Item
              key={option}
              onSelect={() => onLocaleChange(option)}
              className="flex h-7 cursor-default select-none items-center gap-2 rounded px-2 outline-none data-[highlighted]:bg-lab-accent/[0.18]"
            >
              <span className="flex h-4 w-4 items-center justify-center">
                {locale === option ? (
                  <Check className="h-3.5 w-3.5 text-lab-accent" />
                ) : null}
              </span>
              <span>{localeLabels[option]}</span>
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

function LayerMenu({
  locale,
  layers,
  onLayerChange,
}: {
  locale: Locale
  layers: LayerState
  onLayerChange: (key: keyof LayerState, value: boolean) => void
}) {
  const [open, setOpen] = useState(false)
  const closeTimer = useRef<number | null>(null)
  const triggerRef = useRef<HTMLButtonElement | null>(null)

  function clearCloseTimer() {
    if (closeTimer.current !== null) {
      window.clearTimeout(closeTimer.current)
      closeTimer.current = null
    }
  }

  function scheduleClose() {
    clearCloseTimer()
    closeTimer.current = window.setTimeout(() => {
      setOpen(false)
      closeTimer.current = null
    }, 1200)
  }

  function handleOpenChange(nextOpen: boolean) {
    clearCloseTimer()
    setOpen(nextOpen)
  }

  function handleCloseAutoFocus(event: Event) {
    event.preventDefault()
    triggerRef.current?.blur()
  }

  useEffect(() => clearCloseTimer, [])

  return (
    <DropdownMenu.Root open={open} onOpenChange={handleOpenChange}>
      <Tooltip label={t(locale, "tooltip.canvasLayers")}>
        <DropdownMenu.Trigger asChild>
          <Button ref={triggerRef} size="icon" variant="outline">
            <Layers className="h-4 w-4" />
          </Button>
        </DropdownMenu.Trigger>
      </Tooltip>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          onPointerEnter={clearCloseTimer}
          onPointerLeave={scheduleClose}
          onFocusCapture={clearCloseTimer}
          onBlurCapture={scheduleClose}
          onCloseAutoFocus={handleCloseAutoFocus}
          className="z-50 w-52 rounded-md border border-lab-line bg-lab-panel2 p-2 text-sm text-lab-text shadow-xl"
        >
          {(Object.keys(layers) as Array<LayerKey>).map((key) => (
            <DropdownMenu.CheckboxItem
              key={key}
              checked={layers[key]}
              onSelect={(event) => event.preventDefault()}
              onCheckedChange={(value) => onLayerChange(key, value)}
              className="flex h-7 cursor-default select-none items-center gap-2 rounded px-2 outline-none data-[highlighted]:bg-lab-accent/[0.18]"
            >
              <DropdownMenu.ItemIndicator>
                <ChevronRight className="h-3.5 w-3.5 rotate-90 text-lab-accent" />
              </DropdownMenu.ItemIndicator>
              <span>{layerLabel(locale, key)}</span>
            </DropdownMenu.CheckboxItem>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

function SceneHierarchy({
  frame,
  selected,
  onSelect,
  locale,
}: {
  frame: FrameRecord
  selected: SelectedEntity | null
  onSelect: (entity: SelectedEntity) => void
  locale: Locale
}) {
  const collidersByBody = useMemo(() => {
    const groups = new Map<number, DebugCollider[]>()
    for (const collider of frame.snapshot.colliders) {
      const bodyColliders = groups.get(collider.body) ?? []
      bodyColliders.push(collider)
      groups.set(collider.body, bodyColliders)
    }
    return groups
  }, [frame.snapshot.colliders])

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PanelHeader>
        <PanelTitle>{t(locale, "panel.sceneHierarchy")}</PanelTitle>
        <Badge className="tabular-nums">
          {frame.snapshot.stats.step_index}
        </Badge>
      </PanelHeader>
      <div className="min-h-0 flex-1 overflow-auto p-2">
        <TreeGroup
          icon={<Box className="h-4 w-4" />}
          label={t(locale, "tree.bodies")}
          count={frame.snapshot.bodies.length}
          locale={locale}
        >
          {frame.snapshot.bodies.map((body) => {
            const bodyColliders = collidersByBody.get(body.handle) ?? []
            const selectedOwnedCollider =
              selected?.kind === "collider" &&
              bodyColliders.some((collider) => collider.handle === selected.id)
            return (
              <div key={body.handle} className="mb-1">
                <TreeRow
                  active={
                    selected?.kind === "body" && selected.id === body.handle
                  }
                  ancestorActive={selectedOwnedCollider}
                  icon={<Square className="h-3.5 w-3.5" />}
                  label={entityLabel(locale, "body", body.handle)}
                  meta={
                    <BodyTypePill locale={locale} bodyType={body.body_type} />
                  }
                  onClick={() => onSelect({ kind: "body", id: body.handle })}
                />
                <div className="ml-[17px] border-l border-lab-line/40 pl-1">
                  {bodyColliders.length > 0 ? (
                    bodyColliders.map((collider) => (
                      <TreeRow
                        key={collider.handle}
                        active={
                          selected?.kind === "collider" &&
                          selected.id === collider.handle
                        }
                        level={1}
                        icon={<CircleDot className="h-3.5 w-3.5" />}
                        label={entityLabel(locale, "collider", collider.handle)}
                        meta={dynamicValueLabel(locale, collider.shape.kind)}
                        onClick={() =>
                          onSelect({ kind: "collider", id: collider.handle })
                        }
                      />
                    ))
                  ) : (
                    <div className="px-2 py-1 text-xs text-lab-muted">
                      {t(locale, "tree.empty")}
                    </div>
                  )}
                </div>
              </div>
            )
          })}
        </TreeGroup>
        <TreeGroup
          icon={<Waypoints className="h-4 w-4" />}
          label={t(locale, "tree.contacts")}
          count={frame.snapshot.contacts.length}
          locale={locale}
        >
          {frame.snapshot.contacts.map((contact) => (
            <TreeRow
              key={contact.id}
              active={
                selected?.kind === "contact" && selected.id === contact.id
              }
              icon={<MousePointer2 className="h-3.5 w-3.5" />}
              label={entityLabel(locale, "contact", contact.id)}
              meta={`${t(locale, "fact.depth")} ${contact.depth.toFixed(3)}`}
              onClick={() => onSelect({ kind: "contact", id: contact.id })}
            />
          ))}
        </TreeGroup>
        <TreeGroup
          icon={<Braces className="h-4 w-4" />}
          label={t(locale, "tree.joints")}
          count={frame.snapshot.joints.length}
          locale={locale}
        >
          {frame.snapshot.joints.map((joint) => (
            <TreeRow
              key={joint.handle}
              active={
                selected?.kind === "joint" && selected.id === joint.handle
              }
              icon={<Settings2 className="h-3.5 w-3.5" />}
              label={entityLabel(locale, "joint", joint.handle)}
              meta={dynamicValueLabel(locale, joint.kind)}
              onClick={() => onSelect({ kind: "joint", id: joint.handle })}
            />
          ))}
        </TreeGroup>
      </div>
    </div>
  )
}

function TreeGroup({
  icon,
  label,
  count,
  children,
  locale,
}: {
  icon: ReactNode
  label: string
  count: number
  children: ReactNode
  locale: Locale
}) {
  return (
    <section className="mb-3">
      <div className="mb-1 flex h-7 items-center gap-2 rounded px-1.5 text-xs font-semibold uppercase tracking-normal text-lab-muted">
        {icon}
        <span className="flex-1">{label}</span>
        <span className="tabular-nums">{count}</span>
      </div>
      <div className="space-y-0.5">
        {count > 0 ? (
          children
        ) : (
          <div className="px-8 py-1 text-xs text-lab-muted">
            {t(locale, "tree.empty")}
          </div>
        )}
      </div>
    </section>
  )
}

function TreeRow({
  active,
  ancestorActive = false,
  icon,
  label,
  meta,
  level = 0,
  onClick,
}: {
  active: boolean
  ancestorActive?: boolean
  icon: ReactNode
  label: string
  meta: ReactNode
  level?: 0 | 1
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full items-center gap-2 rounded-r px-2 text-left transition-colors border-l-2",
        level === 0 ? "h-7 text-sm" : "h-6 text-[13px] my-0.5",
        active
          ? "border-lab-accent bg-lab-accent/10 text-lab-text"
          : ancestorActive
            ? "border-transparent text-lab-text hover:bg-white/5"
            : "border-transparent text-lab-muted hover:bg-white/5 hover:text-lab-text",
      )}
    >
      <span className={cn("shrink-0", level === 1 && "text-lab-muted")}>
        {icon}
      </span>
      <span className="min-w-0 flex-1 truncate">{label}</span>
      <span className="shrink-0 truncate text-[11px]">{meta}</span>
    </button>
  )
}

function BodyTypePill({
  locale,
  bodyType,
}: {
  locale: Locale
  bodyType: DebugBody["body_type"]
}) {
  return (
    <span
      className={cn(
        "rounded border px-1.5 py-0.5 text-[10px] font-semibold leading-none",
        bodyType === "dynamic" &&
          "border-lab-accent/50 bg-lab-accent/15 text-lab-accent",
        bodyType === "static" &&
          "border-lab-line bg-white/[0.04] text-lab-muted",
        bodyType === "kinematic" &&
          "border-amber-400/45 bg-amber-400/10 text-amber-200",
      )}
    >
      {bodyTypeLabel(locale, bodyType)}
    </span>
  )
}

function Inspector({
  frame,
  selected,
  locale,
}: {
  frame: FrameRecord
  locale: Locale
  selected:
    | { kind: "body"; entity: DebugBody }
    | { kind: "collider"; entity: DebugCollider; body?: DebugBody }
    | { kind: "contact"; entity: FrameRecord["snapshot"]["contacts"][number] }
    | { kind: "joint"; entity: FrameRecord["snapshot"]["joints"][number] }
    | null
}) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PanelHeader>
        <PanelTitle>{t(locale, "panel.inspector")}</PanelTitle>
        <Badge tone="warn">{t(locale, "panel.firstSliceFacts")}</Badge>
      </PanelHeader>
      <div className="min-h-0 flex-1 overflow-auto">
        <FrameSummaryStrip frame={frame} locale={locale} />
        <div className="space-y-3 px-3 pb-4 pt-3">
          {selected ? (
            <EntityInspector selected={selected} locale={locale} />
          ) : (
            <EmptyInspector locale={locale} />
          )}
          <StageFactsCard frame={frame} locale={locale} />
          <PendingMeasurementsCard locale={locale} />
        </div>
      </div>
    </div>
  )
}

function FrameSummaryStrip({
  frame,
  locale,
}: {
  frame: FrameRecord
  locale: Locale
}) {
  return (
    <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1 border-b border-lab-line/60 bg-lab-panel/95 px-3 py-2 text-[11px]">
      <SummaryItem
        label={t(locale, "metric.bodies")}
        value={frame.snapshot.bodies.length}
      />
      <SummaryItem
        label={t(locale, "metric.contacts")}
        value={frame.snapshot.contacts.length}
      />
      <SummaryItem
        label={t(locale, "metric.dt")}
        value={frame.snapshot.meta.dt.toFixed(4)}
      />
    </div>
  )
}

function SummaryItem({
  label,
  value,
}: {
  label: string
  value: string | number
}) {
  return (
    <span className="inline-flex items-baseline gap-1.5">
      <span className="uppercase tracking-normal text-lab-muted">{label}</span>
      <span className="font-mono tabular-nums text-lab-text">{value}</span>
    </span>
  )
}

function StageFactsCard({
  frame,
  locale,
}: {
  frame: FrameRecord
  locale: Locale
}) {
  return (
    <section className="rounded-md border border-lab-line bg-lab-panel2/70 p-3">
      <div className="mb-2 flex items-center gap-2">
        <Layers className="h-3.5 w-3.5 text-lab-accent" />
        <h3 className="flex-1 text-[11px] font-semibold uppercase tracking-wider text-lab-muted">
          {t(locale, "inspector.stageFacts")}
        </h3>
        <Badge tone="accent" className="tabular-nums">
          {frame.snapshot.stats.step_index}
        </Badge>
      </div>
      <div className="space-y-px">
        <Fact
          label={t(locale, "inspector.broadphaseCandidates")}
          value={String(frame.snapshot.stats.broadphase_candidate_count)}
        />
        <Fact
          label={t(locale, "inspector.warmStart")}
          value={warmStartTriplet(frame.snapshot.stats)}
        />
        <Fact
          label={t(locale, "inspector.ccd")}
          value={ccdQuad(frame.snapshot.stats)}
        />
      </div>
    </section>
  )
}

function PendingMeasurementsCard({ locale }: { locale: Locale }) {
  const placeholder = t(locale, "inspector.unmeasured")
  return (
    <section className="rounded-md border border-dashed border-lab-warn/35 bg-lab-warn/[0.04] p-3">
      <div className="mb-2 flex items-center gap-2">
        <Gauge className="h-3.5 w-3.5 text-lab-warn" />
        <h3 className="flex-1 text-[11px] font-semibold uppercase tracking-wider text-lab-muted">
          {t(locale, "inspector.pendingMeasurements")}
        </h3>
        <Badge tone="warn">{placeholder}</Badge>
      </div>
      <div className="space-y-px">
        <Fact label={t(locale, "inspector.forces")} value={placeholder} muted />
        <Fact
          label={t(locale, "inspector.torques")}
          value={placeholder}
          muted
        />
      </div>
    </section>
  )
}

function EmptyInspector({ locale }: { locale: Locale }) {
  return (
    <section className="flex flex-col items-start gap-3 rounded-md border border-dashed border-lab-line bg-lab-panel2/40 p-4">
      <div className="grid h-9 w-9 place-items-center rounded-md bg-lab-accent/10 text-lab-accent">
        <MousePointer2 className="h-4 w-4" />
      </div>
      <div>
        <h4 className="text-sm font-semibold text-lab-text">
          {t(locale, "inspector.emptyTitle")}
        </h4>
        <p className="mt-1 text-xs leading-relaxed text-lab-muted">
          {t(locale, "inspector.emptySelection")}
        </p>
      </div>
      <div className="flex items-center gap-2 text-[11px] text-lab-muted">
        <kbd className="rounded border border-lab-line bg-lab-panel2 px-1.5 py-0.5 font-mono text-[10px] text-lab-text shadow-sm">
          Esc
        </kbd>
        <span>{t(locale, "inspector.emptyHintEsc")}</span>
      </div>
    </section>
  )
}

function FactGroup({
  title,
  children,
  defaultOpen = true,
}: {
  title: string
  children: ReactNode
  defaultOpen?: boolean
}) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        aria-expanded={open}
        className="group/group flex w-full items-center gap-1 rounded-sm px-1.5 py-0.5 text-left transition-colors hover:bg-white/[0.04] focus-visible:outline-none focus-visible:shadow-focus"
      >
        <ChevronRight
          className={cn(
            "h-3 w-3 shrink-0 text-lab-muted/60 transition-transform",
            open && "rotate-90",
          )}
        />
        <span className="text-[10px] font-semibold uppercase tracking-wider text-lab-muted/80 group-hover/group:text-lab-muted">
          {title}
        </span>
      </button>
      {open ? <div className="space-y-px pt-0.5">{children}</div> : null}
    </div>
  )
}

function EntityInspector({
  selected,
  locale,
}: {
  locale: Locale
  selected:
    | { kind: "body"; entity: DebugBody }
    | { kind: "collider"; entity: DebugCollider; body?: DebugBody }
    | { kind: "contact"; entity: FrameRecord["snapshot"]["contacts"][number] }
    | { kind: "joint"; entity: FrameRecord["snapshot"]["joints"][number] }
}) {
  const title = entityLabel(locale, selected.kind, entityId(selected))
  return (
    <section className="overflow-hidden rounded-md border border-lab-line bg-lab-panel2 ring-1 ring-lab-accent/15">
      <EntityInspectorHeader
        title={title}
        kind={selected.kind}
        subtitle={selectedSubtitle(locale, selected)}
        locale={locale}
      />
      <div className="space-y-3 px-3 py-3">
        {selected.kind === "body" && (
          <>
            <FactGroup title={t(locale, "group.transform")}>
              <Fact
                label={t(locale, "fact.type")}
                value={bodyTypeLabel(locale, selected.entity.body_type)}
                mono={false}
              />
              <VectorFact
                label={t(locale, "fact.position")}
                value={selected.entity.transform.translation}
              />
              <Fact
                label={t(locale, "fact.sleeping")}
                value={booleanLabel(locale, selected.entity.sleeping)}
                mono={false}
              />
            </FactGroup>
            <FactGroup title={t(locale, "group.massProperties")}>
              <Fact
                label={t(locale, "fact.mass")}
                value={selected.entity.mass_properties.mass.toFixed(3)}
              />
              <Fact
                label={t(locale, "fact.inverseMass")}
                value={selected.entity.mass_properties.inverse_mass.toFixed(3)}
              />
              <VectorFact
                label={t(locale, "fact.centerOfMass")}
                value={selected.entity.mass_properties.local_center_of_mass}
              />
              <Fact
                label={t(locale, "fact.inertia")}
                value={selected.entity.mass_properties.inertia.toFixed(3)}
              />
              <Fact
                label={t(locale, "fact.inverseInertia")}
                value={selected.entity.mass_properties.inverse_inertia.toFixed(3)}
              />
            </FactGroup>
            <FactGroup title={t(locale, "group.velocities")}>
              <VectorFact
                label={t(locale, "fact.linearVelocity")}
                value={selected.entity.linear_velocity}
              />
              <Fact
                label={t(locale, "fact.angularVelocity")}
                value={selected.entity.angular_velocity.toFixed(3)}
              />
            </FactGroup>
          </>
        )}
        {selected.kind === "collider" && (
          <>
            <FactGroup title={t(locale, "group.transform")}>
              <Fact
                label={t(locale, "fact.body")}
                value={String(selected.entity.body)}
              />
              <Fact
                label={t(locale, "fact.shape")}
                value={dynamicValueLabel(locale, selected.entity.shape.kind)}
                mono={false}
              />
              <VectorFact
                label={t(locale, "fact.center")}
                value={selected.entity.world_transform.translation}
              />
              <Fact
                label={t(locale, "fact.sensor")}
                value={booleanLabel(locale, selected.entity.is_sensor)}
                mono={false}
              />
            </FactGroup>
            <FactGroup title={t(locale, "group.material")}>
              <Fact
                label={t(locale, "fact.friction")}
                value={selected.entity.material.friction.toFixed(3)}
              />
              <Fact
                label={t(locale, "fact.restitution")}
                value={selected.entity.material.restitution.toFixed(3)}
              />
            </FactGroup>
            <FactGroup title={t(locale, "group.velocities")}>
              {selected.body ? (
                <VectorFact
                  label={t(locale, "fact.ownerVelocity")}
                  value={selected.body.linear_velocity}
                />
              ) : (
                <Fact
                  label={t(locale, "fact.ownerVelocity")}
                  value={t(locale, "common.unknown")}
                  mono={false}
                />
              )}
            </FactGroup>
          </>
        )}
        {selected.kind === "contact" && (
          <>
            <FactGroup title={t(locale, "group.contactInfo")}>
              <VectorFact
                label={t(locale, "fact.point")}
                value={selected.entity.point}
              />
              <VectorFact
                label={t(locale, "fact.normal")}
                value={selected.entity.normal}
              />
              <Fact
                label={t(locale, "fact.depth")}
                value={selected.entity.depth.toFixed(4)}
              />
              <Fact
                label={t(locale, "fact.feature")}
                value={String(selected.entity.feature_id)}
              />
              <Fact
                label={t(locale, "fact.reduction")}
                value={dynamicValueLabel(
                  locale,
                  selected.entity.reduction_reason,
                )}
                mono={false}
              />
              <Fact
                label={t(locale, "fact.restitution")}
                value={
                  selected.entity.restitution_applied
                    ? t(locale, "contact.applied")
                    : t(locale, "contact.suppressed")
                }
                mono={false}
              />
            </FactGroup>
            <FactGroup title={t(locale, "group.warmStart")}>
              <Fact
                label={t(locale, "inspector.warmStart")}
                value={dynamicValueLabel(
                  locale,
                  selected.entity.warm_start_reason ?? "miss_no_previous",
                )}
                mono={false}
              />
              <Fact
                label={t(locale, "fact.warmStartNormal")}
                value={selected.entity.normal_impulse.toFixed(4)}
              />
              <Fact
                label={t(locale, "fact.warmStartTangent")}
                value={selected.entity.tangent_impulse.toFixed(4)}
              />
            </FactGroup>
            <FactGroup title={t(locale, "group.solver")}>
              <Fact
                label={t(locale, "fact.solverNormal")}
                value={(selected.entity.solver_normal_impulse ?? 0).toFixed(4)}
              />
              <Fact
                label={t(locale, "fact.solverTangent")}
                value={(selected.entity.solver_tangent_impulse ?? 0).toFixed(4)}
              />
              <Fact
                label={t(locale, "fact.normalClamped")}
                value={booleanLabel(
                  locale,
                  selected.entity.normal_impulse_clamped ?? false,
                )}
                mono={false}
              />
              <Fact
                label={t(locale, "fact.tangentClamped")}
                value={booleanLabel(
                  locale,
                  selected.entity.tangent_impulse_clamped ?? false,
                )}
                mono={false}
              />
            </FactGroup>
            {selected.entity.generic_convex_trace && (
              <FactGroup
                title={t(locale, "group.trace")}
                defaultOpen={false}
              >
                <Fact
                  label={t(locale, "fact.genericFallback")}
                  value={dynamicValueLabel(
                    locale,
                    selected.entity.generic_convex_trace.fallback_reason,
                  )}
                  mono={false}
                />
                <Fact
                  label={t(locale, "fact.gjk")}
                  value={traceStage(
                    locale,
                    selected.entity.generic_convex_trace.gjk_termination,
                    selected.entity.generic_convex_trace.gjk_iterations,
                  )}
                />
                <Fact
                  label={t(locale, "fact.epa")}
                  value={traceStage(
                    locale,
                    selected.entity.generic_convex_trace.epa_termination,
                    selected.entity.generic_convex_trace.epa_iterations,
                  )}
                />
                <Fact
                  label={t(locale, "fact.simplex")}
                  value={String(
                    selected.entity.generic_convex_trace.simplex_len,
                  )}
                />
              </FactGroup>
            )}
            {selected.entity.ccd_trace && (
              <FactGroup
                title={t(locale, "group.ccdTrace")}
                defaultOpen={false}
              >
                <Fact
                  label={t(locale, "fact.ccdToi")}
                  value={selected.entity.ccd_trace.toi.toFixed(5)}
                />
                <Fact
                  label={t(locale, "fact.ccdAdvancement")}
                  value={selected.entity.ccd_trace.advancement.toFixed(5)}
                />
                <Fact
                  label={t(locale, "fact.ccdClamp")}
                  value={selected.entity.ccd_trace.clamp.toFixed(5)}
                />
                <Fact
                  label={t(locale, "fact.ccdSlop")}
                  value={selected.entity.ccd_trace.slop.toFixed(5)}
                />
                <VectorFact
                  label={t(locale, "fact.ccdSweptStart")}
                  value={selected.entity.ccd_trace.swept_start}
                />
                <VectorFact
                  label={t(locale, "fact.ccdSweptEnd")}
                  value={selected.entity.ccd_trace.swept_end}
                />
                <VectorFact
                  label={t(locale, "fact.ccdToiPoint")}
                  value={selected.entity.ccd_trace.toi_point}
                />
              </FactGroup>
            )}
          </>
        )}
        {selected.kind === "joint" && (
          <FactGroup title={t(locale, "group.jointInfo")}>
            <Fact
              label={t(locale, "fact.kind")}
              value={dynamicValueLabel(locale, selected.entity.kind)}
              mono={false}
            />
            <Fact
              label={t(locale, "tree.bodies")}
              value={selected.entity.bodies.join(", ")}
            />
            <Fact
              label={t(locale, "fact.anchors")}
              value={
                <span className="inline-flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
                  {selected.entity.anchors.map((anchor, index) => (
                    <span
                      key={index}
                      className="inline-flex items-baseline gap-1"
                    >
                      <VectorValue value={anchor} />
                      {index < selected.entity.anchors.length - 1 ? (
                        <span className="text-lab-muted">→</span>
                      ) : null}
                    </span>
                  ))}
                </span>
              }
              copyValue={selected.entity.anchors.map(vec).join(" -> ")}
            />
          </FactGroup>
        )}
      </div>
    </section>
  )
}

function EntityInspectorHeader({
  title,
  kind,
  subtitle,
  locale,
}: {
  title: string
  kind: EntityKind
  subtitle: string
  locale: Locale
}) {
  const tone = entityKindTone(kind)
  return (
    <header className="flex items-start gap-3 border-b border-lab-line/70 bg-gradient-to-b from-white/[0.03] to-transparent px-3 py-3">
      <div
        className={cn(
          "grid h-8 w-8 shrink-0 place-items-center rounded-md border",
          tone.iconWrap,
        )}
      >
        {entityKindIcon(kind, "h-4 w-4")}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <h3 className="truncate text-sm font-semibold text-lab-text">
            {title}
          </h3>
          <Badge tone={tone.badge}>{entityKindLabel(locale, kind)}</Badge>
        </div>
        <p className="mt-0.5 truncate text-[11px] text-lab-muted">{subtitle}</p>
      </div>
    </header>
  )
}

function entityKindIcon(kind: EntityKind, className?: string): ReactNode {
  switch (kind) {
    case "body":
      return <Square className={className} />
    case "collider":
      return <CircleDot className={className} />
    case "contact":
      return <MousePointer2 className={className} />
    case "joint":
      return <Settings2 className={className} />
  }
}

function entityKindTone(kind: EntityKind): {
  iconWrap: string
  badge: "neutral" | "accent" | "warn" | "danger" | "green"
} {
  switch (kind) {
    case "body":
      return {
        iconWrap: "border-lab-accent/40 bg-lab-accent/10 text-lab-accent",
        badge: "accent",
      }
    case "collider":
      return {
        iconWrap: "border-lab-green/40 bg-lab-green/10 text-lab-green",
        badge: "green",
      }
    case "contact":
      return {
        iconWrap: "border-lab-warn/40 bg-lab-warn/10 text-lab-warn",
        badge: "warn",
      }
    case "joint":
      return {
        iconWrap: "border-lab-line bg-white/[0.04] text-lab-text",
        badge: "neutral",
      }
  }
}

function selectedSubtitle(
  locale: Locale,
  selected:
    | { kind: "body"; entity: DebugBody }
    | { kind: "collider"; entity: DebugCollider; body?: DebugBody }
    | { kind: "contact"; entity: FrameRecord["snapshot"]["contacts"][number] }
    | { kind: "joint"; entity: FrameRecord["snapshot"]["joints"][number] },
): string {
  switch (selected.kind) {
    case "body": {
      const type = bodyTypeLabel(locale, selected.entity.body_type)
      const sleeping = selected.entity.sleeping
        ? `· ${t(locale, "fact.sleeping")}`
        : ""
      return `${type} ${sleeping}`.trim()
    }
    case "collider": {
      const shape = dynamicValueLabel(locale, selected.entity.shape.kind)
      const sensor = selected.entity.is_sensor
        ? `· ${t(locale, "fact.sensor")}`
        : ""
      return `${shape} ${sensor}`.trim()
    }
    case "contact": {
      return `${t(locale, "fact.depth")} ${selected.entity.depth.toFixed(3)}`
    }
    case "joint": {
      const kind = dynamicValueLabel(locale, selected.entity.kind)
      const bodyCount = selected.entity.bodies.length
      return `${kind} · ${bodyCount} ${t(locale, "tree.bodies")}`
    }
  }
}

function VectorValue({ value }: { value: { x: number; y: number } }) {
  return (
    <span className="inline-flex flex-wrap items-baseline gap-x-2 gap-y-0">
      <VectorAxis axis="x" value={value.x} />
      <VectorAxis axis="y" value={value.y} />
    </span>
  )
}

function VectorAxis({ axis, value }: { axis: "x" | "y"; value: number }) {
  return (
    <span className="inline-flex items-baseline gap-1">
      <span className="text-[10px] uppercase text-lab-muted/70">{axis}</span>
      <span className="font-mono tabular-nums">{value.toFixed(3)}</span>
    </span>
  )
}

function entityId(
  selected:
    | { kind: "body"; entity: DebugBody }
    | { kind: "collider"; entity: DebugCollider; body?: DebugBody }
    | { kind: "contact"; entity: FrameRecord["snapshot"]["contacts"][number] }
    | { kind: "joint"; entity: FrameRecord["snapshot"]["joints"][number] },
): number {
  return selected.kind === "contact"
    ? selected.entity.id
    : selected.entity.handle
}

function BottomTimeline({
  frames,
  frameIndex,
  onFrameChange,
  logs,
  frameCount,
  setFrameCount,
  useCustomGravity,
  setUseCustomGravity,
  gravityY,
  setGravityY,
  locale,
  onPlay,
  onPause,
  onStep,
  onReset,
}: {
  frames: FrameRecord[]
  frameIndex: number
  onFrameChange: (value: number) => void
  logs: WorkbenchLog[]
  frameCount: number
  setFrameCount: (value: number) => void
  useCustomGravity: boolean
  setUseCustomGravity: (value: boolean) => void
  gravityY: number
  setGravityY: (value: number) => void
  locale: Locale
  onPlay: () => void
  onPause: () => void
  onStep: () => void
  onReset: () => void
}) {
  const frame = frames[Math.min(frameIndex, Math.max(0, frames.length - 1))]
  return (
    <Tabs.Root defaultValue="timeline" className="flex h-full min-h-0 flex-col">
      <PanelHeader>
        <div className="flex items-center gap-3">
          <div className="flex items-center rounded-md bg-black/20 p-0.5 shadow-inner">
            <Tooltip label={t(locale, "tooltip.pausePlayback")}>
              <button
                type="button"
                className="flex h-6 w-7 items-center justify-center rounded-sm text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text"
                onClick={onPause}
              >
                <Pause className="h-3.5 w-3.5" />
              </button>
            </Tooltip>
            <Tooltip label={t(locale, "tooltip.playTimeline")}>
              <button
                type="button"
                className="flex h-6 w-7 items-center justify-center rounded-sm text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text"
                onClick={onPlay}
              >
                <Gauge className="h-3.5 w-3.5" />
              </button>
            </Tooltip>
            <Tooltip label={t(locale, "tooltip.advanceFrame")}>
              <button
                type="button"
                className="flex h-6 w-7 items-center justify-center rounded-sm text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text"
                onClick={onStep}
              >
                <SkipForward className="h-3.5 w-3.5" />
              </button>
            </Tooltip>
            <Tooltip label={t(locale, "tooltip.resetTimeline")}>
              <button
                type="button"
                className="flex h-6 w-7 items-center justify-center rounded-sm text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text"
                onClick={onReset}
              >
                <RotateCcw className="h-3.5 w-3.5" />
              </button>
            </Tooltip>
          </div>
          <div className="h-4 w-px bg-lab-line/80" />
          <Tabs.List className="flex items-center gap-1">
            <Tabs.Trigger value="timeline" className="tab-trigger">
              {t(locale, "timeline.timeline")}
            </Tabs.Trigger>
            <Tabs.Trigger value="logs" className="tab-trigger">
              {t(locale, "timeline.logs")}
            </Tabs.Trigger>
            <Tabs.Trigger value="run" className="tab-trigger">
              {t(locale, "timeline.runSetup")}
            </Tabs.Trigger>
          </Tabs.List>
        </div>
        <div className="flex items-center gap-2 text-xs text-lab-muted">
          <span className="tabular-nums">
            {frame?.simulated_time.toFixed(3)}s
          </span>
          <span className="tabular-nums">{frame?.state_hash}</span>
        </div>
      </PanelHeader>

      <Tabs.Content
        value="timeline"
        className="min-h-0 flex-1 p-3 outline-none"
      >
        <div className="mb-3 flex items-center gap-3">
          <span className="w-20 text-xs text-lab-muted">
            {t(locale, "timeline.frameAt", { frame: frameIndex })}
          </span>
          <Slider
            value={frameIndex}
            min={0}
            max={Math.max(0, frames.length - 1)}
            step={1}
            onValueChange={onFrameChange}
          />
          <span className="w-20 text-right text-xs text-lab-muted">
            {t(locale, "timeline.totalFrames", { count: frames.length })}
          </span>
        </div>
        <div className="grid grid-cols-4 gap-2">
          <Metric
            label={t(locale, "metric.step")}
            value={frame?.snapshot.stats.step_index ?? 0}
          />
          <Metric
            label={t(locale, "metric.simTime")}
            value={(frame?.snapshot.meta.simulated_time ?? 0).toFixed(3)}
          />
          <Metric
            label={t(locale, "metric.gravity")}
            value={vec(frame?.snapshot.meta.gravity ?? { x: 0, y: 0 })}
          />
          <Metric
            label={t(locale, "metric.manifolds")}
            value={frame?.snapshot.manifolds.length ?? 0}
          />
        </div>
      </Tabs.Content>

      <Tabs.Content
        value="logs"
        className="min-h-0 flex-1 overflow-auto p-2 outline-none"
      >
        <div className="space-y-1 font-mono text-xs">
          {logs.map((entry, index) => (
            <div
              key={`${entry.time}-${index}`}
              className="grid grid-cols-[74px_52px_1fr] gap-2 rounded px-2 py-1 hover:bg-white/5"
            >
              <span className="text-lab-muted">{entry.time}</span>
              <span
                className={
                  entry.level === "error"
                    ? "text-lab-danger"
                    : entry.level === "warn"
                      ? "text-lab-warn"
                      : "text-lab-accent"
                }
              >
                {entry.level}
              </span>
              <span className="min-w-0 truncate text-lab-text">
                {entry.message}
              </span>
            </div>
          ))}
        </div>
      </Tabs.Content>

      <Tabs.Content value="run" className="min-h-0 flex-1 p-3 outline-none">
        <div className="grid max-w-2xl grid-cols-[140px_1fr] items-center gap-3">
          <label className="text-sm text-lab-muted">
            {t(locale, "run.frameCount")}
          </label>
          <Input
            type="number"
            min={1}
            max={2000}
            value={frameCount}
            onChange={(event) =>
              setFrameCount(Math.max(1, Number(event.target.value) || 1))
            }
          />
          <label className="text-sm text-lab-muted">
            {t(locale, "run.gravityOverride")}
          </label>
          <Checkbox
            checked={useCustomGravity}
            onCheckedChange={setUseCustomGravity}
            label={t(locale, "run.sendOverride")}
          />
          <label className="text-sm text-lab-muted">
            {t(locale, "run.gravityY")}
          </label>
          <Input
            type="number"
            step="0.1"
            value={gravityY}
            disabled={!useCustomGravity}
            onChange={(event) => setGravityY(Number(event.target.value) || 0)}
          />
        </div>
      </Tabs.Content>
    </Tabs.Root>
  )
}

function Metric({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-md border border-lab-line bg-lab-panel2 px-2 py-1.5">
      <div className="truncate text-[11px] uppercase tracking-normal text-lab-muted">
        {label}
      </div>
      <div className="truncate font-mono text-sm tabular-nums text-lab-text">
        {value}
      </div>
    </div>
  )
}

function Fact({
  label,
  value,
  muted = false,
  mono = true,
  copyValue,
}: {
  label: string
  value: ReactNode
  muted?: boolean
  mono?: boolean
  copyValue?: string
}) {
  const fallback = typeof value === "string" ? value : undefined
  const effectiveCopy = copyValue ?? fallback
  return (
    <div className="group/fact relative grid grid-cols-[minmax(88px,40%)_1fr] items-baseline gap-3 rounded-sm px-2 py-1 transition-colors hover:bg-white/[0.04]">
      <span className="truncate text-[11px] text-lab-muted">{label}</span>
      <span
        className={cn(
          "min-w-0 break-words text-xs",
          mono && "font-mono tabular-nums",
          muted ? "text-lab-warn" : "text-lab-text",
        )}
      >
        {value}
      </span>
      {effectiveCopy ? <CopyButton value={effectiveCopy} /> : null}
    </div>
  )
}

function VectorFact({
  label,
  value,
}: {
  label: string
  value: { x: number; y: number }
}) {
  return (
    <Fact
      label={label}
      value={<VectorValue value={value} />}
      copyValue={vec(value)}
    />
  )
}

function CopyButton({ value }: { value: string }) {
  const [copied, setCopied] = useState(false)
  const timerRef = useRef<number | null>(null)

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current)
      }
    }
  }, [])

  function handleClick(event: React.MouseEvent<HTMLButtonElement>) {
    event.stopPropagation()
    void navigator.clipboard
      .writeText(value)
      .then(() => {
        setCopied(true)
        if (timerRef.current !== null) {
          window.clearTimeout(timerRef.current)
        }
        timerRef.current = window.setTimeout(() => {
          setCopied(false)
          timerRef.current = null
        }, 1200)
      })
      .catch(() => {})
  }

  return (
    <button
      type="button"
      onClick={handleClick}
      title={value}
      aria-label="copy"
      className={cn(
        "absolute right-1 top-1/2 -translate-y-1/2 rounded-sm p-1 text-lab-muted opacity-0 outline-none transition-opacity",
        "bg-lab-panel2/85 backdrop-blur-sm hover:text-lab-accent",
        "focus-visible:opacity-100 focus-visible:shadow-focus",
        "group-hover/fact:opacity-100",
      )}
    >
      {copied ? (
        <Check className="h-3 w-3 text-lab-green" />
      ) : (
        <Copy className="h-3 w-3" />
      )}
    </button>
  )
}

function ResizeHandle({ vertical = false }: { vertical?: boolean }) {
  return (
    <PanelResizeHandle
      className={cn(
        "shrink-0 bg-lab-line transition-colors hover:bg-lab-accent/70",
        vertical ? "h-1 w-full" : "h-full w-1",
      )}
    />
  )
}

function resolveSelection(frame: FrameRecord, selected: SelectedEntity | null) {
  if (!selected) {
    return null
  }
  if (selected.kind === "body") {
    const entity = frame.snapshot.bodies.find(
      (body) => body.handle === selected.id,
    )
    return entity ? { kind: selected.kind, entity } : null
  }
  if (selected.kind === "collider") {
    const entity = frame.snapshot.colliders.find(
      (collider) => collider.handle === selected.id,
    )
    const body = entity
      ? frame.snapshot.bodies.find((entry) => entry.handle === entity.body)
      : undefined
    return entity ? { kind: selected.kind, entity, body } : null
  }
  if (selected.kind === "contact") {
    const entity = frame.snapshot.contacts.find(
      (contact) => contact.id === selected.id,
    )
    return entity ? { kind: selected.kind, entity } : null
  }
  const entity = frame.snapshot.joints.find(
    (joint) => joint.handle === selected.id,
  )
  return entity ? { kind: selected.kind, entity } : null
}

function vec(value: { x: number; y: number }): string {
  return `${value.x.toFixed(3)}, ${value.y.toFixed(3)}`
}

function log(level: WorkbenchLog["level"], message: string): WorkbenchLog {
  return {
    time: new Date().toLocaleTimeString("en-US", { hour12: false }),
    level,
    message,
  }
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
