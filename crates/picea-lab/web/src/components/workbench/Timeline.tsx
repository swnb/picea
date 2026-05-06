import { memo, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react"
import { ClipboardCopy, Gauge, Pause, RotateCcw, SkipForward } from "lucide-react"

import { Input } from "../ui/input"
import { PanelHeader } from "../ui/panel"
import { Checkbox, Select, Slider, Tooltip } from "../ui/radix"
import {
  diagnosticMarkerLabel,
  diagnosticSeverityLabel,
  diagnosticSourceLabel,
  entityLabel,
  layerLabel,
  missingEvidenceLabel,
  sourceLabel,
  statusLabel,
  t,
  type LayerKey,
  type Locale,
  type StatusKind,
} from "../../i18n"
import type {
  DiagnosticMarker,
  DiagnosticSeverity,
  DiagnosticSource,
  FrameDiagnostics as FrameDiagnosticsRecord,
  FrameRecord,
  MissingEvidence,
  MissingEvidenceKind,
  PerfArtifact,
  SelectedEntity,
  WorkbenchLog,
} from "../../types"
import { vec } from "./format"
import {
  buildStackMarkers,
  buildStackStabilitySummary,
  type EvidenceMetric,
  type StackMarker,
} from "./stackStability"
import {
  buildTrajectoryMarkers,
  buildTrajectoryOverlay,
  type TrajectoryMarker,
} from "./trajectory"
import { deriveLatticeProxy } from "./types"
import type {
  CanvasDebugView,
  LayerState,
  PerfEvidenceStatus,
  RunMode,
  SourceKind,
  TrajectorySettings,
} from "./types"

type BottomPanelId =
  | "timeline"
  | "trajectory"
  | "lattice"
  | "stack"
  | "logs"
  | "diagnostics"
  | "evidence"
  | "run"

type TimelineJumpMarker = {
  key: string
  frameIndex: number
  label: string
  score: number | null
  severity?: DiagnosticSeverity
  source: "diagnostics" | "stack" | "trajectory"
  evidenceSource: DiagnosticSource
}

export function BottomTimeline({
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
  source,
  status,
  sessionId,
  runId,
  selectedEntity,
  layers,
  trajectorySettings,
  onTrajectorySettingsChange,
  canvasView,
  debugContextText,
  perfArtifact,
  perfStatus,
  controlBusy,
  runMode,
  setRunMode,
  onPlay,
  onPause,
  onStep,
  onReset,
  onCopyDebugContext,
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
  runMode: RunMode
  setRunMode: (value: RunMode) => void
  onPlay: () => void
  onPause: () => void
  onStep: () => void
  onReset: () => void
  source: SourceKind
  status: StatusKind
  sessionId: string | null
  runId: string | null
  selectedEntity: SelectedEntity | null
  layers: LayerState
  trajectorySettings: TrajectorySettings
  onTrajectorySettingsChange: (value: TrajectorySettings) => void
  canvasView: CanvasDebugView | null
  debugContextText: string | null
  perfArtifact: PerfArtifact | null
  perfStatus: PerfEvidenceStatus
  controlBusy: boolean
  onCopyDebugContext: () => void
}) {
  const frame = frames[Math.min(frameIndex, Math.max(0, frames.length - 1))]
  const previousFrame = frameIndex > 0 ? frames[frameIndex - 1] : null
  const stackSummary = buildStackStabilitySummary(frames, frameIndex)
  const stackMarkers = buildStackMarkers(frames)
  const diagnosticMarkers = buildDiagnosticTimelineMarkers(locale, frames)
  const trajectoryOverlay = buildTrajectoryOverlay(
    frames,
    frameIndex,
    selectedEntity,
    trajectorySettings,
  )
  const trajectoryMarkers = buildTrajectoryMarkers(frames)
  const latticeSummary = frame ? deriveLatticeProxy(frame, frames[0] ?? frame) : null
  const railMarkers = pickRailMarkers([
    ...diagnosticMarkers,
    ...stackMarkers.map((marker) => ({
      key: `stack-${marker.kind}-${marker.frameIndex}`,
      frameIndex: marker.frameIndex,
      label: stackMarkerLabel(locale, marker.kind),
      score: marker.score,
      source: "stack" as const,
      evidenceSource: "web_derived" as const,
    })),
    ...trajectoryMarkers.map((marker) => ({
      key: `trajectory-${marker.kind}-${marker.frameIndex}`,
      frameIndex: marker.frameIndex,
      label: trajectoryMarkerLabel(locale, marker.kind),
      score: marker.score,
      source: "trajectory" as const,
      evidenceSource: "web_derived" as const,
    })),
  ])
  const handlePlay = useStableEvent(onPlay)
  const handlePause = useStableEvent(onPause)
  const handleStep = useStableEvent(onStep)
  const handleReset = useStableEvent(onReset)
  const [activePanel, setActivePanel] = useState<BottomPanelId>("timeline")
  const handlePanelChange = useCallback((panel: BottomPanelId) => {
    setActivePanel(panel)
  }, [])
  return (
    <div className="flex h-full min-h-0 flex-col">
      <TimelineHeader
        locale={locale}
        source={source}
        status={status}
        activePanel={activePanel}
        controlBusy={controlBusy}
        onPanelChange={handlePanelChange}
        onPlay={handlePlay}
        onPause={handlePause}
        onStep={handleStep}
        onReset={handleReset}
      />

      {activePanel === "timeline" ? (
      <div
        id="bottom-panel-timeline"
        role="tabpanel"
        aria-labelledby="bottom-panel-tab-timeline"
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
        <TimelineMarkerRail
          locale={locale}
          frameIndex={frameIndex}
          markers={railMarkers}
          onFrameChange={onFrameChange}
        />
        <div className="mt-3 rounded-md border border-lab-line bg-lab-panel2 px-2 py-1 text-xs text-lab-muted">
          {t(locale, "timeline.sessionStatus", {
            sessionId: sessionId ?? "-",
            buffered: frames.length,
            current: frameIndex,
          })}
        </div>
        <FrameIdentityRow locale={locale} frame={frame} />
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
      </div>
      ) : null}

      {activePanel === "stack" ? (
      <div
        id="bottom-panel-stack"
        role="tabpanel"
        aria-labelledby="bottom-panel-tab-stack"
        className="min-h-0 flex-1 overflow-auto p-3 outline-none"
      >
        <StackStabilityPanel
          locale={locale}
          frameIndex={frameIndex}
          summary={stackSummary}
          markers={stackMarkers}
          onFrameChange={onFrameChange}
        />
      </div>
      ) : null}

      {activePanel === "trajectory" ? (
      <div
        id="bottom-panel-trajectory"
        role="tabpanel"
        aria-labelledby="bottom-panel-tab-trajectory"
        className="min-h-0 flex-1 overflow-auto p-3 outline-none"
      >
        <TrajectoryPanel
          locale={locale}
          frameIndex={frameIndex}
          settings={trajectorySettings}
          overlay={trajectoryOverlay}
          markers={trajectoryMarkers}
          onSettingsChange={onTrajectorySettingsChange}
          onFrameChange={onFrameChange}
        />
      </div>
      ) : null}

      {activePanel === "lattice" ? (
      <div
        id="bottom-panel-lattice"
        role="tabpanel"
        aria-labelledby="bottom-panel-tab-lattice"
        className="min-h-0 flex-1 overflow-auto p-3 outline-none"
      >
        <LatticeProxyPanel
          locale={locale}
          summary={latticeSummary}
          frameIndex={frameIndex}
        />
      </div>
      ) : null}

      {activePanel === "diagnostics" ? (
      <div
        id="bottom-panel-diagnostics"
        role="tabpanel"
        aria-labelledby="bottom-panel-tab-diagnostics"
        className="min-h-0 flex-1 overflow-auto p-3 outline-none"
      >
        <FrameDiagnostics
          locale={locale}
          frame={frame}
          previousFrame={previousFrame}
        />
      </div>
      ) : null}

      {activePanel === "evidence" ? (
      <div
        id="bottom-panel-evidence"
        role="tabpanel"
        aria-labelledby="bottom-panel-tab-evidence"
        className="min-h-0 flex-1 overflow-auto p-3 outline-none"
      >
        <EvidencePanel
          locale={locale}
          source={source}
          sessionId={sessionId}
          runId={runId}
          frameIndex={frameIndex}
          selectedEntity={selectedEntity}
          layers={layers}
          canvasView={canvasView}
          debugContextText={debugContextText}
          perfArtifact={perfArtifact}
          perfStatus={perfStatus}
          onCopyDebugContext={onCopyDebugContext}
        />
      </div>
      ) : null}

      {activePanel === "logs" ? (
      <div
        id="bottom-panel-logs"
        role="tabpanel"
        aria-labelledby="bottom-panel-tab-logs"
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
      </div>
      ) : null}

      {activePanel === "run" ? (
      <div
        id="bottom-panel-run"
        role="tabpanel"
        aria-labelledby="bottom-panel-tab-run"
        className="min-h-0 flex-1 p-3 outline-none"
      >
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
            {t(locale, "run.mode")}
          </label>
          <Select
            value={runMode}
            onValueChange={(value) => setRunMode(value as RunMode)}
            ariaLabel={t(locale, "run.mode")}
            items={[
              {
                value: "artifact_replay",
                label: t(locale, "run.modeArtifact"),
              },
              {
                value: "live_session",
                label: t(locale, "run.modeLive"),
              },
            ]}
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
      </div>
      ) : null}
    </div>
  )
}

function useStableEvent(handler: () => void): () => void {
  const handlerRef = useRef(handler)
  useEffect(() => {
    handlerRef.current = handler
  }, [handler])
  return useCallback(() => handlerRef.current(), [])
}

const TimelineHeader = memo(function TimelineHeader({
  locale,
  source,
  status,
  activePanel,
  controlBusy,
  onPanelChange,
  onPlay,
  onPause,
  onStep,
  onReset,
}: {
  locale: Locale
  source: SourceKind
  status: StatusKind
  activePanel: BottomPanelId
  controlBusy: boolean
  onPanelChange: (panel: BottomPanelId) => void
  onPlay: () => void
  onPause: () => void
  onStep: () => void
  onReset: () => void
}) {
  return (
    <PanelHeader>
      <div className="flex items-center gap-3">
        <div className="flex items-center rounded-md bg-black/20 p-0.5 shadow-inner">
          <TimelineIconButton
            label={t(locale, "tooltip.pausePlayback")}
            disabled={controlBusy}
            onClick={onPause}
            icon={<Pause className="h-3.5 w-3.5" />}
          />
          <TimelineIconButton
            label={t(locale, "tooltip.playTimeline")}
            disabled={controlBusy}
            onClick={onPlay}
            icon={<Gauge className="h-3.5 w-3.5" />}
          />
          <TimelineIconButton
            label={t(locale, "tooltip.advanceFrame")}
            disabled={controlBusy}
            onClick={onStep}
            icon={<SkipForward className="h-3.5 w-3.5" />}
          />
          <TimelineIconButton
            label={t(locale, "tooltip.resetTimeline")}
            disabled={controlBusy}
            onClick={onReset}
            icon={<RotateCcw className="h-3.5 w-3.5" />}
          />
        </div>
        <div className="h-4 w-px bg-lab-line/80" />
        <div role="tablist" className="flex items-center gap-1">
          <BottomPanelTabButton
            panel="timeline"
            activePanel={activePanel}
            onPanelChange={onPanelChange}
          >
            {t(locale, "timeline.timeline")}
          </BottomPanelTabButton>
          <BottomPanelTabButton
            panel="trajectory"
            activePanel={activePanel}
            onPanelChange={onPanelChange}
          >
            {t(locale, "timeline.trajectory")}
          </BottomPanelTabButton>
          <BottomPanelTabButton
            panel="lattice"
            activePanel={activePanel}
            onPanelChange={onPanelChange}
          >
            {t(locale, "timeline.lattice")}
          </BottomPanelTabButton>
          <BottomPanelTabButton
            panel="stack"
            activePanel={activePanel}
            onPanelChange={onPanelChange}
          >
            {t(locale, "timeline.stack")}
          </BottomPanelTabButton>
          <BottomPanelTabButton
            panel="logs"
            activePanel={activePanel}
            onPanelChange={onPanelChange}
          >
            {t(locale, "timeline.logs")}
          </BottomPanelTabButton>
          <BottomPanelTabButton
            panel="diagnostics"
            activePanel={activePanel}
            onPanelChange={onPanelChange}
          >
            {t(locale, "timeline.diagnostics")}
          </BottomPanelTabButton>
          <BottomPanelTabButton
            panel="evidence"
            activePanel={activePanel}
            onPanelChange={onPanelChange}
          >
            {t(locale, "timeline.evidence")}
          </BottomPanelTabButton>
          <BottomPanelTabButton
            panel="run"
            activePanel={activePanel}
            onPanelChange={onPanelChange}
          >
            {t(locale, "timeline.runSetup")}
          </BottomPanelTabButton>
        </div>
      </div>
      <div className="flex items-center gap-2 text-xs text-lab-muted">
        <span>
          {t(locale, "timeline.sourceStatus", {
            source: sourceLabel(locale, source),
            status: statusLabel(locale, status),
          })}
        </span>
      </div>
    </PanelHeader>
  )
})

function BottomPanelTabButton({
  panel,
  activePanel,
  onPanelChange,
  children,
}: {
  panel: BottomPanelId
  activePanel: BottomPanelId
  onPanelChange: (panel: BottomPanelId) => void
  children: ReactNode
}) {
  const active = panel === activePanel
  return (
    <button
      id={`bottom-panel-tab-${panel}`}
      type="button"
      role="tab"
      aria-selected={active}
      aria-controls={`bottom-panel-${panel}`}
      data-state={active ? "active" : "inactive"}
      className="tab-trigger"
      onClick={() => onPanelChange(panel)}
    >
      {children}
    </button>
  )
}

function TimelineIconButton({
  label,
  disabled,
  onClick,
  icon,
}: {
  label: string
  disabled: boolean
  onClick: () => void
  icon: ReactNode
}) {
  return (
    <Tooltip label={label}>
      <button
        type="button"
        disabled={disabled}
        aria-label={label}
        className="flex h-6 w-7 items-center justify-center rounded-sm text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text disabled:pointer-events-none disabled:opacity-45"
        onClick={onClick}
      >
        {icon}
      </button>
    </Tooltip>
  )
}

function FrameIdentityRow({
  locale,
  frame,
}: {
  locale: Locale
  frame: FrameRecord | undefined
}) {
  return (
    <div className="mt-2 mb-3 grid grid-cols-[100px_1fr] gap-2 rounded-md border border-lab-line bg-black/15 px-2 py-1 text-xs">
      <span className="text-lab-muted">{t(locale, "metric.simTime")}</span>
      <span className="truncate text-right font-mono tabular-nums text-lab-text">
        {(frame?.simulated_time ?? 0).toFixed(3)}s
      </span>
      <span className="text-lab-muted">{t(locale, "diagnostics.stateHash")}</span>
      <span className="truncate text-right font-mono tabular-nums text-lab-text">
        {frame?.state_hash ?? "-"}
      </span>
    </div>
  )
}

function TimelineMarkerRail({
  locale,
  frameIndex,
  markers,
  onFrameChange,
}: {
  locale: Locale
  frameIndex: number
  markers: TimelineJumpMarker[]
  onFrameChange: (value: number) => void
}) {
  if (markers.length === 0) {
    return null
  }
  return (
    <div className="mb-3 flex flex-wrap items-center gap-1.5 rounded-md border border-lab-line bg-lab-panel2/60 px-2 py-1.5">
      {markers.map((marker) => {
        const active = marker.frameIndex === frameIndex
        return (
          <button
            key={marker.key}
            type="button"
            onClick={() => onFrameChange(marker.frameIndex)}
            title={`${marker.label} · ${diagnosticSourceLabel(locale, marker.evidenceSource)}`}
            className={`inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-[11px] transition-colors ${
              active
                ? "border-lab-accent bg-lab-accent/10 text-lab-text"
                : "border-lab-line bg-black/15 text-lab-muted hover:bg-white/5 hover:text-lab-text"
            }`}
          >
            <span>{marker.label}</span>
            <span className="font-mono">f{marker.frameIndex}</span>
          </button>
        )
      })}
    </div>
  )
}

function buildDiagnosticTimelineMarkers(
  locale: Locale,
  frames: FrameRecord[],
): TimelineJumpMarker[] {
  return frames.flatMap((frame) =>
    (frame.diagnostics?.markers ?? []).map((marker, index) => {
      const markerFrameIndex = marker.frame_index ?? frame.frame_index
      return {
        key: `diagnostics-${marker.kind}-${markerFrameIndex}-${index}`,
        frameIndex: markerFrameIndex,
        label: diagnosticMarkerLabel(locale, marker.kind),
        score: finiteNumberOrNull(marker.score),
        severity: marker.severity,
        source: "diagnostics" as const,
        evidenceSource: sourceOrMissing(marker.source),
      }
    }),
  )
}

function sourceOrMissing(source: DiagnosticSource | null | undefined): DiagnosticSource {
  return source ?? "missing"
}

function finiteNumberOrNull(value: number | null | undefined): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null
}

function pickRailMarkers(markers: TimelineJumpMarker[]): TimelineJumpMarker[] {
  const limit = 12
  const sourceOrder = ["diagnostics", "stack", "trajectory"] as const
  const perSourceTarget = Math.max(1, Math.floor(limit / sourceOrder.length))
  const selected = new Map<string, TimelineJumpMarker>()
  const leftovers: TimelineJumpMarker[] = []

  // Keep the rail balanced across exported diagnostics and Web-side fallback
  // markers first, then spend the remaining slots by score.
  for (const source of sourceOrder) {
    const sourceMarkers = markers
      .filter((marker) => marker.source === source)
      .sort(compareMarkerPriority)
    sourceMarkers.slice(0, perSourceTarget).forEach((marker) => {
      selected.set(marker.key, marker)
    })
    leftovers.push(...sourceMarkers.slice(perSourceTarget))
  }

  for (const marker of leftovers.sort(compareMarkerPriority)) {
    if (selected.size >= limit) {
      break
    }
    selected.set(marker.key, marker)
  }

  return [...selected.values()].sort(
    (left, right) =>
      left.frameIndex - right.frameIndex || markerScore(right) - markerScore(left),
  )
}

function compareMarkerPriority(left: TimelineJumpMarker, right: TimelineJumpMarker): number {
  return (
    severityRank(right.severity) - severityRank(left.severity) ||
    left.frameIndex - right.frameIndex ||
    markerScore(right) - markerScore(left)
  )
}

function markerScore(marker: TimelineJumpMarker): number {
  return marker.score ?? Number.NEGATIVE_INFINITY
}

function severityRank(severity: DiagnosticSeverity | null | undefined): number {
  if (severity === "severe") {
    return 2
  }
  if (severity === "warning") {
    return 1
  }
  return 0
}

function StackStabilityPanel({
  locale,
  frameIndex,
  summary,
  markers,
  onFrameChange,
}: {
  locale: Locale
  frameIndex: number
  summary: ReturnType<typeof buildStackStabilitySummary>
  markers: StackMarker[]
  onFrameChange: (value: number) => void
}) {
  return (
    <div className="grid gap-3 lg:grid-cols-[1.1fr_0.9fr]">
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-2">
        <div className="mb-2 flex items-center justify-between gap-2">
          <div>
            <div className="text-xs font-semibold text-lab-text">
              {t(locale, "panel.stackStability")}
            </div>
            <div className="text-[11px] text-lab-muted">
              {t(locale, "stability.derived")} / {t(locale, "stability.overlay")}
            </div>
          </div>
          <span className="rounded border border-lab-line px-1.5 py-0.5 text-[11px] text-lab-muted">
            {t(locale, "stability.window")} {summary.windowStart}-{summary.windowEnd}
          </span>
        </div>
        <div className="grid grid-cols-2 gap-2 xl:grid-cols-4">
          <Metric
            label={t(locale, "stability.dynamicBodies")}
            value={summary.dynamicBodyCount}
          />
          <Metric
            label={t(locale, "stability.awakeSleep")}
            value={`${summary.awakeBodyCount}/${summary.sleepingBodyCount}`}
          />
          <Metric
            label={t(locale, "metric.contacts")}
            value={summary.contactCount}
          />
          <Metric
            label={t(locale, "stability.contactSpike")}
            value={`+${summary.contactSpike}`}
          />
          <Metric
            label={t(locale, "stability.impulses")}
            value={`${formatEvidenceMetric(locale, summary.normalImpulseTotal)} / ${formatEvidenceMetric(locale, summary.tangentImpulseTotal)}`}
          />
          <Metric
            label={t(locale, "stability.solverRows")}
            value={summary.solverRows ?? t(locale, "stability.missing")}
          />
          <Metric
            label={t(locale, "stability.islands")}
            value={formatPairMetric(
              locale,
              summary.activeIslandCount,
              summary.sleepingIslandCount,
            )}
          />
          <Metric
            label={t(locale, "stability.maxDrift")}
            value={formatMaybeNumber(locale, summary.maxBodyDrift)}
          />
          <Metric
            label={t(locale, "stability.maxAngularDrift")}
            value={formatMaybeNumber(locale, summary.maxAngularDrift)}
          />
          <Metric
            label={t(locale, "stability.jitter")}
            value={formatMaybeNumber(locale, summary.jitterProxy)}
          />
          <Metric
            label={t(locale, "stability.quietWindow")}
            value={`${summary.quietFrames}f`}
          />
          <Metric label={t(locale, "canvas.frame")} value={frameIndex} />
        </div>
      </div>
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-2">
        <div className="mb-2 text-xs font-semibold text-lab-text">
          {t(locale, "panel.stackStability")}
        </div>
        {markers.length === 0 ? (
          <EmptyState label={t(locale, "stability.missing")} />
        ) : (
          <div className="space-y-2">
            {markers.map((marker) => (
              <button
                key={`${marker.kind}-${marker.frameIndex}`}
                type="button"
                onClick={() => onFrameChange(marker.frameIndex)}
                className="flex w-full items-center justify-between rounded border border-lab-line bg-black/15 px-2 py-2 text-left transition-colors hover:bg-white/5"
              >
                <span className="text-xs text-lab-text">
                  {stackMarkerLabel(locale, marker.kind)}
                </span>
                <span className="font-mono text-[11px] text-lab-muted">
                  f{marker.frameIndex} / {marker.score.toFixed(2)}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function TrajectoryPanel({
  locale,
  frameIndex,
  settings,
  overlay,
  markers,
  onSettingsChange,
  onFrameChange,
}: {
  locale: Locale
  frameIndex: number
  settings: TrajectorySettings
  overlay: ReturnType<typeof buildTrajectoryOverlay>
  markers: TrajectoryMarker[]
  onSettingsChange: (value: TrajectorySettings) => void
  onFrameChange: (value: number) => void
}) {
  return (
    <div className="grid gap-3 lg:grid-cols-[1.15fr_0.85fr]">
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-3">
        <div className="mb-3 flex items-center justify-between gap-2">
          <div>
            <div className="text-xs font-semibold text-lab-text">
              {t(locale, "panel.trajectory")}
            </div>
            <div className="text-[11px] text-lab-muted">
              {t(locale, "stability.derived")}
            </div>
          </div>
          <span className="rounded border border-lab-line px-1.5 py-0.5 text-[11px] text-lab-muted">
            {t(locale, "trajectory.summary.window")} {overlay.windowStart}-{overlay.windowEnd}
          </span>
        </div>
        <div className="grid gap-3 md:grid-cols-2">
          <div className="space-y-1.5">
            <label className="text-[11px] uppercase tracking-normal text-lab-muted">
              {t(locale, "trajectory.mode")}
            </label>
            <Select
              value={settings.mode}
              onValueChange={(value) =>
                onSettingsChange({
                  ...settings,
                  mode: value as TrajectorySettings["mode"],
                })
              }
              ariaLabel={t(locale, "trajectory.mode")}
              items={[
                {
                  value: "selectedBody",
                  label: t(locale, "trajectory.mode.selectedBody"),
                },
                {
                  value: "allDynamic",
                  label: t(locale, "trajectory.mode.allDynamic"),
                },
                {
                  value: "selectedIsland",
                  label: t(locale, "trajectory.mode.selectedIsland"),
                },
                {
                  value: "contacts",
                  label: t(locale, "trajectory.mode.contacts"),
                },
                {
                  value: "ccd",
                  label: t(locale, "trajectory.mode.ccd"),
                },
              ]}
            />
          </div>
          <div className="space-y-1.5">
            <label className="text-[11px] uppercase tracking-normal text-lab-muted">
              {t(locale, "trajectory.colorMode")}
            </label>
            <Select
              value={settings.colorMode}
              onValueChange={(value) =>
                onSettingsChange({
                  ...settings,
                  colorMode: value as TrajectorySettings["colorMode"],
                })
              }
              ariaLabel={t(locale, "trajectory.colorMode")}
              items={[
                {
                  value: "body",
                  label: t(locale, "trajectory.colorMode.body"),
                },
                {
                  value: "island",
                  label: t(locale, "trajectory.colorMode.island"),
                },
              ]}
            />
          </div>
          <TrajectorySliderRow
            locale={locale}
            label={t(locale, "trajectory.historyLength")}
            min={24}
            max={240}
            step={8}
            value={settings.historyLength}
            onChange={(value) =>
              onSettingsChange({ ...settings, historyLength: value })
            }
          />
          <TrajectorySliderRow
            locale={locale}
            label={t(locale, "trajectory.samplingStride")}
            min={1}
            max={12}
            step={1}
            value={settings.samplingStride}
            onChange={(value) =>
              onSettingsChange({ ...settings, samplingStride: value })
            }
          />
          <TrajectorySliderRow
            locale={locale}
            label={t(locale, "trajectory.fade")}
            min={10}
            max={95}
            step={5}
            value={Math.round(settings.fade * 100)}
            onChange={(value) =>
              onSettingsChange({ ...settings, fade: value / 100 })
            }
          />
          <div className="grid grid-cols-3 gap-2 rounded-md border border-lab-line bg-black/10 px-2 py-2">
            <Metric
              label={t(locale, "trajectory.summary.mode")}
              value={modeLabel(locale, settings.mode)}
            />
            <Metric
              label={t(locale, "tree.contacts")}
              value={overlay.contactTrails.length}
            />
            <Metric
              label={t(locale, "fact.ccdToi")}
              value={overlay.ccdTrails.length}
            />
          </div>
        </div>
        <div className="mt-3">
          {overlay.emptyState ? (
            <EmptyState label={trajectoryEmptyLabel(locale, overlay.emptyState)} />
          ) : (
            <div className="grid grid-cols-2 gap-2 xl:grid-cols-4">
              <Metric
                label={t(locale, "metric.bodies")}
                value={overlay.bodyTrails.length}
              />
              <Metric
                label={t(locale, "tree.contacts")}
                value={overlay.contactTrails.length}
              />
              <Metric
                label={t(locale, "fact.ccdToi")}
                value={overlay.ccdTrails.length}
              />
              <Metric
                label={t(locale, "canvas.frame")}
                value={frameIndex}
              />
            </div>
          )}
        </div>
      </div>
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-3">
        <div className="mb-2 text-xs font-semibold text-lab-text">
          {t(locale, "timeline.trajectory")}
        </div>
        {markers.length === 0 ? (
          <EmptyState label={t(locale, "stability.missing")} />
        ) : (
          <div className="space-y-2">
            {markers.map((marker) => (
              <button
                key={`${marker.kind}-${marker.frameIndex}`}
                type="button"
                onClick={() => onFrameChange(marker.frameIndex)}
                className="flex w-full items-center justify-between rounded border border-lab-line bg-black/15 px-2 py-2 text-left transition-colors hover:bg-white/5"
              >
                <span className="text-xs text-lab-text">
                  {trajectoryMarkerLabel(locale, marker.kind)}
                </span>
                <span className="font-mono text-[11px] text-lab-muted">
                  f{marker.frameIndex} / {marker.score.toFixed(2)}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function LatticeProxyPanel({
  locale,
  summary,
  frameIndex,
}: {
  locale: Locale
  summary: ReturnType<typeof deriveLatticeProxy> | null
  frameIndex: number
}) {
  if (!summary || !summary.enabled) {
    return (
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-3">
        <div className="mb-2 text-xs font-semibold text-lab-text">
          {t(locale, "panel.latticeProxy")}
        </div>
        <EmptyState label={t(locale, "stability.missing")} />
      </div>
    )
  }

  return (
    <div className="grid gap-3 lg:grid-cols-[1fr_0.85fr]">
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-3">
        <div className="mb-3 flex items-center justify-between gap-2">
          <div>
            <div className="text-xs font-semibold text-lab-text">
              {t(locale, "panel.latticeProxy")}
            </div>
            <div className="text-[11px] text-lab-muted">
              {t(locale, "lattice.proxyLabel")}
            </div>
          </div>
          <span className="rounded border border-lab-line px-1.5 py-0.5 text-[11px] text-lab-muted">
            f{frameIndex}
          </span>
        </div>
        <div className="mb-3 rounded border border-lab-line bg-black/15 px-2 py-2 text-xs leading-relaxed text-lab-muted">
          {t(locale, "lattice.notSoftBody")}
        </div>
        <div className="grid grid-cols-2 gap-2 xl:grid-cols-4">
          <Metric label={t(locale, "lattice.nodes")} value={summary.nodes.length} />
          <Metric label={t(locale, "lattice.edges")} value={summary.edges.length} />
          <Metric label={t(locale, "lattice.anchors")} value={summary.worldAnchorEdgeCount} />
          <Metric
            label={t(locale, "lattice.maxStretchRatio")}
            value={summary.maxStretchRatio == null ? "-" : summary.maxStretchRatio.toFixed(3)}
          />
          <Metric
            label={t(locale, "stability.solverRows")}
            value={summary.jointRowCount ?? "-"}
          />
          <Metric label={t(locale, "tree.contacts")} value={summary.contactCount} />
          <Metric
            label={t(locale, "stability.islands")}
            value={`${summary.activeIslandCount ?? "?"}/${summary.sleepingIslandCount ?? "?"}`}
          />
          <Metric
            label={t(locale, "inspector.latticeProxy")}
            value={summary.enabled ? t(locale, "stability.derived") : "-"}
          />
        </div>
      </div>
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-3">
        <div className="mb-2 text-xs font-semibold text-lab-text">
          {t(locale, "lattice.edges")}
        </div>
        <div className="max-h-52 space-y-1.5 overflow-auto pr-1">
          {summary.edges.slice(0, 18).map((edge) => (
            <div
              key={edge.joint.handle}
              className="grid grid-cols-[56px_1fr_72px] items-center gap-2 rounded border border-lab-line bg-black/15 px-2 py-1.5 text-[11px]"
            >
              <span className="font-mono text-lab-text">J{edge.joint.handle}</span>
              <span className="truncate text-lab-muted">
                {edge.joint.kind} · {edge.joint.bodies.join(" / ")}
              </span>
              <span className="text-right font-mono text-lab-text">
                {edge.stretchRatio == null ? "-" : edge.stretchRatio.toFixed(3)}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}

function TrajectorySliderRow({
  locale,
  label,
  min,
  max,
  step,
  value,
  onChange,
}: {
  locale: Locale
  label: string
  min: number
  max: number
  step: number
  value: number
  onChange: (value: number) => void
}) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between gap-2 text-[11px] uppercase tracking-normal text-lab-muted">
        <span>{label}</span>
        <span className="font-mono text-lab-text">{value}</span>
      </div>
      <div className="grid grid-cols-[1fr_72px] items-center gap-2">
        <Slider
          value={value}
          min={min}
          max={max}
          step={step}
          onValueChange={onChange}
        />
        <Input
          type="number"
          min={min}
          max={max}
          step={step}
          value={value}
          aria-label={`${label} ${locale}`}
          onChange={(event) =>
            onChange(
              clampValue(
                Number(event.target.value) || min,
                min,
                max,
              ),
            )
          }
        />
      </div>
    </div>
  )
}

function FrameDiagnostics({
  locale,
  frame,
  previousFrame: _previousFrame,
}: {
  locale: Locale
  frame: FrameRecord | undefined
  previousFrame: FrameRecord | null
}) {
  if (!frame?.diagnostics) {
    return <EmptyState label={t(locale, "diagnostics.empty")} />
  }
  const diagnostics = frame.diagnostics
  const counterDelta = diagnostics.performance?.counter_delta ?? {}
  const stability = diagnostics.stability ?? {}
  const penetration = stability.penetration ?? {}
  const contactChurn = stability.contact_churn ?? {}
  const warmStart = stability.warm_start ?? {}
  const impulse = stability.impulse ?? {}
  const sleep = stability.sleep ?? {}
  const island = stability.island ?? {}
  const markers = diagnostics.markers ?? []
  const counterMetrics = [
    "broadphase_candidate_count",
    "broadphase_traversal_count",
    "broadphase_pruned_count",
    "contact_count",
    "island_count",
    "solver_row_count",
    "ccd_candidate_count",
  ] as const
  const missingEvidence = collectDiagnosticsMissingEvidence(locale, diagnostics)

  return (
    <div className="grid gap-3 lg:grid-cols-[1.05fr_0.95fr]">
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-2">
        <div className="mb-2 flex items-center justify-between gap-2">
          <div className="text-xs font-semibold text-lab-text">
            {t(locale, "diagnostics.exported")}
          </div>
          <SourceBadge locale={locale} source={counterDelta.source} />
        </div>
        <div className="mb-2 grid grid-cols-3 gap-2">
          <Metric
            label={t(locale, "metric.step")}
            value={frame.snapshot.stats.step_index ?? 0}
          />
          <Metric
            label={t(locale, "metric.simTime")}
            value={frame.simulated_time.toFixed(3)}
          />
          <Metric label={t(locale, "diagnostics.stateHash")} value={frame.state_hash} />
        </div>
        <div className="mb-3 rounded border border-lab-line/70 bg-black/15 p-2">
          <div className="mb-2 flex items-center justify-between gap-2">
            <div className="text-[11px] font-semibold text-lab-text">
              {t(locale, "diagnostics.performance")} / {t(locale, "diagnostics.counterDelta")}
            </div>
            <SourceBadge locale={locale} source={counterDelta.source} />
          </div>
          <div className="grid grid-cols-[1fr_88px] gap-1 text-xs">
            <span className="text-lab-muted">{t(locale, "fact.kind")}</span>
            <span className="text-right text-lab-muted">{t(locale, "diagnostics.delta")}</span>
            {counterMetrics.map((key) => (
              <DiagnosticsValueRow
                key={key}
                locale={locale}
                label={String(key)}
                value={formatCounterValue(locale, counterDelta[key] ?? null)}
              />
            ))}
          </div>
        </div>
        <div className="grid gap-2 md:grid-cols-2">
          <DiagnosticsFactCard
            locale={locale}
            title={t(locale, "diagnostics.penetration")}
            source={penetration.source}
            rows={[
              { label: "max_depth", value: formatDiagnosticNumber(locale, penetration.max_depth) },
              { label: "total_depth", value: formatDiagnosticNumber(locale, penetration.total_depth) },
              {
                label: "penetrating_contact_count",
                value: formatDiagnosticInteger(locale, penetration.penetrating_contact_count),
              },
            ]}
          />
          <DiagnosticsFactCard
            locale={locale}
            title={t(locale, "diagnostics.contactChurn")}
            source={contactChurn.source}
            rows={[
              { label: "entered", value: formatDiagnosticInteger(locale, contactChurn.entered) },
              { label: "persisted", value: formatDiagnosticInteger(locale, contactChurn.persisted) },
              { label: "exited", value: formatDiagnosticInteger(locale, contactChurn.exited) },
            ]}
          />
          <DiagnosticsFactCard
            locale={locale}
            title={t(locale, "diagnostics.warmStart")}
            source={warmStart.source}
            rows={[
              { label: "hit_count", value: formatDiagnosticInteger(locale, warmStart.hit_count) },
              { label: "miss_count", value: formatDiagnosticInteger(locale, warmStart.miss_count) },
              { label: "drop_count", value: formatDiagnosticInteger(locale, warmStart.drop_count) },
              {
                label: "counts_source",
                value: diagnosticSourceLabel(locale, warmStart.counts_source),
              },
              {
                label: "drop_reasons_source",
                value: diagnosticSourceLabel(
                  locale,
                  warmStart.drop_reasons_source,
                ),
              },
              {
                label: "drop_reasons",
                value: formatDiagnosticList(locale, warmStart.drop_reasons),
              },
            ]}
          />
          <DiagnosticsFactCard
            locale={locale}
            title={t(locale, "diagnostics.impulse")}
            source={impulse.source}
            rows={[
              {
                label: "total_normal_impulse",
                value: formatDiagnosticNumber(locale, impulse.total_normal_impulse),
              },
              {
                label: "total_tangent_impulse",
                value: formatDiagnosticNumber(locale, impulse.total_tangent_impulse),
              },
              {
                label: "max_normal_impulse",
                value: formatDiagnosticNumber(locale, impulse.max_normal_impulse),
              },
              {
                label: "max_tangent_impulse",
                value: formatDiagnosticNumber(locale, impulse.max_tangent_impulse),
              },
            ]}
          />
          <DiagnosticsFactCard
            locale={locale}
            title={t(locale, "diagnostics.sleep")}
            source={sleep.source}
            rows={[
              {
                label: "awake_dynamic_body_count",
                value: formatDiagnosticInteger(locale, sleep.awake_dynamic_body_count),
              },
              {
                label: "sleeping_dynamic_body_count",
                value: formatDiagnosticInteger(locale, sleep.sleeping_dynamic_body_count),
              },
              {
                label: "transition_count",
                value: formatDiagnosticInteger(locale, sleep.transition_count),
              },
              {
                label: "transition_count_source",
                value: diagnosticSourceLabel(
                  locale,
                  sleep.transition_count_source,
                ),
              },
              {
                label: "reasons",
                value: formatDiagnosticList(locale, sleep.reasons),
              },
            ]}
          />
          <DiagnosticsFactCard
            locale={locale}
            title={t(locale, "diagnostics.island")}
            source={island.source}
            rows={[
              { label: "island_count", value: formatDiagnosticInteger(locale, island.island_count) },
              {
                label: "active_island_count",
                value: formatDiagnosticInteger(locale, island.active_island_count),
              },
              {
                label: "sleeping_island_skip_count",
                value: formatDiagnosticInteger(locale, island.sleeping_island_skip_count),
              },
              {
                label: "solver_body_slot_count",
                value: formatDiagnosticInteger(locale, island.solver_body_slot_count),
              },
              {
                label: "contact_row_count",
                value: formatDiagnosticInteger(locale, island.contact_row_count),
              },
              {
                label: "joint_row_count",
                value: formatDiagnosticInteger(locale, island.joint_row_count),
              },
            ]}
          />
        </div>
      </div>
      <div className="grid gap-3">
        <div className="rounded-md border border-lab-line bg-lab-panel2 p-2">
          <div className="mb-2 text-xs font-semibold text-lab-text">
            {t(locale, "diagnostics.markers")}
          </div>
          {markers.length === 0 ? (
            <EmptyState label={t(locale, "diagnostics.noEvents")} />
          ) : (
            <div className="space-y-2">
              {markers.map((marker, index) => (
                <DiagnosticsMarkerCard key={`${marker.kind}-${index}`} locale={locale} marker={marker} />
              ))}
            </div>
          )}
        </div>
        <div className="rounded-md border border-lab-line bg-lab-panel2 p-2">
          <div className="mb-2 text-xs font-semibold text-lab-text">
            {t(locale, "diagnostics.missingEvidence")}
          </div>
          {missingEvidence.length === 0 ? (
            <EmptyState label={diagnosticSourceLabel(locale, "missing")} />
          ) : (
            <div className="space-y-1">
              {missingEvidence.map((entry, index) => (
                <div
                  key={`${entry.kind}-${entry.detail ?? ""}-${index}`}
                  className="rounded border border-lab-line/70 bg-black/15 px-2 py-1 text-xs"
                >
                  <div className="text-lab-text">{missingEvidenceLabel(locale, entry.kind)}</div>
                  {entry.detail ? (
                    <div className="mt-0.5 font-mono text-lab-muted">{entry.detail}</div>
                  ) : null}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

function DiagnosticsValueRow({
  locale,
  label,
  value,
}: {
  locale: Locale
  label: string
  value: string
}) {
  return (
    <>
      <span className="truncate font-mono text-lab-muted">{label}</span>
      <span className="text-right font-mono tabular-nums text-lab-text">
        {value || formatCounterValue(locale, null)}
      </span>
    </>
  )
}

function formatDiagnosticNumber(
  locale: Locale,
  value: number | null | undefined,
  digits = 4,
): string {
  return typeof value === "number" && Number.isFinite(value)
    ? value.toFixed(digits)
    : diagnosticSourceLabel(locale, "missing")
}

function formatDiagnosticInteger(
  locale: Locale,
  value: number | null | undefined,
): string {
  return typeof value === "number" && Number.isFinite(value)
    ? String(value)
    : diagnosticSourceLabel(locale, "missing")
}

function formatDiagnosticList(
  locale: Locale,
  values: string[] | null | undefined,
): string {
  return values?.length ? values.join(", ") : diagnosticSourceLabel(locale, "missing")
}

function DiagnosticsFactCard({
  locale,
  title,
  source,
  rows,
}: {
  locale: Locale
  title: string
  source: DiagnosticSource | null | undefined
  rows: Array<{ label: string; value: string }>
}) {
  return (
    <div className="rounded border border-lab-line/70 bg-black/15 p-2">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="text-[11px] font-semibold text-lab-text">{title}</div>
        <SourceBadge locale={locale} source={source} />
      </div>
      <div className="grid grid-cols-[1fr_auto] gap-1 text-xs">
        {rows.map((row) => (
          <DiagnosticsValueRow
            key={`${title}-${row.label}`}
            locale={locale}
            label={row.label}
            value={row.value}
          />
        ))}
      </div>
    </div>
  )
}

function DiagnosticsMarkerCard({
  locale,
  marker,
}: {
  locale: Locale
  marker: DiagnosticMarker
}) {
  const missingMarkerEvidence = marker.missing_evidence ?? []
  return (
    <div className="rounded border border-lab-line/70 bg-black/15 p-2 text-xs">
      <div className="mb-1 flex flex-wrap items-center gap-1.5">
        <span className="font-semibold text-lab-text">
          {diagnosticMarkerLabel(locale, marker.kind)}
        </span>
        <span className="rounded border border-lab-line px-1.5 py-0.5 text-[10px] text-lab-muted">
          {diagnosticSeverityLabel(locale, marker.severity)}
        </span>
        <SourceBadge locale={locale} source={marker.source} />
        <span className="font-mono text-lab-muted">
          f{marker.frame_index ?? diagnosticSourceLabel(locale, "missing")}
        </span>
      </div>
      <div className="grid grid-cols-[110px_1fr] gap-1">
        <EvidenceRow
          label={t(locale, "diagnostics.threshold")}
          value={marker.threshold_name ?? diagnosticSourceLabel(locale, "missing")}
        />
        <EvidenceRow
          label={t(locale, "diagnostics.score")}
          value={formatDiagnosticNumber(locale, marker.score, 2)}
        />
        <EvidenceRow
          label={t(locale, "diagnostics.evidenceFields")}
          value={marker.evidence_fields?.join(", ") || "-"}
        />
        <EvidenceRow
          label={t(locale, "diagnostics.missingEvidence")}
          value={
            missingMarkerEvidence
              .map((kind) => missingEvidenceLabel(locale, kind))
              .join(", ") || "-"
          }
        />
      </div>
    </div>
  )
}

function SourceBadge({
  locale,
  source,
}: {
  locale: Locale
  source: DiagnosticSource | null | undefined
}) {
  return (
    <span className="rounded border border-lab-line px-1.5 py-0.5 text-[10px] text-lab-muted">
      {diagnosticSourceLabel(locale, source)}
    </span>
  )
}

function collectDiagnosticsMissingEvidence(
  locale: Locale,
  diagnostics: FrameDiagnosticsRecord,
): MissingEvidence[] {
  const seen = new Set<string>()
  const entries: MissingEvidence[] = []

  const push = (kind: MissingEvidenceKind | null | undefined, detail = "") => {
    if (!kind) {
      return
    }
    const key = `${kind}:${detail}`
    if (!seen.has(key)) {
      seen.add(key)
      entries.push({ kind, detail })
    }
  }

  ;(diagnostics.missing_evidence ?? []).forEach((entry) =>
    push(entry.kind, entry.detail),
  )
  ;(diagnostics.stability?.contact_churn?.missing_evidence ?? []).forEach((kind) =>
    push(kind, "diagnostics.stability.contact_churn"),
  )
  ;(diagnostics.markers ?? []).forEach((marker) => {
    ;(marker.missing_evidence ?? []).forEach((kind) =>
      push(kind, diagnosticMarkerLabel(locale, marker.kind)),
    )
  })

  return entries
}

function EvidencePanel({
  locale,
  source,
  sessionId,
  runId,
  frameIndex,
  selectedEntity,
  layers,
  canvasView,
  debugContextText,
  perfArtifact,
  perfStatus,
  onCopyDebugContext,
}: {
  locale: Locale
  source: SourceKind
  sessionId: string | null
  runId: string | null
  frameIndex: number
  selectedEntity: SelectedEntity | null
  layers: LayerState
  canvasView: CanvasDebugView | null
  debugContextText: string | null
  perfArtifact: PerfArtifact | null
  perfStatus: PerfEvidenceStatus
  onCopyDebugContext: () => void
}) {
  const summary = perfArtifact?.counter_summary ?? null
  const contextPreview = useMemo(
    () => summarizeDebugContext(debugContextText),
    [debugContextText],
  )
  const modeLabel =
    canvasView?.mode === "locked_core"
      ? t(locale, "canvas.modeLockedCore")
      : t(locale, "canvas.modeFree")
  const frameLabel = canvasView ? `${modeLabel} @ ${canvasView.zoom.toFixed(1)}` : "-"
  const centerLabel = canvasView
    ? `${canvasView.center.x.toFixed(2)}, ${canvasView.center.y.toFixed(2)}`
    : "-"
  const targetBoundsLabel = formatTargetBounds(canvasView?.targetBounds ?? null)
  const layerList = enabledLayerKeys(layers)
    .map((key) => layerLabel(locale, key))
    .join(", ")
  const selectionLabel = selectedEntity
    ? entityLabel(locale, selectedEntity.kind, selectedEntity.id)
    : "-"
  return (
    <div className="grid gap-3 lg:grid-cols-[1fr_1fr]">
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-2">
        <div className="mb-2 flex items-center justify-between gap-2">
          <div className="text-xs font-semibold text-lab-text">
            {t(locale, "evidence.perfSummary")}
          </div>
          <span className="rounded border border-lab-line px-1.5 py-0.5 text-[11px] text-lab-muted">
            {perfStatusLabel(locale, perfStatus)}
          </span>
        </div>
        {summary ? (
          <div className="grid grid-cols-2 gap-2">
            <Metric
              label="broadphase"
              value={formatCounterGroup(locale, [
                summary.total_broadphase_candidate_count,
                summary.total_broadphase_traversal_count,
                summary.total_broadphase_pruned_count,
              ])}
            />
            <Metric
              label="tree depth"
              value={formatCounterValue(locale, summary.max_broadphase_tree_depth ?? null)}
            />
            <Metric
              label="islands"
              value={formatCounterGroup(locale, [
                summary.total_island_count,
                summary.total_active_island_count,
                summary.total_sleeping_island_skip_count,
              ])}
            />
            <Metric
              label="solver rows"
              value={formatCounterGroup(locale, [
                summary.total_solver_body_slot_count,
                summary.total_contact_row_count,
                summary.total_joint_row_count,
              ])}
            />
            <Metric
              label="CCD"
              value={formatCounterGroup(locale, [
                summary.total_ccd_candidate_count,
                summary.total_ccd_hit_count,
              ])}
            />
            <Metric
              label={t(locale, "perf.elapsed")}
              value={formatCounterValue(locale, perfArtifact?.elapsed_micros ?? null)}
            />
          </div>
        ) : (
          <EmptyState
            label={
              source === "live"
                ? t(locale, "evidence.noArtifact")
                : t(locale, "perf.missing")
            }
          />
        )}
      </div>
      <div className="rounded-md border border-lab-line bg-lab-panel2 p-2">
        <div className="mb-2 flex items-center justify-between gap-2">
          <div className="text-xs font-semibold text-lab-text">
            {t(locale, "debug.copyContext")}
          </div>
          <Tooltip label={t(locale, "debug.copyContext")}>
            <button
              type="button"
              onClick={onCopyDebugContext}
              className="flex h-7 w-7 items-center justify-center rounded border border-lab-line text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text"
              aria-label={t(locale, "debug.copyContext")}
            >
              <ClipboardCopy className="h-3.5 w-3.5" />
            </button>
          </Tooltip>
        </div>
        <div className="grid grid-cols-[108px_1fr] gap-1 text-xs">
          <EvidenceRow label={t(locale, "evidence.source")} value={sourceLabel(locale, source)} />
          <EvidenceRow
            label={t(locale, "evidence.scenario")}
            value={contextPreview.scenario}
          />
          <EvidenceRow label={t(locale, "evidence.session")} value={sessionId ?? "-"} />
          <EvidenceRow label={t(locale, "evidence.run")} value={runId ?? "-"} />
          <EvidenceRow label={t(locale, "evidence.frame")} value={String(frameIndex)} />
          <EvidenceRow
            label={t(locale, "evidence.stateHash")}
            value={contextPreview.stateHash}
          />
          <EvidenceRow
            label={t(locale, "evidence.selection")}
            value={selectionLabel}
          />
          <EvidenceRow
            label={t(locale, "evidence.layers")}
            value={layerList.length > 0 ? layerList : "-"}
          />
          <EvidenceRow label={t(locale, "evidence.camera")} value={frameLabel} />
          <EvidenceRow label={t(locale, "evidence.center")} value={centerLabel} />
          <EvidenceRow
            label={t(locale, "evidence.target")}
            value={canvasView?.targetDescription ?? "-"}
          />
          <EvidenceRow
            label={t(locale, "evidence.targetBounds")}
            value={targetBoundsLabel}
          />
          <EvidenceRow
            label={t(locale, "evidence.damping")}
            value={canvasView ? booleanLabel(locale, canvasView.dampingEnabled) : "-"}
          />
          <EvidenceRow
            label={t(locale, "evidence.grid")}
            value={canvasView ? booleanLabel(locale, canvasView.grid) : "-"}
          />
          <EvidenceRow
            label={t(locale, "evidence.rulers")}
            value={canvasView ? booleanLabel(locale, canvasView.rulers) : "-"}
          />
          <EvidenceRow
            label={t(locale, "evidence.perfStatus")}
            value={perfStatusLabel(locale, perfStatus)}
          />
          <EvidenceRow
            label={t(locale, "evidence.trajectory")}
            value={contextPreview.trajectory}
          />
          <EvidenceRow
            label={t(locale, "evidence.stack")}
            value={contextPreview.stack}
          />
          <EvidenceRow
            label={t(locale, "evidence.diagnostics")}
            value={contextPreview.diagnostics}
          />
          <EvidenceRow
            label={t(locale, "evidence.lattice")}
            value={contextPreview.lattice}
          />
          <EvidenceRow
            label={t(locale, "evidence.perturbation")}
            value={contextPreview.perturbation}
          />
        </div>
        {debugContextText ? (
          <div className="mt-2">
            <div className="mb-1 text-[11px] text-lab-muted">
              {t(locale, "debug.contextPreview")}
            </div>
            <textarea
              readOnly
              value={debugContextText}
              aria-label={t(locale, "debug.contextPreview")}
              className="h-24 w-full resize-y rounded border border-lab-line bg-black/20 p-2 font-mono text-[11px] text-lab-muted outline-none"
            />
          </div>
        ) : null}
      </div>
    </div>
  )
}

function EvidenceRow({ label, value }: { label: string; value: string }) {
  return (
    <>
      <span className="text-lab-muted">{label}</span>
      <span className="truncate text-right font-mono text-lab-text">{value}</span>
    </>
  )
}

function EmptyState({ label }: { label: string }) {
  return (
    <div className="rounded border border-dashed border-lab-line px-2 py-3 text-center text-xs text-lab-muted">
      {label}
    </div>
  )
}

function enabledLayerKeys(layers: LayerState): LayerKey[] {
  return (Object.keys(layers) as LayerKey[]).filter((key) => layers[key])
}

function perfStatusLabel(locale: Locale, status: PerfEvidenceStatus): string {
  if (status === "available") {
    return t(locale, "perf.available")
  }
  if (status === "missing") {
    return t(locale, "perf.missing")
  }
  if (status === "live_unavailable") {
    return t(locale, "perf.liveUnavailable")
  }
  return t(locale, "evidence.noArtifact")
}

function booleanLabel(locale: Locale, value: boolean): string {
  return value ? t(locale, "common.true") : t(locale, "common.false")
}

function formatTargetBounds(bounds: CanvasDebugView["targetBounds"]): string {
  if (!bounds) {
    return "-"
  }
  return `${bounds.min.x.toFixed(2)},${bounds.min.y.toFixed(2)} -> ${bounds.max.x.toFixed(2)},${bounds.max.y.toFixed(2)}`
}

function summarizeDebugContext(debugContextText: string | null): {
  scenario: string
  stateHash: string
  trajectory: string
  stack: string
  diagnostics: string
  lattice: string
  perturbation: string
} {
  if (!debugContextText) {
    return {
      scenario: "-",
      stateHash: "-",
      trajectory: "-",
      stack: "-",
      diagnostics: "-",
      lattice: "-",
      perturbation: "-",
    }
  }

  try {
    const context = JSON.parse(debugContextText) as {
      scenario?: { name?: string; group?: string; description?: string }
      session?: { stateHash?: string }
      trajectory?: {
        settings?: { mode?: string; historyLength?: number; samplingStride?: number }
        overlay?: {
          bodyTrailCount?: number
          contactTrailCount?: number
          ccdTrailCount?: number
        }
        markers?: Array<{ kind?: string; frameIndex?: number }>
      }
      stackStability?: {
        contactCount?: number
        contactSpike?: number
        quietFrames?: number
        activeIslandCount?: number | null
        sleepingIslandCount?: number | null
      }
      diagnostics?: {
        summary?: {
          available?: boolean
          marker_count?: number | null
          marker_kinds?: string[]
          missing_evidence?: string[]
          counter_delta_source?: string | null
          top_marker_threshold?: string | null
        }
      }
      latticeProxy?: {
        enabled?: boolean
        nodeCount?: number
        edgeCount?: number
        maxStretchRatio?: number | null
      }
      perturbation?: {
        preview?: { actionId?: string; rejectionReason?: string | null } | null
        commit?: {
          accepted?: boolean
          rejectionReason?: string | null
          querySyncStatus?: string | null
        } | null
        currentFrameProvenance?: Array<{ actionId?: string }>
      }
    }
    const trajectoryCounts = context.trajectory?.overlay
    const trajectoryLabel = [
      "derived",
      context.trajectory?.settings?.mode ?? "mode?",
      context.trajectory?.settings?.historyLength != null
        ? `h${context.trajectory.settings.historyLength}`
        : null,
      context.trajectory?.settings?.samplingStride != null
        ? `s${context.trajectory.settings.samplingStride}`
        : null,
      trajectoryCounts
        ? `b${trajectoryCounts.bodyTrailCount ?? 0}/c${trajectoryCounts.contactTrailCount ?? 0}/ccd${trajectoryCounts.ccdTrailCount ?? 0}`
        : null,
      context.trajectory?.markers?.length
        ? `m${context.trajectory.markers.length}`
        : null,
    ]
      .filter(Boolean)
      .join(" ")
    const stack = context.stackStability
      ? `derived c${context.stackStability.contactCount ?? 0} spike+${context.stackStability.contactSpike ?? 0} quiet ${context.stackStability.quietFrames ?? 0} island ${context.stackStability.activeIslandCount ?? "?"}/${context.stackStability.sleepingIslandCount ?? "?"}`
      : "-"
    const diagnostics = context.diagnostics?.summary
      ? [
          context.diagnostics.summary.counter_delta_source
            ? `perf:${context.diagnostics.summary.counter_delta_source}`
            : null,
          context.diagnostics.summary.available === false ? "missing" : null,
          context.diagnostics.summary.marker_count != null
            ? `m${context.diagnostics.summary.marker_count}`
            : null,
          context.diagnostics.summary.marker_kinds?.length
            ? `top:${context.diagnostics.summary.marker_kinds.join("/")}`
            : null,
          context.diagnostics.summary.top_marker_threshold
            ? `th:${context.diagnostics.summary.top_marker_threshold}`
            : null,
          context.diagnostics.summary.missing_evidence?.length
            ? `miss:${context.diagnostics.summary.missing_evidence.join("/")}`
            : null,
        ]
          .filter(Boolean)
          .join(" ")
      : "-"
    const lattice = context.latticeProxy?.enabled
      ? `proxy/not-soft-body n${context.latticeProxy.nodeCount ?? 0} e${context.latticeProxy.edgeCount ?? 0} stretch ${formatCompactNumber(context.latticeProxy.maxStretchRatio)}`
      : "proxy off / not-soft-body"
    const perturbationParts = [
      context.perturbation?.preview?.actionId
        ? `preview:${context.perturbation.preview.rejectionReason ?? "ready"}`
        : null,
      context.perturbation?.commit
        ? `commit:${context.perturbation.commit.accepted ? "accepted" : (context.perturbation.commit.rejectionReason ?? "pending")}`
        : null,
      context.perturbation?.commit?.querySyncStatus
        ? `query:${context.perturbation.commit.querySyncStatus}`
        : null,
      context.perturbation?.currentFrameProvenance?.length
        ? `prov:${context.perturbation.currentFrameProvenance.length}`
        : null,
    ]
      .filter(Boolean)
      .join(" ")

    return {
      scenario: [
        context.scenario?.name,
        context.scenario?.group,
        context.scenario?.description,
      ]
        .filter(Boolean)
        .join(" / ") || "-",
      stateHash: context.session?.stateHash ?? "-",
      trajectory: trajectoryLabel || "-",
      stack,
      diagnostics: diagnostics || "-",
      lattice,
      perturbation: perturbationParts || "-",
    }
  } catch {
    return {
      scenario: "-",
      stateHash: "-",
      trajectory: "-",
      stack: "-",
      diagnostics: "-",
      lattice: "-",
      perturbation: "-",
    }
  }
}

function formatCompactNumber(value: number | null | undefined): string {
  return typeof value === "number" ? value.toFixed(3) : "?"
}

function formatEvidenceMetric(locale: Locale, metric: EvidenceMetric): string {
  if (metric.value == null) {
    return t(locale, "stability.missing")
  }
  const value = metric.value.toFixed(3)
  return metric.missingCount > 0 ? `${value} + ?${metric.missingCount}` : value
}

function formatMaybeNumber(locale: Locale, value: number | null): string {
  return value == null ? t(locale, "stability.missing") : value.toFixed(3)
}

function formatPairMetric(
  locale: Locale,
  left: number | null,
  right: number | null,
): string {
  return left == null || right == null
    ? t(locale, "stability.missing")
    : `${left}/${right}`
}

function stackMarkerLabel(locale: Locale, kind: StackMarker["kind"]): string {
  switch (kind) {
    case "contactSpike":
      return t(locale, "stability.marker.contactSpike")
    case "wakeTransition":
      return t(locale, "stability.marker.wakeTransition")
    case "driftSpike":
      return t(locale, "stability.marker.driftSpike")
    case "angularDrift":
      return t(locale, "stability.marker.angularDrift")
    case "solverSpike":
      return t(locale, "stability.marker.solverSpike")
    case "quietWindow":
      return t(locale, "stability.marker.quietWindow")
  }
}

function trajectoryMarkerLabel(
  locale: Locale,
  kind: TrajectoryMarker["kind"],
): string {
  switch (kind) {
    case "bodyMotion":
      return t(locale, "trajectory.marker.bodyMotion")
    case "contactAppeared":
      return t(locale, "trajectory.marker.contactAppeared")
    case "contactDisappeared":
      return t(locale, "trajectory.marker.contactDisappeared")
    case "contactBurst":
      return t(locale, "trajectory.marker.contactBurst")
    case "ccdClamp":
      return t(locale, "trajectory.marker.ccdClamp")
    case "ccdHit":
      return t(locale, "trajectory.marker.ccdHit")
  }
}

function trajectoryEmptyLabel(
  locale: Locale,
  emptyState: NonNullable<ReturnType<typeof buildTrajectoryOverlay>["emptyState"]>,
): string {
  switch (emptyState) {
    case "selectedBody":
      return t(locale, "trajectory.empty.selectedBody")
    case "allDynamic":
      return t(locale, "trajectory.empty.allDynamic")
    case "selectedIsland":
      return t(locale, "trajectory.empty.selectedIsland")
    case "contacts":
      return t(locale, "trajectory.empty.contacts")
    case "ccd":
      return t(locale, "trajectory.empty.ccd")
    case "unsupportedSelection":
      return t(locale, "trajectory.empty.unsupportedSelection")
  }
}

function modeLabel(locale: Locale, mode: TrajectorySettings["mode"]): string {
  switch (mode) {
    case "selectedBody":
      return t(locale, "trajectory.mode.selectedBody")
    case "allDynamic":
      return t(locale, "trajectory.mode.allDynamic")
    case "selectedIsland":
      return t(locale, "trajectory.mode.selectedIsland")
    case "contacts":
      return t(locale, "trajectory.mode.contacts")
    case "ccd":
      return t(locale, "trajectory.mode.ccd")
  }
}

function clampValue(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value))
}

function formatCounterValue(
  locale: Locale,
  value: number | string | null,
): string {
  return value == null ? t(locale, "stability.missing") : String(value)
}

function formatCounterGroup(
  locale: Locale,
  values: Array<number | null | undefined>,
): string {
  return values
    .map((value) => formatCounterValue(locale, value ?? null))
    .join("/")
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
