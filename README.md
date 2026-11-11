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

Settings (environment variables): `GED_ADDR` (default `0.0.0.0:8080`),
`GED_CONTENT` (overrides the content folder search).
