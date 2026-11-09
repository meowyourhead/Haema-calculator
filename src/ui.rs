//! Reusable, CSS-like widgets: text, buttons, fields, cards, pills and ornaments.
use crate::theme::{rgba, Palette, Role, Theme};
use eframe::egui::{
    self, epaint::StrokeKind, text::LayoutJob, text::TextFormat, Align, Align2, Color32, CornerRadius, Id, Layout,
    Margin, Mesh, Painter, Pos2, Rect, Response, RichText, Sense, Shadow, Shape, Stroke, TextEdit, Ui, UiBuilder,
    Vec2,
};
use eframe::egui::{pos2, vec2};
use std::sync::Arc;

pub fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

pub fn pill() -> f32 {
    255.0
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

pub fn galley(ui: &Ui, th: &Theme, s: &str, role: Role, size: f32, color: Color32, ls_em: f32, wrap: f32) -> Arc<egui::Galley> {
    let mut job = LayoutJob::single_section(
        s.to_owned(),
        TextFormat {
            font_id: th.font(role, size),
            color,
            extra_letter_spacing: ls_em * size,
            italics: false,
            ..Default::default()
        },
    );
    job.wrap.max_width = wrap;
    ui.fonts(|f| f.layout_job(job))
}

pub fn paint_galley(p: &Painter, th: &Theme, role: Role, pos: Pos2, g: Arc<egui::Galley>, color: Color32) {
    p.galley(pos, g.clone(), color);
    if th.faux_bold(role) {
        p.galley(pos + vec2(0.45, 0.0), g, color);
    }
}

/// Draw text anchored at `pos`; returns its rectangle.
pub fn text(ui: &Ui, th: &Theme, pos: Pos2, anchor: Align2, s: &str, role: Role, size: f32, color: Color32) -> Rect {
    text_ls(ui, th, pos, anchor, s, role, size, color, 0.0)
}

pub fn text_ls(ui: &Ui, th: &Theme, pos: Pos2, anchor: Align2, s: &str, role: Role, size: f32, color: Color32, ls: f32) -> Rect {
    let g = galley(ui, th, s, role, size, color, ls, f32::INFINITY);
    let rect = anchor.anchor_size(pos, g.size());
    paint_galley(ui.painter(), th, role, rect.min, g, color);
    rect
}

pub fn text_width(ui: &Ui, th: &Theme, s: &str, role: Role, size: f32) -> f32 {
    galley(ui, th, s, role, size, Color32::WHITE, 0.0, f32::INFINITY).size().x
}

/// Wrapped paragraph; returns the rect it occupies.
pub fn paragraph(ui: &Ui, th: &Theme, pos: Pos2, width: f32, s: &str, role: Role, size: f32, color: Color32, line: f32) -> Rect {
    let mut job = LayoutJob::single_section(
        s.to_owned(),
        TextFormat {
            font_id: th.font(role, size),
            color,
            line_height: Some(size * line),
            ..Default::default()
        },
    );
    job.wrap.max_width = width;
    let g = ui.fonts(|f| f.layout_job(job));
    let rect = Rect::from_min_size(pos, g.size());
    paint_galley(ui.painter(), th, role, pos, g, color);
    rect
}

pub fn paragraph_height(ui: &Ui, th: &Theme, width: f32, s: &str, role: Role, size: f32, line: f32) -> f32 {
    let mut job = LayoutJob::single_section(
        s.to_owned(),
        TextFormat { font_id: th.font(role, size), color: Color32::WHITE, line_height: Some(size * line), ..Default::default() },
    );
    job.wrap.max_width = width;
    ui.fonts(|f| f.layout_job(job)).size().y
}

// ---------------------------------------------------------------------------
// Shape helpers
// ---------------------------------------------------------------------------

/// The Genshin page backdrop: a tiny, pre-blurred crop of the reference game
/// screenshot, embedded as raw RGBA so release builds need no image decoder.
pub fn genshin_backdrop_texture(ctx: &egui::Context) -> egui::TextureHandle {
    const W: usize = 192;
    const H: usize = 128;
    let pixels = include_bytes!("../assets/genshin-backdrop.rgba");
    let image = egui::ColorImage::from_rgba_unmultiplied([W, H], pixels);
    ctx.load_texture("genshin-backdrop", image, egui::TextureOptions::LINEAR)
}

/// The Pokémon backdrop: a blurred sunlit meadow from the Let's Go reference.
pub fn pokemon_backdrop_texture(ctx: &egui::Context) -> egui::TextureHandle {
    const W: usize = 192;
    const H: usize = 108;
    let pixels = include_bytes!("../assets/pokemon-backdrop.rgba");
    let image = egui::ColorImage::from_rgba_unmultiplied([W, H], pixels);
    ctx.load_texture("pokemon-backdrop", image, egui::TextureOptions::LINEAR)
}

pub fn fill(p: &Painter, rect: Rect, r: f32, c: Color32) {
    p.rect_filled(rect, cr(r), c);
}

pub fn stroke_in(p: &Painter, rect: Rect, r: f32, w: f32, c: Color32) {
    p.rect_stroke(rect, cr(r), Stroke::new(w, c), StrokeKind::Inside);
}

pub fn diamond(p: &Painter, c: Pos2, half: f32, fill_c: Color32, stroke: Stroke) {
    p.add(Shape::convex_polygon(
        vec![pos2(c.x, c.y - half), pos2(c.x + half, c.y), pos2(c.x, c.y + half), pos2(c.x - half, c.y)],
        fill_c,
        stroke,
    ));
}

/// Four-pointed star (the Genshin row sparkle).
pub fn star(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = size / 24.0;
    let o = |x: f32, y: f32| pos2(c.x + (x - 12.0) * s, c.y + (y - 12.0) * s);
    let mut m = Mesh::default();
    let pts = [
        o(12.0, 0.0), o(14.6, 9.4), o(9.4, 9.4), // top
        o(24.0, 12.0), o(14.6, 14.6), o(14.6, 9.4), // right
        o(12.0, 24.0), o(9.4, 14.6), o(14.6, 14.6), // bottom
        o(0.0, 12.0), o(9.4, 9.4), o(9.4, 14.6), // left
        o(9.4, 9.4), o(14.6, 9.4), o(14.6, 14.6), o(9.4, 14.6), // centre
    ];
    for (i, q) in pts.iter().enumerate() {
        let _ = i;
        m.colored_vertex(*q, color);
    }
    for t in 0..4 {
        m.add_triangle(t * 3, t * 3 + 1, t * 3 + 2);
    }
    m.add_triangle(12, 13, 14);
    m.add_triangle(12, 14, 15);
    p.add(Shape::mesh(m));
}

/// Radial glow (used for the Genshin page background).
pub fn radial(p: &Painter, center: Pos2, rx: f32, ry: f32, color: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(center, color);
    let n = 48;
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        m.colored_vertex(pos2(center.x + a.cos() * rx, center.y + a.sin() * ry), Color32::TRANSPARENT);
    }
    for i in 0..n as u32 {
        m.add_triangle(0, i + 1, i + 2);
    }
    p.add(Shape::mesh(m));
}

pub fn sun_icon(p: &Painter, c: Pos2, color: Color32) {
    let k = 16.0 / 24.0;
    let st = Stroke::new(2.0 * k, color);
    p.circle_stroke(c, 4.0 * k, st);
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::FRAC_PI_4;
        let (s, co) = a.sin_cos();
        let d = vec2(s, -co);
        p.line_segment([c + d * (8.0 * k), c + d * (10.0 * k)], st);
    }
}

pub fn moon_icon(p: &Painter, c: Pos2, color: Color32) {
    let k = 16.0 / 24.0;
    let o = pos2(c.x - 12.0 * k, c.y - 12.0 * k);
    let to = |x: f32, y: f32| pos2(o.x + x * k, o.y + y * k);
    let (c1, r1) = ((12.0f32, 12.0f32), 9.04f32);
    let (c2, r2) = ((16.8f32, 7.2f32), 7.0f32);
    let mut pts: Vec<Pos2> = vec![];
    // outer arc: points on circle 1 that lie outside circle 2
    for i in 0..=72 {
        let a = i as f32 / 72.0 * std::f32::consts::TAU;
        let (x, y) = (c1.0 + r1 * a.cos(), c1.1 + r1 * a.sin());
        if ((x - c2.0).powi(2) + (y - c2.1).powi(2)).sqrt() >= r2 - 0.05 {
            pts.push(to(x, y));
        }
    }
    // inner edge: points on circle 2 that lie inside circle 1
    let mut inner = vec![];
    for i in 0..=72 {
        let a = i as f32 / 72.0 * std::f32::consts::TAU;
        let (x, y) = (c2.0 + r2 * a.cos(), c2.1 + r2 * a.sin());
        if ((x - c1.0).powi(2) + (y - c1.1).powi(2)).sqrt() <= r1 + 0.05 {
            inner.push(to(x, y));
        }
    }
    pts.extend(inner);
    pts.push(pts[0]);
    p.add(Shape::line(pts, Stroke::new(2.0 * k, color)));
}

/// 12×12 pixel pokéball.
pub fn pokeball(p: &Painter, c: Pos2, size: f32, ink: Color32, red: Color32, white: Color32) {
    let r = size / 2.0;
    p.circle_filled(c, r, white);
    let mut half = vec![];
    for i in 0..=24 {
        let a = std::f32::consts::PI + i as f32 / 24.0 * std::f32::consts::PI;
        half.push(pos2(c.x + a.cos() * r, c.y + a.sin() * r));
    }
    p.add(Shape::convex_polygon(half, red, Stroke::NONE));
    p.rect_filled(Rect::from_center_size(c, vec2(size, size * 0.13)), 0.0, ink);
    p.circle_stroke(c, r, Stroke::new(1.0, ink));
    p.circle_filled(c, size * 0.19, white);
    p.circle_stroke(c, size * 0.19, Stroke::new(1.0, ink));
}

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    Primary,
    Secondary,
    Soft,
    Mini,
    MiniMuted,
}

pub fn button(ui: &mut Ui, th: &Theme, label: &str, kind: Btn, enabled: bool) -> Response {
    let small = matches!(kind, Btn::Mini | Btn::MiniMuted);
    let (size, role, pad_x, h) = if small { (12.8, Role::SansB, 12.0, 32.0) } else { (13.44, Role::SansB, 14.0, 36.0) };
    let tw = text_width(ui, th, label, role, size);
    let (rect, resp) = ui.allocate_exact_size(vec2(tw + pad_x * 2.0 + 2.0, h), if enabled { Sense::click() } else { Sense::hover() });
    if enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
    }
    let hovered = enabled && resp.hovered();
    let pressed = enabled && resp.is_pointer_button_down_on();
    let p = ui.painter();
    let off = if pressed { 1.0 } else { 0.0 };
    let rect = rect.translate(vec2(0.0, off));
    let dim = |c: Color32| if enabled { c } else { rgba(c, 0.45 * (c.a() as f32 / 255.0)) };

    let (mut bg, mut border, mut fg, mut radius) = match kind {
        Btn::Primary => (if hovered { th.accent_dark } else { th.accent }, Color32::TRANSPARENT, th.on_accent, th.radius),
        Btn::Secondary => (th.secondary, if hovered { th.line_strong } else { Color32::TRANSPARENT }, th.text, th.radius),
        Btn::Soft => (if hovered { th.surface } else { th.accent_soft }, th.accent, th.accent_dark, th.radius),
        Btn::Mini => (if hovered { th.accent_soft } else { Color32::TRANSPARENT }, if hovered { th.accent } else { th.line }, th.accent, th.radius),
        Btn::MiniMuted => (if hovered { th.accent_soft } else { Color32::TRANSPARENT }, if hovered { th.accent } else { th.line }, th.muted, th.radius),
    };
    let mut ledge = None::<Shape>;
    let mut glow = None::<Color32>;
    let mut border_w = 1.0;
    match th.palette {
        Palette::Genshin => {
            radius = pill();
            match kind {
                Btn::Primary => {
                    border = rgba(th.gold, 0.55);
                    if hovered {
                        bg = crate::theme::mix(0.9, th.accent, Color32::WHITE);
                        border = th.gold;
                        glow = Some(rgba(th.gold, 0.38));
                    }
                    if pressed {
                        bg = crate::theme::mix(0.78, th.gold, th.accent);
                    } else {
                        ledge = Some(Shape::rect_filled(rect.translate(vec2(0.0, 2.0)), cr(radius), Color32::from_black_alpha(56)));
                    }
                }
                Btn::Secondary => {
                    // On the dark navy surface a cream wash leaves cream text
                    // on a cream plate. Use a dark translucent plate instead,
                    // with a bright rim, so the label stays legible.
                    if th.dark {
                        bg = Color32::from_black_alpha(if hovered { 72 } else { 46 });
                        border = if hovered { th.gold } else { rgba(th.accent, 0.62) };
                    } else {
                        bg = rgba(th.accent, if hovered { 0.24 } else { 0.14 });
                        border = if hovered { th.gold } else { rgba(th.accent, 0.52) };
                    }
                    fg = th.text;
                    if hovered {
                        glow = Some(rgba(th.gold, 0.34));
                    }
                }
                Btn::Soft => {
                    // "Show calculations" style: outlined gold pill on a dark
                    // plate in dark mode, warm tinted plate in light mode.
                    if th.dark {
                        bg = Color32::from_black_alpha(if hovered { 78 } else { 52 });
                        border = if hovered { th.gold } else { rgba(th.gold, 0.80) };
                    } else {
                        bg = rgba(th.gold, if hovered { 0.34 } else { 0.20 });
                        border = if hovered { th.gold } else { rgba(th.gold, 0.70) };
                    }
                    fg = th.text;
                    if hovered {
                        glow = Some(rgba(th.gold, 0.36));
                    }
                }
                Btn::Mini | Btn::MiniMuted => {
                    border = if hovered { th.gold } else { rgba(th.gold, 0.78) };
                    bg = if hovered { rgba(th.gold, 0.22) } else { Color32::TRANSPARENT };
                    if hovered {
                        glow = Some(rgba(th.gold, 0.30));
                    }
                }
            }
        }
        Palette::Pokemon => {
            // Let's Go: soft rounded pills, milky plates and a red CTA.
            radius = pill();
            border_w = 2.0;
            if enabled && !pressed {
                ledge = Some(Shape::from(
                    Shadow {
                        offset: [0, 2],
                        blur: 5,
                        spread: 0,
                        color: if th.dark { Color32::from_black_alpha(110) } else { rgba(th.ink, 0.18) },
                    }
                    .as_shape(rect, cr(radius)),
                ));
            }
            match kind {
                Btn::Primary => {
                    bg = if hovered { th.accent_dark } else { th.accent };
                    border = if th.dark { rgba(th.text, 0.28) } else { Color32::from_white_alpha(210) };
                    fg = th.on_accent;
                }
                Btn::Secondary => {
                    bg = if hovered { th.surface2 } else { th.surface };
                    border = th.ring;
                    fg = th.text;
                }
                Btn::Soft => {
                    bg = if hovered { th.surface } else { th.surface2 };
                    border = th.line_strong;
                    fg = th.text;
                }
                Btn::Mini => {
                    bg = if hovered { th.surface2 } else { Color32::TRANSPARENT };
                    border = if hovered { th.ring } else { th.line };
                    fg = th.ring;
                }
                Btn::MiniMuted => {
                    bg = if hovered { th.surface2 } else { Color32::TRANSPARENT };
                    border = if hovered { th.ring } else { th.line };
                    fg = th.muted;
                }
            }
        }
        Palette::Default => {}
    }
    if let Some(s) = ledge {
        p.add(s);
    }
    if let Some(g) = glow {
        p.rect_stroke(rect.expand(1.5), cr(radius + 1.5), Stroke::new(3.0, g), StrokeKind::Middle);
    }
    fill(p, rect, radius, dim(bg));
    stroke_in(p, rect, radius, border_w, dim(border));
    if th.palette == Palette::Genshin && matches!(kind, Btn::Primary) && !pressed {
        p.line_segment([rect.left_top() + vec2(radius.min(rect.height() / 2.0), 1.0), rect.right_top() + vec2(-radius.min(rect.height() / 2.0), 1.0)], Stroke::new(1.0, Color32::from_white_alpha(110)));
    }
    text(ui, th, rect.center(), Align2::CENTER_CENTER, label, role, size, dim(fg));
    resp
}

// ---------------------------------------------------------------------------
// Fields
// ---------------------------------------------------------------------------

pub const FIELD_H: f32 = 40.0;
pub const LABEL_H: f32 = 13.0;
pub const LABEL_GAP: f32 = 7.0;

/// Uppercase caption above a control.
pub fn label(ui: &Ui, th: &Theme, pos: Pos2, s: &str) {
    text_ls(ui, th, pos, Align2::LEFT_TOP, &s.to_uppercase(), Role::SansB, 10.88, th.muted, 0.03);
}

/// Number field at an exact rectangle. Returns whether the text changed.
pub fn field(ui: &mut Ui, th: &Theme, rect: Rect, id: Id, value: &mut String, hint: &str, invalid: bool) -> bool {
    // Record every field in draw order so Tab / Enter can walk them predictably.
    ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_default::<Vec<(Id, Rect)>>(Id::new("haem-field-order"))
            .push((id, rect));
    });
    let focused = ui.memory(|m| m.has_focus(id));
    let outer = ui.interact(rect, id.with("outer"), Sense::click());
    if outer.clicked() {
        ui.memory_mut(|m| m.request_focus(id));
    }
    if outer.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }
    let p = ui.painter().clone();
    let pk = th.palette == Palette::Pokemon;
    let r = if pk { 12.0 } else { th.radius };
    let (mut bg, mut border) = if focused { (th.surface, th.accent) } else { (th.bg, th.line_strong) };
    if pk {
        bg = th.field;
        border = if focused { th.ring } else { th.line_strong };
    }
    if th.palette == Palette::Genshin && focused {
        border = th.gold;
    }
    if invalid {
        border = th.bad;
        bg = th.bad_bg;
    }
    if focused {
        let ring = match th.palette {
            Palette::Genshin => rgba(th.gold, 0.30),
            Palette::Pokemon => rgba(th.ring, 0.30),
            _ => rgba(th.ring, 0.25),
        };
        p.rect_stroke(rect.expand(1.5), cr(r + 1.5), Stroke::new(3.0, ring), StrokeKind::Middle);
    }
    fill(&p, rect, r, bg);
    if pk && !focused {
        // soft top highlight instead of the old pixel inset
        p.line_segment(
            [rect.left_top() + vec2(8.0, 1.0), rect.right_top() + vec2(-8.0, 1.0)],
            Stroke::new(1.0, rgba(th.line_strong, 0.35)),
        );
    }
    stroke_in(&p, rect, r, if pk { 1.5 } else { 1.0 }, border);

    let inner = Rect::from_min_max(rect.min + vec2(11.0, 1.0), rect.max - vec2(11.0, 1.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)));
    let font = th.font(Role::Mono, 14.72);
    let before = value.clone();
    child.add(
        TextEdit::singleline(value)
            .id(id)
            .frame(false)
            .lock_focus(true)
            .margin(Margin::ZERO)
            .desired_width(inner.width())
            .font(font)
            .text_color(th.text)
            .hint_text(RichText::new(hint).color(th.muted))
            .vertical_align(Align::Center),
    );
    value.retain(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'));
    *value != before
}

/// Caption + field stacked, occupying `rect.min` .. `rect.min + (w, 60)`.
pub fn labeled_field(ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, caption: &str, id: Id, value: &mut String, hint: &str, invalid: bool) -> bool {
    label(ui, th, pos, caption);
    let r = Rect::from_min_size(pos + vec2(0.0, LABEL_H + LABEL_GAP), vec2(w, FIELD_H));
    field(ui, th, r, id, value, hint, invalid)
}

// ---------------------------------------------------------------------------
// Pills, badges, notes
// ---------------------------------------------------------------------------

/// Rounded chip with text; returns its rect. `anchor` decides how `pos` is interpreted.
#[allow(clippy::too_many_arguments)]
pub fn chip(ui: &Ui, th: &Theme, pos: Pos2, anchor: Align2, s: &str, role: Role, size: f32, fg: Color32, bg: Color32, border: Option<(f32, Color32)>, pad: Vec2, radius: f32) -> Rect {
    let g = galley(ui, th, s, role, size, fg, 0.0, f32::INFINITY);
    let size_v = g.size() + pad * 2.0 + vec2(0.0, 0.0);
    let rect = anchor.anchor_size(pos, size_v);
    let p = ui.painter();
    fill(p, rect, radius, bg);
    if let Some((w, c)) = border {
        stroke_in(p, rect, radius, w, c);
    }
    paint_galley(p, th, role, rect.min + pad, g, fg);
    rect
}

/// Status dot + label ("Acceptable" etc.), right-aligned at `right_mid`.
pub fn badge(ui: &Ui, th: &Theme, right_mid: Pos2, s: &str, color: Color32) -> f32 {
    let g = galley(ui, th, s, Role::SansB, 11.52, color, 0.0, f32::INFINITY);
    let w = g.size().x + 12.0;
    let left = right_mid.x - w;
    let p = ui.painter();
    if th.palette == Palette::Pokemon {
        p.rect_filled(Rect::from_center_size(pos2(left + 3.0, right_mid.y), vec2(7.0, 7.0)), 2.0, color);
    } else {
        p.circle_filled(pos2(left + 3.0, right_mid.y), 3.0, color);
    }
    paint_galley(p, th, Role::SansB, pos2(left + 12.0, right_mid.y - g.size().y / 2.0), g, color);
    w
}

/// The warn-coloured "≈ …" note. Returns its size when drawn at `pos` (anchor given).
pub fn note(ui: &Ui, th: &Theme, pos: Pos2, anchor: Align2, s: &str, max_w: f32) -> Rect {
    let full = format!("≈\u{a0}{}", s);
    let mut job = LayoutJob::single_section(
        full,
        TextFormat { font_id: th.font(Role::SansB, 12.8), color: th.warn, line_height: Some(17.3), ..Default::default() },
    );
    job.wrap.max_width = (max_w - 20.0).max(40.0);
    let g = ui.fonts(|f| f.layout_job(job));
    let size = g.size() + vec2(20.0, 8.0);
    let rect = anchor.anchor_size(pos, size);
    let r = if th.palette == Palette::Pokemon { 12.0 } else { 6.0 };
    fill(ui.painter(), rect, r, th.warn_bg);
    stroke_in(ui.painter(), rect, r, 1.0, th.warn);
    paint_galley(ui.painter(), th, Role::SansB, rect.min + vec2(10.0, 4.0), g, th.warn);
    rect
}

pub fn note_size(ui: &Ui, th: &Theme, s: &str, max_w: f32) -> Vec2 {
    let full = format!("≈\u{a0}{}", s);
    let mut job = LayoutJob::single_section(
        full,
        TextFormat { font_id: th.font(Role::SansB, 12.8), color: th.warn, line_height: Some(17.3), ..Default::default() },
    );
    job.wrap.max_width = (max_w - 20.0).max(40.0);
    ui.fonts(|f| f.layout_job(job)).size() + vec2(20.0, 8.0)
}

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

pub struct CardOpts<'a> {
    pub title: &'a str,
    pub unit: &'a str,
    pub bar: Option<Color32>,
    pub head_bg: Option<Color32>,
    pub title_color: Option<Color32>,
    /// Right-hand chip text; `big` selects the pill style used on FBC cards.
    pub chip: Option<(String, bool)>,
    pub min_height: f32,
}

pub const HEAD_H: f32 = 57.0;

pub fn card_shadow(p: &Painter, th: &Theme, rect: Rect, r: f32) -> Shape {
    if th.palette == Palette::Pokemon {
        return Shape::from(
            Shadow {
                offset: [0, 3],
                blur: 9,
                spread: 0,
                color: if th.dark { Color32::from_black_alpha(120) } else { rgba(th.ink, 0.18) },
            }
            .as_shape(rect, cr(r)),
        );
    }
    let c = match (th.palette, th.dark) {
        (Palette::Genshin, false) => Color32::from_rgba_unmultiplied(59, 71, 92, 36),
        (_, true) => Color32::from_black_alpha(90),
        _ => Color32::from_rgba_unmultiplied(61, 56, 38, 20),
    };
    let _ = p;
    Shape::from(Shadow { offset: [0, 1], blur: 3, spread: 0, color: c }.as_shape(rect, cr(r)))
}

/// Draws a card at `rect.min` with the given width and lets `body` fill it.
/// Returns the final rectangle (the body decides the height, `min_height` stretches).
pub fn card(ui: &mut Ui, th: &Theme, min: Pos2, width: f32, o: &CardOpts, body: impl FnOnce(&mut Ui, f32)) -> Rect {
    let r = th.radius_lg;
    let p = ui.painter().clone();
    let shadow_i = p.add(Shape::Noop);
    let bg_i = p.add(Shape::Noop);
    let band_i = p.add(Shape::Noop);
    let bar_i = p.add(Shape::Noop);

    let bl = if o.bar.is_some() { 4.0 } else { 1.0 };
    let head_y = min.y;
    let title_c = o.title_color.unwrap_or(th.text);
    // header text
    let tx = min.x + bl + 20.0;
    let mid_y = head_y + 18.0 + 12.0;
    let t_rect = text_ls(ui, th, pos2(tx, mid_y), Align2::LEFT_CENTER, o.title, Role::Display, 16.0, title_c, if th.palette == Palette::Genshin { -0.02 } else if th.palette == Palette::Pokemon { 0.0 } else { -0.01 });
    if !o.unit.is_empty() {
        text(ui, th, pos2(t_rect.right() + 8.0, mid_y + 1.5), Align2::LEFT_CENTER, o.unit, Role::Sans, 11.84, th.muted);
    }
    if let Some((s, big)) = &o.chip {
        let right = min.x + width - 1.0 - 20.0;
        if *big {
            chip(ui, th, pos2(right, mid_y), Align2::RIGHT_CENTER, s, Role::MonoB, 16.0, th.accent_dark, th.accent_soft, Some((1.0, th.accent_dark)), vec2(11.0, 2.0), pill());
        } else {
            text(ui, th, pos2(right, mid_y), Align2::RIGHT_CENTER, s, Role::MonoB, 11.84, th.muted);
        }
    }

    // body
    let inner_w = width - bl - 1.0 - 40.0;
    let body_rect = Rect::from_min_size(pos2(min.x + bl + 20.0, head_y + HEAD_H + 20.0), vec2(inner_w, 10_000.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(body_rect).layout(Layout::top_down(Align::Min)));
    body(&mut child, inner_w);
    let used_bottom = child.min_rect().bottom().max(body_rect.top());
    let h = (used_bottom + 20.0 + 1.0 - min.y).max(o.min_height);
    let rect = Rect::from_min_size(min, vec2(width, h));

    // backgrounds (set now that the height is known)
    let full = |c: Color32| Shape::rect_filled(rect, cr(r), c);
    p.set(shadow_i, card_shadow(&p, th, rect, r));
    p.set(bg_i, full(th.surface));
    if let Some(hb) = o.head_bg {
        let head_rect = Rect::from_min_size(min, vec2(width, HEAD_H));
        p.with_clip_rect(head_rect).set(band_i, full(hb));
    }
    if let Some(b) = o.bar {
        let bar_rect = Rect::from_min_size(min, vec2(4.0, h));
        p.with_clip_rect(bar_rect).set(bar_i, full(b));
    }
    // border + inner frame + header divider
    let border = if th.palette == Palette::Pokemon {
        if th.dark { rgba(th.text, 0.24) } else { Color32::from_white_alpha(235) }
    } else {
        th.line
    };
    let border_w = if th.palette == Palette::Pokemon { 2.5 } else { 1.0 };
    if o.bar.is_none() || th.palette == Palette::Pokemon {
        stroke_in(&p, rect, r, border_w, border);
    } else {
        // the left edge is the coloured bar; draw the other three sides
        let c = border;
        p.rect_stroke(rect, cr(r), Stroke::new(1.0, c), StrokeKind::Inside);
        p.with_clip_rect(Rect::from_min_size(min, vec2(4.0, h))).rect_filled(rect, cr(r), o.bar.unwrap());
    }
    if th.palette == Palette::Pokemon {
        stroke_in(&p, rect.shrink(2.5), r - 2.5, 1.0, rgba(th.ring, 0.20));
    }
    let divider_y = min.y + HEAD_H - 0.5;
    let x0 = min.x + bl;
    let x1 = min.x + width - 1.0;
    match th.palette {
        Palette::Pokemon => {
            p.line_segment([pos2(min.x + 20.0, divider_y), pos2(min.x + width - 20.0, divider_y)], Stroke::new(1.5, rgba(th.line_strong, 0.55)));
        }
        Palette::Genshin => {
            p.line_segment([pos2(min.x + 20.0, divider_y), pos2(min.x + width - 20.0, divider_y)], Stroke::new(1.0, rgba(th.gold, 0.65)));
            diamond(&p, pos2(min.x + 14.0 + 3.5, divider_y), 3.5, th.gold, Stroke::NONE);
            diamond(&p, pos2(min.x + width - 14.0 - 3.5, divider_y), 3.5, th.gold, Stroke::NONE);
        }
        Palette::Default => {
            p.line_segment([pos2(x0, divider_y), pos2(x1, divider_y)], Stroke::new(1.0, th.line));
        }
    }
    rect
}

/// Plain horizontal divider-less panel used by the summary bar.
pub fn summary_bar(ui: &mut Ui, th: &Theme, min: Pos2, width: f32, status: &str, kind: crate::logic::Status) -> Rect {
    let h = 61.0;
    let rect = Rect::from_min_size(min, vec2(width, h));
    let p = ui.painter().clone();
    let r = th.radius_lg;
    let (bg, border, fg) = match th.palette {
        Palette::Genshin => (crate::theme::hex(0x3a4556), Color32::from_white_alpha(20), crate::theme::hex(0xe4e6dc)),
        Palette::Pokemon => (th.surface, if th.dark { rgba(th.text, 0.24) } else { Color32::from_white_alpha(235) }, th.text),
        _ => (th.surface, th.line, th.text),
    };
    p.add(card_shadow(&p, th, rect, r));
    fill(&p, rect, r, bg);
    stroke_in(&p, rect, r, if th.palette == Palette::Pokemon { 2.5 } else { 1.0 }, border);
    if th.palette == Palette::Genshin {
        diamond(&p, pos2(rect.left() + 10.5, rect.center().y), 3.5, crate::theme::hex(0xcbba8c), Stroke::NONE);
    }
    if th.palette == Palette::Pokemon {
        pokeball(&p, pos2(rect.left() + 13.0, rect.center().y), 12.0, th.ink, th.accent, Color32::WHITE);
    }
    let label_x = if th.palette == Palette::Pokemon { rect.left() + 27.0 } else { rect.left() + 20.0 };
    text(ui, th, pos2(label_x, rect.center().y), Align2::LEFT_CENTER, "Overall status", Role::SansB, 14.72, fg);
    let (pf, pb) = match kind {
        crate::logic::Status::Ok => (th.ok, th.ok_bg),
        crate::logic::Status::Bad => (th.bad, th.bad_bg),
        crate::logic::Status::Pending => (th.neutral, th.neutral_bg),
    };
    chip(ui, th, pos2(rect.right() - 20.0, rect.center().y), Align2::RIGHT_CENTER, status, Role::SansB, 12.48, pf, pb, None, vec2(13.0, 5.0), pill());
    rect
}
