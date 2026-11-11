# Alaska GED Study App — Project Spec

Nov 10, 2026 · Jon · Repo: https://github.com/suwanagit/gedpasser.git

## Overview

A free, self-hosted web app that helps Alaskans study for and pass the GED, starting with Math. It runs on a home server, is reached by browser on the LAN first and on the internet later, and finds each learner's weak skills so practice goes where it counts.

**Goals**

- Make daily practice enjoyable: quick sessions, clear feedback, light game elements.
- Pinpoint weak skills per learner and steer practice toward them.
- Keep every question accurate, with a worked explanation.
- Work well on a PC, with a lighter mobile mode for review on the go.

**Non-goals (for now)**

- No money, ads, or paid tiers. It is a free side project.
- No subjects beyond Math until Math is solid.
- No official GED content. Questions are original and aligned to the published test specifications. GED and GED Ready are trademarks of GED Testing Service, so the app avoids implying endorsement.
- No strong security model on the LAN. Login is for personalization; real auth arrives with public hosting.

## Decisions so far

| Area | Decision | Why |
| --- | --- | --- |
| Purpose | Free side project | No business model to support |
| First subject | Math | Most common sticking point; answers can be checked by code |
| Backend | Rust (Axum) + SQLite | Uses Jon's Rust setup; one binary, one data file |
| Frontend | React + TypeScript + Vite + Motion | Fun, animated, interactive UI; types shared with the Rust API |
| Main device | PC browser, with a mobile review mode | Long sessions on PC, quick review on a phone |
| Login | Local profiles with a password or PIN | Personalization and progress tracking |
| Questions | AI-drafted, validated, reviewed | Fast to grow, accuracy enforced by checks |
| Adding questions | Files in the repo, added by Claude Code | Easy for Claude Code, not a public editor |
| Hosting | vs00 in Docker, behind Caddy | Matches the existing home lab |
| Internet access | Cloudflare Tunnel on gatherd.org | Works with the ISP-locked router; no port forwarding |

**Working assumptions**

- Public access starts invite-only, behind Cloudflare Access. Opening it to anyone is a later decision.
- Mobile away from home goes through the public URL once the tunnel is live; Tailscale works before that.
- Some word problems are set in Alaska (fisheries, the PFD, village life, travel by ferry or bush plane).
- Passwords are hashed from day one, even on the LAN, so going public needs no rewrite.

## Architecture and stack

The app ships as one Rust executable that serves the API, the built React files, and the question bank, with all learner data in one SQLite file.

```
PC browser ──LAN──> Caddy ─────────┐
                                    ├──> Rust binary (Axum) ──reads/writes──> SQLite (ged.db)
Phone ──internet──> Cloudflare ──tunnel──> cloudflared ─┘        ^
                    (ged.gatherd.org, Access login)               │ loads at startup
                                                         content/ (skills + questions)
                                                                  ^ git
                                                         Claude Code on the LOQ
```

Home devices reach the app through Caddy; outside devices go through Cloudflare and the outbound tunnel, so the router never needs an open port.

| Layer | Choice | Notes |
| --- | --- | --- |
| Web server | Rust, Axum, Tokio | JSON API under `/api`, static files for everything else |
| Database | SQLite via sqlx | Migrations in the repo; one `ged.db` file to back up |
| Question bank | TOML files in `content/` | Loaded and validated at startup; synced into SQLite |
| Frontend | React, TypeScript, Vite | Built into `web/dist`, embedded in the binary |
| Animation | Motion | Respects the system reduce-motion setting |
| Math display | KaTeX | Fractions, exponents, and formulas render cleanly |
| Shared types | ts-rs (or similar) | Rust structs generate TypeScript types, so the API stays in sync |
| Dev machine | LOQ laptop, `C:\dev\gedpasser` | Built with Claude Code on Windows |
| Runtime | Docker on vs00 | Caddy on the LAN; `cloudflared` for the internet |

**Repo layout**

```
gedpasser/
  CLAUDE.md          instructions for Claude Code
  .claude/skills/    rust-windows-dev skill
  server/            Rust crate (Axum API, DB, content loader)
  web/               React + Vite app
  content/
    skills.toml      the skill map
    math/            question files, one per skill
  tools/             validator and answer checker
  docs/              this spec, decisions log
  Dockerfile
```

## Skill map

Every question is tagged with exactly one skill, and the skill list is the backbone for practice, progress, and weak-area detection. It is written first, in `content/skills.toml`, before any questions.

The GED Math test splits into quantitative problem solving and algebraic problem solving. The draft list below is a starting point; Phase 0 checks it against the GED Testing Service's published assessment targets for Mathematical Reasoning and adjusts names and boundaries to match.

| Skill id | Area | Skill |
| --- | --- | --- |
| q-number-ops | Quantitative | Whole numbers, fractions, decimals, order of operations |
| q-ratios-percent | Quantitative | Ratios, rates, proportions, percents |
| q-exponents-roots | Quantitative | Exponents, square roots, scientific notation |
| q-geometry-2d | Quantitative | Perimeter, area, circles, Pythagorean theorem |
| q-geometry-3d | Quantitative | Surface area and volume |
| q-data-stats | Quantitative | Mean, median, mode, range, reading charts and tables |
| q-probability | Quantitative | Simple and compound probability, counting |
| a-expressions | Algebraic | Writing, simplifying, and evaluating expressions |
| a-linear-equations | Algebraic | Solving linear equations and inequalities |
| a-systems | Algebraic | Systems of two linear equations |
| a-polynomials | Algebraic | Factoring and quadratics |
| a-graphs-lines | Algebraic | Slope, intercepts, graphing lines |
| a-functions | Algebraic | Function notation, reading and comparing functions |

Each skill entry also holds a short plain-language description and a difficulty range, so the app can explain what a weak area means.

## Question bank and accuracy pipeline

Questions live as files in the repo, and no question reaches learners until it passes automated checks and a separate review. Claude Code adds questions by writing files and running the validator; there is no in-app editor.

**Question format (example)**

```toml
[[question]]
id = "q-ratios-percent-0007"
skill = "q-ratios-percent"
difficulty = 2                 # 1 easy to 3 hard
type = "multiple_choice"       # also: fill_in, dropdown, drag_drop
status = "reviewed"            # draft | reviewed | retired
alaska = true
prompt = """
A Juneau fish processor packs 240 kg of salmon in 6 hours.
At the same rate, how many kilograms does it pack in 9 hours?
"""
choices = ["320", "360", "400", "480"]
answer = "360"
check = "240 / 6 * 9"          # expression the checker evaluates
explanation = """
Rate = 240 / 6 = 40 kg per hour. 40 x 9 = 360 kg.
"""
common_mistake = "Adding 3 hours' worth to 240 without finding the hourly rate first."
```

**Pipeline for every new batch**

1. Claude Code drafts questions for one skill with `status = "draft"`.
2. The validator checks the schema, ids, skill tags, duplicate or near-duplicate prompts, and that the answer is among the choices.
3. The answer checker evaluates each `check` expression and fails any question whose stored answer does not match.
4. A fresh Claude session that did not write the batch reviews it for wrong keys, ambiguity, more than one correct answer, and level fit, and writes a short review note.
5. Jon skims the batch and flips passing questions to `reviewed`. Only reviewed questions are served.
6. In the app, a "flag this question" button records problems; flagged questions drop out of rotation until fixed.

Question types should mirror the real test's formats over time, starting with multiple choice and fill-in, then dropdown and drag-and-drop.

## Features

The MVP is a practice loop with progress tracking; game elements, mobile review, and a timed test mode build on it.

**Profiles and login**

- Profile picker on the start screen, each profile with a password or PIN.
- Passwords hashed with Argon2; session cookie after login.
- No email, no recovery flow on the LAN. Jon can reset a profile from the server.

**Practice**

- Practice by skill, or a mixed set weighted toward weak skills.
- Instant feedback with the worked explanation and the common mistake.
- On-screen calculator and a formula sheet, as on the real test.

**Progress and weak areas**

- Mastery per skill: accuracy over recent answers, with newer answers counting more.
- A "focus next" card with the three weakest skills and a one-tap practice set for each.
- Skill map view coloured by mastery, so gaps show at a glance.

**Mobile review**

- A narrow-screen mode with short sets of 5 to 10 questions.
- Spaced review: missed questions come back sooner, mastered ones less often.
- Installable to the home screen as a web app.

**Timed test mode**

- A full-length practice test under the official time limit, plain and distraction-free.
- Score report by skill afterwards, feeding the progress model.

**Fun layer**

- XP, daily streaks, and small Motion animations for correct answers and level-ups.
- Milestone badges per skill.
- Everything respects reduce-motion, and test mode turns the fun layer off.

## Data model

Six SQLite tables cover the MVP; questions and skills are copied in from the content files at startup, and the files stay the source of truth.

| Table | Holds | Key columns |
| --- | --- | --- |
| users | Profiles | id, display_name, password_hash, created_at |
| skills | Skill map, synced from `skills.toml` | id, area, name, description |
| questions | Question bank, synced from `content/` | id, skill_id, difficulty, type, status, content_hash |
| attempts | Every answer given | id, user_id, question_id, correct, time_ms, mode, answered_at |
| review_queue | Spaced review schedule | user_id, question_id, due_at, interval_days |
| flags | Reported problems | id, user_id, question_id, note, resolved |

Mastery is computed from `attempts` rather than stored, so the formula can change without a migration. `content_hash` lets the app spot when a question's text changed after people answered it.

## Roadmap

Seven phases take the app from an empty repo to a public site; Phases 0 to 3 are the MVP, and each phase ends only when every box under it is ticked.

**Phase 0: Foundations**

- [x] Skill map checked against the official Math assessment targets and committed as `content/skills.toml`
- [x] Question schema written; validator and answer checker run from one command
- [x] Rust server serves a placeholder React page on the LAN from one binary
- [x] Repo pushed to GitHub with `CLAUDE.md`, this spec, and the decisions log

**Phase 1: Core practice loop**

- [x] Create a profile, log in, and log out
- [x] Pick a skill, answer questions, see the explanation after each
- [x] Every answer saved to `attempts`; data survives a server restart
- [x] KaTeX renders fractions and exponents correctly

**Phase 2: Question bank v1**

- [ ] At least 10 reviewed questions for every skill
- [ ] Validator and answer checker pass with zero errors
- [ ] Every batch has a review note from a separate session
- [ ] Flag button works and removes flagged questions from rotation

**Phase 3: Progress and weak areas (MVP complete)**

- [ ] Mastery shown per skill on a dashboard
- [ ] "Focus next" lists the three weakest skills and starts a set in one tap
- [ ] Mixed practice weights toward weak skills
- [ ] Jon uses it for a week and the weak areas it shows match his own sense of them

**Phase 4: Mobile and fun**

- [ ] Mobile review mode usable one-handed on a phone
- [ ] Spaced review queue working
- [ ] XP, streaks, and animations in place; reduce-motion respected

**Phase 5: Timed test mode**

- [ ] Full-length timed test with calculator and formula sheet
- [ ] Score report by skill feeds mastery

**Phase 6: Deploy and go public**

- [ ] Docker image runs on vs00 behind Caddy on the LAN
- [ ] Nightly backup of `ged.db`
- [ ] Cloudflare Tunnel publishes it on gatherd.org behind Cloudflare Access
- [ ] Login hardened: rate limiting, no cross-user data access

## Risks and open questions

The biggest risk is a wrong answer key reaching a learner; the pipeline exists to stop that, and the flag button catches what slips through.

| Risk | Effect | Mitigation |
| --- | --- | --- |
| Wrong or ambiguous AI questions | Learners lose trust or learn errors | Answer checker, separate review session, Jon's sign-off, flag button |
| Questions drift from the real test | Practice doesn't transfer | Skill map tied to official targets; mirror real question formats |
| Scope creep into more subjects | Math never gets finished | No new subject until Phase 3 is done |
| vs00 or home internet down | Site unavailable | Acceptable for a free project; backups protect data |
| Public login abused | Others' data exposed | Cloudflare Access first; hardened login before opening wider |
| Trademark wording | Implied endorsement | Say "GED test practice", never claim official status |

**Open questions**

- Who gets access once it's public: invited people only, or anyone?
- Will any learners be under 18? If so, what consent is needed before public use?
- Should Alaska Adult Education programs ever be told about it, as a free extra for their students?
- Which subject comes second: Reasoning Through Language Arts, Science, or Social Studies?
