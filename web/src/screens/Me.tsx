import { useEffect, useState } from 'react'
import { api, errorMessage, type Scope } from '../api/client'
import type { Plan } from '../api/types/Plan'
import type { SkillSummary } from '../api/types/SkillSummary'
import type { Stats } from '../api/types/Stats'
import { Icon } from '../components/Icon'
import { MasteryBar } from '../components/MasteryBar'
import { FocusList } from '../components/StudyLinks'
import { setSoundOn, soundOn } from '../sound'
import { useSubjects } from '../subjects'

/** Me: level and streak, test date, what to work on, strengths, settings. */
export function Me({
  name,
  stats,
  plan,
  onChanged,
  onStart,
  onLogout,
}: {
  name: string
  stats: Stats | null
  plan: Plan | null
  onChanged: () => void
  onStart: (scope: Scope) => void
  onLogout: () => void
}) {
  const subjects = useSubjects()
  const [skills, setSkills] = useState<SkillSummary[]>([])
  const [error, setError] = useState<string | null>(null)
  const [sound, setSound] = useState(soundOn)

  useEffect(() => {
    api.skills().then(setSkills).catch((e) => setError(errorMessage(e)))
  }, [])

  async function saveDate(value: string) {
    try {
      await api.setTestDate(value || null)
      onChanged()
    } catch (e) {
      setError(errorMessage(e))
    }
  }

  const days = plan?.days_left ?? null
  return (
    <div className="me stack">
      <section className="me-head">
        <span className="avatar">{name.slice(0, 1).toUpperCase()}</span>
        <div className="me-level">
          <h1>{name}</h1>
          {stats && (
            <>
              <span className="bar sun">
                <span style={{ width: `${(stats.level_xp / stats.level_size) * 100}%` }} />
              </span>
              <span className="muted small">
                Level {stats.level} · {stats.level_xp}/{stats.level_size} XP
              </span>
            </>
          )}
        </div>
      </section>

      {stats && (
        <section className="stat-grid">
          <span className="stat panel">
            <span className="stat-icon flame">
              <Icon name="flame" fill />
            </span>
            <strong>{stats.streak_days}</strong>
            <span className="muted small">day streak</span>
          </span>
          <span className="stat panel">
            <span className="stat-icon xp">
              <Icon name="bolt" fill />
            </span>
            <strong>{stats.xp}</strong>
            <span className="muted small">total XP</span>
          </span>
          <span className="stat panel">
            <span className="stat-icon strong">
              <Icon name="star" fill />
            </span>
            <strong>{stats.badges.length}</strong>
            <span className="muted small">strong skills</span>
          </span>
        </section>
      )}
      {error && <p className="error">{error}</p>}

      <section className="panel test-date">
        <label htmlFor="test-date">
          <span className="eyebrow">Test date</span>
          <strong className="countdown">
            {days === null ? 'Not set' : days < 0 ? 'Passed' : days === 0 ? 'Today!' : `${days} day${days === 1 ? '' : 's'} to go`}
          </strong>
        </label>
        <input id="test-date" type="date" value={plan?.test_date ?? ''} onChange={(e) => void saveDate(e.target.value)} />
      </section>

      {plan && plan.priorities.length > 0 && (
        <section>
          <h2>Work on next</h2>
          <FocusList items={plan.priorities.slice(0, 5)} onPractice={(p) => onStart({ mode: 'principle', id: p.principle.id, name: p.principle.name })} />
        </section>
      )}

      <section>
        <h2>Subjects</h2>
        {subjects.map((sub) =>
          sub.id === 'math' ? (
            <details key={sub.id} className={`fold subject-fold subj-${sub.id}`}>
              <summary>
                <span className="subject-icon">
                  <Icon name={sub.icon} size={20} />
                </span>
                <span className="fold-text">{sub.name}</span>
                <span className="muted small">
                  {skills.filter((s) => s.level === 'strong').length}/{skills.length} strong
                </span>
                <Icon name="down" size={18} />
              </summary>
              <ul className="rows">
                {skills.map((s) => (
                  <li key={s.id} className="strength">
                    <span>{s.name}</span>
                    <MasteryBar mastery={s.mastery} level={s.level} />
                  </li>
                ))}
              </ul>
            </details>
          ) : (
            <div key={sub.id} className={`row subject-fold soon subj-${sub.id}`}>
              <span className="subject-icon">
                <Icon name={sub.icon} size={20} />
              </span>
              <span className="fold-text">{sub.name}</span>
              {sub.ready && sub.info ? (
                <span className="muted small">
                  {sub.info.stages_passed}/{sub.info.stages_total} sets
                </span>
              ) : (
                <span className="pill">Soon</span>
              )}
            </div>
          ),
        )}
      </section>

      <section className="settings">
        <button
          className="row row-btn"
          role="switch"
          aria-checked={sound}
          onClick={() => {
            setSoundOn(!sound)
            setSound(!sound)
          }}
        >
          <Icon name={sound ? 'sound' : 'mute'} />
          <span>Sound</span>
          <strong>{sound ? 'On' : 'Off'}</strong>
        </button>
        <button className="row row-btn" onClick={onLogout}>
          <Icon name="logout" />
          <span>Log out</span>
        </button>
        <details className="fold about">
          <summary>
            <span className="fold-text">About</span>
            <Icon name="down" size={18} />
          </summary>
          <p>
            Questions and lessons are written by AI and checked by a separate AI reviewer and by code, not by a teacher. They may contain
            mistakes. If something looks wrong, tap the flag on the question.
          </p>
          <p>
            Free GED test practice. Not affiliated with or endorsed by GED Testing Service. GED® is a registered trademark of the American
            Council on Education.
          </p>
        </details>
      </section>
    </div>
  )
}
