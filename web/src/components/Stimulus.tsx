import type { StimulusView } from '../api/types/StimulusView'
import { Figure } from './Figure'
import { LessonText } from './LessonText'
import { MathText } from './MathText'

/**
 * A shared text, table or figure that several questions are about. On a
 * phone it sits above the question and can be folded away; on a wide
 * screen it sits beside the question, as on the real test.
 */
export function Stimulus({ s }: { s: StimulusView }) {
  return (
    <details className="stimulus" open>
      <summary>{s.title ?? 'Read this first'}</summary>
      {s.text && <LessonText text={s.text} />}
      {s.table && (
        <div className="stim-table-wrap">
          <table className="stim-table">
            <thead>
              <tr>
                {s.table.headers.map((h, i) => (
                  <th key={i}>
                    <MathText text={h} />
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {s.table.rows.map((row, r) => (
                <tr key={r}>
                  {row.map((cell, c) => (
                    <td key={c}>
                      <MathText text={cell} />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {s.figure && <Figure view={s.figure} />}
      <p className="stim-source">Source: {s.source}</p>
    </details>
  )
}
