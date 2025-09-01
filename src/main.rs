//
// Test out Rust and ratatui.
//

pub mod lvm;
pub mod lvmapp;

use color_eyre::Result;

fn main() -> Result<()> {
    if !lvm::init() {
        panic!("Failed to scan blockdevs");
    }
    color_eyre::install()?;

    unsafe {
        let current_uid = nix::libc::geteuid();
        if current_uid == 0 {
            println!("Running as root user.");
        } else {
            println!("Running as user with UID: {}", current_uid);
            panic!("Run as root/sudo! ...");
        }
    }
    let terminal = ratatui::init();
    let app_result = lvmapp::LvmApp::new().run(terminal);
    ratatui::restore();
    app_result
}
