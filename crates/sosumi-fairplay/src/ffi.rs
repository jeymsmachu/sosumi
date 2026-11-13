//! The foreign declarations.
//!
//! This is the only module in the workspace with `unsafe` in it, and it is kept
//! to two kinds of statement: `extern "C"` declarations, and the thin wrappers
//! that call them. Everything the rest of the application sees is in [`super::session`].
//!
//! The types below must match `native/drm_lib.h` and `drm_hybris.h` exactly. A
//! mismatch is not caught by the compiler on either side, so each struct records
//! its C counterpart and the build fails loudly if the header is absent.

use std::ffi::{c_char, c_int, c_void};

/// Called when the DRM library needs a credential or a verification code.
///
/// `kind` is `"credentials"` or `"2fa"`. The callee writes into `out_buf`, which
/// the caller allocates with `out_size` bytes.
pub type AuthCallback = unsafe extern "C" fn(
    kind: *const c_char,
    out_buf: *mut c_char,
    out_size: c_int,
    ud: *mut c_void,
);

/// Called on state transitions, with one of the strings named in [`super::session::SessionState`].
pub type StateCallback = unsafe extern "C" fn(state: *const c_char, ud: *mut c_void);

/// Mirrors `drm_lib_config_t` in `native/session/drm_lib.h`.
///
/// Field order is load-bearing: the C side reads by offset.
///
/// No `Debug`: every field is a raw pointer, and printing their addresses in a
/// log would be noise at best. The safe wrapper reports state instead.
#[repr(C)]
pub struct DrmLibConfig {
    pub base_dir: *const c_char,
    pub lib64_dir: *const c_char,
    pub username: *const c_char,
    pub password: *const c_char,
    pub device_info: *const c_char,
    pub offline_only: c_int,
    pub auth_cb: Option<AuthCallback>,
    pub auth_ud: *mut c_void,
    pub state_cb: Option<StateCallback>,
    pub state_ud: *mut c_void,
}

/// Reports whether each pointer is set, never what it points at.
///
/// A derived `Debug` is not possible for raw pointers, and printing addresses
/// would be noise. Printing the *contents* would be far worse: two of these point
/// at a password and at a token. So the fields are reported as set or unset.
impl std::fmt::Debug for DrmLibConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let present = |p: *const c_char| if p.is_null() { "unset" } else { "set" };
        f.debug_struct("DrmLibConfig")
            .field("base_dir", &present(self.base_dir))
            .field("lib64_dir", &present(self.lib64_dir))
            .field("username", &present(self.username))
            .field("password", &present(self.password))
            .field("device_info", &present(self.device_info))
            .field("offline_only", &self.offline_only)
            .field("auth_cb", &self.auth_cb.is_some())
            .field("state_cb", &self.state_cb.is_some())
            .finish_non_exhaustive()
    }
}

unsafe extern "C" {
    /// Runs the full DRM initialisation and blocks until the `FairPlay` lease is
    /// acquired and account tokens are cached.
    ///
    /// Returns 0 on success. Must be called exactly once.
    pub fn drm_lib_init(config: *const DrmLibConfig) -> c_int;

    /// Releases the init lock after `drm_lib_init` failed to return.
    ///
    /// The C documentation calls this an exception barrier: it exists so a
    /// process can recover from a sign-in attempt that hung rather than
    /// deadlocking every later attempt.
    pub fn drm_lib_abort_init();

    /// Loads Apple's libraries through the embedded `libhybris` core.
    ///
    /// Returns 0 on success. `linker_dir` holds the hybris linker, `lib64_dir`
    /// holds Apple's `.so` files, and `core_path` names the embedded blob.
    pub fn hybris_backend_init(
        linker_dir: *const c_char,
        lib64_dir: *const c_char,
        core_path: *const c_char,
    ) -> c_int;

    /// Why the last sign-in failed, in words fit for a sign-in form.
    ///
    /// Returns an empty string when unknown. Valid until the next attempt.
    pub fn hybris_last_error() -> *const c_char;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    /// The C struct's layout, written out so a mismatch is visible rather than
    /// merely unequal. Field order is load-bearing: the C side reads by offset.
    ///
    /// ```text
    /// offset  0  base_dir       *const c_char   8
    /// offset  8  lib64_dir      *const c_char   8
    /// offset 16  username       *const c_char   8
    /// offset 24  password       *const c_char   8
    /// offset 32  device_info    *const c_char   8
    /// offset 40  offline_only   c_int           4
    /// offset 44  (padding)                      4
    /// offset 48  auth_cb        fn pointer      8
    /// offset 56  auth_ud        *mut c_void     8
    /// offset 64  state_cb       fn pointer      8
    /// offset 72  state_ud       *mut c_void     8
    ///            total                          80
    /// ```
    #[test]
    fn the_config_layout_matches_the_c_struct() {
        assert_eq!(size_of::<DrmLibConfig>(), 80);
        // The padding after the single `int` is what a hand-written size check
        // usually gets wrong, so the two offsets that straddle it are pinned too.
        assert_eq!(std::mem::offset_of!(DrmLibConfig, offline_only), 40);
        assert_eq!(std::mem::offset_of!(DrmLibConfig, auth_cb), 48);
        assert_eq!(std::mem::offset_of!(DrmLibConfig, state_ud), 72);
    }

    #[test]
    fn callbacks_are_pointer_sized() {
        assert_eq!(size_of::<Option<AuthCallback>>(), size_of::<*const c_char>());
        assert_eq!(size_of::<Option<StateCallback>>(), size_of::<*const c_char>());
    }
}
