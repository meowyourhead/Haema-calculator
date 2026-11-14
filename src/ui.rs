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

/// Vertical gradient strip (top colour to bottom colour).
pub fn v_gradient(p: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(rect.left_top(), top);
    m.colored_vertex(rect.right_top(), top);
    m.colored_vertex(rect.right_bottom(), bottom);
    m.colored_vertex(rect.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
}

/// The Cyberpunk "alert" treatment for anything flagged: a solid flag-red
/// block with dark type, plus a cyan copy knocked a few pixels off-register
/// like the game's glitching hostile tags. Everyday UI red stays the softer
/// salmon, so this is the only loud red on screen.
pub fn cyber_alert_block(p: &Painter, th: &Theme, rect: Rect, cut: f32) {
    cyber_stroke(p, rect.translate(vec2(-2.5, 2.0)), cut, 1.0, rgba(th.ring, 0.85));
    cyber_fill(p, rect, cut, th.bad);
}

/// Cyan L-brackets in the two square corners: the game's selection cursor.
pub fn cyber_focus_box(p: &Painter, th: &Theme, rect: Rect) {
    let st = Stroke::new(2.0_f32, th.ring);
    let a = 7.0;
    let tl = rect.left_top() + vec2(3.0, 3.0);
    p.line_segment([tl, tl + vec2(a, 0.0)], st);
    p.line_segment([tl, tl + vec2(0.0, a)], st);
    let br = rect.right_bottom() - vec2(3.0, 3.0);
    p.line_segment([br, br - vec2(a, 0.0)], st);
    p.line_segment([br, br - vec2(0.0, a)], st);
}

pub fn fill(p: &Painter, rect: Rect, r: f32, c: Color32) {
    p.rect_filled(rect, cr(r), c);
}

pub fn stroke_in(p: &Painter, rect: Rect, r: f32, w: f32, c: Color32) {
    p.rect_stroke(rect, cr(r), Stroke::new(w, c), StrokeKind::Inside);
}

/// Octagonal silhouette with the top-right and bottom-left corners clipped.
/// This is the Cyberpunk panel/button shape; other palettes keep rounded rects.
pub fn chamfer_points(rect: Rect, cut: f32) -> Vec<Pos2> {
    let c = cut.min(rect.width() * 0.30).min(rect.height() * 0.40).max(0.0);
    vec![
        pos2(rect.left(), rect.top()),
        pos2(rect.right() - c, rect.top()),
        pos2(rect.right(), rect.top() + c),
        pos2(rect.right(), rect.bottom()),
        pos2(rect.left() + c, rect.bottom()),
        pos2(rect.left(), rect.bottom() - c),
    ]
}

pub fn cyber_fill(p: &Painter, rect: Rect, cut: f32, c: Color32) {
    p.add(Shape::convex_polygon(chamfer_points(rect, cut), c, Stroke::NONE));
}

pub fn cyber_stroke(p: &Painter, rect: Rect, cut: f32, w: f32, c: Color32) {
    p.add(Shape::closed_line(chamfer_points(rect, cut), Stroke::new(w, c)));
}

/// Filled + outlined Cyberpunk panel.
pub fn cyber_panel(p: &Painter, rect: Rect, cut: f32, fill_c: Color32, border_w: f32, border_c: Color32) {
    cyber_fill(p, rect, cut, fill_c);
    if border_w > 0.0 && border_c != Color32::TRANSPARENT {
        cyber_stroke(p, rect, cut, border_w, border_c);
    }
}

/// Header band of a chamfered card: follows the top-right cut, square below.
pub fn cyber_band(rect: Rect, cut: f32, h: f32, c: Color32) -> Shape {
    let top = rect.top();
    let right = rect.right();
    let left = rect.left();
    Shape::convex_polygon(
        vec![
            pos2(left, top),
            pos2(right - cut, top),
            pos2(right, top + cut),
            pos2(right, top + h),
            pos2(left, top + h),
        ],
        c,
        Stroke::NONE,
    )
}

/// Left colour bar of a chamfered card: follows the bottom-left cut.
pub fn cyber_bar(rect: Rect, cut: f32, w: f32, c: Color32) -> Shape {
    let left = rect.left();
    let top = rect.top();
    let bottom = rect.bottom();
    Shape::convex_polygon(
        vec![
            pos2(left, top),
            pos2(left + w, top),
            pos2(left + w, bottom),
            pos2(left + cut, bottom),
            pos2(left, bottom - cut),
        ],
        c,
        Stroke::NONE,
    )
}

pub fn diamond(p: &Painter, c: Pos2, half: f32, fill_c: Color32, stroke: Stroke) {
    p.add(Shape::convex_polygon(
        vec![pos2(c.x, c.y - half), pos2(c.x + half, c.y), pos2(c.x, c.y + half), pos2(c.x - half, c.y)],
        fill_c,
        stroke,
    ));
}

/// Horizontal gradient strip (the feathered tail of a Genshin settings row).
pub fn h_gradient(p: &Painter, rect: Rect, left: Color32, right: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(rect.left_top(), left);
    m.colored_vertex(rect.right_top(), right);
    m.colored_vertex(rect.right_bottom(), right);
    m.colored_vertex(rect.left_bottom(), left);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
}

/// The Genshin dialog frame: an inset champagne hairline plus the small
/// L-shaped corner brackets with diamond terminals that every in-game popup
/// (Replenish Resin, Craft, Edit Nickname) carries at its four corners.
pub fn genshin_frame(p: &Painter, th: &Theme, rect: Rect, r: f32) {
    let gold = th.gold;
    let inset = rect.shrink(4.0);
    stroke_in(p, inset, (r - 3.0).max(1.0), 1.0, rgba(gold, if th.dark { 0.34 } else { 0.55 }));
    let arm = 11.0;
    let st = Stroke::new(1.5_f32, gold);
    let o = 1.5; // brackets sit just outside the panel edge
    for (cx, cy, sx, sy) in [
        (rect.left() - o, rect.top() - o, 1.0, 1.0),
        (rect.right() + o, rect.top() - o, -1.0, 1.0),
        (rect.left() - o, rect.bottom() + o, 1.0, -1.0),
        (rect.right() + o, rect.bottom() + o, -1.0, -1.0),
    ] {
        let c = pos2(cx, cy);
        p.line_segment([c + vec2(3.0 * sx, 0.0), c + vec2(arm * sx, 0.0)], st);
        p.line_segment([c + vec2(0.0, 3.0 * sy), c + vec2(0.0, arm * sy)], st);
        diamond(p, c, 2.6, gold, Stroke::NONE);
    }
}

/// Glyph inside a Genshin button's leading icon disc.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GIcon {
    Confirm,
    Cancel,
    Plus,
    Minus,
}

/// The circular controller-prompt disc on the left of every Genshin button:
/// a gold ring for confirm-type actions and a pale-blue ✕ for cancel/reset.
pub fn genshin_icon(p: &Painter, c: Pos2, r: f32, icon: GIcon, plate: Color32) {
    match icon {
        GIcon::Cancel => {
            p.circle_filled(c, r, crate::theme::hex(0x6a9fe0));
            let k = r * 0.42;
            let st = Stroke::new(1.6_f32, Color32::WHITE);
            p.line_segment([c + vec2(-k, -k), c + vec2(k, k)], st);
            p.line_segment([c + vec2(-k, k), c + vec2(k, -k)], st);
        }
        _ => {
            p.circle_filled(c, r, crate::theme::hex(0xf5c443));
            let ink = crate::theme::hex(0x3b4255);
            match icon {
                GIcon::Plus | GIcon::Minus => {
                    let k = r * 0.5;
                    let st = Stroke::new(1.8_f32, ink);
                    p.line_segment([c + vec2(-k, 0.0), c + vec2(k, 0.0)], st);
                    if icon == GIcon::Plus {
                        p.line_segment([c + vec2(0.0, -k), c + vec2(0.0, k)], st);
                    }
                }
                _ => {
                    p.circle_filled(c, r * 0.55, plate);
                }
            }
        }
    }
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

/// Small chamfered data-chip used by the Cyberpunk palette swatch.
pub fn cyber_chip(p: &Painter, c: Pos2, size: f32, fill: Color32, stroke: Color32, dot: Color32) {
    let h = size / 2.0;
    let cut = (size * 0.26).max(1.0);
    let pts = vec![
        pos2(c.x - h, c.y - h),
        pos2(c.x + h - cut, c.y - h),
        pos2(c.x + h, c.y - h + cut),
        pos2(c.x + h, c.y + h),
        pos2(c.x - h, c.y + h),
    ];
    p.add(Shape::convex_polygon(pts, fill, Stroke::new(1.0_f32, stroke)));
    p.rect_filled(Rect::from_center_size(c, vec2(size * 0.30, size * 0.30)), 0.0, dot);
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
    p.circle_stroke(c, r, Stroke::new(1.0_f32, ink));
    p.circle_filled(c, size * 0.19, white);
    p.circle_stroke(c, size * 0.19, Stroke::new(1.0_f32, ink));
}

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    Primary,
    Secondary,
    Mini,
    MiniMuted,
}

pub fn button(ui: &mut Ui, th: &Theme, label: &str, kind: Btn, enabled: bool) -> Response {
    let small = matches!(kind, Btn::Mini | Btn::MiniMuted);
    let label = if th.palette == Palette::Cyberpunk { label.to_uppercase() } else { label.to_string() };
    // Genshin buttons lead with a controller-prompt disc; a "+ " / "− " prefix
    // moves into that disc instead of staying in the label.
    let g_icon = if th.palette == Palette::Genshin && !small {
        Some(if label.starts_with("+ ") {
            GIcon::Plus
        } else if label.starts_with("− ") {
            GIcon::Minus
        } else if kind == Btn::Secondary {
            GIcon::Cancel
        } else {
            GIcon::Confirm
        })
    } else {
        None
    };
    let label = match g_icon {
        Some(GIcon::Plus | GIcon::Minus) => label.chars().skip(2).collect::<String>(),
        _ => label,
    };
    let label = label.as_str();
    let (size, role, pad_x, h) = if small { (12.8, Role::SansB, 12.0, 32.0) } else { (13.44, Role::SansB, 14.0, 36.0) };
    let ls = if th.palette == Palette::Cyberpunk { 0.05 } else { 0.0 };
    let tw = text_width(ui, th, label, role, size) + ls * size * label.chars().count() as f32;
    // Cyberpunk secondary actions lead with the cyan controller-prompt disc
    // of the game's BACK / NEXT / CLOSE buttons.
    let c_icon = th.palette == Palette::Cyberpunk && !small && kind != Btn::Primary;
    let icon_w = if g_icon.is_some() { 24.0 } else if c_icon { 18.0 } else { 0.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(tw + icon_w + pad_x * 2.0 + 2.0, h), if enabled { Sense::click() } else { Sense::hover() });
    if enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
    }
    let hovered = enabled && resp.hovered();
    let pressed = enabled && resp.is_pointer_button_down_on();
    crate::pikachu::perch(ui, resp.id, rect, crate::pikachu::PerchKind::Top, pressed);
    let p = ui.painter();
    let off = if pressed { 1.0 } else { 0.0 };
    let rect = rect.translate(vec2(0.0, off));
    // Disabled controls stay legible (they still say what they would do).
    let dim = |c: Color32| if enabled { c } else { rgba(c, 0.75 * (c.a() as f32 / 255.0)) };

    let (mut bg, mut border, mut fg, mut radius) = match kind {
        Btn::Primary => (if hovered { th.accent_dark } else { th.accent }, Color32::TRANSPARENT, th.on_accent, th.radius),
        Btn::Secondary => (th.secondary, if hovered { th.line_strong } else { Color32::TRANSPARENT }, th.text, th.radius),
        Btn::Mini => (if hovered { th.accent_soft } else { Color32::TRANSPARENT }, if hovered { th.accent } else { th.line }, th.accent, th.radius),
        Btn::MiniMuted => (if hovered { th.accent_soft } else { Color32::TRANSPARENT }, if hovered { th.accent } else { th.line }, th.muted, th.radius),
    };
    let mut ledge = None::<Shape>;
    let mut glow = None::<Color32>;
    let mut border_w = 1.0;
    match th.palette {
        Palette::Genshin => {
            // In-game dialog buttons: every action is the same pill and the
            // leading disc says what it does. Slate plates with cream type sit
            // on parchment (light); cream plates with slate type sit on the
            // dark slate screens. Hover is the game's soft white halo.
            radius = pill();
            let slate = crate::theme::hex(0x4a5366);
            let cream = crate::theme::hex(0xece5d8);
            let (plate, ink) = if th.dark { (cream, slate) } else { (slate, cream) };
            match kind {
                Btn::Primary | Btn::Secondary => {
                    bg = if pressed {
                        crate::theme::mix(0.86, plate, Color32::BLACK)
                    } else if hovered {
                        crate::theme::mix(0.90, plate, Color32::WHITE)
                    } else {
                        plate
                    };
                    border = if th.dark { rgba(Color32::WHITE, 0.55) } else { rgba(th.gold, 0.40) };
                    fg = ink;
                }
                Btn::Mini | Btn::MiniMuted => {
                    border = if hovered { th.gold } else { rgba(th.gold, 0.78) };
                    bg = if hovered { rgba(th.gold, 0.22) } else { Color32::TRANSPARENT };
                }
            }
            if hovered {
                glow = Some(rgba(Color32::WHITE, if th.dark { 0.40 } else { 0.60 }));
            }
            if enabled && !pressed && !matches!(kind, Btn::Mini | Btn::MiniMuted) {
                ledge = Some(Shape::from(
                    Shadow { offset: [0, 2], blur: 6, spread: 0, color: Color32::from_black_alpha(if th.dark { 90 } else { 46 }) }
                        .as_shape(rect, cr(radius)),
                ));
            }
        }
        Palette::Cyberpunk => {
            // Menu controls: a selected-row red bar for the main action,
            // dark red-outlined plates for everything else. No soft shadows.
            radius = 0.0;
            match kind {
                Btn::Primary => {
                    bg = if hovered { th.accent_dark } else { th.accent };
                    border = th.accent_dark;
                    fg = th.on_accent;
                    if hovered {
                        glow = Some(rgba(th.accent, 0.30));
                    }
                }
                Btn::Secondary => {
                    bg = if hovered { rgba(th.accent, 0.14) } else { rgba(th.surface, 0.80) };
                    border = if hovered { th.accent } else { rgba(th.accent, 0.62) };
                    fg = if hovered { th.text } else { th.accent };
                }
                Btn::Mini => {
                    bg = if hovered { rgba(th.accent, 0.14) } else { Color32::TRANSPARENT };
                    border = if hovered { th.accent } else { rgba(th.accent, 0.55) };
                    fg = if hovered { th.text } else { th.accent };
                }
                Btn::MiniMuted => {
                    bg = if hovered { rgba(th.text, 0.08) } else { Color32::TRANSPARENT };
                    border = if hovered { th.line_strong } else { th.line };
                    fg = th.muted;
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
        if th.palette == Palette::Cyberpunk {
            p.add(Shape::closed_line(
                chamfer_points(rect.expand(2.0), 8.0),
                Stroke::new(3.0_f32, g),
            ));
        } else {
            p.rect_stroke(rect.expand(1.5), cr(radius + 1.5), Stroke::new(3.0_f32, g), StrokeKind::Middle);
        }
    }
    if th.palette == Palette::Cyberpunk {
        let cut = if small { 5.0 } else { 6.0 };
        cyber_panel(p, rect, cut, dim(bg), border_w, dim(border));
    } else {
        fill(p, rect, radius, dim(bg));
        stroke_in(p, rect, radius, border_w, dim(border));
    }
    if let Some(icon) = g_icon {
        let c = pos2(rect.left() + 19.0, rect.center().y);
        let core = if bg.a() == 255 { bg } else { crate::theme::hex(0x4a5366) };
        genshin_icon(p, c, 9.0, icon, dim(core));
        text(ui, th, pos2(rect.left() + 36.0, rect.center().y), Align2::LEFT_CENTER, label, role, size, dim(fg));
    } else if c_icon {
        let c = pos2(rect.left() + 17.0, rect.center().y);
        p.circle_filled(c, 6.0, dim(th.ring));
        p.circle_filled(c, 2.6, dim(th.surface));
        text_ls(ui, th, pos2(rect.left() + 29.0, rect.center().y), Align2::LEFT_CENTER, label, role, size, dim(fg), ls);
    } else if th.palette == Palette::Cyberpunk {
        text_ls(ui, th, rect.center(), Align2::CENTER_CENTER, label, role, size, dim(fg), ls);
    } else {
        text(ui, th, rect.center(), Align2::CENTER_CENTER, label, role, size, dim(fg));
    }
    resp
}

// ---------------------------------------------------------------------------
// Fields
// ---------------------------------------------------------------------------

/// How a number field should draw attention to itself.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldState {
    Normal,
    /// The text is not a usable number.
    Invalid,
    /// The number parses but is the run that disagrees with the others.
    Flagged,
}

/// Number field at an exact rectangle. Returns whether the text changed.
pub fn field(ui: &mut Ui, th: &Theme, rect: Rect, id: Id, value: &mut String, hint: &str, invalid: bool) -> bool {
    field_s(ui, th, rect, id, value, hint, if invalid { FieldState::Invalid } else { FieldState::Normal })
}

/// `field` with an explicit attention state.
pub fn field_s(ui: &mut Ui, th: &Theme, rect: Rect, id: Id, value: &mut String, hint: &str, state: FieldState) -> bool {
    let invalid = state == FieldState::Invalid;
    let flagged = state == FieldState::Flagged;
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
    let cyber = th.palette == Palette::Cyberpunk;
    let gen = th.palette == Palette::Genshin;
    let r = if pk { 12.0 } else if cyber { 0.0 } else if gen { pill() } else { th.radius };
    // Focus uses the palette's info hue, never the accent (which reads as a flag).
    let (mut bg, mut border) = if focused { (th.surface, th.ring) } else { (th.bg, th.line_strong) };
    // Genshin entry boxes are the bright ivory capsule of "Edit Nickname":
    // ivory on parchment, cream on the slate screens, slate ink in both.
    let mut ink = th.text;
    let mut hint_c = th.muted;
    if gen {
        bg = if th.dark { crate::theme::hex(0xece5d8) } else { crate::theme::hex(0xf8f4ec) };
        border = if focused { th.gold } else if th.dark { rgba(Color32::WHITE, 0.0) } else { rgba(crate::theme::hex(0x4a5366), 0.26) };
        ink = crate::theme::hex(0x3b4255);
        hint_c = crate::theme::hex(0x7a7f8c);
    }
    if pk {
        bg = th.field;
        border = if focused { th.ring } else { th.line_strong };
    }
    if cyber {
        bg = th.field;
        border = if focused { th.ring } else { th.line_strong };
    }
    if invalid {
        border = th.bad;
        bg = th.bad_bg;
    }
    // Genshin's ivory capsules need a deep red; the dark palette's pastel flag
    // colour would vanish against them.
    let flag_c = if gen { crate::theme::hex(0xb03a30) } else { th.bad };
    if flagged {
        border = flag_c;
    }
    if focused && !cyber {
        let ring = match th.palette {
            Palette::Genshin => rgba(th.gold, 0.65),
            Palette::Pokemon => rgba(th.ring, 0.42),
            _ => rgba(th.ring, 0.40),
        };
        p.rect_stroke(rect.expand(1.5), cr(r + 1.5), Stroke::new(3.0_f32, ring), StrokeKind::Middle);
    }
    if cyber {
        // Dark outlined data cell; focus is the cyan selection cursor.
        if focused {
            bg = rgba(th.ring, 0.06);
        }
        cyber_panel(&p, rect, 4.0, th.field, 0.0, Color32::TRANSPARENT);
        cyber_panel(&p, rect, 4.0, bg, if flagged { 2.0 } else { 1.0 }, border);
        if focused {
            cyber_focus_box(&p, th, rect);
        }
    } else {
        fill(&p, rect, r, bg);
        if pk && !focused {
            // soft top highlight instead of the old pixel inset
            p.line_segment(
                [rect.left_top() + vec2(8.0, 1.0), rect.right_top() + vec2(-8.0, 1.0)],
                Stroke::new(1.0_f32, rgba(th.line_strong, 0.35)),
            );
        }
        if gen && !invalid {
            // faint inner shade along the top edge, like an inset capsule
            p.line_segment(
                [rect.left_top() + vec2(r.min(rect.height() / 2.0), 1.5), rect.right_top() + vec2(-r.min(rect.height() / 2.0), 1.5)],
                Stroke::new(1.0_f32, Color32::from_black_alpha(18)),
            );
        }
        stroke_in(&p, rect, r, if flagged { 2.0 } else if pk { 1.5 } else { 1.0 }, border);
    }
    let mut pad_r = 0.0;
    if flagged && cyber {
        // inverted alert tag on the cell's right edge
        let tag = Rect::from_min_max(pos2(rect.right() - 26.0, rect.top() + 7.0), pos2(rect.right() - 7.0, rect.bottom() - 7.0));
        cyber_alert_block(&p, th, tag, 4.0);
        status_glyph(&p, tag.center(), 6.0, crate::logic::Status::Bad, th.on_accent);
        pad_r = 22.0;
    } else if flagged {
        let gc = pos2(rect.right() - if gen { 20.0 } else { 14.0 }, rect.center().y);
        status_glyph(&p, gc, 6.0, crate::logic::Status::Bad, flag_c);
        pad_r = 14.0;
    }

    // Genshin capsules keep the text clear of their round ends; shorter
    // capsules have tighter ends, so the inset scales with height.
    let pad = if gen { (rect.height() * 0.45).min(18.0) } else { 11.0 };
    let inner = Rect::from_min_max(rect.min + vec2(pad, 1.0), rect.max - vec2(pad + pad_r, 1.0));
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
            .text_color(if invalid { th.text } else { ink })
            .hint_text(RichText::new(hint).color(hint_c))
            .vertical_align(Align::Center),
    );
    value.retain(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'));
    *value != before
}

// ---------------------------------------------------------------------------
// Status: shape + colour, so a result never depends on colour alone
// ---------------------------------------------------------------------------

/// (foreground, background) for a status in this palette.
pub fn status_fg_bg(th: &Theme, s: crate::logic::Status) -> (Color32, Color32) {
    use crate::logic::Status;
    match s {
        Status::Ok => (th.ok, th.ok_bg),
        Status::Bad => (th.bad, th.bad_bg),
        Status::Pending => (th.neutral, th.neutral_bg),
    }
}

/// A tick (acceptable), an exclamation mark (flagged) or an open ring (waiting),
/// drawn as strokes so it never depends on a font having the glyph.
pub fn status_glyph(p: &Painter, c: Pos2, r: f32, s: crate::logic::Status, color: Color32) {
    use crate::logic::Status;
    let w = (r * 0.30).max(1.5);
    match s {
        Status::Ok => {
            let pts = vec![c + vec2(-0.62 * r, 0.02 * r), c + vec2(-0.18 * r, 0.48 * r), c + vec2(0.66 * r, -0.46 * r)];
            p.add(Shape::line(pts, Stroke::new(w, color)));
        }
        Status::Bad => {
            p.line_segment([c + vec2(0.0, -0.70 * r), c + vec2(0.0, 0.18 * r)], Stroke::new(w, color));
            p.circle_filled(c + vec2(0.0, 0.62 * r), w * 0.62, color);
        }
        Status::Pending => {
            p.circle_stroke(c, 0.52 * r, Stroke::new(w * 0.75, color));
        }
    }
}

/// A status capsule: a solid disc holding the glyph, then the message.
/// `pos` is interpreted through `anchor`; returns the capsule rect.
#[allow(clippy::too_many_arguments)]
pub fn status_pill(ui: &Ui, th: &Theme, pos: Pos2, anchor: Align2, msg: &str, s: crate::logic::Status, h: f32, size: f32) -> Rect {
    let (mut fg, mut bg) = status_fg_bg(th, s);
    let alert = th.palette == Palette::Cyberpunk && s == crate::logic::Status::Bad;
    if alert {
        // inverted: dark type on the solid flag-red block
        (fg, bg) = (th.on_accent, th.bad);
    }
    let msg = if th.palette == Palette::Cyberpunk { msg.to_uppercase() } else { msg.to_string() };
    let g = galley(ui, th, &msg, Role::SansB, size, fg, if th.palette == Palette::Cyberpunk { 0.05 } else { 0.0 }, f32::INFINITY);
    let disc = h - 12.0;
    let pad_l = 6.0;
    let pad_r = (h * 0.42).max(10.0);
    let w = pad_l + disc + 8.0 + g.size().x + pad_r;
    let rect = anchor.anchor_size(pos, vec2(w, h));
    let p = ui.painter();
    if alert {
        cyber_alert_block(p, th, rect, 5.0);
    } else if th.palette == Palette::Cyberpunk {
        cyber_panel(p, rect, 5.0, bg, 1.5, rgba(fg, 0.85));
    } else {
        fill(p, rect, pill(), bg);
        stroke_in(p, rect, pill(), 1.5, rgba(fg, if s == crate::logic::Status::Pending { 0.45 } else { 0.80 }));
    }
    let dc = pos2(rect.left() + pad_l + disc / 2.0, rect.center().y);
    if s == crate::logic::Status::Pending {
        p.circle_stroke(dc, disc * 0.24, Stroke::new(1.5_f32, rgba(fg, 0.75)));
    } else {
        p.circle_filled(dc, disc / 2.0, fg);
        status_glyph(p, dc, disc * 0.36, s, bg);
    }
    paint_galley(p, th, Role::SansB, pos2(dc.x + disc / 2.0 + 8.0, rect.center().y - g.size().y / 2.0), g, fg);
    rect
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
    if th.palette == Palette::Cyberpunk {
        let (bw, bc) = border.unwrap_or((0.0, Color32::TRANSPARENT));
        cyber_panel(p, rect, 4.0, bg, bw, bc);
    } else {
        fill(p, rect, radius, bg);
        if let Some((w, c)) = border {
            stroke_in(p, rect, radius, w, c);
        }
    }
    paint_galley(p, th, role, rect.min + pad, g, fg);
    rect
}

/// Status glyph + label ("✓ Acceptable", "! Flagged"), right-aligned at `right_mid`.
pub fn badge(ui: &Ui, th: &Theme, right_mid: Pos2, s: &str, status: crate::logic::Status) -> f32 {
    let (color, _) = status_fg_bg(th, status);
    let color = if status == crate::logic::Status::Pending { th.muted } else { color };
    let label = if th.palette == Palette::Cyberpunk { s.to_uppercase() } else { s.to_string() };
    if th.palette == Palette::Cyberpunk && status == crate::logic::Status::Bad {
        let g = galley(ui, th, &label, Role::SansB, 12.0, th.on_accent, 0.05, f32::INFINITY);
        let w = g.size().x + 26.0;
        let rect = Rect::from_min_max(pos2(right_mid.x - w, right_mid.y - 10.0), pos2(right_mid.x, right_mid.y + 10.0));
        cyber_alert_block(ui.painter(), th, rect, 4.0);
        status_glyph(ui.painter(), pos2(rect.left() + 9.0, right_mid.y), 6.0, status, th.on_accent);
        paint_galley(ui.painter(), th, Role::SansB, pos2(rect.left() + 18.0, right_mid.y - g.size().y / 2.0), g, th.on_accent);
        return w;
    }
    let g = galley(ui, th, &label, Role::SansB, 12.0, color, 0.0, f32::INFINITY);
    let w = g.size().x + 17.0;
    let left = right_mid.x - w;
    status_glyph(ui.painter(), pos2(left + 5.0, right_mid.y), 6.0, status, color);
    paint_galley(ui.painter(), th, Role::SansB, pos2(left + 17.0, right_mid.y - g.size().y / 2.0), g, color);
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
    if th.palette == Palette::Cyberpunk {
        cyber_panel(ui.painter(), rect, 5.0, th.warn_bg, 1.0, th.warn);
        ui.painter().rect_filled(Rect::from_min_size(rect.min, vec2(2.0, rect.height())), 0.0, th.warn);
    } else {
        let r = if th.palette == Palette::Pokemon { 12.0 } else { 6.0 };
        fill(ui.painter(), rect, r, th.warn_bg);
        stroke_in(ui.painter(), rect, r, 1.0, th.warn);
    }
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
    /// A comparison in this card failed: flag-coloured edge and a "Review" tag.
    pub alert: bool,
    /// Headline status shown right after the title (replaces the "Review" tag).
    pub status: Option<(crate::logic::Status, &'a str)>,
    /// Draw only the header band (the body closure is not run).
    pub collapsed: bool,
}

pub const HEAD_H: f32 = 48.0;
/// Space between the header divider / card edge and the body.
pub const CARD_PAD: f32 = 16.0;

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
    if th.palette == Palette::Genshin {
        let c = if th.dark { Color32::from_black_alpha(110) } else { Color32::from_rgba_unmultiplied(40, 46, 60, 54) };
        return Shape::from(Shadow { offset: [0, 4], blur: 14, spread: 0, color: c }.as_shape(rect, cr(r)));
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
    let cyber = th.palette == Palette::Cyberpunk;
    let p = ui.painter().clone();
    let shadow_i = p.add(Shape::Noop);
    let bg_i = p.add(Shape::Noop);
    let band_i = p.add(Shape::Noop);
    let bar_i = p.add(Shape::Noop);

    let gen = th.palette == Palette::Genshin;
    // Genshin cards are framed dialogs; the parameter colour becomes a small
    // element gem beside the title instead of a coloured left edge.
    // Cyberpunk cards share one red frame; the parameter colour shrinks to a
    // small data-chip tag in front of a red heading.
    let bl = if o.bar.is_some() && !gen && !cyber { 4.0 } else { 1.0 };
    let head_y = min.y;
    let mid_y = head_y + HEAD_H / 2.0;
    let title_c = if cyber { th.accent } else { o.title_color.unwrap_or(th.text) };
    // header text
    let mut tx = min.x + bl + 20.0;
    if cyber {
        if let Some(b) = o.bar {
            cyber_chip(ui.painter(), pos2(tx + 5.0, mid_y), 11.0, b, b, rgba(th.surface, 0.85));
            tx += 18.0;
        }
    }
    if gen {
        if let Some(b) = o.bar {
            let gc = pos2(tx + 6.0, mid_y);
            diamond(ui.painter(), gc, 7.5, th.gold, Stroke::NONE);
            diamond(ui.painter(), gc, 5.5, b, Stroke::NONE);
            diamond(ui.painter(), gc + vec2(-1.2, -1.6), 1.6, rgba(Color32::WHITE, 0.70), Stroke::NONE);
            tx += 20.0;
        }
    }
    let title = if cyber { o.title.to_uppercase() } else { o.title.to_string() };
    let title_ls = match th.palette {
        Palette::Genshin => -0.02,
        Palette::Pokemon => 0.0,
        Palette::Cyberpunk => 0.055,
        Palette::Default => -0.01,
    };
    let t_rect = text_ls(ui, th, pos2(tx, mid_y), Align2::LEFT_CENTER, &title, Role::Display, if cyber { 15.5 } else { 16.0 }, title_c, title_ls);
    let mut head_right = t_rect.right();
    if !o.unit.is_empty() {
        let unit_role = if cyber { Role::Mono } else { Role::Sans };
        head_right = text(ui, th, pos2(t_rect.right() + 8.0, mid_y + 1.5), Align2::LEFT_CENTER, o.unit, unit_role, 11.84, th.muted).right();
    }
    if let Some((s, msg)) = o.status {
        status_pill(ui, th, pos2(head_right + 14.0, mid_y), Align2::LEFT_CENTER, msg, s, 28.0, 13.0);
    }
    let mut chip_left = min.x + width - 1.0 - 20.0;
    if let Some((s, big)) = &o.chip {
        let right = min.x + width - 1.0 - 20.0;
        let chip_r = if *big && gen {
            // level-tag style: slate capsule with cream type (light), a
            // smoked capsule with a gold rim on the dark screens
            let (fg, bg, bd) = if th.dark {
                (th.gold, Color32::from_black_alpha(70), rgba(th.gold, 0.70))
            } else {
                (crate::theme::hex(0xece5d8), crate::theme::hex(0x4a5366), rgba(th.gold, 0.0))
            };
            chip(ui, th, pos2(right, mid_y), Align2::RIGHT_CENTER, s, Role::MonoB, 15.0, fg, bg, Some((1.0, bd)), vec2(12.0, 3.0), pill())
        } else if *big {
            // The limit is reference information, so it is neutral: the
            // accent/red family is reserved for results that need review.
            chip(ui, th, pos2(right, mid_y), Align2::RIGHT_CENTER, s, Role::MonoB, 14.5, th.text, th.neutral_bg, Some((1.0, th.line_strong)), vec2(10.0, 2.0), pill())
        } else {
            text(ui, th, pos2(right, mid_y), Align2::RIGHT_CENTER, s, Role::MonoB, 11.84, th.muted)
        };
        chip_left = chip_r.left();
    }
    if o.alert && o.status.is_none() {
        status_pill(ui, th, pos2(chip_left - 8.0, mid_y), Align2::RIGHT_CENTER, "Review", crate::logic::Status::Bad, 26.0, 12.0);
    }

    // body
    let inner_w = width - bl - 1.0 - 40.0;
    let body_rect = Rect::from_min_size(pos2(min.x + bl + 20.0, head_y + HEAD_H + CARD_PAD), vec2(inner_w, 10_000.0));
    let h = if o.collapsed {
        HEAD_H
    } else {
        let mut child = ui.new_child(UiBuilder::new().max_rect(body_rect).layout(Layout::top_down(Align::Min)));
        body(&mut child, inner_w);
        let used_bottom = child.min_rect().bottom().max(body_rect.top());
        (used_bottom + CARD_PAD + 1.0 - min.y).max(o.min_height)
    };
    let rect = Rect::from_min_size(min, vec2(width, h));
    crate::pikachu::perch(ui, Id::new(("pika-card", o.title, width as i32)), rect, crate::pikachu::PerchKind::Top, false);

    // backgrounds (set now that the height is known)
    let cut = 9.0;
    // A flagged card trades its parameter colour for the flag colour on the edge.
    let bar = if o.alert && !gen { Some(th.bad) } else { o.bar };
    let full = |c: Color32| Shape::rect_filled(rect, cr(r), c);
    if cyber {
        p.set(shadow_i, Shape::convex_polygon(chamfer_points(rect.translate(vec2(0.0, 2.0)), cut), Color32::from_black_alpha(if th.dark { 120 } else { 32 }), Stroke::NONE));
        p.set(bg_i, Shape::convex_polygon(chamfer_points(rect, cut), if th.dark { rgba(th.surface, 0.90) } else { th.surface }, Stroke::NONE));
        // every card gets the same faint red title band
        let _ = o.head_bg;
        p.set(band_i, cyber_band(rect, cut, HEAD_H, rgba(th.accent, if th.dark { 0.045 } else { 0.05 })));
        if o.alert {
            p.set(bar_i, cyber_bar(rect, cut, 4.0, th.bad));
        }
    } else {
        p.set(shadow_i, card_shadow(&p, th, rect, r));
        p.set(bg_i, full(if gen && th.dark { rgba(th.surface, 0.94) } else { th.surface }));
        if let Some(hb) = o.head_bg {
            let head_rect = Rect::from_min_size(min, vec2(width, HEAD_H));
            p.with_clip_rect(head_rect).set(band_i, full(hb));
        } else if gen {
            // untinted cards still get the darker dialog title band
            let head_rect = Rect::from_min_size(min, vec2(width, HEAD_H));
            p.with_clip_rect(head_rect).set(band_i, full(th.surface2));
        }
        if let (Some(b), false) = (bar, gen) {
            let bar_rect = Rect::from_min_size(min, vec2(4.0, h));
            p.with_clip_rect(bar_rect).set(bar_i, full(b));
        }
    }
    // border + inner frame + header divider
    let border = match th.palette {
        Palette::Pokemon => {
            if th.dark { rgba(th.text, 0.24) } else { Color32::from_white_alpha(235) }
        }
        Palette::Cyberpunk => rgba(th.accent, if th.dark { 0.42 } else { 0.50 }),
        _ => th.line,
    };
    let border_w = if th.palette == Palette::Pokemon { 2.5 } else { 1.0 };
    if cyber {
        cyber_stroke(&p, rect, cut, 1.0, border);
        // The stacked red tabs that run down the left of every game list,
        // a short bright cap on the top edge and an L-bracket bottom-right.
        if !o.alert {
            for k in 0..3 {
                let y = rect.top() + 12.0 + k as f32 * 15.0;
                p.rect_filled(Rect::from_min_size(pos2(rect.left() - 4.0, y), vec2(3.0, 11.0)), 0.0, rgba(th.accent, 0.80 - 0.22 * k as f32));
            }
        }
        p.line_segment(
            [pos2(rect.left() + 1.0, rect.top() + 1.0), pos2(rect.left() + 46.0, rect.top() + 1.0)],
            Stroke::new(2.0_f32, th.accent),
        );
        p.line_segment(
            [pos2(rect.right() - 26.0, rect.bottom() + 3.0), pos2(rect.right() + 3.0, rect.bottom() + 3.0)],
            Stroke::new(1.0_f32, rgba(th.accent, 0.70)),
        );
        p.line_segment(
            [pos2(rect.right() + 3.0, rect.bottom() + 3.0), pos2(rect.right() + 3.0, rect.bottom() - 16.0)],
            Stroke::new(1.0_f32, rgba(th.accent, 0.70)),
        );
    } else if gen {
        stroke_in(&p, rect, r, 1.0, if th.dark { rgba(Color32::WHITE, 0.09) } else { rgba(th.gold, 0.45) });
        genshin_frame(&p, th, rect, r);
    } else if o.bar.is_none() || th.palette == Palette::Pokemon {
        stroke_in(&p, rect, r, border_w, border);
    } else {
        // the left edge is the coloured bar; draw the other three sides
        let c = border;
        p.rect_stroke(rect, cr(r), Stroke::new(1.0_f32, c), StrokeKind::Inside);
        p.with_clip_rect(Rect::from_min_size(min, vec2(4.0, h))).rect_filled(rect, cr(r), bar.unwrap());
    }
    if th.palette == Palette::Pokemon {
        stroke_in(&p, rect.shrink(2.5), r - 2.5, 1.0, rgba(th.ring, 0.20));
    }
    if o.alert {
        if cyber {
            cyber_stroke(&p, rect, cut, 2.0, th.bad);
        } else {
            stroke_in(&p, rect, r, 2.0, th.bad);
        }
    }
    if o.collapsed {
        return rect;
    }
    let divider_y = min.y + HEAD_H - 0.5;
    let x0 = min.x + bl;
    let x1 = min.x + width - 1.0;
    match th.palette {
        Palette::Pokemon => {
            p.line_segment([pos2(min.x + 20.0, divider_y), pos2(min.x + width - 20.0, divider_y)], Stroke::new(1.5_f32, rgba(th.line_strong, 0.55)));
        }
        Palette::Genshin => {
            p.line_segment([pos2(min.x + 20.0, divider_y), pos2(min.x + width - 20.0, divider_y)], Stroke::new(1.0_f32, rgba(th.gold, 0.65)));
            diamond(&p, pos2(min.x + 14.0 + 3.5, divider_y), 3.5, th.gold, Stroke::NONE);
            diamond(&p, pos2(min.x + width - 14.0 - 3.5, divider_y), 3.5, th.gold, Stroke::NONE);
        }
        Palette::Cyberpunk => {
            p.line_segment(
                [pos2(min.x + bl + 8.0, divider_y), pos2(min.x + width - 18.0, divider_y)],
                Stroke::new(1.0_f32, rgba(th.accent, 0.40)),
            );
            p.line_segment(
                [pos2(min.x + width - 18.0, divider_y - 3.0), pos2(min.x + width - 18.0, divider_y + 3.0)],
                Stroke::new(1.0_f32, th.accent),
            );
        }
        Palette::Default => {
            p.line_segment([pos2(x0, divider_y), pos2(x1, divider_y)], Stroke::new(1.0_f32, th.line));
        }
    }
    rect
}

