import {
  useEffect,
  useRef,
  useState,
  type MouseEvent,
  type ReactNode,
} from "react"
import { Check, ChevronRight, Copy } from "lucide-react"

import { cn } from "../../lib/utils"
import { vec } from "./format"

export function FactGroup({
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

export function Fact({
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

export function VectorFact({
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

export function VectorValue({ value }: { value: { x: number; y: number } }) {
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

  function handleClick(event: MouseEvent<HTMLButtonElement>) {
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
