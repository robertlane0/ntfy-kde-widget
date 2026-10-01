# ntfy KDE Widget — Plan

A Plasma 6 applet that lives in the panel, subscribes to ntfy topics over SSE and
raises desktop notifications when messages arrive.

## Constraints

- Rust 2024, `#![forbid(unsafe_code)]`
- Only UI / network-stack crates (`reqwest`, `tokio`). Everything else is hand written.
- macOS-inspired UI.

## Architecture

```
plasmashell
 └── contents/code/libntfyapplet.so      KPluginFactory + QML module "NtfyWidget"
      ├── NtfyApplet   Plasma::Applet    applet instance
      ├── Bridge       QObject           Rust event sink, D-Bs notifier, URL opener
      └── libntfy_core.a                 static Rust core (linked in)
           └── tokio worker threads → one SSE stream per subscription
```

The shell (`CompactApplet.qml`) owns the popup window. The applet only supplies
`compactRepresentation` (panel button) and `fullRepresentation` (popup content),
and toggles `expanded` to open/close it.

### Rust core (`rust/`)

| file       | role                                                     |
| ---------- | -------------------------------------------------------- |
| `json.rs`  | minimal JSON parser + writer (no serde)                   |
| `model.rs` | `Subscription`, `Message`, `LinkState`                    |
| `store.rs` | JSON persistence under `$XDG_CONFIG_HOME/ntfy-kde-widget`  |
| `client.rs`| one resilient SSE subscription loop                       |
| `engine.rs`| supervisor: tasks, unread counters, event fan-out         |
| `ffi.rs`   | `extern "C"` surface consumed by the C++ bridge           |

### C ABI

Rust → C++: sinks registered per applet instance,
`void sink(u32 kind, u64 token, const u8 *json, usize len)` marshalled with a
queued Qt connection. Kinds: `1` state, `2` notify, `3` log.

C++ → Rust: `char *ntfy_command(u32 op, const char *a, const char *b, const char *c)`
returns a JSON result string. Ops: add, remove, set-enabled, mark-read,
mark-all-read, set-min-priority.

## UI (macOS-inspired)

- Panel: bell glyph with a macOS-style accent badge for the unread total.
- Popup: 380×520 popover.
  - Header: app title + live connection summary, hairline divider.
  - Body: sidebar-style rows — status dot, topic, host, unread pill, mute and
    delete buttons, hover highlight.
  - Footer: accent pill button "Add Subscription"; the add form floats above it
    in its own card.
  - Empty state: centred glyph, headline, subline, call to action.
- Palette adapts to the Plasma colour scheme and to light/dark.

## Milestones

1. [x] Environment survey, toolchain check
2. [ ] Skeleton applet (C++ + QML) loads in the panel
3. [ ] Rust core: JSON, store, engine, FFI
4. [ ] SSE client with reconnect
5. [ ] macOS UI
6. [ ] End-to-end verification with screenshots
7. [ ] README / docs