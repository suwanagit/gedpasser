import { useEffect, useState } from 'react'
import { ApiError, api, errorMessage, type Scope } from './api/client'
import type { Me } from './api/types/Me'
import { Dashboard } from './screens/Dashboard'
import { Lessons } from './screens/Lessons'
import { Practice } from './screens/Practice'
import { ProfilePicker } from './screens/ProfilePicker'

type Screen =
  | { kind: 'starting' }
  | { kind: 'error'; message: string }
  | { kind: 'profiles' }
  | { kind: 'dashboard'; me: Me }
  | { kind: 'practice'; me: Me; scope: Scope; setLength: number | null; run: number }
  | { kind: 'lessons'; me: Me }

export default function App() {
  const [screen, setScreen] = useState<Screen>({ kind: 'starting' })

  useEffect(() => {
    api
      .me()
      .then((me) => setScreen({ kind: 'dashboard', me }))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 401) setScreen({ kind: 'profiles' })
        else setScreen({ kind: 'error', message: errorMessage(e) })
      })
  }, [])

  const me = 'me' in screen ? screen.me : null

  async function logout() {
    try {
      await api.logout()
    } finally {
      setScreen({ kind: 'profiles' })
    }
  }

  // Each start gets a new `run`, so "Another set" remounts a fresh session.
  function start(me: Me, scope: Scope, setLength: number | null) {
    setScreen({ kind: 'practice', me, scope, setLength, run: Date.now() })
  }

  return (
    <div className="app">
      <header className="top">
        <span className="brand">GED Passer</span>
        {me && (
          <span className="who">
            <button className="link" onClick={() => setScreen({ kind: 'dashboard', me })}>
              Dashboard
            </button>
            <button className="link" onClick={() => setScreen({ kind: 'lessons', me })}>
              Lessons
            </button>
            <span className="who-name">{me.display_name}</span>
            <button className="secondary small" onClick={() => void logout()}>
              Log out
            </button>
          </span>
        )}
      </header>
      {me?.serve_drafts && (
        <p className="dev-banner">Development mode: unreviewed draft questions are being shown.</p>
      )}

      <main className="page">
        {screen.kind === 'starting' && <p className="muted">Loading…</p>}
        {screen.kind === 'error' && (
          <p className="error">Can’t reach the server ({screen.message}). Is it running?</p>
        )}
        {screen.kind === 'profiles' && <ProfilePicker onLoggedIn={(me) => setScreen({ kind: 'dashboard', me })} />}
        {screen.kind === 'dashboard' && <Dashboard onStart={(scope, n) => start(screen.me, scope, n)} />}
        {screen.kind === 'practice' && (
          <Practice
            key={screen.run}
            scope={screen.scope}
            setLength={screen.setLength}
            onExit={() => setScreen({ kind: 'dashboard', me: screen.me })}
            onStart={(scope, n) => start(screen.me, scope, n)}
          />
        )}
        {screen.kind === 'lessons' && (
          <Lessons
            onBack={() => setScreen({ kind: 'dashboard', me: screen.me })}
            onPractice={(scope, n) => start(screen.me, scope, n)}
          />
        )}
      </main>

      <footer className="fine">
        <p>
          Questions and lessons are written and checked by AI and may contain mistakes. If something looks wrong, use
          “Flag this question”.
        </p>
        <p>
          Free GED test practice. Not affiliated with or endorsed by GED Testing Service. GED® is a registered
          trademark of the American Council on Education.
        </p>
      </footer>
    </div>
  )
}
