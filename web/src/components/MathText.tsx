import katex from 'katex'
import 'katex/dist/katex.min.css'
import { Fragment, useMemo } from 'react'

type Piece = { kind: 'text' | 'inline' | 'display'; text: string }

/**
 * Split text into plain, inline-math ($...$) and display-math ($$...$$)
 * pieces. A literal dollar sign is written `\$`. Unclosed math is shown as
 * plain text.
 */
export function splitMath(source: string): Piece[] {
  const pieces: Piece[] = []
  let buf = ''
  let mode = 'text' as Piece['kind']
  const flush = (next: Piece['kind']) => {
    if (buf) pieces.push({ kind: mode, text: buf })
    buf = ''
    mode = next
  }
  for (let i = 0; i < source.length; i++) {
    const c = source[i]
    if (c === '\\' && source[i + 1] === '$') {
      buf += mode === 'text' ? '$' : '\\$'
      i++
    } else if (c === '$' && source[i + 1] === '$') {
      i++
      flush(mode === 'display' ? 'text' : mode === 'text' ? 'display' : mode)
    } else if (c === '$') {
      flush(mode === 'inline' ? 'text' : mode === 'text' ? 'inline' : mode)
    } else {
      buf += c
    }
  }
  if (mode !== 'text') {
    // Unclosed math: show it as written.
    pieces.push({ kind: 'text', text: (mode === 'display' ? '$$' : '$') + buf })
  } else if (buf) {
    pieces.push({ kind: 'text', text: buf })
  }
  return pieces
}

/** Plain text with **bold** runs. */
function Bold({ text }: { text: string }) {
  const parts = text.split('**')
  return (
    <>
      {parts.map((p, i) => (i % 2 === 1 ? <strong key={i}>{p}</strong> : <Fragment key={i}>{p}</Fragment>))}
    </>
  )
}

/**
 * Renders text with inline and display KaTeX math. Line breaks are kept;
 * display math sits on its own centered line. Bold runs (**like this**)
 * may contain inline math.
 */
export function MathText({ text, className }: { text: string; className?: string }) {
  const pieces = useMemo(() => {
    // Bold markers can wrap math, so split on ** first, then on math.
    const runs = text.split('**')
    return runs.map((run, r) =>
      splitMath(run).map((p) => ({
        ...p,
        bold: r % 2 === 1,
        html:
          p.kind === 'text'
            ? ''
            : katex.renderToString(p.kind === 'inline' ? `\\displaystyle ${p.text}` : p.text, {
                throwOnError: false,
                displayMode: p.kind === 'display',
                output: 'htmlAndMathml',
              }),
      })),
    )
  }, [text])
  const flat = pieces.flat()
  return (
    <span className={className ? `math-text ${className}` : 'math-text'}>
      {flat.map((p, i) => {
        if (p.kind === 'text') {
          // Display math already breaks the line; drop the newline next to it.
          let t = p.text
          if (flat[i - 1]?.kind === 'display') t = t.replace(/^\n/, '')
          if (flat[i + 1]?.kind === 'display') t = t.replace(/\n$/, '')
          if (!t) return null
          return p.bold ? <strong key={i}>{t}</strong> : <Bold key={i} text={t} />
        }
        const el = <span key={i} className={p.kind === 'display' ? 'math-block' : undefined} dangerouslySetInnerHTML={{ __html: p.html }} />
        return p.bold ? <strong key={i}>{el}</strong> : el
      })}
    </span>
  )
}
