# GED Passer

A free, self-hosted study app for the GED, built for Alaskans. Starts with Math.

- Practice by skill, with worked explanations
- Tracks progress per skill and points you to your weakest areas
- Works in a PC browser, with a mobile review mode

Not affiliated with or endorsed by GED Testing Service. GED® is a registered
trademark of the American Council on Education.

See `docs/spec.md` for the full plan.

## How to run

From the repo root, in PowerShell:

```powershell
cargo content                       # validate questions and check every answer
cargo test                          # unit + fixture tests; regenerates TS API types
.\scripts\build.ps1                 # types -> web\dist -> target\release\gedpasser-server.exe
.\target\release\gedpasser-server.exe   # serves http://<this-pc>:8080 on the LAN
```

After logging in you land on **Learn**. The four tabs:

- **Learn**: an "up next" button, today's goal and tasks, and the four GED
  subjects. A subject opens its week path: seven days of two short sets
  (about 15 minutes a day), easiest first, ending in a final check.
- **Practice**: Quick 5, Weak spots, Review, the topics to focus on (each
  with study links), then the subjects. A subject opens its practice tests
  (10, 25 or 46 questions, timed, with a calculator and formula sheet) and
  its skills.
- **Library**: the subjects, each with Quick review cards by skill, study
  links, memory hacks and the formula sheet, plus test-day tactics
  (printable).
- **Me**: level, streak, test date, what to work on next (with study
  links), strengths by subject, sound, log out, About.

After any answer, "Show me how" opens the worked explanation, the common
mistake, and free pages to study that topic on your own.

Math and Science have content: Science has 120 questions on 25 original or
public-data passages, a 7-day week path and practice tests. Language Arts
and Social Studies are placeholders marked "Soon".

Set a test date in Me to get daily tasks that put the most common, weakest
principles first.

To try the multi-subject features before real content exists, build a test
bank with a made-up demo Science subject and point the server at it:

```powershell
cargo run -q -p ged-content -- demo $env:TEMP\ged-demo
$env:GED_CONTENT = "$env:TEMP\ged-demo"; $env:GED_SERVE_DRAFTS = 1; cargo run -p gedpasser-server
```

For frontend work, run `cargo run -p gedpasser-server` and `npm --prefix web run dev`
together; Vite forwards `/api` to the server on port 8080.

The exe can be double-clicked: it finds the repo's `content` folder by looking
in the current folder, then beside the exe and in its parent folders. Keep its
window open while you use the app; closing it stops the server.

Only `reviewed` questions are served. While questions are still drafts,
double-click `scripts\run-with-drafts.cmd` instead: it sets
`GED_SERVE_DRAFTS=1`, and the app shows a "Development mode" banner and a
"Draft" badge on each unreviewed question.

Learner data lives in `data\ged.db` (beside `content`). Back up that one file.

Settings (environment variables):

| Variable | Default | Meaning |
| --- | --- | --- |
| `GED_ADDR` | `0.0.0.0:8080` | Listen address |
| `GED_CONTENT` | nearest `content` folder | Question bank folder |
| `GED_DB` | `data\ged.db` beside `content` | SQLite database file |
| `GED_SERVE_DRAFTS` | off | `1` also serves draft questions (development only) |

## About the content

Questions and lessons are written and checked by AI and may contain mistakes.
Each skill's batch is reviewed by a separate AI reviewer that solves every
question blind before seeing the answer key, and every answer is also checked
by code. Learners can report a problem with **Flag this question**: the
question leaves practice for everyone until it is fixed.

Flags, from the repo root:

```powershell
.\target\release\gedpasser-server.exe flags              # list open flags
.\target\release\gedpasser-server.exe flags resolve 3    # dismiss flag #3
```

Editing a flagged question in `content\` clears its flag on the next start.

## Writing questions

Questions live in `content\math\<skill>.toml`. Each needs a `principle`, an
official indicator code from `content\principles.toml` that belongs to the
same skill. Each principle has a short `lesson`: paragraphs separated by
blank lines, with lines starting `- ` for bullets. Put math between `$...$`
(KaTeX), write a literal dollar sign as `\$`, and use TOML literal strings
(`'...'` or `'''...'''`) so backslashes need no escaping. For an answer that
isn't a plain number (like `$2x + 12$`), give `answer_value` and a `check`
that both evaluate the expression at the same test value. Run `cargo content`
before committing.

Review workflow for a skill (see `content\reviews\CHECKLIST.md`):

```powershell
cargo content-quiz <skill>       # questions without answers, for a blind solve
cargo content-template <skill>   # review-note skeleton with content hashes
# reviewer writes content\reviews\<skill>.toml
cargo content-approve <skill>    # marks items with a current pass as reviewed
```
