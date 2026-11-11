import { useEffect, useState, type FormEvent } from 'react'
import { api, errorMessage } from '../api/client'
import type { Me } from '../api/types/Me'
import type { Profile } from '../api/types/Profile'

type Mode = { kind: 'pick' } | { kind: 'login'; profile: Profile } | { kind: 'create' }

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

  if (mode.kind === 'login') {
    return (
      <LoginForm
        profile={mode.profile}
        onBack={() => setMode({ kind: 'pick' })}
        onLoggedIn={onLoggedIn}
      />
    )
  }
  if (mode.kind === 'create') {
    return (
      <CreateForm
        canGoBack={(profiles?.length ?? 0) > 0}
        onBack={() => setMode({ kind: 'pick' })}
        onLoggedIn={onLoggedIn}
      />
    )
  }

  return (
    <section className="card">
      <h2>Who is practicing?</h2>
      {error && <p className="error">{error}</p>}
      {profiles === null && !error && <p className="muted">Loading…</p>}
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
            +
          </span>
          New profile
        </button>
      </div>
    </section>
  )
}

function LoginForm({
  profile,
  onBack,
  onLoggedIn,
}: {
  profile: Profile
  onBack: () => void
  onLoggedIn: (me: Me) => void
}) {
  const [secret, setSecret] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function submit(e: FormEvent) {
    e.preventDefault()
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
    <form className="card form" onSubmit={submit}>
      <h2>Hi, {profile.display_name}</h2>
      <label>
        Password or PIN
        <input
          type="password"
          autoFocus
          autoComplete="current-password"
          value={secret}
          onChange={(e) => setSecret(e.target.value)}
        />
      </label>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="actions">
        <button type="button" className="secondary" onClick={onBack}>
          Back
        </button>
        <button type="submit" className="primary" disabled={busy || secret.length === 0}>
          {busy ? 'Logging in…' : 'Log in'}
        </button>
      </div>
    </form>
  )
}

function CreateForm({
  canGoBack,
  onBack,
  onLoggedIn,
}: {
  canGoBack: boolean
  onBack: () => void
  onLoggedIn: (me: Me) => void
}) {
  const [name, setName] = useState('')
  const [secret, setSecret] = useState('')
  const [confirm, setConfirm] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function submit(e: FormEvent) {
    e.preventDefault()
    if (secret !== confirm) {
      setError('The two entries don’t match.')
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
    <form className="card form" onSubmit={submit}>
      <h2>Create a profile</h2>
      <label>
        Your name
        <input autoFocus maxLength={30} autoComplete="nickname" value={name} onChange={(e) => setName(e.target.value)} />
      </label>
      <label>
        <span>
          Password or PIN <span className="muted">(at least 4 characters)</span>
        </span>
        <input type="password" autoComplete="new-password" value={secret} onChange={(e) => setSecret(e.target.value)} />
      </label>
      <label>
        Type it again
        <input type="password" autoComplete="new-password" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
      </label>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="actions">
        {canGoBack && (
          <button type="button" className="secondary" onClick={onBack}>
            Back
          </button>
        )}
        <button type="submit" className="primary" disabled={busy || !name.trim() || secret.length < 4}>
          {busy ? 'Creating…' : 'Create and start'}
        </button>
      </div>
    </form>
  )
}
