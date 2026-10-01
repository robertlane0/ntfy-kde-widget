# ntfy KDE Widget — Plan

A Plasma 6 applet that lives in the panel, subscribes to ntfy topics over SSE and
raises desktop notifications when messages arrive.

## Constraints

- Rust 2024, no `unsafe` outside the C ABI shim
- Only UI / network-stack crates (`reqwest`, `tokio`). Everything else is hand written.
- macOS-inspired UI.

## Architecture

See [docs/architecture.md](docs/architecture.md) for the full picture. In short:

- The applet is a plain QML `KPackage`. Plasma never loads a library out of an applet
  package, so the C++ side ships as a normal QML extension module (`org.ntfy.widget`)
  installed into Qt's import directory. It exports one `Bridge` singleton.
- `Bridge` is a thin `QObject` wrapper over a Rust static library linked into the same
  `.so`. It forwards commands in and hops JSON events from Rust's worker threads onto
  the GUI thread.
- The Rust core runs a tokio task per subscription, each holding an SSE request, and
  keeps per-topic unread counters.

## Layout

```
rust/      engine, SSE client, JSON, storage, C ABI
cpp/       QML extension plugin and the Bridge object
packaging/ the KPackage: metadata.json and the QML
tools/     ntfyprobe, a package-resolution helper
docs/      architecture notes
```

## Milestones

1. [x] Environment survey, toolchain check
2. [x] Rust core: JSON, storage, engine, FFI, tests
3. [x] SSE client with reconnect and cursor resume
4. [x] Ship the C++ bridge as a QML module so the applet stays pure QML
5. [x] macOS UI: panel button, popover, rows, add sheet
6. [x] End-to-end verification with screenshots
7. [x] README and architecture notes

## Still open

- Per-topic priority editing is stored and honoured but not editable in the UI.
- No automated tests on the Qt side.
- Backdrop blur is not available on Wayland; the popover leans on Plasma's background.