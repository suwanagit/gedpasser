import { useCallback, useEffect, useState } from 'react'
import { ApiError, api, errorMessage, type Scope } from './api/client'
import type { Me as MeInfo } from './api/types/Me'
import type { Plan } from './api/types/Plan'
import type { PlanTask } from './api/types/PlanTask'
import type { Stats } from './api/types/Stats'
import type { TestKind } from './api/types/TestKind'
import { Icon, type IconName } from './components/Icon'
import { Mascot } from './components/Mascot'
import { Learn, StageRunner } from './screens/Journey'
import { Library, type LibraryTab } from './screens/Library'
import { Me } from './screens/Me'
import { Practice, PracticeMenu } from './screens/Practice'
import { ProfilePicker } from './screens/ProfilePicker'
import { TestRunner } from './screens/Tests'

type Tab = 'learn' | 'practice' | 'library' | 'me'

type View =
  | { kind: 'learn' }
  | { kind: 'stage'; stageId: string; run: number }
  | { kind: 'practice-menu' }
  | { kind: 'practice'; scope: Scope; setLength: number; run: number; back: Tab }
  | { kind: 'test'; testKind: TestKind; run: number }
  | { kind: 'library'; tab: LibraryTab }
  | { kind: 'me' }

type Screen =
  | { kind: 'starting' }
  | { kind: 'error'; message: string }
  | { kind: 'profiles' }
  | { kind: 'app'; me: MeInfo; view: View }

const TABS: { tab: Tab; label: string; icon: IconName }[] = [
  { tab: 'learn', label: 'Learn', icon: 'route' },
  { tab: 'practice', label: 'Practice', icon: 'target' },
  { tab: 'library', label: 'Library', icon: 'book' },
  { tab: 'me', label: 'Me', icon: 'user' },
]

function tabView(t: Tab): View {
  switch (t) {
    case 'learn':
      return { kind: 'learn' }
    case 'practice':
      return { kind: 'practice-menu' }
    case 'library':
      return { kind: 'library', tab: 'lessons' }
    case 'me':
      return { kind: 'me' }
  }
}

function tabOf(v: View): Tab | null {
  switch (v.kind) {
    case 'learn':
      return 'learn'
    case 'practice-menu':
      return 'practice'
    case 'library':
      return 'library'
    case 'me':
      return 'me'
    default:
      return null // a session: full screen, no navigation
  }
}

/** The tab named in the address bar, so reload and the Back button work. */
function hashTab(): Tab {
  const h = window.location.hash.slice(1)
  return TABS.some((t) => t.tab === h) ? (h as Tab) : 'learn'
}

export default function App() {
  const [screen, setScreen] = useState<Screen>({ kind: 'starting' })
  const [stats, setStats] = useState<Stats | null>(null)
  const [plan, setPlan] = useState<Plan | null>(null)

  useEffect(() => {
    api
      .me()
      .then((me) => setScreen({ kind: 'app', me, view: tabView(hashTab()) }))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 401) setScreen({ kind: 'profiles' })
        else setScreen({ kind: 'error', message: errorMessage(e) })
      })
  }, [])

  const me = screen.kind === 'app' ? screen.me : null
  const view = screen.kind === 'app' ? screen.view : null
  const current = view ? tabOf(view) : null

  const refresh = useCallback(() => {
    Promise.all([api.stats(), api.plan()])
      .then(([s, p]) => {
        setStats(s)
        setPlan(p)
      })
      .catch(() => {
        // the screens show their own errors
      })
  }, [])

  // Fresh numbers every time the learner lands on a tab.
  const meId = me?.id
  useEffect(() => {
    if (meId !== undefined && current) refresh()
  }, [meId, current, refresh])

  // Back and Forward move between tabs; Back from a session returns to its tab.
  useEffect(() => {
    const onPop = () => setScreen((s) => (s.kind === 'app' ? { ...s, view: tabView(hashTab()) } : s))
    window.addEventListener('popstate', onPop)
    return () => window.removeEventListener('popstate', onPop)
  }, [])

  async function logout() {
    try {
      await api.logout()
    } finally {
      setStats(null)
      setPlan(null)
      setScreen({ kind: 'profiles' })
    }
  }

  const go = (next: View) => {
    if (me) {
      const hash = `#${tabOf(next) ?? 'go'}`
      // Leaving a session replaces its entry, so Back never lands on a dead one.
      if (view && tabOf(view) === null) window.history.replaceState(null, '', hash)
      else if (window.location.hash !== hash) window.history.pushState(null, '', hash)
      setScreen({ kind: 'app', me, view: next })
      window.scrollTo({ top: 0 })
    }
  }
  const back: Tab = current ?? (view?.kind === 'practice' ? view.back : 'learn')
  const start = (scope: Scope, setLength = 10) => go({ kind: 'practice', scope, setLength, run: Date.now(), back })
  const openStage = (stageId: string) => go({ kind: 'stage', stageId, run: Date.now() })
  const startTest = (testKind: TestKind) => go({ kind: 'test', testKind, run: Date.now() })

  function runTask(t: PlanTask) {
    switch (t.kind) {
      case 'diagnostic':
        return startTest('short')
      case 'full_test':
        return startTest('full')
      case 'power_set':
        return start({ mode: 'principle', id: t.principles.join(','), name: 'Power set' })
      case 'review':
        return start({ mode: 'review' })
      case 'hacks':
        return go({ kind: 'library', tab: 'hacks' })
      case 'tactics':
        return go({ kind: 'library', tab: 'testday' })
      case 'set_date':
        return go({ kind: 'me' })
      case 'journey':
      case 'rest':
        return go({ kind: 'learn' })
    }
  }

  if (screen.kind === 'starting')
    return (
      <div className="center loading">
        <Mascot mood="think" />
      </div>
    )
  if (screen.kind === 'error')
    return (
      <div className="center loading">
        <Mascot mood="think" />
        <p className="error">Can’t reach the server.</p>
        <p className="muted small">{screen.message}</p>
      </div>
    )
  if (screen.kind === 'profiles' || !me || !view)
    return <ProfilePicker onLoggedIn={(m) => setScreen({ kind: 'app', me: m, view: { kind: 'learn' } })} />

  if (view.kind === 'stage')
    return <StageRunner key={view.run} stageId={view.stageId} onExit={() => go({ kind: 'learn' })} onOpen={openStage} />
  if (view.kind === 'practice')
    return (
      <Practice
        key={view.run}
        scope={view.scope}
        setLength={view.setLength}
        onExit={() => go(tabView(view.back))}
        onAgain={() => start(view.scope, view.setLength)}
      />
    )
  if (view.kind === 'test') return <TestRunner key={view.run} kind={view.testKind} onExit={() => go({ kind: 'practice-menu' })} />

  const days = plan?.days_left ?? null
  return (
    <div className="shell">
      <nav className="rail" aria-label="Main">
        <span className="brand">
          <Mascot size={40} /> GED Passer
        </span>
        {TABS.map((t) => (
          <button key={t.tab} className={current === t.tab ? 'nav-btn on' : 'nav-btn'} aria-current={current === t.tab ? 'page' : undefined} onClick={() => go(tabView(t.tab))}>
            <Icon name={t.icon} size={26} />
            <span>{t.label}</span>
          </button>
        ))}
      </nav>

      <div className="main">
        <header className="topbar">
          <span className="brand brand-small">
            <Mascot size={32} />
          </span>
          <span className="chips">
            <span className="chip flame" title="Day streak">
              <Icon name="flame" fill size={20} />
              {stats?.streak_days ?? 0}
              <span className="sr-only"> day streak</span>
            </span>
            <span className="chip xp" title="XP">
              <Icon name="bolt" fill size={20} />
              {stats?.xp ?? 0}
              <span className="sr-only"> XP</span>
            </span>
            <button className="chip date" onClick={() => go({ kind: 'me' })} title="Test date">
              <Icon name="calendar" size={20} />
              {days === null ? 'Set date' : days < 0 ? 'Past' : days === 0 ? 'Today!' : `${days}d`}
            </button>
          </span>
        </header>
        {me.serve_drafts && <p className="dev-banner">Dev mode: drafts shown</p>}

        <main className="page">
          {view.kind === 'learn' && <Learn stats={stats} plan={plan} onOpen={openStage} onTask={runTask} />}
          {view.kind === 'practice-menu' && <PracticeMenu stats={stats} plan={plan} onStart={start} onTest={startTest} />}
          {view.kind === 'library' && <Library initialTab={view.tab} onPractice={start} />}
          {view.kind === 'me' && (
            <Me name={me.display_name} stats={stats} plan={plan} onChanged={refresh} onStart={start} onLogout={() => void logout()} />
          )}
        </main>
      </div>

      <nav className="bottom-nav" aria-label="Main">
        {TABS.map((t) => (
          <button key={t.tab} className={current === t.tab ? 'nav-btn on' : 'nav-btn'} aria-current={current === t.tab ? 'page' : undefined} onClick={() => go(tabView(t.tab))}>
            <Icon name={t.icon} size={26} />
            <span>{t.label}</span>
          </button>
        ))}
      </nav>
    </div>
  )
}
