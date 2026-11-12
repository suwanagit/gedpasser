import type { Priority } from '../api/types/Priority'
import type { StudyView } from '../api/types/StudyView'
import { Icon } from './Icon'

/** A web search for the topic words only; nothing about the learner. */
function searchUrl(words: string): string {
  return `https://duckduckgo.com/?q=${encodeURIComponent(words)}`
}

/**
 * Where to go study a topic on your own: free pages to read, and a web
 * search for the topic. Opens in a new tab.
 */
export function StudyLinks({ study, compact = false }: { study: StudyView; compact?: boolean }) {
  const links = compact ? study.links.slice(0, 1) : study.links
  return (
    <ul className={`study-links${compact ? ' compact' : ''}`}>
      {links.map((l) => (
        <li key={l.url}>
          <a href={l.url} target="_blank" rel="noopener noreferrer">
            <Icon name="book" size={18} />
            <span>{l.title}</span>
          </a>
        </li>
      ))}
      <li>
        <a href={searchUrl(study.search)} target="_blank" rel="noopener noreferrer">
          <Icon name="search" size={18} />
          <span>Search “{study.search}”</span>
        </a>
      </li>
    </ul>
  )
}

/** What to focus on: weakest high-value topics, each with practice and study links. */
export function FocusList({ items, onPractice }: { items: Priority[]; onPractice: (p: Priority) => void }) {
  return (
    <ul className="focus-list">
      {items.map((p) => (
        <li key={p.principle.id} className="focus-item">
          <div className="focus-head">
            <span className={`dot level-${p.level}`} />
            <strong>{p.principle.name}</strong>
            <button className="secondary small" onClick={() => onPractice(p)}>
              <Icon name="target" size={16} /> Practice
            </button>
          </div>
          {p.study && <StudyLinks study={p.study} compact />}
        </li>
      ))}
    </ul>
  )
}
