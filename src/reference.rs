//! "Reference" card: formulas and reference intervals (drawn as a manual table).
use crate::theme::{Palette, Role, Theme};
use crate::ui::*;
use eframe::egui::{self, pos2, vec2, Align2, Pos2, Rect, Sense, Stroke, Ui, UiBuilder};

const NOTE: &str = "In specimens with a low hematocrit, the percentage of reticulocytes may be falsely elevated because whole blood contains fewer RBCs. A correction factor is used, with the average normal hematocrit considered to be 45%.";
const ROW_H: f32 = 40.0;
/// Cell padding, each side.
const PAD: f32 = 12.0;
/// The parameter column: names wrap onto two or three lines.
const PARAM_W: f32 = 140.0;
const FORMULA_SIZE: f32 = 13.12;
const CELL_SIZE: f32 = 13.76;

const FORMULAS: [(Option<&str>, &str, &str); 3] = [
    (None, "Reticulocyte count × 100", "1000"),
    (Some("Observed count ×"), "Measured hematocrit (%)", "45%"),
    (None, "Percentage reticulocyte × RBC (10⁹)", "100"),
];
const AGES: [&str; 7] = [">19 yrs", "HCT – Male", "HCT – Female", ">19 yrs", "0 Day – 1 Month", "1 – 6 Months", "6 Months – 6 Years"];
const RANGES: [&str; 7] = ["0.84 – 2.21", "41 – 53", "36 – 46", "38.3 – 110.5", "20.0 – 60.0", "40.0 – 100.0", "30.0 – 100.0"];
const HEADS: [&str; 4] = ["Parameter", "Formula", "Age / sex", "Reference interval"];

fn head_w(ui: &Ui, th: &Theme, s: &str) -> f32 {
    let up = s.to_uppercase();
    text_width(ui, th, &up, Role::SansB, 10.88) + 0.03 * 10.88 * up.chars().count() as f32
}

fn formula_w(ui: &Ui, th: &Theme, term: Option<&str>, num: &str, den: &str) -> f32 {
    let eq = text_width(ui, th, "=", Role::Mono, FORMULA_SIZE) + 8.0;
    let t = term.map(|t| text_width(ui, th, t, Role::Mono, FORMULA_SIZE) + 8.0).unwrap_or(0.0);
    eq + t + text_width(ui, th, num, Role::Mono, FORMULA_SIZE).max(text_width(ui, th, den, Role::Mono, FORMULA_SIZE))
}

/// Each column as wide as its widest entry (parameter names wrap instead).
/// Units live in the parameter names, so there is no separate unit column.
fn columns(ui: &Ui, th: &Theme) -> [f32; 4] {
    let widest = |items: &[&str], role: Role, size: f32, head: &str| {
        items.iter().map(|s| text_width(ui, th, s, role, size)).fold(head_w(ui, th, head), f32::max) + 2.0 * PAD
    };
    let formula = FORMULAS.iter().map(|(t, n, d)| formula_w(ui, th, *t, n, d)).fold(head_w(ui, th, HEADS[1]), f32::max) + 2.0 * PAD;
    [
        PARAM_W,
        formula.ceil(),
        widest(&AGES, Role::Sans, CELL_SIZE, HEADS[2]).ceil(),
        widest(&RANGES, Role::Mono, CELL_SIZE, HEADS[3]).ceil(),
    ]
}

/// Width the table needs before it has to scroll sideways.
pub fn natural_width(ui: &Ui, th: &Theme) -> f32 {
    columns(ui, th).iter().sum()
}

/// The reference table, folded to its header until opened. The whole header
/// is the toggle; `open` is flipped when it is clicked.
pub fn reference_card(ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, open: &mut bool) -> f32 {
    let head = Rect::from_min_size(pos, vec2(w, HEAD_H));
    let resp = ui.interact(head, egui::Id::new("ref-toggle"), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.clicked() {
        *open = !*open;
    }
    let opts = CardOpts { title: "Reference", unit: "formulas and reference ranges", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0, alert: false, status: None, collapsed: !*open };
    let rect = card(ui, th, pos, w, &opts, |ui, iw| {
        let o = ui.cursor().min;
        let cols = columns(ui, th);
        let tw = iw.max(natural_width(ui, th));
        // spare width goes to the last column, so formula and age / sex stay together
        let cols = [cols[0], cols[1], cols[2], cols[3] + (tw - natural_width(ui, th))];
        let note_w = cols[1] + cols[2] + cols[3] - 2.0 * PAD - 3.0;
        let note_h = paragraph_height(ui, th, note_w, &format!("Note: {}", NOTE), Role::Serif, 12.48, 1.5);
        let note_row = (note_h + 22.0 + 2.0).max(ROW_H);
        let table_h = 25.0 + 66.0 + (ROW_H * 2.0 + note_row) + ROW_H * 4.0;
        let area = Rect::from_min_size(o, vec2(iw, table_h + 14.0));
        ui.scope_builder(UiBuilder::new().max_rect(area), |ui| {
            egui::ScrollArea::horizontal().id_salt("ref-scroll").auto_shrink([false, true]).drag_to_scroll(false).show(ui, |ui| {
                let origin = ui.cursor().min;
                draw_table(ui, th, origin, cols, note_row);
                ui.allocate_rect(Rect::from_min_size(origin, vec2(tw, table_h)), Sense::hover());
            });
        });
        let src_y = o.y + table_h + 14.0 + 12.0;
        let r = text(ui, th, pos2(o.x, src_y), Align2::LEFT_TOP, "Source: NUH (reticulocyte reference intervals)", Role::SerifI, 12.16, th.muted);
        ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, r.bottom())), Sense::hover());
    });

    // "Show ⌄" / "Hide ⌃" at the right of the header
    let fg = if resp.hovered() { th.text } else { th.muted };
    let fg = if th.palette == Palette::Genshin && resp.hovered() { th.gold } else { fg };
    let mid = head.center().y;
    let cx = head.right() - 1.0 - 20.0 - 6.0;
    let d = if *open { -2.5 } else { 2.5 };
    ui.painter().add(egui::Shape::line(vec![pos2(cx - 5.0, mid - d), pos2(cx, mid + d), pos2(cx + 5.0, mid - d)], Stroke::new(1.6_f32, fg)));
    let label = if *open { "Hide" } else { "Show" };
    let label = if th.palette == Palette::Cyberpunk { label.to_uppercase() } else { label.to_string() };
    text(ui, th, pos2(cx - 14.0, mid), Align2::RIGHT_CENTER, &label, Role::SansB, 12.8, fg);
    rect.bottom()
}

fn draw_table(ui: &mut Ui, th: &Theme, o: Pos2, cols: [f32; 4], note_row: f32) {
    let p = ui.painter().clone();
    let mut xs = [o.x; 5];
    for i in 0..4 {
        xs[i + 1] = xs[i] + cols[i];
    }
    let tw = xs[4] - o.x;
    let line = Stroke::new(1.0_f32, th.line);
    let line2 = Stroke::new(2.0_f32, th.line);

    // header
    let hy = o.y;
    for (i, name) in HEADS.iter().enumerate() {
        text_ls(ui, th, pos2(xs[i] + PAD, hy), Align2::LEFT_TOP, &name.to_uppercase(), Role::SansB, 10.88, th.muted, 0.03);
    }
    let mut y = hy + 25.0;
    p.line_segment([pos2(o.x, y - 1.0), pos2(o.x + tw, y - 1.0)], line2);

    let accent_bar = if th.palette == Palette::Genshin { th.gold } else { th.accent };
    let param_cell = |ui: &Ui, top: f32, h: f32, name: &str| {
        let r = Rect::from_min_max(pos2(xs[0], top), pos2(xs[1], top + h));
        let cell_bg = match th.palette {
            Palette::Pokemon | Palette::Cyberpunk => th.surface2,
            _ => th.accent_soft,
        };
        p.rect_filled(r, 0.0, cell_bg);
        p.rect_filled(Rect::from_min_size(r.min, vec2(3.0, h)), 0.0, accent_bar);
        let g = galley(ui, th, name, Role::SansB, 13.76, th.text, 0.0, r.width() - 12.0 - 15.0);
        let pos = pos2(r.left() + 3.0 + 12.0, r.center().y - g.size().y / 2.0);
        paint_galley(&p, th, Role::SansB, pos, g, th.text);
    };
    let age_range = |ui: &Ui, top: f32, h: f32, age: &str, range: &str| {
        let cy = top + (h - 1.0) / 2.0;
        text(ui, th, pos2(xs[2] + 12.0, cy), Align2::LEFT_CENTER, age, Role::Sans, 13.76, th.text);
        text(ui, th, pos2(xs[3] + PAD, cy), Align2::LEFT_CENTER, range, Role::Mono, 13.76, th.text);
    };
    // ---- group 1: reticulocyte count
    let h1 = 66.0;
    param_cell(ui, y, h1, "Reticulocyte count (%)");
    let (t, n, d) = FORMULAS[0];
    formula(ui, th, pos2(xs[1] + PAD, y + (h1 - 2.0) / 2.0), t, n, d);
    age_range(ui, y, h1, AGES[0], RANGES[0]);
    y += h1;
    p.line_segment([pos2(o.x, y - 1.0), pos2(o.x + tw, y - 1.0)], line2);

    // ---- group 2: corrected retic + HCT ranges, then the note across
    // formula, age and range so it stays short
    let top2 = y;
    let h2 = ROW_H * 2.0 + note_row;
    param_cell(ui, top2, h2, "Corrected reticulocyte count (%)");
    let (t, n, d) = FORMULAS[1];
    formula(ui, th, pos2(xs[1] + PAD, top2 + ROW_H - 1.0), t, n, d);
    age_range(ui, y, ROW_H, AGES[1], RANGES[1]);
    p.line_segment([pos2(xs[2], y + ROW_H - 0.5), pos2(xs[4], y + ROW_H - 0.5)], line);
    y += ROW_H;
    age_range(ui, y, ROW_H, AGES[2], RANGES[2]);
    y += ROW_H;
    // note row
    let nr = Rect::from_min_max(pos2(xs[1], y), pos2(xs[4], y + note_row));
    p.rect_filled(nr, 0.0, th.warn_bg);
    p.rect_filled(Rect::from_min_size(nr.min, vec2(3.0, nr.height())), 0.0, th.warn);
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = nr.width() - 24.0 - 3.0;
    let fid = th.font(Role::Serif, 12.48);
    job.append("Note: ", 0.0, egui::text::TextFormat { font_id: fid.clone(), color: th.warn, line_height: Some(12.48 * 1.5), extra_letter_spacing: 0.3, ..Default::default() });
    job.append(NOTE, 0.0, egui::text::TextFormat { font_id: fid, color: th.muted, line_height: Some(12.48 * 1.5), ..Default::default() });
    let g = ui.fonts(|f| f.layout_job(job));
    p.galley(pos2(nr.left() + 3.0 + 12.0, nr.center().y - g.size().y / 2.0 - 1.0), g, th.muted);
    y += note_row;
    p.line_segment([pos2(o.x, y - 1.0), pos2(o.x + tw, y - 1.0)], line2);

    // ---- group 3: absolute reticulocyte
    let top3 = y;
    let h3 = ROW_H * 4.0;
    param_cell(ui, top3, h3, "Absolute reticulocyte (×10⁹/L)");
    let (t, n, d) = FORMULAS[2];
    formula(ui, th, pos2(xs[1] + PAD, top3 + h3 / 2.0), t, n, d);
    for i in 0..4 {
        age_range(ui, y, ROW_H, AGES[3 + i], RANGES[3 + i]);
        if i < 3 {
            p.line_segment([pos2(xs[2], y + ROW_H - 0.5), pos2(xs[4], y + ROW_H - 0.5)], line);
        }
        y += ROW_H;
    }

    // Column rules: formula | age / sex | reference interval. The note spans the last three
    // columns, so the two inner rules break around it.
    let rule = Stroke::new(1.0_f32, th.line);
    let note_top = top2 + ROW_H * 2.0;
    let note_bottom = note_top + note_row;
    let top = o.y - 4.0;
    p.line_segment([pos2(xs[1] - 0.5, top), pos2(xs[1] - 0.5, y)], rule);
    for x in [xs[2], xs[3]] {
        p.line_segment([pos2(x - 0.5, top), pos2(x - 0.5, note_top)], rule);
        p.line_segment([pos2(x - 0.5, note_bottom), pos2(x - 0.5, y)], rule);
    }
}

/// "= [term] numerator / denominator" with a drawn fraction bar, vertically centred on `mid`.
fn formula(ui: &Ui, th: &Theme, left_mid: Pos2, term: Option<&str>, num: &str, den: &str) {
    let size = 13.12;
    let lh = size * 1.35;
    let mut x = left_mid.x;
    let cy = left_mid.y;
    let eq = text(ui, th, pos2(x, cy), Align2::LEFT_CENTER, "=", Role::Mono, size, th.muted);
    x = eq.right() + 8.0;
    if let Some(t) = term {
        let r = text(ui, th, pos2(x, cy), Align2::LEFT_CENTER, t, Role::Mono, size, th.text);
        x = r.right() + 8.0;
    }
    let wn = text_width(ui, th, num, Role::Mono, size);
    let wd = text_width(ui, th, den, Role::Mono, size);
    let w = wn.max(wd);
    let total = lh * 2.0 + 7.0;
    let top = cy - total / 2.0;
    text(ui, th, pos2(x + w / 2.0, top + lh / 2.0), Align2::CENTER_CENTER, num, Role::Mono, size, th.text);
    let ly = top + lh + 3.0;
    ui.painter().line_segment([pos2(x, ly + 0.5), pos2(x + w, ly + 0.5)], Stroke::new(1.0_f32, th.text));
    text(ui, th, pos2(x + w / 2.0, ly + 1.0 + 3.0 + lh / 2.0), Align2::CENTER_CENTER, den, Role::Mono, size, th.text);
}
