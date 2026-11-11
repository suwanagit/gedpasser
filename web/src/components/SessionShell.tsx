import type { ReactNode } from 'react'
import { Icon } from './Icon'

/**
 * The full-screen frame for anything the learner works through: a close
 * button and progress bar on top, one thing in the middle, one main button
 * fixed at the bottom.
 */
export function SessionShell({
  progress,
  onClose,
  onFlag,
  tone,
  footer,
  children,
}: {
  /** 0–1, or null to hide the bar. */
  progress: number | null
  onClose: () => void
  onFlag?: () => void
  /** Colors the footer after an answer. */
  tone?: 'right' | 'wrong'
  footer?: ReactNode
  children: ReactNode
}) {
  return (
    <div className="session">
      <header className="session-top">
        <button className="icon-btn" onClick={onClose} aria-label="Close">
          <Icon name="x" size={26} />
        </button>
        {progress !== null ? (
          <span className="bar" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(progress * 100)}>
            <span style={{ width: `${Math.max(progress, 0.04) * 100}%` }} />
          </span>
        ) : (
          <span className="session-gap" />
        )}
        {onFlag ? (
          <button className="icon-btn" onClick={onFlag} aria-label="Report a problem with this question">
            <Icon name="flag" size={20} />
          </button>
        ) : (
          <span className="session-pad" />
        )}
      </header>
      <div className="session-body">
        <div className="session-inner">{children}</div>
      </div>
      {footer && (
        <footer className={`session-foot ${tone ?? ''}`}>
          <div className="session-inner">{footer}</div>
        </footer>
      )}
    </div>
  )
}
