import { useCallback, useEffect, useState } from 'react'
import { ApiError, api, errorMessage, type Scope } from './api/client'
import type { Me as MeInfo } from './api/types/Me'
import type { Plan } from './api/types/Plan'
import type { PlanTask } from './api/types/PlanTask'
import type { Stats } from './api/types/Stats'
import type { TestKind } from './api/types/TestKind'
import { Icon, type IconName } from './components/Icon'
import { Mascot } from './components/Mascot'
import { StageRunner, SubjectPath } from './screens/Journey'
import { Learn } from './screens/Learn'
import { Library, LibrarySubject, TestDay, type LibraryTab } from './screens/Library'
import { Me } from './screens/Me'
import { Practice, PracticeMenu, PracticeSubject } from './screens/Practice'
import { ProfilePicker } from './screens/ProfilePicker'
import { TestRunner } from './screens/Tests'
import { SUBJECTS, subjectOf, type Subject, type SubjectId } from './subjects'

type Tab = 'learn' | 'practice' | 'library' | 'me'

/** A screen with the navigation showing: a tab, or a page one layer down. */
type Place =
  | { kind: 'learn' }
  | { kind: 'path'; subject: SubjectId }
  | { kind: 'practice-menu' }
  | { kind: 'practice-subject'; subject: SubjectId }
  | { kind: 'library' }
  | { kind: 'library-subject'; subject: SubjectId; tab: LibraryTab }
  | { kind: 'testday' }
  | { kind: 'me' }

/** A session is full screen, and remembers where to go back to. */
type Session =
  | { kind: 'stage'; stageId: string; run: number; back: Place }
  | { kind: 'practice'; scope: Scope; setLength: number; run: number; back: Place }
  | { kind: 'test'; testKind: TestKind; run: number; back: Place }

type View = Place | Session

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

function tabView(t: Tab): Place {
  switch (t) {
    case 'learn':
      return { kind: 'learn' }
    case 'practice':
      return { kind: 'practice-menu' }
    case 'library':
      return { kind: 'library' }
    case 'me':
      return { kind: 'me' }
  }
}

function isSession(v: View): v is Session {
  return v.kind === 'stage' || v.kind === 'practice' || v.kind === 'test'
}

function tabOf(p: Place): Tab {
  switch (p.kind) {
    case 'learn':
    case 'path':
      return 'learn'
    case 'practice-menu':
    case 'practice-subject':
      return 'practice'
    case 'library':
    case 'library-subject':
    case 'testday':
      return 'library'
    case 'me':
      return 'me'
  }
}

/** The address of a place, so reload and the Back button work. */
function placeHash(p: Place): string {
  if (p.kind === 'testday') return 'library/testday'
  return 'subject' in p ? `${tabOf(p)}/${p.subject}` : tabOf(p)
}

function hashPlace(): Place {
  const [tab, sub] = window.location.hash.slice(1).split('/')
  const subject = SUBJECTS.find((s) => s.id === sub)?.id
  if (tab === 'learn' && subject) return { kind: 'path', subject }
  if (tab === 'practice' && subject) return { kind: 'practice-subject', subject }
  if (tab === 'library' && subject) return { kind: 'library-subject', subject, tab: 'lessons' }
  if (tab === 'library' && sub === 'testday') return { kind: 'testday' }
  return tabView(TABS.some((t) => t.tab === tab) ? (tab as Tab) : 'learn')
}

function subj(id: SubjectId): Subject {
  return subjectOf(id) ?? SUBJECTS[0]
}

export default function App() {
  const [screen, setScreen] = useState<Screen>({ kind: 'starting' })
  const [stats, setStats] = useState<Stats | null>(null)
  const [plan, setPlan] = useState<Plan | null>(null)

  useEffect(() => {
    api
      .me()
      .then((me) => setScreen({ kind: 'app', me, view: hashPlace() }))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 401) setScreen({ kind: 'profiles' })
        else setScreen({ kind: 'error', message: errorMessage(e) })
      })
  }, [])

  const me = screen.kind === 'app' ? screen.me : null
  const view = screen.kind === 'app' ? screen.view : null
  const here: Place | null = view ? (isSession(view) ? view.back : view) : null
  const onPlace = !!view && !isSession(view)
  const current = onPlace && here ? tabOf(here) : null
  const placeKey = onPlace && here ? placeHash(here) : null

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

  // Fresh numbers every time the learner lands on a page.
  const meId = me?.id
  useEffect(() => {
    if (meId !== undefined && placeKey) refresh()
  }, [meId, placeKey, refresh])

  // Back and Forward move between pages; Back from a session returns to its page.
  useEffect(() => {
    const onPop = () => setScreen((s) => (s.kind === 'app' ? { ...s, view: hashPlace() } : s))
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
      const hash = `#${isSession(next) ? 'go' : placeHash(next)}`
      // Leaving a session replaces its entry, so Back never lands on a dead one.
      if (view && isSession(view)) window.history.replaceState(null, '', hash)
      else if (window.location.hash !== hash) window.history.pushState(null, '', hash)
      setScreen({ kind: 'app', me, view: next })
      window.scrollTo({ top: 0 })
    }
  }
  const back: Place = here ?? { kind: 'learn' }
  const start = (scope: Scope, setLength = 10) => go({ kind: 'practice', scope, setLength, run: Date.now(), back })
  const openStage = (stageId: string) => go({ kind: 'stage', stageId, run: Date.now(), back })
  const startTest = (testKind: TestKind) => go({ kind: 'test', testKind, run: Date.now(), back })

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
        return go({ kind: 'library-subject', subject: 'math', tab: 'hacks' })
      case 'tactics':
        return go({ kind: 'testday' })
      case 'set_date':
        return go({ kind: 'me' })
      case 'journey':
        return go({ kind: 'path', subject: 'math' })
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
    return <StageRunner key={view.run} stageId={view.stageId} onExit={() => go(view.back)} onOpen={openStage} />
  if (view.kind === 'practice')
    return (
      <Practice
        key={view.run}
        scope={view.scope}
        setLength={view.setLength}
        onExit={() => go(view.back)}
        onAgain={() => start(view.scope, view.setLength)}
      />
    )
  if (view.kind === 'test') return <TestRunner key={view.run} kind={view.testKind} onExit={() => go(view.back)} />

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
          {view.kind === 'learn' && <Learn stats={stats} plan={plan} onSubject={(subject) => go({ kind: 'path', subject })} onOpen={openStage} onTask={runTask} />}
          {view.kind === 'path' && <SubjectPath key={view.subject} subject={subj(view.subject)} onOpen={openStage} onBack={() => go({ kind: 'learn' })} />}
          {view.kind === 'practice-menu' && (
            <PracticeMenu stats={stats} plan={plan} onStart={start} onSubject={(subject) => go({ kind: 'practice-subject', subject })} />
          )}
          {view.kind === 'practice-subject' && (
            <PracticeSubject key={view.subject} subject={subj(view.subject)} onBack={() => go({ kind: 'practice-menu' })} onStart={start} onTest={startTest} />
          )}
          {view.kind === 'library' && (
            <Library onSubject={(subject) => go({ kind: 'library-subject', subject, tab: 'lessons' })} onTestDay={() => go({ kind: 'testday' })} />
          )}
          {view.kind === 'library-subject' && (
            <LibrarySubject key={view.subject} subject={subj(view.subject)} initialTab={view.tab} onBack={() => go({ kind: 'library' })} onPractice={start} />
          )}
          {view.kind === 'testday' && <TestDay onBack={() => go({ kind: 'library' })} />}
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
