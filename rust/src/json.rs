//! Minimal JSON value, parser and writer.
//!
//! Only what the ntfy protocol and our own config file need: no external crates.

use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    /// Numbers keep their source text so large ids survive a round trip.
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn str(s: impl Into<String>) -> Self {
        Json::Str(s.into())
    }

    pub fn int(v: i64) -> Self {
        Json::Num(v.to_string())
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Num(n) => n.parse().ok(),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }

    /// Convenience for string fields with a fallback.
    pub fn field_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Json::as_str)
    }

    pub fn to_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(true) => out.push_str("true"),
            Json::Bool(false) => out.push_str("false"),
            Json::Num(n) => out.push_str(n),
            Json::Str(s) => write_string(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(entries) => {
                out.push('{');
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn parse(input: &str) -> Result<Json, String> {
    let bytes: Vec<char> = input.chars().collect();
    let mut p = Parser { src: &bytes, pos: 0 };
    p.skip_ws();
    let value = p.value()?;
    p.skip_ws();
    if p.pos != p.src.len() {
        return Err("trailing data after JSON value".into());
    }
    Ok(value)
}

struct Parser<'a> {
    src: &'a [char],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.bump() == Some(c) {
            Ok(())
        } else {
            Err(format!("expected '{c}' at offset {}", self.pos))
        }
    }

    fn literal(&mut self, word: &str) -> Result<(), String> {
        for c in word.chars() {
            if self.bump() != Some(c) {
                return Err(format!("bad literal, expected {word}"));
            }
        }
        Ok(())
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            None => Err("unexpected end of input".into()),
            Some('n') => self.literal("null").map(|()| Json::Null),
            Some('t') => self.literal("true").map(|()| Json::Bool(true)),
            Some('f') => self.literal("false").map(|()| Json::Bool(false)),
            Some('"') => self.string().map(Json::Str),
            Some('[') => self.array(),
            Some('{') => self.object(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            Some(c) => Err(format!("unexpected character '{c}' at offset {}", self.pos)),
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some(']') => return Ok(Json::Arr(items)),
                _ => return Err("malformed array".into()),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut entries: Vec<(String, Json)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(Json::Obj(entries));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(':')?;
            let value = self.value()?;
            match entries.iter_mut().find(|(k, _)| *k == key) {
                Some(slot) => slot.1 = value,
                None => entries.push((key, value)),
            }
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some('}') => return Ok(Json::Obj(entries)),
                _ => return Err("malformed object".into()),
            }
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-')
        {
            self.pos += 1;
        }
        let text: String = self.src[start..self.pos].iter().collect();
        if text.is_empty() || text.parse::<f64>().is_err() {
            return Err(format!("malformed number '{text}'"));
        }
        Ok(Json::Num(text))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err("unterminated string".into()),
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{08}'),
                    Some('f') => out.push('\u{0c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => out.push(self.unicode_escape()?),
                    _ => return Err("bad escape".into()),
                },
                Some(c) => out.push(c),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            let c = self.bump().ok_or("truncated \\u escape")?;
            let d = c.to_digit(16).ok_or("bad hex digit in \\u escape")?;
            v = v * 16 + d;
        }
        Ok(v)
    }

    fn unicode_escape(&mut self) -> Result<char, String> {
        let first = self.hex4()?;
        // Surrogate pair.
        if (0xD800..0xDC00).contains(&first) {
            if self.peek() == Some('\\') {
                self.pos += 1;
                self.expect('u')?;
                let second = self.hex4()?;
                let combined = 0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00);
                return char::from_u32(combined).ok_or_else(|| "bad surrogate pair".to_string());
            }
            return Err("lone surrogate".into());
        }
        char::from_u32(first).ok_or_else(|| "bad code point".to_string())
    }
}

/// Ordered object helper used when building outgoing JSON.
#[derive(Default)]
pub struct Obj(Vec<(String, Json)>);

impl Obj {
    pub fn new() -> Self {
        Obj(Vec::new())
    }

    pub fn set(mut self, key: &str, value: Json) -> Self {
        self.0.push((key.to_string(), value));
        self
    }

    pub fn build(self) -> Json {
        Json::Obj(self.0)
    }
}

/// Helper for map-shaped lookups in our own config files.
pub fn obj_map(value: &Json) -> BTreeMap<&str, &Json> {
    match value {
        Json::Obj(entries) => entries.iter().map(|(k, v)| (k.as_str(), v)).collect(),
        _ => BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sse_payload() {
        let v = parse(r#"{"id":"abc","time":1700,"event":"message","message":"hi\nthere"}"#).unwrap();
        assert_eq!(v.field_str("id"), Some("abc"));
        assert_eq!(v.get("time").unwrap().as_i64(), Some(1700));
        assert_eq!(v.field_str("message"), Some("hi\nthere"));
    }

    #[test]
    fn round_trips_unicode_and_escapes() {
        let v = parse(r#"{"a":"é😀","b":[1,2,{"c":null}]}"#).unwrap();
        assert_eq!(v.field_str("a"), Some("é😀"));
        let out = v.to_string();
        let again = parse(&out).unwrap();
        assert_eq!(v, again);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("{\"a\":}").is_err());
        assert!(parse("[1,2").is_err());
        assert!(parse("nul").is_err());
    }
}