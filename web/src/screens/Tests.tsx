import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { TestKind } from '../api/types/TestKind'
import type { TestPaper } from '../api/types/TestPaper'
import type { TestReport } from '../api/types/TestReport'
import type { TestSummary } from '../api/types/TestSummary'
import { Calculator } from '../components/Calculator'
import { Figure } from '../components/Figure'
import { FormulaSheet } from '../components/FormulaSheet'
import { Hack } from '../components/LessonCard'
import { MathText } from '../components/MathText'

const KINDS: { kind: TestKind; title: string; detail: string }[] = [
  { kind: 'short', title: 'Short', detail: '10 questions · 25 minutes' },
  { kind: 'long', title: 'Long', detail: '25 questions · 60 minutes' },
  { kind: 'full', title: 'Full length', detail: '46 questions · 115 minutes — about the real test' },
]

const BAND_TEXT: Record<string, string> = {
  ready: 'Looking ready',
  close: 'Close — keep going',
  building: 'Building — focus on your weak spots',
  early: 'Early days — the journey will help',
}

/** Choose a practice test, and see past results. */
export function TestsHome({ onStart }: { onStart: (kind: TestKind) => void }) {
  const [history, setHistory] = useState<TestSummary[] | null>(null)
  useEffect(() => {
    api.tests().then(setHistory).catch(() => setHistory([]))
  }, [])
  return (
    <section>
      <h2 className="screen-title">Practice tests</h2>
      <p className="muted intro">Timed, no hints, no feedback until the end — like test day. Calculator and formula sheet included.</p>
      <div className="test-kinds">
        {KINDS.map((k) => (
          <button key={k.kind} className="card test-kind" onClick={() => onStart(k.kind)}>
            <span className="skill-name">{k.title}</span>
            <span className="muted">{k.detail}</span>
          </button>
        ))}
      </div>
      {history && history.length > 0 && (
        <>
          <h3>Past results</h3>
          <ul className="history">
            {history.map((h) => (
              <li key={h.id}>
                <span>{KINDS.find((k) => k.kind === h.kind)?.title}</span>
                <span>
                  {h.correct}/{h.total} · <strong>{h.percent}%</strong>
                </span>
                <span className="muted">{new Date(h.finished_at * 1000).toLocaleDateString()}</span>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  )
}

/** Take a test: timer, navigation, flag for review, calculator, formula sheet. */
export function TestRunner({ kind, onExit }: { kind: TestKind; onExit: () => void }) {
  const [paper, setPaper] = useState<TestPaper | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [index, setIndex] = useState(0)
  const [answers, setAnswers] = useState<Record<string, string>>({})
  const [flags, setFlags] = useState<Set<string>>(new Set())
  const [left, setLeft] = useState(0)
  const [tool, setTool] = useState<'none' | 'calc' | 'formulas'>('none')
  const [confirming, setConfirming] = useState(false)
  const [report, setReport] = useState<TestReport | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const spent = useRef<Record<string, number>>({})
  const shownAt = useRef(performance.now())

  useEffect(() => {
    api
      .newTest(kind)
      .then((p) => {
        setPaper(p)
        setLeft(p.time_limit_secs)
        shownAt.current = performance.now()
      })
      .catch((e) => setError(errorMessage(e)))
  }, [kind])

  const track = useCallback(() => {
    if (!paper) return
    const id = paper.questions[index].id
    spent.current[id] = (spent.current[id] ?? 0) + (performance.now() - shownAt.current)
    shownAt.current = performance.now()
  }, [paper, index])

  const submit = useCallback(async () => {
    if (!paper || submitting) return
    track()
    setSubmitting(true)
    try {
      const r = await api.submitTest(Number(paper.id), {
        answers: paper.questions.map((q) => ({
          question_id: q.id,
          response: answers[q.id] ?? '',
          time_ms: Math.round(spent.current[q.id] ?? 0),
        })),
      })
      setReport(r)
    } catch (e) {
      setError(errorMessage(e))
      setSubmitting(false)
    }
  }, [paper, answers, submitting, track])

  // Countdown; time's up submits.
  useEffect(() => {
    if (!paper || report) return
    if (left <= 0) {
      void submit()
      return
    }
    const t = setTimeout(() => setLeft((s) => s - 1), 1000)
    return () => clearTimeout(t)
  }, [left, paper, report, submit])

  if (error) return <p className="error">{error}</p>
  if (report && paper) return <TestReportView paper={paper} report={report} onExit={onExit} />
  if (!paper) return <p className="muted">Building your test…</p>

  const q = paper.questions[index]
  const go = (i: number) => {
    track()
    setIndex(i)
  }
  const answeredCount = paper.questions.filter((x) => (answers[x.id] ?? '').trim()).length
  const mm = Math.floor(left / 60)
  const ss = String(left % 60).padStart(2, '0')
  return (
    <section className="test-runner">
      <div className="test-bar">
        <span className={`timer${left < 300 ? ' low' : ''}`} aria-live="off">
          ⏱ {mm}:{ss}
        </span>
        <span className="muted">
          {index + 1} / {paper.questions.length} · {answeredCount} answered
        </span>
        <span className="test-tools">
          <button className={`secondary small${tool === 'calc' ? ' on' : ''}`} onClick={() => setTool(tool === 'calc' ? 'none' : 'calc')}>
            Calculator
          </button>
          <button className={`secondary small${tool === 'formulas' ? ' on' : ''}`} onClick={() => setTool(tool === 'formulas' ? 'none' : 'formulas')}>
            Formulas
          </button>
        </span>
      </div>
      {tool === 'calc' && <Calculator onClose={() => setTool('none')} />}
      {tool === 'formulas' && (
        <div className="card tool-panel">
          <FormulaSheet />
        </div>
      )}

      <article className="card question">
        <p className="prompt">
          <MathText text={q.prompt} />
        </p>
        {q.figure && <Figure view={q.figure} />}
        {q.kind === 'fill_in' ? (
          <label className="fill-in-label">
            Your answer
            <input
              value={answers[q.id] ?? ''}
              onChange={(e) => setAnswers({ ...answers, [q.id]: e.target.value })}
              autoComplete="off"
              placeholder="e.g. 12, 3.5 or 3/4"
            />
          </label>
        ) : (
          <div className="choices" role="radiogroup" aria-label="Answer choices">
            {q.choices.map((c, i) => (
              <button
                key={c}
                role="radio"
                aria-checked={answers[q.id] === c}
                className={`choice${answers[q.id] === c ? ' picked' : ''}`}
                onClick={() => setAnswers({ ...answers, [q.id]: c })}
              >
                <span className="key" aria-hidden="true">
                  {String.fromCharCode(65 + i)}
                </span>
                <MathText text={c} />
              </button>
            ))}
          </div>
        )}
        <div className="actions spread">
          <label className="flag-review">
            <input
              type="checkbox"
              checked={flags.has(q.id)}
              onChange={() => {
                const n = new Set(flags)
                if (n.has(q.id)) n.delete(q.id)
                else n.add(q.id)
                setFlags(n)
              }}
            />
            Flag for review
          </label>
          <span>
            <button className="secondary" disabled={index === 0} onClick={() => go(index - 1)}>
              ← Previous
            </button>{' '}
            {index + 1 < paper.questions.length ? (
              <button className="primary" onClick={() => go(index + 1)}>
                Next →
              </button>
            ) : (
              <button className="primary" onClick={() => setConfirming(true)}>
                Finish
              </button>
            )}
          </span>
        </div>
      </article>

      <nav className="test-grid" aria-label="Questions">
        {paper.questions.map((x, i) => (
          <button
            key={x.id}
            className={[i === index ? 'current' : '', (answers[x.id] ?? '').trim() ? 'done' : '', flags.has(x.id) ? 'flagged' : ''].join(' ')}
            onClick={() => go(i)}
            aria-label={`Question ${i + 1}${flags.has(x.id) ? ', flagged' : ''}${(answers[x.id] ?? '').trim() ? ', answered' : ''}`}
          >
            {i + 1}
          </button>
        ))}
      </nav>
      <p className="actions">
        <button className="secondary" onClick={() => setConfirming(true)}>
          Submit test
        </button>
      </p>

      {confirming && (
        <div className="overlay" onClick={() => setConfirming(false)}>
          <div className="lesson-panel" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
            <h2>Submit your test?</h2>
            <p>
              You answered {answeredCount} of {paper.questions.length}.{' '}
              {answeredCount < paper.questions.length && 'Blank answers count as wrong — a guess costs nothing.'}
              {flags.size > 0 && ` ${flags.size} flagged for review.`}
            </p>
            <div className="actions">
              <button className="secondary" onClick={() => setConfirming(false)}>
                Keep working
              </button>
              <button className="primary" disabled={submitting} onClick={() => void submit()}>
                {submitting ? 'Scoring…' : 'Submit'}
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  )
}

function TestReportView({ paper, report, onExit }: { paper: TestPaper; report: TestReport; onExit: () => void }) {
  const byId = useMemo(() => new Map(paper.questions.map((q) => [q.id, q])), [paper])
  const [showAll, setShowAll] = useState(false)
  const items = showAll ? report.items : report.items.filter((i) => !i.correct)
  const mins = Math.round(report.time_used_secs / 60)
  return (
    <section className="test-report">
      <div className="card report-head">
        <div className="big-score">{report.percent}%</div>
        <div>
          <h2>
            {report.correct} of {report.total} correct
          </h2>
          <p>
            <strong>{BAND_TEXT[report.band] ?? report.band}</strong> · {mins} min
          </p>
          <p className="muted fine-print">A practice estimate from this app, not an official GED score.</p>
        </div>
      </div>
      <h3>By skill</h3>
      <ul className="by-skill">
        {report.by_skill.map((s) => (
          <li key={s.skill}>
            <span>{s.name}</span>
            <span className="mini-bar" aria-hidden="true">
              <span style={{ width: `${(s.correct / s.total) * 100}%` }} />
            </span>
            <span>
              {s.correct}/{s.total}
            </span>
          </li>
        ))}
      </ul>
      <div className="practice-bar">
        <h3>{showAll ? 'Every question' : 'What you missed'}</h3>
        <button className="link" onClick={() => setShowAll(!showAll)}>
          {showAll ? 'Show missed only' : 'Show all'}
        </button>
      </div>
      {items.length === 0 && <p>Nothing missed. Excellent.</p>}
      {items.map((it) => {
        const q = byId.get(it.question_id)
        return (
          <article key={it.question_id} className={`card feedback ${it.correct ? 'right' : 'wrong'}`}>
            {q && (
              <p className="prompt small">
                <MathText text={q.prompt} />
              </p>
            )}
            <p>
              Your answer: <strong>{it.response ? <MathText text={it.response} /> : '— (blank)'}</strong>
              {!it.correct && (
                <>
                  {' '}
                  · Correct: <MathText text={it.answer} className="strong" />
                </>
              )}
            </p>
            <details>
              <summary>How to solve it</summary>
              <p className="explanation">
                <MathText text={it.explanation} />
              </p>
            </details>
            {!it.correct && it.hack && <Hack text={it.hack} />}
          </article>
        )
      })}
      <div className="actions">
        <button className="primary" onClick={onExit}>
          Done
        </button>
      </div>
    </section>
  )
}
