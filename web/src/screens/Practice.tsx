import { AnimatePresence, motion, useReducedMotion } from 'motion/react'
import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react'
import { api, errorMessage } from '../api/client'
import type { AnswerResult } from '../api/types/AnswerResult'
import type { PracticeQuestion } from '../api/types/PracticeQuestion'
import type { SkillSummary } from '../api/types/SkillSummary'
import { LessonPanel } from '../components/LessonPanel'
import { MathText } from '../components/MathText'

type State =
  | { kind: 'loading' }
  | { kind: 'empty' }
  | { kind: 'error'; message: string }
  | { kind: 'asking'; q: PracticeQuestion }
  | { kind: 'answered'; q: PracticeQuestion; response: string; result: AnswerResult }

export function Practice({ skill, onExit }: { skill: SkillSummary; onExit: () => void }) {
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [selected, setSelected] = useState<string | null>(null)
  const [typed, setTyped] = useState('')
  const [busy, setBusy] = useState(false)
  const [tally, setTally] = useState({ right: 0, total: 0 })
  const [lessonFor, setLessonFor] = useState<string | null>(null)
  const shownAt = useRef(0)
  const hintUsed = useRef(false)
  const reduce = useReducedMotion()
  const closeLesson = useCallback(() => setLessonFor(null), [])

  const load = useCallback(
    async (after?: string) => {
      setState({ kind: 'loading' })
      setSelected(null)
      setTyped('')
      hintUsed.current = false
      try {
        const { question } = await api.next(skill.id, after)
        if (!question) {
          setState({ kind: 'empty' })
          return
        }
        shownAt.current = performance.now()
        setState({ kind: 'asking', q: question })
      } catch (e) {
        setState({ kind: 'error', message: errorMessage(e) })
      }
    },
    [skill.id],
  )

  useEffect(() => {
    void load()
  }, [load])

  const submit = useCallback(async () => {
    if (state.kind !== 'asking' || busy) return
    const response = state.q.kind === 'fill_in' ? typed.trim() : selected
    if (!response) return
    setBusy(true)
    try {
      const time_ms = Math.round(performance.now() - shownAt.current)
      const result = await api.answer({ question_id: state.q.id, response, time_ms, hint_used: hintUsed.current })
      setTally((t) => ({ right: t.right + (result.correct ? 1 : 0), total: t.total + 1 }))
      setState({ kind: 'answered', q: state.q, response, result })
    } catch (e) {
      setState({ kind: 'error', message: errorMessage(e) })
    } finally {
      setBusy(false)
    }
  }, [state, typed, selected, busy])

  const next = useCallback(() => {
    if (state.kind === 'answered') void load(state.q.id)
  }, [state, load])

  // Keyboard: 1-4 pick a choice, Enter checks or moves on.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const inField = e.target instanceof HTMLInputElement
      if (state.kind === 'asking' && state.q.kind !== 'fill_in' && !inField) {
        const n = Number(e.key)
        if (n >= 1 && n <= state.q.choices.length) setSelected(state.q.choices[n - 1])
        if (e.key === 'Enter' && selected) {
          e.preventDefault()
          void submit()
        }
      } else if (state.kind === 'answered' && e.key === 'Enter') {
        e.preventDefault()
        next()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [state, selected, submit, next])

  const enter = reduce ? false : { opacity: 0, y: 12 }
  const shown = { opacity: 1, y: 0 }
  const leave = reduce ? { opacity: 1 } : { opacity: 0, y: -8 }
  const transition = { duration: reduce ? 0 : 0.2 }

  return (
    <section className="practice">
      <div className="practice-bar">
        <button className="link" onClick={onExit}>
          ← All skills
        </button>
        <h2>{skill.name}</h2>
        <span className="muted" aria-live="polite">
          {tally.total > 0 ? `${tally.right} / ${tally.total} this session` : ''}
        </span>
      </div>

      {state.kind === 'loading' && <p className="muted">Loading…</p>}
      {state.kind === 'error' && (
        <div className="card">
          <p className="error">{state.message}</p>
          <button className="primary" onClick={() => void load()}>
            Try again
          </button>
        </div>
      )}
      {state.kind === 'empty' && <p className="card">There are no questions for this skill yet.</p>}

      <AnimatePresence mode="wait" initial={false}>
        {(state.kind === 'asking' || state.kind === 'answered') && (
          <motion.div key={state.q.id} initial={enter} animate={shown} exit={leave} transition={transition}>
            <article className="card question">
              <div className="badges">
                {state.q.draft && <span className="badge draft">Draft – not reviewed</span>}
                <span className="badge">Difficulty {state.q.difficulty}</span>
              </div>
              <p className="prompt">
                <MathText text={state.q.prompt} />
              </p>
              {state.kind === 'asking' && (
                <p className="refresher">
                  Need a refresher?{' '}
                  <button
                    className="link"
                    onClick={() => {
                      hintUsed.current = true
                      setLessonFor(state.q.principle.id)
                    }}
                  >
                    {state.q.principle.name}
                  </button>
                </p>
              )}

              {state.q.kind === 'fill_in' ? (
                <FillIn
                  value={state.kind === 'answered' ? state.response : typed}
                  disabled={state.kind === 'answered'}
                  onChange={setTyped}
                  onSubmit={() => void submit()}
                />
              ) : (
                <div className="choices" role="radiogroup" aria-label="Answer choices">
                  {state.q.choices.map((c, i) => {
                    const answered = state.kind === 'answered'
                    const isRight = answered && c === state.result.answer
                    const isPicked = (answered ? state.response : selected) === c
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
                        disabled={answered}
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

              {state.kind === 'asking' && state.q.kind !== 'fill_in' && (
                <div className="actions">
                  <button className="primary" disabled={!selected || busy} onClick={() => void submit()}>
                    Check
                  </button>
                </div>
              )}
            </article>

            {state.kind === 'answered' && (
              <motion.div
                className={`card feedback ${state.result.correct ? 'right' : 'wrong'}`}
                initial={enter}
                animate={shown}
                transition={transition}
                role="status"
              >
                <h3>{state.result.correct ? 'Correct!' : 'Not quite.'}</h3>
                {!state.result.correct && (
                  <p>
                    The answer is <MathText text={state.result.answer} className="strong" />.
                  </p>
                )}
                <h4>How to solve it</h4>
                <p className="explanation">
                  <MathText text={state.result.explanation} />
                </p>
                <h4>Common mistake</h4>
                <p className="explanation">
                  <MathText text={state.result.common_mistake} />
                </p>
                <h4>Principle</h4>
                <p className="explanation">
                  This question tests <span className="badge">{state.result.principle.id}</span>{' '}
                  {state.result.principle.name}.{' '}
                  <button className="link" onClick={() => setLessonFor(state.result.principle.id)}>
                    Learn this →
                  </button>
                </p>
                <div className="actions">
                  <button className="primary" autoFocus onClick={next}>
                    Next question
                  </button>
                </div>
              </motion.div>
            )}
          </motion.div>
        )}
      </AnimatePresence>

      {lessonFor && <LessonPanel principleId={lessonFor} onClose={closeLesson} />}
    </section>
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
