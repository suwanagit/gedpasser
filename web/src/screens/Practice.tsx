import { useCallback, useEffect, useState } from 'react'
import { api, errorMessage, type Scope } from '../api/client'
import type { PracticeQuestion } from '../api/types/PracticeQuestion'
import type { PrincipleRef } from '../api/types/PrincipleRef'
import { LessonPanel } from '../components/LessonPanel'
import { QuestionView } from '../components/QuestionView'

type Outcome = { principle: PrincipleRef; correct: boolean }

function scopeTitle(scope: Scope): string {
  if (scope.mode === 'mixed') return 'Mixed practice'
  if (scope.mode === 'review') return 'Review'
  return scope.name
}

/**
 * Practice from a skill, principle(s), a weighted mix, or due reviews. With
 * `setLength`, the session ends after that many answers with a summary.
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
  const [q, setQ] = useState<PracticeQuestion | null>(null)
  const [status, setStatus] = useState<'loading' | 'ready' | 'empty' | 'summary'>('loading')
  const [error, setError] = useState<string | null>(null)
  const [outcomes, setOutcomes] = useState<Outcome[]>([])
  const [xp, setXp] = useState(0)
  const [notice, setNotice] = useState<string | null>(null)
  const [lessonFor, setLessonFor] = useState<string | null>(null)
  const closeLesson = useCallback(() => setLessonFor(null), [])

  const load = useCallback(
    async (after?: string) => {
      setStatus('loading')
      setError(null)
      try {
        const { question } = await api.next(scope, after)
        setQ(question)
        setStatus(question ? 'ready' : 'empty')
      } catch (e) {
        setError(errorMessage(e))
      }
    },
    [scope],
  )

  useEffect(() => {
    void load()
  }, [load])

  const done = setLength !== null && outcomes.length >= setLength
  const next = useCallback(() => {
    if (done) setStatus('summary')
    else void load(q?.id)
  }, [done, load, q])

  const mode = scope.mode
  return (
    <section className="practice">
      <div className="practice-bar">
        <button className="link" onClick={onExit}>
          ← Back
        </button>
        <h2>{scopeTitle(scope)}</h2>
        <span className="muted" aria-live="polite">
          {setLength !== null && status !== 'summary'
            ? `Question ${Math.min(outcomes.length + (status === 'ready' && q && outcomes.length < setLength ? 1 : 0), setLength)} of ${setLength}`
            : outcomes.length > 0
              ? `${outcomes.filter((o) => o.correct).length} / ${outcomes.length} · +${xp} XP`
              : ''}
        </span>
      </div>
      {setLength !== null && status !== 'summary' && (
        <div className="set-progress" aria-hidden="true">
          <span style={{ width: `${(outcomes.length / setLength) * 100}%` }} />
        </div>
      )}

      {notice && (
        <p className="flash" role="status">
          {notice}{' '}
          <button className="link" onClick={() => setNotice(null)} aria-label="Dismiss">
            ×
          </button>
        </p>
      )}
      {error && (
        <div className="card">
          <p className="error">{error}</p>
          <button className="primary" onClick={() => void load()}>
            Try again
          </button>
        </div>
      )}
      {status === 'loading' && !error && <p className="muted">Loading…</p>}
      {status === 'empty' && (
        <div className="card">
          <p>{mode === 'review' ? 'Nothing is due for review right now. Nice work.' : 'There are no questions here yet.'}</p>
          <button className="primary" onClick={onExit}>
            Back
          </button>
        </div>
      )}
      {status === 'ready' && q && (
        <QuestionView
          key={q.id}
          q={q}
          mode={mode === 'review' ? 'review' : mode}
          showSkill={mode !== 'skill'}
          nextLabel={setLength !== null && outcomes.length >= setLength ? 'See results' : 'Next question'}
          onAnswered={(r) => {
            setOutcomes((o) => [...o, { principle: r.principle, correct: r.correct }])
            setXp((x) => x + r.xp)
          }}
          onNext={next}
          onFlagged={() => {
            setNotice('Thanks — this question is out of practice until it has been checked.')
            void load(q.id)
          }}
        />
      )}
      {status === 'summary' && (
        <SetSummary
          outcomes={outcomes}
          xp={xp}
          onLearn={setLessonFor}
          onPractice={(p) => onStart({ mode: 'principle', id: p.id, name: p.name }, setLength)}
          onAgain={() => onStart(scope, setLength)}
          onExit={onExit}
        />
      )}
      {lessonFor && <LessonPanel principleId={lessonFor} onClose={closeLesson} />}
    </section>
  )
}

/** End-of-set results: score, and the principles missed with ways to fix them. */
function SetSummary({
  outcomes,
  xp,
  onLearn,
  onPractice,
  onAgain,
  onExit,
}: {
  outcomes: Outcome[]
  xp: number
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
      <p className="muted">+{xp} XP</p>
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
          Done
        </button>
        <button className="primary" autoFocus onClick={onAgain}>
          Another set
        </button>
      </div>
    </section>
  )
}
