//! The four tool panels: FBC / PT-APTT checkers, Reticulocyte and Dilution.
use crate::app::*;
use crate::logic::*;
use crate::theme::{hex, rgba, Palette, Role, Theme};
use crate::ui::*;
use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, CornerRadius, Id, Layout, Pos2, Rect, Sense, Stroke, Ui, UiBuilder};

fn toolbar(ui: &mut Ui, origin: Pos2, width: f32, f: impl FnOnce(&mut Ui)) -> f32 {
    let r = Rect::from_min_size(origin, vec2(width, 36.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(r).layout(Layout::right_to_left(Align::Min)));
    child.spacing_mut().item_spacing.x = 10.0;
    f(&mut child);
    origin.y + 36.0 + 16.0
}

fn status_colors(th: &Theme, s: Status) -> Color32 {
    match s {
        Status::Ok => th.ok,
        Status::Bad => th.bad,
        Status::Pending => th.muted,
    }
}

// ---------------------------------------------------------------------------
// Duplicate-run checkers (FBC, PT / APTT)
// ---------------------------------------------------------------------------

fn overall_status(ck: &Checker) -> Status {
    let mut all = vec![];
    for (p, vals) in ck.params.iter().zip(&ck.vals) {
        let pairs: &[(usize, usize)] = if ck.third { &[(0, 1), (0, 2), (1, 2)] } else { &[(0, 1)] };
        for (a, b) in pairs {
            all.push(pair_result(parse_positive(&vals[*a]), parse_positive(&vals[*b]), p.limit).status);
        }
    }
    if all.contains(&Status::Bad) {
        Status::Bad
    } else if !all.is_empty() && all.iter().all(|s| *s == Status::Ok) {
        Status::Ok
    } else {
        Status::Pending
    }
}

/// A Genshin settings row (Settings / Key Bindings): a soft bar holding the
/// label, and a brighter value cell on the right whose left edge feathers in
/// behind a faint sparkle.
fn genshin_row_overlay(p: &egui::Painter, th: &Theme, rect: Rect, row_w: f32) {
    let r = 5.0;
    let (base, cell, sparkle) = if th.dark {
        (hex(0x384053), hex(0x4a5368), rgba(th.gold, 0.55))
    } else {
        (crate::theme::mix(0.07, hex(0x4a5366), th.surface), hex(0xf8f4ec), rgba(th.gold, 0.60))
    };
    fill(p, rect, r, base);
    let sw = (row_w * 0.34).clamp(150.0, 210.0);
    let feather = 46.0;
    let seg = Rect::from_min_max(pos2(rect.right() - sw, rect.top()), rect.max);
    p.with_clip_rect(seg).rect_filled(rect, cr(r), cell);
    let fr = Rect::from_min_max(pos2(seg.left() - feather, rect.top()), pos2(seg.left(), rect.bottom()));
    h_gradient(p, fr, Color32::TRANSPARENT, cell);
    star(p, pos2(seg.left() - 8.0, rect.center().y), 13.0, sparkle);
}

fn result_row(ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, label: &str, res: &PairResult) {
    let h = 41.0;
    let p = ui.painter().clone();
    match th.palette {
        Palette::Default => {
            p.line_segment([pos2(pos.x, pos.y + 0.5), pos2(pos.x + w, pos.y + 0.5)], Stroke::new(1.0, th.line));
        }
        Palette::Pokemon => {
            let r = Rect::from_min_size(pos2(pos.x - 14.0, pos.y + 3.0), vec2(w + 28.0, h - 6.0));
            fill(&p, r, 12.0, th.field);
            stroke_in(&p, r, 12.0, 1.5, rgba(th.ring, 0.35));
        }
        Palette::Genshin => genshin_row_overlay(&p, th, Rect::from_min_size(pos2(pos.x - 14.0, pos.y + 3.0), vec2(w + 28.0, h - 6.0)), w),
        Palette::Cyberpunk => {
            let r = Rect::from_min_size(pos2(pos.x - 14.0, pos.y + 3.0), vec2(w + 28.0, h - 6.0));
            cyber_panel(&p, r, 4.0, rgba(th.surface2, if th.dark { 0.72 } else { 0.55 }), 1.0, rgba(th.ring, 0.20));
            let sc = status_colors(th, res.status);
            p.rect_filled(Rect::from_min_size(r.min, vec2(2.0, r.height())), 0.0, rgba(sc, 0.85));
            p.line_segment([pos2(r.right() - 22.0, r.bottom() - 1.0), pos2(r.right() - 2.0, r.bottom() - 1.0)], Stroke::new(1.0_f32, rgba(th.ring, 0.42)));
        }
    }
    let mid = pos.y + 1.0 + (h - 1.0) / 2.0;
    text(ui, th, pos2(pos.x + 1.0, mid), Align2::LEFT_CENTER, label, Role::Sans, 12.8, th.muted);
    let color = status_colors(th, res.status);
    let bw = badge(ui, th, pos2(pos.x + w - 1.0, mid), res.badge, color);
    text(ui, th, pos2(pos.x + w - 1.0 - bw - 10.0, mid), Align2::RIGHT_CENTER, &res.percent, Role::MonoB, 14.4, th.text);
}

fn checker_body(ui: &mut Ui, th: &Theme, iw: f32, key: &str, p: &ParamDef, vals: &mut [String; 3], third: bool) {
    let o = ui.cursor().min;
    let n = if third { 3 } else { 2 };
    let gap = 12.0;
    let colw = (iw - gap * (n as f32 - 1.0)) / n as f32;
    for k in 0..n {
        let pos = o + vec2(k as f32 * (colw + gap), 0.0);
        let invalid = parse_positive(&vals[k]) == Num::Invalid;
        labeled_field(ui, th, pos, colw, &format!("Run {}", k + 1), Id::new((key, p.id, k)), &mut vals[k], p.ph[k], invalid);
    }
    let mut y = o.y + 60.0 + 18.0;
    let pairs: &[(usize, usize)] = if third { &[(0, 1), (0, 2), (1, 2)] } else { &[(0, 1)] };
    for (a, b) in pairs {
        let res = pair_result(parse_positive(&vals[*a]), parse_positive(&vals[*b]), p.limit);
        result_row(ui, th, pos2(o.x, y), iw, &format!("Run {} vs Run {}", a + 1, b + 1), &res);
        y += 41.0;
    }
    ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y)), Sense::hover());
}

impl App {
    pub fn checker_panel(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32, is_fbc: bool) -> f32 {
        let screen_w = ui.ctx().screen_rect().width();
        let ck = if is_fbc { &mut self.fbc } else { &mut self.pt };
        let key = ck.key;
        let params = ck.params;
        let mut y = origin.y;

        let (mut clear, mut toggle) = (false, false);
        let third_now = ck.third;
        y = toolbar(ui, pos2(origin.x, y), w, |ui| {
            if button(ui, th, "Clear", Btn::Secondary, true).clicked() {
                clear = true;
            }
            let label = if third_now { "− Remove third run" } else { "+ Add third run" };
            if button(ui, th, label, Btn::Primary, true).clicked() {
                toggle = true;
            }
        });
        if toggle {
            ck.third = !ck.third;
            if !ck.third {
                for v in ck.vals.iter_mut() {
                    v[2].clear();
                }
            }
        }
        if clear {
            for v in ck.vals.iter_mut() {
                for s in v.iter_mut() {
                    s.clear();
                }
            }
            ui.memory_mut(|m| m.request_focus(Id::new((key, params[0].id, 0usize))));
        }

        let cols = if screen_w > 900.0 { 2 } else { 1 };
        let cw = (w - 16.0 * (cols as f32 - 1.0)) / cols as f32;
        let third = ck.third;
        let n = params.len();
        let mut idx = 0;
        while idx < n {
            let end = (idx + cols).min(n);
            let mut row_bottom = y;
            for (c, i) in (idx..end).enumerate() {
                let pd = &params[i];
                let x = origin.x + c as f32 * (cw + 16.0);
                let (bar, head_bg, title_c) = match pd.color {
                    Some(k) => (Some(th.param[k].0), Some(th.param[k].1), Some(th.param[k].0)),
                    None => (None, None, None),
                };
                let opts = CardOpts {
                    title: pd.name,
                    unit: pd.unit,
                    bar,
                    head_bg,
                    title_color: title_c,
                    chip: Some((format!("≤ {}%", fmt_js(pd.limit)), is_fbc)),
                    min_height: 0.0,
                };
                let vals = &mut ck.vals[i];
                let rect = card(ui, th, pos2(x, y), cw, &opts, |ui, iw| checker_body(ui, th, iw, key, pd, vals, third));
                row_bottom = row_bottom.max(rect.bottom());
            }
            y = row_bottom + 16.0;
            idx = end;
        }

        let st = overall_status(ck);
        let msg = match st {
            Status::Bad => "Review flagged result",
            Status::Ok => "All acceptable",
            Status::Pending => "Awaiting results",
        };
        let r = summary_bar(ui, th, pos2(origin.x, y + 4.0), w, msg, st);
        r.bottom()
    }
}

// ---------------------------------------------------------------------------
// Reticulocyte
// ---------------------------------------------------------------------------

pub struct ReticOut {
    pub corrected_text: String,
    pub corrected_rounded: bool,
    pub corrected_note: Option<String>,
    pub absolute_text: String,
    pub absolute_note: Option<String>,
    pub rbc_note: Option<String>,
}

pub enum EqPart {
    Val(String),
    Op(String),
    Res(String),
}

#[derive(PartialEq, Clone, Copy)]
pub enum StepState {
    Pending,
    Done,
    Nil,
}

pub struct Step {
    pub num: usize,
    pub title: &'static str,
    pub state: StepState,
    pub formula: String,
    pub eq: Vec<EqPart>,
    pub tags: Vec<(String, bool)>,
    pub hint: Option<String>,
    pub result: Option<String>,
}

impl App {
    fn retic_nums(&self) -> (Num, Num, Num, Corrected) {
        let retic = parse_number(&self.r_retic);
        let hct = parse_number(&self.r_hct);
        let rbc = parse_number(&self.r_rbc);
        (retic, hct, rbc, computed_corrected(retic, hct, self.gender))
    }

    pub fn compute_retic(&self) -> ReticOut {
        let (retic, _hct, rbc, corrected) = self.retic_nums();
        let reticv = retic.ok();
        let rbcv = rbc.ok();
        let mut out = ReticOut {
            corrected_text: "—".into(),
            corrected_rounded: false,
            corrected_note: None,
            absolute_text: "—".into(),
            absolute_note: None,
            rbc_note: None,
        };
        let rbc_r = rbcv.map(round1);
        if let (Some(r), Some(rr)) = (rbcv, rbc_r) {
            if is_rounded(r, rr) {
                out.rbc_note = Some(format!("1 dp: {} → {}", fmt_raw(r), to_fixed(rr, 1)));
            }
        }
        let mut corrected_r = None;
        let mut corrected_was = false;
        match corrected {
            Corrected::NotComputable => {}
            Corrected::Nil => out.corrected_text = "NIL".into(),
            Corrected::Value(c) => {
                let cr = round2(c);
                corrected_r = Some(cr);
                corrected_was = is_rounded(c, cr);
                out.corrected_text = to_fixed(cr, 2);
                out.corrected_rounded = corrected_was;
                if corrected_was {
                    out.corrected_note = Some(format!("Rounded to 2 dp: {} → {}.", fmt_raw(c), to_fixed(cr, 2)));
                }
            }
        }
        if let (Some(retic), Some(rbc_r)) = (reticv, rbc_r) {
            match corrected {
                Corrected::Value(_) => {
                    let cr = corrected_r.unwrap();
                    out.absolute_text = to_fixed(cr * rbc_r * 10.0, 1);
                    if corrected_was {
                        out.absolute_note = Some(format!("Using corrected retic % at 2 dp ({})", to_fixed(cr, 2)));
                    }
                }
                _ => {
                    let rr = round2(retic);
                    out.absolute_text = to_fixed(rr * rbc_r * 10.0, 1);
                    if is_rounded(retic, rr) {
                        out.absolute_note = Some(format!("Retic % at 2 dp: {} → {}.", fmt_raw(retic), to_fixed(rr, 2)));
                    }
                }
            }
        }
        out
    }

    pub fn build_steps(&self) -> Vec<Step> {
        let (retic, hct, rbc, corrected) = self.retic_nums();
        let mut steps = vec![];
        let op = |s: &str| EqPart::Op(s.to_string());
        let val = |s: String| EqPart::Val(s);

        // 1. counting helper
        let vals: Vec<f64> = self.attempts.iter().filter_map(|s| parse_number(s).ok()).collect();
        let f1 = "Average of attempts ÷ 10".to_string();
        if vals.is_empty() {
            steps.push(Step { num: 1, title: "Counting helper: average % retic", state: StepState::Pending, formula: f1, eq: vec![], tags: vec![], hint: Some("Enter counts in the counting helper to see the average.".into()), result: None });
        } else {
            let avg = vals.iter().sum::<f64>() / vals.len() as f64 / 10.0;
            let avg_r = round2(avg);
            let mut eq = vec![op("(")];
            for (i, v) in vals.iter().enumerate() {
                if i > 0 {
                    eq.push(op("+"));
                }
                eq.push(val(fmt_js(*v)));
            }
            eq.extend([op(")"), op("÷"), val(vals.len().to_string()), op("attempts ÷"), val("10".into()), op("="), EqPart::Res(fmt_raw(avg))]);
            let tags = if is_rounded(avg, avg_r) {
                vec![(format!("Rounded to 2 dp when sent with “Use as retic %”: {} → {}", fmt_raw(avg), to_fixed(avg_r, 2)), false)]
            } else {
                vec![]
            };
            steps.push(Step { num: 1, title: "Counting helper: average % retic", state: StepState::Done, formula: f1, eq, tags, hint: None, result: Some(format!("{} %", to_fixed(avg_r, 2))) });
        }

        // 2. correction check
        let f2 = "Correct only if round(HCT) < 41 (male) or 36 (female)".to_string();
        match (hct, self.gender) {
            (Num::Val(h), Some(g)) => {
                let thr = if g == Gender::Male { 41.0 } else { 36.0 };
                let hr = js_round(h);
                let applies = hr < thr;
                let gl = if g == Gender::Male { "male" } else { "female" };
                let eq = vec![op("round("), val(fmt_raw(h)), op(")"), op("="), val(fmt_js(hr)), op(if applies { "<" } else { "≥" }), val(fmt_js(thr)), op(&format!("({} threshold)", gl))];
                steps.push(Step {
                    num: 2,
                    title: "Correction check",
                    state: if applies { StepState::Done } else { StepState::Nil },
                    formula: f2,
                    eq,
                    tags: vec![],
                    hint: None,
                    result: Some(if applies { "Correction applies".into() } else { "No correction (NIL)".into() }),
                });
            }
            _ => steps.push(Step { num: 2, title: "Correction check", state: StepState::Pending, formula: f2, eq: vec![], tags: vec![], hint: Some("Enter HCT and select gender to check whether correction applies.".into()), result: None }),
        }

        // 3. corrected retic
        let f3 = "Observed retic % × (HCT ÷ 45)".to_string();
        match corrected {
            Corrected::NotComputable => steps.push(Step { num: 3, title: "Corrected retic %", state: StepState::Pending, formula: f3, eq: vec![], tags: vec![], hint: Some("Enter retic %, HCT and gender to calculate the corrected retic %.".into()), result: None }),
            Corrected::Nil => steps.push(Step { num: 3, title: "Corrected retic %", state: StepState::Nil, formula: f3, eq: vec![], tags: vec![], hint: Some("HCT is not below the threshold, so no correction is applied. The observed retic % is used in the next step.".into()), result: Some("NIL".into()) }),
            Corrected::Value(c) => {
                let cr = round2(c);
                let (r, h) = (retic.ok().unwrap(), hct.ok().unwrap());
                let eq = vec![val(fmt_raw(r)), op("× ("), val(fmt_raw(h)), op("÷"), val("45".into()), op(")"), op("="), EqPart::Res(fmt_raw(c))];
                let tags = if is_rounded(c, cr) { vec![(format!("Rounded to 2 dp: {} → {}. The 2 dp value is used in step 4.", fmt_raw(c), to_fixed(cr, 2)), false)] } else { vec![] };
                steps.push(Step { num: 3, title: "Corrected retic %", state: StepState::Done, formula: f3, eq, tags, hint: None, result: Some(format!("{} %", to_fixed(cr, 2))) });
            }
        }

        // 4. absolute retic
        let f4 = "% retic × RBC (×10¹²/L) × 10".to_string();
        match (retic.ok(), rbc.ok()) {
            (Some(r), Some(b)) => {
                let mut tags: Vec<(String, bool)> = vec![];
                let (inp, source);
                if let Corrected::Value(c) = corrected {
                    inp = round2(c);
                    source = "corrected retic % (2 dp)";
                } else {
                    inp = round2(r);
                    source = "observed retic % (2 dp)";
                    if is_rounded(r, inp) {
                        tags.push((format!("Observed retic % rounded to 2 dp: {} → {}", fmt_raw(r), to_fixed(inp, 2)), false));
                    }
                    if corrected == Corrected::NotComputable {
                        tags.push(("HCT / gender not entered, so no correction is applied".into(), true));
                    }
                }
                let rbc_r = round1(b);
                if is_rounded(b, rbc_r) {
                    tags.push((format!("RBC rounded to 1 dp: {} → {}", fmt_raw(b), to_fixed(rbc_r, 1)), false));
                }
                let raw = inp * rbc_r * 10.0;
                tags.push((format!("Using {}", source), true));
                tags.push(("Result rounded to 1 dp".into(), true));
                let eq = vec![val(to_fixed(inp, 2)), op("×"), val(to_fixed(rbc_r, 1)), op("×"), val("10".into()), op("="), EqPart::Res(fmt_raw(raw))];
                steps.push(Step { num: 4, title: "Absolute retic", state: StepState::Done, formula: f4, eq, tags, hint: None, result: Some(format!("{} ×10⁹/L", to_fixed(raw, 1))) });
            }
            _ => steps.push(Step { num: 4, title: "Absolute retic", state: StepState::Pending, formula: f4, eq: vec![], tags: vec![], hint: Some("Enter retic % and RBC count to calculate the absolute retic.".into()), result: None }),
        }
        steps
    }

    fn reset_retic(&mut self) {
        self.r_retic.clear();
        self.r_hct.clear();
        self.r_rbc.clear();
        self.gender = None;
        self.attempts = vec![String::new(); MIN_ATTEMPTS];
        self.retic_note = None;
    }

    pub fn retic_panel(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        let screen_w = ui.ctx().screen_rect().width();
        let mut y = origin.y;
        let (mut reset, mut toggle) = (false, false);
        let calc_open = self.calc_open;
        y = toolbar(ui, pos2(origin.x, y), w, |ui| {
            if button(ui, th, "Reset", Btn::Secondary, true).clicked() {
                reset = true;
            }
            let (l, k) = if calc_open { ("Hide calculations", Btn::Primary) } else { ("Show calculations", Btn::Soft) };
            if button(ui, th, l, k, true).clicked() {
                toggle = true;
            }
        });
        if reset {
            self.reset_retic();
            ui.memory_mut(|m| m.request_focus(Id::new("r-retic")));
        }
        if toggle {
            self.calc_open = !self.calc_open;
        }

        let out = self.compute_retic();
        let two_col = screen_w > 900.0;
        let gap = 16.0;
        let h_id = Id::new("retic-grid-h");
        let min_h: f32 = if two_col { ui.ctx().data(|d| d.get_temp(h_id)).unwrap_or(0.0) } else { 0.0 };

        let (helper_pos, helper_w, main_pos, main_w);
        if two_col {
            let u = (w - gap) / 3.0;
            helper_pos = pos2(origin.x, y);
            helper_w = u;
            main_pos = pos2(origin.x + u + gap, y);
            main_w = 2.0 * u;
        } else {
            main_pos = pos2(origin.x, y);
            main_w = w;
            helper_w = w;
            helper_pos = pos2(origin.x, y); // fixed below
        }

        let main_opts = CardOpts { title: "Reticulocyte", unit: "", bar: None, head_bg: None, title_color: None, chip: None, min_height: min_h };
        let main_rect = card(ui, th, main_pos, main_w, &main_opts, |ui, iw| self.retic_main_body(ui, th, iw, &out));
        let helper_pos = if two_col { helper_pos } else { pos2(origin.x, main_rect.bottom() + gap) };
        let helper_opts = CardOpts { title: "Counting helper", unit: "retics / 1000 RBCs", bar: None, head_bg: None, title_color: None, chip: None, min_height: min_h };
        let helper_rect = card(ui, th, helper_pos, helper_w, &helper_opts, |ui, iw| self.helper_body(ui, th, iw));
        let row_h = main_rect.height().max(helper_rect.height());
        if two_col {
            let prev: f32 = ui.ctx().data(|d| d.get_temp(h_id)).unwrap_or(0.0);
            if (prev - row_h).abs() > 0.5 {
                ui.ctx().data_mut(|d| d.insert_temp(h_id, row_h));
                ui.ctx().request_repaint();
            }
        }
        y = main_rect.bottom().max(helper_rect.bottom()) + 16.0;

        if self.calc_open {
            y = self.calc_card(ui, th, pos2(origin.x, y), w, screen_w) + 16.0;
        }
        let bottom = crate::reference::reference_card(ui, th, pos2(origin.x, y), w);
        bottom
    }

    fn retic_main_body(&mut self, ui: &mut Ui, th: &Theme, iw: f32, out: &ReticOut) {
        let o = ui.cursor().min;
        let gap = 10.0;
        let colw = (iw - 3.0 * gap) / 4.0;
        let xc = |k: usize| o.x + k as f32 * (colw + gap);

        let inv = parse_number(&self.r_retic) == Num::Invalid;
        if labeled_field(ui, th, pos2(xc(0), o.y), colw, "retic (%)", Id::new("r-retic"), &mut self.r_retic, "e.g. 0.9", inv) {
            self.retic_note = None;
        }
        let inv = parse_number(&self.r_hct) == Num::Invalid;
        labeled_field(ui, th, pos2(xc(1), o.y), colw, "hct (%)", Id::new("r-hct"), &mut self.r_hct, "e.g. 38", inv);
        label(ui, th, pos2(xc(2), o.y), "gender");
        let g_rect = Rect::from_min_size(pos2(xc(2), o.y + LABEL_H + LABEL_GAP), vec2(colw, FIELD_H));
        self.gender_toggle(ui, th, g_rect);
        let inv = parse_number(&self.r_rbc) == Num::Invalid;
        labeled_field(ui, th, pos2(xc(3), o.y), colw, "rbc (×10¹²)", Id::new("r-rbc"), &mut self.r_rbc, "e.g. 4.1", inv);

        let mut row_h: f32 = 60.0;
        if let Some(n) = &self.retic_note {
            let r = note(ui, th, pos2(xc(0), o.y + 64.0), Align2::LEFT_TOP, n, colw);
            row_h = row_h.max(r.bottom() - o.y);
        }
        if let Some(n) = &out.rbc_note {
            let r = note(ui, th, pos2(xc(3), o.y + 64.0), Align2::LEFT_TOP, n, colw);
            row_h = row_h.max(r.bottom() - o.y);
        }

        let mut y = o.y + row_h + 18.0;
        y += hero_row(ui, th, pos2(o.x, y), iw, "corrected retic (%)", &out.corrected_text, out.corrected_rounded, out.corrected_note.as_deref(), false);
        y += hero_row(ui, th, pos2(o.x, y), iw, "absolute retic (×10⁹)", &out.absolute_text, false, out.absolute_note.as_deref(), true);

        // disclaimer
        y += 16.0;
        let msg = "Verify these calculated results against your laboratory's SOP before use. You are solely responsible for deriving and confirming your own results.";
        let tw = iw - 28.0 - 3.0;
        let th_h = paragraph_height(ui, th, tw, msg, Role::Serif, 12.8, 1.5);
        let r = Rect::from_min_size(pos2(o.x, y), vec2(iw, th_h + 24.0));
        let rad = if th.palette == Palette::Pokemon { 14 } else { th.radius.round() as u8 };
        ui.painter().rect_filled(r, CornerRadius { nw: 0, sw: 0, ne: rad, se: rad }, th.warn_bg);
        ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 0.0, th.warn);
        paragraph(ui, th, pos2(r.left() + 3.0 + 14.0, r.top() + 12.0), tw, msg, Role::Serif, 12.8, th.warn, 1.5);
        y = r.bottom();
        ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y)), Sense::hover());
    }

    fn gender_toggle(&mut self, ui: &mut Ui, th: &Theme, rect: Rect) {
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
            Palette::Cyberpunk => rgba(th.ring, 0.55),
            _ => th.line,
        };
        if th.palette == Palette::Cyberpunk {
            cyber_panel(&p, rect, 4.0, th.surface2, 1.0, border);
        } else {
            fill(&p, rect, r, th.surface2);
        }
        let halves = [
            (Gender::Male, "Male", Rect::from_min_max(rect.min, pos2(rect.center().x, rect.bottom()))),
            (Gender::Female, "Female", Rect::from_min_max(pos2(rect.center().x, rect.top()), rect.max)),
        ];
        for (i, (g, name, half)) in halves.iter().enumerate() {
            let resp = ui.interact(*half, Id::new(("gender", i)), Sense::click());
            let active = self.gender == Some(*g);
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                self.gender = Some(*g);
            }
            if active {
                p.with_clip_rect(*half).rect_filled(rect, cr(r), th.accent);
            } else if resp.hovered() && th.palette == Palette::Genshin {
                p.with_clip_rect(*half).rect_filled(rect, cr(r), crate::theme::mix(0.16, th.gold, th.surface2));
            }
            let fg = if active { th.on_accent } else { th.text };
            text(ui, th, half.center(), Align2::CENTER_CENTER, name, Role::SansB, 14.08, fg);
        }
        p.line_segment([pos2(rect.center().x, rect.top() + 1.0), pos2(rect.center().x, rect.bottom() - 1.0)], Stroke::new(1.0, border));
        if th.palette == Palette::Cyberpunk {
            cyber_stroke(&p, rect, 4.0, 1.0, border);
        } else {
            stroke_in(&p, rect, r, 1.0, border);
        }
    }

    fn helper_body(&mut self, ui: &mut Ui, th: &Theme, iw: f32) {
        let o = ui.cursor().min;
        let n = self.attempts.len();
        for i in 0..n {
            let ry = o.y + i as f32 * 48.0;
            label(ui, th, pos2(o.x, ry + (FIELD_H - LABEL_H) / 2.0), &format!("Attempt {}", i + 1));
            let invalid = parse_number(&self.attempts[i]) == Num::Invalid;
            let hint = match i {
                0 => "e.g. 10",
                1 => "e.g. 8",
                2 => "e.g. 10",
                _ => "e.g. 9",
            };
            let r = Rect::from_min_size(pos2(o.x + 86.0, ry), vec2(iw - 86.0, FIELD_H));
            field(ui, th, r, Id::new(("r-attempt", i)), &mut self.attempts[i], hint, invalid);
        }
        let mut y = o.y + n as f32 * 48.0 - 8.0 + 12.0;

        let (mut add, mut remove) = (false, false);
        let r = Rect::from_min_size(pos2(o.x, y), vec2(iw, 32.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(r).layout(Layout::left_to_right(Align::Center)));
        child.spacing_mut().item_spacing.x = 8.0;
        if button(&mut child, th, "+ Add attempt", Btn::Mini, n < MAX_ATTEMPTS).clicked() {
            add = true;
        }
        if button(&mut child, th, "− Remove last", Btn::MiniMuted, n > MIN_ATTEMPTS).clicked() {
            remove = true;
        }
        let cnt = format!("{} attempts", n);
        let cw = text_width(ui, th, &cnt, Role::Sans, 12.16);
        let used = child.min_rect().width();
        if used + 8.0 + cw <= iw {
            text(ui, th, pos2(o.x + iw, y + 16.0), Align2::RIGHT_CENTER, &cnt, Role::Sans, 12.16, th.muted);
        } else {
            y += 32.0 + 8.0;
            text(ui, th, pos2(o.x + iw, y + 8.0), Align2::RIGHT_CENTER, &cnt, Role::Sans, 12.16, th.muted);
            y -= 16.0;
        }
        y += 32.0 + 18.0;

        // average row
        ui.painter().line_segment([pos2(o.x, y + 0.5), pos2(o.x + iw, y + 0.5)], Stroke::new(1.0, th.line));
        let avg = average_attempts(&self.attempts);
        let avg_text = avg.map(|a| format!("{} %", to_fixed(a, 2))).unwrap_or_else(|| "—".into());
        let need = text_width(ui, th, "avg % retic", Role::Sans, 13.12) + text_width(ui, th, &avg_text, Role::MonoB, 16.0) + text_width(ui, th, "Use as retic %", Role::SansB, 12.8) + 26.0 + 30.0;
        let stacked = need > iw;
        let mut row = Rect::from_min_size(pos2(o.x, y + 1.0 + 12.0), vec2(iw, 32.0));
        if stacked {
            text(ui, th, pos2(o.x + 1.0, row.top() + 8.0), Align2::LEFT_CENTER, "avg % retic", Role::Sans, 13.12, th.muted);
            row = row.translate(vec2(0.0, 22.0));
        } else {
            text(ui, th, pos2(o.x + 1.0, row.center().y), Align2::LEFT_CENTER, "avg % retic", Role::Sans, 13.12, th.muted);
        }
        let mut child = ui.new_child(UiBuilder::new().max_rect(row).layout(Layout::right_to_left(Align::Center)));
        let b = button(&mut child, th, "Use as retic %", Btn::Mini, true);
        text(ui, th, pos2(b.rect.left() - 10.0, row.center().y), Align2::RIGHT_CENTER, &avg_text, Role::MonoB, 16.0, th.accent);
        if b.clicked() {
            if let Some(a) = avg {
                let ar = round2(a);
                self.r_retic = to_fixed(ar, 2);
                self.retic_note = if is_rounded(a, ar) { Some(format!("2 dp: {} → {}", fmt_raw(a), to_fixed(ar, 2))) } else { None };
            }
        }
        y = row.bottom() + 12.0;
        ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y)), Sense::hover());

        if add && n < MAX_ATTEMPTS {
            self.attempts.push(String::new());
            ui.memory_mut(|m| m.request_focus(Id::new(("r-attempt", n))));
        }
        if remove && n > MIN_ATTEMPTS {
            self.attempts.pop();
        }
    }

    fn calc_card(&mut self, ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, screen_w: f32) -> f32 {
        let steps = self.build_steps();
        let opts = CardOpts { title: "Calculations", unit: "step by step · updates as you type", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0 };
        let rect = card(ui, th, pos, w, &opts, |ui, iw| {
            let o = ui.cursor().min;
            let big = screen_w >= 1100.0;
            let (cols, gap) = if big { (4usize, 16.0) } else { ((((iw + 8.0) / 218.0).floor() as usize).max(1), 8.0) };
            let cols = cols.min(steps.len().max(1));
            let cw = (iw - gap * (cols as f32 - 1.0)) / cols as f32;
            let mut y = o.y;
            let mut i = 0;
            while i < steps.len() {
                let end = (i + cols).min(steps.len());
                let mut row_h: f32 = 0.0;
                let mut stored: f32 = 0.0;
                for k in i..end {
                    let id = Id::new(("calc-step-h", k));
                    stored = stored.max(ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(0.0));
                }
                for (c, k) in (i..end).enumerate() {
                    let x = o.x + c as f32 * (cw + gap);
                    let h = draw_step(ui, th, pos2(x, y), cw, &steps[k], stored);
                    row_h = row_h.max(h);
                    if big && k + 1 < steps.len() {
                        text(ui, th, pos2(x + cw + gap / 2.0, y + 18.0), Align2::CENTER_CENTER, "→", Role::SansB, 12.8, th.line_strong);
                    }
                }
                for k in i..end {
                    let id = Id::new(("calc-step-h", k));
                    let prev: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(0.0);
                    if (prev - row_h).abs() > 0.5 {
                        ui.ctx().data_mut(|d| d.insert_temp(id, row_h));
                        ui.ctx().request_repaint();
                    }
                }
                y += row_h + gap;
                i = end;
            }
            ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y - gap)), Sense::hover());
        });
        rect.bottom()
    }
}

#[allow(clippy::too_many_arguments)]
fn hero_row(ui: &mut Ui, th: &Theme, pos: Pos2, iw: f32, label_s: &str, value: &str, rounded: bool, note_s: Option<&str>, accent: bool) -> f32 {
    let suffix = if rounded { " (2 dp)" } else { "" };
    let vw = text_width(ui, th, value, Role::MonoB, 20.8) + if rounded { 6.0 + text_width(ui, th, suffix.trim(), Role::SansB, 10.88) } else { 0.0 };
    let pill_w = (vw + 36.0 + 3.0).max(96.0);
    let side_w = ((iw - pill_w - 20.0) / 2.0).max(40.0);
    let note_sz = note_s.map(|n| note_size(ui, th, n, side_w));
    let content_h = 34.4_f32.max(note_sz.map(|s| s.y).unwrap_or(0.0));
    let h = 1.0 + 10.0 + content_h + 10.0;
    let p = ui.painter().clone();
    p.line_segment([pos2(pos.x, pos.y + 0.5), pos2(pos.x + iw, pos.y + 0.5)], Stroke::new(1.0, th.line));
    let cy = pos.y + 1.0 + 10.0 + content_h / 2.0;
    if th.palette == Palette::Cyberpunk {
        text_ls(ui, th, pos2(pos.x + 1.0, cy), Align2::LEFT_CENTER, &label_s.to_uppercase(), Role::MonoB, 11.8, th.muted, 0.06);
    } else {
        text(ui, th, pos2(pos.x + 1.0, cy), Align2::LEFT_CENTER, label_s, Role::Sans, 13.12, th.muted);
    }
    let pr = Rect::from_center_size(pos2(pos.x + iw / 2.0, cy), vec2(pill_w, 34.4));
    let (fg, bg, bd) = if th.palette == Palette::Cyberpunk {
        if accent {
            (th.on_accent, th.accent, th.accent)
        } else {
            (th.ring, rgba(th.ring, 0.10), rgba(th.ring, 0.75))
        }
    } else if accent {
        (th.on_accent, th.accent, th.accent)
    } else {
        (th.accent_dark, th.accent_soft, th.accent)
    };
    let radius = if th.palette == Palette::Cyberpunk { 0.0 } else { pill() };
    if th.palette == Palette::Cyberpunk {
        cyber_panel(&p, pr, 5.0, bg, 1.5, bd);
    } else {
        fill(&p, pr, radius, bg);
        stroke_in(&p, pr, radius, 1.5, bd);
    }
    if rounded {
        let total = vw;
        let x0 = pr.center().x - total / 2.0;
        let r1 = text(ui, th, pos2(x0, cy), Align2::LEFT_CENTER, value, Role::MonoB, 20.8, fg);
        text(ui, th, pos2(r1.right() + 6.0, cy + 2.0), Align2::LEFT_CENTER, suffix.trim(), Role::SansB, 10.88, th.warn);
    } else {
        text(ui, th, pr.center(), Align2::CENTER_CENTER, value, Role::MonoB, 20.8, fg);
    }
    if let Some(n) = note_s {
        note(ui, th, pos2(pos.x + iw, cy), Align2::RIGHT_CENTER, n, side_w);
    }
    h
}

/// Draws one calculation step; returns its height.
fn draw_step(ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, s: &Step, min_h: f32) -> f32 {
    let p = ui.painter().clone();
    let bg_i = p.add(egui::Shape::Noop);
    let iw = w - 2.0 - 20.0;
    let x = pos.x + 11.0;
    let mut y = pos.y + 1.0 + 8.0;
    let pending = s.state == StepState::Pending;

    // header row: number badge + title (+ result chip)
    let num_c = pos2(x + 11.0, y + 11.0);
    let (nb, nf) = match s.state {
        StepState::Pending => (th.line_strong, th.bg),
        StepState::Nil => (th.neutral, th.on_accent),
        StepState::Done => (th.accent, th.on_accent),
    };
    if th.palette == Palette::Pokemon {
        p.rect_filled(Rect::from_center_size(num_c, vec2(22.0, 22.0)), 7.0, nb);
    } else if th.palette == Palette::Cyberpunk {
        cyber_panel(&p, Rect::from_center_size(num_c, vec2(22.0, 22.0)), 5.0, nb, 1.0, rgba(th.ring, 0.45));
    } else {
        p.circle_filled(num_c, 11.0, nb);
    }
    text(ui, th, num_c, Align2::CENTER_CENTER, &s.num.to_string(), Role::SansB, 11.52, nf);
    let title_c = if pending { th.muted } else { th.text };
    let title_w = iw - 28.0;
    let step_title = if th.palette == Palette::Cyberpunk { s.title.to_uppercase() } else { s.title.to_string() };
    let tg = galley(ui, th, &step_title, Role::Display, 13.76, title_c, if th.palette == Palette::Cyberpunk { 0.04 } else { 0.0 }, title_w);
    let title_h = tg.size().y.max(22.0);
    let one_line = tg.rows.len() <= 1;
    let tw_used = tg.size().x;
    paint_galley(&p, th, Role::Display, pos2(x + 28.0, y + title_h / 2.0 - tg.size().y / 2.0), tg, title_c);
    let mut head_bottom = y + title_h;
    if let Some(res) = &s.result {
        let (fg, bgc, bd) = if s.state == StepState::Nil { (th.neutral, th.neutral_bg, th.line_strong) } else { (th.accent_dark, th.accent_soft, th.accent) };
        let radius = pill();
        let rw = text_width(ui, th, res, Role::MonoB, 14.08) + 22.0;
        let fits = one_line && x + 28.0 + tw_used + 6.0 + rw <= x + iw;
        let (rpos, anchor) = if fits { (pos2(x + iw, y + 11.0), Align2::RIGHT_CENTER) } else { (pos2(x, head_bottom + 4.0 + 12.0), Align2::LEFT_CENTER) };
        if !fits {
            head_bottom += 4.0 + 24.0;
        }
        chip(ui, th, rpos, anchor, res, Role::MonoB, 14.08, fg, bgc, Some((1.0, bd)), vec2(10.0, 1.0), radius);
    }
    y = head_bottom + 1.0;

    // formula
    let g = galley(ui, th, &s.formula, Role::SerifI, 11.52, th.muted, 0.0, iw);
    paint_galley(&p, th, Role::SerifI, pos2(x, y), g.clone(), th.muted);
    y += g.size().y;

    // equation chips
    if !s.eq.is_empty() {
        y += 5.0;
        let (mut cx, mut cy) = (x, y);
        let row_h = 21.0;
        for item in &s.eq {
            let (txt, kind) = match item {
                EqPart::Val(t) => (t.as_str(), 0),
                EqPart::Op(t) => (t.as_str(), 1),
                EqPart::Res(t) => (t.as_str(), 2),
            };
            let role = if kind == 1 { Role::Mono } else { Role::MonoB };
            let tw = text_width(ui, th, txt, role, 12.8);
            let iw_item = if kind == 1 { tw } else { tw + 16.0 + if kind == 0 { 2.0 } else { 0.0 } };
            if cx > x && cx + iw_item > x + iw {
                cx = x;
                cy += row_h + 3.0;
            }
            let mid = cy + row_h / 2.0;
            match kind {
                0 => {
                    chip(ui, th, pos2(cx, mid), Align2::LEFT_CENTER, txt, Role::MonoB, 12.8, th.text, th.surface2, Some((1.0, th.line)), vec2(8.0, 1.0), if th.palette == Palette::Pokemon { 7.0 } else { 6.0 });
                }
                1 => {
                    text(ui, th, pos2(cx, mid), Align2::LEFT_CENTER, txt, Role::Mono, 12.8, th.muted);
                }
                _ => {
                    chip(ui, th, pos2(cx, mid), Align2::LEFT_CENTER, txt, Role::MonoB, 12.8, th.accent_dark, th.accent_soft, None, vec2(8.0, 1.0), if th.palette == Palette::Pokemon { 7.0 } else { 6.0 });
                }
            }
            cx += iw_item + 5.0;
        }
        y = cy + row_h;
    }
    if let Some(h) = &s.hint {
        y += 4.0;
        let r = paragraph(ui, th, pos2(x, y), iw, h, Role::Serif, 12.16, th.muted, 1.22);
        y = r.bottom();
    }
    if !s.tags.is_empty() {
        y += 5.0;
        let (mut cx, mut cy) = (x, y);
        let mut row_h: f32 = 0.0;
        for (t, info) in &s.tags {
            let label = if *info { t.clone() } else { format!("≈\u{a0}{}", t) };
            let mut job = egui::text::LayoutJob::single_section(label, egui::text::TextFormat { font_id: th.font(Role::Sans, 12.8), color: if *info { th.muted } else { th.warn }, line_height: Some(17.3), ..Default::default() });
            job.wrap.max_width = iw - 18.0;
            let g = ui.fonts(|f| f.layout_job(job));
            let sz = g.size() + vec2(18.0, 6.0);
            if cx > x && cx + sz.x > x + iw {
                cx = x;
                cy += row_h + 4.0;
                row_h = 0.0;
            }
            let r = Rect::from_min_size(pos2(cx, cy), sz);
            let (fg, bgc, bd) = if *info { (th.muted, th.neutral_bg, th.line_strong) } else { (th.warn, th.warn_bg, th.warn) };
            let rad = if th.palette == Palette::Pokemon { 8.0 } else { 6.0 };
            fill(&p, r, rad, bgc);
            stroke_in(&p, r, rad, 1.0, bd);
            p.galley(r.min + vec2(9.0, 3.0), g, fg);
            row_h = row_h.max(sz.y);
            cx += sz.x + 4.0;
        }
        y = cy + row_h;
    }
    let h = (y + 8.0 + 1.0 - pos.y).max(min_h);
    let rect = Rect::from_min_size(pos, vec2(w, h));
    let rad = th.radius;
    if th.palette == Palette::Cyberpunk {
        p.set(bg_i, egui::Shape::convex_polygon(chamfer_points(rect, 5.0), if pending { th.surface2 } else { th.surface }, Stroke::NONE));
        cyber_stroke(&p, rect, 5.0, 1.0, if pending { th.line } else { rgba(th.ring, 0.24) });
        p.rect_filled(Rect::from_min_size(rect.min, vec2(2.0, rect.height().min(18.0))), 0.0, if pending { th.line_strong } else { th.accent });
    } else {
        p.set(bg_i, egui::Shape::rect_filled(rect, cr(rad), if pending { th.surface2 } else { th.surface }));
        stroke_in(&p, rect, rad, if th.palette == Palette::Pokemon { 1.5 } else { 1.0 }, if th.palette == Palette::Pokemon { th.line_strong } else { th.line });
    }
    h
}

// ---------------------------------------------------------------------------
// Dilution
// ---------------------------------------------------------------------------

impl App {
    pub fn dilution_panel(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        let mut reset = false;
        let y = toolbar(ui, origin, w, |ui| {
            if button(ui, th, "Reset", Btn::Secondary, true).clicked() {
                reset = true;
            }
        });
        if reset {
            for s in self.dil.iter_mut() {
                s.clear();
            }
            ui.memory_mut(|m| m.request_focus(Id::new(("dil", 0usize))));
        }
        let opts = CardOpts { title: "Dilution", unit: "1:2 diluted → calculated neat (×2)", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0 };
        let rect = card(ui, th, pos2(origin.x, y), w, &opts, |ui, iw| {
            let o = ui.cursor().min;
            let c1 = iw * 0.30;
            let c2 = iw * 0.35;
            let c3 = iw - c1 - c2;
            let hy = o.y;
            let th_txt = |ui: &Ui, x: f32, anchor: Align2, s: &str| {
                text_ls(ui, th, pos2(x, hy), anchor, &s.to_uppercase(), Role::SansB, 10.88, th.muted, 0.03);
            };
            th_txt(ui, o.x + 8.0, Align2::LEFT_TOP, "Parameter");
            th_txt(ui, o.x + c1 + c2 - 8.0, Align2::RIGHT_TOP, "1:2 Diluted");
            th_txt(ui, o.x + iw - 8.0, Align2::RIGHT_TOP, "Calculated Neat");
            let line_y = hy + LABEL_H + 10.0;
            ui.painter().line_segment([pos2(o.x, line_y + 0.5), pos2(o.x + iw, line_y + 0.5)], Stroke::new(1.0, th.line));
            let mut y = line_y + 1.0;
            for (i, (name, hint)) in DIL_PARAMS.iter().enumerate() {
                let rh = 10.0 + FIELD_H + 10.0 + 1.0;
                let mid = y + 10.0 + FIELD_H / 2.0;
                text(ui, th, pos2(o.x + 8.0, mid), Align2::LEFT_CENTER, name, Role::SansB, 16.0, th.text);
                let fw = 130.0_f32.min(c2 - 16.0);
                let fr = Rect::from_min_size(pos2(o.x + c1 + c2 - 8.0 - fw, y + 10.0), vec2(fw, FIELD_H));
                let n = parse_number(&self.dil[i]);
                field(ui, th, fr, Id::new(("dil", i)), &mut self.dil[i], hint, false);
                let n = if n == Num::Empty { parse_number(&self.dil[i]) } else { parse_number(&self.dil[i]) };
                let out = match n {
                    Num::Empty => "—".to_string(),
                    Num::Invalid => "Invalid".to_string(),
                    Num::Val(v) => fmt_js(v * 2.0),
                };
                text(ui, th, pos2(o.x + c1 + c2 + c3 - 8.0, mid), Align2::RIGHT_CENTER, &out, Role::MonoB, 16.0, th.accent);
                y += rh;
                if i + 1 < DIL_PARAMS.len() {
                    ui.painter().line_segment([pos2(o.x, y - 0.5), pos2(o.x + iw, y - 0.5)], Stroke::new(1.0, th.line));
                }
            }
            ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y - 1.0)), Sense::hover());
        });
        rect.bottom()
    }
}
