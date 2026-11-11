import { useEffect, useState } from 'react'
import { api, errorMessage } from '../api/client'
import type { Formula } from '../api/types/Formula'
import { MathText } from './MathText'

/** The formula sheet, grouped. Loads from the strategy content. */
export function FormulaSheet({ formulas: given }: { formulas?: Formula[] }) {
  const [formulas, setFormulas] = useState<Formula[] | null>(given ?? null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    if (given) return
    api
      .strategy()
      .then((s) => setFormulas(s.formulas))
      .catch((e) => setError(errorMessage(e)))
  }, [given])
  if (error) return <p className="error">{error}</p>
  if (!formulas) return <p className="muted">Loading…</p>
  if (formulas.length === 0) return <p className="muted">The formula sheet isn’t available yet.</p>
  const groups = [...new Set(formulas.map((f) => f.group))]
  return (
    <div className="formula-sheet">
      {groups.map((g) => (
        <section key={g}>
          <h4>{g}</h4>
          <dl>
            {formulas
              .filter((f) => f.group === g)
              .map((f) => (
                <div key={f.name} className="formula">
                  <dt>{f.name}</dt>
                  <dd>
                    <MathText text={`$$${f.tex}$$`} />
                  </dd>
                </div>
              ))}
          </dl>
        </section>
      ))}
    </div>
  )
}
