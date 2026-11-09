//! 人物との会話の画面：探偵が質問を選び、人物が気持ちをまじえて答える。
use super::*;
use crate::talk::{self, Talk};

/// 質問の行の高さ
const ASK_ROW: f32 = 44.;
/// 探偵の吹き出しの色
const YOU: [f32; 4] = [0.12, 0.30, 0.40, 0.95];

impl Game {
    pub(super) fn talk_ui(&mut self, i: usize, t: &'static Talk, page: usize) {
        let p = &PEOPLE[i];
        let compact = self.gfx.compact;
        let terms = notes::for_person(p.id, page);
        let cw0 = 860.0f32.min(self.gfx.width as f32 - 32.);
        let head = if compact { 150. } else { 170. };
        let bh = self.talk_body(i, t, page, 0., 0., cw0 - 48., false);
        let mh = self.memo_h(&terms, cw0 - 48.);
        let need = head + bh + if mh > 0. { mh + 12. } else { 0. } + 60.;
        let [x, y, cw, ch] = self.card(860., need);
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
        self.text(p.role, tx, y + 74., tw, 20., Font::Normal, CYAN);
        self.text(
            "会話は、史料をもとに気持ちを想像したものです",
            tx,
            y + 98.,
            tw,
            18.,
            Font::Small,
            MUTED,
        );
        // 会話となぜ？ノート（入りきらなければ、なぞって読む）
        let by = y + head;
        let view_h = ch - head - 60.;
        self.scroll_areas.push(([x, by, cw, view_h], Scroll::Overlay));
        let off = self.scroll[Scroll::Overlay as usize];
        self.gfx.clip_begin(x, by, x + cw, by + view_h);
        let ly = by - off;
        let bh = self.talk_body(i, t, page, x + 24., ly, cw - 48., true);
        let mh = self.memo(&terms, x + 24., ly + bh + 12., cw - 48.);
        self.gfx.clip_end();
        self.scroll_max[Scroll::Overlay as usize] =
            bh + if mh > 0. { mh + 12. } else { 0. } - view_h;
        self.button(
            "詳しく（図鑑）",
            [x + 24., y + ch - 46., 150., 34.],
            Act::Detail(Entry::Person(i)),
            false,
        );
        let last = page + 1 >= talk::pages(i);
        let all = self.asked.count_ones() as usize >= t.asks.len();
        self.button(
            if last { "さようなら" } else { "別れる" },
            [x + cw - 200., y + ch - 46., 176., 34.],
            Act::Next,
            last || all,
        );
    }

    /// 会話の本文（ト書き・吹き出し・質問）。draw が false なら高さだけを測る
    fn talk_body(
        &mut self,
        i: usize,
        t: &'static Talk,
        page: usize,
        x: f32,
        y: f32,
        w: f32,
        draw: bool,
    ) -> f32 {
        let n = t.asks.len();
        let mut yy = y;
        // ト書きと、探偵の質問
        let (how, line) = if page == 0 {
            (t.scene, t.greet)
        } else if page <= n {
            let a = &t.asks[page - 1];
            yy += self.talk_bubble("あなた", a.q, x + w * 0.25, yy, w * 0.75, YOU, draw) + 10.;
            (a.how, a.a)
        } else {
            ("", t.bye)
        };
        if !how.is_empty() {
            let s = if page == 0 {
                how.to_string()
            } else {
                format!("（{how}）")
            };
            let h = self.gfx.measure_h(&s, Font::Normal, w - 16.);
            if draw {
                self.text(&s, x + 8., yy, w - 16., h, Font::Normal, MUTED);
            }
            yy += h + 10.;
        }
        let who = PEOPLE[i].name.rsplit('・').next().unwrap_or(PEOPLE[i].name);
        yy += self.talk_bubble(who, line, x, yy, w, [0.05, 0.08, 0.14, 0.95], draw);
        // 質問（別れぎわには出さない）
        if page <= n {
            yy += 16.;
            if draw {
                self.text("きいてみよう", x + 4., yy, w, 20., Font::Small, GOLD);
            }
            yy += 24.;
            for (k, a) in t.asks.iter().enumerate() {
                let here = page == k + 1;
                let done = self.asked & (1 << k) != 0;
                if draw {
                    let r = [x, yy, w, ASK_ROW - 6.];
                    let fill = if here {
                        [0.18, 0.36, 0.46, 1.]
                    } else if done {
                        [0.10, 0.14, 0.22, 1.]
                    } else {
                        PANEL2
                    };
                    self.gfx
                        .fill_round(r[0], r[1], r[0] + r[2], r[1] + r[3], 8., fill);
                    self.gfx.stroke_round(
                        r[0],
                        r[1],
                        r[0] + r[2],
                        r[1] + r[3],
                        8.,
                        1.,
                        if done { EDGE } else { alpha(CYAN, 0.8) },
                    );
                    self.text(
                        if done { "✓" } else { "▶" },
                        r[0] + 6.,
                        r[1] + 7.,
                        24.,
                        24.,
                        Font::NormalCenter,
                        if done { MUTED } else { CYAN },
                    );
                    self.text(
                        a.q,
                        r[0] + 36.,
                        r[1] + 7.,
                        r[2] - 48.,
                        26.,
                        Font::Big,
                        if done && !here { MUTED } else { TEXT },
                    );
                    self.buttons.push(Button {
                        rect: r,
                        act: Act::Ask(k),
                    });
                }
                yy += ASK_ROW;
            }
        }
        yy - y
    }

    /// 話す人の名前つきの吹き出し。高さを返す
    #[allow(clippy::too_many_arguments)]
    fn talk_bubble(
        &mut self,
        who: &str,
        s: &str,
        x: f32,
        y: f32,
        w: f32,
        fill: [f32; 4],
        draw: bool,
    ) -> f32 {
        let th = self.gfx.measure_h(s, Font::Big, w - 44.);
        let h = th + 52.;
        if draw {
            self.gfx.fill_round(x, y, x + w, y + h, 10., fill);
            self.gfx
                .stroke_round(x, y, x + w, y + h, 10., 1., alpha(GOLD, 0.6));
            self.text(who, x + 22., y + 10., w - 44., 20., Font::Small, GOLD);
            self.text(s, x + 22., y + 34., w - 44., th, Font::Big, TEXT);
        }
        h
    }
}
