import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { TestKind } from '../api/types/TestKind'
import type { TestPaper } from '../api/types/TestPaper'
import type { TestReport } from '../api/types/TestReport'
import { AnswerInput } from '../components/AnswerInput'
import { isStructured } from '../components/answers'
import { Calculator } from '../components/Calculator'
import { Figure } from '../components/Figure'
import { FormulaSheet } from '../components/FormulaSheet'
import { Icon } from '../components/Icon'
import { Hack } from '../components/LessonCard'
import { Mascot } from '../components/Mascot'
import { MathText } from '../components/MathText'
import { Ring } from '../components/Ring'
import { SessionShell } from '../components/SessionShell'
import { Stimulus } from '../components/Stimulus'

const BAND_TEXT: Record<string, string> = {
  ready: 'Looking ready',
  close: 'Close',
  building: 'Building',
  early: 'Early days',
}

/**
 * Take a test: timer, question list, mark for review, calculator, formula
 * sheet. Plain on purpose: it should feel like test day, not a game.
 */
export function TestRunner({ kind, subject = 'math', onExit }: { kind: TestKind; subject?: string; onExit: () => void }) {
  const [paper, setPaper] = useState<TestPaper | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [index, setIndex] = useState(0)
  const [answers, setAnswers] = useState<Record<string, string>>({})
  const [flags, setFlags] = useState<Set<string>>(new Set())
  const [left, setLeft] = useState(0)
  const [tool, setTool] = useState<'none' | 'calc' | 'formulas'>('none')
  const [grid, setGrid] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const [report, setReport] = useState<TestReport | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const spent = useRef<Record<string, number>>({})
  const shownAt = useRef(performance.now())

  useEffect(() => {
    api
      .newTest(kind, subject)
      .then((p) => {
        setPaper(p)
        setLeft(p.time_limit_secs)
        shownAt.current = performance.now()
      })
      .catch((e) => setError(errorMessage(e)))
  }, [kind, subject])

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

  if (error)
    return (
      <SessionShell
        progress={null}
        onClose={onExit}
        footer={
          <button className="primary wide" onClick={onExit}>
            Back
          </button>
        }
      >
        <div className="center loading">
          <Mascot mood="think" />
          <p className="error">{error}</p>
        </div>
      </SessionShell>
    )
  if (report && paper) return <TestReportView paper={paper} report={report} onExit={onExit} />
  if (!paper)
    return (
      <div className="center loading">
        <Mascot mood="think" />
      </div>
    )

  const q = paper.questions[index]
  const go = (i: number) => {
    track()
    setIndex(i)
  }
  const isAnswered = (id: string) => (answers[id] ?? '').trim() !== ''
  const answeredCount = paper.questions.filter((x) => isAnswered(x.id)).length
  const mm = Math.floor(left / 60)
  const ss = String(left % 60).padStart(2, '0')
  const last = index + 1 >= paper.questions.length
  const toggleFlag = () => {
    const n = new Set(flags)
    if (n.has(q.id)) n.delete(q.id)
    else n.add(q.id)
    setFlags(n)
  }
  return (
    <div className="session test">
      <header className="test-top">
        <span className={`timer${left < 300 ? ' low' : ''}`}>
          <Icon name="clock" size={20} /> {mm}:{ss}
        </span>
        <button className="secondary small" onClick={() => setGrid(!grid)} aria-expanded={grid}>
          {index + 1} / {paper.questions.length} <Icon name="down" size={16} />
        </button>
        <span className="test-tools">
          {(subject === 'math' || q.calculator) && (
            <button
              className={`icon-btn${tool === 'calc' ? ' on' : ''}`}
              aria-label="Calculator"
              aria-pressed={tool === 'calc'}
              onClick={() => setTool(tool === 'calc' ? 'none' : 'calc')}
            >
              <Icon name="calc" />
            </button>
          )}
          {subject === 'math' && (
            <button
              className={`icon-btn${tool === 'formulas' ? ' on' : ''}`}
              aria-label="Formula sheet"
              aria-pressed={tool === 'formulas'}
              onClick={() => setTool(tool === 'formulas' ? 'none' : 'formulas')}
            >
              <Icon name="sigma" />
            </button>
          )}
        </span>
      </header>

      <div className="session-body">
        <div className="session-inner">
          {grid && (
            <nav className="test-grid" aria-label="Questions">
              {paper.questions.map((x, i) => (
                <button
                  key={x.id}
                  className={[i === index ? 'current' : '', isAnswered(x.id) ? 'done' : '', flags.has(x.id) ? 'flagged' : ''].join(' ')}
                  onClick={() => {
                    go(i)
                    setGrid(false)
                  }}
                  aria-label={`Question ${i + 1}${flags.has(x.id) ? ', marked' : ''}${isAnswered(x.id) ? ', answered' : ''}`}
                >
                  {i + 1}
                </button>
              ))}
              <button className="secondary small submit-early" onClick={() => setConfirming(true)}>
                Submit test
              </button>
            </nav>
          )}
          {tool === 'formulas' && (
            <div className="panel">
              <FormulaSheet />
            </div>
          )}
          <div className={`question${q.stimulus ? ' with-stimulus' : ''}`} key={q.id}>
            {q.stimulus && <Stimulus s={q.stimulus} />}
            <div className="question-main">
            {q.kind !== 'dropdown' && (
              <p className="prompt">
                <MathText text={q.prompt} />
              </p>
            )}
            {q.figure && <Figure view={q.figure} />}
            {isStructured(q) ? (
              <AnswerInput q={q} value={answers[q.id] ?? ''} onChange={(v) => setAnswers({ ...answers, [q.id]: v })} />
            ) : q.kind === 'fill_in' ? (
              <input
                className="answer-box"
                aria-label="Your answer"
                placeholder="Type your answer"
                value={answers[q.id] ?? ''}
                onChange={(e) => setAnswers({ ...answers, [q.id]: e.target.value })}
                autoComplete="off"
              />
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
            </div>
          </div>
        </div>
      </div>
      {tool === 'calc' && <Calculator onClose={() => setTool('none')} />}

      <footer className="session-foot">
        <div className="session-inner test-foot">
          <button className="secondary" disabled={index === 0} onClick={() => go(index - 1)} aria-label="Previous question">
            <Icon name="back" />
          </button>
          <button className={`secondary${flags.has(q.id) ? ' on' : ''}`} aria-pressed={flags.has(q.id)} onClick={toggleFlag}>
            <Icon name="flag" size={18} /> Mark
          </button>
          {last ? (
            <button className="primary wide" onClick={() => setConfirming(true)}>
              Finish
            </button>
          ) : (
            <button className="primary wide" onClick={() => go(index + 1)}>
              Next
            </button>
          )}
        </div>
      </footer>

      {confirming && (
        <div className="overlay" onClick={() => setConfirming(false)}>
          <div className="lesson-panel" role="dialog" aria-modal="true" aria-labelledby="submit-title" onClick={(e) => e.stopPropagation()}>
            <h2 id="submit-title">Submit your test?</h2>
            <p>
              {answeredCount} of {paper.questions.length} answered
              {flags.size > 0 && ` · ${flags.size} marked`}
            </p>
            {answeredCount < paper.questions.length && <p className="muted">Blanks count as wrong. Guessing is free.</p>}
            <div className="actions">
              <button className="secondary" onClick={() => setConfirming(false)}>
                Keep going
              </button>
              <button className="primary" disabled={submitting} onClick={() => void submit()}>
                {submitting ? 'Scoring…' : 'Submit'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

function TestReportView({ paper, report, onExit }: { paper: TestPaper; report: TestReport; onExit: () => void }) {
  const byId = useMemo(() => new Map(paper.questions.map((q) => [q.id, q])), [paper])
  const missed = report.items.filter((i) => !i.correct)
  const mins = Math.round(report.time_used_secs / 60)
  return (
    <SessionShell
      progress={null}
      onClose={onExit}
      footer={
        <button className="primary wide" onClick={onExit}>
          Done
        </button>
      }
    >
      <div className="report">
        <div className="center">
          <Ring value={report.percent / 100} size={132} stroke={14}>
            <span className="big-score">{report.percent}%</span>
          </Ring>
          <h1>{BAND_TEXT[report.band] ?? report.band}</h1>
          <p className="muted">
            {report.correct} / {report.total} · {mins} min
          </p>
          <p className="muted small">A practice estimate, not a GED score.</p>
        </div>

        <h2>By skill</h2>
        <ul className="rows">
          {report.by_skill.map((s) => (
            <li key={s.skill} className="row skill-row">
              <span>{s.name}</span>
              <span className="bar">
                <span style={{ width: `${(s.correct / s.total) * 100}%` }} />
              </span>
              <strong>
                {s.correct}/{s.total}
              </strong>
            </li>
          ))}
        </ul>

        {missed.length > 0 && <h2>Missed</h2>}
        {missed.map((it) => {
          const q = byId.get(it.question_id)
          return (
            <details key={it.question_id} className="fold panel">
              <summary>
                <span className="fold-text">{q && <MathText text={q.prompt} />}</span>
                <Icon name="down" size={18} />
              </summary>
              <p>
                You: <strong>{it.response ? <MathText text={it.response} /> : 'blank'}</strong> · Answer: <MathText text={it.answer} className="strong" />
              </p>
              <p>
                <MathText text={it.explanation} />
              </p>
              {it.hack && <Hack text={it.hack} />}
            </details>
          )
        })}
      </div>
    </SessionShell>
  )
}
