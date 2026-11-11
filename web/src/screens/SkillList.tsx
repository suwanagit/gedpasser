import { useEffect, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { SkillSummary } from '../api/types/SkillSummary'

const AREAS: { id: string; title: string }[] = [
  { id: 'quantitative', title: 'Quantitative problem solving' },
  { id: 'algebraic', title: 'Algebraic problem solving' },
]

export function SkillList({ onPick }: { onPick: (skill: SkillSummary) => void }) {
  const [skills, setSkills] = useState<SkillSummary[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api
      .skills()
      .then(setSkills)
      .catch((e) => setError(errorMessage(e)))
  }, [])

  if (error) return <p className="error">{error}</p>
  if (!skills) return <p className="muted">Loading skills…</p>

  return (
    <>
      <h2 className="screen-title">Pick a skill to practise</h2>
      {AREAS.map((area) => (
        <section key={area.id} className="area">
          <h3>{area.title}</h3>
          <div className="skill-grid">
            {skills
              .filter((s) => s.area === area.id)
              .map((s) => (
                <button key={s.id} className="skill" disabled={s.available === 0} onClick={() => onPick(s)}>
                  <span className="skill-name">{s.name}</span>
                  <span className="skill-desc">{s.description}</span>
                  <span className="skill-stats">
                    {s.available === 0
                      ? 'No questions yet'
                      : s.answered === 0
                        ? `${s.available} questions · not started`
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
