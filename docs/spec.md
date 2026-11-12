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
- No subjects beyond Math until Math is solid. The goal is all four GED tests (Phase 7); Math comes first.
- No official GED content. Questions are original and aligned to the published test specifications. GED and GED Ready are trademarks of GED Testing Service, so the app avoids implying endorsement.
- No strong security model on the LAN. Login is for personalization; real auth arrives with public hosting.
- No human educator sign-off. Questions and lessons are written by AI and checked by a separate AI reviewer and by code; the app says so plainly and lets learners flag mistakes.

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

**Principles.** Under the skills sit 62 principles, one for each official assessment indicator (Q.1.a to Q.8.b and A.1.a to A.7.d). They live in `content/principles.toml`. Each principle has the official code as its id, a plain-language name, a one-line summary and a short lesson with a worked example, all written in our own words. Every question names exactly one principle within its skill. Learners can open the lesson while answering (recorded as a hint), after answering, or from the Lessons page. Practicing one principle at a time comes in Phase 3. Lessons go through the same draft → reviewed process as questions.

## Question bank and accuracy pipeline

Questions live as files in the repo, and no question reaches learners until it passes automated checks and a separate review. Claude Code adds questions by writing files and running the validator; there is no in-app editor.

**Question format (example)**

```toml
[[question]]
id = "q-ratios-percent-0007"
skill = "q-ratios-percent"
principle = "Q.3.a"            # official indicator code, in content/principles.toml
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
4. A fresh reviewer subagent that did not write the batch solves each question blind, then checks the key, ambiguity, more than one correct answer, distractors, explanation and level fit. It also checks the skill's lessons. It writes a verdict (pass, fix or reject) and a note for each item in `content/reviews/<skill>.toml`, tied to the item's content hash.
5. `cargo content-approve <skill>` flips items with a current `pass` to `reviewed`. The validator rejects any `reviewed` item without a matching pass, so a hand-flip or an edit after review is caught. Only reviewed questions and lessons are served. (Jon decided on 2026-11-10 that AI review replaces human sign-off, with a disclaimer in the app.)
6. In the app, a "flag this question" button records problems. A flagged question drops out of rotation for everyone until it is fixed (editing it clears the flag) or the flag is dismissed with `gedpasser-server flags resolve`.

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
- Skill map view colored by mastery, so gaps show at a glance.

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
| principles | Principles (official indicators), synced from `principles.toml` | id, skill_id, name, status |
| questions | Question bank, synced from `content/` | id, skill_id, principle_id, difficulty, type, status, content_hash |
| attempts | Every answer given | id, user_id, question_id, principle_id, correct, hint_used, time_ms, mode, answered_at |
| sessions | Login sessions | token_hash, user_id, expires_at |
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

**Phase 1b: Principles and lessons**

- [x] All 62 principles (official indicators) have a drafted lesson in `content/principles.toml`
- [x] Every question links to a principle in its own skill, checked by the validator
- [x] Lessons open while answering, after answering, and from a Lessons page
- [x] Attempts record the principle and whether the lesson was opened first

**Phase 2: Question bank v1**

- [x] At least 10 reviewed questions for every skill (12 each, 156 total; "reviewed" = AI-reviewed, see pipeline)
- [x] Lessons reviewed for every principle used by a reviewed question (all 62)
- [x] Validator and answer checker pass with zero errors
- [x] Every batch has a review note from a separate session (`content/reviews/<skill>.toml`, one per skill)
- [x] Flag button works and removes flagged questions from rotation

**Phase 3: Progress and weak areas (MVP complete)**

- [x] Mastery shown per skill on a dashboard
- [x] "Focus next" lists the three weakest skills and starts a set in one tap
- [x] Mixed practice weights toward weak skills
- [x] Practice one principle at a time, with progress shown per principle
- [ ] Jon uses it for a week and the weak areas it shows match his own sense of them (needs Jon; if they don't match, tune the weights in `server/src/mastery.rs`)

**Phase 4: Mobile and fun**

- [x] Mobile review mode usable one-handed on a phone (bottom tab bar, Quick 5, Review; Jon to try on his phone)
- [x] Spaced review queue working (10 minutes after a miss, then 1/3/7/14/30 days)
- [x] XP, streaks, and animations in place; reduce-motion respected

**Phase 5: Timed test mode**

- [x] Full-length timed test with calculator and formula sheet (short 10 / long 25 / full 46 questions)
- [x] Score report by skill feeds mastery (test answers are attempts)

**Phase 5b: Journey and test-week plan** (added 2026-11-11 at Jon's request)

- [x] A guided journey: 24 stages + 6 checkpoints covering all 62 principles, starting with three easy questions and growing in size and difficulty
- [x] A short card per principle (one rule, ≤4 steps, one worked example, one memory hack; ≤90 words, enforced by the validator), reviewed like questions
- [x] Real equations and graphs: display math, and figures (coordinate planes, number lines, bar/dot/box plots, labeled shapes) on cards and questions
- [x] Self-paced exploration at every level: area → skill → principle → card → full lesson → practice
- [x] Test-week plan: test date, days left, priorities by yield × weakness, daily tasks (check-up, power set, review, full test 2–3 days out, tactics, rest)
- [x] Tips: test-day tactics, every memory hack, printable formula sheet

**Phase 5c: Redesign** (added 2026-11-11 at Jon's request: clean, playful, few words; frontend only)

- [x] Four tabs (Learn, Practice, Library, Me): bottom bar on phones, side rail on wide screens; Back and reload work
- [x] The path is the home screen; one tap starts the next stage; today's goal ring and tasks beside it
- [x] Full-screen sessions: one question per screen, Check fixed at the bottom, verdict in the footer, explanation behind a tap
- [x] Lesson cards reveal one part per tap; result screen with stars, XP, confetti and one hack
- [x] Own look: Alaska palette in light and dark, bundled rounded font, one SVG icon set, moose mascot, pressable buttons, optional sounds
- [x] Practice test kept plain; report with a score ring, a bar per skill, and missed questions that open on tap
- [x] Subject layer: each tab opens on the four GED subjects; Math opens its path, tests, skills and lessons one layer down; the other three are placeholders marked "Soon"
- [x] Jon checks the look on a phone and a PC, in light and dark mode (approved 2026-11-11)

**Phase 5d: Practice playground** (added 2026-11-12 at Jon's request: "not a schoolhouse, but a playground")

- [x] The path is a week: 7 days, two 5-question sets a day (about 15 minutes), a 10-question final check on day 7. Easy wins first (day 1 only the easiest questions), harder toward the end. Covers the high- and medium-yield principles; the rest stay in Practice and the tests
- [x] Test first: path sets start with a question; Quick review is one tap away; "Show me how" opens the worked explanation, the common mistake and study links
- [x] Study links: each principle has a search topic and free pages (OpenStax textbooks, Math is Fun), each checked to open, in "Show me how", the Library and the focus lists
- [x] What to focus on: Practice and Me list the weakest high-value topics, each with practice and study links
- [x] `docs/EXPLANATION_STYLE.md` sets how explanations are written
- [ ] Jon tries the week path

Full lessons (one idea each, with a lesson player) were built on branch `lessons` and parked: too much scope for a practice app.

**Phase 6: Deploy and go public**

- [ ] Docker image runs on vs00 behind Caddy on the LAN
- [ ] Nightly backup of `ged.db`
- [ ] Cloudflare Tunnel publishes it on gatherd.org behind Cloudflare Access
- [ ] Login hardened: rate limiting, no cross-user data access

**Phase 7: The other three GED tests** (Jon, 2026-11-11: prepare learners for all four tests)

Each sub-phase is planned and approved separately. Research notes with sources are in `docs/subjects/`.

- [x] 7a. Research and design (2026-11-11). The official assessment guides, learner study guides and educator handbook were read for Reasoning Through Language Arts, Science and Social Studies. Each test's structure, timing, weights, targets and question types are recorded in `docs/subjects/<subject>.md` with a source and page for every fact, and checked by an independent reviewer. Format decisions and the subject order are in `docs/subjects/item-formats.md`.
- [ ] 7b. Make the app subject-aware. Add a subject level above skills in content, schema, API and UI (the UI already shows four subjects; `web/src/subjects.ts` holds the placeholders). Math keeps working unchanged. New content and question formats, in this order:
  - a shared stimulus (text, table, figure, or a mix) that several questions point to;
  - a content-topic tag on each question, beside its skill and principle, so practice tests can match each test's content weights;
  - fill-in answers that accept a list of words or phrases;
  - drop-down questions inside a text;
  - tap-to-place questions (put in order, sort into groups);
  - hot spot questions are left for later; a labeled figure with multiple choice stands in.

  Status (2026-11-12, branch `phase-7b-subjects`): content tools, server and screens are built and tested against a made-up demo subject (`fixtures/demo-science`, `cargo run -p ged-content -- demo OUT`). Each subject has its own week path, practice tests sized and weighted from `content/subjects.toml`, and study links. Still to do before ticking: check the screens in a browser (passages, drop-down, order, sort, a science practice test), and Jon's look.
- [x] 7c. Content pipeline for subjects without a code check (built 2026-11-12; rules in `docs/CONTENT_RULES.md`) (Science keeps a `check` expression for its numeric questions):
  - Every factual claim cites a source.
  - Passages are original, or public domain (for example founding documents, which the real Social Studies and RLA tests lean on), never copyrighted text.
  - Two independent reviewer subagents per batch, both solving blind. Both must pass an item.
  - A third reviewer re-checks facts against the cited sources.
  - Readability is checked against the target grade level.
  - Distractors are analyzed for being "defensibly wrong"; drop-down and tap-to-place questions must have exactly one defensible answer.
  - Cards, journey stages, hacks and a test-week plan follow the Math pattern. Timed practice tests use each test's real time limit (Science 90 minutes, Social Studies 70, RLA 150 with a break and a 45-minute essay).
- [ ] 7d. Build the subjects end to end, each with its own journey and practice tests, in this order: Science, then Social Studies, then RLA.
  - Science batch 1 of 7 done (2026-11-12): Designing investigations (SP.2.a–e), 18 questions on 5 original passages, reviewed by two blind solvers and a fact-checker, approved. Science is now live with Day 1 of its week path. Jon approved the style (2026-11-12).
  - Science batch 2 of 7 done (2026-11-12): Reading science texts (SP.1.a, SP.1.b, SP.6.c), 21 questions on 5 passages, four review rounds, approved. Day 2 of the Science path added.
  - Science batch 3 of 7 done (2026-11-12): Claims and evidence (SP.3.a, SP.3.b, SP.4.a, SP.5.a), 19 questions on 5 passages (one built from NOAA carbon dioxide data), four review rounds, approved. Day 3 of the Science path added. Also found and fixed: answer choices were served in file order with the answer first; they are now shuffled, and a truly blind re-solve of batches 1–2 matched all 39 keys.
  - Science batch 4 of 7 done (2026-11-12): Predictions and models (SP.3.c, SP.7.a), 16 questions on 4 new passages (NASA planet data, hares and lynx, cooling tea, the seasons and the midnight sun), two review rounds, approved. Day 4 of the Science path added.
  - Science batch 5 of 7 done (2026-11-12): Graphs, tables and diagrams (SP.1.c, SP.6.a, SP.6.b), 16 questions on 3 passages with a table and line graphs, three review rounds, approved. Day 5 of the Science path added. Plane figures gained `free_scale` for data graphs; graphs checked in the browser.
  - Science batch 6 of 7 done (2026-11-12): Probability and sampling (SP.3.d, SP.8.b, SP.8.c), 15 questions on a Punnett square and a lake fish survey, three review rounds, approved. Day 6 of the Science path added.
- [ ] 7e. Decide how to give feedback on RLA's extended response (a checklist the learner self-scores against the three rubric traits, or AI scoring through the Claude API). That needs Jon's decision on cost and privacy.

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
- How should the RLA essay be scored: a self-scored checklist, or AI scoring? (Phase 7e.)
- GED Testing Service does not publish how many questions each test has. The practice tests use the real time limits and commonly reported counts; revisit if an official number turns up.
