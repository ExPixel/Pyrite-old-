extern crate cc;
extern crate bindgen;

use std::env;
use std::path::PathBuf;

pub fn main() {
    cc::Build::new()
        .cpp(true)
        .flag_if_supported("-Wno-return-type-c-linkage")
        .define("IMGUI_DISABLE_OBSOLETE_FUNCTIONS", "1")
        .include("./cimgui")
        .include("./cimgui/imgui")
        .file("./cimgui/imgui/imgui.cpp")
        .file("./cimgui/imgui/imgui_draw.cpp")
        .file("./cimgui/imgui/imgui_demo.cpp")
        .file("./cimgui/imgui/imgui_widgets.cpp")
        .file("./cimgui/cimgui.cpp")
        .compile("libcimgui");

    let bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .rustfmt_bindings(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings.");

    // only rebuild if these files are changed.
    println!("cargo:rerun-if-changed=./cimgui/imgui/imgui.cpp");
    println!("cargo:rerun-if-changed=./cimgui/imgui/imgui_draw.cpp");
    println!("cargo:rerun-if-changed=./cimgui/imgui/imgui_demo.cpp");
    println!("cargo:rerun-if-changed=./cimgui/imgui/imgui_widgets.cpp");
    println!("cargo:rerun-if-changed=./cimgui/cimgui.cpp");
    println!("cargo:rerun-if-changed=./cimgui/cimgui.h");
    println!("cargo:rerun-if-changed=./cimgui/generator/output/cimgui_impl.h");
    println!("cargo:rerun-if-changed=./wrapper.hpp");
    println!("cargo:rerun-if-changed=./cimgui/imgui/imgui.h");
    println!("cargo:rerun-if-changed=./cimgui/imgui/imgui_internal.h");
    println!("cargo:rerun-if-changed=./cimgui/imgui/imstb_rectpack.h");
    println!("cargo:rerun-if-changed=./cimgui/imgui/imstb_textedit.h");
    println!("cargo:rerun-if-changed=./cimgui/imgui/imstb_truetype.h");
    println!("cargo:rerun-if-env-changed=CC");
}
