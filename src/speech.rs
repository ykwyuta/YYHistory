//! 読み上げ専用の読み辞書（YYEarth と同じしくみ）。画面の表記や保存データには適用しない。
//!
//! `speech_ja.tsv` に「表記<TAB>読み」を登録する。同じ位置で最も長く一致する表記を優先し、
//! 置換後のかなは再び置換しない。一文字の漢字は置換しない（文脈で読みが変わるため）。
//! 読み上げる文の熟語がすべて登録されていることは、game.rs のテストで確かめる。
use crate::lang::Lang;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

// 「方」のような多義語は単独で置換せず、意味が決まる句で指定する。
// 人名・官職・年号などの読みはブラウザとWindowsで共通に管理する。
const JA_READINGS: &str = include_str!("speech_ja.tsv");

fn readings() -> &'static HashMap<char, Vec<(&'static str, &'static str)>> {
    static INDEX: OnceLock<HashMap<char, Vec<(&str, &str)>>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index: HashMap<char, Vec<(&str, &str)>> = HashMap::new();
        for line in JA_READINGS
            .lines()
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
        {
            let (written, reading) = line.split_once('\t').expect("読み辞書は表記と読みのTSV");
            index
                .entry(written.chars().next().expect("空の表記は禁止"))
                .or_default()
                .push((written, reading));
        }
        for entries in index.values_mut() {
            entries.sort_by_key(|(written, _)| std::cmp::Reverse(written.len()));
        }
        index
    })
}

pub fn text_for_speech(text: &str, lang: Lang) -> Cow<'_, str> {
    if lang != Lang::Ja || text.is_empty() {
        return Cow::Borrowed(text);
    }
    let mut spoken = String::new();
    let mut cursor = 0;
    let mut copied = 0;
    while cursor < text.len() {
        let rest = &text[cursor..];
        let first = rest.chars().next().unwrap();
        let matched = readings().get(&first).and_then(|entries| {
            entries
                .iter()
                .find(|(written, _)| rest.starts_with(written))
        });
        if let Some(&(written, reading)) = matched {
            spoken.push_str(&text[copied..cursor]);
            spoken.push_str(reading);
            cursor += written.len();
            copied = cursor;
        } else {
            cursor += first.len_utf8();
        }
    }
    if copied == 0 {
        Cow::Borrowed(text)
    } else {
        spoken.push_str(&text[copied..]);
        Cow::Owned(spoken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrects_names_titles_and_era_names() {
        assert_eq!(
            text_for_speech("藤原道長は摂政となった。", Lang::Ja),
            "ふじわらのみちながはせっしょうとなった。"
        );
        assert_eq!(
            text_for_speech("阿倍仲麻呂（朝衡）", Lang::Ja),
            "あべのなかまろ（ちょうこう）"
        );
        assert_eq!(
            text_for_speech("慶応4年（1868年）4月11日", Lang::Ja),
            "けいおう4年（1868年）4月11日"
        );
    }

    #[test]
    fn preserves_other_languages_and_stop_requests() {
        for lang in [Lang::En, Lang::Zh] {
            let text = "執政官、元老院。Consul.";
            assert_eq!(text_for_speech(text, lang), text);
        }
        assert_eq!(text_for_speech("", Lang::Ja), "");
        assert!(matches!(
            text_for_speech("こんにちは", Lang::Ja),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn dictionary_has_unique_keys_and_stable_output() {
        let mut keys = std::collections::HashSet::new();
        for line in JA_READINGS
            .lines()
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
        {
            let (written, reading) = line.split_once('\t').unwrap();
            assert!(keys.insert(written), "表記が重複: {written}");
            assert!(written.chars().count() >= 2, "一文字置換は禁止: {written}");
            assert!(
                !reading.is_empty()
                    && reading
                        .chars()
                        .all(|c| matches!(c, 'ぁ'..='ゖ' | 'ァ'..='ヶ' | 'ー' | '・')),
                "読みはかなのみ: {written}: {reading}"
            );
            assert_eq!(text_for_speech(written, Lang::Ja), reading);
            assert_eq!(text_for_speech(reading, Lang::Ja), reading);
        }
    }

    #[test]
    fn longest_match_handles_compounds_and_utf8_surroundings() {
        assert_eq!(
            text_for_speech("🔊「終身独裁官」と「独裁官」🌏", Lang::Ja),
            "🔊「しゅうしんどくさいかん」と「どくさいかん」🌏"
        );
        assert!(matches!(
            text_for_speech("🌏 Hello! カタカナとひらがな", Lang::Ja),
            Cow::Borrowed(_)
        ));
    }
}
