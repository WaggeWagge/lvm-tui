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
    let terminal = ratatui::init();
    let app_result = lvmapp::LvmApp::new().run(terminal);
    ratatui::restore();
    app_result
}
