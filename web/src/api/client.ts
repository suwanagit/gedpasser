// Typed fetch helpers. All request/response types are generated from Rust
// (ts-rs) into ./types — never hand-write API types here.
import type { AnswerRequest } from './types/AnswerRequest'
import type { AnswerResult } from './types/AnswerResult'
import type { ApiErrorBody } from './types/ApiErrorBody'
import type { CreateProfile } from './types/CreateProfile'
import type { FlagRequest } from './types/FlagRequest'
import type { JourneyRun } from './types/JourneyRun'
import type { JourneyView } from './types/JourneyView'
import type { Lesson } from './types/Lesson'
import type { LoginRequest } from './types/LoginRequest'
import type { Me } from './types/Me'
import type { NextQuestion } from './types/NextQuestion'
import type { Plan } from './types/Plan'
import type { PracticeMode } from './types/PracticeMode'
import type { PrincipleSummary } from './types/PrincipleSummary'
import type { Profile } from './types/Profile'
import type { Progress } from './types/Progress'
import type { SkillSummary } from './types/SkillSummary'
import type { StageResult } from './types/StageResult'
import type { Stats } from './types/Stats'
import type { StrategyView } from './types/StrategyView'
import type { SubjectView } from './types/SubjectView'
import type { TestKind } from './types/TestKind'
import type { TestPaper } from './types/TestPaper'
import type { TestReport } from './types/TestReport'
import type { TestSubmission } from './types/TestSubmission'
import type { TestSummary } from './types/TestSummary'

/**
 * What a practice session draws questions from. `name` is for display.
 * For `principle`, `id` may be a comma-separated list (a power set).
 */
export type Scope =
  | { mode: Extract<PracticeMode, 'skill'>; id: string; name: string }
  | { mode: Extract<PracticeMode, 'principle'>; id: string; name: string }
  | { mode: Extract<PracticeMode, 'mixed'>; subject?: string }
  | { mode: Extract<PracticeMode, 'review'> }

export class ApiError extends Error {
  status: number
  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

async function request<T>(method: 'GET' | 'POST', url: string, body?: unknown): Promise<T> {
  const res = await fetch(url, {
    method,
    credentials: 'same-origin',
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  if (!res.ok) {
    let message = `Something went wrong (HTTP ${res.status})`
    try {
      message = ((await res.json()) as ApiErrorBody).error
    } catch {
      // not JSON; keep the generic message
    }
    throw new ApiError(res.status, message)
  }
  if (res.status === 204) return undefined as T
  return (await res.json()) as T
}

/** Minutes east of UTC, for streaks and "today" (Alaska is about −540). */
const tz = () => -new Date().getTimezoneOffset()

export const api = {
  me: () => request<Me>('GET', '/api/me'),
  profiles: () => request<Profile[]>('GET', '/api/profiles'),
  createProfile: (body: CreateProfile) => request<Me>('POST', '/api/profiles', body),
  login: (body: LoginRequest) => request<Me>('POST', '/api/login', body),
  logout: () => request<void>('POST', '/api/logout'),
  subjects: () => request<SubjectView[]>('GET', '/api/subjects'),
  skills: (subject = 'math') => request<SkillSummary[]>('GET', `/api/skills?subject=${subject}`),
  next: (scope: Scope, after?: string) => {
    const q = new URLSearchParams()
    if (scope.mode === 'skill') q.set('skill', scope.id)
    else if (scope.mode === 'principle') q.set('principle', scope.id)
    else q.set('mode', scope.mode)
    if (scope.mode === 'mixed' && scope.subject) q.set('subject', scope.subject)
    if (after) q.set('after', after)
    return request<NextQuestion>('GET', `/api/practice/next?${q.toString()}`)
  },
  progress: (subject = 'math') => request<Progress>('GET', `/api/progress?subject=${subject}`),
  answer: (body: AnswerRequest) => request<AnswerResult>('POST', '/api/answer', body),
  principles: (subject = 'math') => request<PrincipleSummary[]>('GET', `/api/principles?subject=${subject}`),
  lesson: (id: string) => request<Lesson>('GET', `/api/principles/${encodeURIComponent(id)}`),
  flag: (body: FlagRequest) => request<void>('POST', '/api/flags', body),
  journey: (subject = 'math') => request<JourneyView>('GET', `/api/journey?subject=${subject}`),
  startStage: (id: string) => request<JourneyRun>('POST', `/api/journey/${encodeURIComponent(id)}/start`),
  finishStage: (runId: number) => request<StageResult>('POST', `/api/journey/runs/${runId}/finish`),
  stats: () => request<Stats>('GET', `/api/stats?tz=${tz()}`),
  plan: () => request<Plan>('GET', `/api/plan?tz=${tz()}`),
  setTestDate: (test_date: string | null) => request<void>('POST', '/api/me/test-date', { test_date }),
  strategy: (subject = 'math') => request<StrategyView>('GET', `/api/strategy?subject=${subject}`),
  newTest: (kind: TestKind, subject = 'math') => request<TestPaper>('POST', '/api/tests', { kind, subject }),
  submitTest: (id: number, body: TestSubmission) => request<TestReport>('POST', `/api/tests/${id}/submit`, body),
  tests: (subject = 'math') => request<TestSummary[]>('GET', `/api/tests?subject=${subject}`),
}

export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}
