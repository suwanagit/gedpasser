import type { ReactNode } from 'react'
import type { Subject } from '../subjects'
import { Icon } from './Icon'
import { Mascot } from './Mascot'

/** One subject as a big tappable card: icon, name, a short line, progress. */
export function SubjectCard({
  subject,
  sub,
  value,
  onClick,
}: {
  subject: Subject
  /** One short line under the name. */
  sub?: string
  /** 0–1 progress; leave out to hide the bar. */
  value?: number
  onClick: () => void
}) {
  return (
    <button className={`subject-card subj-${subject.id}${subject.ready ? '' : ' soon'}`} onClick={onClick}>
      <span className="subject-icon">
        <Icon name={subject.icon} size={28} />
      </span>
      <span className="subject-text">
        <strong>{subject.name}</strong>
        {subject.ready ? (
          <>
            {value !== undefined && (
              <span className="bar">
                <span style={{ width: `${Math.round(value * 100)}%` }} />
              </span>
            )}
            {sub && <span className="muted small">{sub}</span>}
          </>
        ) : (
          <span className="pill">Soon</span>
        )}
      </span>
      <Icon name="right" size={20} />
    </button>
  )
}

/** The header of a page one layer down: back arrow, subject icon, name. */
export function SubHead({ subject, title, onBack, children }: { subject?: Subject; title?: string; onBack: () => void; children?: ReactNode }) {
  return (
    <header className={`sub-head no-print${subject ? ` subj-${subject.id}` : ''}`}>
      <button className="icon-btn" aria-label="Back" onClick={onBack}>
        <Icon name="back" />
      </button>
      {subject && (
        <span className="subject-icon">
          <Icon name={subject.icon} size={22} />
        </span>
      )}
      <h1>{title ?? subject?.name}</h1>
      {children}
    </header>
  )
}

/** Shown on a placeholder subject's pages. */
export function ComingSoon() {
  return (
    <div className="soon-note panel">
      <Mascot mood="think" size={56} />
      <span>
        <strong>Coming soon</strong>
        <span className="muted small">This is a preview of the layout.</span>
      </span>
    </div>
  )
}
