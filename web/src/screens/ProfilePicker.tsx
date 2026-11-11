import { useEffect, useState, type FormEvent, type ReactNode } from 'react'
import { api, errorMessage } from '../api/client'
import type { Me } from '../api/types/Me'
import type { Profile } from '../api/types/Profile'
import { Icon } from '../components/Icon'
import { Mascot } from '../components/Mascot'

type Mode = { kind: 'pick' } | { kind: 'login'; profile: Profile } | { kind: 'create' }

/** Sign-in: pick a profile or make one. */
export function ProfilePicker({ onLoggedIn }: { onLoggedIn: (me: Me) => void }) {
  const [profiles, setProfiles] = useState<Profile[] | null>(null)
  const [mode, setMode] = useState<Mode>({ kind: 'pick' })
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api
      .profiles()
      .then((list) => {
        setProfiles(list)
        if (list.length === 0) setMode({ kind: 'create' })
      })
      .catch((e) => setError(errorMessage(e)))
  }, [])

  const back = () => setMode({ kind: 'pick' })
  return (
    <div className="signin center">
      <Mascot size={120} />
      {mode.kind === 'pick' && (
        <>
          <h1>GED Passer</h1>
          {error && <p className="error">{error}</p>}
          <div className="profile-grid">
            {profiles?.map((p) => (
              <button key={p.id} className="profile" onClick={() => setMode({ kind: 'login', profile: p })}>
                <span className="avatar" aria-hidden="true">
                  {p.display_name.slice(0, 1).toUpperCase()}
                </span>
                {p.display_name}
              </button>
            ))}
            <button className="profile new" onClick={() => setMode({ kind: 'create' })}>
              <span className="avatar" aria-hidden="true">
                <Icon name="plus" />
              </span>
              New
            </button>
          </div>
        </>
      )}
      {mode.kind === 'login' && <LoginForm profile={mode.profile} onBack={back} onLoggedIn={onLoggedIn} />}
      {mode.kind === 'create' && <CreateForm canGoBack={(profiles?.length ?? 0) > 0} onBack={back} onLoggedIn={onLoggedIn} />}
      <p className="fine">Free practice, written and checked by AI. Not affiliated with GED Testing Service.</p>
    </div>
  )
}

function Form({ title, error, onSubmit, children }: { title: string; error: string | null; onSubmit: () => Promise<void>; children: ReactNode }) {
  return (
    <form
      className="signin-form"
      onSubmit={(e: FormEvent) => {
        e.preventDefault()
        void onSubmit()
      }}
    >
      <h1>{title}</h1>
      {children}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </form>
  )
}

function LoginForm({ profile, onBack, onLoggedIn }: { profile: Profile; onBack: () => void; onLoggedIn: (me: Me) => void }) {
  const [secret, setSecret] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function submit() {
    setBusy(true)
    setError(null)
    try {
      onLoggedIn(await api.login({ user_id: profile.id, secret }))
    } catch (err) {
      setError(errorMessage(err))
      setSecret('')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Form title={`Hi, ${profile.display_name}!`} error={error} onSubmit={submit}>
      <input
        type="password"
        autoFocus
        autoComplete="current-password"
        aria-label="Password or PIN"
        placeholder="Password or PIN"
        value={secret}
        onChange={(e) => setSecret(e.target.value)}
      />
      <button type="submit" className="primary wide" disabled={busy || secret.length === 0}>
        Log in
      </button>
      <button type="button" className="link" onClick={onBack}>
        Back
      </button>
    </Form>
  )
}

function CreateForm({ canGoBack, onBack, onLoggedIn }: { canGoBack: boolean; onBack: () => void; onLoggedIn: (me: Me) => void }) {
  const [name, setName] = useState('')
  const [secret, setSecret] = useState('')
  const [confirm, setConfirm] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function submit() {
    if (secret !== confirm) {
      setError('Those don’t match.')
      return
    }
    setBusy(true)
    setError(null)
    try {
      onLoggedIn(await api.createProfile({ display_name: name.trim(), secret }))
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Form title="Let’s get started" error={error} onSubmit={submit}>
      <input autoFocus maxLength={30} autoComplete="nickname" aria-label="Your name" placeholder="Your name" value={name} onChange={(e) => setName(e.target.value)} />
      <input
        type="password"
        autoComplete="new-password"
        aria-label="Password or PIN, at least 4 characters"
        placeholder="Password or PIN (4+)"
        value={secret}
        onChange={(e) => setSecret(e.target.value)}
      />
      <input type="password" autoComplete="new-password" aria-label="Type it again" placeholder="Type it again" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
      <button type="submit" className="primary wide" disabled={busy || !name.trim() || secret.length < 4}>
        Start
      </button>
      {canGoBack && (
        <button type="button" className="link" onClick={onBack}>
          Back
        </button>
      )}
    </Form>
  )
}
