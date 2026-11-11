import { motion, useReducedMotion } from 'motion/react'
import { useEffect, useRef, useState, type FormEvent } from 'react'
import { api, errorMessage } from '../api/client'
import type { FlagReason } from '../api/types/FlagReason'

const REASONS: { value: FlagReason; label: string }[] = [
  { value: 'wrong_answer', label: 'The answer looks wrong' },
  { value: 'unclear', label: 'The question is confusing or could mean two things' },
  { value: 'typo', label: 'Typo or display problem' },
  { value: 'other', label: 'Something else' },
]

/**
 * Report a problem with a question. On success the question leaves practice
 * for everyone until it is checked.
 */
export function FlagDialog({
  questionId,
  onDone,
  onCancel,
}: {
  questionId: string
  onDone: () => void
  onCancel: () => void
}) {
  const [reason, setReason] = useState<FlagReason>('wrong_answer')
  const [note, setNote] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const firstRef = useRef<HTMLInputElement>(null)
  const reduce = useReducedMotion()

  useEffect(() => {
    const before = document.activeElement as HTMLElement | null
    firstRef.current?.focus()
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault()
        e.stopPropagation()
        onCancel()
      } else if (e.key !== 'Tab' && e.key !== 'Enter' && e.key !== ' ') {
        // Keep the practice screen's number shortcuts from firing underneath,
        // but let typing in the note box work.
        if (!(e.target instanceof HTMLTextAreaElement)) e.stopPropagation()
      } else if (e.key === 'Enter' && !(e.target instanceof HTMLTextAreaElement)) {
        e.stopPropagation()
      }
    }
    window.addEventListener('keydown', onKey, true)
    return () => {
      window.removeEventListener('keydown', onKey, true)
      before?.focus()
    }
  }, [onCancel])

  async function submit(e: FormEvent) {
    e.preventDefault()
    setBusy(true)
    setError(null)
    try {
      await api.flag({ question_id: questionId, reason, note })
      onDone()
    } catch (err) {
      setError(errorMessage(err))
      setBusy(false)
    }
  }

  return (
    <motion.div
      className="overlay"
      onClick={onCancel}
      initial={reduce ? false : { opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: reduce ? 0 : 0.15 }}
    >
      <motion.form
        className="lesson-panel flag-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="flag-title"
        onClick={(e) => e.stopPropagation()}
        onSubmit={submit}
        initial={reduce ? false : { y: 16, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        transition={{ duration: reduce ? 0 : 0.2 }}
      >
        <h2 id="flag-title">Flag this question</h2>
        <p className="muted">
          Thanks for helping. A flagged question is taken out of practice for everyone until it has been checked.
        </p>
        <fieldset className="reasons">
          <legend>What's wrong?</legend>
          {REASONS.map((r, i) => (
            <label key={r.value} className="reason">
              <input
                ref={i === 0 ? firstRef : undefined}
                type="radio"
                name="reason"
                value={r.value}
                checked={reason === r.value}
                onChange={() => setReason(r.value)}
              />
              {r.label}
            </label>
          ))}
        </fieldset>
        <label className="note">
          Details <span className="muted">(optional)</span>
          <textarea
            rows={3}
            maxLength={500}
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="For example: I get 26.30, not 26.35."
          />
        </label>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="actions">
          <button type="button" className="secondary" onClick={onCancel}>
            Cancel
          </button>
          <button type="submit" className="primary" disabled={busy}>
            {busy ? 'Sending…' : 'Send flag'}
          </button>
        </div>
      </motion.form>
    </motion.div>
  )
}
