import { motion, useReducedMotion } from 'motion/react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { JourneyRun } from '../api/types/JourneyRun'
import type { JourneyView } from '../api/types/JourneyView'
import type { Plan } from '../api/types/Plan'
import type { PlanTask } from '../api/types/PlanTask'
import type { StageResult } from '../api/types/StageResult'
import type { StageView } from '../api/types/StageView'
import type { Stats } from '../api/types/Stats'
import { Confetti } from '../components/Confetti'
import { Icon, type IconName } from '../components/Icon'
import { CARD_PARTS, Hack, LessonCard } from '../components/LessonCard'
import { Mascot } from '../components/Mascot'
import { QuestionView } from '../components/QuestionView'
import { Ring } from '../components/Ring'
import { SessionShell } from '../components/SessionShell'
import { play } from '../sound'

export const DAILY_GOAL = 10

const TASK_ICON: Record<PlanTask['kind'], IconName> = {
  diagnostic: 'clock',
  power_set: 'bolt',
  review: 'refresh',
  full_test: 'clock',
  hacks: 'bulb',
  tactics: 'target',
  journey: 'route',
  set_date: 'calendar',
  rest: 'star',
}

const TASK_LABEL: Record<PlanTask['kind'], string> = {
  diagnostic: 'Short test',
  power_set: 'Power set',
  review: 'Review',
  full_test: 'Full test',
  hacks: 'Hacks',
  tactics: 'Test-day tips',
  journey: 'Journey',
  set_date: 'Set test date',
  rest: 'Rest up',
}

/** Home: the path, with today's goal beside it. One tap starts the next stage. */
export function Learn({
  stats,
  plan,
  onOpen,
  onTask,
}: {
  stats: Stats | null
  plan: Plan | null
  onOpen: (stageId: string) => void
  onTask: (t: PlanTask) => void
}) {
  const [view, setView] = useState<JourneyView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const currentRef = useRef<HTMLLIElement>(null)

  useEffect(() => {
    api.journey().then(setView).catch((e) => setError(errorMessage(e)))
  }, [])

  // Land on the stage that's up next.
  const loaded = !!view
  useEffect(() => {
    if (loaded) currentRef.current?.scrollIntoView({ block: 'center' })
  }, [loaded])

  if (error) return <p className="error">{error}</p>
  if (!view)
    return (
      <div className="center loading">
        <Mascot mood="think" />
      </div>
    )

  // A band is the run of stages up to and including a checkpoint.
  const bands: StageView[][] = [[]]
  for (const s of view.stages) {
    bands[bands.length - 1].push(s)
    if (s.checkpoint) bands.push([])
  }
  if (bands[bands.length - 1].length === 0) bands.pop()

  const today = stats?.answered_today ?? 0
  const tasks = (plan?.today ?? []).filter((t) => t.kind !== 'journey' && t.kind !== 'rest').slice(0, 3)
  let n = 0
  return (
    <div className="learn">
      <aside className="today panel">
        <Ring value={today / DAILY_GOAL} size={64} className="sun">
          {today >= DAILY_GOAL ? <Icon name="check" /> : `${today}/${DAILY_GOAL}`}
        </Ring>
        <div className="today-text">
          <h3>{today >= DAILY_GOAL ? 'Goal done!' : 'Today'}</h3>
          <div className="today-tasks">
            {tasks.map((t, i) => (
              <button key={i} className="task-chip" onClick={() => onTask(t)} title={t.detail}>
                <Icon name={TASK_ICON[t.kind]} size={18} />
                {TASK_LABEL[t.kind]}
              </button>
            ))}
          </div>
        </div>
      </aside>

      <div className="path-col">
        {bands.map((band, b) => {
          const done = band.filter((s) => s.status === 'done').length
          return (
            <section key={b} className={`band band-${b % 6}`}>
              <header className="band-head">
                <span>Part {b + 1}</span>
                <span className="band-count">
                  {done}/{band.length}
                </span>
              </header>
              <ol className="path">
                {band.map((s) => {
                  const i = n++
                  const isCurrent = s.id === view.current
                  const icon: IconName = s.status === 'locked' ? 'lock' : s.checkpoint ? 'flag' : s.status === 'done' ? 'check' : 'play'
                  return (
                    <li
                      key={s.id}
                      ref={isCurrent ? currentRef : undefined}
                      className={`node ${s.status}${isCurrent ? ' current' : ''}`}
                      style={{ ['--dx' as string]: `${Math.round(Math.sin(i * 0.9) * 64)}px` }}
                    >
                      {isCurrent && <span className="start-bubble">Start</span>}
                      <button className="node-btn" disabled={s.status === 'locked'} onClick={() => onOpen(s.id)} aria-label={`${s.title}, ${s.status}`}>
                        <Icon name={icon} size={isCurrent ? 34 : 28} fill={icon === 'play'} />
                      </button>
                      {s.status === 'done' && (
                        <span className="node-stars" aria-label={`${s.stars} of 3 stars`}>
                          {[0, 1, 2].map((k) => (
                            <span key={k} className={k < s.stars ? 'on' : ''}>
                              <Icon name="star" size={14} fill />
                            </span>
                          ))}
                        </span>
                      )}
                      <span className="node-title">{s.title}</span>
                    </li>
                  )
                })}
              </ol>
            </section>
          )
        })}
      </div>
    </div>
  )
}

type Phase =
  | { kind: 'loading' }
  | { kind: 'error'; message: string }
  | { kind: 'cards'; index: number; part: number }
  | { kind: 'quiz'; index: number }
  | { kind: 'finishing' }
  | { kind: 'result'; result: StageResult }

/** One stage: cards a piece at a time, then the quiz, then the result. */
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
  const reduce = useReducedMotion()

  useEffect(() => {
    api
      .startStage(stageId)
      .then((r) => {
        setRun(r)
        setPhase(r.cards.length > 0 ? { kind: 'cards', index: 0, part: 1 } : { kind: 'quiz', index: 0 })
      })
      .catch((e) => setPhase({ kind: 'error', message: errorMessage(e) }))
  }, [stageId])

  const finish = useCallback(async () => {
    if (!run) return
    setPhase({ kind: 'finishing' })
    try {
      const result = await api.finishStage(Number(run.run_id))
      if (result.passed) play('done')
      setPhase({ kind: 'result', result })
    } catch (e) {
      setPhase({ kind: 'error', message: errorMessage(e) })
    }
  }, [run])

  if (phase.kind === 'error')
    return (
      <SessionShell
        progress={null}
        onClose={onExit}
        footer={
          <button className="primary wide" onClick={onExit}>
            Back
          </button>
        }
      >
        <div className="center loading">
          <Mascot mood="think" />
          <p className="error">{phase.message}</p>
        </div>
      </SessionShell>
    )
  if (phase.kind === 'loading' || phase.kind === 'finishing' || !run)
    return (
      <SessionShell progress={null} onClose={onExit}>
        <div className="center loading">
          <Mascot mood="think" />
        </div>
      </SessionShell>
    )

  const cardSteps = run.cards.length * CARD_PARTS
  const total = cardSteps + run.questions.length

  if (phase.kind === 'cards') {
    const { index, part } = phase
    const lastPart = part >= CARD_PARTS
    const lastCard = index + 1 >= run.cards.length
    const next = () =>
      setPhase(!lastPart ? { kind: 'cards', index, part: part + 1 } : !lastCard ? { kind: 'cards', index: index + 1, part: 1 } : { kind: 'quiz', index: 0 })
    return (
      <SessionShell
        progress={(index * CARD_PARTS + part - 1) / total}
        onClose={onExit}
        footer={
          <button key={`${index}-${part}`} className="primary wide" autoFocus onClick={next}>
            {lastPart && lastCard ? 'Start quiz' : lastPart ? 'Next' : 'Continue'}
          </button>
        }
      >
        <motion.div key={index} initial={reduce ? false : { opacity: 0, x: 28 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: reduce ? 0 : 0.2 }}>
          <LessonCard card={run.cards[index]} parts={part} />
        </motion.div>
      </SessionShell>
    )
  }

  if (phase.kind === 'quiz') {
    const { index } = phase
    const advance = () => (index + 1 < run.questions.length ? setPhase({ kind: 'quiz', index: index + 1 }) : void finish())
    return (
      <QuestionView
        key={run.questions[index].id}
        q={run.questions[index]}
        mode="journey"
        progress={(cardSteps + index) / total}
        onClose={onExit}
        onNext={advance}
        onFlagged={advance}
      />
    )
  }

  const { result } = phase
  const again = () => onOpen(stageId)
  const nextStage = result.passed && result.next_stage ? result.next_stage : null
  return (
    <SessionShell
      progress={null}
      onClose={onExit}
      footer={
        <button className="primary wide" autoFocus onClick={nextStage ? () => onOpen(nextStage) : result.passed ? onExit : again}>
          {nextStage ? 'Next stage' : result.passed ? 'Done' : 'Try again'}
        </button>
      }
    >
      <ResultScreen
        passed={result.passed}
        stars={result.stars}
        title={result.passed ? (result.stars === 3 ? 'Perfect!' : 'Stage passed!') : 'So close!'}
        score={`${result.correct} / ${result.total}`}
        xp={result.bonus_xp}
        hack={result.hacks[0]?.hack}
      />
    </SessionShell>
  )
}

/** The end-of-session screen: mascot, stars, score, XP, one hack. */
export function ResultScreen({
  passed,
  stars,
  title,
  score,
  xp,
  hack,
}: {
  passed: boolean
  /** Leave out to hide the stars. */
  stars?: number
  title: string
  score: string
  xp: number
  hack?: string | null
}) {
  const reduce = useReducedMotion()
  const shownXp = useCountUp(xp)
  return (
    <div className="result center" role="status">
      {passed && <Confetti />}
      <Mascot mood={passed ? 'cheer' : 'think'} size={128} />
      {stars !== undefined && (
        <div className="result-stars" aria-label={`${stars} of 3 stars`}>
          {[0, 1, 2].map((i) => (
            <motion.span
              key={i}
              className={i < stars ? 'on' : ''}
              initial={reduce ? false : { scale: 0, rotate: -40 }}
              animate={{ scale: 1, rotate: 0 }}
              transition={reduce ? { duration: 0 } : { delay: 0.2 + 0.18 * i, type: 'spring', stiffness: 380, damping: 14 }}
            >
              <Icon name="star" size={52} fill />
            </motion.span>
          ))}
        </div>
      )}
      <h1>{title}</h1>
      <div className="result-stats">
        <span className="result-stat">
          <span className="eyebrow">Score</span>
          <strong>{score}</strong>
        </span>
        {xp > 0 && (
          <span className="result-stat sun">
            <span className="eyebrow">XP</span>
            <strong>+{shownXp}</strong>
          </span>
        )}
      </div>
      {hack && <Hack text={hack} />}
    </div>
  )
}

function useCountUp(target: number): number {
  const reduce = useReducedMotion()
  const [n, setN] = useState(reduce ? target : 0)
  useEffect(() => {
    if (reduce || target <= 0) {
      setN(target)
      return
    }
    const began = performance.now()
    let frame = requestAnimationFrame(function tick(now) {
      const t = Math.min(1, (now - began) / 700)
      setN(Math.round(target * t))
      if (t < 1) frame = requestAnimationFrame(tick)
    })
    return () => cancelAnimationFrame(frame)
  }, [target, reduce])
  return n
}
