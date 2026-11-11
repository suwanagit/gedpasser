import type { Level } from '../api/types/Level'

export const LEVEL_LABEL: Record<Level, string> = {
  not_started: 'Not started',
  needs_work: 'Needs work',
  getting_there: 'Getting there',
  strong: 'Strong',
}

/**
 * A mastery bar with its level written out, so color is never the only cue.
 * No animation, so nothing to switch off for reduced motion.
 */
export function MasteryBar({
  mastery,
  level,
  compact = false,
}: {
  mastery: number | null
  level: Level
  compact?: boolean
}) {
  const pct = mastery === null ? 0 : Math.round(mastery * 100)
  const label = mastery === null ? LEVEL_LABEL[level] : `${LEVEL_LABEL[level]} · ${pct}%`
  return (
    <span className={`mastery ${compact ? 'compact' : ''}`}>
      <span
        className={`mastery-track level-${level}`}
        role="meter"
        aria-label="Mastery"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct}
        aria-valuetext={label}
      >
        <span className="mastery-fill" style={{ width: `${pct}%` }} />
      </span>
      <span className="mastery-label">{label}</span>
    </span>
  )
}
