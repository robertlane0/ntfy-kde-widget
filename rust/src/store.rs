//! Persisted subscription list, stored as JSON next to the user's other app data.

use std::fs;

use crate::json::{self, Json, Obj};
use crate::model::Subscription;
use crate::util;

pub fn load() -> Vec<Subscription> {
    let path = util::config_file();
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(doc) = json::parse(&text) else {
        return Vec::new();
    };
    let Some(items) = doc.get("subscriptions").and_then(Json::as_array) else {
        return Vec::new();
    };
    items.iter().filter_map(from_json).collect()
}

fn from_json(v: &Json) -> Option<Subscription> {
    let server = util::normalise_server(v.field_str("server")?);
    let topic = v.field_str("topic")?.trim().to_string();
    if server.is_empty() || topic.is_empty() {
        return None;
    }
    let token = v
        .field_str("token")
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string);
    Some(Subscription {
        id: v.field_str("id").map(str::to_string).unwrap_or_else(util::new_id),
        server,
        topic,
        token,
        enabled: v.get("enabled").and_then(Json::as_bool).unwrap_or(true),
        min_priority: v
            .get("minPriority")
            .and_then(Json::as_i64)
            .unwrap_or(1)
            .clamp(1, 5) as u8,
        unread: 0,
        state: crate::model::LinkState::Connecting,
        detail: String::new(),
        created_at: v
            .get("createdAt")
            .and_then(Json::as_i64)
            .unwrap_or_else(util::now_secs),
    })
}

pub fn save(subs: &[Subscription]) -> Result<(), String> {
    let dir = util::config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;

    let items = Json::Arr(
        subs.iter()
            .map(|s| {
                Obj::new()
                    .set("id", Json::str(&s.id))
                    .set("server", Json::str(&s.server))
                    .set("topic", Json::str(&s.topic))
                    .set(
                        "token",
                        s.token.clone().map_or(Json::Null, Json::Str),
                    )
                    .set("enabled", Json::Bool(s.enabled))
                    .set("minPriority", Json::int(s.min_priority as i64))
                    .set("createdAt", Json::int(s.created_at))
                    .build()
            })
            .collect(),
    );
    let doc = Obj::new().set("version", Json::int(1)).set("subscriptions", items).build();

    // Write to a sibling temp file and rename so a crash cannot truncate the real one.
    let target = util::config_file();
    let tmp = target.with_extension("json.tmp");
    fs::write(&tmp, doc.to_string()).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, &target).map_err(|e| format!("cannot replace {}: {e}", target.display()))?;
    Ok(())
}