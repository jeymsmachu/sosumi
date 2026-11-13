//! Apple ID sign-in and lossless keys, through Apple's own `FairPlay` libraries.
//!
//! # What this crate is for
//!
//! Apple's web sign-in cannot be hosted on Linux: the page loads `MusicKit`, which
//! registers a DRM key system while starting up, and `WebKitGTK` as distributions
//! build it has Encrypted Media compiled out. Measured on the development
//! machine, `navigator.requestMediaKeySystemAccess` does not exist, so `MusicKit`
//! fails to initialize and the page reports an authorization error after a
//! correct password and verification code. See `docs/PLATFORM-LIMITS.md`.
//!
//! The remaining route is Apple's own account libraries, the ones its Android
//! client uses. They are x86-64 ELF objects that link bionic rather than glibc,
//! so they cannot simply be `dlopen`ed. This crate loads them through embedded
//! `libhybris`, which is what bridges that gap without emulation, a container, or a
//! second operating system. See `docs/adr/0007-in-process-fairplay.md`.
//!
//! # Scope, stated plainly
//!
//! The wrapper here authenticates an account and obtains keys for streams that
//! account is entitled to. It does not decrypt anything independently, does not
//! extract keys for redistribution, and provides no mechanism to play without a
//! subscription. The libraries it drives are Apple's, supplied by the user, never
//! redistributed, and checksum-pinned. See `docs/LEGAL-SCOPE.md` and `NOTICE.md`.
//!
//! # The unsafe boundary
//!
//! Every other crate in this workspace sets `forbid(unsafe_code)`. This one
//! cannot: it calls into C. The relaxation is confined to [`ffi`], which is a
//! thin declaration layer, and the safe wrapper in [`session`] is the only thing
//! the rest of the application touches.
//!
//! The rules that keep the boundary honest:
//!
//! 1. **One call per foreign function, wrapped immediately.** No raw pointer
//!    crosses into safe code.
//! 2. **C strings are built in one place.** `CString` ownership and lifetime are
//!    handled in one helper rather than at each call site.
//! 3. **Callbacks are the only inversion.** The C library calls back for
//!    credentials and state; those trampolines are declared beside the types they
//!    need and nowhere else.
//! 4. **The session is a singleton.** The C library maintains global state and
//!    documents `drm_lib_init` as callable once. [`session::Session`] enforces
//!    that with a guard, so a second attempt fails in Rust rather than in C.

// The workspace sets `forbid(unsafe_code)`; this is the one crate that cannot
// honour it, because it calls into C. `forbid` cannot be overridden by a child
// module, so it is relaxed here for the whole crate and re-tightened by hand:
// `ffi` holds the declarations, `session` holds the calls, and everything else
// is ordinary Rust. `clippy::undocumented_unsafe_blocks` is denied in the
// workspace lints, so every `unsafe` block above these two modules must justify
// itself in a comment or the build fails.
#![allow(unsafe_code)]
// Re-tighten what can still be enforced: no `unsafe fn` without saying why, and
// no raw pointer arithmetic in safe-looking code.
#![deny(unsafe_op_in_unsafe_fn)]

pub mod ffi;
pub mod runtime;
pub mod session;

pub use runtime::{InstallOutcome, PinnedFile, RuntimeLayout, RuntimeManifest, RuntimeStatus};
pub use session::{AuthChallenge, ChallengeHandler, Session, SessionState, SignInError};
