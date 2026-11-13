//! Builds the vendored `FairPlay` bridge.
//!
//! The bridge is C and C++, and it is compiled here rather than shipped as a
//! binary so the whole build is reproducible from source and auditable with the
//! rest of the tree.
//!
//! Three things are compiled, in this order:
//!
//! 1. **The embedded libhybris blob.** `libhybris-core.so` is turned into a C
//!    array and linked in, so the loader can place it in a memfd at runtime
//!    instead of writing a file to disk or requiring it on the search path.
//! 2. **The loader and the flat C API.** These load Apple's libraries through
//!    the blob above and expose a small C surface to Rust.
//! 3. **The vendored host-native `FairPlay` session.** Apple's own account and
//!    key-exchange calls, driven from the host.
//!
//! The Android NDK is not required. Nothing here is built for Android: Apple's
//! libraries are x86-64 ELF objects linking bionic, and libhybris is what lets a
//! glibc process load them. See `docs/adr/0007-in-process-fairplay.md`.

// The workspace denies `expect` and `panic`, which is right for the application
// but cannot hold for a build script: Cargo gives it no way to return an error,
// so a failure must panic. Allowing it here, once, with that reason, is clearer
// than contorting every fallible step into a `match` that ends in a panic anyway.
// Each `expect` still names the step that failed, so the message stays useful.
#![allow(clippy::expect_used, clippy::panic)]

use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    // The layout mirrors Adam's `drm/` tree: `native/` is the equivalent of
    // `drm/` (the loader and flat API) and `native/session/` of `drm/native/`
    // (the vendored wrapper). Keeping the shape identical means the sources
    // compile unchanged and a future diff against upstream stays readable.
    let native = Path::new("native");
    let session = native.join("session");

    // Source files, grouped so the reason for each group is visible.
    let loader = ["embedded_loader.c", "drm_hybris.c", "drm_linux.c"];
    let session_c = [
        "main.c",
        "cmdline.c",
        "hybris_stubs.c",
        "drm_lib.c",
        "cJSON.c",
        "android_shim.c",
    ];
    let session_cxx = ["main.cpp", "guard.cpp"];

    // A rebuild must happen when any vendored source or header changes.
    for name in &loader {
        println!("cargo:rerun-if-changed={}", native.join(name).display());
    }
    for name in session_c.iter().chain(&session_cxx) {
        println!("cargo:rerun-if-changed={}", session.join(name).display());
    }
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-changed=build.rs");

    // ── The embedded libhybris blob ───────────────────────────────────────────
    //
    // Generated as a C array. The loader places it in an anonymous memfd and
    // loads it from `/proc/self/fd/N`, so no file is written and the host's
    // dynamic loader never sees it.
    let hybris_core = Path::new("third_party/libhybris-core.so");
    assert!(
        hybris_core.exists(),
        "third_party/libhybris-core.so is missing.\n\
         It is the libhybris loader that makes Apple's bionic libraries loadable\n\
         from a glibc process. Fetch it with scripts/fetch-fairplay-runtime.sh,\n\
         which pins the source and verifies the checksum."
    );
    let blob_source = generate_blob(hybris_core, &out_dir);

    // ── The loader and flat API ───────────────────────────────────────────────
    //
    // `drm_hybris.c` includes `native/drm_lib.h`, so the include root must be the
    // crate directory rather than `native/`: the header is referenced by its path
    // from there. The `native/` entry is kept as well so a bare `#include` of a
    // sibling still resolves.
    let mut loader_build = cc::Build::new();
    loader_build
        .files(loader.iter().map(|name| native.join(name)))
        .include(native)
        .include(Path::new("."))
        .file(&blob_source)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-format-truncation")
        .flag_if_supported("-Wno-stringop-overflow")
        // The sources define `_GNU_SOURCE` themselves; declaring it here too
        // produces a redefinition warning that hides real ones.
        .flag_if_supported("-Wno-unused-variable")
        // The vendored loader has one cosmetic const-qualifier warning. Silencing
        // it keeps the build output readable, so a real warning is not buried.
        .flag_if_supported("-Wno-discarded-qualifiers")
        .warnings(true);
    loader_build.compile("sosumi_fairplay_loader");

    // ── The vendored host-native session ──────────────────────────────────────
    //
    // Compiler settings, in two passes, because these sources are mixed C and
    // C++ and must be compiled as their own language. Passing `.cpp(true)` over
    // a `.c` file makes `gcc` parse it as C++, which rejects `malloc`'s implicit
    // `void*` conversion — the first thing that breaks.
    //
    // The common settings are shared so the two passes cannot drift apart.
    let common: Vec<(&str, Option<&str>)> = vec![
        ("DRM_LIB_BUILD", None),
        ("MyRelease", None),
        // `hybris_stubs.c` maps the Android loader calls the session code expects
        // onto the embedded loader's exports. Renaming these would break the
        // link silently at run time rather than at compile time.
        ("android_dlopen", Some("aml_android_dlopen")),
        ("android_dlsym", Some("aml_android_dlsym")),
        ("android_dlerror", Some("aml_android_dlerror")),
        (
            "android_update_LD_LIBRARY_PATH",
            Some("aml_android_update_LD_LIBRARY_PATH"),
        ),
    ];

    let quiet = [
        "-Wno-unused-variable",
        "-Wno-unused-function",
        "-Wno-unused-but-set-variable",
        "-Wno-format-truncation",
        "-Wno-stringop-overflow",
        "-Wno-discarded-qualifiers",
        "-Wno-int-conversion",
    ];

    let mut session_c_build = cc::Build::new();
    session_c_build
        .files(session_c.iter().map(|name| session.join(name)))
        .include(&session)
        .include(native)
        // `main.c` uses `accept4` and `gettimeofday`, both of which need these
        // headers to be declared. Adam passes the same pair for the same reason.
        // `cc` forwards each flag separately, so the `-include` and its argument
        // go in one string rather than two entries.
        .define("_GNU_SOURCE", None)
        .flag("-include").flag("sys/time.h");
    for (name, value) in &common {
        session_c_build.define(name, *value);
    }
    for flag in quiet {
        session_c_build.flag_if_supported(flag);
    }
    session_c_build.warnings(false);
    session_c_build.compile("sosumi_fairplay_session_c");

    let mut session_cxx_build = cc::Build::new();
    session_cxx_build
        .cpp(true)
        .std("c++17")
        .files(session_cxx.iter().map(|name| session.join(name)))
        .include(&session)
        .include(native);
    for (name, value) in &common {
        session_cxx_build.define(name, *value);
    }
    for flag in quiet {
        session_cxx_build.flag_if_supported(flag);
    }
    session_cxx_build.flag_if_supported("-fexceptions");
    session_cxx_build.warnings(false);
    session_cxx_build.compile("sosumi_fairplay_session_cxx");

    // The libraries the vendored code links against. All are host libraries;
    // nothing here comes from Android.
    println!("cargo:rustc-link-lib=dylib=stdc++");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-link-lib=dylib=dl");
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=curl");
    println!("cargo:rustc-link-lib=dylib=crypto");
    println!("cargo:rustc-link-lib=dylib=ssl");
}

/// Turns a binary into a C array the linker can take.
///
/// Emitted as a C file rather than as a Rust constant because the C compiler
/// handles a multi-megabyte array literal trivially, and `rustc` does not.
///
/// # Panics
///
/// A build script cannot return a `Result`, so every failure here panics with the
/// step that failed named in the message.
fn generate_blob(source: &Path, out_dir: &Path) -> PathBuf {
    let bytes = std::fs::read(source).expect("the libhybris blob is readable");
    let target = out_dir.join("libhybris_core_blob.c");

    let mut out = String::with_capacity(bytes.len() * 5 + 256);
    out.push_str("/* Generated by build.rs. Do not edit. */\n");
    out.push_str("#include <stdint.h>\n\n");
    // These names are what `embedded_loader.c` declares. Renaming them would
    // break the link, and the failure would appear at run time rather than here.
    out.push_str("const uint8_t libhybris_core_so_data[] = {\n");
    for (index, byte) in bytes.iter().enumerate() {
        if index % 16 == 0 {
            out.push_str("  ");
        }
        // Writing to a `String` is infallible.
        write!(out, "0x{byte:02x},").expect("a String write cannot fail");
        if index % 16 == 15 {
            out.push('\n');
        }
    }
    out.push_str("\n};\n\n");
    writeln!(out, "const uint64_t libhybris_core_so_size = {};", bytes.len())
        .expect("a String write cannot fail");

    std::fs::write(&target, out).expect("the blob source is writable");
    target
}
