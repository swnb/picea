import { type ReactNode } from "react"
import { CircleDot, Gauge, Layers, MousePointer2, Settings2, Square } from "lucide-react"

import { Badge } from "../ui/badge"
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
import type { DebugBody, DebugCollider, FrameRecord } from "../../types"
import { Fact, FactGroup, VectorFact, VectorValue } from "./Facts"
import { ccdQuad, traceStage, vec, warmStartTriplet } from "./format"
import type { ResolvedSelection } from "./types"

export function Inspector({
  frame,
  selected,
  locale,
}: {
  frame: FrameRecord
  locale: Locale
  selected: ResolvedSelection
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

function EntityInspector({
  selected,
  locale,
}: {
  locale: Locale
  selected: NonNullable<ResolvedSelection>
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
                  label={t(locale, "fact.ccdTargetKind")}
                  value={dynamicValueLabel(
                    locale,
                    selected.entity.ccd_trace.target_kind ?? "static",
                  )}
                  mono={false}
                />
                <Fact
                  label={t(locale, "fact.ccdTargetClamp")}
                  value={(selected.entity.ccd_trace.target_clamp ?? 0).toFixed(
                    5,
                  )}
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
