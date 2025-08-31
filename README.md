# lvm-tui
Lvm tui/gui written in rust/ratatui. Very basic atm, read-only/listin existing LV/VG/PVs.

<img width="698" height="695" alt="image" src="https://github.com/user-attachments/assets/790b8077-a3b9-4db6-8f9f-df850b443a52" />

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
lvm-tui requires liblockdev version 3, glib2-devel and clang to build.

c-libs/packages
-----------------------
Common:
 clang - for code-generation

debian/ubuntu:
 libblockdev-dev (v 3+)
 libblockdev-lvm-dev
 libglib2.0-dev

redhat/fedora/suse:
 libblockdev-devel
 libblockdev-lvm-devel
 glib2-devel

archlinux:
 libblockdev
 libblockdev-lvm
 --possibly glib2 as well.

Runtime deps
=====================================================================
debian/ubuntu:
 libblockdev-lvm3
 libblockdev3

