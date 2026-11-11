import { useEffect, useState } from 'react'
import { api, errorMessage, type Scope } from '../api/client'
import type { JourneyView } from '../api/types/JourneyView'
import type { Plan } from '../api/types/Plan'
import type { PlanTask } from '../api/types/PlanTask'
import type { Stats } from '../api/types/Stats'
import type { TestKind } from '../api/types/TestKind'
import { Hack } from '../components/LessonCard'
import { MasteryBar } from '../components/MasteryBar'

const AI_NOTICE_KEY = 'ged.aiNoticeDismissed'
function readDismissed(): boolean {
  try {
    return localStorage.getItem(AI_NOTICE_KEY) === '1'
  } catch {
    return false
  }
}

const TASK_ICON: Record<PlanTask['kind'], string> = {
  diagnostic: '🧭',
  power_set: '⚡',
  review: '🔁',
  full_test: '⏱',
  hacks: '💡',
  tactics: '🎯',
  journey: '🗺',
  set_date: '📅',
  rest: '😴',
}

/** Home: progress at a glance, today's plan, and one-tap ways to study. */
export function Home({
  onStart,
  onJourney,
  onTest,
  onTips,
}: {
  onStart: (scope: Scope, setLength: number | null) => void
  onJourney: (stageId?: string) => void
  onTest: (kind: TestKind) => void
  onTips: () => void
}) {
  const [stats, setStats] = useState<Stats | null>(null)
  const [plan, setPlan] = useState<Plan | null>(null)
  const [journey, setJourney] = useState<JourneyView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [date, setDate] = useState('')
  const [aiNoticeHidden, setAiNoticeHidden] = useState(readDismissed)
  const [reload, setReload] = useState(0)

  useEffect(() => {
    Promise.all([api.stats(), api.plan(), api.journey()])
      .then(([s, p, j]) => {
        setStats(s)
        setPlan(p)
        setJourney(j)
        setDate(p.test_date ?? '')
      })
      .catch((e) => setError(errorMessage(e)))
  }, [reload])

  async function saveDate(value: string | null) {
    try {
      await api.setTestDate(value)
      setReload((n) => n + 1)
    } catch (e) {
      setError(errorMessage(e))
    }
  }

  function runTask(t: PlanTask) {
    switch (t.kind) {
      case 'diagnostic':
        return onTest('short')
      case 'full_test':
        return onTest('full')
      case 'power_set':
        return onStart({ mode: 'principle', id: t.principles.join(','), name: 'Power set' }, 10)
      case 'review':
        return onStart({ mode: 'review' }, 10)
      case 'hacks':
      case 'tactics':
        return onTips()
      case 'journey':
        return onJourney(journey?.current ?? undefined)
      case 'set_date':
        document.getElementById('test-date')?.focus()
        return
      case 'rest':
        return
    }
  }

  if (error) return <p className="error">{error}</p>
  if (!stats || !plan || !journey) return <p className="muted">Loading…</p>

  const currentStage = journey.stages.find((s) => s.id === journey.current)
  const days = plan.days_left
  return (
    <>
      <section className="stats-strip" aria-label="Your progress">
        <div className="level">
          <span className="level-num">Lv {stats.level}</span>
          <span className="level-bar" aria-hidden="true">
            <span style={{ width: `${(stats.level_xp / stats.level_size) * 100}%` }} />
          </span>
          <span className="muted small-text">
            {stats.xp} XP · {stats.level_size - stats.level_xp} to next level
          </span>
        </div>
        <div className="stat">
          <span className="stat-num">🔥 {stats.streak_days}</span>
          <span className="muted small-text">day streak</span>
        </div>
        <div className="stat">
          <span className="stat-num">{stats.answered_today}</span>
          <span className="muted small-text">today</span>
        </div>
        <div className="stat">
          <span className="stat-num">{stats.review_due}</span>
          <span className="muted small-text">to review</span>
        </div>
      </section>

      {!aiNoticeHidden && (
        <div className="card ai-notice">
          <span>
            Questions and lessons are written by AI and checked by a separate AI reviewer and by code — not by a
            teacher. If something looks wrong, use <strong>Flag this question</strong>.
          </span>
          <button
            className="secondary small"
            onClick={() => {
              setAiNoticeHidden(true)
              try {
                localStorage.setItem(AI_NOTICE_KEY, '1')
              } catch {
                // fine
              }
            }}
          >
            Got it
          </button>
        </div>
      )}

      <section className="card plan">
        <div className="plan-head">
          <div>
            <h2>
              {days === null
                ? 'When is your test?'
                : days < 0
                  ? 'Your test date has passed'
                  : days === 0
                    ? 'Test day!'
                    : `${days} day${days === 1 ? '' : 's'} to your test`}
            </h2>
            {days !== null && days > 0 && (
              <p className="muted">
                Aim for your top {plan.focus_count} principles — the ones that come up most and that you find hardest.
              </p>
            )}
          </div>
          <form
            className="date-form"
            onSubmit={(e) => {
              e.preventDefault()
              void saveDate(date || null)
            }}
          >
            <input id="test-date" type="date" value={date} onChange={(e) => setDate(e.target.value)} aria-label="Test date" />
            <button className="secondary small" type="submit">
              Save
            </button>
          </form>
        </div>
        <h3>Today</h3>
        <ul className="tasks">
          {currentStage && (
            <li>
              <button className="task" onClick={() => onJourney(currentStage.id)}>
                <span className="task-icon" aria-hidden="true">
                  🗺
                </span>
                <span>
                  <strong>Journey: {currentStage.title}</strong>
                  <span className="muted"> · a few minutes</span>
                </span>
              </button>
            </li>
          )}
          {plan.today
            .filter((t) => t.kind !== 'journey')
            .map((t, i) => (
              <li key={i}>
                <button className="task" onClick={() => runTask(t)} disabled={t.kind === 'rest'}>
                  <span className="task-icon" aria-hidden="true">
                    {TASK_ICON[t.kind]}
                  </span>
                  <span>
                    <strong>{t.title}</strong>
                    <span className="muted"> · {t.detail}</span>
                  </span>
                </button>
              </li>
            ))}
        </ul>
        {plan.hacks.length > 0 && (
          <>
            <h3>Today’s hacks</h3>
            {plan.hacks.slice(0, 3).map((h) => (
              <div key={h.principle.id} className="hack-row">
                <strong>{h.title}</strong>
                <Hack text={h.hack} />
              </div>
            ))}
          </>
        )}
      </section>

      <section className="quick">
        <button className="card quick-btn" onClick={() => onStart({ mode: 'mixed' }, 5)}>
          <span className="quick-icon" aria-hidden="true">
            ⚡
          </span>
          <span>
            <strong>Quick 5</strong>
            <span className="muted">Five mixed questions</span>
          </span>
        </button>
        <button className="card quick-btn" disabled={stats.review_due === 0} onClick={() => onStart({ mode: 'review' }, 10)}>
          <span className="quick-icon" aria-hidden="true">
            🔁
          </span>
          <span>
            <strong>Review</strong>
            <span className="muted">{stats.review_due > 0 ? `${stats.review_due} due` : 'Nothing due'}</span>
          </span>
        </button>
        <button className="card quick-btn" onClick={() => onTest('short')}>
          <span className="quick-icon" aria-hidden="true">
            ⏱
          </span>
          <span>
            <strong>Practice test</strong>
            <span className="muted">10 questions, timed</span>
          </span>
        </button>
      </section>

      {plan.priorities.length > 0 && (
        <section>
          <h3>Your priority principles</h3>
          <p className="muted">Ranked by how often they come up × how much you need them.</p>
          <ul className="priority-list">
            {plan.priorities.slice(0, 6).map((p) => (
              <li key={p.principle.id}>
                <button className="principle" onClick={() => onStart({ mode: 'principle', id: p.principle.id, name: p.principle.name }, 10)}>
                  <span className="badge">{p.principle.id}</span>
                  <span className="principle-text">
                    <span className="principle-name">{p.principle.name}</span>
                    <span className="principle-summary">
                      {p.skill_name} · {p.yield_level === 'high' ? 'comes up a lot' : p.yield_level === 'medium' ? 'comes up often' : 'less common'}
                    </span>
                    <MasteryBar mastery={p.mastery} level={p.level} compact />
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </>
  )
}
