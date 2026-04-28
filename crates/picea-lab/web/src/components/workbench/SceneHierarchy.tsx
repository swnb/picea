import { useMemo, type ReactNode } from "react"
import {
  Box,
  Braces,
  CircleDot,
  MousePointer2,
  Settings2,
  Square,
  Waypoints,
} from "lucide-react"

import { Badge } from "../ui/badge"
import { PanelHeader, PanelTitle } from "../ui/panel"
import {
  bodyTypeLabel,
  dynamicValueLabel,
  entityLabel,
  t,
  type Locale,
} from "../../i18n"
import { cn } from "../../lib/utils"
import type {
  DebugBody,
  DebugCollider,
  FrameRecord,
  SelectedEntity,
} from "../../types"

export function SceneHierarchy({
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
