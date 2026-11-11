import { useEffect, useState } from 'react'
import { ApiError, api, errorMessage } from './api/client'
import type { Me } from './api/types/Me'
import type { SkillSummary } from './api/types/SkillSummary'
import { Lessons } from './screens/Lessons'
import { Practice } from './screens/Practice'
import { ProfilePicker } from './screens/ProfilePicker'
import { SkillList } from './screens/SkillList'

type Screen =
  | { kind: 'starting' }
  | { kind: 'error'; message: string }
  | { kind: 'profiles' }
  | { kind: 'skills'; me: Me }
  | { kind: 'practice'; me: Me; skill: SkillSummary }
  | { kind: 'lessons'; me: Me }

export default function App() {
  const [screen, setScreen] = useState<Screen>({ kind: 'starting' })

  useEffect(() => {
    api
      .me()
      .then((me) => setScreen({ kind: 'skills', me }))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 401) setScreen({ kind: 'profiles' })
        else setScreen({ kind: 'error', message: errorMessage(e) })
      })
  }, [])

  const me = screen.kind === 'skills' || screen.kind === 'practice' || screen.kind === 'lessons' ? screen.me : null

  async function logout() {
    try {
      await api.logout()
    } finally {
      setScreen({ kind: 'profiles' })
    }
  }

  return (
    <div className="app">
      <header className="top">
        <span className="brand">GED Passer</span>
        {me && (
          <span className="who">
            <button className="link" onClick={() => setScreen({ kind: 'skills', me })}>
              Practice
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
        {screen.kind === 'profiles' && <ProfilePicker onLoggedIn={(me) => setScreen({ kind: 'skills', me })} />}
        {screen.kind === 'skills' && (
          <SkillList onPick={(skill) => setScreen({ kind: 'practice', me: screen.me, skill })} />
        )}
        {screen.kind === 'practice' && (
          <Practice skill={screen.skill} onExit={() => setScreen({ kind: 'skills', me: screen.me })} />
        )}
        {screen.kind === 'lessons' && <Lessons onBack={() => setScreen({ kind: 'skills', me: screen.me })} />}
      </main>

      <footer className="fine">
        Free GED test practice. Not affiliated with or endorsed by GED Testing Service. GED® is a registered
        trademark of the American Council on Education.
      </footer>
    </div>
  )
}
