//! Application state, page chrome (background + sidebar) and persistence.
use crate::logic::Gender;
use crate::theme::{hex, rgba, Palette, Role, Theme};
use crate::ui::*;
use eframe::egui::{
    self, epaint::StrokeKind, pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, Ui,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Fbc,
    Retic,
    PtAptt,
    Dilution,
    Hbel,
}

pub struct ParamDef {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub limit: f64,
    pub ph: [&'static str; 3],
    /// index into `Theme::param` for coloured cards (FBC only)
    pub color: Option<usize>,
}

pub const FBC_PARAMS: [ParamDef; 4] = [
    ParamDef { id: "wbc", name: "WBC", unit: "×10⁹/L", limit: 10.0, ph: ["e.g. 7.2", "e.g. 7.5", "e.g. 7.4"], color: Some(0) },
    ParamDef { id: "rbc", name: "RBC", unit: "×10¹²/L", limit: 4.0, ph: ["e.g. 4.20", "e.g. 4.15", "e.g. 4.18"], color: Some(1) },
    ParamDef { id: "hgb", name: "HGB", unit: "g/dL", limit: 4.0, ph: ["e.g. 13.5", "e.g. 13.7", "e.g. 13.6"], color: Some(2) },
    ParamDef { id: "plt", name: "PLT", unit: "×10⁹/L", limit: 25.0, ph: ["e.g. 250", "e.g. 262", "e.g. 256"], color: Some(3) },
];

pub const PT_PARAMS: [ParamDef; 2] = [
    ParamDef { id: "pt", name: "PT", unit: "seconds", limit: 5.0, ph: ["e.g. 12.4", "e.g. 12.6", "e.g. 12.5"], color: None },
    ParamDef { id: "aptt", name: "APTT", unit: "seconds", limit: 5.0, ph: ["e.g. 31.2", "e.g. 31.8", "e.g. 31.5"], color: None },
];

pub const DIL_PARAMS: [(&str, &str); 5] = [
    ("WBC", "e.g. 6.28"),
    ("RBC", "e.g. 3.69"),
    ("HGB", "e.g. 6.45"),
    ("HCT", "e.g. 21.4"),
    ("PLT", "e.g. 127"),
];

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
    pub hbel: crate::hbel_panel::HbelState,
    genshin_backdrop: egui::TextureHandle,
    pokemon_backdrop: egui::TextureHandle,
    styled: Option<(Palette, bool)>,
    #[cfg(feature = "screenshot")]
    shot: crate::app::Shot,
}

#[cfg(feature = "screenshot")]
#[derive(Default)]
pub struct Shot {
    path: Option<String>,
    frame: u32,
    scroll: f32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::install(&cc.egui_ctx);
        let (dark, palette) = load_settings();
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
            hbel: Default::default(),
            genshin_backdrop: genshin_backdrop_texture(&cc.egui_ctx),
            pokemon_backdrop: pokemon_backdrop_texture(&cc.egui_ctx),
            styled: None,
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
            Palette::Default => {}
        }
        save_settings(self.dark, self.palette);
    }

    pub fn toggle_dark(&mut self) {
        self.dark = !self.dark;
        save_settings(self.dark, self.palette);
    }
}

// ---------------------------------------------------------------------------
// Persistence: %APPDATA%\HaematologyCalculator\settings.txt (theme=… / palette=…)
// ---------------------------------------------------------------------------

fn settings_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    Some(base.join("HaematologyCalculator").join("settings.txt"))
}

fn load_settings() -> (bool, Palette) {
    let mut dark = false;
    let mut palette = Palette::Default;
    if let Some(Ok(s)) = settings_path().map(std::fs::read_to_string) {
        for line in s.lines() {
            if let Some(v) = line.strip_prefix("theme=") {
                dark = v.trim() == "dark";
            } else if let Some(v) = line.strip_prefix("palette=") {
                palette = Palette::from_key(v.trim());
            }
        }
    }
    (dark, palette)
}

fn save_settings(dark: bool, palette: Palette) {
    if let Some(path) = settings_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, format!("theme={}\npalette={}\n", if dark { "dark" } else { "light" }, palette.key()));
    }
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let th = self.theme();
        self.apply_style(ctx, &th);

        // Field navigation state: rebuilt every frame from the fields actually drawn.
        ctx.data_mut(|d| d.insert_temp(Id::new("haem-field-order"), Vec::<(Id, Rect)>::new()));
        let focus_before = ctx.memory(|m| m.focused());

        crate::chrome::title_bar(self, ctx);

        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(th.bg)).show(ctx, |ui| {
            let screen = ui.max_rect();
            paint_page_background(ui, &th, screen, &self.genshin_backdrop, &self.pokemon_backdrop);

            let app_w = (screen.width() - 48.0).min(1440.0).max(480.0);
            let left = screen.left() + ((screen.width() - app_w) / 2.0).max(24.0);
            // The theme rail ends around left+98; keep a 38 pt gutter rather
            // than leaving a dead 100 pt band between it and the main body.
            let content_left = left + 136.0;
            let content_w = (left + app_w - content_left).max(300.0);

            #[allow(unused_mut)]
            let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]).drag_to_scroll(false);
            #[cfg(feature = "screenshot")]
            {
                area = area.vertical_scroll_offset(self.shot.scroll);
            }
            area.show(ui, |ui| {
                let origin = pos2(content_left, ui.cursor().min.y + 36.0);
                let bottom = match self.tab {
                    Tab::Fbc => self.checker_panel(ui, &th, origin, content_w, true),
                    Tab::PtAptt => self.checker_panel(ui, &th, origin, content_w, false),
                    Tab::Retic => self.retic_panel(ui, &th, origin, content_w),
                    Tab::Dilution => self.dilution_panel(ui, &th, origin, content_w),
                    Tab::Hbel => self.hbel_panel(ui, &th, origin, content_w),
                };
                let top = ui.cursor().min;
                ui.allocate_rect(Rect::from_min_max(top, pos2(ui.max_rect().right(), bottom + 60.0)), Sense::hover());
                nav_fields(ui, focus_before);
            });

            self.sidebar(ui, &th, pos2(left, screen.top() + 36.0));
            crate::chrome::resize_frame(ui);
        });

        #[cfg(feature = "screenshot")]
        self.screenshot_tick(ctx);
    }
}

fn paint_page_background(ui: &Ui, th: &Theme, screen: Rect, genshin: &egui::TextureHandle, pokemon: &egui::TextureHandle) {
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

impl App {
    fn sidebar(&mut self, ui: &mut Ui, th: &Theme, origin: egui::Pos2) {
        let w = 88.0; // compact content width
        let toggle_h = 20.0;
        let gap = 3.5;
        let total_h = toggle_h + 3.0 * (gap + toggle_h);
        let sb = Rect::from_min_size(origin, vec2(w + 20.0, total_h));
        let p = ui.painter().clone();

        match th.palette {
            Palette::Genshin => {
                let r = Rect::from_min_max(pos2(sb.left() - 10.0, sb.top() - 14.0), pos2(sb.right() - 10.0, sb.bottom() + 14.0));
                p.add(card_shadow(&p, th, r, th.radius_lg));
                fill(&p, r, th.radius_lg, hex(0x3a4556));
                stroke_in(&p, r, th.radius_lg, 1.0, Color32::from_white_alpha(20));
            }
            Palette::Pokemon => {
                // A milky rounded menu card, like the Let's Go bag/party panels.
                let r = Rect::from_min_max(pos2(sb.left() - 10.0, sb.top() - 14.0), pos2(sb.right() - 10.0, sb.bottom() + 14.0));
                p.add(card_shadow(&p, th, r, th.radius_lg));
                fill(&p, r, th.radius_lg, th.surface);
                stroke_in(&p, r, th.radius_lg, 2.5, if th.dark { rgba(th.text, 0.20) } else { Color32::from_white_alpha(220) });
                stroke_in(&p, r.shrink(2.5), th.radius_lg - 2.5, 1.0, rgba(th.ring, 0.35));
            }
            Palette::Default => {
                p.line_segment([pos2(sb.right() - 0.5, sb.top()), pos2(sb.right() - 0.5, sb.bottom())], Stroke::new(1.0, th.line));
            }
        }

        let mut y = origin.y;

        // --- dark mode switch
        let rect = Rect::from_min_size(pos2(origin.x, y), vec2(w, toggle_h));
        let resp = side_toggle(ui, th, rect, Id::new("dark-toggle"), false);
        let icon_c = pos2(rect.left() + 10.0, rect.center().y);
        let fgc = if th.palette == Palette::Genshin { hex(0x2f3a4c) } else { th.text };
        if th.dark {
            moon_icon(&p, icon_c, fgc);
        } else {
            sun_icon(&p, icon_c, fgc);
        }
        // switch
        let sw = Rect::from_min_size(pos2(rect.right() - 8.0 - 22.0, rect.center().y - 6.0), vec2(22.0, 12.0));
        let on = ui.ctx().animate_bool_with_time(Id::new("dark-anim"), th.dark, 0.2);
        let track = match (th.palette, th.dark) {
            (Palette::Genshin, true) => hex(0x6993d7),
            (Palette::Genshin, false) => hex(0xb3ab9a),
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

        // --- palettes
        for (i, (pal, name)) in [(Palette::Default, "Default"), (Palette::Genshin, "Genshin"), (Palette::Pokemon, "Pokémon")].iter().enumerate() {
            y += gap;
            let rect = Rect::from_min_size(pos2(origin.x, y), vec2(w, toggle_h));
            let active = self.palette == *pal;
            let resp = side_toggle(ui, th, rect, Id::new(("palette", i)), active);
            let sc = pos2(rect.left() + 10.0, rect.center().y);
            let (fg, sw_fill, sw_border) = if th.palette == Palette::Genshin {
                if active {
                    (hex(0x232c3a), hex(0x232c3a), hex(0x232c3a))
                } else {
                    (hex(0x2f3a4c), Color32::TRANSPARENT, hex(0x6f6553))
                }
            } else if th.palette == Palette::Pokemon {
                if active {
                    (th.text, th.ring, th.ring)
                } else {
                    (th.text, Color32::TRANSPARENT, th.line_strong)
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
            }
            text(ui, th, pos2(sc.x + 5.0 + 5.0, rect.center().y), Align2::LEFT_CENTER, name, Role::Sans, 12.0, fg);
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
            // Cream pills with a warm rim: the selected state gets a darker
            // ink and border so it reads clearly against the navy rail.
            let (bg, border) = if active {
                (hex(0xcbba8c), hex(0x8f7440))
            } else {
                (hex(0xf7f0e6), if hovered { hex(0xcbba8c) } else { hex(0xb9ad98) })
            };
            if !pressed {
                p.rect_filled(rect.translate(vec2(0.0, 2.0)), cr(pill()), Color32::from_black_alpha(56));
            }
            if hovered {
                p.rect_stroke(rect.expand(1.5), cr(pill()), Stroke::new(3.0, rgba(hex(0xcbba8c), 0.55)), StrokeKind::Middle);
            }
            fill(&p, rect, pill(), bg);
            stroke_in(&p, rect, pill(), 1.0, border);
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
                        "dil" => Tab::Dilution,
                        "hbel" => Tab::Hbel,
                        _ => Tab::Fbc,
                    }
                }
                "palette" => self.palette = Palette::from_key(v),
                "dark" => self.dark = v == "1",
                "fill" => fill_data = v == "1",
                "calc" => self.calc_open = v == "1",
                "third" => {
                    self.fbc.third = v == "1";
                    self.pt.third = v == "1";
                }
                "scroll" => self.shot.scroll = v.parse().unwrap_or(0.0),
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
        if self.shot.frame == 8 {
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
