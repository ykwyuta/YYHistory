//! 時空を渡って歴史の現場を歩くゲーム。
//!
//! 世界地図で時代と地域を選んで転移し、その時代の街のミニマップを移動して
//! 人物に会い、出来事を観察する。出会った人物と観察した出来事は図鑑
//! （人物一覧表・年表・出来事集）に記録される。
mod briefing;
mod conversation;
mod intro;
mod voice;
mod warp;
use intro::{restore_speech, Target};

use crate::{
    gfx::{Font, Gfx, Texture},
    history::*,
    images, notes, platform, talk, Ev,
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
    const C: [[f32; 4]; 17] = [
        [0.85, 0.42, 0.35, 1.],
        [0.90, 0.66, 0.30, 1.],
        [0.80, 0.52, 0.78, 1.],
        [0.40, 0.76, 0.56, 1.],
        [0.48, 0.66, 0.92, 1.],
        [0.36, 0.60, 0.95, 1.],
        [0.55, 0.78, 0.88, 1.],
        [0.92, 0.55, 0.62, 1.],
        [0.95, 0.40, 0.40, 1.],
        [0.86, 0.80, 0.42, 1.],
        [0.62, 0.84, 0.48, 1.],
        [0.98, 0.72, 0.50, 1.],
        [0.66, 0.60, 0.95, 1.],
        [0.95, 0.62, 0.82, 1.],
        [0.45, 0.85, 0.80, 1.],
        [0.80, 0.70, 0.55, 1.],
        [0.70, 0.88, 0.95, 1.],
    ];
    if i < C.len() {
        return C[i];
    }
    // 18番目からは、色相を黄金角ずつずらして作る
    let h = (i as f32 * 0.618_034).fract() * 6.;
    let x = 1. - ((h % 2.) - 1.).abs();
    let (r, g, b) = match h as u32 {
        0 => (1., x, 0.),
        1 => (x, 1., 0.),
        2 => (0., 1., x),
        3 => (0., x, 1.),
        4 => (x, 0., 1.),
        _ => (1., 0., x),
    };
    [0.45 + 0.5 * r, 0.45 + 0.5 * g, 0.45 + 0.5 * b, 1.]
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
    Words,
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
    /// はじめのおはなしの表紙（ここで押すと、iPad などでも音声が使えるようになる）
    Title,
    /// はじめのおはなし（ページ）
    Story(usize),
    /// 時空の復元力のはじめての説明（会った人物）
    Restore(usize),
    /// クロノの時代の説明（時代, ページ）
    Briefing(usize, usize),
}

#[derive(Clone, Copy, Debug)]
enum Act {
    Atlas,
    Codex,
    PickEra(usize),
    Shift(usize, Option<usize>),
    Goto(usize),
    Meet(usize),
    /// 会話で質問する（何番目の質問か）
    Ask(usize),
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
    Story(usize),
    StoryBack,
    StorySkip,
    TipsOff,
    /// キャッシュを消して読み込み直す（ブラウザ版）
    ReloadClear,
    Replay,
    /// なぜ？ノートを開く・閉じる（ノートの番号）
    Note(usize),
    /// クロノの時代の説明を読み直す
    Brief(usize),
    BriefBack,
    BriefSkip,
}

struct Button {
    rect: [f32; 4],
    act: Act,
}

#[cfg(test)]
impl Button {
    fn clone_rect(&self) -> [f32; 4] {
        self.rect
    }
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
    /// 導入の進み具合（FLAG_*）
    flags: u8,
    /// クロノに聞いた「なぜ？ノート」
    notes: Vec<bool>,
}

/// はじめのおはなしを見た
const FLAG_INTRO: u8 = 1;
/// 時空の復元力の説明を見た
const FLAG_RESTORE: u8 = 2;
/// 地図の上を移動した
const FLAG_MOVED: u8 = 4;
/// 図鑑を開いた
const FLAG_CODEX: u8 = 8;
/// クロノの案内を出さない
const FLAG_TIPS_OFF: u8 = 128;
/// 導入をすべて済ませた
const FLAGS_DONE: u8 = FLAG_INTRO | FLAG_RESTORE | FLAG_MOVED | FLAG_CODEX;

const MAGIC: &[u8; 8] = b"YYHIST\x07\0";
/// 時代を番号（8つまで）で記録していた前の版
const MAGIC_V6: &[u8; 8] = b"YYHIST\x06\0";
/// 用語メモ（廃止）を記録していた前の版
const MAGIC_V5: &[u8; 8] = b"YYHIST\x05\0";
/// 用語メモの無い前の版の記録
const MAGIC_V4: &[u8; 8] = b"YYHIST\x04\0";
/// 導入の進み具合の無い前の版の記録
const MAGIC_V3: &[u8; 8] = b"YYHIST\x03\0";
/// 秘宝の無い前の版の記録
const MAGIC_V2: &[u8; 8] = b"YYHIST\x02\0";
/// 前の版（v2〜v6）の時代の番号の並び。時代を足しても、古い記録はこの並びで読む
const OLD_ERAS: [&str; 7] = [
    "rome",
    "changan",
    "heian",
    "florence",
    "tenochtitlan",
    "paris",
    "edo",
];

fn era_index(id: &str) -> Option<usize> {
    ERAS.iter().position(|e| e.id == id)
}

/// 記録の読み取り位置
struct Reader<'a> {
    b: &'a [u8],
    at: usize,
    end: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        (self.at < self.end).then(|| {
            self.at += 1;
            self.b[self.at - 1]
        })
    }
    fn str(&mut self) -> Option<&str> {
        let len = self.byte()? as usize;
        if self.at + len > self.end {
            return None;
        }
        self.at += len;
        std::str::from_utf8(&self.b[self.at - len..self.at]).ok()
    }
    /// id の一覧を読み、見つかった印を返す（知らない id や重複は不正）
    fn ids(&mut self, find: &dyn Fn(&str) -> Option<usize>, n_all: usize) -> Option<Vec<bool>> {
        let n = u16::from_le_bytes([self.byte()?, self.byte()?]) as usize;
        let mut out = vec![false; n_all];
        for _ in 0..n {
            let i = find(self.str()?)?;
            if std::mem::replace(&mut out[i], true) {
                return None;
            }
        }
        Some(out)
    }
    /// id の一覧を読み飛ばす
    fn skip_ids(&mut self) -> Option<()> {
        let n = u16::from_le_bytes([self.byte()?, self.byte()?]) as usize;
        for _ in 0..n {
            self.str()?;
        }
        Some(())
    }
}

fn push_ids(b: &mut Vec<u8>, ids: Vec<&str>) {
    b.extend_from_slice(&(ids.len() as u16).to_le_bytes());
    for id in ids {
        b.push(id.len() as u8);
        b.extend_from_slice(id.as_bytes());
    }
}

fn marked<'a, T>(
    all: &'a [T],
    marks: &[bool],
    id: impl Fn(&'a T) -> &'static str,
) -> Vec<&'static str> {
    all.iter()
        .zip(marks)
        .filter(|(_, m)| **m)
        .map(|(x, _)| id(x))
        .collect()
}

impl Journal {
    fn new() -> Self {
        Self {
            era: ROME,
            place: ERAS[ROME].start,
            visited: vec![false; ERAS.len()],
            met: vec![false; PEOPLE.len()],
            seen: vec![false; EVENTS.len()],
            treasures: vec![false; ERAS.len()],
            flags: 0,
            notes: vec![false; notes::NOTES.len()],
        }
    }
    fn bytes(&self) -> Vec<u8> {
        let mut b = MAGIC.to_vec();
        let era = ERAS[self.era].id;
        b.push(era.len() as u8);
        b.extend_from_slice(era.as_bytes());
        b.push(self.place as u8);
        b.push(self.flags);
        push_ids(&mut b, marked(ERAS, &self.visited, |e| e.id));
        push_ids(&mut b, marked(ERAS, &self.treasures, |e| e.id));
        push_ids(&mut b, marked(PEOPLE, &self.met, |p| p.id));
        push_ids(&mut b, marked(EVENTS, &self.seen, |e| e.id));
        push_ids(&mut b, marked(notes::NOTES, &self.notes, |n| n.id));
        let hash = checksum(&b);
        b.extend_from_slice(&hash.to_le_bytes());
        b
    }
    fn read(b: &[u8]) -> Result<Self, String> {
        let invalid =
            || "時空事件簿の記録として読み込めません。現在の記録はそのままです。".to_string();
        let head = if b.len() >= 8 { &b[..8] } else { &[][..] };
        let legacy = [MAGIC_V6, MAGIC_V5, MAGIC_V4, MAGIC_V3, MAGIC_V2]
            .iter()
            .position(|m| head == *m);
        if b.len() < 16 || b.len() > 65536 || (head != MAGIC && legacy.is_none()) {
            return Err(invalid());
        }
        let end = b.len() - 4;
        if checksum(&b[..end]) != u32::from_le_bytes(b[end..].try_into().unwrap()) {
            return Err(invalid());
        }
        let mut r = Reader { b, at: 8, end };
        let j = match legacy {
            None => Self::read_v7(&mut r),
            Some(k) => Self::read_legacy(&mut r, 6 - k as u8),
        }
        .ok_or_else(invalid)?;
        if r.at != end || j.place >= ERAS[j.era].places.len() {
            return Err(invalid());
        }
        Ok(j)
    }
    fn read_v7(r: &mut Reader) -> Option<Self> {
        let era = era_index(r.str()?)?;
        let place = r.byte()? as usize;
        let flags = r.byte()?;
        Some(Self {
            era,
            place,
            flags,
            visited: r.ids(&era_index, ERAS.len())?,
            treasures: r.ids(&era_index, ERAS.len())?,
            met: r.ids(&person_index, PEOPLE.len())?,
            seen: r.ids(&event_index, EVENTS.len())?,
            notes: r.ids(&notes::index, notes::NOTES.len())?,
        })
    }
    /// v2〜v6：時代は OLD_ERAS の番号、訪れた時代と秘宝は1バイトの印
    fn read_legacy(r: &mut Reader, version: u8) -> Option<Self> {
        let old = |k: usize| era_index(OLD_ERAS.get(k)?);
        let era = old(r.byte()? as usize)?;
        let place = r.byte()? as usize;
        let marks = |byte: u8| -> Option<Vec<bool>> {
            let mut v = vec![false; ERAS.len()];
            for k in 0..8 {
                if byte & (1 << k) != 0 {
                    v[old(k)?] = true;
                }
            }
            Some(v)
        };
        let visited = marks(r.byte()?)?;
        let treasures = if version >= 3 {
            marks(r.byte()?)?
        } else {
            vec![false; ERAS.len()]
        };
        let stored_flags = if version >= 4 { Some(r.byte()?) } else { None };
        let met = r.ids(&person_index, PEOPLE.len())?;
        let seen = r.ids(&event_index, EVENTS.len())?;
        let notes = match version {
            6 => r.ids(&notes::index, notes::NOTES.len())?,
            5 => {
                // 廃止した用語メモの一覧を読み飛ばす
                r.skip_ids()?;
                vec![false; notes::NOTES.len()]
            }
            _ => vec![false; notes::NOTES.len()],
        };
        // 前の版の記録：もう旅をしていれば導入は済んだものとする
        let played = met.iter().chain(&seen).any(|x| *x);
        let flags = stored_flags.unwrap_or(if played { FLAGS_DONE } else { 0 });
        Some(Self {
            era,
            place,
            visited,
            met,
            seen,
            treasures,
            flags,
            notes,
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
    /// おはなしのページを開いてからの時間
    story_t: f32,
    /// 時空の復元力の演出（会った人物, 経過時間）
    restore_fx: Option<(usize, f32)>,
    /// クロノの案内が指す場所（この描画で見つけたもの）
    targets: Vec<(Target, [f32; 4])>,
    /// 最後に読み上げた文（声 ON にしたときに読む）
    last_speech: String,
    /// 最後に読み上げたクロノの案内
    last_tip: Option<&'static str>,
    /// 最後に読み上げを始めた時刻
    spoke_at: f32,
    /// いま開いている枠（🔊 ボタンを置く）
    card_rect: Option<[f32; 4]>,
    /// 開いているなぜ？ノート（どの画面のノートか）
    memo_open: Option<usize>,
    memo_for: Overlay,
    /// いまの会話で、もうきいた質問（ビット）
    asked: u32,
    /// 世界地図の一覧で、見えるようにした時代
    atlas_shown: Option<usize>,
    /// ページを送ったら、その用語を読み上げる
    memo_turned: bool,
    /// 時代の説明を読み終えたら転移する先
    briefing_then: Option<(usize, Option<usize>)>,
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
        let p = &ERAS[ROME].places[ERAS[ROME].start];
        let g = Self {
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
            pick: ROME,
            shift_t: 0.,
            shift_to: (0, None),
            avatar: (p.x, p.y),
            travel: None,
            trial_offered: vec![false; ERAS.len()],
            story_t: 0.,
            restore_fx: None,
            targets: vec![],
            last_speech: String::new(),
            last_tip: None,
            spoke_at: -1.,
            card_rect: None,
            memo_open: None,
            memo_for: Overlay::None,
            asked: 0,
            atlas_shown: None,
            memo_turned: false,
            briefing_then: None,
            voice: platform::voice_enabled(),
            dirty: false,
            save_in: 0.,
        };
        platform::load_slot(0);
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
                "時空事件簿 人物{}/{} 出来事{}/{}",
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
        let prev = std::mem::replace(&mut self.overlay, Overlay::None);
        self.hush();
        if let Overlay::Meet(i, _) = prev {
            // 話した記憶は、時空の復元力で歴史から消える
            if self.journal.flags & FLAG_RESTORE == 0 {
                self.overlay = Overlay::Restore(i);
                self.story_t = 0.;
                self.say(restore_speech(i));
                return;
            }
            self.restore_fx = Some((i, 0.));
        }
        if let Overlay::Restore(_) = prev {
            self.journal.flags |= FLAG_RESTORE;
            self.change();
        }
        let era = self.journal.era;
        if self.mode == Mode::Field
            && self.era_complete(era)
            && !self.journal.treasures[era]
            && !self.trial_offered[era]
        {
            self.trial_offered[era] = true;
            self.overlay = Overlay::TrialIntro(era);
            self.say(voice::trial_intro_speech(era));
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
        // 一覧の項目はボタンだが、一覧の上でなぞればどこでもスクロールする。
        // 軽く押して離せば、ブラウザ側がタップとして送るので項目を選べる（YYEarth と同じ）
        if self.scroll_areas.iter().any(|(r, _)| inside(*r, x, y)) {
            return 3;
        }
        1
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
        self.speak_tip();
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
        self.story_t += dt;
        if let Some((_, t)) = &mut self.restore_fx {
            *t += dt;
            if *t > intro::RESTORE_FX_SECS {
                self.restore_fx = None;
            }
        }
        // はじめて遊ぶとき：記録の読み込みを少し待ってから、はじめのおはなしを見せる
        if self.journal.flags & FLAG_INTRO == 0
            && self.mode == Mode::Atlas
            && self.overlay == Overlay::None
            && self.clock > 0.5
        {
            self.overlay = Overlay::Title;
            self.story_t = 0.;
        }
        if self.mode == Mode::Shift {
            self.shift_t += dt;
            if self.shift_t >= warp::SHIFT_SECS {
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
                self.journal.flags |= FLAG_MOVED;
                self.scroll[Scroll::Panel as usize] = 0.;
                self.change();
                let place = &ERAS[self.journal.era].places[target];
                let people = self.people_here().len();
                let events = self.events_here().len();
                self.notify(&format!(
                    "{}に着いた。人物{}人・出来事{}件",
                    place.name, people, events
                ));
                self.say(voice::place_speech(self.journal.era, target));
            } else {
                let a = tr.points[tr.at];
                let b = tr.points[tr.at + 1];
                self.avatar = (a.0 + (b.0 - a.0) * tr.t, a.1 + (b.1 - a.1) * tr.t);
            }
        }
    }

    /// 転移の演出を始める
    fn start_shift(&mut self, era: usize, place: Option<usize>) {
        self.overlay = Overlay::None;
        self.shift_to = (era, place);
        self.shift_t = 0.;
        self.mode = Mode::Shift;
        self.travel = None;
        self.hush();
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
        self.say(voice::era_speech(era));
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
        self.asked = 0;
        self.scroll[Scroll::Overlay as usize] = 0.;
        if !self.journal.met[i] {
            self.journal.met[i] = true;
            self.notify(&format!("{} を人物一覧表に登録しました。", p.name));
            self.change();
        }
        self.say(voice::meet_speech(i, 0));
    }

    /// 会話の場面を変える
    fn show_talk(&mut self, i: usize, page: usize) {
        self.overlay = Overlay::Meet(i, page);
        self.scroll[Scroll::Overlay as usize] = 0.;
        self.say(voice::meet_speech(i, page));
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
        self.say(voice::observe_speech(i));
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
                if self.journal.flags & FLAG_CODEX == 0 {
                    self.journal.flags |= FLAG_CODEX;
                    self.change();
                }
                self.mode = Mode::Codex;
                self.overlay = Overlay::None;
                self.scroll[Scroll::Codex as usize] = 0.;
            }
            Act::PickEra(i) => {
                self.pick = i;
                self.say(voice::era_speech(i));
            }
            Act::Shift(era, place) => {
                // はじめて行く時代では、クロノがまず前後の流れを説明する
                if self.journal.visited[era] {
                    self.start_shift(era, place);
                } else {
                    self.open_briefing(era, Some((era, place)));
                }
            }
            Act::Brief(era) => self.open_briefing(era, None),
            Act::BriefBack => {
                if let Overlay::Briefing(era, page) = self.overlay {
                    self.show_briefing(era, page.saturating_sub(1));
                }
            }
            Act::BriefSkip => self.finish_briefing(),
            Act::Goto(p) => self.goto(p),
            Act::Meet(i) => self.meet(i),
            Act::Ask(k) => {
                if let Overlay::Meet(i, _) = self.overlay {
                    self.asked |= 1 << k;
                    self.show_talk(i, k + 1);
                }
            }
            Act::Observe(i) => self.observe(i),
            Act::Next => match self.overlay {
                // 会話：「別れる」で別れぎわのひとこと、もう一度押すと閉じる
                Overlay::Meet(i, page) => {
                    let last = talk::pages(i) - 1;
                    if page < last {
                        self.show_talk(i, last);
                    } else {
                        self.close_overlay();
                    }
                }
                Overlay::None => (),
                Overlay::TrialIntro(era) => self.act(Act::Begin(era)),
                Overlay::Title => self.open_story(0),
                Overlay::Briefing(era, page) => {
                    if page + 1 < briefing::pages(era).len() {
                        self.show_briefing(era, page + 1);
                    } else {
                        self.finish_briefing();
                    }
                }
                Overlay::Story(page) => {
                    if page + 1 < intro::PAGES.len() {
                        self.open_story(page + 1);
                    } else {
                        self.finish_story();
                    }
                }
                Overlay::Trial(_, _, Some(_), _) => self.act(Act::TrialNext),
                Overlay::Trial(..) => (),
                _ => self.close_overlay(),
            },
            // 会話：声をかけたところに戻る
            Act::Prev => {
                if let Overlay::Meet(i, page) = self.overlay {
                    if page > 0 {
                        self.show_talk(i, 0);
                    }
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
                    let _ = ok;
                    self.say(voice::answer_speech(era, q, k));
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
                        self.say(voice::result_speech(era, score));
                    }
                }
            }
            Act::Treasure(era) => {
                self.overlay = Overlay::Treasure(era);
                self.scroll[Scroll::Overlay as usize] = 0.;
                if self.journal.treasures[era] || self.reveal {
                    self.say(voice::treasure_speech(era));
                }
            }
            Act::Replay => self.replay(),
            Act::Note(i) => {
                if self.memo_open == Some(i) {
                    self.memo_open = None;
                } else {
                    self.memo_open = Some(i);
                    self.memo_turned = true;
                }
            }
            Act::Story(page) => self.open_story(page),
            Act::StoryBack => {
                if let Overlay::Story(page) = self.overlay {
                    self.open_story(page.saturating_sub(1));
                }
            }
            Act::StorySkip => self.finish_story(),
            Act::ReloadClear => {
                if platform::CAN_RELOAD {
                    self.notify("キャッシュを消して、読み込み直します…");
                    platform::reload_clear();
                }
            }
            Act::TipsOff => {
                self.journal.flags ^= FLAG_TIPS_OFF;
                self.change();
                if self.journal.flags & FLAG_TIPS_OFF != 0 {
                    self.notify("クロノの案内を消しました。「使い方」からまた表示できます。");
                }
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
                let known = match e {
                    Entry::Person(i) => self.journal.met[i],
                    Entry::Event(i) => self.journal.seen[i],
                } || self.reveal;
                if known {
                    self.say(voice::detail_speech(e));
                } else {
                    self.hush();
                }
            }
            Act::Help => {
                self.overlay = Overlay::Help;
                self.scroll[Scroll::Overlay as usize] = 0.;
            }
            Act::Voice => {
                self.voice = !self.voice;
                platform::save_voice(self.voice);
                if self.voice {
                    self.replay();
                } else {
                    self.hush();
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

    fn speak_question(&mut self) {
        if let Overlay::Trial(era, q, None, _) = self.overlay {
            self.say(voice::question_speech(era, q));
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
                if let Overlay::Briefing(..) = self.overlay {
                    match k {
                        27 => self.act(Act::BriefSkip),
                        37 | 65 => self.act(Act::BriefBack),
                        13 | 32 | 39 | 68 => self.act(Act::Next),
                        _ => (),
                    }
                    return;
                }
                if let Overlay::Story(_) | Overlay::Title = self.overlay {
                    match k {
                        27 => self.act(Act::StorySkip),
                        37 | 65 => self.act(Act::StoryBack),
                        13 | 32 | 39 | 68 => self.act(Act::Next),
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
                            self.act(Act::PickEra((self.pick + ERAS.len() - 1) % ERAS.len()));
                        } else {
                            self.step(-1., 0.);
                        }
                    }
                    39 | 68 => {
                        if let Overlay::Meet(..) = self.overlay {
                            self.act(Act::Next);
                        } else if self.mode == Mode::Atlas {
                            self.act(Act::PickEra((self.pick + 1) % ERAS.len()));
                        } else {
                            self.step(1., 0.);
                        }
                    }
                    38 | 87 => {
                        if self.mode == Mode::Atlas {
                            self.act(Act::PickEra((self.pick + ERAS.len() - 1) % ERAS.len()));
                        } else {
                            self.step(0., -1.);
                        }
                    }
                    40 | 83 => {
                        if self.mode == Mode::Atlas {
                            self.act(Act::PickEra((self.pick + 1) % ERAS.len()));
                        } else {
                            self.step(0., 1.);
                        }
                    }
                    0x71 => self.act(Act::Codex),
                    0x73 => self.act(Act::Replay),
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
            // 会話・おはなしの枠の外を押したら次へ進む
            if let Overlay::Meet(..)
            | Overlay::Story(_)
            | Overlay::Restore(_)
            | Overlay::Briefing(..) = self.overlay
            {
                self.act(Act::Next);
            }
            // 表紙は「はじめる」のボタンで進む（最初のタップで読み上げを始めるため）
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
        let scale = (aw / 300.).min(ah / 100.);
        // 経度 -125〜175、緯度 68〜-32 を中心に収める（南アメリカのリオまで入るように）
        let cx = x0 + aw / 2. - 25. * scale;
        let cy = top + ah / 2. + 18. * scale;
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
        if known {
            if let Some(p) = PEOPLE.iter().find(|p| p.name == name) {
                if let Some(img) = self.gfx.image(&images::portrait(p.id)) {
                    self.gfx.clip_round(x - r, y - r, x + r, y + r, r);
                    self.gfx.draw_image(&img, x - r, y - r, x + r, y + r, 1.);
                    self.gfx.clip_end();
                    self.gfx.stroke_circle(x, y, r, 1.5, GOLD);
                    return;
                }
            }
        }
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
        self.targets.clear();
        self.card_rect = None;
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        self.gfx.begin(NIGHT);
        self.gfx.set_screen_xform();
        match self.mode {
            Mode::Atlas => self.atlas_geometry(),
            Mode::Shift => {
                self.atlas_geometry();
                self.shift_geometry();
            }
            Mode::Field => self.field_geometry(),
            Mode::Codex => (),
        }
        self.gfx.flush();
        if !self.gfx.d2d_begin() {
            return;
        }
        match self.mode {
            Mode::Atlas => self.atlas_ui(),
            Mode::Shift => (),
            Mode::Field => self.field_ui(),
            Mode::Codex => self.codex_ui(),
        }
        self.top_bar();
        if self.overlay == Overlay::None && self.mode != Mode::Shift {
            self.restore_ui();
            self.tip_ui();
        }
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
            Overlay::Story(page) => self.story_ui(page),
            Overlay::Title => self.title_ui(),
            Overlay::Briefing(era, page) => self.briefing_ui(era, page),
            Overlay::Restore(i) => self.restore_explain_ui(i),
        }
        // 開いている枠の右上に 🔊（おはなしと復元力の説明は、それぞれの画面に置く）
        if let (Some(r), false) = (
            self.card_rect,
            matches!(
                self.overlay,
                Overlay::Help
                    | Overlay::Title
                    | Overlay::Story(_)
                    | Overlay::Restore(_)
                    | Overlay::Briefing(..)
            ),
        ) {
            self.speaker_button(r[0] + r[2] - 96., r[1] + 12.);
        }
        if self.mode == Mode::Shift {
            self.shift_ui();
        }
        if self.toast_left > 0.
            && self.mode != Mode::Shift
            && !matches!(
                self.overlay,
                Overlay::Title | Overlay::Story(_) | Overlay::Restore(_) | Overlay::Briefing(..)
            )
            // スマートフォンでは通知が上に出るので、枠の見出しに重ねない
            && !(self.gfx.compact && self.overlay != Overlay::None)
        {
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
            "時空事件簿",
            18.,
            if compact { 6. } else { 8. },
            200.,
            30.,
            Font::TitleMid,
            GOLD,
        );
        if !compact {
            self.text("じくう じけんぼ", 20., 40., 220., 16., Font::Small, MUTED);
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
            if let Act::Codex = a {
                self.targets.push((Target::Codex, [x, by, 96., bh]));
            }
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
        // 選んだ時代が変わったら、一覧のその行が見えるようにずらす
        if self.atlas_shown != Some(self.pick) {
            self.atlas_shown = Some(self.pick);
            let y = self.pick as f32 * row;
            let s = &mut self.scroll[Scroll::Atlas as usize];
            if y < *s {
                *s = y;
            } else if y + row > *s + view[3] {
                *s = y + row - view[3];
            }
        }
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

        // 地図の上の名札：選んだ時代から順に、重ならない場所に置く。置けない名札は出さない（点は押せる）
        let card = if compact { 150. } else { 176. };
        let bounds = [lw + 4., top + 4., w - 4., h - card - 4.];
        let (tw, th) = (136., 28.);
        // 点は名札でかくさない
        let dots: Vec<[f32; 4]> = ERAS
            .iter()
            .map(|e| {
                let (x, y) = self.atlas_project(e.lon, e.lat);
                [x - 8., y - 8., 16., 16.]
            })
            .collect();
        let mut placed: Vec<[f32; 4]> = vec![];
        let mut order: Vec<usize> = (0..ERAS.len()).collect();
        order.sort_by_key(|i| *i != self.pick);
        let mut tags: Vec<(usize, Option<[f32; 4]>)> = vec![];
        for &i in &order {
            let e = &ERAS[i];
            let (x, y) = self.atlas_project(e.lon, e.lat);
            let candidates = [
                e.tag,
                (14., -th / 2.),
                (-tw - 14., -th / 2.),
                (-tw / 2., -th - 12.),
                (-tw / 2., 12.),
                (14., -th - 8.),
                (-tw - 14., -th - 8.),
                (14., 8.),
                (-tw - 14., 8.),
            ];
            let free = |r: [f32; 4]| {
                r[0] >= bounds[0]
                    && r[1] >= bounds[1]
                    && r[0] + r[2] <= bounds[2]
                    && r[1] + r[3] <= bounds[3]
                    && placed.iter().chain(&dots).all(|q| {
                        r[0] + r[2] + 4. <= q[0]
                            || q[0] + q[2] + 4. <= r[0]
                            || r[1] + r[3] + 2. <= q[1]
                            || q[1] + q[3] + 2. <= r[1]
                    })
            };
            let r = candidates
                .iter()
                .map(|(dx, dy)| [x + dx, y + dy, tw, th])
                .find(|r| free(*r));
            if let Some(r) = r {
                placed.push(r);
            }
            tags.push((i, r));
        }
        for (i, r) in tags.into_iter().rev() {
            let e = &ERAS[i];
            let (x, y) = self.atlas_project(e.lon, e.lat);
            let sel = i == self.pick;
            self.gfx.fill_circle(
                x,
                y,
                if sel { 6. } else { 4.5 },
                if sel { GOLD } else { TEXT },
            );
            self.gfx
                .stroke_circle(x, y, 9., 1.5, alpha(era_color(i), 0.9));
            self.buttons.push(Button {
                rect: [x - 12., y - 12., 24., 24.],
                act: Act::PickEra(i),
            });
            let Some(r) = r else { continue };
            self.gfx.fill_round(
                r[0],
                r[1],
                r[0] + tw,
                r[1] + th,
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
                r[1] + th,
                6.,
                1.,
                if sel { GOLD } else { alpha(era_color(i), 0.8) },
            );
            self.text(
                e.name,
                r[0] + 4.,
                r[1] + 3.,
                tw - 8.,
                22.,
                Font::NormalCenter,
                TEXT,
            );
            self.buttons.push(Button {
                rect: r,
                act: Act::PickEra(i),
            });
        }

        // 選んだ時代の札
        let e = &ERAS[self.pick];
        let (x0, y0, x1, y1) = (lw + 16., h - card + 4., w - 16., h - 12.);
        self.panel_box(x0, y0, x1, y1);
        // 時代の扉絵があれば左に
        let art = self.gfx.image(&images::era_art(self.pick));
        let aw = if art.is_some() {
            (y1 - y0 - 20.) * 16. / 9.
        } else {
            0.
        };
        if let Some(img) = art {
            self.gfx
                .draw_image(&img, x0 + 10., y0 + 10., x0 + 10. + aw, y1 - 10., 1.);
            self.gfx.stroke_rect(
                x0 + 10.,
                y0 + 10.,
                x0 + 10. + aw,
                y1 - 10.,
                1.,
                alpha(GOLD, 0.6),
            );
            image_tag(&mut self.gfx, x0 + 10. + aw, y1 - 10.);
        }
        let tx0 = x0 + 20. + if aw > 0. { aw + 6. } else { 0. };
        // 右の操作欄（幅 RIGHT）と文の欄が重ならないようにする
        const RIGHT: f32 = 330.;
        let tw = (x1 - tx0 - RIGHT).max(60.);
        // 集めた数は文の欄の右上に（右の欄の上はクロノの指さしが来る）
        let pc = PEOPLE.iter().filter(|p| p.era == self.pick).count();
        let ec = EVENTS.iter().filter(|x| x.era == self.pick).count();
        let count = format!(
            "人物 {}/{}　出来事 {}/{}",
            self.met_count(Some(self.pick)),
            pc,
            self.seen_count(Some(self.pick)),
            ec
        );
        let cwid = self.gfx.measure(&count, Font::Small) + 4.;
        self.text(
            &count,
            tx0 + tw - cwid,
            y0 + 12.,
            cwid,
            18.,
            Font::Small,
            CYAN,
        );
        self.text(
            &format!("{}　／　{}", e.years, e.area),
            tx0,
            y0 + 12.,
            (tw - cwid - 12.).max(20.),
            18.,
            Font::Small,
            GOLD,
        );
        let title = format!("{} ― {}", e.name, e.period);
        let title_h = self.gfx.measure_h(&title, Font::Big, tw).max(26.);
        self.text(&title, tx0, y0 + 32., tw, title_h, Font::Big, TEXT);
        // 題が折り返したら、説明はその下から
        let iy = y0 + 36. + title_h;
        self.gfx.clip_begin(tx0, iy - 2., tx0 + tw, y1 - 8.);
        self.text(
            e.intro,
            tx0,
            iy,
            tw,
            (y1 - 4. - iy).max(0.),
            Font::Normal,
            MUTED,
        );
        self.gfx.clip_end();
        self.button(
            "転移する",
            [x1 - 220., y0 + 50., 200., 48.],
            Act::Shift(self.pick, None),
            true,
        );
        self.targets
            .push((Target::Shift, [x1 - 220., y0 + 50., 200., 48.]));
        self.speaker_button(x1 - 310., y0 + 58.);
        self.button(
            "クロノの説明",
            [
                x1 - 310.,
                y0 + if compact { 102. } else { 108. },
                150.,
                if compact { 28. } else { 32. },
            ],
            Act::Brief(self.pick),
            false,
        );
        if !compact {
            let hint = "Enter でも転移できます";
            let hw = self.gfx.measure(hint, Font::Small) + 4.;
            if hw <= 150. {
                self.text(hint, x1 - 20. - hw, y0 + 116., hw, 18., Font::Small, MUTED);
            }
        }
    }

    fn shift_ui(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let t = (self.shift_t / warp::SHIFT_SECS).clamp(0., 1.);
        let fade = (t / 0.12).min(1.) * ((1. - t) / 0.12).min(1.);
        // 最後に光に包まれて、転移先の景色が浮かび上がる
        let flash = (1. - ((t - 0.74) / 0.08).abs()).clamp(0., 1.);
        let arrive = ((t - 0.74) / 0.2).clamp(0., 1.);
        if let Some(img) = self.gfx.image(&images::era_art(self.shift_to.0)) {
            cover(&mut self.gfx, &img, [0., 0., w, h], 16. / 9., arrive);
        } else {
            self.gfx
                .fill_rect(0., 0., w, h, [0.05, 0.08, 0.14, arrive * 0.9]);
        }
        self.gfx.fill_rect(0., 0., w, h, [1., 1., 1., flash * 0.85]);
        let (era, _) = self.shift_to;
        let e = &ERAS[era];
        let (cx, cy) = (w / 2., h * 0.78);
        self.gfx.fill_round(
            cx - 320.,
            cy - 76.,
            cx + 320.,
            cy + 74.,
            16.,
            [0.03, 0.04, 0.12, 0.55 * fade],
        );
        self.text(
            "時 空 転 移",
            cx - 300.,
            cy - 70.,
            600.,
            44.,
            Font::Title,
            alpha(CYAN, fade),
        );
        self.text(
            &format!("{}　{}", e.years, e.name),
            cx - 300.,
            cy - 18.,
            600.,
            30.,
            Font::BigCenter,
            alpha(GOLD, fade),
        );
        let n = (self.shift_t * 8.) as usize;
        self.text(
            &format!("時間をさかのぼっています{}", "・".repeat(n % 4)),
            cx - 300.,
            cy + 20.,
            600.,
            24.,
            Font::NormalCenter,
            alpha(TEXT, fade),
        );
        let _ = h;
    }

    // -------------------------------------------------------------- ミニマップ
    fn field_geometry(&mut self) {
        let r = self.map_rect();
        let era = &ERAS[self.journal.era];
        let tint = era.tint;
        let m = |x: f32, y: f32| (r[0] + x * r[2], r[1] + y * r[3]);
        let has_map = self.gfx.image(&images::map(self.journal.era)).is_some();
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
        if has_map {
            // 絵地図は文字・UI の層に描く（field_ui）
            return;
        }
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
    }

    /// マップ上の時空探偵（プレイヤー）のアイコンを描く
    fn player_avatar(&mut self, px: f32, py: f32) {
        let walking = self.travel.is_some();
        let pulse = 0.5 + 0.5 * (self.clock * 3.5).sin();
        let bob = (self.clock * if walking { 9. } else { 4. }).sin() * if walking { 3. } else { 2.5 };
        // 現在地の輪（止まっているときは場所の印を光らせる）
        if !walking {
            self.gfx
                .stroke_circle(px, py, 20. + pulse * 5., 2.5, [0.35, 0.90, 1.0, 0.80 - pulse * 0.4]);
            self.gfx.stroke_circle(px, py, 16., 3., GOLD);
        }
        // 場所の右側には「人」「！」バッジ、下側には地名札があるので、左上側に探偵アイコンを出す
        let side = if self.avatar.0 < 0.12 { 1. } else { -1. };
        let (cx, cy) = if walking {
            (px, py - 24. + bob)
        } else {
            (px + side * 28., py - 22. + bob)
        };
        // 現在地とアイコンをつなぐ光の粒（または足元の影と歩く足）
        if walking {
            self.gfx.fill_circle(px, py + 2., 10., [0., 0., 0., 0.32]);
            let step = (self.clock * 12.).sin() * 3.5;
            self.gfx
                .fill_circle(cx - 5., cy + 20. + step, 3.8, [0.38, 0.22, 0.10, 1.]);
            self.gfx
                .fill_circle(cx + 5., cy + 20. - step, 3.8, [0.38, 0.22, 0.10, 1.]);
        } else {
            for k in 1..=3 {
                let f = k as f32 / 4.;
                let dx = cx + (px - cx) * f;
                let dy = (cy + 14.) + (py - (cy + 14.)) * f;
                self.gfx.fill_circle(dx, dy, 2.8, [0.05, 0.12, 0.22, 0.7]);
                self.gfx.fill_circle(dx, dy, 2.0, GOLD);
            }
            self.gfx.fill_circle(px, py, 5.5, GOLD);
            self.gfx.stroke_circle(px, py, 5.5, 1.5, [0.10, 0.16, 0.28, 1.]);
        }
        // メダリオン背景（どんな絵地図の上でもくっきり見えるように）
        self.gfx.fill_circle(cx, cy + 3., 22., [0., 0., 0., 0.38]);
        self.gfx
            .stroke_circle(cx, cy, 22. + pulse * 3., 2., [0.35, 0.90, 1.0, 0.65]);
        self.gfx.fill_circle(cx, cy, 20., [0.07, 0.16, 0.28, 0.96]);
        self.gfx
            .stroke_circle(cx, cy, 20., 3.5, [1.0, 0.98, 0.90, 1.0]);
        self.gfx.stroke_circle(cx, cy, 20., 2.0, GOLD);

        // 探偵マントと赤い蝶ネクタイ
        self.gfx.fill_round(
            cx - 13.,
            cy + 7.,
            cx + 13.,
            cy + 18.,
            6.,
            [0.18, 0.38, 0.60, 1.],
        );
        self.gfx
            .fill_circle(cx - 3.2, cy + 10.5, 2.6, [0.90, 0.28, 0.25, 1.]);
        self.gfx
            .fill_circle(cx + 3.2, cy + 10.5, 2.6, [0.90, 0.28, 0.25, 1.]);
        self.gfx.fill_circle(cx, cy + 10.5, 1.8, GOLD);

        // 顔・耳・前髪
        self.gfx
            .fill_circle(cx - 10.5, cy + 0.5, 2.8, [1.0, 0.87, 0.74, 1.]);
        self.gfx
            .fill_circle(cx + 10.5, cy + 0.5, 2.8, [1.0, 0.87, 0.74, 1.]);
        self.gfx
            .fill_circle(cx, cy - 0.5, 10.5, [1.0, 0.90, 0.78, 1.]);
        self.gfx.fill_round(
            cx - 10.,
            cy - 8.5,
            cx + 10.,
            cy - 2.5,
            4.,
            [0.32, 0.20, 0.12, 1.],
        );
        self.gfx
            .fill_circle(cx, cy + 1.0, 9.3, [1.0, 0.90, 0.78, 1.]);
        // ほっぺ
        self.gfx
            .fill_circle(cx - 6.2, cy + 2.5, 2.4, [1.0, 0.55, 0.58, 0.65]);
        self.gfx
            .fill_circle(cx + 6.2, cy + 2.5, 2.4, [1.0, 0.55, 0.58, 0.65]);
        // 目（まばたき＋ハイライト）
        let blink = (self.clock * 0.8) % 3.4 < 0.12;
        for ex in [cx - 3.8, cx + 3.8] {
            if blink {
                self.gfx.fill_rect(
                    ex - 2.2,
                    cy - 0.2,
                    ex + 2.2,
                    cy + 1.0,
                    [0.18, 0.12, 0.08, 1.],
                );
            } else {
                self.gfx
                    .fill_circle(ex, cy + 0.2, 2.3, [0.14, 0.10, 0.08, 1.]);
                self.gfx
                    .fill_circle(ex + 0.7, cy - 0.5, 0.9, [1.0, 1.0, 1.0, 0.95]);
            }
        }
        // にっこり口
        self.gfx.fill_round(
            cx - 2.2,
            cy + 4.0,
            cx + 2.2,
            cy + 6.0,
            1.0,
            [0.80, 0.32, 0.28, 1.],
        );

        // 探偵帽（鹿撃ち帽風）
        self.gfx.fill_round(
            cx - 11.5,
            cy - 15.5,
            cx + 11.5,
            cy - 5.5,
            6.,
            [0.78, 0.54, 0.26, 1.],
        );
        self.gfx
            .fill_circle(cx, cy - 16.5, 2.5, [0.52, 0.32, 0.14, 1.]);
        self.gfx.fill_rect(
            cx - 11.,
            cy - 9.0,
            cx + 11.,
            cy - 6.5,
            [0.42, 0.24, 0.10, 1.],
        );
        self.gfx.fill_round(
            cx - 13.5,
            cy - 7.0,
            cx + 13.5,
            cy - 4.2,
            2.,
            [0.62, 0.40, 0.18, 1.],
        );
        self.gfx.fill_circle(cx, cy - 11.0, 2.8, GOLD);
        self.gfx.fill_circle(cx, cy - 11.0, 1.4, CYAN);

        // 虫眼鏡
        self.gfx.fill_round(
            cx + 14.5,
            cy + 7.5,
            cx + 19.5,
            cy + 14.5,
            1.5,
            [0.45, 0.26, 0.12, 1.],
        );
        self.gfx
            .fill_circle(cx + 14., cy + 4., 5.5, [0.65, 0.95, 1.0, 0.88]);
        self.gfx.stroke_circle(cx + 14., cy + 4., 5.5, 2.0, GOLD);
        self.gfx
            .fill_circle(cx + 12.2, cy + 2.2, 1.5, [1.0, 1.0, 1.0, 0.95]);

        // 「きみ」の名札
        let ly = if cy - 38. < self.map_rect()[1] + 4. {
            cy + 21.
        } else {
            cy - 38.
        };
        self.gfx
            .fill_round(cx - 22., ly, cx + 22., ly + 18., 9., [1.0, 0.96, 0.82, 0.98]);
        self.gfx
            .stroke_round(cx - 22., ly, cx + 22., ly + 18., 9., 1.8, GOLD);
        self.text(
            "きみ",
            cx - 22.,
            ly,
            44.,
            18.,
            Font::NormalCenter,
            [0.15, 0.10, 0.05, 1.],
        );
    }

    fn field_ui(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let top = self.top();
        let r = self.map_rect();
        let ei = self.journal.era;
        let era = &ERAS[ei];
        let compact = self.gfx.compact;
        // 絵地図があれば、それを敷いて印を重ねる
        let map_img = self.gfx.image(&images::map(ei));
        if let Some(img) = &map_img {
            self.gfx
                .draw_image(img, r[0], r[1], r[0] + r[2], r[1] + r[3], 1.);
            image_tag(&mut self.gfx, r[0] + r[2], r[1] + r[3]);
            for (i, p) in era.places.iter().enumerate() {
                let (x, y) = (r[0] + p.x * r[2], r[1] + p.y * r[3]);
                let here = i == self.journal.place && self.travel.is_none();
                if here {
                    let pulse = 0.5 + 0.5 * (self.clock * 3.).sin();
                    self.gfx
                        .fill_circle(x, y, 26. + pulse * 6., [1., 0.85, 0.45, 0.30]);
                }
                self.gfx
                    .stroke_circle(x, y, 16., 3., [0.2, 0.12, 0.05, 0.6]);
                self.gfx.stroke_circle(
                    x,
                    y,
                    16.,
                    2.,
                    if here {
                        [1., 0.78, 0.3, 1.]
                    } else {
                        [0.98, 0.94, 0.84, 0.95]
                    },
                );
            }
        }
        // 地名の注記
        for f in era.features.iter().filter(|_| map_img.is_none()) {
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
            self.targets
                .push((Target::Place(i), [x - 22., y - 22., 44., 44.]));
            self.buttons.push(Button {
                rect: [x - 22., y - 22., 44., 44.],
                act: Act::Goto(i),
            });
        }
        let (ax, ay) = (r[0] + self.avatar.0 * r[2], r[1] + self.avatar.1 * r[3]);
        self.player_avatar(ax, ay);
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
        self.speaker_button(ix + iw - 76., y0 + 52.);
        self.button(
            "説明",
            [ix + iw - 160., y0 + 52., 78., 32.],
            Act::Brief(ei),
            false,
        );
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
            .fill_rect(ix, y0 + 88., ix + iw, y0 + 89., alpha(EDGE, 0.8));
        let foot = if compact { 52. } else { 60. };
        let cy0 = y0 + 92.;
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
                self.targets.push((Target::Person(i), rr));
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
                self.targets.push((Target::Event(i), rr));
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
        self.card_rect = Some([x, y, cw, ch]);
        [x, y, cw, ch]
    }

    fn meet_ui(&mut self, i: usize, page: usize) {
        self.talk_ui(i, talk::of(i), page);
    }

    /// 出来事の場面の高さ（画像が無ければ0）
    fn banner_h(&mut self, i: usize, tw: f32) -> f32 {
        if self.gfx.image(&images::event(EVENTS[i].id)).is_some() {
            (tw * 9. / 16.).min(320.).min(self.gfx.height as f32 * 0.32) + 12.
        } else {
            0.
        }
    }

    /// 出来事の場面の画像を描き、使った高さを返す
    fn banner(&mut self, i: usize, x: f32, y: f32, tw: f32) -> f32 {
        let Some(img) = self.gfx.image(&images::event(EVENTS[i].id)) else {
            return 0.;
        };
        let h = (tw * 9. / 16.).min(320.).min(self.gfx.height as f32 * 0.32);
        cover(&mut self.gfx, &img, [x, y, tw, h], 16. / 9., 1.);
        self.gfx
            .stroke_rect(x, y, x + tw, y + h, 1., alpha(GOLD, 0.6));
        image_tag(&mut self.gfx, x + tw, y + h);
        h + 12.
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
        need += self.banner_h(i, tw);
        let terms = notes::for_event(e.id);
        let mh = self.memo_h(&terms, tw);
        if mh > 0. {
            need += mh + 12.;
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
        yy += self.banner(i, x + 30., yy, tw);
        for para in e.body {
            let ph = self.gfx.measure_h(para, Font::Normal, tw);
            self.text(para, x + 30., yy, tw, ph, Font::Normal, TEXT);
            yy += ph + 10.;
        }
        let mh = self.memo(&terms, x + 30., yy, tw);
        if mh > 0. {
            yy += mh + 12.;
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
            ("なぜ？ノート", Tab::Words),
        ];
        let tw = if compact { 96. } else { 136. };
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
        let rx = 16. + 5. * (tw + 8.);
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
            Tab::Words => self.codex_words(gy - off, w, gy, h),
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

    /// 図鑑「なぜ？ノート」：クロノに聞いた疑問と答え
    fn codex_words(&mut self, y0: f32, w: f32, vy0: f32, vy1: f32) -> f32 {
        let q = self.query.clone();
        let mut order: Vec<usize> = (0..notes::NOTES.len()).collect();
        order.sort_by_key(|i| (notes::era(*i), *i));
        let items: Vec<usize> = order
            .into_iter()
            .filter(|i| self.filter.is_none_or(|f| notes::era(*i) == f))
            .filter(|i| {
                let n = &notes::NOTES[*i];
                q.is_empty()
                    || ((self.journal.notes[*i] || self.reveal) && contains(&[n.q, n.a], &q))
            })
            .collect();
        let learned = self.journal.notes.iter().filter(|t| **t).count();
        self.text(
            &format!(
                "クロノに聞いたなぜ？ {}/{}　（人物の話や出来事で「なぜ？」を押すとたまるよ）",
                learned,
                notes::NOTES.len()
            ),
            18.,
            y0 + 2.,
            w - 36.,
            20.,
            Font::Normal,
            CYAN,
        );
        let cols = (((w - 32.) / 420.) as usize).max(1);
        let cw = (w - 32. - (cols - 1) as f32 * 10.) / cols as f32;
        let ch = if self.gfx.compact { 150. } else { 170. };
        let y0 = y0 + 28.;
        for (k, i) in items.iter().enumerate() {
            let x = 16. + (k % cols) as f32 * (cw + 10.);
            let y = y0 + (k / cols) as f32 * (ch + 10.);
            if y + ch < vy0 || y > vy1 {
                continue;
            }
            let n = &notes::NOTES[*i];
            let found = self.journal.notes[*i];
            let known = found || self.reveal;
            let c = era_color(notes::era(*i));
            self.gfx.fill_round(
                x,
                y,
                x + cw,
                y + ch,
                8.,
                if known {
                    [0.98, 0.96, 0.89, 1.]
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
            self.text(
                &format!("{}　{}", ERAS[notes::era(*i)].name, notes::scene(*i)),
                x + 14.,
                y + 8.,
                cw - 28.,
                16.,
                Font::Small,
                if known { INK_SOFT } else { alpha(c, 0.95) },
            );
            if known {
                self.text(n.q, x + 14., y + 26., cw - 28., 24., Font::Big, INK);
                self.gfx
                    .clip_begin(x, y.max(vy0), x + cw, (y + ch - 6.).min(vy1));
                self.text(
                    n.a,
                    x + 14.,
                    y + 54.,
                    cw - 28.,
                    ch - 60.,
                    Font::Small,
                    INK_SOFT,
                );
                self.gfx.clip_end();
            } else {
                self.text("？？？", x + 14., y + 30., cw - 28., 24., Font::Big, MUTED);
                self.text(
                    "この場面で、クロノに「なぜ？」を聞いてみよう",
                    x + 14.,
                    y + 60.,
                    cw - 28.,
                    40.,
                    Font::Small,
                    LOCKED,
                );
            }
        }
        if items.is_empty() {
            self.text(
                "この時代のなぜ？ノートは準備中です。",
                18.,
                y0 + 10.,
                w - 36.,
                24.,
                Font::Normal,
                MUTED,
            );
        }
        28. + items.len().div_ceil(cols) as f32 * (ch + 10.)
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
            self.gem_era(x + 50., y + 70., 30., *era, known);
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

    /// 秘宝の印。秘宝の画像があればそれを描く
    fn gem_era(&mut self, x: f32, y: f32, r: f32, era: usize, lit: bool) {
        if lit {
            if let Some(img) = self.gfx.image(&images::treasure_image(era)) {
                let p = 0.5 + 0.5 * (self.clock * 2.).sin();
                self.gfx
                    .fill_circle(x, y, r + 8. + p * 3., alpha(era_color(era), 0.22));
                let s = r * 1.25;
                self.gfx.draw_image(&img, x - s, y - s, x + s, y + s, 1.);
                return;
            }
        }
        self.gem(x, y, r, era_color(era), lit);
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
        if let Some(img) = self.gfx.image(images::AKASHIC) {
            cover(&mut self.gfx, &img, [0., 0., w, h], 16. / 9., 0.9);
        }
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
        self.card_rect = Some([x, y, cw, ch]);
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
        // 答えたあとで、問題に出てきた用語をクロノが説明する（答えの手がかりにならないように）
        let terms: Vec<usize> = vec![];
        let mh = self.memo_h(&terms, cw0 - 60.);
        let need = 46. + 18. + qh + 4. * row + 34. + nh + 64. + if mh > 0. { mh + 10. } else { 0. };
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
            cw - 260.,
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
            let cx = x + cw - 124. - (list.len() - 1 - k) as f32 * 20.;
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
            let nh = self.gfx.measure_h(&note, Font::Normal, cw - 60.);
            self.text(&note, x + 30., ny + 28., cw - 60., nh, Font::Normal, MUTED);
            self.memo(&terms, x + 30., ny + 28. + nh + 10., cw - 60.);
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
            self.gem_era(x + 80., gy + 20., 40., era, true);
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
        self.gem_era(x + 74., y + 90., 40., era, known);
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
                // 場面の画像があれば右に小さく
                let thumb = self.gfx.image(&images::event(e.id));
                let tw = if thumb.is_some() { cw - 150. } else { cw - 28. };
                self.gfx
                    .clip_begin(x, y.max(vy0), x + cw, (y + ch - 4.).min(vy1));
                if let Some(img) = thumb {
                    let (tx, ty) = (x + cw - 130., y + 30.);
                    self.gfx.draw_image(&img, tx, ty, tx + 118., ty + 66.4, 1.);
                    self.gfx
                        .stroke_rect(tx, ty, tx + 118., ty + 66.4, 1., alpha(GOLD, 0.5));
                }
                self.text(e.title, x + 14., y + 30., tw, 24., Font::Big, TEXT);
                let lead = e.body[0];
                self.gfx.clip_end();
                self.gfx
                    .clip_begin(x, y.max(vy0), x + 14. + tw, (y + ch - 26.).min(vy1));
                self.text(lead, x + 14., y + 56., tw, 36., Font::Small, MUTED);
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
            if let Entry::Event(i) = e {
                yy += self.banner(i, x + 30., yy, tw);
            }
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
            let terms = entry_notes(e);
            for para in paras {
                let ph = self.gfx.measure_h(para, Font::Normal, tw);
                self.text(para, x + 30., yy, tw, ph, Font::Normal, TEXT);
                yy += ph + 10.;
            }
            let mh = self.memo(&terms, x + 30., yy, tw);
            if mh > 0. {
                yy += mh + 12.;
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
        let _ = &paras;
        let terms = entry_notes(e);
        let mh = self.memo_h(&terms, tw);
        if mh > 0. {
            need += mh + 12.;
        }
        if let Entry::Event(i) = e {
            need += self.banner_h(i, tw);
        }
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
            "時空事件簿",
            x + 30.,
            y + 20.,
            cw - 60.,
            40.,
            Font::TitleMid,
            GOLD,
        );
        let body = "あなたは歴史のなぞを追う「時空探偵」。タイムマシンで昔の時代へ行き、何が起きたのかを調べます。歴史上の人物と話すこともできますが、歴史を変えることはできません。探偵との会話は「時空の復元力」によって歴史から消えてしまうからです。（時空探偵とナビのクロノは物語のキャラクターです。出会う人物と出来事は本当の歴史です）\n\n1　世界地図で行き先の時代を選び、「転移する」でその時代へ渡ります。\n\n2　時代の地図では、場所を押すと道をたどって移動します。矢印キーでも移動できます。「人」の印はまだ会っていない人物、「！」はまだ観察していない出来事のある場所です。\n\n3　人物に「会う」と人物一覧表に、出来事を「観察」すると出来事集と年表に記録されます。\n\n4　その時代の人物と出来事をすべて集めると「アカシックレコードの試練」が始まります。時代についての四択問題5問に全問正解すると、その時代にゆかりのある実在の品が「秘宝」として手に入ります。何度でも挑み直せます。\n\n5　図鑑では、人物一覧表・年表・出来事集・秘宝を見返せます。「未発見も表示」をONにすると、まだ見つけていない項目も読めます（学習用）。\n\n―― 内容の正確さについて ――\n年代・人物・出来事は、各項目に示した史料と研究に基づいてまとめています。日付に諸説あるものや史料によって食い違うものは本文に記しています。和暦の月日は当時の暦（旧暦）のままです。\n人物の言葉は実際の発言ではありません。会話は、史料に伝わる出来事や人柄をもとに、その人の気持ちを想像して書いたものです。人物がいる場所は、その人物にゆかりのある場所です。地図は位置関係を示す概略図で、建物の形や距離は正確ではありません。肖像や場面の画像は、時代考証に基づくイメージ（想像図を含む）です。\n\nF2 図鑑 ／ F3 使い方 ／ F11 全画面 ／ Esc 戻る ／ Enter 次へ・転移";
        let by = y + 72.;
        let bh = ch - 72. - 62.;
        self.scroll_areas.push(([x, by, cw, bh], Scroll::Overlay));
        let off = self.scroll[Scroll::Overlay as usize];
        let th = self.gfx.measure_h(body, Font::Normal, cw - 64.);
        self.gfx.clip_begin(x, by, x + cw, by + bh);
        self.text(body, x + 32., by - off, cw - 64., th, Font::Normal, TEXT);
        // 古い版が残って新しくならないとき用（記録は消えない）
        let mut extra = 0.;
        if platform::CAN_RELOAD {
            let ry = by - off + th + 18.;
            let note = "画面が新しい版にならないときは、キャッシュを消して読み込み直せます。旅の記録は消えません。";
            let nh = self.gfx.measure_h(note, Font::Small, cw - 64.);
            self.text(note, x + 32., ry, cw - 64., nh, Font::Small, MUTED);
            let r = [x + 32., ry + nh + 8., 300., 38.];
            if r[1] >= by && r[1] + r[3] <= by + bh {
                self.button("キャッシュを消して再読み込み", r, Act::ReloadClear, false);
            } else {
                self.gfx
                    .fill_round(r[0], r[1], r[0] + r[2], r[1] + r[3], 6., PANEL2);
                self.text(
                    "キャッシュを消して再読み込み",
                    r[0] + 4.,
                    r[1] + 1.,
                    r[2] - 8.,
                    r[3] - 2.,
                    Font::NormalCenter,
                    TEXT,
                );
            }
            extra = 18. + nh + 8. + 38.;
        }
        self.gfx.clip_end();
        self.scroll_max[Scroll::Overlay as usize] = th + extra - bh + 10.;
        self.button(
            "はじめのおはなし",
            [x + 30., y + ch - 50., 190., 36.],
            Act::Story(0),
            false,
        );
        let tips_off = self.journal.flags & FLAG_TIPS_OFF != 0;
        self.button(
            if tips_off {
                "クロノの案内：OFF"
            } else {
                "クロノの案内：ON"
            },
            [x + 230., y + ch - 50., 190., 36.],
            Act::TipsOff,
            !tips_off,
        );
        self.button(
            "旅に戻る",
            [x + cw - 230., y + ch - 50., 200., 36.],
            Act::Close,
            true,
        );
    }
}

/// 図鑑の説明に添えるなぜ？ノート
fn entry_notes(e: Entry) -> Vec<usize> {
    match e {
        Entry::Person(i) => notes::for_person_all(PEOPLE[i].id),
        Entry::Event(i) => notes::for_event(EVENTS[i].id),
    }
}

/// 画像を枠いっぱいに、縦横比を保って描く（はみ出す分は切り抜く）
fn cover(g: &mut Gfx, img: &crate::gfx::Image, r: [f32; 4], aspect: f32, a: f32) {
    let [x, y, w, h] = r;
    let (iw, ih) = if w / h > aspect {
        (w, w / aspect)
    } else {
        (h * aspect, h)
    };
    g.clip_begin(x, y, x + w, y + h);
    g.draw_image(
        img,
        x + (w - iw) / 2.,
        y + (h - ih) / 2.,
        x + (w + iw) / 2.,
        y + (h + ih) / 2.,
        a,
    );
    g.clip_end();
}

/// 画像の隅に「イメージ」と示す
fn image_tag(g: &mut Gfx, x1: f32, y1: f32) {
    g.fill_round(x1 - 62., y1 - 24., x1 - 6., y1 - 6., 4., [0., 0., 0., 0.55]);
    g.text(
        "イメージ",
        x1 - 62.,
        y1 - 24.,
        56.,
        18.,
        Font::NormalCenter,
        [1., 1., 1., 0.9],
    );
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
    /// 会話を最後まで進めて閉じるのに必要な「次へ」の回数
    fn meet_steps(_i: usize) -> usize {
        2
    }
    use super::*;

    /// 導入を済ませた状態のゲーム（ほかの機能を調べるため）
    fn game(gfx: Gfx) -> Game {
        let mut g = Game::new(gfx);
        g.journal.flags = FLAGS_DONE;
        g
    }

    /// 転移する。はじめての時代では、クロノの時代の説明を最後まで読んでから転移する
    fn shift(g: &mut Game, era: usize) {
        g.act(Act::Shift(era, None));
        let mut pages = 0;
        while let Overlay::Briefing(e, _) = g.overlay {
            assert_eq!(e, era);
            g.render();
            g.act(Act::Next);
            pages += 1;
        }
        assert!(pages == 0 || pages == briefing::pages(era).len());
        assert_eq!(g.mode, Mode::Shift);
    }

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
        let mut g = game(Gfx::new(1600, 900));
        for era in 0..ERAS.len() {
            g.act(Act::PickEra(era));
            shift(&mut g, era);
            assert_eq!(g.mode, Mode::Shift);
            run(&mut g, warp::SHIFT_SECS + 0.2);
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
                    for _ in 0..meet_steps(i) {
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
        let mut g = game(Gfx::new(1600, 900));
        shift(&mut g, EDO);
        run(&mut g, warp::SHIFT_SECS + 0.2);
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
    fn first_launch_tells_the_story_then_guides() {
        let mut g = Game::new(Gfx::new(1600, 900));
        run(&mut g, 1.);
        // 表紙：最初のタップは「はじめる」。ここで iPad などの音声も使えるようになる
        assert_eq!(g.overlay, Overlay::Title);
        g.render();
        assert!(g.buttons.iter().any(|b| matches!(b.act, Act::Next)));
        g.click(10., 10.);
        assert_eq!(
            g.overlay,
            Overlay::Title,
            "a stray tap does not skip the first page"
        );
        g.handle(Ev::Key(13, true));
        assert_eq!(g.overlay, Overlay::Story(0));
        assert!(g.last_speech.starts_with(intro::PAGES[0].title));
        for page in 1..intro::PAGES.len() {
            g.handle(Ev::Key(13, true));
            assert_eq!(g.overlay, Overlay::Story(page));
            g.render();
        }
        g.act(Act::StoryBack);
        assert_eq!(g.overlay, Overlay::Story(intro::PAGES.len() - 2));
        g.handle(Ev::Key(13, true));
        g.handle(Ev::Key(13, true));
        assert_eq!(g.overlay, Overlay::None);
        assert!(g.journal.flags & FLAG_INTRO != 0);
        run(&mut g, 1.);
        assert_eq!(g.overlay, Overlay::None, "the story is shown only once");
        // クロノが「転移する」を指す
        assert_eq!(g.current_tip().unwrap().1, Some(intro::Target::Shift));
        assert!(g.targets.iter().any(|(t, _)| *t == intro::Target::Shift));
        shift(&mut g, ROME);
        run(&mut g, warp::SHIFT_SECS + 0.2);
        // フォルムにはアントニウスがいる
        let antony = person_index("antony").unwrap();
        assert_eq!(
            g.current_tip().unwrap().1,
            Some(intro::Target::Person(antony))
        );
        g.act(Act::Meet(antony));
        for _ in 0..meet_steps(antony) {
            g.act(Act::Next);
        }
        // はじめての会話のあとは、時空の復元力の説明
        assert_eq!(g.overlay, Overlay::Restore(antony));
        g.render();
        g.act(Act::Next);
        assert_eq!(g.overlay, Overlay::None);
        assert!(g.journal.flags & FLAG_RESTORE != 0);
        // 2回目からは、操作を止めない短い演出
        let cicero = person_index("cicero").unwrap();
        g.act(Act::Goto(place_index(ROME, "palatine").unwrap()));
        run(&mut g, 10.);
        g.act(Act::Meet(cicero));
        for _ in 0..meet_steps(cicero) {
            g.act(Act::Next);
        }
        assert_eq!(g.overlay, Overlay::None);
        assert!(g.restore_fx.is_some());
        g.render();
        run(&mut g, intro::RESTORE_FX_SECS + 0.2);
        assert!(g.restore_fx.is_none());
        // 次は出来事の場所へ案内
        assert!(matches!(
            g.current_tip().unwrap().1,
            Some(intro::Target::Place(_))
        ));
        g.act(Act::Goto(place_index(ROME, "forum").unwrap()));
        run(&mut g, 10.);
        assert!(matches!(
            g.current_tip().unwrap().1,
            Some(intro::Target::Event(_))
        ));
        g.act(Act::Observe(event_index("lupercalia").unwrap()));
        g.act(Act::Close);
        assert_eq!(g.current_tip().unwrap().1, Some(intro::Target::Codex));
        g.act(Act::Codex);
        assert!(g.current_tip().is_none());
        // 導入の進み具合は保存される
        let j = Journal::read(&g.journal.bytes()).unwrap();
        assert_eq!(j.flags & FLAGS_DONE, FLAGS_DONE);
    }

    /// クロノのなぜ？ノート：疑問を押すと答えが開き、読み上げ、図鑑に残る
    #[test]
    fn chrono_answers_why_questions() {
        let mut g = game(Gfx::new(1600, 900));
        shift(&mut g, FLORENCE);
        run(&mut g, warp::SHIFT_SECS + 0.2);
        g.act(Act::Goto(place_index(FLORENCE, "signoria").unwrap()));
        run(&mut g, 10.);
        let cascina = event_index("cascina").unwrap();
        g.act(Act::Observe(cascina));
        g.render();
        // 疑問が並び、まだ答えは開いていない
        let list = notes::for_event("cascina");
        assert!(list.len() >= 3);
        for i in &list {
            assert!(g
                .buttons
                .iter()
                .any(|b| matches!(b.act, Act::Note(k) if k == *i)));
        }
        assert_eq!(g.memo_open, None);
        let patron = notes::index("cascina-patron").unwrap();
        let why = notes::index("cascina-why-battle").unwrap();
        let closed = g.memo_h(&list, 800.);
        g.act(Act::Note(patron));
        g.render();
        assert_eq!(g.memo_open, Some(patron));
        assert!(
            g.memo_h(&list, 800.) > closed,
            "the answer opens below the question"
        );
        assert!(g.last_speech.contains("パトロン"));
        assert!(g.journal.notes[patron]);
        assert!(!g.journal.notes[why]);
        // 別の疑問を押すと、そちらに切り替わる。もう一度押すと閉じる
        g.act(Act::Note(why));
        g.render();
        assert_eq!(g.memo_open, Some(why));
        assert!(g.last_speech.contains("ピサ"));
        g.act(Act::Note(why));
        assert_eq!(g.memo_open, None);
        // あわせて見る出来事へ
        g.act(Act::Note(patron));
        g.render();
        assert!(g
            .buttons
            .iter()
            .any(|b| matches!(b.act, Act::Detail(Entry::Event(k)) if EVENTS[k].id == "anghiari")));
        // 人物の話のページにもノートがある。ページが変わると閉じた状態から
        g.act(Act::Close);
        g.act(Act::Goto(place_index(FLORENCE, "opera").unwrap()));
        run(&mut g, 10.);
        let mi = person_index("michelangelo").unwrap();
        g.act(Act::Meet(mi));
        g.act(Act::Ask(2));
        g.render();
        let pieta = notes::index("michelangelo-pieta").unwrap();
        assert!(g
            .buttons
            .iter()
            .any(|b| matches!(b.act, Act::Note(k) if k == pieta)));
        g.act(Act::Note(pieta));
        g.render();
        assert_eq!(g.memo_open, Some(pieta));
        g.act(Act::Ask(0));
        g.render();
        assert_eq!(g.memo_open, None);
        // 保存と図鑑
        let j = Journal::read(&g.journal.bytes()).unwrap();
        assert_eq!(j.notes, g.journal.notes);
        g.act(Act::Close);
        g.act(Act::Codex);
        g.act(Act::Tab(Tab::Words));
        g.render();
        g.gfx.set_compact(true);
        g.render();
    }

    /// スマートフォンでも、答えを開いたノートが会話の枠の中で読める長さ
    #[test]
    fn why_notes_fit_on_phones() {
        let mut g = game(Gfx::new(844, 390));
        g.gfx.set_compact(true);
        for i in 0..notes::NOTES.len() {
            g.memo_open = Some(i);
            g.memo_for = g.overlay;
            let h = g.memo_h(&[i], g.gfx.width as f32 - 80.);
            assert!(h < g.gfx.height as f32 * 1.2, "{}", notes::NOTES[i].id);
        }
    }

    /// 読み上げる文の熟語は、すべて読み辞書（src/speech_ja.tsv）でかなに直る
    #[test]
    fn every_spoken_compound_has_a_reading() {
        use crate::{lang::Lang, speech::text_for_speech};
        let mut missing = std::collections::BTreeMap::new();
        for text in voice::all_speech() {
            let spoken = text_for_speech(&text, Lang::Ja);
            let mut run = String::new();
            for c in spoken.chars().chain(std::iter::once(' ')) {
                if matches!(c, '一'..='龯' | '々') {
                    run.push(c);
                } else {
                    if run.chars().count() >= 2 {
                        missing.entry(run.clone()).or_insert_with(|| text.clone());
                    }
                    run.clear();
                }
            }
            assert_eq!(
                text_for_speech(&spoken, Lang::Ja),
                spoken,
                "再変換で読みが変わる: {text}"
            );
        }
        if !missing.is_empty() {
            let list: Vec<String> = missing.keys().cloned().collect();
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("output/missing-readings.txt");
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            let _ = std::fs::write(&path, list.join("\n"));
            panic!(
                "読み辞書に無い熟語が{}語あります（output/missing-readings.txt）。例: {:?}",
                list.len(),
                &list[..list.len().min(10)]
            );
        }
    }

    #[test]
    fn chrono_tips_follow_the_arrival_guide_instead_of_cutting_it_off() {
        let mut g = Game::new(Gfx::new(1600, 900));
        g.journal.flags = FLAG_INTRO;
        shift(&mut g, HEIAN);
        run(&mut g, warp::SHIFT_SECS + 0.2);
        assert!(
            g.last_speech.starts_with(ERAS[HEIAN].name),
            "{}",
            g.last_speech
        );
        assert!(g.last_speech.contains("クロノ。"));
        // 平安京の東市は「ひがしのいち」、長安の東市は「とうし」
        assert!(
            voice::place_speech(HEIAN, place_index(HEIAN, "eastmarket").unwrap())
                .starts_with("ひがしのいち")
        );
        assert!(
            voice::place_speech(CHANGAN, place_index(CHANGAN, "east").unwrap())
                .starts_with("とうし")
        );
    }

    #[test]
    fn replay_reads_the_current_screen() {
        let mut g = game(Gfx::new(1600, 900));
        g.voice = false;
        g.act(Act::Replay);
        assert!(g.last_speech.contains(ERAS[ROME].name));
        shift(&mut g, ROME);
        run(&mut g, warp::SHIFT_SECS + 0.2);
        let antony = person_index("antony").unwrap();
        g.act(Act::Meet(antony));
        assert!(g.last_speech.contains("フォルム"));
        g.render();
        assert!(g.buttons.iter().any(|b| matches!(b.act, Act::Replay)));
        // 質問すると、その答えを読む。🔊（F4）で同じ答えを読み直す
        g.act(Act::Ask(2));
        assert!(g.last_speech.contains("執政官"));
        let said = g.last_speech.clone();
        g.handle(Ev::Key(0x73, true));
        assert_eq!(g.last_speech, said);
    }

    /// すべての人物の会話のすべての場面が、スマートフォンでも最後の質問と「別れる」まで届く
    #[test]
    fn every_talk_fits_on_phones() {
        let mut g = game(Gfx::new(844, 390));
        g.gfx.set_compact(true);
        for i in 0..PEOPLE.len() {
            let n = talk::of(i).asks.len();
            for page in 0..talk::pages(i) {
                g.overlay = Overlay::Meet(i, page);
                g.scroll[Scroll::Overlay as usize] = 0.;
                g.render();
                assert!(
                    g.buttons.iter().any(|b| matches!(b.act, Act::Next)),
                    "{} {page}",
                    PEOPLE[i].id
                );
                g.scroll[Scroll::Overlay as usize] = g.scroll_max[Scroll::Overlay as usize].max(0.);
                g.render();
                if page <= n {
                    assert!(
                        g.buttons
                            .iter()
                            .any(|b| matches!(b.act, Act::Ask(k) if k == n - 1)),
                        "{} {page}",
                        PEOPLE[i].id
                    );
                }
            }
        }
    }

    /// 使い方の画面の下に、キャッシュを消して読み込み直すボタンがある（スマートフォンでも届く）
    #[test]
    fn help_offers_cache_clearing() {
        for (w, h, compact) in [(844, 390, true), (1600, 900, false)] {
            let mut g = game(Gfx::new(w, h));
            g.gfx.set_compact(compact);
            g.overlay = Overlay::Help;
            g.render();
            g.scroll[Scroll::Overlay as usize] = g.scroll_max[Scroll::Overlay as usize].max(0.);
            g.render();
            assert!(
                g.buttons.iter().any(|b| matches!(b.act, Act::ReloadClear)),
                "{w}x{h}"
            );
        }
    }

    /// 転移先の一覧・図鑑・右の欄：項目の上でもなぞればスクロールし、押せば選べる
    #[test]
    fn list_items_accept_swipes_and_taps_at_mobile_scales() {
        for (w, h, compact) in [(844, 390, true), (1194, 834, true), (1600, 900, false)] {
            let mut g = game(Gfx::new(w, h));
            g.gfx.set_compact(compact);
            let ui = g.gfx.ui;
            let center = |r: [f32; 4]| ((r[0] + r[2] / 2.) * ui, (r[1] + r[3] / 2.) * ui);
            let tap = |g: &mut Game, (x, y): (f32, f32)| {
                g.handle(Ev::MouseMove(x, y));
                g.handle(Ev::Down(0, x, y));
                g.handle(Ev::Up(0, x, y));
            };
            // 転移先の一覧
            g.render();
            let row = g
                .buttons
                .iter()
                .find(|b| matches!(b.act, Act::PickEra(1)) && b.rect[0] < g.atlas_list_w())
                .unwrap()
                .rect;
            let (x, y) = center(row);
            assert_eq!(g.hit_kind(x, y), 3, "era row at {w}x{h}");
            if g.scroll_max[Scroll::Atlas as usize] > 0. {
                g.handle(Ev::MouseMove(x, y));
                g.handle(Ev::Scroll(-40. * ui));
                assert!(g.scroll[Scroll::Atlas as usize] > 0.);
                g.handle(Ev::Scroll(10000.));
                assert_eq!(g.scroll[Scroll::Atlas as usize], 0.);
            }
            tap(&mut g, (x, y));
            assert_eq!(g.pick, 1);
            // 上の帯のボタンは、なぞってもスクロールしない
            let top = g
                .buttons
                .iter()
                .find(|b| matches!(b.act, Act::Codex))
                .unwrap()
                .rect;
            let (tx, ty) = center(top);
            assert_eq!(g.hit_kind(tx, ty), 1);
            // 図鑑のカード
            g.act(Act::Codex);
            g.render();
            let card = g
                .buttons
                .iter()
                .find(|b| matches!(b.act, Act::Detail(_)))
                .unwrap()
                .clone_rect();
            let (cx, cy) = center(card);
            assert_eq!(g.hit_kind(cx, cy), 3, "codex card at {w}x{h}");
            assert!(g.scroll_max[Scroll::Codex as usize] > 0.);
            g.handle(Ev::MouseMove(cx, cy));
            g.handle(Ev::Scroll(-60. * ui));
            assert!(g.scroll[Scroll::Codex as usize] > 0.);
            g.handle(Ev::Scroll(100000.));
            g.render();
            tap(&mut g, (cx, cy));
            assert!(matches!(g.overlay, Overlay::Detail(_)));
            // 図鑑のタブは、なぞってもスクロールしない
            g.act(Act::Close);
            g.render();
            let tab = g
                .buttons
                .iter()
                .find(|b| matches!(b.act, Act::Tab(_)))
                .unwrap()
                .rect;
            let (tx, ty) = center(tab);
            assert_eq!(g.hit_kind(tx, ty), 1);
        }
    }

    /// はじめての時代では、転移の前にクロノが時代の前後の流れを説明する
    #[test]
    fn chrono_briefs_an_era_before_the_first_visit() {
        let mut g = game(Gfx::new(1600, 900));
        g.act(Act::Shift(EDO, None));
        assert_eq!(g.overlay, Overlay::Briefing(EDO, 0));
        assert!(g.last_speech.starts_with(briefing::OPENING));
        g.render();
        g.act(Act::Next);
        assert_eq!(g.overlay, Overlay::Briefing(EDO, 1));
        g.act(Act::BriefBack);
        assert_eq!(g.overlay, Overlay::Briefing(EDO, 0));
        // とばしても転移はする
        g.handle(Ev::Key(27, true));
        assert_eq!(g.mode, Mode::Shift);
        run(&mut g, warp::SHIFT_SECS + 0.2);
        assert!(g.journal.visited[EDO]);
        // 2回目からは説明なしで転移し、読み直しはいつでもできる
        g.act(Act::Atlas);
        g.act(Act::Shift(EDO, None));
        assert_eq!(g.mode, Mode::Shift);
        run(&mut g, warp::SHIFT_SECS + 0.2);
        g.render();
        assert!(g.buttons.iter().any(|b| matches!(b.act, Act::Brief(EDO))));
        g.act(Act::Brief(EDO));
        for _ in 0..briefing::pages(EDO).len() {
            g.render();
            g.act(Act::Next);
        }
        assert_eq!(g.overlay, Overlay::None);
        assert_eq!(g.mode, Mode::Field, "reviewing does not travel again");
        // スマートフォンでも描ける
        g.gfx.set_compact(true);
        for era in 0..ERAS.len() {
            for page in 0..briefing::pages(era).len() {
                g.show_briefing(era, page);
                g.render();
            }
        }
    }

    #[test]
    fn story_can_be_skipped_and_replayed() {
        let mut g = Game::new(Gfx::new(900, 420));
        g.gfx.set_compact(true);
        run(&mut g, 1.);
        for page in 0..intro::PAGES.len() {
            g.open_story(page);
            g.render();
        }
        g.handle(Ev::Key(27, true));
        assert_eq!(g.overlay, Overlay::None);
        assert!(g.journal.flags & FLAG_INTRO != 0);
        g.act(Act::Help);
        g.render();
        g.act(Act::Story(0));
        assert_eq!(g.overlay, Overlay::Story(0));
        g.act(Act::StorySkip);
        g.act(Act::TipsOff);
        assert!(g.current_tip().is_none());
    }

    /// 前の版（v2〜v6）の形の記録を作る
    fn legacy_save(
        magic: &[u8; 8],
        era: u8,
        visited: u8,
        treasures: u8,
        flags: u8,
        met: &[&str],
        notes_ids: &[&str],
    ) -> Vec<u8> {
        let mut b = magic.to_vec();
        b.extend([era, 0, visited]);
        if magic != MAGIC_V2 {
            b.push(treasures);
        }
        if magic != MAGIC_V2 && magic != MAGIC_V3 {
            b.push(flags);
        }
        push_ids(&mut b, met.to_vec());
        push_ids(&mut b, vec![]);
        if magic == MAGIC_V5 || magic == MAGIC_V6 {
            push_ids(&mut b, notes_ids.to_vec());
        }
        let hash = checksum(&b);
        b.extend_from_slice(&hash.to_le_bytes());
        b
    }

    #[test]
    fn older_saves_skip_the_introduction() {
        let b = legacy_save(MAGIC_V3, 0, 1, 0, 0, &["caesar"], &[]);
        assert_eq!(Journal::read(&b).unwrap().flags, FLAGS_DONE);
    }

    #[test]
    fn reads_saves_without_treasures() {
        let b = legacy_save(MAGIC_V2, 0, 0b11, 0, 0, &["antony"], &[]);
        let j = Journal::read(&b).unwrap();
        assert!(j.visited[ROME] && j.visited[CHANGAN]);
        assert!(j.met[person_index("antony").unwrap()]);
        assert!(j.treasures.iter().all(|t| !t));
        assert_eq!(j.flags, FLAGS_DONE);
    }

    /// 時代を番号で記録していた版：番号は前の7つの時代の並びのまま読む
    #[test]
    fn old_era_numbers_keep_their_meaning() {
        let note = notes::NOTES[0].id;
        let b = legacy_save(
            MAGIC_V6,
            6,
            1 << 6 | 1,
            1 << 6,
            FLAGS_DONE,
            &["katsu"],
            &[note],
        );
        let j = Journal::read(&b).unwrap();
        assert_eq!(j.era, EDO);
        assert!(j.visited[EDO] && j.visited[ROME]);
        assert_eq!(j.visited.iter().filter(|v| **v).count(), 2);
        assert!(j.treasures[EDO]);
        assert!(j.notes[0]);
        // 用語メモの一覧を持つ v5 も読める
        let b = legacy_save(MAGIC_V5, 5, 0, 0, FLAGS_DONE, &[], &["old-term"]);
        assert_eq!(Journal::read(&b).unwrap().era, PARIS);
        // 番号の範囲を外れた印は不正
        assert!(Journal::read(&legacy_save(MAGIC_V6, 0, 1 << 7, 0, 0, &[], &[])).is_err());
    }

    #[test]
    fn meeting_requires_being_there() {
        let mut g = game(Gfx::new(1600, 900));
        let caesar = person_index("caesar").unwrap();
        g.act(Act::Meet(caesar));
        assert!(!g.journal.met[caesar]);
        shift(&mut g, ROME);
        run(&mut g, warp::SHIFT_SECS + 0.2);
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
        let mut g = game(Gfx::new(1600, 900));
        shift(&mut g, ROME);
        run(&mut g, warp::SHIFT_SECS + 0.2);
        // フォルムから西へ：ポンペイウス劇場（道でつながる）かカピトリウム
        g.handle(Ev::Key(37, true));
        run(&mut g, 10.);
        let id = ERAS[ROME].places[g.journal.place].id;
        assert!(id == "capitol" || id == "pompey", "{id}");
    }

    #[test]
    fn all_views_render_and_search_filters() {
        for (w, h, c) in [(1600, 900, false), (1360, 720, false), (900, 420, true)] {
            let mut g = game(Gfx::new(w, h));
            g.gfx.set_compact(c);
            g.frame(&[], 0.016, true);
            for era in 0..ERAS.len() {
                shift(&mut g, era);
                run(&mut g, warp::SHIFT_SECS + 0.2);
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
        assert_eq!(EVENTS[c[0]].era, ALEXANDRIA);
    }
}
