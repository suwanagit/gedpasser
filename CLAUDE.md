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

## Rules
- Work one roadmap phase at a time (see `docs/spec.md`, Roadmap). Plan in
  Plan Mode and get Jon's approval before editing. Stop when that phase's
  checkboxes are met.
- The question bank lives in `content/`. Never edit questions in the database.
- Every new question starts as `status = "draft"` with a `check` expression
  and a worked explanation. Run the validator and answer checker before
  finishing. Never mark a question `reviewed` yourself.
- Every question has exactly one skill from `content/skills.toml`.
- Never use real GED test items or claim official GED status.
- Hash passwords with Argon2. Never log passwords or session tokens.
- Use metric units in all questions and UI.
- Respect `prefers-reduced-motion` in every animation.
- Generate TypeScript types from Rust structs (ts-rs); don't hand-write API types.
- Keep `docs/decisions.md` updated when a design decision changes.
- Commit at meaningful milestones with clear messages; push to `origin main`.
