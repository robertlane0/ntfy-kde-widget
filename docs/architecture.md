# Architecture

## How it fits into Plasma

Plasma loads the applet as an ordinary QML `KPackage`:

```
~/.local/share/plasma/plasmoids/org.ntfy.widget/
├── metadata.json           Plasma/Applet, declarativeappletscript
└── contents/ui/
    ├── main.qml            PlasmoidItem: compact + full representation
    ├── PanelIcon.qml       the panel button
    ├── SubscriptionManager.qml
    ├── SubscriptionRow.qml
    ├── AddSubscriptionSheet.qml
    ├── ActionButton.qml
    ├── Field.qml
    ├── Icon.qml
    ├── Theme.qml           pragma Singleton, design tokens
    └── qmldir              lists the types above
```

Plasma never loads a library out of the applet package, so anything C++ has to reach
QML as a separate QML module:

```
/usr/lib/qt6/qml/org/ntfy/widget/
├── libntfywidget.so        QML extension plugin
└── qmldir                  module org.ntfy.widget / plugin ntfywidget
```

`main.qml` does `import org.ntfy.widget` and gets the `Bridge` singleton. Qt resolves
modules by URI in its import path, which is why this one is installed next to the
other Qt modules rather than under the user's data directory.

The shell owns the popup window; the applet only supplies `compactRepresentation` and
`fullRepresentation` and toggles `expanded`.

## Process layout

Everything runs inside `plasmashell`:

```
plasmashell
 └─ libntfywidget.so          QML extension plugin (GUI thread)
     └─ Bridge                QObject, marshals events onto the GUI thread
         └─ ntfy_core (Rust)  statics linked into the plugin
             ├─ ntfy-events   thread: drains the event queue, updates state
             └─ tokio runtime
                 └─ one task per subscription, each holding an SSE request
```

Rust emits JSON events from its worker threads. The C++ sink trampoline hands each
one to `QMetaObject::invokeMethod` with `Qt::QueuedConnection`, so QML and D-Bus only
ever see the GUI thread.

## The C ABI

`cpp/ntfy.h` mirrors `rust/src/ffi.rs`.

Rust → C++: a sink callback `(kind, json, len)`. `kind` is 1 for a full state snapshot,
2 for a message to notify about, 3 for a log line. The payload is always JSON, which
keeps the surface small and easy to extend.

C++ → Rust: `ntfy_command(op, a, b, c)` returns a malloc'd JSON result
(`{"ok":true}` or `{"ok":false,"error":"..."}`) that the caller frees with
`ntfy_string_free`.

| Op | a | b | c |
| --- | --- | --- | --- |
| add | server | topic | token |
| remove | id | | |
| set-enabled | id | `1`/`0` | |
| mark-read | id | | |
| mark-all-read | | | |
| open | url | | |

## The Rust core

| file | role |
| --- | --- |
| `json.rs` | JSON parser and writer; numbers keep their source text |
| `model.rs` | `Subscription`, `Notification`, `LinkState`, URL building |
| `store.rs` | atomic read/write of `subscriptions.json` |
| `client.rs` | one SSE stream with reconnect and backoff |
| `engine.rs` | task supervision, unread counters, event fan-out |
| `ffi.rs` | the C ABI |
| `util.rs` | config paths, percent-encoding, id generation |

### Streaming

Each subscription opens `GET <server>/<topic>/sse?since=<timestamp>` and parses the SSE
frames by hand. `since` is a unix timestamp, not a message id: ntfy ignores a value it
cannot read as a timestamp or duration and replays the whole topic instead.

The engine keeps a per-subscription cursor — the newest message's timestamp plus the
ids of the recent messages — and hands it to a task when it starts, so restarting a
task resumes rather than replays. Both halves are persisted, so a restart resumes too.
The ids matter because a timestamp cannot separate two messages published in the same
second, which is exactly the boundary a resume lands on.

With nothing read yet the stream asks for messages published since the subscription was
created, so a new subscription does not arrive with a day of history.

Backoff doubles from one second to sixty. Responses that will never succeed (401, 403,
404) back off to five minutes instead, since retrying hard only wastes the server's
time. `Retry-After` is honoured where the server sends it.

### Deciding what to notify about

A message raises a notification when it arrives after `createdAt` and its priority is
at least `minPriority`. Anything else only increments the unread counter. Both are
filtered in `engine.rs`, before the event leaves the core. Messages the cursor already
covers are dropped outright, so a replayed stream neither notifies nor counts twice.

## Lifetime

The core runs inside `plasmashell`, so its lifetime is tied to the applet's rather than
the process's:

- `main.qml` calls `Bridge.activate()` when it completes and `Bridge.deactivate()` when
  it is destroyed.
- The bridge counts live applet views and starts the core on the first, stops it on the
  last. `ntfy_stop()` cancels the SSE tasks and shuts the runtime down.
- `libntfywidget.so` cannot be unloaded — Qt keeps QML extension plugins for the life of
  the engine — so the sink is removed explicitly on the way out.

Without this the applet could be removed from the panel while the engine kept streaming
and posting notifications for the rest of the session.

Locks: the message handler holds `subs` and passes the stored cursor into
`note_message`, which locks only `cursors`. `std::sync::Mutex` is not reentrant, so
`note_message` must never reach for `subs` itself.

## Tests

`cargo test` covers the JSON round trip, the SSE frame accumulator, server
normalisation and id generation, and the cursor's replay suppression — including a
stored cursor seeding a fresh engine, which is the restart case. The Qt side has no
automated tests; the `ntfyprobe` helper covers package discovery.

## Notes and limits

- Plasma 6.11 moved `PathSvg` into `QtQuick` and `ShapePath` has no `visible`
  property, so `Icon.qml` draws one `Shape` per glyph with `PathSvg` sub-paths.
- A `Shape` with `layer.enabled` is composited *after* its plain siblings, so
  anything meant to draw over an icon has to live inside the `Shape`. That is why
  the mute icon's strikethrough is a `ShapePath` rather than a rotated rectangle.
- The mute icon is a bell outline with the slash knocked out, and the knockout is
  stroked in the row's own colours (`Icon.slashLayers`) so it blends with the row
  whether it is idle, hovered, or showing the delete confirmation.
- Notifications are posted over `org.freedesktop.Notifications` directly. Actions are
  label/key pairs; both entries share a label so the server renders a single button.
  Urgent messages stay up for 30s and the rest for 10s, rather than never expiring.
- The notification server is plasmashell itself, so its own `Notify` calls are loopback
  and invisible to `dbus-monitor`. Verifying delivery needs a screenshot, not a bus
  trace.
- Backdrop blur is not available to Qt Quick windows on Wayland, so the popover uses
  Plasma's own background and gets its macOS feel from the content: rounded cards,
  hairline separators, restrained type and an accent used sparingly.
- Every labelled action is a `PillButton`. A `Button` stretches its `contentItem`
  across the whole content area, so the icon and label live in a `Row` centred
  inside an `Item`; otherwise the label drifts into a corner.

## Development helpers

Both are behind `-DNTFY_BUILD_TOOLS=ON`.

- `ntfyprobe` reports how Plasma resolves the installed package.
- `ntfypreview <file.qml>` opens a QML file in a window, which is how the icons
  and buttons in `contents/ui` were checked without restarting the shell.
  `docs/preview.qml` renders every icon and button at once.