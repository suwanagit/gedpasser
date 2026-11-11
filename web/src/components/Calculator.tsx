import { useState } from 'react'

/**
 * A small on-screen calculator: + − × ÷, powers, square roots, π and
 * parentheses. Uses its own parser (no eval).
 */
export function Calculator({ onClose }: { onClose: () => void }) {
  const [expr, setExpr] = useState('')
  const [result, setResult] = useState<string>('')

  function press(k: string) {
    if (k === 'C') {
      setExpr('')
      setResult('')
    } else if (k === '⌫') {
      setExpr((e) => e.slice(0, -1))
    } else if (k === '=') {
      const v = evaluate(expr)
      setResult(v === null ? 'Error' : format(v))
    } else if (k === 'ans') {
      if (result && result !== 'Error') setExpr((e) => e + result)
    } else {
      setExpr((e) => e + k)
    }
  }

  const keys = ['C', '(', ')', '⌫', '7', '8', '9', '÷', '4', '5', '6', '×', '1', '2', '3', '−', '0', '.', '^', '+', '√(', 'π', 'ans', '=']
  return (
    <div className="calculator" role="group" aria-label="Calculator">
      <div className="calc-head">
        <strong>Calculator</strong>
        <button className="link" onClick={onClose} aria-label="Close calculator">
          ×
        </button>
      </div>
      <input
        className="calc-display"
        value={expr}
        onChange={(e) => setExpr(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault()
            e.stopPropagation()
            press('=')
          }
        }}
        aria-label="Expression"
        inputMode="decimal"
      />
      <div className="calc-result" aria-live="polite">
        {result ? `= ${result}` : ' '}
      </div>
      <div className="calc-keys">
        {keys.map((k) => (
          <button key={k} className={k === '=' ? 'primary' : 'secondary'} onClick={() => press(k)}>
            {k}
          </button>
        ))}
      </div>
    </div>
  )
}

function format(v: number): string {
  if (!isFinite(v)) return 'Error'
  const r = Math.round(v * 1e10) / 1e10
  return String(r)
}

/** Evaluate a calculator expression; null if it doesn't parse. */
export function evaluate(src: string): number | null {
  const s = src.replace(/×/g, '*').replace(/÷/g, '/').replace(/−/g, '-').replace(/π/g, 'p').replace(/√/g, 'r').replace(/\s+/g, '')
  let i = 0
  const peek = () => s[i]
  function expr(): number {
    let v = term()
    while (peek() === '+' || peek() === '-') {
      const op = s[i++]
      const r = term()
      v = op === '+' ? v + r : v - r
    }
    return v
  }
  function term(): number {
    let v = unary()
    while (peek() === '*' || peek() === '/' || peek() === '(' || peek() === 'p' || peek() === 'r') {
      // Implicit multiplication: 2(3), 2π, 2√(9)
      const op = peek() === '*' || peek() === '/' ? s[i++] : '*'
      const r = unary()
      v = op === '*' ? v * r : v / r
    }
    return v
  }
  function unary(): number {
    if (peek() === '-') {
      i++
      return -unary()
    }
    return power()
  }
  function power(): number {
    const b = atom()
    if (peek() === '^') {
      i++
      return Math.pow(b, unary())
    }
    return b
  }
  function atom(): number {
    const c = peek()
    if (c === '(') {
      i++
      const v = expr()
      if (s[i++] !== ')') throw new Error('paren')
      return v
    }
    if (c === 'p') {
      i++
      return Math.PI
    }
    if (c === 'r') {
      i++
      return Math.sqrt(atom())
    }
    const m = /^\d*\.?\d+(?:e[+-]?\d+)?/.exec(s.slice(i)) ?? /^\d+\.?/.exec(s.slice(i))
    if (!m) throw new Error('number')
    i += m[0].length
    return parseFloat(m[0])
  }
  try {
    if (!s) return null
    const v = expr()
    if (i !== s.length) return null
    return v
  } catch {
    return null
  }
}
