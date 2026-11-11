import { useCallback, useEffect, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { PrincipleSummary } from '../api/types/PrincipleSummary'
import type { SkillSummary } from '../api/types/SkillSummary'
import { LessonPanel } from '../components/LessonPanel'
import { MathText } from '../components/MathText'

const AREAS = [
  { id: 'quantitative', title: 'Quantitative problem solving' },
  { id: 'algebraic', title: 'Algebraic problem solving' },
]

/** Every principle, grouped by area and skill, each opening its lesson. */
export function Lessons({ onBack }: { onBack: () => void }) {
  const [skills, setSkills] = useState<SkillSummary[] | null>(null)
  const [principles, setPrinciples] = useState<PrincipleSummary[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [open, setOpen] = useState<string | null>(null)
  const close = useCallback(() => setOpen(null), [])

  useEffect(() => {
    Promise.all([api.skills(), api.principles()])
      .then(([s, p]) => {
        setSkills(s)
        setPrinciples(p)
      })
      .catch((e) => setError(errorMessage(e)))
  }, [])

  if (error) return <p className="error">{error}</p>
  if (!skills || !principles) return <p className="muted">Loading lessons…</p>

  return (
    <section>
      <div className="practice-bar">
        <button className="link" onClick={onBack}>
          ← All skills
        </button>
        <h2>Lessons</h2>
        <span className="muted">{principles.length} principles</span>
      </div>
      <p className="muted intro">
        Each principle lines up with one item (shown by its code) in the published assessment targets for GED Math.
        Open one to read a short lesson with a worked example.
      </p>
      {AREAS.map((area) => (
        <section key={area.id} className="area">
          <h3>{area.title}</h3>
          {skills
            .filter((s) => s.area === area.id)
            .map((s) => (
              <div key={s.id} className="lesson-group">
                <h4>{s.name}</h4>
                <ul className="principle-list">
                  {principles
                    .filter((p) => p.skill === s.id)
                    .map((p) => (
                      <li key={p.id}>
                        <button className="principle" onClick={() => setOpen(p.id)}>
                          <span className="badge">{p.id}</span>
                          <span className="principle-text">
                            <span className="principle-name">{p.name}</span>
                            <span className="principle-summary">
                              <MathText text={p.summary} />
                            </span>
                          </span>
                          <span className="principle-stats">
                            {p.answered > 0 ? `${p.correct}/${p.answered}` : ''}
                            {!p.has_lesson && <span className="muted"> soon</span>}
                          </span>
                        </button>
                      </li>
                    ))}
                </ul>
              </div>
            ))}
        </section>
      ))}
      {open && <LessonPanel principleId={open} onClose={close} />}
    </section>
  )
}
