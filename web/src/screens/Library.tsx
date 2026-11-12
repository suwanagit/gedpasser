import { useCallback, useEffect, useState } from 'react'
import { api, errorMessage, type Scope } from '../api/client'
import type { PrincipleSummary } from '../api/types/PrincipleSummary'
import type { SkillSummary } from '../api/types/SkillSummary'
import type { StrategyView } from '../api/types/StrategyView'
import { FormulaSheet } from '../components/FormulaSheet'
import { Icon } from '../components/Icon'
import { Hack } from '../components/LessonCard'
import { LessonPanel } from '../components/LessonPanel'
import { Mascot } from '../components/Mascot'
import { ComingSoon, SubHead, SubjectCard } from '../components/Subject'
import { useSubjects, type Subject, type SubjectId } from '../subjects'

export type LibraryTab = 'lessons' | 'hacks' | 'formulas'

const TABS: { tab: LibraryTab; label: string }[] = [
  { tab: 'lessons', label: 'Lessons' },
  { tab: 'hacks', label: 'Hacks' },
  { tab: 'formulas', label: 'Formulas' },
]

/** Library tab: pick a subject to look things up, or read the test-day tips. */
export function Library({ onSubject, onTestDay }: { onSubject: (id: SubjectId) => void; onTestDay: () => void }) {
  const subjects = useSubjects()
  return (
    <div className="library">
      <div className="subject-grid">
        {subjects.map((s) => (
          <SubjectCard key={s.id} subject={s} sub={s.id === 'math' ? 'Lessons · Hacks · Formulas' : undefined} onClick={() => onSubject(s.id)} />
        ))}
      </div>
      <h2>For every test</h2>
      <button className="subject-card subj-any" onClick={onTestDay}>
        <span className="subject-icon">
          <Icon name="target" size={28} />
        </span>
        <span className="subject-text">
          <strong>Test day</strong>
          <span className="muted small">Tips for the day itself</span>
        </span>
        <Icon name="right" size={20} />
      </button>
    </div>
  )
}

/** Lesson cards, memory hacks and formulas for one subject. */
export function LibrarySubject({
  subject,
  initialTab,
  onBack,
  onPractice,
}: {
  subject: Subject
  initialTab: LibraryTab
  onBack: () => void
  onPractice: (scope: Scope) => void
}) {
  const [tab, setTab] = useState<LibraryTab>(initialTab)
  const [skills, setSkills] = useState<SkillSummary[] | null>(null)
  const [principles, setPrinciples] = useState<PrincipleSummary[] | null>(null)
  const [strategy, setStrategy] = useState<StrategyView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState<string | null>(null)
  const close = useCallback(() => setOpen(null), [])

  useEffect(() => {
    if (!subject.ready) return
    Promise.all([api.skills(subject.id), api.principles(subject.id), api.strategy()])
      .then(([s, p, st]) => {
        setSkills(s)
        setPrinciples(p)
        setStrategy(st)
      })
      .catch((e) => setError(errorMessage(e)))
  }, [subject.ready, subject.id])

  if (!subject.ready)
    return (
      <div className="library">
        <SubHead subject={subject} onBack={onBack} />
        <ComingSoon />
        <div className="seg" role="tablist">
          <button role="tab" aria-selected className="on">
            Lessons
          </button>
          <button role="tab" aria-selected={false} disabled>
            Hacks
          </button>
        </div>
        {subject.units.map((u) => (
          <div key={u} className="row soon-row">
            <span className="dot" />
            <span>{u}</span>
            <Icon name="lock" size={18} />
          </div>
        ))}
      </div>
    )

  if (error) return <p className="error">{error}</p>
  if (!skills || !principles || !strategy)
    return (
      <div className="center loading">
        <Mascot mood="think" />
      </div>
    )

  const words = query.trim().toLowerCase()
  const matches = (p: PrincipleSummary) => !words || `${p.name} ${p.summary} ${p.id}`.toLowerCase().includes(words)
  const opened = principles.find((p) => p.id === open)

  return (
    <div className="library">
      <SubHead subject={subject} onBack={onBack} />
      <div className="seg no-print" role="tablist">
        {TABS.filter((t) => subject.id === 'math' || t.tab === 'lessons').map((t) => (
          <button key={t.tab} role="tab" aria-selected={tab === t.tab} className={tab === t.tab ? 'on' : ''} onClick={() => setTab(t.tab)}>
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'lessons' && (
        <>
          <label className="search">
            <Icon name="search" size={20} />
            <input type="search" placeholder="Search" aria-label="Search lessons" value={query} onChange={(e) => setQuery(e.target.value)} />
          </label>
          {skills.map((s) => {
            const list = principles.filter((p) => p.skill === s.id && matches(p))
            if (list.length === 0) return null
            return (
              <details key={s.id} className="fold skill-fold" open={!!words}>
                <summary>
                  <span className={`dot level-${s.level}`} />
                  <span className="fold-text">{s.name}</span>
                  <span className="muted small">{list.length}</span>
                  <Icon name="down" size={18} />
                </summary>
                {list.map((p) => (
                  <button key={p.id} className="row row-btn" onClick={() => setOpen(p.id)}>
                    <span className={`dot level-${p.level}`} />
                    <span>{p.name}</span>
                    <Icon name="right" size={18} />
                  </button>
                ))}
              </details>
            )
          })}
        </>
      )}

      {tab !== 'lessons' && (
        <button className="secondary small no-print print-btn" onClick={() => window.print()}>
          <Icon name="print" size={18} /> Print
        </button>
      )}

      <div className={tab === 'hacks' ? 'stack' : 'print-only'}>
        <h2 className="print-only">Memory hacks</h2>
        {strategy.hacks.map((h) => (
          <div key={h.principle.id} className="hack-item">
            <strong>{h.title}</strong>
            <Hack text={h.hack} />
          </div>
        ))}
      </div>

      <div className={tab === 'formulas' ? '' : 'print-only'}>
        <h2 className="print-only">Formulas</h2>
        <FormulaSheet formulas={strategy.formulas} />
      </div>

      {open && (
        <LessonPanel
          principleId={open}
          onClose={close}
          onPractice={opened && opened.available > 0 ? () => onPractice({ mode: 'principle', id: opened.id, name: opened.name }) : undefined}
        />
      )}
    </div>
  )
}

/** Test-day tactics: the same for every subject. */
export function TestDay({ onBack }: { onBack: () => void }) {
  const [strategy, setStrategy] = useState<StrategyView | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api.strategy().then(setStrategy).catch((e) => setError(errorMessage(e)))
  }, [])

  return (
    <div className="library">
      <SubHead title="Test day" onBack={onBack}>
        <button className="icon-btn" aria-label="Print" onClick={() => window.print()}>
          <Icon name="print" />
        </button>
      </SubHead>
      <h2 className="print-only">Test day</h2>
      {error && <p className="error">{error}</p>}
      <div className="stack">
        {strategy?.tactics.map((t, i) => (
          <div key={t.id} className="tactic panel">
            <span className="tactic-num">{i + 1}</span>
            <span>
              <strong>{t.title}</strong>
              <span className="muted">{t.text}</span>
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}
