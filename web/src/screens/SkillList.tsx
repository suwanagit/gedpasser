import { useEffect, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { SkillSummary } from '../api/types/SkillSummary'

const AREAS: { id: string; title: string }[] = [
  { id: 'quantitative', title: 'Quantitative problem solving' },
  { id: 'algebraic', title: 'Algebraic problem solving' },
]

const AI_NOTICE_KEY = 'ged.aiNoticeDismissed'

function readDismissed(): boolean {
  try {
    return localStorage.getItem(AI_NOTICE_KEY) === '1'
  } catch {
    return false
  }
}

export function SkillList({ onPick }: { onPick: (skill: SkillSummary) => void }) {
  const [skills, setSkills] = useState<SkillSummary[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [aiNoticeHidden, setAiNoticeHidden] = useState(readDismissed)

  function hideAiNotice() {
    setAiNoticeHidden(true)
    try {
      localStorage.setItem(AI_NOTICE_KEY, '1')
    } catch {
      // storage unavailable; the notice just comes back next visit
    }
  }

  useEffect(() => {
    api
      .skills()
      .then(setSkills)
      .catch((e) => setError(errorMessage(e)))
  }, [])

  if (error) return <p className="error">{error}</p>
  if (!skills) return <p className="muted">Loading skills…</p>

  const nothingReviewed = skills.every((s) => s.available === 0)

  return (
    <>
      <h2 className="screen-title">Pick a skill to practice</h2>
      {!aiNoticeHidden && (
        <div className="card ai-notice">
          <span>
            Every question and lesson here was written by AI and checked by a separate AI reviewer and by code — not by a
            teacher. Mistakes are possible. If something looks wrong, use <strong>Flag this question</strong> and it
            will be pulled until it’s checked.
          </span>
          <button className="secondary small" onClick={hideAiNotice}>
            Got it
          </button>
        </div>
      )}
      {nothingReviewed && (
        <div className="card notice">
          <strong>No reviewed questions yet.</strong> Only reviewed questions are shown to learners. To try the
          unreviewed drafts on this PC, close the server window and double-click{' '}
          <code>scripts\run-with-drafts.cmd</code> instead.
        </div>
      )}
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
