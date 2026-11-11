import { AnimatePresence, motion, useReducedMotion } from 'motion/react'
import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react'
import { api, errorMessage, type Scope } from '../api/client'
import type { AnswerResult } from '../api/types/AnswerResult'
import type { PracticeQuestion } from '../api/types/PracticeQuestion'
import type { PrincipleRef } from '../api/types/PrincipleRef'
import { FlagDialog } from '../components/FlagDialog'
import { LessonPanel } from '../components/LessonPanel'
import { MathText } from '../components/MathText'

type State =
  | { kind: 'loading' }
  | { kind: 'empty' }
  | { kind: 'error'; message: string }
  | { kind: 'asking'; q: PracticeQuestion }
  | { kind: 'answered'; q: PracticeQuestion; response: string; result: AnswerResult }
  | { kind: 'summary' }

type Outcome = { principle: PrincipleRef; correct: boolean }

function scopeTitle(scope: Scope): string {
  return scope.mode === 'mixed' ? 'Mixed practice' : scope.name
}

/**
 * Practice questions from a skill, a principle, or a weighted mix.
 * With `setLength`, the session ends after that many answers with a summary.
 */
export function Practice({
  scope,
  setLength,
  onExit,
  onStart,
}: {
  scope: Scope
  setLength: number | null
  onExit: () => void
  onStart: (scope: Scope, setLength: number | null) => void
}) {
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [selected, setSelected] = useState<string | null>(null)
  const [typed, setTyped] = useState('')
  const [busy, setBusy] = useState(false)
  const [tally, setTally] = useState({ right: 0, total: 0 })
  const [outcomes, setOutcomes] = useState<Outcome[]>([])
  const [lessonFor, setLessonFor] = useState<string | null>(null)
  const [flagging, setFlagging] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const shownAt = useRef(0)
  const hintUsed = useRef(false)
  const reduce = useReducedMotion()
  const closeLesson = useCallback(() => setLessonFor(null), [])
  const cancelFlag = useCallback(() => setFlagging(false), [])

  const load = useCallback(
    async (after?: string) => {
      setState({ kind: 'loading' })
      setSelected(null)
      setTyped('')
      hintUsed.current = false
      try {
        const { question } = await api.next(scope, after)
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
    [scope],
  )

  useEffect(() => {
    void load()
  }, [load])

  const submit = useCallback(async () => {
    if (state.kind !== 'asking' || busy) return
    const response = state.q.kind === 'fill_in' ? typed.trim() : selected
    if (!response) return
    setBusy(true)
    setNotice(null)
    try {
      const time_ms = Math.round(performance.now() - shownAt.current)
      const result = await api.answer({
        question_id: state.q.id,
        response,
        time_ms,
        hint_used: hintUsed.current,
        mode: scope.mode,
      })
      setTally((t) => ({ right: t.right + (result.correct ? 1 : 0), total: t.total + 1 }))
      setOutcomes((o) => [...o, { principle: result.principle, correct: result.correct }])
      setState({ kind: 'answered', q: state.q, response, result })
    } catch (e) {
      setState({ kind: 'error', message: errorMessage(e) })
    } finally {
      setBusy(false)
    }
  }, [state, typed, selected, busy, scope.mode])

  const setDone = setLength !== null && tally.total >= setLength

  const next = useCallback(() => {
    if (state.kind !== 'answered') return
    if (setDone) setState({ kind: 'summary' })
    else void load(state.q.id)
  }, [state, load, setDone])

  const flagged = useCallback(() => {
    setFlagging(false)
    setNotice('Thanks — this question is out of practice until it has been checked.')
    if (state.kind === 'asking' || state.kind === 'answered') void load(state.q.id)
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
          ← Dashboard
        </button>
        <h2>{scopeTitle(scope)}</h2>
        <span className="muted" aria-live="polite">
          {setLength !== null && state.kind !== 'summary'
            ? `Question ${Math.min(tally.total + (state.kind === 'answered' ? 0 : 1), setLength)} of ${setLength}`
            : tally.total > 0
              ? `${tally.right} / ${tally.total} this session`
              : ''}
        </span>
      </div>

      {state.kind === 'summary' && (
        <SetSummary
          outcomes={outcomes}
          onLearn={setLessonFor}
          onPractice={(p) => onStart({ mode: 'principle', id: p.id, name: p.name }, setLength)}
          onAgain={() => onStart(scope, setLength)}
          onExit={onExit}
        />
      )}

      {notice && (
        <p className="flash" role="status">
          {notice}{' '}
          <button className="link" onClick={() => setNotice(null)} aria-label="Dismiss">
            ×
          </button>
        </p>
      )}
      {state.kind === 'loading' && <p className="muted">Loading…</p>}
      {state.kind === 'error' && (
        <div className="card">
          <p className="error">{state.message}</p>
          <button className="primary" onClick={() => void load()}>
            Try again
          </button>
        </div>
      )}
      {state.kind === 'empty' && <p className="card">There are no questions here yet.</p>}

      <AnimatePresence mode="wait" initial={false}>
        {(state.kind === 'asking' || state.kind === 'answered') && (
          <motion.div key={state.q.id} initial={enter} animate={shown} exit={leave} transition={transition}>
            <article className="card question">
              <div className="badges">
                {state.q.draft && <span className="badge draft">Draft – not reviewed</span>}
                {scope.mode !== 'skill' && <span className="badge">{state.q.skill_name}</span>}
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
              <p className="flag-row">
                <button className="link small-link" onClick={() => setFlagging(true)}>
                  ⚑ Flag this question
                </button>
              </p>
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
                    {setDone ? 'See results' : 'Next question'}
                  </button>
                </div>
              </motion.div>
            )}
          </motion.div>
        )}
      </AnimatePresence>

      {lessonFor && <LessonPanel principleId={lessonFor} onClose={closeLesson} />}
      {flagging && (state.kind === 'asking' || state.kind === 'answered') && (
        <FlagDialog questionId={state.q.id} onDone={flagged} onCancel={cancelFlag} />
      )}
    </section>
  )
}

/** End-of-set results: score, and the principles missed with ways to fix them. */
function SetSummary({
  outcomes,
  onLearn,
  onPractice,
  onAgain,
  onExit,
}: {
  outcomes: Outcome[]
  onLearn: (principleId: string) => void
  onPractice: (p: PrincipleRef) => void
  onAgain: () => void
  onExit: () => void
}) {
  const right = outcomes.filter((o) => o.correct).length
  const missed = new Map<string, { p: PrincipleRef; count: number }>()
  for (const o of outcomes) {
    if (o.correct) continue
    const m = missed.get(o.principle.id)
    if (m) m.count += 1
    else missed.set(o.principle.id, { p: o.principle, count: 1 })
  }
  return (
    <section className="card summary" role="status">
      <h3>
        Set complete: {right} of {outcomes.length} correct
      </h3>
      {missed.size === 0 ? (
        <p>Nothing missed — nice work.</p>
      ) : (
        <>
          <h4>Worth another look</h4>
          <ul className="missed">
            {[...missed.values()].map(({ p, count }) => (
              <li key={p.id}>
                <span>
                  <span className="badge">{p.id}</span> {p.name}
                  {count > 1 ? ` (missed ${count})` : ''}
                </span>
                <span className="missed-actions">
                  <button className="link" onClick={() => onLearn(p.id)}>
                    Learn this
                  </button>
                  <button className="secondary small" onClick={() => onPractice(p)}>
                    Practice this
                  </button>
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
      <div className="actions">
        <button className="secondary" onClick={onExit}>
          Back to dashboard
        </button>
        <button className="primary" autoFocus onClick={onAgain}>
          Another set
        </button>
      </div>
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
