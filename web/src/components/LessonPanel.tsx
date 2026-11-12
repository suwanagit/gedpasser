import { motion, useReducedMotion } from 'motion/react'
import { useEffect, useRef, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { Lesson } from '../api/types/Lesson'
import { Icon } from './Icon'
import { LessonCard } from './LessonCard'
import { LessonPlayer } from './LessonPlayer'
import { LessonText } from './LessonText'
import { MathText } from './MathText'

/**
 * One principle, shown as a dialog over the current screen: its full
 * lessons, then the Quick review card. Esc, the Close button, or clicking
 * outside closes it. A lesson opens full screen on top.
 */
export function LessonPanel({
  principleId,
  onClose,
  onPractice,
}: {
  principleId: string
  onClose: () => void
  /** When given, shows a "Practice this principle" button. */
  onPractice?: () => void
}) {
  const [lesson, setLesson] = useState<Lesson | null>(null)
  const [error, setError] = useState<string | null>(null)
  const closeRef = useRef<HTMLButtonElement>(null)
  const [playing, setPlaying] = useState<{ id: string; review: boolean } | null>(null)
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
      if (playing) return // the lesson on top handles its own keys
      if (e.key === 'Escape') {
        e.preventDefault()
        e.stopPropagation()
        onClose()
      }
    }
    // Capture phase so Enter/number shortcuts on the practice screen don't fire underneath.
    function block(e: KeyboardEvent) {
      if (!playing && e.key !== 'Escape' && e.key !== 'Tab') e.stopPropagation()
    }
    window.addEventListener('keydown', onKey, true)
    window.addEventListener('keydown', block, true)
    return () => {
      window.removeEventListener('keydown', onKey, true)
      window.removeEventListener('keydown', block, true)
      before?.focus()
    }
  }, [onClose, playing])

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
          <p className="eyebrow">{lesson?.skill_name}</p>
          <button ref={closeRef} className="icon-btn" onClick={onClose} aria-label="Close">
            <Icon name="x" />
          </button>
        </div>
        {error && <p className="error">{error}</p>}
        {!lesson && !error && <p className="muted">Loading…</p>}
        {lesson && (
          <>
            <h2 id="lesson-title">{lesson.name}</h2>
            {lesson.lessons.length > 0 && (
              <section className="lesson-list">
                <p className="eyebrow">Lessons</p>
                {lesson.lessons.map((l) => (
                  <div key={l.id} className="lesson-row">
                    <button className="row row-btn" onClick={() => setPlaying({ id: l.id, review: false })}>
                      <span className="lesson-play">
                        <Icon name="play" size={16} fill />
                      </span>
                      <span>
                        {l.title} {l.draft && <span className="pill draft">Draft</span>}
                      </span>
                      <Icon name="right" size={18} />
                    </button>
                    <button className="link small" onClick={() => setPlaying({ id: l.id, review: true })}>
                      Quick review
                    </button>
                  </div>
                ))}
              </section>
            )}
            {lesson.card ? (
              <div className="quick-review">
                <p className="eyebrow">{lesson.lessons.length > 0 ? 'Quick review: the whole topic' : 'Quick review'}</p>
                <LessonCard card={lesson.card} />
              </div>
            ) : (
              <p className="lesson-summary">
                <MathText text={lesson.summary} />
              </p>
            )}
            {lesson.lesson ? (
              <details className="more-detail" open={!lesson.card}>
                <summary>More detail</summary>
                {lesson.draft && <p className="pill draft">Draft</p>}
                <LessonText text={lesson.lesson} />
              </details>
            ) : (
              !lesson.card && <p className="muted">Lesson coming soon.</p>
            )}
            {onPractice && (
              <div className="actions">
                <button className="primary wide" onClick={onPractice}>
                  Practice
                </button>
              </div>
            )}
          </>
        )}
      </motion.div>
      {playing && (
        <div onClick={(e) => e.stopPropagation()}>
          <LessonPlayer
            lessonId={playing.id}
            startAtReview={playing.review}
            onClose={() => setPlaying(null)}
            onPractice={
              onPractice
                ? () => {
                    setPlaying(null)
                    onPractice()
                  }
                : undefined
            }
          />
        </div>
      )}
    </motion.div>
  )
}
