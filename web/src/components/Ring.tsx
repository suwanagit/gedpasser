import type { ReactNode } from 'react'

/** A progress ring. `value` is 0–1; the label (or children) sits in the middle. */
export function Ring({
  value,
  size = 56,
  stroke = 7,
  className = '',
  children,
}: {
  value: number
  size?: number
  stroke?: number
  className?: string
  children?: ReactNode
}) {
  const r = (size - stroke) / 2
  const c = 2 * Math.PI * r
  const v = Math.max(0, Math.min(1, value))
  return (
    <span className={`ring ${className}`} style={{ width: size, height: size }}>
      <svg width={size} height={size} aria-hidden="true">
        <circle className="ring-track" cx={size / 2} cy={size / 2} r={r} strokeWidth={stroke} />
        <circle className="ring-fill" cx={size / 2} cy={size / 2} r={r} strokeWidth={stroke} strokeDasharray={c} strokeDashoffset={c * (1 - v)} />
      </svg>
      <span className="ring-label">{children}</span>
    </span>
  )
}
