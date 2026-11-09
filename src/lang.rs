//! 表示言語（日本語・英語・中国語）
//!
//! 画面の文字は `t!`（決まった文）・`tf!`（数や名前を差し込む文）で三言語を並べて書き、
//! プラットフォーム層で利用する表示言語の補助。ゲーム本文は現在日本語。

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Ja = 0,
    En = 1,
    Zh = 2,
}

pub const LANGS: [Lang; 3] = [Lang::Ja, Lang::En, Lang::Zh];

static LANG: AtomicU8 = AtomicU8::new(0);

impl Lang {
    /// 言語の名前（その言語で書く）
    pub fn native(self) -> &'static str {
        match self {
            Lang::Ja => "日本語",
            Lang::En => "English",
            Lang::Zh => "中文",
        }
    }
    /// 設定ファイルに書く符号
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ja => "ja",
            Lang::En => "en",
            Lang::Zh => "zh",
        }
    }
    pub fn from_code(s: &str) -> Option<Lang> {
        LANGS.into_iter().find(|l| l.code() == s.trim())
    }
    pub fn next(self) -> Lang {
        LANGS[(self as usize + 1) % LANGS.len()]
    }
}

pub fn lang() -> Lang {
    LANGS[LANG.load(Ordering::Relaxed) as usize % LANGS.len()]
}

pub fn set_lang(l: Lang) {
    LANG.store(l as u8, Ordering::Relaxed);
}

/// 三言語の文から、いまの言語のものを選ぶ
pub fn pick(ja: &'static str, en: &'static str, zh: &'static str) -> &'static str {
    match lang() {
        Lang::Ja => ja,
        Lang::En => en,
        Lang::Zh => zh,
    }
}

/// 決まった文：`t!("日本語", "English", "中文")`
#[macro_export]
macro_rules! t {
    ($ja:expr, $en:expr, $zh:expr $(,)?) => {
        $crate::lang::pick($ja, $en, $zh)
    };
}

/// 数や名前を差し込む文：`tf!("{}年", "{} years", "{}年", n)`
#[macro_export]
macro_rules! tf {
    ($ja:literal, $en:literal, $zh:literal $(, $a:expr)* $(,)?) => {
        match $crate::lang::lang() {
            $crate::lang::Lang::Ja => format!($ja $(, $a)*),
            $crate::lang::Lang::En => format!($en $(, $a)*),
            $crate::lang::Lang::Zh => format!($zh $(, $a)*),
        }
    };
}

/// 窓の題名
#[cfg_attr(not(windows), allow(dead_code))]
pub fn window_title() -> &'static str {
    "時空事件簿"
}
