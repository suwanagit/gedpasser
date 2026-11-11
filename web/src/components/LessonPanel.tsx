import { motion, useReducedMotion } from 'motion/react'
import { useEffect, useRef, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { Lesson } from '../api/types/Lesson'
import { LessonText } from './LessonText'
import { MathText } from './MathText'

/**
 * A lesson for one principle, shown as a dialog over the current screen.
 * Esc, the Close button, or clicking outside closes it.
 */
export function LessonPanel({ principleId, onClose }: { principleId: string; onClose: () => void }) {
  const [lesson, setLesson] = useState<Lesson | null>(null)
  const [error, setError] = useState<string | null>(null)
  const closeRef = useRef<HTMLButtonElement>(null)
  const reduce = useReducedMotion()

  useEffect(() => {
    let live = true
    api
      .lesson(principleId)
      .then((l) => live && setLesson(l))
      .catch((e) => live && setError(errorMessage(e)))
    return () => {
      live = false
    }
  }, [principleId])

  useEffect(() => {
    const before = document.activeElement as HTMLElement | null
    closeRef.current?.focus()
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault()
        e.stopPropagation()
        onClose()
      }
    }
    // Capture phase so Enter/number shortcuts on the practice screen don't fire underneath.
    function block(e: KeyboardEvent) {
      if (e.key !== 'Escape' && e.key !== 'Tab') e.stopPropagation()
    }
    window.addEventListener('keydown', onKey, true)
    window.addEventListener('keydown', block, true)
    return () => {
      window.removeEventListener('keydown', onKey, true)
      window.removeEventListener('keydown', block, true)
      before?.focus()
    }
  }, [onClose])

  return (
    <motion.div
      className="overlay"
      onClick={onClose}
      initial={reduce ? false : { opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: reduce ? 0 : 0.15 }}
    >
      <motion.div
        className="lesson-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="lesson-title"
        onClick={(e) => e.stopPropagation()}
        initial={reduce ? false : { y: 16, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        transition={{ duration: reduce ? 0 : 0.2 }}
      >
        <div className="lesson-head">
          <div>
            <span className="badge">{principleId}</span>
            {lesson && <span className="muted lesson-skill"> {lesson.skill_name}</span>}
          </div>
          <button ref={closeRef} className="secondary small" onClick={onClose}>
            Close
          </button>
        </div>
        {error && <p className="error">{error}</p>}
        {!lesson && !error && <p className="muted">Loading lesson…</p>}
        {lesson && (
          <>
            <h2 id="lesson-title">{lesson.name}</h2>
            <p className="lesson-summary">
              <MathText text={lesson.summary} />
            </p>
            {lesson.draft && <p className="badge draft">Draft lesson – not reviewed</p>}
            {lesson.lesson ? (
              <LessonText text={lesson.lesson} />
            ) : (
              <p className="muted">The full lesson for this principle is coming soon.</p>
            )}
          </>
        )}
      </motion.div>
    </motion.div>
  )
}
