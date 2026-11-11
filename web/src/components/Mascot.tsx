export type Mood = 'happy' | 'cheer' | 'think'

/** The app's moose. Drawn from simple shapes so it stays crisp at any size. */
export function Mascot({ mood = 'happy', size = 96 }: { mood?: Mood; size?: number }) {
  const fur = '#a8703f'
  const furDark = '#7d4f28'
  const antler = '#e2b877'
  const muzzle = '#d9a877'
  return (
    <svg className="mascot" width={size} height={size} viewBox="0 0 120 120" aria-hidden="true">
      {/* antlers */}
      <g fill={antler} stroke={furDark} strokeWidth="2.5" strokeLinejoin="round">
        <path d="M44 40c-9 1-18-1-25-7l-9 3c-3-6-3-13 0-19l7 7 1-13 8 9 4-14 7 12 6-9c2 9 3 20 5 29z" />
        <path transform="translate(120 0) scale(-1 1)" d="M44 40c-9 1-18-1-25-7l-9 3c-3-6-3-13 0-19l7 7 1-13 8 9 4-14 7 12 6-9c2 9 3 20 5 29z" />
      </g>
      {/* ears */}
      <ellipse cx="30" cy="50" rx="10" ry="6" fill={fur} stroke={furDark} strokeWidth="2.5" transform="rotate(-25 30 50)" />
      <ellipse cx="90" cy="50" rx="10" ry="6" fill={fur} stroke={furDark} strokeWidth="2.5" transform="rotate(25 90 50)" />
      {/* head */}
      <path d="M36 52c0-14 10-22 24-22s24 8 24 22v20c0 4-2 8-5 10H41c-3-2-5-6-5-10z" fill={fur} stroke={furDark} strokeWidth="2.5" />
      {/* muzzle */}
      <rect x="32" y="70" width="56" height="40" rx="20" fill={muzzle} stroke={furDark} strokeWidth="2.5" />
      <ellipse cx="50" cy="84" rx="3" ry="4" fill={furDark} />
      <ellipse cx="70" cy="84" rx="3" ry="4" fill={furDark} />
      {/* eyes */}
      {mood === 'cheer' ? (
        <g fill="none" stroke="#2a1a0c" strokeWidth="3.5" strokeLinecap="round">
          <path d="M43 58q5-7 10 0" />
          <path d="M67 58q5-7 10 0" />
        </g>
      ) : (
        <g>
          <circle cx="48" cy="56" r="7" fill="#fff" />
          <circle cx="72" cy="56" r="7" fill="#fff" />
          <circle cx={mood === 'think' ? 50 : 49} cy={mood === 'think' ? 53 : 57} r="3.5" fill="#2a1a0c" />
          <circle cx={mood === 'think' ? 74 : 71} cy={mood === 'think' ? 53 : 57} r="3.5" fill="#2a1a0c" />
        </g>
      )}
      {/* mouth */}
      {mood === 'cheer' && <path d="M50 94q10 12 20 0z" fill="#7a2f2f" stroke={furDark} strokeWidth="2.5" strokeLinejoin="round" />}
      {mood === 'happy' && <path d="M50 96q10 8 20 0" fill="none" stroke={furDark} strokeWidth="3" strokeLinecap="round" />}
      {mood === 'think' && <path d="M54 98h12" fill="none" stroke={furDark} strokeWidth="3" strokeLinecap="round" />}
    </svg>
  )
}
