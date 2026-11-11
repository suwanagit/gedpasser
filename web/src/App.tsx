import { useEffect, useState } from 'react'
import { ApiError, api, errorMessage, type Scope } from './api/client'
import type { Me } from './api/types/Me'
import type { TestKind } from './api/types/TestKind'
import { Dashboard } from './screens/Dashboard'
import { Home } from './screens/Home'
import { JourneyMap, StageRunner } from './screens/Journey'
import { Lessons } from './screens/Lessons'
import { Practice } from './screens/Practice'
import { ProfilePicker } from './screens/ProfilePicker'
import { TestRunner, TestsHome } from './screens/Tests'
import { Tips } from './screens/Tips'

type Tab = 'home' | 'journey' | 'practice' | 'tests' | 'explore' | 'tips'

type View =
  | { kind: 'home' }
  | { kind: 'journey' }
  | { kind: 'stage'; stageId: string; run: number }
  | { kind: 'practice-menu' }
  | { kind: 'practice'; scope: Scope; setLength: number | null; run: number; back: Tab }
  | { kind: 'tests' }
  | { kind: 'test'; testKind: TestKind; run: number }
  | { kind: 'explore' }
  | { kind: 'tips' }

type Screen =
  | { kind: 'starting' }
  | { kind: 'error'; message: string }
  | { kind: 'profiles' }
  | { kind: 'app'; me: Me; view: View }

const TABS: { tab: Tab; label: string; icon: string }[] = [
  { tab: 'home', label: 'Home', icon: '🏠' },
  { tab: 'journey', label: 'Journey', icon: '🗺' },
  { tab: 'practice', label: 'Practice', icon: '✏️' },
  { tab: 'tests', label: 'Tests', icon: '⏱' },
  { tab: 'explore', label: 'Explore', icon: '📚' },
  { tab: 'tips', label: 'Tips', icon: '💡' },
]

function tabOf(v: View): Tab {
  switch (v.kind) {
    case 'home':
      return 'home'
    case 'journey':
    case 'stage':
      return 'journey'
    case 'practice-menu':
      return 'practice'
    case 'practice':
      return v.back
    case 'tests':
    case 'test':
      return 'tests'
    case 'explore':
      return 'explore'
    case 'tips':
      return 'tips'
  }
}

function tabView(t: Tab): View {
  switch (t) {
    case 'home':
      return { kind: 'home' }
    case 'journey':
      return { kind: 'journey' }
    case 'practice':
      return { kind: 'practice-menu' }
    case 'tests':
      return { kind: 'tests' }
    case 'explore':
      return { kind: 'explore' }
    case 'tips':
      return { kind: 'tips' }
  }
}

export default function App() {
  const [screen, setScreen] = useState<Screen>({ kind: 'starting' })

  useEffect(() => {
    api
      .me()
      .then((me) => setScreen({ kind: 'app', me, view: { kind: 'home' } }))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 401) setScreen({ kind: 'profiles' })
        else setScreen({ kind: 'error', message: errorMessage(e) })
      })
  }, [])

  async function logout() {
    try {
      await api.logout()
    } finally {
      setScreen({ kind: 'profiles' })
    }
  }

  const me = screen.kind === 'app' ? screen.me : null
  const go = (view: View) => {
    if (me) {
      setScreen({ kind: 'app', me, view })
      window.scrollTo({ top: 0 })
    }
  }
  const start = (scope: Scope, setLength: number | null, back: Tab) =>
    go({ kind: 'practice', scope, setLength, run: Date.now(), back })
  const openStage = (stageId?: string) =>
    go(stageId ? { kind: 'stage', stageId, run: Date.now() } : { kind: 'journey' })
  const startTest = (testKind: TestKind) => go({ kind: 'test', testKind, run: Date.now() })

  const view = screen.kind === 'app' ? screen.view : null
  const current = view ? tabOf(view) : null
  // A running test hides the navigation, like the real thing.
  const inTest = view?.kind === 'test'

  return (
    <div className="app">
      <header className="top">
        <span className="brand">GED Passer</span>
        {me && !inTest && (
          <nav className="tabs-nav" aria-label="Main">
            {TABS.map((t) => (
              <button key={t.tab} className={current === t.tab ? 'nav-btn on' : 'nav-btn'} aria-current={current === t.tab ? 'page' : undefined} onClick={() => go(tabView(t.tab))}>
                <span className="nav-icon" aria-hidden="true">
                  {t.icon}
                </span>
                <span className="nav-label">{t.label}</span>
              </button>
            ))}
          </nav>
        )}
        {me && (
          <span className="who">
            <span className="who-name">{me.display_name}</span>
            <button className="secondary small" onClick={() => void logout()}>
              Log out
            </button>
          </span>
        )}
      </header>
      {me?.serve_drafts && <p className="dev-banner">Development mode: unreviewed draft questions are being shown.</p>}

      <main className="page">
        {screen.kind === 'starting' && <p className="muted">Loading…</p>}
        {screen.kind === 'error' && <p className="error">Can’t reach the server ({screen.message}). Is it running?</p>}
        {screen.kind === 'profiles' && (
          <ProfilePicker onLoggedIn={(me) => setScreen({ kind: 'app', me, view: { kind: 'home' } })} />
        )}
        {view?.kind === 'home' && (
          <Home onStart={(s, n) => start(s, n, 'home')} onJourney={openStage} onTest={startTest} onTips={() => go({ kind: 'tips' })} />
        )}
        {view?.kind === 'journey' && <JourneyMap onOpen={(id) => openStage(id)} />}
        {view?.kind === 'stage' && (
          <StageRunner key={view.run} stageId={view.stageId} onExit={() => go({ kind: 'journey' })} onOpen={(id) => openStage(id)} />
        )}
        {view?.kind === 'practice-menu' && <Dashboard onStart={(s, n) => start(s, n, 'practice')} />}
        {view?.kind === 'practice' && (
          <Practice
            key={view.run}
            scope={view.scope}
            setLength={view.setLength}
            onExit={() => go(tabView(view.back))}
            onStart={(s, n) => start(s, n, view.back)}
          />
        )}
        {view?.kind === 'tests' && <TestsHome onStart={startTest} />}
        {view?.kind === 'test' && <TestRunner key={view.run} kind={view.testKind} onExit={() => go({ kind: 'tests' })} />}
        {view?.kind === 'explore' && (
          <Lessons onBack={() => go({ kind: 'home' })} onPractice={(s, n) => start(s, n, 'explore')} />
        )}
        {view?.kind === 'tips' && <Tips />}
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
