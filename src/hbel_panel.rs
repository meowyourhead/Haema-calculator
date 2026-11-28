//! HBEL tab: results view (chips → suggested comment) and source sheet view.
use crate::app::App;
use crate::hbel_custom;
use crate::hbel_data::{library, visible, Entry, Group, Library, Link, Row, GROUPS, ROWS};
use std::sync::Arc;
use crate::hbel_match::{describe, hint, row_name, suggest, Eval, Inputs, NumState, Outcome, State};
use crate::theme::{mix, rgba, Role, Theme};
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
    /// Where the divider sits, as a fraction of the window width (None = half).
    pub split: Option<f32>,
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
        self.form = Some(NewForm { comment: "HbA present. New pattern under review.".into(), example: "26XX1234".into(), confirm_create: confirm, ..Default::default() });
    }

    /// Re-read the shared folder.
    pub fn reload(&mut self) {
        let dir = hbel_custom::folder_path();
        let (custom, mut errors) = match &dir {
            Some(dir) => hbel_custom::load(dir),
            None => (Vec::new(), Vec::new()),
        };
        errors.extend(hbel_custom::combined_errors(library(), &custom));
        let mut lib = hbel_custom::combined(library(), &custom);
        if let Some(dir) = &dir {
            let (examples, bad) = hbel_custom::load_examples(dir);
            errors.extend(bad);
            errors.extend(hbel_custom::attach_examples(&mut lib, &examples));
        }
        self.lib = Some(Arc::new(lib));
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
const NUM_W: f32 = 72.0;
const CHIP_GAP: f32 = 6.0;
const ROW_GAP: f32 = 8.0;
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

/// Width of the divider's hit area between the two columns (the visible grip is much slimmer).
const SPLIT_HIT: f32 = 18.0;
/// Space between a column's content and the divider.
const SPLIT_INNER: f32 = 10.0;
/// Narrowest a column may be dragged.
const SPLIT_MIN: f32 = 340.0;

impl App {
    /// The left column: results (with their actions), the new-comment form and the reference table.
    pub fn hbel_panel(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        self.hbel_results(ui, th, origin, w)
    }

    /// The whole HBEL page inside `screen`: results on the left; on the right
    /// either the suggested comment or the source sheet. A slim draggable grip
    /// divides them (by default the right gets room for the whole source table;
    /// double-click resets it).
    /// Returns the left column's content height.
    pub fn hbel_page(&mut self, ui: &mut Ui, th: &Theme, screen: Rect, focus_before: Option<Id>) -> f32 {
        let lo = (SPLIT_MIN / screen.width()).min(0.5);
        // By default the right column is wide enough for every sheet column of
        // the source table (only "Notes & links" sits past the edge), and never
        // narrower than half.
        let fixed: f32 = TABLE_COLS.iter().map(|c| c.1).sum();
        let right_need = fixed + COMMENT_COMFY + SPLIT_INNER + crate::app::PAGE_MARGIN + 12.0 + SPLIT_HIT / 2.0;
        let default = (1.0 - right_need / screen.width()).min(0.5);
        let frac = self.hbel.split.unwrap_or(default).clamp(lo, 1.0 - lo);
        let split_x = (screen.left() + screen.width() * frac).round();
        let grip = Rect::from_min_max(pos2(split_x - SPLIT_HIT / 2.0, screen.top()), pos2(split_x + SPLIT_HIT / 2.0, screen.bottom()));
        let left = Rect::from_min_max(screen.min, pos2(grip.left(), screen.bottom()));
        let right = Rect::from_min_max(pos2(grip.right(), screen.top()), screen.max);

        // --- divider: a short rounded grip, a faint guide line only while hovered or dragged
        let resp = ui.interact(grip, Id::new("hbel-split"), Sense::click_and_drag()).on_hover_text("Drag to resize · double-click to reset");
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if resp.dragged() {
            if let Some(p) = resp.interact_pointer_pos() {
                self.hbel.split = Some(((p.x - screen.left()) / screen.width()).clamp(lo, 1.0 - lo));
            }
        }
        if resp.double_clicked() {
            self.hbel.split = None;
        }
        let hot = ui.ctx().animate_bool_with_time(Id::new("hbel-split-hot"), resp.hovered() || resp.dragged(), 0.12);
        let accent = if th.palette == crate::theme::Palette::Genshin { th.gold } else { th.accent };
        let p = ui.painter();
        if hot > 0.0 {
            p.line_segment([pos2(split_x, screen.top() + 8.0), pos2(split_x, screen.bottom() - 8.0)], Stroke::new(1.0_f32, rgba(accent, 0.35 * hot)));
        }
        let gh = 36.0 + 12.0 * hot;
        let gw = 4.0 + 1.0 * hot;
        let g = Rect::from_center_size(pos2(split_x, screen.center().y), vec2(gw, gh));
        fill(p, g, gw / 2.0, crate::theme::mix(hot, accent, th.line_strong));

        // --- columns, each scrolling on its own
        let mut left_h = 0.0;
        ui.scope_builder(UiBuilder::new().max_rect(left), |ui| {
            egui::ScrollArea::vertical().id_salt("hbel-left").auto_shrink([false, false]).drag_to_scroll(false).show(ui, |ui| {
                let o = ui.cursor().min + vec2(crate::app::PAGE_MARGIN, crate::app::PAGE_MARGIN);
                let w = (ui.available_width() - crate::app::PAGE_MARGIN - SPLIT_INNER).max(240.0);
                let bottom = self.hbel_results(ui, th, o, w);
                let top = ui.cursor().min;
                ui.allocate_rect(Rect::from_min_max(top, pos2(o.x + w, bottom + crate::app::PAGE_MARGIN)), Sense::hover());
                left_h = bottom - o.y;
                crate::app::nav_fields(ui, focus_before);
            });
        });
        ui.scope_builder(UiBuilder::new().max_rect(right), |ui| {
            egui::ScrollArea::vertical().id_salt(("hbel-right", self.hbel.source_open)).auto_shrink([false, false]).drag_to_scroll(false).show(ui, |ui| {
                // the view toggle row centres on the Results card's title bar
                let o = ui.cursor().min + vec2(SPLIT_INNER, crate::app::PAGE_MARGIN + (HEAD_H - VIEW_H) / 2.0);
                let w = (ui.available_width() - crate::app::PAGE_MARGIN - SPLIT_INNER).max(240.0);
                let bottom = if self.hbel.source_open { self.hbel_source(ui, th, o, w) } else { self.hbel_suggestion(ui, th, o, w) };
                let top = ui.cursor().min;
                ui.allocate_rect(Rect::from_min_max(top, pos2(o.x + w, bottom + crate::app::PAGE_MARGIN)), Sense::hover());
            });
        });
        left_h
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
        // header: title + back to the suggestion, then search and filters
        let tip = match &self.excel_path {
            Some(p) => format!("Open {}\n(change the file in Settings)", p.display()),
            None => "Choose the HBEL comments Excel file, then open it".to_string(),
        };
        let busy = self.excel_picker.busy();
        // one row: view toggle · search · Open Excel
        let toggle = view_toggle(ui, th, origin, &mut self.hbel.source_open);
        let hr = Rect::from_min_max(pos2(toggle.right() + 12.0, origin.y), pos2(origin.x + w, origin.y + VIEW_H));
        let mut hc = ui.new_child(UiBuilder::new().max_rect(hr).layout(Layout::right_to_left(Align::Min)));
        let excel_r = button(&mut hc, th, "Open Excel", Btn::Secondary, !busy).on_hover_text(tip);
        if excel_r.clicked() {
            let ctx = ui.ctx().clone();
            self.open_excel(&ctx);
        }
        let room = excel_r.rect.left() - 10.0 - hr.left();
        // too narrow for the search beside the toggle: it takes its own row
        let sr = if room >= 170.0 {
            let sw = room.min(420.0);
            Rect::from_min_size(pos2(excel_r.rect.left() - 10.0 - sw, origin.y + (VIEW_H - FIELD_H) / 2.0), vec2(sw, FIELD_H))
        } else {
            Rect::from_min_size(pos2(origin.x, origin.y + VIEW_H + 12.0), vec2(w, FIELD_H))
        };
        text_field(ui, th, sr, Id::new("hbel-search"), &mut self.hbel.search, "Search comments…", false);

        let mut y = sr.bottom().max(origin.y + VIEW_H) + 16.0;
        if let Some(msg) = self.excel_msg.clone() {
            y = paragraph(ui, th, pos2(origin.x, y), w, &msg, Role::SansB, 13.0, th.bad, 1.4).bottom() + 10.0;
        }
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
        // the variant library link(s) close the filter row, right-aligned when they fit
        let links_w: f32 = library().resources.iter().map(|l| text_width(ui, th, &format!("{} ↗", l.label), Role::SansB, 13.0)).sum::<f32>()
            + 24.0 * library().resources.len().saturating_sub(1) as f32;
        let (lx, ly) = if x + 16.0 + links_w <= origin.x + w { (origin.x + w - links_w, y + (CHIP_H - 18.0) / 2.0) } else { (origin.x, y + CHIP_H + 10.0) };
        let lb = resource_links(ui, th, lx, ly, origin.x + w - lx);
        y = (y + CHIP_H).max(lb) + 18.0;

        for err in self.hbel.folder_errors.clone() {
            y = note(ui, th, pos2(origin.x, y), Align2::LEFT_TOP, &format!("{}: {}", hbel_custom::FOLDER, err), w).bottom() + 8.0;
        }
        let lib = self.hbel.lib();
        let hl = highlight_codes(&lib, &self.hbel.inputs, self.hbel.viewing());
        let shown = fits_first(visible(&lib, &self.hbel.search, self.hbel.filter), &hl);
        let n_hl = shown.iter().filter(|e| hl.contains(&e.code)).count();
        let copied = self.hbel_copied(ui);
        let mut copy_req = None;
        let mut view_req = None;
        if shown.is_empty() {
            y = paragraph(ui, th, pos2(origin.x, y), w, "No comments match.", Role::Sans, 14.0, th.muted, 1.45).bottom();
        } else {
            let (bottom, first) = source_table(ui, th, pos2(origin.x, y), w, &shown, &hl, n_hl, copied, &mut copy_req, &mut view_req);
            y = bottom;
            if hl != self.hbel.last_highlight {
                if let Some(r) = first {
                    // fitting rows head the table: scroll only if they are out of view
                    ui.scroll_to_rect(r.expand2(vec2(0.0, 40.0)), None);
                }
                self.hbel.last_highlight = hl;
            }
        }
        if let Some(code) = copy_req {
            self.hbel_copy(ui, code);
        }
        if let Some(code) = view_req {
            // the suggestion takes the sheet's place on the right
            self.hbel.view_code(code);
            self.hbel.source_open = false;
        }
        ui.allocate_rect(Rect::from_min_max(origin, pos2(origin.x + w, y)), Sense::hover());
        y
    }

    fn hbel_results(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        self.hbel.refresh_viewing();
        let lib = self.hbel.lib();
        let form_open = self.hbel.form.is_some();
        let mut y = origin.y;
        if let Some(msg) = self.hbel.saved.clone() {
            let r = chip(ui, th, pos2(origin.x, y), Align2::LEFT_TOP, &format!("✓ {}", msg), Role::SansB, 13.0, th.ok, th.ok_bg, Some((1.0, th.ok)), vec2(12.0, 6.0), th.radius);
            y = r.bottom() + 14.0;
        }

        let gap = 16.0;
        let opts = CardOpts { title: "Results", unit: "", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0, alert: false, status: None, collapsed: false };
        let inputs = &mut self.hbel.inputs;
        let r_rect = card(ui, th, pos2(origin.x, y), w, &opts, |ui, iw| results_body(ui, th, iw, inputs));

        // the card's own actions sit in its title bar, right-aligned
        let hr = Rect::from_min_max(pos2(r_rect.left() + 120.0, r_rect.top()), pos2(r_rect.right() - 16.0, r_rect.top() + HEAD_H));
        let mut hc = ui.new_child(UiBuilder::new().max_rect(hr).layout(Layout::right_to_left(Align::Center)));
        hc.spacing_mut().item_spacing.x = 8.0;
        let clear = button(&mut hc, th, "Clear", Btn::MiniMuted, true).on_hover_text("Clear every result").clicked();
        let new_label = if form_open { "Cancel new" } else { "New comment" };
        let open_form = button(&mut hc, th, new_label, Btn::Mini, true).on_hover_text("Add a new comment for a case no comment fits").clicked();
        if clear {
            self.hbel.clear();
        }
        if open_form {
            self.hbel.form = if form_open { None } else { Some(NewForm { scroll_to: true, ..Default::default() }) };
            self.hbel.saved = None;
        }

        let mut bottom = r_rect.bottom();
        if self.hbel.form.is_some() {
            bottom = self.hbel_form(ui, th, pos2(origin.x, bottom + gap), w, &lib);
        }
        footer(ui, th, origin.x, bottom + 18.0, w, &self.hbel.inputs)
    }

    /// The right column when the source sheet is closed: the suggested comment.
    pub fn hbel_suggestion(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        let lib = self.hbel.lib();
        // the view toggle heads the column
        let y = view_toggle(ui, th, origin, &mut self.hbel.source_open).bottom() + 16.0;
        let sug = suggest(&lib, &self.hbel.inputs);
        let viewing = self.hbel.viewing();
        let copied = self.hbel_copied(ui);
        let mut copy_req: Option<u8> = None;
        let opts = CardOpts { title: "Suggested comment", unit: "", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0, alert: false, status: None, collapsed: false };
        let s_rect = card(ui, th, pos2(origin.x, y), w, &opts, |ui, iw| {
            let o = ui.cursor().min;
            let mut y = o.y;
            let muted = |ui: &Ui, y: f32, s: &str| paragraph(ui, th, pos2(o.x, y), iw, s, Role::Sans, 14.0, th.muted, 1.45).bottom();
            if let Some(e) = viewing.and_then(|c| entry(&lib, c)) {
                y = muted(ui, y, &format!("Viewing code {} (not from your results)", e.code)) + 10.0;
                y = entry_block(ui, th, o.x, y, iw, e, None, copied, &mut copy_req);
            } else {
                match sug.state {
                    State::Empty => y = muted(ui, y, "Enter results on the left."),
                    State::NoFit => y = muted(ui, y, "No comment fits exactly. If this is a new kind of case, use \u{201c}New comment\u{201d} on the left.") + 4.0,
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
        s_rect.bottom()
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
        let opts = CardOpts { title: "Add new comment", unit: &unit, bar: Some(th.accent), head_bg: None, title_color: None, chip: None, min_height: 0.0, alert: false, status: None, collapsed: false };
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
            labeled_text_field(ui, th, pos2(o.x + half + 16.0, y), half, "Example sample ID (optional)", Id::new("hbel-new-example"), &mut form.example, "e.g. 26XX1234");
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

/// Reference ranges (rows of the table below): parameter, its row, reference interval.
const REF_RANGES: [(&str, Row, &str); 2] = [("HbA2 (%)", Row::Hba2, "2.2 – 3.4"), ("HbF (%)", Row::Hbf, "0 – 1.3")];
const REF_HEADS: [&str; 3] = ["Parameter", "Reference range", "Result bands (as the chips read a typed %)"];
const REF_PAD: f32 = 10.0;
const BAND_H: f32 = 22.0;
const BAND_GAP: f32 = 6.0;

/// A chip band's limits in words: `< 2.2`, `2.2 – < 3.5`, `≤ 1.3`, `> 50`.
pub(crate) fn band_text(r: &crate::hbel_data::Range) -> String {
    let f = crate::logic::fmt_js;
    if r.lo == 0.0 && r.lo_incl {
        format!("{} {}", if r.hi_incl { "≤" } else { "<" }, f(r.hi))
    } else if r.hi == 100.0 && r.hi_incl {
        format!("{} {}", if r.lo_incl { "≥" } else { ">" }, f(r.lo))
    } else {
        format!("{}{} – {}{}", if r.lo_incl { "" } else { "> " }, f(r.lo), if r.hi_incl { "" } else { "< " }, f(r.hi))
    }
}

/// Lay out (or, with `draw`, paint) one row's band pills inside `w`; returns the height used.
fn band_pills(ui: &Ui, th: &Theme, pos: Pos2, w: f32, row: Row, inputs: &Inputs, draw: bool) -> f32 {
    let (mut x, mut y) = (pos.x, pos.y);
    let accent = if th.palette == crate::theme::Palette::Genshin { th.gold } else { th.accent };
    for (i, opt) in row.options().iter().enumerate() {
        let Some(range) = opt.range else { continue };
        let lim = band_text(&range);
        let lw = text_width(ui, th, opt.label, Role::SansB, 12.0);
        let rw = text_width(ui, th, &lim, Role::Mono, 12.0);
        let pw = (lw + rw + 8.0 + 20.0).ceil();
        if x > pos.x && x + pw > pos.x + w {
            x = pos.x;
            y += BAND_H + BAND_GAP;
        }
        if draw {
            let r = Rect::from_min_size(pos2(x, y), vec2(pw, BAND_H));
            let on = inputs.chip_selected(row, i);
            let p = ui.painter();
            let (bg, border, fg) = if on { (th.accent_soft, accent, th.accent_dark) } else { (th.surface2, th.line, th.text) };
            fill(p, r, pill(), bg);
            stroke_in(p, r, pill(), 1.0, border);
            text(ui, th, pos2(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, opt.label, Role::SansB, 12.0, fg);
            text(ui, th, pos2(r.left() + 10.0 + lw + 8.0, r.center().y), Align2::LEFT_CENTER, &lim, Role::Mono, 12.0, if on { fg } else { th.muted });
        }
        x += pw + BAND_GAP;
    }
    y + BAND_H - pos.y
}

/// Reference ranges as a small table. Bands that the
/// entered HbA2 / HbF fall in are highlighted. Returns the bottom y.
pub(crate) fn footer(ui: &mut Ui, th: &Theme, x: f32, y: f32, w: f32, inputs: &Inputs) -> f32 {
    let head_size = 10.88;
    let head_w = |s: &str| text_width(ui, th, &s.to_uppercase(), Role::SansB, head_size) + 0.03 * head_size * s.chars().count() as f32;
    let c0 = REF_RANGES.iter().map(|(n, _, _)| text_width(ui, th, n, Role::SansB, 13.0)).fold(head_w(REF_HEADS[0]), f32::max) + 2.0 * REF_PAD + 4.0;
    let c1 = REF_RANGES.iter().map(|(_, _, r)| text_width(ui, th, r, Role::Mono, 13.0)).fold(head_w(REF_HEADS[1]), f32::max) + 2.0 * REF_PAD;
    let (c0, c1) = (c0.ceil(), c1.ceil());
    let c2 = (w - c0 - c1).max(150.0);
    let tw = c0 + c1 + c2;
    let xs = [x, x + c0, x + c0 + c1];
    let head_h = 30.0;
    let band_w = c2 - 2.0 * REF_PAD;
    let row_h: Vec<f32> = REF_RANGES.iter().map(|(_, row, _)| (band_pills(ui, th, pos2(0.0, 0.0), band_w, *row, inputs, false) + 2.0 * REF_PAD).max(40.0)).collect();
    let table_h = head_h + row_h.iter().sum::<f32>();
    let frame = Rect::from_min_size(pos2(x, y), vec2(tw, table_h));

    let p = ui.painter().clone();
    let rad = th.radius_lg;
    p.rect_filled(frame, cr(rad), th.surface);
    p.with_clip_rect(Rect::from_min_size(frame.min, vec2(tw, head_h))).rect_filled(frame, cr(rad), th.surface2);
    for (i, h) in REF_HEADS.iter().enumerate() {
        let hw = if i == 2 { c2 } else { [c0, c1][i] } - 2.0 * REF_PAD;
        let mut s = h.to_uppercase();
        // the long band header drops its explanation when the column is narrow
        if i == 2 && head_w(h) > hw {
            s = "RESULT BANDS".into();
        }
        text_ls(ui, th, pos2(xs[i] + REF_PAD, y + head_h / 2.0), Align2::LEFT_CENTER, &s, Role::SansB, head_size, th.muted, 0.03);
    }
    let accent_bar = if th.palette == crate::theme::Palette::Genshin { th.gold } else { th.accent };
    let mut ry = y + head_h;
    for (k, (name, row, range)) in REF_RANGES.iter().enumerate() {
        let h = row_h[k];
        p.line_segment([pos2(x, ry), pos2(x + tw, ry)], Stroke::new(1.0_f32, th.line));
        let pc = Rect::from_min_max(pos2(xs[0], ry), pos2(xs[1], ry + h));
        let cell_bg = match th.palette {
            crate::theme::Palette::Pokemon | crate::theme::Palette::Cyberpunk => th.surface2,
            _ => th.accent_soft,
        };
        // filled as the rounded frame clipped to the cell, so the bottom-left
        // corner keeps the table's rounding
        p.with_clip_rect(pc).rect_filled(frame, cr(rad), cell_bg);
        p.with_clip_rect(Rect::from_min_size(pc.min, vec2(3.0, h))).rect_filled(frame, cr(rad), accent_bar);
        text(ui, th, pos2(pc.left() + 3.0 + REF_PAD, pc.center().y), Align2::LEFT_CENTER, name, Role::SansB, 13.0, th.text);
        text(ui, th, pos2(xs[1] + REF_PAD, ry + h / 2.0), Align2::LEFT_CENTER, range, Role::Mono, 13.0, th.text);
        let bh = band_pills(ui, th, pos2(0.0, 0.0), band_w, *row, inputs, false);
        band_pills(ui, th, pos2(xs[2] + REF_PAD, ry + (h - bh) / 2.0), band_w, *row, inputs, true);
        ry += h;
    }
    for cx in [xs[1], xs[2]] {
        p.line_segment([pos2(cx - 0.5, y), pos2(cx - 0.5, y + table_h)], Stroke::new(1.0_f32, rgba(th.line, 0.8)));
    }
    stroke_in(&p, frame, rad, if th.palette == crate::theme::Palette::Pokemon { 2.0 } else { 1.0 }, th.line);
    ui.allocate_rect(frame, Sense::hover());
    frame.bottom()
}

/// The tab-wide links (Bio-Rad variant library, …), wrapping within `w`. Returns the bottom y.
pub(crate) fn resource_links(ui: &mut Ui, th: &Theme, x: f32, y: f32, w: f32) -> f32 {
    let mut lx = x;
    let mut line_y = y;
    let mut bottom = y;
    for l in &library().resources {
        let lw = text_width(ui, th, &format!("{} ↗", l.label), Role::SansB, 13.0);
        if lx > x && lx + lw > x + w {
            lx = x;
            line_y = bottom + 8.0;
        }
        let lr = link(ui, th, pos2(lx, line_y), l, w);
        lx = lr.right() + 24.0;
        bottom = bottom.max(lr.bottom());
    }
    ui.allocate_rect(Rect::from_min_max(pos2(x, y), pos2(x + w, bottom)), Sense::hover());
    bottom
}

/// Height of the right column's header row (view toggle, search, Open Excel).
const VIEW_H: f32 = 36.0;

/// One control that switches the right column between the suggested comment
/// and the source sheet. The filled thumb slides to the chosen side.
/// Returns its rect.
fn view_toggle(ui: &mut Ui, th: &Theme, pos: Pos2, source_open: &mut bool) -> Rect {
    use crate::theme::Palette;
    let names = ["Suggested comment", "Source sheet"];
    let pad = 16.0;
    let ws = names.map(|n| text_width(ui, th, n, Role::SansB, 13.44).ceil() + 2.0 * pad);
    let rect = Rect::from_min_size(pos, vec2(ws[0] + ws[1], VIEW_H));
    let p = ui.painter().clone();
    let r = match th.palette {
        Palette::Genshin => pill(),
        Palette::Pokemon => 14.0,
        Palette::Cyberpunk => 0.0,
        _ => th.radius,
    };
    let border = match th.palette {
        Palette::Pokemon => th.ring,
        Palette::Genshin => rgba(th.gold, 0.45),
        Palette::Cyberpunk => rgba(th.accent, 0.62),
        _ => th.line,
    };
    if th.palette == Palette::Cyberpunk {
        cyber_panel(&p, rect, 4.0, th.surface2, 1.0, border);
    } else {
        fill(&p, rect, r, th.surface2);
    }
    // thumb: slides between the halves, inset so the track shows around it
    let t = ui.ctx().animate_bool_with_time(Id::new("hbel-view-toggle"), *source_open, 0.16);
    let inset = 3.0;
    let tw = ws[0] + t * (ws[1] - ws[0]);
    let thumb = Rect::from_min_size(pos2(rect.left() + inset + t * ws[0], rect.top() + inset), vec2(tw - 2.0 * inset, VIEW_H - 2.0 * inset));
    let tr = if r >= pill() { pill() } else { (r - inset).max(0.0) };
    if th.palette == Palette::Cyberpunk {
        cyber_panel(&p, thumb, 3.0, th.accent, 0.0, th.accent);
    } else {
        fill(&p, thumb, tr, th.accent);
    }
    for (i, name) in names.iter().enumerate() {
        let hr = Rect::from_min_size(pos2(rect.left() + if i == 0 { 0.0 } else { ws[0] }, rect.top()), vec2(ws[i], VIEW_H));
        let want = i == 1;
        let tip = if want { "Show the full comment sheet" } else { "Show the comment that fits the entered results" };
        let resp = ui.interact(hr, Id::new(("hbel-view", i)), Sense::click()).on_hover_text(tip);
        if resp.hovered() && *source_open != want {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            *source_open = want;
        }
        // label colour follows the thumb, so it crossfades as it slides
        let on = if want { t } else { 1.0 - t };
        let off_c = if resp.hovered() { th.text } else { th.muted };
        text(ui, th, hr.center(), Align2::CENTER_CENTER, name, Role::SansB, 13.44, mix(on, th.on_accent, off_c));
    }
    if th.palette == Palette::Cyberpunk {
        cyber_stroke(&p, rect, 4.0, 1.0, border);
    } else {
        stroke_in(&p, rect, r, 1.0, border);
    }
    ui.allocate_rect(rect, Sense::hover());
    rect
}

fn results_body(ui: &mut Ui, th: &Theme, iw: f32, inputs: &mut Inputs) {
    let o = ui.cursor().min;
    let mut y = o.y;
    // HbA2 / HbF: the % box sits on the name's line, between it and the chips
    let name_w = ROWS.iter().filter(|r| r.is_numeric()).map(|r| text_width(ui, th, r.label(), Role::SansB, 13.0)).fold(0.0_f32, f32::max).ceil() + 8.0;
    let label_w = LABEL_W.max(name_w + NUM_W + 12.0);
    let chips_x0 = o.x + label_w;
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
            let rect = Rect::from_min_size(pos2(o.x + name_w, y), vec2(NUM_W, CHIP_H));
            let value = if row == Row::Hba2 { &mut inputs.hba2 } else { &mut inputs.hbf };
            if text_field(ui, th, rect, id, value, "type %", invalid) {
                inputs.number_edited(row);
            }
            left_bottom = left_bottom.max(rect.bottom());
            if invalid {
                let r = text(ui, th, pos2(rect.left(), rect.bottom() + 3.0), Align2::LEFT_TOP, "0–100, use a dot", Role::Sans, 11.0, th.bad);
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
    // copy icon beside the code label, as in the source sheet
    if copy_button(ui, th, pos2(hr.right() + 6.0, hr.center().y), copied == Some(e.code), Id::new(("hbel-copy", e.code))).clicked() {
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
/// Comment column width the default divider position aims for.
const COMMENT_COMFY: f32 = 210.0;
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

/// Rows that fit the results first (in sheet order), then every other row (also in sheet order).
pub(crate) fn fits_first<'a>(rows: Vec<&'a Entry>, highlight: &[u8]) -> Vec<&'a Entry> {
    let (mut fit, rest): (Vec<&Entry>, Vec<&Entry>) = rows.into_iter().partition(|e| highlight.contains(&e.code));
    fit.extend(rest);
    fit
}

/// Height of the "Other comments" band between the fitting rows and the rest.
const SPLIT_H: f32 = 26.0;

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

/// The source sheet as a table. Comment and the sheet columns fit the visible width;
/// only "Notes & links" sits past the right edge (scroll sideways to read it).
/// Rows whose code is in `highlight` are highlighted; the first `n_hl` rows are those,
/// and an "Other comments" band separates them from the rest. Returns the bottom y
/// and the first highlighted row's rect.
#[allow(clippy::too_many_arguments)]
fn source_table(ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, rows: &[&Entry], highlight: &[u8], n_hl: usize, copied: Option<u8>, copy_req: &mut Option<u8>, view_req: &mut Option<u8>) -> (f32, Option<Rect>) {
    let split = n_hl > 0 && n_hl < rows.len();
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
            let mut h = CODE_LINE + paragraph_height(ui, th, wrap(0), &e.comment, Role::Serif, COMMENT_FONT, 1.35);
            for (k, (col, _)) in TABLE_COLS.iter().enumerate() {
                h = h.max(paragraph_height(ui, th, wrap(k + 1), &cell_text(e, col), Role::Sans, CELL_FONT, 1.35));
            }
            let notes = notes_text(e);
            let nh = if notes.is_empty() { 0.0 } else { paragraph_height(ui, th, wrap(last), &notes, Role::Sans, 11.5, 1.35) + 4.0 };
            h.max(nh + e.links.len() as f32 * link_h) + CELL_PAD * 2.0
        })
        .collect();
    let head_h = heads.iter().enumerate().map(|(i, h)| paragraph_height(ui, th, wrap(i), h, Role::SansB, 11.5, 1.25)).fold(0.0_f32, f32::max) + 12.0;
    let table_h = head_h + row_h.iter().sum::<f32>() + if split { SPLIT_H } else { 0.0 };
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
            let mut split_band: Option<Rect> = None;
            for (ri, e) in rows.iter().enumerate() {
                if split && ri == n_hl {
                    let band = Rect::from_min_size(pos2(o.x, y), vec2(tw, SPLIT_H));
                    p.rect_filled(band, 0.0, th.surface2);
                    p.line_segment([pos2(o.x, y), pos2(o.x + tw, y)], Stroke::new(2.0_f32, th.line_strong));
                    split_band = Some(band);
                    y += SPLIT_H;
                }
                let rh = row_h[ri];
                let row_rect = Rect::from_min_size(pos2(o.x, y), vec2(tw, rh));
                let hl = highlight.contains(&e.code);
                // Dark mode: only a faint warm lift (an accent wash there drags the
                // light text down to mid-grey contrast); the bar marks the row and
                // its text is brightened instead.
                let (fg, fg_muted) = if hl && th.dark { (mix(0.55, egui::Color32::WHITE, th.text), th.text) } else { (th.text, th.muted) };
                if hl {
                    let fill_c = if th.dark { mix(0.15, hl_bar, th.surface) } else { rgba(hl_bar, 0.14) };
                    p.rect_filled(row_rect, 0.0, fill_c);
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
                let resp = ui.interact(cb, Id::new(("hbel-src-code", e.code)), Sense::click()).on_hover_text("Show as the suggested comment");
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    *view_req = Some(e.code);
                }
                let mut bx = cb.right() + 8.0;
                if e.source.is_some() {
                    bx = chip(ui, th, pos2(cb.right() + 6.0, top + 1.0), Align2::LEFT_TOP, "Added", Role::SansB, 10.5, th.ok, th.ok_bg, None, vec2(6.0, 1.0), pill()).right() + 8.0;
                }
                // copy button next to the code label
                if copy_button(ui, th, pos2(bx - 4.0, cb.center().y), copied == Some(e.code), Id::new(("hbel-src-copy", e.code))).clicked() {
                    *copy_req = Some(e.code);
                }
                // comment (selectable)
                selectable_paragraph(ui, th, pos2(x0, top + CODE_LINE), wrap(0), &e.comment, Role::Serif, COMMENT_FONT, fg, 1.35);
                let mut x = o.x + widths[0];
                for (k, (col, _)) in TABLE_COLS.iter().enumerate() {
                    let t = cell_text(e, col);
                    if t.is_empty() {
                        text(ui, th, pos2(x + CELL_PAD, top), Align2::LEFT_TOP, "—", Role::Sans, CELL_FONT, fg_muted);
                    } else {
                        selectable_paragraph(ui, th, pos2(x + CELL_PAD, top), wrap(k + 1), &t, Role::Sans, CELL_FONT, fg, 1.35);
                    }
                    x += widths[k + 1];
                }
                let notes = notes_text(e);
                let mut ny = top;
                if !notes.is_empty() {
                    ny = selectable_paragraph(ui, th, pos2(x + CELL_PAD, top), wrap(last), &notes, Role::Sans, 11.5, fg, 1.35).bottom() + 4.0;
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
            // the "Other comments" band spans every column (painted over the rules)
            if let Some(band) = split_band {
                p.rect_filled(Rect::from_min_max(band.min + vec2(0.0, 1.0), band.max), 0.0, th.surface2);
                let s = format!("OTHER COMMENTS · {} ", rows.len() - n_hl);
                text_ls(ui, th, pos2(band.left() + CELL_PAD + 2.0, band.center().y + 0.5), Align2::LEFT_CENTER, &s, Role::SansB, 10.88, th.muted, 0.03);
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
        // built-in comments carry no sample IDs (they come from the shared folder)
        assert!(lib.entries.iter().all(|x| x.examples.is_empty()));
        assert!(notes_text(e(21)).starts_with("Reference: Patel"));
        let mut lib2 = lib.clone();
        let (ex, bad) = hbel_custom::parse_examples("# note\n4: 26XX0001, 26XX0002\n");
        assert!(bad.is_empty());
        assert!(hbel_custom::attach_examples(&mut lib2, &ex).is_empty());
        assert_eq!(notes_text(lib2.entries.iter().find(|x| x.code == 4).unwrap()), "Examples: 26XX0001, 26XX0002");
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
    fn band_limits_read_naturally() {
        let t = |row: Row| row.options().iter().map(|o| band_text(&o.range.unwrap())).collect::<Vec<_>>();
        assert_eq!(t(Row::Hba2), vec!["< 2.2", "2.2 – < 3.5", "3.5 – < 4", "4 – 10", "> 10"]);
        assert_eq!(t(Row::Hbf), vec!["≤ 1.3", "> 1.3 – < 5", "5 – 50", "> 50"]);
    }

    #[test]
    fn fitting_rows_move_to_the_top() {
        let lib = library();
        let all: Vec<&Entry> = lib.entries.iter().collect();
        let codes: Vec<u8> = fits_first(all.clone(), &[21, 17]).iter().map(|e| e.code).collect();
        assert_eq!(&codes[..2], &[17, 21], "fits keep sheet order");
        assert_eq!(codes.len(), all.len(), "the other rows stay visible");
        assert_eq!(codes[2], all[0].code);
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
