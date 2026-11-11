import { motion, useReducedMotion } from 'motion/react'
import { useState } from 'react'

const COLORS = ['var(--brand)', 'var(--sun)', 'var(--coral)', 'var(--violet)', 'var(--blue)', 'var(--ok)']

/** A short burst of confetti. Nothing is drawn under reduced motion. */
export function Confetti() {
  const reduce = useReducedMotion()
  const [bits] = useState(() =>
      Array.from({ length: 36 }, (_, i) => ({
        x: (Math.random() - 0.5) * 520,
        y: 260 + Math.random() * 260,
        r: Math.random() * 720 - 360,
        d: Math.random() * 0.25,
        c: COLORS[i % COLORS.length],
      })),
  )
  if (reduce) return null
  return (
    <div className="confetti" aria-hidden="true">
      {bits.map((b, i) => (
        <motion.span
          key={i}
          style={{ background: b.c }}
          initial={{ x: 0, y: -40, opacity: 1, rotate: 0 }}
          animate={{ x: b.x, y: b.y, opacity: 0, rotate: b.r }}
          transition={{ duration: 1.6, delay: b.d, ease: 'easeOut' }}
        />
      ))}
    </div>
  )
}
