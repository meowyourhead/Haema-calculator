//! Themed pointers (Genshin Impact, Pokémon), as native Windows cursors.
//!
//! Genshin's mouse set ships a gold four-point star for the idle pointer, a
//! sparkle hand over links, diamond arms for resizing and two animated busy
//! spinners. The Pokémon set (ful1e5/pokemon-cursor, "Pokemon-Large") adds its
//! own text-select and unavailable pointers. Windows owns their rendering
//! (hotspots, DPI scaling and `.ani` timing), so the files are embedded,
//! written to the app's local data folder and loaded with `LoadCursorFromFileW`.
//!
//! winit resets the cursor from its own cache on every `WM_SETCURSOR`, which
//! would stomp a cursor set from `App::update`. While a themed set is active
//! we therefore subclass the window procedure and answer client-area
//! `WM_SETCURSOR` ourselves; every other message is passed through untouched.

use eframe::egui;

/// A themed cursor set.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Set {
    Genshin,
    Pokemon,
}

/// Install the cursor override on the app window (Windows only; no-op elsewhere).
pub fn install(frame: &eframe::Frame) {
    #[cfg(windows)]
    imp::install(frame);
    #[cfg(not(windows))]
    let _ = frame;
}

/// Follow this frame's requested cursor with `set` while a themed palette is
/// active, and hand the pointer back to winit when `set` is `None`.
pub fn apply(ctx: &egui::Context, set: Option<Set>, icon: egui::CursorIcon) {
    #[cfg(windows)]
    imp::apply(ctx, set, icon);
    #[cfg(not(windows))]
    let _ = (ctx, set, icon);
}

#[cfg(windows)]
mod imp {
    use super::{egui, Set};
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering::Relaxed};
    use std::sync::OnceLock;

    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    type Hwnd = *mut core::ffi::c_void;
    type Hcursor = *mut core::ffi::c_void;
    type Wndproc = Option<unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize>;

    const WM_SETCURSOR: u32 = 0x0020;
    const HTCLIENT: u32 = 1;
    const GWLP_WNDPROC: i32 = -4;

    /// `MAKEINTRESOURCE` ids of the stock Windows cursors.
    const IDC_ARROW: usize = 32512;
    const IDC_IBEAM: usize = 32513;
    const IDC_WAIT: usize = 32514;
    const IDC_CROSS: usize = 32515;
    const IDC_APPSTARTING: usize = 32516;
    const IDC_SIZENWSE: usize = 32643;
    const IDC_SIZENESW: usize = 32642;
    const IDC_SIZEWE: usize = 32644;
    const IDC_SIZENS: usize = 32645;
    const IDC_SIZEALL: usize = 32646;
    const IDC_HAND: usize = 32649;
    const IDC_HELP: usize = 32651;

    #[link(name = "user32")]
    extern "system" {
        fn SetCursor(hcursor: Hcursor) -> Hcursor;
        fn LoadCursorFromFileW(name: *const u16) -> Hcursor;
        fn LoadCursorW(instance: Hwnd, name: *const u16) -> Hcursor;
        fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> isize;
        fn CallWindowProcW(previous: Wndproc, hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn DefWindowProcW(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn PostMessageW(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> i32;
    }

    static WINDOW: AtomicIsize = AtomicIsize::new(0);
    static ORIGINAL_WNDPROC: AtomicIsize = AtomicIsize::new(0);
    static INSTALLED: AtomicBool = AtomicBool::new(false);
    /// Whether the last frame wanted a themed pointer.
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    /// The themed cursor the window procedure should return for client areas.
    static ACTIVE_CURSOR: AtomicIsize = AtomicIsize::new(0);

    /// The cursor states a themed set can provide, in table order.
    #[derive(Clone, Copy)]
    #[repr(usize)]
    enum Kind {
        Normal = 0,
        Link = 1,
        Help = 2,
        Move = 3,
        Location = 4,
        ResizeHorizontal = 5,
        ResizeVertical = 6,
        ResizeDiagonal1 = 7,
        ResizeDiagonal2 = 8,
        Busy = 9,
        Working = 10,
        Text = 11,
        NotAllowed = 12,
    }

    const KINDS: usize = 13;

    /// One slot per `Kind`; `None` falls back to the set's normal pointer.
    type Files = [Option<(&'static str, &'static [u8])>; KINDS];

    const GENSHIN: Files = [
        Some(("normal.cur", include_bytes!("../assets/cursors/genshin/normal.cur"))),
        Some(("link-select.cur", include_bytes!("../assets/cursors/genshin/link-select.cur"))),
        Some(("help-select.cur", include_bytes!("../assets/cursors/genshin/help-select.cur"))),
        Some(("move.cur", include_bytes!("../assets/cursors/genshin/move.cur"))),
        Some(("location-select.cur", include_bytes!("../assets/cursors/genshin/location-select.cur"))),
        Some(("resize-horizontal.cur", include_bytes!("../assets/cursors/genshin/resize-horizontal.cur"))),
        Some(("resize-vertical.cur", include_bytes!("../assets/cursors/genshin/resize-vertical.cur"))),
        Some(("resize-diagonal-1.cur", include_bytes!("../assets/cursors/genshin/resize-diagonal-1.cur"))),
        Some(("resize-diagonal-2.cur", include_bytes!("../assets/cursors/genshin/resize-diagonal-2.cur"))),
        Some(("busy.ani", include_bytes!("../assets/cursors/genshin/busy.ani"))),
        Some(("working-in-background.ani", include_bytes!("../assets/cursors/genshin/working-in-background.ani"))),
        None,
        None,
    ];

    const POKEMON: Files = [
        Some(("normal.cur", include_bytes!("../assets/cursors/pokemon/normal.cur"))),
        Some(("link-select.cur", include_bytes!("../assets/cursors/pokemon/link-select.cur"))),
        Some(("help-select.cur", include_bytes!("../assets/cursors/pokemon/help-select.cur"))),
        Some(("move.cur", include_bytes!("../assets/cursors/pokemon/move.cur"))),
        Some(("location-select.cur", include_bytes!("../assets/cursors/pokemon/location-select.cur"))),
        Some(("resize-horizontal.cur", include_bytes!("../assets/cursors/pokemon/resize-horizontal.cur"))),
        Some(("resize-vertical.cur", include_bytes!("../assets/cursors/pokemon/resize-vertical.cur"))),
        Some(("resize-diagonal-1.cur", include_bytes!("../assets/cursors/pokemon/resize-diagonal-1.cur"))),
        Some(("resize-diagonal-2.cur", include_bytes!("../assets/cursors/pokemon/resize-diagonal-2.cur"))),
        Some(("busy.ani", include_bytes!("../assets/cursors/pokemon/busy.ani"))),
        Some(("working-in-background.ani", include_bytes!("../assets/cursors/pokemon/working-in-background.ani"))),
        Some(("text-select.cur", include_bytes!("../assets/cursors/pokemon/text-select.cur"))),
        Some(("unavailable.cur", include_bytes!("../assets/cursors/pokemon/unavailable.cur"))),
    ];

    struct Cursors {
        handles: [isize; KINDS],
    }

    impl Cursors {
        fn get(&self, kind: Kind) -> isize {
            match self.handles[kind as usize] {
                0 => self.handles[Kind::Normal as usize],
                h => h,
            }
        }
    }

    static GENSHIN_CURSORS: OnceLock<Cursors> = OnceLock::new();
    static POKEMON_CURSORS: OnceLock<Cursors> = OnceLock::new();

    fn cursors(set: Set) -> &'static Cursors {
        match set {
            Set::Genshin => GENSHIN_CURSORS.get_or_init(|| load("genshin", &GENSHIN)),
            Set::Pokemon => POKEMON_CURSORS.get_or_init(|| load("pokemon", &POKEMON)),
        }
    }

    /// Write the embedded set next to the settings file, then load each cursor.
    /// Rewritten only when a file is missing or stale, so upgrades replace it.
    fn load(sub: &str, files: &Files) -> Cursors {
        let mut handles = [0isize; KINDS];
        let Some(dir) = materialize(sub, files) else { return Cursors { handles } };
        for (i, entry) in files.iter().enumerate() {
            let Some((name, _)) = entry else { continue };
            let wide: Vec<u16> = dir.join(name).as_os_str().encode_wide().chain(Some(0)).collect();
            // Never destroyed: cursor handles live for the process lifetime.
            handles[i] = unsafe { LoadCursorFromFileW(wide.as_ptr()) } as isize;
        }
        Cursors { handles }
    }

    fn materialize(sub: &str, files: &Files) -> Option<PathBuf> {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .unwrap_or_else(std::env::temp_dir);
        let dir = base.join("HaematologyCalculator").join("cursors").join(sub);
        std::fs::create_dir_all(&dir).ok()?;
        for (name, data) in files.iter().flatten() {
            let path = dir.join(name);
            let current = std::fs::read(&path).ok();
            if current.as_deref() != Some(*data) {
                // A failed write leaves the previous copy in place; loading below
                // then falls back to whatever is still readable.
                let _ = std::fs::write(&path, data);
            }
        }
        Some(dir)
    }

    pub fn install(frame: &eframe::Frame) {
        if INSTALLED.load(Relaxed) {
            return;
        }
        let Ok(handle) = frame.window_handle() else { return };
        let RawWindowHandle::Win32(win) = handle.as_raw() else { return };
        let hwnd = win.hwnd.get() as Hwnd;
        let previous = unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, wndproc as *const () as usize as isize) };
        if previous != 0 {
            WINDOW.store(hwnd as isize, Relaxed);
            ORIGINAL_WNDPROC.store(previous, Relaxed);
            INSTALLED.store(true, Relaxed);
        }
    }

    pub fn apply(ctx: &egui::Context, set: Option<Set>, icon: egui::CursorIcon) {
        let inside = ctx.input(|i| i.pointer.latest_pos().is_some());
        if let Some(set) = set {
            let handle = cursors(set).get(kind(icon));
            if handle != 0 {
                let changed = ACTIVE_CURSOR.swap(handle, Relaxed) != handle || !ACTIVE.load(Relaxed);
                ACTIVE.store(true, Relaxed);
                if inside {
                    unsafe { SetCursor(handle as Hcursor) };
                }
                if changed {
                    // winit applies its own cursor after `update` returns, so ask for
                    // one more WM_SETCURSOR once the frame is painted.
                    let hwnd = WINDOW.load(Relaxed);
                    if hwnd != 0 {
                        unsafe {
                            PostMessageW(hwnd as Hwnd, WM_SETCURSOR, hwnd as usize, HTCLIENT as isize);
                        }
                    }
                }
                return;
            }
        }
        if ACTIVE.swap(false, Relaxed) {
            // Leaving the palette: put back the stock cursor winit would have set.
            let stock = stock_cursor(icon);
            if inside && !stock.is_null() {
                unsafe { SetCursor(stock) };
            }
        }
    }

    /// Map egui's semantic cursor onto the closest themed mouse asset.
    fn kind(icon: egui::CursorIcon) -> Kind {
        use egui::CursorIcon as C;
        match icon {
            C::PointingHand => Kind::Link,
            C::Text | C::VerticalText => Kind::Text,
            C::NotAllowed | C::NoDrop => Kind::NotAllowed,
            C::Help => Kind::Help,
            C::Move => Kind::Move,
            C::Crosshair => Kind::Location,
            C::Wait => Kind::Busy,
            C::Progress => Kind::Working,
            C::ResizeHorizontal | C::ResizeEast | C::ResizeWest | C::ResizeColumn => Kind::ResizeHorizontal,
            C::ResizeVertical | C::ResizeNorth | C::ResizeSouth | C::ResizeRow => Kind::ResizeVertical,
            C::ResizeNwSe | C::ResizeNorthWest | C::ResizeSouthEast => Kind::ResizeDiagonal1,
            C::ResizeNeSw | C::ResizeNorthEast | C::ResizeSouthWest => Kind::ResizeDiagonal2,
            _ => Kind::Normal,
        }
    }

    fn stock_cursor(icon: egui::CursorIcon) -> Hcursor {
        use egui::CursorIcon as C;
        let id = match icon {
            C::PointingHand => IDC_HAND,
            C::Help => IDC_HELP,
            C::Wait => IDC_WAIT,
            C::Progress => IDC_APPSTARTING,
            C::Crosshair => IDC_CROSS,
            C::Text | C::VerticalText | C::Cell => IDC_IBEAM,
            C::Move | C::AllScroll => IDC_SIZEALL,
            C::ResizeHorizontal | C::ResizeEast | C::ResizeWest | C::ResizeColumn => IDC_SIZEWE,
            C::ResizeVertical | C::ResizeNorth | C::ResizeSouth | C::ResizeRow => IDC_SIZENS,
            C::ResizeNwSe | C::ResizeNorthWest | C::ResizeSouthEast => IDC_SIZENWSE,
            C::ResizeNeSw | C::ResizeNorthEast | C::ResizeSouthWest => IDC_SIZENESW,
            _ => IDC_ARROW,
        };
        unsafe { LoadCursorW(core::ptr::null_mut(), id as *const u16) }
    }

    /// The themed cursors win over winit's cache inside the client area.
    unsafe extern "system" fn wndproc(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize {
        if msg == WM_SETCURSOR && ACTIVE.load(Relaxed) && ((lparam as u32) & 0xffff) == HTCLIENT {
            let handle = ACTIVE_CURSOR.load(Relaxed);
            if handle != 0 {
                unsafe { SetCursor(handle as Hcursor) };
                return 1;
            }
        }
        let previous = ORIGINAL_WNDPROC.load(Relaxed);
        if previous == 0 {
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        } else {
            let proc: Wndproc = unsafe { std::mem::transmute(previous) };
            unsafe { CallWindowProcW(proc, hwnd, msg, wparam, lparam) }
        }
    }
}
