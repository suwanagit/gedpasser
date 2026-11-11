import { motion, useReducedMotion } from 'motion/react'
import { useCallback, useEffect, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { JourneyRun } from '../api/types/JourneyRun'
import type { JourneyView } from '../api/types/JourneyView'
import type { StageResult } from '../api/types/StageResult'
import type { StageView } from '../api/types/StageView'
import { Hack, LessonCard } from '../components/LessonCard'
import { QuestionView } from '../components/QuestionView'

/** The guided path: stages in order, each unlocked by passing the last. */
export function JourneyMap({ onOpen }: { onOpen: (stageId: string) => void }) {
  const [view, setView] = useState<JourneyView | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    api.journey().then(setView).catch((e) => setError(errorMessage(e)))
  }, [])
  if (error) return <p className="error">{error}</p>
  if (!view) return <p className="muted">Loading your journey…</p>
  const total = view.stages.length
  return (
    <section className="journey">
      <div className="journey-head">
        <h2>Your journey</h2>
        <span className="muted">
          {view.passed} of {total} stages
        </span>
      </div>
      <div className="set-progress" aria-hidden="true">
        <span style={{ width: `${(view.passed / total) * 100}%` }} />
      </div>
      <p className="muted intro">Short lessons, then a few questions. Easy first — it builds as you go.</p>
      <ol className="path">
        {view.stages.map((s, i) => (
          <li key={s.id} className={`path-node ${s.status}${s.checkpoint ? ' checkpoint' : ''}${s.id === view.current ? ' current' : ''}`} style={{ ['--offset' as string]: `${Math.sin(i * 0.9) * 22}%` }}>
            <button disabled={s.status === 'locked'} onClick={() => onOpen(s.id)} aria-label={`${s.title}, ${s.status}`}>
              <span className="node-dot" aria-hidden="true">
                {s.status === 'locked' ? '🔒' : s.status === 'done' ? '✓' : s.checkpoint ? '🏁' : i + 1}
              </span>
              <span className="node-text">
                <span className="node-title">{s.title}</span>
                <span className="node-meta">
                  {s.checkpoint ? `Checkpoint · ${s.size} questions` : `${s.principles.length} ideas · ${s.size} questions`}
                  {s.stars > 0 && <span className="stars"> {'★'.repeat(s.stars)}{'☆'.repeat(3 - s.stars)}</span>}
                </span>
              </span>
              {s.id === view.current && <span className="node-go">Start</span>}
            </button>
          </li>
        ))}
      </ol>
    </section>
  )
}

type Phase =
  | { kind: 'loading' }
  | { kind: 'error'; message: string }
  | { kind: 'cards'; index: number }
  | { kind: 'quiz'; index: number }
  | { kind: 'finishing' }
  | { kind: 'result'; result: StageResult }

/** One stage: read the cards, answer the quiz, see how it went. */
export function StageRunner({
  stageId,
  onExit,
  onOpen,
}: {
  stageId: string
  onExit: () => void
  onOpen: (stageId: string) => void
}) {
  const [run, setRun] = useState<JourneyRun | null>(null)
  const [phase, setPhase] = useState<Phase>({ kind: 'loading' })
  const [right, setRight] = useState(0)
  const reduce = useReducedMotion()

  useEffect(() => {
    api
      .startStage(stageId)
      .then((r) => {
        setRun(r)
        setPhase(r.cards.length > 0 ? { kind: 'cards', index: 0 } : { kind: 'quiz', index: 0 })
      })
      .catch((e) => setPhase({ kind: 'error', message: errorMessage(e) }))
  }, [stageId])

  const finish = useCallback(async () => {
    if (!run) return
    setPhase({ kind: 'finishing' })
    try {
      setPhase({ kind: 'result', result: await api.finishStage(Number(run.run_id)) })
    } catch (e) {
      setPhase({ kind: 'error', message: errorMessage(e) })
    }
  }, [run])

  if (phase.kind === 'loading') return <p className="muted">Getting the stage ready…</p>
  if (phase.kind === 'error')
    return (
      <div className="card">
        <p className="error">{phase.message}</p>
        <button className="primary" onClick={onExit}>
          Back to the journey
        </button>
      </div>
    )
  if (!run) return null
  const stage: StageView = run.stage

  return (
    <section className="stage">
      <div className="practice-bar">
        <button className="link" onClick={onExit}>
          ← Journey
        </button>
        <h2>{stage.title}</h2>
        <span className="muted">
          {phase.kind === 'cards' && `Lesson ${phase.index + 1} of ${run.cards.length}`}
          {phase.kind === 'quiz' && `Question ${phase.index + 1} of ${run.questions.length}`}
        </span>
      </div>

      {phase.kind === 'cards' && (
        <motion.div key={`card${phase.index}`} initial={reduce ? false : { opacity: 0, x: 24 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: reduce ? 0 : 0.2 }}>
          <LessonCard card={run.cards[phase.index]} />
          <div className="actions">
            {phase.index > 0 && (
              <button className="secondary" onClick={() => setPhase({ kind: 'cards', index: phase.index - 1 })}>
                Back
              </button>
            )}
            <button
              className="primary"
              autoFocus
              onClick={() =>
                setPhase(phase.index + 1 < run.cards.length ? { kind: 'cards', index: phase.index + 1 } : { kind: 'quiz', index: 0 })
              }
            >
              {phase.index + 1 < run.cards.length ? 'Next' : `Try ${run.questions.length} questions`}
            </button>
          </div>
        </motion.div>
      )}

      {phase.kind === 'quiz' && (
        <QuestionView
          key={run.questions[phase.index].id}
          q={run.questions[phase.index]}
          mode="journey"
          nextLabel={phase.index + 1 < run.questions.length ? 'Next question' : 'See how you did'}
          onAnswered={(r) => r.correct && setRight((n) => n + 1)}
          onNext={() => (phase.index + 1 < run.questions.length ? setPhase({ kind: 'quiz', index: phase.index + 1 }) : void finish())}
          onFlagged={() => (phase.index + 1 < run.questions.length ? setPhase({ kind: 'quiz', index: phase.index + 1 }) : void finish())}
        />
      )}

      {phase.kind === 'finishing' && <p className="muted">Scoring… ({right} right so far)</p>}

      {phase.kind === 'result' && (
        <StageResultCard result={phase.result} onRetry={() => onOpen(stageId)} onNext={phase.result.next_stage ? () => onOpen(phase.result.next_stage!) : undefined} onExit={onExit} />
      )}
    </section>
  )
}

function StageResultCard({
  result,
  onRetry,
  onNext,
  onExit,
}: {
  result: StageResult
  onRetry: () => void
  onNext?: () => void
  onExit: () => void
}) {
  const reduce = useReducedMotion()
  return (
    <section className={`card stage-result ${result.passed ? 'right' : 'wrong'}`} role="status">
      <div className="result-stars" aria-label={`${result.stars} of 3 stars`}>
        {[0, 1, 2].map((i) => (
          <motion.span
            key={i}
            className={i < result.stars ? 'star on' : 'star'}
            initial={reduce ? false : { scale: 0, rotate: -30 }}
            animate={{ scale: 1, rotate: 0 }}
            transition={{ delay: reduce ? 0 : 0.15 * i, duration: reduce ? 0 : 0.3 }}
          >
            ★
          </motion.span>
        ))}
      </div>
      <h3>
        {result.passed ? (result.stars === 3 ? 'Perfect!' : 'Stage passed!') : 'Almost — give it another go.'} {result.correct} of {result.total}
      </h3>
      {result.bonus_xp > 0 && <p className="xp-pop big">+{result.bonus_xp} XP stage bonus</p>}
      {!result.passed && <p className="muted">Pass with about two out of three right. The lesson cards are there to help.</p>}
      {result.hacks.length > 0 && (
        <>
          <h4>{result.missed.length > 0 ? 'Take these with you' : 'Hacks to remember'}</h4>
          {result.hacks.map((h) => (
            <div key={h.principle.id} className="hack-row">
              <span className="badge">{h.principle.id}</span> <strong>{h.title}</strong>
              <Hack text={h.hack} />
            </div>
          ))}
        </>
      )}
      <div className="actions">
        <button className="secondary" onClick={onExit}>
          Journey
        </button>
        {!result.passed || !onNext ? (
          <button className="primary" autoFocus onClick={onRetry}>
            Try again
          </button>
        ) : (
          <button className="primary" autoFocus onClick={onNext}>
            Next stage →
          </button>
        )}
      </div>
    </section>
  )
}
