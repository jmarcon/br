//! Shared types/helpers for the optional `br` background daemon (PRD §8.1,
//! "processo de fundo opcional: hot-reload de config, IPC local").
//!
//! The daemon listens on a localhost TCP port and accepts simple newline-delimited
//! `OPEN` requests from `br open`, avoiding the cold-start cost of loading the
//! config and discovering browsers on every link click.

pub mod client;

/// Loopback TCP port used for daemon IPC. Chosen in the IANA dynamic/private range.
pub const PORT: u16 = 47823;

/// One request line sent by `br open` to the daemon: `OPEN <url>\t<source_app>\t<app>\t<private>`.
/// `<source_app>`, `<app>` use `-` for "none"; `<private>` is `0` or `1`.
#[derive(Debug, Clone)]
pub struct OpenRequest {
    pub url: String,
    pub source_app: Option<String>,
    pub app: Option<String>,
    pub private: bool,
    pub modifier_keys: Vec<String>,
}

impl OpenRequest {
    pub fn encode(&self) -> String {
        format!(
            "OPEN\t{}\t{}\t{}\t{}\t{}\n",
            self.url,
            self.source_app.as_deref().unwrap_or("-"),
            self.app.as_deref().unwrap_or("-"),
            if self.private { 1 } else { 0 },
            if self.modifier_keys.is_empty() {
                "-".to_string()
            } else {
                self.modifier_keys.join(",")
            },
        )
    }

    pub fn decode(line: &str) -> Option<Self> {
        let mut parts = line.trim_end().split('\t');
        if parts.next()? != "OPEN" {
            return None;
        }
        let url = parts.next()?.to_string();
        let source_app = match parts.next()? {
            "-" => None,
            s => Some(s.to_string()),
        };
        let app = match parts.next()? {
            "-" => None,
            s => Some(s.to_string()),
        };
        let private = parts.next()? == "1";
        let modifier_keys = parts
            .next()
            .map(|s| {
                if s == "-" {
                    Vec::new()
                } else {
                    s.split(',').map(str::to_string).collect()
                }
            })
            .unwrap_or_default();
        Some(OpenRequest {
            url,
            source_app,
            app,
            private,
            modifier_keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_full_request() {
        let req = OpenRequest {
            url: "https://example.com".to_string(),
            source_app: Some("Slack".to_string()),
            app: Some("chrome-default".to_string()),
            private: true,
            modifier_keys: vec!["shift".to_string()],
        };
        let decoded = OpenRequest::decode(&req.encode()).unwrap();
        assert_eq!(decoded.url, req.url);
        assert_eq!(decoded.source_app, req.source_app);
        assert_eq!(decoded.app, req.app);
        assert_eq!(decoded.private, req.private);
        assert_eq!(decoded.modifier_keys, req.modifier_keys);
    }

    #[test]
    fn round_trips_minimal_request() {
        let req = OpenRequest {
            url: "https://example.com".to_string(),
            source_app: None,
            app: None,
            private: false,
            modifier_keys: Vec::new(),
        };
        let decoded = OpenRequest::decode(&req.encode()).unwrap();
        assert_eq!(decoded.source_app, None);
        assert_eq!(decoded.app, None);
        assert!(!decoded.private);
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(OpenRequest::decode("NOT-OPEN\tx\n").is_none());
        assert!(OpenRequest::decode("OPEN\tonly-url\n").is_none());
    }
}
