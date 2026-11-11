import { motion, useReducedMotion } from 'motion/react'
import type { ReactNode } from 'react'
import type { CardView } from '../api/types/CardView'
import { Figure } from './Figure'
import { Icon } from './Icon'
import { MathText } from './MathText'

/** How many pieces a card has: rule, steps, example, hack. */
export const CARD_PARTS = 4

/**
 * A principle's short card. `parts` reveals it a piece at a time (rule,
 * steps, example, hack); leave it out to show the whole card.
 */
export function LessonCard({ card, parts = CARD_PARTS }: { card: CardView; parts?: number }) {
  return (
    <article className="lesson-card">
      <h3>
        {card.title} {card.draft && <span className="pill draft">Draft</span>}
      </h3>
      <div className="card-key">
        <MathText text={card.key} />
      </div>
      {card.figure && <Figure view={card.figure} />}
      {parts >= 2 && (
        <Reveal>
          <ol className="card-steps">
            {card.steps.map((s, i) => (
              <li key={i}>
                <span>
                  <MathText text={s} />
                </span>
              </li>
            ))}
          </ol>
        </Reveal>
      )}
      {parts >= 3 && (
        <Reveal>
          <div className="card-example">
            <span className="eyebrow">Example</span>
            <MathText text={card.example} />
          </div>
        </Reveal>
      )}
      {parts >= 4 && (
        <Reveal>
          <Hack text={card.hack} />
        </Reveal>
      )}
    </article>
  )
}

function Reveal({ children }: { children: ReactNode }) {
  const reduce = useReducedMotion()
  return (
    <motion.div initial={reduce ? false : { opacity: 0, y: 14 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: reduce ? 0 : 0.22 }}>
      {children}
    </motion.div>
  )
}

/** A memory hack: the thing to carry into the test. */
export function Hack({ text }: { text: string }) {
  return (
    <aside className="hack" aria-label="Memory hack">
      <Icon name="bulb" size={20} />
      <span>
        <MathText text={text} />
      </span>
    </aside>
  )
}
