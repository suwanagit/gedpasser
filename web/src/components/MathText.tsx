import katex from 'katex'
import 'katex/dist/katex.min.css'
import { useMemo } from 'react'

type Piece = { math: boolean; text: string }

/**
 * Split question text into plain and math pieces.
 * Math goes between `$...$`; a literal dollar sign is written `\$`.
 * An unclosed `$` is shown as plain text.
 */
export function splitMath(source: string): Piece[] {
  const pieces: Piece[] = []
  let buf = ''
  let inMath = false
  for (let i = 0; i < source.length; i++) {
    const c = source[i]
    if (c === '\\' && source[i + 1] === '$') {
      buf += inMath ? '\\$' : '$'
      i++
    } else if (c === '$') {
      if (buf) pieces.push({ math: inMath, text: buf })
      buf = ''
      inMath = !inMath
    } else {
      buf += c
    }
  }
  if (inMath) pieces.push({ math: false, text: '$' + buf })
  else if (buf) pieces.push({ math: false, text: buf })
  return pieces
}

/** Renders text with inline KaTeX math. Line breaks in the text are kept. */
export function MathText({ text, className }: { text: string; className?: string }) {
  const pieces = useMemo(
    () =>
      splitMath(text).map((p) => ({
        ...p,
        // \displaystyle keeps fractions full-size inside a line of text.
        html: p.math
          ? katex.renderToString(`\\displaystyle ${p.text}`, { throwOnError: false, output: 'htmlAndMathml' })
          : '',
      })),
    [text],
  )
  return (
    <span className={className ? `math-text ${className}` : 'math-text'}>
      {pieces.map((p, i) =>
        p.math ? <span key={i} dangerouslySetInnerHTML={{ __html: p.html }} /> : <span key={i}>{p.text}</span>,
      )}
    </span>
  )
}
