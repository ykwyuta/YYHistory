//! ゲームで使う画像の一覧。
//!
//! 一覧はゲームのデータ（時代・場所・人物・出来事・秘宝）から作るので、
//! 人物や出来事を足せば必要な画像も自動で増える。
//! `assets/images/manifest.json` と `docs/images.md` はこの一覧から生成したもので、
//! テストが食い違いを検出する（`npm run images:list` で更新）。
//! 置いた画像の抜けやサイズ違いは `npm run images:check` で調べる。
//!
//! 画像が無いあいだ、ゲームはこれまでどおり図形で描く。
#![cfg_attr(not(test), allow(dead_code))]
use crate::history::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Portrait,
    Event,
    Treasure,
    Era,
    Map,
    Ui,
}

impl Category {
    pub fn key(self) -> &'static str {
        match self {
            Category::Portrait => "portrait",
            Category::Event => "event",
            Category::Treasure => "treasure",
            Category::Era => "era",
            Category::Map => "map",
            Category::Ui => "ui",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Category::Portrait => "人物の肖像",
            Category::Event => "出来事の場面",
            Category::Treasure => "秘宝",
            Category::Era => "時代の扉絵",
            Category::Map => "時代の地図",
            Category::Ui => "画面の背景",
        }
    }
    /// 幅・高さ・形式・透過・使う場所
    pub fn spec(self) -> (u32, u32, &'static str, bool, &'static str) {
        match self {
            Category::Portrait => (
                512,
                512,
                "jpg",
                false,
                "会話の画面、右の欄、人物一覧表、図鑑の詳細（円形に切り抜いて表示）",
            ),
            Category::Event => (
                1280,
                720,
                "jpg",
                false,
                "観察の画面、出来事集の詳細、出来事集のカード",
            ),
            Category::Treasure => (
                512,
                512,
                "png",
                true,
                "試練の結果、秘宝の欄・詳細（背景を透過、品物を中央に）",
            ),
            Category::Era => (
                1600,
                900,
                "jpg",
                false,
                "世界地図の時代の札、時空転移の演出",
            ),
            Category::Map => (
                1600,
                1000,
                "jpg",
                false,
                "時代の地図（場所の印・名札・道は画像の上にゲームが重ねる）",
            ),
            Category::Ui => (1600, 900, "jpg", false, "アカシックレコードの試練の背景"),
        }
    }
    pub const ALL: [Category; 6] = [
        Category::Portrait,
        Category::Event,
        Category::Treasure,
        Category::Era,
        Category::Map,
        Category::Ui,
    ];
}

pub struct ImageSpec {
    pub category: Category,
    /// assets/images/ からの相対パス
    pub path: String,
    pub title: String,
    pub era: Option<usize>,
    /// 描く内容と、時代考証の注意
    pub brief: String,
}

pub fn portrait(id: &str) -> String {
    format!("portraits/{id}.jpg")
}
pub fn event(id: &str) -> String {
    format!("events/{id}.jpg")
}
pub fn treasure_image(era: usize) -> String {
    format!("treasures/{}.png", ERAS[era].id)
}
pub fn era_art(era: usize) -> String {
    format!("eras/{}.jpg", ERAS[era].id)
}
pub fn map(era: usize) -> String {
    format!("maps/{}.jpg", ERAS[era].id)
}
pub const AKASHIC: &str = "ui/akashic.jpg";

fn place_name(era: usize, id: &str) -> &'static str {
    place_index(era, id)
        .map(|k| ERAS[era].places[k].name)
        .unwrap_or("")
}

/// 必要な画像をすべて、表示の順に並べる
pub fn required() -> Vec<ImageSpec> {
    let mut v = vec![];
    for era in 0..ERAS.len() {
        let e = &ERAS[era];
        v.push(ImageSpec {
            category: Category::Era,
            path: era_art(era),
            title: format!("{}（{}）", e.name, e.years),
            era: Some(era),
            brief: format!(
                "{}・{}の風景。{} 時代を一目で伝える遠景にする。後世の建物や、その年より後にできたものを描かない。",
                e.area, e.period, e.intro
            ),
        });
        let places: Vec<String> = e
            .places
            .iter()
            .map(|p| {
                format!(
                    "{}（左から{:.0}%・上から{:.0}%）",
                    p.name,
                    p.x * 100.,
                    p.y * 100.
                )
            })
            .collect();
        let roads: Vec<String> = e
            .roads
            .iter()
            .map(|(a, b)| format!("{}－{}", e.places[*a].name, e.places[*b].name))
            .collect();
        v.push(ImageSpec {
            category: Category::Map,
            path: map(era),
            title: format!("{}の地図", e.name),
            era: Some(era),
            brief: format!(
                "{}の鳥瞰図風の絵地図（上が北）。次の場所を、指定の位置（画像の幅・高さに対する割合）に描く：{}。場所どうしを結ぶ道（{}）も描く。ゲームはこの位置に印と名札を重ねるので、位置をずらさない。文字は入れない。{}",
                e.area,
                places.join("、"),
                roads.join("、"),
                e.map_note
            ),
        });
        for p in PEOPLE.iter().filter(|p| p.era == era) {
            v.push(ImageSpec {
                category: Category::Portrait,
                path: portrait(p.id),
                title: p.name.to_string(),
                era: Some(era),
                brief: format!(
                    "{}（{}）、{}、{}。{}（{}）の服装・髪型で、胸から上。同時代の肖像・像が残る人物はそれを参考にし、残らない人物は想像図とする（ゲームでは「イメージ」と表示）。",
                    p.name, p.native, p.life, p.role, e.name, e.years
                ),
            });
        }
        let mut events: Vec<usize> = (0..EVENTS.len())
            .filter(|i| EVENTS[*i].era == era)
            .collect();
        events.sort_by_key(|i| event_key(*i));
        for i in events {
            let ev = &EVENTS[i];
            let people: Vec<&str> = ev
                .people
                .iter()
                .filter_map(|id| person_index(id))
                .map(|k| PEOPLE[k].name)
                .collect();
            v.push(ImageSpec {
                category: Category::Event,
                path: event(ev.id),
                title: format!("{}　{}", ev.date, ev.title),
                era: Some(era),
                brief: format!(
                    "{}、{}。{}{} 本文と史料（{}）に書かれていないことは描き足さない。暴力の場面は直接の描写を避ける。",
                    ev.date,
                    place_name(era, ev.place),
                    ev.body[0],
                    if people.is_empty() {
                        String::new()
                    } else {
                        format!(" 登場する人物：{}。", people.join("、"))
                    },
                    ev.source
                ),
            });
        }
        let t = treasure(era);
        v.push(ImageSpec {
            category: Category::Treasure,
            path: treasure_image(era),
            title: t.name.to_string(),
            era: Some(era),
            brief: format!(
                "{} 実物（{}）の形・模様・文字に忠実に。背景は透過。",
                t.desc, t.source
            ),
        });
    }
    v.push(ImageSpec {
        category: Category::Ui,
        path: AKASHIC.to_string(),
        title: "アカシックレコードの試練".to_string(),
        era: None,
        brief: "あらゆる時代の記録が収められた、星空のような書庫の抽象的な背景。中央に問題の枠を重ねるので、中央は暗く落ち着いた色にする。特定の宗教や実在の建物を描かない。".to_string(),
    });
    v
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// 確認用スクリプトが読む一覧（assets/images/manifest.json）
pub fn manifest_json() -> String {
    let mut o = String::from(
        "{\n  \"note\": \"src/images.rs から生成。手で編集しない\",\n  \"categories\": {\n",
    );
    for (k, c) in Category::ALL.iter().enumerate() {
        let (w, h, fmt, alpha, _) = c.spec();
        o.push_str(&format!(
            "    {}: {{\"title\": {}, \"width\": {}, \"height\": {}, \"format\": {}, \"alpha\": {}}}{}\n",
            json_str(c.key()),
            json_str(c.title()),
            w,
            h,
            json_str(fmt),
            alpha,
            if k + 1 < Category::ALL.len() { "," } else { "" }
        ));
    }
    o.push_str("  },\n  \"images\": [\n");
    let list = required();
    for (k, s) in list.iter().enumerate() {
        o.push_str(&format!(
            "    {{\"path\": {}, \"category\": {}, \"title\": {}, \"era\": {}}}{}\n",
            json_str(&s.path),
            json_str(s.category.key()),
            json_str(&s.title),
            json_str(s.era.map_or("", |e| ERAS[e].name)),
            if k + 1 < list.len() { "," } else { "" }
        ));
    }
    o.push_str("  ]\n}\n");
    o
}

/// 作る人向けの一覧（docs/images.md）
pub fn manifest_md() -> String {
    let list = required();
    let mut o = String::from("# 必要な画像の一覧\n\n");
    o.push_str("このファイルは `src/images.rs` から生成しています（手で編集しないでください）。人物や出来事を追加すると、必要な画像もここに増えます。\n\n");
    o.push_str("画像は `assets/images/` の下に、表のパスのとおりに置きます。置いた画像の抜け・サイズ違い・余分なファイルは `npm run images:check` で確かめられます。画像が無いあいだは、ゲームはこれまでの図形の絵で表示します。\n\n");
    o.push_str("## 規格\n\n| 種類 | 枚数 | 大きさ | 形式 | 使う場所 |\n|---|---|---|---|---|\n");
    for c in Category::ALL {
        let (w, h, fmt, alpha, usage) = c.spec();
        let n = list.iter().filter(|s| s.category == c).count();
        o.push_str(&format!(
            "| {} | {} | {}×{} | {}{} | {} |\n",
            c.title(),
            n,
            w,
            h,
            fmt.to_uppercase(),
            if alpha { "（透過）" } else { "" },
            usage
        ));
    }
    o.push_str(&format!("\n合計 {} 枚。\n\n", list.len()));
    o.push_str("## 時代考証の約束\n\n");
    o.push_str("- 教育用のため、各画像の説明と、ゲーム内の人物・出来事の本文と史料に合わせる。書かれていないこと（人物の表情や持ち物、群衆の人数など）を事実のように強調しない。\n");
    o.push_str("- 服装・建物・道具は、その年代と地域のものにする。後世の建物や、その年より後に生まれたものを描かない。\n");
    o.push_str("- 同時代の肖像・像が残らない人物（紫式部、清少納言、カルプルニア、マリンツィンなど）は想像図になる。ゲームでは画像に「イメージ」と表示する。\n");
    o.push_str("- 暴力や死の場面（暗殺、虐殺、処刑など）は、直接の描写を避ける。\n");
    o.push_str("- 画像の中に文字を入れない（名前や題はゲームが表示する）。\n");
    o.push_str("- 他人の作品や写真をそのまま使わない。博物館の画像を使う場合は、利用条件を確かめて `docs/sources.md` に出典を書く。\n\n");
    for era in 0..ERAS.len() {
        let e = &ERAS[era];
        o.push_str(&format!("## {}（{}）\n\n", e.name, e.years));
        o.push_str("| パス | 内容 | 描く内容と注意 |\n|---|---|---|\n");
        for s in list.iter().filter(|s| s.era == Some(era)) {
            o.push_str(&format!(
                "| `{}` | {}：{} | {} |\n",
                s.path,
                s.category.title(),
                s.title,
                s.brief.replace('|', "／")
            ));
        }
        o.push('\n');
    }
    o.push_str("## 共通\n\n| パス | 内容 | 描く内容と注意 |\n|---|---|---|\n");
    for s in list.iter().filter(|s| s.era.is_none()) {
        o.push_str(&format!(
            "| `{}` | {}：{} | {} |\n",
            s.path,
            s.category.title(),
            s.title,
            s.brief
        ));
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_has_an_image() {
        let list = required();
        let mut paths: Vec<&str> = list.iter().map(|s| s.path.as_str()).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), list.len(), "duplicate image paths");
        for p in PEOPLE {
            assert!(list.iter().any(|s| s.path == portrait(p.id)));
        }
        for e in EVENTS {
            assert!(list.iter().any(|s| s.path == event(e.id)));
        }
        assert_eq!(list.len(), PEOPLE.len() + EVENTS.len() + ERAS.len() * 3 + 1);
    }

    /// 生成した一覧が、リポジトリに置いた一覧と同じか
    #[test]
    fn images_manifest_is_current() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for (path, want) in [
            ("assets/images/manifest.json", manifest_json()),
            ("docs/images.md", manifest_md()),
        ] {
            let file = root.join(path);
            if std::env::var_os("UPDATE_IMAGE_MANIFEST").is_some() {
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(&file, &want).unwrap();
            }
            let have = std::fs::read_to_string(&file).unwrap_or_default();
            assert!(
                have == want,
                "{path} が古くなっています。npm run images:list で更新してください"
            );
        }
    }
}
