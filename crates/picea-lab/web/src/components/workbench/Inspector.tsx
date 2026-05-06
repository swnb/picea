import { type ReactNode } from "react"
import { CircleDot, Gauge, Layers, MousePointer2, Settings2, Square } from "lucide-react"

import { Badge } from "../ui/badge"
import { Button } from "../ui/button"
import { Input } from "../ui/input"
import { PanelHeader, PanelTitle } from "../ui/panel"
import {
  bodyTypeLabel,
  booleanLabel,
  dynamicValueLabel,
  entityKindLabel,
  entityLabel,
  t,
  type EntityKind,
  type Locale,
} from "../../i18n"
import { cn } from "../../lib/utils"
import type {
  DebugBody,
  DebugCollider,
  FrameRecord,
  LivePerturbationProvenance,
  SelectedEntity,
} from "../../types"
import { Fact, FactGroup, VectorFact, VectorValue } from "./Facts"
import { ccdQuad, traceStage, vec, warmStartTriplet } from "./format"
import {
  buildStabilityContribution,
  type ContactStabilityContribution,
} from "./stackStability"
import { buildTrajectorySummary } from "./trajectory"
import { deriveLatticeProxy } from "./types"
import type {
  ResolvedSelection,
  TrajectorySettings,
  VelocityPerturbationPanelState,
} from "./types"

export function Inspector({
  frame,
  frames,
  frameIndex,
  selected,
  selectedEntity,
  trajectorySettings,
  velocityPerturbation,
  onVelocityPerturbationDeltaChange,
  onVelocityPerturbationPreview,
  onVelocityPerturbationSubmit,
  locale,
}: {
  frame: FrameRecord
  frames: FrameRecord[]
  frameIndex: number
  locale: Locale
  selected: ResolvedSelection
  selectedEntity: SelectedEntity | null
  trajectorySettings: TrajectorySettings
  velocityPerturbation: VelocityPerturbationPanelState
  onVelocityPerturbationDeltaChange: (axis: "x" | "y", value: string) => void
  onVelocityPerturbationPreview: () => void
  onVelocityPerturbationSubmit: () => void
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
            <EntityInspector
              frames={frames}
              frameIndex={frameIndex}
              selected={selected}
              selectedEntity={selectedEntity}
              trajectorySettings={trajectorySettings}
              locale={locale}
            />
          ) : (
            <EmptyInspector locale={locale} />
          )}
          <VelocityPerturbationCard
            frame={frame}
            velocityPerturbation={velocityPerturbation}
            onVelocityPerturbationDeltaChange={onVelocityPerturbationDeltaChange}
            onVelocityPerturbationPreview={onVelocityPerturbationPreview}
            onVelocityPerturbationSubmit={onVelocityPerturbationSubmit}
            locale={locale}
          />
          <StageFactsCard frame={frame} locale={locale} />
          <ProcessFactsCard frame={frame} locale={locale} />
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

function VelocityPerturbationCard({
  frame,
  velocityPerturbation,
  onVelocityPerturbationDeltaChange,
  onVelocityPerturbationPreview,
  onVelocityPerturbationSubmit,
  locale,
}: {
  frame: FrameRecord
  velocityPerturbation: VelocityPerturbationPanelState
  onVelocityPerturbationDeltaChange: (axis: "x" | "y", value: string) => void
  onVelocityPerturbationPreview: () => void
  onVelocityPerturbationSubmit: () => void
  locale: Locale
}) {
  const provenance = velocityPerturbation.target
    ? (frame.perturbation_provenance ?? []).filter(
        (entry) => entry.body_handle === velocityPerturbation.target?.bodyHandle,
      )
    : []
  const previewAccepted =
    velocityPerturbation.preview &&
    !velocityPerturbation.preview.rejection_reason
  const previewDisabled =
    velocityPerturbation.busy !== "idle" ||
    velocityPerturbation.unavailableReason !== null
  const submitDisabled =
    velocityPerturbation.busy !== "idle" ||
    velocityPerturbation.unavailableReason !== null ||
    !previewAccepted

  return (
    <section className="rounded-md border border-lab-line bg-lab-panel2/70 p-3">
      <div className="mb-2 flex items-center gap-2">
        <Gauge className="h-3.5 w-3.5 text-lab-accent" />
        <h3 className="flex-1 text-[11px] font-semibold uppercase tracking-wider text-lab-muted">
          {t(locale, "panel.velocityPerturbation")}
        </h3>
        <Badge
          tone={velocityPerturbation.unavailableReason ? "warn" : "accent"}
          className="tabular-nums"
        >
          {velocityPerturbation.enabled
            ? t(locale, "perturbation.pausedOnlyBadge")
            : t(locale, "perturbation.liveOnlyBadge")}
        </Badge>
      </div>
      <p className="mb-3 text-xs leading-relaxed text-lab-muted">
        {t(locale, "perturbation.absoluteVelocityHelp")}
      </p>
      <div className="space-y-3">
        <FactGroup title={t(locale, "panel.velocityPerturbation")}>
          <Fact
            label={t(locale, "perturbation.selectedBody")}
            value={
              velocityPerturbation.target
                ? entityLabel(locale, "body", velocityPerturbation.target.bodyHandle)
                : t(locale, "common.unknown")
            }
            mono={false}
          />
          <Fact
            label={t(locale, "fact.type")}
            value={
              velocityPerturbation.target
                ? bodyTypeLabel(locale, velocityPerturbation.target.bodyType)
                : t(locale, "common.unknown")
            }
            mono={false}
          />
          <Fact
            label={t(locale, "perturbation.worldRevision")}
            value={
              velocityPerturbation.worldRevision == null
                ? t(locale, "common.unknown")
                : String(velocityPerturbation.worldRevision)
            }
            mono={velocityPerturbation.worldRevision != null}
          />
          <Fact
            label={t(locale, "perturbation.sessionEpoch")}
            value={
              velocityPerturbation.sessionEpoch == null
                ? t(locale, "common.unknown")
                : String(velocityPerturbation.sessionEpoch)
            }
            mono={velocityPerturbation.sessionEpoch != null}
          />
          <Fact
            label={t(locale, "perturbation.frameIndex")}
            value={String(velocityPerturbation.frameIndex)}
          />
          {velocityPerturbation.target ? (
            <VectorFact
              label={t(locale, "fact.linearVelocity")}
              value={velocityPerturbation.target.currentVelocity}
            />
          ) : null}
        </FactGroup>

        <div className="grid grid-cols-2 gap-3">
          <label className="space-y-1">
            <span className="text-[11px] text-lab-muted">
              {t(locale, "perturbation.deltaX")}
            </span>
            <Input
              type="number"
              step="0.1"
              value={velocityPerturbation.deltaX}
              onChange={(event) =>
                onVelocityPerturbationDeltaChange("x", event.target.value)
              }
            />
          </label>
          <label className="space-y-1">
            <span className="text-[11px] text-lab-muted">
              {t(locale, "perturbation.deltaY")}
            </span>
            <Input
              type="number"
              step="0.1"
              value={velocityPerturbation.deltaY}
              onChange={(event) =>
                onVelocityPerturbationDeltaChange("y", event.target.value)
              }
            />
          </label>
        </div>

        <div className="flex gap-2">
          <Button
            variant="outline"
            className="flex-1"
            disabled={previewDisabled}
            onClick={onVelocityPerturbationPreview}
          >
            {t(locale, "perturbation.preview")}
          </Button>
          <Button
            className="flex-1"
            disabled={submitDisabled}
            onClick={onVelocityPerturbationSubmit}
          >
            {t(locale, "perturbation.submit")}
          </Button>
        </div>

        {velocityPerturbation.unavailableReason ? (
          <Fact
            label={t(locale, "perturbation.unavailable")}
            value={velocityPerturbation.unavailableReason}
            mono={false}
            muted
          />
        ) : null}
        {velocityPerturbation.requestError ? (
          <Fact
            label={t(locale, "perturbation.requestError")}
            value={velocityPerturbation.requestError}
            mono={false}
            muted
          />
        ) : null}

        {velocityPerturbation.preview ? (
          <PerturbationPreviewFacts
            preview={velocityPerturbation.preview}
            locale={locale}
          />
        ) : null}
        {velocityPerturbation.commit ? (
          <PerturbationCommitFacts
            commit={velocityPerturbation.commit}
            locale={locale}
          />
        ) : null}

        <FactGroup title={t(locale, "perturbation.currentFrameProvenance")}>
          {provenance.length === 0 ? (
            <Fact
              label={t(locale, "perturbation.currentFrameProvenance")}
              value={t(locale, "perturbation.noProvenance")}
              mono={false}
              muted
            />
          ) : (
            provenance.map((entry) => (
              <PerturbationProvenanceFacts
                key={`${entry.action_id}-${entry.session_epoch}-${entry.world_revision}`}
                entry={entry}
                locale={locale}
              />
            ))
          )}
        </FactGroup>
      </div>
    </section>
  )
}

function PerturbationPreviewFacts({
  preview,
  locale,
}: {
  preview: VelocityPerturbationPanelState["preview"]
  locale: Locale
}) {
  if (!preview) {
    return null
  }
  return (
    <FactGroup title={t(locale, "perturbation.previewResult")}>
      <PerturbationSharedFacts
        actionId={preview.action_id}
        worldRevision={preview.world_revision}
        sessionEpoch={preview.session_epoch}
        frameIndex={preview.frame_index}
        beforeVelocity={preview.before_velocity}
        delta={preview.requested_delta}
        targetVelocity={preview.computed_target_velocity}
        querySyncStatus={null}
        rejectionReason={preview.rejection_reason}
        locale={locale}
      />
    </FactGroup>
  )
}

function PerturbationCommitFacts({
  commit,
  locale,
}: {
  commit: VelocityPerturbationPanelState["commit"]
  locale: Locale
}) {
  if (!commit) {
    return null
  }
  return (
    <FactGroup title={t(locale, "perturbation.commitResult")}>
      <PerturbationSharedFacts
        actionId={commit.action_id}
        worldRevision={commit.world_revision}
        sessionEpoch={commit.session_epoch}
        frameIndex={commit.frame_index}
        beforeVelocity={commit.before_velocity}
        delta={commit.requested_delta}
        targetVelocity={commit.computed_target_velocity}
        querySyncStatus={commit.query_sync_status}
        rejectionReason={commit.rejection_reason}
        locale={locale}
      />
      <Fact
        label={t(locale, "perturbation.commitOutcome")}
        value={commit.accepted ? dynamicValueLabel(locale, "accepted") : t(locale, "common.false")}
        mono={false}
      />
    </FactGroup>
  )
}

function PerturbationProvenanceFacts({
  entry,
  locale,
}: {
  entry: LivePerturbationProvenance
  locale: Locale
}) {
  return (
    <div className="space-y-1.5 rounded border border-lab-line/60 bg-lab-panel/60 p-2">
      <PerturbationSharedFacts
        actionId={entry.action_id}
        worldRevision={entry.world_revision}
        sessionEpoch={entry.session_epoch}
        frameIndex={entry.frame_index}
        beforeVelocity={entry.before_velocity}
        delta={entry.requested_delta}
        targetVelocity={entry.computed_target_velocity}
        querySyncStatus={entry.query_sync_status}
        rejectionReason={null}
        locale={locale}
      />
      <Fact
        label={t(locale, "perturbation.commitOutcome")}
        value={dynamicValueLabel(locale, entry.commit_outcome)}
        mono={false}
      />
      <Fact
        label={t(locale, "perturbation.wakeIntent")}
        value={booleanLabel(locale, entry.wake_intent)}
        mono={false}
      />
    </div>
  )
}

function PerturbationSharedFacts({
  actionId,
  worldRevision,
  sessionEpoch,
  frameIndex,
  beforeVelocity,
  delta,
  targetVelocity,
  querySyncStatus,
  rejectionReason,
  locale,
}: {
  actionId: string
  worldRevision: number
  sessionEpoch: number
  frameIndex: number
  beforeVelocity: { x: number; y: number } | null
  delta: { x: number; y: number } | null
  targetVelocity: { x: number; y: number } | null
  querySyncStatus: string | null
  rejectionReason: string | null
  locale: Locale
}) {
  return (
    <>
      <Fact label={t(locale, "perturbation.actionId")} value={actionId} />
      <Fact
        label={t(locale, "perturbation.worldRevision")}
        value={String(worldRevision)}
      />
      <Fact
        label={t(locale, "perturbation.sessionEpoch")}
        value={String(sessionEpoch)}
      />
      <Fact
        label={t(locale, "perturbation.frameIndex")}
        value={String(frameIndex)}
      />
      <OptionalVectorFact
        label={t(locale, "perturbation.beforeVelocity")}
        value={beforeVelocity}
        locale={locale}
      />
      <OptionalVectorFact
        label={t(locale, "perturbation.delta")}
        value={delta}
        locale={locale}
      />
      <OptionalVectorFact
        label={t(locale, "perturbation.targetVelocity")}
        value={targetVelocity}
        locale={locale}
      />
      <Fact
        label={t(locale, "perturbation.querySync")}
        value={querySyncStatus ? dynamicValueLabel(locale, querySyncStatus) : t(locale, "common.unknown")}
        mono={false}
      />
      <Fact
        label={t(locale, "perturbation.rejectionReason")}
        value={
          dynamicValueLabel(locale, rejectionReason ?? "none")
        }
        mono={false}
        muted={Boolean(rejectionReason)}
      />
    </>
  )
}

function OptionalVectorFact({
  label,
  value,
  locale,
}: {
  label: string
  value: { x: number; y: number } | null
  locale: Locale
}) {
  if (!value) {
    return (
      <Fact
        label={label}
        value={t(locale, "common.unknown")}
        mono={false}
      />
    )
  }
  return <VectorFact label={label} value={value} />
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

function ProcessFactsCard({
  frame,
  locale,
}: {
  frame: FrameRecord
  locale: Locale
}) {
  const tree = frame.snapshot.broadphase_tree
  const islands = frame.snapshot.islands ?? []
  const provenance = frame.compound_provenance ?? []
  const leafCount = tree?.nodes.filter((node) => node.collider != null).length ?? 0

  return (
    <section className="rounded-md border border-lab-line bg-lab-panel2/70 p-3">
      <div className="mb-2 flex items-center gap-2">
        <Settings2 className="h-3.5 w-3.5 text-lab-accent" />
        <h3 className="flex-1 text-[11px] font-semibold uppercase tracking-wider text-lab-muted">
          {t(locale, "panel.processFacts")}
        </h3>
        <Badge tone="accent" className="tabular-nums">
          {provenance.length + islands.length + (tree?.nodes.length ?? 0)}
        </Badge>
      </div>
      <div className="space-y-3">
        <FactGroup title={t(locale, "inspector.broadphaseTree")}>
          <Fact
            label={t(locale, "inspector.treeShape")}
            value={
              tree
                ? `${tree.nodes.length} nodes / ${leafCount} leaves / depth ${tree.depth}`
                : t(locale, "inspector.treeEmpty")
            }
            mono={false}
          />
          <Fact
            label={t(locale, "inspector.treeCounters")}
            value={`${frame.snapshot.stats.broadphase_candidate_count}/${frame.snapshot.stats.broadphase_traversal_count ?? 0}/${frame.snapshot.stats.broadphase_pruned_count ?? 0}`}
          />
        </FactGroup>
        <FactGroup title={t(locale, "inspector.islandLifecycle")}>
          <Fact
            label={t(locale, "metric.step")}
            value={`${frame.snapshot.stats.island_count ?? islands.length}/${frame.snapshot.stats.active_island_count ?? 0}/${frame.snapshot.stats.sleeping_island_skip_count ?? 0}`}
          />
          {islands.length === 0 ? (
            <Fact
              label={t(locale, "inspector.islandLifecycle")}
              value={t(locale, "inspector.islandEmpty")}
              mono={false}
              muted
            />
          ) : (
            islands.map((island) => (
              <Fact
                key={island.id}
                label={`I${island.id}`}
                value={`${island.bodies.length} bodies / ${booleanLabel(locale, island.sleeping)} / ${dynamicValueLabel(locale, island.reason)}`}
                mono={false}
              />
            ))
          )}
        </FactGroup>
        <FactGroup title={t(locale, "inspector.compoundProvenance")}>
          {provenance.length === 0 ? (
            <Fact
              label={t(locale, "inspector.compoundProvenance")}
              value={t(locale, "inspector.provenanceEmpty")}
              mono={false}
              muted
            />
          ) : (
            provenance.map((entry) => (
              <div key={`${entry.authored_body_index}-${entry.body_handle ?? "none"}`} className="space-y-1.5 rounded border border-lab-line/60 bg-lab-panel/60 p-2">
                <Fact
                  label={t(locale, "inspector.authoredBody")}
                  value={`${entry.authored_body_index} -> ${entry.body_handle ?? t(locale, "common.unknown")}`}
                  mono={false}
                />
                <Fact label={t(locale, "inspector.material")} value={dynamicValueLabel(locale, entry.inherited_material)} mono={false} />
                <Fact label={t(locale, "inspector.filter")} value={dynamicValueLabel(locale, entry.inherited_filter)} mono={false} />
                <Fact label={t(locale, "inspector.density")} value={entry.inherited_density.toFixed(2)} />
                <Fact label={t(locale, "inspector.sensor")} value={booleanLabel(locale, entry.inherited_is_sensor)} mono={false} />
                <Fact
                  label={t(locale, "inspector.validationPath")}
                  value={entry.validation_path}
                />
                <Fact
                  label={t(locale, "inspector.generatedPieces")}
                  value={String(entry.pieces.length)}
                />
                {entry.pieces.map((piece) => (
                  <Fact
                    key={piece.generated_piece_index}
                    label={`${t(locale, "inspector.pieceOrder")} ${piece.generated_piece_index}`}
                    value={`${piece.collider_handle ?? t(locale, "common.unknown")} / ${formatPoseTriplet(piece.local_pose)} / ${piece.validation_path}`}
                    mono={false}
                  />
                ))}
              </div>
            ))
          )}
        </FactGroup>
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

function formatPoseTriplet([x, y, angle]: [number, number, number]) {
  return `${x.toFixed(2)}, ${y.toFixed(2)}, ${angle.toFixed(2)}`
}

function EntityInspector({
  frames,
  frameIndex,
  selected,
  selectedEntity,
  trajectorySettings,
  locale,
}: {
  frames: FrameRecord[]
  frameIndex: number
  locale: Locale
  selected: NonNullable<ResolvedSelection>
  selectedEntity: SelectedEntity | null
  trajectorySettings: TrajectorySettings
}) {
  const title = entityLabel(locale, selected.kind, entityId(selected))
  const frame = frames[Math.min(frameIndex, Math.max(0, frames.length - 1))]
  const contribution = buildStabilityContribution(frames, frameIndex, selected)
  const trajectorySummary = buildTrajectorySummary(
    frames,
    frameIndex,
    selectedEntity,
    trajectorySettings,
  )
  const latticeSummary = frame ? deriveLatticeProxy(frame, frames[0] ?? frame) : null
  return (
    <section className="overflow-hidden rounded-md border border-lab-line bg-lab-panel2 ring-1 ring-lab-accent/15">
      <EntityInspectorHeader
        title={title}
        kind={selected.kind}
        subtitle={selectedSubtitle(locale, selected)}
        locale={locale}
      />
      <div className="space-y-3 px-3 py-3">
        {contribution ? (
          <StabilityContributionGroup
            locale={locale}
            contribution={contribution}
          />
        ) : null}
        {trajectorySummary ? (
          <TrajectorySummaryGroup
            locale={locale}
            summary={trajectorySummary}
          />
        ) : null}
        {latticeSummary ? (
          <LatticeProxyGroup
            locale={locale}
            summary={latticeSummary}
            selected={selected}
          />
        ) : null}
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
                value={formatOptionalNumber(locale, selected.entity.solver_normal_impulse)}
                mono={selected.entity.solver_normal_impulse != null}
              />
              <Fact
                label={t(locale, "fact.solverTangent")}
                value={formatOptionalNumber(locale, selected.entity.solver_tangent_impulse)}
                mono={selected.entity.solver_tangent_impulse != null}
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
                  label={t(locale, "fact.ccdTargetKind")}
                  value={dynamicValueLabel(
                    locale,
                    selected.entity.ccd_trace.target_kind ?? "static",
                  )}
                  mono={false}
                />
                <Fact
                  label={t(locale, "fact.ccdTargetClamp")}
                  value={formatOptionalNumber(
                    locale,
                    selected.entity.ccd_trace.target_clamp,
                    5,
                  )}
                  mono={selected.entity.ccd_trace.target_clamp != null}
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
                  label={t(locale, "fact.ccdTargetSweptStart")}
                  value={
                    selected.entity.ccd_trace.target_swept_start ??
                    selected.entity.ccd_trace.swept_start
                  }
                />
                <VectorFact
                  label={t(locale, "fact.ccdTargetSweptEnd")}
                  value={
                    selected.entity.ccd_trace.target_swept_end ??
                    selected.entity.ccd_trace.swept_end
                  }
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
  selected: NonNullable<ResolvedSelection>,
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

function entityId(selected: NonNullable<ResolvedSelection>): number {
  return selected.kind === "contact"
    ? selected.entity.id
    : selected.entity.handle
}

function StabilityContributionGroup({
  locale,
  contribution,
}: {
  locale: Locale
  contribution: NonNullable<ReturnType<typeof buildStabilityContribution>>
}) {
  return (
    <FactGroup title={t(locale, "stability.contribution")}>
      {contribution.kind === "body" ? (
        <>
          <Fact
            label={t(locale, "stability.maxDrift")}
            value={formatOptionalNumber(locale, contribution.drift)}
            mono={contribution.drift != null}
          />
          <Fact
            label={t(locale, "stability.maxAngularDrift")}
            value={formatOptionalNumber(locale, contribution.angularDrift)}
            mono={contribution.angularDrift != null}
          />
          <Fact
            label={t(locale, "stability.jitter")}
            value={formatOptionalNumber(locale, contribution.jitterProxy)}
            mono={contribution.jitterProxy != null}
          />
          <Fact
            label={t(locale, "stability.contactFrames")}
            value={String(contribution.contactFrames)}
          />
          <Fact
            label={t(locale, "stability.impulses")}
            value={`${formatContributionMetric(locale, contribution.normalImpulseTotal)} / ${formatContributionMetric(locale, contribution.tangentImpulseTotal)}`}
            mono={false}
          />
          <Fact
            label={t(locale, "stability.islandId")}
            value={
              contribution.islandId == null
                ? t(locale, "stability.missing")
                : String(contribution.islandId)
            }
            mono={contribution.islandId != null}
          />
          <Fact
            label={t(locale, "fact.sleeping")}
            value={
              contribution.sleeping == null
                ? t(locale, "stability.missing")
                : booleanLabel(locale, contribution.sleeping)
            }
            mono={false}
          />
          <Fact
            label={t(locale, "stability.awakeTransitions")}
            value={String(contribution.awakeTransitions)}
          />
        </>
      ) : (
        <ContactContributionFacts locale={locale} contribution={contribution} />
      )}
    </FactGroup>
  )
}

function TrajectorySummaryGroup({
  locale,
  summary,
}: {
  locale: Locale
  summary: NonNullable<ReturnType<typeof buildTrajectorySummary>>
}) {
  return (
    <FactGroup title={t(locale, "group.trace")}>
      <Fact
        label={t(locale, "trajectory.summary.derived")}
        value={t(locale, "stability.derived")}
        mono={false}
      />
      <Fact
        label={t(locale, "trajectory.summary.mode")}
        value={trajectoryModeLabel(locale, summary.mode)}
        mono={false}
      />
      <Fact
        label={t(locale, "trajectory.summary.window")}
        value={`${summary.windowStart}-${summary.windowEnd}`}
      />
      <Fact
        label={t(locale, "trajectory.summary.distance")}
        value={formatOptionalNumber(locale, summary.distance)}
        mono={summary.distance != null}
      />
      <Fact
        label={t(locale, "trajectory.summary.maxSpeed")}
        value={formatOptionalNumber(locale, summary.maxSpeed)}
        mono={summary.maxSpeed != null}
      />
      <Fact
        label={t(locale, "trajectory.summary.awakeSleep")}
        value={formatAwakeSleep(locale, summary.awakeFrames, summary.sleepingFrames)}
        mono={false}
      />
      <Fact
        label={t(locale, "trajectory.summary.contactCount")}
        value={String(summary.contactCount)}
      />
      <Fact
        label={t(locale, "trajectory.summary.ccdEvidence")}
        value={String(summary.ccdEvidenceCount)}
      />
    </FactGroup>
  )
}

function LatticeProxyGroup({
  locale,
  summary,
  selected,
}: {
  locale: Locale
  summary: ReturnType<typeof deriveLatticeProxy>
  selected: NonNullable<ResolvedSelection>
}) {
  const selectedEdge =
    selected.kind === "joint"
      ? summary.edges.find((edge) => edge.joint.handle === selected.entity.handle)
      : null
  const selectedNode =
    selected.kind === "body"
      ? summary.nodes.find((node) => node.body.handle === selected.entity.handle)
      : null
  if (!summary.enabled && !selectedEdge && !selectedNode) {
    return null
  }

  return (
    <FactGroup title={t(locale, "panel.latticeProxy")}>
      <Fact
        label={t(locale, "lattice.proxyLabel")}
        value={t(locale, "lattice.notSoftBody")}
        mono={false}
      />
      <Fact label={t(locale, "lattice.nodes")} value={String(summary.nodes.length)} />
      <Fact label={t(locale, "lattice.edges")} value={String(summary.edges.length)} />
      <Fact
        label={t(locale, "lattice.anchors")}
        value={String(summary.worldAnchorEdgeCount)}
      />
      <Fact
        label={t(locale, "lattice.maxStretchRatio")}
        value={formatOptionalNumber(locale, summary.maxStretchRatio, 3)}
        mono={summary.maxStretchRatio != null}
      />
      <Fact
        label={t(locale, "stability.solverRows")}
        value={formatOptionalNumber(locale, summary.jointRowCount, 0)}
        mono={summary.jointRowCount != null}
      />
      <Fact label={t(locale, "tree.contacts")} value={String(summary.contactCount)} />
      <Fact
        label={t(locale, "stability.islands")}
        value={`${summary.activeIslandCount ?? "?"} / ${summary.sleepingIslandCount ?? "?"}`}
        mono={false}
      />
      {selectedNode ? (
        <>
          <Fact
            label={t(locale, "inspector.latticeProxy")}
            value={`${selectedNode.connectedEdges.length} ${t(locale, "lattice.edges")}`}
            mono={false}
          />
          <Fact
            label={t(locale, "lattice.anchors")}
            value={String(selectedNode.anchorEdgeCount)}
          />
          <Fact
            label={t(locale, "stability.islandId")}
            value={selectedNode.islandId == null ? t(locale, "stability.missing") : String(selectedNode.islandId)}
            mono={selectedNode.islandId != null}
          />
          <Fact
            label={t(locale, "trajectory.summary.maxSpeed")}
            value={selectedNode.speed.toFixed(3)}
          />
        </>
      ) : null}
      {selectedEdge ? (
        <>
          <Fact
            label={t(locale, "fact.kind")}
            value={dynamicValueLabel(locale, selectedEdge.joint.kind)}
            mono={false}
          />
          <Fact
            label={t(locale, "lattice.currentLength")}
            value={selectedEdge.currentLength.toFixed(3)}
          />
          <Fact
            label={t(locale, "lattice.referenceLength")}
            value={formatOptionalNumber(locale, selectedEdge.referenceLength, 3)}
            mono={selectedEdge.referenceLength != null}
          />
          <Fact
            label={t(locale, "lattice.stretchRatio")}
            value={formatOptionalNumber(locale, selectedEdge.stretchRatio, 3)}
            mono={selectedEdge.stretchRatio != null}
          />
        </>
      ) : null}
    </FactGroup>
  )
}

function ContactContributionFacts({
  locale,
  contribution,
}: {
  locale: Locale
  contribution: ContactStabilityContribution
}) {
  return (
    <>
      <Fact
        label={t(locale, "stability.bodyIds")}
        value={contribution.bodyIds.join(", ")}
      />
      <Fact
        label={t(locale, "stability.contactFrames")}
        value={String(contribution.contactFrames)}
      />
      <Fact
        label={t(locale, "stability.impulses")}
        value={`${formatContributionMetric(locale, contribution.normalImpulse)} / ${formatContributionMetric(locale, contribution.tangentImpulse)}`}
        mono={false}
      />
      <Fact
        label={t(locale, "stability.islands")}
        value={contribution.islandIds.map((id) => id ?? "?").join(" / ")}
        mono={false}
      />
      <Fact
        label={t(locale, "stability.sleepingBodies")}
        value={contribution.sleepingBodies.map((value) => value == null ? t(locale, "stability.missing") : booleanLabel(locale, value)).join(" / ")}
        mono={false}
      />
    </>
  )
}

function formatOptionalNumber(
  locale: Locale,
  value: number | undefined | null,
  digits = 4,
): string {
  return value == null ? t(locale, "stability.missing") : value.toFixed(digits)
}

function formatAwakeSleep(
  locale: Locale,
  awakeFrames: number | null,
  sleepingFrames: number | null,
): string {
  return awakeFrames == null || sleepingFrames == null
    ? t(locale, "stability.missing")
    : `${awakeFrames}/${sleepingFrames}`
}

function trajectoryModeLabel(
  locale: Locale,
  mode: TrajectorySettings["mode"],
): string {
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

function formatContributionMetric(
  locale: Locale,
  metric: {
    value: number | null
    missingCount: number
  },
): string {
  if (metric.value == null) {
    return t(locale, "stability.missing")
  }
  const value = metric.value.toFixed(3)
  return metric.missingCount > 0 ? `${value} + ?${metric.missingCount}` : value
}
