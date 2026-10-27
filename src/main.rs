#![cfg_attr(windows, windows_subsystem = "windows")]

mod icon;
mod settings;
mod usage;

#[cfg(windows)]
mod win;

fn main() {
    #[cfg(windows)]
    win::run();
    #[cfg(not(windows))]
    {
        eprintln!("이 프로그램은 Windows 트레이에서만 실행합니다.");
        std::process::exit(1);
    }
}
