//! HBEL tab: results view (chips → suggested comment) and source sheet view.
use crate::app::App;
use crate::hbel_data::{library, visible, Entry, Group, Link, Row, GROUPS, ROWS};
use crate::hbel_match::{describe, hint, row_name, suggest, Eval, Inputs, NumState, Outcome, State};
use crate::panels::toolbar;
use crate::theme::{rgba, Role, Theme};
use crate::ui::*;
use eframe::egui::{self, pos2, vec2, Align, Align2, Id, Layout, Pos2, Rect, Sense, Stroke, Ui, UiBuilder};

/// Everything the HBEL tab remembers during a session.
#[derive(Default)]
pub struct HbelState {
    pub inputs: Inputs,
    pub source_open: bool,
    pub search: String,
    pub filter: Option<Group>,
    viewing: Option<(u8, Inputs)>,
    copied: Option<(u8, f64)>,
}

impl HbelState {
    /// Show `code` in the suggestion card until the results change.
    pub fn view_code(&mut self, code: u8) {
        self.viewing = Some((code, self.inputs.clone()));
    }

    /// Forget the viewed code once the results differ from when it was opened.
    pub fn refresh_viewing(&mut self) {
        if matches!(&self.viewing, Some((_, snap)) if *snap != self.inputs) {
            self.viewing = None;
        }
    }

    pub fn viewing(&self) -> Option<u8> {
        match &self.viewing {
            Some((code, snap)) if *snap == self.inputs => Some(*code),
            _ => None,
        }
    }

    fn clear(&mut self) {
        self.inputs.clear();
        self.viewing = None;
    }
}

const LABEL_W: f32 = 112.0;
const NUM_W: f32 = 84.0;
const CHIP_GAP: f32 = 6.0;
const ROW_GAP: f32 = 10.0;
const BODY: f32 = 15.2;

/// Sheet column → input row (for match marks next to the verbatim criteria).
fn column_row(col: &str) -> Option<Row> {
    match col {
        "A2" => Some(Row::Hba2),
        "F" => Some(Row::Hbf),
        "Hgb" => Some(Row::Hb),
        "RBC indices" => Some(Row::Indices),
        "C window" => Some(Row::CWin),
        "S window" => Some(Row::SWin),
        "HbH inclusion body" => Some(Row::Hbh),
        _ => None,
    }
}

fn entry(code: u8) -> &'static Entry {
    library().entries.iter().find(|e| e.code == code).expect("code in library")
}

/// Uppercase section caption; returns the y below it.
fn caption(ui: &Ui, th: &Theme, x: f32, y: f32, s: &str) -> f32 {
    label(ui, th, pos2(x, y), s);
    y + LABEL_H + 6.0
}

/// Underlined link text that opens in the default browser. Returns its rect.
pub(crate) fn link(ui: &mut Ui, th: &Theme, pos: Pos2, l: &Link, max_w: f32) -> Rect {
    let s = format!("{} ↗", l.label);
    let g = galley(ui, th, &s, Role::SansB, 13.0, th.accent, 0.0, max_w);
    let rect = Rect::from_min_size(pos, g.size());
    let resp = ui.interact(rect, Id::new(("hbel-link", &l.url, pos.x as i32, pos.y as i32)), Sense::click()).on_hover_text(&l.url);
    let c = if resp.hovered() { th.accent_dark } else { th.accent };
    paint_galley(ui.painter(), th, Role::SansB, pos, g, c);
    ui.painter().line_segment([pos2(rect.left(), rect.bottom()), pos2(rect.right(), rect.bottom())], Stroke::new(1.0_f32, rgba(c, 0.6)));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(&l.url));
    }
    rect
}

/// A small button placed with its top-right corner at `right_top`.
fn button_at(ui: &mut Ui, th: &Theme, right_top: Pos2, label_s: &str, kind: Btn) -> egui::Response {
    let r = Rect::from_min_max(pos2(right_top.x - 300.0, right_top.y), pos2(right_top.x, right_top.y + 36.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(r).layout(Layout::right_to_left(Align::Min)));
    button(&mut child, th, label_s, kind, true)
}

impl App {
    pub fn hbel_panel(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        self.hbel_results(ui, th, origin, w)
    }

    /// Copy feedback: the code whose Copy button should read "Copied".
    fn hbel_copied(&self, ui: &Ui) -> Option<u8> {
        let now = ui.input(|i| i.time);
        let c = self.hbel.copied.filter(|(_, t)| now - t < 1.5).map(|(c, _)| c);
        if c.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
        c
    }

    fn hbel_copy(&mut self, ui: &Ui, code: u8) {
        ui.ctx().copy_text(entry(code).comment.clone());
        self.hbel.copied = Some((code, ui.input(|i| i.time)));
    }

    /// The source sheet, drawn in the right-hand side panel.
    pub fn hbel_source(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        // header: title + close, then search and filters
        text_ls(ui, th, pos2(origin.x, origin.y + 18.0), Align2::LEFT_CENTER, "Source sheet", Role::Display, 18.0, th.text, -0.01);
        if button_at(ui, th, pos2(origin.x + w, origin.y), "Close", Btn::Secondary).clicked() {
            self.hbel.source_open = false;
        }
        let sr = Rect::from_min_size(pos2(origin.x, origin.y + 36.0 + 14.0), vec2(w.min(460.0), FIELD_H));
        text_field(ui, th, sr, Id::new("hbel-search"), &mut self.hbel.search, "Search comments, criteria, sample IDs…", false);

        let mut y = sr.bottom() + 12.0;
        let mut x = origin.x;
        let mut pick: Option<Option<Group>> = None;
        let filters: Vec<(Option<Group>, &str)> = std::iter::once((None, "All")).chain(GROUPS.iter().map(|g| (Some(*g), g.label()))).collect();
        for (i, (g, name)) in filters.iter().enumerate() {
            let cw = chip_button_width(ui, th, name);
            if x > origin.x && x + cw > origin.x + w {
                x = origin.x;
                y += CHIP_H + CHIP_GAP;
            }
            let (r, resp) = chip_button(ui, th, pos2(x, y), name, self.hbel.filter == *g, Id::new(("hbel-filter", i)));
            if resp.clicked() {
                pick = Some(*g);
            }
            x = r.right() + CHIP_GAP;
        }
        if let Some(g) = pick {
            self.hbel.filter = g;
        }
        y += CHIP_H + 12.0;
        y = footer(ui, th, origin.x, y, w) + 18.0;

        let shown = visible(library(), &self.hbel.search, self.hbel.filter);
        let copied = self.hbel_copied(ui);
        let mut copy_req = None;
        let mut view_req = None;
        if shown.is_empty() {
            y = paragraph(ui, th, pos2(origin.x, y), w, "No comments match.", Role::Sans, 14.0, th.muted, 1.45).bottom();
        } else {
            let n = format!("{} of {} comments · click a code to open it in the results view", shown.len(), library().entries.len());
            y = text(ui, th, pos2(origin.x, y), Align2::LEFT_TOP, &n, Role::Sans, 12.5, th.muted).bottom() + 8.0;
            y = source_table(ui, th, pos2(origin.x, y), w, &shown, copied, &mut copy_req, &mut view_req);
        }
        if let Some(code) = copy_req {
            self.hbel_copy(ui, code);
        }
        if let Some(code) = view_req {
            self.hbel.view_code(code);
        }
        ui.allocate_rect(Rect::from_min_max(origin, pos2(origin.x + w, y)), Sense::hover());
        y
    }

    fn hbel_results(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        self.hbel.refresh_viewing();
        let mut clear = false;
        let mut open_src = false;
        let y = toolbar(ui, origin, w, |ui| {
            if button(ui, th, "Clear", Btn::Secondary, true).clicked() {
                clear = true;
            }
            let src_label = if self.hbel.source_open { "Hide source sheet" } else { "Show source sheet" };
            if button(ui, th, src_label, Btn::Secondary, true).clicked() {
                open_src = true;
            }
        });
        if clear {
            self.hbel.clear();
        }
        if open_src {
            self.hbel.source_open = !self.hbel.source_open;
        }

        let wide = w >= 900.0;
        let gap = 16.0;
        let (rw, sw) = if wide { ((w - gap) * 0.56, (w - gap) * 0.44) } else { (w, w) };
        let opts = CardOpts { title: "Results", unit: "blank rows match anything", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0 };
        let inputs = &mut self.hbel.inputs;
        let r_rect = card(ui, th, pos2(origin.x, y), rw, &opts, |ui, iw| results_body(ui, th, iw, inputs));

        let s_pos = if wide { pos2(origin.x + rw + gap, y) } else { pos2(origin.x, r_rect.bottom() + gap) };
        let sug = suggest(library(), &self.hbel.inputs);
        let viewing = self.hbel.viewing();
        let copied = self.hbel_copied(ui);
        let mut copy_req: Option<u8> = None;
        let opts = CardOpts { title: "Suggested comment", unit: "", bar: None, head_bg: None, title_color: None, chip: None, min_height: if wide { r_rect.height() } else { 0.0 } };
        let s_rect = card(ui, th, s_pos, sw, &opts, |ui, iw| {
            let o = ui.cursor().min;
            let mut y = o.y;
            let muted = |ui: &Ui, y: f32, s: &str| paragraph(ui, th, pos2(o.x, y), iw, s, Role::Sans, 14.0, th.muted, 1.45).bottom();
            if let Some(code) = viewing {
                y = muted(ui, y, &format!("Viewing code {} (not from your results)", code)) + 10.0;
                y = entry_block(ui, th, o.x, y, iw, entry(code), None, copied, &mut copy_req);
            } else {
                match sug.state {
                    State::Empty => y = muted(ui, y, "Enter results on the left."),
                    State::NoFit => y = muted(ui, y, "No comment fits exactly.") + 4.0,
                    State::Fits => {
                        if sug.top.len() > 1 {
                            let t = format!("{} comments fit equally well", sug.top.len());
                            y = text(ui, th, pos2(o.x, y), Align2::LEFT_TOP, &t, Role::SansB, 14.0, th.warn).bottom() + 12.0;
                        }
                        for (k, ev) in sug.top.iter().enumerate() {
                            if k > 0 {
                                ui.painter().line_segment([pos2(o.x, y + 8.0), pos2(o.x + iw, y + 8.0)], Stroke::new(1.0_f32, th.line));
                                y += 20.0;
                            }
                            y = entry_block(ui, th, o.x, y, iw, entry(ev.code), Some(ev), copied, &mut copy_req);
                            if sug.top.len() > 1 {
                                if let Some(h) = hint(ev, entry(ev.code)) {
                                    y = paragraph(ui, th, pos2(o.x, y + 8.0), iw, &format!("To confirm: {}", h.trim_start_matches("also expects ")), Role::SerifI, 13.5, th.warn, 1.4).bottom();
                                }
                            }
                        }
                    }
                }
                if !sug.also.is_empty() {
                    y += 18.0;
                    ui.painter().line_segment([pos2(o.x, y), pos2(o.x + iw, y)], Stroke::new(1.0_f32, th.line));
                    y = caption(ui, th, o.x, y + 14.0, "Also possible");
                    for ev in &sug.also {
                        let e = entry(ev.code);
                        let tail = hint(ev, e).unwrap_or_else(|| e.comment.clone());
                        let code_r = text(ui, th, pos2(o.x, y), Align2::LEFT_TOP, &format!("Code {}", ev.code), Role::SansB, 13.5, th.text);
                        y = paragraph(ui, th, pos2(o.x + 70.0, y), iw - 70.0, &tail, Role::Sans, 13.5, th.muted, 1.4).bottom().max(code_r.bottom()) + 8.0;
                    }
                }
            }
            ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y)), Sense::hover());
        });
        if let Some(code) = copy_req {
            self.hbel_copy(ui, code);
        }

        let bottom = if wide { r_rect.bottom().max(s_rect.bottom()) } else { s_rect.bottom() };
        footer(ui, th, origin.x, bottom + 18.0, w)
    }
}

/// Reference ranges and tab-wide links. Returns the bottom y.
pub(crate) fn footer(ui: &mut Ui, th: &Theme, x: f32, y: f32, w: f32) -> f32 {
    let r = text(ui, th, pos2(x, y), Align2::LEFT_TOP, "Reference ranges: HbA2 2.2–3.4 %, HbF 0–1.3 %", Role::Sans, 13.0, th.muted);
    let mut lx = r.right() + 24.0;
    let mut ly = y;
    let mut bottom = r.bottom();
    for l in &library().resources {
        let lw = text_width(ui, th, &format!("{} ↗", l.label), Role::SansB, 13.0);
        if lx + lw > x + w {
            lx = x;
            ly = bottom + 8.0;
        }
        let lr = link(ui, th, pos2(lx, ly), l, w);
        lx = lr.right() + 24.0;
        bottom = bottom.max(lr.bottom());
    }
    ui.allocate_rect(Rect::from_min_max(pos2(x, y), pos2(x + w, bottom)), Sense::hover());
    bottom
}

fn results_body(ui: &mut Ui, th: &Theme, iw: f32, inputs: &mut Inputs) {
    let o = ui.cursor().min;
    let mut y = o.y;
    let chips_x0 = o.x + LABEL_W;
    for (ri, row) in ROWS.iter().copied().enumerate() {
        if ri > 0 {
            ui.painter().line_segment([pos2(o.x, y - ROW_GAP / 2.0 - 0.5), pos2(o.x + iw, y - ROW_GAP / 2.0 - 0.5)], Stroke::new(1.0_f32, rgba(th.line, 0.7)));
        }
        let line_h = CHIP_H;
        let mut x = chips_x0;
        let row_x0 = x;
        let mut cy = y;
        let mut clicked = None;
        for (i, opt) in row.options().iter().enumerate() {
            let cw = chip_button_width(ui, th, opt.label);
            if x > row_x0 && x + cw > o.x + iw {
                x = row_x0;
                cy += CHIP_H + CHIP_GAP;
            }
            let (r, resp) = chip_button(ui, th, pos2(x, cy), opt.label, inputs.chip_selected(row, i), Id::new(("hbel-chip", row.key(), i)));
            if resp.clicked() {
                clicked = Some(i);
            }
            x = r.right() + CHIP_GAP;
        }
        if let Some(i) = clicked {
            inputs.click_chip(row, i);
        }
        // row name (not uppercased: "HbA2", "HbA1c" must keep their case)
        let cap = text(ui, th, pos2(o.x, y + line_h / 2.0), Align2::LEFT_CENTER, row.label(), Role::SansB, 13.0, th.text);
        let mut left_bottom = cap.bottom();
        if row.is_numeric() {
            let id = Id::new(("hbel-num", row.key()));
            let invalid = inputs.number(row) == NumState::Invalid;
            let rect = Rect::from_min_size(pos2(o.x, cap.bottom() + 6.0), vec2(NUM_W, FIELD_H - 4.0));
            let value = if row == Row::Hba2 { &mut inputs.hba2 } else { &mut inputs.hbf };
            if text_field(ui, th, rect, id, value, "type %", invalid) {
                inputs.number_edited(row);
            }
            left_bottom = rect.bottom();
            if invalid {
                let r = text(ui, th, pos2(o.x, rect.bottom() + 3.0), Align2::LEFT_TOP, "0–100, use a dot", Role::Sans, 11.0, th.bad);
                left_bottom = r.bottom();
            }
        }
        if row == Row::Film {
            left_bottom = text(ui, th, pos2(o.x, cap.bottom() + 1.0), Align2::LEFT_TOP, "any that apply", Role::Sans, 11.0, th.muted).bottom();
        }
        let row_bottom = (cy + CHIP_H).max(left_bottom);
        y = row_bottom + ROW_GAP;
    }
    ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y - ROW_GAP)), Sense::hover());
}

/// One comment with its criteria and supporting content. Returns the bottom y.
#[allow(clippy::too_many_arguments)]
fn entry_block(ui: &mut Ui, th: &Theme, x: f32, y: f32, w: f32, e: &Entry, ev: Option<&Eval>, copied: Option<u8>, copy_req: &mut Option<u8>) -> f32 {
    let set_i = ev.map_or(0, |v| v.set);
    let mut head = format!("Code {}", e.code);
    if let (Some(_), Some(name)) = (ev, e.sets[set_i].name.as_deref()) {
        head = format!("{} ({})", head, name);
    }
    let hr = chip(ui, th, pos2(x, y), Align2::LEFT_TOP, &head, Role::MonoB, 13.0, th.accent_dark, th.accent_soft, Some((1.0, th.accent)), vec2(10.0, 3.0), pill());
    let label_s = if copied == Some(e.code) { "Copied" } else { "Copy" };
    if button_at(ui, th, pos2(x + w, y - 4.0), label_s, Btn::Mini).clicked() {
        *copy_req = Some(e.code);
    }
    let mut y = hr.bottom() + 12.0;
    y = paragraph(ui, th, pos2(x, y), w, &e.comment, Role::Serif, BODY, th.text, 1.5).bottom() + 16.0;

    // criteria: the sheet's own wording; for code 12 without a match, both sets
    let sets: Vec<usize> = if ev.is_some() || e.sets.len() == 1 { vec![set_i] } else { (0..e.sets.len()).collect() };
    y = caption(ui, th, x, y, "Criteria (sheet)");
    for si in sets {
        let set = &e.sets[si];
        if ev.is_none() {
            if let Some(name) = &set.name {
                y = text(ui, th, pos2(x, y), Align2::LEFT_TOP, name, Role::SansB, 13.0, th.text).bottom() + 4.0;
            }
        }
        let crit_rows: Vec<Row> = set.crits.iter().map(|c| c.row).collect();
        for (col, val) in &set.sheet {
            let row = column_row(col).filter(|r| crit_rows.contains(r));
            let oc = ev.zip(row).and_then(|(v, r)| v.outcome(r));
            y = crit_line(ui, th, x, y, w, col, val, oc, false);
        }
        // defining criteria with no sheet column of their own (HbA2 peak, film)
        let covered: Vec<Row> = set.sheet.iter().filter_map(|(c, _)| column_row(c)).collect();
        for c in set.crits.iter().filter(|c| !c.assumed && !covered.contains(&c.row)) {
            let oc = ev.and_then(|v| v.outcome(c.row));
            let d = describe(c);
            let (_, rest) = d.split_once(": ").unwrap_or(("", &d));
            y = crit_line(ui, th, x, y, w, row_name(c.row), rest, oc, false);
        }
        for c in set.crits.iter().filter(|c| c.assumed) {
            let oc = ev.and_then(|v| v.outcome(c.row));
            let d = describe(c);
            let (_, rest) = d.split_once(": ").unwrap_or(("", &d));
            y = crit_line(ui, th, x, y, w, row_name(c.row), &format!("{} (assumed)", rest), oc, true);
        }
    }
    let list = |ui: &mut Ui, y: f32, title: &str, items: &[String]| -> f32 {
        if items.is_empty() {
            return y;
        }
        let y = caption(ui, th, x, y + 10.0, title);
        paragraph(ui, th, pos2(x, y), w, &items.join(" · "), Role::Sans, 13.5, th.text, 1.45).bottom()
    };
    y = list(ui, y, "Remarks", &e.remarks);
    y = list(ui, y, "Example cases", &e.examples);
    y = list(ui, y, "References", &e.references);
    if !e.links.is_empty() {
        y = caption(ui, th, x, y + 10.0, "Links");
        let mut lx = x;
        let mut bottom = y;
        for l in &e.links {
            let lw = text_width(ui, th, &format!("{} ↗", l.label), Role::SansB, 13.0);
            if lx > x && lx + lw > x + w {
                lx = x;
                y = bottom + 6.0;
            }
            let r = link(ui, th, pos2(lx, y), l, w);
            lx = r.right() + 20.0;
            bottom = bottom.max(r.bottom());
        }
        y = bottom;
    }
    y
}

/// Sheet columns shown in the source table: (sheet column, minimum width).
pub(crate) const TABLE_COLS: [(&str, f32); 8] = [
    ("A2", 104.0),
    ("F", 92.0),
    ("Hgb", 94.0),
    ("RBC indices", 124.0),
    ("C window", 70.0),
    ("S window", 70.0),
    ("Fe", 86.0),
    ("HbH inclusion body", 96.0),
];
const CODE_COL: f32 = 66.0;
const COMMENT_COL: f32 = 262.0;
const NOTES_COL: f32 = 210.0;
const CELL_PAD: f32 = 10.0;
const CELL_FONT: f32 = 12.8;

/// The sheet's text for one column; named criteria sets (code 12) are prefixed and stacked.
pub(crate) fn cell_text(e: &Entry, col: &str) -> String {
    let named = e.sets.len() > 1;
    e.sets
        .iter()
        .filter_map(|s| {
            let v = s.sheet.iter().find(|(c, _)| c == col)?;
            Some(match (&s.name, named) {
                (Some(n), true) => format!("{}: {}", n, v.1),
                _ => v.1.clone(),
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Remarks, example cases and references for the Notes column (links are drawn separately).
pub(crate) fn notes_text(e: &Entry) -> String {
    let mut out = Vec::new();
    if !e.remarks.is_empty() {
        out.push(format!("Remarks: {}", e.remarks.join(" · ")));
    }
    if !e.examples.is_empty() {
        out.push(format!("Examples: {}", e.examples.join(", ")));
    }
    for r in &e.references {
        out.push(format!("Reference: {}", r));
    }
    out.join("\n")
}

/// Layout of a comment cell: where the copy button goes (left-middle, relative to the
/// cell's text origin) and the cell's text height including the button.
fn comment_cell(ui: &Ui, th: &Theme, width: f32, comment: &str, copy_label: &str) -> (Pos2, f32) {
    let (end, h) = text_end(ui, th, width, comment, Role::Serif, 13.6, 1.4);
    let bw = copy_button_width(ui, th, copy_label);
    if end.x + 10.0 + bw <= width {
        (pos2(end.x + 10.0, end.y), h)
    } else {
        (pos2(0.0, h + 11.0), h + 22.0)
    }
}

/// The source sheet as a table (scrolls sideways when narrow). Returns the bottom y.
#[allow(clippy::too_many_arguments)]
fn source_table(ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, rows: &[&Entry], copied: Option<u8>, copy_req: &mut Option<u8>, view_req: &mut Option<u8>) -> f32 {
    // column x positions: spare width goes to Comment and Notes
    let min_w = CODE_COL + COMMENT_COL + NOTES_COL + TABLE_COLS.iter().map(|c| c.1).sum::<f32>();
    let tw = w.max(min_w);
    let extra = tw - min_w;
    let mut widths = vec![CODE_COL, COMMENT_COL + extra * 0.6];
    widths.extend(TABLE_COLS.iter().map(|c| c.1));
    widths.push(NOTES_COL + extra * 0.4);
    let heads: Vec<&str> = ["Code", "Comment"].into_iter().chain(TABLE_COLS.iter().map(|c| c.0)).chain(["Notes & links"]).collect();

    // measure rows
    let wrap = |i: usize| widths[i] - CELL_PAD * 2.0;
    let link_h = 20.0;
    let row_h: Vec<f32> = rows
        .iter()
        .map(|e| {
            let mut h = CHIP_H;
            h = h.max(comment_cell(ui, th, wrap(1), &e.comment, "Copied").1);
            for (k, (col, _)) in TABLE_COLS.iter().enumerate() {
                h = h.max(paragraph_height(ui, th, wrap(k + 2), &cell_text(e, col), Role::Sans, CELL_FONT, 1.4));
            }
            let notes = notes_text(e);
            let nh = if notes.is_empty() { 0.0 } else { paragraph_height(ui, th, wrap(widths.len() - 1), &notes, Role::Sans, 12.2, 1.4) + 4.0 };
            h.max(nh + e.links.len() as f32 * link_h) + CELL_PAD * 2.0
        })
        .collect();
    let head_h = heads.iter().enumerate().map(|(i, h)| paragraph_height(ui, th, wrap(i), h, Role::SansB, 12.5, 1.3)).fold(0.0_f32, f32::max) + 16.0;
    let table_h = head_h + row_h.iter().sum::<f32>();
    let scroll_pad = if tw > w { 14.0 } else { 0.0 };

    let area = Rect::from_min_size(pos, vec2(w, table_h + scroll_pad));
    ui.scope_builder(UiBuilder::new().max_rect(area), |ui| {
        egui::ScrollArea::horizontal().id_salt("hbel-table").auto_shrink([false, true]).drag_to_scroll(false).show(ui, |ui| {
            let o = ui.cursor().min;
            let p = ui.painter().clone();
            let frame = Rect::from_min_size(o, vec2(tw, table_h));
            let r = th.radius_lg;
            p.rect_filled(frame, cr(r), th.surface);
            // header
            let head = Rect::from_min_size(o, vec2(tw, head_h));
            p.with_clip_rect(head).rect_filled(frame, cr(r), th.surface2);
            let mut x = o.x;
            for (i, h) in heads.iter().enumerate() {
                paragraph(ui, th, pos2(x + CELL_PAD, o.y + 8.0), wrap(i), h, Role::SansB, 12.5, th.muted, 1.3);
                x += widths[i];
            }
            let mut y = o.y + head_h;
            for (ri, e) in rows.iter().enumerate() {
                let rh = row_h[ri];
                if ri % 2 == 1 {
                    p.rect_filled(Rect::from_min_size(pos2(o.x, y), vec2(tw, rh)), 0.0, rgba(th.surface2, 0.55));
                }
                p.line_segment([pos2(o.x, y), pos2(o.x + tw, y)], Stroke::new(1.0_f32, th.line));
                let top = y + CELL_PAD;
                let mut x = o.x;
                // code: chip opens it in the results view
                let (_, resp) = chip_button(ui, th, pos2(x + CELL_PAD, top), &e.code.to_string(), true, Id::new(("hbel-src-code", e.code)));
                if resp.on_hover_text("Show in the results view").clicked() {
                    *view_req = Some(e.code);
                }
                x += widths[0];
                // comment (selectable) with the copy button after its last word
                let cpos = pos2(x + CELL_PAD, top);
                selectable_paragraph(ui, th, cpos, wrap(1), &e.comment, Role::Serif, 13.6, th.text, 1.4);
                let cl = if copied == Some(e.code) { "Copied" } else { "Copy" };
                let (at, _) = comment_cell(ui, th, wrap(1), &e.comment, cl);
                if copy_button(ui, th, cpos + at.to_vec2(), cl, Id::new(("hbel-src-copy", e.code))).clicked() {
                    *copy_req = Some(e.code);
                }
                x += widths[1];
                for (k, (col, _)) in TABLE_COLS.iter().enumerate() {
                    let t = cell_text(e, col);
                    if t.is_empty() {
                        text(ui, th, pos2(x + CELL_PAD, top), Align2::LEFT_TOP, "—", Role::Sans, CELL_FONT, th.muted);
                    } else {
                        selectable_paragraph(ui, th, pos2(x + CELL_PAD, top), wrap(k + 2), &t, Role::Sans, CELL_FONT, th.text, 1.4);
                    }
                    x += widths[k + 2];
                }
                let notes = notes_text(e);
                let mut ny = top;
                if !notes.is_empty() {
                    ny = selectable_paragraph(ui, th, pos2(x + CELL_PAD, top), wrap(widths.len() - 1), &notes, Role::Sans, 12.2, th.text, 1.4).bottom() + 4.0;
                }
                for l in &e.links {
                    ny = link(ui, th, pos2(x + CELL_PAD, ny), l, wrap(widths.len() - 1)).bottom() + 4.0;
                }
                y += rh;
            }
            // column rules and frame
            let mut x = o.x;
            for wd in &widths[..widths.len() - 1] {
                x += wd;
                p.line_segment([pos2(x, o.y), pos2(x, o.y + table_h)], Stroke::new(1.0_f32, rgba(th.line, 0.6)));
            }
            stroke_in(&p, frame, r, if th.palette == crate::theme::Palette::Pokemon { 2.0 } else { 1.0 }, th.line);
            ui.allocate_rect(frame, Sense::hover());
        });
    });
    ui.allocate_rect(area, Sense::hover());
    area.bottom()
}

/// "● Label   value" criteria line. The dot shows match (ok), conflict (bad) or nothing.
#[allow(clippy::too_many_arguments)]
fn crit_line(ui: &Ui, th: &Theme, x: f32, y: f32, w: f32, col: &str, val: &str, oc: Option<Outcome>, assumed: bool) -> f32 {
    let col_w = 132.0;
    let (dot, vc) = match oc {
        Some(Outcome::Match) => (Some(th.ok), th.text),
        Some(Outcome::Conflict) | Some(Outcome::KeyUnmet) => (Some(th.bad), th.bad),
        _ => (None, if assumed { th.muted } else { th.text }),
    };
    if let Some(c) = dot {
        ui.painter().circle_filled(pos2(x + 4.0, y + 9.0), 3.5, c);
    }
    text(ui, th, pos2(x + 14.0, y + 1.0), Align2::LEFT_TOP, col, Role::SansB, 12.5, th.muted);
    let role = if assumed { Role::SerifI } else { Role::Sans };
    paragraph(ui, th, pos2(x + 14.0 + col_w, y), w - 14.0 - col_w, val, role, 13.5, vc, 1.4).bottom().max(y + 18.0) + 4.0
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::hbel_data::Row;

    #[test]
    fn viewing_clears_on_input_change() {
        let mut s = HbelState::default();
        s.view_code(7);
        assert_eq!(s.viewing(), Some(7));
        s.inputs.click_chip(Row::Hb, 0);
        assert_eq!(s.viewing(), None);
    }

    #[test]
    fn viewing_stays_cleared_after_undo() {
        let mut s = HbelState::default();
        s.view_code(7);
        s.inputs.click_chip(Row::Hb, 0);
        s.refresh_viewing();
        s.inputs.click_chip(Row::Hb, 0); // undo: inputs equal the snapshot again
        s.refresh_viewing();
        assert_eq!(s.viewing(), None);
    }

    #[test]
    fn table_cells() {
        let lib = library();
        let e = |c: u8| lib.entries.iter().find(|x| x.code == c).unwrap();
        assert_eq!(cell_text(e(12), "A2"), "Homozygous: Absent/Low\nHeterozygous: Absent/Low");
        assert_eq!(cell_text(e(12), "Hgb"), "Heterozygous: Normal/ Slightly reduced");
        assert_eq!(cell_text(e(1), "HbH inclusion body"), "NA");
        assert_eq!(cell_text(e(8), "F"), "");
        assert_eq!(notes_text(e(8)), "Remarks: Degradation Peak coeluting with HbA2 peak");
        assert_eq!(notes_text(e(4)), "Examples: [sample ID], [sample ID]");
        assert!(notes_text(e(21)).starts_with("Examples: [sample ID], [sample ID]\nReference: Patel"));
        assert_eq!(TABLE_COLS.iter().map(|(c, _)| *c).collect::<Vec<_>>(), vec!["A2", "F", "Hgb", "RBC indices", "C window", "S window", "Fe", "HbH inclusion body"]);
    }
}
