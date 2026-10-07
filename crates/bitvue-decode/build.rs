//! Locates libvvdec for the optional `vvdec` feature.
//!
//! Without this the crate relied on `#[link(name = "vvdec")]` and the linker's default search
//! path, which does not include Homebrew's `/opt/homebrew/lib`, so a perfectly good install
//! failed with "library 'vvdec' not found". pkg-config also lets us pin the ABI: the bindings in
//! `src/vvdec/ffi.rs` were generated from the 3.x headers, and struct layouts differ between major
//! versions.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "vvdec")]
    link_vvdec();
}

#[cfg(feature = "vvdec")]
fn link_vvdec() {
    let result = pkg_config::Config::new()
        .range_version("3.0".."4.0")
        .probe("libvvdec");
    if let Err(e) = result {
        panic!(
            "the `vvdec` feature needs libvvdec 3.x (found via pkg-config): {e}\n\
             macOS: `brew install vvdec`. Debian/Ubuntu ship no vvdec package: build\n\
             https://github.com/fraunhoferhhi/vvdec (cmake; install it, then point PKG_CONFIG_PATH\n\
             at its `lib/pkgconfig`)."
        );
    }
}
