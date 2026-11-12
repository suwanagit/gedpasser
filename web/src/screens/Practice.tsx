import { useCallback, useEffect, useState } from 'react'
import { api, errorMessage, type Scope } from '../api/client'
import type { Plan } from '../api/types/Plan'
import type { PracticeQuestion } from '../api/types/PracticeQuestion'
import type { PrincipleRef } from '../api/types/PrincipleRef'
import type { Progress } from '../api/types/Progress'
import type { Stats } from '../api/types/Stats'
import type { TestKind } from '../api/types/TestKind'
import type { TestSummary } from '../api/types/TestSummary'
import { Icon, type IconName } from '../components/Icon'
import { LessonPanel } from '../components/LessonPanel'
import { Mascot } from '../components/Mascot'
import { QuestionView } from '../components/QuestionView'
import { Ring } from '../components/Ring'
import { SessionShell } from '../components/SessionShell'
import { FocusList } from '../components/StudyLinks'
import { ComingSoon, SubHead, SubjectCard } from '../components/Subject'
import { play } from '../sound'
import { SUBJECTS, type Subject, type SubjectId } from '../subjects'
import { ResultScreen } from './Journey'

export const TEST_KINDS: { kind: TestKind; title: string; questions: number; minutes: number }[] = [
  { kind: 'short', title: 'Short', questions: 10, minutes: 25 },
  { kind: 'long', title: 'Long', questions: 25, minutes: 60 },
  { kind: 'full', title: 'Full', questions: 46, minutes: 115 },
]

/** Practice tab: three quick ways in, then pick a subject. */
export function PracticeMenu({
  stats,
  plan,
  onStart,
  onSubject,
}: {
  stats: Stats | null
  plan: Plan | null
  onStart: (scope: Scope, setLength?: number) => void
  onSubject: (id: SubjectId) => void
}) {
  const due = stats?.review_due ?? 0
  const weak = (plan?.priorities ?? []).slice(0, 5).map((p) => p.principle.id)
  const quick: { icon: IconName; title: string; sub: string; tone: string; badge?: number; disabled?: boolean; go: () => void }[] = [
    { icon: 'bolt', title: 'Quick 5', sub: '2 min', tone: 'sun', go: () => onStart({ mode: 'mixed' }, 5) },
    {
      icon: 'target',
      title: 'Weak spots',
      sub: '10 questions',
      tone: 'coral',
      go: () => onStart(weak.length > 0 ? { mode: 'principle', id: weak.join(','), name: 'Weak spots' } : { mode: 'mixed' }),
    },
    { icon: 'refresh', title: 'Review', sub: due > 0 ? `${due} due` : 'All caught up', tone: 'violet', badge: due, disabled: due === 0, go: () => onStart({ mode: 'review' }) },
  ]

  return (
    <div className="practice-menu">
      <div className="quick-grid">
        {quick.map((t) => (
          <button key={t.title} className={`tile ${t.tone}`} disabled={t.disabled} onClick={t.go}>
            <span className="tile-icon">
              <Icon name={t.icon} size={28} fill={t.icon === 'bolt'} />
            </span>
            <strong>{t.title}</strong>
            <span className="tile-sub">{t.sub}</span>
            {!!t.badge && <span className="tile-badge">{t.badge}</span>}
          </button>
        ))}
      </div>

      {plan && plan.priorities.length > 0 && (
        <>
          <h2>Focus on</h2>
          <FocusList items={plan.priorities.slice(0, 3)} onPractice={(p) => onStart({ mode: 'principle', id: p.principle.id, name: p.principle.name })} />
        </>
      )}

      <h2>Tests and skills</h2>
      <div className="subject-grid">
        {SUBJECTS.map((s) => (
          <SubjectCard key={s.id} subject={s} onClick={() => onSubject(s.id)} />
        ))}
      </div>
    </div>
  )
}

/** One subject's practice: its practice tests, then its skills. */
export function PracticeSubject({
  subject,
  onBack,
  onStart,
  onTest,
}: {
  subject: Subject
  onBack: () => void
  onStart: (scope: Scope, setLength?: number) => void
  onTest: (kind: TestKind) => void
}) {
  const [progress, setProgress] = useState<Progress | null>(null)
  const [history, setHistory] = useState<TestSummary[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!subject.ready) return
    api.progress().then(setProgress).catch((e) => setError(errorMessage(e)))
    api.tests().then(setHistory).catch(() => setHistory([]))
  }, [subject.ready])

  return (
    <div className="practice-menu">
      <SubHead subject={subject} onBack={onBack} />
      {!subject.ready && <ComingSoon />}

      <h2>Practice test</h2>
      <div className="quick-grid">
        {TEST_KINDS.map((k) => (
          <button key={k.kind} className="tile plain" disabled={!subject.ready} onClick={() => onTest(k.kind)}>
            <strong>{k.title}</strong>
            {subject.ready && (
              <span className="tile-sub">
                {k.questions} Q · {k.minutes} min
              </span>
            )}
          </button>
        ))}
      </div>
      {history.length > 0 && (
        <ul className="rows">
          {history.slice(0, 3).map((h) => (
            <li key={h.id} className="row">
              <span>{TEST_KINDS.find((k) => k.kind === h.kind)?.title}</span>
              <span className="muted small">{new Date(h.finished_at * 1000).toLocaleDateString()}</span>
              <strong>{h.percent}%</strong>
            </li>
          ))}
        </ul>
      )}

      <h2>Skills</h2>
      {error && <p className="error">{error}</p>}
      {progress && progress.skills.every((s) => s.available === 0) && (
        <p className="panel muted">
          No reviewed questions yet. To try drafts, run <code>scripts\run-with-drafts.cmd</code>.
        </p>
      )}
      <div className="skill-grid">
        {progress?.skills.map((s) => (
          <button key={s.id} className="skill-tile" disabled={s.available === 0} onClick={() => onStart({ mode: 'skill', id: s.id, name: s.name })}>
            <Ring value={s.mastery ?? 0} size={48} stroke={6} className={`level-${s.level}`}>
              {s.mastery === null ? '' : Math.round(s.mastery * 100)}
            </Ring>
            <span>{s.name}</span>
          </button>
        ))}
        {!subject.ready &&
          subject.units.map((u) => (
            <button key={u} className="skill-tile" disabled>
              <Ring value={0} size={48} stroke={6} />
              <span>{u}</span>
            </button>
          ))}
      </div>
    </div>
  )
}

type Outcome = { principle: PrincipleRef; correct: boolean }

/** A practice set: `setLength` questions from a scope, then a short result. */
export function Practice({
  scope,
  setLength,
  onExit,
  onAgain,
}: {
  scope: Scope
  setLength: number
  onExit: () => void
  onAgain: () => void
}) {
  const [q, setQ] = useState<PracticeQuestion | null>(null)
  const [status, setStatus] = useState<'loading' | 'ready' | 'empty' | 'summary'>('loading')
  const [error, setError] = useState<string | null>(null)
  const [outcomes, setOutcomes] = useState<Outcome[]>([])
  const [xp, setXp] = useState(0)
  const [lessonFor, setLessonFor] = useState<string | null>(null)
  const closeLesson = useCallback(() => setLessonFor(null), [])

  const finishSet = useCallback(() => {
    play('done')
    setStatus('summary')
  }, [])

  const load = useCallback(
    async (after?: string, answeredSoFar = 0) => {
      setStatus('loading')
      setError(null)
      try {
        const { question } = await api.next(scope, after)
        setQ(question)
        // Running out of questions part-way through still ends with a result.
        if (question) setStatus('ready')
        else if (answeredSoFar > 0) finishSet()
        else setStatus('empty')
      } catch (e) {
        setError(errorMessage(e))
      }
    },
    [scope, finishSet],
  )

  useEffect(() => {
    void load()
  }, [load])

  const count = outcomes.length
  const next = () => (count >= setLength ? finishSet() : void load(q?.id, count))

  if (status === 'ready' && q && !error)
    return (
      <QuestionView
        key={q.id}
        q={q}
        mode={scope.mode}
        showSkill={scope.mode !== 'skill'}
        progress={count / setLength}
        onClose={onExit}
        onAnswered={(r) => {
          setOutcomes((o) => [...o, { principle: r.principle, correct: r.correct }])
          setXp((x) => x + r.xp)
        }}
        onNext={next}
        onFlagged={() => void load(q.id, count)}
      />
    )

  if (status === 'summary') {
    const right = outcomes.filter((o) => o.correct).length
    const missed = [...new Map(outcomes.filter((o) => !o.correct).map((o) => [o.principle.id, o.principle])).values()].slice(0, 3)
    return (
      <SessionShell
        progress={null}
        onClose={onExit}
        footer={
          <div className="check-row">
            <button className="secondary" onClick={onAgain}>
              Again
            </button>
            <button className="primary wide" autoFocus onClick={onExit}>
              Done
            </button>
          </div>
        }
      >
        <ResultScreen passed={right * 3 >= count * 2} title={right === count ? 'Perfect!' : right * 3 >= count * 2 ? 'Nice work!' : 'Keep going!'} score={`${right} / ${count}`} xp={xp} />
        {missed.length > 0 && (
          <div className="missed">
            <p className="eyebrow">Worth a look</p>
            {missed.map((p) => (
              <button key={p.id} className="row row-btn" onClick={() => setLessonFor(p.id)}>
                <span>{p.name}</span>
                <Icon name="right" size={18} />
              </button>
            ))}
          </div>
        )}
        {lessonFor && <LessonPanel principleId={lessonFor} onClose={closeLesson} />}
      </SessionShell>
    )
  }

  return (
    <SessionShell
      progress={null}
      onClose={onExit}
      footer={
        error ? (
          <button className="primary wide" onClick={() => void load()}>
            Try again
          </button>
        ) : status === 'empty' ? (
          <button className="primary wide" autoFocus onClick={onExit}>
            Back
          </button>
        ) : undefined
      }
    >
      <div className="center loading">
        <Mascot mood={status === 'empty' && scope.mode === 'review' ? 'cheer' : 'think'} />
        {error && <p className="error">{error}</p>}
        {status === 'empty' && !error && <h2>{scope.mode === 'review' ? 'All caught up!' : 'No questions here yet'}</h2>}
      </div>
    </SessionShell>
  )
}
