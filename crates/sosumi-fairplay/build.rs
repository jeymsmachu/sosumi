//! Builds the vendored `FairPlay` bridge and the Android stub libraries.
//!
//! Three things are produced, in this order:
//!
//! 1. **The embedded libhybris blob.** `libhybris-core.so` becomes a C array and
//!    is linked in, so the loader can place it in a memfd at runtime rather than
//!    reading a file from disk or requiring it on the search path.
//! 2. **The loader and the flat C API.** These load Apple's libraries through
//!    that blob and expose a small C surface to Rust.
//! 3. **The Android stub libraries.** Apple's objects declare `libandroid.so`,
//!    `libOpenSLES.so`, and `libdl.so` as dependencies but never call into them.
//!    Empty stand-ins are built here and embedded as bytes, so installing the
//!    runtime payload needs no compiler on the user's machine.
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
use std::process::Command;

fn main() {
    // Cargo always sets OUT_DIR; a build script cannot return a Result, so this
    // is the only way to report its absence.
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));

    // The layout mirrors Adam's `drm/` tree: `native/` is the equivalent of
    // `drm/` (the loader and flat API) and `native/session/` of `drm/native/`
    // (the vendored wrapper). Keeping the shape identical means the sources
    // compile unchanged and a future diff against upstream stays readable.
    let native = Path::new("native");
    let session = native.join("session");

    let loader = ["embedded_loader.c", "drm_hybris.c", "drm_linux.c"];
    let session_c =
        ["main.c", "cmdline.c", "hybris_stubs.c", "drm_lib.c", "cJSON.c", "android_shim.c"];
    let session_cxx = ["main.cpp", "guard.cpp"];

    for name in &loader {
        println!("cargo:rerun-if-changed={}", native.join(name).display());
    }
    for name in session_c.iter().chain(&session_cxx) {
        println!("cargo:rerun-if-changed={}", session.join(name).display());
    }
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=third_party/libhybris-core.so");

    build_blob_and_loader(&out_dir, native);
    build_session(native, &session, &session_c, &session_cxx);
    build_stubs(&out_dir);

    // Host libraries the vendored code needs. Nothing here comes from Android.
    for lib in ["stdc++", "pthread", "dl", "m", "curl", "crypto", "ssl"] {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
}

/// Embeds the libhybris core and compiles the loader and flat C API.
fn build_blob_and_loader(out_dir: &Path, native: &Path) {
    let hybris_core = Path::new("third_party/libhybris-core.so");
    assert!(
        hybris_core.exists(),
        "third_party/libhybris-core.so is missing.\n\
         It is the libhybris loader that makes Apple's bionic libraries loadable\n\
         from a glibc process. Fetch it with scripts/fetch-fairplay-runtime.sh,\n\
         which pins the source and verifies the checksum."
    );

    let blob = emit_blob(hybris_core, &out_dir.join("libhybris_core_blob.c"));

    let mut build = cc::Build::new();
    build
        .files(
            ["embedded_loader.c", "drm_hybris.c", "drm_linux.c"]
                .iter()
                .map(|name| native.join(name)),
        )
        .include(native)
        // `drm_hybris.c` includes `native/session/drm_lib.h`, so the crate root
        // is on the path as well as `native/`.
        .include(Path::new("."))
        .file(&blob)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-format-truncation")
        .flag_if_supported("-Wno-stringop-overflow")
        .flag_if_supported("-Wno-unused-variable")
        // One cosmetic const-qualifier warning in the vendored loader. Silenced
        // so a real warning is not buried in it.
        .flag_if_supported("-Wno-discarded-qualifiers")
        .warnings(true);
    build.compile("sosumi_fairplay_loader");
}

/// Compiles the vendored host-native `FairPlay` session.
///
/// Two passes, because these sources are mixed C and C++ and each must be
/// compiled as its own language. Passing `.cpp(true)` over a `.c` file makes the
/// compiler parse it as C++, which rejects `malloc`'s implicit `void*`
/// conversion — the first thing that breaks.
fn build_session(native: &Path, session: &Path, session_c: &[&str], session_cxx: &[&str]) {
    let common: [(&str, Option<&str>); 5] = [
        ("DRM_LIB_BUILD", None),
        ("MyRelease", None),
        // `hybris_stubs.c` maps the Android loader calls the session code expects
        // onto the embedded loader's exports. Renaming these breaks the link
        // silently at run time rather than at compile time.
        ("android_dlopen", Some("aml_android_dlopen")),
        ("android_dlsym", Some("aml_android_dlsym")),
        ("android_update_LD_LIBRARY_PATH", Some("aml_android_update_LD_LIBRARY_PATH")),
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

    let mut c_build = cc::Build::new();
    c_build
        .files(session_c.iter().map(|name| session.join(name)))
        .include(session)
        .include(native)
        // `main.c` uses `accept4` and `gettimeofday`, which need these to be
        // declared and included. Adam passes the same for the same reason; `cc`
        // forwards each flag separately, so the pair goes in one string.
        .define("_GNU_SOURCE", None)
        .flag("-include")
        .flag("sys/time.h");
    for (name, value) in &common {
        c_build.define(name, *value);
    }
    for flag in quiet {
        c_build.flag_if_supported(flag);
    }
    c_build.warnings(false);
    c_build.compile("sosumi_fairplay_session_c");

    let mut cxx_build = cc::Build::new();
    cxx_build
        .cpp(true)
        .std("c++17")
        .files(session_cxx.iter().map(|name| session.join(name)))
        .include(session)
        .include(native)
        .flag_if_supported("-fexceptions");
    for (name, value) in &common {
        cxx_build.define(name, *value);
    }
    for flag in quiet {
        cxx_build.flag_if_supported(flag);
    }
    cxx_build.warnings(false);
    cxx_build.compile("sosumi_fairplay_session_cxx");
}

/// Builds the three Android stub libraries and embeds them as bytes.
///
/// Apple's objects declare these as dependencies, so the loader must be able to
/// resolve them, but nothing is ever called. Building them here means the app can
/// write them out when installing the runtime payload, so a user never needs a
/// compiler.
fn build_stubs(out_dir: &Path) {
    let source = out_dir.join("stub.c");
    std::fs::write(
        &source,
        "/* Empty stand-in for an Android system library Apple's objects declare\n \
         * but never call. Exports nothing, on purpose. */\n \
         char sosumi_android_stub;\n",
    )
    .expect("the stub source is writable");

    // `libdl.so` must carry the `LIBC` version node: Apple's objects are linked
    // against that node, and a versioned symbol lookup fails outright when the
    // node is absent. The dl* functions themselves come from the hybris linker.
    let map = out_dir.join("libdl.map");
    std::fs::write(&map, "/* The version node Android's libdl exports under. */\nLIBC { };\n")
        .expect("the version map is writable");

    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_owned());
    let mut generated = String::with_capacity(1024);

    for (target, version_script) in
        [("libandroid.so", None), ("libOpenSLES.so", None), ("libdl.so", Some(&map))]
    {
        let output = out_dir.join(target);
        let mut command = Command::new(&cc);
        // `-nostdlib` is deliberate: these must not link a C runtime. They are
        // loaded where bionic's symbols matter, and a glibc dependency inside a
        // file the bionic linker loads is exactly the mix that breaks.
        command
            .arg("-shared")
            .arg("-fPIC")
            .arg("-nostdlib")
            .arg("-O2")
            .arg("-Wl,--hash-style=both")
            .arg(format!("-Wl,-soname,{target}"))
            .arg("-o")
            .arg(&output)
            .arg(&source);
        if let Some(map) = version_script {
            command.arg(format!("-Wl,--version-script={}", map.display()));
        }
        let status = command.status().expect("the C compiler runs");
        assert!(status.success(), "building {target} failed");

        // Exposed as Rust constants so the installer can write them out.
        let ident = target.replace('.', "_").to_uppercase();
        let path = output.display();
        writeln!(generated, "pub(crate) static {ident}: &[u8] = include_bytes!(\"{path}\");")
            .expect("a String write cannot fail");
    }

    std::fs::write(out_dir.join("stubs.rs"), generated).expect("the stub module is writable");
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
fn emit_blob(source: &Path, target: &Path) -> PathBuf {
    let bytes = std::fs::read(source).expect("the libhybris blob is readable");

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

    std::fs::write(target, out).expect("the blob source is writable");
    target.to_path_buf()
}
