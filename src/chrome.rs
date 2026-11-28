//! Frameless window chrome: a Chromium-style tab strip that doubles as the title bar.
//!
//! The strip hosts the three page tabs (FBC with dilution, Retic, PT / APTT), a drag area and the
//! minimize / maximize / close buttons. There is no edge resizing: the window keeps the Retic
//! page's default size (see `App::fit_window`).
use crate::app::{App, Tab};
use crate::theme::{hex, rgba, Palette, Role, Theme};
use crate::ui::*;
use eframe::egui::epaint::StrokeKind;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Response, Sense, Stroke, Ui,
};

pub const CHROME_H: f32 = 44.0;
const TAB_H: f32 = 36.0;
const TAB_PAD: f32 = 20.0;
const TAB_GAP: f32 = 4.0;
const TAB_FONT: f32 = 14.5;
const BTN_W: f32 = 46.0;
const CLOSE_W: f32 = 62.0;

/// The page tabs, in strip order (also the Ctrl+Tab order).
pub const TABS: [(Tab, &str); 4] = [
    (Tab::Fbc, "FBC"),
    (Tab::Retic, "Retic"),
    (Tab::PtAptt, "PT / APTT"),
    (Tab::Hbel, "HBEL"),
];

/// Draw the title bar. Must be called before the central panel.
pub fn title_bar(app: &mut App, ctx: &egui::Context) {
    let th = app.theme();
    egui::TopBottomPanel::top("window-chrome")
        .exact_height(CHROME_H)
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(strip_bg(&th)))
        .show(ctx, |ui| paint_chrome(ui, app, &th));
}

struct TabDraw {
    rect: Rect,
    idx: usize,
    hovered: bool,
    active: bool,
}

fn tab_widths(ui: &Ui, th: &Theme) -> Vec<f32> {
    TABS.iter()
        .map(|(_, label)| text_width(ui, th, label, Role::SansB, TAB_FONT).ceil() + TAB_PAD * 2.0)
        .collect()
}

/// The settings (gear) button, as wide as a window button.
const SETTINGS_W: f32 = BTN_W;

/// Narrowest window that still fits the tabs, the Theme button and the
/// window buttons without overlap.
pub fn min_width(ui: &Ui, th: &Theme) -> f32 {
    let tabs = tab_widths(ui, th).iter().sum::<f32>() + TAB_GAP * (TABS.len() - 1) as f32;
    // Tabs slide left of centre when space is short, so this is the tightest fit.
    12.0 + tabs + 12.0 + CLOSE_W + BTN_W * 2.0 + SETTINGS_W
}

fn paint_chrome(ui: &mut Ui, app: &mut App, th: &Theme) {
    let rect = ui.max_rect();
    let p = ui.painter().clone();
    let tab_top = rect.bottom() - TAB_H;

    // --- tabs (centred, with generous hit targets) -----------------------
    let widths = tab_widths(ui, th);
    let total_w = widths.iter().sum::<f32>() + TAB_GAP * (TABS.len() - 1) as f32;
    let controls_left = rect.right() - (CLOSE_W + BTN_W * 2.0 + SETTINGS_W);
    let min_x = rect.left() + 12.0;
    let max_x = (controls_left - total_w - 12.0).max(min_x);
    let mut x = (rect.center().x - total_w / 2.0).clamp(min_x, max_x);

    let mut draws: Vec<TabDraw> = Vec::with_capacity(TABS.len());
    for (i, (tab, _label)) in TABS.iter().enumerate() {
        let w = widths[i];
        let r = Rect::from_min_size(pos2(x, tab_top), vec2(w, TAB_H));
        let resp = ui.interact(r, Id::new(("chrome-tab", i)), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            app.tab = *tab;
        }
        crate::pikachu::perch(ui, Id::new(("pika-tab", i)), r, crate::pikachu::PerchKind::Hang, resp.is_pointer_button_down_on());
        draws.push(TabDraw { rect: r, idx: i, hovered: resp.hovered(), active: app.tab == *tab });
        x += w + TAB_GAP;
    }
    let tabs_right = x;

    if th.palette == Palette::Genshin {
        paint_genshin_tabs(ui, th, rect, &draws);
    } else {
        paint_tabs(ui, th, rect, &p, &draws);
    }

    // --- window buttons --------------------------------------------------
    let btn_top = rect.top();
    let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
    let close = Rect::from_min_max(pos2(rect.right() - CLOSE_W, btn_top), pos2(rect.right(), btn_top + CHROME_H));
    let maximize = Rect::from_min_max(pos2(close.left() - BTN_W, btn_top), pos2(close.left(), btn_top + CHROME_H));
    let minimize = Rect::from_min_max(pos2(maximize.left() - BTN_W, btn_top), pos2(maximize.left(), btn_top + CHROME_H));
    let min_resp = caption(ui, th, minimize, "min", Caption::Minimize);
    let max_resp = caption(ui, th, maximize, "max", if maximized { Caption::Restore } else { Caption::Maximize });
    let close_resp = caption(ui, th, close, "close", Caption::Close);
    if min_resp.clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
    if max_resp.clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
    if close_resp.clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }

    // --- settings (gear): the theme / scale / Excel / updates popover ----
    let theme_btn = Rect::from_min_max(pos2(minimize.left() - SETTINGS_W, btn_top), pos2(minimize.left(), btn_top + CHROME_H));
    if settings_button(ui, th, theme_btn, app.menu_open).clicked() {
        app.menu_open = !app.menu_open;
    }
    app.menu_btn = theme_btn;

    // --- "Update available" (only once the silent check found a newer release)
    let mut drag_right = theme_btn.left();
    if let crate::update::Status::Available { tag, .. } = app.update.status() {
        let label = match app.update.install_status() {
            crate::update::Install::Downloading { done, total } if total > 0 => format!("Updating {}%", (done * 100 / total).min(100)),
            crate::update::Install::Downloading { .. } | crate::update::Install::Restarting => "Updating…".to_string(),
            _ => "Update available".to_string(),
        };
        let label = label.as_str();
        let uw = text_width(ui, th, "Update available", Role::SansB, 12.0).ceil().max(text_width(ui, th, label, Role::SansB, 12.0).ceil()) + 30.0;
        let ur = Rect::from_min_max(pos2(theme_btn.left() - 8.0 - uw, theme_btn.top() + 3.0), pos2(theme_btn.left() - 8.0, theme_btn.bottom() - 3.0));
        // only where it does not cover the tabs
        if ur.left() > tabs_right + 8.0 {
            let resp = ui.interact(ur, Id::new("chrome-update"), Sense::click());
            let accent = if th.palette == Palette::Genshin { hex(0xd3bc8e) } else { th.accent };
            let r = match th.palette {
                Palette::Cyberpunk => 0.0,
                _ => pill(),
            };
            fill(&p, ur, r, rgba(accent, if resp.hovered() { 0.28 } else { 0.16 }));
            stroke_in(&p, ur, r, 1.0, rgba(accent, 0.75));
            p.circle_filled(pos2(ur.left() + 12.0, ur.center().y), 3.0, accent);
            let fg = if th.palette == Palette::Genshin { hex(0xece5d8) } else { th.text };
            text(ui, th, pos2(ur.left() + 20.0, ur.center().y), Align2::LEFT_CENTER, label, Role::SansB, 12.0, fg);
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let resp = resp.on_hover_text(match app.update.install_status() {
                crate::update::Install::Downloading { .. } | crate::update::Install::Restarting => format!("Downloading version {}. Click to see progress.", crate::update::display(&tag)),
                _ => format!("Version {} is out. Click to update (you are asked first).", crate::update::display(&tag)),
            });
            if resp.clicked() {
                app.menu_open = false;
                app.update_prompt = true;
            }
            drag_right = ur.left();
        }
    }

    // --- drag areas (either side of the centred tab group) ---------------
    let first_tab = draws.first().map(|d| d.rect.left()).unwrap_or(rect.left());
    let drag_rects = [
        (Id::new("chrome-drag-left"), Rect::from_min_max(pos2(rect.left(), rect.top()), pos2(first_tab, rect.bottom()))),
        (Id::new("chrome-drag"), Rect::from_min_max(pos2(tabs_right, rect.top()), pos2(drag_right, rect.bottom()))),
    ];
    for (id, drag_rect) in drag_rects {
        if drag_rect.width() <= 0.0 {
            continue;
        }
        let drag = ui.interact(drag_rect, id, Sense::click_and_drag());
        if drag.drag_started_by(egui::PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
    }
}

/// Genshin's notice-board header ("Game | Event"): text tabs on the dark
/// frame, the active one in bright cream with a short gold underline, a
/// centre diamond and a faint glow; a champagne hairline closes the strip.
fn paint_genshin_tabs(ui: &Ui, th: &Theme, rect: Rect, draws: &[TabDraw]) {
    let p = ui.painter();
    let gold = hex(0xd3bc8e);
    let cream = hex(0xece5d8);
    let sep_y = rect.bottom() - 0.5;
    // hairline that fades out toward both window edges
    let mid = rect.center().x;
    let half = rect.width() / 2.0;
    h_gradient(p, Rect::from_min_max(pos2(rect.left(), sep_y - 0.5), pos2(mid, sep_y + 0.5)), Color32::TRANSPARENT, rgba(gold, 0.55));
    h_gradient(p, Rect::from_min_max(pos2(mid, sep_y - 0.5), pos2(rect.right(), sep_y + 0.5)), rgba(gold, 0.55), Color32::TRANSPARENT);
    let _ = half;
    for d in draws {
        let c = if d.active {
            cream
        } else if d.hovered {
            rgba(cream, 0.92)
        } else {
            rgba(cream, 0.58)
        };
        if d.active {
            radial(p, pos2(d.rect.center().x, rect.bottom()), d.rect.width() * 0.62, 20.0, rgba(gold, 0.30));
            let uy = rect.bottom() - 2.5;
            let w = (d.rect.width() * 0.52).max(28.0);
            p.line_segment([pos2(d.rect.center().x - w / 2.0, uy), pos2(d.rect.center().x + w / 2.0, uy)], Stroke::new(2.0_f32, gold));
            diamond(p, pos2(d.rect.center().x, uy), 3.5, gold, Stroke::NONE);
        } else if d.hovered {
            let uy = rect.bottom() - 2.5;
            let w = d.rect.width() * 0.30;
            p.line_segment([pos2(d.rect.center().x - w / 2.0, uy), pos2(d.rect.center().x + w / 2.0, uy)], Stroke::new(1.0_f32, rgba(gold, 0.6)));
        }
        text(ui, th, d.rect.center() + vec2(0.0, -1.0), Align2::CENTER_CENTER, TABS[d.idx].1, Role::SansB, TAB_FONT, c);
    }
}

fn paint_tabs(ui: &Ui, th: &Theme, rect: Rect, p: &egui::Painter, draws: &[TabDraw]) {
    let corners = tab_corners(th);
    for d in draws.iter().filter(|d| !d.active && d.hovered) {
        p.rect_filled(d.rect, corners, hover_bg(th));
    }

    // separator under the strip, interrupted where the active tab opens into the page
    let (sep_w, sep_c) = match th.palette {
        Palette::Pokemon => (2.0, rgba(th.ring, 0.35)),
        Palette::Genshin => (1.0, rgba(th.gold, 0.45)),
        Palette::Cyberpunk => (1.0, rgba(th.accent, 0.45)),
        Palette::Default => (1.0, th.line),
    };
    let sep_y = rect.bottom() - sep_w / 2.0;
    let active_rect = draws.iter().find(|d| d.active).map(|d| d.rect);
    let spans: Vec<(f32, f32)> = match active_rect {
        Some(r) => vec![(rect.left(), r.left()), (r.right(), rect.right())],
        None => vec![(rect.left(), rect.right())],
    };
    for (x0, x1) in spans {
        if x1 > x0 {
            p.line_segment([pos2(x0, sep_y), pos2(x1, sep_y)], Stroke::new(sep_w, sep_c));
        }
    }

    // active tab: page-coloured, rounded top, open at the bottom
    if let Some(r) = active_rect {
        let fill = if th.palette == Palette::Pokemon { th.surface } else { th.bg };
        if th.palette == Palette::Cyberpunk {
            // The game's top nav: the selected section is cyan on a faint cyan
            // wash with a solid underline; no tab box at all.
            let _ = fill;
            crate::ui::v_gradient(p, r, rgba(th.ring, 0.0), rgba(th.ring, 0.16));
            p.line_segment(
                [pos2(r.left() + 6.0, r.bottom() - 1.5), pos2(r.right() - 6.0, r.bottom() - 1.5)],
                Stroke::new(3.0_f32, th.ring),
            );
        } else {
            p.rect_filled(r, corners, fill);
            let (rad, bw) = if th.palette == Palette::Pokemon { (14.0, 2.0) } else { (8.0, 1.0_f32) };
            p.add(tab_outline(r, rad, Stroke::new(bw, tab_border(th))));
            match th.palette {
                Palette::Genshin => {
                    p.line_segment(
                        [pos2(r.left() + 10.0, r.top() + 1.5), pos2(r.right() - 10.0, r.top() + 1.5)],
                        Stroke::new(2.0_f32, th.gold),
                    );
                }
                Palette::Pokemon => {
                    // friendly red tick, inset from the rounded corners
                    p.line_segment(
                        [pos2(r.left() + 16.0, r.top() + 3.0), pos2(r.right() - 16.0, r.top() + 3.0)],
                        Stroke::new(3.0_f32, th.accent),
                    );
                }
                Palette::Cyberpunk | Palette::Default => {}
            }
        }
    }

    // labels on top of every fill
    for d in draws {
        let c = if th.palette == Palette::Cyberpunk {
            if d.active {
                th.ring
            } else if d.hovered {
                th.accent_dark
            } else {
                th.muted
            }
        } else if d.active || d.hovered {
            th.text
        } else if th.palette == Palette::Genshin && !th.dark {
            hex(0x454f68)
        } else {
            th.muted
        };
        if th.palette == Palette::Cyberpunk {
            text_ls(ui, th, d.rect.center(), Align2::CENTER_CENTER, &TABS[d.idx].1.to_uppercase(), Role::SansB, 13.7, c, 0.08);
        } else {
            text(ui, th, d.rect.center(), Align2::CENTER_CENTER, TABS[d.idx].1, Role::SansB, TAB_FONT, c);
        }
    }

    if th.palette == Palette::Cyberpunk {
        // Short red service ticks at the far ends of the strip.
        p.line_segment([pos2(rect.left() + 14.0, rect.top() + 1.0), pos2(rect.left() + 152.0, rect.top() + 1.0)], Stroke::new(1.0_f32, rgba(th.accent, 0.55)));
        p.line_segment([pos2(rect.right() - 204.0, rect.top() + 1.0), pos2(rect.right() - 160.0, rect.top() + 1.0)], Stroke::new(1.0_f32, rgba(th.accent, 0.35)));
    }

}

// ---------------------------------------------------------------------------
// Window controls
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Caption {
    Minimize,
    Maximize,
    Restore,
    Close,
}

fn caption(ui: &mut Ui, th: &Theme, rect: Rect, id: &'static str, kind: Caption) -> Response {
    let resp = ui.interact(rect, Id::new(("chrome-btn", id)), Sense::click());
    let hovered = resp.hovered();
    let pressed = resp.is_pointer_button_down_on();
    let p = ui.painter();
    let bg = if hovered || pressed {
        let c = match kind {
            Caption::Close => {
                if pressed {
                    hex(0xa8281b)
                } else {
                    hex(0xc42b1c)
                }
            }
            _ => match th.palette {
                Palette::Genshin => rgba(hex(0xcbba8c), if pressed { 0.28 } else { 0.16 }),
                Palette::Pokemon => rgba(th.text, if pressed { 0.16 } else { 0.08 }),
                Palette::Cyberpunk => rgba(th.ring, if pressed { 0.26 } else { 0.12 }),
                Palette::Default => rgba(th.text, if pressed { 0.14 } else { 0.075 }),
            },
        };
        p.rect_filled(rect, 0.0, c);
        Some(c)
    } else {
        None
    };
    let fg = match kind {
        Caption::Close if hovered || pressed => Color32::WHITE,
        _ if th.palette == Palette::Genshin => hex(0xece5d8),
        _ => th.text,
    };
    let c = rect.center();
    let s = 5.0;
    let stroke = Stroke::new(1.0_f32, fg);
    match kind {
        Caption::Minimize => {
            p.line_segment([pos2(c.x - s, c.y + 0.5), pos2(c.x + s, c.y + 0.5)], stroke);
        }
        Caption::Maximize => {
            p.rect_stroke(
                Rect::from_center_size(c, vec2(s * 2.0, s * 2.0)),
                CornerRadius::same(1),
                stroke,
                StrokeKind::Inside,
            );
        }
        Caption::Restore => {
            let back = Rect::from_min_size(pos2(c.x - 5.0, c.y - 5.0), vec2(8.0, 8.0));
            let front = back.translate(vec2(2.0, 2.0));
            p.rect_stroke(back, 0.0, stroke, StrokeKind::Inside);
            p.rect_filled(front.shrink(1.0), 0.0, bg.unwrap_or(strip_bg(th)));
            p.rect_stroke(front, 0.0, stroke, StrokeKind::Inside);
        }
        Caption::Close => {
            p.line_segment([pos2(c.x - s, c.y - s), pos2(c.x + s, c.y + s)], stroke);
            p.line_segment([pos2(c.x - s, c.y + s), pos2(c.x + s, c.y - s)], stroke);
        }
    }
    let tip = match kind {
        Caption::Minimize => "Minimize",
        Caption::Maximize => "Maximize",
        Caption::Restore => "Restore down",
        Caption::Close => "Close",
    };
    resp.on_hover_text(tip)
}

/// Settings: a plain gear icon styled like the window buttons beside it.
fn settings_button(ui: &mut Ui, th: &Theme, rect: Rect, open: bool) -> Response {
    let resp = ui.interact(rect, Id::new("chrome-settings"), Sense::click());
    let hovered = resp.hovered();
    let pressed = resp.is_pointer_button_down_on();
    let p = ui.painter();
    if hovered || pressed || open {
        let strong = pressed || open;
        let c = match th.palette {
            Palette::Genshin => rgba(hex(0xcbba8c), if strong { 0.28 } else { 0.16 }),
            Palette::Pokemon => rgba(th.text, if strong { 0.16 } else { 0.08 }),
            Palette::Cyberpunk => rgba(th.ring, if strong { 0.26 } else { 0.12 }),
            Palette::Default => rgba(th.text, if strong { 0.14 } else { 0.075 }),
        };
        p.rect_filled(rect, 0.0, c);
    }
    let fg = if th.palette == Palette::Genshin { hex(0xece5d8) } else { th.text };
    gear(p, rect.center(), fg);
    resp.on_hover_text("Settings: theme, display scale, Excel file and updates")
}

/// An eight-toothed gear outline centred on `c`.
fn gear(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    use std::f32::consts::TAU;
    let (outer, inner, teeth) = (7.5_f32, 5.6_f32, 8);
    let mut pts = Vec::with_capacity(teeth * 4);
    for i in 0..teeth {
        let a = i as f32 / teeth as f32 * TAU;
        let step = TAU / teeth as f32;
        // tooth top spans ~40% of the pitch, flanks slope down to the root
        for (da, r) in [(-0.20, outer), (0.20, outer), (0.30, inner), (0.70, inner)] {
            let t = a + da * step;
            pts.push(c + vec2(t.cos(), t.sin()) * r);
        }
    }
    p.add(egui::Shape::closed_line(pts, Stroke::new(1.3_f32, color)));
    p.circle_stroke(c, 2.4, Stroke::new(1.3_f32, color));
}

// ---------------------------------------------------------------------------
// Palette bits
// ---------------------------------------------------------------------------

fn strip_bg(th: &Theme) -> Color32 {
    match (th.palette, th.dark) {
        // the notice board's dark slate frame, in both modes
        (Palette::Genshin, false) => hex(0x3b4255),
        (Palette::Genshin, true) => hex(0x161a24),
        (Palette::Cyberpunk, true) => hex(0x07060a),
        (Palette::Cyberpunk, false) => hex(0xebe5e2),
        (Palette::Default, true) => hex(0x1f1f1d),
        _ => th.surface2,
    }
}

fn hover_bg(th: &Theme) -> Color32 {
    match th.palette {
        Palette::Genshin => rgba(th.gold, 0.16),
        Palette::Pokemon => th.secondary,
        Palette::Cyberpunk => rgba(th.accent, 0.10),
        Palette::Default => rgba(th.text, 0.07),
    }
}

fn tab_border(th: &Theme) -> Color32 {
    match th.palette {
        Palette::Genshin => rgba(th.gold, 0.55),
        Palette::Pokemon => th.ring,
        Palette::Cyberpunk => rgba(th.ring, 0.62),
        Palette::Default => th.line,
    }
}

fn tab_corners(th: &Theme) -> CornerRadius {
    let r = match th.palette {
        Palette::Pokemon => 14,
        Palette::Cyberpunk => 0,
        _ => 8,
    };
    CornerRadius { nw: r, ne: r, sw: 0, se: 0 }
}

/// Rounded-top outline that is open at the bottom (a tab silhouette).
fn tab_outline(r: Rect, radius: f32, stroke: Stroke) -> egui::Shape {
    use std::f32::consts::{FRAC_PI_2, PI};
    let rad = radius.min(r.width() / 2.0).min(r.height());
    let mut pts = vec![pos2(r.left(), r.bottom())];
    if rad > 0.5 {
        let steps = 6;
        for i in 0..=steps {
            let a = PI + i as f32 / steps as f32 * FRAC_PI_2;
            pts.push(pos2(r.left() + rad + a.cos() * rad, r.top() + rad + a.sin() * rad));
        }
        for i in 0..=steps {
            let a = -FRAC_PI_2 + i as f32 / steps as f32 * FRAC_PI_2;
            pts.push(pos2(r.right() - rad + a.cos() * rad, r.top() + rad + a.sin() * rad));
        }
    } else {
        pts.push(pos2(r.left(), r.top()));
        pts.push(pos2(r.right(), r.top()));
    }
    pts.push(pos2(r.right(), r.bottom()));
    egui::Shape::line(pts, stroke)
}
