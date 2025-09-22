//
// Test out Rust and ratatui.
//

pub mod lvm;
pub mod lvmapp;

use std::process::{ExitCode, Termination};

pub enum LinuxExitCode {
    EOk,
    EErr(u8),
}

impl Termination for LinuxExitCode {
    fn report(self) -> ExitCode {
        match self {
            LinuxExitCode::EOk => ExitCode::SUCCESS,
            LinuxExitCode::EErr(v) => ExitCode::from(v),
        }
    }
}

fn main() -> LinuxExitCode {
    unsafe {
        let current_uid = nix::libc::geteuid();
        if current_uid != 0 {
            println!("Running as user with UID: {}", current_uid);
            println!("Run as root/sudo! ...");
            return LinuxExitCode::EErr(1);
        }
    }

    let terminal = ratatui::init();
    let result = terminal.size();
    match result {
        Err(e) => {
            println!("Failed to get terminal size: {:#}", e);
            return LinuxExitCode::EErr(1);
        }
        Ok(s) => {
            if s.height < 25 || s.width < 80 {
                print!("Terminal is too small, need a size of at least 25x80!");
                return LinuxExitCode::EErr(1);
            }
        }
    }

    let app_result = lvmapp::LvmApp::new().run(terminal);
    ratatui::restore();
    match app_result {
        Ok(_) => return LinuxExitCode::EOk,
        Err(e) => {
            println!("Error: {:#}", e);
            return LinuxExitCode::EErr(1);
        }
    }
}
