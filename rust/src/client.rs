//! Resilient Server-Sent Events client for a single ntfy subscription.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};

use crate::json::{self, Json};
use crate::model::{LinkState, Notification};

/// Immutable view of a subscription handed to a stream task.
#[derive(Debug, Clone)]
pub struct Spec {
    pub id: String,
    pub server: String,
    pub topic: String,
    pub token: Option<String>,
    pub min_priority: u8,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub enum Event {
    Link { id: String, state: LinkState, detail: String },
    Message { id: String, cursor: String, notification: Notification, time: i64 },
}

/// Why a stream attempt ended.
struct Failure {
    state: LinkState,
    detail: String,
    /// Terminal errors should not hammer the server.
    fatal: bool,
}

impl Failure {
    fn retry(state: LinkState, detail: impl Into<String>) -> Self {
        Failure { state, detail: detail.into(), fatal: false }
    }

    fn fatal(state: LinkState, detail: impl Into<String>) -> Self {
        Failure { state, detail: detail.into(), fatal: true }
    }
}

/// Longest gap between reconnect attempts for a transient failure.
const MAX_RETRY: Duration = Duration::from_secs(60);

/// Gap after an unrecoverable response (auth failure, missing topic).
const FATAL_RETRY: Duration = Duration::from_secs(300);

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(90))
        .user_agent("ntfy-kde-widget/0.1")
        .build()
        .unwrap_or_default()
}

/// Stream one subscription forever, reporting every state change and message.
///
/// `resume_from` is the last message id seen, so restarting the task continues
/// the stream instead of replaying the backlog.
pub async fn run(
    http: reqwest::Client,
    spec: Spec,
    resume_from: Option<String>,
    tx: std::sync::mpsc::Sender<Event>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut cursor: Option<String> = resume_from;
    let mut backoff = Duration::from_secs(1);

    loop {
        if *shutdown.borrow() {
            return;
        }

        // Without a cursor, ask for messages published since the subscription
        // was created: nothing older is a message the user can act on.
        let since = cursor
            .clone()
            .unwrap_or_else(|| spec.created_at.to_string());

        let url = crate::model::Subscription {
            id: spec.id.clone(),
            server: spec.server.clone(),
            topic: spec.topic.clone(),
            token: spec.token.clone(),
            enabled: true,
            min_priority: spec.min_priority,
            unread: 0,
            state: LinkState::Connecting,
            detail: String::new(),
            created_at: spec.created_at,
        }
        .stream_url(&since);

        match connect(&http, &spec, &url, &tx, &mut cursor, &mut shutdown).await {
            // The stream opened at least once: the ladder starts over.
            Ok(true) => {
                backoff = Duration::from_secs(1);
                let failure = Failure::retry(LinkState::Retrying, "stream closed by server");
                report(&tx, &spec, &shutdown, failure);
                if sleep_with_shutdown(&mut backoff, Duration::from_secs(60), &mut shutdown).await {
                    return;
                }
            }
            // Failed before opening: keep escalating the delay.
            Ok(false) => {
                let failure = Failure::retry(LinkState::Retrying, "stream unavailable");
                report(&tx, &spec, &shutdown, failure);
                if sleep_with_shutdown(&mut backoff, Duration::from_secs(60), &mut shutdown).await {
                    return;
                }
            }
            Err(failure) => {
                let cap = if failure.fatal { FATAL_RETRY } else { MAX_RETRY };
                report(&tx, &spec, &shutdown, failure);
                if sleep_with_shutdown(&mut backoff, cap, &mut shutdown).await {
                    return;
                }
            }
        }
    }
}

fn report(
    tx: &std::sync::mpsc::Sender<Event>,
    spec: &Spec,
    shutdown: &tokio::sync::watch::Receiver<bool>,
    failure: Failure,
) {
    if *shutdown.borrow() {
        return;
    }
    let _ = tx.send(Event::Link {
        id: spec.id.clone(),
        state: failure.state,
        detail: failure.detail,
    });
}

/// Returns `Ok(opened)` when the stream ended on its own, `Err` with the reason otherwise.
async fn connect(
    http: &reqwest::Client,
    spec: &Spec,
    url: &str,
    tx: &std::sync::mpsc::Sender<Event>,
    cursor: &mut Option<String>,
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<bool, Failure> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("ntfy-kde-widget/0.1"));
    if let Some(token) = spec.token.as_deref().filter(|t| !t.is_empty()) {
        let Ok(value) = HeaderValue::from_str(&format!("Bearer {token}")) else {
            return Err(Failure::fatal(LinkState::Failed, "invalid access token"));
        };
        headers.insert(AUTHORIZATION, value);
    }

    let response = http
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| Failure::retry(LinkState::Retrying, short_error(&e)))?;

    let status = response.status();
    if !status.is_success() {
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let state = if status.as_u16() == 404 {
            LinkState::Failed
        } else if status.as_u16() == 401 || status.as_u16() == 403 {
            LinkState::Failed
        } else if status.as_u16() >= 500 {
            LinkState::Retrying
        } else {
            LinkState::Failed
        };
        let mut detail = format!("HTTP {}", status.as_u16());
        if let Some(secs) = retry_after {
            detail = format!("{detail} · retry in {secs}s");
        }
        return Err(Failure { state, detail, fatal: state == LinkState::Failed });
    }

    let _ = tx.send(Event::Link {
        id: spec.id.clone(),
        state: LinkState::Live,
        detail: "streaming".into(),
    });

    let mut response = response;
    let mut buffer: Vec<u8> = Vec::new();
    let mut frame = Frame::default();

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    return Ok(true);
                }
            }
            chunk = response.chunk() => {
                let chunk = match chunk {
                    Ok(Some(bytes)) => bytes,
                    Ok(None) => break,
                    Err(e) => {
                        // A dropped stream is normal; keep the cursor and resume.
                        return Err(Failure::retry(LinkState::Retrying, short_error(&e)));
                    }
                };
                buffer.extend_from_slice(&chunk);

                while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
                    let line: Vec<u8> = buffer.drain(..=pos).collect();
                    let line = String::from_utf8_lossy(&line[..line.len() - 1]);
                    let line = line.strip_suffix('\r').unwrap_or(&line).to_string();
                    if frame.push(&line) {
                        if let Some(event) = frame.build() {
                            dispatch(&event, spec, cursor, tx);
                        }
                        frame = Frame::default();
                    }
                }
            }
        }
    }

    Ok(true)
}

/// Accumulates one SSE frame across lines.
#[derive(Default)]
struct Frame {
    event: String,
    data: String,
    id: String,
}

impl Frame {
    /// Returns true when the frame is terminated (blank line).
    fn push(&mut self, line: &str) -> bool {
        if line.is_empty() {
            return true;
        }
        if line.starts_with(':') {
            return false;
        }
        let (name, value) = match line.split_once(':') {
            Some((n, v)) => (n, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        match name {
            "event" => self.event = value.to_string(),
            "data" => {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(value);
            }
            "id" => self.id = value.to_string(),
            _ => {}
        }
        false
    }

    fn build(self) -> Option<SseEvent> {
        if self.data.trim().is_empty() {
            return None;
        }
        Some(SseEvent { event: self.event, id: self.id, data: self.data })
    }
}

struct SseEvent {
    event: String,
    id: String,
    data: String,
}

fn dispatch(
    event: &SseEvent,
    spec: &Spec,
    cursor: &mut Option<String>,
    tx: &std::sync::mpsc::Sender<Event>,
) {
    let Ok(value) = json::parse(&event.data) else {
        return;
    };
    let kind = if !event.event.is_empty() {
        event.event.clone()
    } else {
        value.field_str("event").unwrap_or_default().to_string()
    };
    let message_id = value
        .field_str("id")
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| event.id.clone());

    if !message_id.is_empty() {
        *cursor = Some(message_id.clone());
    }

    match kind.as_str() {
        "message" => {
            let tags = value
                .get("tags")
                .and_then(Json::as_array)
                .map(|items| {
                    items.iter().filter_map(Json::as_str).map(str::to_string).collect()
                })
                .unwrap_or_default();
            let notification = Notification {
                subscription_id: spec.id.clone(),
                topic: value
                    .field_str("topic")
                    .unwrap_or(&spec.topic)
                    .to_string(),
                title: value.field_str("title").unwrap_or_default().to_string(),
                body: value.field_str("message").unwrap_or_default().to_string(),
                priority: value
                    .get("priority")
                    .and_then(Json::as_i64)
                    .unwrap_or(3)
                    .clamp(1, 5) as u8,
                tags,
                url: String::new(),
                received_at: value.get("time").and_then(Json::as_i64).unwrap_or_else(crate::util::now_secs),
            };
            let _ = tx.send(Event::Message {
                id: spec.id.clone(),
                cursor: message_id,
                notification,
                time: value.get("time").and_then(Json::as_i64).unwrap_or_else(crate::util::now_secs),
            });
        }
        _ => {}
    }
}

/// Sleep with exponential backoff. Returns true when shutdown was requested.
async fn sleep_with_shutdown(
    backoff: &mut Duration,
    cap: Duration,
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
) -> bool {
    let delay = *backoff;
    *backoff = (*backoff * 2).min(cap);
    // Jitter avoids synchronised reconnect storms across many subscriptions.
    let jitter = crate::util::random_hex(2).len() as u64 % 500;
    let total = delay + Duration::from_millis(jitter);

    tokio::select! {
        _ = tokio::time::sleep(total) => false,
        _ = shutdown.changed() => *shutdown.borrow(),
    }
}

fn short_error(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        return "connection timed out".into();
    }
    if e.is_connect() {
        return "cannot reach server".into();
    }
    let text = e.to_string();
    // reqwest appends the whole source chain; the first clause is the useful part.
    text.split(':').next().unwrap_or("request failed").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_collects_multiline_data() {
        let mut f = Frame::default();
        assert!(!f.push("event: message"));
        assert!(!f.push("data: {\"a\":1,"));
        assert!(!f.push("data: \"b\":2}"));
        // A blank line closes the frame.
        assert!(f.push(""));
        let ev = f.build().unwrap();
        assert_eq!(ev.event, "message");
        assert_eq!(ev.data, "{\"a\":1,\n\"b\":2}");
        assert_eq!(json::parse(&ev.data).unwrap().get("b").unwrap().as_i64(), Some(2));
    }

    #[test]
    fn frame_ignores_comments_and_empty() {
        let mut f = Frame::default();
        assert!(!f.push(": keepalive"));
        assert!(f.push(""));
        assert!(f.build().is_none());
    }
}