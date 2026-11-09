//! 記録（セーブデータ）の置き場所とファイルダイアログ（Windows）
//!
//! 記録は %APPDATA%\yyhistory\saves\slot{N}.yyhsav に保存し、見出しと保存日時を slot{N}.txt に置く。
//! 書き込みは一時ファイルに書いてから置き換えるので、途中で落ちても前の記録は壊れない。

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;

/// 記録画面に出す説明
pub fn storage_note() -> &'static str {
    t!("記録はこのパソコン（%APPDATA%\\yyhistory\\saves）に保存される。別のパソコンへ移すときや控えには、書き出したファイルを読み込む", "Saves are kept on this PC (%APPDATA%\\yyhistory\\saves). To move them to another PC or keep a backup, export a file and load it there.", "存档保存在本机（%APPDATA%\\yyhistory\\saves）。如需转移到其他电脑或备份，请导出文件后再读取。")
}

const SLOTS: usize = 4;

thread_local! {
    /// 読み込んだ記録（次のフレームで反映する）
    static LOADED: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
    static NOTES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// 枠の見出しと保存日時（ファイルを毎フレーム読まないよう覚えておく）
    static INFO: RefCell<Option<Vec<Option<(String, String)>>>> = const { RefCell::new(None) };
}

fn note(s: String) {
    NOTES.with(|n| n.borrow_mut().push(s));
}

/// 設定ファイル（%APPDATA%\\yyhistory\\settings.txt）
fn settings_path() -> PathBuf {
    let d = save_dir();
    d.parent()
        .map(Path::to_path_buf)
        .unwrap_or(d)
        .join("settings.txt")
}

/// 起動時の表示言語：保存した設定、無ければ Windows の表示言語（日本語・中国語・それ以外は英語）
pub fn initial_lang() -> crate::lang::Lang {
    use crate::lang::Lang;
    if let Some(l) = std::env::var("YY_LANG")
        .ok()
        .and_then(|v| Lang::from_code(&v))
    {
        return l;
    }
    let saved = std::fs::read_to_string(settings_path()).ok().and_then(|s| {
        s.lines()
            .find_map(|l| l.strip_prefix("lang=").and_then(Lang::from_code))
    });
    if let Some(l) = saved {
        return l;
    }
    let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
    match id & 0x3ff {
        0x11 => Lang::Ja,
        0x04 => Lang::Zh,
        _ => Lang::En,
    }
}

/// 表示言語を設定ファイルに書く
pub fn save_lang(l: crate::lang::Lang) {
    let _ = write_atomic(&settings_path(), format!("lang={}\n", l.code()).as_bytes());
}

/// このゲームのデータの置き場所（%APPDATA%\\yyhistory）
pub fn app_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("yyhistory")
}

/// 記録の置き場所（無ければ作る）
fn save_dir() -> PathBuf {
    let d = app_dir().join("saves");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn data_path(slot: u8) -> PathBuf {
    save_dir().join(format!("slot{}.yyhsav", slot))
}

fn meta_path(slot: u8) -> PathBuf {
    save_dir().join(format!("slot{}.txt", slot))
}

/// いまの日時（ローカル時刻）
fn now_text() -> String {
    let t = unsafe { GetLocalTime() };
    format!(
        "{}/{:02}/{:02} {:02}:{:02}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute
    )
}

/// 一時ファイルに書いてから置き換える
fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

fn read_info(slot: u8) -> Option<(String, String)> {
    if !data_path(slot).exists() {
        return None;
    }
    let s = std::fs::read_to_string(meta_path(slot)).unwrap_or_default();
    let mut it = s.lines();
    let meta = it
        .next()
        .unwrap_or(t!("（見出しなし）", "(no title)", "（无标题）"))
        .to_string();
    let date = it.next().unwrap_or("").to_string();
    Some((meta, date))
}

fn with_info<T>(f: impl FnOnce(&mut Vec<Option<(String, String)>>) -> T) -> T {
    INFO.with(|i| {
        let mut i = i.borrow_mut();
        let v = i.get_or_insert_with(|| (0..SLOTS as u8).map(read_info).collect());
        f(v)
    })
}

/// 記録を枠に保存する
pub fn save_slot(slot: u8, data: &[u8], meta: &str) {
    let date = now_text();
    let r = write_atomic(&data_path(slot), data)
        .and_then(|_| write_atomic(&meta_path(slot), format!("{}\n{}\n", meta, date).as_bytes()));
    match r {
        Ok(()) => with_info(|v| {
            if let Some(e) = v.get_mut(slot as usize) {
                *e = Some((meta.to_string(), date));
            }
        }),
        Err(e) => note(tf!(
            "記録できなかった：{}",
            "Could not save: {}",
            "无法保存：{}",
            e
        )),
    }
}

/// 枠の記録を読み込む（次のフレームで反映する）
pub fn load_slot(slot: u8) {
    match std::fs::read(data_path(slot)) {
        Ok(d) => LOADED.with(|l| *l.borrow_mut() = Some(d)),
        Err(e) => note(tf!(
            "読み込めなかった：{}",
            "Could not load: {}",
            "无法读取：{}",
            e
        )),
    }
}

/// 枠の見出しと保存日時（空きなら None）
pub fn slot_info(slot: u8) -> Option<(String, String)> {
    with_info(|v| v.get(slot as usize).cloned().flatten())
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

fn filter() -> &'static str {
    t!(
        "YYHistoryの記録 (*.yyhsav)\0*.yyhsav\0すべてのファイル (*.*)\0*.*\0\0",
        "YYHistory save (*.yyhsav)\0*.yyhsav\0All files (*.*)\0*.*\0\0",
        "历史相册存档 (*.yyhsav)\0*.yyhsav\0所有文件 (*.*)\0*.*\0\0"
    )
}

/// 枠の記録をファイルとして書き出す（保存先を尋ねる）
pub fn export_slot(slot: u8) {
    let src = data_path(slot);
    let data = match std::fs::read(&src) {
        Ok(d) => d,
        Err(_) => {
            note(
                t!(
                    "この枠には記録がない",
                    "This slot is empty",
                    "此栏位没有存档"
                )
                .into(),
            );
            return;
        }
    };
    let (date, _) = slot_info(slot).map(|(m, d)| (d, m)).unwrap_or_default();
    let stamp: String = date.chars().filter(|c| c.is_ascii_digit()).collect();
    let name = format!(
        "yyhistory-{}-{}.yyhsav",
        if slot == 0 {
            "auto".to_string()
        } else {
            format!("slot{}", slot)
        },
        stamp
    );
    let mut buf = [0u16; 1024];
    let w = wide(&name);
    buf[..w.len().min(1023)].copy_from_slice(&w[..w.len().min(1023)]);
    let filter: Vec<u16> = filter().encode_utf16().collect();
    let title = HSTRING::from(t!("記録を書き出す", "Export save", "导出存档"));
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: unsafe { GetActiveWindow() },
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrDefExt: w!("yyesav"),
        lpstrTitle: PCWSTR(title.as_ptr()),
        Flags: OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    if !unsafe { GetSaveFileNameW(&mut ofn) }.as_bool() {
        return;
    }
    let path = from_wide(&buf);
    match std::fs::write(&path, data) {
        Ok(()) => note(tf!("書き出した：{}", "Exported: {}", "已导出：{}", path)),
        Err(e) => note(tf!(
            "書き出せなかった：{}",
            "Could not export: {}",
            "无法导出：{}",
            e
        )),
    }
}

/// 記録ファイルを選んで読み込む
/// 「キャッシュを消して再読み込み」はブラウザ版だけ
pub const CAN_RELOAD: bool = false;

pub fn reload_clear() {}

pub fn import_file() {
    let mut buf = [0u16; 1024];
    let filter: Vec<u16> = filter().encode_utf16().collect();
    let title = HSTRING::from(t!(
        "記録ファイルを読み込む",
        "Load a save file",
        "读取存档文件"
    ));
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: unsafe { GetActiveWindow() },
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrTitle: PCWSTR(title.as_ptr()),
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    if !unsafe { GetOpenFileNameW(&mut ofn) }.as_bool() {
        return;
    }
    match std::fs::read(from_wide(&buf)) {
        Ok(d) => LOADED.with(|l| *l.borrow_mut() = Some(d)),
        Err(e) => note(tf!(
            "読み込めなかった：{}",
            "Could not load: {}",
            "无法读取：{}",
            e
        )),
    }
}

/// 読み込んだ記録を受け取る
pub fn take_loaded() -> Option<Vec<u8>> {
    LOADED.with(|l| l.borrow_mut().take())
}

/// 通知を受け取る
pub fn take_notes() -> Vec<String> {
    NOTES.with(|n| std::mem::take(&mut *n.borrow_mut()))
}

/// 乱数の種（時刻から）
pub fn random_u32() -> u32 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    (t ^ (t >> 32) ^ (std::process::id() as u128) << 7) as u32
}

/// 操作の足跡（異常終了の調査ログへ）
pub fn crumb(s: String) {
    crate::crashlog::crumb(s);
}

/// Voice settings are independent of the save and language preferences.
pub fn voice_enabled() -> bool {
    std::fs::read_to_string(app_dir().join("voice.txt")).map_or(true, |s| s.trim() != "off")
}
pub fn save_voice(enabled: bool) {
    let _ = write_atomic(
        &app_dir().join("voice.txt"),
        if enabled { b"on" } else { b"off" },
    );
}

/// SAPI runs on its own COM thread, so speech never blocks the game loop.
pub fn speak(text: &str) {
    #[cfg(test)]
    {
        let _ = text;
    }
    #[cfg(not(test))]
    {
        use std::sync::{mpsc, OnceLock};
        static SPEECH: OnceLock<mpsc::Sender<(String, crate::lang::Lang)>> = OnceLock::new();
        let sender = SPEECH.get_or_init(|| {
            let (tx, rx) = mpsc::channel::<(String, crate::lang::Lang)>();
            let _ = std::thread::Builder::new().name("voice-guide".into()).spawn(move || unsafe {
                use windows::Win32::{Media::Speech::*, System::Com::*};
                if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() { return; }
                {
                    let voice: windows::core::Result<ISpVoice> = CoCreateInstance(&SpVoice, None, CLSCTX_ALL);
                    if let Ok(voice) = voice {
                        let _ = voice.SetRate(-2);
                        while let Ok(mut request) = rx.recv() {
                            while let Ok(newer) = rx.try_recv() { request = newer; }
                            let (text, lang) = request;
                            let category: windows::core::Result<ISpObjectTokenCategory> = CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL);
                            if let Ok(category) = category {
                                let _ = category.SetId(w!("HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Speech\\Voices"), false);
                                let filter = HSTRING::from(match lang {
                                    crate::lang::Lang::Ja => "Language=411",
                                    crate::lang::Lang::Zh => "Language=804",
                                    crate::lang::Lang::En => "Language=409",
                                });
                                if let Ok(tokens) = category.EnumTokens(&filter, PCWSTR::null()) {
                                    if let Ok(token) = tokens.Item(0) { let _ = voice.SetVoice(&token); }
                                }
                            }
                            let _ = voice.Speak(&HSTRING::from(text), (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0 | SPF_IS_NOT_XML.0) as u32, None);
                        }
                    }
                }
                CoUninitialize();
            });
            tx
        });
        // 本文は日本語だけなので、表示言語にかかわらず日本語の音声と読み辞書で読む
        let lang = crate::lang::Lang::Ja;
        let text = crate::speech::text_for_speech(text, lang);
        let _ = sender.send((text.into_owned(), lang));
    }
}
