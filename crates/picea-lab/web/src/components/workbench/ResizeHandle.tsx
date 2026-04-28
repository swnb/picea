import { PanelResizeHandle } from "react-resizable-panels"

import { cn } from "../../lib/utils"

export function ResizeHandle({ vertical = false }: { vertical?: boolean }) {
  return (
    <PanelResizeHandle
      className={cn(
        "shrink-0 bg-lab-line transition-colors hover:bg-lab-accent/70",
        vertical ? "h-1 w-full" : "h-full w-1",
      )}
    />
  )
}
