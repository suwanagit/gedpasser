import type { IconName } from './components/Icon'

export type SubjectId = 'math' | 'language' | 'science' | 'social'

export type Subject = {
  id: SubjectId
  name: string
  icon: IconName
  /** False for a placeholder: the layout is there, the content is not. */
  ready: boolean
  /** Placeholder outline, shown until the subject has real content. */
  units: string[]
}

/** The four GED tests. Only Math has content so far. */
export const SUBJECTS: Subject[] = [
  { id: 'math', name: 'Math', icon: 'calc', ready: true, units: [] },
  {
    id: 'language',
    name: 'Language Arts',
    icon: 'pen',
    ready: false,
    units: ['Reading for meaning', 'Arguments and evidence', 'Grammar and usage', 'Essay writing'],
  },
  {
    id: 'science',
    name: 'Science',
    icon: 'flask',
    ready: false,
    units: ['Life science', 'Physical science', 'Earth and space', 'Science skills'],
  },
  {
    id: 'social',
    name: 'Social Studies',
    icon: 'globe',
    ready: false,
    units: ['Civics and government', 'US history', 'Economics', 'Geography and the world'],
  },
]

export function subjectOf(id: string): Subject | undefined {
  return SUBJECTS.find((s) => s.id === id)
}
