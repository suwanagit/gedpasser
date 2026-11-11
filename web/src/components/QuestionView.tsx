import { AnimatePresence, motion, useReducedMotion } from 'motion/react'
import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react'
import { api, errorMessage } from '../api/client'
import type { AnswerResult } from '../api/types/AnswerResult'
import type { PracticeMode } from '../api/types/PracticeMode'
import type { PracticeQuestion } from '../api/types/PracticeQuestion'
import { Figure } from './Figure'
import { FlagDialog } from './FlagDialog'
import { Hack } from './LessonCard'
import { LessonPanel } from './LessonPanel'
import { MathText } from './MathText'

/**
 * One question: answer it, see the feedback (explanation, common mistake,
 * memory hack, XP), then move on. Used by practice, review and the journey.
 */
export function QuestionView({
  q,
  mode,
  nextLabel,
  onAnswered,
  onNext,
  onFlagged,
  showSkill = false,
}: {
  q: PracticeQuestion
  mode: PracticeMode
  nextLabel: string
  onAnswered?: (result: AnswerResult) => void
  onNext: () => void
  onFlagged: () => void
  showSkill?: boolean
}) {
  const [selected, setSelected] = useState<string | null>(null)
  const [typed, setTyped] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [answered, setAnswered] = useState<{ response: string; result: AnswerResult } | null>(null)
  const [lessonFor, setLessonFor] = useState<string | null>(null)
  const [flagging, setFlagging] = useState(false)
  const shownAt = useRef(performance.now())
  const hintUsed = useRef(false)
  const reduce = useReducedMotion()
  const closeLesson = useCallback(() => setLessonFor(null), [])
  const cancelFlag = useCallback(() => setFlagging(false), [])

  const submit = useCallback(async () => {
    if (answered || busy) return
    const response = q.kind === 'fill_in' ? typed.trim() : selected
    if (!response) return
    setBusy(true)
    setError(null)
    try {
      const result = await api.answer({
        question_id: q.id,
        response,
        time_ms: Math.round(performance.now() - shownAt.current),
        hint_used: hintUsed.current,
        mode,
      })
      setAnswered({ response, result })
      onAnswered?.(result)
    } catch (e) {
      setError(errorMessage(e))
    } finally {
      setBusy(false)
    }
  }, [answered, busy, q, typed, selected, mode, onAnswered])

  // Keyboard: 1–4 pick a choice, Enter checks or moves on.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const inField = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement
      if (!answered && q.kind !== 'fill_in' && !inField) {
        const n = Number(e.key)
        if (n >= 1 && n <= q.choices.length) setSelected(q.choices[n - 1])
        if (e.key === 'Enter' && selected) {
          e.preventDefault()
          void submit()
        }
      } else if (answered && e.key === 'Enter' && !inField) {
        e.preventDefault()
        onNext()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [answered, q, selected, submit, onNext])

  const enter = reduce ? false : { opacity: 0, y: 12 }
  const transition = { duration: reduce ? 0 : 0.2 }

  return (
    <motion.div initial={enter} animate={{ opacity: 1, y: 0 }} transition={transition}>
      <article className="card question">
        <div className="badges">
          {q.draft && <span className="badge draft">Draft – not reviewed</span>}
          {showSkill && <span className="badge">{q.skill_name}</span>}
          <span className="badge">Difficulty {q.difficulty}</span>
        </div>
        <p className="prompt">
          <MathText text={q.prompt} />
        </p>
        {q.figure && <Figure view={q.figure} />}
        {!answered && (
          <p className="refresher">
            Need a refresher?{' '}
            <button
              className="link"
              onClick={() => {
                hintUsed.current = true
                setLessonFor(q.principle.id)
              }}
            >
              {q.principle.name}
            </button>
          </p>
        )}

        {q.kind === 'fill_in' ? (
          <FillIn
            value={answered ? answered.response : typed}
            disabled={!!answered}
            onChange={setTyped}
            onSubmit={() => void submit()}
          />
        ) : (
          <div className="choices" role="radiogroup" aria-label="Answer choices">
            {q.choices.map((c, i) => {
              const isRight = !!answered && c === answered.result.answer
              const isPicked = (answered ? answered.response : selected) === c
              const cls = [
                'choice',
                isPicked ? 'picked' : '',
                answered && isRight ? 'right' : '',
                answered && isPicked && !isRight ? 'wrong' : '',
              ].join(' ')
              return (
                <button
                  key={c}
                  className={cls}
                  role="radio"
                  aria-checked={isPicked}
                  disabled={!!answered}
                  onClick={() => setSelected(c)}
                >
                  <span className="key" aria-hidden="true">
                    {i + 1}
                  </span>
                  <MathText text={c} />
                </button>
              )
            })}
          </div>
        )}
        {error && <p className="error">{error}</p>}
        {!answered && q.kind !== 'fill_in' && (
          <div className="actions">
            <button className="primary" disabled={!selected || busy} onClick={() => void submit()}>
              Check
            </button>
          </div>
        )}
        <p className="flag-row">
          <button className="link small-link" onClick={() => setFlagging(true)}>
            ⚑ Flag this question
          </button>
        </p>
      </article>

      <AnimatePresence>
        {answered && (
          <motion.div
            className={`card feedback ${answered.result.correct ? 'right' : 'wrong'}`}
            initial={enter}
            animate={{ opacity: 1, y: 0 }}
            transition={transition}
            role="status"
          >
            <div className="feedback-head">
              <h3>{answered.result.correct ? 'Correct!' : 'Not quite.'}</h3>
              <motion.span
                className="xp-pop"
                initial={reduce ? false : { scale: 0.6, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                transition={{ duration: reduce ? 0 : 0.3 }}
              >
                +{answered.result.xp} XP
              </motion.span>
            </div>
            {!answered.result.correct && (
              <p>
                The answer is <MathText text={answered.result.answer} className="strong" />.
              </p>
            )}
            <h4>How to solve it</h4>
            <p className="explanation">
              <MathText text={answered.result.explanation} />
            </p>
            {!answered.result.correct && (
              <>
                <h4>Common mistake</h4>
                <p className="explanation">
                  <MathText text={answered.result.common_mistake} />
                </p>
              </>
            )}
            {answered.result.hack && <Hack text={answered.result.hack} />}
            <p className="explanation principle-line">
              <span className="badge">{answered.result.principle.id}</span> {answered.result.principle.name}{' '}
              <button className="link" onClick={() => setLessonFor(answered.result.principle.id)}>
                Learn this →
              </button>
            </p>
            <div className="actions">
              <button className="primary" autoFocus onClick={onNext}>
                {nextLabel}
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {lessonFor && <LessonPanel principleId={lessonFor} onClose={closeLesson} />}
      {flagging && (
        <FlagDialog
          questionId={q.id}
          onDone={() => {
            setFlagging(false)
            onFlagged()
          }}
          onCancel={cancelFlag}
        />
      )}
    </motion.div>
  )
}

function FillIn({
  value,
  disabled,
  onChange,
  onSubmit,
}: {
  value: string
  disabled: boolean
  onChange: (v: string) => void
  onSubmit: () => void
}) {
  function submit(e: FormEvent) {
    e.preventDefault()
    onSubmit()
  }
  return (
    <form className="fill-in" onSubmit={submit}>
      <label>
        Your answer
        <input
          autoFocus
          inputMode="text"
          autoComplete="off"
          value={value}
          disabled={disabled}
          onChange={(e) => onChange(e.target.value)}
          placeholder="e.g. 12, 3.5 or 3/4"
        />
      </label>
      {!disabled && (
        <button type="submit" className="primary" disabled={!value.trim()}>
          Check
        </button>
      )}
    </form>
  )
}
