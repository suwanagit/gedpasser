import { useEffect, useState } from 'react'
import type { Health } from './api/types/Health'

type Status =
  | { kind: 'loading' }
  | { kind: 'ok'; health: Health }
  | { kind: 'error'; message: string }

export default function App() {
  const [status, setStatus] = useState<Status>({ kind: 'loading' })

  useEffect(() => {
    fetch('/api/health')
      .then((r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`)
        return r.json() as Promise<Health>
      })
      .then((health) => setStatus({ kind: 'ok', health }))
      .catch((e: unknown) =>
        setStatus({ kind: 'error', message: e instanceof Error ? e.message : String(e) }),
      )
  }, [])

  return (
    <main className="page">
      <h1>GED Passer</h1>
      <p className="lead">Free GED test practice for Alaskans. Math practice is coming soon.</p>

      <section className="card" aria-live="polite">
        <h2>Server status</h2>
        {status.kind === 'loading' && <p>Checking…</p>}
        {status.kind === 'ok' && (
          <p>
            <span className="dot ok" aria-hidden="true" /> Online · version {status.health.version} ·{' '}
            {status.health.skills} math skills loaded
          </p>
        )}
        {status.kind === 'error' && (
          <p>
            <span className="dot err" aria-hidden="true" /> Can’t reach the server ({status.message})
          </p>
        )}
      </section>

      <footer className="fine">
        Not affiliated with or endorsed by GED Testing Service. GED® is a registered trademark of the
        American Council on Education.
      </footer>
    </main>
  )
}
