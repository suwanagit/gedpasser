import type { CardView } from '../api/types/CardView'
import { Figure } from './Figure'
import { MathText } from './MathText'

/**
 * A principle's short card: the one rule, a few steps, one worked example,
 * a memory hack. Built to be read in under a minute.
 */
export function LessonCard({ card, compact = false }: { card: CardView; compact?: boolean }) {
  return (
    <article className={`lesson-card${compact ? ' compact' : ''}`}>
      <header className="lesson-card-head">
        <span className="badge">{card.principle.id}</span>
        <h3>{card.title}</h3>
        {card.draft && <span className="badge draft">Draft</span>}
      </header>
      <div className="card-key">
        <MathText text={card.key} />
      </div>
      {card.figure && <Figure view={card.figure} />}
      <ol className="card-steps">
        {card.steps.map((s, i) => (
          <li key={i}>
            <MathText text={s} />
          </li>
        ))}
      </ol>
      <div className="card-example">
        <span className="card-label">Example</span>
        <MathText text={card.example} />
      </div>
      <Hack text={card.hack} />
    </article>
  )
}

/** A memory hack, styled as a sticky note learners can carry into the test. */
export function Hack({ text }: { text: string }) {
  return (
    <aside className="hack" aria-label="Memory hack">
      <span className="hack-icon" aria-hidden="true">
        💡
      </span>
      <span>
        <MathText text={text} />
      </span>
    </aside>
  )
}
