import type { FrameRecord, SelectedEntity } from "../../types"
import type { ResolvedSelection } from "./types"

export function resolveSelection(
  frame: FrameRecord,
  selected: SelectedEntity | null,
): ResolvedSelection {
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
