# lvm-tui
Lvm tui/gui written in rust/ratatui (https://ratatui.rs/). Read-only/listing existing LV/VG/PVs.

<img width="654" height="613" alt="image" src="https://github.com/user-attachments/assets/c2d7d906-25ee-4d03-b6d5-8c65edd87a8e" />

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

