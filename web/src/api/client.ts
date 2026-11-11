// Typed fetch helpers. All request/response types are generated from Rust
// (ts-rs) into ./types — never hand-write API types here.
import type { AnswerRequest } from './types/AnswerRequest'
import type { AnswerResult } from './types/AnswerResult'
import type { ApiErrorBody } from './types/ApiErrorBody'
import type { CreateProfile } from './types/CreateProfile'
import type { FlagRequest } from './types/FlagRequest'
import type { Lesson } from './types/Lesson'
import type { LoginRequest } from './types/LoginRequest'
import type { Me } from './types/Me'
import type { NextQuestion } from './types/NextQuestion'
import type { PracticeMode } from './types/PracticeMode'
import type { PrincipleSummary } from './types/PrincipleSummary'
import type { Progress } from './types/Progress'
import type { Profile } from './types/Profile'
import type { SkillSummary } from './types/SkillSummary'

/** What a practice session draws questions from. `name` is for display. */
export type Scope =
  | { mode: Extract<PracticeMode, 'skill'>; id: string; name: string }
  | { mode: Extract<PracticeMode, 'principle'>; id: string; name: string }
  | { mode: Extract<PracticeMode, 'mixed'> }

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

export const api = {
  me: () => request<Me>('GET', '/api/me'),
  profiles: () => request<Profile[]>('GET', '/api/profiles'),
  createProfile: (body: CreateProfile) => request<Me>('POST', '/api/profiles', body),
  login: (body: LoginRequest) => request<Me>('POST', '/api/login', body),
  logout: () => request<void>('POST', '/api/logout'),
  skills: () => request<SkillSummary[]>('GET', '/api/skills'),
  next: (scope: Scope, after?: string) => {
    const q = new URLSearchParams()
    if (scope.mode === 'skill') q.set('skill', scope.id)
    else if (scope.mode === 'principle') q.set('principle', scope.id)
    else q.set('mode', 'mixed')
    if (after) q.set('after', after)
    return request<NextQuestion>('GET', `/api/practice/next?${q.toString()}`)
  },
  progress: () => request<Progress>('GET', '/api/progress'),
  answer: (body: AnswerRequest) => request<AnswerResult>('POST', '/api/answer', body),
  principles: () => request<PrincipleSummary[]>('GET', '/api/principles'),
  lesson: (id: string) => request<Lesson>('GET', `/api/principles/${encodeURIComponent(id)}`),
  flag: (body: FlagRequest) => request<void>('POST', '/api/flags', body),
}

export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}
