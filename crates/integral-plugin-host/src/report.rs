//! JSON-lines output: one object per event, on stdout, flushed per line.

use std::io::Write;
use std::time::Instant;

use serde_json::{Map, Value, json};

/// Writes events as JSON lines, each stamped with seconds since start.
pub struct Report {
    started: Instant,
}

impl Report {
    /// Starts the clock.
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }

    fn elapsed(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    /// Builds `{"t": ..., "event": <event>, ...fields}`.
    ///
    /// `fields` must be a JSON object (anything else is ignored). It cannot
    /// overwrite `t` or `event`.
    fn line(&self, event: &str, fields: Value) -> Value {
        let mut line = Map::new();
        if let Value::Object(fields) = fields {
            line.extend(fields);
        }
        line.insert(
            "t".into(),
            json!((self.elapsed() * 1000.0).round() / 1000.0),
        );
        line.insert("event".into(), json!(event));
        Value::Object(line)
    }

    /// Emits one event as one line.
    pub fn emit(&self, event: &str, fields: Value) {
        let mut out = std::io::stdout().lock();
        // A closed stdout must not kill the run half-way through a drag.
        let _ = writeln!(out, "{}", self.line(event, fields));
        let _ = out.flush();
    }

    /// Emits an `assert` event and returns whether it held.
    pub fn check(&self, name: &str, ok: bool, detail: Value) -> bool {
        self.emit(
            "assert",
            json!({ "name": name, "ok": ok, "detail": detail }),
        );
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_cannot_overwrite_the_envelope() {
        let line = Report::new().line("real", json!({ "t": "x", "event": "y", "k": 1 }));
        assert_eq!(line["event"], "real");
        assert!(line["t"].is_number());
        assert_eq!(line["k"], 1);
    }
}
