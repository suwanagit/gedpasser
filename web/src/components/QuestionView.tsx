import { motion, useReducedMotion } from 'motion/react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { AnswerResult } from '../api/types/AnswerResult'
import type { PracticeMode } from '../api/types/PracticeMode'
import type { PracticeQuestion } from '../api/types/PracticeQuestion'
import { play } from '../sound'
import { AnswerInput } from './AnswerInput'
import { complete, isStructured } from './answers'
import { Calculator } from './Calculator'
import { Figure } from './Figure'
import { FlagDialog } from './FlagDialog'
import { Icon } from './Icon'
import { Hack } from './LessonCard'
import { LessonPanel } from './LessonPanel'
import { StudyLinks } from './StudyLinks'
import { MathText } from './MathText'
import { SessionShell } from './SessionShell'
import { Stimulus } from './Stimulus'

const CHEERS = ['Nice!', 'Correct!', 'You got it!', 'Great!']

/**
 * One question, full screen: answer, tap Check, get a short verdict in the
 * footer, tap Continue. The explanation is one tap away, never in the way.
 * Used by practice, review and the journey.
 */
export function QuestionView({
  q,
  mode,
  progress,
  onClose,
  onAnswered,
  onNext,
  onFlagged,
  showSkill = false,
}: {
  q: PracticeQuestion
  mode: PracticeMode
  progress: number | null
  onClose: () => void
  onAnswered?: (result: AnswerResult) => void
  onNext: () => void
  onFlagged: () => void
  showSkill?: boolean
}) {
  const [selected, setSelected] = useState<string | null>(null)
  const [typed, setTyped] = useState('')
  const [structured, setStructured] = useState('')
  const [calc, setCalc] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [answered, setAnswered] = useState<{ response: string; result: AnswerResult } | null>(null)
  const [why, setWhy] = useState(false)
  const [lessonFor, setLessonFor] = useState<string | null>(null)
  const [flagging, setFlagging] = useState(false)
  const [cheer] = useState(() => CHEERS[Math.floor(Math.random() * CHEERS.length)])
  const shownAt = useRef(performance.now())
  const hintUsed = useRef(false)
  const reduce = useReducedMotion()
  const closeLesson = useCallback(() => setLessonFor(null), [])
  const cancelFlag = useCallback(() => setFlagging(false), [])

  const response = isStructured(q) ? (complete(q, structured) ? structured : '') : q.kind === 'fill_in' ? typed.trim() : selected
  const submit = useCallback(async () => {
    if (answered || busy || !response) return
    setBusy(true)
    setError(null)
    try {
      const result = await api.answer({
        question_id: q.id,
        response,
        time_ms: Math.round(performance.now() - shownAt.current),
        hint_used: hintUsed.current,
        mode,
      })
      play(result.correct ? 'right' : 'wrong')
      setAnswered({ response, result })
      onAnswered?.(result)
    } catch (e) {
      setError(errorMessage(e))
    } finally {
      setBusy(false)
    }
  }, [answered, busy, q, response, mode, onAnswered])

  // Keyboard: 1–4 pick a choice, Enter checks or moves on.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const inField = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement
      if (answered) {
        if (e.key === 'Enter') {
          e.preventDefault()
          onNext()
        }
        return
      }
      if (q.kind === 'multiple_choice' && !inField) {
        const n = Number(e.key)
        if (n >= 1 && n <= q.choices.length) setSelected(q.choices[n - 1])
      }
      if (e.key === 'Enter') {
        e.preventDefault()
        void submit()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [answered, q, submit, onNext])

  const result = answered?.result
  const footer = result ? (
    <motion.div
      className="verdict"
      role="status"
      initial={reduce ? false : { y: 24, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: reduce ? 0 : 0.18 }}
    >
      <div className="verdict-head">
        <span className="verdict-mark">
          <Icon name={result.correct ? 'check' : 'x'} size={22} />
        </span>
        <h3>{result.correct ? cheer : 'Not quite'}</h3>
        {result.correct && <span className="pill xp">+{result.xp} XP</span>}
      </div>
      {!result.correct && (
        <p className="verdict-answer">
          Answer: <MathText text={result.answer} className="strong" />
        </p>
      )}
      {!result.correct && result.hack && !why && <Hack text={result.hack} />}
      {why && (
        <div className="verdict-why">
          <div className="work-lines">
            {result.explanation.split('\n').filter((l) => l.trim()).map((line, i) => (
              <div key={i}>
                <MathText text={line} />
              </div>
            ))}
          </div>
          <p>
            <strong>Watch out:</strong> <MathText text={result.common_mistake} />
          </p>
          {result.study && (
            <div className="verdict-study">
              <p className="eyebrow">Study this</p>
              <StudyLinks study={result.study} />
            </div>
          )}
          <button className="link" onClick={() => setLessonFor(result.principle.id)}>
            Quick review
          </button>
        </div>
      )}
      <div className="verdict-actions">
        {!why && (
          <button className="link" onClick={() => setWhy(true)}>
            Show me how
          </button>
        )}
        <button className={`primary wide ${result.correct ? 'ok' : 'bad'}`} autoFocus onClick={onNext}>
          Continue
        </button>
      </div>
    </motion.div>
  ) : (
    <div className="check-row">
      <button
        className="secondary"
        aria-label="Quick review"
        onClick={() => {
          hintUsed.current = true
          setLessonFor(q.principle.id)
        }}
      >
        <Icon name="bulb" size={20} /> Quick review
      </button>
      {q.calculator && (
        <button className={`secondary${calc ? ' on' : ''}`} aria-label="Calculator" aria-pressed={calc} onClick={() => setCalc(!calc)}>
          <Icon name="calc" size={20} />
        </button>
      )}
      <button className="primary wide" disabled={!response || busy} onClick={() => void submit()}>
        Check
      </button>
    </div>
  )

  return (
    <SessionShell
      progress={progress}
      onClose={onClose}
      onFlag={() => setFlagging(true)}
      tone={result ? (result.correct ? 'right' : 'wrong') : undefined}
      footer={footer}
    >
      <motion.div
        className={`question${q.stimulus ? ' with-stimulus' : ''}`}
        initial={reduce ? false : { opacity: 0, x: 28 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ duration: reduce ? 0 : 0.2 }}
      >
        {(showSkill || q.draft) && (
          <p className="eyebrow">
            {showSkill && q.skill_name} {q.draft && <span className="pill draft">Draft</span>}
          </p>
        )}
        {q.stimulus && <Stimulus s={q.stimulus} />}
        <div className="question-main">
          {q.kind !== 'dropdown' && (
            <p className="prompt">
              <MathText text={q.prompt} />
            </p>
          )}
          {q.figure && <Figure view={q.figure} />}

          {isStructured(q) ? (
            <AnswerInput q={q} value={answered ? answered.response : structured} onChange={setStructured} disabled={!!answered} />
          ) : q.kind === 'fill_in' ? (
          <input
            className="answer-box"
            autoFocus
            inputMode="text"
            autoComplete="off"
            aria-label="Your answer"
            placeholder="Type your answer"
            value={answered ? answered.response : typed}
            disabled={!!answered}
            onChange={(e) => setTyped(e.target.value)}
          />
        ) : (
          <div className="choices" role="radiogroup" aria-label="Answer choices">
            {q.choices.map((c, i) => {
              const isRight = !!result && c === result.answer
              const isPicked = (answered ? answered.response : selected) === c
              const cls = ['choice', isPicked ? 'picked' : '', isRight ? 'right' : '', result && isPicked && !isRight ? 'wrong' : ''].join(' ')
              return (
                <button key={c} className={cls} role="radio" aria-checked={isPicked} disabled={!!answered} onClick={() => setSelected(c)}>
                  <span className="key" aria-hidden="true">
                    {i + 1}
                  </span>
                  <MathText text={c} />
                </button>
              )
            })}
          </div>
        )}
          {error && <p className="error">{error}</p>}
        </div>
      </motion.div>

      {calc && <Calculator onClose={() => setCalc(false)} />}
      {lessonFor && <LessonPanel principleId={lessonFor} onClose={closeLesson} />}
      {flagging && (
        <FlagDialog
          questionId={q.id}
          onDone={() => {
            setFlagging(false)
            onFlagged()
          }}
          onCancel={cancelFlag}
        />
      )}
    </SessionShell>
  )
}
