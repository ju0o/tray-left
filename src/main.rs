#![cfg_attr(windows, windows_subsystem = "windows")]

mod icon;
mod look;
mod settings;
mod usage;

#[cfg(windows)]
mod win;

fn main() {
    #[cfg(windows)]
    if let Err(failure) = win::run() {
        if let Some(scene) = failure.scene {
            eprintln!(
                "Jusage preview failed at {} for {} (code {}).",
                failure.stage, scene, failure.code
            );
        } else {
            eprintln!(
                "Jusage preview failed at {} (code {}).",
                failure.stage, failure.code
            );
        }
        std::process::exit(failure.code);
    }
    #[cfg(not(windows))]
    {
        eprintln!("이 프로그램은 Windows 트레이에서만 실행합니다.");
        std::process::exit(1);
    }
}
