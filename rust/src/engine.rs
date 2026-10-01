//! Supervises one streaming task per subscription and fans events out to the UI.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};

use crate::client::{self, Event, Spec};
use crate::model::{LinkState, Subscription};
use crate::{store, util};

/// Everything the UI needs to render a single event, as JSON text.
pub struct UiEvent {
    pub kind: u32,
    pub json: String,
}

const KIND_STATE: u32 = 1;
const KIND_NOTIFY: u32 = 2;
const KIND_LOG: u32 = 3;

/// How many message ids to remember per subscription.
const RECENT_IDS: usize = 128;

/// How far one subscription has been read.
///
/// `since` on the wire is a unix timestamp, which cannot separate two messages
/// published in the same second, so `recent` also remembers the ids already
/// covered. Together they let a reconnect resume without renotifying.
struct Cursor {
    time: i64,
    recent: VecDeque<String>,
}

impl Cursor {
    fn new(time: i64, recent: Vec<String>) -> Self {
        Cursor {
            time,
            recent: recent.into(),
        }
    }

    fn has(&self, message_id: &str) -> bool {
        self.recent.iter().any(|id| id == message_id)
    }

    fn remember(&mut self, message_id: &str) {
        self.recent.push_back(message_id.to_string());
        while self.recent.len() > RECENT_IDS {
            self.recent.pop_front();
        }
    }
}

/// A listener the core pushes events to. Must not block.
type Sink = Box<dyn Fn(UiEvent) + Send + Sync>;

pub struct Engine {
    subs: Mutex<Vec<Subscription>>,
    cursors: Mutex<Vec<(String, Cursor)>>,
    sinks: Mutex<Vec<Sink>>,
    runtime: Mutex<Option<tokio::runtime::Runtime>>,
    shutdown: Mutex<Option<tokio::sync::watch::Sender<bool>>>,
    tx: Mutex<Option<mpsc::Sender<Event>>>,
    running: AtomicBool,
}

static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();

pub fn engine() -> Arc<Engine> {
    ENGINE
        .get_or_init(|| {
            Arc::new(Engine {
                subs: Mutex::new(Vec::new()),
                cursors: Mutex::new(Vec::new()),
                sinks: Mutex::new(Vec::new()),
                runtime: Mutex::new(None),
                shutdown: Mutex::new(None),
                tx: Mutex::new(None),
                running: AtomicBool::new(false),
            })
        })
        .clone()
}

impl Engine {
    /// The engine is kept alive by a `OnceLock`; clone that Arc for thread work.
    fn shared(&self) -> Arc<Engine> {
        ENGINE.get().expect("engine is initialised").clone()
    }

    pub fn add_sink(&self, sink: Sink) -> usize {
        let mut sinks = self.sinks.lock().unwrap();
        sinks.push(sink);
        sinks.len() - 1
    }

    pub fn remove_sink(&self, index: usize) {
        let mut sinks = self.sinks.lock().unwrap();
        if index < sinks.len() {
            drop(sinks.remove(index));
        }
    }

    fn emit(&self, kind: u32, json: String) {
        let sinks = self.sinks.lock().unwrap();
        for sink in sinks.iter() {
            sink(UiEvent {
                kind,
                json: json.clone(),
            });
        }
    }

    fn emit_state(&self) {
        let subs = self.snapshot();
        let unread: u32 = subs.iter().map(|s| s.unread).sum();
        let doc = crate::json::Obj::new()
            .set(
                "subscriptions",
                crate::json::Json::Arr(subs.iter().map(|s| s.to_json()).collect()),
            )
            .set("unread", crate::json::Json::int(unread as i64))
            .build();
        self.emit(KIND_STATE, doc.to_text());
    }

    fn log(&self, level: &str, text: &str) {
        let doc = crate::json::Obj::new()
            .set("level", crate::json::Json::str(level))
            .set("text", crate::json::Json::str(text))
            .build();
        self.emit(KIND_LOG, doc.to_text());
    }

    pub fn snapshot(&self) -> Vec<Subscription> {
        self.subs.lock().unwrap().clone()
    }

    // --- lifecycle -------------------------------------------------------

    pub fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let mut subs = store::load();
        for sub in subs.iter_mut() {
            if !sub.enabled {
                sub.state = LinkState::Muted;
                sub.detail = "muted".into();
            }
        }
        if let Err(e) = store::save(&subs) {
            self.log("warning", &e);
        }
        *self.subs.lock().unwrap() = subs;

        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                self.log("error", &format!("cannot start runtime: {e}"));
                self.running.store(false, Ordering::SeqCst);
                return;
            }
        };

        let (tx, rx) = mpsc::channel::<Event>();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        *self.shutdown.lock().unwrap() = Some(stop_tx);

        let me = self.shared();
        std::thread::Builder::new()
            .name("ntfy-events".into())
            .spawn(move || {
                while let Ok(event) = rx.recv() {
                    me.handle(event);
                }
            })
            .ok();

        let handle = runtime.handle().clone();
        *self.runtime.lock().unwrap() = Some(runtime);
        *self.tx.lock().unwrap() = Some(tx.clone());
        for spec in self.specs() {
            self.spawn_stream(&handle, spec, tx.clone(), stop_rx.clone());
        }

        self.log(
            "info",
            &format!("{} subscription(s) loaded", self.snapshot().len()),
        );
        self.emit_state();
    }

    pub fn stop(&self) {
        if !self.running.swap(false, Ordering::SeqCst) {
            return;
        }
        if let Some(stop) = self.shutdown.lock().unwrap().take() {
            let _ = stop.send(true);
        }
        if let Some(runtime) = self.runtime.lock().unwrap().take() {
            runtime.shutdown_timeout(std::time::Duration::from_millis(250));
        }
        *self.tx.lock().unwrap() = None;
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn specs(&self) -> Vec<Spec> {
        self.subs
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.enabled)
            .map(|s| Spec {
                id: s.id.clone(),
                server: s.server.clone(),
                topic: s.topic.clone(),
                token: s.token.clone(),
                min_priority: s.min_priority,
                created_at: s.created_at,
            })
            .collect()
    }

    fn spawn_stream(
        &self,
        handle: &tokio::runtime::Handle,
        spec: Spec,
        tx: mpsc::Sender<Event>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) {
        let http = client::client();
        // Continue from the newest message already handled, so restarting the
        // task for any reason does not replay recent messages.
        let resume_from = self.resume_point(&spec.id);
        handle.spawn(async move {
            client::run(http, spec, resume_from, tx, shutdown).await;
        });
    }

    fn restart_streams(&self) {
        let handle = {
            let runtime = self.runtime.lock().unwrap();
            runtime.as_ref().map(|r| r.handle().clone())
        };
        let Some(handle) = handle else {
            return;
        };
        if let Some(stop) = self.shutdown.lock().unwrap().as_ref() {
            let _ = stop.send(true);
        }
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        *self.shutdown.lock().unwrap() = Some(stop_tx);

        let tx = self.tx.lock().unwrap().clone();
        let Some(tx) = tx else {
            return;
        };
        for spec in self.specs() {
            self.spawn_stream(&handle, spec, tx.clone(), stop_rx.clone());
        }
    }

    /// Where a subscription's stream should pick up, and which ids it has already
    /// covered. The cursor and the ids are both kept in memory and in the store,
    /// so a reconnect or a restart resumes instead of replaying.
    fn resume_point(&self, id: &str) -> i64 {
        let in_session = self
            .cursors
            .lock()
            .unwrap()
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, cursor)| cursor.time);
        let stored = self
            .subs
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .cloned();
        // In-session cursor wins; otherwise the stored one; otherwise the
        // subscription's own creation time.
        in_session
            .or_else(|| stored.as_ref().map(Subscription::resume_from))
            .unwrap_or(0)
    }

    /// The resume point and covered ids held in the store. Read separately
    /// because the message handler already holds `subs` when it needs a seed.
    fn stored_cursor(&self, id: &str) -> Option<(i64, Vec<String>)> {
        self.subs
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .map(|s| (s.cursor, s.recent.clone()))
    }

    /// Advances the cursor for a message. Returns false when the message is one
    /// already accounted for, which is what a replayed stream produces.
    fn note_message(
        &self,
        id: &str,
        message_id: &str,
        time: i64,
        seed: Option<(i64, Vec<String>)>,
    ) -> bool {
        let mut cursors = self.cursors.lock().unwrap();
        if !cursors.iter().any(|(key, _)| key == id) {
            // Seed from the store on first use, so ids covered before a restart
            // are still suppressed after it. Nothing stored means this message
            // is the first one seen, so it counts.
            let Some((start, ids)) = seed else {
                cursors.push((id.to_string(), Cursor::new(time, remembered(message_id))));
                return true;
            };
            cursors.push((id.to_string(), Cursor::new(start, ids)));
        }
        let Some((_, cursor)) = cursors.iter_mut().find(|(key, _)| key == id) else {
            return false;
        };
        // Before the resume point, or one of the ids it already covered.
        if time < cursor.time || (!message_id.is_empty() && cursor.has(message_id)) {
            return false;
        }
        cursor.time = time;
        if !message_id.is_empty() {
            cursor.remember(message_id);
        }
        true
    }

    // --- event handling --------------------------------------------------

    fn handle(&self, event: Event) {
        match event {
            Event::Link { id, state, detail } => {
                let mut subs = self.subs.lock().unwrap();
                let Some(sub) = subs.iter_mut().find(|s| s.id == id) else {
                    return;
                };
                if sub.state == state && sub.detail == detail {
                    return;
                }
                sub.state = state;
                sub.detail = detail;
                drop(subs);
                self.emit_state();
            }
            Event::Message {
                id,
                message_id,
                mut notification,
                time,
            } => {
                // Read the stored cursor before taking `subs`: note_message
                // needs it and must not lock `subs` itself.
                let seed = self.stored_cursor(&id);
                let (is_new, url, label, snapshot) = {
                    let mut subs = self.subs.lock().unwrap();
                    let Some(index) = subs.iter().position(|s| s.id == id) else {
                        return;
                    };
                    // A reconnect can hand back messages already handled.
                    if !self.note_message(&id, &message_id, time, seed) {
                        return;
                    }

                    let sub = &mut subs[index];
                    sub.unread = sub.unread.saturating_add(1);
                    // Messages older than the subscription, or below its priority
                    // filter, only bump the unread counter.
                    let is_new =
                        time >= sub.created_at && notification.priority >= sub.min_priority;
                    // Persisted with the rest, so a restart resumes here rather
                    // than replaying everything since the subscription was added.
                    sub.cursor = time;
                    if !message_id.is_empty() {
                        sub.recent.push(message_id.clone());
                        while sub.recent.len() > RECENT_IDS {
                            sub.recent.remove(0);
                        }
                    }
                    (
                        is_new,
                        sub.web_url(),
                        format!("{} · {}", sub.topic, host_of(&sub.server)),
                        subs.clone(),
                    )
                };
                let _ = store::save(&snapshot);
                if notification.title.is_empty() {
                    notification.title = label;
                }
                notification.url = url;
                if is_new {
                    self.emit(KIND_NOTIFY, notification.to_json().to_text());
                }
                self.emit_state();
            }
        }
    }

    // --- commands --------------------------------------------------------

    pub fn add(
        &self,
        server: &str,
        topic: &str,
        token: &str,
        min_priority: u8,
    ) -> Result<(), String> {
        let server = util::normalise_server(server);
        let topic = topic.trim().to_string();
        if server.is_empty() {
            return Err("Enter a server, for example ntfy.sh".into());
        }
        if topic.is_empty() {
            return Err("Enter a topic name".into());
        }
        if !topic
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(
                "Topics may only contain letters, digits, dots, dashes and underscores".into(),
            );
        }

        let mut subs = self.subs.lock().unwrap();
        if let Some(existing) = subs
            .iter_mut()
            .find(|s| s.server.eq_ignore_ascii_case(&server) && s.topic == topic)
        {
            // Re-adding an existing topic re-enables it and updates its token.
            existing.token = Some(token.trim().to_string()).filter(|t| !t.is_empty());
            existing.enabled = true;
            existing.min_priority = min_priority.clamp(1, 5);
            existing.state = LinkState::Connecting;
            existing.detail = String::new();
        } else {
            subs.push(Subscription {
                id: util::new_id(),
                server,
                topic,
                token: Some(token.trim().to_string()).filter(|t| !t.is_empty()),
                enabled: true,
                min_priority: min_priority.clamp(1, 5),
                unread: 0,
                state: LinkState::Connecting,
                detail: String::new(),
                created_at: util::now_secs(),
                cursor: 0,
                recent: Vec::new(),
            });
        }
        let _ = store::save(&subs);
        drop(subs);

        self.restart_streams();
        self.emit_state();
        Ok(())
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        let mut subs = self.subs.lock().unwrap();
        let before = subs.len();
        subs.retain(|s| s.id != id);
        if subs.len() == before {
            return Err("That subscription is already gone".into());
        }
        self.cursors.lock().unwrap().retain(|(key, _)| key != id);
        let _ = store::save(&subs);
        drop(subs);

        self.restart_streams();
        self.emit_state();
        Ok(())
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        {
            let mut subs = self.subs.lock().unwrap();
            let sub = subs
                .iter_mut()
                .find(|s| s.id == id)
                .ok_or("Unknown subscription")?;
            sub.enabled = enabled;
            if enabled {
                sub.state = LinkState::Connecting;
                sub.detail.clear();
            } else {
                sub.state = LinkState::Muted;
                sub.detail = "muted".into();
            }
            let _ = store::save(&subs);
        }
        self.restart_streams();
        self.emit_state();
        Ok(())
    }

    pub fn mark_read(&self, id: &str) -> Result<(), String> {
        let mut subs = self.subs.lock().unwrap();
        let sub = subs
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("Unknown subscription")?;
        sub.unread = 0;
        drop(subs);
        self.emit_state();
        Ok(())
    }

    pub fn mark_all_read(&self) -> Result<(), String> {
        {
            let mut subs = self.subs.lock().unwrap();
            for sub in subs.iter_mut() {
                sub.unread = 0;
            }
        }
        self.emit_state();
        Ok(())
    }

    pub fn open(&self, url: &str) -> Result<(), String> {
        if url.is_empty() {
            return Ok(());
        }
        std::process::Command::new("xdg-open")
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Cannot open {url}: {e}"))
    }
}

fn remembered(message_id: &str) -> Vec<String> {
    let mut ids = Vec::new();
    if !message_id.is_empty() {
        ids.push(message_id.to_string());
    }
    ids
}

fn host_of(server: &str) -> String {
    let trimmed = server
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    trimmed.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare() -> Engine {
        Engine {
            subs: Mutex::new(Vec::new()),
            cursors: Mutex::new(Vec::new()),
            sinks: Mutex::new(Vec::new()),
            runtime: Mutex::new(None),
            shutdown: Mutex::new(None),
            tx: Mutex::new(None),
            running: AtomicBool::new(false),
        }
    }

    #[test]
    fn replayed_messages_are_ignored() {
        let engine = bare();
        assert!(engine.note_message("a", "m1", 100, None));
        // The same message arriving again, as a replayed stream produces.
        assert!(!engine.note_message("a", "m1", 100, None));
        // A different message in the same second is still new.
        assert!(engine.note_message("a", "m2", 100, None));
        // Anything behind the resume point is history, not news.
        assert!(!engine.note_message("a", "m3", 99, None));
        // A newer message moves the resume point on.
        assert!(engine.note_message("a", "m4", 101, None));
        assert!(!engine.note_message("a", "m2", 100, None));
    }

    #[test]
    fn cursors_are_per_subscription() {
        let engine = bare();
        assert!(engine.note_message("a", "m1", 100, None));
        assert!(engine.note_message("b", "m1", 100, None));
        assert!(engine.note_message("b", "m2", 100, None));
    }

    #[test]
    fn a_stored_cursor_suppresses_a_replayed_boundary_message() {
        let engine = bare();

        // The stream replays the message on the boundary second: already seen.
        assert!(!engine.note_message("a", "m2", 100, Some((100, vec!["m2".into()]))));
        // A genuinely new message in that same second still gets through.
        assert!(engine.note_message("a", "m3", 100, Some((100, vec!["m2".into()]))));
    }

    #[test]
    fn a_subscription_with_no_history_starts_at_its_creation_time() {
        // Nothing stored means nothing has been read, so there is no resume
        // point beyond the subscription's own creation time.
        let engine = bare();
        assert_eq!(engine.resume_point("a"), 0);
        // Its first message counts, and nothing is suppressed.
        assert!(engine.note_message("a", "m1", 100, None));
    }

    #[test]
    fn a_stored_resume_point_is_used_when_nothing_was_read_yet() {
        let mut engine = bare();
        engine.subs = Mutex::new(vec![Subscription {
            id: "a".into(),
            server: "https://ntfy.sh".into(),
            topic: "alerts".into(),
            token: None,
            enabled: true,
            min_priority: 3,
            unread: 0,
            state: LinkState::Live,
            detail: String::new(),
            created_at: 10,
            cursor: 100,
            recent: vec!["m2".into()],
        }]);
        assert_eq!(engine.resume_point("a"), 100);
    }
}
