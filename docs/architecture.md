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

Each subscription opens `GET <server>/<topic>/sse?since=<cursor>` and parses the SSE
frames by hand. `cursor` is the last message id seen for that topic; it is held by the
engine and handed to a task when it starts, so restarting a task — which happens on
every add, remove or mute — resumes instead of replaying.

With no cursor the stream asks for messages published since the subscription was
created (`since=<unix timestamp>`), so a new subscription does not arrive with a day
of history.

Backoff doubles from one second to sixty. Responses that will never succeed (401, 403,
404) back off to five minutes instead, since retrying hard only wastes the server's
time. `Retry-After` is honoured where the server sends it.

### Deciding what to notify about

A message raises a notification when it arrives after `createdAt` and its priority is
at least `minPriority`. Anything else only increments the unread counter. Both are
filtered in `engine.rs`, before the event leaves the core.

## Tests

`cargo test` covers the JSON round trip, the SSE frame accumulator, server
normalisation and id generation. The Qt side has no automated tests; the
`ntfyprobe` helper covers package discovery.

## Notes and limits

- Plasma 6.11 moved `PathSvg` into `QtQuick` and `ShapePath` has no `visible`
  property, so `Icon.qml` uses one `Shape` per glyph.
- Notifications are posted over `org.freedesktop.Notifications` directly. Actions are
  label/key pairs; both entries share a label so the server renders a single button.
- Backdrop blur is not available to Qt Quick windows on Wayland, so the popover uses
  Plasma's own background and gets its macOS feel from the content: rounded cards,
  hairline separators, restrained type and an accent used sparingly.