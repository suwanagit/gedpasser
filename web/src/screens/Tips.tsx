import { useEffect, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { StrategyView } from '../api/types/StrategyView'
import { FormulaSheet } from '../components/FormulaSheet'
import { MathText } from '../components/MathText'

type Tab = 'tactics' | 'hacks' | 'formulas'

/** Test-day tactics, every memory hack, and the formula sheet. Printable. */
export function Tips() {
  const [data, setData] = useState<StrategyView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [tab, setTab] = useState<Tab>('tactics')
  useEffect(() => {
    api.strategy().then(setData).catch((e) => setError(errorMessage(e)))
  }, [])
  if (error) return <p className="error">{error}</p>
  if (!data) return <p className="muted">Loading…</p>
  return (
    <section className="tips">
      <div className="practice-bar">
        <h2>Tips &amp; tricks</h2>
        <button className="secondary small no-print" onClick={() => window.print()}>
          Print cheat sheet
        </button>
      </div>
      <div className="tabs no-print" role="tablist">
        {(['tactics', 'hacks', 'formulas'] as Tab[]).map((t) => (
          <button key={t} role="tab" aria-selected={tab === t} className={tab === t ? 'tab on' : 'tab'} onClick={() => setTab(t)}>
            {t === 'tactics' ? 'Test day' : t === 'hacks' ? 'Memory hacks' : 'Formulas'}
          </button>
        ))}
      </div>

      <div className={tab === 'tactics' ? '' : 'print-only'}>
        <h3 className="print-only">Test day</h3>
        <ol className="tactics">
          {data.tactics.map((t) => (
            <li key={t.id} className="card tactic">
              <strong>{t.title}</strong>
              <span>{t.text}</span>
            </li>
          ))}
        </ol>
      </div>

      <div className={tab === 'hacks' ? '' : 'print-only'}>
        <h3 className="print-only">Memory hacks</h3>
        <ul className="hack-list">
          {data.hacks.map((h) => (
            <li key={h.principle.id}>
              <span className="badge">{h.principle.id}</span> <strong>{h.title}</strong>
              <p>
                <MathText text={h.hack} />
              </p>
            </li>
          ))}
        </ul>
      </div>

      <div className={tab === 'formulas' ? '' : 'print-only'}>
        <h3 className="print-only">Formulas</h3>
        <FormulaSheet formulas={data.formulas} />
      </div>
    </section>
  )
}
