//! The HBEL source spreadsheet: remembering where it is, choosing it with the
//! Windows "Open" dialog and opening it in Excel (or whatever opens .xlsx files).
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// What the background file dialog produced: `Some(None)` = cancelled.
type PickResult = Arc<Mutex<Option<Option<PathBuf>>>>;

/// A file dialog running on its own thread, so the window keeps drawing.
#[derive(Default)]
pub struct Picker {
    result: Option<PickResult>,
    /// Open the file once chosen (the dialog came from "Open Excel").
    pub open_after: bool,
}

impl Picker {
    pub fn busy(&self) -> bool {
        self.result.is_some()
    }

    /// Show the dialog (ignored while one is already showing).
    pub fn start(&mut self, ctx: &egui::Context, current: Option<&Path>, open_after: bool) {
        if self.busy() {
            return;
        }
        self.open_after = open_after;
        let slot: PickResult = Arc::new(Mutex::new(None));
        self.result = Some(slot.clone());
        let dir = current.and_then(|p| p.parent()).map(Path::to_path_buf);
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("excel-picker".into()).spawn(move || {
            let picked = pick_file(dir.as_deref());
            if let Ok(mut s) = slot.lock() {
                *s = Some(picked);
            }
            ctx.request_repaint();
        });
        if spawned.is_err() {
            self.result = None;
        }
    }

    /// The dialog's answer once it has closed: `Some(None)` when cancelled.
    pub fn poll(&mut self) -> Option<Option<PathBuf>> {
        let done = self.result.as_ref()?.lock().ok()?.take()?;
        self.result = None;
        Some(done)
    }
}

/// Open `path` with its default program. Returns a message on failure.
pub fn open(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("The Excel file was not found at {}.", path.display()));
    }
    open_with_shell(path)
}

/// File name for display, shortened from the middle to at most `max` characters.
pub fn short_name(path: &Path, max: usize) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.display().to_string());
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= max || max < 5 {
        return name;
    }
    let keep = max - 1;
    let head = keep / 2;
    let tail = keep - head;
    format!("{}…{}", chars[..head].iter().collect::<String>(), chars[chars.len() - tail..].iter().collect::<String>())
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    #[repr(C)]
    pub struct OpenFileNameW {
        pub struct_size: u32,
        pub owner: *mut c_void,
        pub instance: *mut c_void,
        pub filter: *const u16,
        pub custom_filter: *mut u16,
        pub max_cust_filter: u32,
        pub filter_index: u32,
        pub file: *mut u16,
        pub max_file: u32,
        pub file_title: *mut u16,
        pub max_file_title: u32,
        pub initial_dir: *const u16,
        pub title: *const u16,
        pub flags: u32,
        pub file_offset: u16,
        pub file_extension: u16,
        pub def_ext: *const u16,
        pub cust_data: isize,
        pub hook: *const c_void,
        pub template_name: *const u16,
        pub reserved_ptr: *mut c_void,
        pub reserved: u32,
        pub flags_ex: u32,
    }

    #[link(name = "comdlg32")]
    extern "system" {
        pub fn GetOpenFileNameW(ofn: *mut OpenFileNameW) -> i32;
    }
    #[link(name = "ole32")]
    extern "system" {
        pub fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> i32;
        pub fn CoUninitialize();
    }
    #[link(name = "shell32")]
    extern "system" {
        pub fn ShellExecuteW(hwnd: *mut c_void, op: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> isize;
    }

    pub fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        s.encode_wide().chain(std::iter::once(0)).collect()
    }
}

#[cfg(windows)]
fn pick_file(dir: Option<&Path>) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use win::*;
    const OFN_HIDEREADONLY: u32 = 0x4;
    const OFN_NOCHANGEDIR: u32 = 0x8;
    const OFN_PATHMUSTEXIST: u32 = 0x800;
    const OFN_FILEMUSTEXIST: u32 = 0x1000;
    const OFN_EXPLORER: u32 = 0x80000;
    const COINIT_APARTMENTTHREADED: u32 = 0x2;

    // "label\0pattern\0label\0pattern\0\0"
    let filter: Vec<u16> = "Excel workbooks (*.xlsx, *.xlsm, *.xls)\0*.xlsx;*.xlsm;*.xls\0All files (*.*)\0*.*\0\0".encode_utf16().collect();
    let title = wide(std::ffi::OsStr::new("Choose the HBEL comments Excel file"));
    let init = dir.map(|d| wide(d.as_os_str()));
    let mut buf = vec![0u16; 4096];
    let mut ofn = OpenFileNameW {
        struct_size: std::mem::size_of::<OpenFileNameW>() as u32,
        owner: std::ptr::null_mut(),
        instance: std::ptr::null_mut(),
        filter: filter.as_ptr(),
        custom_filter: std::ptr::null_mut(),
        max_cust_filter: 0,
        filter_index: 1,
        file: buf.as_mut_ptr(),
        max_file: buf.len() as u32,
        file_title: std::ptr::null_mut(),
        max_file_title: 0,
        initial_dir: init.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
        title: title.as_ptr(),
        flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_HIDEREADONLY,
        file_offset: 0,
        file_extension: 0,
        def_ext: std::ptr::null(),
        cust_data: 0,
        hook: std::ptr::null(),
        template_name: std::ptr::null(),
        reserved_ptr: std::ptr::null_mut(),
        reserved: 0,
        flags_ex: 0,
    };
    // SAFETY: COM is initialised for this (dialog-only) thread and released
    // after; `ofn` and every buffer it points to outlive the call.
    let ok = unsafe {
        let co = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
        let ok = GetOpenFileNameW(&mut ofn) != 0;
        if co >= 0 {
            CoUninitialize();
        }
        ok
    };
    if !ok {
        return None;
    }
    let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    Some(PathBuf::from(std::ffi::OsString::from_wide(&buf[..len])))
}

#[cfg(not(windows))]
fn pick_file(_dir: Option<&Path>) -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn open_with_shell(path: &Path) -> Result<(), String> {
    use win::*;
    let op = wide(std::ffi::OsStr::new("open"));
    let file = wide(path.as_os_str());
    const SW_SHOWNORMAL: i32 = 1;
    // SAFETY: both strings are NUL-terminated and live for the call.
    let r = unsafe { ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
    // ShellExecute reports success with a value above 32
    if r > 32 {
        Ok(())
    } else {
        Err(format!("Windows could not open {} (error {}). Is Excel installed?", path.display(), r))
    }
}

#[cfg(not(windows))]
fn open_with_shell(path: &Path) -> Result<(), String> {
    std::process::Command::new("xdg-open").arg(path).spawn().map(|_| ()).map_err(|e| format!("Could not open {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names() {
        assert_eq!(short_name(Path::new(r"C:\Lab\HBEL_Comment.xlsx"), 30), "HBEL_Comment.xlsx");
        let s = short_name(Path::new(r"C:\Lab\A very long workbook name for HBEL comments.xlsx"), 20);
        assert_eq!(s.chars().count(), 20);
        assert!(s.starts_with("A very lo") && s.ends_with("ents.xlsx") && s.contains('…'), "{}", s);
    }

    #[test]
    fn missing_file_is_reported() {
        let e = open(Path::new(r"Z:\definitely\not\here.xlsx")).unwrap_err();
        assert!(e.contains("not found"));
    }
}
