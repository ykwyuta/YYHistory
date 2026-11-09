//! 読み上げ：画面ごとに読み上げる文を組み立てる。
//!
//! 文はここで組み立てて `say` に渡す。日本語の熟語は platform::speak の中で
//! 読み辞書（src/speech_ja.tsv）によってかなに直される。
//! 読み上げる文はすべて `all_speech` から作れるようにしておき、
//! 辞書に無い熟語が残っていないかをテストで確かめる。
use super::*;

/// クロノのことばメモの読み上げ（用語は読みがなで読む）
pub fn memo_speech(terms: &[usize]) -> String {
    if terms.is_empty() {
        return String::new();
    }
    let mut s = String::from("クロノの ことばメモ。");
    for &i in terms {
        let t = &glossary::TERMS[i];
        s += &format!("{}。{}", t.kana, t.kid);
    }
    s
}

/// 時代によって読みの違う地名（長安の東市・西市は「とうし・せいし」、
/// 平安京は「ひがしのいち・にしのいち」）
pub fn era_readings(era: usize, text: String) -> String {
    let pairs: &[(&str, &str)] = match ERAS[era].id {
        "changan" => &[("東市", "とうし"), ("西市", "せいし")],
        "heian" => &[("東市", "ひがしのいち"), ("西市", "にしのいち")],
        _ => &[],
    };
    pairs.iter().fold(text, |t, (w, r)| t.replace(w, r))
}

pub fn era_speech(era: usize) -> String {
    era_readings(era, {
        let e = &ERAS[era];
        format!("{}。{}。{}", e.name, e.years, e.intro)
    })
}

pub fn place_speech(era: usize, place: usize) -> String {
    era_readings(era, {
        let p = &ERAS[era].places[place];
        let people = PEOPLE
            .iter()
            .filter(|q| q.era == era && q.place == p.id)
            .count();
        let events = EVENTS
            .iter()
            .filter(|q| q.era == era && q.place == p.id)
            .count();
        format!(
            "{}に ついた。{}。ここには、人物が{}人、出来事が{}件。",
            p.name, p.desc, people, events
        )
    })
}

/// 人物の話（1ページ目は名前と肩書きから）
pub fn meet_speech(i: usize, page: usize) -> String {
    era_readings(PEOPLE[i].era, {
        let p = &PEOPLE[i];
        let line = p.lines[page.min(p.lines.len() - 1)];
        let (head, terms) = if page == 0 {
            (
                format!("{}。{}。", p.name, p.role),
                glossary::find(&format!("{}{}", p.role, line)),
            )
        } else {
            (String::new(), glossary::find(line))
        };
        format!("{head}{line}{}", memo_speech(&terms))
    })
}

pub fn observe_speech(i: usize) -> String {
    era_readings(EVENTS[i].era, {
        let e = &EVENTS[i];
        let body = e.body.join("");
        format!(
            "{}。{}。{}{}",
            e.date,
            e.title,
            body,
            memo_speech(&glossary::find(&body))
        )
    })
}

pub fn detail_speech(e: Entry) -> String {
    era_readings(
        match e {
            Entry::Person(i) => PEOPLE[i].era,
            Entry::Event(i) => EVENTS[i].era,
        },
        {
            match e {
                Entry::Person(i) => {
                    let p = &PEOPLE[i];
                    format!(
                        "{}。{}。{}。{}{}",
                        p.name,
                        p.life,
                        p.role,
                        p.bio,
                        memo_speech(&glossary::find(p.bio))
                    )
                }
                Entry::Event(i) => observe_speech(i),
            }
        },
    )
}

pub fn question_speech(era: usize, q: usize) -> String {
    era_readings(era, {
        let quiz = &QUIZZES[quizzes(era)[q]];
        format!(
            "第{}問。{}。1、{}。2、{}。3、{}。4、{}。",
            q + 1,
            quiz.q,
            quiz.choices[0],
            quiz.choices[1],
            quiz.choices[2],
            quiz.choices[3]
        )
    })
}

pub fn answer_speech(era: usize, q: usize, pick: usize) -> String {
    era_readings(era, {
        let quiz = &QUIZZES[quizzes(era)[q]];
        let terms = glossary::find(&format!("{}{}{}", quiz.q, quiz.choices.join(""), quiz.note));
        if pick == quiz.answer {
            format!("正解。{}{}", quiz.note, memo_speech(&terms))
        } else {
            format!(
                "不正解。正解は、{}。{}{}",
                quiz.choices[quiz.answer],
                quiz.note,
                memo_speech(&terms)
            )
        }
    })
}

pub fn trial_intro_speech(era: usize) -> String {
    format!(
        "この時代の記録は すべて そろった。アカシックレコードの試練が はじまる。{}についての四択問題が{}問。全問正解すると、この時代の秘宝が 手に入る。",
        ERAS[era].name,
        quizzes(era).len()
    )
}

pub fn result_speech(era: usize, score: usize) -> String {
    let n = quizzes(era).len();
    if score == n {
        let t = treasure(era);
        format!("全問正解。秘宝、{}を 手に入れた。{}", t.name, t.desc)
    } else {
        format!(
            "{}問中{}問 正解。全問正解には とどかなかった。図鑑で 読み返してから、もう一度 挑もう。",
            n, score
        )
    }
}

pub fn treasure_speech(era: usize) -> String {
    let t = treasure(era);
    format!("{}。{}", t.name, t.desc)
}

pub fn story_speech(page: usize) -> String {
    let p = &intro::PAGES[page];
    format!("{}。{}", p.title, p.body.replace('\n', ""))
}

impl Game {
    /// 読み上げる（声 OFF のときは覚えておくだけ。🔊 で聞き直せる）
    pub(super) fn say(&mut self, text: String) {
        if self.voice {
            platform::speak(&text);
        }
        self.last_speech = text;
        self.spoke_at = self.clock;
    }

    /// いまの読み上げを止める
    pub(super) fn hush(&mut self) {
        platform::speak("");
    }

    /// 🔊：いまの画面をもう一度読む（声 OFF でも、押したときは読む）
    pub(super) fn replay(&mut self) {
        let text = self.current_speech();
        if !text.is_empty() {
            platform::speak(&text);
            self.last_speech = text;
        }
    }

    /// いま開いている画面の読み上げ
    fn current_speech(&self) -> String {
        match self.overlay {
            Overlay::Meet(i, page) => meet_speech(i, page),
            Overlay::Observe(i) => observe_speech(i),
            Overlay::Detail(e) => detail_speech(e),
            Overlay::TrialIntro(era) => trial_intro_speech(era),
            Overlay::Trial(era, q, None, _) => question_speech(era, q),
            Overlay::Trial(era, q, Some(p), _) => answer_speech(era, q, p),
            Overlay::TrialResult(era, score) => result_speech(era, score),
            Overlay::Treasure(era) => treasure_speech(era),
            Overlay::Story(page) => story_speech(page),
            Overlay::Title => story_speech(0),
            Overlay::Restore(i) => restore_speech(i),
            Overlay::Help | Overlay::None => match self.mode {
                Mode::Atlas => era_speech(self.pick),
                Mode::Field if self.travel.is_none() => {
                    place_speech(self.journal.era, self.journal.place)
                }
                _ => self.last_speech.clone(),
            },
        }
    }

    /// クロノの案内が変わったら読み上げる
    pub(super) fn speak_tip(&mut self) {
        let tip = if self.overlay == Overlay::None && self.mode != Mode::Shift {
            self.current_tip().map(|(t, _)| t)
        } else {
            None
        };
        if tip != self.last_tip {
            self.last_tip = tip;
            if let Some(t) = tip {
                // 同じときに読み始めた説明（到着の案内など）をさえぎらず、そのあとに続ける
                let tip = format!("クロノ。{t}");
                if self.clock - self.spoke_at < 0.3 && !self.last_speech.is_empty() {
                    let sep = if self.last_speech.ends_with('。') {
                        ""
                    } else {
                        "。"
                    };
                    let both = format!("{}{sep}{tip}", self.last_speech);
                    self.say(both);
                } else {
                    self.say(tip);
                }
            }
        }
    }

    /// 読み上げ用の 🔊 ボタン
    pub(super) fn speaker_button(&mut self, x: f32, y: f32) {
        self.button("🔊 きく", [x, y, 76., 32.], Act::Replay, false);
    }
}

/// 読み上げる可能性のある文のすべて（読み辞書の検査用）
#[cfg(test)]
pub fn all_speech() -> Vec<String> {
    let mut v = vec![];
    for era in 0..ERAS.len() {
        v.push(era_speech(era));
        for place in 0..ERAS[era].places.len() {
            v.push(place_speech(era, place));
        }
        for q in 0..quizzes(era).len() {
            v.push(question_speech(era, q));
            for pick in 0..4 {
                v.push(answer_speech(era, q, pick));
            }
        }
        v.push(trial_intro_speech(era));
        for score in 0..=quizzes(era).len() {
            v.push(result_speech(era, score));
        }
        v.push(treasure_speech(era));
    }
    for i in 0..PEOPLE.len() {
        for page in 0..PEOPLE[i].lines.len() {
            v.push(meet_speech(i, page));
        }
        v.push(detail_speech(Entry::Person(i)));
        v.push(restore_speech(i));
    }
    for i in 0..EVENTS.len() {
        v.push(detail_speech(Entry::Event(i)));
    }
    for page in 0..intro::PAGES.len() {
        v.push(story_speech(page));
    }
    v.extend(intro::TIPS.iter().map(|t| format!("クロノ。{t}")));
    v.push("この時代の記録は すべて 集まった。アカシックレコードの試練が 始まる。".to_string());
    v
}
