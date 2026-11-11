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
