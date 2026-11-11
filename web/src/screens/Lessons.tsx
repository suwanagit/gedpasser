import { useCallback, useEffect, useState } from 'react'
import { api, errorMessage, type Scope } from '../api/client'
import type { PrincipleSummary } from '../api/types/PrincipleSummary'
import type { SkillSummary } from '../api/types/SkillSummary'
import { LessonPanel } from '../components/LessonPanel'
import { MasteryBar } from '../components/MasteryBar'
import { MathText } from '../components/MathText'
import { SET_LENGTH } from './Dashboard'

const AREAS = [
  { id: 'quantitative', title: 'Quantitative problem solving' },
  { id: 'algebraic', title: 'Algebraic problem solving' },
]

/** Every principle, grouped by area and skill: progress, lesson and practice. */
export function Lessons({
  onBack,
  onPractice,
}: {
  onBack: () => void
  onPractice: (scope: Scope, setLength: number | null) => void
}) {
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

  const practice = (p: PrincipleSummary) => onPractice({ mode: 'principle', id: p.id, name: p.name }, SET_LENGTH)
  const openPrinciple = principles.find((p) => p.id === open)

  return (
    <section>
      <div className="practice-bar">
        <button className="link" onClick={onBack}>
          ← Dashboard
        </button>
        <h2>Lessons</h2>
        <span className="muted">{principles.length} principles</span>
      </div>
      <p className="muted intro">
        Each principle lines up with one item (shown by its code) in the published assessment targets for GED Math.
        Open one to read a short lesson with a worked example, or practice just that principle.
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
                      <li key={p.id} className="principle-row">
                        <button className="principle" onClick={() => setOpen(p.id)}>
                          <span className="badge">{p.id}</span>
                          <span className="principle-text">
                            <span className="principle-name">{p.name}</span>
                            <span className="principle-summary">
                              <MathText text={p.summary} />
                            </span>
                            <MasteryBar mastery={p.mastery} level={p.level} compact />
                          </span>
                          {!p.has_lesson && <span className="muted principle-stats">lesson soon</span>}
                        </button>
                        <button
                          className="secondary small practice-btn"
                          disabled={p.available === 0}
                          onClick={() => practice(p)}
                          title={p.available === 0 ? 'No questions for this principle yet' : undefined}
                        >
                          {p.available === 0 ? 'No questions' : 'Practice'}
                        </button>
                      </li>
                    ))}
                </ul>
              </div>
            ))}
        </section>
      ))}
      {open && (
        <LessonPanel
          principleId={open}
          onClose={close}
          onPractice={openPrinciple && openPrinciple.available > 0 ? () => practice(openPrinciple) : undefined}
        />
      )}
    </section>
  )
}
