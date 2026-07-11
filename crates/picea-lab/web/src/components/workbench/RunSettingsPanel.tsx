import type { ReactNode } from "react"

import { Input } from "../ui/input"
import { PanelHeader, PanelTitle } from "../ui/panel"
import { Checkbox, Select } from "../ui/radix"
import type {
  GrabMode,
  GrabSettings,
  ScenarioDescriptor,
  ScenarioParameterValue,
} from "../../types"
import { t, type Locale, type StatusKind } from "../../i18n"
import { GravityDial, type GravityVector } from "./GravityDial"
import { ParameterPanel } from "./parameters/ParameterPanel"
import type { RunMode, SourceKind } from "./types"

export function RunSettingsPanel({
  locale,
  scenario,
  frameCount,
  setFrameCount,
  runMode,
  setRunMode,
  grabSettings,
  setGrabSettings,
  source,
  sessionId,
  status,
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
  headerActions,
}: {
  locale: Locale
  scenario: ScenarioDescriptor
  frameCount: number
  setFrameCount: (value: number) => void
  runMode: RunMode
  setRunMode: (value: RunMode) => void
  grabSettings: GrabSettings
  setGrabSettings: (value: GrabSettings) => void
  source: SourceKind
  sessionId: string | null
  status: StatusKind
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
  headerActions?: ReactNode
}) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PanelHeader className="gap-2">
        <PanelTitle className="min-w-0 truncate">
          {t(locale, "timeline.runSetup")}
        </PanelTitle>
        {headerActions}
      </PanelHeader>
      <div className="min-h-0 flex-1 overflow-auto">
        <div className="grid gap-4 px-3 pb-4 pt-3">
          <div className="grid gap-3">
            <label className="grid gap-1.5">
              <span className="text-xs text-lab-muted">
                {t(locale, "run.frameCount")}
              </span>
              <Input
                type="number"
                min={1}
                max={6000}
                value={frameCount}
                onChange={(event) =>
                  setFrameCount(Math.max(1, Number(event.target.value) || 1))
                }
              />
            </label>
            <label className="grid gap-1.5">
              <span className="text-xs text-lab-muted">
                {t(locale, "run.mode")}
              </span>
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
            </label>
            <label className="grid gap-1.5">
              <span className="text-xs text-lab-muted">
                {t(locale, "run.grabMode")}
              </span>
              <Select
                value={grabSettings.mode}
                onValueChange={(value) =>
                  setGrabSettings({ ...grabSettings, mode: value as GrabMode })
                }
                ariaLabel={t(locale, "run.grabMode")}
                items={[
                  { value: "spring", label: t(locale, "run.grabModeSpring") },
                  { value: "direct", label: t(locale, "run.grabModeDirect") },
                ]}
              />
            </label>
            <label className="grid gap-1.5">
              <span className="text-xs text-lab-muted">
                {t(locale, "run.grabStiffness")}
              </span>
              <Input
                type="number"
                min={2}
                max={420}
                step={1}
                value={grabSettings.stiffness}
                onChange={(event) =>
                  setGrabSettings({
                    ...grabSettings,
                    stiffness: Number(event.target.value) || 40,
                  })
                }
                aria-label={t(locale, "run.grabStiffness")}
              />
            </label>
            <div className="grid gap-1.5">
              <span className="text-xs text-lab-muted">
                {t(locale, "run.gravityOverride")}
              </span>
              <Checkbox
                checked={useCustomGravity}
                onCheckedChange={setUseCustomGravity}
                label={t(locale, "run.sendOverride")}
              />
            </div>
          </div>
          <div className="grid gap-2">
            <div className="text-xs text-lab-muted">
              {t(locale, "run.gravityVector")}
            </div>
            <GravityDial
              locale={locale}
              enabled={useCustomGravity}
              vector={gravityVector}
              onEnabledChange={setUseCustomGravity}
              onVectorChange={setGravityVector}
              showNextRunHint={source !== "demo" || status !== "paused"}
              appliedVector={appliedGravityVector}
              liveApplyAvailable={source === "live" && sessionId != null}
              canApply={canApplyGravity}
              canUndo={canUndoGravity}
              applyBusy={gravityPatchBusy}
              applyError={gravityPatchError}
              onApply={onApplyGravity}
              onUndo={onUndoGravity}
              onReset={onResetGravity}
            />
          </div>
          <ParameterPanel
            locale={locale}
            schema={scenario.parameter_schema ?? []}
            values={sceneParamDraft}
            defaultValues={defaultSceneParams}
            runningValues={runningSceneParams}
            effectiveValues={effectiveSceneParams}
            onChange={onSceneParamChange}
            onResetDefaults={onResetSceneParams}
            onRevertRunning={onRevertRunningSceneParams}
          />
        </div>
      </div>
    </div>
  )
}
