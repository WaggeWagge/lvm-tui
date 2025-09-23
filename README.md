# lvm-tui
Lvm tui/gui written in rust/ratatui (https://ratatui.rs/). Read-only/listing existing LV/VG/PVs.

<img width="824" height="713" alt="Screenshot From 2025-09-18 19-42-06" src="https://github.com/user-attachments/assets/c34fbd07-1fca-4e3f-89e5-7a0691b84115" />



Build and run
================

Install rust compiler etc. 

https://rustup.rs/

Build:

$ cargo build --release

Make a debian package, optional:

$ cargo deb # 

Run it:

$ sudo ./target/release/lvm-tui

Dependencies for building
=====================================================================
lvm-tui - n/a.

Runtime deps
=====================================================================
lvm2 tools (lvs, pvs, vgs)

