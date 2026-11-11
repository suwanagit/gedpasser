# Decisions log

Newest first. One line per decision: date, decision, reason.

| Date | Decision | Reason |
| --- | --- | --- |
| 2026-11-10 | Public access later via Cloudflare Tunnel on gatherd.org, invite-only behind Cloudflare Access | ISP-locked router, no port forwarding; keeps login simple at first |
| 2026-11-10 | Host on vs00 in Docker behind Caddy, not Vercel | Long-running Rust server + SQLite file don't fit serverless |
| 2026-11-10 | Questions stored as TOML files in `content/`, synced into SQLite at startup | Easy for Claude Code to add; files stay the source of truth |
| 2026-11-10 | Rust (Axum) + SQLite backend, React + TS + Vite + Motion frontend, one binary | Uses Jon's Rust setup; simple to deploy and back up |
| 2026-11-10 | Math is the first and only subject until the MVP is done | Most common sticking point; answers can be checked by code |
| 2026-11-10 | Free side project | No business model to support |
