# GED Passer — Alaska GED Study App

Free, self-hosted web app that helps Alaskans study for the GED. Math first.
Full spec: `docs/spec.md` (read it when planning a phase; don't re-read it every turn).
Repo: https://github.com/suwanagit/gedpasser.git

## Environment
- Dev machine is Jon's Windows PC. Use the `rust-windows-dev` skill in
  `.claude/skills/`. The toolchain is installed and verified: run one version
  check (`rustc --version; cargo --version; node --version`), then build.
  Do not install, upgrade, or survey the toolchain.
- Shell is PowerShell 5.1: chain commands with `;`, not `&&`.
- Disk space is tight. Warn Jon before anything that downloads or builds a lot.

## Stack
- Backend: Rust (Axum, Tokio, sqlx, SQLite) in `server/`.
- Frontend: React + TypeScript + Vite + Motion + KaTeX in `web/`.
- One binary serves `/api` and the built frontend. Docker on vs00 in production.
- Question bank: TOML files in `content/` (skill map in `content/skills.toml`).
- Validator and answer checker in `tools/`: run `cargo content` from the
  repo root. Release build: `scripts\build.ps1` (see README "How to run").
- PowerShell 5.1 mangles double quotes in native-exe arguments: for commit
  messages containing `"`, write the message to a file and `git commit -F`.

## Rules
- Work one roadmap phase at a time (see `docs/spec.md`, Roadmap). Plan in
  Plan Mode and get Jon's approval before editing. Stop when that phase's
  checkboxes are met.
- The question bank lives in `content/`. Never edit questions in the database.
- Every new question starts as `status = "draft"` with a `check` expression,
  a worked explanation and a common mistake. Run `cargo content` before
  finishing.
- Review (Jon's decision, 2026-11-10: AI review replaces human sign-off; the
  app carries a disclaimer): each skill's batch is reviewed by a FRESH
  reviewer subagent that did not write it and solves each question blind
  before reading the key. It writes `content/reviews/<skill>.toml` (start
  from `cargo content-template <skill>`). Only `cargo content-approve <skill>`
  may set `status = "reviewed"`, and only for items with a current `pass`.
  Never set `reviewed` by hand. Any edit to a reviewed item needs a new review.
- Every question has exactly one skill from `content/skills.toml` and exactly
  one `principle` from `content/principles.toml` that belongs to that skill.
- Principles are the official GED Math indicator codes (Q.1.a … A.7.d), but
  names and lessons are our own words. Never copy text from the GED guide.
  Lessons follow the same draft -> reviewed rule as questions.
- Every principle has a short card in `content/cards/<skill>.toml` (format in
  `content/cards/WRITING.md`): as few words as possible, high-school reading
  level, one memory hack. Cards, figures and `content/strategy.toml` go
  through the same independent review and `cargo content-approve`.
- Figures must match the prompt and must not reveal the answer.
- Never use real GED test items or claim official GED status.
- Hash passwords with Argon2. Never log passwords or session tokens.
- Use metric units in all questions and UI, with US spelling (meters,
  liters, practice, center).
- Respect `prefers-reduced-motion` in every animation.
- Generate TypeScript types from Rust structs (ts-rs); don't hand-write API types.
- Keep `docs/decisions.md` updated when a design decision changes.
- Commit at meaningful milestones with clear messages; push to `origin main`.
