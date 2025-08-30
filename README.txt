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

