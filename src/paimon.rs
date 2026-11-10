//! Genshin-theme mascot: a pixel-art Paimon who floats around the UI.
//!
//! She shares the perch registry with the Pikachu mascot (buttons, cards, the
//! theme rail, the window floor) but moves her own way. She never walks or
//! falls: she glides along curved flight paths, hovers with a gentle bob (her
//! halo lagging a beat behind), perches on card edges with her legs dangling,
//! and now and then vanishes in a burst of stars and reappears elsewhere like
//! a waypoint teleport.
//!
//! Her interactions echo Pikachu's but are her own:
//! * right-click anywhere and she flies straight there (she can fly, so it is
//!   the exact spot, not the nearest surface) and has a look around;
//! * click her and she twirls and giggles, but poke her three times in a row
//!   and she puffs up ("Paimon is NOT emergency food!") and storms off;
//! * move the cursor near her and she may tag along behind it for a while;
//! * flick the cursor at her and she zips out of the way;
//! * she gets hungry over time and stops for a snack.

use crate::pikachu::{find, perches, Perch, PerchKind};
use crate::theme::{hex, rgba, Role, Theme};
use eframe::egui::{
    self, epaint::StrokeKind, pos2, vec2, Align2, Color32, Id, LayerId, Order, Painter, Pos2, Rect,
    Shape, Stroke, Vec2,
};
use std::f32::consts::TAU;

/// Size of one sprite pixel, in points (snapped to whole physical pixels).
/// Two screen pixels per sprite pixel at 1x: she stands about 100 pt tall.
const S_PT: f32 = 2.0;
/// Sprite height (halo tip to soles), points.
const HEIGHT: f32 = 50.0 * S_PT;
/// Lowest a hovering Paimon's soles may be raised: keeps her halo below the title bar.
const TOP_CLEAR: f32 = 54.0 + HEIGHT;

/// `rows` sprite pixels, in points.
fn px(rows: f32) -> f32 {
    rows * S_PT
}
/// Hover height above a surface, points.
const HOVER: f32 = 10.0;

// ---------------------------------------------------------------------------
// Behaviour
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Idle {
    Float,
    Look,
    Twirl,
    Snack,
    Think,
    Cheer,
    Wave,
    Doze,
    Sit,
}

#[derive(Clone, Copy, Debug)]
enum Station {
    /// Over (or sitting on) a perch, `dx` from its left end.
    Perch { id: Id, dx: f32 },
    /// A point in mid-air (feet position).
    Air(Pos2),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Then {
    Rest,
    Visit,
}

#[derive(Clone, Copy, Debug)]
enum Act {
    Idle { kind: Idle, dur: f32 },
    Fly { from: Pos2, ctrl: Pos2, to: Station, dur: f32, then: Then },
    Teleport { to: Station, moved: bool },
    Tag { dur: f32 },
    Angry { dur: f32 },
    Visit { dur: f32 },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PartKind {
    Star,
    Glint,
    Anger,
    Z,
    Crumb,
}

#[derive(Clone, Copy)]
struct Particle {
    pos: Pos2,
    vel: Vec2,
    life: f32,
    max: f32,
    kind: PartKind,
}

const HAPPY: [&str; 5] = ["Hehe!", "Ehe~", "Let's go, Traveler!", "Paimon's the best guide!", "Ooh, shiny!"];

pub struct Paimon {
    rng: u64,
    on: bool,
    /// Feet position (bottom of the boots).
    pos: Pos2,
    station: Station,
    facing: f32,
    act: Act,
    t: f32,
    clock: f32,
    hunger: f32,
    energy: f32,
    alpha: f32,
    vel: Vec2,
    clicks: Vec<f32>,
    still_t: f32,
    face_cd: f32,
    blink_in: f32,
    blink_t: f32,
    fx_t: f32,
    last_choice: u8,
    bubble: Option<(&'static str, f32)>,
    parts: Vec<Particle>,
}

impl Default for Paimon {
    fn default() -> Self {
        Self::new()
    }
}

impl Paimon {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64 ^ 0x5bd1_e995)
            .unwrap_or(0x2545_f491)
            | 1;
        Paimon {
            rng: seed,
            on: false,
            pos: Pos2::ZERO,
            station: Station::Air(Pos2::ZERO),
            facing: 1.0,
            act: Act::Idle { kind: Idle::Float, dur: 1.0 },
            t: 0.0,
            clock: 0.0,
            hunger: 0.2,
            energy: 0.9,
            alpha: 1.0,
            vel: Vec2::ZERO,
            clicks: Vec::new(),
            still_t: 0.0,
            face_cd: 0.0,
            blink_in: 2.0,
            blink_t: 0.0,
            fx_t: 0.0,
            last_choice: 255,
            bubble: None,
            parts: Vec::new(),
        }
    }

    fn rnd(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.rnd()
    }

    fn chance(&mut self, p: f32) -> bool {
        self.rnd() < p
    }

    fn set(&mut self, a: Act) {
        self.act = a;
        self.t = 0.0;
    }

    fn say(&mut self, s: &'static str, life: f32) {
        self.bubble = Some((s, life));
    }

    fn emit(&mut self, kind: PartKind, pos: Pos2, vel: Vec2, life: f32) {
        if self.parts.len() < 72 {
            self.parts.push(Particle { pos, vel, life, max: life, kind });
        }
    }

    /// Advance and draw. `enabled` is false outside the Genshin theme.
    pub fn tick(&mut self, ctx: &egui::Context, th: &Theme, enabled: bool) {
        if !enabled {
            self.on = false;
            return;
        }
        let ps = perches(ctx);
        if ps.is_empty() {
            return;
        }
        let screen = ctx.screen_rect();
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.05);
        self.clock += dt;
        self.hunger = (self.hunger + 0.008 * dt).min(1.0);
        if !self.on {
            self.on = true;
            self.parts.clear();
            self.station = self.pick_station(&ps, screen);
            if let Some(p) = self.station_point(self.station, &ps, false) {
                self.pos = p;
            }
            self.alpha = 0.0;
            self.set(Act::Teleport { to: self.station, moved: true });
            self.t = 0.6;
            self.say("Hey! Paimon's here!", 1.8);
        }
        self.update_act(dt, &ps, screen, ctx);
        self.pointer(ctx, &ps, screen, dt);
        self.timers(dt);
        self.update_particles(dt);
        self.draw(ctx, th, &ps);
        ctx.request_repaint();
    }

    fn station_point(&self, st: Station, ps: &[Perch], sit: bool) -> Option<Pos2> {
        match st {
            Station::Air(p) => Some(p),
            Station::Perch { id, dx } => find(ps, id).map(|p| {
                let x = p.x0 + dx.clamp(0.0, p.x1 - p.x0);
                if sit {
                    // seat on the edge, legs dangling over it
                    pos2(x, p.y + 4.0 * S_PT)
                } else {
                    pos2(x, p.y - HOVER)
                }
            }),
        }
    }

    fn pick_station(&mut self, ps: &[Perch], screen: Rect) -> Station {
        if self.chance(0.22) {
            let x = self.range(screen.left() + 70.0, screen.right() - 70.0);
            let y = self.range(screen.top() + TOP_CLEAR + 10.0, screen.bottom() - 40.0);
            return Station::Air(pos2(x, y));
        }
        let cur = match self.station {
            Station::Perch { id, .. } => Some(id),
            Station::Air(_) => None,
        };
        let mut best: Option<(f32, Station)> = None;
        for p in ps {
            if p.kind == PerchKind::Hang || Some(p.id) == cur || too_high(p, screen) {
                continue;
            }
            let dx = self.range(0.0, p.x1 - p.x0);
            let at = pos2(p.x0 + dx, p.y);
            let d = (at - self.pos).length();
            let mut score = self.range(0.1, 1.0) / (1.0 + (d - 280.0).abs() / 320.0);
            if p.kind == PerchKind::Floor {
                score *= 0.3;
            }
            if best.as_ref().map_or(true, |b| score > b.0) {
                best = Some((score, Station::Perch { id: p.id, dx }));
            }
        }
        best.map(|b| b.1).unwrap_or(Station::Air(pos2(screen.center().x, screen.center().y)))
    }

    fn fly_to(&mut self, to: Station, ps: &[Perch], then: Then, speed: f32) {
        let from = self.pos;
        let Some(end) = self.station_point(to, ps, false) else { return };
        let d = end - from;
        let dist = d.length().max(1.0);
        let perp = vec2(-d.y, d.x) / dist;
        let bend = self.range(-0.45, 0.45) * dist.min(420.0);
        let rise = self.range(10.0, 60.0);
        let ctrl = from + d * 0.5 + perp * bend - vec2(0.0, rise);
        let dur = ((0.55 + dist / 430.0) * self.range(0.9, 1.15) / speed).clamp(0.3, 2.3);
        if d.x.abs() > 4.0 {
            self.facing = d.x.signum();
        }
        self.energy -= 0.02;
        self.set(Act::Fly { from, ctrl, to, dur, then });
    }

    fn update_act(&mut self, dt: f32, ps: &[Perch], screen: Rect, ctx: &egui::Context) {
        self.t += dt;
        self.fx_t -= dt;
        match self.act {
            Act::Idle { kind, dur } => {
                match self.station_point(self.station, ps, kind == Idle::Sit) {
                    Some(p) => self.pos = p,
                    None => {
                        // her surface went away (tab switch, scroll): float off somewhere else
                        let st = self.pick_station(ps, screen);
                        self.fly_to(st, ps, Then::Rest, 1.0);
                        return;
                    }
                }
                self.idle_tick(kind, dt);
                if self.t >= dur {
                    self.next(ps, screen);
                }
            }
            Act::Fly { from, ctrl, to, dur, then } => {
                let Some(end) = self.station_point(to, ps, false) else {
                    let st = self.pick_station(ps, screen);
                    self.fly_to(st, ps, then, 1.0);
                    return;
                };
                let u = (self.t / dur).min(1.0);
                let e = smoother(u);
                let prev = self.pos;
                self.pos = quad_bezier(from, ctrl, end, e);
                let v = self.pos - prev;
                if v.x.abs() > 0.4 {
                    self.facing = v.x.signum();
                }
                self.energy -= 0.03 * dt;
                if self.fx_t <= 0.0 && v.length() > 1.0 {
                    self.fx_t = self.range(0.04, 0.09);
                    let at = self.pos + vec2(px(self.range(-6.0, 6.0)), -px(self.range(8.0, 30.0)));
                    let k = if self.chance(0.4) { PartKind::Star } else { PartKind::Glint };
                    self.emit(k, at, -v / dt.max(0.001) * 0.05, 0.6);
                }
                if u >= 1.0 {
                    self.station = to;
                    match then {
                        Then::Visit => {
                            let dur = self.range(2.5, 4.5);
                            if self.chance(0.6) {
                                let s = if self.chance(0.5) { "Ooh, what's this?" } else { "Over here, Traveler!" };
                                self.say(s, 1.8);
                            }
                            self.set(Act::Visit { dur });
                        }
                        Then::Rest => self.next(ps, screen),
                    }
                }
            }
            Act::Teleport { to, moved } => {
                if self.t < 0.35 {
                    self.alpha = 1.0 - self.t / 0.35;
                    if self.fx_t <= 0.0 {
                        self.fx_t = 0.03;
                        self.burst(2);
                    }
                } else if self.t < 0.6 {
                    self.alpha = 0.0;
                } else {
                    if !moved {
                        if let Some(p) = self.station_point(to, ps, false) {
                            self.pos = p;
                        }
                        self.station = to;
                        self.burst(10);
                        self.set(Act::Teleport { to, moved: true });
                        self.t = 0.6;
                    }
                    self.alpha = ((self.t - 0.6) / 0.35).min(1.0);
                    if let Some(p) = self.station_point(self.station, ps, false) {
                        self.pos = p;
                    }
                    if self.t >= 0.95 {
                        self.alpha = 1.0;
                        if self.bubble.is_none() && self.chance(0.5) {
                            self.say("Ta-da!", 1.2);
                        }
                        self.set(Act::Idle { kind: Idle::Cheer, dur: 0.9 });
                    }
                }
            }
            Act::Tag { dur } => {
                let (hover, pv) = ctx.input(|i| (i.pointer.hover_pos(), i.pointer.velocity()));
                let Some(cur) = hover else {
                    let st = self.pick_station(ps, screen);
                    self.fly_to(st, ps, Then::Rest, 1.0);
                    return;
                };
                if pv.x.abs() > 30.0 {
                    self.facing = pv.x.signum();
                }
                // trail behind and a little below the cursor, on a soft spring
                let target = cur + vec2(-self.facing * (px(16.0) + 14.0), px(30.0));
                let target = pos2(
                    target.x.clamp(screen.left() + px(15.0), screen.right() - px(15.0)),
                    target.y.clamp(screen.top() + TOP_CLEAR, screen.bottom() - 4.0),
                );
                self.vel += ((target - self.pos) * 28.0 - self.vel * 8.5) * dt;
                self.pos += self.vel * dt;
                if self.fx_t <= 0.0 && self.vel.length() > 60.0 {
                    self.fx_t = 0.08;
                    let at = self.pos + vec2(0.0, -px(self.range(9.0, 28.0)));
                    self.emit(PartKind::Glint, at, -self.vel * 0.05, 0.5);
                }
                if self.t > dur || self.still_t > 1.6 {
                    let st = self.nearest_station(ps, screen);
                    if self.chance(0.4) {
                        self.say("Phew, Paimon needs a break.", 1.6);
                    }
                    self.fly_to(st, ps, Then::Rest, 1.0);
                }
            }
            Act::Angry { dur } => {
                if self.fx_t <= 0.0 {
                    self.fx_t = self.range(0.25, 0.4);
                    let at = self.pos + vec2(px(self.range(5.0, 12.0)) * if self.chance(0.5) { 1.0 } else { -1.0 }, -px(42.0));
                    self.emit(PartKind::Anger, at, vec2(0.0, -10.0), 0.6);
                }
                if self.t >= dur {
                    // storm off somewhere far away
                    let mut st = self.pick_station(ps, screen);
                    for _ in 0..6 {
                        let far = self.station_point(st, ps, false).map_or(0.0, |p| (p - self.pos).length());
                        if far > 260.0 {
                            break;
                        }
                        st = self.pick_station(ps, screen);
                    }
                    self.say("Hmph!", 1.0);
                    self.fly_to(st, ps, Then::Rest, 1.3);
                }
            }
            Act::Visit { dur } => {
                if let Some(p) = self.station_point(self.station, ps, false) {
                    self.pos = p;
                }
                if self.face_cd <= 0.0 {
                    self.face_cd = self.range(0.6, 1.4);
                    self.facing = -self.facing;
                }
                if self.t >= dur {
                    self.next(ps, screen);
                }
            }
        }
    }

    fn burst(&mut self, n: usize) {
        for _ in 0..n {
            let a = self.range(0.0, TAU);
            let sp = self.range(30.0, 90.0);
            let at = self.pos + vec2(0.0, -px(25.0));
            let k = if self.chance(0.5) { PartKind::Star } else { PartKind::Glint };
            let life = self.range(0.4, 0.8);
            self.emit(k, at, vec2(a.cos() * sp, a.sin() * sp), life);
        }
    }

    fn nearest_station(&mut self, ps: &[Perch], screen: Rect) -> Station {
        let mut best: Option<(f32, Station)> = None;
        for p in ps.iter().filter(|p| p.kind != PerchKind::Hang && !too_high(p, screen)) {
            let dx = (self.pos.x - p.x0).clamp(0.0, p.x1 - p.x0);
            let d = (pos2(p.x0 + dx, p.y - HOVER) - self.pos).length();
            if best.as_ref().map_or(true, |b| d < b.0) {
                best = Some((d, Station::Perch { id: p.id, dx }));
            }
        }
        best.map(|b| b.1).unwrap_or(Station::Air(self.pos))
    }

    fn idle_tick(&mut self, kind: Idle, dt: f32) {
        self.energy = (self.energy + if kind == Idle::Doze { 0.1 } else { 0.02 } * dt).min(1.0);
        match kind {
            Idle::Look => {
                if self.face_cd <= 0.0 {
                    self.face_cd = self.range(0.5, 1.4);
                    self.facing = -self.facing;
                }
            }
            Idle::Snack => {
                if self.fx_t <= 0.0 {
                    self.fx_t = self.range(0.25, 0.5);
                    let at = self.pos + vec2(px(self.range(-3.0, 3.0)), -px(22.0));
                    let vx = self.range(-20.0, 20.0);
                    self.emit(PartKind::Crumb, at, vec2(vx, 10.0), 0.6);
                }
                self.hunger = (self.hunger - 0.4 * dt).max(0.0);
            }
            Idle::Doze => {
                if self.fx_t <= 0.0 {
                    self.fx_t = self.range(1.0, 1.6);
                    let at = self.pos + vec2(self.facing * px(12.0), -px(42.0));
                    self.emit(PartKind::Z, at, vec2(self.facing * 6.0, -14.0), 1.8);
                }
            }
            Idle::Twirl | Idle::Cheer => {
                if self.fx_t <= 0.0 {
                    self.fx_t = 0.12;
                    let at = self.pos + vec2(px(self.range(-13.0, 13.0)), -px(self.range(5.0, 45.0)));
                    self.emit(PartKind::Star, at, vec2(0.0, -20.0), 0.6);
                }
            }
            _ => {}
        }
    }

    fn next(&mut self, ps: &[Perch], screen: Rect) {
        self.energy = self.energy.clamp(0.0, 1.0);
        let on_top = match self.station {
            Station::Perch { id, .. } => find(ps, id).is_some_and(|p| p.kind == PerchKind::Top),
            Station::Air(_) => false,
        };
        let e = self.energy;
        let h = self.hunger;
        let mut opts: Vec<(f32, u8)> = vec![
            (2.4, 0),
            (1.4, 1),
            (0.7, 2),
            (0.3 + 2.5 * h, 3),
            (0.6, 4),
            (0.5, 5),
            (0.5, 6),
            (if e < 0.3 { 4.0 } else { 0.1 }, 7),
            (if on_top { 1.5 } else { 0.0 }, 8),
            (3.0 * e + 0.4, 20),
            (0.35, 21),
        ];
        for o in &mut opts {
            if o.1 == self.last_choice {
                o.0 *= 0.25;
            }
        }
        let total: f32 = opts.iter().map(|o| o.0).sum();
        let mut r = self.rnd() * total;
        let mut c = 0;
        for o in &opts {
            if r < o.0 {
                c = o.1;
                break;
            }
            r -= o.0;
        }
        self.last_choice = c;
        let idle = |kind: Idle, dur: f32| Act::Idle { kind, dur };
        match c {
            0 => {
                let d = self.range(1.5, 4.0);
                if h > 0.75 && self.chance(0.4) {
                    self.say("Paimon's hungry...", 1.6);
                }
                self.set(idle(Idle::Float, d));
            }
            1 => {
                let d = self.range(1.5, 3.0);
                self.set(idle(Idle::Look, d));
            }
            2 => {
                if self.chance(0.4) {
                    self.say("Wheee~", 1.0);
                }
                self.set(idle(Idle::Twirl, 0.96));
            }
            3 => {
                let d = self.range(2.5, 3.5);
                let s = if self.chance(0.5) { "Sweet Madame!" } else { "Nom nom~" };
                self.say(s, 1.4);
                self.set(idle(Idle::Snack, d));
            }
            4 => {
                let d = self.range(1.8, 2.8);
                if self.chance(0.6) {
                    self.say("Hmm...", 1.4);
                }
                self.set(idle(Idle::Think, d));
            }
            5 => {
                let i = (self.rnd() * HAPPY.len() as f32) as usize % HAPPY.len();
                self.say(HAPPY[i], 1.5);
                self.set(idle(Idle::Cheer, 1.0));
            }
            6 => {
                let d = self.range(1.2, 1.8);
                if self.chance(0.5) {
                    self.say("Hey, Traveler!", 1.4);
                }
                self.set(idle(Idle::Wave, d));
            }
            7 => {
                let d = self.range(6.0, 12.0);
                self.fx_t = 0.6;
                self.set(idle(Idle::Doze, d));
            }
            8 => {
                let d = self.range(4.0, 9.0);
                self.set(idle(Idle::Sit, d));
            }
            21 => {
                let st = self.pick_station(ps, screen);
                self.set(Act::Teleport { to: st, moved: false });
            }
            _ => {
                let st = self.pick_station(ps, screen);
                self.fly_to(st, ps, Then::Rest, 1.0);
            }
        }
    }

    fn busy(&self) -> bool {
        matches!(self.act, Act::Teleport { .. } | Act::Angry { .. })
    }

    fn pointer(&mut self, ctx: &egui::Context, ps: &[Perch], screen: Rect, dt: f32) {
        let (hover, vel, pressed, secondary) = ctx.input(|i| {
            (i.pointer.hover_pos(), i.pointer.velocity(), i.pointer.primary_pressed(), i.pointer.secondary_pressed())
        });
        self.still_t = if vel.length() < 5.0 { self.still_t + dt } else { 0.0 };
        let Some(hp) = hover else { return };

        if secondary {
            if matches!(self.act, Act::Angry { .. }) {
                self.say("Hmph! Not listening!", 1.2);
            } else if !matches!(self.act, Act::Teleport { .. }) {
                if self.chance(0.5) {
                    self.say("Coming, Traveler!", 1.2);
                }
                // body centred on the click: her feet hang below it
                let at = pos2(hp.x, (hp.y + px(25.0)).clamp(screen.top() + TOP_CLEAR, screen.bottom() - 4.0));
                self.fly_to(Station::Air(at), ps, Then::Visit, 1.25);
            }
            return;
        }

        let body = self.body_rect();
        if pressed && body.expand(4.0).contains(hp) && self.alpha > 0.5 {
            self.clicked();
            return;
        }
        if self.busy() {
            return;
        }
        let d = (hp - body.center()).length();
        let speed = vel.length();
        let idle = matches!(self.act, Act::Idle { .. } | Act::Visit { .. });
        if idle && d < 70.0 && speed > 1400.0 {
            // zip out of the way
            self.say("Eek!", 0.9);
            let away = (body.center() - hp).normalized();
            let away = if away.x.is_finite() { away } else { vec2(1.0, 0.0) };
            let to = self.pos + away * self.range(90.0, 150.0);
            let to = pos2(to.x.clamp(screen.left() + px(15.0), screen.right() - px(15.0)), to.y.clamp(screen.top() + TOP_CLEAR, screen.bottom() - 4.0));
            self.fly_to(Station::Air(to), ps, Then::Rest, 2.6);
            return;
        }
        if let Act::Idle { kind, .. } = self.act {
            if kind == Idle::Doze {
                return;
            }
            if d < 220.0 && (80.0..900.0).contains(&speed) && self.chance(0.35 * dt) {
                self.say("Wait for Paimon!", 1.4);
                self.vel = Vec2::ZERO;
                let dur = self.range(4.0, 8.0);
                self.set(Act::Tag { dur });
                return;
            }
            if d < 180.0 && self.face_cd <= 0.0 && (hp.x - self.pos.x).abs() > 10.0 && kind != Idle::Twirl {
                self.facing = (hp.x - self.pos.x).signum();
                self.face_cd = 0.8;
            }
        }
    }

    fn clicked(&mut self) {
        self.clicks.retain(|&c| self.clock - c < 2.5);
        self.clicks.push(self.clock);
        if self.clicks.len() >= 3 {
            self.clicks.clear();
            self.say("Paimon is NOT emergency food!", 2.2);
            self.set(Act::Angry { dur: 1.8 });
            return;
        }
        if let Act::Idle { kind: Idle::Doze, .. } = self.act {
            self.say("Huh?! Paimon wasn't sleeping!", 1.6);
            self.set(Act::Idle { kind: Idle::Cheer, dur: 0.9 });
            return;
        }
        if matches!(self.act, Act::Idle { .. } | Act::Visit { .. }) {
            let line = if self.chance(0.5) { "Hehe!" } else { "Ehe~" };
            self.say(line, 1.1);
            self.burst(5);
            self.set(Act::Idle { kind: Idle::Twirl, dur: 0.96 });
        }
    }

    fn timers(&mut self, dt: f32) {
        self.blink_in -= dt;
        if self.blink_in <= 0.0 {
            self.blink_t = 0.12;
            self.blink_in = if self.chance(0.2) { 0.3 } else { self.range(2.0, 5.0) };
        }
        self.blink_t = (self.blink_t - dt).max(0.0);
        self.face_cd = (self.face_cd - dt).max(0.0);
        if let Some((_, life)) = &mut self.bubble {
            *life -= dt;
            if *life <= 0.0 {
                self.bubble = None;
            }
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for p in &mut self.parts {
            p.pos += p.vel * dt;
            match p.kind {
                PartKind::Crumb => p.vel.y += 160.0 * dt,
                PartKind::Z => p.pos.x += (p.life * 5.0).sin() * 10.0 * dt,
                _ => p.vel *= (1.0 - 3.0 * dt).max(0.0),
            }
            p.life -= dt;
        }
        self.parts.retain(|p| p.life > 0.0);
    }

    fn body_rect(&self) -> Rect {
        let o = self.pos - vec2(0.0, self.bob());
        Rect::from_min_max(pos2(o.x - 13.0 * S_PT, o.y - 42.0 * S_PT), pos2(o.x + 13.0 * S_PT, o.y))
    }

    /// Vertical float offset, points (positive = up).
    fn bob(&self) -> f32 {
        match self.act {
            Act::Idle { kind: Idle::Sit, .. } => 0.0,
            Act::Idle { kind: Idle::Doze, .. } => 1.5 * (self.clock * 1.2).sin() - 3.0,
            Act::Idle { kind: Idle::Cheer, .. } => 3.0 * (self.clock * 2.0).sin() + 6.0 * (self.t * 9.0).sin().abs(),
            Act::Fly { .. } | Act::Tag { .. } => 1.5 * (self.clock * 5.0).sin(),
            _ => 3.0 * (self.clock * 2.0).sin(),
        }
    }

    // --- pose --------------------------------------------------------------

    fn pose(&self) -> Pose {
        let mut p = Pose::default();
        let t = self.t;
        let beat = |rate: f32| (t * rate) as i32;
        // the cape flutters and the legs sway lazily while floating
        p.cape_b = (self.clock * 2.6) as i32 % 2 == 0;
        let sway = (self.clock * 1.6).sin();
        p.legs = if sway > 0.5 { (-1, 0) } else if sway < -0.5 { (0, -1) } else { (0, 0) };
        // the halo trails the bob by a beat
        p.halo_dy = ((self.clock * 2.0 - 0.9).sin() * -1.2).round() as i32;
        p.look = self.facing.signum() as i32;
        match self.act {
            Act::Idle { kind, .. } => match kind {
                Idle::Float => {}
                Idle::Look => p.look = 2 * self.facing.signum() as i32,
                Idle::Twirl => {
                    // front, side, back, side, front... a quick spin
                    let k = beat(12.5) % 4;
                    match k {
                        0 => {}
                        1 => p.look = 2,
                        2 => p.back = true,
                        _ => p.look = -2,
                    }
                    p.arms = (Arm::Out, Arm::Out);
                    p.eyes = Eyes::Happy;
                    p.mouth = Mouth::Open;
                    p.cape_b = k % 2 == 0;
                }
                Idle::Snack => {
                    p.food = true;
                    p.eyes = if t < 0.6 { Eyes::Star } else { Eyes::Happy };
                    p.mouth = if beat(6.0) % 2 == 0 { Mouth::Open } else { Mouth::None };
                    p.look = 0;
                }
                Idle::Think => {
                    p.arms.1 = Arm::Chin;
                    p.look = self.facing.signum() as i32;
                    p.hdy = -1;
                    p.mouth = Mouth::Pout;
                }
                Idle::Cheer => {
                    p.arms = (Arm::Up, Arm::Up);
                    p.eyes = Eyes::Happy;
                    p.mouth = Mouth::Open;
                    p.look = 0;
                }
                Idle::Wave => {
                    p.arms.0 = if beat(7.0) % 2 == 0 { Arm::Up } else { Arm::Out };
                    p.mouth = Mouth::Smile;
                }
                Idle::Doze => {
                    p.eyes = Eyes::Closed;
                    p.hdy = 1;
                    p.hdx = if (self.clock * 0.7).sin() > 0.0 { 1 } else { 0 };
                    p.look = 0;
                    p.legs = (0, 0);
                }
                Idle::Sit => {
                    p.legs = if beat(3.0) % 2 == 0 { (-1, 0) } else { (0, -1) };
                    p.mouth = Mouth::Smile;
                }
            },
            Act::Fly { dur, .. } => {
                let u = (t / dur).min(1.0);
                let fast = (0.2..0.8).contains(&u);
                p.lean = if fast { 2 } else { 1 };
                p.cape_b = (self.clock * 9.0) as i32 % 2 == 0;
                p.legs = (0, 1);
                if fast {
                    p.arms = (Arm::Out, Arm::Out);
                }
                p.mouth = Mouth::Smile;
            }
            Act::Tag { .. } => {
                p.lean = if self.vel.length() > 120.0 { 2 } else { 1 };
                p.cape_b = (self.clock * 7.0) as i32 % 2 == 0;
                p.eyes = Eyes::Happy;
                p.mouth = Mouth::Smile;
            }
            Act::Teleport { .. } => {
                p.eyes = Eyes::Closed;
                p.arms = (Arm::Up, Arm::Up);
            }
            Act::Angry { .. } => {
                p.eyes = Eyes::Angry;
                p.mouth = if beat(4.0) % 2 == 0 { Mouth::Wail } else { Mouth::Pout };
                p.hdx = if beat(28.0) % 2 == 0 { 1 } else { -1 };
                p.blush = R2;
                p.arms = (Arm::Out, Arm::Out);
                p.look = 0;
            }
            Act::Visit { .. } => {
                p.eyes = if t < 0.6 { Eyes::Wide } else { Eyes::Open };
                p.mouth = if t < 1.2 { Mouth::Open } else { Mouth::Smile };
                p.look = 2 * self.facing.signum() as i32;
            }
        }
        if self.blink_t > 0.0 && p.eyes == Eyes::Open {
            p.eyes = Eyes::Closed;
        }
        if p.lean != 0 {
            p.lean *= self.facing.signum() as i32;
        }
        p
    }

    // --- drawing -----------------------------------------------------------

    fn draw(&self, ctx: &egui::Context, th: &Theme, ps: &[Perch]) {
        let ppp = ctx.pixels_per_point();
        let s = (S_PT * ppp).round().max(1.0) / ppp;
        let snap = |v: f32| (v * ppp).round() / ppp;
        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("paimon-layer")));
        let origin = self.pos - vec2(0.0, self.bob());
        let origin = pos2(snap(origin.x), snap(origin.y));

        // A soft shadow on the surface below shows how high she floats.
        if !matches!(self.act, Act::Idle { kind: Idle::Sit, .. }) && self.alpha > 0.05 {
            let below = ps
                .iter()
                .filter(|p| p.kind != PerchKind::Hang && self.pos.x >= p.x0 && self.pos.x <= p.x1 && p.y >= origin.y - 2.0)
                .map(|p| p.y)
                .fold(None, |a: Option<f32>, y| Some(a.map_or(y, |a| a.min(y))));
            if let Some(y) = below {
                let h = (y - origin.y).max(0.0);
                let k = 1.0 / (1.0 + h / 60.0);
                let w = (14.0 * k).round().max(4.0);
                let c = Color32::from_black_alpha((60.0 * k * self.alpha) as u8);
                painter.rect_filled(Rect::from_min_size(pos2(snap(self.pos.x) - w * s / 2.0, snap(y) - s), vec2(w * s, s)), 0.0, c);
            }
        }

        if self.alpha > 0.01 {
            let grid = compose(&self.pose());
            paint_grid(&painter, &grid, origin, s, self.alpha);
        }

        for part in &self.parts {
            let a = (part.life / part.max).clamp(0.0, 1.0);
            let a = (a * 2.5).min(1.0);
            let (bmp, scale): (&[&str], f32) = match part.kind {
                PartKind::Star => (&["..Y..", ".YEY.", "YEEEY", ".YEY.", "..Y.."], 0.6),
                PartKind::Glint => (&[".C.", "CEC", ".C."], 0.5),
                PartKind::Anger => (&["RR.RR", "R...R", ".....", "R...R", "RR.RR"], 0.75),
                PartKind::Z => (&["EEE", "..E", ".E.", "E..", "EEE"], 0.75),
                PartKind::Crumb => (&["D"], 1.0),
            };
            let at = pos2(snap(part.pos.x), snap(part.pos.y));
            paint_bitmap(&painter, bmp, at, s * scale, a);
        }

        if let Some((txt, life)) = self.bubble {
            // Genshin dialogue style: dark slate plate, gold rim, cream text.
            let font = th.font(Role::SansB, 11.5);
            let ink = hex(0xece5d8);
            let galley = painter.layout_no_wrap(txt.to_string(), font.clone(), ink);
            let size = galley.size() + vec2(16.0, 8.0);
            let fade = (life / 0.3).clamp(0.0, 1.0);
            let anchor = pos2(origin.x + self.facing * 10.0 * s, origin.y - 51.0 * s);
            let screen = ctx.screen_rect();
            let mut r = Rect::from_center_size(pos2(anchor.x + self.facing * size.x * 0.3, anchor.y - size.y / 2.0 - 6.0), size);
            let shift_x = (screen.left() + 4.0 - r.left()).max(0.0) + (screen.right() - 4.0 - r.right()).min(0.0);
            let shift_y = (screen.top() + 48.0 - r.top()).max(0.0);
            r = r.translate(vec2(shift_x, shift_y));
            let r = Rect::from_min_max(pos2(snap(r.left()), snap(r.top())), pos2(snap(r.right()), snap(r.bottom())));
            let plate = rgba(hex(0x3b4255), 0.94 * fade);
            let gold = rgba(hex(0xd3bc8e), fade);
            let tx = anchor.x.clamp(r.left() + 8.0, r.right() - 8.0);
            painter.add(Shape::convex_polygon(
                vec![pos2(tx - 4.0, r.bottom() - 1.0), pos2(tx + 4.0, r.bottom() - 1.0), pos2(anchor.x, r.bottom() + 6.0)],
                plate,
                Stroke::new(1.0_f32, gold),
            ));
            painter.rect_filled(r, r.height() / 2.0, plate);
            painter.rect_stroke(r, r.height() / 2.0, Stroke::new(1.0_f32, gold), StrokeKind::Inside);
            painter.add(Shape::convex_polygon(
                vec![pos2(tx - 2.8, r.bottom() - 2.0), pos2(tx + 2.8, r.bottom() - 2.0), pos2(anchor.x + (tx - anchor.x) * 0.5, r.bottom() + 2.5)],
                plate,
                Stroke::NONE,
            ));
            painter.text(r.center(), Align2::CENTER_CENTER, txt, font, rgba(ink, fade));
        }
    }
}

/// Paimon is tall: hovering over a surface this close to the top would push
/// her halo into the title bar.
fn too_high(p: &Perch, screen: Rect) -> bool {
    p.y - HOVER < screen.top() + TOP_CLEAR
}

fn smoother(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

fn quad_bezier(a: Pos2, c: Pos2, b: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    pos2(u * u * a.x + 2.0 * u * t * c.x + t * t * b.x, u * u * a.y + 2.0 * u * t * c.y + t * t * b.y)
}

// ---------------------------------------------------------------------------
// Pixel art
//
// Traced from references/genshin/paimon.jpg at its native resolution (the
// reference is 4.33 image pixels per art pixel), then cleaned up by hand and
// split into layers so every animation reuses the same art:
//
//   rows 0-7    halo: a thin ring split by its cross-spike
//   rows 8-26   hair and face: the white bob with tufts flaring out at the
//               sides, a black four-point star pin and a blue clip
//   rows 27-38  dress: the black cape collar, red cape flaps, grey dress
//               and red hem
//   rows 39-49  legs: thighs and the flared red-and-white boots
//
// 29 columns wide, mirrored across column 14. Arms, the face and props are
// overlays; the soles sit on row 49.
// ---------------------------------------------------------------------------

const KO: u8 = 1; // outline
const WH: u8 = 2; // hair white
const GR: u8 = 3; // dress / hair grey
const SK: u8 = 4; // skin
const VI: u8 = 5; // iris violet
const VD: u8 = 6; // dark violet lash
const NV: u8 = 7; // cape black-navy
const GO: u8 = 8; // gold
const OR: u8 = 9; // cape red light
const RD: u8 = 10; // red
const RK: u8 = 11; // dark red
const BL: u8 = 12; // blue clip
const XS: u8 = 13; // star pin black
const PK: u8 = 14; // blush
const MO: u8 = 15; // mouth line
const MI: u8 = 16; // mouth open
const EW: u8 = 17; // eye white / highlight
const R2: u8 = 18; // angry flush
const WL: u8 = 19; // hair light grey
const GD: u8 = 20; // hair shade
const SP: u8 = 21; // skin shade

fn color(c: u8) -> Color32 {
    match c {
        KO => hex(0x26232e),
        WH => hex(0xf1ece6),
        GR => hex(0xb3abb2),
        SK => hex(0xfbd2bd),
        VI => hex(0x7d5bbd),
        VD => hex(0x2e2150),
        NV => hex(0x1d1e2c),
        GO => hex(0xf2c14e),
        OR => hex(0xd06a55),
        RD => hex(0xb4473f),
        RK => hex(0x6a2a2c),
        BL => hex(0x5d7fc6),
        XS => hex(0x131218),
        PK => hex(0xf2a3a0),
        MO => hex(0xa85a55),
        MI => hex(0x8e3438),
        EW => Color32::WHITE,
        R2 => hex(0xee7466),
        WL => hex(0xd6d0cf),
        GD => hex(0x8f8590),
        SP => hex(0xe2ab98),
        _ => Color32::TRANSPARENT,
    }
}

fn code(ch: u8) -> Option<u8> {
    Some(match ch {
        b'K' => KO,
        b'W' => WH,
        b'w' => WL,
        b'G' => GR,
        b'g' => GD,
        b'P' => SK,
        b'p' => SP,
        b'N' => NV,
        b'Y' => GO,
        b'O' => OR,
        b'R' => RD,
        b'r' => RK,
        b'L' => BL,
        b'X' => XS,
        b'V' => VI,
        b'v' => VD,
        b'E' => EW,
        _ => return None,
    })
}

/// Rows 0-7.
const HALO: [&str; 8] = [
    "..............R..............",
    ".............rRr.............",
    "...........rRRRRRr...........",
    ".............rRr.............",
    "..........rRRRRRRRr..........",
    "........rRRr..R..rRRr........",
    "........rRr...r...rRr........",
    ".........rrrrrrrrrrr.........",
];

/// Rows 8-26: hair, tufts, pin, clip and a bare face (features are overlays).
const HEAD: [&str; 19] = [
    "..........KWWWWWWWK..........",
    ".........KWWWWWWWWWK.........",
    "........KWWWWwWWWWWWK........",
    ".......KWWWWwwWWWWWWwK.......",
    "......KWWWWwwwWWWWWwwwK......",
    ".....KWWWWWGwwwWWWWwwwwK.....",
    ".....KWWWWWGwwwwWWWwwwwwK....",
    ".....KWWWWWGWWwGwwwwwwwwK....",
    "..KGWGWWWWPPwwwPWwwwwwGwwWWK.",
    "..KwGGWWWPPPPwwPPPwwwwGGwwK..",
    "..KKGwWWPPPPPwPPPPGwwwwGGK...",
    "....KWWWGPPPPPPPPPPGwwwGK....",
    "....KWWwPPPPPPPPPPPPwwwwK....",
    "...KWWwwPPPPPPPPPPPPwwwGGK...",
    "...KWWWWPPPPPPPPPPPPgWwwGK...",
    "...KwGWwGPPPPPPPPPPgwWwGGLLK.",
    "...KwGWwwGPPPPPPPPgGWwwGLLLK.",
    "...KwgGwwwGpPPPPpGWWwGwGLLK..",
    "...KwKKwwGGGKppKGGwWWwGwGK...",
];

/// Rows 27-38: cape collar, flaps, dress and hem.
const DRESS: [&str; 12] = [
    "..KrGNNNNNNNNNNNNNNNwGGK.....",
    "..rRKNNNNNNNNNNNNNNNKwGrRK...",
    ".KrRrKNNNNNNNNNNNNNKGwGrRRrK.",
    ".KRRrNNNNNNNNNNNNNNNGGrrRRK..",
    ".KrGGNNNGNNNNGNNNNGgWGGRrK...",
    ".KrGGGGgGNWGGGNGGGGgGGwGrRK..",
    "KrRRrGGGgGGGGGGGGgrrGGGrRRK..",
    "KrRRRrrrGWGGgrrggGGgrrRRRRRK.",
    "KrrRRRRRGGGGGGGGGGGRRRRRRrRK.",
    "KGNNNKrGGrRGgGGGGGGGgrKNNGgK.",
    ".KKKKKRRrRRGrGGGGGGGRRKKKK...",
    "......KrRRRRRKKGGGGrKK.......",
];

/// The cape flaps lifted a row, alternated with DRESS rows 2-5 to flutter.
const FLAP: [&str; 4] = [
    "KrRrKNNNNNNNNNNNNNNKGwGrRRrK.",
    "KRRrNNNNNNNNNNNNNNNNGGrrRRRK.",
    ".KrGGNNNGNNNNGNNNNGgWGGRrRK..",
    ".KrGGGGgGNWGGGNGGGGgGGwGrK...",
];

/// Left leg, rows 39-49 (mirrored for the right). Thigh, white sock,
/// flared red boot cuff, white boot and toe.
const LEG: [&str; 11] = [
    "...KPPK.",
    "..KPPpK.",
    "..KPPpK.",
    "..KWWGK.",
    ".KrRWGK.",
    "KrRRRGK.",
    "KRRrRrK.",
    ".KRRWGK.",
    ".KGWWK..",
    ".KWWK...",
    "..KK....",
];

// Arm overlays (left; mirrored for the right).
const HAND_REST: [&str; 4] = [".KK.", "KPpK", "KpPK", ".KK."];
const ARM_UP: [&str; 10] = [
    ".KK...", "KPPK..", "KPpK..", ".KWK..", ".KWWK.", "..KWK.", "..KWWK", "...KWK", "...KWW", "....KW",
];
const ARM_OUT: [&str; 3] = ["KKKKKK.", "KPpWWWW", "KKKKKK."];
/// Right hand under the chin (not mirrored).
const HAND_CHIN: [&str; 3] = [".KK.", "KPpK", ".KK."];
/// A plate of Sweet Madame held up in both hands.
const FOOD: [&str; 5] = ["...KKKK...", "..KYOYOK..", ".KYYOOYYK.", "KPKWWWWKPK", ".K.KKKK.K."];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Eyes {
    Open,
    Closed,
    Happy,
    Wide,
    Angry,
    Star,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mouth {
    None,
    Smile,
    Open,
    Pout,
    Wail,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arm {
    Rest,
    Up,
    Out,
    Chin,
}

#[derive(Clone, Copy, Debug)]
struct Pose {
    cape_b: bool,
    /// Leg lifts, pixels (negative = up).
    legs: (i32, i32),
    eyes: Eyes,
    mouth: Mouth,
    blush: u8,
    /// Face turned toward the heading, pixels (signed: + is screen-right).
    look: i32,
    hdx: i32,
    hdy: i32,
    halo_dy: i32,
    /// Lean toward the heading, pixels (signed).
    lean: i32,
    arms: (Arm, Arm),
    food: bool,
    /// Seen from behind (mid-twirl).
    back: bool,
}

impl Default for Pose {
    fn default() -> Self {
        Pose {
            cape_b: false,
            legs: (0, 0),
            eyes: Eyes::Open,
            mouth: Mouth::None,
            blush: PK,
            look: 0,
            hdx: 0,
            hdy: 0,
            halo_dy: 0,
            lean: 0,
            arms: (Arm::Rest, Arm::Rest),
            food: false,
            back: false,
        }
    }
}

const GW: i32 = 44;
const GH: i32 = 58;
const CO: i32 = 7;
const RO: i32 = 3;
/// Sprite rows to the soles.
const SOLE: i32 = 50;

struct Grid(Vec<u8>);

impl Grid {
    fn new() -> Self {
        Grid(vec![0; (GW * GH) as usize])
    }

    fn idx(c: i32, r: i32) -> Option<usize> {
        let (i, j) = (c + CO, r + RO);
        (i >= 0 && j >= 0 && i < GW && j < GH).then(|| (j * GW + i) as usize)
    }

    fn get(&self, c: i32, r: i32) -> u8 {
        Self::idx(c, r).map_or(0, |k| self.0[k])
    }

    fn set(&mut self, c: i32, r: i32, v: u8) {
        if let Some(k) = Self::idx(c, r) {
            self.0[k] = v;
        }
    }

    /// Paint only on skin (face details).
    fn skin(&mut self, c: i32, r: i32, v: u8) {
        if matches!(self.get(c, r), SK | SP) {
            self.set(c, r, v);
        }
    }

    fn st(&mut self, rows: &[&str], ox: i32, oy: i32, mirror: bool) {
        for (j, row) in rows.iter().enumerate() {
            for (i, ch) in row.bytes().enumerate() {
                if let Some(v) = code(ch) {
                    let c = ox + i as i32;
                    self.set(if mirror { 28 - c } else { c }, oy + j as i32, v);
                }
            }
        }
    }

    fn shift_rows(&mut self, f: impl Fn(i32) -> i32) {
        for j in 0..GH {
            let s = f(j - RO);
            if s == 0 {
                continue;
            }
            let start = (j * GW) as usize;
            let old: Vec<u8> = self.0[start..start + GW as usize].to_vec();
            for i in 0..GW {
                let src = i - s;
                self.0[start + i as usize] = if (0..GW).contains(&src) { old[src as usize] } else { 0 };
            }
        }
    }

    /// Close any gap where colour meets empty space (layers are already outlined).
    fn outline(&mut self) {
        let src = self.0.clone();
        let at = |i: i32, j: i32| -> u8 {
            if i < 0 || j < 0 || i >= GW || j >= GH {
                0
            } else {
                src[(j * GW + i) as usize]
            }
        };
        for j in 0..GH {
            for i in 0..GW {
                if at(i, j) != 0 {
                    continue;
                }
                let solid = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(di, dj)| {
                    let v = at(i + di, j + dj);
                    v != 0 && v != KO && v != RK && v != NV && v != XS
                });
                if solid {
                    self.0[(j * GW + i) as usize] = KO;
                }
            }
        }
    }
}

fn face(g: &mut Grid, p: &Pose, hx: i32, hy: i32) {
    let fx = hx + p.look.clamp(-1, 1);
    let ey = 19 + hy;
    for (ex, right) in [(9 + fx, false), (15 + fx, true)] {
        // `outer` is the eye corner toward the hair
        let outer = if right { ex + 3 } else { ex - 1 };
        let inner = if right { ex } else { ex + 2 };
        match p.eyes {
            Eyes::Open | Eyes::Wide => {
                for c in 0..3 {
                    g.skin(ex + c, ey, VD);
                    g.skin(ex + c, ey + 1, VI);
                    g.skin(ex + c, ey + 2, VI);
                }
                g.skin(outer, ey, VD);
                g.set(ex + 1, ey + 1, EW);
                g.skin(ex + 1, ey + 2, VD);
                if p.eyes == Eyes::Wide {
                    for c in 0..3 {
                        g.skin(ex + c, ey - 1, VD);
                    }
                    g.set(ex + 1, ey, EW);
                }
            }
            Eyes::Closed => {
                g.skin(outer, ey + 1, VD);
                g.skin(ex, ey + 1, VD);
                g.skin(ex + 1, ey + 2, VD);
                g.skin(ex + 2, ey + 1, VD);
            }
            Eyes::Happy => {
                g.skin(ex, ey + 2, VD);
                g.skin(ex + 1, ey + 1, VD);
                g.skin(ex + 2, ey + 2, VD);
                g.skin(outer, ey + 2, VD);
            }
            Eyes::Angry => {
                // brows pulled down toward the nose
                g.skin(inner, ey, VD);
                for c in 0..3 {
                    g.skin(ex + c, ey + 1, VD);
                    g.skin(ex + c, ey + 2, VI);
                }
                g.skin(outer, ey + 1, VD);
            }
            Eyes::Star => {
                g.skin(ex + 1, ey, GO);
                g.skin(ex, ey + 1, GO);
                g.set(ex + 1, ey + 1, EW);
                g.skin(ex + 2, ey + 1, GO);
                g.skin(ex + 1, ey + 2, GO);
            }
        }
    }
    for c in [9, 10, 16, 17] {
        g.skin(c + fx, 23 + hy, p.blush);
    }
    let (mx, my) = (13 + fx, 24 + hy);
    match p.mouth {
        Mouth::None => g.skin(mx, my, MO),
        Mouth::Smile => {
            g.skin(mx - 1, my, MO);
            g.skin(mx, my + 1, MO);
            g.skin(mx + 1, my, MO);
        }
        Mouth::Open => {
            for c in -1..=1 {
                g.skin(mx + c, my, MI);
            }
            g.skin(mx, my + 1, MI);
        }
        Mouth::Pout => {
            g.skin(mx - 1, my + 1, MO);
            g.skin(mx, my, MO);
            g.skin(mx + 1, my + 1, MO);
        }
        Mouth::Wail => {
            for r in 0..2 {
                for c in -1..=1 {
                    g.skin(mx + c, my + r, MI);
                }
            }
        }
    }
}

fn compose(p: &Pose) -> Grid {
    let mut g = Grid::new();
    let (hx, hy) = (p.hdx, p.hdy);

    // legs and dress first; the head overlaps the collar
    g.st(&LEG, 4, 39 + p.legs.0, false);
    g.st(&LEG, 4, 39 + p.legs.1, true);
    g.st(&DRESS, 0, 27, false);
    if p.cape_b {
        g.st(&FLAP, 0, 28, false);
    }

    g.st(&HEAD, hx, 8 + hy, false);
    // the black four-point star pin, high on the right of her hair
    for (c, r) in [(21, 11), (21, 12), (20, 13), (21, 13), (22, 13), (18, 14), (19, 14), (20, 14), (21, 14), (22, 14), (23, 14), (24, 14), (20, 15), (21, 15), (22, 15), (21, 16), (21, 17)] {
        g.set(c + hx, r + hy, XS);
    }
    if p.back {
        // the back of her head: hair where the face was
        for r in 8..27 {
            for c in -2..31 {
                let v = g.get(c + hx, r + hy);
                if v == SK || v == SP {
                    g.set(c + hx, r + hy, if r > 22 { GR } else if (c + r) % 5 == 0 { WL } else { WH });
                }
            }
        }
    } else {
        face(&mut g, p, hx, hy);
    }

    for (arm, mirror) in [(p.arms.0, false), (p.arms.1, true)] {
        match arm {
            Arm::Rest => g.st(&HAND_REST, -1, 32, mirror),
            Arm::Up => g.st(&ARM_UP, -2, 19 + hy, mirror),
            Arm::Out => g.st(&ARM_OUT, -5, 30, mirror),
            Arm::Chin => g.st(&HAND_CHIN, 14 + hx, 25 + hy, false),
        }
    }
    if p.food {
        g.st(&FOOD, 9, 26 + hy, false);
    }

    g.outline();
    // The halo goes on after the outline pass so its ring stays thin.
    g.st(&HALO, hx, p.halo_dy + hy, false);
    if p.lean != 0 {
        // the head (and halo) lead into the flight, the dress follows
        let lean = p.lean;
        g.shift_rows(|r| if r < 27 { lean } else if r < 39 { lean / 2 } else { 0 });
    }
    g
}

/// Paint with sprite column 14 centred on `origin.x` and the soles on `origin.y`.
fn paint_grid(p: &Painter, g: &Grid, origin: Pos2, s: f32, alpha: f32) {
    let x0 = origin.x - 14.5 * s;
    for j in 0..GH {
        let top = origin.y + (j - RO - SOLE) as f32 * s;
        let mut i = 0;
        while i < GW {
            let v = g.0[(j * GW + i) as usize];
            if v == 0 {
                i += 1;
                continue;
            }
            let start = i;
            while i < GW && g.0[(j * GW + i) as usize] == v {
                i += 1;
            }
            let (c0, c1) = (start - CO, i - CO);
            let c = color(v);
            let c = if alpha < 1.0 { rgba(c, alpha) } else { c };
            p.rect_filled(Rect::from_min_max(pos2(x0 + c0 as f32 * s, top), pos2(x0 + c1 as f32 * s, top + s)), 0.0, c);
        }
    }
}

/// Particle bitmaps: Y gold, E white, C cyan, R red, D crumb.
fn paint_bitmap(p: &Painter, rows: &[&str], center: Pos2, s: f32, alpha: f32) {
    let h = rows.len() as f32;
    let w = rows.iter().map(|r| r.len()).max().unwrap_or(0) as f32;
    let o = center - vec2(w * s / 2.0, h * s / 2.0);
    for (j, r) in rows.iter().enumerate() {
        for (i, ch) in r.bytes().enumerate() {
            let c = match ch {
                b'Y' => hex(0xf2c14e),
                b'E' => Color32::WHITE,
                b'C' => hex(0x9fe6ff),
                b'R' => hex(0xe0473a),
                b'D' => hex(0xc98a3a),
                _ => continue,
            };
            p.rect_filled(Rect::from_min_size(o + vec2(i as f32 * s, j as f32 * s), vec2(s, s)), 0.0, rgba(c, alpha));
        }
    }
}

// ---------------------------------------------------------------------------
// Developer helpers (cargo feature "screenshot")
// ---------------------------------------------------------------------------

#[cfg(feature = "screenshot")]
impl Paimon {
    /// Simulate a right-click at `at` (HAEM_SCENE=goal=X:Y).
    pub fn debug_goal(&mut self, ctx: &egui::Context, at: Pos2) {
        let ps = perches(ctx);
        let to = Station::Air(pos2(at.x, at.y + px(25.0)));
        self.fly_to(to, &ps, Then::Visit, 1.25);
    }
}

#[cfg(feature = "screenshot")]
pub fn debug_sheet(ctx: &egui::Context) {
    let mut k = Paimon::new();
    let idle = |kind: Idle| Act::Idle { kind, dur: 3.0 };
    let air = Station::Air(Pos2::ZERO);
    let cases: Vec<(&str, Act, f32)> = vec![
        ("float", idle(Idle::Float), 0.0),
        ("look", idle(Idle::Look), 0.0),
        ("twirl side", idle(Idle::Twirl), 0.09),
        ("twirl back", idle(Idle::Twirl), 0.17),
        ("snack", idle(Idle::Snack), 0.3),
        ("snack chew", idle(Idle::Snack), 0.9),
        ("think", idle(Idle::Think), 0.5),
        ("cheer", idle(Idle::Cheer), 0.2),
        ("wave", idle(Idle::Wave), 0.1),
        ("doze", idle(Idle::Doze), 1.0),
        ("sit", idle(Idle::Sit), 0.1),
        ("fly", Act::Fly { from: Pos2::ZERO, ctrl: Pos2::ZERO, to: air, dur: 1.0, then: Then::Rest }, 0.5),
        ("tag", Act::Tag { dur: 3.0 }, 0.5),
        ("angry", Act::Angry { dur: 2.0 }, 0.1),
        ("visit", Act::Visit { dur: 3.0 }, 0.2),
        ("teleport", Act::Teleport { to: air, moved: false }, 0.1),
    ];
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("paimon-sheet")));
    painter.rect_filled(screen, 0.0, hex(0x2b3a4f));
    let s = 3.0;
    let cols = 8usize;
    for (i, (name, act, t)) in cases.iter().enumerate() {
        k.act = *act;
        k.t = *t;
        k.blink_t = 0.0;
        k.facing = 1.0;
        let mut pose = k.pose();
        if name == &"teleport" {
            pose.eyes = Eyes::Closed;
        }
        let g = compose(&pose);
        let cell = pos2(screen.left() + 75.0 + (i % cols) as f32 * 142.0, screen.top() + 150.0 + (i / cols) as f32 * 200.0);
        let alpha = if name == &"teleport" { 0.45 } else { 1.0 };
        paint_grid(&painter, &g, cell, s, alpha);
        painter.text(pos2(cell.x, cell.y + 14.0), Align2::CENTER_CENTER, *name, egui::FontId::proportional(12.0), Color32::from_white_alpha(190));
    }
}
