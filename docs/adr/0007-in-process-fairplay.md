# 0007 — Apple's FairPlay libraries load in-process through libhybris

- Status: Accepted
- Date: 2026-11-13

## Context

Sign-in has to use Apple's own account flow. The web path is closed: Apple's
sign-in page loads MusicKit, which registers a DRM key system while starting up,
and WebKitGTK as distributions build it has Encrypted Media compiled out. So
`navigator.requestMediaKeySystemAccess` does not exist, MusicKit cannot
initialize, and the page reports an authorization error after a correct password
and verification code. See `docs/PLATFORM-LIMITS.md` for the measurement.

That leaves Apple's native account libraries, the ones the Android client uses.
They are x86-64 ELF objects that link against **bionic** (`libc.so`, `liblog.so`,
`libdl.so`), not glibc, so they cannot simply be `dlopen`ed by a normal Linux
process.

Four ways to run them were considered:

1. **A full Android container or emulator.** Correct isolation, but hundreds of
   megabytes and a whole second operating system to keep alive.
2. **QEMU, as the wrapper's recommended launcher does.** Portable, but emulation
   is pure overhead when the guest and host share an architecture.
3. **A separate helper process with a bionic-compatible loader.**
4. **In-process loading through libhybris.**

## Decision

Option 4: load Apple's libraries **in-process**, through libhybris.

libhybris exists for exactly this: it loads bionic-linked libraries inside a
normal glibc process by providing the bionic symbols they expect and translating
the ones that need it. There is no virtualization layer because none is needed —
the architecture already matches.

Two details that make it work:

- **libhybris's core is embedded, not installed.** It is compiled into the
  library as a blob and loaded from an anonymous `memfd` via `/proc/self/fd/N`,
  so nothing is extracted to disk and the host's dynamic loader is never
  disturbed.
- **Apple's libraries load from a real path, not a memfd.** FairPlay resolves
  files relative to its own library location, so loading it from a memfd makes it
  crash. The core is embedded; the libraries are read from a directory.

Apple's libraries depend on `libandroid.so`, `libOpenSLES.so`, and `libdl.so`,
which they declare but never call. Standing in empty stubs for those avoids
pulling in Android's graphics and media stacks. The `libdl.so` stub must carry the
`LIBC` version node, because Apple's objects are linked against that node and the
linker resolves versioned symbols strictly.

## Consequences

**Easier:** sign-in works with no emulation, no container, and no second
operating system. The measured result on the development machine is
`hybris_backend_init: rc=0` after loading `libCoreFP.so`,
`libstoreservicescore.so`, and `libandroidappmusic.so`. The wrapper's native layer
builds with host `gcc` and `g++` — the Android NDK is only needed for the
Android-layer binary, which is not used.

**Harder:** the loading code is C and is inherently `unsafe`. It is confined to
one crate (`sosumi-fairplay`), which relaxes `forbid(unsafe_code)` for that FFI
surface alone; every other crate keeps it. That boundary is the thing to watch in
review.

**Accepted:** the loaded libraries are Apple's, obtained by the user and never
redistributed, checksum-pinned, and used to authenticate an account the user
pays for. See `docs/LEGAL-SCOPE.md` for the scope this project holds itself to.
