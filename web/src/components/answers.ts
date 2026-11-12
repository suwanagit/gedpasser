import type { PracticeQuestion } from '../api/types/PracticeQuestion'

/** Read a structured response (a JSON string), or `fallback`. */
export function parseAnswer<T>(value: string, fallback: T): T {
  try {
    return value ? (JSON.parse(value) as T) : fallback
  } catch {
    return fallback
  }
}

/** True for the formats this component draws (multiple choice and fill-in are drawn by the caller). */
export function isStructured(q: PracticeQuestion): boolean {
  return q.kind === 'dropdown' || q.kind === 'order' || q.kind === 'sort'
}

/** Whether a structured answer is complete enough to check. */
export function complete(q: PracticeQuestion, value: string): boolean {
  if (!value) return false
  if (q.kind === 'dropdown') {
    const picks = parseAnswer<string[]>(value, [])
    return picks.length === q.blanks.length && picks.every((p) => p)
  }
  if (q.kind === 'order') return parseAnswer<string[]>(value, []).length === q.items.length
  if (q.kind === 'sort') return Object.keys(parseAnswer<Record<string, string>>(value, {})).length === q.items.length
  return value.trim() !== ''
}
