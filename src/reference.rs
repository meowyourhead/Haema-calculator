//! "Reference" card: formulas and reference intervals (drawn as a manual table).
use crate::theme::{Palette, Role, Theme};
use crate::ui::*;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Ui, UiBuilder};

const NOTE: &str = "In specimens with a low hematocrit, the percentage of reticulocytes may be falsely elevated because whole blood contains fewer RBCs. A correction factor is used, with the average normal hematocrit considered to be 45%.";
const ROW_H: f32 = 40.0;

pub fn reference_card(ui: &mut Ui, th: &Theme, pos: Pos2, w: f32) -> f32 {
    let opts = CardOpts { title: "Reference", unit: "", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0 };
    let rect = card(ui, th, pos, w, &opts, |ui, iw| {
        let o = ui.cursor().min;
        let tw = iw.max(960.0);
        let note_w = tw * 0.39 - 24.0 - 3.0;
        let note_h = paragraph_height(ui, th, note_w, &format!("Note: {}", NOTE), Role::Serif, 12.48, 1.5);
        let note_row = (note_h + 22.0 + 2.0).max(ROW_H);
        let table_h = 25.0 + 66.0 + (ROW_H * 2.0 + note_row) + ROW_H * 4.0;
        let area = Rect::from_min_size(o, vec2(iw, table_h + 14.0));
        ui.scope_builder(UiBuilder::new().max_rect(area), |ui| {
            egui::ScrollArea::horizontal().id_salt("ref-scroll").auto_shrink([false, true]).drag_to_scroll(false).show(ui, |ui| {
                let origin = ui.cursor().min;
                draw_table(ui, th, origin, tw, note_row);
                ui.allocate_rect(Rect::from_min_size(origin, vec2(tw, table_h)), Sense::hover());
            });
        });
        let src_y = o.y + table_h + 14.0 + 12.0;
        let r = text(ui, th, pos2(o.x, src_y), Align2::LEFT_TOP, "Source: NUH (reticulocyte reference intervals)", Role::SerifI, 12.16, th.muted);
        ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, r.bottom())), Sense::hover());
    });
    rect.bottom()
}

fn draw_table(ui: &mut Ui, th: &Theme, o: Pos2, tw: f32, note_row: f32) {
    let p = ui.painter().clone();
    let fr = [0.17, 0.44, 0.17, 0.12, 0.10];
    let mut xs = [o.x; 6];
    for i in 0..5 {
        xs[i + 1] = xs[i] + tw * fr[i];
    }
    let line = Stroke::new(1.0, th.line);
    let line2 = Stroke::new(2.0, th.line);

    // header
    let hy = o.y;
    for (i, name) in ["Parameter", "Formula", "Age / Sex", "Reference range", "SI unit"].iter().enumerate() {
        let (pos, anchor) = if i == 3 { (pos2(xs[i + 1] - 12.0, hy), Align2::RIGHT_TOP) } else { (pos2(xs[i] + 12.0, hy), Align2::LEFT_TOP) };
        text_ls(ui, th, pos, anchor, &name.to_uppercase(), Role::SansB, 10.88, th.muted, 0.03);
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
        text(ui, th, pos2(xs[4] - 12.0, cy), Align2::RIGHT_CENTER, range, Role::Mono, 13.76, th.text);
    };
    let unit_cell = |ui: &Ui, top: f32, h: f32, unit: &str| {
        text(ui, th, pos2(xs[4] + 12.0, top + (h - 1.0) / 2.0), Align2::LEFT_CENTER, unit, Role::Sans, 13.76, th.muted);
    };

    // ---- group 1: reticulocyte count
    let h1 = 66.0;
    param_cell(ui, y, h1, "Reticulocyte count (%)");
    formula(ui, th, pos2(xs[1] + 12.0, y + (h1 - 2.0) / 2.0), None, "Reticulocyte count × 100", "1000");
    age_range(ui, y, h1, ">19 yrs", "0.84 – 2.21");
    unit_cell(ui, y, h1, "%");
    y += h1;
    p.line_segment([pos2(o.x, y - 1.0), pos2(o.x + tw, y - 1.0)], line2);

    // ---- group 2: corrected retic + HCT ranges
    let top2 = y;
    let h2 = ROW_H * 2.0 + note_row;
    param_cell(ui, top2, h2, "Corrected reticulocyte count (%)");
    formula(ui, th, pos2(xs[1] + 12.0, top2 + (h2 - 2.0) / 2.0), Some("Observed count ×"), "Measured hematocrit (%)", "45%");
    age_range(ui, y, ROW_H, "HCT – Male", "41 – 53");
    p.line_segment([pos2(xs[2], y + ROW_H - 0.5), pos2(xs[4], y + ROW_H - 0.5)], line);
    y += ROW_H;
    age_range(ui, y, ROW_H, "HCT – Female", "36 – 46");
    p.line_segment([pos2(xs[2], y + ROW_H - 0.5), pos2(xs[4], y + ROW_H - 0.5)], line);
    unit_cell(ui, top2, ROW_H * 2.0, "%");
    y += ROW_H;
    // note row
    let nr = Rect::from_min_max(pos2(xs[2], y), pos2(xs[5], y + note_row));
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
    param_cell(ui, top3, h3, "Absolute reticulocyte");
    formula(ui, th, pos2(xs[1] + 12.0, top3 + h3 / 2.0), None, "Percentage reticulocyte × RBC (10⁹)", "100");
    let rows = [(">19 yrs", "38.3 – 110.5"), ("0 Day – 1 Month", "20.0 – 60.0"), ("1 – 6 Months", "40.0 – 100.0"), ("6 Months – 6 Years", "30.0 – 100.0")];
    for (i, (a, r)) in rows.iter().enumerate() {
        age_range(ui, y, ROW_H, a, r);
        if i < 3 {
            p.line_segment([pos2(xs[2], y + ROW_H - 0.5), pos2(xs[4], y + ROW_H - 0.5)], line);
        }
        y += ROW_H;
    }
    unit_cell(ui, top3, h3, "×10⁹/L");
    let _ = (Color32::WHITE, CornerRadius::ZERO);
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
    ui.painter().line_segment([pos2(x, ly + 0.5), pos2(x + w, ly + 0.5)], Stroke::new(1.0, th.text));
    text(ui, th, pos2(x + w / 2.0, ly + 1.0 + 3.0 + lh / 2.0), Align2::CENTER_CENTER, den, Role::Mono, size, th.text);
}
