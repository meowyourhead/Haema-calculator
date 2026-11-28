//! Update check against the project's GitHub releases.
//!
//! Releases are tagged with their date (`26-Nov-2026`, see RELEASING.md) and
//! this build's release date is the package version (`2026.11.26`). The check
//! runs once, silently, on a background thread at start-up (and again when the
//! user asks from the Theme menu). Nothing is shown unless a newer release
//! exists; then the title bar shows "Update available". Clicking it asks the
//! user to confirm. Only after "Update" is the release's EXE downloaded (with
//! the system's curl) next to the running one, swapped into its place, and the
//! app relaunched. Windows lets a running EXE be renamed but not overwritten,
//! so the old one steps aside as `<name>.old-<time>.exe` and is deleted on a
//! later start, once nothing runs from it. If any step fails, nothing is
//! replaced and the dialog offers the release's download page instead.
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// A newer release: its tag (`3-Dec-2026`), the page to download it from,
    /// and its EXE (None if the release has no EXE attached).
    Available { tag: String, url: String, asset: Option<Asset> },
    /// No network, GitHub unreachable or the reply was not understood.
    Failed,
}

/// The EXE attached to a release.
#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    pub url: String,
    /// Bytes, as GitHub reports it (0 if unknown).
    pub size: u64,
}

/// Progress of an approved update.
#[derive(Clone, Debug, PartialEq)]
pub enum Install {
    Idle,
    Downloading { done: u64, total: u64 },
    /// The new EXE is in place and has been started; this window should close.
    Restarting,
    /// Nothing was replaced; the message says why.
    Failed(String),
}

#[derive(Clone)]
pub struct Checker {
    state: Arc<Mutex<Status>>,
    install: Arc<Mutex<Install>>,
    cancel: Arc<AtomicBool>,
}

impl Default for Checker {
    fn default() -> Self {
        Checker { state: Arc::new(Mutex::new(Status::Idle)), install: Arc::new(Mutex::new(Install::Idle)), cancel: Arc::new(AtomicBool::new(false)) }
    }
}

impl Checker {
    pub fn status(&self) -> Status {
        self.state.lock().map(|s| s.clone()).unwrap_or(Status::Failed)
    }

    pub fn install_status(&self) -> Install {
        self.install.lock().map(|s| s.clone()).unwrap_or(Install::Idle)
    }

    fn set_install(&self, s: Install) {
        if let Ok(mut st) = self.install.lock() {
            *st = s;
        }
    }

    /// Back to "not updating" after a failure was shown.
    pub fn dismiss_install(&self) {
        if matches!(self.install_status(), Install::Failed(_)) {
            self.set_install(Install::Idle);
        }
    }

    /// Stop a running download (the half-downloaded file is removed).
    pub fn cancel_install(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Screenshot harness: pretend an update is at `s`.
    #[cfg(feature = "screenshot")]
    pub fn force_install(&self, s: Install) {
        self.set_install(s);
    }

    /// The user approved the update: download `asset`, swap it in for the
    /// running EXE and start it. Ignored while one is already running.
    pub fn begin_install(&self, ctx: &egui::Context, asset: Asset) {
        {
            let Ok(mut s) = self.install.lock() else { return };
            if matches!(*s, Install::Downloading { .. } | Install::Restarting) {
                return;
            }
            *s = Install::Downloading { done: 0, total: asset.size };
        }
        self.cancel.store(false, Ordering::SeqCst);
        let me = self.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("update-install".into()).spawn(move || {
            let result = match install(&me, &ctx, &asset) {
                Ok(()) => Install::Restarting,
                Err(None) => Install::Idle,
                Err(Some(msg)) => Install::Failed(msg),
            };
            me.set_install(result);
            ctx.request_repaint();
        });
        if spawned.is_err() {
            self.set_install(Install::Failed("Couldn't start the download.".into()));
        }
    }

    /// Screenshot harness: pretend the check found `s`.
    #[cfg(feature = "screenshot")]
    pub fn force(&self, s: Status) {
        if let Ok(mut st) = self.state.lock() {
            *st = s;
        }
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
            Status::Available { tag, url, asset: exe_asset(body) }
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

/// The unsigned number value of `"key": 123` that starts at `body[from..]`.
fn json_num_at(body: &str, from: usize) -> Option<u64> {
    let rest = body[from..].trim_start().strip_prefix(':')?.trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// The first `.exe` among the release's assets: its download URL and size.
/// GitHub lists each asset's `"size"` before its `"browser_download_url"`.
fn exe_asset(body: &str) -> Option<Asset> {
    let key = "\"browser_download_url\"";
    let mut from = 0;
    while let Some(i) = body[from..].find(key) {
        let at = from + i;
        from = at + key.len();
        let Some(url) = json_str(&body[at..], "browser_download_url") else { continue };
        if !url.to_ascii_lowercase().ends_with(".exe") || !url.starts_with("https://") {
            continue;
        }
        let size = body[..at].rfind("\"size\"").and_then(|s| json_num_at(body, s + "\"size\"".len())).unwrap_or(0);
        return Some(Asset { url, size });
    }
    None
}

/// The system's curl (bundled with Windows 10 and later), so the app needs
/// no TLS stack of its own, started without a console window.
fn curl() -> std::process::Command {
    let mut cmd = std::process::Command::new("curl");
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashes up from the GUI app.
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// Ask GitHub for the latest release.
fn fetch_latest() -> Option<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let mut cmd = curl();
    cmd.args(["-s", "-f", "-L", "--max-time", "15", "-H", "Accept: application/vnd.github+json", "-A", "Haematology-Calculator-update-check", &url])
        .stdout(std::process::Stdio::piped());
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// The running EXE, its folder and its name without `.exe`.
fn running_exe() -> Option<(PathBuf, PathBuf, String)> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    let stem = exe.file_stem()?.to_string_lossy().into_owned();
    Some((exe, dir, stem))
}

/// A complete Windows program: the size GitHub reported (when known) and the
/// `MZ` header every EXE starts with.
fn looks_complete(path: &Path, size: u64) -> bool {
    use std::io::Read;
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if meta.len() < 64 * 1024 || (size > 0 && meta.len() != size) {
        return false;
    }
    let mut head = [0u8; 2];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_ok() && head == *b"MZ"
}

/// Download, swap and relaunch. `Err(None)` = cancelled; `Err(Some(msg))` = failed.
/// Nothing is replaced unless the download is complete.
fn install(me: &Checker, ctx: &egui::Context, asset: &Asset) -> Result<(), Option<String>> {
    let fail = |s: &str| Some(s.to_string());
    let (exe, dir, stem) = running_exe().ok_or_else(|| fail("Couldn't find the folder this app runs from."))?;
    // next to the running EXE, so the swap is a rename on the same drive
    let tmp = dir.join(format!("{}.update.tmp", stem));
    let _ = std::fs::remove_file(&tmp);
    let mut child = curl()
        .args(["-s", "-f", "-L", "--connect-timeout", "20", "--max-time", "900", "-A", "Haematology-Calculator-update", "-o"])
        .arg(&tmp)
        .arg(&asset.url)
        .spawn()
        .map_err(|_| fail("Couldn't start the download (curl was not found)."))?;
    let code = loop {
        if me.cancel.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&tmp);
            return Err(None);
        }
        match child.try_wait() {
            Ok(Some(st)) => break st.code(),
            Ok(None) => {}
            Err(_) => {
                let _ = child.kill();
                break None;
            }
        }
        let done = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
        me.set_install(Install::Downloading { done, total: asset.size });
        ctx.request_repaint();
        std::thread::sleep(std::time::Duration::from_millis(150));
    };
    let cleanup = || {
        let _ = std::fs::remove_file(&tmp);
    };
    match code {
        Some(0) => {}
        // curl: "failed writing received data to disk"
        Some(23) => {
            cleanup();
            return Err(fail("Couldn't save the update in the app's folder. You may not have permission to change files there."));
        }
        _ => {
            cleanup();
            return Err(fail("Couldn't download the update. Check the network connection and try again."));
        }
    }
    if !looks_complete(&tmp, asset.size) {
        cleanup();
        return Err(fail("The download was incomplete. Nothing was changed."));
    }
    // Windows allows renaming a running EXE, not overwriting it
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let old = dir.join(format!("{}.old-{}.exe", stem, secs));
    if std::fs::rename(&exe, &old).is_err() {
        cleanup();
        return Err(fail("Windows wouldn't let the app be replaced. You may not have permission to change files in its folder. Nothing was changed."));
    }
    if std::fs::rename(&tmp, &exe).is_err() {
        let _ = std::fs::rename(&old, &exe);
        cleanup();
        return Err(fail("Couldn't put the new version in place. Nothing was changed."));
    }
    if std::process::Command::new(&exe).current_dir(&dir).spawn().is_err() {
        return Err(fail("The new version is installed but couldn't start. Close this window and open the app again."));
    }
    Ok(())
}

/// Delete EXEs left behind by earlier updates. Any still running (another
/// user's window, or this one's predecessor still closing) cannot be deleted
/// and stays until a later start.
pub fn cleanup_old() {
    let _ = std::thread::Builder::new().name("update-cleanup".into()).spawn(|| {
        let Some((_, dir, stem)) = running_exe() else { return };
        let prefix = format!("{}.old-", stem);
        let Ok(entries) = std::fs::read_dir(&dir) else { return };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with(&prefix) && name.to_ascii_lowercase().ends_with(".exe") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    });
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
            Status::Available { tag, url, asset } => {
                assert_eq!(tag, "3-Dec-2026");
                assert!(url.ends_with("/releases/tag/3-Dec-2026"));
                assert_eq!(asset, None);
            }
            s => panic!("{:?}", s),
        }
        assert_eq!(classify(body, (2026, 12, 3)), Status::UpToDate);
        assert_eq!(classify(body, (2027, 1, 1)), Status::UpToDate);
        assert_eq!(classify(r#"{"message":"Not Found"}"#, (2026, 11, 26)), Status::Failed);
    }

    #[test]
    fn exe_asset_is_found() {
        // shape of GitHub's reply: a zip first, then the EXE
        let body = r#"{"tag_name":"3-Dec-2026","assets":[
            {"name":"notes.zip","uploader":{"login":"x"},"size":10,"browser_download_url":"https://github.com/a/b/releases/download/3-Dec-2026/notes.zip"},
            {"name":"Haematology-Calculator-3-Dec-2026.exe","uploader":{"login":"x"},"size" : 7654321,"browser_download_url":"https://github.com/a/b/releases/download/3-Dec-2026/Haematology-Calculator-3-Dec-2026.exe"}
        ]}"#;
        let a = exe_asset(body).expect("exe");
        assert!(a.url.ends_with("Haematology-Calculator-3-Dec-2026.exe"));
        assert_eq!(a.size, 7654321);
        assert_eq!(exe_asset(r#"{"assets":[]}"#), None);
    }

    #[test]
    fn json_strings() {
        assert_eq!(json_str(r#"{"a": "x\"y"}"#, "a").as_deref(), Some("x\"y"));
        assert_eq!(json_str(r#"{"a": 5}"#, "a"), None);
    }
}
