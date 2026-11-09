//! 導入：はじめのおはなし、ナビのクロノの案内、時空の復元力の演出。
//!
//! プレイヤーは歴史のなぞを追う「時空探偵」。タイムマシンで昔の時代へ行き、
//! 何が起きたのかを調べる。歴史上の人物と話せるが、歴史は変えられない。
//! 探偵との会話は「時空の復元力」によって歴史から消えてしまう。
//! 小学校低学年でも読めるよう、文はひらがなを中心に分かち書きにする。
use super::*;

/// クロノの案内が指すもの
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Shift,
    Codex,
    Place(usize),
    Person(usize),
    Event(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Detective,
    Machine,
    People,
    Rule,
    Erase,
    Book,
    Chrono,
    Truth,
}

pub struct Page {
    pub scene: Scene,
    pub title: &'static str,
    pub body: &'static str,
}

/// クロノの案内の文
pub static TIPS: [&str; 6] = [
    "いきたい じだいを えらんで、「転移する」を おしてね！",
    "「会う」を おして、おはなしを きいてみよう！",
    "「人」の しるしの ある ばしょを おしてみよう。そこまで あるいて いけるよ。",
    "「観察」を おすと、ここで おきた できごとが みられるよ。",
    "「！」の しるしの ある ばしょへ いってみよう。",
    "あつめた きろくは「図鑑」で みられるよ。おしてみて！",
];

pub static PAGES: &[Page] = &[
    Page {
        scene: Scene::Detective,
        title: "ようこそ、じくう たんてい！",
        body: "きみは、れきしの なぞを おいかける「じくう たんてい」。\nむかし ほんとうに なにが おきたのかを しらべるのが、\nきみの しごとだよ。",
    },
    Page {
        scene: Scene::Machine,
        title: "タイムマシンで しゅっぱつ！",
        body: "きみの タイムマシンは、むかしの じだいへ ひとっとび。\nローマ、ちょうあん、へいあんきょう、パリ、えど……\nせかいの いろいろな まちへ いけるよ。",
    },
    Page {
        scene: Scene::People,
        title: "むかしの ひとに あえる",
        body: "その じだいに いきていた ひとたちに あって、\nおはなしを きく ことが できるんだ。\nだれに あえるかな？",
    },
    Page {
        scene: Scene::Rule,
        title: "でも、ひとつ だいじな きまり",
        body: "じくう たんていは、れきしを かえる ことは できない。\nきみが あった ことや はなした ことは、\n「じくうの ふくげんりょく」で れきしから きえてしまうんだ。",
    },
    Page {
        scene: Scene::Erase,
        title: "じくうの ふくげんりょく って？",
        body: "れきしが「もとの かたちに もどろう」と する、ふしぎな ちから。\nむかしの ひとは、きみと はなした ことを わすれてしまう。\nだから れきしは かわらない。あんしんして いろいろ きいてみよう！",
    },
    Page {
        scene: Scene::Book,
        title: "しらべた ことは ずかんに のこる",
        body: "きみが みたり きいたり した ことは、きみの「ずかん」に のこるよ。\nその じだいの きろくを ぜんぶ あつめると……\n「アカシックレコードの しれん」が まっている！",
    },
    Page {
        scene: Scene::Chrono,
        title: "ぼくは ナビの クロノ！",
        body: "タイムマシンの あんないがかり、クロノだよ。\nどこを おせば いいか、ぼくが おしえるね。\nこまったら「使い方」を おしてね。",
    },
    Page {
        scene: Scene::Truth,
        title: "さあ、しゅっぱつ！",
        body: "じくう たんていと クロノは、おはなしの なかの キャラクター。\nでも、であう ひとや できごとは、ほんとうに あった れきしだよ。\nさあ、れきしの なぞを しらべに いこう！",
    },
];

/// 会話のあとの、時空の復元力の演出の長さ（秒）
pub const RESTORE_FX_SECS: f32 = 3.2;

/// はじめての復元力の説明の読み上げ
pub fn restore_speech(i: usize) -> String {
    format!(
        "あっ、じくうの ふくげんりょくだ。{}は、きみと はなした ことを わすれてしまった。れきしは もとの まま。きみが きいた おはなしは、ずかんに のこっているよ。",
        PEOPLE[i].name
    )
}

const BODY: [f32; 4] = [0.46, 0.80, 0.93, 1.];

impl Game {
    pub(super) fn open_story(&mut self, page: usize) {
        let page = page.min(PAGES.len() - 1);
        self.overlay = Overlay::Story(page);
        self.story_t = 0.;
        self.say(voice::story_speech(page));
    }

    pub(super) fn finish_story(&mut self) {
        self.overlay = Overlay::None;
        self.hush();
        if self.journal.flags & FLAG_INTRO == 0 {
            self.journal.flags |= FLAG_INTRO;
            self.change();
        }
        self.notify("まずは いきたい じだいを えらぼう！");
    }

    // -------------------------------------------------------------- 絵の部品
    /// 星空（時空のあいだ）
    pub(super) fn starfield(&mut self, a: f32) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        self.gfx.fill_rect(0., 0., w, h, [0.02, 0.04, 0.10, a]);
        for k in 0..90u32 {
            let hx = (k.wrapping_mul(2654435761) >> 7) % 10000;
            let hy = (k.wrapping_mul(40503).wrapping_add(977) >> 3) % 10000;
            let tw = 0.5 + 0.5 * (self.clock * (0.8 + (k % 5) as f32 * 0.3) + k as f32).sin();
            self.gfx.fill_circle(
                hx as f32 / 10000. * w,
                hy as f32 / 10000. * h,
                0.8 + (k % 3) as f32 * 0.6,
                [0.8, 0.92, 1., (0.25 + 0.6 * tw) * a],
            );
        }
        let (cx, cy) = (w / 2., h * 0.34);
        for k in 0..4 {
            let r = 90. + k as f32 * 70. + (self.clock * 0.8 + k as f32).sin() * 5.;
            self.gfx.stroke_circle(
                cx,
                cy,
                r,
                1.,
                alpha(CYAN, (0.10 + 0.04 * (3 - k) as f32) * a),
            );
        }
    }

    /// ナビのクロノ（丸いロボット）。s は体の半径
    pub(super) fn chrono(&mut self, x: f32, y: f32, s: f32, wave: bool) {
        let t = self.clock;
        let y = y + (t * 2.2).sin() * s * 0.05;
        // アンテナ
        self.gfx.fill_rect(
            x - 1.5,
            y - s * 1.42,
            x + 1.5,
            y - s * 0.92,
            [0.25, 0.35, 0.45, 1.],
        );
        let glow = 0.6 + 0.4 * (t * 4.).sin();
        self.gfx
            .fill_circle(x, y - s * 1.5, s * 0.22, alpha(GOLD, 0.35 * glow));
        self.gfx.fill_circle(x, y - s * 1.5, s * 0.14, GOLD);
        // 腕
        let wv = if wave { (t * 6.).sin() * s * 0.35 } else { 0. };
        self.gfx
            .fill_circle(x - s * 1.02, y + s * 0.25, s * 0.2, BODY);
        self.gfx.fill_circle(
            x + s * 1.02,
            y + s * 0.25 - s * 0.3 - wv.abs(),
            s * 0.2,
            BODY,
        );
        // 足
        self.gfx.fill_round(
            x - s * 0.55,
            y + s * 0.85,
            x - s * 0.15,
            y + s * 1.12,
            s * 0.12,
            [0.30, 0.55, 0.68, 1.],
        );
        self.gfx.fill_round(
            x + s * 0.15,
            y + s * 0.85,
            x + s * 0.55,
            y + s * 1.12,
            s * 0.12,
            [0.30, 0.55, 0.68, 1.],
        );
        // 体
        self.gfx.fill_circle(x, y, s, BODY);
        self.gfx
            .fill_circle(x - s * 0.38, y - s * 0.45, s * 0.22, [1., 1., 1., 0.35]);
        self.gfx.stroke_circle(x, y, s, 2., [0.22, 0.42, 0.55, 1.]);
        // 顔の画面
        self.gfx.fill_round(
            x - s * 0.66,
            y - s * 0.5,
            x + s * 0.66,
            y + s * 0.22,
            s * 0.3,
            [0.07, 0.12, 0.22, 1.],
        );
        let blink = (t * 0.7) % 3.2 < 0.12;
        for ex in [x - s * 0.28, x + s * 0.28] {
            if blink {
                self.gfx.fill_rect(
                    ex - s * 0.12,
                    y - s * 0.16,
                    ex + s * 0.12,
                    y - s * 0.11,
                    CYAN,
                );
            } else {
                self.gfx.fill_circle(ex, y - s * 0.14, s * 0.13, CYAN);
                self.gfx
                    .fill_circle(ex + s * 0.04, y - s * 0.18, s * 0.04, [1., 1., 1., 0.9]);
            }
        }
        self.gfx.fill_round(
            x - s * 0.16,
            y + s * 0.02,
            x + s * 0.16,
            y + s * 0.1,
            s * 0.04,
            CYAN,
        );
        // おなかの時計
        let (cx, cy, cr) = (x, y + s * 0.58, s * 0.2);
        self.gfx.fill_circle(cx, cy, cr, [0.98, 0.95, 0.85, 1.]);
        self.gfx.stroke_circle(cx, cy, cr, 2., GOLD);
        let a = t * 1.5;
        for k in 1..4 {
            let r = cr * 0.22 * k as f32;
            self.gfx
                .fill_circle(cx + a.cos() * r, cy + a.sin() * r, 1.4, NIGHT);
        }
        for k in 1..3 {
            let r = cr * 0.25 * k as f32;
            let b = a / 12. - 1.2;
            self.gfx
                .fill_circle(cx + b.cos() * r, cy + b.sin() * r, 1.6, NIGHT);
        }
    }

    /// 時空の粒（下から上へ消えていく光）
    fn sparkles(&mut self, x: f32, y: f32, w: f32, h: f32, n: u32, t: f32, a: f32) {
        for k in 0..n {
            let fx = ((k.wrapping_mul(2654435761) >> 8) % 1000) as f32 / 1000.;
            let speed = 0.25 + ((k * 37) % 10) as f32 / 20.;
            let p = (t * speed + fx * 3.7) % 1.;
            let px = x + fx * w + (t * 2. + k as f32).sin() * 6.;
            let py = y + h - p * h;
            let fade = (1. - p) * (p * 4.).min(1.);
            self.gfx.fill_circle(
                px,
                py,
                1.5 + (k % 3) as f32,
                alpha(if k % 2 == 0 { CYAN } else { GOLD }, fade * a),
            );
        }
    }

    /// 吹き出し（左下に向いたしっぽつき）
    fn bubble(&mut self, x: f32, y: f32, w: f32, h: f32, a: f32) {
        self.gfx
            .fill_round(x, y, x + w, y + h, 14., [0.98, 0.97, 0.92, 0.97 * a]);
        self.gfx
            .stroke_round(x, y, x + w, y + h, 14., 2., alpha(GOLD, a));
        for k in 0..4 {
            let f = k as f32;
            self.gfx.fill_circle(
                x - 4. - f * 7.,
                y + h * 0.55 + f * 6.,
                6. - f * 1.2,
                [0.98, 0.97, 0.92, 0.97 * a],
            );
        }
    }

    // -------------------------------------------------------------- はじめのおはなし
    /// 表紙：「はじめる」を押すと、おはなしが読み上げとともに始まる
    pub(super) fn title_ui(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let compact = self.gfx.compact;
        self.starfield(0.97);
        let cy = h * if compact { 0.20 } else { 0.26 };
        self.text("YYHistory", 0., cy - 40., w, 60., Font::Title, GOLD);
        self.text(
            "じくう たんていの じけんぼ",
            0.,
            cy + 26.,
            w,
            40.,
            Font::TitleMid,
            TEXT,
        );
        let s = if compact { 30. } else { 46. };
        self.chrono(w / 2., cy + 80. + s * 1.6, s, true);
        let bw = 300.;
        let by = h * if compact { 0.70 } else { 0.68 };
        let p = 0.5 + 0.5 * (self.clock * 3.).sin();
        self.gfx.stroke_round(
            w / 2. - bw / 2. - 4. - p * 4.,
            by - 4. - p * 4.,
            w / 2. + bw / 2. + 4. + p * 4.,
            by + 64. + p * 4.,
            16.,
            2.,
            alpha(GOLD, 0.4 + 0.5 * p),
        );
        self.button(
            "▶ はじめる",
            [w / 2. - bw / 2., by, bw, 60.],
            Act::Next,
            true,
        );
        self.text(
            if self.voice {
                "おはなしを こえで よみあげるよ（「声」で けせるよ）"
            } else {
                "「声」を ON に すると、おはなしを よみあげるよ"
            },
            0.,
            by + 72.,
            w,
            24.,
            Font::NormalCenter,
            MUTED,
        );
        self.button(
            "とばす",
            [w - 110., self.top() + 12., 92., 32.],
            Act::StorySkip,
            false,
        );
        self.button(
            if self.voice { "声 ON" } else { "声 OFF" },
            [w - 196., self.top() + 12., 76., 32.],
            Act::Voice,
            self.voice,
        );
    }

    pub(super) fn story_ui(&mut self, page: usize) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let compact = self.gfx.compact;
        let fade = (self.story_t * 3.).min(1.);
        self.starfield(0.97);
        let p = &PAGES[page];
        let (cx, cy) = (w / 2., h * if compact { 0.30 } else { 0.34 });
        let s = if compact { 0.75 } else { 1. };
        match p.scene {
            Scene::Detective => {
                let bob = (self.clock * 1.6).sin() * 6.;
                for (k, (dx, dy)) in [(-190., -40.), (170., -70.), (-130., 70.), (210., 50.)]
                    .into_iter()
                    .enumerate()
                {
                    let fl = (self.clock * 1.3 + k as f32).sin() * 8.;
                    self.text(
                        "？",
                        cx + dx * s - 30.,
                        cy + dy * s + fl - 30.,
                        60.,
                        60.,
                        Font::Title,
                        alpha(GOLD, 0.65 * fade),
                    );
                }
                let r = 74. * s;
                self.gfx
                    .fill_circle(cx, cy + bob, r, alpha(CYAN, 0.16 * fade));
                self.gfx
                    .stroke_circle(cx, cy + bob, r, 12. * s, alpha(GOLD, fade));
                self.gfx.fill_circle(
                    cx - r * 0.35,
                    cy + bob - r * 0.35,
                    r * 0.2,
                    [1., 1., 1., 0.35 * fade],
                );
                for k in 0..7 {
                    let f = 0.78 + k as f32 * 0.12;
                    self.gfx.fill_circle(
                        cx + r * f,
                        cy + bob + r * f,
                        11. * s,
                        alpha([0.55, 0.36, 0.20, 1.], fade),
                    );
                }
            }
            Scene::Machine => {
                let bob = (self.clock * 2.).sin() * 8.;
                let (x0, y0) = (cx - 160. * s, cy - 62. * s + bob);
                let (x1, y1) = (cx + 160. * s, cy + 62. * s + bob);
                for k in 0..6 {
                    let ly = cy + bob - 40. * s + k as f32 * 16. * s;
                    let len = 60. + ((self.clock * 7. + k as f32 * 1.7).sin() * 0.5 + 0.5) * 120.;
                    self.gfx.fill_rect(
                        x0 - len * s,
                        ly,
                        x0 - 10.,
                        ly + 3.,
                        alpha(CYAN, 0.35 * fade),
                    );
                }
                self.gfx
                    .fill_round(x0, y0, x1, y1, 62. * s, alpha([0.32, 0.42, 0.62, 1.], fade));
                self.gfx
                    .stroke_round(x0, y0, x1, y1, 62. * s, 3., alpha(GOLD, fade));
                self.gfx.fill_round(
                    x0 + 30. * s,
                    y0 + 10. * s,
                    x1 - 30. * s,
                    y0 + 26. * s,
                    8.,
                    [1., 1., 1., 0.25 * fade],
                );
                // 窓の時計
                let (wx, wy, wr) = (cx - 55. * s, cy + bob, 38. * s);
                self.gfx
                    .fill_circle(wx, wy, wr, alpha([0.98, 0.95, 0.85, 1.], fade));
                self.gfx.stroke_circle(wx, wy, wr, 4., alpha(GOLD, fade));
                let a = self.clock * 3.;
                for k in 1..6 {
                    let r = wr * 0.15 * k as f32;
                    self.gfx.fill_circle(
                        wx + a.cos() * r,
                        wy + a.sin() * r,
                        2.5,
                        alpha(NIGHT, fade),
                    );
                }
                for k in 1..4 {
                    let r = wr * 0.16 * k as f32;
                    let b = -a / 6.;
                    self.gfx.fill_circle(
                        wx + b.cos() * r,
                        wy + b.sin() * r,
                        3.,
                        alpha(NIGHT, fade),
                    );
                }
                // ランプ
                for k in 0..3 {
                    let on = ((self.clock * 3.) as i32 + k) % 3 == 0;
                    self.gfx.fill_circle(
                        cx + 40. * s + k as f32 * 28. * s,
                        cy + bob,
                        9. * s,
                        alpha(if on { GOLD } else { [0.5, 0.45, 0.3, 1.] }, fade),
                    );
                }
                self.chrono(x1 - 10. * s, y0 - 18. * s, 22. * s, false);
            }
            Scene::People => {
                // こども向けに、よみがなで
                let ids = [
                    ("caesar", "カエサル", "ローマ"),
                    ("murasaki", "むらさきしきぶ", "へいあんきょう"),
                    ("leonardo", "レオナルド", "フィレンツェ"),
                    ("katsu", "かつ かいしゅう", "えど"),
                ];
                let r = 56. * s;
                let gap = (w - 120.).min(640. * s) / 4.;
                for (k, (id, name, town)) in ids.iter().enumerate() {
                    let Some(i) = person_index(id) else { continue };
                    let x = cx + (k as f32 - 1.5) * gap;
                    let y = cy + ((self.clock * 1.5 + k as f32).sin() * 6.) - 10.;
                    self.medallion(PEOPLE[i].name, x, y, r, era_color(PEOPLE[i].era), true);
                    self.text(
                        name,
                        x - gap / 2.,
                        y + r + 8.,
                        gap,
                        22.,
                        Font::BigCenter,
                        alpha(TEXT, fade),
                    );
                    self.text(
                        &format!("（{town}）"),
                        x - gap / 2.,
                        y + r + 32.,
                        gap,
                        18.,
                        Font::NormalCenter,
                        alpha(era_color(PEOPLE[i].era), fade),
                    );
                }
            }
            Scene::Rule => {
                // 一本の歴史の流れを、時空の膜が守っている
                let (x0, x1) = (cx - 300. * s, cx + 300. * s);
                self.gfx
                    .fill_round(x0, cy - 4., x1, cy + 4., 4., alpha(GOLD, fade));
                for (k, e) in ERAS.iter().enumerate() {
                    let x = x0 + (x1 - x0) * k as f32 / (ERAS.len() - 1) as f32;
                    self.gfx.fill_circle(x, cy, 9., alpha(era_color(k), fade));
                    self.text(
                        e.years.split('（').next().unwrap_or(e.years),
                        x - 60.,
                        cy + 16. + (k % 2) as f32 * 18.,
                        120.,
                        18.,
                        Font::NormalCenter,
                        alpha(MUTED, fade),
                    );
                }
                let p = 0.5 + 0.5 * (self.clock * 2.).sin();
                self.gfx.stroke_round(
                    x0 - 30.,
                    cy - 70. * s,
                    x1 + 30.,
                    cy + 70. * s,
                    60. * s,
                    3.,
                    alpha(CYAN, (0.4 + 0.4 * p) * fade),
                );
                self.text(
                    "れきし",
                    cx - 100.,
                    cy - 60. * s,
                    200.,
                    30.,
                    Font::BigCenter,
                    alpha(GOLD, fade),
                );
            }
            Scene::Erase => {
                let t = self.story_t % 3.5;
                let a = if t < 1.2 {
                    1.
                } else {
                    (1. - (t - 1.2) / 1.3).max(0.)
                };
                if let Some(i) = person_index("katsu") {
                    self.medallion(
                        PEOPLE[i].name,
                        cx - 150. * s,
                        cy + 20.,
                        50. * s,
                        era_color(PEOPLE[i].era),
                        true,
                    );
                }
                let (bx, by, bw, bh) = (cx - 70. * s, cy - 80. * s, 260. * s, 90. * s);
                self.bubble(bx, by, bw, bh, a * fade);
                self.text(
                    "こんにちは、\nたんていさん！",
                    bx,
                    by,
                    bw,
                    bh,
                    Font::BigCenter,
                    alpha(INK, a * fade),
                );
                let burst = if t > 1.0 {
                    ((t - 1.0) / 1.5).min(1.)
                } else {
                    0.
                };
                self.sparkles(
                    bx,
                    by - 60.,
                    bw,
                    bh + 60.,
                    40,
                    self.story_t,
                    burst * (1. - (t - 2.6).max(0.)) * fade,
                );
            }
            Scene::Book => {
                let (bw, bh) = (180. * s, 150. * s);
                let glow = 0.5 + 0.5 * (self.clock * 2.).sin();
                self.gfx
                    .fill_circle(cx, cy, 170. * s, alpha(GOLD, (0.06 + 0.06 * glow) * fade));
                for (k, sx) in [-1f32, 1.].into_iter().enumerate() {
                    let x0 = if sx < 0. { cx - bw } else { cx };
                    self.gfx.fill_round(
                        x0 + 2.,
                        cy - bh / 2.,
                        x0 + bw - 2.,
                        cy + bh / 2.,
                        10.,
                        alpha([0.97, 0.94, 0.84, 1.], fade),
                    );
                    for l in 0..5 {
                        let ly = cy - bh / 2. + 22. + l as f32 * 22. * s;
                        self.gfx.fill_rect(
                            x0 + 20.,
                            ly,
                            x0 + bw - 20. - (l % 2) as f32 * 30.,
                            ly + 3.,
                            alpha([0.55, 0.45, 0.32, 1.], fade),
                        );
                    }
                    let (label, c) = if k == 0 {
                        ("人", [0.20, 0.50, 0.75, 1.])
                    } else {
                        ("！", [0.82, 0.35, 0.22, 1.])
                    };
                    self.gfx
                        .fill_circle(x0 + bw / 2., cy + bh / 2. - 26., 16., alpha(c, fade));
                    self.text(
                        label,
                        x0 + bw / 2. - 16.,
                        cy + bh / 2. - 42.,
                        32.,
                        32.,
                        Font::BigCenter,
                        alpha(TEXT, fade),
                    );
                }
                self.gfx.fill_rect(
                    cx - 2.,
                    cy - bh / 2.,
                    cx + 2.,
                    cy + bh / 2.,
                    alpha([0.5, 0.38, 0.22, 1.], fade),
                );
                self.gem(cx, cy - bh / 2. - 36., 22., GOLD, true);
                self.sparkles(cx - bw, cy - bh, bw * 2., bh, 24, self.clock, 0.8 * fade);
            }
            Scene::Chrono => {
                self.chrono(cx, cy, 70. * s, true);
            }
            Scene::Truth => {
                let n = ERAS.len() as f32;
                let tw = ((w - 80.) / n - 10.).min(150.);
                let th = tw * 9. / 16.;
                let x0 = cx - (tw + 10.) * n / 2. + 5.;
                for k in 0..ERAS.len() {
                    let x = x0 + k as f32 * (tw + 10.);
                    let y = cy - th / 2. + ((self.clock * 1.4 + k as f32 * 0.9).sin() * 5.);
                    if let Some(img) = self.gfx.image(&images::era_art(k)) {
                        self.gfx.draw_image(&img, x, y, x + tw, y + th, fade);
                    } else {
                        self.gfx.fill_round(
                            x,
                            y,
                            x + tw,
                            y + th,
                            6.,
                            alpha(mix(NIGHT, era_color(k), 0.6), fade),
                        );
                    }
                    self.gfx
                        .stroke_rect(x, y, x + tw, y + th, 1.5, alpha(era_color(k), fade));
                    self.text(
                        ERAS[k].name,
                        x - 20.,
                        y + th + 6. + (k % 2) as f32 * 18.,
                        tw + 40.,
                        18.,
                        Font::NormalCenter,
                        alpha(TEXT, fade),
                    );
                }
                self.chrono(cx, cy + th / 2. + 96. * s, 26. * s, true);
            }
        }

        // 文の枠
        let cw = (w - 40.).min(900.);
        let ch = if compact { 170. } else { 210. };
        let (x, y) = ((w - cw) / 2., h - ch - if compact { 12. } else { 28. });
        self.gfx
            .fill_round(x, y, x + cw, y + ch, 14., [0.05, 0.09, 0.17, 0.94]);
        self.gfx
            .stroke_round(x, y, x + cw, y + ch, 14., 1.5, alpha(CYAN, 0.8));
        self.text(
            p.title,
            x + 20.,
            y + 10.,
            cw - 40.,
            if compact { 34. } else { 44. },
            Font::TitleMid,
            GOLD,
        );
        let ty = y + if compact { 46. } else { 60. };
        self.text(
            p.body,
            x + 24.,
            ty,
            cw - 48.,
            ch - (ty - y) - 50.,
            Font::BigCenter,
            alpha(TEXT, fade),
        );
        // ページの印
        for k in 0..PAGES.len() {
            let dx = x + cw / 2. + (k as f32 - (PAGES.len() - 1) as f32 / 2.) * 18.;
            self.gfx.fill_circle(
                dx,
                y + ch - 22.,
                4.,
                if k == page { GOLD } else { alpha(MUTED, 0.5) },
            );
        }
        let last = page + 1 == PAGES.len();
        if page > 0 {
            self.button(
                "◀ もどる",
                [x + 16., y + ch - 42., 120., 32.],
                Act::StoryBack,
                false,
            );
        }
        self.button(
            if last {
                "しゅっぱつ！"
            } else {
                "つぎへ ▶"
            },
            [x + cw - 176., y + ch - 44., 160., 36.],
            Act::Next,
            true,
        );
        self.button(
            "とばす",
            [w - 110., self.top() + 12., 92., 32.],
            Act::StorySkip,
            false,
        );
        self.speaker_button(w - 196., self.top() + 12.);
    }

    // -------------------------------------------------------------- 時空の復元力
    /// はじめて人物と話したあと：時空の復元力の説明
    pub(super) fn restore_explain_ui(&mut self, i: usize) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let compact = self.gfx.compact;
        self.starfield(0.9);
        let p = &PEOPLE[i];
        let (cx, cy) = (w / 2., h * if compact { 0.28 } else { 0.32 });
        let t = self.story_t;
        // 会話の吹き出しが、光の粒になって消える
        let a = (1. - (t - 0.8) / 1.4).clamp(0., 1.);
        self.medallion(p.name, cx - 120., cy + 10., 56., era_color(p.era), true);
        let (bx, by, bw, bh) = (cx - 20., cy - 80., 220., 80.);
        self.bubble(bx, by, bw, bh, a);
        self.text("……", bx, by, bw, bh, Font::Title, alpha(INK, a));
        self.sparkles(
            bx - 40.,
            by - 90.,
            bw + 80.,
            bh + 120.,
            50,
            t,
            (t * 1.2).min(1.) * 0.9,
        );
        self.chrono(cx + 240., cy + 40., 34., false);
        let cw = (w - 40.).min(860.);
        let ch = if compact { 190. } else { 230. };
        let (x, y) = ((w - cw) / 2., h - ch - if compact { 12. } else { 28. });
        self.gfx
            .fill_round(x, y, x + cw, y + ch, 14., [0.05, 0.09, 0.17, 0.95]);
        self.gfx
            .stroke_round(x, y, x + cw, y + ch, 14., 1.5, alpha(CYAN, 0.8));
        self.text(
            "あっ！ じくうの ふくげんりょくだ！",
            x + 20.,
            y + 10.,
            cw - 40.,
            if compact { 34. } else { 44. },
            Font::TitleMid,
            GOLD,
        );
        let body = format!(
            "{}は、きみと はなした ことを わすれてしまった。\nれきしは もとの まま。だから だいじょうぶ。\nきみが きいた おはなしは、ずかんに ちゃんと のこっているよ。",
            p.name
        );
        let ty = y + if compact { 48. } else { 62. };
        self.text(
            &body,
            x + 24.,
            ty,
            cw - 48.,
            ch - (ty - y) - 52.,
            Font::BigCenter,
            TEXT,
        );
        self.button(
            "わかった！",
            [x + cw / 2. - 90., y + ch - 46., 180., 36.],
            Act::Close,
            true,
        );
        self.speaker_button(x + cw - 96., y + ch - 44.);
    }

    /// 話し終えるたびの、短い復元力の演出（操作は止めない）
    pub(super) fn restore_ui(&mut self) {
        let Some((i, t)) = self.restore_fx else {
            return;
        };
        if self.mode != Mode::Field {
            return;
        }
        let r = self.map_rect();
        let a = (t / 0.3).min(1.) * ((RESTORE_FX_SECS - t) / 0.8).clamp(0., 1.);
        let bw = (r[2] - 40.).min(560.);
        let (bx, by) = (r[0] + (r[2] - bw) / 2., r[1] + 14.);
        self.sparkles(bx, by - 10., bw, 90., 30, t, a);
        self.gfx
            .fill_round(bx, by, bx + bw, by + 56., 10., [0.05, 0.10, 0.20, 0.88 * a]);
        self.gfx
            .stroke_round(bx, by, bx + bw, by + 56., 10., 1.5, alpha(CYAN, a));
        self.text(
            "じくうの ふくげんりょく……",
            bx + 12.,
            by + 4.,
            bw - 24.,
            22.,
            Font::NormalCenter,
            alpha(CYAN, a),
        );
        self.text(
            &format!("{}は、きみとの かいわを わすれた", PEOPLE[i].name),
            bx + 12.,
            by + 26.,
            bw - 24.,
            24.,
            Font::BigCenter,
            alpha(TEXT, a),
        );
    }

    // -------------------------------------------------------------- クロノの案内
    /// いまの状態に合った案内（文, 指す場所）
    pub(super) fn current_tip(&self) -> Option<(&'static str, Option<Target>)> {
        let f = self.journal.flags;
        if f & FLAG_INTRO == 0 || f & FLAG_TIPS_OFF != 0 || self.travel.is_some() {
            return None;
        }
        let met_any = self.journal.met.iter().any(|m| *m);
        let seen_any = self.journal.seen.iter().any(|m| *m);
        match self.mode {
            Mode::Atlas if !self.journal.visited.iter().any(|v| *v) => {
                Some((TIPS[0], Some(Target::Shift)))
            }
            Mode::Field if !met_any => {
                if let Some(&i) = self.people_here().first() {
                    Some((TIPS[1], Some(Target::Person(i))))
                } else {
                    Some((
                        TIPS[2],
                        self.place_with(|g, k| {
                            PEOPLE.iter().enumerate().any(|(i, p)| {
                                p.era == g.journal.era
                                    && p.place == ERAS[g.journal.era].places[k].id
                                    && !g.journal.met[i]
                            })
                        })
                        .map(Target::Place),
                    ))
                }
            }
            Mode::Field if !seen_any => {
                if let Some(&i) = self.events_here().first() {
                    Some((TIPS[3], Some(Target::Event(i))))
                } else {
                    Some((
                        TIPS[4],
                        self.place_with(|g, k| {
                            EVENTS.iter().any(|e| {
                                e.era == g.journal.era
                                    && e.place == ERAS[g.journal.era].places[k].id
                            })
                        })
                        .map(Target::Place),
                    ))
                }
            }
            Mode::Field if f & FLAG_CODEX == 0 => Some((TIPS[5], Some(Target::Codex))),
            _ => None,
        }
    }

    /// いまの場所からいちばん近い、条件に合う場所
    fn place_with(&self, ok: impl Fn(&Game, usize) -> bool) -> Option<usize> {
        let era = &ERAS[self.journal.era];
        let here = &era.places[self.journal.place];
        (0..era.places.len())
            .filter(|k| ok(self, *k))
            .min_by(|a, b| {
                let d = |k: &usize| {
                    let p = &era.places[*k];
                    ((p.x - here.x) * 1.6).powi(2) + (p.y - here.y).powi(2)
                };
                d(a).total_cmp(&d(b))
            })
    }

    pub(super) fn tip_ui(&mut self) {
        let Some((text, target)) = self.current_tip() else {
            return;
        };
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        // 指す場所を光らせる
        if let Some(r) =
            target.and_then(|t| self.targets.iter().find(|(k, _)| *k == t).map(|(_, r)| *r))
        {
            let p = 0.5 + 0.5 * (self.clock * 4.).sin();
            let g = 4. + p * 5.;
            self.gfx.stroke_round(
                r[0] - g,
                r[1] - g,
                r[0] + r[2] + g,
                r[1] + r[3] + g,
                10.,
                3.,
                alpha(GOLD, 0.5 + 0.5 * p),
            );
            let ax = r[0] + r[2] / 2.;
            let ay = r[1] - g - 6. - p * 6.;
            self.text("▼", ax - 15., ay - 26., 30., 26., Font::BigCenter, GOLD);
        }
        // クロノと吹き出し
        let (bx, by) = match self.mode {
            Mode::Field => {
                let r = self.map_rect();
                (r[0] + 70., r[1] + r[3] - 92.)
            }
            _ => {
                let card = if self.gfx.compact { 150. } else { 176. };
                (self.atlas_list_w() + 70., h - card - 100.)
            }
        };
        let s = if self.gfx.compact { 22. } else { 28. };
        let bw = (w * 0.42).clamp(260., 460.);
        let bh = (self.gfx.measure_h(text, Font::Big, bw - 44.) + 18.).max(46.);
        self.gfx
            .fill_circle(bx - 20., by + 46., s + 14., [0.02, 0.04, 0.10, 0.45]);
        self.chrono(bx - 20., by + 46., s, false);
        let (tx, ty) = (bx + s + 6., by);
        self.bubble(tx, ty, bw, bh, 1.);
        self.text(text, tx + 14., ty + 6., bw - 44., bh - 12., Font::Big, INK);
        let close = [tx + bw - 30., ty + 6., 24., 24.];
        self.gfx
            .fill_circle(close[0] + 12., close[1] + 12., 11., [0., 0., 0., 0.12]);
        self.text(
            "×",
            close[0],
            close[1],
            24.,
            24.,
            Font::NormalCenter,
            INK_SOFT,
        );
        self.buttons.push(Button {
            rect: close,
            act: Act::TipsOff,
        });
    }
}

// ------------------------------------------------------------------ クロノのことばメモ
impl Game {
    /// ことばメモの高さ（用語が無ければ0）。一度に1語ずつ見せるので、いちばん長い説明に合わせる
    pub(super) fn memo_h(&mut self, terms: &[usize], w: f32) -> f32 {
        if terms.is_empty() {
            return 0.;
        }
        let tw = w - 92.;
        let tallest = terms
            .iter()
            .map(|&i| self.gfx.measure_h(glossary::TERMS[i].kid, Font::Normal, tw))
            .fold(0f32, f32::max);
        let nav = if terms.len() > 1 { 40. } else { 0. };
        (38. + 24. + tallest + 8. + nav + 4.).max(96.)
    }

    /// クロノが用語をやさしく説明する（1語ずつページ送り）。描いた高さを返す。
    /// 見せた用語は図鑑の「ことば」に残る
    pub(super) fn memo(&mut self, terms: &[usize], x: f32, y: f32, w: f32) -> f32 {
        let h = self.memo_h(terms, w);
        if h == 0. {
            return 0.;
        }
        // 別の画面・別のページに変わったら、1語目から
        if self.memo_for != self.overlay || self.memo_len != terms.len() {
            self.memo_for = self.overlay;
            self.memo_page = 0;
        }
        self.memo_len = terms.len();
        let k = self.memo_page.min(terms.len() - 1);
        let i = terms[k];
        if std::mem::take(&mut self.memo_turned) {
            let t = &glossary::TERMS[i];
            self.say(format!("{}。{}", t.kana, t.kid));
        }
        self.gfx
            .fill_round(x, y, x + w, y + h, 12., [0.98, 0.96, 0.89, 0.97]);
        self.gfx
            .stroke_round(x, y, x + w, y + h, 12., 1.5, alpha(CYAN, 0.9));
        self.chrono(x + 38., y + 44., 20., false);
        self.text(
            "クロノの ことばメモ",
            x + 76.,
            y + 8.,
            w - 90.,
            22.,
            Font::Normal,
            [0.15, 0.45, 0.60, 1.],
        );
        let tw = w - 92.;
        let t = &glossary::TERMS[i];
        self.text(
            &format!("{}（{}）", t.name(), t.kana),
            x + 76.,
            y + 36.,
            tw,
            22.,
            Font::Big,
            INK,
        );
        let eh = self.gfx.measure_h(t.kid, Font::Normal, tw);
        self.text(t.kid, x + 76., y + 60., tw, eh, Font::Normal, INK_SOFT);
        if terms.len() > 1 {
            let ny = y + h - 40.;
            self.button(
                "‹ まえ",
                [x + w - 236., ny, 84., 30.],
                Act::MemoPage(-1),
                false,
            );
            self.text(
                &format!("{} / {}", k + 1, terms.len()),
                x + w - 148.,
                ny + 4.,
                56.,
                22.,
                Font::NormalCenter,
                INK,
            );
            self.button(
                "つぎ ›",
                [x + w - 92., ny, 84., 30.],
                Act::MemoPage(1),
                true,
            );
        }
        if !std::mem::replace(&mut self.journal.terms[i], true) {
            self.change();
        }
        h
    }
}
