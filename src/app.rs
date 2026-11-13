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
    pub dil: [String; 5],
    /// Theme popover (opened from the title bar's Theme button).
    pub menu_open: bool,
    pub menu_btn: Rect,
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
        let Settings { dark, palette, mascots, cyberpunk_unlocked } = Settings::load();
        let mut app = App {
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
            dil: Default::default(),
            menu_open: false,
            menu_btn: Rect::NOTHING,
            genshin_backdrop: genshin_backdrop_texture(&cc.egui_ctx),
            pokemon_backdrop: pokemon_backdrop_texture(&cc.egui_ctx),
            styled: None,
            pika: crate::pikachu::Pikachu::new(),
            paimon: crate::paimon::Paimon::new(),
            mascots,
            cyberpunk_unlocked,
            typed: String::new(),
            #[cfg(feature = "screenshot")]
            shot: Default::default(),
        };
        #[cfg(feature = "screenshot")]
        app.apply_scene();
        app
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
        v.selection.stroke = Stroke::new(1.0, th.text);
        v.text_cursor.stroke = Stroke::new(1.5, th.text);
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
        Settings { dark: self.dark, palette: self.palette, mascots: self.mascots, cyberpunk_unlocked: self.cyberpunk_unlocked }.save();
    }
}

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
// (theme=… / palette=… / mascots=shown|hidden / cyberpunk=unlocked|locked)
// ---------------------------------------------------------------------------

struct Settings {
    dark: bool,
    palette: Palette,
    mascots: bool,
    cyberpunk_unlocked: bool,
}

fn settings_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    Some(base.join("HaematologyCalculator").join("settings.txt"))
}

impl Settings {
    fn load() -> Settings {
        let mut s = Settings { dark: false, palette: Palette::Default, mascots: true, cyberpunk_unlocked: false };
        if let Some(Ok(text)) = settings_path().map(std::fs::read_to_string) {
            s = Settings::parse(&text);
        }
        s
    }

    fn parse(text: &str) -> Settings {
        let mut s = Settings { dark: false, palette: Palette::Default, mascots: true, cyberpunk_unlocked: false };
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("theme=") {
                s.dark = v.trim() == "dark";
            } else if let Some(v) = line.strip_prefix("palette=") {
                s.palette = Palette::from_key(v.trim());
            } else if let Some(v) = line.strip_prefix("mascots=") {
                s.mascots = v.trim() != "hidden";
            } else if let Some(v) = line.strip_prefix("cyberpunk=") {
                s.cyberpunk_unlocked = v.trim() == "unlocked";
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
                    "theme={}\npalette={}\nmascots={}\ncyberpunk={}\n",
                    if self.dark { "dark" } else { "light" },
                    self.palette.key(),
                    if self.mascots { "shown" } else { "hidden" },
                    if self.cyberpunk_unlocked { "unlocked" } else { "locked" }
                ),
            );
        }
    }
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
// Frame
// ---------------------------------------------------------------------------

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        crate::cursor::install(frame);

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

        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(th.bg)).show(ctx, |ui| {
            let screen = ui.max_rect();
            paint_page_background(ui, &th, screen, &self.genshin_backdrop, &self.pokemon_backdrop);

            // Theme controls live in the title bar, so the results get the full
            // width (capped so rows stay scannable on wide monitors).
            let app_w = (screen.width() - 48.0).min(1280.0).max(300.0);
            let left = screen.left() + ((screen.width() - app_w) / 2.0).max(24.0);
            let content_left = left;
            let content_w = app_w;

            #[allow(unused_mut)]
            let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]).drag_to_scroll(false);
            #[cfg(feature = "screenshot")]
            if self.shot.path.is_some() {
                // Only while rendering a screenshot: pinning the offset every
                // frame otherwise stops the page from scrolling at all.
                area = area.vertical_scroll_offset(self.shot.scroll);
            }
            area.show(ui, |ui| {
                let origin = pos2(content_left, ui.cursor().min.y + 12.0);
                let bottom = match self.tab {
                    Tab::Fbc => self.checker_panel(ui, &th, origin, content_w, true),
                    Tab::PtAptt => self.checker_panel(ui, &th, origin, content_w, false),
                    Tab::Retic => self.retic_panel(ui, &th, origin, content_w),
                };
                let top = ui.cursor().min;
                ui.allocate_rect(Rect::from_min_max(top, pos2(ui.max_rect().right(), bottom + 60.0)), Sense::hover());
                nav_fields(ui, focus_before);
            });

            crate::pikachu::perch(ui, Id::new("pika-floor"), screen, crate::pikachu::PerchKind::Floor, false);
            crate::chrome::resize_frame(ui);
        });

        if self.menu_open {
            self.theme_menu(ctx);
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
fn nav_fields(ui: &Ui, focus_before: Option<Id>) {
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

const MENU_W: f32 = 150.0;
const MENU_ROW: f32 = 28.0;
const MENU_GAP: f32 = 4.0;
const MENU_SPLIT: f32 = 9.0;

fn menu_content_h(rows: usize) -> f32 {
    // dark-mode row, a divider, then one row per listed palette
    MENU_ROW + MENU_SPLIT + rows as f32 * (MENU_GAP + MENU_ROW)
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
        let panel = Rect::from_min_size(pos2(btn.right() - panel_w, crate::chrome::CHROME_H + 6.0), vec2(panel_w, menu_content_h(self.menu_palettes().len()) + 28.0));
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
        let total_h = menu_content_h(palettes.len());
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

        // --- dark mode switch
        let rect = Rect::from_min_size(pos2(origin.x, y), vec2(w, toggle_h));
        let resp = side_toggle(ui, th, rect, Id::new("dark-toggle"), false);
        let icon_c = pos2(rect.left() + 10.0, rect.center().y);
        let fgc = if th.palette == Palette::Genshin { hex(0xece5d8) } else { th.text };
        if th.dark {
            moon_icon(&p, icon_c, fgc);
        } else {
            sun_icon(&p, icon_c, fgc);
        }
        text(ui, th, pos2(icon_c.x + 12.0, rect.center().y), Align2::LEFT_CENTER, "Dark mode", Role::Sans, 13.0, fgc);
        // switch
        let sw = Rect::from_min_size(pos2(rect.right() - 8.0 - 22.0, rect.center().y - 6.0), vec2(22.0, 12.0));
        let on = ui.ctx().animate_bool_with_time(Id::new("dark-anim"), th.dark, 0.2);
        let track = match (th.palette, th.dark) {
            (Palette::Genshin, true) => hex(0xd3bc8e),
            (Palette::Genshin, false) => hex(0x646d84),
            (_, true) => th.accent,
            _ => th.line_strong,
        };
        let sr = 6.0;
        fill(&p, sw, sr, track);
        if th.palette == Palette::Pokemon {
            stroke_in(&p, sw, sr, 1.0, rgba(th.ink, if th.dark { 0.35 } else { 0.18 }));
        }
        let kx = sw.left() + 6.0 + 10.0 * on;
        p.circle_filled(pos2(kx, sw.center().y + 1.0), 4.0, rgba(th.ink, 0.22));
        p.circle_filled(pos2(kx, sw.center().y), 4.0, Color32::WHITE);
        if resp.clicked() {
            self.toggle_dark();
        }
        y += toggle_h;
        let div_c = if th.palette == Palette::Genshin { rgba(hex(0xd3bc8e), 0.35) } else { th.line };
        p.line_segment([pos2(origin.x + 2.0, y + MENU_SPLIT / 2.0), pos2(origin.x + w - 2.0, y + MENU_SPLIT / 2.0)], Stroke::new(1.0, div_c));
        y += MENU_SPLIT - gap;

        // --- palettes
        for (i, (pal, name)) in palettes.iter().enumerate()
        {
            y += gap;
            let rect = Rect::from_min_size(pos2(origin.x, y), vec2(w, toggle_h));
            let active = self.palette == *pal;
            let resp = side_toggle(ui, th, rect, Id::new(("palette", i)), active);
            let sc = pos2(rect.left() + 10.0, rect.center().y);
            let (fg, sw_fill, sw_border) = if th.palette == Palette::Genshin {
                if active {
                    (hex(0x3b4255), hex(0x3b4255), hex(0x3b4255))
                } else {
                    (rgba(hex(0xece5d8), 0.80), Color32::TRANSPARENT, hex(0xd3bc8e))
                }
            } else if th.palette == Palette::Pokemon {
                if active {
                    (th.text, th.ring, th.ring)
                } else {
                    (th.text, Color32::TRANSPARENT, th.line_strong)
                }
            } else if th.palette == Palette::Cyberpunk {
                // the selected list row is a solid red bar with dark type
                if active {
                    (th.on_accent, th.on_accent, th.on_accent)
                } else {
                    (th.accent, Color32::TRANSPARENT, th.accent)
                }
            } else if active {
                (th.accent_dark, th.accent, th.accent)
            } else {
                (th.text, Color32::TRANSPARENT, th.line_strong)
            };
            match pal {
                Palette::Default => {
                    p.circle_filled(sc, 5.0, sw_border);
                    p.circle_filled(sc, 4.0, th.surface);
                    p.circle_filled(sc, 2.5, if active { th.accent } else if active_bg_secondary(th) { th.secondary } else { th.surface });
                }
                Palette::Genshin => {
                    diamond(&p, sc, 4.5, if active { sw_fill } else { Color32::TRANSPARENT }, Stroke::new(1.0, sw_border));
                }
                Palette::Pokemon => {
                    pokeball(&p, sc, 8.0, th.ink, th.accent, Color32::WHITE);
                }
                Palette::Cyberpunk => {
                    cyber_chip(
                        &p,
                        sc,
                        9.0,
                        sw_fill,
                        sw_border,
                        if active { th.accent } else { th.ring },
                    );
                }
            }
            text(ui, th, pos2(sc.x + 12.0, rect.center().y), Align2::LEFT_CENTER, name, Role::Sans, 13.0, fg);
            if resp.clicked() {
                self.set_palette(*pal);
            }
            y += toggle_h;
        }
    }
}

fn active_bg_secondary(_th: &Theme) -> bool {
    true
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
