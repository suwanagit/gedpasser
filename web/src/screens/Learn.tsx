import { useEffect, useState } from 'react'
import { api } from '../api/client'
import type { JourneyView } from '../api/types/JourneyView'
import type { Plan } from '../api/types/Plan'
import type { PlanTask } from '../api/types/PlanTask'
import type { Stats } from '../api/types/Stats'
import { Icon, type IconName } from '../components/Icon'
import { Ring } from '../components/Ring'
import { SubjectCard } from '../components/Subject'
import { SUBJECTS, type SubjectId } from '../subjects'

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

/** Home: one button to carry on, then the four subjects. */
export function Learn({
  stats,
  plan,
  onSubject,
  onOpen,
  onTask,
}: {
  stats: Stats | null
  plan: Plan | null
  onSubject: (id: SubjectId) => void
  onOpen: (stageId: string) => void
  onTask: (t: PlanTask) => void
}) {
  // Only Math has a journey so far.
  const [journey, setJourney] = useState<JourneyView | null>(null)
  useEffect(() => {
    api.journey().then(setJourney).catch(() => setJourney(null))
  }, [])

  const today = stats?.answered_today ?? 0
  const tasks = (plan?.today ?? []).filter((t) => t.kind !== 'journey' && t.kind !== 'rest').slice(0, 3)
  const next = journey?.stages.find((s) => s.id === journey.current)
  const total = journey?.stages.length ?? 0

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

      <div className="learn-main">
        {next && (
          <section className="hero subj-math">
            <div className="hero-text">
              <span className="eyebrow">Math · up next</span>
              <h2>{next.title}</h2>
            </div>
            <button className="hero-btn" onClick={() => onOpen(next.id)}>
              <Icon name="play" size={18} fill /> Start
            </button>
          </section>
        )}

        <div className="subject-grid">
          {SUBJECTS.map((s) => (
            <SubjectCard
              key={s.id}
              subject={s}
              value={s.id === 'math' && total > 0 ? (journey?.passed ?? 0) / total : undefined}
              sub={s.id === 'math' && total > 0 ? `${journey?.passed ?? 0} / ${total} stages` : undefined}
              onClick={() => onSubject(s.id)}
            />
          ))}
        </div>
      </div>
    </div>
  )
}
