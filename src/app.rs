//! Application state, page chrome (background + sidebar) and persistence.
use crate::logic::Gender;
use crate::theme::{hex, rgba, Palette, Role, Theme};
use crate::ui::*;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, Ui,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Fbc,
    Retic,
    PtAptt,
    Hbel,
}

pub struct ParamDef {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub limit: f64,
    /// index into `Theme::param` for coloured cards (FBC only)
    pub color: Option<usize>,
}

pub const FBC_PARAMS: [ParamDef; 4] = [
    ParamDef { id: "wbc", name: "WBC", unit: "×10⁹/L", limit: 10.0, color: Some(0) },
    ParamDef { id: "rbc", name: "RBC", unit: "×10¹²/L", limit: 4.0, color: Some(1) },
    ParamDef { id: "hgb", name: "HGB", unit: "g/dL", limit: 4.0, color: Some(2) },
    ParamDef { id: "plt", name: "PLT", unit: "×10⁹/L", limit: 25.0, color: Some(3) },
];

pub const PT_PARAMS: [ParamDef; 2] = [
    ParamDef { id: "pt", name: "PT", unit: "seconds", limit: 5.0, color: None },
    ParamDef { id: "aptt", name: "APTT", unit: "seconds", limit: 5.0, color: None },
];

pub const DIL_PARAMS: [&str; 5] = ["WBC", "RBC", "HGB", "HCT", "PLT"];

pub struct Checker {
    pub key: &'static str,
    pub params: &'static [ParamDef],
    pub vals: Vec<[String; 3]>,
    pub third: bool,
}

impl Checker {
    fn new(key: &'static str, params: &'static [ParamDef]) -> Self {
        Checker { key, params, vals: vec![Default::default(); params.len()], third: false }
    }
}

pub const MIN_ATTEMPTS: usize = 3;
pub const MAX_ATTEMPTS: usize = 12;

pub struct App {
    pub palette: Palette,
    pub dark: bool,
    pub tab: Tab,
    pub fbc: Checker,
    pub pt: Checker,
    pub r_retic: String,
    pub r_hct: String,
    pub r_rbc: String,
    pub gender: Option<Gender>,
    pub attempts: Vec<String>,
    pub retic_note: Option<String>,
    pub calc_open: bool,
    /// Retic reference table unfolded (folded by default).
    pub ref_open: bool,
    pub dil: [String; 5],
    /// Theme popover (opened from the title bar's Theme button).
    pub menu_open: bool,
    pub menu_btn: Rect,
    pub hbel: crate::hbel_panel::HbelState,
    genshin_backdrop: egui::TextureHandle,
    pokemon_backdrop: egui::TextureHandle,
    styled: Option<(Palette, bool)>,
    pika: crate::pikachu::Pikachu,
    paimon: crate::paimon::Paimon,
    /// Theme mascots shown (Ctrl+H toggles; remembered across launches).
    pub mascots: bool,
    /// Cyberpunk theme revealed by typing "2077" (remembered across launches).
    pub cyberpunk_unlocked: bool,
    /// The last few characters typed, for spotting the unlock code.
    typed: String,
    /// Display scale (egui zoom factor; remembered across launches).
    pub scale: f32,
    /// Zoom factor last seen on the context, to notice Ctrl+Plus / Minus.
    seen_zoom: f32,
    /// Background check for a newer GitHub release.
    pub update: crate::update::Checker,
    /// "Update now?" confirmation shown (nothing happens until the user agrees).
    pub update_prompt: bool,
    /// Ctrl+Tab (+1) / Ctrl+Shift+Tab (−1) presses taken out of this frame's input.
    tab_steps: i32,
    /// The HBEL comments spreadsheet ("Open Excel"; remembered across launches).
    pub excel_path: Option<std::path::PathBuf>,
    pub excel_picker: crate::excel::Picker,
    /// Problem opening the spreadsheet, shown in the source sheet panel.
    pub excel_msg: Option<String>,
    /// Tab shown last frame (re-read the shared HBEL folder on entering HBEL).
    seen_tab: Tab,
    seen_tab_set: bool,
    /// Last window size requested by `fit_window`.
    fit_req: Option<egui::Vec2>,
    /// The page fits the window `fit_window` last asked for (no scrollbar needed).
    page_fits: bool,
    /// Default window size (every page at launch fits) for the theme it was
    /// measured in; re-measured when the theme changes.
    base: Option<((Palette, bool), egui::Vec2)>,
    /// Off-screen context used by `default_window`, kept so a theme change
    /// does not reparse every embedded font (a visible hitch otherwise).
    measure_ctx: std::cell::OnceCell<egui::Context>,
    #[cfg(feature = "screenshot")]
    shot: crate::app::Shot,
}

#[cfg(feature = "screenshot")]
#[derive(Default)]
pub struct Shot {
    path: Option<String>,
    frame: u32,
    scroll: f32,
    /// Frame on which to capture (HAEM_FRAMES, default 8).
    at: u32,
    pika_sheet: bool,
    paimon_sheet: bool,
    /// Simulated right-click, sent once Pikachu has landed (HAEM_SCENE=goal=X:Y).
    goal: Option<egui::Pos2>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::install(&cc.egui_ctx);
        let Settings { dark, palette, mascots, cyberpunk_unlocked, scale, excel } = Settings::load();
        #[allow(unused_mut)]
        let mut app = App::blank(palette, dark, mascots, cyberpunk_unlocked, genshin_backdrop_texture(&cc.egui_ctx), pokemon_backdrop_texture(&cc.egui_ctx));
        app.scale = scale;
        app.excel_path = excel;
        app.seen_zoom = cc.egui_ctx.zoom_factor();
        cc.egui_ctx.set_zoom_factor(scale);
        #[cfg(feature = "screenshot")]
        app.apply_scene();
        // Silent update check: only a newer release shows anything.
        #[cfg(feature = "screenshot")]
        let check = app.shot.path.is_none();
        #[cfg(not(feature = "screenshot"))]
        let check = true;
        if check {
            app.update.start(&cc.egui_ctx);
            crate::update::cleanup_old();
        }
        app
    }

    /// Every page empty and folded, as at launch.
    fn blank(palette: Palette, dark: bool, mascots: bool, cyberpunk_unlocked: bool, genshin_backdrop: egui::TextureHandle, pokemon_backdrop: egui::TextureHandle) -> Self {
        App {
            palette,
            dark,
            tab: Tab::Fbc,
            fbc: Checker::new("fbc", &FBC_PARAMS),
            pt: Checker::new("ptaptt", &PT_PARAMS),
            r_retic: String::new(),
            r_hct: String::new(),
            r_rbc: String::new(),
            gender: None,
            attempts: vec![String::new(); MIN_ATTEMPTS],
            retic_note: None,
            calc_open: false,
            ref_open: false,
            dil: Default::default(),
            menu_open: false,
            menu_btn: Rect::NOTHING,
            // the source sheet opens beside the results by default
            hbel: {
                let mut h = crate::hbel_panel::HbelState::default();
                h.source_open = true;
                h
            },
            genshin_backdrop,
            pokemon_backdrop,
            styled: None,
            pika: crate::pikachu::Pikachu::new(),
            paimon: crate::paimon::Paimon::new(),
            mascots,
            cyberpunk_unlocked,
            typed: String::new(),
            scale: 1.0,
            seen_zoom: 1.0,
            update: Default::default(),
            update_prompt: false,
            tab_steps: 0,
            excel_path: None,
            excel_picker: Default::default(),
            excel_msg: None,
            seen_tab: Tab::Fbc,
            seen_tab_set: false,
            fit_req: None,
            page_fits: true,
            base: None,
            measure_ctx: std::cell::OnceCell::new(),
            #[cfg(feature = "screenshot")]
            shot: Default::default(),
        }
    }

    /// The default window: big enough for every page as it is at launch
    /// (empty, Run 3 off, reference folded, calculations hidden) in this
    /// theme, plus the margin and title bar. In practice the width comes from
    /// Retic and the height from FBC with its dilution sheet. Laid out in a
    /// throwaway egui context, so nothing on screen (fields, mascot perches,
    /// focus) sees it. The first pass only loads the fonts.
    fn default_window(&self) -> egui::Vec2 {
        let ctx = self
            .measure_ctx
            .get_or_init(|| {
                let ctx = egui::Context::default();
                crate::fonts::install(&ctx);
                ctx
            })
            .clone();
        // Start from clean widget state, as a fresh context would (and so the
        // perch / field lists the panels append to cannot pile up).
        ctx.memory_mut(|m| m.data.clear());
        let mut tmp = App::blank(self.palette, self.dark, false, self.cyberpunk_unlocked, self.genshin_backdrop.clone(), self.pokemon_backdrop.clone());
        let th = tmp.theme();
        let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(4000.0, 4000.0))), ..Default::default() };
        let mut size = vec2(0.0, 0.0);
        // The last pass is Retic with its reference table unfolded: the
        // default height shows all of it. It does not set the width.
        let passes = [(Tab::Retic, false), (Tab::Retic, false), (Tab::Fbc, false), (Tab::PtAptt, false), (Tab::Retic, true)];
        for (pass, (tab, ref_open)) in passes.into_iter().enumerate() {
            tmp.tab = tab;
            tmp.ref_open = ref_open;
            let _ = ctx.run(raw.clone(), |ctx| {
                tmp.apply_style(ctx, &th);
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                    let w = tmp.page_width(ui, &th).max(crate::chrome::min_width(ui, &th) - 2.0 * PAGE_MARGIN).ceil();
                    let o = pos2(0.0, 0.0);
                    let bottom = match tab {
                        Tab::Retic => tmp.retic_panel(ui, &th, o, w),
                        Tab::Fbc => tmp.checker_panel(ui, &th, o, w, true),
                        Tab::PtAptt => tmp.checker_panel(ui, &th, o, w, false),
                        Tab::Hbel => tmp.hbel_panel(ui, &th, o, w),
                    };
                    if pass > 0 {
                        let page_w = if ref_open { 0.0 } else { w + 2.0 * PAGE_MARGIN };
                        size = size.max(vec2(page_w, crate::chrome::CHROME_H + bottom + 2.0 * PAGE_MARGIN));
                    }
                });
            });
        }
        vec2(size.x.ceil(), size.y.ceil())
    }

    pub fn theme(&self) -> Theme {
        Theme::new(self.palette, self.dark)
    }

    fn apply_style(&mut self, ctx: &egui::Context, th: &Theme) {
        if self.styled == Some((th.palette, th.dark)) {
            return;
        }
        self.styled = Some((th.palette, th.dark));
        let mut style = (*ctx.style()).clone();
        style.visuals = if th.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        let v = &mut style.visuals;
        v.panel_fill = th.bg;
        v.window_fill = th.surface;
        v.override_text_color = None;
        v.selection.bg_fill = rgba(th.ring, 0.35);
        v.selection.stroke = Stroke::new(1.0_f32, th.text);
        v.text_cursor.stroke = Stroke::new(1.5_f32, th.text);
        v.widgets.inactive.bg_stroke = Stroke::NONE;
        v.widgets.noninteractive.bg_stroke = Stroke::NONE;
        v.widgets.hovered.bg_stroke = Stroke::NONE;
        v.widgets.active.bg_stroke = Stroke::NONE;
        // scroll bar tinted like the page
        v.widgets.inactive.bg_fill = th.line_strong;
        v.widgets.hovered.bg_fill = th.muted;
        v.widgets.active.bg_fill = th.muted;
        v.extreme_bg_color = th.surface2;
        style.spacing.scroll = egui::style::ScrollStyle::solid();
        style.spacing.scroll.bar_width = 10.0;
        style.spacing.scroll.floating = false;
        style.spacing.item_spacing = vec2(8.0, 0.0);
        style.animation_time = 0.12;
        ctx.set_style(style);
    }

    pub fn set_palette(&mut self, p: Palette) {
        if self.palette == p {
            return;
        }
        self.palette = p;
        // Genshin opens in dark mode, Pokémon in light mode (still toggleable afterwards)
        match p {
            Palette::Genshin => self.dark = true,
            Palette::Pokemon => self.dark = false,
            Palette::Cyberpunk => self.dark = true,
            Palette::Default => {}
        }
        self.save();
    }

    pub fn toggle_dark(&mut self) {
        self.dark = !self.dark;
        self.save();
    }

    /// Show or hide Pikachu and Paimon. When shown again they make a fresh
    /// entrance (Pikachu drops in, Paimon teleports in).
    pub fn toggle_mascots(&mut self) {
        self.mascots = !self.mascots;
        self.save();
    }

    /// Set the display scale (clamped, snapped to whole percent). Takes effect
    /// next frame; the window is re-fitted at the new scale.
    pub fn set_scale(&mut self, ctx: &egui::Context, s: f32) {
        let s = ((s.clamp(MIN_SCALE, MAX_SCALE)) * 100.0).round() / 100.0;
        if (s - self.scale).abs() < 0.001 {
            return;
        }
        self.scale = s;
        ctx.set_zoom_factor(s);
        self.fit_req = None;
        self.save();
    }

    /// One step up or down the scale menu's list.
    pub fn step_scale(&mut self, ctx: &egui::Context, up: bool) {
        let cur = self.scale;
        let next = if up {
            SCALES.iter().copied().find(|s| *s > cur + 0.001).unwrap_or(MAX_SCALE)
        } else {
            SCALES.iter().rev().copied().find(|s| *s < cur - 0.001).unwrap_or(MIN_SCALE)
        };
        self.set_scale(ctx, next);
    }

    /// Adopt a zoom changed outside the menu (Ctrl+Plus / Minus / 0) and
    /// re-fit the window whenever the zoom changes.
    fn sync_zoom(&mut self, ctx: &egui::Context) {
        let z = ctx.zoom_factor();
        if (z - self.seen_zoom).abs() < 0.001 {
            return;
        }
        self.seen_zoom = z;
        self.fit_req = None;
        if (z - self.scale).abs() >= 0.001 {
            self.set_scale(ctx, z);
        }
    }

    /// "Open Excel": open the chosen spreadsheet, or ask for it first (also
    /// when the remembered file has moved).
    pub fn open_excel(&mut self, ctx: &egui::Context) {
        self.excel_msg = None;
        match self.excel_path.clone() {
            Some(p) if p.is_file() => {
                if let Err(e) = crate::excel::open(&p) {
                    self.excel_msg = Some(e);
                }
            }
            Some(p) => {
                self.excel_msg = Some(format!("The Excel file is no longer at {}. Choose where it is now.", p.display()));
                self.excel_picker.start(ctx, Some(&p), true);
            }
            None => self.excel_picker.start(ctx, None, true),
        }
    }

    /// Choose (or change) the spreadsheet without opening it.
    pub fn choose_excel(&mut self, ctx: &egui::Context) {
        self.excel_msg = None;
        self.excel_picker.start(ctx, self.excel_path.as_deref(), false);
    }

    /// Take the file dialog's answer once it closes.
    fn poll_excel(&mut self) {
        let Some(picked) = self.excel_picker.poll() else { return };
        let Some(path) = picked else { return };
        self.excel_path = Some(path.clone());
        self.save();
        if self.excel_picker.open_after {
            self.excel_msg = crate::excel::open(&path).err();
        } else {
            self.excel_msg = None;
        }
    }

    /// Ctrl+Tab / Ctrl+Shift+Tab: next / previous tab, wrapping at the ends.
    fn cycle_tab(&mut self, ctx: &egui::Context, steps: i32) {
        let order = crate::chrome::TABS;
        let n = order.len() as i32;
        let i = order.iter().position(|(t, _)| *t == self.tab).unwrap_or(0) as i32;
        self.tab = order[(i + steps).rem_euclid(n) as usize].0;
        self.menu_open = false;
        // the focused field belongs to the page being left
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
    }

    /// Typing "2077" anywhere reveals the Cyberpunk theme in the menu; typing
    /// it again hides it (and leaves Cyberpunk if it was the active theme).
    pub fn toggle_cyberpunk_unlock(&mut self) {
        self.cyberpunk_unlocked = !self.cyberpunk_unlocked;
        if !self.cyberpunk_unlocked && self.palette == Palette::Cyberpunk {
            self.palette = Palette::Default;
        }
        self.save();
    }

    /// Watch typed text for the unlock code. Characters still reach whatever
    /// field has focus; only the last few are remembered here.
    fn watch_unlock_code(&mut self, ctx: &egui::Context) {
        let typed: String = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Text(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect()
        });
        for _ in 0..feed_unlock_code(&mut self.typed, &typed) {
            self.toggle_cyberpunk_unlock();
        }
    }

    fn save(&self) {
        Settings { dark: self.dark, palette: self.palette, mascots: self.mascots, cyberpunk_unlocked: self.cyberpunk_unlocked, scale: self.scale, excel: self.excel_path.clone() }.save();
    }

    /// Page width inside the default window (before the margin), once measured.
    pub fn default_page_width(&self) -> Option<f32> {
        self.base.map(|(_, b)| b.x - 2.0 * PAGE_MARGIN)
    }

    /// The window keeps one size (`default_window`) whichever tab is shown;
    /// pages that need more room scroll. The exception is expanded content
    /// (Run 3 on FBC or PT / APTT; the reference table or calculations on
    /// Retic), which grows the window just enough to show it, and shrinks it
    /// back when folded away again. Never
    /// while maximized; only re-requested when the wanted size changes.
    fn fit_window(&mut self, ctx: &egui::Context, content_w: f32, content_h: f32) {
        let key = (self.palette, self.dark);
        if self.base.map(|(k, _)| k) != Some(key) {
            self.base = Some((key, self.default_window()));
        }
        let base = self.base.map(|(_, b)| b).unwrap_or(vec2(860.0, 560.0));
        let (minimized, maximized, monitor, outer) = ctx.input(|i| {
            let v = i.viewport();
            (v.minimized.unwrap_or(false), v.maximized.unwrap_or(false), v.monitor_size, v.outer_rect)
        });
        if maximized {
            // re-fit on restore, in case the page changed meanwhile
            self.fit_req = None;
            return;
        }
        if minimized {
            return;
        }
        let expanded = match self.tab {
            Tab::Fbc => self.fbc.third,
            Tab::PtAptt => self.pt.third,
            Tab::Retic => self.ref_open || self.calc_open,
            // HBEL fills whatever width the window has and scrolls.
            Tab::Hbel => false,
        };
        // a roomier window than the tightest fit, with more background around the page
        // Height: Retic with its reference unfolded (measured in `default_window`).
        let (mut w, mut h) = ((base.x * WINDOW_GROW).ceil(), base.y.ceil());
        let need_h = crate::chrome::CHROME_H + content_h + 2.0 * PAGE_MARGIN;
        if expanded {
            w = w.max(content_w + 2.0 * PAGE_MARGIN);
            h = h.max(need_h);
        }
        if let Some(mon) = monitor {
            // keep clear of the taskbar
            let max_h = (mon.y - TASKBAR_ALLOWANCE).max(300.0);
            if h > max_h {
                h = max_h;
                if expanded && need_h > h {
                    let s = &ctx.style().spacing.scroll;
                    w += s.bar_inner_margin + s.bar_width + s.bar_outer_margin;
                }
            }
            w = w.min(mon.x);
        }
        let want = vec2(w.ceil(), h.ceil());
        // Will the whole page fit once the window has this size? Then no
        // scrollbar is needed, even in the frame or two before it arrives.
        self.page_fits = want.y + 0.5 >= need_h;
        if self.fit_req == Some(want) {
            return;
        }
        self.fit_req = Some(want);

        // If growing would push the window off the bottom or right of the
        // screen, slide it back on first, so it never pokes off-screen and
        // then jumps. Only for a window on the primary monitor, whose
        // coordinates start at the origin.
        if let (Some(mon), Some(o)) = (monitor, outer) {
            if o.left() >= 0.0 && o.top() >= 0.0 && o.left() < mon.x && o.top() < mon.y {
                let chrome_extra = o.size() - ctx.screen_rect().size();
                let full = want + chrome_extra.max(vec2(0.0, 0.0));
                let x = o.left().min((mon.x - full.x).max(0.0));
                let y = o.top().min((mon.y - TASKBAR_ALLOWANCE - full.y).max(0.0));
                if (x - o.left()).abs() > 0.5 || (y - o.top()).abs() > 0.5 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos2(x, y)));
                }
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(want));
    }
}

/// Background left showing around the page on every side.
pub const PAGE_MARGIN: f32 = 24.0;
/// Narrowest side margin, when a scrollbar has taken some of it.
const MIN_SIDE: f32 = 8.0;
/// Room left at the bottom of the screen for the taskbar.
const TASKBAR_ALLOWANCE: f32 = 56.0;
/// The default window is this much wider than the tightest fit of every page.
const WINDOW_GROW: f32 = 1.3;

/// Display scale steps offered in the Theme menu (Ctrl+Plus / Minus also work).
pub const SCALES: [f32; 8] = [0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];
pub const MIN_SCALE: f32 = 0.5;
pub const MAX_SCALE: f32 = 3.0;

/// Typed anywhere in the app to show / hide the Cyberpunk theme.
const UNLOCK_CODE: &str = "2077";

/// Append typed text to the rolling buffer; returns how many times the unlock
/// code was completed. The buffer is cleared on a match, so "20772077" counts
/// twice but the overlapping tail of one code never starts the next.
fn feed_unlock_code(buf: &mut String, typed: &str) -> usize {
    let mut hits = 0;
    for c in typed.chars() {
        buf.push(c);
        if buf.chars().count() > UNLOCK_CODE.len() {
            buf.remove(0);
        }
        if buf == UNLOCK_CODE {
            buf.clear();
            hits += 1;
        }
    }
    hits
}

// ---------------------------------------------------------------------------
// Persistence: %APPDATA%\HaematologyCalculator\settings.txt
// (theme=… / palette=… / mascots=shown|hidden / cyberpunk=unlocked|locked / scale=1.25)
// ---------------------------------------------------------------------------

struct Settings {
    dark: bool,
    palette: Palette,
    mascots: bool,
    cyberpunk_unlocked: bool,
    scale: f32,
    excel: Option<std::path::PathBuf>,
}

fn settings_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    Some(base.join("HaematologyCalculator").join("settings.txt"))
}

impl Settings {
    fn load() -> Settings {
        match settings_path().map(std::fs::read_to_string) {
            Some(Ok(text)) => Settings::parse(&text),
            _ => Settings::parse(""),
        }
    }

    fn parse(text: &str) -> Settings {
        let mut s = Settings { dark: false, palette: Palette::Default, mascots: true, cyberpunk_unlocked: false, scale: 1.0, excel: None };
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("theme=") {
                s.dark = v.trim() == "dark";
            } else if let Some(v) = line.strip_prefix("palette=") {
                s.palette = Palette::from_key(v.trim());
            } else if let Some(v) = line.strip_prefix("mascots=") {
                s.mascots = v.trim() != "hidden";
            } else if let Some(v) = line.strip_prefix("cyberpunk=") {
                s.cyberpunk_unlocked = v.trim() == "unlocked";
            } else if let Some(v) = line.strip_prefix("hbel_excel=") {
                let v = v.trim();
                s.excel = (!v.is_empty()).then(|| std::path::PathBuf::from(v));
            } else if let Some(v) = line.strip_prefix("scale=") {
                if let Ok(x) = v.trim().parse::<f32>() {
                    if x.is_finite() {
                        s.scale = x.clamp(MIN_SCALE, MAX_SCALE);
                    }
                }
            }
        }
        // Cyberpunk is a hidden theme: without the unlock it falls back to Default.
        if s.palette == Palette::Cyberpunk && !s.cyberpunk_unlocked {
            s.palette = Palette::Default;
        }
        s
    }

    fn save(&self) {
        if let Some(path) = settings_path() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(
                path,
                format!(
                    "theme={}\npalette={}\nmascots={}\ncyberpunk={}\nscale={:.2}\nhbel_excel={}\n",
                    if self.dark { "dark" } else { "light" },
                    self.palette.key(),
                    if self.mascots { "shown" } else { "hidden" },
                    if self.cyberpunk_unlocked { "unlocked" } else { "locked" },
                    self.scale,
                    // one line: a path never holds a newline on Windows
                    self.excel.as_ref().map(|p| p.display().to_string().replace(['\r', '\n'], "")).unwrap_or_default()
                ),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

impl eframe::App for App {
    /// Ctrl+Tab / Ctrl+Shift+Tab are taken out before egui sees the input, so
    /// they switch tabs without also moving focus between fields.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        let mut steps = 0;
        raw.events.retain(|e| match e {
            egui::Event::Key { key: egui::Key::Tab, pressed, modifiers, .. } if modifiers.ctrl || modifiers.command => {
                if *pressed {
                    steps += if modifiers.shift { -1 } else { 1 };
                }
                false
            }
            _ => true,
        });
        self.tab_steps += steps;
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        crate::cursor::install(frame);
        self.sync_zoom(ctx);
        self.poll_excel();
        let steps = std::mem::take(&mut self.tab_steps);
        if steps != 0 {
            self.cycle_tab(ctx, steps);
        }
        // Coming back to HBEL re-reads the shared folder (comments added on
        // other PCs), in place of the old Reload button.
        if self.tab == Tab::Hbel && self.seen_tab != Tab::Hbel && self.seen_tab_set {
            self.hbel.reload();
        }
        self.seen_tab = self.tab;
        self.seen_tab_set = true;

        // Ctrl+H hides / shows the theme mascots. Consumed here so a focused
        // text field never sees it.
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::H)) {
            self.toggle_mascots();
        }
        self.watch_unlock_code(ctx);

        let th = self.theme();
        self.apply_style(ctx, &th);

        // Field navigation state: rebuilt every frame from the fields actually drawn.
        ctx.data_mut(|d| d.insert_temp(Id::new("haem-field-order"), Vec::<(Id, Rect)>::new()));
        crate::pikachu::begin_frame(ctx);
        let focus_before = ctx.memory(|m| m.focused());

        crate::chrome::title_bar(self, ctx);

        let mut fit: Option<(f32, f32)> = None;
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(th.bg)).show(ctx, |ui| {
            let screen = ui.max_rect();
            paint_page_background(ui, &th, screen, &self.genshin_backdrop, &self.pokemon_backdrop);

            // HBEL: two columns on one continuous page background (no side
            // panel, so nothing interrupts the tab where it opens into the page).
            if self.tab == Tab::Hbel {
                let h = self.hbel_page(ui, &th, screen, focus_before);
                fit = Some((0.0, h));
                crate::pikachu::perch(ui, Id::new("pika-floor"), screen, crate::pikachu::PerchKind::Floor, false);
                return;
            }

            // The page is laid out at its natural width and the window is
            // sized around it (see `fit_window`), leaving the same band of
            // background on every side.
            let natural = self.page_width(ui, &th).max(crate::chrome::min_width(ui, &th) - 2.0 * PAGE_MARGIN).ceil();

            #[allow(unused_mut)]
            let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]).drag_to_scroll(false);
            if self.page_fits {
                // The window is about to grow to fit the page: a scrollbar
                // flashing in for the frames in between would shift the page.
                area = area.scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
            }
            #[cfg(feature = "screenshot")]
            if self.shot.path.is_some() {
                // Only while rendering a screenshot: pinning the offset every
                // frame otherwise stops the page from scrolling at all.
                area = area.vertical_scroll_offset(self.shot.scroll);
            }
            area.show(ui, |ui| {
                let view = ui.max_rect();
                // A scrollbar eats into the side margin before it squeezes the page.
                // Centred on the whole window, not the area beside a
                // scrollbar, so the page stays put when a scrollbar comes or goes.
                let content_w = natural.min(view.width() - 2.0 * MIN_SIDE).max(300.0);
                let left = (screen.left() + ((screen.width() - content_w) / 2.0).max(MIN_SIDE)).min(view.right() - MIN_SIDE - content_w).floor();
                let origin = pos2(left, ui.cursor().min.y + PAGE_MARGIN);
                let bottom = match self.tab {
                    Tab::Fbc => self.checker_panel(ui, &th, origin, content_w, true),
                    Tab::PtAptt => self.checker_panel(ui, &th, origin, content_w, false),
                    Tab::Retic => self.retic_panel(ui, &th, origin, content_w),
                    Tab::Hbel => self.hbel_panel(ui, &th, origin, content_w),
                };
                let top = ui.cursor().min;
                ui.allocate_rect(Rect::from_min_max(top, pos2(ui.max_rect().right(), bottom + PAGE_MARGIN)), Sense::hover());
                fit = Some((natural, bottom - origin.y));
                nav_fields(ui, focus_before);
            });

            crate::pikachu::perch(ui, Id::new("pika-floor"), screen, crate::pikachu::PerchKind::Floor, false);
        });
        if let Some((w, h)) = fit {
            self.fit_window(ctx, w, h);
        }

        if self.menu_open {
            self.theme_menu(ctx);
        }
        if self.update_prompt {
            self.update_dialog(ctx);
        }
        // developer test: HAEM_AUTOUPDATE=1 approves an available update unasked
        #[cfg(feature = "screenshot")]
        if std::env::var("HAEM_AUTOUPDATE").is_ok_and(|v| v == "1") && self.update.install_status() == crate::update::Install::Idle {
            if let crate::update::Status::Available { asset: Some(a), .. } = self.update.status() {
                self.update.begin_install(ctx, a);
                self.update_prompt = true;
            }
        }
        // an approved update is in place and the new version has started
        if self.update.install_status() == crate::update::Install::Restarting {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Theme mascots: roam the perches registered above.
        self.pika.tick(ctx, &th, self.mascots && self.palette == Palette::Pokemon);
        self.paimon.tick(ctx, &th, self.mascots && self.palette == Palette::Genshin);
        #[cfg(feature = "screenshot")]
        {
            if self.shot.pika_sheet {
                crate::pikachu::debug_sheet(ctx);
            }
            if self.shot.paimon_sheet {
                crate::paimon::debug_sheet(ctx);
            }
            if self.shot.frame == 90 {
                if let Some(g) = self.shot.goal {
                    self.pika.debug_goal(g);
                    if self.palette == Palette::Genshin {
                        self.paimon.debug_goal(ctx, g);
                    }
                }
            }
        }

        // Hand this frame's requested cursor to the palette's themed pointer set
        // (or back to the stock cursor when the palette has none).
        let icon = ctx.output(|o| o.cursor_icon);
        let set = match self.palette {
            Palette::Genshin => Some(crate::cursor::Set::Genshin),
            Palette::Pokemon => Some(crate::cursor::Set::Pokemon),
            _ => None,
        };
        crate::cursor::apply(ctx, set, icon);

        #[cfg(feature = "screenshot")]
        self.screenshot_tick(ctx);
    }
}

fn paint_page_background(
    ui: &Ui,
    th: &Theme,
    screen: Rect,
    genshin: &egui::TextureHandle,
    pokemon: &egui::TextureHandle,
) {
    let p = ui.painter();
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    match th.palette {
        Palette::Genshin => {
            // A soft, out-of-focus world scene behind a tinted veil, mirroring
            // the blurred menu backgrounds in the Genshin UI reference.
            p.add(egui::Shape::image(genshin.id(), screen, uv, Color32::WHITE));
            p.rect_filled(screen, 0.0, rgba(th.bg, if th.dark { 0.46 } else { 0.68 }));

            let c = pos2(screen.left() + screen.width() * 0.82, screen.top() - 0.08 * 480.0);
            radial(p, c, 900.0, 480.0, rgba(th.gold, if th.dark { 0.16 } else { 0.12 }));
            let cool = pos2(screen.left() + screen.width() * 0.14, screen.bottom() + 90.0);
            radial(p, cool, 680.0, 380.0, rgba(th.ring, if th.dark { 0.10 } else { 0.05 }));
        }
        Palette::Pokemon => {
            // Sunlit meadow behind a creamy (or moonlit) wash, with the soft
            // coloured bokeh of a Let's Go menu.
            p.add(egui::Shape::image(pokemon.id(), screen, uv, Color32::WHITE));
            let veil = if th.dark { rgba(th.bg, 0.58) } else { rgba(hex(0xfff8e4), 0.42) };
            p.rect_filled(screen, 0.0, veil);

            let warm = pos2(screen.right() - screen.width() * 0.10, screen.top() + screen.height() * 0.10);
            radial(p, warm, 760.0, 430.0, rgba(th.gold, if th.dark { 0.10 } else { 0.16 }));
            let coral = pos2(screen.left() + screen.width() * 0.10, screen.bottom() - screen.height() * 0.04);
            radial(p, coral, 640.0, 360.0, rgba(th.accent, if th.dark { 0.06 } else { 0.08 }));
            let mint = pos2(screen.left() + screen.width() * 0.76, screen.bottom() + 60.0);
            radial(p, mint, 560.0, 320.0, rgba(hex(0x9bdc7a), if th.dark { 0.06 } else { 0.22 }));
            let sky = pos2(screen.left() + screen.width() * 0.18, screen.top() + screen.height() * 0.16);
            radial(p, sky, 520.0, 300.0, rgba(hex(0x8ed8ef), if th.dark { 0.05 } else { 0.12 }));
        }
        Palette::Cyberpunk => {
            // The menu backdrop: near-black with a deep red glow pooling at
            // the top and bottom edges, and a fine red grid with scanlines.
            let red = hex(0x8a1420);
            let w = screen.width();
            let h = screen.height();
            radial(p, pos2(screen.left() + w * 0.5, screen.bottom() + h * 0.10), w * 0.85, h * 0.62, rgba(red, if th.dark { 0.42 } else { 0.035 }));
            radial(p, pos2(screen.left() + w * 0.16, screen.top() - h * 0.05), w * 0.55, h * 0.45, rgba(red, if th.dark { 0.28 } else { 0.025 }));
            radial(p, pos2(screen.right() + w * 0.02, screen.top() + h * 0.30), w * 0.40, h * 0.50, rgba(red, if th.dark { 0.20 } else { 0.02 }));

            let grid = Stroke::new(1.0_f32, rgba(th.accent, if th.dark { 0.035 } else { 0.05 }));
            let mut x = screen.left() + 24.0;
            while x < screen.right() {
                p.line_segment([pos2(x, screen.top()), pos2(x, screen.bottom())], grid);
                x += 48.0;
            }
            let mut gy = screen.top() + 24.0;
            while gy < screen.bottom() {
                p.line_segment([pos2(screen.left(), gy), pos2(screen.right(), gy)], grid);
                gy += 48.0;
            }

            // Faint horizontal scanlines keep the scene reading as a terminal.
            let step = if th.dark { 3.0 } else { 4.0 };
            let line = Stroke::new(1.0_f32, if th.dark { Color32::from_black_alpha(40) } else { Color32::from_black_alpha(10) });
            let mut y = screen.top();
            while y < screen.bottom() {
                p.line_segment([pos2(screen.left(), y), pos2(screen.right(), y)], line);
                y += step;
            }
        }
        Palette::Default => {}
    }
}

/// Tab / Enter move to the next field, Shift+Tab / Shift+Enter to the previous one.
/// Fields are walked in the order they were drawn (which matches reading order) and wrap
/// at the ends. Works whether or not a field currently has focus.
pub(crate) fn nav_fields(ui: &Ui, focus_before: Option<Id>) {
    let back = ui.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key { key, pressed: true, modifiers, .. }
                if matches!(key, egui::Key::Tab | egui::Key::Enter) =>
            {
                Some(modifiers.shift)
            }
            _ => None,
        })
    });
    let Some(back) = back else { return };

    let fields: Vec<(Id, Rect)> = ui
        .ctx()
        .data(|d| d.get_temp(Id::new("haem-field-order")))
        .unwrap_or_default();
    if fields.is_empty() {
        return;
    }
    let current = focus_before.and_then(|f| fields.iter().position(|(id, _)| *id == f));
    let target = match current {
        Some(i) if back => (i + fields.len() - 1) % fields.len(),
        Some(i) => (i + 1) % fields.len(),
        None if back => fields.len() - 1,
        None => 0,
    };
    let (id, rect) = fields[target];
    ui.ctx().memory_mut(|m| m.request_focus(id));
    if !ui.clip_rect().contains_rect(rect) {
        ui.scroll_to_rect(rect.expand(8.0), None);
    }
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

const MENU_W: f32 = 184.0;
const MENU_ROW: f32 = 28.0;
const MENU_GAP: f32 = 4.0;
const MENU_SPLIT: f32 = 9.0;
/// Height of the palette swatches.
const SWATCH_H: f32 = 30.0;
const SWATCH_GAP: f32 = 8.0;

fn menu_content_h() -> f32 {
    // "Theme" row with the light / dark switch, the swatch grid, a divider,
    // then the mascots, display scale, Excel file and update rows
    MENU_ROW + 6.0 + SWATCH_H + 2.0 * MENU_SPLIT + 4.0 * MENU_ROW + 3.0 * MENU_GAP
}

/// What a palette's swatch shows: the dark mode it opens in.
fn swatch_dark(p: Palette, dark: bool) -> bool {
    match p {
        Palette::Genshin | Palette::Cyberpunk => true,
        Palette::Pokemon => false,
        Palette::Default => dark,
    }
}

/// Corner radius / chamfer of a palette's swatch tile.
fn swatch_radius(p: Palette) -> f32 {
    match p {
        Palette::Default => 6.0,
        Palette::Genshin => 3.0,
        Palette::Pokemon => 11.0,
        Palette::Cyberpunk => 7.0,
    }
}

/// Outline following a swatch tile's shape (selection / hover ring).
fn swatch_outline(p: &egui::Painter, pal: Palette, r: Rect, w: f32, c: Color32) {
    if pal == Palette::Cyberpunk {
        cyber_stroke(p, r, swatch_radius(pal), w, c);
    } else {
        p.rect_stroke(r, swatch_radius(pal), Stroke::new(w, c), egui::StrokeKind::Inside);
    }
}

/// A palette's swatch: a tiny scene in that theme's own materials.
fn swatch(p: &egui::Painter, pal: Palette, pt: &Theme, r: Rect) {
    let rad = swatch_radius(pal);
    let (w, h) = (r.width(), r.height());
    let c = r.center();
    match pal {
        // a lab sheet: paper card on the page, clay tab, two lines of text
        Palette::Default => {
            fill(p, r, rad, pt.bg);
            stroke_in(p, r, rad, 1.0, pt.line_strong);
            let card = Rect::from_min_max(pos2(r.left() + w * 0.18, r.top() + 6.0), pos2(r.right() - w * 0.18, r.bottom() + 0.0));
            p.rect_filled(card, egui::CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, pt.surface);
            p.rect_stroke(card, egui::CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, Stroke::new(1.0_f32, pt.line_strong), egui::StrokeKind::Inside);
            let x0 = card.left() + 4.0;
            fill(p, Rect::from_min_size(pos2(x0, card.top() + 4.0), vec2((card.width() * 0.42).max(8.0), 4.0)), 2.0, pt.accent);
            let line = Stroke::new(1.3_f32, rgba(pt.text, 0.45));
            p.line_segment([pos2(x0, card.top() + 12.0), pos2(card.right() - 4.0, card.top() + 12.0)], line);
            p.line_segment([pos2(x0, card.top() + 16.5), pos2(x0 + (card.width() - 8.0) * 0.62, card.top() + 16.5)], line);
        }
        // a midnight slate tablet: champagne corner brackets around a gold sparkle
        Palette::Genshin => {
            let gold = hex(0xd3bc8e);
            fill(p, r, rad, hex(0x1d2230));
            radial(p, c, w * 0.55, h * 0.6, rgba(gold, 0.28));
            let b = r.shrink(3.0);
            let st = Stroke::new(1.2_f32, gold);
            let l = 4.5;
            for (corner, dx, dy) in [(b.left_top(), 1.0, 1.0), (b.right_top(), -1.0, 1.0), (b.left_bottom(), 1.0, -1.0), (b.right_bottom(), -1.0, -1.0)] {
                p.line_segment([corner, corner + vec2(l * dx, 0.0)], st);
                p.line_segment([corner, corner + vec2(0.0, l * dy)], st);
            }
            let mut m = egui::epaint::Mesh::default();
            m.colored_vertex(c, gold);
            for i in 0..=8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_2;
                // long vertical points, shorter side points, a narrow waist
                let rr = match i % 4 {
                    0 => h * 0.36,
                    2 => h * 0.24,
                    _ => h * 0.09,
                };
                m.colored_vertex(c + vec2(a.cos(), a.sin()) * rr, gold);
            }
            for i in 0..8u32 {
                m.add_triangle(0, i + 1, i + 2);
            }
            p.add(egui::Shape::mesh(m));
            p.circle_filled(c, 1.1, hex(0xece5d8));
        }
        // a sunny meadow inside the thick milky rim, with a lightning bolt
        Palette::Pokemon => {
            // the milky rim is the tile itself; the meadow sits inside it, so
            // nothing can spill past the rounded edge
            fill(p, r, rad, Color32::from_white_alpha(240));
            let inner = r.shrink(2.5);
            let ir = rad - 2.5;
            fill(p, inner, ir, hex(0xbfe6f4));
            let band = inner.top() + inner.height() * 0.62;
            let hill = p.with_clip_rect(Rect::from_min_max(pos2(inner.left() + 1.0, inner.top()), pos2(inner.right() - 1.0, band + 0.5)));
            hill.circle_filled(pos2(inner.left() + w * 0.28, band + h * 0.32), h * 0.5, hex(0x8fd16a));
            hill.circle_filled(pos2(inner.right() - w * 0.12, band + h * 0.36), h * 0.42, hex(0x7cc35a));
            p.rect_filled(Rect::from_min_max(pos2(inner.left(), band), inner.max), egui::CornerRadius { nw: 0, ne: 0, sw: ir as u8, se: ir as u8 }, hex(0x8fd16a));
            // bolt: two blades plus the joint, outlined in ink
            let k = h / 26.0;
            let o = c + vec2(w * 0.12, -0.5);
            let pt_ = |x: f32, y: f32| o + vec2(x, y) * k;
            let pts = [pt_(1.0, -7.0), pt_(-3.5, 0.8), pt_(0.0, 0.8), pt_(-1.2, 7.0), pt_(3.5, -0.8), pt_(0.0, -0.8)];
            let yellow = hex(0xf6cf3d);
            let mut m = egui::epaint::Mesh::default();
            for q in pts {
                m.colored_vertex(q, yellow);
            }
            for t in [[0u32, 1, 2], [0, 2, 5], [5, 2, 3], [5, 3, 4]] {
                m.add_triangle(t[0], t[1], t[2]);
            }
            p.add(egui::Shape::mesh(m));
            p.add(egui::Shape::closed_line(pts.to_vec(), Stroke::new(1.0_f32, hex(0x2c3b4a))));
            stroke_in(p, inner, ir, 1.0, rgba(hex(0x1b8580), 0.40));
            stroke_in(p, r, rad, 1.0, rgba(hex(0x2c3b4a), 0.12));
        }
        // a black chamfered chip: red scanlines, an offset cyan / red glitch bar
        Palette::Cyberpunk => {
            let red = hex(0xff6158);
            let cyan = hex(0x5ef6ff);
            cyber_fill(p, r, rad, hex(0x0a090e));
            let cut = rad.min(w * 0.30).min(h * 0.40);
            let mut y = r.top() + 2.5;
            while y < r.bottom() - 1.0 {
                let x0 = r.left() + (cut - (r.bottom() - y)).max(0.0) + 1.0;
                let x1 = r.right() - (cut - (y - r.top())).max(0.0) - 1.0;
                p.line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(1.0_f32, rgba(red, 0.16)));
                y += 3.0;
            }
            p.rect_filled(Rect::from_min_size(pos2(r.left() + w * 0.16, c.y - 3.0), vec2(w * 0.52, 3.0)), 0.0, cyan);
            p.rect_filled(Rect::from_min_size(pos2(r.left() + w * 0.30, c.y + 2.0), vec2(w * 0.40, 2.0)), 0.0, red);
            p.rect_filled(Rect::from_min_size(pos2(r.left() + w * 0.72, c.y - 3.0), vec2(w * 0.08, 3.0)), 0.0, rgba(cyan, 0.45));
            cyber_stroke(p, r, rad, 1.0, rgba(red, 0.75));
            p.line_segment([pos2(r.left() + 1.0, r.top() + 1.0), pos2(r.left() + w * 0.35, r.top() + 1.0)], Stroke::new(2.0_f32, red));
        }
    }
}

/// A small on / off switch (track `sw`, knob position `on` from 0 to 1).
fn mini_switch(p: &egui::Painter, th: &Theme, sw: Rect, on: f32) {
    let track = match th.palette {
        Palette::Genshin => crate::theme::mix(on, hex(0xd3bc8e), hex(0x646d84)),
        _ => crate::theme::mix(on, th.accent, th.line_strong),
    };
    let sr = sw.height() / 2.0;
    fill(p, sw, sr, track);
    if th.palette == Palette::Pokemon {
        stroke_in(p, sw, sr, 1.0, rgba(th.ink, if th.dark { 0.35 } else { 0.18 }));
    }
    let kr = sr - 2.0;
    let kx = sw.left() + sr + (sw.width() - 2.0 * sr) * on;
    p.circle_filled(pos2(kx, sw.center().y + 1.0), kr, rgba(th.ink, 0.22));
    p.circle_filled(pos2(kx, sw.center().y), kr, Color32::WHITE);
}

impl App {
    /// Palettes offered in the theme menu; Cyberpunk only once unlocked.
    fn menu_palettes(&self) -> Vec<(Palette, &'static str)> {
        let mut v = vec![(Palette::Default, "Default"), (Palette::Genshin, "Genshin"), (Palette::Pokemon, "Pokémon")];
        if self.cyberpunk_unlocked {
            v.push((Palette::Cyberpunk, "Cyberpunk"));
        }
        v
    }

    /// The theme popover under the title bar's Theme button. Closes on Esc or
    /// a click anywhere else; stays open while palettes are compared.
    fn theme_menu(&mut self, ctx: &egui::Context) {
        let th = self.theme();
        let panel_w = MENU_W + 20.0;
        let btn = self.menu_btn;
        let panel = Rect::from_min_size(pos2(btn.right() - panel_w, crate::chrome::CHROME_H + 6.0), vec2(panel_w, menu_content_h() + 28.0));
        let origin = panel.min + vec2(10.0, 14.0);
        egui::Area::new(Id::new("theme-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(panel.min)
            .show(ctx, |ui| {
                ui.allocate_rect(panel, Sense::click());
                self.menu_body(ui, &th, origin);
            });
        let (esc, press) = ctx.input(|i| (i.key_pressed(egui::Key::Escape), if i.pointer.any_pressed() { i.pointer.interact_pos() } else { None }));
        if esc || press.is_some_and(|pt| !panel.contains(pt) && !btn.contains(pt)) {
            self.menu_open = false;
        }
    }

    fn menu_body(&mut self, ui: &mut Ui, th: &Theme, origin: egui::Pos2) {
        let w = MENU_W;
        let toggle_h = MENU_ROW;
        let gap = MENU_GAP;
        let palettes = self.menu_palettes();
        let total_h = menu_content_h();
        let sb = Rect::from_min_size(origin, vec2(w + 20.0, total_h));
        let p = ui.painter().clone();

        match th.palette {
            Palette::Genshin => {
                let r = Rect::from_min_max(pos2(sb.left() - 10.0, sb.top() - 14.0), pos2(sb.right() - 10.0, sb.bottom() + 14.0));
                // Settings-menu dialog: dark slate with the gold corner
                // brackets, holding a text list whose selection is a cream bar.
                p.add(card_shadow(&p, th, r, th.radius_lg));
                fill(&p, r, th.radius_lg, if th.dark { rgba(hex(0x1b202c), 0.97) } else { hex(0x3b4255) });
                stroke_in(&p, r, th.radius_lg, 1.0, rgba(Color32::WHITE, 0.10));
                let mut gt = *th;
                gt.dark = true;
                gt.gold = hex(0xd3bc8e);
                crate::ui::genshin_frame(&p, &gt, r, th.radius_lg);
            }
            Palette::Pokemon => {
                // A milky rounded menu card, like the Let's Go bag/party panels.
                let r = Rect::from_min_max(pos2(sb.left() - 10.0, sb.top() - 14.0), pos2(sb.right() - 10.0, sb.bottom() + 14.0));
                p.add(card_shadow(&p, th, r, th.radius_lg));
                fill(&p, r, th.radius_lg, th.surface);
                stroke_in(&p, r, th.radius_lg, 2.5, if th.dark { rgba(th.text, 0.20) } else { Color32::from_white_alpha(220) });
                stroke_in(&p, r.shrink(2.5), th.radius_lg - 2.5, 1.0, rgba(th.ring, 0.35));
            }
            Palette::Cyberpunk => {
                // A chamfered menu panel: near-black, red hairline, a bright
                // red cap on the top edge.
                let r = Rect::from_min_max(pos2(sb.left() - 10.0, sb.top() - 14.0), pos2(sb.right() - 10.0, sb.bottom() + 14.0));
                crate::ui::cyber_fill(&p, r.translate(vec2(0.0, 3.0)), 9.0, Color32::from_black_alpha(140));
                cyber_panel(&p, r, 9.0, if th.dark { rgba(th.surface, 0.97) } else { th.surface }, 1.0, rgba(th.accent, 0.60));
                p.line_segment([pos2(r.left() + 1.0, r.top() + 1.0), pos2(r.left() + 46.0, r.top() + 1.0)], Stroke::new(2.0_f32, th.accent));
            }
            Palette::Default => {
                let r = Rect::from_min_max(pos2(sb.left() - 10.0, sb.top() - 14.0), pos2(sb.right() - 10.0, sb.bottom() + 14.0));
                p.add(egui::Shape::from(
                    egui::Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(if th.dark { 120 } else { 40 }) }
                        .as_shape(r, crate::ui::cr(th.radius_lg)),
                ));
                fill(&p, r, th.radius_lg, th.surface);
                stroke_in(&p, r, th.radius_lg, 1.0, th.line_strong);
            }
        }

        let mut y = origin.y;

        let fgc = if th.palette == Palette::Genshin { hex(0xece5d8) } else { th.text };
        let accent = if th.palette == Palette::Genshin { hex(0xd3bc8e) } else { th.accent };

        // --- "Theme · <name>" and a compact sun / moon switch
        let row = Rect::from_min_size(pos2(origin.x, y), vec2(w, toggle_h));
        let tr = text(ui, th, pos2(row.left() + 2.0, row.center().y), Align2::LEFT_CENTER, "Theme", Role::SansB, 13.0, fgc);
        let cur = palettes.iter().find(|(pl, _)| *pl == self.palette).map_or("", |(_, n)| *n);
        text(ui, th, pos2(tr.right() + 6.0, row.center().y), Align2::LEFT_CENTER, cur, Role::Sans, 12.5, rgba(fgc, 0.62));
        let sw = Rect::from_min_size(pos2(row.right() - 46.0, row.center().y - 11.0), vec2(46.0, 22.0));
        let resp = ui.interact(sw.expand(3.0), Id::new("dark-toggle"), Sense::click()).on_hover_text(if th.dark { "Dark mode · click for light" } else { "Light mode · click for dark" });
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let on = ui.ctx().animate_bool_with_time(Id::new("dark-anim"), th.dark, 0.2);
        let tr_r = if th.palette == Palette::Cyberpunk { 3.0 } else { sw.height() / 2.0 };
        fill(&p, sw, tr_r, rgba(fgc, if resp.hovered() { 0.14 } else { 0.08 }));
        stroke_in(&p, sw, tr_r, 1.0, rgba(fgc, if resp.hovered() { 0.32 } else { 0.18 }));
        let (sun_c, moon_c) = (pos2(sw.left() + 11.0, sw.center().y), pos2(sw.right() - 11.0, sw.center().y));
        let knob = pos2(sun_c.x + (moon_c.x - sun_c.x) * on, sw.center().y);
        let kr = sw.height() / 2.0 - 2.0;
        if th.palette == Palette::Cyberpunk {
            fill(&p, Rect::from_center_size(knob, vec2(2.0 * kr, 2.0 * kr)), 2.0, accent);
        } else {
            p.circle_filled(knob, kr, accent);
        }
        let on_knob = if th.palette == Palette::Genshin { hex(0x3b4255) } else { th.on_accent };
        // icons scaled down to fit the knob
        let small_sun = |c: egui::Pos2, col: Color32| {
            p.circle_stroke(c, 2.4, Stroke::new(1.2_f32, col));
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let d = vec2(a.sin(), -a.cos());
                p.line_segment([c + d * 4.4, c + d * 5.8], Stroke::new(1.2_f32, col));
            }
        };
        small_sun(sun_c, if on < 0.5 { on_knob } else { rgba(fgc, 0.55) });
        crate::ui::crescent(&p, moon_c, 5.0, if on >= 0.5 { on_knob } else { rgba(fgc, 0.55) });
        if resp.clicked() {
            self.toggle_dark();
        }
        y += toggle_h + 6.0;

        // --- palettes: a row of colour swatches (2 × 2 grid of each theme's colours)
        let n = palettes.len().max(1) as f32;
        let sw_w = ((w - SWATCH_GAP * (n - 1.0)) / n).floor();
        for (i, (pal, name)) in palettes.iter().enumerate() {
            let r = Rect::from_min_size(pos2(origin.x + i as f32 * (sw_w + SWATCH_GAP), y), vec2(sw_w, SWATCH_H));
            let active = self.palette == *pal;
            let resp = ui.interact(r.expand(2.0), Id::new(("palette", i)), Sense::click()).on_hover_text(*name);
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let pt = Theme::new(*pal, swatch_dark(*pal, self.dark));
            let r = if resp.is_pointer_button_down_on() { r.shrink(1.0) } else { r };
            swatch(&p, *pal, &pt, r);
            if active {
                swatch_outline(&p, *pal, r.expand(3.0), 2.0, accent);
            } else if resp.hovered() {
                swatch_outline(&p, *pal, r.expand(3.0), 1.0, rgba(fgc, 0.45));
            }
            if resp.clicked() {
                self.set_palette(*pal);
            }
        }
        y += SWATCH_H + MENU_SPLIT;
        let div_c = if th.palette == Palette::Genshin { rgba(hex(0xd3bc8e), 0.35) } else { th.line };
        p.line_segment([pos2(origin.x + 2.0, y), pos2(origin.x + w - 2.0, y)], Stroke::new(1.0_f32, div_c));
        y += MENU_SPLIT;

        // --- mascots (Pikachu in Pokémon, Paimon in Genshin)
        let rect = Rect::from_min_size(pos2(origin.x, y), vec2(w, toggle_h));
        let resp = side_toggle(ui, th, rect, Id::new("mascots-toggle"), false).on_hover_text("Pikachu (Pokémon theme) and Paimon (Genshin theme) · Ctrl+H");
        let ic = pos2(rect.left() + 10.0, rect.center().y);
        // a paw print
        p.circle_filled(ic + vec2(0.0, 2.2), 2.7, fgc);
        for d in [vec2(-3.4, -1.6), vec2(-1.2, -3.9), vec2(1.2, -3.9), vec2(3.4, -1.6)] {
            p.circle_filled(ic + d, 1.15, fgc);
        }
        text(ui, th, pos2(ic.x + 12.0, rect.center().y), Align2::LEFT_CENTER, "Mascots", Role::Sans, 13.0, fgc);
        let on = ui.ctx().animate_bool_with_time(Id::new("mascots-anim"), self.mascots, 0.2);
        mini_switch(&p, th, Rect::from_min_size(pos2(rect.right() - 8.0 - 24.0, rect.center().y - 7.0), vec2(24.0, 14.0)), on);
        if resp.clicked() {
            self.toggle_mascots();
        }
        y += toggle_h + gap;

        // --- display scale: − 100% +
        let ctx = ui.ctx().clone();
        let rect = Rect::from_min_size(pos2(origin.x, y), vec2(w, toggle_h));
        let icon_c = pos2(rect.left() + 10.0, rect.center().y);
        text(ui, th, pos2(icon_c.x - 4.0, icon_c.y + 1.0), Align2::CENTER_CENTER, "A", Role::SansB, 9.5, fgc);
        text(ui, th, pos2(icon_c.x + 3.0, icon_c.y), Align2::CENTER_CENTER, "A", Role::SansB, 13.0, fgc);
        text(ui, th, pos2(icon_c.x + 12.0, rect.center().y), Align2::LEFT_CENTER, "Scale", Role::Sans, 13.0, fgc);
        let bs = 20.0;
        let val_w = 42.0;
        let plus = Rect::from_center_size(pos2(rect.right() - 6.0 - bs / 2.0, rect.center().y), vec2(bs, bs));
        let val = Rect::from_center_size(pos2(plus.left() - val_w / 2.0, rect.center().y), vec2(val_w, bs));
        let minus = Rect::from_center_size(pos2(val.left() - bs / 2.0, rect.center().y), vec2(bs, bs));
        let step_btn = |ui: &mut Ui, r: Rect, id: &str, plus_sign: bool, enabled: bool| -> bool {
            let resp = ui.interact(r, Id::new(("scale-btn", id)), if enabled { Sense::click() } else { Sense::hover() });
            let c = if enabled { fgc } else { rgba(fgc, 0.35) };
            if enabled && resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                fill(&p, r, 5.0, rgba(fgc, 0.12));
            }
            stroke_in(&p, r, 5.0, 1.0, rgba(fgc, if enabled { 0.35 } else { 0.18 }));
            let m = r.center();
            p.line_segment([pos2(m.x - 4.5, m.y), pos2(m.x + 4.5, m.y)], Stroke::new(1.4_f32, c));
            if plus_sign {
                p.line_segment([pos2(m.x, m.y - 4.5), pos2(m.x, m.y + 4.5)], Stroke::new(1.4_f32, c));
            }
            enabled && resp.clicked()
        };
        if step_btn(ui, minus, "down", false, self.scale > SCALES[0] + 0.001) {
            self.step_scale(&ctx, false);
        }
        if step_btn(ui, plus, "up", true, self.scale < SCALES[SCALES.len() - 1] - 0.001) {
            self.step_scale(&ctx, true);
        }
        let vr = ui.interact(val, Id::new("scale-reset"), Sense::click()).on_hover_text("Display scale (Ctrl+Plus / Ctrl+Minus). Click for 100 %.");
        if vr.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if vr.clicked() {
            self.set_scale(&ctx, 1.0);
        }
        let pct = format!("{}%", (self.scale * 100.0).round() as i32);
        text(ui, th, val.center(), Align2::CENTER_CENTER, &pct, Role::SansB, 12.5, fgc);
        y += toggle_h + gap;

        // --- HBEL Excel file (what "Open Excel" opens)
        let rect = Rect::from_min_size(pos2(origin.x, y), vec2(w, toggle_h));
        let busy = self.excel_picker.busy();
        let tip = match &self.excel_path {
            Some(p) => format!("HBEL comments spreadsheet:\n{}\nClick to choose a different file", p.display()),
            None => "Choose the HBEL comments spreadsheet that \u{201c}Open Excel\u{201d} opens".to_string(),
        };
        let resp = side_toggle(ui, th, rect, Id::new("excel-file"), false).on_hover_text(tip);
        // a small sheet with a grid
        let ic = pos2(rect.left() + 10.0, rect.center().y);
        let sheet = Rect::from_center_size(ic, vec2(10.0, 12.0));
        stroke_in(&p, sheet, 1.5, 1.2, fgc);
        p.line_segment([pos2(sheet.left(), ic.y - 1.5), pos2(sheet.right(), ic.y - 1.5)], Stroke::new(1.0_f32, fgc));
        p.line_segment([pos2(sheet.left(), ic.y + 2.0), pos2(sheet.right(), ic.y + 2.0)], Stroke::new(1.0_f32, fgc));
        p.line_segment([pos2(ic.x - 1.0, sheet.top() + 3.0), pos2(ic.x - 1.0, sheet.bottom())], Stroke::new(1.0_f32, fgc));
        text(ui, th, pos2(ic.x + 12.0, rect.center().y), Align2::LEFT_CENTER, "Excel", Role::Sans, 13.0, fgc);
        let accent = if th.palette == Palette::Genshin { hex(0xd3bc8e) } else { th.accent };
        let (msg, mc) = match (&self.excel_path, busy) {
            (_, true) => ("Choosing…".to_string(), rgba(fgc, 0.65)),
            (Some(p), _) => (crate::excel::short_name(p, 18), if p.is_file() { rgba(fgc, 0.85) } else { th.bad }),
            (None, _) => ("Choose file…".to_string(), accent),
        };
        text(ui, th, pos2(rect.right() - 8.0, rect.center().y), Align2::RIGHT_CENTER, &msg, Role::SansB, 11.5, mc);
        if resp.clicked() && !busy {
            self.choose_excel(&ctx);
        }
        y += toggle_h + gap;

        // --- updates
        let rect = Rect::from_min_size(pos2(origin.x, y), vec2(w, toggle_h));
        let status = self.update.status();
        let installing = matches!(self.update.install_status(), crate::update::Install::Downloading { .. } | crate::update::Install::Restarting);
        let this_s = format!("This version: {}", crate::update::date_label(crate::update::current()));
        let resp = side_toggle(ui, th, rect, Id::new("update-check"), false);
        let resp = match &status {
            _ if installing => resp.on_hover_text(format!("{}\nClick to see the update's progress", this_s)),
            crate::update::Status::Available { .. } => resp.on_hover_text(format!("{}\nClick to update (you are asked first)", this_s)),
            _ => resp.on_hover_text(format!("{}\nClick to check GitHub for a newer release", this_s)),
        };
        let icon_c = pos2(rect.left() + 10.0, rect.center().y);
        // circular arrow
        let pts: Vec<egui::Pos2> = (0..=14).map(|i| {
            let a = 0.6 + i as f32 / 14.0 * 4.9;
            icon_c + vec2(a.cos(), a.sin()) * 4.5
        }).collect();
        let tip = *pts.last().unwrap();
        p.add(egui::Shape::line(pts, Stroke::new(1.3_f32, fgc)));
        p.add(egui::Shape::convex_polygon(vec![tip + vec2(-3.0, -1.5), tip + vec2(2.0, -2.5), tip + vec2(0.5, 2.5)], fgc, Stroke::NONE));
        text(ui, th, pos2(icon_c.x + 12.0, rect.center().y), Align2::LEFT_CENTER, "Updates", Role::Sans, 13.0, fgc);
        let (msg, mc) = match &status {
            _ if installing => ("Updating…".to_string(), if th.palette == Palette::Genshin { hex(0xd3bc8e) } else { th.accent }),
            crate::update::Status::Idle => ("Check now".to_string(), rgba(fgc, 0.65)),
            crate::update::Status::Checking => ("Checking…".to_string(), rgba(fgc, 0.65)),
            crate::update::Status::UpToDate => ("Up to date".to_string(), if th.palette == Palette::Genshin { hex(0x9fd39a) } else { th.ok }),
            crate::update::Status::Failed => ("Couldn't check".to_string(), rgba(fgc, 0.65)),
            crate::update::Status::Available { tag, .. } => (format!("{} available", crate::update::display(tag)), if th.palette == Palette::Genshin { hex(0xd3bc8e) } else { th.accent }),
        };
        text(ui, th, pos2(rect.right() - 8.0, rect.center().y), Align2::RIGHT_CENTER, &msg, Role::SansB, 11.5, mc);
        if resp.clicked() {
            match status {
                _ if installing => {
                    self.menu_open = false;
                    self.update_prompt = true;
                }
                crate::update::Status::Available { .. } => {
                    self.menu_open = false;
                    self.update_prompt = true;
                }
                crate::update::Status::Checking => {}
                _ => self.update.start(&ctx),
            }
        }
    }

    /// The update dialog. Before anything happens it asks "Update and
    /// restart?"; once approved it shows the download's progress (the app
    /// then restarts on its own), or why it failed with the release's download
    /// page as the way out. Esc or a click outside hides it (a running
    /// download carries on; the title bar and Settings reopen it).
    fn update_dialog(&mut self, ctx: &egui::Context) {
        use crate::update::Install;
        let crate::update::Status::Available { tag, url, asset } = self.update.status() else {
            self.update_prompt = false;
            return;
        };
        let install = self.update.install_status();
        let th = self.theme();
        let screen = ctx.screen_rect();
        let w = 420.0_f32.min(screen.width() - 32.0);
        let pad = 22.0;
        /// What a dialog button does.
        #[derive(Clone, Copy, PartialEq)]
        enum Act {
            Install,
            OpenPage,
            Cancel,
            Hide,
            Close,
        }
        let new_s = crate::update::display(&tag);
        let cur_s = crate::update::date_label(crate::update::current());
        let mb = |b: u64| format!("{:.1}", b as f64 / 1_048_576.0);
        // title, main line, secondary line, progress (0–1, or -1 when the size is unknown), buttons (first = primary)
        let (title, body, more, progress, buttons): (String, String, String, Option<f32>, Vec<(&str, Act)>) = match (&install, &asset) {
            (Install::Downloading { done, total }, _) => (
                format!("Updating to {}", new_s),
                if *total > 0 { format!("Downloading… {} of {} MB", mb(*done), mb(*total)) } else { format!("Downloading… {} MB", mb(*done)) },
                "The app restarts on its own when the download finishes.".into(),
                Some(if *total > 0 { (*done as f32 / *total as f32).clamp(0.0, 1.0) } else { -1.0 }),
                vec![("Hide", Act::Hide), ("Cancel update", Act::Cancel)],
            ),
            (Install::Restarting, _) => (format!("Updating to {}", new_s), "Restarting…".into(), String::new(), Some(1.0), vec![]),
            (Install::Failed(msg), _) => (
                "Update didn't finish".into(),
                msg.clone(),
                "You can still download the new version from its release page and replace the app yourself.".into(),
                None,
                vec![("Open download page", Act::OpenPage), ("Close", Act::Close)],
            ),
            (Install::Idle, Some(_)) => (
                "Update available".into(),
                format!("Version {} is available (you have {}).", new_s, cur_s),
                "The new version downloads, replaces this one and the app restarts on its own. Results you have entered are cleared.".into(),
                None,
                vec![("Update and restart", Act::Install), ("Not now", Act::Close)],
            ),
            (Install::Idle, None) => (
                "Update available".into(),
                format!("Version {} is available (you have {}).", new_s, cur_s),
                "This release can't be installed from here. Its download page opens in your browser.".into(),
                None,
                vec![("Open download page", Act::OpenPage), ("Not now", Act::Close)],
            ),
        };
        let busy = matches!(install, Install::Downloading { .. } | Install::Restarting);
        if busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let mut act: Option<Act> = None;
        egui::Area::new(Id::new("update-dialog")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            // dim the page; a click on it hides the dialog
            let dim = ui.allocate_rect(screen, Sense::click());
            let p = ui.painter().clone();
            p.rect_filled(screen, 0.0, Color32::from_black_alpha(if th.dark { 130 } else { 90 }));
            let fg = th.text;
            let iw = w - 2.0 * pad;
            let more_h = if more.is_empty() { 0.0 } else { 8.0 + paragraph_height(ui, &th, iw, &more, Role::Sans, 13.0, 1.45) };
            let bar_h = if progress.is_some() { 14.0 + 6.0 } else { 0.0 };
            let btn_h = if buttons.is_empty() { 0.0 } else { 20.0 + 36.0 };
            let h = pad + 26.0 + 12.0 + paragraph_height(ui, &th, iw, &body, Role::Sans, 14.0, 1.45) + bar_h + more_h + btn_h + pad;
            let r = Rect::from_center_size(screen.center(), vec2(w, h));
            // the panel itself swallows clicks so they do not reach the dim layer
            let _ = ui.interact(r, Id::new("update-dialog-panel"), Sense::click());
            match th.palette {
                Palette::Cyberpunk => cyber_panel(&p, r, 9.0, th.surface, 1.0, rgba(th.accent, 0.6)),
                _ => {
                    p.add(card_shadow(&p, &th, r, th.radius_lg));
                    fill(&p, r, th.radius_lg, th.surface);
                    stroke_in(&p, r, th.radius_lg, 1.0, th.line_strong);
                }
            }
            let x = r.left() + pad;
            let mut y = r.top() + pad;
            text_ls(ui, &th, pos2(x, y + 13.0), Align2::LEFT_CENTER, &title, Role::Display, 19.0, fg, -0.01);
            y += 26.0 + 12.0;
            y = paragraph(ui, &th, pos2(x, y), iw, &body, Role::Sans, 14.0, fg, 1.45).bottom();
            if let Some(f) = progress {
                // track + fill; an unknown size sweeps a short segment back and forth
                let track = Rect::from_min_size(pos2(x, y + 14.0), vec2(iw, 6.0));
                let accent = if th.palette == Palette::Genshin { th.gold } else { th.accent };
                fill(&p, track, 3.0, th.surface2);
                let seg = if f >= 0.0 {
                    Rect::from_min_size(track.min, vec2(iw * f, 6.0))
                } else {
                    let t = (ui.input(|i| i.time) as f32 * 0.8).fract();
                    let s = (if t < 0.5 { t * 2.0 } else { 2.0 - t * 2.0 }) * iw * 0.75;
                    Rect::from_min_size(pos2(x + s, track.top()), vec2(iw * 0.25, 6.0))
                };
                if seg.width() > 0.5 {
                    fill(&p, seg, 3.0, accent);
                }
                y = track.bottom();
            }
            if !more.is_empty() {
                y = paragraph(ui, &th, pos2(x, y + 8.0), iw, &more, Role::Sans, 13.0, th.muted, 1.45).bottom();
            }
            if !buttons.is_empty() {
                let br = Rect::from_min_size(pos2(x, y + 20.0), vec2(iw, 36.0));
                let mut bc = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::right_to_left(egui::Align::Min)));
                bc.spacing_mut().item_spacing.x = 10.0;
                for (k, (label, a)) in buttons.iter().enumerate() {
                    let kind = if k == 0 && !matches!(a, Act::Hide) { Btn::Primary } else { Btn::Secondary };
                    let mut resp = button(&mut bc, &th, label, kind, true);
                    if *a == Act::OpenPage {
                        resp = resp.on_hover_text(url.as_str());
                    }
                    if resp.clicked() {
                        act = Some(*a);
                    }
                }
            }
            if dim.clicked() {
                act = Some(if busy { Act::Hide } else { Act::Close });
            }
        });
        if act.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            act = Some(if busy { Act::Hide } else { Act::Close });
        }
        match act {
            Some(Act::Install) => {
                if let Some(a) = asset {
                    self.update.begin_install(ctx, a);
                }
            }
            Some(Act::OpenPage) => {
                ctx.open_url(egui::OpenUrl::new_tab(url));
                self.update.dismiss_install();
                self.update_prompt = false;
            }
            Some(Act::Cancel) => self.update.cancel_install(),
            Some(Act::Hide) => self.update_prompt = false,
            Some(Act::Close) => {
                self.update.dismiss_install();
                self.update_prompt = false;
            }
            None => {}
        }
    }
}

/// Sidebar pill button (theme / palette toggles).
fn side_toggle(ui: &mut Ui, th: &Theme, rect: Rect, id: Id, active: bool) -> egui::Response {
    let resp = ui.interact(rect, id, Sense::click());
    let hovered = resp.hovered();
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let pressed = resp.is_pointer_button_down_on();
    let p = ui.painter().clone();
    let rect = if pressed { rect.translate(vec2(0.0, 1.0)) } else { rect };
    let r = th.radius;
    match th.palette {
        Palette::Default => {
            let (bg, border) = if active { (th.accent_soft, th.accent) } else { (th.secondary, if hovered { th.line_strong } else { Color32::TRANSPARENT }) };
            fill(&p, rect, r, bg);
            stroke_in(&p, rect, r, 1.0, border);
        }
        Palette::Genshin => {
            // Settings list rows: plain text on the slate rail; the selected
            // entry is a cream bar that feathers out to the right, hover is a
            // faint white wash.
            let r = 3.0;
            let wide = Rect::from_min_max(rect.min - vec2(4.0, 0.0), rect.max - vec2(2.0, 0.0));
            if active {
                let cream = hex(0xece5d8);
                let solid = Rect::from_min_max(wide.min, pos2(wide.right() - 22.0, wide.bottom()));
                fill(&p, solid, r, cream);
                crate::ui::h_gradient(&p, Rect::from_min_max(pos2(solid.right() - 1.0, wide.top()), wide.max), cream, rgba(cream, 0.0));
            } else if hovered {
                fill(&p, wide, r, rgba(Color32::WHITE, 0.08));
            }
        }
        Palette::Pokemon => {
            let r = 14.0;
            if !pressed {
                p.add(card_shadow(&p, th, rect, r));
            }
            let (bg, border) = if active {
                (th.surface2, th.ring)
            } else {
                (th.field, if hovered { th.ring } else { th.line })
            };
            fill(&p, rect, r, bg);
            stroke_in(&p, rect, r, if active { 2.0 } else { 1.0 }, border);
            if active {
                stroke_in(&p, rect.shrink(2.0), r - 2.0, 1.0, if th.dark { rgba(Color32::WHITE, 0.10) } else { Color32::from_white_alpha(180) });
            }
        }
        Palette::Cyberpunk => {
            // Game list rows: red-outlined, a small red tab on the left, and a
            // solid red bar for the selected entry.
            let (bg, border) = if active {
                (th.accent, th.accent)
            } else if hovered {
                (rgba(th.accent, 0.14), th.accent)
            } else {
                (rgba(th.surface, 0.6), rgba(th.accent, 0.40))
            };
            cyber_panel(&p, rect, 4.0, bg, 1.0, border);
            p.rect_filled(Rect::from_min_size(rect.min - vec2(6.0, 0.0), vec2(3.0, rect.height())), 0.0, rgba(th.accent, if active { 1.0 } else { 0.55 }));
        }
    }
    resp
}

#[cfg(test)]
mod settings_tests {
    use super::*;

    #[test]
    fn cyberpunk_needs_unlock() {
        assert_eq!(Settings::parse("palette=cyberpunk\n").palette, Palette::Default);
        let s = Settings::parse("palette=cyberpunk\ncyberpunk=unlocked\n");
        assert_eq!(s.palette, Palette::Cyberpunk);
        assert!(s.cyberpunk_unlocked);
        assert!(!Settings::parse("").cyberpunk_unlocked);
    }

    #[test]
    fn unlock_code_spans_keystrokes() {
        let mut buf = String::new();
        // typed one key per frame, with other text before it
        let hits: usize = ["1", "2", "0", "7", "7"].iter().map(|k| feed_unlock_code(&mut buf, k)).sum();
        assert_eq!(hits, 1);
        assert_eq!(feed_unlock_code(&mut buf, "20772077"), 2);
        assert_eq!(feed_unlock_code(&mut buf, "207.7"), 0);
        assert_eq!(feed_unlock_code(&mut buf, "2o77"), 0);
    }
}

// ---------------------------------------------------------------------------
// Developer screenshot harness (cargo feature "screenshot")
// HAEM_SHOT=out.png HAEM_SCENE="tab=retic,palette=genshin,dark=1,fill=1,calc=1,third=1" HAEM_SIZE=1180x820
// ---------------------------------------------------------------------------
#[cfg(feature = "screenshot")]
impl App {
    fn apply_scene(&mut self) {
        self.shot.path = std::env::var("HAEM_SHOT").ok();
        self.shot.at = std::env::var("HAEM_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
        let scene = std::env::var("HAEM_SCENE").unwrap_or_default();
        let mut fill_data = false;
        for kv in scene.split(',') {
            let mut it = kv.splitn(2, '=');
            let (k, v) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
            match k {
                "tab" => {
                    self.tab = match v {
                        "retic" => Tab::Retic,
                        "pt" => Tab::PtAptt,
                        "hbel" => Tab::Hbel,
                        _ => Tab::Fbc,
                    }
                }
                "palette" => {
                    self.palette = Palette::from_key(v);
                    if self.palette == Palette::Cyberpunk {
                        self.cyberpunk_unlocked = true;
                    }
                }
                "cyber" => self.cyberpunk_unlocked = v == "1",
                "dark" => self.dark = v == "1",
                "fill" => fill_data = v == "1",
                "calc" => self.calc_open = v == "1",
                "ref" => self.ref_open = v == "1",
                "menu" => self.menu_open = v == "1",
                "third" => {
                    self.fbc.third = v == "1";
                    self.pt.third = v == "1";
                }
                "scroll" => self.shot.scroll = v.parse().unwrap_or(0.0),
                "pika" => self.shot.pika_sheet = v == "sheet",
                "paimon" => self.shot.paimon_sheet = v == "sheet",
                "goal" => {
                    let xy: Vec<f32> = v.split(':').filter_map(|n| n.parse().ok()).collect();
                    if xy.len() == 2 {
                        self.shot.goal = Some(pos2(xy[0], xy[1]));
                    }
                }
                "hbelfill" if v == "1" => {
                    use crate::hbel_data::Row;
                    let i = &mut self.hbel.inputs;
                    i.hba2 = "3.7".into();
                    i.hbf = "0.5".into();
                    i.click_chip(Row::Hb, 0);
                    i.click_chip(Row::Indices, 0);
                }
                "hbeltie" if v == "1" => {
                    self.hbel.inputs.hba2 = "2.8".into();
                    self.hbel.inputs.hbf = "3".into();
                }
                "hbelsplit" if v == "1" => {
                    use crate::hbel_data::Row;
                    let i = &mut self.hbel.inputs;
                    i.hbf = "0.5".into();
                    i.click_chip(Row::Peak, 1);
                    i.click_chip(Row::Hb, 0);
                    i.click_chip(Row::Indices, 0);
                }
                "hbelsrc" => self.hbel.source_open = v == "1",
                // update=1 (title bar), ask (dialog), dl (downloading), fail (failed)
                "update" if matches!(v, "1" | "ask" | "dl" | "fail") => {
                    let asset = crate::update::Asset { url: "https://github.com/x/y/releases/download/3-Dec-2026/app.exe".into(), size: 9_800_000 };
                    self.update.force(crate::update::Status::Available { tag: "3-Dec-2026".into(), url: "https://github.com/x/y/releases/tag/3-Dec-2026".into(), asset: Some(asset) });
                    self.update_prompt = v != "1";
                    match v {
                        "dl" => self.update.force_install(crate::update::Install::Downloading { done: 4_200_000, total: 9_800_000 }),
                        "fail" => self.update.force_install(crate::update::Install::Failed("Couldn't save the update in the app's folder. You may not have permission to change files there.".into())),
                        _ => {}
                    }
                }
                "hbeladd" => self.hbel.demo_form(v == "2"),
                _ => {}
            }
        }
        if fill_data {
            let a = [["7.2", "7.5", "7.4"], ["4.20", "4.15", "4.18"], ["13.5", "13.7", "14.9"], ["250", "262", "256"]];
            for (i, row) in a.iter().enumerate() {
                for (k, v) in row.iter().enumerate() {
                    self.fbc.vals[i][k] = v.to_string();
                }
            }
            self.pt.vals[0] = ["12.4".into(), "12.6".into(), "12.5".into()];
            self.pt.vals[1] = ["31.2".into(), "31.8".into(), "".into()];
            self.r_retic = "0.9".into();
            self.r_hct = "38".into();
            self.r_rbc = "4.13".into();
            self.gender = Some(Gender::Male);
            self.attempts = vec!["10".into(), "8".into(), "9".into()];
            self.dil[0] = "6.28".into();
            self.dil[1] = "3.69".into();
            self.dil[2] = "6.45".into();
        }
    }

    fn screenshot_tick(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot.path.clone() else { return };
        self.shot.frame += 1;
        ctx.request_repaint();
        if self.shot.frame == self.shot.at {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let img = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = img {
            let [w, h] = img.size;
            let mut buf = image::RgbaImage::new(w as u32, h as u32);
            for (i, px) in img.pixels.iter().enumerate() {
                buf.put_pixel((i % w) as u32, (i / w) as u32, image::Rgba([px.r(), px.g(), px.b(), 255]));
            }
            let _ = buf.save(&path);
            std::process::exit(0);
        }
    }
}
