//! Core data types: subscriptions, received messages and connection state.

use crate::json::{Json, Obj};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    Connecting,
    Live,
    Retrying,
    Muted,
    Failed,
}

impl LinkState {
    pub fn as_str(self) -> &'static str {
        match self {
            LinkState::Connecting => "connecting",
            LinkState::Live => "live",
            LinkState::Retrying => "retrying",
            LinkState::Muted => "muted",
            LinkState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub server: String,
    pub topic: String,
    pub token: Option<String>,
    pub enabled: bool,
    pub min_priority: u8,
    pub unread: u32,
    pub state: LinkState,
    pub detail: String,
    pub created_at: i64,
}

impl Subscription {
    /// Stream url with a `since` cursor so reconnects resume instead of replaying.
    pub fn stream_url(&self, since: &str) -> String {
        format!(
            "{}/{}/sse?since={}",
            self.server.trim_end_matches('/'),
            crate::util::encode_segment(&self.topic),
            crate::util::encode_query(since),
        )
    }

    pub fn web_url(&self) -> String {
        format!(
            "{}/{}",
            self.server.trim_end_matches('/'),
            crate::util::encode_segment(&self.topic)
        )
    }

    pub fn to_json(&self) -> Json {
        Obj::new()
            .set("id", Json::str(&self.id))
            .set("server", Json::str(&self.server))
            .set("topic", Json::str(&self.topic))
            .set("enabled", Json::Bool(self.enabled))
            .set("minPriority", Json::int(self.min_priority as i64))
            .set("unread", Json::int(self.unread as i64))
            .set("state", Json::str(self.state.as_str()))
            .set("detail", Json::str(&self.detail))
            .set("url", Json::str(self.web_url()))
            .set("secured", Json::Bool(self.token.is_some()))
            .build()
    }
}

/// A message that became a desktop notification.
#[derive(Debug, Clone)]
pub struct Notification {
    pub subscription_id: String,
    pub topic: String,
    pub title: String,
    pub body: String,
    pub priority: u8,
    pub tags: Vec<String>,
    pub url: String,
    pub received_at: i64,
}

impl Notification {
    pub fn to_json(&self) -> Json {
        Obj::new()
            .set("subscriptionId", Json::str(&self.subscription_id))
            .set("topic", Json::str(&self.topic))
            .set("title", Json::str(&self.title))
            .set("body", Json::str(&self.body))
            .set("priority", Json::int(self.priority as i64))
            .set(
                "tags",
                Json::Arr(self.tags.iter().map(Json::str).collect()),
            )
            .set("url", Json::str(&self.url))
            .set("receivedAt", Json::int(self.received_at))
            .build()
    }
}