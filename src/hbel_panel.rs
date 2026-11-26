//! HBEL tab: results view (chips → suggested comment) and source sheet view.
use crate::app::App;
use crate::hbel_custom;
use crate::hbel_data::{library, visible, Entry, Group, Library, Link, Row, GROUPS, ROWS};
use std::sync::Arc;
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
    /// Built-in comments plus those added through the shared "6. HBEL" folder.
    lib: Option<Arc<Library>>,
    /// Problems reading the shared folder (bad files, clashing codes).
    pub folder_errors: Vec<String>,
    form: Option<NewForm>,
    /// Confirmation after saving a new comment.
    saved: Option<String>,
    /// Rows highlighted in the source sheet last frame (scroll when it changes).
    last_highlight: Vec<u8>,
}

/// The "Add a new comment" form.
#[derive(Default)]
struct NewForm {
    comment: String,
    remark: String,
    example: String,
    /// The folder is missing and the user is being asked whether to create it.
    confirm_create: bool,
    error: Option<String>,
    /// Scroll the form into view on the next frame (set when it is opened).
    scroll_to: bool,
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
        self.saved = None;
    }

    /// Screenshot harness: open the form with sample text (and the folder prompt if `confirm`).
    #[cfg(feature = "screenshot")]
    pub fn demo_form(&mut self, confirm: bool) {
        self.form = Some(NewForm { comment: "HbA present. New pattern under review.".into(), example: "26AB1234".into(), confirm_create: confirm, ..Default::default() });
    }

    /// Re-read the shared folder.
    pub fn reload(&mut self) {
        let (custom, mut errors) = match hbel_custom::folder_path() {
            Some(dir) => hbel_custom::load(&dir),
            None => (Vec::new(), Vec::new()),
        };
        errors.extend(hbel_custom::combined_errors(library(), &custom));
        self.lib = Some(Arc::new(hbel_custom::combined(library(), &custom)));
        self.folder_errors = errors;
    }

    /// The comment library, reading the shared folder the first time it is needed.
    pub fn lib(&mut self) -> Arc<Library> {
        if self.lib.is_none() {
            self.reload();
        }
        self.lib.clone().unwrap()
    }

    /// Build the new comment from the form and the entered results and save it.
    /// `create` = the user agreed to create the missing folder.
    fn save_new(&mut self, create: bool) {
        let lib = self.lib();
        let Some(form) = self.form.as_mut() else { return };
        form.error = None;
        let entry = match hbel_custom::entry_from_inputs(hbel_custom::next_code(&lib), &form.comment, &form.remark, &form.example, &self.inputs, &hbel_custom::stamp()) {
            Ok(e) => e,
            Err(e) => {
                form.error = Some(e);
                return;
            }
        };
        let Some(dir) = hbel_custom::folder_path() else {
            form.error = Some("Could not find the folder this program is running from.".into());
            return;
        };
        if !dir.is_dir() {
            if !create {
                form.confirm_create = true;
                return;
            }
            if let Err(e) = std::fs::create_dir_all(&dir) {
                form.error = Some(format!("Could not create {}: {}", dir.display(), e));
                form.confirm_create = false;
                return;
            }
        }
        match hbel_custom::save(&dir, &entry) {
            Ok((code, path)) => {
                self.form = None;
                self.reload();
                self.saved = Some(format!("Saved as code {} to {}", code, path.display()));
                self.view_code(code);
            }
            Err(e) => form.error = Some(e),
        }
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

fn entry(lib: &Library, code: u8) -> Option<&Entry> {
    lib.entries.iter().find(|e| e.code == code)
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
        let lib = self.hbel.lib();
        let Some(e) = entry(&lib, code) else { return };
        ui.ctx().copy_text(e.comment.clone());
        self.hbel.copied = Some((code, ui.input(|i| i.time)));
    }

    /// The source sheet, drawn in the right-hand side panel.
    pub fn hbel_source(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        // header: title + close, then search and filters
        text_ls(ui, th, pos2(origin.x, origin.y + 18.0), Align2::LEFT_CENTER, "Source sheet", Role::Display, 18.0, th.text, -0.01);
        let hr = Rect::from_min_max(pos2(origin.x + w - 400.0, origin.y), pos2(origin.x + w, origin.y + 36.0));
        let mut hc = ui.new_child(UiBuilder::new().max_rect(hr).layout(Layout::right_to_left(Align::Min)));
        hc.spacing_mut().item_spacing.x = 10.0;
        if button(&mut hc, th, "Close", Btn::Secondary, true).clicked() {
            self.hbel.source_open = false;
        }
        if button(&mut hc, th, "Reload", Btn::Secondary, true).on_hover_text("Re-read new comments added on other PCs").clicked() {
            self.hbel.reload();
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

        for err in self.hbel.folder_errors.clone() {
            y = note(ui, th, pos2(origin.x, y), Align2::LEFT_TOP, &format!("{}: {}", hbel_custom::FOLDER, err), w).bottom() + 8.0;
        }
        let lib = self.hbel.lib();
        let shown = visible(&lib, &self.hbel.search, self.hbel.filter);
        let copied = self.hbel_copied(ui);
        let mut copy_req = None;
        let mut view_req = None;
        if shown.is_empty() {
            y = paragraph(ui, th, pos2(origin.x, y), w, "No comments match.", Role::Sans, 14.0, th.muted, 1.45).bottom();
        } else {
            let n = format!("{} of {} comments · click a code to open it in the results view", shown.len(), lib.entries.len());
            let n = format!("{} · highlighted rows fit the entered results", n);
            y = text(ui, th, pos2(origin.x, y), Align2::LEFT_TOP, &n, Role::Sans, 12.5, th.muted).bottom() + 8.0;
            let hl = highlight_codes(&lib, &self.hbel.inputs, self.hbel.viewing());
            let (bottom, first) = source_table(ui, th, pos2(origin.x, y), w, &shown, &hl, copied, &mut copy_req, &mut view_req);
            y = bottom;
            if hl != self.hbel.last_highlight {
                if let Some(r) = first {
                    ui.scroll_to_rect(r, Some(Align::Center));
                }
                self.hbel.last_highlight = hl;
            }
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
        let lib = self.hbel.lib();
        let mut clear = false;
        let mut open_src = false;
        let mut open_form = false;
        let form_open = self.hbel.form.is_some();
        let mut y = toolbar(ui, origin, w, |ui| {
            if button(ui, th, "Clear", Btn::Secondary, true).clicked() {
                clear = true;
            }
            if button(ui, th, if form_open { "Cancel new" } else { "New comment" }, Btn::Secondary, true).on_hover_text("Add a new comment for a case no comment fits").clicked() {
                open_form = true;
            }
            let src_label = if self.hbel.source_open { "Hide sheet" } else { "Source sheet" };
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
        if open_form {
            self.hbel.form = if form_open { None } else { Some(NewForm { scroll_to: true, ..Default::default() }) };
            self.hbel.saved = None;
        }
        if let Some(msg) = self.hbel.saved.clone() {
            let r = chip(ui, th, pos2(origin.x, y), Align2::LEFT_TOP, &format!("✓ {}", msg), Role::SansB, 13.0, th.ok, th.ok_bg, Some((1.0, th.ok)), vec2(12.0, 6.0), th.radius);
            y = r.bottom() + 14.0;
        }

        let wide = false; // suggested comment always sits below the results
        let gap = 16.0;
        let (rw, sw) = if wide { ((w - gap) * 0.56, (w - gap) * 0.44) } else { (w, w) };
        let opts = CardOpts { title: "Results", unit: "blank rows match anything", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0 };
        let inputs = &mut self.hbel.inputs;
        let r_rect = card(ui, th, pos2(origin.x, y), rw, &opts, |ui, iw| results_body(ui, th, iw, inputs));

        let s_pos = if wide { pos2(origin.x + rw + gap, y) } else { pos2(origin.x, r_rect.bottom() + gap) };
        let sug = suggest(&lib, &self.hbel.inputs);
        let viewing = self.hbel.viewing();
        let copied = self.hbel_copied(ui);
        let mut copy_req: Option<u8> = None;
        let opts = CardOpts { title: "Suggested comment", unit: "", bar: None, head_bg: None, title_color: None, chip: None, min_height: if wide { r_rect.height() } else { 0.0 } };
        let s_rect = card(ui, th, s_pos, sw, &opts, |ui, iw| {
            let o = ui.cursor().min;
            let mut y = o.y;
            let muted = |ui: &Ui, y: f32, s: &str| paragraph(ui, th, pos2(o.x, y), iw, s, Role::Sans, 14.0, th.muted, 1.45).bottom();
            if let Some(e) = viewing.and_then(|c| entry(&lib, c)) {
                y = muted(ui, y, &format!("Viewing code {} (not from your results)", e.code)) + 10.0;
                y = entry_block(ui, th, o.x, y, iw, e, None, copied, &mut copy_req);
            } else {
                match sug.state {
                    State::Empty => y = muted(ui, y, "Enter results on the left."),
                    State::NoFit => y = muted(ui, y, "No comment fits exactly. If this is a new kind of case, use \u{201c}Add new comment\u{201d}.") + 4.0,
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
                            let Some(e) = entry(&lib, ev.code) else { continue };
                            y = entry_block(ui, th, o.x, y, iw, e, Some(ev), copied, &mut copy_req);
                            if sug.top.len() > 1 {
                                if let Some(h) = hint(ev, e) {
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
                        let Some(e) = entry(&lib, ev.code) else { continue };
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

        let mut bottom = if wide { r_rect.bottom().max(s_rect.bottom()) } else { s_rect.bottom() };
        if self.hbel.form.is_some() {
            bottom = self.hbel_form(ui, th, pos2(origin.x, bottom + gap), w, &lib);
        }
        footer(ui, th, origin.x, bottom + 18.0, w)
    }
}

impl App {
    /// "Add a new comment" card: criteria come from the entered results; saving exports
    /// the comment to the shared "6. HBEL" folder next to the exe. Returns the bottom y.
    fn hbel_form(&mut self, ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, lib: &Library) -> f32 {
        let next = hbel_custom::next_code(lib);
        let dir = hbel_custom::folder_path();
        let dir_s = dir.as_ref().map_or("(unknown)".to_string(), |d| d.display().to_string());
        let exists = dir.as_ref().is_some_and(|d| d.is_dir());
        let unit = format!("will be code {}", next);
        let opts = CardOpts { title: "Add new comment", unit: &unit, bar: Some(th.accent), head_bg: None, title_color: None, chip: None, min_height: 0.0 };
        let inputs = self.hbel.inputs.clone();
        let mut act: Option<&str> = None;
        let Some(form) = self.hbel.form.as_mut() else { return pos.y };
        let rect = card(ui, th, pos, w, &opts, |ui, iw| {
            let o = ui.cursor().min;
            let mut y = o.y;
            y = caption(ui, th, o.x, y, "Criteria (from the results entered above)");
            let mut any = false;
            for row in ROWS {
                if inputs.value(row).is_none() {
                    continue;
                }
                any = true;
                let shown = match inputs.number(row) {
                    NumState::Valid(x) => {
                        let chip_l = row.options().iter().find(|o| o.range.is_some_and(|r| r.contains(x))).map_or("", |o| o.label);
                        format!("{} % (saved as the {} range)", crate::logic::fmt_js(x), chip_l)
                    }
                    _ => row.options().iter().enumerate().filter(|(i, _)| inputs.chip_selected(row, *i)).map(|(_, o)| o.label).collect::<Vec<_>>().join(", "),
                };
                y = crit_line(ui, th, o.x, y, iw, row_name(row), &shown, None, false);
            }
            if !any {
                y = paragraph(ui, th, pos2(o.x, y), iw, "No results entered yet. Enter the new case's results above; they become this comment's criteria.", Role::Sans, 13.5, th.bad, 1.4).bottom() + 4.0;
            }
            y += 10.0;
            label(ui, th, pos2(o.x, y), "Comment text (as it should go into LabWare)");
            let ar = Rect::from_min_size(pos2(o.x, y + LABEL_H + LABEL_GAP), vec2(iw, 84.0));
            text_area(ui, th, ar, Id::new("hbel-new-comment"), &mut form.comment, "e.g. HbA present. …");
            y = ar.bottom() + 14.0;
            let half = (iw - 16.0) / 2.0;
            labeled_text_field(ui, th, pos2(o.x, y), half, "Remark (optional)", Id::new("hbel-new-remark"), &mut form.remark, "");
            labeled_text_field(ui, th, pos2(o.x + half + 16.0, y), half, "Example sample ID (optional)", Id::new("hbel-new-example"), &mut form.example, "e.g. 26AB1234");
            y += LABEL_H + LABEL_GAP + FIELD_H + 16.0;
            let msg = format!(
                "Saving exports this comment to the shared folder {} . Everyone who runs the app from this location will see it and it will be used for matching.",
                dir_s
            );
            y = note(ui, th, pos2(o.x, y), Align2::LEFT_TOP, &msg, iw).bottom() + 12.0;
            if let Some(err) = &form.error {
                y = paragraph(ui, th, pos2(o.x, y), iw, err, Role::SansB, 13.5, th.bad, 1.4).bottom() + 10.0;
            }
            let br = Rect::from_min_size(pos2(o.x, y), vec2(iw, 36.0));
            let mut bc = ui.new_child(UiBuilder::new().max_rect(br).layout(Layout::left_to_right(Align::Min)));
            bc.spacing_mut().item_spacing.x = 10.0;
            if form.confirm_create && !exists {
                let q = format!("The folder \u{201c}{}\u{201d} does not exist yet. Create it now and save?", hbel_custom::FOLDER);
                let qr = paragraph(ui, th, pos2(o.x, y), iw, &q, Role::SansB, 13.5, th.warn, 1.4);
                let br2 = Rect::from_min_size(pos2(o.x, qr.bottom() + 8.0), vec2(iw, 36.0));
                let mut bc2 = ui.new_child(UiBuilder::new().max_rect(br2).layout(Layout::left_to_right(Align::Min)));
                bc2.spacing_mut().item_spacing.x = 10.0;
                if button(&mut bc2, th, "Create folder and save", Btn::Primary, true).clicked() {
                    act = Some("create");
                }
                if button(&mut bc2, th, "Don't create", Btn::Secondary, true).clicked() {
                    act = Some("no-create");
                }
                y = br2.bottom();
            } else {
                let save_l = format!("Export to {}", hbel_custom::FOLDER);
                if button(&mut bc, th, &save_l, Btn::Primary, true).clicked() {
                    act = Some("save");
                }
                if button(&mut bc, th, "Cancel", Btn::Secondary, true).clicked() {
                    act = Some("cancel");
                }
                y = br.bottom();
            }
            ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y)), Sense::hover());
        });
        match act {
            Some("save") => self.hbel.save_new(false),
            Some("create") => self.hbel.save_new(true),
            Some("no-create") => {
                if let Some(f) = self.hbel.form.as_mut() {
                    f.confirm_create = false;
                    f.error = Some(format!("Not saved: the folder \u{201c}{}\u{201d} was not created.", hbel_custom::FOLDER));
                }
            }
            Some("cancel") => self.hbel.form = None,
            _ => {}
        }
        if let Some(f) = self.hbel.form.as_mut() {
            if std::mem::take(&mut f.scroll_to) {
                ui.scroll_to_rect(rect, Some(Align::TOP));
            }
        }
        rect.bottom()
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

/// Sheet columns shown in the source table: (sheet column, width).
pub(crate) const TABLE_COLS: [(&str, f32); 8] = [
    ("A2", 62.0),
    ("F", 52.0),
    ("Hgb", 52.0),
    ("RBC indices", 64.0),
    ("C window", 34.0),
    ("S window", 34.0),
    ("Fe", 46.0),
    ("HbH inclusion body", 50.0),
];
/// Short headers so the narrow columns stay readable.
const TABLE_HEADS: [&str; 8] = ["A2", "F", "Hgb", "RBC indices", "C", "S", "Fe", "HbH"];
const COMMENT_MIN: f32 = 148.0;
const NOTES_COL: f32 = 230.0;
const CELL_PAD: f32 = 5.0;
const CELL_FONT: f32 = 11.4;
const COMMENT_FONT: f32 = 12.4;
const CODE_LINE: f32 = 24.0;

/// Codes to highlight in the source sheet: the viewed code, else every top fit.
pub(crate) fn highlight_codes(lib: &Library, inputs: &Inputs, viewing: Option<u8>) -> Vec<u8> {
    if let Some(c) = viewing {
        return vec![c];
    }
    suggest(lib, inputs).top.iter().map(|e| e.code).collect()
}

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
    if let Some(f) = &e.source {
        out.push(format!("File: {}\\{}", hbel_custom::FOLDER, f));
    }
    out.join("\n")
}

/// Layout of a comment cell: where the copy button goes (left-middle, relative to the
/// cell's text origin) and the cell's text height including the button.
fn comment_cell(ui: &Ui, th: &Theme, width: f32, comment: &str, copy_label: &str) -> (Pos2, f32) {
    let (end, h) = text_end(ui, th, width, comment, Role::Serif, COMMENT_FONT, 1.35);
    let bw = copy_button_width(ui, th, copy_label);
    if end.x + 8.0 + bw <= width {
        (pos2(end.x + 8.0, end.y), h)
    } else {
        (pos2(0.0, h + 10.0), h + 20.0)
    }
}

/// The source sheet as a table. Comment and the sheet columns fit the visible width;
/// only "Notes & links" sits past the right edge (scroll sideways to read it).
/// Rows whose code is in `highlight` are highlighted. Returns the bottom y and the
/// first highlighted row's rect.
#[allow(clippy::too_many_arguments)]
fn source_table(ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, rows: &[&Entry], highlight: &[u8], copied: Option<u8>, copy_req: &mut Option<u8>, view_req: &mut Option<u8>) -> (f32, Option<Rect>) {
    // spare width: up to 60% widens the sheet columns (to at most 2x), the rest goes to Comment
    let fixed: f32 = TABLE_COLS.iter().map(|c| c.1).sum();
    let spare = (w - fixed - COMMENT_MIN).max(0.0);
    let grow = (spare * 0.6 / fixed).min(1.0);
    let cols: Vec<f32> = TABLE_COLS.iter().map(|c| c.1 * (1.0 + grow)).collect();
    let mut widths = vec![(w - cols.iter().sum::<f32>()).max(COMMENT_MIN)];
    widths.extend(cols);
    widths.push(NOTES_COL);
    let tw: f32 = widths.iter().sum();
    let heads: Vec<&str> = std::iter::once("Comment").chain(TABLE_HEADS).chain(["Notes & links →"]).collect();
    let wrap = |i: usize| widths[i] - CELL_PAD * 2.0;
    let last = widths.len() - 1;
    let link_h = 18.0;

    let row_h: Vec<f32> = rows
        .iter()
        .map(|e| {
            let mut h = CODE_LINE + comment_cell(ui, th, wrap(0), &e.comment, "Copied").1;
            for (k, (col, _)) in TABLE_COLS.iter().enumerate() {
                h = h.max(paragraph_height(ui, th, wrap(k + 1), &cell_text(e, col), Role::Sans, CELL_FONT, 1.35));
            }
            let notes = notes_text(e);
            let nh = if notes.is_empty() { 0.0 } else { paragraph_height(ui, th, wrap(last), &notes, Role::Sans, 11.5, 1.35) + 4.0 };
            h.max(nh + e.links.len() as f32 * link_h) + CELL_PAD * 2.0
        })
        .collect();
    let head_h = heads.iter().enumerate().map(|(i, h)| paragraph_height(ui, th, wrap(i), h, Role::SansB, 11.5, 1.25)).fold(0.0_f32, f32::max) + 12.0;
    let table_h = head_h + row_h.iter().sum::<f32>();
    let area = Rect::from_min_size(pos, vec2(w, table_h + 14.0));
    let mut first_hl: Option<Rect> = None;
    let hl_bar = if th.palette == crate::theme::Palette::Genshin { th.gold } else { th.accent };

    ui.scope_builder(UiBuilder::new().max_rect(area), |ui| {
        egui::ScrollArea::horizontal().id_salt("hbel-table").auto_shrink([false, true]).drag_to_scroll(false).show(ui, |ui| {
            let o = ui.cursor().min;
            let p = ui.painter().clone();
            let frame = Rect::from_min_size(o, vec2(tw, table_h));
            let r = th.radius_lg;
            p.rect_filled(frame, cr(r), th.surface);
            let head = Rect::from_min_size(o, vec2(tw, head_h));
            p.with_clip_rect(head).rect_filled(frame, cr(r), th.surface2);
            let mut x = o.x;
            for (i, h) in heads.iter().enumerate() {
                let c = if i == last { th.muted } else { th.text };
                paragraph(ui, th, pos2(x + CELL_PAD, o.y + 6.0), wrap(i), h, Role::SansB, 11.5, c, 1.25);
                x += widths[i];
            }
            let mut y = o.y + head_h;
            for (ri, e) in rows.iter().enumerate() {
                let rh = row_h[ri];
                let row_rect = Rect::from_min_size(pos2(o.x, y), vec2(tw, rh));
                if highlight.contains(&e.code) {
                    p.rect_filled(row_rect, 0.0, rgba(hl_bar, if th.dark { 0.22 } else { 0.14 }));
                    p.rect_filled(Rect::from_min_size(row_rect.min, vec2(4.0, rh)), 0.0, hl_bar);
                    if first_hl.is_none() {
                        first_hl = Some(row_rect);
                    }
                } else if ri % 2 == 1 {
                    p.rect_filled(row_rect, 0.0, rgba(th.surface2, 0.55));
                }
                p.line_segment([pos2(o.x, y), pos2(o.x + tw, y)], Stroke::new(1.0_f32, th.line));
                let top = y + CELL_PAD;
                let x0 = o.x + CELL_PAD + 2.0;
                // code badge (click: show in the results view) + "Added" badge
                let cb = chip(ui, th, pos2(x0, top), Align2::LEFT_TOP, &format!("Code {}", e.code), Role::MonoB, 11.5, th.accent_dark, th.accent_soft, Some((1.0, th.accent)), vec2(7.0, 1.5), pill());
                let resp = ui.interact(cb, Id::new(("hbel-src-code", e.code)), Sense::click()).on_hover_text("Show in the results view");
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    *view_req = Some(e.code);
                }
                if e.source.is_some() {
                    chip(ui, th, pos2(cb.right() + 6.0, top + 1.0), Align2::LEFT_TOP, "Added", Role::SansB, 10.5, th.ok, th.ok_bg, None, vec2(6.0, 1.0), pill());
                }
                // comment (selectable) with the copy button after its last word
                let cpos = pos2(x0, top + CODE_LINE);
                selectable_paragraph(ui, th, cpos, wrap(0), &e.comment, Role::Serif, COMMENT_FONT, th.text, 1.35);
                let cl = if copied == Some(e.code) { "Copied" } else { "Copy" };
                let (at, _) = comment_cell(ui, th, wrap(0), &e.comment, cl);
                if copy_button(ui, th, cpos + at.to_vec2(), cl, Id::new(("hbel-src-copy", e.code))).clicked() {
                    *copy_req = Some(e.code);
                }
                let mut x = o.x + widths[0];
                for (k, (col, _)) in TABLE_COLS.iter().enumerate() {
                    let t = cell_text(e, col);
                    if t.is_empty() {
                        text(ui, th, pos2(x + CELL_PAD, top), Align2::LEFT_TOP, "—", Role::Sans, CELL_FONT, th.muted);
                    } else {
                        selectable_paragraph(ui, th, pos2(x + CELL_PAD, top), wrap(k + 1), &t, Role::Sans, CELL_FONT, th.text, 1.35);
                    }
                    x += widths[k + 1];
                }
                let notes = notes_text(e);
                let mut ny = top;
                if !notes.is_empty() {
                    ny = selectable_paragraph(ui, th, pos2(x + CELL_PAD, top), wrap(last), &notes, Role::Sans, 11.5, th.text, 1.35).bottom() + 4.0;
                }
                for l in &e.links {
                    ny = link(ui, th, pos2(x + CELL_PAD, ny), l, wrap(last)).bottom() + 4.0;
                }
                y += rh;
            }
            let mut x = o.x;
            for (i, wd) in widths[..last].iter().enumerate() {
                x += wd;
                let c = if i == last - 1 { th.line_strong } else { rgba(th.line, 0.6) };
                p.line_segment([pos2(x, o.y), pos2(x, o.y + table_h)], Stroke::new(1.0_f32, c));
            }
            stroke_in(&p, frame, r, if th.palette == crate::theme::Palette::Pokemon { 2.0 } else { 1.0 }, th.line);
            ui.allocate_rect(frame, Sense::hover());
        });
    });
    ui.allocate_rect(area, Sense::hover());
    (area.bottom(), first_hl)
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
        assert_eq!(notes_text(e(4)), "Examples: 26AK7710, 26AQ1798");
        assert!(notes_text(e(21)).starts_with("Examples: 24AR2337, 24AU2617\nReference: Patel"));
        assert_eq!(TABLE_COLS.iter().map(|(c, _)| *c).collect::<Vec<_>>(), vec!["A2", "F", "Hgb", "RBC indices", "C window", "S window", "Fe", "HbH inclusion body"]);
    }

    #[test]
    fn save_flow_asks_before_creating_folder() {
        let dir = hbel_custom::folder_path().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = HbelState::default();
        s.inputs.hba2 = "3.7".into();
        s.inputs.click_chip(Row::Hb, 1);
        s.form = Some(NewForm { comment: "A brand new pattern".into(), ..Default::default() });
        s.save_new(false);
        assert!(!dir.exists(), "must not create the folder without asking");
        assert!(s.form.as_ref().unwrap().confirm_create);
        s.save_new(true);
        assert!(dir.is_dir());
        assert!(s.form.is_none());
        assert!(s.saved.as_deref().unwrap().starts_with("Saved as code 27 to "));
        assert_eq!(s.viewing(), Some(27));
        assert!(s.lib().entries.iter().any(|e| e.code == 27 && e.comment == "A brand new pattern."));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn highlight_follows_suggestion() {
        let lib = library();
        let mut i = Inputs { hba2: "2.8".into(), hbf: "3".into(), ..Default::default() };
        assert_eq!(highlight_codes(lib, &i, None), vec![17, 21, 23]);
        assert_eq!(highlight_codes(lib, &i, Some(7)), vec![7]);
        i.clear();
        assert!(highlight_codes(lib, &i, None).is_empty());
    }
}
