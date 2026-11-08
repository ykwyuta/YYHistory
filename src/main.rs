#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() {
    #[cfg(windows)]
    yyhistory::desktop_main();
    #[cfg(not(windows))]
    eprintln!("Build the browser version with bash build.sh");
}
