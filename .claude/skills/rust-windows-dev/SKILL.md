---
name: "rust-windows-dev"
description: "Use when starting, building or working on a Rust or Tauri project on Jon's Windows PC (C:\\dev). Holds the verified toolchain, workflow and pitfalls so no environment probing is needed."
---

# Rust / Tauri development on Jon's Windows PC

## Verified environment (do not re-probe)
- Machine: LOQ laptop, Windows 11 Pro 25H2 x64, i5-13420H, 32 GB RAM, RTX 3050 6 GB.
- Rust 1.99.1, default toolchain stable-x86_64-pc-windows-msvc (via rustup).
- Visual Studio 2022 Build Tools (C++ workload) installed; linking verified with a hello-world build.
- tauri-cli 2.12 (`cargo tauri`), Node 26, Git 2.56, ffmpeg 9.0.2 full build (gyan.dev, GPL) on PATH.
- WebView2 present (Windows 11).
- Projects live in `C:\dev\<project-name>`. Defender exclusions exist for `C:\dev`, `%USERPROFILE%\.cargo`, `%USERPROFILE%\.rustup`.
- Shell is Windows PowerShell 5.1: chain commands with `;`, not `&&`.
- Disk space is tight. Check free space before large builds and tell Jon if it is under about 20 GB.

## Where work happens
- Build natively on the PC through the connected-folder shell (device bash) scoped to `C:\dev`.
- If the device tools are missing, tell Jon to open the Claude desktop app and use "+" -> Add folder -> `C:\dev`. Do not fall back to cloud cross-compiling unless he asks.
- The device shell cannot delete files by default (`cargo clean` and `rm` fail). Request delete permission for `C:\dev` when cleanup is needed.

## Workflow
1. **One check, then build:** `rustc --version; cargo --version; cargo tauri --version`. If all three answer, proceed. Do not inspect the system further.
2. **Scaffold** in `C:\dev`:
   - CLI or library: `cargo new <name>`.
   - Desktop GUI: Tauri 2. Use `npm create tauri-app@latest <name> -- --template vanilla-ts --manager npm --yes` (or another template Jon picks), then `npm install`.
3. **Git:** `git init`, commit, add the remote for the GitHub repo Jon names, push. The first push may open a Git Credential Manager sign-in on Jon's screen; tell him to expect it.
4. **Iterate:** use `cargo check` or `cargo build` for fast verification. `cargo tauri dev` opens a window and keeps running, so start it in the background and tell Jon to look at the window rather than blocking on it.
5. **Release build:** `cargo tauri build` produces the .exe plus MSI/NSIS installers under `src-tauri\target\release\bundle\`.
6. Commit and push at meaningful milestones, with clear messages.

## Bundling ffmpeg (video/audio apps)
- Use a Tauri sidecar: list `binaries/ffmpeg` in `bundle.externalBin` and copy the exe to `src-tauri\binaries\ffmpeg-x86_64-pc-windows-msvc.exe`. Find the source with `(Get-Command ffmpeg).Source`.
- The gyan.dev full build is GPL. Fine for personal use. Flag the licence to Jon before any public distribution.

## Non-Latin text (Jon needs Thai)
- Bundle an OFL font with full Thai coverage (e.g. Noto Sans Thai, Sarabun, Kanit) rather than relying on system fonts.
- For ffmpeg/libass subtitle burn-in, pass the bundled font folder via `fontsdir` so exports match the preview.
- Test with real Thai text containing stacked vowels and tone marks.

## Don'ts
- Don't reinstall or upgrade toolchains unless Jon asks or the version check fails.
- Don't run broad environment surveys (CPU, RAM, PATH dumps). Everything needed is above.
- Don't write into folders outside `C:\dev`.