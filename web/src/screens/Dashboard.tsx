import { useEffect, useState } from 'react'
import { api, errorMessage, type Scope } from '../api/client'
import type { Progress } from '../api/types/Progress'
import type { SkillSummary } from '../api/types/SkillSummary'
import { MasteryBar } from '../components/MasteryBar'

export const SET_LENGTH = 10

const AREAS: { id: string; title: string }[] = [
  { id: 'quantitative', title: 'Quantitative problem solving' },
  { id: 'algebraic', title: 'Algebraic problem solving' },
]

/** Practice menu: focus next, mixed practice, and the skill map by mastery. */
export function Dashboard({ onStart }: { onStart: (scope: Scope, setLength: number | null) => void }) {
  const [progress, setProgress] = useState<Progress | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api
      .progress()
      .then(setProgress)
      .catch((e) => setError(errorMessage(e)))
  }, [])

  if (error) return <p className="error">{error}</p>
  if (!progress) return <p className="muted">Loading your progress…</p>

  const skills = progress.skills
  const byId = new Map(skills.map((s) => [s.id, s]))
  const focus = progress.focus.map((id) => byId.get(id)).filter((s): s is SkillSummary => !!s)
  const nothingReviewed = skills.every((s) => s.available === 0)
  const started = skills.some((s) => s.answered > 0)
  const skillScope = (s: SkillSummary): Scope => ({ mode: 'skill', id: s.id, name: s.name })

  return (
    <>
      {nothingReviewed && (
        <div className="card notice">
          <strong>No reviewed questions yet.</strong> Only reviewed questions are shown to learners. To try the
          unreviewed drafts on this PC, close the server window and double-click{' '}
          <code>scripts\run-with-drafts.cmd</code> instead.
        </div>
      )}

      {focus.length > 0 && (
        <section className="focus">
          <h2>Focus next</h2>
          <p className="muted">
            {started
              ? 'Your three weakest skills right now. A short set on each is the fastest way to improve.'
              : 'Not sure where to start? Try a short set in one of these.'}
          </p>
          <div className="focus-grid">
            {focus.map((s) => (
              <div key={s.id} className="card focus-card">
                <span className="skill-name">{s.name}</span>
                <MasteryBar mastery={s.mastery} level={s.level} />
                <button className="primary" onClick={() => onStart(skillScope(s), SET_LENGTH)}>
                  Start {SET_LENGTH}-question set
                </button>
              </div>
            ))}
          </div>
        </section>
      )}

      {!nothingReviewed && (
        <div className="card mixed-card">
          <div>
            <h3>Mixed practice</h3>
            <p className="muted">Questions from every skill, with more from the ones you find hardest.</p>
          </div>
          <button className="primary" onClick={() => onStart({ mode: 'mixed' }, SET_LENGTH)}>
            Start mixed set
          </button>
        </div>
      )}

      <h2 className="screen-title">Practice by skill</h2>
      {AREAS.map((area) => (
        <section key={area.id} className="area">
          <h3>{area.title}</h3>
          <div className="skill-grid">
            {skills
              .filter((s) => s.area === area.id)
              .map((s) => (
                <button
                  key={s.id}
                  className={`skill tile level-${s.level}`}
                  disabled={s.available === 0}
                  onClick={() => onStart(skillScope(s), null)}
                >
                  <span className="skill-name">{s.name}</span>
                  <span className="skill-desc">{s.description}</span>
                  <MasteryBar mastery={s.mastery} level={s.level} compact />
                  <span className="skill-stats">
                    {s.available === 0
                      ? 'No questions yet'
                      : s.answered === 0
                        ? `${s.available} questions`
                        : `${s.correct} of ${s.answered} correct`}
                  </span>
                </button>
              ))}
          </div>
        </section>
      ))}
    </>
  )
}
