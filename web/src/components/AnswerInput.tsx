import type { PracticeQuestion } from '../api/types/PracticeQuestion'
import { Icon } from './Icon'
import { MathText } from './MathText'
import { parseAnswer as parse } from './answers'

/**
 * Answers for drop-down, order and sort questions. Everything is tap-based
 * so it works on a phone. `value` is the response as sent to the server (a
 * JSON string); `onChange('')` until the answer is complete.
 */
export function AnswerInput({
  q,
  value,
  onChange,
  disabled = false,
}: {
  q: PracticeQuestion
  value: string
  onChange: (v: string) => void
  disabled?: boolean
}) {
  if (q.kind === 'dropdown') return <Dropdowns q={q} value={value} onChange={onChange} disabled={disabled} />
  if (q.kind === 'order') return <Order q={q} value={value} onChange={onChange} disabled={disabled} />
  if (q.kind === 'sort') return <Sort q={q} value={value} onChange={onChange} disabled={disabled} />
  return null
}

type Props = { q: PracticeQuestion; value: string; onChange: (v: string) => void; disabled: boolean }

/** The prompt with a menu at each {{n}} marker. */
function Dropdowns({ q, value, onChange, disabled }: Props) {
  const picks = parse<string[]>(value, [])
  const draft = q.blanks.map((_, i) => picks[i] ?? '')
  const parts = q.prompt.split(/\{\{\s*(\d+)\s*\}\}/)
  const set = (i: number, v: string) => {
    const next = draft.map((p, j) => (j === i ? v : p))
    onChange(JSON.stringify(next))
  }
  return (
    <p className="prompt dropdown-prompt">
      {parts.map((part, i) => {
        if (i % 2 === 0) return <MathText key={i} text={part} />
        const n = Number(part) - 1
        return (
          <select key={i} className="blank" aria-label={`Blank ${n + 1}`} value={draft[n] ?? ''} disabled={disabled} onChange={(e) => set(n, e.target.value)}>
            <option value="">Select…</option>
            {(q.blanks[n] ?? []).map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        )
      })}
    </p>
  )
}

/** Tap items in order to build the sequence; tap a placed item to take it back. */
function Order({ q, value, onChange, disabled }: Props) {
  const placed = parse<string[]>(value, []).filter((i) => q.items.includes(i))
  const left = q.items.filter((i) => !placed.includes(i))
  const update = (next: string[]) => onChange(next.length ? JSON.stringify(next) : '')
  return (
    <div className="order">
      <ol className="order-placed" aria-label="Your order">
        {placed.map((item, i) => (
          <li key={item}>
            <button className="choice picked" disabled={disabled} onClick={() => update(placed.filter((x) => x !== item))}>
              <span className="key">{i + 1}</span>
              <MathText text={item} />
              {!disabled && <Icon name="x" size={16} />}
            </button>
          </li>
        ))}
        {placed.length === 0 && <li className="order-hint muted">Tap the items below in order.</li>}
      </ol>
      {left.length > 0 && (
        <div className="choices order-pool" aria-label="Items to place">
          {left.map((item) => (
            <button key={item} className="choice" disabled={disabled} onClick={() => update([...placed, item])}>
              <span className="key">
                <Icon name="plus" size={14} />
              </span>
              <MathText text={item} />
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

/** For each item, tap the group it belongs in. */
function Sort({ q, value, onChange, disabled }: Props) {
  const placed = parse<Record<string, string>>(value, {})
  const set = (item: string, group: string) => {
    const next = { ...placed, [item]: group }
    onChange(JSON.stringify(next))
  }
  return (
    <div className="sort">
      {q.items.map((item) => (
        <div key={item} className="sort-row">
          <span className="sort-item">
            <MathText text={item} />
          </span>
          <span className="seg" role="radiogroup" aria-label={`Group for ${item}`}>
            {q.groups.map((g) => (
              <button
                key={g}
                role="radio"
                aria-checked={placed[item] === g}
                className={placed[item] === g ? 'on' : ''}
                disabled={disabled}
                onClick={() => set(item, g)}
              >
                {g}
              </button>
            ))}
          </span>
        </div>
      ))}
    </div>
  )
}
