import { Panel, PanelGroup } from "react-resizable-panels"

import { t, type Locale, type StatusKind } from "../../i18n"
import type {
  FrameRecord,
  ScenarioDescriptor,
  SelectedEntity,
  WorkbenchLog,
} from "../../types"
import { BottomTimeline } from "./Timeline"
import { Inspector } from "./Inspector"
import { ResizeHandle } from "./ResizeHandle"
import { SceneHierarchy } from "./SceneHierarchy"
import { Toolbar } from "./Toolbar"
import { WorldCanvas } from "./WorldCanvas"
import type {
  ControlAction,
  LayerState,
  ResolvedSelection,
  SourceKind,
} from "./types"

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
  onRun,
  layers,
  onLayerChange,
  currentFrame,
  frames,
  frameIndex,
  onFrameChange,
  selectedEntity,
  selectedDetails,
  onSelectEntity,
  logs,
  frameCount,
  setFrameCount,
  useCustomGravity,
  setUseCustomGravity,
  gravityY,
  setGravityY,
  onControl,
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
  onRun: () => void
  layers: LayerState
  onLayerChange: (key: keyof LayerState, value: boolean) => void
  currentFrame: FrameRecord
  frames: FrameRecord[]
  frameIndex: number
  onFrameChange: (value: number) => void
  selectedEntity: SelectedEntity | null
  selectedDetails: ResolvedSelection
  onSelectEntity: (entity: SelectedEntity | null) => void
  logs: WorkbenchLog[]
  frameCount: number
  setFrameCount: (value: number) => void
  useCustomGravity: boolean
  setUseCustomGravity: (value: boolean) => void
  gravityY: number
  setGravityY: (value: number) => void
  onControl: (action: ControlAction) => void
}) {
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
        layers={layers}
        onLayerChange={onLayerChange}
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
                frames={frames}
                frameIndex={frameIndex}
                selected={selectedEntity}
                layers={layers}
                labels={{
                  frame: t(locale, "canvas.frame"),
                  colliders: t(locale, "canvas.colliders"),
                  contacts: t(locale, "canvas.contacts"),
                }}
                onSelect={onSelectEntity}
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
                frameCount={frameCount}
                setFrameCount={setFrameCount}
                useCustomGravity={useCustomGravity}
                setUseCustomGravity={setUseCustomGravity}
                gravityY={gravityY}
                setGravityY={setGravityY}
                locale={locale}
                onPlay={() => onControl("play")}
                onPause={() => onControl("pause")}
                onStep={() => onControl("step")}
                onReset={() => onControl("reset")}
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
