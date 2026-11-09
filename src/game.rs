//! 時空を渡って歴史の現場を歩くゲーム。
//!
//! 世界地図で時代と地域を選んで転移し、その時代の街のミニマップを移動して
//! 人物に会い、出来事を観察する。出会った人物と観察した出来事は図鑑
//! （人物一覧表・年表・出来事集）に記録される。
use crate::{
    gfx::{Font, Gfx, Texture},
    history::*,
    platform, Ev,
};

// ------------------------------------------------------------------ 色
const NIGHT: [f32; 4] = [0.05, 0.08, 0.14, 1.];
const PANEL: [f32; 4] = [0.09, 0.13, 0.22, 0.97];
const PANEL2: [f32; 4] = [0.13, 0.19, 0.31, 1.];
const EDGE: [f32; 4] = [0.30, 0.40, 0.56, 1.];
const GOLD: [f32; 4] = [0.90, 0.75, 0.42, 1.];
const CYAN: [f32; 4] = [0.50, 0.86, 0.94, 1.];
const TEXT: [f32; 4] = [0.95, 0.93, 0.87, 1.];
const MUTED: [f32; 4] = [0.66, 0.71, 0.78, 1.];
const LOCKED: [f32; 4] = [0.36, 0.42, 0.52, 1.];
const INK: [f32; 4] = [0.20, 0.15, 0.10, 1.];
const INK_SOFT: [f32; 4] = [0.36, 0.29, 0.21, 1.];
const WATER: [f32; 4] = [0.56, 0.72, 0.78, 1.];

/// 時代ごとの見分けの色
fn era_color(i: usize) -> [f32; 4] {
    const C: [[f32; 4]; 7] = [
        [0.85, 0.42, 0.35, 1.],
        [0.90, 0.66, 0.30, 1.],
        [0.80, 0.52, 0.78, 1.],
        [0.40, 0.76, 0.56, 1.],
        [0.48, 0.66, 0.92, 1.],
        [0.36, 0.60, 0.95, 1.],
        [0.86, 0.80, 0.42, 1.],
    ];
    C[i % C.len()]
}

fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn alpha(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

// ------------------------------------------------------------------ 状態
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    /// 世界地図で時代を選ぶ
    Atlas,
    /// 転移の演出
    Shift,
    /// 時代のミニマップ
    Field,
    /// 図鑑
    Codex,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tab {
    People,
    Timeline,
    Events,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Entry {
    Person(usize),
    Event(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Overlay {
    None,
    Meet(usize, usize),
    Observe(usize),
    Detail(Entry),
    Help,
}

#[derive(Clone, Copy, Debug)]
enum Act {
    Atlas,
    Codex,
    PickEra(usize),
    Shift(usize, Option<usize>),
    Goto(usize),
    Meet(usize),
    Observe(usize),
    Next,
    Prev,
    Close,
    Tab(Tab),
    Filter(Option<usize>),
    Detail(Entry),
    Help,
    Voice,
    Export,
    Import,
    Reveal,
    Search,
}

struct Button {
    rect: [f32; 4],
    act: Act,
}

/// 旅の記録（保存される）
#[derive(Clone, Debug, PartialEq)]
struct Journal {
    era: usize,
    place: usize,
    visited: Vec<bool>,
    met: Vec<bool>,
    seen: Vec<bool>,
}

const MAGIC: &[u8; 8] = b"YYHIST\x02\0";

impl Journal {
    fn new() -> Self {
        Self {
            era: 0,
            place: ERAS[0].start,
            visited: vec![false; ERAS.len()],
            met: vec![false; PEOPLE.len()],
            seen: vec![false; EVENTS.len()],
        }
    }
    fn bytes(&self) -> Vec<u8> {
        let mut b = MAGIC.to_vec();
        b.push(self.era as u8);
        b.push(self.place as u8);
        let mut v = 0u8;
        for (i, x) in self.visited.iter().enumerate() {
            if *x {
                v |= 1 << i;
            }
        }
        b.push(v);
        let ids = |list: &mut Vec<u8>, ids: Vec<&str>| {
            list.extend_from_slice(&(ids.len() as u16).to_le_bytes());
            for id in ids {
                list.push(id.len() as u8);
                list.extend_from_slice(id.as_bytes());
            }
        };
        ids(
            &mut b,
            PEOPLE
                .iter()
                .zip(&self.met)
                .filter(|(_, m)| **m)
                .map(|(p, _)| p.id)
                .collect(),
        );
        ids(
            &mut b,
            EVENTS
                .iter()
                .zip(&self.seen)
                .filter(|(_, m)| **m)
                .map(|(e, _)| e.id)
                .collect(),
        );
        let hash = checksum(&b);
        b.extend_from_slice(&hash.to_le_bytes());
        b
    }
    fn read(b: &[u8]) -> Result<Self, String> {
        let invalid =
            || "YYHistoryの記録として読み込めません。現在の記録はそのままです。".to_string();
        if b.len() < 19 || b.len() > 65536 || &b[..8] != MAGIC {
            return Err(invalid());
        }
        let end = b.len() - 4;
        if checksum(&b[..end]) != u32::from_le_bytes(b[end..].try_into().unwrap()) {
            return Err(invalid());
        }
        let era = b[8] as usize;
        let place = b[9] as usize;
        if era >= ERAS.len() || place >= ERAS[era].places.len() {
            return Err(invalid());
        }
        let visited = (0..ERAS.len()).map(|i| b[10] & (1 << i) != 0).collect();
        let mut at = 11;
        let mut list = |find: &dyn Fn(&str) -> Option<usize>, n_all: usize| {
            if at + 2 > end {
                return Err(invalid());
            }
            let n = u16::from_le_bytes([b[at], b[at + 1]]) as usize;
            at += 2;
            let mut out = vec![false; n_all];
            for _ in 0..n {
                if at >= end {
                    return Err(invalid());
                }
                let len = b[at] as usize;
                at += 1;
                if at + len > end {
                    return Err(invalid());
                }
                let id = std::str::from_utf8(&b[at..at + len]).map_err(|_| invalid())?;
                at += len;
                let i = find(id).ok_or_else(invalid)?;
                if std::mem::replace(&mut out[i], true) {
                    return Err(invalid());
                }
            }
            Ok(out)
        };
        let met = list(&person_index, PEOPLE.len())?;
        let seen = list(&event_index, EVENTS.len())?;
        if at != end {
            return Err(invalid());
        }
        Ok(Self {
            era,
            place,
            visited,
            met,
            seen,
        })
    }
}

fn checksum(b: &[u8]) -> u32 {
    b.iter()
        .fold(2166136261u32, |h, v| (h ^ *v as u32).wrapping_mul(16777619))
}

/// ミニマップ上の移動
struct Travel {
    points: Vec<(f32, f32)>,
    target: usize,
    at: usize,
    t: f32,
}

/// 縦にずらせる領域
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scroll {
    Atlas = 0,
    Panel = 1,
    Codex = 2,
    Overlay = 3,
}

pub struct Game {
    gfx: Gfx,
    world: Texture,
    journal: Journal,
    mode: Mode,
    overlay: Overlay,
    buttons: Vec<Button>,
    scroll_areas: Vec<([f32; 4], Scroll)>,
    scroll: [f32; 4],
    scroll_max: [f32; 4],
    mouse: (f32, f32),
    down: Option<(f32, f32)>,
    moved: f32,
    clock: f32,
    toast: String,
    toast_left: f32,
    query: String,
    searching: bool,
    tab: Tab,
    filter: Option<usize>,
    reveal: bool,
    pick: usize,
    shift_t: f32,
    shift_to: (usize, Option<usize>),
    avatar: (f32, f32),
    travel: Option<Travel>,
    voice: bool,
    dirty: bool,
    save_in: f32,
}

impl Game {
    pub fn new(gfx: Gfx) -> Self {
        let image = image::load_from_memory(include_bytes!("../assets/world.png"))
            .expect("offline map")
            .into_rgba8();
        let (w, h) = image.dimensions();
        let data: Vec<u32> = image
            .as_raw()
            .chunks_exact(4)
            .map(|p| u32::from_le_bytes([p[0], p[1], p[2], p[3]]))
            .collect();
        let world = gfx.create_texture(w, h, &data).expect("world texture");
        let journal = Journal::new();
        let p = &ERAS[0].places[ERAS[0].start];
        let mut g = Self {
            gfx,
            world,
            journal,
            mode: Mode::Atlas,
            overlay: Overlay::None,
            buttons: vec![],
            scroll_areas: vec![],
            scroll: [0.; 4],
            scroll_max: [0.; 4],
            mouse: (0., 0.),
            down: None,
            moved: 0.,
            clock: 0.,
            toast: String::new(),
            toast_left: 0.,
            query: String::new(),
            searching: false,
            tab: Tab::People,
            filter: None,
            reveal: false,
            pick: 0,
            shift_t: 0.,
            shift_to: (0, None),
            avatar: (p.x, p.y),
            travel: None,
            voice: platform::voice_enabled(),
            dirty: false,
            save_in: 0.,
        };
        platform::load_slot(0);
        g.notify("行き先の時代を選び、「転移する」でその時代へ渡ろう。");
        g
    }

    // -------------------------------------------------------------- 外部とのつなぎ目
    #[cfg_attr(windows, allow(dead_code))]
    pub fn set_query(&mut self, query: String) {
        self.query = query.chars().take(60).collect();
        self.scroll[Scroll::Codex as usize] = 0.;
    }
    #[cfg_attr(windows, allow(dead_code))]
    pub fn search_visible(&self) -> bool {
        self.mode == Mode::Codex && self.overlay == Overlay::None
    }
    /// 検索欄の位置（UI 座標）
    #[cfg_attr(windows, allow(dead_code))]
    pub fn search_box(&self) -> [f32; 4] {
        let w = self.gfx.width as f32;
        let bw = if self.gfx.compact { 200. } else { 260. };
        [w - bw - 20., 74., bw, 34.]
    }
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn state_text(&self) -> String {
        format!(
            "YYHistory {:?} era={} place={} met={} seen={}",
            self.mode,
            ERAS[self.journal.era].id,
            ERAS[self.journal.era].places[self.journal.place].id,
            self.met_count(None),
            self.seen_count(None)
        )
    }
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn crash_bytes(&self) -> Option<Vec<u8>> {
        Some(self.journal.bytes())
    }
    pub fn notify(&mut self, s: &str) {
        self.toast = s.to_string();
        self.toast_left = 4.5;
    }
    pub fn apply_loaded(&mut self, b: &[u8]) {
        match Journal::read(b) {
            Ok(j) => {
                self.journal = j;
                self.pick = self.journal.era;
                self.travel = None;
                let p = &ERAS[self.journal.era].places[self.journal.place];
                self.avatar = (p.x, p.y);
                self.mode = Mode::Atlas;
                self.overlay = Overlay::None;
                self.notify("旅の記録を読み込みました。");
            }
            Err(e) => self.notify(&e),
        }
    }
    fn save(&mut self) {
        platform::save_slot(
            0,
            &self.journal.bytes(),
            &format!(
                "YYHistory 人物{}/{} 出来事{}/{}",
                self.met_count(None),
                PEOPLE.len(),
                self.seen_count(None),
                EVENTS.len()
            ),
        );
        self.dirty = false;
    }
    fn change(&mut self) {
        self.dirty = true;
        self.save_in = 0.8;
    }

    fn met_count(&self, era: Option<usize>) -> usize {
        PEOPLE
            .iter()
            .zip(&self.journal.met)
            .filter(|(p, m)| **m && era.is_none_or(|e| p.era == e))
            .count()
    }
    fn seen_count(&self, era: Option<usize>) -> usize {
        EVENTS
            .iter()
            .zip(&self.journal.seen)
            .filter(|(e, m)| **m && era.is_none_or(|x| e.era == x))
            .count()
    }

    #[cfg_attr(windows, allow(dead_code))]
    pub fn hit_kind(&self, x: f32, y: f32) -> u32 {
        let (x, y) = (x / self.gfx.ui, y / self.gfx.ui);
        if self.buttons.iter().any(|b| inside(b.rect, x, y)) {
            return 1;
        }
        if self.scroll_areas.iter().any(|(r, _)| inside(*r, x, y)) {
            3
        } else {
            1
        }
    }

    // -------------------------------------------------------------- 1フレーム
    pub fn frame(&mut self, evs: &[Ev], dt: f32, _focused: bool) -> bool {
        if let Some(b) = platform::take_loaded() {
            self.apply_loaded(&b);
        }
        for n in platform::take_notes() {
            self.notify(&n);
        }
        self.clock = (self.clock + dt).min(1e7);
        self.toast_left -= dt;
        for e in evs {
            self.handle(*e);
        }
        self.update(dt);
        if self.dirty {
            self.save_in -= dt;
            if self.save_in <= 0. {
                self.save();
            }
        }
        self.render();
        false
    }

    fn update(&mut self, dt: f32) {
        if self.mode == Mode::Shift {
            self.shift_t += dt;
            if self.shift_t >= 1.6 {
                self.arrive();
            }
        }
        if let Some(tr) = &mut self.travel {
            // 地図の横幅は高さの1.6倍。見た目の速さを揃える
            let mut left = dt * 0.32;
            while left > 0. && tr.at + 1 < tr.points.len() {
                let a = tr.points[tr.at];
                let b = tr.points[tr.at + 1];
                let len = (((b.0 - a.0) * 1.6).powi(2) + (b.1 - a.1).powi(2))
                    .sqrt()
                    .max(1e-4);
                let need = (1. - tr.t) * len;
                if left >= need {
                    left -= need;
                    tr.at += 1;
                    tr.t = 0.;
                } else {
                    tr.t += left / len;
                    left = 0.;
                }
            }
            if tr.at + 1 >= tr.points.len() {
                let target = tr.target;
                self.avatar = *tr.points.last().unwrap();
                self.travel = None;
                self.journal.place = target;
                self.scroll[Scroll::Panel as usize] = 0.;
                self.change();
                let place = &ERAS[self.journal.era].places[target];
                let people = self.people_here().len();
                let events = self.events_here().len();
                self.notify(&format!(
                    "{}に着いた。人物{}人・出来事{}件",
                    place.name, people, events
                ));
            } else {
                let a = tr.points[tr.at];
                let b = tr.points[tr.at + 1];
                self.avatar = (a.0 + (b.0 - a.0) * tr.t, a.1 + (b.1 - a.1) * tr.t);
            }
        }
    }

    /// 転移を終えて時代に着く
    fn arrive(&mut self) {
        let (era, place) = self.shift_to;
        self.journal.era = era;
        self.journal.place = place.unwrap_or(ERAS[era].start);
        self.journal.visited[era] = true;
        let p = &ERAS[era].places[self.journal.place];
        self.avatar = (p.x, p.y);
        self.travel = None;
        self.mode = Mode::Field;
        self.scroll[Scroll::Panel as usize] = 0.;
        self.change();
        self.notify(&format!(
            "{}・{}に到着。地図の場所を選んで移動しよう。",
            ERAS[era].years, ERAS[era].name
        ));
        if self.voice {
            platform::speak(ERAS[era].intro);
        }
    }

    fn people_here(&self) -> Vec<usize> {
        let era = &ERAS[self.journal.era];
        let id = era.places[self.journal.place].id;
        (0..PEOPLE.len())
            .filter(|i| PEOPLE[*i].era == self.journal.era && PEOPLE[*i].place == id)
            .collect()
    }
    fn events_here(&self) -> Vec<usize> {
        let era = &ERAS[self.journal.era];
        let id = era.places[self.journal.place].id;
        let mut v: Vec<usize> = (0..EVENTS.len())
            .filter(|i| EVENTS[*i].era == self.journal.era && EVENTS[*i].place == id)
            .collect();
        v.sort_by_key(|i| event_key(*i));
        v
    }

    /// 道をたどって場所へ向かう（幅優先探索）
    fn goto(&mut self, target: usize) {
        if self.mode != Mode::Field {
            return;
        }
        let era = &ERAS[self.journal.era];
        let from = match &self.travel {
            Some(t) => t.target,
            None => self.journal.place,
        };
        if target == from && self.travel.is_none() {
            return;
        }
        let n = era.places.len();
        let mut prev = vec![usize::MAX; n];
        let mut queue = std::collections::VecDeque::from([from]);
        prev[from] = from;
        while let Some(a) = queue.pop_front() {
            for &(x, y) in era.roads {
                for (p, q) in [(x, y), (y, x)] {
                    if p == a && prev[q] == usize::MAX {
                        prev[q] = a;
                        queue.push_back(q);
                    }
                }
            }
        }
        if prev[target] == usize::MAX {
            return;
        }
        let mut path = vec![target];
        while *path.last().unwrap() != from {
            path.push(prev[*path.last().unwrap()]);
        }
        path.reverse();
        let mut points = vec![self.avatar];
        points.extend(
            path.iter()
                .skip(1)
                .map(|i| (era.places[*i].x, era.places[*i].y)),
        );
        if points.len() == 1 {
            points.push((era.places[target].x, era.places[target].y));
        }
        self.travel = Some(Travel {
            points,
            target,
            at: 0,
            t: 0.,
        });
    }

    fn meet(&mut self, i: usize) {
        let p = &PEOPLE[i];
        if self.mode != Mode::Field
            || self.travel.is_some()
            || p.era != self.journal.era
            || p.place != ERAS[self.journal.era].places[self.journal.place].id
        {
            self.notify("その人物のいる場所まで行こう。");
            return;
        }
        self.overlay = Overlay::Meet(i, 0);
        if !self.journal.met[i] {
            self.journal.met[i] = true;
            self.notify(&format!("{} を人物一覧表に登録しました。", p.name));
            self.change();
        }
        if self.voice {
            platform::speak(p.lines[0]);
        }
    }

    fn observe(&mut self, i: usize) {
        let e = &EVENTS[i];
        if self.mode != Mode::Field
            || self.travel.is_some()
            || e.era != self.journal.era
            || e.place != ERAS[self.journal.era].places[self.journal.place].id
        {
            self.notify("その出来事が起きた場所まで行こう。");
            return;
        }
        self.overlay = Overlay::Observe(i);
        self.scroll[Scroll::Overlay as usize] = 0.;
        if !self.journal.seen[i] {
            self.journal.seen[i] = true;
            self.notify(&format!("「{}」を出来事集と年表に記録しました。", e.title));
            self.change();
        }
        if self.voice {
            platform::speak(&format!("{}。{}。{}", e.date, e.title, e.body.join("")));
        }
    }

    fn act(&mut self, a: Act) {
        match a {
            Act::Atlas => {
                self.mode = Mode::Atlas;
                self.overlay = Overlay::None;
                self.travel = None;
                self.pick = self.journal.era;
            }
            Act::Codex => {
                self.mode = Mode::Codex;
                self.overlay = Overlay::None;
                self.scroll[Scroll::Codex as usize] = 0.;
            }
            Act::PickEra(i) => {
                self.pick = i;
                if self.voice {
                    platform::speak(ERAS[i].intro);
                }
            }
            Act::Shift(era, place) => {
                self.overlay = Overlay::None;
                self.shift_to = (era, place);
                self.shift_t = 0.;
                self.mode = Mode::Shift;
                self.travel = None;
                platform::speak("");
            }
            Act::Goto(p) => self.goto(p),
            Act::Meet(i) => self.meet(i),
            Act::Observe(i) => self.observe(i),
            Act::Next => match self.overlay {
                Overlay::Meet(i, page) => {
                    if page + 1 < PEOPLE[i].lines.len() {
                        self.overlay = Overlay::Meet(i, page + 1);
                        if self.voice {
                            platform::speak(PEOPLE[i].lines[page + 1]);
                        }
                    } else {
                        self.overlay = Overlay::None;
                        platform::speak("");
                    }
                }
                Overlay::None => (),
                _ => {
                    self.overlay = Overlay::None;
                    platform::speak("");
                }
            },
            Act::Prev => {
                if let Overlay::Meet(i, page) = self.overlay {
                    self.overlay = Overlay::Meet(i, page.saturating_sub(1));
                }
            }
            Act::Close => {
                self.overlay = Overlay::None;
                platform::speak("");
            }
            Act::Tab(t) => {
                self.tab = t;
                self.scroll[Scroll::Codex as usize] = 0.;
            }
            Act::Filter(f) => {
                self.filter = f;
                self.scroll[Scroll::Codex as usize] = 0.;
            }
            Act::Detail(e) => {
                self.overlay = Overlay::Detail(e);
                self.scroll[Scroll::Overlay as usize] = 0.;
            }
            Act::Help => {
                self.overlay = Overlay::Help;
                self.scroll[Scroll::Overlay as usize] = 0.;
            }
            Act::Voice => {
                self.voice = !self.voice;
                platform::save_voice(self.voice);
                if !self.voice {
                    platform::speak("");
                }
            }
            Act::Export => {
                self.save();
                platform::export_slot(0);
            }
            Act::Import => platform::import_file(),
            Act::Reveal => self.reveal = !self.reveal,
            Act::Search => self.searching = true,
        }
    }

    /// 矢印キー：その向きにある、道でつながった場所へ
    fn step(&mut self, dx: f32, dy: f32) {
        if self.mode != Mode::Field || self.overlay != Overlay::None {
            return;
        }
        let era = &ERAS[self.journal.era];
        let from = self
            .travel
            .as_ref()
            .map_or(self.journal.place, |t| t.target);
        let a = &era.places[from];
        let mut best = None;
        let mut score = 0.3;
        for &(x, y) in era.roads {
            let other = if x == from {
                y
            } else if y == from {
                x
            } else {
                continue;
            };
            let b = &era.places[other];
            let (vx, vy) = ((b.x - a.x) * 1.6, b.y - a.y);
            let l = (vx * vx + vy * vy).sqrt().max(1e-4);
            let s = (vx * dx + vy * dy) / l;
            if s > score {
                score = s;
                best = Some(other);
            }
        }
        if let Some(b) = best {
            self.goto(b);
        }
    }

    fn handle(&mut self, e: Ev) {
        let ui = self.gfx.ui;
        match e {
            Ev::Resize(w, h) => self.gfx.resize(w.max(1), h.max(1)),
            Ev::Compact(c) => self.gfx.set_compact(c),
            Ev::Down(0, x, y) => {
                self.mouse = (x / ui, y / ui);
                self.down = Some(self.mouse);
                self.moved = 0.;
            }
            Ev::MouseMove(x, y) => {
                let (x, y) = (x / ui, y / ui);
                if let Some(start) = self.down {
                    self.moved += (x - self.mouse.0).abs() + (y - self.mouse.1).abs();
                    // マウスでのドラッグは一覧のスクロールに使う
                    if let Some(s) = self.scroll_at(start) {
                        self.scroll_by(s, self.mouse.1 - y);
                    }
                }
                self.mouse = (x, y);
            }
            Ev::Up(0, x, y) => {
                let (x, y) = (x / ui, y / ui);
                if self.down.take().is_some() && self.moved < 8. {
                    self.click(x, y);
                }
            }
            Ev::Wheel(d) => {
                if let Some(s) = self.scroll_at(self.mouse) {
                    self.scroll_by(s, -d * 60.);
                }
            }
            Ev::Scroll(d) => {
                if let Some(s) = self.scroll_at(self.mouse) {
                    self.scroll_by(s, -d / ui);
                }
            }
            Ev::Char(c) => {
                if self.searching && !c.is_control() && self.query.chars().count() < 60 {
                    self.query.push(c);
                    self.scroll[Scroll::Codex as usize] = 0.;
                }
            }
            Ev::Key(k, true) => {
                if self.searching {
                    match k {
                        8 => {
                            self.query.pop();
                        }
                        13 | 27 => self.searching = false,
                        _ => (),
                    }
                    return;
                }
                match k {
                    27 => {
                        if self.overlay != Overlay::None {
                            self.act(Act::Close);
                        } else if self.mode == Mode::Codex {
                            self.mode = if self.journal.visited.iter().any(|v| *v) {
                                Mode::Field
                            } else {
                                Mode::Atlas
                            };
                        } else if self.mode == Mode::Field {
                            self.act(Act::Atlas);
                        }
                    }
                    13 | 32 => {
                        if self.overlay != Overlay::None {
                            self.act(Act::Next);
                        } else if self.mode == Mode::Atlas {
                            self.act(Act::Shift(self.pick, None));
                        }
                    }
                    37 | 65 => {
                        if let Overlay::Meet(..) = self.overlay {
                            self.act(Act::Prev);
                        } else if self.mode == Mode::Atlas {
                            self.pick = (self.pick + ERAS.len() - 1) % ERAS.len();
                        } else {
                            self.step(-1., 0.);
                        }
                    }
                    39 | 68 => {
                        if let Overlay::Meet(..) = self.overlay {
                            self.act(Act::Next);
                        } else if self.mode == Mode::Atlas {
                            self.pick = (self.pick + 1) % ERAS.len();
                        } else {
                            self.step(1., 0.);
                        }
                    }
                    38 | 87 => {
                        if self.mode == Mode::Atlas {
                            self.pick = (self.pick + ERAS.len() - 1) % ERAS.len();
                        } else {
                            self.step(0., -1.);
                        }
                    }
                    40 | 83 => {
                        if self.mode == Mode::Atlas {
                            self.pick = (self.pick + 1) % ERAS.len();
                        } else {
                            self.step(0., 1.);
                        }
                    }
                    0x71 => self.act(Act::Codex),
                    0x72 => self.act(Act::Help),
                    0x7a => self.gfx.toggle_fullscreen(),
                    _ => (),
                }
            }
            _ => (),
        }
    }

    fn scroll_at(&self, p: (f32, f32)) -> Option<Scroll> {
        self.scroll_areas
            .iter()
            .rev()
            .find(|(r, _)| inside(*r, p.0, p.1))
            .map(|(_, s)| *s)
    }
    fn scroll_by(&mut self, s: Scroll, d: f32) {
        let i = s as usize;
        self.scroll[i] = (self.scroll[i] + d).clamp(0., self.scroll_max[i].max(0.));
    }

    fn click(&mut self, x: f32, y: f32) {
        if let Some(act) = self
            .buttons
            .iter()
            .rev()
            .find(|b| inside(b.rect, x, y))
            .map(|b| b.act)
        {
            self.searching = false;
            self.act(act);
            return;
        }
        self.searching = false;
        if self.overlay != Overlay::None {
            // 会話の枠の外を押したら次へ進む
            if let Overlay::Meet(..) = self.overlay {
                self.act(Act::Next);
            }
            return;
        }
        if self.mode == Mode::Field {
            let r = self.map_rect();
            if inside(r, x, y) {
                let era = &ERAS[self.journal.era];
                let mut best = None;
                let mut dist = 34.;
                for (i, p) in era.places.iter().enumerate() {
                    let (px, py) = (r[0] + p.x * r[2], r[1] + p.y * r[3]);
                    let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
                    if d < dist {
                        dist = d;
                        best = Some(i);
                    }
                }
                if let Some(i) = best {
                    self.goto(i);
                }
            }
        }
    }

    // -------------------------------------------------------------- 配置
    fn top(&self) -> f32 {
        if self.gfx.compact {
            50.
        } else {
            60.
        }
    }
    fn panel_w(&self) -> f32 {
        let w = self.gfx.width as f32;
        if self.gfx.compact {
            (w * 0.36).clamp(300., 380.)
        } else {
            (w * 0.27).clamp(340., 420.)
        }
    }
    /// ミニマップの紙面（縦横比 1.6:1）
    fn map_rect(&self) -> [f32; 4] {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let top = self.top();
        let aw = w - self.panel_w() - 28.;
        let ah = h - top - 52.;
        let mw = aw.min(ah * 1.6);
        let mh = mw / 1.6;
        [14. + (aw - mw) / 2., top + 12. + (ah - mh) / 2., mw, mh]
    }
    fn atlas_list_w(&self) -> f32 {
        if self.gfx.compact {
            300.
        } else {
            360.
        }
    }
    /// 世界地図の表示範囲（経度・緯度）を画面に合わせる
    fn atlas_view(&self) -> (f32, f32, f32) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let x0 = self.atlas_list_w();
        let top = self.top();
        let card = if self.gfx.compact { 150. } else { 176. };
        let aw = w - x0;
        let ah = h - top - card;
        let scale = (aw / 300.).min(ah / 88.);
        // 経度 -125〜175、緯度 70〜-18 を中心に収める
        let cx = x0 + aw / 2. - 25. * scale;
        let cy = top + ah / 2. + 26. * scale;
        (scale, cx, cy)
    }
    fn atlas_project(&self, lon: f32, lat: f32) -> (f32, f32) {
        let (s, cx, cy) = self.atlas_view();
        (cx + lon * s, cy - lat * s)
    }

    // -------------------------------------------------------------- 描画の部品
    fn text(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: [f32; 4]) {
        self.gfx.text(s, x, y, w, h, f, c);
    }
    fn button(&mut self, s: &str, r: [f32; 4], a: Act, active: bool) {
        let [x, y, w, h] = r;
        let hover = inside(r, self.mouse.0, self.mouse.1);
        let fill = if active {
            mix(GOLD, [0.6, 0.45, 0.2, 1.], 0.15)
        } else if hover {
            [0.20, 0.28, 0.43, 1.]
        } else {
            PANEL2
        };
        self.gfx.fill_round(x, y, x + w, y + h, 6., fill);
        self.gfx.stroke_round(
            x + 0.5,
            y + 0.5,
            x + w - 0.5,
            y + h - 0.5,
            6.,
            1.,
            if active { GOLD } else { EDGE },
        );
        self.text(
            s,
            x + 4.,
            y + 1.,
            w - 8.,
            h - 2.,
            Font::NormalCenter,
            if active { NIGHT } else { TEXT },
        );
        self.buttons.push(Button { rect: r, act: a });
    }
    /// 人物の肖像の代わりの紋章（名前の頭文字）
    fn medallion(&mut self, name: &str, x: f32, y: f32, r: f32, c: [f32; 4], known: bool) {
        self.gfx
            .fill_circle(x, y, r + 2., if known { GOLD } else { LOCKED });
        self.gfx.fill_circle(
            x,
            y,
            r,
            if known {
                mix(NIGHT, c, 0.55)
            } else {
                [0.14, 0.18, 0.26, 1.]
            },
        );
        self.gfx.stroke_circle(x, y, r - 3., 1., alpha(TEXT, 0.25));
        let ch: String = if known {
            name.chars().next().unwrap_or('？').to_string()
        } else {
            "？".to_string()
        };
        let font = if r > 26. {
            Font::BigCenter
        } else {
            Font::NormalCenter
        };
        self.text(
            &ch,
            x - r,
            y - r,
            r * 2.,
            r * 2.,
            font,
            if known { TEXT } else { MUTED },
        );
    }
    fn panel_box(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.gfx.fill_round(x0, y0, x1, y1, 10., PANEL);
        self.gfx
            .stroke_round(x0 + 0.5, y0 + 0.5, x1 - 0.5, y1 - 0.5, 10., 1., EDGE);
        // 角の飾り
        let (l, t) = (18., 3.);
        for (cx, cy, sx, sy) in [
            (x0, y0, 1f32, 1f32),
            (x1, y0, -1., 1.),
            (x0, y1, 1., -1.),
            (x1, y1, -1., -1.),
        ] {
            let (ax, ay) = (cx + sx * 4., cy + sy * 4.);
            let (bx, by) = (ax + sx * l, ay + sy * t);
            self.gfx
                .fill_rect(ax.min(bx), ay.min(by), ax.max(bx), ay.max(by), GOLD);
            let (bx, by) = (ax + sx * t, ay + sy * l);
            self.gfx
                .fill_rect(ax.min(bx), ay.min(by), ax.max(bx), ay.max(by), GOLD);
        }
    }

    // -------------------------------------------------------------- 描画
    fn render(&mut self) {
        self.buttons.clear();
        self.scroll_areas.clear();
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        self.gfx.begin(NIGHT);
        self.gfx.set_screen_xform();
        match self.mode {
            Mode::Atlas | Mode::Shift => self.atlas_geometry(),
            Mode::Field => self.field_geometry(),
            Mode::Codex => (),
        }
        self.gfx.flush();
        if !self.gfx.d2d_begin() {
            return;
        }
        match self.mode {
            Mode::Atlas => self.atlas_ui(),
            Mode::Shift => {
                self.atlas_ui();
                self.buttons.clear();
                self.scroll_areas.clear();
            }
            Mode::Field => self.field_ui(),
            Mode::Codex => self.codex_ui(),
        }
        self.top_bar();
        if self.overlay != Overlay::None || self.mode == Mode::Shift {
            self.buttons.clear();
            self.scroll_areas.clear();
        }
        match self.overlay {
            Overlay::None => (),
            Overlay::Meet(i, page) => self.meet_ui(i, page),
            Overlay::Observe(i) => self.observe_ui(i),
            Overlay::Detail(e) => self.detail_ui(e),
            Overlay::Help => self.help_ui(),
        }
        if self.mode == Mode::Shift {
            self.shift_ui();
        }
        if self.toast_left > 0. && self.mode != Mode::Shift {
            let tw = (w * 0.55).min(640.);
            let tx = (w - tw) / 2.;
            let ty = if self.gfx.compact {
                self.top() + 6.
            } else {
                h - 46.
            };
            let a = self.toast_left.min(1.);
            self.gfx.fill_round(
                tx,
                ty,
                tx + tw,
                ty + 34.,
                8.,
                alpha([0.10, 0.16, 0.27, 0.95], a),
            );
            self.gfx
                .stroke_round(tx, ty, tx + tw, ty + 34., 8., 1., alpha(GOLD, a));
            let t = self.toast.clone();
            self.text(
                &t,
                tx + 10.,
                ty + 4.,
                tw - 20.,
                26.,
                Font::NormalCenter,
                alpha(TEXT, a),
            );
        }
        self.gfx.d2d_end();
        self.gfx.present();
    }

    fn top_bar(&mut self) {
        let w = self.gfx.width as f32;
        let top = self.top();
        let compact = self.gfx.compact;
        self.gfx.fill_rect(0., 0., w, top, [0.06, 0.09, 0.16, 1.]);
        self.gfx
            .fill_rect(0., top - 2., w, top - 1., alpha(GOLD, 0.7));
        self.text(
            "YYHistory",
            18.,
            if compact { 6. } else { 8. },
            200.,
            30.,
            Font::TitleMid,
            GOLD,
        );
        if !compact {
            self.text(
                "時空を渡り、歴史に出会う",
                20.,
                40.,
                220.,
                16.,
                Font::Small,
                MUTED,
            );
        }
        let bh = if compact { 32. } else { 36. };
        let by = (top - bh) / 2. - 1.;
        let mut x = if compact { 200. } else { 270. };
        let tabs = [
            (
                "世界地図",
                Act::Atlas,
                matches!(self.mode, Mode::Atlas | Mode::Shift),
            ),
            ("図鑑", Act::Codex, self.mode == Mode::Codex),
        ];
        for (label, a, on) in tabs {
            self.button(label, [x, by, 96., bh], a, on);
            x += 104.;
        }
        if self.journal.visited.iter().any(|v| *v) && self.mode != Mode::Field {
            let era = self.journal.era;
            self.button(
                &format!("{}へ戻る", ERAS[era].name),
                [x, by, if compact { 170. } else { 210. }, bh],
                Act::Shift(era, Some(self.journal.place)),
                false,
            );
        }
        let stat = format!(
            "人物 {}/{}　出来事 {}/{}",
            self.met_count(None),
            PEOPLE.len(),
            self.seen_count(None),
            EVENTS.len()
        );
        let right = w - if compact { 200. } else { 330. };
        self.text(&stat, right - 230., by + 8., 220., 22., Font::Normal, CYAN);
        if compact {
            self.button(
                "使い方",
                [w - 190., by, 84., bh],
                Act::Help,
                self.overlay == Overlay::Help,
            );
            self.button(
                if self.voice { "声 ON" } else { "声 OFF" },
                [w - 98., by, 84., bh],
                Act::Voice,
                self.voice,
            );
        } else {
            self.button("書き出し", [w - 322., by, 76., bh], Act::Export, false);
            self.button("読み込み", [w - 240., by, 76., bh], Act::Import, false);
            self.button(
                "使い方",
                [w - 158., by, 70., bh],
                Act::Help,
                self.overlay == Overlay::Help,
            );
            self.button(
                if self.voice { "声 ON" } else { "声 OFF" },
                [w - 82., by, 66., bh],
                Act::Voice,
                self.voice,
            );
        }
    }

    // -------------------------------------------------------------- 世界地図
    fn atlas_geometry(&mut self) {
        let (s, cx, cy) = self.atlas_view();
        let (x0, y0) = (cx - 180. * s, cy - 90. * s);
        self.gfx.rect(
            0.,
            0.,
            self.gfx.width as f32,
            self.gfx.height as f32,
            [0.06, 0.12, 0.20, 1.],
        );
        self.gfx.draw_textured(
            &self.world,
            x0,
            y0,
            x0 + 360. * s,
            y0 + 180. * s,
            [0.55, 0.68, 0.80, 1.],
            true,
        );
        // 経緯線
        let gc = [0.6, 0.8, 1.0, 0.10];
        for k in -6..=6 {
            let (x, _) = self.atlas_project(k as f32 * 30., 0.);
            self.gfx.line((x, y0), (x, y0 + 180. * s), 1., gc);
        }
        for k in -2..=2 {
            let (_, y) = self.atlas_project(0., k as f32 * 30.);
            self.gfx.line((x0, y), (x0 + 360. * s, y), 1., gc);
        }
        // 行き先の光
        let t = self.clock;
        for (i, e) in ERAS.iter().enumerate() {
            let (x, y) = self.atlas_project(e.lon, e.lat);
            let sel = i == self.pick;
            let r = if sel { 26. } else { 14. } + (t * 2.4 + i as f32).sin() * 2.;
            self.gfx.radial(
                x,
                y,
                r,
                28,
                alpha(era_color(i), if sel { 0.65 } else { 0.4 }),
                [0.5, 0.8, 1., 0.],
            );
        }
    }

    fn atlas_ui(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let top = self.top();
        let lw = self.atlas_list_w();
        let compact = self.gfx.compact;
        // 時代の一覧
        self.gfx.fill_rect(0., top, lw, h, [0.07, 0.10, 0.18, 0.97]);
        self.gfx.fill_rect(lw - 1., top, lw, h, EDGE);
        self.text(
            "転移先の時代",
            20.,
            top + 12.,
            lw - 40.,
            26.,
            Font::Big,
            GOLD,
        );
        self.text(
            "時代と地域を選んでください",
            20.,
            top + 38.,
            lw - 40.,
            18.,
            Font::Small,
            MUTED,
        );
        let list_y = top + 64.;
        let row = if compact { 62. } else { 72. };
        let view = [0., list_y, lw, h - list_y];
        self.scroll_areas.push((view, Scroll::Atlas));
        self.scroll_max[Scroll::Atlas as usize] = ERAS.len() as f32 * row + 12. - view[3];
        let off = self.scroll[Scroll::Atlas as usize];
        self.gfx.clip_begin(0., list_y, lw, h);
        for (i, e) in ERAS.iter().enumerate() {
            let y = list_y + i as f32 * row - off;
            if y + row < list_y || y > h {
                continue;
            }
            let sel = i == self.pick;
            let r = [12., y + 4., lw - 24., row - 8.];
            self.gfx.fill_round(
                r[0],
                r[1],
                r[0] + r[2],
                r[1] + r[3],
                8.,
                if sel {
                    [0.18, 0.25, 0.40, 1.]
                } else {
                    [0.11, 0.16, 0.26, 1.]
                },
            );
            self.gfx.stroke_round(
                r[0],
                r[1],
                r[0] + r[2],
                r[1] + r[3],
                8.,
                1.,
                if sel { GOLD } else { alpha(EDGE, 0.6) },
            );
            self.gfx.fill_rect(
                r[0] + 1.,
                r[1] + 8.,
                r[0] + 5.,
                r[1] + r[3] - 8.,
                era_color(i),
            );
            self.text(
                e.years,
                26.,
                y + 9.,
                lw - 120.,
                18.,
                Font::Small,
                if sel { GOLD } else { MUTED },
            );
            self.text(e.name, 26., y + 26., lw - 110., 24., Font::Big, TEXT);
            if !compact {
                self.text(e.area, 26., y + 50., lw - 60., 16., Font::Small, MUTED);
            }
            let done = self.met_count(Some(i)) + self.seen_count(Some(i));
            let all = PEOPLE.iter().filter(|p| p.era == i).count()
                + EVENTS.iter().filter(|x| x.era == i).count();
            let pct = format!("{}%", done * 100 / all.max(1));
            self.text(
                &pct,
                lw - 86.,
                y + 26.,
                56.,
                24.,
                Font::NormalCenter,
                if done == all { GOLD } else { CYAN },
            );
            self.buttons.push(Button {
                rect: r,
                act: Act::PickEra(i),
            });
        }
        self.gfx.clip_end();

        // 地図の上の名札
        for (i, e) in ERAS.iter().enumerate() {
            let (x, y) = self.atlas_project(e.lon, e.lat);
            let sel = i == self.pick;
            let (tx, ty) = (x + e.tag.0, y + e.tag.1);
            let tw = 136.;
            self.gfx.fill_circle(
                x,
                y,
                if sel { 6. } else { 4.5 },
                if sel { GOLD } else { TEXT },
            );
            self.gfx
                .stroke_circle(x, y, 9., 1.5, alpha(era_color(i), 0.9));
            let r = [tx, ty, tw, 28.];
            self.gfx.fill_round(
                r[0],
                r[1],
                r[0] + tw,
                r[1] + 28.,
                6.,
                if sel {
                    [0.18, 0.25, 0.40, 0.96]
                } else {
                    [0.08, 0.12, 0.20, 0.88]
                },
            );
            self.gfx.stroke_round(
                r[0],
                r[1],
                r[0] + tw,
                r[1] + 28.,
                6.,
                1.,
                if sel { GOLD } else { alpha(era_color(i), 0.8) },
            );
            self.text(
                e.name,
                tx + 4.,
                ty + 3.,
                tw - 8.,
                22.,
                Font::NormalCenter,
                TEXT,
            );
            self.buttons.push(Button {
                rect: r,
                act: Act::PickEra(i),
            });
            self.buttons.push(Button {
                rect: [x - 12., y - 12., 24., 24.],
                act: Act::PickEra(i),
            });
        }

        // 選んだ時代の札
        let e = &ERAS[self.pick];
        let card = if compact { 150. } else { 176. };
        let (x0, y0, x1, y1) = (lw + 16., h - card + 4., w - 16., h - 12.);
        self.panel_box(x0, y0, x1, y1);
        self.text(
            &format!("{}　／　{}", e.years, e.area),
            x0 + 20.,
            y0 + 12.,
            x1 - x0 - 260.,
            18.,
            Font::Small,
            GOLD,
        );
        self.text(
            &format!("{} ― {}", e.name, e.period),
            x0 + 20.,
            y0 + 32.,
            x1 - x0 - 260.,
            26.,
            Font::Big,
            TEXT,
        );
        self.gfx.clip_begin(x0, y0 + 60., x1 - 240., y1 - 8.);
        self.text(
            e.intro,
            x0 + 20.,
            y0 + 62.,
            x1 - x0 - 260.,
            y1 - y0 - 66.,
            Font::Normal,
            MUTED,
        );
        self.gfx.clip_end();
        let pc = PEOPLE.iter().filter(|p| p.era == self.pick).count();
        let ec = EVENTS.iter().filter(|x| x.era == self.pick).count();
        self.text(
            &format!(
                "人物 {}/{}　出来事 {}/{}",
                self.met_count(Some(self.pick)),
                pc,
                self.seen_count(Some(self.pick)),
                ec
            ),
            x1 - 230.,
            y0 + 18.,
            210.,
            20.,
            Font::NormalCenter,
            CYAN,
        );
        self.button(
            "転移する",
            [x1 - 220., y0 + 50., 200., 48.],
            Act::Shift(self.pick, None),
            true,
        );
        if !compact {
            self.text(
                "Enter でも転移できます",
                x1 - 230.,
                y0 + 108.,
                210.,
                18.,
                Font::Small,
                MUTED,
            );
        }
    }

    fn shift_ui(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let t = (self.shift_t / 1.6).clamp(0., 1.);
        let fade = if t < 0.15 {
            t / 0.15
        } else if t > 0.85 {
            (1. - t) / 0.15
        } else {
            1.
        };
        self.gfx
            .fill_rect(0., 0., w, h, [0.02, 0.04, 0.09, 0.92 * fade]);
        let (cx, cy) = (w / 2., h / 2.);
        for k in 0..7 {
            let p = ((t * 1.8 + k as f32 / 7.) % 1.).powf(1.4);
            let r = 20. + p * w.max(h) * 0.6;
            self.gfx.stroke_circle(
                cx,
                cy,
                r,
                2. + 3. * (1. - p),
                alpha(mix(CYAN, GOLD, k as f32 / 7.), (1. - p) * fade),
            );
        }
        let (era, _) = self.shift_to;
        let e = &ERAS[era];
        self.text(
            "時 空 転 移",
            cx - 300.,
            cy - 92.,
            600.,
            40.,
            Font::Title,
            alpha(CYAN, fade),
        );
        self.text(
            e.years,
            cx - 300.,
            cy - 30.,
            600.,
            30.,
            Font::BigCenter,
            alpha(GOLD, fade),
        );
        self.text(
            &format!("{} ― {}", e.name, e.area),
            cx - 300.,
            cy + 6.,
            600.,
            30.,
            Font::BigCenter,
            alpha(TEXT, fade),
        );
        let n = (t * 24.) as usize;
        self.text(
            &format!("座標固定中{}", "・".repeat(n % 4)),
            cx - 300.,
            cy + 56.,
            600.,
            22.,
            Font::NormalCenter,
            alpha(MUTED, fade),
        );
    }

    // -------------------------------------------------------------- ミニマップ
    fn field_geometry(&mut self) {
        let r = self.map_rect();
        let era = &ERAS[self.journal.era];
        let tint = era.tint;
        let m = |x: f32, y: f32| (r[0] + x * r[2], r[1] + y * r[3]);
        let g = &mut self.gfx;
        // 額縁と紙
        g.rect(
            r[0] - 8.,
            r[1] - 8.,
            r[0] + r[2] + 8.,
            r[1] + r[3] + 8.,
            [0.35, 0.27, 0.16, 1.],
        );
        g.rect(
            r[0] - 4.,
            r[1] - 4.,
            r[0] + r[2] + 4.,
            r[1] + r[3] + 4.,
            GOLD,
        );
        g.rect_vgrad(
            r[0],
            r[1],
            r[0] + r[2],
            r[1] + r[3],
            tint,
            mix(tint, [0.70, 0.60, 0.45, 1.], 0.25),
        );
        // 紙のしみ
        for k in 0..9 {
            let fx = ((k as f32 * 0.618 + 0.13) % 1.) * 0.9 + 0.05;
            let fy = ((k as f32 * 0.414 + 0.37) % 1.) * 0.9 + 0.05;
            let (x, y) = m(fx, fy);
            g.radial(
                x,
                y,
                r[2] * 0.12,
                20,
                [0.55, 0.42, 0.25, 0.07],
                [0.55, 0.42, 0.25, 0.],
            );
        }
        let ink = [0.42, 0.32, 0.20, 0.55];
        for f in era.features {
            match f {
                Feature::Water(pts) => fill_poly(g, pts, &m, WATER),
                Feature::Land(pts) => fill_poly(g, pts, &m, mix(tint, [0.75, 0.68, 0.5, 1.], 0.15)),
                Feature::River(pts, wd) => {
                    let wd = wd * r[3];
                    for s in pts.windows(2) {
                        g.line(m(s[0].0, s[0].1), m(s[1].0, s[1].1), wd, WATER);
                    }
                    for p in pts.iter() {
                        let (x, y) = m(p.0, p.1);
                        g.circle(x, y, wd / 2., 12, WATER);
                    }
                }
                Feature::Hill(x, y, rad) => {
                    let (x, y) = m(*x, *y);
                    let rr = rad * r[3];
                    g.radial(
                        x,
                        y,
                        rr,
                        24,
                        [0.55, 0.45, 0.28, 0.45],
                        [0.55, 0.45, 0.28, 0.],
                    );
                    for k in 0..3 {
                        let ox = (k as f32 - 1.) * rr * 0.35;
                        let pk = (x + ox, y - rr * 0.32);
                        g.line((pk.0 - rr * 0.18, pk.1 + rr * 0.16), pk, 1.4, ink);
                        g.line(pk, (pk.0 + rr * 0.18, pk.1 + rr * 0.16), 1.4, ink);
                    }
                }
                Feature::Wall(pts) => {
                    for k in 0..pts.len() {
                        let a = pts[k];
                        let b = pts[(k + 1) % pts.len()];
                        g.line(m(a.0, a.1), m(b.0, b.1), 2.6, [0.40, 0.30, 0.18, 0.75]);
                    }
                }
                Feature::Grid(x0, y0, x1, y1, nx, ny) => {
                    let c = [0.45, 0.36, 0.24, 0.22];
                    for k in 1..*nx {
                        let x = x0 + (x1 - x0) * k as f32 / *nx as f32;
                        g.line(m(x, *y0), m(x, *y1), 1., c);
                    }
                    for k in 1..*ny {
                        let y = y0 + (y1 - y0) * k as f32 / *ny as f32;
                        g.line(m(*x0, y), m(*x1, y), 1., c);
                    }
                }
                Feature::Road(pts) => {
                    for s in pts.windows(2) {
                        g.line(
                            m(s[0].0, s[0].1),
                            m(s[1].0, s[1].1),
                            3.,
                            [0.62, 0.52, 0.36, 0.7],
                        );
                    }
                }
                Feature::Label(..) => (),
            }
        }
        // 道（点線）
        for &(a, b) in era.roads {
            let pa = m(era.places[a].x, era.places[a].y);
            let pb = m(era.places[b].x, era.places[b].y);
            let len = ((pb.0 - pa.0).powi(2) + (pb.1 - pa.1).powi(2)).sqrt();
            let n = (len / 10.).max(1.) as usize;
            for k in (0..n).step_by(2) {
                let t0 = k as f32 / n as f32;
                let t1 = ((k + 1) as f32 / n as f32).min(1.);
                g.line(
                    (pa.0 + (pb.0 - pa.0) * t0, pa.1 + (pb.1 - pa.1) * t0),
                    (pa.0 + (pb.0 - pa.0) * t1, pa.1 + (pb.1 - pa.1) * t1),
                    2.4,
                    [0.48, 0.30, 0.16, 0.8],
                );
            }
        }
        // 方位
        let (cx, cy) = (r[0] + r[2] - 34., r[1] + 38.);
        g.tri(
            (cx, cy - 22.),
            (cx - 6., cy),
            (cx + 6., cy),
            [0.55, 0.20, 0.15, 0.9],
        );
        g.tri(
            (cx, cy + 22.),
            (cx - 6., cy),
            (cx + 6., cy),
            [0.40, 0.32, 0.22, 0.7],
        );
        g.ring(cx, cy, 14., 1.2, 24, [0.40, 0.32, 0.22, 0.6]);
        // 場所の図柄
        for (i, p) in era.places.iter().enumerate() {
            let (x, y) = m(p.x, p.y);
            let here = i == self.journal.place && self.travel.is_none();
            if here {
                let pulse = 0.5 + 0.5 * (self.clock * 3.).sin();
                g.radial(
                    x,
                    y,
                    34. + pulse * 6.,
                    30,
                    [1., 0.85, 0.45, 0.45],
                    [1., 0.85, 0.45, 0.],
                );
            }
            g.circle(x + 2., y + 3., 19., 24, [0.2, 0.12, 0.05, 0.25]);
            g.circle(x, y, 19., 24, [0.97, 0.93, 0.83, 1.]);
            g.ring(
                x,
                y,
                19.,
                2.2,
                24,
                if here {
                    [0.80, 0.55, 0.15, 1.]
                } else {
                    [0.42, 0.30, 0.18, 1.]
                },
            );
            icon(g, p.kind, x, y, 11.);
        }
        // 自分
        let (ax, ay) = m(self.avatar.0, self.avatar.1);
        let bob = (self.clock * 5.).sin() * 2.;
        let (ax, ay) = (ax + 14., ay - 26. + bob);
        g.circle(ax, ay + 24. - bob, 7., 16, [0., 0., 0., 0.25]);
        g.tri(
            (ax, ay + 20.),
            (ax - 9., ay + 4.),
            (ax + 9., ay + 4.),
            [0.15, 0.55, 0.75, 1.],
        );
        g.circle(ax, ay, 9., 20, [0.15, 0.55, 0.75, 1.]);
        g.circle(ax, ay, 6., 20, [0.85, 0.97, 1., 1.]);
    }

    fn field_ui(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let top = self.top();
        let r = self.map_rect();
        let ei = self.journal.era;
        let era = &ERAS[ei];
        let compact = self.gfx.compact;
        // 地名の注記
        for f in era.features {
            if let Feature::Label(s, x, y) = f {
                let (x, y) = (r[0] + x * r[2], r[1] + y * r[3]);
                self.text(
                    s,
                    x - 110.,
                    y - 9.,
                    220.,
                    18.,
                    Font::NormalCenter,
                    [0.38, 0.27, 0.16, 0.85],
                );
            }
        }
        // 場所の名前と印
        for (i, p) in era.places.iter().enumerate() {
            let (x, y) = (r[0] + p.x * r[2], r[1] + p.y * r[3]);
            let here = i == self.journal.place && self.travel.is_none();
            let tw = self.gfx.measure(p.name, Font::Small).min(190.) + 14.;
            let ly = if p.y > 0.86 { y - 44. } else { y + 22. };
            self.gfx.fill_round(
                x - tw / 2.,
                ly,
                x + tw / 2.,
                ly + 19.,
                4.,
                if here {
                    [0.42, 0.22, 0.08, 0.92]
                } else {
                    [0.20, 0.14, 0.08, 0.80]
                },
            );
            self.text(
                p.name,
                x - tw / 2. + 4.,
                ly + 1.,
                tw - 8.,
                17.,
                Font::Small,
                [1., 0.96, 0.88, 1.],
            );
            // 未発見の人物・出来事の印
            let unmet = PEOPLE
                .iter()
                .enumerate()
                .filter(|(k, q)| q.era == ei && q.place == p.id && !self.journal.met[*k])
                .count();
            let unseen = EVENTS
                .iter()
                .enumerate()
                .filter(|(k, q)| q.era == ei && q.place == p.id && !self.journal.seen[*k])
                .count();
            let mut bx = x + 14.;
            if unmet > 0 {
                self.gfx
                    .fill_circle(bx, y - 16., 9., [0.20, 0.50, 0.75, 1.]);
                self.gfx.stroke_circle(bx, y - 16., 9., 1.5, TEXT);
                self.text("人", bx - 9., y - 25., 18., 18., Font::NormalCenter, TEXT);
                bx += 19.;
            }
            if unseen > 0 {
                let b = 1. + 0.12 * (self.clock * 4. + i as f32).sin();
                self.gfx
                    .fill_circle(bx, y - 16., 9. * b, [0.82, 0.35, 0.22, 1.]);
                self.gfx.stroke_circle(bx, y - 16., 9. * b, 1.5, TEXT);
                self.text("！", bx - 9., y - 25., 18., 18., Font::NormalCenter, TEXT);
            }
            self.buttons.push(Button {
                rect: [x - 22., y - 22., 44., 44.],
                act: Act::Goto(i),
            });
        }
        self.text(
            era.map_note,
            r[0],
            r[1] + r[3] + 12.,
            r[2],
            18.,
            Font::Small,
            MUTED,
        );

        // 右の情報欄
        let pw = self.panel_w();
        let x0 = w - pw - 8.;
        let (y0, y1) = (top + 10., h - 10.);
        self.panel_box(x0, y0, w - 8., y1);
        let iw = pw - 36.;
        let ix = x0 + 18.;
        self.text(
            &format!("{}　{}", era.years, era.period),
            ix,
            y0 + 12.,
            iw,
            18.,
            Font::Small,
            GOLD,
        );
        self.text(era.name, ix, y0 + 30., iw, 26., Font::Big, TEXT);
        let pc = PEOPLE.iter().filter(|p| p.era == ei).count();
        let ec = EVENTS.iter().filter(|x| x.era == ei).count();
        self.text(
            &format!(
                "人物 {}/{}　出来事 {}/{}",
                self.met_count(Some(ei)),
                pc,
                self.seen_count(Some(ei)),
                ec
            ),
            ix,
            y0 + 56.,
            iw,
            18.,
            Font::Small,
            CYAN,
        );
        self.gfx
            .fill_rect(ix, y0 + 80., ix + iw, y0 + 81., alpha(EDGE, 0.8));
        let foot = if compact { 52. } else { 60. };
        let cy0 = y0 + 88.;
        let cy1 = y1 - foot;
        self.scroll_areas
            .push(([x0, cy0, pw, cy1 - cy0], Scroll::Panel));
        let off = self.scroll[Scroll::Panel as usize];
        self.gfx.clip_begin(x0, cy0, w - 8., cy1);
        let mut y = cy0 + 6. - off;
        if let Some(tr) = &self.travel {
            let dest = era.places[tr.target].name;
            self.text("移動中…", ix, y, iw, 24., Font::Big, CYAN);
            self.text(
                &format!("行き先：{}", dest),
                ix,
                y + 28.,
                iw,
                20.,
                Font::Normal,
                TEXT,
            );
            y += 60.;
        } else {
            let p = &era.places[self.journal.place];
            self.text("現在地", ix, y, iw, 16., Font::Small, MUTED);
            self.text(p.name, ix, y + 16., iw, 26., Font::Big, GOLD);
            let dh = self.gfx.measure_h(p.desc, Font::Normal, iw);
            self.text(p.desc, ix, y + 46., iw, dh, Font::Normal, MUTED);
            y += 54. + dh;
            let people = self.people_here();
            self.text("この場所にいる人物", ix, y, iw, 20., Font::Normal, TEXT);
            y += 24.;
            if people.is_empty() {
                self.text("（いまは誰もいない）", ix, y, iw, 20., Font::Small, MUTED);
                y += 24.;
            }
            for i in people {
                let p = &PEOPLE[i];
                let rr = [ix, y, iw, 50.];
                let hover = inside(rr, self.mouse.0, self.mouse.1);
                self.gfx.fill_round(
                    ix,
                    y,
                    ix + iw,
                    y + 50.,
                    7.,
                    if hover {
                        [0.20, 0.28, 0.43, 1.]
                    } else {
                        PANEL2
                    },
                );
                self.medallion(p.name, ix + 25., y + 25., 18., era_color(ei), true);
                self.text(p.name, ix + 50., y + 6., iw - 110., 20., Font::Normal, TEXT);
                self.text(
                    p.role,
                    ix + 50.,
                    y + 27.,
                    iw - 110.,
                    17.,
                    Font::Small,
                    MUTED,
                );
                self.text(
                    if self.journal.met[i] {
                        "再会"
                    } else {
                        "会う"
                    },
                    ix + iw - 58.,
                    y + 13.,
                    50.,
                    24.,
                    Font::NormalCenter,
                    if self.journal.met[i] { CYAN } else { GOLD },
                );
                self.buttons.push(Button {
                    rect: rr,
                    act: Act::Meet(i),
                });
                y += 56.;
            }
            y += 8.;
            let events = self.events_here();
            self.text("ここで起きた出来事", ix, y, iw, 20., Font::Normal, TEXT);
            y += 24.;
            if events.is_empty() {
                self.text(
                    "（この場所の出来事は収録していません）",
                    ix,
                    y,
                    iw,
                    20.,
                    Font::Small,
                    MUTED,
                );
                y += 24.;
            }
            for i in events {
                let e = &EVENTS[i];
                let rr = [ix, y, iw, 46.];
                let hover = inside(rr, self.mouse.0, self.mouse.1);
                self.gfx.fill_round(
                    ix,
                    y,
                    ix + iw,
                    y + 46.,
                    7.,
                    if hover {
                        [0.20, 0.28, 0.43, 1.]
                    } else {
                        PANEL2
                    },
                );
                self.gfx.fill_rect(
                    ix + 1.,
                    y + 6.,
                    ix + 4.,
                    y + 40.,
                    if self.journal.seen[i] {
                        CYAN
                    } else {
                        [0.82, 0.35, 0.22, 1.]
                    },
                );
                self.text(e.date, ix + 12., y + 4., iw - 80., 17., Font::Small, GOLD);
                self.text(
                    e.title,
                    ix + 12.,
                    y + 22.,
                    iw - 80.,
                    20.,
                    Font::Normal,
                    TEXT,
                );
                self.text(
                    if self.journal.seen[i] {
                        "記録済"
                    } else {
                        "観察"
                    },
                    ix + iw - 62.,
                    y + 11.,
                    56.,
                    24.,
                    Font::NormalCenter,
                    if self.journal.seen[i] { CYAN } else { GOLD },
                );
                self.buttons.push(Button {
                    rect: rr,
                    act: Act::Observe(i),
                });
                y += 52.;
            }
        }
        let content = y + off - cy0;
        self.scroll_max[Scroll::Panel as usize] = content - (cy1 - cy0);
        self.gfx.clip_end();
        let bw = (iw - 10.) / 2.;
        self.button(
            "図鑑を開く",
            [ix, y1 - foot + 10., bw, foot - 22.],
            Act::Codex,
            false,
        );
        self.button(
            "時代を選び直す",
            [ix + bw + 10., y1 - foot + 10., bw, foot - 22.],
            Act::Atlas,
            false,
        );
        // 手前の案内
        if !compact {
            self.text(
                "場所を押すと道をたどって移動します（矢印キーでも移動）",
                r[0],
                top + 14. - 12.,
                r[2],
                0.,
                Font::Small,
                MUTED,
            );
        }
    }

    // -------------------------------------------------------------- 会話・観察
    fn card(&mut self, cw: f32, ch: f32) -> [f32; 4] {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        self.gfx.fill_rect(0., 0., w, h, [0.01, 0.02, 0.05, 0.72]);
        let cw = cw.min(w - 32.);
        let ch = ch.min(h - 24.);
        let x = (w - cw) / 2.;
        let y = (h - ch) / 2.;
        self.panel_box(x, y, x + cw, y + ch);
        [x, y, cw, ch]
    }

    fn meet_ui(&mut self, i: usize, page: usize) {
        let p = &PEOPLE[i];
        let compact = self.gfx.compact;
        let [x, y, cw, ch] = self.card(860., if compact { 420. } else { 470. });
        let mr = if compact { 44. } else { 58. };
        self.medallion(
            p.name,
            x + 30. + mr,
            y + 30. + mr,
            mr,
            era_color(p.era),
            true,
        );
        let tx = x + 60. + mr * 2.;
        let tw = cw - (tx - x) - 24.;
        self.text(
            &format!("{}　／　{}", ERAS[p.era].name, p.life),
            tx,
            y + 24.,
            tw,
            18.,
            Font::Small,
            GOLD,
        );
        self.text(p.name, tx, y + 44., tw, 28., Font::Big, TEXT);
        self.text(p.native, tx, y + 74., tw, 18., Font::Small, MUTED);
        self.text(p.role, tx, y + 96., tw, 20., Font::Normal, CYAN);
        // 言葉
        let by = y + if compact { 150. } else { 170. };
        let bh = ch - (by - y) - 96.;
        self.gfx.fill_round(
            x + 24.,
            by,
            x + cw - 24.,
            by + bh,
            8.,
            [0.05, 0.08, 0.14, 0.95],
        );
        self.gfx
            .stroke_round(x + 24., by, x + cw - 24., by + bh, 8., 1., alpha(GOLD, 0.6));
        let line = p.lines[page.min(p.lines.len() - 1)];
        self.text(line, x + 46., by + 18., cw - 92., bh - 30., Font::Big, TEXT);
        self.text(
            &format!("{} / {}", page + 1, p.lines.len()),
            x + cw - 120.,
            by + bh - 26.,
            80.,
            18.,
            Font::Small,
            MUTED,
        );
        self.text(
            "※人物の言葉は、史料に基づく事実を本人の語りとして再構成したものです。「」内の史料の引用を除き、実際の発言ではありません。",
            x + 26., y + ch - 84., cw - 52., 34., Font::Small, MUTED,
        );
        let last = page + 1 >= p.lines.len();
        self.button(
            "詳しく（図鑑）",
            [x + 24., y + ch - 46., 150., 34.],
            Act::Detail(Entry::Person(i)),
            false,
        );
        if page > 0 {
            self.button(
                "‹ 前へ",
                [x + cw - 330., y + ch - 46., 120., 34.],
                Act::Prev,
                false,
            );
        }
        self.button(
            if last { "別れる" } else { "次へ ›" },
            [x + cw - 200., y + ch - 46., 176., 34.],
            Act::Next,
            true,
        );
    }

    fn observe_ui(&mut self, i: usize) {
        let e = &EVENTS[i];
        let tw = 860.0f32.min(self.gfx.width as f32 - 32.) - 60.;
        let mut need = 98. + 60. + 60.;
        for para in e.body {
            need += self.gfx.measure_h(para, Font::Normal, tw) + 10.;
        }
        need += self.gfx.measure_h(e.source, Font::Small, tw - 40.);
        if !e.people.is_empty() {
            need += 64.;
        }
        let [x, y, cw, ch] = self.card(860., need.max(320.));
        let place = place_index(e.era, e.place)
            .map(|k| ERAS[e.era].places[k].name)
            .unwrap_or("");
        self.text(
            "― 観察 ―",
            x + 24.,
            y + 14.,
            cw - 48.,
            20.,
            Font::NormalCenter,
            CYAN,
        );
        self.text(
            &format!("{}　／　{}　／　{}", e.date, ERAS[e.era].name, place),
            x + 30.,
            y + 40.,
            cw - 60.,
            18.,
            Font::Small,
            GOLD,
        );
        self.text(e.title, x + 30., y + 60., cw - 60., 30., Font::Big, TEXT);
        let body_y = y + 98.;
        let body_h = ch - 98. - 60.;
        self.scroll_areas
            .push(([x, body_y, cw, body_h], Scroll::Overlay));
        let off = self.scroll[Scroll::Overlay as usize];
        self.gfx.clip_begin(x, body_y, x + cw, body_y + body_h);
        let mut yy = body_y + 4. - off;
        for para in e.body {
            let ph = self.gfx.measure_h(para, Font::Normal, tw);
            self.text(para, x + 30., yy, tw, ph, Font::Normal, TEXT);
            yy += ph + 10.;
        }
        if !e.people.is_empty() {
            self.text(
                "関わった人物",
                x + 30.,
                yy + 4.,
                tw,
                18.,
                Font::Small,
                MUTED,
            );
            yy += 26.;
            let mut px = x + 30.;
            for id in e.people {
                if let Some(k) = person_index(id) {
                    let known = self.journal.met[k] || self.reveal;
                    let name = if known { PEOPLE[k].name } else { "？？？" };
                    let nw = self.gfx.measure(name, Font::Small) + 22.;
                    if px + nw > x + cw - 30. {
                        px = x + 30.;
                        yy += 30.;
                    }
                    self.gfx.fill_round(
                        px,
                        yy,
                        px + nw,
                        yy + 24.,
                        12.,
                        if known {
                            PANEL2
                        } else {
                            [0.10, 0.13, 0.20, 1.]
                        },
                    );
                    self.gfx.stroke_round(
                        px,
                        yy,
                        px + nw,
                        yy + 24.,
                        12.,
                        1.,
                        if known { GOLD } else { LOCKED },
                    );
                    self.text(
                        name,
                        px + 11.,
                        yy + 4.,
                        nw - 22.,
                        17.,
                        Font::Small,
                        if known { TEXT } else { MUTED },
                    );
                    px += nw + 8.;
                }
            }
            yy += 34.;
        }
        let sh = self.gfx.measure_h(e.source, Font::Small, tw - 40.);
        self.text(
            &format!("史料：{}", e.source),
            x + 30.,
            yy,
            tw,
            sh + 4.,
            Font::Small,
            MUTED,
        );
        yy += sh + 10.;
        self.scroll_max[Scroll::Overlay as usize] = yy + off - body_y - body_h;
        self.gfx.clip_end();
        self.button(
            "図鑑で見る",
            [x + 24., y + ch - 48., 150., 34.],
            Act::Detail(Entry::Event(i)),
            false,
        );
        self.button(
            "観察を終える",
            [x + cw - 200., y + ch - 48., 176., 34.],
            Act::Close,
            true,
        );
    }

    // -------------------------------------------------------------- 図鑑
    fn codex_match_person(&self, i: usize) -> bool {
        let p = &PEOPLE[i];
        if self.filter.is_some_and(|f| p.era != f) {
            return false;
        }
        if self.query.is_empty() {
            return true;
        }
        (self.journal.met[i] || self.reveal)
            && contains(
                &[p.name, p.native, p.role, p.bio, p.life, ERAS[p.era].name],
                &self.query,
            )
    }
    fn codex_match_event(&self, i: usize) -> bool {
        let e = &EVENTS[i];
        if self.filter.is_some_and(|f| e.era != f) {
            return false;
        }
        if self.query.is_empty() {
            return true;
        }
        (self.journal.seen[i] || self.reveal)
            && (contains(&[e.title, e.date, ERAS[e.era].name], &self.query)
                || contains(e.body, &self.query))
    }

    fn codex_ui(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let top = self.top();
        let compact = self.gfx.compact;
        self.gfx.fill_rect(0., top, w, h, [0.06, 0.09, 0.16, 1.]);
        // 見出しと切り替え
        let tabs = [
            ("人物一覧表", Tab::People),
            ("年表", Tab::Timeline),
            ("出来事集", Tab::Events),
        ];
        let tw = if compact { 112. } else { 140. };
        for (k, (label, t)) in tabs.iter().enumerate() {
            self.button(
                label,
                [16. + k as f32 * (tw + 8.), top + 12., tw, 36.],
                Act::Tab(*t),
                self.tab == *t,
            );
        }
        let sb = self.search_box();
        let sx = sb[0];
        self.gfx
            .fill_round(sb[0], sb[1], sb[0] + sb[2], sb[1] + sb[3], 6., PANEL2);
        self.gfx.stroke_round(
            sb[0],
            sb[1],
            sb[0] + sb[2],
            sb[1] + sb[3],
            6.,
            1.,
            if self.searching { GOLD } else { EDGE },
        );
        if self.query.is_empty() {
            self.text(
                if self.searching {
                    "入力してください…"
                } else {
                    "名前・出来事・年で検索"
                },
                sb[0],
                sb[1] + 2.,
                sb[2],
                sb[3] - 4.,
                Font::NormalCenter,
                MUTED,
            );
        } else {
            let q = self.query.clone();
            self.text(
                &q,
                sb[0],
                sb[1] + 2.,
                sb[2],
                sb[3] - 4.,
                Font::NormalCenter,
                TEXT,
            );
        }
        self.buttons.push(Button {
            rect: sb,
            act: Act::Search,
        });
        let rx = 16. + 3. * (tw + 8.);
        if sx - rx > 150. {
            self.button(
                if self.reveal {
                    "未発見も表示：ON"
                } else {
                    "未発見も表示：OFF"
                },
                [sx - 166., top + 14., 150., 32.],
                Act::Reveal,
                self.reveal,
            );
        }
        // 時代の絞り込み
        let mut fx = 16.;
        let fy = top + 58.;
        let chips: Vec<(String, Option<usize>)> = std::iter::once(("すべて".to_string(), None))
            .chain(
                ERAS.iter()
                    .enumerate()
                    .map(|(i, e)| (e.name.to_string(), Some(i))),
            )
            .collect();
        for (label, f) in chips {
            let cw = self.gfx.measure(&label, Font::Small) + 24.;
            if fx + cw > w - 16. {
                break;
            }
            let on = self.filter == f;
            let c = f.map(era_color).unwrap_or(GOLD);
            self.gfx.fill_round(
                fx,
                fy,
                fx + cw,
                fy + 26.,
                13.,
                if on { alpha(c, 0.9) } else { PANEL2 },
            );
            self.gfx
                .stroke_round(fx, fy, fx + cw, fy + 26., 13., 1., alpha(c, 0.9));
            self.text(
                &label,
                fx + 4.,
                fy + 4.,
                cw - 8.,
                18.,
                Font::Small,
                if on { NIGHT } else { TEXT },
            );
            self.buttons.push(Button {
                rect: [fx, fy, cw, 26.],
                act: Act::Filter(f),
            });
            fx += cw + 6.;
        }
        let gy = fy + 36.;
        let view = [0., gy, w, h - gy];
        self.scroll_areas.push((view, Scroll::Codex));
        let off = self.scroll[Scroll::Codex as usize];
        self.gfx.clip_begin(0., gy, w, h);
        let content = match self.tab {
            Tab::People => self.codex_people(gy - off, w, gy, h),
            Tab::Timeline => self.codex_timeline(gy - off, w, gy, h),
            Tab::Events => self.codex_events(gy - off, w, gy, h),
        };
        self.gfx.clip_end();
        self.scroll_max[Scroll::Codex as usize] = content - view[3] + 20.;
    }

    /// 戻り値：中身の高さ
    fn codex_people(&mut self, y0: f32, w: f32, vy0: f32, vy1: f32) -> f32 {
        let order = people_order();
        let items: Vec<usize> = order
            .iter()
            .copied()
            .filter(|i| self.codex_match_person(*i))
            .collect();
        let cols = (((w - 32.) / 230.) as usize).max(2);
        let cw = (w - 32. - (cols - 1) as f32 * 10.) / cols as f32;
        let ch = 132.;
        self.text(
            &format!(
                "出会った人物 {}/{}",
                self.met_count(self.filter),
                PEOPLE
                    .iter()
                    .filter(|p| self.filter.is_none_or(|f| p.era == f))
                    .count()
            ),
            18.,
            y0 + 2.,
            400.,
            20.,
            Font::Normal,
            CYAN,
        );
        let y0 = y0 + 28.;
        for (k, i) in items.iter().enumerate() {
            let x = 16. + (k % cols) as f32 * (cw + 10.);
            let y = y0 + (k / cols) as f32 * (ch + 10.);
            if y + ch < vy0 || y > vy1 {
                continue;
            }
            let p = &PEOPLE[*i];
            let known = self.journal.met[*i] || self.reveal;
            let found = self.journal.met[*i];
            self.gfx.fill_round(
                x,
                y,
                x + cw,
                y + ch,
                8.,
                if known {
                    PANEL2
                } else {
                    [0.09, 0.12, 0.19, 1.]
                },
            );
            self.gfx.stroke_round(
                x,
                y,
                x + cw,
                y + ch,
                8.,
                1.,
                if found {
                    alpha(era_color(p.era), 0.9)
                } else {
                    alpha(LOCKED, 0.7)
                },
            );
            let no = order.iter().position(|q| q == i).unwrap_or(0) + 1;
            self.text(
                &format!("No.{:03}", no),
                x + 10.,
                y + 8.,
                80.,
                16.,
                Font::Small,
                if found { GOLD } else { LOCKED },
            );
            self.medallion(p.name, x + 36., y + 62., 24., era_color(p.era), known);
            let tx = x + 70.;
            let tw = cw - 80.;
            if known {
                self.text(p.name, tx, y + 34., tw, 40., Font::Normal, TEXT);
                self.text(p.life, tx, y + 76., tw, 16., Font::Small, MUTED);
            } else {
                self.text("？？？", tx, y + 40., tw, 20., Font::Normal, MUTED);
                let place = place_index(p.era, p.place)
                    .map(|k| ERAS[p.era].places[k].name)
                    .unwrap_or("");
                self.text(
                    &format!("{}で会える", place),
                    tx,
                    y + 66.,
                    tw,
                    30.,
                    Font::Small,
                    LOCKED,
                );
            }
            self.text(
                ERAS[p.era].name,
                x + 10.,
                y + ch - 24.,
                cw - 20.,
                16.,
                Font::Small,
                alpha(era_color(p.era), 0.95),
            );
            if inside([x, y, cw, ch], x, y) && y >= vy0 - ch {
                self.buttons.push(Button {
                    rect: [x, y.max(vy0), cw, (y + ch).min(vy1) - y.max(vy0)],
                    act: Act::Detail(Entry::Person(*i)),
                });
            }
        }
        if items.is_empty() {
            self.text(
                "該当する人物がいません。（検索は発見済みの項目が対象です）",
                18.,
                y0 + 10.,
                w - 36.,
                24.,
                Font::Normal,
                MUTED,
            );
        }
        28. + (items.len().div_ceil(cols)) as f32 * (ch + 10.)
    }

    fn codex_events(&mut self, y0: f32, w: f32, vy0: f32, vy1: f32) -> f32 {
        let items: Vec<usize> = chronology()
            .into_iter()
            .filter(|i| self.codex_match_event(*i))
            .collect();
        let cols = (((w - 32.) / 300.) as usize).max(2);
        let cw = (w - 32. - (cols - 1) as f32 * 10.) / cols as f32;
        let ch = 118.;
        self.text(
            &format!(
                "観察した出来事 {}/{}",
                self.seen_count(self.filter),
                EVENTS
                    .iter()
                    .filter(|e| self.filter.is_none_or(|f| e.era == f))
                    .count()
            ),
            18.,
            y0 + 2.,
            400.,
            20.,
            Font::Normal,
            CYAN,
        );
        let y0 = y0 + 28.;
        for (k, i) in items.iter().enumerate() {
            let x = 16. + (k % cols) as f32 * (cw + 10.);
            let y = y0 + (k / cols) as f32 * (ch + 10.);
            if y + ch < vy0 || y > vy1 {
                continue;
            }
            let e = &EVENTS[*i];
            let known = self.journal.seen[*i] || self.reveal;
            let found = self.journal.seen[*i];
            let c = era_color(e.era);
            self.gfx.fill_round(
                x,
                y,
                x + cw,
                y + ch,
                8.,
                if known {
                    PANEL2
                } else {
                    [0.09, 0.12, 0.19, 1.]
                },
            );
            self.gfx.stroke_round(
                x,
                y,
                x + cw,
                y + ch,
                8.,
                1.,
                if found {
                    alpha(c, 0.9)
                } else {
                    alpha(LOCKED, 0.7)
                },
            );
            self.gfx.fill_rect(
                x + 1.,
                y + 10.,
                x + 5.,
                y + ch - 10.,
                if found { c } else { LOCKED },
            );
            let no = chronology().iter().position(|q| q == i).unwrap_or(0) + 1;
            self.text(
                &format!("No.{:03}", no),
                x + 14.,
                y + 8.,
                80.,
                16.,
                Font::Small,
                if found { GOLD } else { LOCKED },
            );
            self.text(
                e.date,
                x + 84.,
                y + 8.,
                cw - 96.,
                16.,
                Font::Small,
                if known { GOLD } else { MUTED },
            );
            if known {
                self.text(e.title, x + 14., y + 30., cw - 28., 24., Font::Big, TEXT);
                let lead = e.body[0];
                self.gfx
                    .clip_begin(x, y.max(vy0), x + cw, (y + ch - 26.).min(vy1));
                self.text(lead, x + 14., y + 56., cw - 28., 36., Font::Small, MUTED);
                self.gfx.clip_end();
            } else {
                self.text("？？？", x + 14., y + 30., cw - 28., 24., Font::Big, MUTED);
                let place = place_index(e.era, e.place)
                    .map(|k| ERAS[e.era].places[k].name)
                    .unwrap_or("");
                self.text(
                    &format!("{}で観察できる", place),
                    x + 14.,
                    y + 58.,
                    cw - 28.,
                    30.,
                    Font::Small,
                    LOCKED,
                );
            }
            self.text(
                ERAS[e.era].name,
                x + 14.,
                y + ch - 22.,
                cw - 28.,
                16.,
                Font::Small,
                alpha(c, 0.95),
            );
            self.buttons.push(Button {
                rect: [x, y.max(vy0), cw, (y + ch).min(vy1) - y.max(vy0)],
                act: Act::Detail(Entry::Event(*i)),
            });
        }
        if items.is_empty() {
            self.text(
                "該当する出来事がありません。（検索は発見済みの項目が対象です）",
                18.,
                y0 + 10.,
                w - 36.,
                24.,
                Font::Normal,
                MUTED,
            );
        }
        28. + (items.len().div_ceil(cols)) as f32 * (ch + 10.)
    }

    fn codex_timeline(&mut self, y0: f32, w: f32, vy0: f32, vy1: f32) -> f32 {
        let items: Vec<usize> = chronology()
            .into_iter()
            .filter(|i| self.codex_match_event(*i))
            .collect();
        let lx = 16.;
        let axis = lx + if self.gfx.compact { 200. } else { 250. };
        let mut y = y0 + 6.;
        let mut last_era = usize::MAX;
        let row = 46.;
        let line_top = y;
        for i in &items {
            let e = &EVENTS[*i];
            if e.era != last_era {
                last_era = e.era;
                let era = &ERAS[e.era];
                y += 8.;
                if y + 40. > vy0 && y < vy1 {
                    let c = era_color(e.era);
                    self.gfx.fill_round(
                        lx,
                        y,
                        w - 16.,
                        y + 36.,
                        6.,
                        alpha(mix(NIGHT, c, 0.35), 1.),
                    );
                    self.text(
                        &format!("{}　{} ― {}", era.years, era.name, era.period),
                        lx + 14.,
                        y + 8.,
                        w - 60.,
                        22.,
                        Font::Big,
                        TEXT,
                    );
                }
                y += 46.;
            }
            if y + row > vy0 && y < vy1 {
                let known = self.journal.seen[*i] || self.reveal;
                let found = self.journal.seen[*i];
                let c = era_color(e.era);
                let rr = [lx, y, w - 32., row - 6.];
                if inside(rr, self.mouse.0, self.mouse.1) {
                    self.gfx.fill_round(
                        rr[0],
                        rr[1],
                        rr[0] + rr[2],
                        rr[1] + rr[3],
                        6.,
                        [0.13, 0.19, 0.31, 1.],
                    );
                }
                self.text(
                    e.date,
                    lx + 8.,
                    y + 10.,
                    axis - lx - 24.,
                    20.,
                    Font::Normal,
                    if known { GOLD } else { MUTED },
                );
                self.gfx
                    .fill_circle(axis, y + 20., 7., if found { c } else { LOCKED });
                self.gfx.stroke_circle(
                    axis,
                    y + 20.,
                    7.,
                    1.5,
                    if found { TEXT } else { alpha(MUTED, 0.6) },
                );
                if known {
                    self.text(
                        e.title,
                        axis + 22.,
                        y + 4.,
                        w - axis - 60.,
                        22.,
                        Font::Normal,
                        TEXT,
                    );
                    let place = place_index(e.era, e.place)
                        .map(|k| ERAS[e.era].places[k].name)
                        .unwrap_or("");
                    self.text(
                        place,
                        axis + 22.,
                        y + 24.,
                        w - axis - 60.,
                        16.,
                        Font::Small,
                        MUTED,
                    );
                } else {
                    let place = place_index(e.era, e.place)
                        .map(|k| ERAS[e.era].places[k].name)
                        .unwrap_or("");
                    self.text(
                        "？？？",
                        axis + 22.,
                        y + 4.,
                        w - axis - 60.,
                        22.,
                        Font::Normal,
                        MUTED,
                    );
                    self.text(
                        &format!("{}で観察できる", place),
                        axis + 22.,
                        y + 24.,
                        w - axis - 60.,
                        16.,
                        Font::Small,
                        LOCKED,
                    );
                }
                self.buttons.push(Button {
                    rect: [
                        rr[0],
                        rr[1].max(vy0),
                        rr[2],
                        (rr[1] + rr[3]).min(vy1) - rr[1].max(vy0),
                    ],
                    act: Act::Detail(Entry::Event(*i)),
                });
            }
            y += row;
        }
        if !items.is_empty() {
            self.gfx.fill_rect(
                axis - 1.,
                line_top.max(vy0),
                axis + 1.,
                y.min(vy1),
                alpha(EDGE, 0.5),
            );
        } else {
            self.text(
                "該当する出来事がありません。（検索は発見済みの項目が対象です）",
                18.,
                y0 + 10.,
                w - 36.,
                24.,
                Font::Normal,
                MUTED,
            );
        }
        y - y0 + 10.
    }

    fn detail_ui(&mut self, e: Entry) {
        let need = self.detail_height(e);
        let [x, y, cw, ch] = self.card(880., need);
        let body_y;
        let tw = cw - 60.;
        let (era, place_id, known, found) = match e {
            Entry::Person(i) => (
                PEOPLE[i].era,
                PEOPLE[i].place,
                self.journal.met[i] || self.reveal,
                self.journal.met[i],
            ),
            Entry::Event(i) => (
                EVENTS[i].era,
                EVENTS[i].place,
                self.journal.seen[i] || self.reveal,
                self.journal.seen[i],
            ),
        };
        let place = place_index(era, place_id);
        let place_name = place.map(|k| ERAS[era].places[k].name).unwrap_or("");
        match e {
            Entry::Person(i) => {
                let p = &PEOPLE[i];
                let no = people_order().iter().position(|q| *q == i).unwrap_or(0) + 1;
                self.text(
                    &format!(
                        "人物一覧表　No.{:03}{}",
                        no,
                        if found { "" } else { "（未発見）" }
                    ),
                    x + 30.,
                    y + 16.,
                    tw,
                    18.,
                    Font::Small,
                    if found { GOLD } else { LOCKED },
                );
                self.medallion(p.name, x + 74., y + 86., 42., era_color(era), known);
                let hx = x + 136.;
                if known {
                    self.text(p.name, hx, y + 44., cw - 166., 28., Font::Big, TEXT);
                    self.text(p.native, hx, y + 74., cw - 166., 18., Font::Small, MUTED);
                    self.text(
                        &format!("{}　／　{}", p.life, p.role),
                        hx,
                        y + 96.,
                        cw - 166.,
                        36.,
                        Font::Normal,
                        CYAN,
                    );
                } else {
                    self.text("？？？", hx, y + 54., cw - 166., 28., Font::Big, MUTED);
                    self.text(
                        &format!("ヒント：{}の{}で会える", ERAS[era].name, place_name),
                        hx,
                        y + 90.,
                        cw - 166.,
                        20.,
                        Font::Normal,
                        LOCKED,
                    );
                }
                body_y = y + 144.;
            }
            Entry::Event(i) => {
                let ev = &EVENTS[i];
                let no = chronology().iter().position(|q| *q == i).unwrap_or(0) + 1;
                self.text(
                    &format!(
                        "出来事集　No.{:03}{}",
                        no,
                        if found { "" } else { "（未発見）" }
                    ),
                    x + 30.,
                    y + 16.,
                    tw,
                    18.,
                    Font::Small,
                    if found { GOLD } else { LOCKED },
                );
                self.text(
                    &format!("{}　／　{}　／　{}", ev.date, ERAS[era].name, place_name),
                    x + 30.,
                    y + 40.,
                    tw,
                    18.,
                    Font::Small,
                    GOLD,
                );
                if known {
                    self.text(ev.title, x + 30., y + 60., tw, 30., Font::Big, TEXT);
                } else {
                    self.text("？？？", x + 30., y + 60., tw, 30., Font::Big, MUTED);
                    self.text(
                        &format!("ヒント：{}の{}で観察できる", ERAS[era].name, place_name),
                        x + 30.,
                        y + 92.,
                        tw,
                        20.,
                        Font::Normal,
                        LOCKED,
                    );
                }
                body_y = y + 104.;
            }
        }
        let body_h = y + ch - 60. - body_y;
        self.gfx.fill_rect(
            x + 30.,
            body_y - 6.,
            x + cw - 30.,
            body_y - 5.,
            alpha(EDGE, 0.8),
        );
        if known {
            self.scroll_areas
                .push(([x, body_y, cw, body_h], Scroll::Overlay));
            let off = self.scroll[Scroll::Overlay as usize];
            self.gfx.clip_begin(x, body_y, x + cw, body_y + body_h);
            let mut yy = body_y + 6. - off;
            let (paras, source, links): (Vec<&str>, &str, Vec<Entry>) = match e {
                Entry::Person(i) => (
                    vec![PEOPLE[i].bio],
                    PEOPLE[i].source,
                    chronology()
                        .into_iter()
                        .filter(|k| EVENTS[*k].people.contains(&PEOPLE[i].id))
                        .map(Entry::Event)
                        .collect(),
                ),
                Entry::Event(i) => (
                    EVENTS[i].body.to_vec(),
                    EVENTS[i].source,
                    EVENTS[i]
                        .people
                        .iter()
                        .filter_map(|id| person_index(id))
                        .map(Entry::Person)
                        .collect(),
                ),
            };
            for para in paras {
                let ph = self.gfx.measure_h(para, Font::Normal, tw);
                self.text(para, x + 30., yy, tw, ph, Font::Normal, TEXT);
                yy += ph + 10.;
            }
            self.text(
                &format!("ゆかりの場所：{}（{}）", place_name, ERAS[era].area),
                x + 30.,
                yy,
                tw,
                20.,
                Font::Small,
                MUTED,
            );
            yy += 28.;
            if !links.is_empty() {
                self.text(
                    if matches!(e, Entry::Person(_)) {
                        "関わった出来事"
                    } else {
                        "関わった人物"
                    },
                    x + 30.,
                    yy,
                    tw,
                    18.,
                    Font::Small,
                    MUTED,
                );
                yy += 24.;
                for l in links {
                    let (label, ok) = match l {
                        Entry::Person(k) => (
                            PEOPLE[k].name.to_string(),
                            self.journal.met[k] || self.reveal,
                        ),
                        Entry::Event(k) => (
                            format!("{}　{}", EVENTS[k].date, EVENTS[k].title),
                            self.journal.seen[k] || self.reveal,
                        ),
                    };
                    let label = if ok { label } else { "？？？".to_string() };
                    let rr = [x + 30., yy, tw, 28.];
                    self.gfx
                        .fill_round(rr[0], rr[1], rr[0] + rr[2], rr[1] + rr[3], 6., PANEL2);
                    self.text(
                        &label,
                        x + 42.,
                        yy + 5.,
                        tw - 24.,
                        18.,
                        Font::Normal,
                        if ok { TEXT } else { MUTED },
                    );
                    if yy + 28. > body_y && yy < body_y + body_h {
                        self.buttons.push(Button {
                            rect: [
                                rr[0],
                                rr[1].max(body_y),
                                rr[2],
                                (rr[1] + rr[3]).min(body_y + body_h) - rr[1].max(body_y),
                            ],
                            act: Act::Detail(l),
                        });
                    }
                    yy += 34.;
                }
                yy += 4.;
            }
            let sh = self.gfx.measure_h(source, Font::Small, tw - 40.);
            self.text(
                &format!("史料：{}", source),
                x + 30.,
                yy,
                tw,
                sh + 4.,
                Font::Small,
                MUTED,
            );
            yy += sh + 12.;
            self.scroll_max[Scroll::Overlay as usize] = yy + off - body_y - body_h;
            self.gfx.clip_end();
        }
        if let Some(k) = place {
            self.button(
                "その場所へ転移",
                [x + 24., y + ch - 48., 180., 34.],
                Act::Shift(era, Some(k)),
                false,
            );
        }
        self.button(
            "閉じる",
            [x + cw - 160., y + ch - 48., 136., 34.],
            Act::Close,
            true,
        );
    }

    /// 図鑑の詳しい説明の枠の高さ（中身に合わせる）
    fn detail_height(&mut self, e: Entry) -> f32 {
        let tw = 880.0f32.min(self.gfx.width as f32 - 32.) - 60.;
        let (head, paras, source, links, known): (f32, Vec<&str>, &str, usize, bool) = match e {
            Entry::Person(i) => (
                144.,
                vec![PEOPLE[i].bio],
                PEOPLE[i].source,
                EVENTS
                    .iter()
                    .filter(|k| k.people.contains(&PEOPLE[i].id))
                    .count(),
                self.journal.met[i] || self.reveal,
            ),
            Entry::Event(i) => (
                104.,
                EVENTS[i].body.to_vec(),
                EVENTS[i].source,
                EVENTS[i].people.len(),
                self.journal.seen[i] || self.reveal,
            ),
        };
        if !known {
            return head + 120.;
        }
        let mut need = head + 6. + 28. + 60. + 16.;
        for p in paras {
            need += self.gfx.measure_h(p, Font::Normal, tw) + 10.;
        }
        if links > 0 {
            need += 28. + links as f32 * 34.;
        }
        need + self.gfx.measure_h(source, Font::Small, tw - 40.) + 12.
    }

    fn help_ui(&mut self) {
        let [x, y, cw, ch] = self.card(820., 600.);
        self.text(
            "時空を渡り、歴史に出会う",
            x + 30.,
            y + 20.,
            cw - 60.,
            40.,
            Font::TitleMid,
            GOLD,
        );
        let body = "1　世界地図で行き先の時代を選び、「転移する」でその時代へ渡ります。\n\n2　時代の地図では、場所を押すと道をたどって移動します。矢印キーでも移動できます。「人」の印はまだ会っていない人物、「！」はまだ観察していない出来事のある場所です。\n\n3　人物に「会う」と人物一覧表に、出来事を「観察」すると出来事集と年表に記録されます。\n\n4　図鑑では、人物一覧表・年表・出来事集を見返せます。「未発見も表示」をONにすると、まだ見つけていない項目も読めます（学習用）。\n\n―― 内容の正確さについて ――\n年代・人物・出来事は、各項目に示した史料と研究に基づいてまとめています。日付に諸説あるものや史料によって食い違うものは本文に記しています。和暦の月日は当時の暦（旧暦）のままです。\n人物の言葉は、事実を本人の語りとして再構成した教材であり、「」で示した史料の引用を除いて実際の発言ではありません。人物がいる場所は、その人物にゆかりのある場所です。地図は位置関係を示す概略図で、建物の形や距離は正確ではありません。\n\nF2 図鑑 ／ F3 使い方 ／ F11 全画面 ／ Esc 戻る ／ Enter 次へ・転移";
        let by = y + 72.;
        let bh = ch - 72. - 62.;
        self.scroll_areas.push(([x, by, cw, bh], Scroll::Overlay));
        let off = self.scroll[Scroll::Overlay as usize];
        let th = self.gfx.measure_h(body, Font::Normal, cw - 64.);
        self.gfx.clip_begin(x, by, x + cw, by + bh);
        self.text(body, x + 32., by - off, cw - 64., th, Font::Normal, TEXT);
        self.gfx.clip_end();
        self.scroll_max[Scroll::Overlay as usize] = th - bh + 10.;
        self.button(
            "旅に戻る",
            [x + cw / 2. - 120., y + ch - 50., 240., 36.],
            Act::Close,
            true,
        );
    }
}

fn contains(fields: &[&str], q: &str) -> bool {
    let q = q.to_lowercase();
    fields.iter().any(|f| f.to_lowercase().contains(&q))
}

fn inside(r: [f32; 4], x: f32, y: f32) -> bool {
    x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3]
}

/// 多角形を耳切り法で三角形に分けて塗る
fn fill_poly(g: &mut Gfx, pts: &[(f32, f32)], m: &dyn Fn(f32, f32) -> (f32, f32), c: [f32; 4]) {
    let p: Vec<(f32, f32)> = pts.iter().map(|q| m(q.0, q.1)).collect();
    for [a, b, d] in triangulate(&p) {
        g.tri(p[a], p[b], p[d], c);
    }
}

fn triangulate(p: &[(f32, f32)]) -> Vec<[usize; 3]> {
    let n = p.len();
    if n < 3 {
        return vec![];
    }
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    let mut idx: Vec<usize> = if area > 0. {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };
    let cross = |a: (f32, f32), b: (f32, f32), c: (f32, f32)| {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    };
    let mut out = vec![];
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let k = idx.len();
        let mut cut = None;
        for i in 0..k {
            let (ia, ib, ic) = (idx[(i + k - 1) % k], idx[i], idx[(i + 1) % k]);
            let (a, b, c) = (p[ia], p[ib], p[ic]);
            if cross(a, b, c) <= 0. {
                continue;
            }
            let empty = idx.iter().all(|&j| {
                j == ia || j == ib || j == ic || {
                    let q = p[j];
                    !(cross(a, b, q) >= 0. && cross(b, c, q) >= 0. && cross(c, a, q) >= 0.)
                }
            });
            if empty {
                cut = Some(i);
                out.push([ia, ib, ic]);
                break;
            }
        }
        match cut {
            Some(i) => {
                idx.remove(i);
            }
            None => break,
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

/// 場所の種類の図柄
fn icon(g: &mut Gfx, kind: Kind, x: f32, y: f32, s: f32) {
    let ink = [0.36, 0.24, 0.14, 1.];
    let red = [0.66, 0.24, 0.18, 1.];
    let blue = [0.25, 0.48, 0.62, 1.];
    match kind {
        Kind::Palace => {
            g.rect(x - s * 0.8, y - s * 0.05, x + s * 0.8, y + s * 0.6, red);
            g.tri(
                (x - s, y - s * 0.05),
                (x + s, y - s * 0.05),
                (x, y - s * 0.6),
                ink,
            );
            g.rect(x - s * 0.15, y + s * 0.2, x + s * 0.15, y + s * 0.6, ink);
        }
        Kind::Temple => {
            g.tri(
                (x - s * 0.9, y - s * 0.2),
                (x + s * 0.9, y - s * 0.2),
                (x, y - s * 0.75),
                ink,
            );
            for k in 0..4 {
                let cx = x - s * 0.6 + k as f32 * s * 0.4;
                g.rect(cx - s * 0.08, y - s * 0.15, cx + s * 0.08, y + s * 0.5, ink);
            }
            g.rect(x - s * 0.85, y + s * 0.5, x + s * 0.85, y + s * 0.65, ink);
        }
        Kind::Church => {
            g.rect(x - s * 0.55, y - s * 0.1, x + s * 0.55, y + s * 0.65, red);
            g.circle(x, y - s * 0.1, s * 0.5, 16, red);
            g.rect(x - s * 0.06, y - s * 0.9, x + s * 0.06, y - s * 0.5, ink);
            g.rect(x - s * 0.2, y - s * 0.78, x + s * 0.2, y - s * 0.68, ink);
        }
        Kind::Market => {
            for k in 0..3 {
                let cx = x - s * 0.55 + k as f32 * s * 0.55;
                g.tri(
                    (cx - s * 0.3, y - s * 0.05),
                    (cx + s * 0.3, y - s * 0.05),
                    (cx, y - s * 0.45),
                    if k == 1 { blue } else { red },
                );
                g.rect(cx - s * 0.24, y - s * 0.05, cx + s * 0.24, y + s * 0.5, ink);
            }
        }
        Kind::Gate => {
            g.rect(x - s * 0.9, y - s * 0.3, x + s * 0.9, y + s * 0.6, ink);
            g.rect(
                x - s * 0.3,
                y + s * 0.05,
                x + s * 0.3,
                y + s * 0.6,
                [0.97, 0.93, 0.83, 1.],
            );
            g.tri(
                (x - s, y - s * 0.3),
                (x + s, y - s * 0.3),
                (x, y - s * 0.75),
                red,
            );
        }
        Kind::Forum => {
            g.rect(x - s * 0.9, y + s * 0.45, x + s * 0.9, y + s * 0.6, ink);
            for k in 0..3 {
                let cx = x - s * 0.6 + k as f32 * s * 0.6;
                g.rect(cx - s * 0.1, y - s * 0.4, cx + s * 0.1, y + s * 0.45, ink);
            }
            g.rect(x - s * 0.9, y - s * 0.55, x + s * 0.9, y - s * 0.4, ink);
        }
        Kind::Theater => {
            g.ring(x, y + s * 0.3, s * 0.85, s * 0.35, 20, ink);
            g.rect(
                x - s,
                y + s * 0.3,
                x + s,
                y + s * 0.95,
                [0.97, 0.93, 0.83, 1.],
            );
            g.rect(x - s * 0.7, y + s * 0.3, x + s * 0.7, y + s * 0.45, red);
        }
        Kind::House => {
            g.rect(
                x - s * 0.6,
                y - s * 0.05,
                x + s * 0.6,
                y + s * 0.6,
                [0.62, 0.45, 0.28, 1.],
            );
            g.tri(
                (x - s * 0.85, y - s * 0.05),
                (x + s * 0.85, y - s * 0.05),
                (x, y - s * 0.65),
                red,
            );
            g.rect(x - s * 0.12, y + s * 0.2, x + s * 0.12, y + s * 0.6, ink);
        }
        Kind::Bridge => {
            g.rect(x - s, y + s * 0.25, x + s, y + s * 0.6, blue);
            g.rect(x - s, y - s * 0.15, x + s, y + s * 0.05, ink);
            for k in 0..3 {
                let cx = x - s * 0.6 + k as f32 * s * 0.6;
                g.rect(cx - s * 0.08, y, cx + s * 0.08, y + s * 0.45, ink);
            }
        }
        Kind::Hill => {
            g.tri(
                (x - s, y + s * 0.6),
                (x + s * 0.2, y + s * 0.6),
                (x - s * 0.4, y - s * 0.3),
                [0.45, 0.55, 0.30, 1.],
            );
            g.tri(
                (x - s * 0.3, y + s * 0.6),
                (x + s, y + s * 0.6),
                (x + s * 0.35, y - s * 0.6),
                [0.36, 0.46, 0.24, 1.],
            );
        }
        Kind::Castle => {
            g.rect(
                x - s * 0.9,
                y + s * 0.3,
                x + s * 0.9,
                y + s * 0.6,
                [0.55, 0.52, 0.48, 1.],
            );
            g.rect(
                x - s * 0.5,
                y - s * 0.1,
                x + s * 0.5,
                y + s * 0.3,
                [0.95, 0.95, 0.92, 1.],
            );
            g.tri(
                (x - s * 0.75, y - s * 0.1),
                (x + s * 0.75, y - s * 0.1),
                (x, y - s * 0.45),
                ink,
            );
            g.rect(
                x - s * 0.3,
                y - s * 0.55,
                x + s * 0.3,
                y - s * 0.3,
                [0.95, 0.95, 0.92, 1.],
            );
            g.tri(
                (x - s * 0.5, y - s * 0.5),
                (x + s * 0.5, y - s * 0.5),
                (x, y - s * 0.85),
                ink,
            );
        }
        Kind::Fortress => {
            g.rect(
                x - s * 0.8,
                y - s * 0.3,
                x + s * 0.8,
                y + s * 0.6,
                [0.48, 0.44, 0.40, 1.],
            );
            for k in 0..4 {
                let cx = x - s * 0.7 + k as f32 * s * 0.46;
                g.rect(
                    cx - s * 0.12,
                    y - s * 0.55,
                    cx + s * 0.12,
                    y - s * 0.3,
                    [0.48, 0.44, 0.40, 1.],
                );
            }
            g.rect(x - s * 0.2, y + s * 0.15, x + s * 0.2, y + s * 0.6, ink);
        }
        Kind::Hall => {
            g.rect(
                x - s * 0.85,
                y - s * 0.2,
                x + s * 0.85,
                y + s * 0.6,
                [0.70, 0.58, 0.42, 1.],
            );
            g.rect(x - s * 0.95, y - s * 0.4, x + s * 0.95, y - s * 0.2, ink);
            for k in 0..3 {
                let cx = x - s * 0.5 + k as f32 * s * 0.5;
                g.rect(
                    cx - s * 0.12,
                    y,
                    cx + s * 0.12,
                    y + s * 0.35,
                    [0.30, 0.42, 0.55, 1.],
                );
            }
        }
        Kind::Water => {
            g.circle(x, y + s * 0.15, s * 0.75, 18, blue);
            g.line(
                (x - s * 0.45, y + s * 0.05),
                (x + s * 0.1, y + s * 0.05),
                1.6,
                [0.9, 0.96, 1., 1.],
            );
            g.line(
                (x - s * 0.1, y + s * 0.4),
                (x + s * 0.45, y + s * 0.4),
                1.6,
                [0.9, 0.96, 1., 1.],
            );
        }
    }
    let _ = INK;
    let _ = INK_SOFT;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(g: &mut Game, secs: f32) {
        let mut t = 0.;
        while t < secs {
            g.frame(&[], 0.05, true);
            t += 0.05;
        }
    }

    #[test]
    fn journal_roundtrip_and_corruption() {
        let mut j = Journal::new();
        j.era = EDO;
        j.place = 3;
        j.visited[EDO] = true;
        j.met[5] = true;
        j.seen[7] = true;
        let b = j.bytes();
        assert_eq!(Journal::read(&b).unwrap(), j);
        for n in 0..b.len() {
            assert!(Journal::read(&b[..n]).is_err());
        }
        let mut bad = b.clone();
        bad[14] ^= 1;
        assert!(Journal::read(&bad).is_err());
    }

    #[test]
    fn every_person_and_event_can_be_found() {
        let mut g = Game::new(Gfx::new(1600, 900));
        for era in 0..ERAS.len() {
            g.act(Act::PickEra(era));
            g.act(Act::Shift(era, None));
            assert_eq!(g.mode, Mode::Shift);
            run(&mut g, 2.);
            assert_eq!(g.mode, Mode::Field);
            assert_eq!(g.journal.era, era);
            for place in 0..ERAS[era].places.len() {
                g.act(Act::Goto(place));
                run(&mut g, 12.);
                assert!(g.travel.is_none());
                assert_eq!(g.journal.place, place);
                for i in g.people_here() {
                    g.act(Act::Meet(i));
                    assert_eq!(g.overlay, Overlay::Meet(i, 0));
                    for _ in 0..PEOPLE[i].lines.len() {
                        g.render();
                        g.act(Act::Next);
                    }
                    assert_eq!(g.overlay, Overlay::None);
                }
                for i in g.events_here() {
                    g.act(Act::Observe(i));
                    g.render();
                    g.act(Act::Close);
                }
            }
        }
        assert!(g.journal.met.iter().all(|m| *m));
        assert!(g.journal.seen.iter().all(|m| *m));
        assert!(g.journal.visited.iter().all(|m| *m));
        let b = g.journal.bytes();
        g.journal = Journal::new();
        g.apply_loaded(&b);
        assert_eq!(g.met_count(None), PEOPLE.len());
        assert_eq!(g.seen_count(None), EVENTS.len());
    }

    #[test]
    fn meeting_requires_being_there() {
        let mut g = Game::new(Gfx::new(1600, 900));
        let caesar = person_index("caesar").unwrap();
        g.act(Act::Meet(caesar));
        assert!(!g.journal.met[caesar]);
        g.act(Act::Shift(ROME, None));
        run(&mut g, 2.);
        g.act(Act::Meet(caesar));
        assert!(
            !g.journal.met[caesar],
            "Caesar is at the Domus Publica, not the Forum"
        );
        let ides = event_index("ides").unwrap();
        g.act(Act::Observe(ides));
        assert!(!g.journal.seen[ides]);
        let domus = place_index(ROME, "domus").unwrap();
        g.act(Act::Goto(domus));
        g.frame(&[], 0.05, true);
        g.act(Act::Meet(caesar));
        assert!(!g.journal.met[caesar], "cannot meet while travelling");
        run(&mut g, 10.);
        g.act(Act::Meet(caesar));
        assert!(g.journal.met[caesar]);
    }

    #[test]
    fn arrow_keys_follow_roads() {
        let mut g = Game::new(Gfx::new(1600, 900));
        g.act(Act::Shift(ROME, None));
        run(&mut g, 2.);
        // フォルムから西へ：ポンペイウス劇場（道でつながる）かカピトリウム
        g.handle(Ev::Key(37, true));
        run(&mut g, 10.);
        let id = ERAS[ROME].places[g.journal.place].id;
        assert!(id == "capitol" || id == "pompey", "{id}");
    }

    #[test]
    fn all_views_render_and_search_filters() {
        for (w, h, c) in [(1600, 900, false), (1360, 720, false), (900, 420, true)] {
            let mut g = Game::new(Gfx::new(w, h));
            g.gfx.set_compact(c);
            g.frame(&[], 0.016, true);
            for era in 0..ERAS.len() {
                g.act(Act::Shift(era, None));
                run(&mut g, 2.);
                g.frame(&[], 0.016, true);
            }
            g.act(Act::Codex);
            for t in [Tab::People, Tab::Timeline, Tab::Events] {
                g.act(Act::Tab(t));
                for f in std::iter::once(None).chain((0..ERAS.len()).map(Some)) {
                    g.act(Act::Filter(f));
                    g.frame(&[], 0.016, true);
                }
            }
            g.act(Act::Detail(Entry::Person(0)));
            g.render();
            g.act(Act::Detail(Entry::Event(0)));
            g.render();
            g.act(Act::Help);
            g.render();
            g.act(Act::Close);
            // 検索は発見済みの項目だけが対象
            g.act(Act::Filter(None));
            g.searching = true;
            for ch in "カエサル".chars() {
                g.handle(Ev::Char(ch));
            }
            assert_eq!(
                (0..PEOPLE.len())
                    .filter(|i| g.codex_match_person(*i))
                    .count(),
                0
            );
            g.act(Act::Reveal);
            assert!(
                (0..PEOPLE.len())
                    .filter(|i| g.codex_match_person(*i))
                    .count()
                    >= 1
            );
            assert!(
                (0..EVENTS.len())
                    .filter(|i| g.codex_match_event(*i))
                    .count()
                    >= 1
            );
            g.handle(Ev::Key(8, true));
            assert_eq!(g.query.chars().count(), 3);
        }
    }

    #[test]
    fn polygons_triangulate() {
        for era in ERAS {
            for f in era.features {
                if let Feature::Water(p) | Feature::Land(p) = f {
                    assert_eq!(triangulate(p).len(), p.len() - 2, "{}", era.id);
                }
            }
        }
    }

    #[test]
    fn chronology_is_ordered() {
        let c = chronology();
        assert!(c.windows(2).all(|w| event_key(w[0]) <= event_key(w[1])));
        assert_eq!(EVENTS[c[0]].id, "lupercalia");
    }
}
