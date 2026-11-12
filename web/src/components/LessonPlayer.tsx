import { motion, useReducedMotion } from 'motion/react'
import { useEffect, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { LessonView } from '../api/types/LessonView'
import type { TryItView } from '../api/types/TryItView'
import { play } from '../sound'
import { Figure } from './Figure'
import { Icon } from './Icon'
import { LessonText } from './LessonText'
import { Mascot } from './Mascot'
import { MathText } from './MathText'
import { SessionShell } from './SessionShell'

type Page =
  | { kind: 'goal' }
  | { kind: 'problem' }
  | { kind: 'idea' }
  | { kind: 'example'; index: number; step: number }
  | { kind: 'turn'; index: number }
  | { kind: 'howto' }
  | { kind: 'mistake'; index: number }
  | { kind: 'practice'; index: number }
  | { kind: 'review' }
  | { kind: 'rate' }

/** Every screen of a lesson, in the order of docs/LESSON_STYLE.md. */
function pagesOf(l: LessonView): Page[] {
  const pages: Page[] = [{ kind: 'goal' }, { kind: 'problem' }, { kind: 'idea' }]
  l.examples.forEach((e, index) => {
    e.steps.forEach((_, step) => pages.push({ kind: 'example', index, step }))
    pages.push({ kind: 'turn', index })
  })
  pages.push({ kind: 'howto' })
  l.mistakes.forEach((_, index) => pages.push({ kind: 'mistake', index }))
  l.practice.forEach((_, index) => pages.push({ kind: 'practice', index }))
  pages.push({ kind: 'review' }, { kind: 'rate' })
  return pages
}

function questionOf(l: LessonView, p: Page): TryItView | null {
  if (p.kind === 'turn') return l.examples[p.index].turn
  if (p.kind === 'practice') return l.practice[p.index]
  return null
}

/**
 * A full lesson, one screen at a time: goal, a problem to think about, the
 * idea with pictures, two worked examples each followed by "your turn",
 * how-to steps, common mistakes, practice, the lesson's Quick review, and
 * "How did that go?".
 */
export function LessonPlayer({
  lessonId,
  startAtReview = false,
  onClose,
  onPractice,
}: {
  lessonId: string
  /** Open straight on the Quick review page. */
  startAtReview?: boolean
  onClose: () => void
  onPractice?: () => void
}) {
  const [lesson, setLesson] = useState<LessonView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [at, setAt] = useState(0)
  const [picked, setPicked] = useState('')
  const [checked, setChecked] = useState<boolean | null>(null)
  const reduce = useReducedMotion()

  useEffect(() => {
    let live = true
    api
      .fullLesson(lessonId)
      .then((l) => {
        if (!live) return
        setLesson(l)
        if (startAtReview) setAt(pagesOf(l).findIndex((p) => p.kind === 'review'))
      })
      .catch((e) => live && setError(errorMessage(e)))
    return () => {
      live = false
    }
  }, [lessonId, startAtReview])

  const pages = lesson ? pagesOf(lesson) : []
  const page = pages[at]
  const question = lesson && page ? questionOf(lesson, page) : null

  const goTo = (i: number) => {
    setPicked('')
    setChecked(null)
    setAt(i)
  }
  const next = () => (at + 1 >= pages.length ? onClose() : goTo(at + 1))
  const check = () => {
    if (!question || !picked.trim()) return
    const ok = same(picked, question.answer)
    play(ok ? 'right' : 'wrong')
    setChecked(ok)
  }

  // Keep the newest thing in view: the step just added, or the worked
  // solution. Any other new page starts at the top.
  useEffect(() => {
    const body = document.querySelector('.lesson-player .session-body')
    const p = lesson ? pagesOf(lesson)[at] : undefined
    if (!body || !p) return
    const frame = requestAnimationFrame(() => {
      const target =
        checked !== null ? body.querySelector('.solution, .verdict-note') : p.kind === 'example' && p.step > 0 ? body.querySelector('.worked li:last-child') : null
      if (target) target.scrollIntoView({ block: 'nearest', behavior: reduce ? 'auto' : 'smooth' })
      else if (checked === null) body.scrollTo({ top: 0 })
    })
    return () => cancelAnimationFrame(frame)
  }, [at, checked, lesson, reduce])

  // Enter moves on (or checks an answer).
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key !== 'Enter' || !page || page.kind === 'rate') return
      e.preventDefault()
      if (question && checked === null) check()
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

  const lastStep = page.kind === 'example' && page.step + 1 === lesson.examples[page.index].steps.length
  let footer
  if (page.kind === 'rate') footer = undefined
  else if (question)
    footer =
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
            <h3>{checked ? 'You got it!' : 'Not quite. Here is how.'}</h3>
          </div>
          <button className={`primary wide ${checked ? 'ok' : 'bad'}`} autoFocus onClick={next}>
            Continue
          </button>
        </div>
      )
  else
    footer = (
      <button className="primary wide" autoFocus onClick={next}>
        {page.kind === 'example' && !lastStep ? 'Next step' : page.kind === 'problem' ? "Let's find out" : 'Continue'}
      </button>
    )

  return (
    <div className="lesson-player">
      <SessionShell
        progress={(at + 1) / pages.length}
        onClose={onClose}
        tone={question && checked !== null ? (checked ? 'right' : 'wrong') : undefined}
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
          <Body
            lesson={lesson}
            page={page}
            picked={picked}
            setPicked={setPicked}
            checked={checked}
            onDone={onClose}
            onPractice={onPractice}
            onAgain={() => goTo(pages.findIndex((p) => p.kind === 'idea'))}
          />
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
  onDone,
  onPractice,
  onAgain,
}: {
  lesson: LessonView
  page: Page
  picked: string
  setPicked: (s: string) => void
  checked: boolean | null
  onDone: () => void
  onPractice?: () => void
  onAgain: () => void
}) {
  const reduce = useReducedMotion()
  switch (page.kind) {
    case 'goal':
      return (
        <div className="lesson-goal">
          <Mascot mood="happy" size={96} />
          <h2>{lesson.title}</h2>
          <p>
            <MathText text={lesson.goal} />
          </p>
        </div>
      )
    case 'problem':
      return (
        <>
          <h2>Think about this</h2>
          <LessonText text={lesson.problem} />
        </>
      )
    case 'idea':
      return (
        <>
          <h2>The idea</h2>
          {lesson.idea.split('{{pictures}}').map((part, i) => (
            <div key={i} className="idea-part">
              {i > 0 && (
                <div className="figure-row">
                  {lesson.figures.map((f, j) => (
                    <Figure key={j} view={f} />
                  ))}
                </div>
              )}
              {part.trim() && <LessonText text={part.trim()} />}
            </div>
          ))}
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
                <p className="say">
                  <MathText text={s.say} />
                </p>
                {s.work.length > 0 && (
                  <div className="work-lines">
                    {s.work.map((w, j) => (
                      <div key={j}>
                        <MathText text={w} />
                      </div>
                    ))}
                  </div>
                )}
              </motion.li>
            ))}
            {done && (
              <li className="worked-end">
                <p className="worked-answer">
                  <MathText text={ex.answer} />
                </p>
                <p className="worked-check">
                  <Icon name="check" size={18} /> <MathText text={ex.check} />
                </p>
                {page.index === 0 && (
                  <p className="worked-problem">
                    <MathText text={lesson.problem_answer} />
                  </p>
                )}
              </li>
            )}
          </ol>
        </>
      )
    }
    case 'turn':
    case 'practice': {
      const q = page.kind === 'turn' ? lesson.examples[page.index].turn : lesson.practice[page.index]
      return (
        <>
          <h2>{page.kind === 'turn' ? 'Your turn' : `Practice ${page.index + 1} of ${lesson.practice.length}`}</h2>
          <div className="prompt">
            <LessonText text={q.prompt} />
          </div>
          {q.choices.length > 0 ? (
            <div className="choices" role="radiogroup" aria-label="Answer choices">
              {q.choices.map((c, i) => {
                const isPicked = picked === c
                const isRight = checked !== null && same(c, q.answer)
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
                  The answer is <MathText text={q.answer} />.
                </strong>
              </p>
              <LessonText text={q.solution} />
            </div>
          )}
          {checked === true && <p className="verdict-note muted">Nice. Keep going.</p>}
        </>
      )
    }
    case 'howto':
      return (
        <>
          <h2>How to do it</h2>
          <p className="muted">This is a short way to say what you did in the examples.</p>
          <ol className="how-to">
            {lesson.how_to.map((h, i) => (
              <li key={i}>
                <MathText text={h} />
              </li>
            ))}
          </ol>
          <div className="hook">
            <Icon name="bulb" size={20} />
            <span>
              <MathText text={lesson.hook} />
            </span>
          </div>
        </>
      )
    case 'mistake': {
      const m = lesson.mistakes[page.index]
      return (
        <>
          <h2>Watch out</h2>
          <div className="mistake wrong">
            <span className="mistake-mark">
              <Icon name="x" size={18} /> Wrong
            </span>
            <LessonText text={m.wrong} />
          </div>
          <div className="mistake right">
            <span className="mistake-mark">
              <Icon name="check" size={18} /> Right
            </span>
            <LessonText text={m.right} />
          </div>
          <LessonText text={m.why} />
        </>
      )
    }
    case 'review':
      return <QuickReview lesson={lesson} />
    case 'rate':
      return (
        <div className="lesson-rate">
          <Mascot mood="cheer" size={96} />
          <h2>How did that go?</h2>
          <button className="primary wide" onClick={onDone}>
            I've got it
          </button>
          {onPractice && (
            <button className="secondary wide" onClick={onPractice}>
              I need more practice
            </button>
          )}
          <button className="secondary wide" onClick={onAgain}>
            Show me again
          </button>
        </div>
      )
  }
}

/** A lesson's own Quick review: its how-to steps, a small example, the hook. */
export function QuickReview({ lesson }: { lesson: LessonView }) {
  return (
    <article className="lesson-card quick">
      <p className="eyebrow">Quick review</p>
      <h3>{lesson.title}</h3>
      <ol className="how-to">
        {lesson.how_to.map((h, i) => (
          <li key={i}>
            <MathText text={h} />
          </li>
        ))}
      </ol>
      <div className="card-example">
        <span className="eyebrow">Example</span>
        <div className="work-lines">
          {lesson.review_example.split('\n').map((line, i) => (
            <div key={i}>
              <MathText text={line} />
            </div>
          ))}
        </div>
      </div>
      <div className="hook">
        <Icon name="bulb" size={20} />
        <span>
          <MathText text={lesson.hook} />
        </span>
      </div>
    </article>
  )
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
