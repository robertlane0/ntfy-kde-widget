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
    /// Stream url with a `since` timestamp so reconnects resume instead of replaying.
    pub fn stream_url(&self, since: i64) -> String {
        format!(
            "{}/{}/sse?since={}",
            self.server.trim_end_matches('/'),
            crate::util::encode_segment(&self.topic),
            since,
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
            .set("tags", Json::Arr(self.tags.iter().map(Json::str).collect()))
            .set("url", Json::str(&self.url))
            .set("receivedAt", Json::int(self.received_at))
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub() -> Subscription {
        Subscription {
            id: "id".into(),
            server: "https://ntfy.sh".into(),
            topic: "alerts".into(),
            token: None,
            enabled: true,
            min_priority: 3,
            unread: 0,
            state: LinkState::Connecting,
            detail: String::new(),
            created_at: 1_700_000_000,
        }
    }

    #[test]
    fn stream_url_resumes_from_a_timestamp() {
        // ntfy reads `since` as a unix timestamp or a duration; anything else is
        // ignored and the whole topic is replayed.
        assert_eq!(
            sub().stream_url(1_790_877_143),
            "https://ntfy.sh/alerts/sse?since=1790877143"
        );
    }

    #[test]
    fn web_url_encodes_the_topic() {
        assert_eq!(sub().web_url(), "https://ntfy.sh/alerts");
    }
}
