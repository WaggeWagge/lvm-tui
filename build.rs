use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to tell rustc to link: -lglib-2.0  -lbd_lvm
    println!("cargo:rustc-link-lib=glib-2.0");
    println!("cargo:rustc-link-lib=bd_lvm");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-I/usr/include/glib-2.0")
        .clang_arg("-I/usr/lib/x86_64-linux-gnu/glib-2.0/include")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
