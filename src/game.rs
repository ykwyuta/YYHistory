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
    Treasures,
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
    /// 試練の始まり（時代）
    TrialIntro(usize),
    /// 試練の問題（時代, 何問目, 選んだ答え, 正解数）
    Trial(usize, usize, Option<usize>, usize),
    /// 試練の結果（時代, 正解数）
    TrialResult(usize, usize),
    /// 秘宝の説明（時代）
    Treasure(usize),
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
    TrialIntro(usize),
    Begin(usize),
    Answer(usize),
    TrialNext,
    Treasure(usize),
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
    /// 試練を突破して手に入れた秘宝（時代ごと）
    treasures: Vec<bool>,
}

const MAGIC: &[u8; 8] = b"YYHIST\x03\0";
/// 秘宝の無い前の版の記録
const MAGIC_V2: &[u8; 8] = b"YYHIST\x02\0";

fn bits(v: &[bool]) -> u8 {
    v.iter()
        .enumerate()
        .filter(|(_, x)| **x)
        .fold(0u8, |a, (i, _)| a | 1 << i)
}

impl Journal {
    fn new() -> Self {
        Self {
            era: 0,
            place: ERAS[0].start,
            visited: vec![false; ERAS.len()],
            met: vec![false; PEOPLE.len()],
            seen: vec![false; EVENTS.len()],
            treasures: vec![false; ERAS.len()],
        }
    }
    fn bytes(&self) -> Vec<u8> {
        let mut b = MAGIC.to_vec();
        b.push(self.era as u8);
        b.push(self.place as u8);
        b.push(bits(&self.visited));
        b.push(bits(&self.treasures));
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
        let v3 = b.len() >= 8 && &b[..8] == MAGIC;
        if b.len() < 19 || b.len() > 65536 || !(v3 || &b[..8] == MAGIC_V2) {
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
        let tb = if v3 { b[11] } else { 0 };
        if tb >> ERAS.len() != 0 {
            return Err(invalid());
        }
        let treasures = (0..ERAS.len()).map(|i| tb & (1 << i) != 0).collect();
        let mut at = if v3 { 12 } else { 11 };
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
            treasures,
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
    /// この起動中に試練の案内を出した時代
    trial_offered: Vec<bool>,
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
            trial_offered: vec![false; ERAS.len()],
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

    /// その時代の人物と出来事をすべて集めたか
    fn era_complete(&self, era: usize) -> bool {
        PEOPLE
            .iter()
            .zip(&self.journal.met)
            .all(|(p, m)| p.era != era || *m)
            && EVENTS
                .iter()
                .zip(&self.journal.seen)
                .all(|(e, m)| e.era != era || *m)
    }
    fn treasure_count(&self) -> usize {
        self.journal.treasures.iter().filter(|t| **t).count()
    }

    /// 会話や観察を閉じたとき：その時代の情報がそろったら試練を始める
    fn close_overlay(&mut self) {
        self.overlay = Overlay::None;
        platform::speak("");
        let era = self.journal.era;
        if self.mode == Mode::Field
            && self.era_complete(era)
            && !self.journal.treasures[era]
            && !self.trial_offered[era]
        {
            self.trial_offered[era] = true;
            self.overlay = Overlay::TrialIntro(era);
            if self.voice {
                platform::speak(
                    "この時代の記録はすべて集まった。アカシックレコードの試練が始まる。",
                );
            }
        }
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
                        self.close_overlay();
                    }
                }
                Overlay::None => (),
                Overlay::TrialIntro(era) => self.act(Act::Begin(era)),
                Overlay::Trial(_, _, Some(_), _) => self.act(Act::TrialNext),
                Overlay::Trial(..) => (),
                _ => self.close_overlay(),
            },
            Act::Prev => {
                if let Overlay::Meet(i, page) = self.overlay {
                    self.overlay = Overlay::Meet(i, page.saturating_sub(1));
                }
            }
            Act::Close => self.close_overlay(),
            Act::TrialIntro(era) => {
                if self.era_complete(era) {
                    self.overlay = Overlay::TrialIntro(era);
                } else {
                    self.notify("この時代の人物と出来事をすべて集めると、試練に挑める。");
                }
            }
            Act::Begin(era) => {
                if self.era_complete(era) {
                    self.overlay = Overlay::Trial(era, 0, None, 0);
                    self.speak_question();
                }
            }
            Act::Answer(k) => {
                if let Overlay::Trial(era, q, None, score) = self.overlay {
                    let quiz = &QUIZZES[quizzes(era)[q]];
                    let ok = k == quiz.answer;
                    self.overlay = Overlay::Trial(era, q, Some(k), score + ok as usize);
                    if self.voice {
                        platform::speak(&format!(
                            "{}。{}",
                            if ok { "正解" } else { "不正解" },
                            quiz.note
                        ));
                    }
                }
            }
            Act::TrialNext => {
                if let Overlay::Trial(era, q, Some(_), score) = self.overlay {
                    let n = quizzes(era).len();
                    if q + 1 < n {
                        self.overlay = Overlay::Trial(era, q + 1, None, score);
                        self.speak_question();
                    } else {
                        self.overlay = Overlay::TrialResult(era, score);
                        if score == n {
                            let first = !self.journal.treasures[era];
                            self.journal.treasures[era] = true;
                            self.save();
                            if first {
                                self.notify(&format!("秘宝を手に入れた：{}", treasure(era).name));
                            }
                        }
                        if self.voice {
                            platform::speak(&if score == n {
                                format!("全問正解。秘宝、{}を手に入れた。", treasure(era).name)
                            } else {
                                format!("{}問中{}問正解。", n, score)
                            });
                        }
                    }
                }
            }
            Act::Treasure(era) => {
                self.overlay = Overlay::Treasure(era);
                self.scroll[Scroll::Overlay as usize] = 0.;
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

    fn speak_question(&self) {
        if let (true, Overlay::Trial(era, q, None, _)) = (self.voice, self.overlay) {
            let quiz = &QUIZZES[quizzes(era)[q]];
            platform::speak(&format!(
                "第{}問。{}。1、{}。2、{}。3、{}。4、{}。",
                q + 1,
                quiz.q,
                quiz.choices[0],
                quiz.choices[1],
                quiz.choices[2],
                quiz.choices[3]
            ));
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
                if let Overlay::Trial(_, _, None, _) = self.overlay {
                    match k {
                        49..=52 => return self.act(Act::Answer((k - 49) as usize)),
                        97..=100 => return self.act(Act::Answer((k - 97) as usize)),
                        _ => (),
                    }
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
            Overlay::TrialIntro(era) => self.trial_intro_ui(era),
            Overlay::Trial(era, q, pick, score) => self.trial_ui(era, q, pick, score),
            Overlay::TrialResult(era, score) => self.trial_result_ui(era, score),
            Overlay::Treasure(era) => self.treasure_ui(era),
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
            "人物 {}/{}　出来事 {}/{}　秘宝 {}/{}",
            self.met_count(None),
            PEOPLE.len(),
            self.seen_count(None),
            EVENTS.len(),
            self.treasure_count(),
            ERAS.len()
        );
        let right = w - if compact { 200. } else { 330. };
        self.text(&stat, right - 300., by + 8., 290., 22., Font::Normal, CYAN);
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
            let pct = if self.journal.treasures[i] {
                "◆秘宝".to_string()
            } else {
                format!("{}%", done * 100 / all.max(1))
            };
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
            if self.era_complete(ei) {
                let got = self.journal.treasures[ei];
                let rr = [ix, y, iw, 58.];
                let glow = 0.5 + 0.5 * (self.clock * 2.5).sin();
                self.gfx.fill_round(
                    rr[0],
                    rr[1],
                    rr[0] + iw,
                    rr[1] + 58.,
                    8.,
                    mix(
                        [0.10, 0.22, 0.32, 1.],
                        [0.16, 0.34, 0.44, 1.],
                        if got { 0. } else { glow },
                    ),
                );
                self.gfx.stroke_round(
                    rr[0],
                    rr[1],
                    rr[0] + iw,
                    rr[1] + 58.,
                    8.,
                    1.5,
                    if got { GOLD } else { CYAN },
                );
                if got {
                    self.text(
                        "◆ この時代の秘宝",
                        ix + 12.,
                        y + 6.,
                        iw - 24.,
                        18.,
                        Font::Small,
                        GOLD,
                    );
                    self.text(
                        treasure(ei).name,
                        ix + 12.,
                        y + 26.,
                        iw - 24.,
                        24.,
                        Font::Big,
                        TEXT,
                    );
                    self.buttons.push(Button {
                        rect: rr,
                        act: Act::Treasure(ei),
                    });
                } else {
                    self.text(
                        "記録がすべてそろった",
                        ix + 12.,
                        y + 6.,
                        iw - 24.,
                        18.,
                        Font::Small,
                        CYAN,
                    );
                    self.text(
                        "アカシックレコードの試練に挑む",
                        ix + 12.,
                        y + 26.,
                        iw - 24.,
                        24.,
                        Font::Big,
                        TEXT,
                    );
                    self.buttons.push(Button {
                        rect: rr,
                        act: Act::TrialIntro(ei),
                    });
                }
                y += 70.;
            }
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
            ("秘宝", Tab::Treasures),
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
        let rx = 16. + 4. * (tw + 8.);
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
            Tab::Treasures => self.codex_treasures(gy - off, w, gy, h),
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

    fn codex_treasures(&mut self, y0: f32, w: f32, vy0: f32, vy1: f32) -> f32 {
        let items: Vec<usize> = (0..ERAS.len())
            .filter(|i| self.filter.is_none_or(|f| f == *i))
            .collect();
        self.text(
            &format!("手に入れた秘宝 {}/{}", self.treasure_count(), ERAS.len()),
            18.,
            y0 + 2.,
            400.,
            20.,
            Font::Normal,
            CYAN,
        );
        let cols = (((w - 32.) / 330.) as usize).max(2);
        let cw = (w - 32. - (cols - 1) as f32 * 10.) / cols as f32;
        let ch = 150.;
        let y0 = y0 + 28.;
        for (k, era) in items.iter().enumerate() {
            let x = 16. + (k % cols) as f32 * (cw + 10.);
            let y = y0 + (k / cols) as f32 * (ch + 10.);
            if y + ch < vy0 || y > vy1 {
                continue;
            }
            let got = self.journal.treasures[*era];
            let known = got || self.reveal;
            let c = era_color(*era);
            let t = treasure(*era);
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
                1.5,
                if got { GOLD } else { alpha(LOCKED, 0.7) },
            );
            self.gem(x + 50., y + 70., 30., c, known);
            let tx = x + 96.;
            let tw = cw - 108.;
            self.text(
                &format!("{}　{}", ERAS[*era].years, ERAS[*era].name),
                tx,
                y + 12.,
                tw,
                18.,
                Font::Small,
                alpha(c, 0.95),
            );
            if known {
                self.text(t.name, tx, y + 34., tw, 26., Font::Big, TEXT);
                self.gfx
                    .clip_begin(x, y.max(vy0), x + cw, (y + ch - 8.).min(vy1));
                self.text(t.desc, tx, y + 64., tw, 72., Font::Small, MUTED);
                self.gfx.clip_end();
            } else {
                self.text("？？？", tx, y + 34., tw, 26., Font::Big, MUTED);
                let hint = if self.era_complete(*era) {
                    "アカシックレコードの試練に全問正解すると手に入る"
                } else {
                    "この時代の人物と出来事をすべて集めると、試練に挑める"
                };
                self.text(hint, tx, y + 66., tw, 40., Font::Small, LOCKED);
            }
            self.buttons.push(Button {
                rect: [x, y.max(vy0), cw, (y + ch).min(vy1) - y.max(vy0)],
                act: Act::Treasure(*era),
            });
        }
        28. + items.len().div_ceil(cols) as f32 * (ch + 10.)
    }

    /// 秘宝の印（輝く宝石）
    fn gem(&mut self, x: f32, y: f32, r: f32, c: [f32; 4], lit: bool) {
        if lit {
            let p = 0.5 + 0.5 * (self.clock * 2.).sin();
            self.gfx.fill_circle(x, y, r + 8. + p * 3., alpha(c, 0.18));
        }
        self.gfx.fill_circle(
            x,
            y,
            r,
            if lit {
                mix(NIGHT, c, 0.75)
            } else {
                [0.13, 0.17, 0.25, 1.]
            },
        );
        self.gfx
            .stroke_circle(x, y, r, 2., if lit { GOLD } else { LOCKED });
        self.gfx.stroke_circle(
            x,
            y,
            r * 0.62,
            1.,
            alpha(if lit { TEXT } else { MUTED }, 0.5),
        );
        self.text(
            if lit { "◆" } else { "？" },
            x - r,
            y - r,
            r * 2.,
            r * 2.,
            Font::BigCenter,
            if lit { TEXT } else { MUTED },
        );
    }

    /// 試練の背景（星の巡る記録の輪）
    fn akashic_backdrop(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        self.gfx.fill_rect(0., 0., w, h, [0.01, 0.03, 0.08, 0.86]);
        let (cx, cy) = (w / 2., h / 2.);
        for k in 0..5 {
            let r = 120. + k as f32 * 90. + (self.clock * 0.6 + k as f32).sin() * 6.;
            self.gfx
                .stroke_circle(cx, cy, r, 1., alpha(CYAN, 0.10 + 0.04 * (4 - k) as f32));
        }
        for k in 0..24 {
            let a = k as f32 / 24. * std::f32::consts::TAU + self.clock * 0.05;
            let r = 300. + (k % 3) as f32 * 60.;
            self.gfx.fill_circle(
                cx + a.cos() * r,
                cy + a.sin() * r * 0.6,
                1.6,
                alpha(CYAN, 0.5),
            );
        }
    }

    fn trial_card(&mut self, cw: f32, ch: f32) -> [f32; 4] {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        self.akashic_backdrop();
        let cw = cw.min(w - 32.);
        let ch = ch.min(h - 24.);
        let (x, y) = ((w - cw) / 2., (h - ch) / 2.);
        self.gfx
            .fill_round(x, y, x + cw, y + ch, 12., [0.05, 0.10, 0.18, 0.97]);
        self.gfx
            .stroke_round(x + 0.5, y + 0.5, x + cw - 0.5, y + ch - 0.5, 12., 1.5, CYAN);
        self.gfx.stroke_round(
            x + 5.,
            y + 5.,
            x + cw - 5.,
            y + ch - 5.,
            9.,
            1.,
            alpha(GOLD, 0.5),
        );
        [x, y, cw, ch]
    }

    fn trial_intro_ui(&mut self, era: usize) {
        let compact = self.gfx.compact;
        let [x, y, cw, ch] = self.trial_card(760., if compact { 420. } else { 460. });
        self.text(
            "― アカシックレコードの試練 ―",
            x,
            y + 26.,
            cw,
            26.,
            Font::BigCenter,
            CYAN,
        );
        self.text(
            &format!("{}　{}", ERAS[era].years, ERAS[era].name),
            x,
            y + 60.,
            cw,
            22.,
            Font::NormalCenter,
            GOLD,
        );
        self.gem(
            x + cw / 2.,
            y + if compact { 130. } else { 150. },
            34.,
            era_color(era),
            false,
        );
        let body = format!(
            "この時代の人物と出来事の記録が、すべてそろった。\nアカシックレコードが、あなたの理解を確かめようとしている。\n\n{}についての四択問題が{}問。全問正解すると、この時代の秘宝が手に入る。\n間違えても、何度でも挑み直せる。",
            ERAS[era].name,
            quizzes(era).len()
        );
        let by = y + if compact { 182. } else { 206. };
        self.text(
            &body,
            x + 40.,
            by,
            cw - 80.,
            ch - (by - y) - 70.,
            Font::NormalCenter,
            TEXT,
        );
        self.button(
            "あとで挑む",
            [x + 30., y + ch - 54., 160., 38.],
            Act::Close,
            false,
        );
        self.button(
            "試練を始める",
            [x + cw - 250., y + ch - 54., 220., 38.],
            Act::Begin(era),
            true,
        );
    }

    fn trial_ui(&mut self, era: usize, q: usize, pick: Option<usize>, score: usize) {
        let compact = self.gfx.compact;
        let list = quizzes(era);
        let quiz = &QUIZZES[list[q]];
        let row = if compact { 42. } else { 50. };
        let cw0 = 860.0f32.min(self.gfx.width as f32 - 32.);
        let qh = self.gfx.measure_h(quiz.q, Font::Big, cw0 - 60.).max(26.);
        let nh = self.gfx.measure_h(quiz.note, Font::Normal, cw0 - 60.) + 24.;
        let need = 46. + 18. + qh + 4. * row + 34. + nh + 64.;
        let [x, y, cw, ch] = self.trial_card(860., need);
        self.text(
            &format!(
                "アカシックレコードの試練　｜　{}　第{}問 / {}",
                ERAS[era].name,
                q + 1,
                list.len()
            ),
            x + 30.,
            y + 18.,
            cw - 160.,
            20.,
            Font::Normal,
            CYAN,
        );
        let qy = y + 46.;
        self.gfx
            .fill_rect(x + 30., qy - 4., x + cw - 30., qy - 3., alpha(CYAN, 0.4));
        self.text(
            &format!("{}", quiz.q),
            x + 30.,
            qy + 6.,
            cw - 60.,
            qh,
            Font::Big,
            TEXT,
        );
        // 進み具合
        for k in 0..list.len() {
            let cx = x + cw - 40. - (list.len() - 1 - k) as f32 * 20.;
            let c = if k < q || (k == q && pick.is_some()) {
                GOLD
            } else if k == q {
                CYAN
            } else {
                LOCKED
            };
            self.gfx.fill_circle(cx, y + 28., 5., c);
        }
        let mut cy = qy + 18. + qh;
        for (k, choice) in quiz.choices.iter().enumerate() {
            let rr = [x + 30., cy, cw - 60., row - 8.];
            let hover = pick.is_none() && inside(rr, self.mouse.0, self.mouse.1);
            let (fill, edge) = match pick {
                Some(_) if k == quiz.answer => ([0.12, 0.36, 0.28, 1.], [0.45, 0.90, 0.62, 1.]),
                Some(p) if p == k => ([0.40, 0.14, 0.14, 1.], [0.95, 0.45, 0.40, 1.]),
                Some(_) => ([0.08, 0.12, 0.20, 1.], alpha(EDGE, 0.5)),
                None if hover => ([0.18, 0.28, 0.42, 1.], CYAN),
                None => (PANEL2, EDGE),
            };
            self.gfx
                .fill_round(rr[0], rr[1], rr[0] + rr[2], rr[1] + rr[3], 8., fill);
            self.gfx
                .stroke_round(rr[0], rr[1], rr[0] + rr[2], rr[1] + rr[3], 8., 1.2, edge);
            self.gfx
                .fill_circle(rr[0] + 22., rr[1] + rr[3] / 2., 13., alpha(edge, 0.9));
            self.text(
                &format!("{}", k + 1),
                rr[0] + 9.,
                rr[1] + rr[3] / 2. - 13.,
                26.,
                26.,
                Font::NormalCenter,
                NIGHT,
            );
            self.text(
                choice,
                rr[0] + 46.,
                rr[1] + 2.,
                rr[2] - 56.,
                rr[3] - 4.,
                Font::NormalCenter,
                TEXT,
            );
            if pick.is_none() {
                self.buttons.push(Button {
                    rect: rr,
                    act: Act::Answer(k),
                });
            }
            cy += row;
        }
        if let Some(p) = pick {
            let ok = p == quiz.answer;
            let ny = cy + 2.;
            self.text(
                if ok { "正解！" } else { "不正解…" },
                x + 30.,
                ny,
                120.,
                24.,
                Font::Big,
                if ok {
                    [0.55, 0.95, 0.68, 1.]
                } else {
                    [0.98, 0.55, 0.50, 1.]
                },
            );
            let note = if ok {
                quiz.note.to_string()
            } else {
                format!("正解は「{}」。{}", quiz.choices[quiz.answer], quiz.note)
            };
            self.text(
                &note,
                x + 30.,
                ny + 28.,
                cw - 60.,
                y + ch - 60. - ny - 28.,
                Font::Normal,
                MUTED,
            );
            let last = q + 1 >= list.len();
            self.button(
                if last {
                    "結果を見る"
                } else {
                    "次の問題へ"
                },
                [x + cw - 230., y + ch - 50., 200., 36.],
                Act::TrialNext,
                true,
            );
        } else {
            self.text(
                "1〜4のキーでも答えられます",
                x + 30.,
                y + ch - 42.,
                300.,
                20.,
                Font::Small,
                MUTED,
            );
        }
        self.text(
            &format!("正解 {}", score),
            x + cw / 2. - 60.,
            y + ch - 42.,
            120.,
            20.,
            Font::NormalCenter,
            MUTED,
        );
    }

    fn trial_result_ui(&mut self, era: usize, score: usize) {
        let n = quizzes(era).len();
        let win = score == n;
        let compact = self.gfx.compact;
        let dh = self.gfx.measure_h(
            treasure(era).desc,
            Font::Normal,
            800.0f32.min(self.gfx.width as f32 - 32.) - 80.,
        );
        let top = if compact { 122. } else { 140. };
        let [x, y, cw, ch] = self.trial_card(800., if win { top + 72. + dh + 76. } else { 340. });
        self.text("― 試練の結果 ―", x, y + 22., cw, 24., Font::BigCenter, CYAN);
        self.text(
            &format!("{}問中 {}問 正解", n, score),
            x,
            y + 54.,
            cw,
            30.,
            Font::BigCenter,
            if win { GOLD } else { TEXT },
        );
        if win {
            let t = treasure(era);
            let gy = y + top;
            self.gem(x + 80., gy + 20., 40., era_color(era), true);
            self.text(
                "秘宝を手に入れた",
                x + 140.,
                gy - 16.,
                cw - 170.,
                18.,
                Font::Small,
                GOLD,
            );
            self.text(t.name, x + 140., gy + 4., cw - 170., 30., Font::Big, TEXT);
            let dy = gy + 72.;
            self.gfx.clip_begin(x, dy, x + cw, y + ch - 60.);
            self.text(
                t.desc,
                x + 40.,
                dy,
                cw - 80.,
                y + ch - 60. - dy,
                Font::Normal,
                MUTED,
            );
            self.gfx.clip_end();
            self.button(
                "図鑑の秘宝を見る",
                [x + 30., y + ch - 50., 190., 36.],
                Act::Treasure(era),
                false,
            );
            self.button(
                "旅に戻る",
                [x + cw - 210., y + ch - 50., 180., 36.],
                Act::Close,
                true,
            );
        } else {
            let body = "全問正解には届かなかった。\n\nこの時代の人物の話や出来事の記録を、図鑑で読み返してから挑み直そう。\n間違えた問題の解説にも、ヒントがある。";
            self.text(
                body,
                x + 50.,
                y + 100.,
                cw - 100.,
                ch - 170.,
                Font::NormalCenter,
                TEXT,
            );
            self.button(
                "図鑑で復習する",
                [x + 30., y + ch - 50., 180., 36.],
                Act::Codex,
                false,
            );
            self.button(
                "もう一度挑む",
                [x + cw / 2. - 90., y + ch - 50., 180., 36.],
                Act::Begin(era),
                true,
            );
            self.button(
                "旅に戻る",
                [x + cw - 190., y + ch - 50., 160., 36.],
                Act::Close,
                false,
            );
        }
    }

    fn treasure_ui(&mut self, era: usize) {
        let got = self.journal.treasures[era];
        let known = got || self.reveal;
        let t = treasure(era);
        let tw = 760.0f32.min(self.gfx.width as f32 - 32.) - 80.;
        let need = if known {
            220. + self.gfx.measure_h(t.desc, Font::Normal, tw)
                + self.gfx.measure_h(t.source, Font::Small, tw)
        } else {
            300.
        };
        let [x, y, cw, ch] = self.card(760., need);
        self.text(
            &format!(
                "秘宝　{}　{}{}",
                ERAS[era].years,
                ERAS[era].name,
                if got { "" } else { "（未獲得）" }
            ),
            x + 30.,
            y + 18.,
            cw - 60.,
            18.,
            Font::Small,
            if got { GOLD } else { LOCKED },
        );
        self.gem(x + 74., y + 90., 40., era_color(era), known);
        if known {
            self.text(t.name, x + 136., y + 72., cw - 166., 30., Font::Big, TEXT);
            let dy = y + 150.;
            let dh = self.gfx.measure_h(t.desc, Font::Normal, cw - 80.);
            self.text(t.desc, x + 40., dy, cw - 80., dh, Font::Normal, TEXT);
            self.text(
                &format!("史料・所蔵：{}", t.source),
                x + 40.,
                dy + dh + 10.,
                cw - 80.,
                40.,
                Font::Small,
                MUTED,
            );
        } else {
            self.text(
                "？？？",
                x + 136.,
                y + 72.,
                cw - 166.,
                30.,
                Font::Big,
                MUTED,
            );
            self.text(
                "この時代の人物と出来事をすべて集め、アカシックレコードの試練に全問正解すると手に入る。",
                x + 40., y + 150., cw - 80., 60., Font::Normal, LOCKED,
            );
        }
        self.button(
            "閉じる",
            [x + cw - 160., y + ch - 48., 136., 34.],
            Act::Close,
            true,
        );
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
        let body = "1　世界地図で行き先の時代を選び、「転移する」でその時代へ渡ります。\n\n2　時代の地図では、場所を押すと道をたどって移動します。矢印キーでも移動できます。「人」の印はまだ会っていない人物、「！」はまだ観察していない出来事のある場所です。\n\n3　人物に「会う」と人物一覧表に、出来事を「観察」すると出来事集と年表に記録されます。\n\n4　その時代の人物と出来事をすべて集めると「アカシックレコードの試練」が始まります。時代についての四択問題5問に全問正解すると、その時代にゆかりのある実在の品が「秘宝」として手に入ります。何度でも挑み直せます。\n\n5　図鑑では、人物一覧表・年表・出来事集・秘宝を見返せます。「未発見も表示」をONにすると、まだ見つけていない項目も読めます（学習用）。\n\n―― 内容の正確さについて ――\n年代・人物・出来事は、各項目に示した史料と研究に基づいてまとめています。日付に諸説あるものや史料によって食い違うものは本文に記しています。和暦の月日は当時の暦（旧暦）のままです。\n人物の言葉は、事実を本人の語りとして再構成した教材であり、「」で示した史料の引用を除いて実際の発言ではありません。人物がいる場所は、その人物にゆかりのある場所です。地図は位置関係を示す概略図で、建物の形や距離は正確ではありません。\n\nF2 図鑑 ／ F3 使い方 ／ F11 全画面 ／ Esc 戻る ／ Enter 次へ・転移";
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
        j.treasures[2] = true;
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
                    assert!(matches!(g.overlay, Overlay::None | Overlay::TrialIntro(_)));
                }
                for i in g.events_here() {
                    g.act(Act::Observe(i));
                    g.render();
                    g.act(Act::Close);
                }
                if !g.era_complete(era) {
                    assert_eq!(g.overlay, Overlay::None, "trial only after everything");
                }
            }
            // 情報がそろうと試練が始まる
            assert!(g.era_complete(era));
            assert_eq!(g.overlay, Overlay::TrialIntro(era));
            g.render();
            pass_trial(&mut g, era, true);
            assert!(g.journal.treasures[era]);
            g.act(Act::Close);
        }
        assert!(g.journal.treasures.iter().all(|t| *t));
        assert!(g.journal.met.iter().all(|m| *m));
        assert!(g.journal.seen.iter().all(|m| *m));
        assert!(g.journal.visited.iter().all(|m| *m));
        let b = g.journal.bytes();
        g.journal = Journal::new();
        g.apply_loaded(&b);
        assert_eq!(g.met_count(None), PEOPLE.len());
        assert_eq!(g.seen_count(None), EVENTS.len());
    }

    /// 試練を最後まで答える（perfect=false なら一問だけ間違える）
    fn pass_trial(g: &mut Game, era: usize, perfect: bool) {
        g.act(Act::Begin(era));
        let n = quizzes(era).len();
        for q in 0..n {
            assert!(matches!(g.overlay, Overlay::Trial(e, k, None, _) if e == era && k == q));
            g.render();
            let quiz = &QUIZZES[quizzes(era)[q]];
            let k = if !perfect && q == 2 {
                (quiz.answer + 1) % 4
            } else {
                quiz.answer
            };
            g.handle(Ev::Key(49 + k as u32, true));
            g.render();
            g.act(Act::Next);
        }
        let score = if perfect { n } else { n - 1 };
        assert_eq!(g.overlay, Overlay::TrialResult(era, score));
        g.render();
    }

    #[test]
    fn trial_needs_everything_and_a_perfect_score() {
        let mut g = Game::new(Gfx::new(1600, 900));
        g.act(Act::Shift(EDO, None));
        run(&mut g, 2.);
        g.act(Act::TrialIntro(EDO));
        assert_eq!(g.overlay, Overlay::None);
        g.act(Act::Begin(EDO));
        assert_eq!(g.overlay, Overlay::None);
        for (i, p) in PEOPLE.iter().enumerate() {
            g.journal.met[i] |= p.era == EDO;
        }
        for (i, e) in EVENTS.iter().enumerate() {
            g.journal.seen[i] |= e.era == EDO;
        }
        g.act(Act::Close);
        assert_eq!(g.overlay, Overlay::TrialIntro(EDO));
        // 一問でも間違えると秘宝は手に入らない
        pass_trial(&mut g, EDO, false);
        assert!(!g.journal.treasures[EDO]);
        // 一度閉じても、また自動では始まらない。欄のボタンから挑み直せる
        g.act(Act::Close);
        assert_eq!(g.overlay, Overlay::None);
        g.act(Act::TrialIntro(EDO));
        pass_trial(&mut g, EDO, true);
        assert!(g.journal.treasures[EDO]);
        assert_eq!(g.treasure_count(), 1);
        g.act(Act::Close);
        g.render();
        g.act(Act::Codex);
        g.act(Act::Tab(Tab::Treasures));
        g.render();
        g.act(Act::Treasure(EDO));
        g.render();
        g.act(Act::Treasure(ROME));
        g.render();
        // 秘宝は保存される
        let b = g.journal.bytes();
        let j = Journal::read(&b).unwrap();
        assert!(j.treasures[EDO] && !j.treasures[ROME]);
    }

    #[test]
    fn reads_saves_without_treasures() {
        let mut j = Journal::new();
        j.met[3] = true;
        j.visited[1] = true;
        let mut b = j.bytes();
        b.truncate(b.len() - 4);
        b[..8].copy_from_slice(MAGIC_V2);
        b.remove(11);
        let hash = checksum(&b);
        b.extend_from_slice(&hash.to_le_bytes());
        assert_eq!(Journal::read(&b).unwrap(), j);
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
            for t in [Tab::People, Tab::Timeline, Tab::Events, Tab::Treasures] {
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
