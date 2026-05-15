import { useEffect, useState } from "react"
import { Panel, PanelGroup } from "react-resizable-panels"

import {
  entityLabel,
  t,
  type Locale,
  type OverlayPresetId,
  type StatusKind,
} from "../../i18n"
import type {
  FrameRecord,
  PerfArtifact,
  ScenarioParameterValue,
  ScenarioDescriptor,
  SelectedEntity,
  WorkbenchLog,
} from "../../types"
import { cn } from "../../lib/utils"
import { BottomTimeline } from "./Timeline"
import { Inspector } from "./Inspector"
import { ResizeHandle } from "./ResizeHandle"
import { RunSettingsPanel } from "./RunSettingsPanel"
import { SceneHierarchy } from "./SceneHierarchy"
import { Toolbar } from "./Toolbar"
import { WorldCanvas } from "./WorldCanvas"
import type { GravityVector } from "./GravityDial"
import type {
  ControlAction,
  CanvasDebugView,
  LayerState,
  LiveCadenceStatus,
  PerfEvidenceStatus,
  ResolvedSelection,
  RunMode,
  SourceKind,
  TrajectorySettings,
  VelocityPerturbationPanelState,
} from "./types"

type RightSidebarMode = "inspector" | "run"

export function WorkbenchLayout({
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
  perfArtifact,
  perfStatus,
  onRun,
  liveControlBusy,
  liveCadence,
  runMode,
  onRunModeChange,
  layers,
  onLayerChange,
  onApplyOverlayPreset,
  trajectorySettings,
  onTrajectorySettingsChange,
  currentFrame,
  frames,
  frameIndex,
  onFrameChange,
  selectedEntity,
  selectedDetails,
  onSelectEntity,
  canvasView,
  debugContextText,
  onCanvasViewChange,
  logs,
  frameCount,
  setFrameCount,
  sceneParamDraft,
  defaultSceneParams,
  runningSceneParams,
  effectiveSceneParams,
  useCustomGravity,
  setUseCustomGravity,
  gravityVector,
  setGravityVector,
  appliedGravityVector,
  canApplyGravity,
  canUndoGravity,
  gravityPatchBusy,
  gravityPatchError,
  onApplyGravity,
  onUndoGravity,
  onResetGravity,
  onSceneParamChange,
  onResetSceneParams,
  onRevertRunningSceneParams,
  velocityPerturbation,
  onVelocityPerturbationDeltaChange,
  onVelocityPerturbationPreview,
  onVelocityPerturbationSubmit,
  onControl,
  onCopyDebugContext,
}: {
  locale: Locale
  onLocaleChange: (locale: Locale) => void
  scenario: ScenarioDescriptor
  scenarios: ScenarioDescriptor[]
  selectedScenario: string
  onScenarioChange: (value: string) => void
  status: StatusKind
  source: SourceKind
  sessionId: string | null
  runId: string | null
  manifestArtifact: string | null
  finalSnapshotArtifact: string | null
  finalSnapshotStep: number | null
  perfArtifact: PerfArtifact | null
  perfStatus: PerfEvidenceStatus
  onRun: () => void
  liveControlBusy: boolean
  liveCadence: LiveCadenceStatus
  runMode: RunMode
  onRunModeChange: (value: RunMode) => void
  layers: LayerState
  onLayerChange: (key: keyof LayerState, value: boolean) => void
  onApplyOverlayPreset: (preset: OverlayPresetId) => void
  trajectorySettings: TrajectorySettings
  onTrajectorySettingsChange: (value: TrajectorySettings) => void
  currentFrame: FrameRecord
  frames: FrameRecord[]
  frameIndex: number
  onFrameChange: (value: number) => void
  selectedEntity: SelectedEntity | null
  selectedDetails: ResolvedSelection
  onSelectEntity: (entity: SelectedEntity | null) => void
  canvasView: CanvasDebugView | null
  debugContextText: string | null
  onCanvasViewChange: (view: CanvasDebugView) => void
  logs: WorkbenchLog[]
  frameCount: number
  setFrameCount: (value: number) => void
  sceneParamDraft: Record<string, ScenarioParameterValue>
  defaultSceneParams: Record<string, ScenarioParameterValue>
  runningSceneParams: Record<string, ScenarioParameterValue>
  effectiveSceneParams: Record<string, ScenarioParameterValue> | null
  useCustomGravity: boolean
  setUseCustomGravity: (value: boolean) => void
  gravityVector: GravityVector
  setGravityVector: (value: GravityVector) => void
  appliedGravityVector: GravityVector
  canApplyGravity: boolean
  canUndoGravity: boolean
  gravityPatchBusy: boolean
  gravityPatchError: string | null
  onApplyGravity: () => void
  onUndoGravity: () => void
  onResetGravity: () => void
  onSceneParamChange: (key: string, value: ScenarioParameterValue) => void
  onResetSceneParams: () => void
  onRevertRunningSceneParams: () => void
  velocityPerturbation: VelocityPerturbationPanelState
  onVelocityPerturbationDeltaChange: (axis: "x" | "y", value: string) => void
  onVelocityPerturbationPreview: () => void
  onVelocityPerturbationSubmit: () => void
  onControl: (action: ControlAction) => void
  onCopyDebugContext: () => void
}) {
  const [rightSidebarMode, setRightSidebarMode] = useState<RightSidebarMode>(
    selectedEntity ? "inspector" : "run",
  )

  useEffect(() => {
    setRightSidebarMode(selectedEntity ? "inspector" : "run")
  }, [selectedEntity])

  const rightSidebarModeSwitch = (
    <RightSidebarModeSwitch
      locale={locale}
      mode={rightSidebarMode}
      onModeChange={setRightSidebarMode}
    />
  )

  return (
    <div className="flex h-screen min-h-[720px] flex-col overflow-hidden bg-lab-canvas text-lab-text">
      <Toolbar
        locale={locale}
        onLocaleChange={onLocaleChange}
        scenario={scenario}
        scenarios={scenarios}
        selectedScenario={selectedScenario}
        onScenarioChange={onScenarioChange}
        status={status}
        source={source}
        sessionId={sessionId}
        runId={runId}
        manifestArtifact={manifestArtifact}
        finalSnapshotArtifact={finalSnapshotArtifact}
        finalSnapshotStep={finalSnapshotStep}
        onRun={onRun}
        runMode={runMode}
        onRunModeChange={onRunModeChange}
        layers={layers}
        onLayerChange={onLayerChange}
        onApplyOverlayPreset={onApplyOverlayPreset}
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
            onSelect={onSelectEntity}
            locale={locale}
          />
        </Panel>
        <ResizeHandle />
        <Panel defaultSize={56} minSize={35} className="min-w-[420px]">
          <PanelGroup direction="vertical">
            <Panel defaultSize={72} minSize={45} className="min-h-[320px]">
              <WorldCanvas
                frame={currentFrame}
                frames={frames}
                frameIndex={frameIndex}
                selected={selectedEntity}
                layers={layers}
                trajectorySettings={trajectorySettings}
                labels={{
                  frame: t(locale, "canvas.frame"),
                  colliders: t(locale, "canvas.colliders"),
                  contacts: t(locale, "canvas.contacts"),
                  zoom: t(locale, "canvas.zoom"),
                  scale: t(locale, "canvas.scale"),
                  mode: t(locale, "canvas.mode"),
                  modeFree: t(locale, "canvas.modeFree"),
                  modeLockedCore: t(locale, "canvas.modeLockedCore"),
                  fit: t(locale, "tooltip.canvasFit"),
                  lock: t(locale, "tooltip.canvasLockCore"),
                  unlock: t(locale, "tooltip.canvasUnlockCore"),
                  reset: t(locale, "tooltip.canvasReset"),
                  zoomIn: t(locale, "tooltip.canvasZoomIn"),
                  zoomOut: t(locale, "tooltip.canvasZoomOut"),
                  targetActive: t(locale, "canvas.targetActive"),
                  targetAllColliders: t(locale, "canvas.targetAllColliders"),
                  targetScene: t(locale, "canvas.targetScene"),
                  trajectoryEmptySelectedBody: t(
                    locale,
                    "trajectory.empty.selectedBody",
                  ),
                  trajectoryEmptyAllDynamic: t(
                    locale,
                    "trajectory.empty.allDynamic",
                  ),
                  trajectoryEmptySelectedIsland: t(
                    locale,
                    "trajectory.empty.selectedIsland",
                  ),
                  trajectoryEmptyContacts: t(
                    locale,
                    "trajectory.empty.contacts",
                  ),
                  trajectoryEmptyCcd: t(locale, "trajectory.empty.ccd"),
                  trajectoryEmptyUnsupportedSelection: t(
                    locale,
                    "trajectory.empty.unsupportedSelection",
                  ),
                  entity: (kind, id) => entityLabel(locale, kind, id),
                }}
                onSelect={onSelectEntity}
                onViewChange={onCanvasViewChange}
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
                onFrameChange={onFrameChange}
                logs={logs}
                locale={locale}
                source={source}
                status={status}
                sessionId={sessionId}
                runId={runId}
                selectedEntity={selectedEntity}
                layers={layers}
                trajectorySettings={trajectorySettings}
                onTrajectorySettingsChange={onTrajectorySettingsChange}
                canvasView={canvasView}
                debugContextText={debugContextText}
                perfArtifact={perfArtifact}
                perfStatus={perfStatus}
                controlBusy={status === "loading" || liveControlBusy}
                liveCadence={liveCadence}
                onPlay={() => onControl("play")}
                onPause={() => onControl("pause")}
                onStep={() => onControl("step")}
                onReset={() => onControl("reset")}
                onCopyDebugContext={onCopyDebugContext}
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
          {rightSidebarMode === "inspector" ? (
            <Inspector
              frame={currentFrame}
              frames={frames}
              frameIndex={frameIndex}
              selected={selectedDetails}
              selectedEntity={selectedEntity}
              trajectorySettings={trajectorySettings}
              velocityPerturbation={velocityPerturbation}
              onVelocityPerturbationDeltaChange={
                onVelocityPerturbationDeltaChange
              }
              onVelocityPerturbationPreview={onVelocityPerturbationPreview}
              onVelocityPerturbationSubmit={onVelocityPerturbationSubmit}
              locale={locale}
              headerActions={rightSidebarModeSwitch}
            />
          ) : (
            <RunSettingsPanel
              locale={locale}
              scenario={scenario}
              frameCount={frameCount}
              setFrameCount={setFrameCount}
              runMode={runMode}
              setRunMode={onRunModeChange}
              source={source}
              sessionId={sessionId}
              status={status}
              sceneParamDraft={sceneParamDraft}
              defaultSceneParams={defaultSceneParams}
              runningSceneParams={runningSceneParams}
              effectiveSceneParams={effectiveSceneParams}
              useCustomGravity={useCustomGravity}
              setUseCustomGravity={setUseCustomGravity}
              gravityVector={gravityVector}
              setGravityVector={setGravityVector}
              appliedGravityVector={appliedGravityVector}
              canApplyGravity={canApplyGravity}
              canUndoGravity={canUndoGravity}
              gravityPatchBusy={gravityPatchBusy}
              gravityPatchError={gravityPatchError}
              onApplyGravity={onApplyGravity}
              onUndoGravity={onUndoGravity}
              onResetGravity={onResetGravity}
              onSceneParamChange={onSceneParamChange}
              onResetSceneParams={onResetSceneParams}
              onRevertRunningSceneParams={onRevertRunningSceneParams}
              headerActions={rightSidebarModeSwitch}
            />
          )}
        </Panel>
      </PanelGroup>
    </div>
  )
}

function RightSidebarModeSwitch({
  locale,
  mode,
  onModeChange,
}: {
  locale: Locale
  mode: RightSidebarMode
  onModeChange: (mode: RightSidebarMode) => void
}) {
  return (
    <div
      role="tablist"
      aria-label={`${t(locale, "panel.inspector")} / ${t(
        locale,
        "timeline.runSetup",
      )}`}
      className="flex shrink-0 items-center rounded-md border border-lab-line bg-black/20 p-0.5"
    >
      {(["inspector", "run"] as const).map((item) => {
        const active = item === mode
        return (
          <button
            key={item}
            type="button"
            role="tab"
            aria-selected={active}
            className={cn(
              "h-6 rounded px-2 text-[11px] font-medium tracking-normal whitespace-nowrap transition-colors",
              active
                ? "bg-lab-accent/15 text-lab-accent shadow-sm"
                : "text-lab-muted hover:bg-white/5 hover:text-lab-text",
            )}
            onClick={() => onModeChange(item)}
          >
            {item === "inspector"
              ? t(locale, "panel.inspector")
              : t(locale, "timeline.runSetup")}
          </button>
        )
      })}
    </div>
  )
}
