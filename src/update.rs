//! Update check against the project's GitHub releases.
//!
//! Releases are tagged with their date (`26-Nov-2026`, see RELEASING.md) and
//! this build's release date is the package version (`2026.11.26`). The check
//! runs once, silently, on a background thread at start-up (and again when the
//! user asks from the Theme menu). Nothing is shown unless a newer release
//! exists; then the title bar offers a link to its download page. Nothing is
//! downloaded or installed automatically.
use eframe::egui;
use std::sync::{Arc, Mutex};

/// GitHub repository the releases are published to (must be public).
pub const REPO: &str = "meowyourhead/Haema-calculator";

/// A release date as (year, month, day), comparable.
pub type Date = (u32, u32, u32);

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Not checked yet.
    Idle,
    Checking,
    UpToDate,
    /// A newer release: its tag (`3-Dec-2026`) and the page to download it from.
    Available { tag: String, url: String },
    /// No network, GitHub unreachable or the reply was not understood.
    Failed,
}

#[derive(Clone)]
pub struct Checker {
    state: Arc<Mutex<Status>>,
}

impl Default for Checker {
    fn default() -> Self {
        Checker { state: Arc::new(Mutex::new(Status::Idle)) }
    }
}

impl Checker {
    pub fn status(&self) -> Status {
        self.state.lock().map(|s| s.clone()).unwrap_or(Status::Failed)
    }

    /// Start a check in the background (ignored while one is running).
    pub fn start(&self, ctx: &egui::Context) {
        {
            let Ok(mut s) = self.state.lock() else { return };
            if *s == Status::Checking {
                return;
            }
            *s = Status::Checking;
        }
        let state = self.state.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("update-check".into()).spawn(move || {
            let result = match fetch_latest() {
                Some(body) => classify(&body, current()),
                None => Status::Failed,
            };
            if let Ok(mut s) = state.lock() {
                *s = result;
            }
            ctx.request_repaint();
        });
        if spawned.is_err() {
            if let Ok(mut s) = self.state.lock() {
                *s = Status::Failed;
            }
        }
    }
}

/// This build's release date, from the package version (`2026.11.26`).
pub fn current() -> Date {
    parse_version(env!("CARGO_PKG_VERSION")).unwrap_or((0, 0, 0))
}

fn parse_version(v: &str) -> Option<Date> {
    let mut it = v.split('.').map(|p| p.parse::<u32>().ok());
    let d = (it.next()??, it.next()??, it.next()??);
    (d.0 >= 2000 && (1..=12).contains(&d.1) && (1..=31).contains(&d.2)).then_some(d)
}

/// `26-Nov-2026` (also `26 Nov 2026`) → (2026, 11, 26).
pub fn parse_tag(tag: &str) -> Option<Date> {
    let parts: Vec<&str> = tag.trim().trim_start_matches(['v', 'V']).split(['-', ' ']).filter(|p| !p.is_empty()).collect();
    let [d, m, y] = parts.as_slice() else { return None };
    let day: u32 = d.parse().ok()?;
    let month = MONTHS.iter().position(|n| n.eq_ignore_ascii_case(m.get(..3).unwrap_or(m)))? as u32 + 1;
    let year: u32 = y.parse().ok()?;
    ((1..=31).contains(&day) && year >= 2000).then_some((year, month, day))
}

/// (2026, 11, 26) → `26 Nov 2026`.
pub fn date_label(d: Date) -> String {
    let m = MONTHS.get((d.1 as usize).wrapping_sub(1)).copied().unwrap_or("?");
    format!("{} {} {}", d.2, m, d.0)
}

/// `26-Nov-2026` → `26 Nov 2026` (how releases are titled).
pub fn display(tag: &str) -> String {
    tag.replace('-', " ")
}

/// Turn the GitHub "latest release" reply into a status.
fn classify(body: &str, current: Date) -> Status {
    let Some(tag) = json_str(body, "tag_name") else { return Status::Failed };
    match parse_tag(&tag) {
        Some(d) if d > current => {
            let url = format!("https://github.com/{}/releases/tag/{}", REPO, tag);
            Status::Available { tag, url }
        }
        Some(_) => Status::UpToDate,
        None => Status::Failed,
    }
}

/// The string value of `"key": "…"` in a JSON object (first occurrence).
fn json_str(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let rest = &body[body.find(&needle)? + needle.len()..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                c => out.push(c),
            },
            c => out.push(c),
        }
    }
    None
}

/// Ask GitHub for the latest release with the system's curl (bundled with
/// Windows 10 and later), so the app needs no TLS stack of its own.
fn fetch_latest() -> Option<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-s", "-f", "-L", "--max-time", "15", "-H", "Accept: application/vnd.github+json", "-A", "Haematology-Calculator-update-check", &url])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashes up from the GUI app.
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_and_versions() {
        assert_eq!(parse_tag("26-Nov-2026"), Some((2026, 11, 26)));
        assert_eq!(parse_tag("3 Dec 2026"), Some((2026, 12, 3)));
        assert_eq!(parse_tag("v1.2.3"), None);
        assert_eq!(parse_version("2026.11.26"), Some((2026, 11, 26)));
        assert_eq!(parse_version("0.2.0"), None);
        assert!(current() >= (2026, 11, 26), "package version must be the release date");
        assert_eq!(display("3-Dec-2026"), "3 Dec 2026");
    }

    #[test]
    fn newer_release_is_offered() {
        let body = r#"{"url":"x","html_url":"https://github.com/a/b/releases/tag/3-Dec-2026","tag_name" : "3-Dec-2026","name":"3 Dec 2026"}"#;
        match classify(body, (2026, 11, 26)) {
            Status::Available { tag, url } => {
                assert_eq!(tag, "3-Dec-2026");
                assert!(url.ends_with("/releases/tag/3-Dec-2026"));
            }
            s => panic!("{:?}", s),
        }
        assert_eq!(classify(body, (2026, 12, 3)), Status::UpToDate);
        assert_eq!(classify(body, (2027, 1, 1)), Status::UpToDate);
        assert_eq!(classify(r#"{"message":"Not Found"}"#, (2026, 11, 26)), Status::Failed);
    }

    #[test]
    fn json_strings() {
        assert_eq!(json_str(r#"{"a": "x\"y"}"#, "a").as_deref(), Some("x\"y"));
        assert_eq!(json_str(r#"{"a": 5}"#, "a"), None);
    }
}
