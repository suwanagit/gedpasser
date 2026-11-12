import { motion, useReducedMotion } from 'motion/react'
import { useEffect, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { LessonView } from '../api/types/LessonView'
import type { TryItView } from '../api/types/TryItView'
import { play } from '../sound'
import { Icon } from './Icon'
import { LessonText } from './LessonText'
import { Mascot } from './Mascot'
import { MathText } from './MathText'
import { SessionShell } from './SessionShell'

type Page =
  | { kind: 'why' }
  | { kind: 'idea' }
  | { kind: 'example'; index: number; step: number }
  | { kind: 'rule' }
  | { kind: 'mistake' }
  | { kind: 'try'; index: number }
  | { kind: 'recap' }

/** Every screen of a lesson, in the order of docs/LESSON_STYLE.md. */
function pagesOf(l: LessonView): Page[] {
  const pages: Page[] = [{ kind: 'why' }, { kind: 'idea' }]
  l.examples.forEach((e, index) => e.steps.forEach((_, step) => pages.push({ kind: 'example', index, step })))
  pages.push({ kind: 'rule' }, { kind: 'mistake' })
  l.tries.forEach((_, index) => pages.push({ kind: 'try', index }))
  pages.push({ kind: 'recap' })
  return pages
}

/**
 * A full lesson, one step on screen at a time: why it matters, the idea,
 * two worked examples a step per tap, the rule, a common mistake, try-its
 * with instant feedback, and a recap that leads to Quick review.
 */
export function LessonPlayer({
  lessonId,
  onClose,
  onQuickReview,
  onPractice,
}: {
  lessonId: string
  onClose: () => void
  onQuickReview?: () => void
  onPractice?: () => void
}) {
  const [lesson, setLesson] = useState<LessonView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [at, setAt] = useState(0)
  // Try-it state for the page on screen.
  const [picked, setPicked] = useState('')
  const [checked, setChecked] = useState<boolean | null>(null)
  const reduce = useReducedMotion()

  useEffect(() => {
    let live = true
    api
      .fullLesson(lessonId)
      .then((l) => live && setLesson(l))
      .catch((e) => live && setError(errorMessage(e)))
    return () => {
      live = false
    }
  }, [lessonId])

  const pages = lesson ? pagesOf(lesson) : []
  const page = pages[at]
  const tryIt: TryItView | null = lesson && page?.kind === 'try' ? lesson.tries[page.index] : null
  const last = at + 1 >= pages.length

  const next = () => {
    setPicked('')
    setChecked(null)
    if (last) onClose()
    else setAt(at + 1)
  }
  const check = () => {
    if (!tryIt || !picked.trim()) return
    const ok = same(picked, tryIt.answer)
    play(ok ? 'right' : 'wrong')
    setChecked(ok)
  }

  // Keep the newest thing in view: the step just revealed, the answer, or
  // the worked solution. A new kind of page starts at the top.
  useEffect(() => {
    const body = document.querySelector('.lesson-player .session-body')
    const page = lesson ? pagesOf(lesson)[at] : undefined
    if (!body || !page) return
    const frame = requestAnimationFrame(() => {
      const target =
        checked === false
          ? body.querySelector('.solution')
          : page.kind === 'example' && page.step > 0
            ? (body.querySelector('.worked-answer') ?? body.querySelector('.worked li.now'))
            : null
      if (target) target.scrollIntoView({ block: 'nearest', behavior: reduce ? 'auto' : 'smooth' })
      else if (checked === null) body.scrollTo({ top: 0 })
    })
    return () => cancelAnimationFrame(frame)
  }, [at, checked, lesson, reduce])

  // Enter moves on (or checks a try-it).
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key !== 'Enter' || !page) return
      e.preventDefault()
      if (tryIt && checked === null) check()
      else next()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  if (error || !lesson || !page)
    return (
      <div className="lesson-player">
        <SessionShell progress={null} onClose={onClose}>
          <div className="center loading">
            <Mascot mood="think" />
            {error && <p className="error">{error}</p>}
          </div>
        </SessionShell>
      </div>
    )

  const isLastStep = page.kind === 'example' && page.step + 1 === lesson.examples[page.index].steps.length
  const footer = tryIt ? (
    checked === null ? (
      <button className="primary wide" disabled={!picked.trim()} onClick={check}>
        Check
      </button>
    ) : (
      <div className="verdict" role="status">
        <div className="verdict-head">
          <span className="verdict-mark">
            <Icon name={checked ? 'check' : 'x'} size={22} />
          </span>
          <h3>{checked ? 'You got it!' : 'Not quite'}</h3>
        </div>
        <button className={`primary wide ${checked ? 'ok' : 'bad'}`} autoFocus onClick={next}>
          Continue
        </button>
      </div>
    )
  ) : page.kind === 'recap' ? (
    <div className="recap-actions">
      {lesson.has_quick_review && onQuickReview && (
        <button className="secondary" onClick={onQuickReview}>
          <Icon name="bulb" size={20} /> Quick review
        </button>
      )}
      {onPractice && (
        <button className="secondary" onClick={onPractice}>
          <Icon name="target" size={20} /> Practice
        </button>
      )}
      <button className="primary wide" autoFocus onClick={onClose}>
        Done
      </button>
    </div>
  ) : (
    <button className="primary wide" autoFocus onClick={next}>
      {page.kind === 'example' && !isLastStep ? 'Next step' : 'Continue'}
    </button>
  )

  return (
    <div className="lesson-player">
      <SessionShell
        progress={(at + 1) / pages.length}
        onClose={onClose}
        tone={tryIt && checked !== null ? (checked ? 'right' : 'wrong') : undefined}
        footer={footer}
      >
        <motion.div
          key={page.kind === 'example' ? `ex${page.index}` : at}
          className="lesson-page"
          initial={reduce ? false : { opacity: 0, x: 28 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: reduce ? 0 : 0.2 }}
        >
          <p className="eyebrow">
            {lesson.title} {lesson.draft && <span className="pill draft">Draft</span>}
          </p>
          <Body lesson={lesson} page={page} picked={picked} setPicked={setPicked} checked={checked} />
        </motion.div>
      </SessionShell>
    </div>
  )
}

function Body({
  lesson,
  page,
  picked,
  setPicked,
  checked,
}: {
  lesson: LessonView
  page: Page
  picked: string
  setPicked: (s: string) => void
  checked: boolean | null
}) {
  const reduce = useReducedMotion()
  switch (page.kind) {
    case 'why':
      return (
        <>
          <h2>Why it matters</h2>
          <LessonText text={lesson.why} />
        </>
      )
    case 'idea':
      return (
        <>
          <h2>The idea</h2>
          <LessonText text={lesson.idea} />
        </>
      )
    case 'example': {
      const ex = lesson.examples[page.index]
      const done = page.step + 1 === ex.steps.length
      return (
        <>
          <h2>
            Example {page.index + 1}: <MathText text={ex.title} />
          </h2>
          <ol className="worked">
            {ex.steps.slice(0, page.step + 1).map((s, i) => (
              <motion.li
                key={i}
                className={i === page.step ? 'now' : ''}
                initial={reduce || i < page.step ? false : { opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: reduce ? 0 : 0.2 }}
              >
                <span className="work">
                  <MathText text={s.work} />
                </span>
                <span className="why">
                  <MathText text={s.why} />
                </span>
              </motion.li>
            ))}
          </ol>
          {done && (
            <p className="worked-answer">
              <MathText text={ex.answer} />
            </p>
          )}
        </>
      )
    }
    case 'rule':
      return (
        <>
          <h2>The rule</h2>
          <p className="muted">This is a short way to say what you did in the examples.</p>
          <div className="rule-box">
            <LessonText text={lesson.rule} />
          </div>
          {lesson.terms.length > 0 && (
            <dl className="terms">
              {lesson.terms.map((t) => (
                <div key={t.term}>
                  <dt>
                    <MathText text={t.term} />
                  </dt>
                  <dd>
                    <MathText text={t.means} />
                  </dd>
                </div>
              ))}
            </dl>
          )}
        </>
      )
    case 'mistake':
      return (
        <>
          <h2>A common mistake</h2>
          <div className="mistake wrong">
            <span className="mistake-mark">
              <Icon name="x" size={18} /> Wrong
            </span>
            <LessonText text={lesson.mistake.wrong} />
          </div>
          <div className="mistake right">
            <span className="mistake-mark">
              <Icon name="check" size={18} /> Right
            </span>
            <LessonText text={lesson.mistake.right} />
          </div>
          <LessonText text={lesson.mistake.why} />
        </>
      )
    case 'try': {
      const t = lesson.tries[page.index]
      return (
        <>
          <h2>
            Try it {page.index + 1} of {lesson.tries.length}
          </h2>
          <div className="prompt">
            <LessonText text={t.prompt} />
          </div>
          {t.choices.length > 0 ? (
            <div className="choices" role="radiogroup" aria-label="Answer choices">
              {t.choices.map((c, i) => {
                const isPicked = picked === c
                const isRight = checked !== null && same(c, t.answer)
                const cls = ['choice', isPicked ? 'picked' : '', isRight ? 'right' : '', checked !== null && isPicked && !isRight ? 'wrong' : ''].join(' ')
                return (
                  <button key={c} className={cls} role="radio" aria-checked={isPicked} disabled={checked !== null} onClick={() => setPicked(c)}>
                    <span className="key" aria-hidden="true">
                      {i + 1}
                    </span>
                    <MathText text={c} />
                  </button>
                )
              })}
            </div>
          ) : (
            <input
              className="answer-box"
              autoFocus
              inputMode="decimal"
              autoComplete="off"
              aria-label="Your answer"
              placeholder="Type your answer"
              value={picked}
              disabled={checked !== null}
              onChange={(e) => setPicked(e.target.value)}
            />
          )}
          {checked === false && (
            <div className="solution">
              <p>
                <strong>
                  Answer: <MathText text={t.answer} />
                </strong>
              </p>
              <p className="eyebrow">How to get it</p>
              <LessonText text={t.solution} />
            </div>
          )}
        </>
      )
    }
    case 'recap':
      return (
        <>
          <Mascot mood="cheer" size={88} />
          <h2>Quick recap</h2>
          <ul className="recap">
            {lesson.recap.map((r, i) => (
              <li key={i}>
                <Icon name="check" size={18} />
                <span>
                  <MathText text={r} />
                </span>
              </li>
            ))}
          </ul>
        </>
      )
  }
}

/** Compare a learner's answer with the key: as numbers when both are, else as text. */
function same(given: string, want: string): boolean {
  const clean = (s: string) => s.replace(/[$,\s]/g, '').toLowerCase()
  const a = clean(given)
  const b = clean(want)
  const na = Number(a)
  const nb = Number(b)
  if (a !== '' && b !== '' && !Number.isNaN(na) && !Number.isNaN(nb)) return Math.abs(na - nb) < 1e-9
  return a === b
}
