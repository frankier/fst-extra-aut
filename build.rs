extern crate cpp_build;

fn main() {
    println!("cargo:rustc-link-lib=hfst");
    cpp_build::build("src/hfst.rs");
}
